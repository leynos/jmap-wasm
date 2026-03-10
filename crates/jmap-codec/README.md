# jmap-codec

`jmap-codec` provides transport-agnostic Rust types and codec helpers for the
subset of JMAP needed by the Wasm mail tool in this repository.

The crate deliberately excludes any concrete HTTP client, async runtime, or
host binding. It focuses on:

- session resource parsing
- generic JMAP request and response envelopes
- mailbox request and response types
- email query, get, and set request and response types

Transport and authentication belong in a separate layer.
