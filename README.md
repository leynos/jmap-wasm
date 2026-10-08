# IMAP WASM Plugin for IronClaw

[![Ask DeepWiki](https://deepwiki.com/badge.svg)](
https://deepwiki.com/leynos/jmap-wasm)

This is a generated project using [Copier](https://copier.readthedocs.io/).

## Build standard

Development builds (`make test`, `make lint`, `make typecheck` and the debug
build) use the parallel `rustc` frontend (`-Zthreads=8`) and, on Linux, the
`mold` linker. Install `mold` before building on Linux: the configuration names
it, so a build without it fails at link time.

The flags live in `.cargo/config.toml`, but Cargo applies exactly one
`rustflags` source and an assigned `RUSTFLAGS` replaces every configuration
source. The Makefile therefore restates the flags in each recipe and keeps any
`RUSTFLAGS` you set, appending the standard flags after yours. Two builds are
held out on purpose: the coverage build assigns its own flags, because a
measurement should not depend on the fast flags, and the release build.
`make release` keeps your `RUSTFLAGS` and names neither fast flag, so a shipped
artefact links with the platform linker. A bare `cargo build --release` is
different: it takes the configuration's flags unless you assign `RUSTFLAGS`
yourself, for example `RUSTFLAGS="" cargo build --release`.

Cranelift is not adopted; the developers' guide records the measurement and the
reason. See [ADR 001](docs/adr-001-rust-build-standard.md) for the reasoning.
