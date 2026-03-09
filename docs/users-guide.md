# JMAP Tool User's Guide

## Overview

`jmap-tool` is an Ironclaw-compatible WebAssembly tool that talks to mail
servers over JMAP using Ironclaw's host-provided HTTP bridge. It implements the
`near:agent/tool@0.3.0` `sandboxed-tool` world and exposes four actions:

- `list_mailboxes`
- `list_messages`
- `get_message`
- `mark_seen`

The Wasm tool crate handles Ironclaw bindings, request parsing, and response
mapping. The reusable transport-agnostic JMAP codec lives in
`crates/jmap-codec/` and contains the JMAP session, envelope, and mail method
types shared by the tool transport layer.

## Why JMAP and not IMAP

Ironclaw exposes outbound network access to Wasm tools through the host
`http-request` capability. That fits JMAP directly because JMAP runs over
HTTP(S). It does not fit raw IMAP well because IMAP requires guest-managed TCP
connections, which are not available in the runtime used for this repository.

## Prerequisites

Install the Rust and Wasm tooling:

```sh
rustup target add wasm32-wasip2
```

For documentation and local validation work you also need:

```sh
cargo install mdformat-cli
npm install --global markdownlint-cli2
```

`make e2e` uses an in-process `rusmes-jmap` server. It does not require Docker.

## Repository layout

The build and package flow uses these paths:

- Wasm artifact: `target/wasm32-wasip2/release/jmap_tool.wasm`
- Capabilities file: `jmap-tool.capabilities.json`
- Package directory: `dist/jmap-tool/`
- Web UI bundle: `dist/jmap-tool-wasm32-wasip2.tar.gz`
- Reusable codec crate: `crates/jmap-codec/`

The package directory contains:

- `jmap-tool.wasm`
- `jmap-tool.capabilities.json`
- `README.md` copied from this guide

The `.tar.gz` bundle contains the same three files at the archive root. This
matches Ironclaw's custom extension installer, which scans the archive for
`jmap-tool.wasm` and `jmap-tool.capabilities.json` by basename.

## Build and validation targets

The repository keeps the fast code-quality gates separate from Wasm packaging
and ignored end-to-end tests.

### Core quality gates

```sh
make check-fmt
make lint
make test
```

These targets cover Rust formatting, Clippy, Rustdoc generation, unit tests,
and the `rstest-bdd` behavioural scenarios.

### Wasm build and packaging

```sh
make wasm
make package
```

`make wasm` produces the release Wasm component with:

```sh
cargo rustc --lib --target wasm32-wasip2 --release --crate-type=cdylib
```

`make package` copies the built component, the capabilities sidecar, and this
guide into `dist/jmap-tool/`, then creates
`dist/jmap-tool-wasm32-wasip2.tar.gz` for the Ironclaw web UI installer.

### End-to-end test target

```sh
make e2e
```

This target:

1. Builds the Wasm artifact.
2. Runs the ignored component-instantiation test that decodes the artifact with
   `wit-component`, instantiates it with Wasmtime, and verifies the exported
   schema and description.
3. Runs the ignored native JMAP service test backed by an in-process
   `rusmes-jmap` Axum server.

## Capabilities and authentication

The sidecar file `jmap-tool.capabilities.json` requests:

- `http` access for `GET` and `POST` to the configured JMAP host
- a bearer-token credential mapping for `jmap_token`
- secret-name checks for `jmap_token` and `jmap_*`

The checked-in sidecar is an example:

```json
{
  "capabilities": {
    "http": {
      "allowlist": [
        {
          "host": "mail.example.com",
          "path_prefix": "/",
          "methods": ["GET", "POST"]
        }
      ]
    },
    "secrets": {
      "allowed_names": ["jmap_token", "jmap_*"]
    }
  }
}
```

Before installing the tool, update the HTTP allowlist and credential
`host_patterns` so they match your provider's hostname.

`auth_secret_name` in the request is a preflight check only. The tool calls
`secret_exists` before making HTTP requests so it can fail fast if the named
secret is missing. The actual bearer token is still injected by the host
according to the capabilities file.

## Request schema

Every request is a JSON object passed to the exported `execute` function. The
shared fields are:

- `action`: one of `list_mailboxes`, `list_messages`, `get_message`, or
  `mark_seen`
- `base_url`: base URL of the JMAP service, for example
  `https://mail.example.com`
- `account_id`: optional JMAP account ID override
- `auth_secret_name`: optional secret name checked with `secret_exists`
- `timeout_ms`: optional per-request timeout, default `30000`

Action-specific fields are:

- `list_mailboxes`: no extra fields
- `list_messages`: optional `mailbox_id`, optional `mailbox_name`, optional
  `limit`, optional `position`
- `get_message`: required `email_id`
- `mark_seen`: required `email_id`

Defaults:

- `limit`: `20`
- `position`: `0`

## Usage examples

### List mailboxes

```json
{
  "action": "list_mailboxes",
  "base_url": "https://mail.example.com",
  "auth_secret_name": "jmap_token"
}
```

Typical success output:

```json
{
  "action": "list_mailboxes",
  "account_id": "acc-1",
  "mailboxes": [
    {
      "id": "mbx-1",
      "name": "Inbox",
      "role": "inbox",
      "parent_id": null,
      "sort_order": 10,
      "is_subscribed": true,
      "total_emails": 4,
      "unread_emails": 2
    }
  ]
}
```

### List messages from one mailbox

```json
{
  "action": "list_messages",
  "base_url": "https://mail.example.com",
  "auth_secret_name": "jmap_token",
  "mailbox_name": "Inbox",
  "limit": 10,
  "position": 0
}
```

Typical success output:

```json
{
  "action": "list_messages",
  "account_id": "acc-1",
  "mailbox_id": "mbx-1",
  "position": 0,
  "total": 1,
  "messages": [
    {
      "id": "email-1",
      "thread_id": "thread-1",
      "mailbox_ids": ["mbx-1"],
      "keywords": ["$seen"],
      "received_at": "2026-03-09T10:00:00Z",
      "subject": "Hello",
      "from": ["Alice <alice@example.com>"],
      "preview": "Body preview",
      "has_attachment": false
    }
  ]
}
```

### Fetch one message

```json
{
  "action": "get_message",
  "base_url": "https://mail.example.com",
  "auth_secret_name": "jmap_token",
  "email_id": "email-1"
}
```

Typical success output:

```json
{
  "action": "get_message",
  "account_id": "acc-1",
  "message": {
    "id": "email-1",
    "thread_id": "thread-1",
    "mailbox_ids": ["mbx-1"],
    "keywords": ["$seen"],
    "received_at": "2026-03-09T10:00:00Z",
    "subject": "Hello",
    "from": ["Alice <alice@example.com>"],
    "to": ["Bob <bob@example.com>"],
    "preview": "Body preview",
    "has_attachment": false,
    "text_body": "Body"
  }
}
```

### Mark a message as seen

```json
{
  "action": "mark_seen",
  "base_url": "https://mail.example.com",
  "auth_secret_name": "jmap_token",
  "email_id": "email-1"
}
```

Typical success output:

```json
{
  "action": "mark_seen",
  "account_id": "acc-1",
  "email_id": "email-1",
  "seen": true,
  "keywords": ["$seen"]
}
```

## End-to-end test behaviour

The e2e story is split into two ignored tests:

- a Wasm artifact test that checks the built component shape and instantiates it
  with Wasmtime
- a native service test that exercises the same JMAP request flow against a
  local `rusmes-jmap` server

That split is intentional. It gives honest coverage for the Wasm component and
the JMAP transport path without pretending that the in-process mock server is a
full provider implementation.

Current `rusmes-jmap` limitations observed in this repository:

- the session document advertises a fixed `https://jmap.example.com` base URL,
  so the native test host rewrites the session response to the real local
  listener before handing it to the service layer
- `Email/query` does not currently return the seeded filesystem-backed message
  set reliably in this harness, so `list_messages` is covered by unit and
  behavioural tests rather than by the native `rusmes-jmap` e2e
- `Email/get` can round-trip a seeded message ID in this harness, but it does
  not currently populate the richer subject and text-body projection reliably,
  so those fields are validated by unit and behavioural tests rather than by
  the native `rusmes-jmap` e2e
- `Email/set` is not implemented, so the `mark_seen` native e2e currently
  asserts that the server returns a `notImplemented` failure

If you need a more complete JMAP integration target, the project plan records
`cyrusimap/cyrus-docker-test-server` as a heavier fallback.

## Installing into an Ironclaw tool directory

After packaging, copy the contents of `dist/jmap-tool/` into the directory your
Ironclaw host reads for tools. The neighbouring Ironclaw examples typically use
`~/.ironclaw/tools/`.

Ensure that:

- the Wasm file and capabilities sidecar keep the same basename
- the capabilities file allowlists your real JMAP host
- the referenced bearer-token secret exists in the Ironclaw host

Example:

```sh
make package
cp dist/jmap-tool/jmap-tool.wasm ~/.ironclaw/tools/jmap-tool.wasm
cp dist/jmap-tool/jmap-tool.capabilities.json ~/.ironclaw/tools/
```

For the Ironclaw web UI custom installer, host or upload:

- `dist/jmap-tool-wasm32-wasip2.tar.gz`

and enter the extension name as `jmap-tool`. The archive must contain
`jmap-tool.wasm` and `jmap-tool.capabilities.json`.
