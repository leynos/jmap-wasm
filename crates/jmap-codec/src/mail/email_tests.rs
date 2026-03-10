//! Unit tests for email-specific JMAP types.

use std::collections::HashMap;

use rstest::rstest;

use super::{
    BodyPartReference, BodyValue, Email, EmailAddress, EmailGetArguments, EmailQueryArguments,
    EmailSetArguments,
};

#[rstest]
fn email_query_builder_sets_mailbox_and_limit() {
    let args = EmailQueryArguments::new("account-1")
        .in_mailbox("mailbox-1")
        .with_limit(25);

    assert_eq!(
        args.filter.and_then(|filter| filter.in_mailbox),
        Some("mailbox-1".to_owned())
    );
    assert_eq!(args.limit, Some(25));
}

#[rstest]
fn email_get_builder_sets_properties() {
    let args = EmailGetArguments::new("account-1", ["email-1"]).with_properties(["id", "subject"]);

    assert_eq!(args.ids, vec!["email-1".to_owned()]);
    assert_eq!(
        args.properties,
        Some(vec!["id".to_owned(), "subject".to_owned()])
    );
}

#[rstest]
fn email_body_references_parse() {
    let email: Email = serde_json::from_str(
        r#"{
            "id": "m1",
            "mailboxIds": {"inbox": true},
            "keywords": {"$seen": true},
            "textBody": [{"partId": "1"}],
            "bodyValues": {"1": {"value": "Hello"}}
        }"#,
    )
    .expect("email should parse");

    assert_eq!(
        email.text_body,
        Some(vec![BodyPartReference {
            part_id: "1".to_owned(),
        }])
    );
}

#[rstest]
fn email_addresses_preserve_optional_names() {
    let address: EmailAddress =
        serde_json::from_str(r#"{"name":"Alice Example","email":"alice@example.com"}"#)
            .expect("address should parse");

    assert_eq!(address.name.as_deref(), Some("Alice Example"));
}

#[rstest]
fn email_collects_text_body_values() {
    let email = Email {
        id: "m1".to_owned(),
        thread_id: None,
        mailbox_ids: HashMap::new(),
        keywords: HashMap::from([("$seen".to_owned(), true)]),
        received_at: None,
        subject: None,
        from: None,
        to: None,
        preview: None,
        has_attachment: None,
        text_body: Some(vec![BodyPartReference {
            part_id: "1".to_owned(),
        }]),
        body_values: Some(HashMap::from([(
            "1".to_owned(),
            BodyValue {
                value: "Hello world".to_owned(),
                is_encoding_problem: None,
                is_truncated: None,
            },
        )])),
    };

    assert_eq!(email.text_body_value(), Some("Hello world".to_owned()));
    assert_eq!(email.keyword_names(), vec!["$seen"]);
}

#[rstest]
fn email_set_mark_seen_creates_seen_patch() {
    let args = EmailSetArguments::mark_seen("account-1", "email-1");

    assert_eq!(
        args.update,
        Some(HashMap::from([(
            "email-1".to_owned(),
            serde_json::json!({ "keywords/$seen": true }),
        )]))
    );
}
