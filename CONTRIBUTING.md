# Contributing

## Tooling

You need a stable Rust toolchain (with `rustfmt` and `clippy`) and
[`just`](https://github.com/casey/just).

| Command | What it does |
|---|---|
| `just fmt` | Format the code |
| `just lint` | `cargo fmt --check` plus `cargo clippy --all-targets -- -D warnings` |
| `just test` | Unit, mock-server and doc tests |
| `just doc` | Build the docs with warnings as errors |
| `just test-integration` | End-to-end tests against a real server (see below) |

CI (`.github/workflows/ci.yml`) runs `just lint`, `just test` and `just doc` on
every pull request and on `main`.

Coding rules for this repository are in [.claude/CLAUDE.md](.claude/CLAUDE.md).
Each change gets a dated entry under `changelog/`.

## Protocol changes

The ground truth for the wire protocol is the official E2B SDKs, not memory.
When you add or change a call, read the SDK source (`pip download e2b --no-deps`
and/or `npm pack e2b`) and record the fact with a file:line citation in
[docs/protocol-notes.md](docs/protocol-notes.md). Only the standard E2B API is
in scope.

## Tests

- Mock-server tests (`tests/rest_api.rs`, `tests/envd_api.rs`) use
  [`wiremock`](https://docs.rs/wiremock) to play both the REST API and envd,
  including Connect streaming envelopes, end-of-stream frames and error frames.
- Integration tests (`tests/integration.rs`) are `#[ignore]`d. They create
  real sandboxes, so run them only with credentials:

  ```sh
  E2B_API_KEY=... just test-integration
  ```

  Optional variables: `E2B_API_URL`, `E2B_DOMAIN`, `E2B_SANDBOX_URL`, and
  `E2B_TEMPLATE` (default `base`).

### Testing against a self-hosted E2B-compatible server

Point the client at the server's API and domain; per-sandbox envd hosts
(`49983-<id>.<domain>`) need wildcard DNS for that domain:

```sh
E2B_API_KEY=... \
E2B_API_URL=https://<server> \
E2B_DOMAIN=<server-domain> \
E2B_TEMPLATE=<template-on-that-server> \
just test-integration
```

If the server has no per-sandbox hosts, set `E2B_SANDBOX_URL` to the single
envd endpoint instead; the `E2b-Sandbox-Id` header tells it which sandbox is
meant.
