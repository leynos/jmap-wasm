# Developers' guide

This guide records how jmap-wasm is built and checked.

## The build standard

Development, test, lint, and typecheck builds use the parallel `rustc` frontend
(`-Zthreads=8`) and, on Linux, the `mold` linker (`-Clink-arg=-fuse-ld=mold`).
These are defaults in `.cargo/config.toml`, which Cargo discovers on its own,
so a bare `cargo build` gets them. `mold` ships for Linux only, so the linker
flag lives in a Linux-only table and macOS and Windows keep their platform
linker. Cargo selects one `rustflags` source rather than merging them, so every
source repeats the same flags apart from the linker.

An assigned `RUSTFLAGS` replaces the configuration's flags, so the Makefile
recipes that set it compose the standard's flags onto any inherited value (CI's
`setup-rust` exports one). The `lint` recipe's Whitaker command is a deliberate
exception: Whitaker runs on its installer-managed toolchain, which must not
inherit the repository's development flags, so it assigns
`RUSTFLAGS="$(RUST_FLAGS)"` (`-D warnings`) alone, keeps none of the caller's
flags and takes neither `-Zthreads=8` nor `mold`, while Clippy in the same
recipe keeps the standard's flags. Two builds are also deliberately excluded:
coverage assigns `RUSTFLAGS` without the fast flags, because a measurement
should not depend on them, and the release recipe and workflow keep the
platform linker, because they assign `RUSTFLAGS` (even an empty value displaces
the configuration). Cargo has no per-profile `rustflags`, so a direct
`cargo build --release` takes the configuration's flags unless `RUSTFLAGS` is
assigned too.

On Linux, install `mold` before building: the configuration names it, so a
build without it fails at link time. CI installs it through `setup-rust`'s
`install-mold` input. `tests/build_standard_contract.rs` holds the standard. It
reads the configuration sources, the commands `make -n` prints for each
development target on a Linux host and a macOS host (each keeping the caller's
own `RUSTFLAGS`) and for the release target (the coverage exclusion is checked
in the workflow steps) on a Linux host, and the `setup-rust` steps of the CI
workflows (each must pass `install-mold`), so a flag lost through a recipe or
workflow edit fails there. For Whitaker the contract runs the other way: a
Whitaker command that carries a development flag, inherits the caller's flags,
drops `-D warnings`, or assigns nothing fails. The decision is recorded in
[ADR 001](adr-001-rust-build-standard.md). The contract runs `make -n`, so a
direct `cargo test` needs GNU make on the `PATH`. It fails when `make` is
missing instead of skipping, so a missing tool cannot read as a pass.

### Cranelift

Exception: Cranelift is not the development-profile backend, because the
repository has no test suite to measure it with. The estate adopts the backend
only where the full suite passes under it, and a build alone proves nothing
about miscompilation or unwinding (recorded 2026-09-29, on the pinned
`nightly-2026-06-05`). The pin is now `nightly-2026-10-30`; the repository
still has no test suite, so no measurement was taken on it and the exception
stands without a deferral date. Revisit when the repository has tests: measure
the whole suite under the backend and adopt it if every test passes.

## Development build footprint

The repository follows the estate's `rust-dev-build-footprint` rule.
`Cargo.toml`'s default development profile sets `incremental = false` and
`debug = 0`, and the test profile inherits both, so builds keep no incremental
state and no debug information and write far fewer intermediate artefacts. The
trade-off is that a debugger sees no symbols or line tables in a default
development build and every rebuild recompiles whole crates. To debug, run with
`CARGO_PROFILE_DEV_DEBUG=true`; to rebuild incrementally, set
`CARGO_INCREMENTAL=1`. `.cargo/config.toml` also sets
`[unstable] embed-metadata = false`, the native Cargo key for nightlies from
2026-07-31, so Cargo coordinates metadata output itself. No raw rustc metadata
flags are used, and the pin is not moved to suit the key. Whoever moves the pin
across a boundary of the rule must switch the key to match the new pin family.
`tests/footprint_contract.rs` asks Cargo itself (`--unit-graph` and
`config get`, which compile nothing) for the effective profiles and the key, so
an override in any configuration file Cargo merges is caught as well as the
manifest's own table.

## The spelling gate

`make spelling` enforces en-GB-oxendict spelling by running the
`typos-config-builder` gate, pinned by `TYPOS_CONFIG_BUILDER_VERSION` in the
`Makefile` (currently `v0.1.3`). The gate regenerates `typos.toml` from the
live shared dictionary and this repository's `typos.local.toml` overlay on
every run, runs Typos over tracked Markdown, and enforces the shared phrase
corrections that single-word checks cannot express. `typos.toml` is generated
and never drift checked in continuous integration; commit the regenerated file
when it changes, and keep repository-specific exceptions in `typos.local.toml`
as narrow patterns. The builder requires Python 3.14 or newer, so the target
passes `--python 3.14` and `uv` fetches that interpreter when the host lacks
one. Raise the pin together with the regenerated `typos.toml`, never on its
own. See [the scripting standards](scripting-standards.md) for the policy.

## CodeScene coverage and the CV-005 contract

Main owns coverage publication (estate rule CV-005). A pull request measures
coverage for its own ratchet check only, in the `build-test` job of `ci.yml`
(`with-ratchet: 'true'`, `publish-artefact: 'false'`); it never contacts
CodeScene. `.github/workflows/coverage-main.yml` runs on every push to `main`
in the `codescene` environment, generates coverage again and uploads it with
`mode: upload`, guarded on `github.ref == 'refs/heads/main'` alone. The token
reaches the uploader only through its `access-token` input, never an `env`.
When the repository has no `CS_ACCESS_TOKEN` (no CodeScene project yet) the
uploader records a `CodeScene upload skipped` notice and a step-summary line
and the run still succeeds.

`make test-workflow-contracts` checks that shape. It runs the shared
`cv005-contracts check --repository .` from the commit named by
`CV005_CONTRACTS_REF` in the `Makefile` (a bump of that pin is a reviewed
change, never a silent upgrade), then the local wiring tests in
`tests/workflow_contracts`. It needs `uv`, Python 3.13 for the checker, and
`pytest` and `pyyaml`, which `uv run --with` supplies. Both runs go through
`UV` and `UV_ENV`, so a `uv` outside `PATH` can be injected. `make all` and CI
run the target; `.github/cv005.toml` holds the repository name and nothing
else. A Dependabot pull request merged by the automerge workflow fires no push
event, so that merge publishes nothing until the next push to `main`
(shared-actions #518 tracks the fix).
