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
`setup-rust` exports one). Two builds are deliberately excluded: coverage
assigns `RUSTFLAGS` without the fast flags, because a measurement should not
depend on them, and the release recipe and workflow keep the platform linker,
because they assign `RUSTFLAGS` (even an empty value displaces the
configuration). Cargo has no per-profile `rustflags`, so a direct
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
workflow edit fails there. The decision is recorded in
[ADR 001](adr-001-rust-build-standard.md). The contract runs `make -n`, so a
direct `cargo test` needs GNU make on the `PATH`. It fails when `make` is
missing instead of skipping, so a missing tool cannot read as a pass.

### Cranelift

Exception: Cranelift is not the development-profile backend, because the
repository has no test suite to measure it with. The estate adopts the backend
only where the full suite passes under it, and a build alone proves nothing
about miscompilation or unwinding (recorded 2026-09-29, on the pinned
`nightly-2026-06-05`). Revisit when the repository has tests: measure the whole
suite under the backend and adopt it if every test passes.

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
