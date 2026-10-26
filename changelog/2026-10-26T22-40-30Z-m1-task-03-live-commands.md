# M1 Task 3: live command tests (S2)

Issue #7, Task 3 of `docs/superpowers/plans/2026-10-26-milestone-1-verify-v0.1.md`.

## What changed

- `tests/live_commands.rs`: one `#[ignore]`d live test per S2 v0.1 command
  claim, CLM-0034 to CLM-0045, next to the existing `clm_0033`:

| Test | Claim |
|---|---|
| `clm_0034_start_streams_chunks_then_exit` | `next_event` yields Stdout and Stderr chunks as produced, then Exit |
| `clm_0035_run_delivers_large_output_in_order` | `run` delivers at least 1 MiB complete and in order |
| `clm_0036_options_env_is_set` | `env` entries reach the command's environment |
| `clm_0037_options_cwd_is_working_directory` | `cwd` sets the working directory |
| `clm_0038_options_user_runs_as_user` | `user` runs the command as that user |
| `clm_0039_options_timeout_ends_long_command` | `timeout` ends the wait with `Error::Timeout` or `deadline_exceeded` |
| `clm_0040_connect_attaches_to_running_command` | `connect(pid)` receives later output and the exit code |
| `clm_0041_list_includes_background_command` | `list` shows a background command's pid and command line |
| `clm_0042_send_stdin_reaches_command` | `send_stdin` reaches a command started with `stdin(true)` |
| `clm_0043_close_stdin_sends_eof` | `close_stdin` delivers EOF |
| `clm_0044_sigterm_terminates_command` | `send_signal(Sigterm)` terminates the command |
| `clm_0045_kill_reports_whether_pid_existed` | `kill` is true for a running pid, false for an unknown one |

## Assertions beyond the plan

- CLM-0034 checks timing as well as content. The command sleeps 2 s between its
  stdout and stderr lines. The test asserts the first Stdout chunk arrives
  before the Stderr chunk and at least 1 s before Exit, which a client that
  buffered output until exit would fail.
- CLM-0035 and CLM-0043 also assert exit code 0.
- CLM-0044 also asserts the pid has left `Commands::list` once the wait returns.

## Live run (hosted E2B, run `20261026T223918Z-hosted-459680`)

All 13 tests in `live_commands` passed on the first run (5.06 s with
`--test-threads=4`). The sweep reported
`killed 0 leftover sandbox(es) of run 20261026T223918Z-hosted-459680`.

## Plan corrections

- Task 3: a dated note says the plan code compiled as written and lists the
  added assertions above. No symbol changed, so later tasks are unaffected.
