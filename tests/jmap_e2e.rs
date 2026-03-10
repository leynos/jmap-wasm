//! End-to-end component checks for the built Wasm artifact.

use std::{collections::HashSet, io::Read as _};

use anyhow::{Context, Result, anyhow, bail};
use camino::Utf8PathBuf;
use cap_std::{ambient_authority, fs::Dir};
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
    //! Generated host bindings used by the Wasmtime integration test.

    wasmtime::component::bindgen!({
        path: "wit",
        world: "sandboxed-tool",
    });
}

use bindings::{SandboxedTool, exports, near};

const WASM_ARTIFACT: &str = "target/wasm32-wasip2/release/jmap_tool.wasm";

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
        method: String,
        url: String,
        _headers_json: String,
        _body: Option<Vec<u8>>,
        _timeout_ms: Option<u32>,
    ) -> std::result::Result<near::agent::host::HttpResponse, String> {
        if method == "GET" && url.ends_with("/.well-known/jmap") {
            return Ok(json_response(
                200,
                r#"{
                    "capabilities": {
                        "urn:ietf:params:jmap:core": {},
                        "urn:ietf:params:jmap:mail": {}
                    },
                    "accounts": {
                        "acc-1": {}
                    },
                    "primaryAccounts": {
                        "urn:ietf:params:jmap:mail": "acc-1"
                    },
                    "username": "user@example.com",
                    "apiUrl": "https://mail.example.com/jmap",
                    "downloadUrl": "https://mail.example.com/download/{accountId}/{blobId}/{name}",
                    "uploadUrl": "https://mail.example.com/upload/{accountId}",
                    "state": "s-1"
                }"#,
            ));
        }

        if method == "POST" && url == "https://mail.example.com/jmap" {
            return Ok(json_response(
                200,
                r#"{
                    "methodResponses": [[
                        "Mailbox/get",
                        {
                            "accountId": "acc-1",
                            "state": "mbx-1",
                            "list": [{
                                "id": "mbx-1",
                                "name": "Inbox",
                                "role": "inbox",
                                "isSubscribed": true,
                                "totalEmails": 1,
                                "unreadEmails": 1
                            }],
                            "notFound": []
                        },
                        "call-0"
                    ]],
                    "sessionState": "s-1"
                }"#,
            ));
        }

        Err(format!("unexpected request: {method} {url}"))
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
fn wasm_component_exports_schema_description_and_executes_jmap_read_path() {
    run_component_check().expect("component e2e should pass");
}

fn run_component_check() -> Result<()> {
    let workspace_dir = Dir::open_ambient_dir(".", ambient_authority())
        .context("failed to open workspace directory for Wasm artifact lookup")?;
    let artifact = Utf8PathBuf::from(WASM_ARTIFACT);
    let artifact_bytes = read_bytes(&workspace_dir, &artifact).with_context(|| {
        format!("failed to read Wasm artifact at '{artifact}' - run `make wasm` first")
    })?;
    if !matches!(
        wit_component::decode(&artifact_bytes)?,
        DecodedWasm::Component(_, _)
    ) {
        bail!("'{artifact}' is not a Wasm component");
    }

    let mut config = Config::new();
    config.wasm_component_model(true);

    let engine = Engine::new(&config)?;
    let component = Component::from_file(&engine, artifact.as_std_path())?;
    let mut linker = Linker::new(&engine);
    wasmtime_wasi::p2::add_to_linker_sync(&mut linker)?;
    SandboxedTool::add_to_linker::<_, HasSelf<_>>(&mut linker, |state| state)?;

    let mut store = Store::new(
        &engine,
        TestState {
            table: ResourceTable::new(),
            wasi: WasiCtxBuilder::new().build(),
            logs: Vec::new(),
            secrets: HashSet::from(["jmap_token".to_owned()]),
        },
    );
    let bindings = SandboxedTool::instantiate(&mut store, &component, &linker)?;
    let tool = bindings.near_agent_tool();

    let schema = tool.call_schema(&mut store)?;
    if !schema.contains("\"base_url\"") {
        return Err(anyhow!("schema did not mention base_url"));
    }

    let description = tool.call_description(&mut store)?;
    if !description.contains("host HTTP bridge") {
        return Err(anyhow!("description did not mention the host HTTP bridge"));
    }

    let response = tool.call_execute(
        &mut store,
        &exports::near::agent::tool::Request {
            params: r#"{
                "action":"list_mailboxes",
                "base_url":"https://mail.example.com",
                "auth_secret_name":"jmap_token"
            }"#
            .to_owned(),
            context: None,
        },
    )?;
    let output = response
        .output
        .ok_or_else(|| anyhow!("JMAP list_mailboxes should have produced output"))?;
    if !output.contains("\"Inbox\"") {
        return Err(anyhow!("successful output did not contain Inbox"));
    }

    Ok(())
}

fn read_bytes(dir: &Dir, path: &Utf8PathBuf) -> Result<Vec<u8>> {
    let mut file = dir
        .open(path.as_std_path())
        .with_context(|| format!("failed to open '{path}'"))?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .with_context(|| format!("failed to read '{path}'"))?;
    Ok(bytes)
}

fn json_response(status: u16, body: &str) -> near::agent::host::HttpResponse {
    near::agent::host::HttpResponse {
        status,
        headers_json: "{\"content-type\":\"application/json\"}".to_owned(),
        body: body.as_bytes().to_vec(),
    }
}
