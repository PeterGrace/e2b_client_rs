# M4 Task 11: Investigate Pty::connect to an exited pid (issue #56)

Plan: `docs/superpowers/plans/2026-10-28-milestone-4-api-stability.md`, Task 11.
Claim: CLM-0178.

## Conclusion: branch (b)

The M2 finding reproduces, and it is envd's behaviour, not the crate's. On
hosted E2B, `Pty::connect` to the pid of a shell whose exit event has
already arrived returns `Ok`, and the first `next_event` is that shell's
`PtyEvent::Exit` with its exit code. envd confirms the attach with a `start`
event and then replays the end event. This held straight after the exit and
5 s later on every run, and also at 8, 10, 12, 15 and 20 s. By 30 s envd
answers `not_found`, the same as for a pid that never existed. The Python
SDK 2.61.0 behaves the same way (`pty.connect` succeeds, `wait()` returns
the exit code), so the crate matches the SDK and there is nothing to fix.
The rustdoc was what was wrong.

`Commands::connect` shows the same behaviour (one probe run, below), so its
rustdoc gets the same correction, as the plan's branch (b) asks. It has no
live test of its own; the probe run is the evidence.

## What changed

- `Pty::connect` rustdoc, `# Errors`: `not_found` when envd no longer knows
  the process; envd keeps an exited shell attachable for a short time
  (tens of seconds on hosted E2B), and the stream's first event is its
  `PtyEvent::Exit`, as with the Python SDK's `pty.connect`. The exact
  window (attachable at 20 s, `not_found` by 30 s, one probe run) is in
  `docs/protocol-notes.md`, not the rustdoc, because it is envd behaviour
  rather than a crate guarantee (review on PR #111).
- `Commands::connect` rustdoc, `# Errors`: the same correction, citing the
  probe run (a `true` connected straight after its exit attached and
  replayed `CommandEvent::Exit`).
- Live test `clm_0178_pty_connect_to_exited_pid` in `tests/live_commands.rs`
  (and `PtyConnectOptions` added to that file's import). It pins the
  observed behaviour: the shell exits with code 3, then `connect` at once
  and after 5 s yields `Attached(Ok(Some(Exit(exit_code 3))))`, and a
  `connect` to pid 999999, which never existed, is refused with
  `not_found`. The exit code 3 shows the event is the shell's own exit.
- `docs/protocol-notes.md`: a PTY row for `Connect` to an exited pid, and
  a Process service row for the same with a command.
- The plan's Task 11: a dated Corrected note for how the test differs from
  Step 1 and Step 4 (b).

## Breaking changes

None.

## Reproduction log (hosted E2B)

The plan's Step 1 test, asserting the documented `not_found`. Attempts 1 to
3 ran the plan's assertion loop, which stops at the first failing case, so
only "at once" is shown. Attempts 4 to 6 were meant to print both cases but
my edit had not applied, so they repeat attempts 1 to 3. Attempts 7 to 9
print both cases. Compiler-warning lines are omitted; everything else is
verbatim from `target/i56-logs/repro.log`.

```text
=== attempt 1 2026-10-28T14:35:13Z
thread 'clm_0178_pty_connect_to_exited_pid' (74261) panicked at tests/live_commands.rs:556:9:
at once: connect to an exited pid gave Attached(Ok(Some(Exit(CommandExit { exit_code: 0, error: None }))))
test clm_0178_pty_connect_to_exited_pid ... FAILED
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 18 filtered out; finished in 6.72s
=== attempt 2 2026-10-28T14:35:19Z
thread 'clm_0178_pty_connect_to_exited_pid' (74698) panicked at tests/live_commands.rs:556:9:
at once: connect to an exited pid gave Attached(Ok(Some(Exit(CommandExit { exit_code: 0, error: None }))))
test clm_0178_pty_connect_to_exited_pid ... FAILED
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 18 filtered out; finished in 6.16s
=== attempt 3 2026-10-28T14:35:26Z
thread 'clm_0178_pty_connect_to_exited_pid' (75065) panicked at tests/live_commands.rs:556:9:
at once: connect to an exited pid gave Attached(Ok(Some(Exit(CommandExit { exit_code: 0, error: None }))))
test clm_0178_pty_connect_to_exited_pid ... FAILED
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 18 filtered out; finished in 6.30s
=== attempt 4 (same code as 1-3; the message edit had not applied) 2026-10-28T14:35:42Z
thread 'clm_0178_pty_connect_to_exited_pid' (76157) panicked at tests/live_commands.rs:556:9:
at once: connect to an exited pid gave Attached(Ok(Some(Exit(CommandExit { exit_code: 0, error: None }))))
test clm_0178_pty_connect_to_exited_pid ... FAILED
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 18 filtered out; finished in 6.14s
=== attempt 5 (same code as 1-3; the message edit had not applied) 2026-10-28T14:35:49Z
thread 'clm_0178_pty_connect_to_exited_pid' (76638) panicked at tests/live_commands.rs:556:9:
at once: connect to an exited pid gave Attached(Ok(Some(Exit(CommandExit { exit_code: 0, error: None }))))
test clm_0178_pty_connect_to_exited_pid ... FAILED
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 18 filtered out; finished in 5.92s
=== attempt 6 (same code as 1-3; the message edit had not applied) 2026-10-28T14:35:55Z
thread 'clm_0178_pty_connect_to_exited_pid' (77000) panicked at tests/live_commands.rs:556:9:
at once: connect to an exited pid gave Attached(Ok(Some(Exit(CommandExit { exit_code: 0, error: None }))))
test clm_0178_pty_connect_to_exited_pid ... FAILED
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 18 filtered out; finished in 6.14s
=== attempt 7 (message shows both cases) 2026-10-28T14:36:15Z
thread 'clm_0178_pty_connect_to_exited_pid' (79346) panicked at tests/live_commands.rs:564:5:
connect to an exited pid gave, at once: Attached(Ok(Some(Exit(CommandExit { exit_code: 0, error: None })))); after 5 s: Attached(Ok(Some(Exit(CommandExit { exit_code: 0, error: None }))))
test clm_0178_pty_connect_to_exited_pid ... FAILED
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 18 filtered out; finished in 6.30s
=== attempt 8 (message shows both cases) 2026-10-28T14:36:22Z
thread 'clm_0178_pty_connect_to_exited_pid' (80071) panicked at tests/live_commands.rs:564:5:
connect to an exited pid gave, at once: Attached(Ok(Some(Exit(CommandExit { exit_code: 0, error: None })))); after 5 s: Attached(Ok(Some(Exit(CommandExit { exit_code: 0, error: None }))))
test clm_0178_pty_connect_to_exited_pid ... FAILED
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 18 filtered out; finished in 5.91s
=== attempt 9 (message shows both cases) 2026-10-28T14:36:28Z
thread 'clm_0178_pty_connect_to_exited_pid' (80771) panicked at tests/live_commands.rs:564:5:
connect to an exited pid gave, at once: Attached(Ok(Some(Exit(CommandExit { exit_code: 0, error: None })))); after 5 s: Attached(Ok(Some(Exit(CommandExit { exit_code: 0, error: None }))))
test clm_0178_pty_connect_to_exited_pid ... FAILED
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 18 filtered out; finished in 6.28s
```

The probes below were temporary tests, not committed (verbatim from
`target/i56-logs/probe.log`). The first starts `true` with
`Commands::start`, waits for it, then `Commands::connect`s to its pid. It
then connects a PTY to a pid that never existed, and to an exited shell's
pid after 30 s. The second connects to an exited shell's pid at 8, 10, 12,
15 and 20 s (those shells exit with code 0).

```text
=== probe (temporary test, not committed) 2026-10-28T14:37:19Z
running 1 test
probe: true exited 0
probe: commands connect at once: Attached(Ok(Some(Exit(CommandExit { exit_code: 0, error: None }))))
probe: pty connect to never-used pid 999999: Err(Rpc { code: NotFound, message: "process with pid 999999 not found" })
probe: pty after 30 s: Refused(Rpc { code: NotFound, message: "process with pid 1552 not found" })
test i56_probe ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 19 filtered out; finished in 30.95s
=== probe window (temporary test, not committed) 2026-10-28T14:38:04Z
running 1 test
probe: pty after 8 s: Attached(Ok(Some(Exit(CommandExit { exit_code: 0, error: None }))))
probe: pty after 10 s: Attached(Ok(Some(Exit(CommandExit { exit_code: 0, error: None }))))
probe: pty after 12 s: Attached(Ok(Some(Exit(CommandExit { exit_code: 0, error: None }))))
probe: pty after 15 s: Attached(Ok(Some(Exit(CommandExit { exit_code: 0, error: None }))))
test i56_probe_window has been running for over 60 seconds
probe: pty after 20 s: Attached(Ok(Some(Exit(CommandExit { exit_code: 0, error: None }))))
test i56_probe_window ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 19 filtered out; finished in 67.02s
```

The pinned test, three runs (from `target/i56-logs/pinned.log`):

```text
=== pinned run 1 2026-10-28T14:40:35Z
running 1 test
test clm_0178_pty_connect_to_exited_pid ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 18 filtered out; finished in 6.27s
=== pinned run 2 2026-10-28T14:40:42Z
running 1 test
test clm_0178_pty_connect_to_exited_pid ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 18 filtered out; finished in 5.99s
=== pinned run 3 2026-10-28T14:40:48Z
running 1 test
test clm_0178_pty_connect_to_exited_pid ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 18 filtered out; finished in 6.06s
```

## SDK comparison (e2b 2.61.0)

The plan's Step 3 script (`target/i56-logs/sdk_pty_connect.py`, not
committed; it prints the version through `importlib.metadata` and the
sandbox id), run three times with
`uv run --no-project --with e2b==2.61.0 python -I` from outside the repo.
Each run kills its sandbox. Verbatim from `target/i56-logs/sdk.log`:

```text
=== sdk run 1 2026-10-28T14:36:43Z
Downloading pygments (1.2MiB)
 Downloaded pygments
Installed 28 packages in 34ms
e2b 2.61.0
sandbox igp62yuh1qk97m1as6p2x
at once: connect ok; wait -> exit_code=0
after 5 s: connect ok; wait -> exit_code=0
killed
=== sdk run 2 2026-10-28T14:36:50Z
e2b 2.61.0
sandbox issst9cnlpk7hu3sdfc3i
at once: connect ok; wait -> exit_code=0
after 5 s: connect ok; wait -> exit_code=0
killed
=== sdk run 3 2026-10-28T14:36:57Z
e2b 2.61.0
sandbox i4mfyfsrhi17vm29rdqig
at once: connect ok; wait -> exit_code=0
after 5 s: connect ok; wait -> exit_code=0
killed
```

## Cleanup

Every Rust live sandbox is a `LiveSandbox`, killed on drop, and each SDK run
kills its own. `cargo test --test live_sweep -- --ignored` (run
`dev-20261028T144102Z-120552`) killed 0 sandboxes, 0 snapshots and 0
volumes. Each attempt above had its own run id, so I also listed running and
paused hosted sandboxes tagged by these runs or the SDK script: none were
left.
