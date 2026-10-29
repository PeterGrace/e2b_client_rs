# M5 Task 4: Volume tests skip when the team has no volumes (issue #126)

Plan: `docs/superpowers/plans/2026-10-28-milestone-5-release.md`, Task 4
(decision 2). Closes #126.

## Why

This hosted E2B team has volumes disabled: `POST /volumes` answers 403
`use of volumes is not enabled`. The three live tests that create a volume
failed on every hosted run, so a hosted run (and the M5 live CI job) could
never be green, though the crate was not at fault.

## What changed

- `tests/live_support/mod.rs`: new `SKIPPED` constant
  (`SKIPPED (environment):`) and `create_volume_or_skip(client, name)`.
  It returns the volume, or on 403 whose message contains `not enabled`
  prints `SKIPPED (environment): volumes are not enabled for this team:
  <message>` with `eprintln!` and returns `None`. Any other error panics,
  so a real failure still fails the test.
- `tests/live_lifecycle.rs`: `clm_0148_volume_create_get_list_destroy`,
  `clm_0149_volume_mount_persists_across_sandboxes` and
  `clm_0153_get_info_reports_volume_mounts` create their volume through the
  helper and return early (`let ... else`) when it skips.
  `clm_0148_missing_volume_is_not_found_and_not_destroyed` is unchanged: it
  passes without volumes.
- `justfile`:
  - `_classify` prints `SKIP` for a test that exits 0 with a
    `SKIPPED (environment):` line in its output.
  - `test-live-paced`: both `case`s (first pass and rerun) accept `SKIP`
    without failing the run; the summary gains `N skipped`, counting
    `SKIP` and `SKIP (rerun)` through the `count` helper.
- `tests/fixtures/live-pacing/SKIP--volumes-not-enabled.txt`: the seventh
  recipe fixture.
- `docs/verification/live-pacing.md`: `SKIP` and `SKIP (rerun)` rows, a
  paragraph on what a skip means, `skipped` in the summary.
- `CONTRIBUTING.md`, "Tests": a skipped live test passes, and its skip is
  an environment limit, never evidence for a claim.

CLM-0148, CLM-0149 and CLM-0153 stay `not_enough_info`: a skip is not
evidence. Running them on a team with volumes is #119.

## Plan corrections

- Task 4 Step 2: the summary counts `SKIP( \(rerun\))?`, not only a
  first-pass `SKIP`, since the rerun `case` leaves the label `SKIP`.
  Task 9 needed no change.

## Validation

- `just test-recipes`: seven `ok` lines.
- Hosted E2B, one at a time: each of the three tests passed and printed
  `SKIPPED (environment): volumes are not enabled for this team: use of
  volumes is not enabled`; `_classify 0` labels each log `SKIP`.
- `just lint`, `just test`, `just doc`.
