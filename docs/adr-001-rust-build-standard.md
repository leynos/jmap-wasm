# Architectural decision record (ADR) 001: Adopt the Rust build standard for development builds

## Status

Accepted on 8 October 2026. Development, test, lint and typecheck builds use
the parallel `rustc` frontend (`-Zthreads=8`) and, on Linux, the `mold` linker,
while coverage and release builds stay off the fast flags.

## Date

2026-10-08.

## Context and problem statement

Edit-compile cycles are dominated by frontend time and by link time. The
estate's build standard (`rust-build-defaults`) addresses both. Cargo applies
exactly one `rustflags` source, and an assigned `RUSTFLAGS` replaces every
configuration source, so a recipe or workflow step that assigns `RUSTFLAGS`
silently loses the flags unless it restates them. Measurements and shipped
artefacts also need a baseline that the fast flags would disturb.

## Decision drivers

- Faster local and CI development builds without changing what the code means.
- Coverage numbers that do not depend on the fast flags.
- A release that ships from the platform linker.
- A flag lost through a recipe or workflow edit must fail a test, not pass
  quietly.

## Options considered

- Configure the flags only in `.cargo/config.toml`. This leaves every recipe
  and CI step that assigns `RUSTFLAGS` without them.
- Restate the flags in each recipe and step, held by a contract test. This is
  the option taken.
- Adopt Cranelift for development builds. The developers' guide records why
  this repository does not.

## Decision outcome

`.cargo/config.toml` carries the flags in every `rustflags` source, and adds the
`mold` linker flag to the Linux table only. The Makefile composes the same
flags into each development recipe, keeping the caller's own `RUSTFLAGS`.
Coverage assigns its own flags and ignores the caller's. The release recipe
keeps the caller's flags and names neither fast flag, so it ships from the
platform linker; a direct `cargo build --release` takes the configuration's
flags unless `RUSTFLAGS` is assigned. Cranelift is not adopted; the developers'
guide records the measurement and the reason.

`tests/build_standard_contract.rs` reads the Cargo configuration sources and
the commands `make -n` prints for each target, and the `setup-rust` steps of
the workflows, so a flag lost through a recipe or workflow edit fails there.

## Consequences

- Linux builds require `mold`. The flag reaches the linker through `cc`, which
  must be GCC 12.1 or newer, or clang.
- A change to a recipe or workflow step that assigns `RUSTFLAGS` must restate
  the flags, and the contract says which clause fails when it does not.
- Revisit Cranelift when the toolchain or the failing tests change.

## Addendum: Whitaker and the development footprint

The direct Whitaker command is exempt from the development flags and from the
caller's `RUSTFLAGS`: it assigns `-D warnings` only. Whitaker runs on its
installer-managed toolchain, which the frontend flag and the `mold` argument
are not meant for. The trade-off is that a caller's extra flags do not reach
Whitaker. The footprint settings (`incremental = false`, `debug = 0` and the
native `embed-metadata` key) are read as text by `tests/footprint_contract.rs`,
which starts no Cargo process.

The footprint trade-offs: an unincremental build recompiles the whole crate on
each edit, in exchange for a `target/` directory that does not grow with every
change; `debug = 0` leaves a debugger without symbols for dependencies and this
crate unless the developer sets `CARGO_PROFILE_DEV_DEBUG=true`; and the native
`embed-metadata = false` key ties the configuration to dated nightlies from
2026-07-31.
