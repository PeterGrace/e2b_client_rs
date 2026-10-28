# M3 Task 6: Lifecycle and idempotency key at create (issue #65)

Plan: `docs/superpowers/plans/2026-10-27-milestone-3-lifecycle-parity.md`, Task 6.
Claims: CLM-0145, CLM-0152 and the lifecycle part of CLM-0153 (S1); CLM-0158
and the lifecycle part of CLM-0167 (S4).

## What changed

- `src/lifecycle.rs` (new): `SnapshotMode` (`full`, `filesystem`),
  `OnTimeout` (`Kill`, or `Pause { mode }` with `pause()` and `pause_with`),
  `Lifecycle` (`on_timeout`, `auto_resume`), and the read side
  `TimeoutAction` (with `Unknown`) and `LifecycleInfo`. `Lifecycle::body()`
  runs the SDK's `build_lifecycle_config` checks and builds `autoPause`,
  `autoPauseMemory` and `autoResume`.
- `CreateSandboxParams`: the `auto_pause: Option<bool>` field is now
  `lifecycle: Lifecycle`. `auto_pause(bool)` keeps its signature and sets
  `on_timeout` (pause or kill), so it and `lifecycle` write the same setting
  and the later call wins. New `lifecycle(Lifecycle)` and
  `idempotency_key(key)`. `body()` is now fallible.
- `Client::create_sandbox` checks the idempotency key, then the lifecycle,
  before any request, and sends the key as the `Idempotency-Key` header.
- `SandboxInfo` gains `lifecycle: Option<LifecycleInfo>` and
  `pause_mode: Option<SnapshotMode>`; an unknown `pauseMode` reads as `None`.
- The five new types are re-exported from the crate root and named in the
  crate doc.
- Tests: 11 mock tests in `tests/rest_lifecycle.rs` (new), 6 unit tests in
  `src/lifecycle.rs`, 3 in `src/models.rs` (`auto_pause` as shorthand, and
  the key's accepted and refused forms), doctests on `Lifecycle` and
  `CreateSandboxParams::lifecycle`, and the live tests
  `clm_0145_lifecycle_pause_with_full_snapshot_pauses_at_timeout` and
  `clm_0153_get_info_reports_lifecycle`. The `SandboxDetail` helper moved
  from `tests/rest_network.rs` to `tests/common/mod.rs` as `detail_json`, so
  both files share it.
- Docs: the "Create body" row gains `autoPauseMemory`, `autoResume` and the
  header; new rows "Create lifecycle", "Lifecycle checks", "Idempotency key",
  "Detail `lifecycle`" and "Detail `pauseMode`" in `docs/protocol-notes.md`,
  whose "Not implemented yet" bullet now names only volume mounts and the MCP
  gateway as the create fields still to come. CLM-0152 and CLM-0158 rows and
  the lifecycle part of CLM-0167 in `docs/verification/s4-evidence-map.md`.

## Why

The SDK resolves the key first, then the lifecycle, so a request with both
wrong names the key here too. It counts the key's length in characters
(Python's `len`) before checking the character set; `str::len` counts bytes,
so the crate counts `chars()` to give the same answer for a non-ASCII key.

The SDK refuses a `mode` with a kill at run time. Here `OnTimeout::Kill` has
no mode field, so that combination cannot be written and needs no check.

## Live result

On hosted E2B, one test at a time:

- `clm_0153_get_info_reports_lifecycle`: passed. The detail reports
  `lifecycle: {onTimeout: kill, autoResume: false}`.
- `clm_0011_auto_pause_pauses_at_timeout` (the reimplemented `auto_pause`):
  passed.
- `clm_0145_lifecycle_pause_with_full_snapshot_pauses_at_timeout`: failed on
  the first run with `pause_mode: None` at the first `paused` poll. A probe
  of the raw detail showed `pauseMode` is absent while the sandbox runs and
  appears (`full`, or `filesystem` for `autoPauseMemory: false`) once the
  pause has finished, as the spec's doc string says
  (`sandbox_detail.py:44-47`). The test now waits for both the paused state
  and a reported mode, and passed.

After review (PR #79): a `lifecycle` whose `onTimeout` is missing or `null`
reads as `TimeoutAction::Unknown` instead of failing `get_info`, and the
`auto_pause` doc says that it drops an earlier pause mode. `clm_0153` and
`clm_0145` passed again on hosted E2B after the change.

## Plan divergences

A dated correction at Task 6 records the check order, the character-count
length check, the `pauseMode` timing and the live test's wait, the shared
`detail_json` helper, and the extra tests; the Step 4, 5 and 7 code blocks
are fixed inline. Tasks 7, 8 and 11 use `SnapshotMode`,
`SnapshotMode::keeps_memory` and `CreateSandboxParams::body()` with the names
and signatures the plan gives, so none of them changes.
