//! End-to-end component checks for the built Wasm artifact.

use std::{collections::HashSet, fs, path::PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use wasmtime::{
    Config, Engine, Store,
    component::{Component, HasSelf, Linker, ResourceTable},
};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};
use wit_component::DecodedWasm;

#[cfg_attr(not(clippy), allow(missing_docs))]
#[expect(
    clippy::integer_division_remainder_used,
    reason = "wasmtime bindgen expands generated arithmetic"
)]
mod bindings {
    wasmtime::component::bindgen!({
        path: "wit",
        world: "sandboxed-tool",
    });
}

use bindings::{SandboxedTool, exports, near};

const WASM_ARTIFACT: &str = "target/wasm32-wasip2/release/imap_tool.wasm";

#[derive(Default)]
struct TestState {
    table: ResourceTable,
    wasi: WasiCtx,
    logs: Vec<(near::agent::host::LogLevel, String)>,
    secrets: HashSet<String>,
}

impl WasiView for TestState {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

impl near::agent::host::Host for TestState {
    fn log(&mut self, level: near::agent::host::LogLevel, message: String) {
        self.logs.push((level, message));
    }

    fn now_millis(&mut self) -> u64 {
        0
    }

    fn workspace_read(&mut self, _path: String) -> Option<String> {
        None
    }

    fn http_request(
        &mut self,
        _method: String,
        _url: String,
        _headers_json: String,
        _body: Option<Vec<u8>>,
        _timeout_ms: Option<u32>,
    ) -> std::result::Result<near::agent::host::HttpResponse, String> {
        Err("http-request is not available in this e2e host".to_owned())
    }

    fn tool_invoke(
        &mut self,
        _alias: String,
        _params_json: String,
    ) -> std::result::Result<String, String> {
        Err("tool-invoke is not available in this e2e host".to_owned())
    }

    fn secret_exists(&mut self, name: String) -> bool {
        self.secrets.contains(&name)
    }
}

#[test]
#[ignore = "requires a built Wasm artifact"]
fn wasm_component_exports_schema_and_description() {
    run_component_check().expect("component e2e should pass");
}

fn run_component_check() -> Result<()> {
    let artifact = PathBuf::from(WASM_ARTIFACT);
    let artifact_bytes = fs::read(&artifact).with_context(|| {
        format!(
            "failed to read Wasm artifact at '{}' - run `make wasm` first",
            artifact.display()
        )
    })?;
    if !matches!(
        wit_component::decode(&artifact_bytes)?,
        DecodedWasm::Component(_, _)
    ) {
        bail!("'{}' is not a Wasm component", artifact.display());
    }

    let mut config = Config::new();
    config.wasm_component_model(true);

    let engine = Engine::new(&config)?;
    let component = Component::from_file(&engine, &artifact)?;
    let mut linker = Linker::new(&engine);
    wasmtime_wasi::p2::add_to_linker_sync(&mut linker)?;
    SandboxedTool::add_to_linker::<_, HasSelf<_>>(&mut linker, |state| state)?;

    let mut store = Store::new(
        &engine,
        TestState {
            table: ResourceTable::new(),
            wasi: WasiCtxBuilder::new().inherit_network().build(),
            logs: Vec::new(),
            secrets: HashSet::from(["imap_password".to_owned()]),
        },
    );
    let bindings = SandboxedTool::instantiate(&mut store, &component, &linker)?;
    let tool = bindings.near_agent_tool();

    let schema = tool.call_schema(&mut store)?;
    if !schema.contains("\"list_mailboxes\"") {
        return Err(anyhow!("schema did not mention list_mailboxes"));
    }

    let description = tool.call_description(&mut store)?;
    if !description.contains("imap-next") {
        return Err(anyhow!("description did not mention imap-next"));
    }

    let response = tool.call_execute(
        &mut store,
        &exports::near::agent::tool::Request {
            params: "{".to_owned(),
            context: None,
        },
    )?;
    if response.error.is_none() {
        return Err(anyhow!(
            "invalid JSON request should have produced an error response"
        ));
    }

    Ok(())
}
