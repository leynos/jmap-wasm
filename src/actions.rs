//! Request parsing and action dispatch for the IMAP tool.

use serde::Deserialize;

use crate::{
    errors::ToolError,
    host::Host,
    outputs::ActionOutput,
    service::{
        ConnectionConfig, GetMessageRequest, ImapService, ListMessagesRequest, MarkSeenRequest,
    },
};

/// Parsed IMAP tool request.
#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(tag = "action", rename_all = "snake_case")]
pub(crate) enum ImapAction {
    /// Enumerate available mailboxes.
    ListMailboxes {
        #[serde(flatten)]
        connection: ConnectionConfig,
    },
    /// List message summaries from one mailbox.
    ListMessages {
        #[serde(flatten)]
        connection: ConnectionConfig,
        mailbox: Option<String>,
        sequence_set: Option<String>,
    },
    /// Fetch one message including its body.
    GetMessage {
        #[serde(flatten)]
        connection: ConnectionConfig,
        mailbox: Option<String>,
        sequence: u32,
    },
    /// Add the `\\Seen` flag to one message.
    MarkSeen {
        #[serde(flatten)]
        connection: ConnectionConfig,
        mailbox: Option<String>,
        sequence: u32,
    },
}

impl ImapAction {
    /// Parse JSON parameters into an action.
    pub(crate) fn parse(params: &str) -> Result<Self, ToolError> {
        serde_json::from_str(params).map_err(ToolError::InvalidParameters)
    }

    /// Return the external action identifier.
    pub(crate) const fn action_name(&self) -> &'static str {
        match self {
            Self::ListMailboxes { .. } => "list_mailboxes",
            Self::ListMessages { .. } => "list_messages",
            Self::GetMessage { .. } => "get_message",
            Self::MarkSeen { .. } => "mark_seen",
        }
    }

    /// Check any requested secret-existence preflight.
    pub(crate) fn verify_secret<H: Host>(&self, host: &H) -> Result<(), ToolError> {
        let connection = match self {
            Self::ListMailboxes { connection }
            | Self::ListMessages { connection, .. }
            | Self::GetMessage { connection, .. }
            | Self::MarkSeen { connection, .. } => connection,
        };

        connection.verify_secret(host)
    }

    /// Execute one parsed action against the selected service implementation.
    pub(crate) fn execute<S: ImapService>(&self, service: &S) -> Result<ActionOutput, ToolError> {
        match self {
            Self::ListMailboxes { connection } => service
                .list_mailboxes(connection)
                .map(ActionOutput::ListMailboxes),
            Self::ListMessages {
                connection,
                mailbox,
                sequence_set,
            } => service
                .list_messages(&ListMessagesRequest::new(
                    connection.clone(),
                    mailbox.clone(),
                    sequence_set.clone(),
                ))
                .map(ActionOutput::ListMessages),
            Self::GetMessage {
                connection,
                mailbox,
                sequence,
            } => service
                .get_message(&GetMessageRequest::new(
                    connection.clone(),
                    mailbox.clone(),
                    *sequence,
                ))
                .map(ActionOutput::GetMessage),
            Self::MarkSeen {
                connection,
                mailbox,
                sequence,
            } => service
                .mark_seen(&MarkSeenRequest::new(
                    connection.clone(),
                    mailbox.clone(),
                    *sequence,
                ))
                .map(ActionOutput::MarkSeen),
        }
    }
}
