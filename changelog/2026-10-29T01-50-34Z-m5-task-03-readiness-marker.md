# M5 Task 3: A sandbox that failed is not a readiness result (issue #122)

Plan: `docs/superpowers/plans/2026-10-28-milestone-5-release.md`, Task 3
(decision 8). Closes #122.

## Why

`just test-live-paced` labelled a failure `FAIL-READINESS` ("capacity,
rerun it") when its output contained `will not become ready`. sandbawks's
message for a sandbox that failed outright starts with the same words
(`sandbox will not become ready without intervention: the sandbox failed
(action_failure)`), so M4's paced run counted `clm_0150`, an unknown
template (gfd/sandbawks#680), as capacity. A server defect could be
counted as capacity and rerun instead of being reported.

## What changed

- `justfile`:
  - New private `_classify status out` recipe: prints `PASS` (exit 0),
    `FAIL-READINESS` (output has `still starting up`, `Service Temporarily
    Unavailable` or `Unschedulable`), `FAIL-SANDBOX-FAILED` (output has
    `the sandbox failed (action_failure)` and no readiness marker) or
    `FAIL`. A readiness marker wins when both appear (M2's capacity runs
    saw both). `will not become ready` is no longer a marker.
  - New `test-recipes` recipe: classifies each fixture in
    `tests/fixtures/live-pacing/` (exit 0 for `PASS`/`SKIP` fixtures, 101
    otherwise) and checks the label against the file name before `--`.
  - `test-live-paced` classifies through `_classify`; readiness and
    sandbox-failed results get the one rerun; any other label, including
    an unexpected or empty one, fails the run. A rerun keeps its new
    label, so it ends `PASS-ON-RERUN`, `FAIL-READINESS (rerun)`,
    `FAIL-SANDBOX-FAILED (rerun)` or `FAIL (rerun)`; anything but a pass
    fails the run. The summary line is built with a `count` helper and
    reads `N pass, N pass on rerun, N fail, N still not ready, N sandbox
    failed`; `fail` now counts `FAIL (rerun)` too.
- `tests/fixtures/live-pacing/`: six fixtures (`PASS--ok`,
  `FAIL--assertion`, `FAIL-READINESS--still-starting`,
  `FAIL-READINESS--unschedulable`, `FAIL-SANDBOX-FAILED--action-failure`,
  `FAIL-READINESS--capacity-and-action-failure`), lines copied from the
  M2 to M4 sandbawks logs.
- `.github/workflows/ci.yml`: a `Recipe tests` step (`just test-recipes`)
  after `Test` in the `check` job.
- `CONTRIBUTING.md`: `just test-recipes` in the tooling table and the CI
  sentence; the paced row says sandbox-failed results are rerun too.
- `docs/verification/live-pacing.md`: the result table gains
  `FAIL-SANDBOX-FAILED`, `FAIL-SANDBOX-FAILED (rerun)` and
  `FAIL (rerun)`; the readiness paragraph lists the three markers and
  says why `will not become ready` left; a paragraph on the new class
  (a person reads each `FAIL-SANDBOX-FAILED (rerun)` and counts it as
  capacity or a server defect) replaces "read a `FAIL-READINESS (rerun)`
  line before counting it"; `just test-recipes`; the validation run.
- `docs/verification/runs/20261029T014807Z-sandbawks-paced-2828998.log`:
  the validation run's log.
- The plan: a dated correction under Task 3 Step 4 (no `SKIP) ;;` branch
  yet; the first-pass `case` ends in `*) status=1 ;;`; the `count` helper
  and the wider `fail` figure), and Task 4 Step 2 now says to add
  `SKIP) ;;` to both `case`s and `$(count SKIP) skipped`; decision 8 now
  says M1, M3 and M4 saw the unknown-template failure.

## Verification

- `just test-recipes`: six `ok` lines, exit 0 (before `_classify` existed
  it failed every fixture).
- `just lint`, `just test` (522 passed, 0 failed, 84 ignored, 30 suites),
  `just doc`: green. `Cargo.lock` unchanged.
- `just test-live-paced sandbawks 30 'clm_0150|clm_0007'` on commit
  `ad2a28a`, clean tree:

  ```text
  RESULT clm_0007_create_returns_running_sandbox PASS
  RESULT clm_0150_mcp_gateway_starts_and_exposes_url_and_token FAIL-SANDBOX-FAILED
  RESULT clm_0150_mcp_gateway_starts_and_exposes_url_and_token FAIL-SANDBOX-FAILED (rerun)
  summary: 1 pass, 0 pass on rerun, 0 fail, 0 still not ready, 1 sandbox failed
  ```

  The sandbox sweep killed 0 leftovers; the snapshot and volume sweeps
  failed with 404 (#121, out of scope), and the recipe exited 1. Neither
  API key appears in the log.
