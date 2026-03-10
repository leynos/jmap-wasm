//! Thin wrappers over imported host functions.

use serde_json::{Map, Value};

use crate::{
    bindings::near::agent::host::{self, LogLevel},
    errors::ToolError,
};

/// Log levels used inside the tool.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HostLogLevel {
    /// Informational diagnostics.
    Info,
}

/// One HTTP response returned by the Ironclaw host bridge.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct HostHttpResponse {
    /// HTTP status code.
    pub(crate) status: u16,
    /// Response headers decoded from the host JSON payload.
    pub(crate) headers: Map<String, Value>,
    /// Raw response body bytes.
    pub(crate) body: Vec<u8>,
}

/// One HTTP request sent through the Ironclaw host bridge.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct HostHttpRequest {
    /// HTTP method.
    pub(crate) method: String,
    /// Absolute URL.
    pub(crate) url: String,
    /// Headers encoded as a JSON object string.
    pub(crate) headers_json: String,
    /// Optional raw body bytes.
    pub(crate) body: Option<Vec<u8>>,
    /// Optional timeout in milliseconds.
    pub(crate) timeout_ms: Option<u32>,
}

/// Host operations used by request execution.
pub(crate) trait Host {
    /// Emit a structured log entry.
    fn log(&self, level: HostLogLevel, message: &str);

    /// Check whether a secret exists in the host environment.
    fn secret_exists(&self, name: &str) -> bool;

    /// Make one HTTP request through the host bridge.
    fn http_request(&self, request: &HostHttpRequest) -> Result<HostHttpResponse, ToolError>;

    /// Emit an informational message.
    fn log_info(&self, message: &str) {
        self.log(HostLogLevel::Info, message);
    }
}

/// Production host implementation backed by the imported Ironclaw interface.
pub(crate) struct WasmHost;

impl Host for WasmHost {
    fn log(&self, level: HostLogLevel, message: &str) {
        host::log(map_level(level), message);
    }

    fn secret_exists(&self, name: &str) -> bool {
        host::secret_exists(name)
    }

    fn http_request(&self, request: &HostHttpRequest) -> Result<HostHttpResponse, ToolError> {
        let response = host::http_request(
            &request.method,
            &request.url,
            &request.headers_json,
            request.body.as_deref(),
            request.timeout_ms,
        )
        .map_err(ToolError::HostHttp)?;
        let headers = serde_json::from_str::<Map<String, Value>>(&response.headers_json)
            .map_err(ToolError::InvalidHeadersJson)?;

        Ok(HostHttpResponse {
            status: response.status,
            headers,
            body: response.body,
        })
    }
}

const fn map_level(level: HostLogLevel) -> LogLevel {
    match level {
        HostLogLevel::Info => LogLevel::Info,
    }
}
