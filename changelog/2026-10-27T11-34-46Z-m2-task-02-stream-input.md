# M2 Task 2: client-streaming transport and streamed stdin

Issue #31, Task 2 of `docs/superpowers/plans/2026-10-27-milestone-2-command-and-file-parity.md`.
Claims CLM-0046 (S2), CLM-0130 and CLM-0131 (S4).

## What changed

- `Commands::stream_input(pid, &StreamInputOptions) -> InputStream` opens
  envd's client-streaming `process.Process/StreamInput` call on a background
  task. The request body sends `start` with the pid, then one `data` envelope
  per chunk sent, plus a `keepalive` after 50 s without input. `send` waits
  once 64 chunks are queued. `close` ends the body and waits for envd's
  answer. The deadline defaults to 60 s (`DEFAULT_STREAM_INPUT_TIMEOUT`), and
  `.timeout(None)` removes it.
- `src/envd/mod.rs`: `client_stream` (new), and `server_stream` now shares one
  POST-and-check-status helper with it (`post_stream`). The keepalive header
  constants moved here from `commands.rs`, with a `keepalive_headers()` helper.
  `within_request_timeout` bounds a stream's opening by the request timeout,
  and `Commands::open` now uses it. Both deadlines go through one private
  `within` function.
- `src/envd/process_pb.rs`: `ProcessInput` is now the `stdin`/`pty` oneof,
  and `StreamInputRequest` is new.
- Dependencies: `futures-util` (no default features), reqwest `stream`, tokio
  `rt` and `sync`. In dev-dependencies, tokio `test-util` adds paused time.
- Tests: three mock tests in `tests/envd_stream_input.rs`, unit tests for the
  message JSON and the request body, a `decode_request_envelopes` helper, and
  the live test `clm_0046_stream_input_delivers_chunks_in_order`.
- Docs: a "Streamed input" section and a client-streaming row in
  `docs/protocol-notes.md`, and rows for CLM-0130 and CLM-0131 in the S4
  evidence map.

## Why it differs from the plan

The plan has been corrected in this same PR:

- The keepalive unit test runs on Tokio's paused clock with the real 50 s
  interval. The plan used a 20 ms interval passed as a parameter. So
  `request_messages` takes no interval argument, and the test checks that
  the keepalive comes exactly 50 s after the last chunk.
- `StreamInputOptions::timeout` is a `pub(crate)` field. The plan made it
  private, but then `commands.rs` could not read it.
- `InputKind::Pty` carries `#[expect(dead_code)]` until Task 3 uses it. The
  rustdoc links to `crate::Pty::stream_input` were left out because they
  would not resolve. Task 3's section says to undo both.
- `EnvdClient::request_timeout()` was removed, because nothing calls it once
  `Commands::open` uses `within_request_timeout`. Task 5's section says to
  re-add it for the upload code.
- CLM-0130 is mapped as `partial`. No stream sends `input.pty` until Task 3
  adds the PTY test to that row.

## Validation

- `just lint`, `just test` and `just doc` pass. Counts from `just test`: 62
  unit, 36 `envd_api`, 3 `envd_stream_input`, 19 `rest_api` and 16 doc tests
  passed. The 48 live tests are ignored.
- Live, on hosted E2B, `clm_0046_stream_input_delivers_chunks_in_order` ran
  on its own and passed (`cat` echoed `one two three\n`, exit 0).
