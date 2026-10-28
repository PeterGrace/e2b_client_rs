# M3 Task 7: Pause and resume modes (issue #66)

Plan: `docs/superpowers/plans/2026-10-27-milestone-3-lifecycle-parity.md`, Task 7.
Claims: CLM-0146 (S1) and CLM-0159 (S4).

## What changed

- `src/lifecycle.rs`: `PauseOptions` (`mode(SnapshotMode)`), `OnResume`
  (`Restore`, `Reboot`) and `ConnectOptions` (`timeout`, `on_resume`). Each
  turns into the API's `memory` flag the way the SDK's
  `resolve_snapshot_memory` and `resolve_connect_memory` do: a pause sends
  `true` for full, `false` for filesystem, nothing with no mode; a connect
  sends `false` only for a reboot.
- `Client::pause_sandbox_with` and `Sandbox::pause_with` send `{memory?}`.
  `Client::connect_sandbox_with` sends `{timeout?, memory?}` on the v2 route
  and `{timeout, memory?}` on the v1 fallback, so a reboot stays a reboot
  when the server only has v1. The v0.1 `pause_sandbox` and
  `connect_sandbox` keep their signatures and delegate.
- `src/models.rs`: `PauseBody` and `ConnectBody` replace `EmptyBody`.
  `TimeoutBody` stays for `set_sandbox_timeout`, and its doc now names
  `SandboxTimeoutRequest`.
- The three new types are re-exported from the crate root and named in the
  crate doc.
- Tests: 5 mock tests in `tests/rest_lifecycle.rs`, 3 unit tests in
  `src/lifecycle.rs`, doctests on `PauseOptions` and `ConnectOptions`, and
  the live tests `clm_0146_resume_restores_memory_or_reboots` and
  `clm_0146_filesystem_pause_cold_boots`. The existing connect and pause
  tests in `tests/rest_api.rs` pass unchanged.
- Docs: the "Connect / resume", "Connect v1" and "Pause" rows of
  `docs/protocol-notes.md`, and the CLM-0159 row of
  `docs/verification/s4-evidence-map.md`. The plan's Task 7 has a dated
  correction, and Task 8's `SnapshotOptions::mode` doc no longer points to
  `PauseOptions::mode` for the 400/409 refusals.

## Why

**The on-disk marker is in `/home/user`.** The plan put it in `/tmp`, and
both live tests failed on their first run with "lost the on-disk file". A
probe on hosted E2B showed why: `/tmp` is on the ext4 root disk, but the
`base` template's `/usr/lib/tmpfiles.d/tmp.conf` has `D /tmp 1777 root root -`,
so systemd empties `/tmp` at every boot. After a reboot, and after a
filesystem pause, `/home/user` and `/var/tmp` kept their files, `/tmp` and
`/dev/shm` lost theirs, and `/proc/sys/kernel/random/boot_id` changed. After
a full pause and a restore, the live test found the `/dev/shm` file still
there. So the `/dev/shm` check does tell a restore from a reboot, as
planned; only the disk marker had to move. The markers are written with `sync` because the API says a
cold boot may lose writes not flushed before the pause.

**400 and 409 for a filesystem-only pause.** The plan carried these over
from snapshots. The SDK documents them for `create_snapshot` only, and its
pause reads every 409 as "already paused". The crate does the same (wire
parity), so a node refusal would show up as `Ok(false)`, and the rustdoc
says so instead of promising an error. Hosted E2B accepted the filesystem
pause, so no environment limit applies.

## Live results (hosted E2B, 2026-10-28 UTC)

Each test was run alone with
`E2B_TEST_TARGET=hosted cargo test --test live_lifecycle -- --ignored --exact <test>`:

| Test | Result |
|---|---|
| `clm_0146_resume_restores_memory_or_reboots` | pass |
| `clm_0146_filesystem_pause_cold_boots` | pass |
| `clm_0014_connect_resumes_paused_sandbox_with_files` | pass |
| `clm_0023_pause_running_sandbox` | pass |
| `clm_0024_pause_paused_sandbox_is_false` | pass |
