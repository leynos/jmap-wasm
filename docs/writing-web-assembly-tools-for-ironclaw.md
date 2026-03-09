# Writing Rust WebAssembly Tools for Ironclaw

## The Guide I Wish I Had Before Building `jmap-tool`

This document is based on actually building and packaging
[`jmap-tool`](../README.md) for Ironclaw, not on an idealized reading of the
WIT alone. The short version is:

- Ironclaw Wasm tools are a good fit for HTTP-shaped integrations.
- They are a poor fit for guest-managed TCP protocols.
- `schema()` and `description()` matter more than you may think.
- Packaging is stricter than "ship a `.wasm` somehow".
- Honest end-to-end testing needs more than one layer.

If you are writing a new tool, this guide should save you from several rounds
of avoidable confusion.

## Read This First

Three facts shape almost every design decision:

1. The public tool boundary is the `sandboxed-tool` world in
   [`wit/tool.wit`](../wit/tool.wit).
2. The host capability surface is small. In practice, the important outbound
   capability is `http-request`.
3. The Ironclaw web UI expects a `.tar.gz` bundle, not a loose directory.

That means the happy path for plugin authors is:

- choose a protocol that already works over HTTP(S)
- keep secrets at the host boundary
- return JSON strings from `execute`
- package the result as a named Wasm bundle

That also means some tempting designs do not fit:

- IMAP over raw TCP from inside the Wasm guest
- SMTP from inside the Wasm guest
- any crate that insists on owning its own `reqwest::Client` when you need to
  route requests through Ironclaw's host HTTP bridge

## What The Tool Boundary Actually Is

Ironclaw tools export three things and import a handful of host functions. The
authoritative contract in this repository is [`wit/tool.wit`](../wit/tool.wit).

At the tool boundary, you export:

- `execute(req: request) -> response`
- `schema() -> string`
- `description() -> string`

At the host boundary, you import:

- `log`
- `now-millis`
- `workspace-read`
- `http-request`
- `tool-invoke`
- `secret-exists`

The crucial nuance is that the request and response payloads are string-based:

- `request.params` is a JSON string
- `request.context` is an optional JSON string
- `response.output` is an optional JSON string
- `response.error` is an optional plain string

If you come in expecting a richly typed cross-component API, you will write the
wrong abstractions first and then unwind them later.

### `params` Is A JSON String, Not A Record

This is the first sharp edge that catches people. Your inner request shape is
your own JSON object, but Ironclaw hands it to you as `request.params: string`.

In `jmap-tool`, the guest entry point is tiny:

```rust
wit_bindgen::generate!({
    path: "wit",
    world: "sandboxed-tool",
});

impl Guest for JmapTool {
    fn execute(req: Request) -> Response {
        match execute_with(&req.params, &host, &service) {
            Ok(output) => Response {
                output: Some(output),
                error: None,
            },
            Err(error) => Response {
                output: None,
                error: Some(error.to_string()),
            },
        }
    }
}
```

That shape is boring, and that is good. Keep it boring.

### `schema()` Returns A JSON Schema String

`schema()` does not return a WIT record or a host-managed descriptor. It
returns a string containing JSON Schema.

In [`src/schema.rs`](../src/schema.rs), `jmap-tool` returns a static JSON
Schema string describing fields such as:

- `action`
- `base_url`
- `account_id`
- `auth_secret_name`
- `timeout_ms`
- `mailbox_id`
- `mailbox_name`
- `limit`
- `position`
- `email_id`

Do not underspecify this. The schema is the closest thing Ironclaw tools have
to a discoverable method contract.

### There Is No Dedicated "List Supported Methods" API

This is another missing detail that matters in practice.

There is no separate outward function like `list_methods()` or `capabilities()`
for business actions. Discovery happens through:

- `description()`
- `schema()`

If you want the host, agent, or an LLM to understand what your tool can do,
your schema needs a crisp `action` enum and your description needs to say when
the tool should be used.

For `jmap-tool`, the supported actions are surfaced through the schema:

- `list_mailboxes`
- `list_messages`
- `get_message`
- `mark_seen`

That pattern is consistent with the neighbouring Ironclaw tools in
`../ironclaw/tools-src/`.

## Capability Reality Beats ABI Theory

The WIT is the contract surface, but the practical capability model is tighter
than "whatever the component model can express".

### HTTP Is The Real Outbound Network Primitive

The host API exposes `http-request`, and that is the mechanism you should plan
around for external I/O.

This is why JMAP was a good fit and IMAP was not.

JMAP already speaks HTTP(S), so `jmap-tool` can:

1. fetch `/.well-known/jmap`
2. resolve the `apiUrl`
3. `POST` JMAP method calls over the host bridge

That entire flow lives comfortably inside
[`src/jmap_transport.rs`](../src/jmap_transport.rs).

IMAP required guest-managed TCP, which did not fit the tool model and did not
work under the runtime we had available.

### "Defined In WIT" Does Not Guarantee "Works In The Host"

`tool-invoke` is present in the world, but that does not mean you should depend
on it today.

In the current Ironclaw host implementation, the Wasm wrapper still returns:

`Tool invocation from WASM tools is not yet supported`

That matters because it changes how you structure composition:

- do not design your plugin around calling sibling tools
- do not hide essential functionality behind a second tool
- keep one Wasm tool self-contained unless you have verified host support

If you need composition, push it up into the agent or into a host-side service.

### Secrets Are Host-Injected, Not Guest-Readable

This is the right security model, but the guide should say it more bluntly:

- the guest can check whether a secret exists
- the guest cannot read the secret value
- credentials should be injected by the host into HTTP requests

In `jmap-tool`, `auth_secret_name` is only a preflight check. The tool calls
`secret_exists()` so it can fail fast with a useful error, but the bearer token
still arrives via host capability configuration in
[`jmap-tool.capabilities.json`](../jmap-tool.capabilities.json).

If you find yourself wanting to pass passwords, tokens, or API keys directly in
tool parameters, you are fighting the platform.

## Pick Libraries That Respect The Host Transport Boundary

This was the most important implementation lesson.

### Transport-Agnostic Codecs Fit Better Than Full Clients

The reusable crate in this repository,
[`crates/jmap-codec`](../crates/jmap-codec/README.md), exists because the host
transport boundary matters.

The codec crate owns:

- JMAP session types
- request and response envelopes
- method argument and response types
- serialization and deserialization support

The Wasm tool crate owns:

- parsing tool parameters
- calling the host HTTP bridge
- mapping protocol types into tool outputs
- logging and error shaping

That split is reusable and publishable. It is also easier to test honestly.

### Avoid Client Crates That Hide HTTP Internals

The `jmap-client` investigation ended with a no.

The problem was not "this crate is bad". The problem was that the tool needed
to route HTTP through Ironclaw's `host.http-request`, while `jmap-client`
constructs and owns its own HTTP client flow. That is the wrong direction of
control for an Ironclaw Wasm tool.

A good rule for Ironclaw plugin authors is:

- codec crates are usually good candidates
- transport-owning client crates are usually a bad fit

If a crate does not let you inject the HTTP transport in a narrow and
non-invasive way, assume it is the wrong building block for a Wasm plugin.

## Recommended Repository Shape

The neighbouring Ironclaw tools are mostly single-crate examples. That shape is
fine for simple tools, but `jmap-tool` needed a slightly richer layout:

```text
.
├── Cargo.toml
├── Makefile
├── jmap-tool.capabilities.json
├── wit/
│   └── tool.wit
├── src/
│   ├── lib.rs
│   ├── actions.rs
│   ├── errors.rs
│   ├── host.rs
│   ├── jmap_transport.rs
│   ├── mappers.rs
│   ├── outputs.rs
│   ├── schema.rs
│   └── service.rs
├── crates/
│   └── jmap-codec/
└── tests/
    └── jmap_e2e.rs
```

Two details are worth copying:

- keep `src/lib.rs` thin
- move transport-agnostic protocol code into its own crate early

That keeps the Wasm-facing code small and the protocol surface reusable.

### Keep A Local Copy Of `tool.wit`

The bundled Ironclaw examples import `../../wit/tool.wit` from the main
Ironclaw repository. This repository keeps its own copy in
[`wit/tool.wit`](../wit/tool.wit).

That trade-off is worth it if you want:

- reproducible local builds
- a clear pinned interface version
- fewer cross-repo assumptions during packaging and testing

The downside is drift. If Ironclaw changes the world, you must reconcile your
copy deliberately.

## Schema Design Advice From The JMAP Tool

### Prefer One Tagged `action` Object

The cleanest outward contract we found was a single request object with a
required `action` field.

That let us keep `execute()` stable while evolving specific operations behind
that tag.

For `jmap-tool`, the shared request fields are:

- `action`
- `base_url`
- `account_id`
- `auth_secret_name`
- `timeout_ms`

Action-specific fields then layer on top.

This buys you:

- easier schema discovery
- one stable entry point
- a simpler LLM prompt surface

### Defaults Matter

If your schema exposes optional pagination, timeouts, or limits, define and
document the defaults in one place. `jmap-tool` does this in the schema and in
the users' guide.

Otherwise, your host, tests, docs, and actual runtime will drift apart.

### Keep `description()` Practical

`description()` is not decoration. It is part of the invocation surface.

`jmap-tool` explicitly says that it:

- lists mailboxes
- lists messages
- fetches one message
- marks one message as seen
- uses the host HTTP bridge instead of guest-managed sockets

That last clause matters. It explains both the security model and the expected
deployment model in one sentence.

## Capabilities Sidecars Are Part Of The Product

Treat the capabilities JSON as first-class shipping artefact, not auxiliary
metadata.

In [`jmap-tool.capabilities.json`](../jmap-tool.capabilities.json), the tool
declares:

- HTTP allowlist entries
- a bearer-token credential mapping
- secret-name checks
- rate limits
- timeout defaults
- auth metadata for the host UI

### Hostnames Must Be Real, Not Aspirational

The checked-in sidecar uses `mail.example.com` as an example. That is useful
for the repository, but a real deployment must replace it with the actual JMAP
provider hostname.

This sounds obvious, but it is an easy way to ship a plugin that installs clean
and fails at runtime.

### Secret Checks And Credential Injection Are Separate Concerns

This split caused confusion during implementation and is worth spelling out.

The tool parameter `auth_secret_name` is a runtime hint to the guest:

- "check that this secret exists before you start"

The sidecar credential mapping is a host instruction:

- "inject this secret into outbound HTTP requests for matching hosts"

Those are complementary. Neither replaces the other.

## Packaging Is Stricter Than The Examples Suggest

This was one of the bigger surprises.

### The Web UI Wants A `.tar.gz` Bundle

The initial packaging target in this repository produced a directory with the
Wasm file and sidecar. That was not enough for the Ironclaw web UI.

The practical installer contract is:

- build a `.tar.gz`
- include `{name}.wasm`
- include `{name}.capabilities.json`
- keep the names aligned with the extension name Ironclaw will install

`jmap-tool` now packages:

- `dist/jmap-tool/jmap-tool.wasm`
- `dist/jmap-tool/jmap-tool.capabilities.json`
- `dist/jmap-tool/README.md`
- `dist/jmap-tool-wasm32-wasip2.tar.gz`

The bundle is produced by [`Makefile`](../Makefile):

```make
package: wasm
	rm -rf $(DIST_DIR)
	mkdir -p $(DIST_DIR)
	cp $(WASM_ARTIFACT) $(DIST_DIR)/jmap-tool.wasm
	cp jmap-tool.capabilities.json $(DIST_DIR)/
	cp docs/users-guide.md $(DIST_DIR)/README.md
	rm -f $(BUNDLE_ARTIFACT)
	tar -C $(DIST_DIR) -czf $(BUNDLE_ARTIFACT) \
		jmap-tool.wasm \
		jmap-tool.capabilities.json \
		README.md
```

### Bundle Naming And Install Naming Must Match

The Ironclaw custom installer extracts files by expected basename. In practice,
that means the extension name entered by the user must match the shipped file
names.

For this tool, that means:

- install it as `jmap-tool`
- ship `jmap-tool.wasm`
- ship `jmap-tool.capabilities.json`

Do not assume Ironclaw will infer or rewrite those names for you.

### A Directory Is Useful, But The Tarball Is The Deliverable

Keeping `dist/jmap-tool/` is still useful for inspection and local debugging,
but the thing users actually need for the web UI is the tarball.

If your `make package` target does not leave behind a directly installable
archive, it is incomplete.

## Testing: Prove The Right Thing At The Right Layer

This is where the old guide most needed scars.

### One "E2E Test" Is Not Enough

For a Wasm plugin like this, there are at least two materially different things
to prove:

1. the built Wasm artefact is a valid component that instantiates and exercises
   the exported world correctly
2. the protocol and service logic actually work against something that behaves
   like the real external system

Those are not the same test.

### Component E2E: Validate The Wasm Artefact

[`tests/jmap_e2e.rs`](../tests/jmap_e2e.rs) covers the Wasm artefact itself.

That test:

- reads `target/wasm32-wasip2/release/jmap_tool.wasm`
- verifies it is a component with `wit-component`
- instantiates it with Wasmtime
- stubs the host imports
- checks `schema()`
- checks `description()`
- executes a read-only action

This catches packaging and ABI mistakes that unit tests will never see.

### Native Protocol E2E: Prove The Mail Flow Honestly

[`src/e2e_tests.rs`](../src/e2e_tests.rs) proves the JMAP service logic against
an in-process `rusmes-jmap` server.

That test exercises:

- mailbox listing
- message retrieval by seeded ID
- the current `Email/set` limitation

It does not pretend to prove more than it proves.

That matters because the mock server has real gaps.

### Mock Servers Have Opinions And Bugs

The `rusmes-jmap` harness was useful, but not frictionless.

The hard edges we hit were:

- the session flow needed host-side help in the test harness
- `Email/set` currently returns `notImplemented`
- a stronger "list messages exactly like production" assertion was less stable
  than the rest of the stack

That is why `make e2e` in this repository runs ignored tests with a split
strategy rather than one oversized integration test.

If your mock server cannot honestly support the mutation or query behaviour you
need, document that explicitly instead of claiming stronger coverage than you
actually have.

### Behavioural Tests Still Pull Their Weight

The repo also uses:

- `rstest` for unit tests
- `rstest-bdd` for behavioural tests

That combination was valuable because it let us keep fast confidence close to
the action parser and response mapping while reserving slower e2e coverage for
the transport boundary.

## Build Targets That Future You Will Thank You For

The current `Makefile` ended up with a target set that feels about right:

- `make check-fmt`
- `make lint`
- `make test`
- `make wasm`
- `make package`
- `make e2e`

That split matters.

`make all` should stay fast enough for normal development. Wasm packaging and
ignored e2e tests should be opt-in, deterministic steps, not hidden side
effects of some giant default target.

## What I Would Tell A New Plugin Author To Do

If you are starting from zero, do this in order:

1. Verify that your external protocol can be expressed through
   `host.http-request`.
2. Copy or vendor the current `tool.wit` locally so you have a pinned contract.
3. Keep `src/lib.rs` tiny and put real logic in normal modules.
4. Design one request object with a required `action`.
5. Write `schema()` and `description()` before you think they are "finished".
6. Treat the capabilities sidecar as part of the shipped plugin.
7. Add `make wasm`, `make package`, and `make e2e` early.
8. Validate the built `.wasm` as a component, not just as a Rust build output.
9. Package a `.tar.gz` that Ironclaw can install directly.
10. Document every runtime caveat that your mock server or host model imposes.

## Hard Edges Future Authors Should Know Up Front

- The Wasm guest does not get raw socket freedom just because the target is
  `wasm32-wasip2`.
- `schema()` returns a string, so schema drift is easy if you do not keep docs
  and tests near it.
- `tool-invoke` exists in WIT but is not yet a reliable design primitive in the
  current host.
- Packaging is name-sensitive. The installer expects matching basenames.
- A valid Wasm component test is not the same thing as a useful protocol test.
- Mock servers are product dependencies. Treat their gaps as part of your
  design surface.

## Final Recommendation

Ironclaw Wasm tools are at their best when they are:

- narrow in purpose
- HTTP-native
- honest about their capability boundary
- explicit about packaging
- tested at both the component and protocol layers

If you stay inside that shape, the platform is pleasant enough.

If you try to smuggle a raw-socket client architecture through the guest, you
will spend your time debugging the wrong problem.
