//! Serializable response payloads returned by the tool.

use serde::Serialize;

/// Successful output variants.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(tag = "action", rename_all = "snake_case")]
pub(crate) enum ActionOutput {
    /// Successful mailbox enumeration.
    ListMailboxes(ListMailboxesOutput),
    /// Successful message listing.
    ListMessages(ListMessagesOutput),
    /// Successful message retrieval.
    GetMessage(GetMessageOutput),
    /// Successful flag update.
    MarkSeen(MarkSeenOutput),
}

/// Mailbox listing output.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub(crate) struct ListMailboxesOutput {
    /// Returned mailboxes.
    pub(crate) mailboxes: Vec<MailboxInfo>,
}

/// One mailbox summary.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub(crate) struct MailboxInfo {
    /// Mailbox name.
    pub(crate) name: String,
    /// Optional hierarchy delimiter.
    pub(crate) delimiter: Option<char>,
    /// IMAP attributes such as `\\Noselect`.
    pub(crate) attributes: Vec<String>,
}

/// Message listing output.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub(crate) struct ListMessagesOutput {
    /// Mailbox queried.
    pub(crate) mailbox: String,
    /// Message summaries.
    pub(crate) messages: Vec<MessageSummary>,
}

/// Retrieved message output.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub(crate) struct GetMessageOutput {
    /// Mailbox queried.
    pub(crate) mailbox: String,
    /// The fetched message.
    pub(crate) message: MessageDetail,
}

/// Message flag update output.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub(crate) struct MarkSeenOutput {
    /// Mailbox updated.
    pub(crate) mailbox: String,
    /// Sequence number updated.
    pub(crate) sequence: u32,
    /// Whether the resulting flags include `\\Seen`.
    pub(crate) seen: bool,
}

/// Summary fields returned by message listing.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub(crate) struct MessageSummary {
    /// Message sequence number.
    pub(crate) sequence: u32,
    /// Optional UID returned by the server.
    pub(crate) uid: Option<u32>,
    /// Flags currently set on the message.
    pub(crate) flags: Vec<String>,
    /// RFC822 size if returned by the server.
    pub(crate) size: Option<u32>,
    /// Envelope subject.
    pub(crate) subject: Option<String>,
    /// Envelope date.
    pub(crate) date: Option<String>,
    /// Envelope senders.
    pub(crate) from: Vec<String>,
}

/// Full fields returned by fetching one message.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub(crate) struct MessageDetail {
    /// Message sequence number.
    pub(crate) sequence: u32,
    /// Optional UID returned by the server.
    pub(crate) uid: Option<u32>,
    /// Flags currently set on the message.
    pub(crate) flags: Vec<String>,
    /// Envelope subject.
    pub(crate) subject: Option<String>,
    /// Envelope date.
    pub(crate) date: Option<String>,
    /// Envelope senders.
    pub(crate) from: Vec<String>,
    /// Raw message body decoded lossily as UTF-8.
    pub(crate) body: Option<String>,
}
