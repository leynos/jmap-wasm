//! JSON schema and high-level description for the tool.

const SCHEMA: &str = concat!(
    "{",
    "\"type\":\"object\",",
    "\"required\":[\"action\",\"host\",\"username\",\"password\"],",
    "\"properties\":{",
    "\"action\":{",
    "\"type\":\"string\",",
    "\"enum\":[\"list_mailboxes\",\"list_messages\",\"get_message\",\"mark_seen\"],",
    "\"description\":\"The IMAP operation to perform.\"",
    "},",
    "\"host\":{",
    "\"type\":\"string\",",
    "\"description\":\"Hostname or IP address of the IMAP server.\"",
    "},",
    "\"port\":{",
    "\"type\":\"integer\",",
    "\"default\":143,",
    "\"description\":\"Plain IMAP TCP port. This tool currently supports non-TLS IMAP only.\"",
    "},",
    "\"username\":{",
    "\"type\":\"string\",",
    "\"description\":\"IMAP username used for LOGIN authentication.\"",
    "},",
    "\"password\":{",
    "\"type\":\"string\",",
    "\"description\":\"IMAP password used for LOGIN authentication. Ironclaw cannot inject non-HTTP secrets into this socket workflow yet, so the password must be supplied in the request.\"",
    "},",
    "\"password_secret_name\":{",
    "\"type\":\"string\",",
    "\"description\":\"Optional host secret name to preflight with secret_exists before attempting the login. The tool checks only presence, not secret contents.\"",
    "},",
    "\"mailbox\":{",
    "\"type\":\"string\",",
    "\"default\":\"INBOX\",",
    "\"description\":\"Mailbox name for message-scoped actions.\"",
    "},",
    "\"sequence_set\":{",
    "\"type\":\"string\",",
    "\"default\":\"1:*\",",
    "\"description\":\"IMAP sequence set for list_messages.\"",
    "},",
    "\"sequence\":{",
    "\"type\":\"integer\",",
    "\"description\":\"One message sequence number for get_message or mark_seen.\"",
    "}",
    "}",
    "}"
);

const DESCRIPTION: &str = concat!(
    "IMAP tool for listing mailboxes, listing messages, fetching one message, ",
    "and marking one message as seen. This implementation uses imap-next's ",
    "sans-I/O client over plain TCP and currently supports non-TLS IMAP on port 143."
);

/// Return the request schema as JSON.
pub(crate) const fn schema_json() -> &'static str {
    SCHEMA
}

/// Return the short tool description.
pub(crate) const fn description() -> &'static str {
    DESCRIPTION
}
