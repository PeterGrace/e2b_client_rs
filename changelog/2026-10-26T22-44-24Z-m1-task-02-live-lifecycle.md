# M1 Task 2: live lifecycle tests (S1)

Issue #6, Task 2 of `docs/superpowers/plans/2026-10-26-milestone-1-verify-v0.1.md`.

## What changed

- `tests/live_lifecycle.rs` now has one live test per S1 v0.1 claim, CLM-0007 to
  CLM-0027 (21 tests). CLM-0014 was already there from Task 1 and is unchanged.
  The tests cover:
  - create and its parameters: metadata, env vars, timeout, auto-pause and
    internet access;
  - connect, get_info and list (metadata filter, state filter, paging);
  - kill, pause, set_timeout, refresh and is_running;
  - the not-found cases for connect, get_info, kill and a second pause.
- Two helpers: `list_all` follows `next_token` across pages and returns every
  `SandboxInfo`, and `ids_of` maps those to ids.
- Every claim that names a `Client::*` method is tested through that method.

## Live run (hosted E2B, run `20261026T224339Z-hosted-683998`)

All 21 lifecycle tests passed, and the sweep reported `killed 0 leftover
sandbox(es)`.

The first run (`20261026T223923Z-hosted-466332`) failed only CLM-0012. A
sandbox created with `allow_internet_access(false)` completed a TCP connect to
`1.1.1.1:443`. A single-test diagnostic run (`20261026T224305Z-hosted-i6diag-655448`,
also swept, 0 leftovers) showed what was happening there:

- every TCP connect succeeded, to any address or port;
- an HTTP request over that socket got no reply;
- `curl https://one.one.one.one` timed out (exit 124).

The control sandbox got answers to both. So egress is blocked, but the host
completes the handshake anyway. That makes the probe wrong, not the crate. The
probe now sends `GET /` to `1.1.1.1:80` and needs an `HTTP/` status line back.
Both assertion messages print the full `CommandResult`.

## Plan corrections

All corrections are in the Task 2 section, under a dated
`Corrected (issue #6, 2026-10-26)` note:

- the CLM-0012 probe, also fixed inline in Step 2;
- tests call `Client::*` methods directly for CLM-0021, 0023, 0024, 0025 and
  0026;
- `list_all` returns `Vec<SandboxInfo>`, there is an `ids_of` helper, and
  CLM-0019 also checks each listed entry's `state`;
- CLM-0016 compares `template_id` with the handle's;
- CLM-0027 uses its own polling loop, so a timeout reports the last answer.

## Review follow-ups (PR #17)

- CLM-0011 fails at once with "sandbox was killed, not paused" if `get_info`
  reports not found. Before, it kept polling for 120 s.
- CLM-0021 polls `get_info` for up to 30 s until it errors, then requires
  not found, so it tolerates a brief lag after a kill.
- CLM-0025 compares `end_at` with the server's own `started_at`, not with the
  local wall clock. The allowed window is 238 s to 242 s plus the time measured
  on a monotonic clock from before create to after `set_sandbox_timeout`.
- The plan's Task 2 correction note records all three.
