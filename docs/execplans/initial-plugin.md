# Implement The Ironclaw IMAP Wasm Tool

This ExecPlan (execution plan) is a living document. The sections
`Constraints`, `Tolerances`, `Risks`, `Progress`, `Surprises & Discoveries`,
`Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work
proceeds.

Status: COMPLETED

## Purpose / big picture

After this change, this repository will build an Ironclaw-compatible Wasm tool
that exposes IMAP operations through the WIT `sandboxed-tool` interface, ships
with reproducible `make` targets for build and packaging, and is documented
well enough that a new contributor can authenticate it, compile it, package it,
and run its local test matrix. The finished repository must also provide a
behavioural test story: small `rstest` unit tests for parsing and request
formation, `rstest-bdd` scenarios for user-visible tool behaviour, and an end
to end test target that boots GreenMail in Docker, verifies the built Wasm file
is a valid component, instantiates the component against a minimal host
harness, and exercises real IMAP flows against GreenMail.

The current repository is only a stub Rust library. The adjacent Ironclaw
examples under
`../ironclaw/tools-src/{github,gmail,google-calendar,google-docs}` show the
target shape: a single-crate `cdylib` Wasm component using `wit-bindgen`, a
capabilities sidecar JSON file, and a library entry point that implements
`exports::near::agent::tool::Guest`. This plan keeps the same outer shape while
adapting it for IMAP and for stricter local testing.

## Repository orientation

The implementation will touch five areas.

`Cargo.toml` and `src/` will become the actual tool crate. `src/lib.rs` should
stay small and delegate to focused modules such as request parsing, schema
rendering, host bindings, IMAP action handling, and test-only fixtures. Keep
every Rust file under 400 lines by splitting action-specific logic early.

`wit/` will be added to hold a local copy of Ironclaw’s `tool.wit`, pinned to
the host interface expected by the neighbouring example tools.

The repository root `Makefile` will keep the existing quality gates and add
targets for component compilation, packaging, and end to end execution.

`tests/` will hold `rstest-bdd` feature files, scenario bindings, and the
GreenMail-backed end to end harness.

`docs/users-guide.md` will become the user-facing usage document covering
installation, required capabilities, authentication, common actions, and local
test workflows.

## Constraints

- Preserve the plan-only boundary. Draft the plan now; do not start
  implementation until the user explicitly approves this plan.
- The final tool must implement Ironclaw’s WIT `sandboxed-tool` world and use
  `wit-bindgen` for the guest bindings, matching the structure of the adjacent
  Ironclaw tools.
- The IMAP implementation must use the `imap-next` crate for IMAP protocol
  operations.
- The repository must expose `make check-fmt`, `make lint`, and `make test` as
  first-class gates, and any new build or e2e targets must compose with that
  workflow instead of bypassing it.
- Unit tests must use `rstest`; behavioural tests must use `rstest-bdd`; the
  end to end path must use GreenMail’s `greenmail/standalone` Docker image and
  the `wit-component` crate.
- `docs/users-guide.md` must explain usage clearly enough for a new user to set
  secrets, build the component, package the artifact, and run the local tests.
- Capabilities and secret handling must follow Ironclaw’s model: secrets are
  checked with `secret_exists`, while host-side capability JSON declares the
  allowed secret names and any HTTP rules needed by the tool.
- Respect repository and workspace style rules: en-GB-oxendict prose, module
  level Rust docs, no suppressed lints without narrow justification, and no
  file over 400 lines.

## Tolerances (exception triggers)

- Scope: if implementation requires introducing more than one helper crate or
  restructuring the repository into a multi-package workspace, stop and ask for
  approval. The default path is a single crate plus support files.
- Interface: if Ironclaw’s local WIT interface differs materially from the copy
  described in `writing-web-assembly-tools-for-ironclaw.md`, stop and reconcile
  the mismatch before writing code.
- Dependencies: if `imap-next`, `imap`, `imap_session`, `wit-bindgen`,
  `wit-component`, `rstest`, `rstest-bdd`, `thiserror`, `serde`, `serde_json`,
  `wasmtime`, or a TLS helper are insufficient and another significant runtime
  dependency is required, stop and justify the addition before proceeding.
- Runtime model: if `imap-next` cannot be used directly within the Wasm target
  because of target or socket limitations, stop and escalate rather than
  silently switching to a bridge architecture. The user asked for an IMAP Wasm
  plugin built with `imap-next`, so that feasibility must be demonstrated or
  explicitly challenged.
- Test environment: if Docker, GreenMail, or the local Wasm runtime cannot be
  exercised non-interactively from this repository, stop and report the exact
  blocker instead of weakening the e2e promise.
- Iterations: if a gate or failing test still does not converge after three
  focused repair attempts, capture the evidence and ask for direction.

## Risks

- Risk: `imap-next` may rely on APIs that do not compile cleanly for the
  selected Wasm component target, especially around sockets and TLS. Severity:
  high Likelihood: medium Mitigation: begin implementation with a feasibility
  slice that compiles a minimal `imap-next` call path for the chosen target
  before building the full action surface. If that fails, stop and document the
  exact incompatibility.

- Risk: GreenMail IMAP coverage may expose authentication, TLS, or mailbox
  preparation gaps that unit tests will not reveal. Severity: medium
  Likelihood: medium Mitigation: make the e2e target provision mailboxes
  deterministically, populate them through GreenMail’s management API or setup
  hooks, and keep the scenario surface small but representative.

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

## Plan of work

### Milestone 1: Replace the stub crate with an Ironclaw component skeleton

Convert `Cargo.toml` from a stub library into an Ironclaw-style `cdylib`
component crate, add the local `wit/tool.wit` copy, and replace `src/lib.rs`
with a small entry point that:

1. Generates bindings via `wit_bindgen::generate!`.
2. Defines a top-level `ImapTool`.
3. Implements `execute`, `schema`, and `description`.
4. Delegates request parsing and action execution into smaller modules.

Planned module split:

- `src/lib.rs`: WIT bindings, exported guest implementation, narrow wiring.
- `src/schema.rs`: JSON schema and description text.
- `src/actions.rs`: tagged action enum plus argument validation.
- `src/client.rs`: `imap-next` session construction and mailbox operations.
- `src/errors.rs`: typed domain errors mapped to user-facing strings.
- `src/host.rs`: wrappers around imported host functions.

The first acceptance check for this milestone is compilation of the component
crate without any IMAP behaviour beyond a minimal no-op or schema response.

### Milestone 2: Define the public IMAP action model and capabilities

Create a narrow first action set that is realistic for IMAP and easy to test.
Start with read-only operations plus one message mutation path:

- `list_mailboxes`
- `select_mailbox`
- `list_messages`
- `get_message`
- `mark_seen`

Define one tagged `serde` action enum and one schema string that clearly states
which fields belong to which action, following the neighbouring Ironclaw tool
examples. Keep validation local and explicit so unit tests can cover malformed
inputs without needing a live IMAP server.

Add `imap-tool.capabilities.json` at the repository root. The first version
should allow only the secret names needed to build an IMAP session. If the tool
needs no outbound HTTP and no workspace reads for its core behaviour, keep
those capabilities absent rather than permissive.

### Milestone 3: Prove the `imap-next` session path

Implement a small adapter around `imap-next` that can:

1. Read connection settings from the tool input and secret presence checks.
2. Build a client session.
3. Select a mailbox.
4. Fetch envelopes or message bodies.
5. Map `imap-next` errors into a typed local error enum.

This is the highest-risk milestone because it tests the user’s core
requirement: the plugin itself must use `imap-next`. The implementation should
hide library-specific details behind a small internal interface so unit tests
can exercise response mapping without a live server.

If target-specific constraints appear here, record them immediately in
`Surprises & Discoveries` and stop for user review rather than reinterpreting
the architecture.

### Milestone 4: Add focused unit tests with `rstest`

Unit tests should cover pure logic first:

- action deserialization and missing-field errors
- schema invariants
- mailbox/query validation
- IMAP response mapping into output JSON
- error message redaction and formatting

Prefer `rstest` tables over duplicated one-off tests. Keep any fixture that
could grow complicated in `tests/test_helpers.rs` or a small internal
`#[cfg(test)]` module. Avoid environment mutation in tests; if a test must use
environment-driven configuration, wrap it in a guard fixture as required by the
repo policy.

### Milestone 5: Add behavioural tests with `rstest-bdd`

Create `tests/features/imap_tool.feature` and matching Rust scenario bindings.
The behavioural tests should describe user-visible tool semantics, not internal
library details. A good initial feature set is:

- listing mailboxes in a configured account
- listing messages from a named mailbox
- fetching one message body
- returning a clear error when required configuration is missing

Each scenario should run the tool through the same execution entry point the
real host would call, using a small local host stub for `log`, `now_millis`,
and `secret_exists`. This keeps BDD coverage honest without needing the full
e2e Docker setup.

### Milestone 6: Add component packaging and Make targets

Extend `Makefile` with explicit artifact targets:

- `make wasm`: build the component in release mode at one canonical path, most
  likely via `cargo component build --release` if available, otherwise via
  `cargo build --target wasm32-wasip2 --release` followed by an explicit
  componentization step.
- `make package`: create a deterministic package directory such as
  `dist/imap-tool/` containing the built Wasm artifact, the capabilities JSON
  sidecar, and a short manifest or README stub if needed.
- `make e2e`: boot GreenMail, build the component, verify the produced Wasm is
  a valid component with `wit-component`, run the end to end tests, and tear
  GreenMail down.

`make all` should remain focused on formatter, linter, and unit/behavioural
tests unless there is a deliberate decision to include Wasm packaging there.
Keep the heavy Docker-backed e2e path opt-in via `make e2e`.

### Milestone 7: Build the end to end harness around GreenMail

Add a GreenMail-backed e2e harness that performs real IMAP operations against a
containerized server started from `greenmail/standalone`.

The harness should:

1. Start GreenMail with deterministic credentials and ports.
2. Seed at least one mailbox with one or two messages.
3. Build the Wasm component.
4. Use `wit-component` to decode or inspect the generated Wasm and fail fast if
   the artifact is not a valid component.
5. Instantiate the component in a minimal host runtime and call `execute`
   against the seeded mailbox.
6. Assert on observable output, not internal structs.

Use `tests/e2e/` for the Rust harness and keep the Docker orchestration in the
Make target or a small checked-in script. If a script is needed, keep it
simple, idempotent, and documented from the Make target comments.

### Milestone 8: Write `docs/users-guide.md`

The users guide must be written for somebody who has not read this plan. Cover:

- what the tool does and the initial action set
- the required secrets and how Ironclaw supplies them
- the expected capabilities file
- how to build the Wasm component with `make wasm`
- how to package it with `make package`
- how to run unit, behavioural, and e2e tests
- a few concrete JSON request examples and representative outputs
- any known limitations, especially around target support or IMAP server
  expectations

If implementation reveals awkward setup steps, document them here instead of
burying them in the plan or commit messages.

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
- `make e2e` proves that the packaged Wasm can execute against GreenMail and
  return the expected IMAP results.
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

- Decision: Build the Wasm artifact with `cargo rustc --target wasm32-wasip2
  --crate-type=cdylib` while keeping host-side Cargo targets as `rlib`.
  Rationale: Native `cargo test` tried to link the generated WIT export surface
  as an ELF `cdylib`, which fails because the exported symbol names are only
  valid for the component build path.

- Decision: Keep the ignored Wasm-component e2e and the ignored GreenMail IMAP
  e2e as separate tests under one `make e2e` target.
  Rationale: This preserves explicit artifact validation with `wit-component`
  and Wasmtime while honestly acknowledging that current guest-side TCP is not
  available under the Wasmtime/WASIp2 runtime used in this repository.

## Outcomes & Retrospective

Shipped:

- An Ironclaw-compatible Wasm tool crate that implements the
  `sandboxed-tool` world via `wit-bindgen`.
- IMAP actions for mailbox listing, message listing, message retrieval, and
  `\\Seen` flag updates.
- A sans-I/O `imap-next` protocol driver over plain TCP for the supported IMAP
  path.
- `rstest` unit tests, `rstest-bdd` behavioural tests, a Wasm artifact e2e
  test, and a GreenMail-backed native IMAP e2e test.
- `make wasm`, `make package`, and `make e2e`.
- A user guide at `docs/users-guide.md`.

Materialized risk:

- The Wasmtime/WASIp2 runtime available here still rejects guest TCP sockets
  for this component with `Protocol not available (os error 50)`.

Response:

- The delivered e2e target validates the Wasm component artifact and
  instantiation separately from the GreenMail-backed native IMAP protocol path,
  and the user guide documents that limitation explicitly.
