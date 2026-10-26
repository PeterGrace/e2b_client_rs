# M1 Task 1: live test harness

Issue #5, Task 1 of `docs/superpowers/plans/2026-10-26-milestone-1-verify-v0.1.md`.

## What changed

- `tests/live_support/mod.rs`: shared helpers for every live test binary:
  - target selection through `E2B_TEST_TARGET` (`hosted` or `sandbawks`);
  - per-target settings (`E2B_API_KEY_<TARGET>`, `E2B_DOMAIN_<TARGET>`,
    `E2B_TEMPLATE_<TARGET>`), read from the process environment first and then
    from the nearest `.env` through `dotenvy::dotenv_iter`. That call only reads
    the file and never mutates the environment;
  - a refusal to run while the standard `E2B_API_KEY`, `E2B_API_URL`,
    `E2B_DOMAIN` or `E2B_SANDBOX_URL` variables are set;
  - a run id (`E2B_TEST_RUN_ID`) written as metadata on every sandbox;
  - a `LiveSandbox` drop guard that kills the sandbox even when the test panics.
    It kills from a fresh thread with its own runtime and client;
  - the `poll_until` and `unix_now` helpers.
- `tests/live_sweep.rs`: lists the run's running and paused sandboxes by
  metadata, following pagination, and kills them.
- `tests/live_lifecycle.rs`, `tests/live_commands.rs` and `tests/live_files.rs`
  hold the three old integration tests, renamed after their claims (`clm_0014`,
  `clm_0033`, `clm_0051`). `tests/integration.rs` is deleted.
- `justfile`: `test-live <target>` replaces `test-integration`. It writes
  `target/live-logs/<run id>.log` and passes `--show-output`, so the sweep line
  and any drop-guard warnings reach the log.
- `CONTRIBUTING.md`: Tooling row and Tests section for live tests. The
  self-hosted section was rewritten around the `sandbawks` target.
- `Cargo.toml`: dev-dependency `dotenvy = "0.15.7"`.

## How to run

Put `E2B_API_KEY_HOSTED`, `E2B_API_KEY_SANDBAWKS` and `E2B_DOMAIN_SANDBAWKS` in
`.env`. Then run `just test-live hosted` or `just test-live sandbawks`.

## Smoke run (hosted E2B, run `20261026T223058Z-hosted`)

- `clm_0014_connect_resumes_paused_sandbox_with_files`: ok
- `clm_0033_run_returns_output_and_exit_code`: ok
- `clm_0051_read_to_string_returns_text`: ok
- `sweep: killed 0 leftover sandbox(es) of run 20261026T223058Z-hosted`

The sweep was also checked once by hand. A throwaway test (not committed)
leaked one tagged sandbox, and the sweep reported `killed 1`. A second sweep
then reported `killed 0`.

## Plan corrections

- Step 5: added `--show-output`, because libtest captured the sweep's
  `eprintln!` and the line never reached the log. The recipe's two comment lines
  were swapped so `just --list` shows the summary.
- Step 6: the self-hosted section of `CONTRIBUTING.md` was rewritten. It
  referred to the removed recipe and to variables the harness now refuses.
- Step 2: the `client()` loop was rewritten in rustfmt's layout.

## Review follow-ups (PR #13)

- The recipe's run id now ends in the shell's pid
  (`<UTC seconds>-<target>-<pid>`). Two runs started in the same second
  against the same target, for example from two worktrees, no longer share a
  tag, so one run's sweep can't kill the other's sandboxes.
- `just test-live` now exits non-zero when the sweep fails, even if every claim
  test passed. A failed sweep means leaked, billed sandboxes.
