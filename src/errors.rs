//! Typed errors returned by the JMAP tool.

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
    /// The host HTTP bridge rejected the request.
    #[error("Host HTTP request failed: {0}")]
    HostHttp(String),
    /// The host returned headers that were not valid JSON.
    #[error("Host returned invalid response headers JSON: {0}")]
    InvalidHeadersJson(#[source] serde_json::Error),
    /// The JMAP server returned a non-successful HTTP status.
    #[error("JMAP server returned HTTP {status}: {body}")]
    UnexpectedHttpStatus {
        /// HTTP status code.
        status: u16,
        /// Response body decoded lossily for diagnostics.
        body: String,
    },
    /// The response body was not valid JSON.
    #[error("JMAP server returned invalid JSON: {0}")]
    InvalidJsonResponse(#[source] serde_json::Error),
    /// The response was syntactically valid JSON but not the shape required.
    #[error("JMAP server returned an unexpected response: {0}")]
    InvalidResponse(String),
    /// The component could not serialize the success payload.
    #[error("Failed to serialize tool output: {0}")]
    SerializeOutput(#[source] serde_json::Error),
}
