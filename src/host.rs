//! Thin wrappers over imported host functions.

use crate::bindings::near::agent::host::{self, LogLevel};

/// Log levels used inside the tool.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HostLogLevel {
    /// Informational diagnostics.
    Info,
}

/// Host operations used by request execution.
pub(crate) trait Host {
    /// Emit a structured log entry.
    fn log(&self, level: HostLogLevel, message: &str);

    /// Check whether a secret exists in the host environment.
    fn secret_exists(&self, name: &str) -> bool;

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
}

const fn map_level(level: HostLogLevel) -> LogLevel {
    match level {
        HostLogLevel::Info => LogLevel::Info,
    }
}
