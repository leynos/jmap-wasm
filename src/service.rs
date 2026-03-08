//! Service abstractions for IMAP actions.

use serde::{Deserialize, Serialize};

use crate::{
    errors::ToolError,
    host::Host,
    outputs::{
        GetMessageOutput, ListMailboxesOutput, ListMessagesOutput, MailboxInfo, MarkSeenOutput,
        MessageDetail, MessageSummary,
    },
    protocol::ImapSession,
};

/// Shared IMAP connection configuration.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub(crate) struct ConnectionConfig {
    /// IMAP hostname.
    pub(crate) host: String,
    /// Plain IMAP TCP port.
    #[serde(default = "default_port")]
    pub(crate) port: u16,
    /// LOGIN username.
    pub(crate) username: String,
    /// LOGIN password.
    pub(crate) password: String,
    /// Optional secret name used only for `secret_exists` preflight.
    pub(crate) password_secret_name: Option<String>,
}

impl ConnectionConfig {
    /// Verify that any optional secret preflight passes.
    pub(crate) fn verify_secret<H: Host>(&self, host: &H) -> Result<(), ToolError> {
        if let Some(secret_name) = &self.password_secret_name
            && !host.secret_exists(secret_name)
        {
            return Err(ToolError::MissingSecret(secret_name.clone()));
        }
        Ok(())
    }
}

/// Request for listing messages.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ListMessagesRequest {
    /// Connection details.
    pub(crate) connection: ConnectionConfig,
    /// Mailbox to query.
    pub(crate) mailbox: String,
    /// IMAP sequence set.
    pub(crate) sequence_set: String,
}

impl ListMessagesRequest {
    /// Construct a list request while filling defaults.
    pub(crate) fn new(
        connection: ConnectionConfig,
        mailbox: Option<String>,
        sequence_set: Option<String>,
    ) -> Self {
        Self {
            connection,
            mailbox: mailbox.unwrap_or_else(default_mailbox),
            sequence_set: sequence_set.unwrap_or_else(default_sequence_set),
        }
    }
}

/// Request for fetching one message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct GetMessageRequest {
    /// Connection details.
    pub(crate) connection: ConnectionConfig,
    /// Mailbox to query.
    pub(crate) mailbox: String,
    /// Message sequence number.
    pub(crate) sequence: u32,
}

impl GetMessageRequest {
    /// Construct a fetch request while filling defaults.
    pub(crate) fn new(
        connection: ConnectionConfig,
        mailbox: Option<String>,
        sequence: u32,
    ) -> Self {
        Self {
            connection,
            mailbox: mailbox.unwrap_or_else(default_mailbox),
            sequence,
        }
    }
}

/// Request for updating one message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MarkSeenRequest {
    /// Connection details.
    pub(crate) connection: ConnectionConfig,
    /// Mailbox to query.
    pub(crate) mailbox: String,
    /// Message sequence number.
    pub(crate) sequence: u32,
}

impl MarkSeenRequest {
    /// Construct a flag-update request while filling defaults.
    pub(crate) fn new(
        connection: ConnectionConfig,
        mailbox: Option<String>,
        sequence: u32,
    ) -> Self {
        Self {
            connection,
            mailbox: mailbox.unwrap_or_else(default_mailbox),
            sequence,
        }
    }
}

/// Transport-agnostic IMAP operations used by request execution.
pub(crate) trait ImapService {
    /// List the mailboxes visible to the logged-in account.
    fn list_mailboxes(
        &self,
        connection: &ConnectionConfig,
    ) -> Result<ListMailboxesOutput, ToolError>;

    /// List messages from one mailbox.
    fn list_messages(&self, request: &ListMessagesRequest)
    -> Result<ListMessagesOutput, ToolError>;

    /// Fetch one full message.
    fn get_message(&self, request: &GetMessageRequest) -> Result<GetMessageOutput, ToolError>;

    /// Mark one message as seen.
    fn mark_seen(&self, request: &MarkSeenRequest) -> Result<MarkSeenOutput, ToolError>;
}

/// Production IMAP implementation backed by `imap-next`.
pub(crate) struct NetworkImapService;

impl ImapService for NetworkImapService {
    fn list_mailboxes(
        &self,
        connection: &ConnectionConfig,
    ) -> Result<ListMailboxesOutput, ToolError> {
        let mut session = ImapSession::connect(connection)?;
        let mailboxes = session
            .list_mailboxes()?
            .into_iter()
            .map(MailboxInfoModel::into_output)
            .collect();

        Ok(ListMailboxesOutput { mailboxes })
    }

    fn list_messages(
        &self,
        request: &ListMessagesRequest,
    ) -> Result<ListMessagesOutput, ToolError> {
        let mut session = ImapSession::connect(&request.connection)?;
        session.select_mailbox(&request.mailbox)?;
        let messages = session
            .list_messages(&request.sequence_set)?
            .into_iter()
            .map(MessageSummaryModel::into_output)
            .collect();

        Ok(ListMessagesOutput {
            mailbox: request.mailbox.clone(),
            messages,
        })
    }

    fn get_message(&self, request: &GetMessageRequest) -> Result<GetMessageOutput, ToolError> {
        let mut session = ImapSession::connect(&request.connection)?;
        session.select_mailbox(&request.mailbox)?;
        let message = session.get_message(request.sequence)?.into_output();

        Ok(GetMessageOutput {
            mailbox: request.mailbox.clone(),
            message,
        })
    }

    fn mark_seen(&self, request: &MarkSeenRequest) -> Result<MarkSeenOutput, ToolError> {
        let mut session = ImapSession::connect(&request.connection)?;
        session.select_mailbox(&request.mailbox)?;
        let seen = session.mark_seen(request.sequence)?;

        Ok(MarkSeenOutput {
            mailbox: request.mailbox.clone(),
            sequence: request.sequence,
            seen,
        })
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub(crate) struct MessageEnvelopeModel {
    /// Envelope subject.
    pub(crate) subject: Option<String>,
    /// Envelope date.
    pub(crate) date: Option<String>,
    /// Envelope senders.
    pub(crate) from: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct MailboxInfoModel {
    /// Mailbox name.
    pub(crate) name: String,
    /// Optional hierarchy delimiter.
    pub(crate) delimiter: Option<char>,
    /// Mailbox attributes.
    pub(crate) attributes: Vec<String>,
}

impl MailboxInfoModel {
    fn into_output(self) -> MailboxInfo {
        MailboxInfo {
            name: self.name,
            delimiter: self.delimiter,
            attributes: self.attributes,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct MessageSummaryModel {
    /// Message sequence number.
    pub(crate) sequence: u32,
    /// Message UID.
    pub(crate) uid: Option<u32>,
    /// Message flags.
    pub(crate) flags: Vec<String>,
    /// RFC822 size.
    pub(crate) size: Option<u32>,
    /// Selected envelope fields.
    pub(crate) envelope: MessageEnvelopeModel,
}

impl MessageSummaryModel {
    fn into_output(self) -> MessageSummary {
        MessageSummary {
            sequence: self.sequence,
            uid: self.uid,
            flags: self.flags,
            size: self.size,
            subject: self.envelope.subject,
            date: self.envelope.date,
            from: self.envelope.from,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct MessageDetailModel {
    /// Message sequence number.
    pub(crate) sequence: u32,
    /// Message UID.
    pub(crate) uid: Option<u32>,
    /// Message flags.
    pub(crate) flags: Vec<String>,
    /// Selected envelope fields.
    pub(crate) envelope: MessageEnvelopeModel,
    /// RFC822 message body rendered as lossy UTF-8.
    pub(crate) body: Option<String>,
}

impl MessageDetailModel {
    fn into_output(self) -> MessageDetail {
        MessageDetail {
            sequence: self.sequence,
            uid: self.uid,
            flags: self.flags,
            subject: self.envelope.subject,
            date: self.envelope.date,
            from: self.envelope.from,
            body: self.body,
        }
    }
}

const fn default_port() -> u16 {
    143
}

fn default_mailbox() -> String {
    "INBOX".to_owned()
}

fn default_sequence_set() -> String {
    "1:*".to_owned()
}
