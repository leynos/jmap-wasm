//! Behavioural tests for the JMAP tool.

use rstest::fixture;
use rstest_bdd_macros::{given, scenario, then, when};

use crate::{
    execute_with,
    outputs::{GetMessageOutput, ListMailboxesOutput, ListMessagesOutput, MarkSeenOutput},
};

use super::test_support::{ToolWorld, inbox_mailbox, message_detail, message_summary};

#[fixture]
fn world() -> ToolWorld {
    ToolWorld::default()
}

#[given("the JMAP auth secret exists")]
fn secret_exists(world: &mut ToolWorld) {
    world.has_secret = true;
}

#[given("the service returns one mailbox")]
fn one_mailbox(world: &mut ToolWorld) {
    world.service.mailboxes.push_back(ListMailboxesOutput {
        account_id: "acc-1".to_owned(),
        mailboxes: vec![inbox_mailbox(1, 1)],
    });
}

#[given("the service returns one message summary")]
fn one_message_summary(world: &mut ToolWorld) {
    world.service.messages.push_back(ListMessagesOutput {
        account_id: "acc-1".to_owned(),
        mailbox_id: Some("mbx-1".to_owned()),
        position: 0,
        total: Some(1),
        messages: vec![message_summary()],
    });
}

#[given("the service returns one message detail")]
fn one_message_detail(world: &mut ToolWorld) {
    world.service.message_details.push_back(GetMessageOutput {
        account_id: "acc-1".to_owned(),
        message: message_detail("email-1"),
    });
}

#[given("the service marks one message as seen")]
fn service_marks_seen(world: &mut ToolWorld) {
    world.service.mark_seen.push_back(MarkSeenOutput {
        account_id: "acc-1".to_owned(),
        email_id: "email-1".to_owned(),
        seen: true,
        keywords: vec!["$seen".to_owned()],
    });
}

#[when("the tool lists mailboxes")]
fn list_mailboxes(world: &mut ToolWorld) {
    run_tool(
        world,
        r#"{
            "action":"list_mailboxes",
            "base_url":"https://mail.example.com",
            "auth_secret_name":"jmap_token"
        }"#,
    );
}

#[when("the tool lists messages")]
fn list_messages(world: &mut ToolWorld) {
    run_tool(
        world,
        r#"{
            "action":"list_messages",
            "base_url":"https://mail.example.com",
            "auth_secret_name":"jmap_token",
            "mailbox_name":"Inbox",
            "limit":10,
            "position":0
        }"#,
    );
}

#[when("the tool fetches one message")]
fn fetch_one_message(world: &mut ToolWorld) {
    run_tool(
        world,
        r#"{
            "action":"get_message",
            "base_url":"https://mail.example.com",
            "auth_secret_name":"jmap_token",
            "email_id":"email-1"
        }"#,
    );
}

#[when("the tool marks one message as seen")]
fn mark_one_message_seen(world: &mut ToolWorld) {
    run_tool(
        world,
        r#"{
            "action":"mark_seen",
            "base_url":"https://mail.example.com",
            "auth_secret_name":"jmap_token",
            "email_id":"email-1"
        }"#,
    );
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

#[then("the response contains subject {subject}")]
fn response_contains_subject(world: &ToolWorld, subject: String) {
    let output = world.output.as_ref().expect("output should be set");
    assert!(output.contains(&subject));
}

#[then("the response contains body fragment {fragment}")]
fn response_contains_body_fragment(world: &ToolWorld, fragment: String) {
    let output = world.output.as_ref().expect("output should be set");
    assert!(output.contains(&fragment));
}

#[then("the response marks the message as seen")]
fn response_marks_message_seen(world: &ToolWorld) {
    let output = world.output.as_ref().expect("output should be set");
    assert!(output.contains(r#""seen":true"#));
    assert!(output.contains("$seen"));
}

#[then("the execution fails with {message}")]
fn execution_fails(world: &ToolWorld, message: String) {
    let error = world.error.as_ref().expect("error should be set");
    assert_eq!(error.as_str(), message);
}

#[scenario(
    path = "tests/features/jmap_tool.feature",
    name = "Listing mailboxes succeeds"
)]
fn bdd_list_mailboxes(world: ToolWorld) {
    let _ = world;
}

#[scenario(
    path = "tests/features/jmap_tool.feature",
    name = "Listing messages succeeds"
)]
fn bdd_list_messages(world: ToolWorld) {
    let _ = world;
}

#[scenario(
    path = "tests/features/jmap_tool.feature",
    name = "Fetching one message succeeds"
)]
fn bdd_get_message(world: ToolWorld) {
    let _ = world;
}

#[scenario(
    path = "tests/features/jmap_tool.feature",
    name = "Marking one message as seen succeeds"
)]
fn bdd_mark_seen(world: ToolWorld) {
    let _ = world;
}

#[scenario(
    path = "tests/features/jmap_tool.feature",
    name = "Missing secret fails fast"
)]
fn bdd_missing_secret(world: ToolWorld) {
    let _ = world;
}

fn run_tool(world: &mut ToolWorld, params: &str) {
    let host = world.host();
    match execute_with(params, &host, &world.service) {
        Ok(output) => world.output = Some(output),
        Err(error) => world.error = Some(error.to_string()),
    }
}
