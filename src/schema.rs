//! JSON schema and high-level description for the tool.

const SCHEMA: &str = concat!(
    "{",
    "\"type\":\"object\",",
    "\"required\":[\"action\",\"base_url\"],",
    "\"properties\":{",
    "\"action\":{",
    "\"type\":\"string\",",
    "\"enum\":[\"list_mailboxes\",\"list_messages\",\"get_message\",\"mark_seen\"],",
    "\"description\":\"The JMAP-backed mail operation to perform.\"",
    "},",
    "\"base_url\":{",
    "\"type\":\"string\",",
    "\"description\":\"Base URL of the JMAP service, for example https://mail.example.com.\"",
    "},",
    "\"account_id\":{",
    "\"type\":\"string\",",
    "\"description\":\"Optional JMAP accountId. When omitted, the tool uses the session's primary mail account.\"",
    "},",
    "\"auth_secret_name\":{",
    "\"type\":\"string\",",
    "\"description\":\"Optional host secret name to preflight with secret_exists before making HTTP requests. The tool checks only presence, not secret contents.\"",
    "},",
    "\"timeout_ms\":{",
    "\"type\":\"integer\",",
    "\"default\":30000,",
    "\"description\":\"Per-request timeout passed to the host HTTP bridge.\"",
    "},",
    "\"mailbox_id\":{",
    "\"type\":\"string\",",
    "\"description\":\"Optional mailbox ID used by list_messages.\"",
    "},",
    "\"mailbox_name\":{",
    "\"type\":\"string\",",
    "\"description\":\"Optional mailbox name resolved to a mailbox ID by list_messages.\"",
    "},",
    "\"limit\":{",
    "\"type\":\"integer\",",
    "\"default\":20,",
    "\"description\":\"Maximum number of messages to return from list_messages.\"",
    "},",
    "\"position\":{",
    "\"type\":\"integer\",",
    "\"default\":0,",
    "\"description\":\"Zero-based query offset for list_messages.\"",
    "},",
    "\"email_id\":{",
    "\"type\":\"string\",",
    "\"description\":\"JMAP email ID for get_message or mark_seen.\"",
    "}",
    "}",
    "}"
);

const DESCRIPTION: &str = concat!(
    "JMAP mail tool for listing mailboxes, listing messages, fetching one ",
    "message, and marking one message as seen. This implementation uses the ",
    "Ironclaw host HTTP bridge instead of guest-managed sockets, so credentials ",
    "should be injected by the host through the tool's HTTP capabilities."
);

/// Return the request schema as JSON.
pub(crate) const fn schema_json() -> &'static str {
    SCHEMA
}

/// Return the short tool description.
pub(crate) const fn description() -> &'static str {
    DESCRIPTION
}
