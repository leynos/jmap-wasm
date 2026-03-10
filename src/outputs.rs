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
    /// Successful keyword update.
    MarkSeen(MarkSeenOutput),
}

/// Mailbox listing output.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub(crate) struct ListMailboxesOutput {
    /// Effective account ID.
    pub(crate) account_id: String,
    /// Returned mailboxes.
    pub(crate) mailboxes: Vec<MailboxInfo>,
}

/// One mailbox summary.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub(crate) struct MailboxInfo {
    /// JMAP mailbox identifier.
    pub(crate) id: String,
    /// Mailbox display name.
    pub(crate) name: String,
    /// Optional role such as `inbox`.
    pub(crate) role: Option<String>,
    /// Optional parent mailbox ID.
    pub(crate) parent_id: Option<String>,
    /// Server sort order.
    pub(crate) sort_order: Option<u32>,
    /// Whether the mailbox is subscribed.
    pub(crate) is_subscribed: Option<bool>,
    /// Total number of emails.
    pub(crate) total_emails: Option<u64>,
    /// Total number of unread emails.
    pub(crate) unread_emails: Option<u64>,
}

/// Message listing output.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub(crate) struct ListMessagesOutput {
    /// Effective account ID.
    pub(crate) account_id: String,
    /// Mailbox filter used for the query, if any.
    pub(crate) mailbox_id: Option<String>,
    /// Query position returned by the server.
    pub(crate) position: u32,
    /// Server total, when available.
    pub(crate) total: Option<u32>,
    /// Message summaries.
    pub(crate) messages: Vec<MessageSummary>,
}

/// Retrieved message output.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub(crate) struct GetMessageOutput {
    /// Effective account ID.
    pub(crate) account_id: String,
    /// The fetched message.
    pub(crate) message: MessageDetail,
}

/// Message keyword update output.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub(crate) struct MarkSeenOutput {
    /// Effective account ID.
    pub(crate) account_id: String,
    /// Email that was updated.
    pub(crate) email_id: String,
    /// Whether the resulting keywords contain `$seen`.
    pub(crate) seen: bool,
    /// Resulting keyword set.
    pub(crate) keywords: Vec<String>,
}

/// Summary fields returned by message listing.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub(crate) struct MessageSummary {
    /// JMAP email identifier.
    pub(crate) id: String,
    /// JMAP thread identifier.
    pub(crate) thread_id: Option<String>,
    /// Mailbox IDs containing the message.
    pub(crate) mailbox_ids: Vec<String>,
    /// Message keywords such as `$seen`.
    pub(crate) keywords: Vec<String>,
    /// Received timestamp in RFC 3339 form when supplied by the server.
    pub(crate) received_at: Option<String>,
    /// Message subject.
    pub(crate) subject: Option<String>,
    /// Message senders.
    pub(crate) from: Vec<String>,
    /// Server-generated preview text.
    pub(crate) preview: Option<String>,
    /// Whether the message has attachments.
    pub(crate) has_attachment: Option<bool>,
}

/// Full fields returned by fetching one message.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub(crate) struct MessageDetail {
    /// JMAP email identifier.
    pub(crate) id: String,
    /// JMAP thread identifier.
    pub(crate) thread_id: Option<String>,
    /// Mailbox IDs containing the message.
    pub(crate) mailbox_ids: Vec<String>,
    /// Message keywords such as `$seen`.
    pub(crate) keywords: Vec<String>,
    /// Received timestamp in RFC 3339 form when supplied by the server.
    pub(crate) received_at: Option<String>,
    /// Message subject.
    pub(crate) subject: Option<String>,
    /// Message senders.
    pub(crate) from: Vec<String>,
    /// Message recipients.
    pub(crate) to: Vec<String>,
    /// Server-generated preview text.
    pub(crate) preview: Option<String>,
    /// Whether the message has attachments.
    pub(crate) has_attachment: Option<bool>,
    /// Concatenated text body extracted from `bodyValues`.
    pub(crate) text_body: Option<String>,
}
