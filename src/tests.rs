//! Unit and behavioural tests for the IMAP tool.

use std::{cell::RefCell, collections::VecDeque};

use rstest::{fixture, rstest};
use rstest_bdd_macros::{given, scenario, then, when};

use crate::{
    actions::ImapAction,
    execute_with,
    host::{Host, HostLogLevel},
    outputs::{
        ListMailboxesOutput, ListMessagesOutput, MailboxInfo, MarkSeenOutput, MessageDetail,
        MessageSummary,
    },
    service::{
        ConnectionConfig, GetMessageRequest, ImapService, ListMessagesRequest, MarkSeenRequest,
    },
};

#[rstest]
fn parses_flattened_connection_fields() {
    let action = ImapAction::parse(
        r#"{"action":"list_messages","host":"mail.example.com","username":"alice","password":"secret"}"#,
    )
    .expect("action should parse");

    assert!(matches!(action, ImapAction::ListMessages { .. }));
}

#[rstest]
fn execute_checks_optional_secret_presence() {
    let host = FakeHost::default();
    let service = FakeService::default();
    let params = r#"{
        "action":"list_mailboxes",
        "host":"mail.example.com",
        "username":"alice",
        "password":"secret",
        "password_secret_name":"imap_password"
    }"#;

    let error = execute_with(params, &host, &service).expect_err("secret check should fail");

    assert_eq!(
        error.to_string(),
        "Required secret 'imap_password' is not configured"
    );
}

#[rstest]
fn execute_serializes_service_output() {
    let mut service = FakeService::default();
    service.mailboxes.push_back(ListMailboxesOutput {
        mailboxes: vec![MailboxInfo {
            name: "INBOX".to_owned(),
            delimiter: Some('/'),
            attributes: vec!["\\Unmarked".to_owned()],
        }],
    });
    let host = FakeHost::with_secret("imap_password");
    let params = r#"{
        "action":"list_mailboxes",
        "host":"mail.example.com",
        "username":"alice",
        "password":"secret",
        "password_secret_name":"imap_password"
    }"#;

    let output = execute_with(params, &host, &service).expect("execution should succeed");

    assert!(output.contains("\"action\":\"list_mailboxes\""));
    assert!(output.contains("\"INBOX\""));
}

#[derive(Default)]
struct ToolWorld {
    has_secret: bool,
    service: FakeService,
    output: Option<String>,
    error: Option<String>,
}

#[fixture]
fn world() -> ToolWorld {
    ToolWorld::default()
}

#[given("the IMAP password secret exists")]
fn secret_exists(world: &mut ToolWorld) {
    world.has_secret = true;
}

#[given("the service returns one mailbox")]
fn one_mailbox(world: &mut ToolWorld) {
    world.service.mailboxes.push_back(ListMailboxesOutput {
        mailboxes: vec![MailboxInfo {
            name: "INBOX".to_owned(),
            delimiter: Some('/'),
            attributes: vec!["\\Unmarked".to_owned()],
        }],
    });
}

#[when("the tool lists mailboxes")]
fn list_mailboxes(world: &mut ToolWorld) {
    let host = if world.has_secret {
        FakeHost::with_secret("imap_password")
    } else {
        FakeHost::default()
    };
    let params = r#"{
        "action":"list_mailboxes",
        "host":"mail.example.com",
        "username":"alice",
        "password":"secret",
        "password_secret_name":"imap_password"
    }"#;

    match execute_with(params, &host, &world.service) {
        Ok(output) => world.output = Some(output),
        Err(error) => world.error = Some(error.to_string()),
    }
}

#[then("the execution succeeds")]
fn execution_succeeds(world: &ToolWorld) {
    assert!(world.error.is_none());
    assert!(world.output.is_some());
}

#[then("the response contains mailbox {mailbox}")]
fn response_contains_mailbox(world: &ToolWorld, mailbox: String) {
    let output = world.output.as_ref().expect("output should be set");
    assert!(output.contains(&mailbox));
}

#[then("the execution fails with {message}")]
fn execution_fails(world: &ToolWorld, message: String) {
    let error = world.error.as_ref().expect("error should be set");
    assert_eq!(error.as_str(), message);
}

#[scenario(
    path = "tests/features/imap_tool.feature",
    name = "Listing mailboxes succeeds"
)]
fn bdd_list_mailboxes(world: ToolWorld) {
    let _ = world;
}

#[scenario(
    path = "tests/features/imap_tool.feature",
    name = "Missing secret fails fast"
)]
fn bdd_missing_secret(world: ToolWorld) {
    let _ = world;
}

#[derive(Default)]
struct FakeHost {
    secrets: Vec<String>,
    logs: RefCell<Vec<(HostLogLevel, String)>>,
}

impl FakeHost {
    fn with_secret(secret: &str) -> Self {
        Self {
            secrets: vec![secret.to_owned()],
            logs: RefCell::default(),
        }
    }
}

impl Host for FakeHost {
    fn log(&self, level: HostLogLevel, message: &str) {
        self.logs.borrow_mut().push((level, message.to_owned()));
    }

    fn secret_exists(&self, name: &str) -> bool {
        self.secrets.iter().any(|secret| secret == name)
    }
}

#[derive(Default)]
struct FakeService {
    mailboxes: VecDeque<ListMailboxesOutput>,
    messages: VecDeque<ListMessagesOutput>,
    message_details: VecDeque<MessageDetail>,
    mark_seen: VecDeque<MarkSeenOutput>,
}

impl ImapService for FakeService {
    fn list_mailboxes(
        &self,
        _connection: &ConnectionConfig,
    ) -> Result<ListMailboxesOutput, crate::errors::ToolError> {
        self.mailboxes.front().cloned().ok_or_else(|| {
            crate::errors::ToolError::InvalidRequest("missing fake mailboxes".to_owned())
        })
    }

    fn list_messages(
        &self,
        request: &ListMessagesRequest,
    ) -> Result<ListMessagesOutput, crate::errors::ToolError> {
        self.messages.front().cloned().map_or_else(
            || {
                Ok(ListMessagesOutput {
                    mailbox: request.mailbox.clone(),
                    messages: vec![MessageSummary {
                        sequence: 1,
                        uid: Some(10),
                        flags: vec!["\\Seen".to_owned()],
                        size: Some(42),
                        subject: Some("Hello".to_owned()),
                        date: Some("Mon, 1 Jan 2024 12:00:00 +0000".to_owned()),
                        from: vec!["Alice <alice@example.com>".to_owned()],
                    }],
                })
            },
            Ok,
        )
    }

    fn get_message(
        &self,
        request: &GetMessageRequest,
    ) -> Result<crate::outputs::GetMessageOutput, crate::errors::ToolError> {
        self.message_details.front().cloned().map_or_else(
            || {
                Ok(crate::outputs::GetMessageOutput {
                    mailbox: request.mailbox.clone(),
                    message: MessageDetail {
                        sequence: request.sequence,
                        uid: Some(10),
                        flags: vec!["\\Seen".to_owned()],
                        subject: Some("Hello".to_owned()),
                        date: Some("Mon, 1 Jan 2024 12:00:00 +0000".to_owned()),
                        from: vec!["Alice <alice@example.com>".to_owned()],
                        body: Some("Body".to_owned()),
                    },
                })
            },
            |message| {
                Ok(crate::outputs::GetMessageOutput {
                    mailbox: request.mailbox.clone(),
                    message,
                })
            },
        )
    }

    fn mark_seen(
        &self,
        request: &MarkSeenRequest,
    ) -> Result<MarkSeenOutput, crate::errors::ToolError> {
        self.mark_seen.front().cloned().map_or_else(
            || {
                Ok(MarkSeenOutput {
                    mailbox: request.mailbox.clone(),
                    sequence: request.sequence,
                    seen: true,
                })
            },
            Ok,
        )
    }
}
