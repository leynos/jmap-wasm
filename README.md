# jmap-tool

*An Ironclaw-compatible Wasm mail tool that talks JMAP over the host HTTP
bridge.*

`jmap-tool` packages a sandboxed WebAssembly component for Ironclaw hosts that
need mailbox listing, message listing, message retrieval, and `$seen` updates
without giving guest code raw socket access. The root crate handles the
Ironclaw bindings and transport orchestration, while the reusable
`crates/jmap-codec` crate holds the transport-agnostic JMAP request and
response models.

______________________________________________________________________

## Why jmap-tool?

- **Fits Ironclaw's security model**: The tool uses the host `http-request`
  capability rather than guest-managed TCP sockets.
- **Ready to package**: `make package` emits the Wasm artifact, capabilities
  sidecar, and README in one predictable directory.
- **Honest testing story**: Unit tests, behavioural tests, Wasmtime component
  checks, and a `rusmes-jmap` harness all live in the repo.
- **Reusable protocol layer**: The `jmap-codec` crate can be reused elsewhere
  without pulling in Ironclaw bindings.

______________________________________________________________________

## Quick start

### Installation

```bash
rustup target add wasm32-wasip2
make wasm
```

### Basic usage

Build the packaged tool bundle:

```bash
make package
```

That produces:

- `target/wasm32-wasip2/release/jmap_tool.wasm`
- `dist/jmap-tool/jmap-tool.wasm`
- `dist/jmap-tool/jmap-tool.capabilities.json`

The core request payload looks like this:

```json
{
  "action": "list_mailboxes",
  "base_url": "https://mail.example.com",
  "auth_secret_name": "jmap_token"
}
```

Ironclaw passes that JSON string to the tool's `execute` method via the shared
`sandboxed-tool` interface. The host injects the bearer token according to
`jmap-tool.capabilities.json`; the tool only checks that the named secret
exists before making the HTTP request.

______________________________________________________________________

## Features

- Implements Ironclaw's `sandboxed-tool` world with `execute`, `schema`, and
  `description`.
- Supports `list_mailboxes`, `list_messages`, `get_message`, and `mark_seen`.
- Packages a reusable `jmap-codec` crate for JMAP session, envelope, and mail
  method types.
- Includes `rstest`, `rstest-bdd`, `wit-component`, and `wasmtime` coverage.
- Ships an Ironclaw capabilities sidecar for HTTP allowlisting and bearer-token
  injection.

______________________________________________________________________

## Learn more

- [Users' Guide](docs/users-guide.md) — build, package, auth, request schema,
  and local test flow
- [Codec README](crates/jmap-codec/README.md) — reusable JMAP codec crate
- [ExecPlan](docs/execplans/initial-plugin.md) — implementation decisions,
  milestones, and limitations
- [Agent instructions](AGENTS.md) — repository conventions and contributor
  gates

______________________________________________________________________

## Licence

MIT OR Apache-2.0 — see [LICENSE](LICENSE) for details.

______________________________________________________________________

## Contributing

Contributions are welcome. Please read [AGENTS.md](AGENTS.md) before making
changes, and run the project gates before sending a patch:

```bash
make check-fmt
make lint
make test
```
