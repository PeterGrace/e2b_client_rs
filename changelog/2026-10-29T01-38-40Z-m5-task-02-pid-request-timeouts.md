# M5 Task 2: Per-call request timeouts on pid-only process and PTY calls (issue #120)

Plan: `docs/superpowers/plans/2026-10-28-milestone-5-release.md`, Task 2.
Closes #120 (Task 1, #125, did the REST half). Backs CLM-0183.

## What changed

- `src/commands.rs`: `Commands::{list, send_stdin, close_stdin,
  send_signal, kill}` take a trailing `&RequestOptions` and pass its
  `request_timeout` to `EnvdClient::unary` in place of
  `RequestTimeout::default()`. `kill` hands its options to `send_signal`;
  `close_stdin` still checks the envd version first and sends nothing on
  an old one. `CommandHandle::{send_stdin, kill}` take and pass them.
- `src/pty.rs`: `Pty::{send_stdin, resize, kill}` and
  `PtyHandle::{send_stdin, resize, kill}` the same; `Pty::kill` goes
  through `Commands::kill` with the caller's options.
- Tests: `tests/envd_request_options.rs` (new: every pid-only call gives up
  on its own 200 ms against a 5 s configured timeout; a per-call `None`
  lifts a 200 ms configured timeout, and unset keeps it); the live test
  `clm_0183_a_one_millisecond_request_timeout_times_out_list`; every caller
  in `tests/envd_api.rs`, `tests/envd_pty.rs`, `tests/error_taxonomy.rs`
  and `tests/live_commands.rs`.
- Docs: rustdoc (an `options` Arguments line and examples) on every
  changed method, and the examples in `src/stream_input.rs` and the
  `Signal`, `ProcessInfo` and `Pty` type docs; the crate docs' options
  paragraph now names the pid-only calls; README (the streaming tour, the
  Processes and PTY rows, the "Not implemented yet" sentence loses its
  timeout clause); `docs/api-review.md` (the `RequestOptions` row and
  "Method shape" widened; "Known limits" loses the request-timeout bullet
  and gains one for the SDK's control-plane `retries`);
  `docs/protocol-notes.md` (a per-call timeout row for the process calls
  and one for the PTY calls, the "Not implemented yet" clause removed).
- The plan: two dated corrections under Task 2 (the callers that actually
  changed, and the live tests to run: `clm_0041` to `clm_0048`,
  `clm_0175` and `clm_0178`, not `clm_0035` to `clm_0040`).

## Breaking changes

| Old | New |
|---|---|
| `commands.list()` | `commands.list(&RequestOptions::new())` |
| `commands.send_stdin(pid, data)` | `commands.send_stdin(pid, data, &RequestOptions::new())` |
| `commands.close_stdin(pid)` | `commands.close_stdin(pid, &RequestOptions::new())` |
| `commands.send_signal(pid, signal)` | `commands.send_signal(pid, signal, &RequestOptions::new())` |
| `commands.kill(pid)` | `commands.kill(pid, &RequestOptions::new())` |
| `handle.send_stdin(data)` (`CommandHandle`) | `handle.send_stdin(data, &RequestOptions::new())` |
| `handle.kill()` (`CommandHandle`) | `handle.kill(&RequestOptions::new())` |
| `pty.send_stdin(pid, data)` | `pty.send_stdin(pid, data, &RequestOptions::new())` |
| `pty.resize(pid, size)` | `pty.resize(pid, size, &RequestOptions::new())` |
| `pty.kill(pid)` | `pty.kill(pid, &RequestOptions::new())` |
| `handle.send_stdin(data)` (`PtyHandle`) | `handle.send_stdin(data, &RequestOptions::new())` |
| `handle.resize(size)` (`PtyHandle`) | `handle.resize(size, &RequestOptions::new())` |
| `handle.kill()` (`PtyHandle`) | `handle.kill(&RequestOptions::new())` |

`stream_input` keeps its `&StreamInputOptions` (its bound is a stream
deadline, not a request timeout).

## Verification

- `just lint`, `just test` (521 passed, 0 failed, 84 ignored), `just doc`:
  green.
- Hosted E2B, one at a time (2026-10-29T01:38Z to 01:40Z, run ids
  `dev-20261029T0138..Z-2786141`): `clm_0183`, `clm_0041` to `clm_0048`,
  `clm_0175` and `clm_0178` pass.

## Why

The SDK takes `request_timeout` on `commands.list`, `kill`, `send_stdin`
and `close_stdin` (`e2b/sandbox_async/commands/command.py:64-71, 99-102,
120-124, 176-179`) and on `pty.kill`, `send_stdin` and `resize`
(`e2b/sandbox_async/commands/pty.py:65-68, 85-89, 256-260`); here they used
the configured timeout only. Decision 5 gives every call with nothing else
to configure a trailing `&RequestOptions`, including `send_signal` and the
handle methods (the SDK's `CommandHandle.kill` takes none), for one rule
everywhere.
