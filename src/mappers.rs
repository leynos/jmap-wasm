//! Output mapping helpers for JMAP mail objects.

use std::collections::HashMap;

use jmap_codec::mail::{Email, EmailAddress};

use crate::outputs::{MailboxInfo, MessageDetail, MessageSummary};

/// Convert one codec mailbox into the tool output shape.
pub(crate) fn map_mailbox(mailbox: jmap_codec::mail::Mailbox) -> MailboxInfo {
    MailboxInfo {
        id: mailbox.id,
        name: mailbox.name,
        role: mailbox.role,
        parent_id: mailbox.parent_id,
        sort_order: mailbox.sort_order,
        is_subscribed: mailbox.is_subscribed,
        total_emails: mailbox.total_emails,
        unread_emails: mailbox.unread_emails,
    }
}

/// Convert one codec email into the summary output shape.
pub(crate) fn map_message_summary(message: Email) -> MessageSummary {
    MessageSummary {
        id: message.id,
        thread_id: message.thread_id,
        mailbox_ids: truthy_map_keys(&message.mailbox_ids),
        keywords: truthy_map_keys(&message.keywords),
        received_at: message.received_at,
        subject: message.subject,
        from: map_addresses(message.from),
        preview: message.preview,
        has_attachment: message.has_attachment,
    }
}

/// Convert one codec email into the detailed output shape.
pub(crate) fn map_message_detail(message: Email) -> MessageDetail {
    let text_body = extract_text_body(&message);

    MessageDetail {
        id: message.id,
        thread_id: message.thread_id,
        mailbox_ids: truthy_map_keys(&message.mailbox_ids),
        keywords: truthy_map_keys(&message.keywords),
        received_at: message.received_at,
        subject: message.subject,
        from: map_addresses(message.from),
        to: map_addresses(message.to),
        preview: message.preview,
        has_attachment: message.has_attachment,
        text_body,
    }
}

/// Return the default summary properties requested from `Email/get`.
pub(crate) fn message_properties() -> Vec<String> {
    vec![
        "id".to_owned(),
        "threadId".to_owned(),
        "mailboxIds".to_owned(),
        "keywords".to_owned(),
        "receivedAt".to_owned(),
        "subject".to_owned(),
        "from".to_owned(),
        "preview".to_owned(),
        "hasAttachment".to_owned(),
    ]
}

/// Return the detailed properties requested from `Email/get`.
pub(crate) fn message_properties_with_body() -> Vec<String> {
    let mut properties = message_properties();
    properties.extend([
        "to".to_owned(),
        "textBody".to_owned(),
        "bodyValues".to_owned(),
    ]);
    properties
}

fn map_addresses(addresses: Option<Vec<EmailAddress>>) -> Vec<String> {
    addresses.map_or_else(Vec::new, |items| {
        items
            .into_iter()
            .map(|address| match address.name {
                Some(name) if !name.is_empty() => format!("{name} <{}>", address.email),
                _ => address.email,
            })
            .collect()
    })
}

fn extract_text_body(message: &Email) -> Option<String> {
    let references = message.text_body.as_ref()?;
    let body_values = message.body_values.as_ref()?;
    let body = references
        .iter()
        .filter_map(|reference| body_values.get(&reference.part_id))
        .map(|value| value.value.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    if body.is_empty() { None } else { Some(body) }
}

pub(crate) fn truthy_map_keys(values: &HashMap<String, bool>) -> Vec<String> {
    let mut keys = values
        .iter()
        .filter(|(_, enabled)| **enabled)
        .map(|(key, _)| key.clone())
        .collect::<Vec<_>>();
    keys.sort();
    keys
}
