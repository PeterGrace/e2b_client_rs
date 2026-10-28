# M4 Task 3: a refused moded pause is an error (issue #81)

A pause that asks for a `SnapshotMode` now reads the 409 body: hosted E2B's
"... is already paused" is `Ok(false)`, and any other 409 is `Error::Api`
with status 409, so a refused filesystem-only pause is no longer mistaken
for success (CLM-0171). A pause without a mode keeps SDK 2.61's reading of
every 409 as "already paused" (`py: sandbox_async/sandbox_api.py:421-423`).
Decided by pgrace while planning M4.

## Breaking changes

- `pause_sandbox_with(id, &PauseOptions::new().mode(..))` and
  `Sandbox::pause_with` return `Err(Error::Api { status: 409, .. })` for a
  409 that is not "already paused", where they returned `Ok(false)`.

## Tests

- New: `a_moded_pause_reads_only_an_already_paused_409_as_false`,
  `a_full_pause_also_reads_the_409_body`,
  `a_pause_without_a_mode_reads_every_409_as_already_paused`
  (`tests/rest_lifecycle.rs`); live `clm_0171_moded_pause_of_a_paused_sandbox_is_false`.
- Replaced: `pause_with_maps_status_codes_as_pause_does`. Renamed:
  `tests/rest_api.rs::pause_sandbox_without_a_mode_maps_status_codes`
  (evidence-map rows CLM-0095 and CLM-0159 follow).
- Live on hosted E2B, 2026-10-28: `clm_0171_moded_pause_of_a_paused_sandbox_is_false`
  passed (run `dev-20261028T131621Z-2883928`) and
  `clm_0024_pause_paused_sandbox_is_false` passed (run
  `dev-20261028T131624Z-2883928`).

## Plan

- Task 3 gains a dated Corrected note (SDK line 421-423, protocol-notes and
  evidence-map line numbers); Task 6's `tests/live_lifecycle.rs`,
  `src/client.rs` and `src/sandbox.rs` line numbers are moved to the
  post-Task-3 files.

## Review follow-up

- The "Pause" row in `docs/protocol-notes.md` says the "already paused"
  match is case-sensitive and that sandbawks's wording is not yet checked.
  Task 17 in the plan now records what sandbawks sends to `clm_0171`.
- `clm_0146`'s assertion message says what a `false` from a running
  sandbox's filesystem pause now means.
