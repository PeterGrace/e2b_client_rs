# M4 Task 5: Filesystem options and per-call request timeouts (issue #93)

Plan: `docs/superpowers/plans/2026-10-28-milestone-4-api-stability.md`, Task 5.
Claim: CLM-0173 (S3); completes the per-call part of CLM-0172, whose S4
evidence-map row is now `yes`.

## What changed

- `FileOptions { user, request_timeout }` and `ListOptions { depth, user,
  request_timeout }` (depth defaults to 1, the SDK's default), re-exported
  from the crate root. Every filesystem call now ends with an options
  reference.
- `request_timeout` on `WriteOptions` (each in-memory upload request; the
  whole transfer for `write_stream`) and `WatchOptions` (opening the watch,
  up to envd's start event).
- `src/config.rs`: the crate-private `RequestTimeout`, one per options
  value. Unset is the configured request timeout; `None` or zero (through
  `no_limit_if_zero`) is no limit; a duration is that duration.
- `src/envd/mod.rs`: `EnvdClient::timeout_for(RequestTimeout)` replaces
  `request_timeout()` and is the one place the value resolves.
  `unary`, `within_request_timeout` and `http_request` take a
  `RequestTimeout`; `server_stream`, `client_stream` and
  `http_request_with_timeout` are unchanged.
- Commands, Pty and `Sandbox::is_running` pass `RequestTimeout::default()`
  (the configured timeout) until Task 7; `Sandbox::mcp_token` reads its
  token with `FileOptions::new().user("root")`.
- `read_stream` and `write_stream` resolve the per-call value against "no
  limit", so they stay unbounded unless the caller sets one.
- `Filesystem::write` passes its `WriteOptions` through to `write_files`,
  so octet-stream, gzip and metadata now apply to it.
- Tests: `tests/envd_file_options.rs` (7), the `src/config.rs` unit test
  `request_timeout_resolves_unset_none_zero_and_a_value`, and every caller in
  `src/`, `tests/`, `README.md` and the doctests converted.
- Docs: rustdoc of every changed call and setter, a `docs/protocol-notes.md`
  row for the per-call request timeout, the README Files row and examples,
  the CLM-0172 evidence-map row, and a Corrected note in the plan's Task 5.

## Breaking changes

| Old | New |
|---|---|
| `files.read(p, None)` (also `read_to_string`, `read_stream`, `get_info`, `exists`, `make_dir`, `remove`) | `files.read(p, &FileOptions::default())` |
| `files.read(p, Some("root"))` (and the same calls) | `files.read(p, &FileOptions::new().user("root"))` |
| `files.rename(a, b, None)` / `files.rename(a, b, Some("root"))` | `files.rename(a, b, &FileOptions::default())` / `files.rename(a, b, &FileOptions::new().user("root"))` |
| `files.write(p, data, None)` | `files.write(p, data, &WriteOptions::default())` |
| `files.write(p, data, Some("root"))` | `files.write(p, data, &WriteOptions::default().user("root"))`; `write` now honours every `WriteOptions` field |
| `files.list(p, 1, None)` | `files.list(p, &ListOptions::default())` |
| `files.list(p, depth, Some("root"))` | `files.list(p, &ListOptions::new().depth(depth).user("root"))` |

`write_files`, `write_stream` and `watch_dir` keep their shape; their
options gain `request_timeout`.

## Live check (hosted E2B)

`cargo test --test live_files -- --ignored --test-threads=1`: 19/19 passed.
`cargo test --test live_sweep -- --ignored` with the same run id: 3/3
passed.
