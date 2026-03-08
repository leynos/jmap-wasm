//! Direct IMAP protocol driver built on `imap-next` sans-I/O primitives.

use std::{
    io::{Read, Write},
    net::TcpStream,
    num::NonZeroU32,
    time::Duration,
};

use imap_next::{
    Interrupt, Io, State,
    client::{Client, Event, Options},
    imap_types::{
        command::{Command, CommandBody},
        response::{Data, Status, StatusKind},
    },
};

use crate::{
    errors::ToolError,
    service::{
        ConnectionConfig, MailboxInfoModel, MessageDetailModel, MessageEnvelopeModel,
        MessageSummaryModel,
    },
};

const READ_BUFFER_SIZE: usize = 8192;
const IO_TIMEOUT: Duration = Duration::from_secs(10);

/// Stateful IMAP connection for one tool execution.
pub(crate) struct ImapSession {
    stream: TcpStream,
    client: Client,
    next_tag: u32,
}

impl ImapSession {
    /// Establish a plain IMAP session and wait for the initial greeting.
    pub(crate) fn connect(connection: &ConnectionConfig) -> Result<Self, ToolError> {
        let stream = TcpStream::connect((connection.host.as_str(), connection.port))
            .map_err(ToolError::Io)?;
        stream
            .set_read_timeout(Some(IO_TIMEOUT))
            .map_err(ToolError::Io)?;
        stream
            .set_write_timeout(Some(IO_TIMEOUT))
            .map_err(ToolError::Io)?;

        let mut session = Self {
            stream,
            client: Client::new(Options::default()),
            next_tag: 1,
        };
        session.wait_for_greeting()?;
        session.login(connection)?;
        Ok(session)
    }

    /// Query all selectable and non-selectable mailboxes.
    pub(crate) fn list_mailboxes(&mut self) -> Result<Vec<MailboxInfoModel>, ToolError> {
        let outcome = self.run_command(CommandBody::list("", "*").map_err(invalid_request)?)?;

        Ok(outcome
            .data
            .into_iter()
            .filter_map(map_mailbox)
            .collect::<Vec<_>>())
    }

    /// Select one mailbox before running message-scoped commands.
    pub(crate) fn select_mailbox(&mut self, mailbox: &str) -> Result<(), ToolError> {
        self.run_command(CommandBody::select(mailbox.to_owned()).map_err(invalid_request)?)
            .map(|_| ())
    }

    /// Fetch message summaries for one sequence set.
    pub(crate) fn list_messages(
        &mut self,
        sequence_set: &str,
    ) -> Result<Vec<MessageSummaryModel>, ToolError> {
        let outcome = self.run_command(
            CommandBody::fetch(
                sequence_set,
                imap_next::imap_types::fetch::Macro::All,
                false,
            )
            .map_err(invalid_request)?,
        )?;

        Ok(outcome
            .data
            .into_iter()
            .filter_map(map_message_summary)
            .collect::<Vec<_>>())
    }

    /// Fetch one message body and envelope.
    pub(crate) fn get_message(&mut self, sequence: u32) -> Result<MessageDetailModel, ToolError> {
        let sequence_set = sequence.to_string();
        let fetch_items = vec![
            imap_next::imap_types::fetch::MessageDataItemName::Uid,
            imap_next::imap_types::fetch::MessageDataItemName::Flags,
            imap_next::imap_types::fetch::MessageDataItemName::Envelope,
            imap_next::imap_types::fetch::MessageDataItemName::Rfc822,
        ];
        let outcome = self.run_command(
            CommandBody::fetch(sequence_set.as_str(), fetch_items, false)
                .map_err(invalid_request)?,
        )?;

        outcome
            .data
            .into_iter()
            .find_map(map_message_detail)
            .ok_or_else(|| ToolError::Server("Message was not returned by the server".to_owned()))
    }

    /// Add the `\\Seen` flag to one message.
    pub(crate) fn mark_seen(&mut self, sequence: u32) -> Result<bool, ToolError> {
        let sequence_number = NonZeroU32::new(sequence)
            .ok_or_else(|| ToolError::InvalidRequest("Sequence numbers start at 1".to_owned()))?;
        let outcome = self.run_command(
            CommandBody::store(
                sequence_number,
                imap_next::imap_types::flag::StoreType::Add,
                imap_next::imap_types::flag::StoreResponse::Silent,
                vec![imap_next::imap_types::flag::Flag::Seen],
                false,
            )
            .map_err(invalid_request)?,
        )?;

        let seen = outcome
            .data
            .into_iter()
            .find_map(map_seen_flag)
            .unwrap_or(true);
        Ok(seen)
    }

    fn login(&mut self, connection: &ConnectionConfig) -> Result<(), ToolError> {
        self.run_command(
            CommandBody::login(connection.username.clone(), connection.password.clone())
                .map_err(invalid_request)?,
        )
        .map(|_| ())
    }

    fn wait_for_greeting(&mut self) -> Result<(), ToolError> {
        loop {
            match self.next_event()? {
                Event::GreetingReceived { .. } => return Ok(()),
                Event::DataReceived { .. } | Event::StatusReceived { .. } => {}
                event => {
                    return Err(ToolError::Protocol(format!(
                        "Unexpected event before greeting: {event:?}"
                    )));
                }
            }
        }
    }

    fn run_command(&mut self, body: CommandBody<'static>) -> Result<CommandOutcome, ToolError> {
        let tag = self.next_tag();
        let command = Command::new(tag.clone(), body).map_err(invalid_request)?;
        let expected_tag = tag.clone();
        self.client.enqueue_command(command);

        let mut data = Vec::new();

        loop {
            match self.next_event()? {
                Event::CommandRejected { status, .. } => {
                    return Err(status_to_server_error(&status));
                }
                Event::DataReceived { data: item } => data.push(item),
                Event::StatusReceived { status }
                    if matching_tagged_status_kind(&status, &expected_tag)
                        == Some(StatusKind::Ok) =>
                {
                    return Ok(CommandOutcome { data });
                }
                Event::StatusReceived { status }
                    if matching_tagged_status_kind(&status, &expected_tag).is_some() =>
                {
                    return Err(status_to_server_error(&status));
                }
                Event::StatusReceived { .. }
                | Event::CommandSent { .. }
                | Event::GreetingReceived { .. } => {}
                Event::ContinuationRequestReceived {
                    continuation_request,
                } => {
                    return Err(ToolError::Protocol(format!(
                        "Unexpected continuation request: {continuation_request:?}"
                    )));
                }
                Event::AuthenticateStarted { .. }
                | Event::AuthenticateContinuationRequestReceived { .. }
                | Event::AuthenticateStatusReceived { .. }
                | Event::IdleCommandSent { .. }
                | Event::IdleAccepted { .. }
                | Event::IdleRejected { .. }
                | Event::IdleDoneSent { .. } => {
                    return Err(ToolError::Protocol(
                        "Unexpected authentication or IDLE event".to_owned(),
                    ));
                }
            }
        }
    }

    fn next_event(&mut self) -> Result<Event, ToolError> {
        loop {
            match self.client.next() {
                Ok(event) => return Ok(event),
                Err(Interrupt::Io(Io::NeedMoreInput)) => self.read_more_input()?,
                Err(Interrupt::Io(Io::Output(bytes))) => {
                    self.stream.write_all(&bytes).map_err(ToolError::Io)?;
                    self.stream.flush().map_err(ToolError::Io)?;
                }
                Err(Interrupt::Error(error)) => {
                    return Err(ToolError::Protocol(error.to_string()));
                }
            }
        }
    }

    fn read_more_input(&mut self) -> Result<(), ToolError> {
        let mut buffer = [0; READ_BUFFER_SIZE];
        let count = self.stream.read(&mut buffer).map_err(ToolError::Io)?;
        if count == 0 {
            return Err(ToolError::ConnectionClosed);
        }

        let Some(bytes) = buffer.get(..count) else {
            return Err(ToolError::Protocol(
                "Read length exceeded the IMAP buffer capacity".to_owned(),
            ));
        };
        self.client.enqueue_input(bytes);
        Ok(())
    }

    fn next_tag(&mut self) -> String {
        let tag = format!("A{}", self.next_tag);
        self.next_tag = self.next_tag.saturating_add(1);
        tag
    }
}

struct CommandOutcome {
    data: Vec<Data<'static>>,
}

fn invalid_request(error: impl std::fmt::Display) -> ToolError {
    ToolError::InvalidRequest(error.to_string())
}

fn matching_tagged_status_kind(status: &Status<'static>, expected_tag: &str) -> Option<StatusKind> {
    let Status::Tagged(tagged) = status else {
        return None;
    };
    if tagged.tag.as_ref() != expected_tag {
        return None;
    }

    Some(tagged.body.kind)
}

fn map_mailbox(data: Data<'static>) -> Option<MailboxInfoModel> {
    match data {
        Data::List {
            items,
            delimiter,
            mailbox,
        }
        | Data::Lsub {
            items,
            delimiter,
            mailbox,
        } => Some(MailboxInfoModel {
            name: mailbox_to_string(&mailbox),
            delimiter: delimiter.map(|item| item.inner()),
            attributes: items.into_iter().map(|item| item.to_string()).collect(),
        }),
        _ => None,
    }
}

fn map_message_summary(data: Data<'static>) -> Option<MessageSummaryModel> {
    let Data::Fetch {
        seq: sequence,
        items,
    } = data
    else {
        return None;
    };

    let mut summary = MessageSummaryModel {
        sequence: sequence.get(),
        uid: None,
        flags: Vec::new(),
        size: None,
        envelope: MessageEnvelopeModel::default(),
    };

    for item in items {
        match item {
            imap_next::imap_types::fetch::MessageDataItem::Uid(uid) => {
                summary.uid = Some(uid.get());
            }
            imap_next::imap_types::fetch::MessageDataItem::Flags(flags) => {
                summary.flags = flags.into_iter().map(flag_fetch_to_string).collect();
            }
            imap_next::imap_types::fetch::MessageDataItem::Rfc822Size(size) => {
                summary.size = Some(size);
            }
            imap_next::imap_types::fetch::MessageDataItem::Envelope(envelope) => {
                summary.envelope = envelope_to_model(envelope);
            }
            _ => {}
        }
    }

    Some(summary)
}

fn map_message_detail(data: Data<'static>) -> Option<MessageDetailModel> {
    let Data::Fetch {
        seq: sequence,
        items,
    } = data
    else {
        return None;
    };

    let mut detail = MessageDetailModel {
        sequence: sequence.get(),
        uid: None,
        flags: Vec::new(),
        envelope: MessageEnvelopeModel::default(),
        body: None,
    };

    for item in items {
        match item {
            imap_next::imap_types::fetch::MessageDataItem::Uid(uid) => {
                detail.uid = Some(uid.get());
            }
            imap_next::imap_types::fetch::MessageDataItem::Flags(flags) => {
                detail.flags = flags.into_iter().map(flag_fetch_to_string).collect();
            }
            imap_next::imap_types::fetch::MessageDataItem::Envelope(envelope) => {
                detail.envelope = envelope_to_model(envelope);
            }
            imap_next::imap_types::fetch::MessageDataItem::Rfc822(body) => {
                detail.body = nstring_to_string(body);
            }
            _ => {}
        }
    }

    Some(detail)
}

fn map_seen_flag(data: Data<'static>) -> Option<bool> {
    let Data::Fetch { items, .. } = data else {
        return None;
    };

    items.into_iter().find_map(|item| match item {
        imap_next::imap_types::fetch::MessageDataItem::Flags(flags) => Some(
            flags
                .into_iter()
                .any(|flag| flag_fetch_to_string(flag) == "\\Seen"),
        ),
        _ => None,
    })
}

fn envelope_to_model(
    envelope: imap_next::imap_types::envelope::Envelope<'static>,
) -> MessageEnvelopeModel {
    MessageEnvelopeModel {
        subject: nstring_to_string(envelope.subject),
        date: nstring_to_string(envelope.date),
        from: envelope.from.into_iter().map(address_to_string).collect(),
    }
}

fn address_to_string(address: imap_next::imap_types::envelope::Address<'static>) -> String {
    let mailbox = nstring_to_string(address.mailbox).unwrap_or_default();
    let host = nstring_to_string(address.host).unwrap_or_default();
    let display_name_text = nstring_to_string(address.name);

    match (display_name_text, mailbox.is_empty(), host.is_empty()) {
        (Some(name_text), false, false) => format!("{name_text} <{mailbox}@{host}>"),
        (None, false, false) => format!("{mailbox}@{host}"),
        (Some(name_text), _, _) => name_text,
        (None, _, _) => mailbox,
    }
}

fn nstring_to_string(value: imap_next::imap_types::core::NString<'static>) -> Option<String> {
    value
        .into_option()
        .map(|bytes| String::from_utf8_lossy(bytes.as_ref()).into_owned())
}

fn flag_fetch_to_string(flag_fetch: imap_next::imap_types::flag::FlagFetch<'static>) -> String {
    match flag_fetch {
        imap_next::imap_types::flag::FlagFetch::Flag(flag) => flag.to_string(),
        imap_next::imap_types::flag::FlagFetch::Recent => "\\Recent".to_owned(),
    }
}

fn mailbox_to_string(mailbox: &imap_next::imap_types::mailbox::Mailbox<'static>) -> String {
    match mailbox {
        imap_next::imap_types::mailbox::Mailbox::Inbox => "INBOX".to_owned(),
        imap_next::imap_types::mailbox::Mailbox::Other(other) => {
            String::from_utf8_lossy(other.inner().as_ref()).into_owned()
        }
    }
}

fn status_to_server_error(status: &Status<'static>) -> ToolError {
    ToolError::Server(status.text().to_string())
}
