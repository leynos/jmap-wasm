# Pivot The Ironclaw Mail Wasm Tool From IMAP To JMAP

This ExecPlan (execution plan) is a living document. The sections
`Constraints`, `Tolerances`, `Risks`, `Progress`, `Surprises & Discoveries`,
`Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work
proceeds.

Status: IN PROGRESS

## Purpose / big picture

After this change, this repository will again build an Ironclaw-compatible Wasm
tool, but the transport will be JMAP over Ironclaw's HTTP host capability
instead of IMAP over guest TCP. A user will be able to point the tool at a JMAP
base URL, rely on host-injected HTTP credentials, list mailboxes, list
messages, fetch one message, and mark a message as seen, all through the WIT
`sandboxed-tool` interface.

This pivot is required because the previously delivered IMAP implementation
does not fit Ironclaw's actual runtime model. The Wasm guest can only reach the
outside world through the host `http-request` function, while guest-side TCP is
not available in the current runtime used here. Investigation in this turn also
showed that the `jmap-client` crate is not transport-agnostic: it constructs
its own `reqwest::Client` internally for session discovery and API calls, so it
cannot be wired to Ironclaw's host HTTP bridge without a fork or invasive
patching.

The neighbouring Ironclaw examples under
`../ironclaw/tools-src/{github,gmail,google-calendar,google-docs}` still define
the correct outer shape: a single-crate `cdylib` Wasm component using
`wit-bindgen`, a capabilities sidecar JSON file, and a library entry point that
implements `exports::near::agent::tool::Guest`. This revised plan keeps that
shape while replacing the internal protocol implementation with a small custom
JMAP client built on `host.http-request`.

## Repository orientation

The implementation will touch seven areas.

The repository root will likely become a small Cargo workspace. One member will
remain the Wasm tool crate, and a new reusable crate will hold the transport-
agnostic JMAP protocol types and codec logic. The IMAP-specific dependency and
protocol adapter must be removed. The tool `src/lib.rs` should stay small and
delegate to focused modules such as request parsing, schema rendering, host
HTTP bindings, and test-only fixtures. Keep every Rust file under 400 lines by
splitting action-specific logic early.

`wit/` will be added to hold a local copy of Ironclaw’s `tool.wit`, pinned to
the host interface expected by the neighbouring example tools.

The repository root `Makefile` will keep the existing quality gates and add
targets for workspace compilation, packaging, and end to end execution.

The new reusable crate, tentatively `crates/jmap-codec/`, must contain only
transport-agnostic JMAP protocol code. It should be publishable on its own and
must not depend on Ironclaw host bindings, Wasm-specific imports, Docker, or
the e2e mock-server stack.

`tests/` will hold `rstest-bdd` feature files, scenario bindings, and the
JMAP-backed end to end harness.

`docs/users-guide.md` must be revised into the user-facing usage document for
the JMAP tool, covering installation, required capabilities, authentication,
common actions, and local test workflows.

## Constraints

- Preserve the plan-only boundary. Draft the plan now; do not start
  implementation until the user explicitly approves this plan.
- The final tool must implement Ironclaw’s WIT `sandboxed-tool` world and use
  `wit-bindgen` for the guest bindings, matching the structure of the adjacent
  Ironclaw tools.
- The new implementation must remove the direct IMAP protocol path and replace
  it with JMAP over Ironclaw's host `http-request` capability.
- The JMAP protocol implementation must live in a separate reusable crate so it
  can be published and reused elsewhere as a transport-agnostic JMAP codec,
  analogous in spirit to `imap-codec` and `imap-types`.
- The reusable JMAP crate must not depend on Ironclaw host bindings or a
  concrete HTTP client. It may define request and response types, builders,
  serialization helpers, and validation logic, but transport must stay outside
  the crate boundary.
- The `jmap-client` crate may only be adopted if it can be used through the
  host HTTP bridge without guest-managed sockets or an invasive local fork. The
  current investigation indicates that it cannot.
- The repository must expose `make check-fmt`, `make lint`, and `make test` as
  first-class gates, and any new build or e2e targets must compose with that
  workflow instead of bypassing it.
- Unit tests must use `rstest`; behavioural tests must use `rstest-bdd`; the
  end to end path must keep using `wit-component` and must use `rusmes-jmap`
  for JMAP server mocking.
- `docs/users-guide.md` must explain usage clearly enough for a new user to set
  secrets, build the component, package the artifact, and run the local tests.
- Capabilities and secret handling must follow Ironclaw’s model: secrets are
  checked with `secret_exists`, while host-side capability JSON declares the
  allowed secret names and the JMAP HTTP endpoints needed by the tool.
- Respect repository and workspace style rules: en-GB-oxendict prose, module
  level Rust docs, no suppressed lints without narrow justification, and no
  file over 400 lines.

## Tolerances (exception triggers)

- Scope: a two-crate shape is now required: the Wasm tool crate plus one
  reusable transport-agnostic JMAP crate. If implementation requires more than
  that or a materially more complex workspace layout, stop and ask for approval.
- Interface: if Ironclaw’s local WIT interface differs materially from the copy
  described in `writing-web-assembly-tools-for-ironclaw.md`, stop and reconcile
  the mismatch before writing code.
- Dependencies: if `wit-bindgen`, `wit-component`, `rstest`, `rstest-bdd`,
  `thiserror`, `serde`, `serde_json`, `wasmtime`, `rusmes-jmap`, or the small
  HTTP/server helpers needed for JMAP e2e are insufficient and another
  significant runtime dependency is required, stop and justify the addition
  before proceeding.
- Transport model: if the JMAP client cannot be implemented entirely through
  the host `http-request` interface, stop and escalate rather than reintroduce
  guest-managed sockets.
- Test environment: if Docker, `rusmes-jmap`, or the local Wasm runtime cannot
  be exercised non-interactively from this repository, stop and report the
  exact blocker instead of weakening the e2e promise.
- Mock server: if `rusmes-jmap` proves too incomplete for an honest JMAP e2e
  after one focused compatibility pass, escalate before replacing it. The user
  has suggested `cyrus-docker-test-server` as a stronger JMAP server option,
  but switching to it changes the test strategy from lightweight mock to full
  server integration and should be recorded explicitly.
- Iterations: if a gate or failing test still does not converge after three
  focused repair attempts, capture the evidence and ask for direction.

## Risks

- Risk: `jmap-client` may not be usable inside the Wasm guest because it owns
  the HTTP transport instead of delegating through Ironclaw's host API.
  Severity: high Likelihood: high Mitigation: treat the current source
  investigation as the deciding feasibility slice. If no transport injection
  point exists, implement a small local JMAP client instead of forcing the
  crate into the wrong runtime model.

- Risk: `rusmes-jmap` may not expose every behaviour needed for e2e, especially
  session discovery and message mutation. Severity: high Likelihood: high
  Mitigation: wrap it in a thin local compatibility harness when needed,
  keeping `rusmes-jmap` as the underlying mock server while documenting any
  shims honestly.

- Risk: `libjmap` may appear transport-pluggable but still be awkward to bind
  to Ironclaw's host HTTP API because it requires a Tower `Service` returning
  `hyper::body::Incoming` and uses async/Tokio-oriented control flow. Severity:
  medium Likelihood: high Mitigation: treat it as investigated unless a trivial
  adapter proves possible. Prefer a smaller local JMAP client over a brittle
  response-body adapter.

- Risk: Ironclaw’s neighbouring examples are simple HTTP tools, so they do not
  provide a ready-made pattern for component execution in local tests.
  Severity: medium Likelihood: high Mitigation: build a tiny host harness in
  `tests/e2e/` that implements only the imported host functions used by this
  tool, then reuse that harness for both BDD and e2e coverage.

- Risk: Packaging can drift if the build target and the packaged output do not
  agree on the final Wasm filename or component format. Severity: medium
  Likelihood: medium Mitigation: make `make wasm` produce one canonical
  artifact path and make `make package` copy only from that path into a
  deterministic package directory.

- Risk: Mapping high-level mail actions onto JMAP may require extra lookup
  calls, such as resolving mailbox names to mailbox IDs before querying
  messages. Severity: medium Likelihood: medium Mitigation: keep the public
  action surface narrow, document defaults explicitly, and centralize the
  mailbox-resolution logic so tests can cover it.

- Risk: designing a reusable JMAP codec crate without overcommitting to one
  transport or one subset of JMAP may expand scope quickly. Severity: medium
  Likelihood: medium Mitigation: keep the published surface deliberately narrow
  around the methods this tool needs first, but structure the crate so new JMAP
  method families can be added without breaking the transport boundary.

## Plan of work

### Milestone 1: Record the feasibility findings and freeze the pivot

Update this ExecPlan before code changes so the repository history explains why
the IMAP path is being removed. Capture three findings explicitly:

1. Ironclaw exposes only the host `http-request` capability for outbound
   network access from Wasm tools.
2. The previously delivered IMAP implementation cannot function inside the Wasm
   runtime here because guest TCP fails.
3. `jmap-client` uses `reqwest` internally without a transport injection point,
   so it is not a drop-in fit for Ironclaw's host-mediated HTTP model.

The acceptance check for this milestone is an updated, self-contained plan that
describes the revised implementation path and the known e2e limitations of
`rusmes-jmap`.

### Milestone 2: Introduce the reusable JMAP codec crate

Create a new crate, tentatively `crates/jmap-codec/`, and move the
transport-agnostic JMAP protocol work there.

The first published surface should include:

1. Session resource types and parsing.
2. Generic JMAP request and response envelopes.
3. Typed request and response models for the mail methods this tool needs
   first, such as `Mailbox/get`, `Email/query`, `Email/get`, and `Email/set`.
4. Serialization helpers and narrow validation utilities.

The crate must compile without Ironclaw or Wasm-specific dependencies. The
acceptance check for this milestone is that the Wasm tool can depend on the
crate without the crate depending back on the tool.

### Milestone 3: Replace IMAP-specific dependencies, modules, and naming

Remove the direct IMAP protocol implementation from the crate:

1. Delete the IMAP-only dependency set and add the small HTTP/server
   dependencies needed for JMAP and e2e.
2. Replace IMAP-specific modules such as `protocol.rs` with JMAP-specific tool
   modules for host HTTP transport, service orchestration, and output mapping.
3. Rename package artefacts, capabilities sidecars, and documentation text from
   IMAP-focused terminology to JMAP-focused terminology where that improves
   user clarity.

The target module split is:

- `src/lib.rs`: WIT bindings, exported guest implementation, narrow wiring.
- `src/schema.rs`: JSON schema and description text for the JMAP tool.
- `src/actions.rs`: tagged action enum plus argument validation.
- `src/host.rs`: wrappers around imported host functions, including HTTP.
- `src/jmap_transport.rs`: low-level host HTTP transport that uses the codec
  crate's request and response types.
- `src/service.rs`: high-level mail actions built on JMAP.
- `src/errors.rs`: typed domain errors mapped to user-facing strings.

### Milestone 4: Implement the host-HTTP JMAP client

Build a small local JMAP client that performs only the operations this tool
needs:

1. Discover the session document from `/.well-known/jmap`.
2. Resolve the effective account ID from the session when the request omits it.
3. List mailboxes via `Mailbox/get`.
4. Resolve mailbox names to mailbox IDs where required.
5. List messages via `Email/query` plus `Email/get`.
6. Fetch one message via `Email/get`.
7. Mark one message as seen via `Email/get` plus `Email/set`, unless the server
   lacks mutation support.

The implementation must route every request through the host HTTP bridge rather
than guest-owned sockets. Keep the raw JSON surface local to the JMAP module so
the outer action layer remains testable with narrow typed outputs.

### Milestone 5: Rebuild the public action model around JMAP semantics

Keep the current user-facing action set if it still reads naturally:

- `list_mailboxes`
- `list_messages`
- `get_message`
- `mark_seen`

Revise the shared configuration fields so they reflect JMAP:

- `base_url`
- `account_id` (optional)
- `auth_secret_name` (optional presence check)
- mailbox selectors appropriate for JMAP, such as `mailbox_id` or
  `mailbox_name`

Update the schema string and output types so they describe JMAP-backed
behaviour rather than IMAP envelopes and sequence numbers.

### Milestone 6: Rebuild unit and behavioural coverage

Replace the IMAP-oriented tests with JMAP-oriented ones:

1. `rstest` unit tests for action parsing, session/account resolution, mailbox
   lookup, request body construction, response mapping, and secret preflight.
2. `rstest-bdd` behavioural tests that run the real execution entry point
   against a fake host/service pair.

The behavioural scenarios should remain user-visible:

- listing mailboxes succeeds
- listing messages from a mailbox succeeds
- fetching one message succeeds
- missing auth secret fails fast

### Milestone 7: Rebuild packaging and capabilities

Update the repository packaging story so it matches the JMAP implementation:

1. Revise the capabilities sidecar to request only the allowed JMAP HTTP
   endpoints plus the secret names used for auth preflight.
2. Keep `make wasm` and `make package`, but update artefact names and copied
   files if the tool is renamed.
3. Keep `make e2e`, but make it run the JMAP-backed ignored tests rather than
   the old GreenMail IMAP path.

`make all` should remain focused on formatter, linter, and unit/behavioural
tests. The Docker-backed e2e path stays opt-in via `make e2e`.

### Milestone 8: Build the e2e harness around `rusmes-jmap`

Construct an e2e harness that uses `rusmes-jmap` as the underlying mock JMAP
server, while documenting and handling its current limitations.

Current findings that the harness must account for:

1. `rusmes-jmap::JmapServer::routes()` is Axum-based and suitable as a local
   test server.
2. The session endpoint currently hardcodes its advertised base URL to
   `https://jmap.example.com`, so a local harness may need to rewrite or proxy
   the session document.
3. `Email/set` currently returns `notImplemented`, so `mark_seen` e2e coverage
   may require a thin compatibility shim or a scoped expectation that mutation
   is unavailable in the mock.

The Cyrus Docker test server remains a fallback if these gaps make the
`rusmes-jmap` path dishonest. Its published README advertises JMAP over HTTP on
port `8080` plus a management API on `8001`, which makes it more realistic but
also heavier than an in-process Axum mock.

The harness should still:

1. Build the Wasm component.
2. Verify the built Wasm is a valid component with `wit-component`.
3. Instantiate the component in a minimal host runtime.
4. Exercise the JMAP read path against the running mock server.
5. Assert honestly on the mutation path according to the chosen compatibility
   strategy.

### Milestone 9: Rewrite `docs/users-guide.md`

The users guide must describe the JMAP tool, not the superseded IMAP path.
Cover:

- what the tool does and the initial action set
- how host-injected HTTP auth works for JMAP
- the expected capabilities file
- how to build the Wasm component with `make wasm`
- how to package it with `make package`
- how to run unit, behavioural, and e2e tests
- representative JSON request examples and outputs
- known limitations, especially any mock-server gaps or unsupported JMAP
  mutation flows in local e2e

## Validation strategy

Run every gate with durable logs and `pipefail`, using this repository’s
preferred log naming pattern.

```bash
set -o pipefail && make check-fmt | tee /tmp/check-fmt-imap-wasm-$(git branch --show).out
set -o pipefail && make lint | tee /tmp/lint-imap-wasm-$(git branch --show).out
set -o pipefail && make test | tee /tmp/test-imap-wasm-$(git branch --show).out
set -o pipefail && make wasm | tee /tmp/wasm-imap-wasm-$(git branch --show).out
set -o pipefail && make package | tee /tmp/package-imap-wasm-$(git branch --show).out
set -o pipefail && make e2e | tee /tmp/e2e-imap-wasm-$(git branch --show).out
set -o pipefail && make markdownlint | tee /tmp/markdownlint-imap-wasm-$(git branch --show).out
set -o pipefail && make nixie | tee /tmp/nixie-imap-wasm-$(git branch --show).out
```

Observable success criteria:

- `make test` runs both `rstest` unit tests and `rstest-bdd` behavioural tests.
- `make wasm` produces exactly one canonical Wasm component artifact.
- `make package` creates a package directory containing the Wasm artifact and
  the capabilities file.
- `make e2e` proves that the packaged Wasm can execute against the JMAP mock
  harness and return the expected mail results, with any mock limitations
  documented explicitly.
- `docs/users-guide.md` is present and sufficient to reproduce the build and
  test flow.

## Progress

- [x] 2026-03-08 22:29 GMT: Reviewed repository instructions, current stub
  state, and neighbouring Ironclaw tool examples.
- [x] 2026-03-08 22:34 GMT: Read the local Ironclaw Wasm guide and the
  `rstest-bdd` user guide to anchor the plan in the intended interfaces.
- [x] 2026-03-08 22:42 GMT: Drafted the implementation milestones, risks, and
  validation strategy in this ExecPlan.
- [x] 2026-03-08 22:49 GMT: User approved the plan and implementation began.
- [x] 2026-03-08 22:54 GMT: Fixed the broken pinned Rust toolchain and excluded
  the local untracked reference guide from repository-wide documentation gates
  so Make targets can reflect tracked project state.
- [x] 2026-03-08 23:18 GMT: Proved that `imap-next` compiles for
  `wasm32-wasip2` when used in sans-I/O mode with default features disabled,
  while `imap_session` and Tokio-backed `imap-next` networking do not.
- [x] 2026-03-08 23:46 GMT: Replaced the stub crate with the Ironclaw WIT
  component skeleton, IMAP action surface, `imap-next` protocol adapter,
  capabilities sidecar, and `rstest` plus `rstest-bdd` coverage.
- [x] 2026-03-09 00:44 GMT: Added `make wasm`, `make package`, and `make e2e`,
  plus a `wit-component` artifact check and Wasmtime component-instantiation
  test.
- [x] 2026-03-09 01:33 GMT: Added a GreenMail-backed ignored e2e test for the
  native `NetworkImapService` path after documenting that guest-side TCP still
  fails under current Wasmtime/WASIp2 execution in this environment.
- [x] 2026-03-09 01:38 GMT: Wrote `docs/users-guide.md` and replayed the full
  repository gates, including `make e2e`.
- [x] 2026-03-09 10:08 GMT: Investigated whether `jmap-client` can replace the
  direct IMAP path and confirmed that it owns the `reqwest` transport
  internally, so it is not a viable drop-in client for Ironclaw's host-mediated
  HTTP model.
- [x] 2026-03-09 10:12 GMT: Investigated `rusmes-jmap` for e2e and confirmed
  that it provides Axum routes suitable for a local mock server, while also
  finding two gaps the harness must handle: the session endpoint advertises a
  fixed `https://jmap.example.com` base URL and `Email/set` currently returns
  `notImplemented`.
- [x] 2026-03-09 10:26 GMT: Investigated `libjmap` 0.1.1 and found that it is
  more transport-pluggable than `jmap-client`, but still mismatched with
  Ironclaw's host HTTP ABI because it expects an async Tower service returning
  `hyper::body::Incoming`.
- [x] 2026-03-09 10:29 GMT: Investigated the published
  `cyrusimap/cyrus-docker-test-server` README and confirmed that it exposes a
  full Cyrus server with JMAP on HTTP port `8080` plus a management service on
  `8001`, making it a plausible fallback e2e target if `rusmes-jmap` proves too
  incomplete.
- [x] 2026-03-09 10:36 GMT: User required the JMAP protocol implementation to
  live in a separate reusable crate so it can be published independently as a
  transport-agnostic JMAP codec.
- [ ] 2026-03-09 10:18 GMT: Remove the IMAP-specific implementation and replace
  it with a host-HTTP JMAP implementation, a reusable codec crate, and revised
  tests, packaging, and user documentation.

## Surprises & Discoveries

- `grepai` and `leta` are installed in this environment, but neither is
  configured for this repository today. Planning relied on direct local file
  inspection instead.
- The current repository contains only a stub `src/lib.rs` and a generic
  `Makefile`; there is no existing Wasm/component packaging pipeline yet.
- The neighbouring Ironclaw examples use the `cdylib` single-crate pattern plus
  a root capabilities JSON file rather than a multi-crate workspace.
- The repository pinned `nightly-2026-06-05`, which does not exist on
  2026-03-08, so even `cargo fmt` could not start until that baseline was
  corrected.
- The requested guide file is currently an untracked local reference document,
  so repository-wide doc gates need to ignore it or they stop reflecting the
  health of tracked project files.
- User-directed fallback order for IMAP client feasibility is now
  `imap-next` first, then `imap`, then `imap_session` if the earlier options do
  not work under the Wasm target.
- `cargo component build` was not a good fit for this plugin because the
  desired socket-capable target is `wasm32-wasip2`, while the working local
  build path was `cargo rustc --target wasm32-wasip2 --crate-type=cdylib`.
- `imap-next` works for this plugin only in sans-I/O mode; its Tokio-backed
  stream support does not compile for the Wasm target used here.
- The built Wasm component decodes and instantiates correctly under Wasmtime,
  but guest-side TCP still fails in this environment with
  `Protocol not available (os error 50)`. The e2e strategy therefore validates
  the component artifact separately from the GreenMail-backed native IMAP flow.
- GreenMail's documented default behaviour of auto-creating an account with
  login and password equal to the recipient address was the most reliable local
  seeding strategy in this environment.
- `jmap-client` 0.4.0 builds its own `reqwest::Client` inside `ClientBuilder`
  and `Client::send`, and the crate source does not expose a transport
  abstraction that would let this tool reuse Ironclaw's host `http-request`
  capability.
- `rusmes-jmap` 0.1.0 exposes Axum routes through `JmapServer::routes()`, so it
  is usable as the basis of a local mock server, but its session endpoint
  currently advertises `https://jmap.example.com` regardless of the real local
  listener.
- `rusmes-jmap` currently leaves `Email/set` unimplemented, so a realistic
  `mark_seen` e2e needs either a thin compatibility shim in the local harness
  or an honest scoped limitation in the test contract.
- `libjmap` 0.1.1 is a more reusable design than `jmap-client` because
  `JmapClient<C>` is generic over a Tower `Service`, but the required service
  shape is still not a clean match for Ironclaw's synchronous `http-request`
  import, especially because it expects `Response<hyper::body::Incoming>`.
- The published `cyrusimap/cyrus-docker-test-server` image advertises a full
  Cyrus stack with JMAP on HTTP port `8080` and a management service on `8001`,
  so it is a credible fallback if the lighter `rusmes-jmap` mock cannot support
  an honest e2e contract.
- The implementation is no longer allowed to stay a single crate. The user now
  requires one reusable transport-agnostic JMAP codec crate plus the Wasm tool
  crate that consumes it.

## Decision Log

- Decision: Keep the planned implementation in a single crate until a concrete
  complexity problem proves otherwise. Rationale: The neighbouring tools follow
  that shape, and it minimizes packaging and documentation drift for a first
  plugin.

- Decision: Treat `make e2e` as a separate heavy gate instead of folding it
  into `make test`. Rationale: Docker-backed GreenMail and Wasm component
  execution are slower and more failure-prone than normal unit and behavioural
  tests; keeping them explicit preserves fast local iteration.

- Decision: Require an early `imap-next` feasibility slice before fleshing out
  the full action surface. Rationale: The user explicitly required `imap-next`,
  and that requirement has the highest technical risk in a Wasm-targeted tool.

- Decision: If the `imap-next` feasibility slice fails, test `imap` and then
  `imap_session` before declaring the direct-client route blocked. Rationale:
  The user explicitly requested that fallback order during implementation.

- Decision: Use `wit-component` in the e2e path as an explicit artifact
  validity check before runtime execution. Rationale: Packaging correctness is
  part of the user request, and the end to end test should fail clearly if the
  build step emits the wrong Wasm shape.

- Decision: Normalize the broken toolchain pin to `stable` before feature work.
  Rationale: The previous pin prevented every Cargo-based gate from running,
  while the dependency set for this task only requires stable Rust 1.85+.

- Decision: Build the Wasm artifact with Cargo Rustc targeting
  `wasm32-wasip2` and crate type `cdylib`, while keeping host-side Cargo
  targets as `rlib`. Rationale: native `cargo test` tried to link the generated
  WIT export surface as an ELF shared library, which fails because the exported
  symbol names are only valid for the component build path.

- Decision: Keep the ignored Wasm-component e2e and the ignored GreenMail IMAP
  e2e as separate tests under one `make e2e` target. Rationale: This preserves
  explicit artifact validation with `wit-component` and Wasmtime while honestly
  acknowledging that current guest-side TCP is not available under the
  Wasmtime/WASIp2 runtime used in this repository.

- Decision: Supersede the completed IMAP implementation with a JMAP rewrite
  rather than trying to salvage guest-side IMAP sockets. Rationale: Ironclaw
  exposes only host-mediated HTTP to Wasm tools, and the current runtime here
  still rejects guest TCP.

- Decision: Treat `jmap-client` as investigated and rejected for this Wasm tool.
  Rationale: The crate constructs and owns `reqwest` clients internally, so it
  does not fit Ironclaw's host-HTTP execution model without a fork or invasive
  local patching.

- Decision: Use a small custom JMAP client over `host.http-request` and keep
  `rusmes-jmap` in the e2e path as the underlying mock server. Rationale: That
  combination aligns with Ironclaw's capability surface and the user's request
  for a JMAP pivot plus `rusmes-jmap`-backed e2e coverage.

- Decision: Treat `libjmap` as investigated but not adopted for the Wasm tool.
  Rationale: Although it abstracts over an HTTP service, its Hyper/Tower body
  contract is still too far from Ironclaw's imported host HTTP function to be a
  low-risk fit.

- Decision: Keep `cyrus-docker-test-server` as a documented fallback rather
  than the default e2e target for now. Rationale: It looks more realistic than
  `rusmes-jmap`, but it is also heavier, Docker-bound, and not yet required if
  a thin `rusmes-jmap` compatibility harness can support honest coverage.

- Decision: Split the repository into a small workspace with a reusable
  transport-agnostic JMAP codec crate plus the Wasm tool crate. Rationale: The
  user explicitly requested a publishable, reusable JMAP protocol crate rather
  than embedding protocol types and builders inside the Ironclaw tool.

## Outcomes & Retrospective

Current state before the JMAP rewrite:

- The repository contains a completed IMAP-oriented Wasm tool implementation,
  full gates, and matching documentation.
- That implementation is now understood to be architecturally mismatched with
  Ironclaw's real Wasm capability surface because it depends on guest TCP.
- The user has directed a pivot to JMAP and removal of the IMAP-specific path.

What this revised plan will produce next:

- A JMAP-oriented Wasm tool that routes all outbound network traffic through
  Ironclaw's host HTTP bridge.
- A reusable JMAP codec crate that can be published and reused outside this
  Ironclaw-specific tool.
- Revised tests, packaging, capabilities, and user documentation that match the
  JMAP implementation.
- An honest e2e harness based on `rusmes-jmap`, with any required compatibility
  shims and remaining limitations documented explicitly.
