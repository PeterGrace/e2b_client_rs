# M4 Task 7: connect options for commands and PTYs, and request timeouts (issue #54)

Plan: `docs/superpowers/plans/2026-10-28-milestone-4-api-stability.md`, Task 7.
Claim: CLM-0174 (zero as "no limit" is CLM-0172, from Task 4).

`Commands::connect` and `Pty::connect` took the stream deadline as a bare
`Option<Duration>`, so `connect` had no default (the SDKs use 60 s) and any
new parameter would have been another breaking change. They now take options
structs (decision 3), and every Commands/PTY call that takes options gains the
SDKs' per-call `request_timeout`.

## What changed

- `src/commands.rs`:
  - `CommandConnectOptions` (`new()`, `Default`, `timeout`,
    `request_timeout`), defaulting to a 60 s stream deadline
    (`DEFAULT_COMMAND_TIMEOUT`) and the configured request timeout, as SDK
    `commands.connect(pid, timeout=60, request_timeout=None)`.
  - `CommandOptions::request_timeout`.
  - `Commands::connect(pid, &CommandConnectOptions)`, with an example.
  - The crate-private `StreamTimeouts { stream, request }` carries both bounds
    into `open_process_stream`, which keeps it at five parameters. The
    request half replaces the `RequestTimeout::default()` that Task 5 left at
    the start/connect/create call site; `EnvdClient::timeout_for` resolves it.
  - `connect` no longer calls `no_limit_if_zero` itself: the `timeout` setter
    does.
- `src/pty.rs`: `PtyConnectOptions` (same shape; 60 s for now, Task 8 changes
  the PTY defaults), `PtyOptions::request_timeout`,
  `Pty::connect(pid, &PtyConnectOptions)` with an example.
- `src/lib.rs`: re-exports `CommandConnectOptions` and `PtyConnectOptions`.
- `docs/protocol-notes.md`: the Commands rows (per-call request timeout, the
  60 s connect default) and the PTY rows.
- The plan's Task 7 has a Corrected note (test imports, the exact-limit
  assertions, the staged files); Task 11's import note is corrected too.

Pid-only calls (`send_stdin`, `kill`, `resize`, `list`, ...) still use the
configured request timeout; per-call timeouts there are out of scope.

## Breaking changes

| Old | New |
|---|---|
| `commands.connect(pid, Some(d))` | `commands.connect(pid, &CommandConnectOptions::new().timeout(Some(d)))` |
| `commands.connect(pid, None)` | `commands.connect(pid, &CommandConnectOptions::new().timeout(None))` (`&CommandConnectOptions::default()` is now 60 s, the SDK's value) |
| `pty.connect(pid, t)` | `pty.connect(pid, &PtyConnectOptions::new().timeout(t))` |
| `CommandOptions { .. }`, `PtyOptions { .. }` | Unchanged for callers (private fields); each gains a `request_timeout` setter |

## Tests

- New `tests/envd_connect_options.rs` (10 tests): the default (60 000 ms),
  chosen (5 000 ms) and absent `Connect-Timeout-Ms` on `Commands::connect`;
  the PTY default; a shorter per-call request timeout bounds the wait for the
  start event (`Error::Timeout(200 ms)`); a longer one overrides the
  configured 200 ms; zero lifts it; `CommandOptions::request_timeout` bounds
  `start` and `PtyOptions::request_timeout` bounds `create`.
  `the_stream_deadline_is_not_the_request_timeout` only shows the two values
  travel separately and is not evidence for CLM-0174.
- Unit tests: `commands::tests::connect_options_default_to_the_sdk_deadline`,
  `commands::tests::a_zero_connect_deadline_means_no_limit`,
  `pty::tests::connect_options_default_to_the_sdk_deadline`.
- Callers updated in `tests/envd_pty.rs`, `tests/envd_api.rs`,
  `tests/zero_timeouts.rs` and `tests/live_commands.rs` (`clm_0040`).

## Live check (hosted E2B)

`clm_0040_connect_attaches_to_running_command`, run
`dev-20261028T135655Z-3542314`: passed (1 passed, 5.16 s). No live test calls
`Pty::connect`.

Closes #54.
