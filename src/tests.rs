//! Unit tests for the JMAP tool entry point.

use rstest::rstest;

use crate::{actions::JmapAction, execute_with, outputs::ListMailboxesOutput};

use super::test_support::{FakeHost, FakeService, inbox_mailbox};

#[rstest]
fn parses_flattened_jmap_fields() {
    let action =
        JmapAction::parse(r#"{"action":"list_messages","base_url":"https://mail.example.com"}"#)
            .expect("action should parse");

    assert!(matches!(action, JmapAction::ListMessages { .. }));
}

#[rstest]
fn execute_checks_optional_secret_presence() {
    let host = FakeHost::default();
    let service = FakeService::default();
    let params = r#"{
        "action":"list_mailboxes",
        "base_url":"https://mail.example.com",
        "auth_secret_name":"jmap_token"
    }"#;

    let error = execute_with(params, &host, &service).expect_err("secret check should fail");

    assert_eq!(
        error.to_string(),
        "Required secret 'jmap_token' is not configured"
    );
}

#[rstest]
fn execute_serializes_service_output() {
    let mut service = FakeService::default();
    service.mailboxes.push_back(ListMailboxesOutput {
        account_id: "acc-1".to_owned(),
        mailboxes: vec![inbox_mailbox(4, 2)],
    });
    let host = FakeHost::with_secret("jmap_token");
    let params = r#"{
        "action":"list_mailboxes",
        "base_url":"https://mail.example.com",
        "auth_secret_name":"jmap_token"
    }"#;

    let output = execute_with(params, &host, &service).expect("execution should succeed");

    assert!(output.contains("\"action\":\"list_mailboxes\""));
    assert!(output.contains("\"Inbox\""));
    assert!(output.contains("\"acc-1\""));
}
