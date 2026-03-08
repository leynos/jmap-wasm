# IMAP Tool User's Guide

## Overview

`imap-tool` is an Ironclaw-compatible WebAssembly tool that talks to IMAP
servers over plain TCP. It implements the `near:agent/tool@0.3.0`
`sandboxed-tool` world and exposes four actions:

- `list_mailboxes`
- `list_messages`
- `get_message`
- `mark_seen`

The current implementation uses `imap-next` in sans-I/O mode over
`std::net::TcpStream`. That combination works for `wasm32-wasip2`, but it also
means the tool currently supports only non-TLS IMAP, usually on port `143`.

## Important limitations

- TLS is not supported yet. Use a local test server, a trusted internal IMAP
  endpoint, or a tunnel that terminates TLS before the Wasm tool connects.
- Ironclaw can only inject secrets automatically into HTTP requests today. IMAP
  socket credentials therefore cannot be injected by the host.
- The `password_secret_name` field is a presence check only. The tool calls
  `secret-exists` to fail early when a named secret is missing, but the actual
  password still has to be provided in the JSON request.
- In this environment, direct guest TCP from the Wasm component still fails
  under Wasmtime/WASIp2 with `Protocol not available (os error 50)`. The e2e
  suite therefore splits validation into two parts:
  component decoding/instantiation for the built Wasm artifact, and a
  GreenMail-backed native IMAP flow that exercises the same `imap-next`
  protocol path outside the guest runtime.

## Prerequisites

Install the Rust and Wasm tooling:

```sh
rustup target add wasm32-wasip2
cargo install cargo-component
```

For documentation and e2e work you also need:

```sh
cargo install mdformat-cli
npm install --global markdownlint-cli2
docker --version
```

`make e2e` uses the Docker-compatible CLI available on this machine. In this
environment the `docker` command is backed by Podman.

## Repository layout

The build and package flow uses these paths:

- Wasm artifact: `target/wasm32-wasip2/release/imap_tool.wasm`
- Capabilities file: `imap-tool.capabilities.json`
- Package directory: `dist/imap-tool/`

The package directory contains:

- `imap-tool.wasm`
- `imap-tool.capabilities.json`
- `README.md` copied from this guide

## Build and validation targets

The repository keeps lightweight quality gates separate from heavier Wasm and
Docker tasks.

### Core quality gates

```sh
make check-fmt
make lint
make test
```

These targets cover formatting, Clippy, Rustdoc generation, unit tests, and the
`rstest-bdd` behavioural scenarios.

### Wasm build and packaging

```sh
make wasm
make package
```

`make wasm` produces the release Wasm component with:

```sh
cargo rustc --lib --target wasm32-wasip2 --release --crate-type=cdylib
```

`make package` copies the build output plus the capabilities sidecar into
`dist/imap-tool/`.

### End-to-end test target

```sh
make e2e
```

This target:

1. Builds the Wasm artifact.
2. Runs the ignored component-instantiation test that decodes the artifact with
   `wit-component`, instantiates it with Wasmtime, and verifies exported
   metadata.
3. Runs the ignored GreenMail-backed native IMAP test.
4. Starts `greenmail/standalone` in Docker. In this Podman-backed environment
   the test uses the fully qualified image name
   `docker.io/greenmail/standalone` to avoid short-name prompts.
5. Seeds GreenMail through SMTP, allowing GreenMail to auto-create the test
   account documented in its FAQ.
6. Exercises real IMAP flows against GreenMail through the native
   `NetworkImapService`.

## Capabilities

The sidecar file `imap-tool.capabilities.json` currently requests only secret
name checks:

```json
{
  "capabilities": {
    "secrets": {
      "allowed_names": ["imap_password", "imap_*"]
    }
  }
}
```

No HTTP, workspace-read, or tool-invocation capability is required for the
current action set.

## Request schema

Every request is a JSON object passed to the exported `execute` function. The
shared connection fields are:

- `host`: IMAP server hostname or IP address
- `port`: optional IMAP port, default `143`
- `username`: LOGIN username
- `password`: LOGIN password
- `password_secret_name`: optional secret name checked with `secret-exists`

Action-specific fields are:

- `list_mailboxes`: no extra fields
- `list_messages`: optional `mailbox`, optional `sequence_set`
- `get_message`: optional `mailbox`, required `sequence`
- `mark_seen`: optional `mailbox`, required `sequence`

Defaults:

- `mailbox`: `INBOX`
- `sequence_set`: `1:*`

## Usage examples

### List mailboxes

```json
{
  "action": "list_mailboxes",
  "host": "127.0.0.1",
  "port": 143,
  "username": "alice",
  "password": "secret",
  "password_secret_name": "imap_password"
}
```

Typical success output:

```json
{
  "action": "list_mailboxes",
  "mailboxes": [
    {
      "name": "INBOX",
      "delimiter": "/",
      "attributes": ["\\Unmarked"]
    }
  ]
}
```

### List messages

```json
{
  "action": "list_messages",
  "host": "127.0.0.1",
  "port": 143,
  "username": "alice",
  "password": "secret",
  "password_secret_name": "imap_password",
  "mailbox": "INBOX",
  "sequence_set": "1:*"
}
```

### Fetch one message

```json
{
  "action": "get_message",
  "host": "127.0.0.1",
  "port": 143,
  "username": "alice",
  "password": "secret",
  "password_secret_name": "imap_password",
  "mailbox": "INBOX",
  "sequence": 1
}
```

### Mark a message as seen

```json
{
  "action": "mark_seen",
  "host": "127.0.0.1",
  "port": 143,
  "username": "alice",
  "password": "secret",
  "password_secret_name": "imap_password",
  "mailbox": "INBOX",
  "sequence": 1
}
```

## Installing into an Ironclaw tool directory

After packaging, copy the tool into the directory your Ironclaw host reads. The
neighbouring Ironclaw examples typically use `~/.ironclaw/tools/`.

```sh
make package
cp dist/imap-tool/imap-tool.wasm ~/.ironclaw/tools/imap-tool.wasm
cp dist/imap-tool/imap-tool.capabilities.json ~/.ironclaw/tools/
```

If your host expects a different filename, keep the Wasm and capabilities files
aligned as a pair.

## Testing strategy

The repository uses three layers of validation:

- Unit tests with `rstest` for request parsing and execution wiring
- Behavioural tests with `rstest-bdd` for user-visible tool behaviour
- Ignored end-to-end tests for GreenMail-backed Wasm execution

Run the lighter suite during ordinary development:

```sh
make test
```

Run the container-backed suite before release or when changing IMAP transport
behaviour:

```sh
make e2e
```

## Troubleshooting

### `make wasm` fails

Check that the Wasm target is installed:

```sh
rustup target list --installed | grep wasm32-wasip2
```

### `make e2e` fails to start GreenMail

Confirm the Docker-compatible CLI can pull and start containers:

```sh
docker pull docker.io/greenmail/standalone
docker ps
```

### The tool reports a missing secret

If you set `password_secret_name`, the host must report that secret as present.
Remember that this is only an early guard. The JSON request still needs the
real `password` value for the current socket-based workflow.

### The IMAP login succeeds but no messages appear

The GreenMail e2e path seeds mail through SMTP before IMAP reads begin. If you
are testing against another server, confirm that messages were delivered into
the selected mailbox and that you are querying the expected sequence set.

### The Wasm component cannot open TCP sockets under Wasmtime

The current `make e2e` flow already captures this limitation and still verifies
the component artifact plus the native IMAP protocol layer separately. If you
are investigating guest-side socket support specifically, expect current
Wasmtime/WASIp2 execution in this environment to fail with
`Protocol not available (os error 50)`.
