//! Typed errors returned by the IMAP tool.

use std::io;

use thiserror::Error;

/// Errors that can be surfaced by the tool.
#[derive(Debug, Error)]
pub(crate) enum ToolError {
    /// The request parameters were not valid JSON.
    #[error("Invalid parameters: {0}")]
    InvalidParameters(#[source] serde_json::Error),
    /// The request parameters were structurally valid but semantically wrong.
    #[error("{0}")]
    InvalidRequest(String),
    /// A named secret was requested for existence checks but was not present.
    #[error("Required secret '{0}' is not configured")]
    MissingSecret(String),
    /// The IMAP server closed the connection unexpectedly.
    #[error("IMAP server closed the connection")]
    ConnectionClosed,
    /// Plain TCP I/O failed.
    #[error("I/O failure: {0}")]
    Io(#[source] io::Error),
    /// The IMAP protocol parser rejected the traffic.
    #[error("IMAP protocol error: {0}")]
    Protocol(String),
    /// The IMAP server rejected a command.
    #[error("IMAP server rejected the command: {0}")]
    Server(String),
    /// The component could not serialize the success payload.
    #[error("Failed to serialize tool output: {0}")]
    SerializeOutput(#[source] serde_json::Error),
}
