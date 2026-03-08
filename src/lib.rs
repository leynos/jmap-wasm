//! Ironclaw-compatible IMAP tool implemented as a WebAssembly component.

mod actions;
mod errors;
mod host;
mod outputs;
mod protocol;
mod schema;
mod service;

#[cfg_attr(not(clippy), allow(missing_docs))]
mod bindings {
    wit_bindgen::generate!({
        path: "wit",
        world: "sandboxed-tool",
    });
}

use crate::bindings::exports::near::agent::tool::Guest;
use actions::ImapAction;
use bindings::{export, exports};
use errors::ToolError;
use host::{Host, WasmHost};
use service::{ImapService, NetworkImapService};

struct ImapTool;

impl Guest for ImapTool {
    fn execute(req: exports::near::agent::tool::Request) -> exports::near::agent::tool::Response {
        let host = WasmHost;
        let service = NetworkImapService;

        match execute_with(&req.params, &host, &service) {
            Ok(output) => exports::near::agent::tool::Response {
                output: Some(output),
                error: None,
            },
            Err(error) => exports::near::agent::tool::Response {
                output: None,
                error: Some(error.to_string()),
            },
        }
    }

    fn schema() -> String {
        schema::schema_json().to_owned()
    }

    fn description() -> String {
        schema::description().to_owned()
    }
}

fn execute_with<H: Host, S: ImapService>(
    params: &str,
    host: &H,
    service: &S,
) -> Result<String, ToolError> {
    let action = ImapAction::parse(params)?;
    action.verify_secret(host)?;
    host.log_info(&format!("Executing IMAP action '{}'", action.action_name()));
    let output = action.execute(service)?;
    serde_json::to_string(&output).map_err(ToolError::SerializeOutput)
}

export!(ImapTool);

#[cfg(test)]
mod e2e_tests;
#[cfg(test)]
mod tests;
