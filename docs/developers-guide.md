# Developers' guide

This guide records how jmap-wasm is built and checked.

## The build standard

Development, test, lint and typecheck builds use the parallel `rustc` frontend
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
depend on them, and release builds keep the platform linker.

### Cranelift

Exception: Cranelift is not the development-profile backend, because the
repository has no test suite to measure it with. The estate adopts the backend
only where the full suite passes under it, and a build alone proves nothing
about miscompilation or unwinding (recorded 2026-09-29, on the pinned
`nightly-2026-06-05`). Revisit when the repository has tests: measure the whole
suite under the backend and adopt it if every test passes.
