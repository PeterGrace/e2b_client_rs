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
| `just test-live <target>` | Live tests against `hosted` E2B or `sandbawks` (see below) |

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
- Live tests (`tests/live_*.rs`) are `#[ignore]`d. Each creates real
  sandboxes, killed on drop and swept at the end of the run. They read keys
  from the environment or the nearest `.env` (the main checkout's `.env` is
  found from any worktree):

  ```sh
  # .env
  E2B_API_KEY_HOSTED=...
  E2B_API_KEY_SANDBAWKS=...
  E2B_DOMAIN_SANDBAWKS=...
  ```

  Then run `just test-live hosted` or `just test-live sandbawks`. Each test is
  named `clm_NNNN_*` after the claim in `claims.json` it backs, and the run log
  in `target/live-logs/` is the evidence for that claim.

### Testing against a self-hosted E2B-compatible server

The `sandbawks` target works for any E2B-compatible server whose API answers
at `https://api.<domain>` and whose per-sandbox envd hosts
(`49983-<id>.<domain>`) resolve through wildcard DNS for that domain:

```sh
# .env
E2B_API_KEY_SANDBAWKS=...
E2B_DOMAIN_SANDBAWKS=<server-domain>
E2B_TEMPLATE_SANDBAWKS=<template-on-that-server>   # optional, default `base`
```

Then run `just test-live sandbawks`. The live harness refuses to run while the
standard `E2B_API_KEY`, `E2B_API_URL`, `E2B_DOMAIN` or `E2B_SANDBOX_URL`
variables are set, so a run cannot silently talk to the wrong server. It does
not support a single shared envd endpoint (`E2B_SANDBOX_URL`); use the client's
`ConnectionConfig::builder().sandbox_url(..)` directly for such a server.
