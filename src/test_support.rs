//! Shared fixtures for unit and behavioural tests.

use std::collections::VecDeque;

use crate::{
    errors::ToolError,
    host::{Host, HostHttpRequest, HostHttpResponse, HostLogLevel},
    outputs::{
        GetMessageOutput, ListMailboxesOutput, ListMessagesOutput, MailboxInfo, MarkSeenOutput,
        MessageDetail, MessageSummary,
    },
    service::{GetMessageRequest, JmapConfig, JmapService, ListMessagesRequest, MarkSeenRequest},
};

/// Shared behavioural-test world.
#[derive(Default)]
pub(crate) struct ToolWorld {
    /// Whether the fake host exposes the configured secret.
    pub(crate) has_secret: bool,
    /// Fake service responses queued by the scenario steps.
    pub(crate) service: FakeService,
    /// Successful JSON output captured from execution.
    pub(crate) output: Option<String>,
    /// Error string captured from execution.
    pub(crate) error: Option<String>,
}

impl ToolWorld {
    /// Build a fake host matching the current world state.
    pub(crate) fn host(&self) -> FakeHost {
        if self.has_secret {
            FakeHost::with_secret("jmap_token")
        } else {
            FakeHost::default()
        }
    }
}

/// Fake host used by unit and behavioural tests.
#[derive(Default)]
pub(crate) struct FakeHost {
    secrets: Vec<String>,
}

impl FakeHost {
    /// Construct a host with one configured secret.
    pub(crate) fn with_secret(secret: &str) -> Self {
        Self {
            secrets: vec![secret.to_owned()],
        }
    }
}

impl Host for FakeHost {
    fn log(&self, _level: HostLogLevel, _message: &str) {}

    fn secret_exists(&self, name: &str) -> bool {
        self.secrets.iter().any(|secret| secret == name)
    }

    fn http_request(&self, _request: &HostHttpRequest) -> Result<HostHttpResponse, ToolError> {
        Ok(HostHttpResponse {
            status: 200,
            headers: serde_json::Map::new(),
            body: Vec::new(),
        })
    }
}

/// Fake service used by tests.
#[derive(Default)]
pub(crate) struct FakeService {
    /// Mailbox results queued for `list_mailboxes`.
    pub(crate) mailboxes: VecDeque<ListMailboxesOutput>,
    /// Message listing results queued for `list_messages`.
    pub(crate) messages: VecDeque<ListMessagesOutput>,
    /// Message detail results queued for `get_message`.
    pub(crate) message_details: VecDeque<GetMessageOutput>,
    /// Keyword update results queued for `mark_seen`.
    pub(crate) mark_seen: VecDeque<MarkSeenOutput>,
}

impl JmapService for FakeService {
    fn list_mailboxes<H: Host>(
        &self,
        _host: &H,
        _config: &JmapConfig,
    ) -> Result<ListMailboxesOutput, ToolError> {
        self.mailboxes
            .front()
            .cloned()
            .ok_or_else(|| ToolError::InvalidRequest("missing fake mailboxes".to_owned()))
    }

    fn list_messages<H: Host>(
        &self,
        _host: &H,
        request: &ListMessagesRequest,
    ) -> Result<ListMessagesOutput, ToolError> {
        self.messages.front().cloned().map_or_else(
            || {
                Ok(ListMessagesOutput {
                    account_id: request
                        .config
                        .account_id
                        .clone()
                        .unwrap_or_else(|| "acc-1".to_owned()),
                    mailbox_id: request
                        .mailbox_id
                        .clone()
                        .or_else(|| Some("mbx-1".to_owned())),
                    position: request.position,
                    total: Some(1),
                    messages: vec![message_summary()],
                })
            },
            Ok,
        )
    }

    fn get_message<H: Host>(
        &self,
        _host: &H,
        request: &GetMessageRequest,
    ) -> Result<GetMessageOutput, ToolError> {
        self.message_details.front().cloned().map_or_else(
            || {
                Ok(GetMessageOutput {
                    account_id: request
                        .config
                        .account_id
                        .clone()
                        .unwrap_or_else(|| "acc-1".to_owned()),
                    message: message_detail(&request.email_id),
                })
            },
            Ok,
        )
    }

    fn mark_seen<H: Host>(
        &self,
        _host: &H,
        request: &MarkSeenRequest,
    ) -> Result<MarkSeenOutput, ToolError> {
        self.mark_seen.front().cloned().map_or_else(
            || {
                Ok(MarkSeenOutput {
                    account_id: request
                        .config
                        .account_id
                        .clone()
                        .unwrap_or_else(|| "acc-1".to_owned()),
                    email_id: request.email_id.clone(),
                    seen: true,
                    keywords: vec!["$seen".to_owned()],
                })
            },
            Ok,
        )
    }
}

/// Build the standard Inbox mailbox fixture.
pub(crate) fn inbox_mailbox(total_emails: u64, unread_emails: u64) -> MailboxInfo {
    MailboxInfo {
        id: "mbx-1".to_owned(),
        name: "Inbox".to_owned(),
        role: Some("inbox".to_owned()),
        parent_id: None,
        sort_order: Some(10),
        is_subscribed: Some(true),
        total_emails: Some(total_emails),
        unread_emails: Some(unread_emails),
    }
}

/// Build the standard message listing fixture.
pub(crate) fn message_summary() -> MessageSummary {
    MessageSummary {
        id: "email-1".to_owned(),
        thread_id: Some("thread-1".to_owned()),
        mailbox_ids: vec!["mbx-1".to_owned()],
        keywords: vec!["$seen".to_owned()],
        received_at: Some("2026-03-09T10:00:00Z".to_owned()),
        subject: Some("Hello".to_owned()),
        from: vec!["Alice <alice@example.com>".to_owned()],
        preview: Some("Body preview".to_owned()),
        has_attachment: Some(false),
    }
}

/// Build the standard message detail fixture.
pub(crate) fn message_detail(email_id: &str) -> MessageDetail {
    MessageDetail {
        id: email_id.to_owned(),
        thread_id: Some("thread-1".to_owned()),
        mailbox_ids: vec!["mbx-1".to_owned()],
        keywords: vec!["$seen".to_owned()],
        received_at: Some("2026-03-09T10:00:00Z".to_owned()),
        subject: Some("Hello".to_owned()),
        from: vec!["Alice <alice@example.com>".to_owned()],
        to: vec!["Bob <bob@example.com>".to_owned()],
        preview: Some("Body preview".to_owned()),
        has_attachment: Some(false),
        text_body: Some("Body".to_owned()),
    }
}
