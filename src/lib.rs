//! Ironclaw-compatible JMAP tool implemented as a WebAssembly component.

mod actions;
mod errors;
mod host;
mod jmap_transport;
mod mappers;
mod outputs;
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
use actions::JmapAction;
use bindings::{export, exports};
use errors::ToolError;
use host::{Host, WasmHost};
use service::{JmapService, NetworkJmapService};

struct JmapTool;

impl Guest for JmapTool {
    fn execute(req: exports::near::agent::tool::Request) -> exports::near::agent::tool::Response {
        let host = WasmHost;
        let service = NetworkJmapService;

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

fn execute_with<H: Host, S: JmapService>(
    params: &str,
    host: &H,
    service: &S,
) -> Result<String, ToolError> {
    let action = JmapAction::parse(params)?;
    action.verify_secret(host)?;
    host.log_info(&format!("Executing JMAP action '{}'", action.action_name()));
    let output = action.execute(host, service)?;
    serde_json::to_string(&output).map_err(ToolError::SerializeOutput)
}

export!(JmapTool);

#[cfg(test)]
mod e2e_tests;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_bdd;
