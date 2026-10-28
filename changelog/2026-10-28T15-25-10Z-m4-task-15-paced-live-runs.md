# M4 Task 15: Paced live runs for sandbawks (issue #57)

Plan: `docs/superpowers/plans/2026-10-28-milestone-4-api-stability.md`, Task 15
(decision 8). Claim: CLM-0181.

## What changed

- `justfile`: new recipe `just test-live-paced <target> [gap=120] [filter]`.
  It lists each live binary's ignored tests, runs them one at a time with
  `--exact --test-threads=1`, sleeps `gap` seconds between tests, reruns each
  readiness failure once after another gap, writes one
  `RESULT <test> PASS|FAIL|FAIL-READINESS|PASS-ON-RERUN` line per test (a
  rerun's line ends ` (rerun)`), runs `live_sweep` and prints a summary.
  `filter` is an extended regex on test names.
- `justfile`: the `commit:`/`dirty:` header moved into a private `_stamp`
  recipe that both `test-live` and `test-live-paced` call. `test-live` writes
  the same header and runs the same commands as before; hosted runs keep it.
- `docs/verification/live-pacing.md` (new): how the recipe works, the result
  lines, the cost of a full run and the validation run.
- `CONTRIBUTING.md`: the recipe in the tooling table and in the self-hosted
  section.
- The plan: a dated correction note on Task 15 (comment order for
  `just --list`; the recipe committed before the validation run).

## The readiness markers

A failure is rerun when its output contains `still starting up` (envd's 503),
`Service Temporarily Unavailable` (nginx's 503), `Unschedulable` (the Pod could
not be placed) or `will not become ready` (sandbawks' 412 `action_failure`).
These are the messages the M2 and M3 sandbawks runs saw while no sandbox was
ready (`docs/verification/m2-live-results.md`, "Why there are four rerun
rounds"; `m3-live-results.md`). A failure reported only by an exit code (M2's
synthetic 75) stays `FAIL` for a person to classify.

## Why

sandbawks keeps two warm sandboxes and needs about 120 s to refill. M2 and M3
needed hand-paced rerun rounds; the recipe does those rounds itself, so M4's
sandbawks run (Task 17) is one run of record.

## Validation

`just test-live-paced sandbawks 30 'clm_0007_create_returns_running_sandbox|clm_0033_run_returns_output_and_exit_code|clm_0050_write_read_round_trips_binary'`
with `E2B_TEMPLATE_SANDBAWKS=compound-sandbox`, run
`20261028T152320Z-sandbawks-paced-608823` on commit `a43141d` (`dirty: no`),
log in `docs/verification/runs/`: all three `PASS`; the sandbox sweep killed
0 leftovers; the snapshot and volume sweeps failed with 404 (no such routes on
sandbawks), so the recipe exited 101. Neither API key is in the log. A dry run
with an unknown target (`just test-live-paced nosuchtarget 1 clm_0007`) gave
the header, one `RESULT ... FAIL` and a failed sweep, and created nothing.

## Review changes

- `gap` and `filter` are assigned with just's `quote()`, so a `$(` or a
  backtick in a filter is matched literally instead of run by bash.
- The paced recipe's last line, `run log: <path>`, is also written to the log,
  so a background run is done when its log prints it (as Task 17 says).
  `just test-live` is unchanged.
- Recheck run `20261028T152914Z-sandbawks-paced-726401` on `0591c6a`: one test
  (anchored filter), `PASS`, 0 leftovers, the log ends with `run log:`. The
  three-test validation log stays as the run of record.
- Not changed: the snapshot and volume sweeps 404 on sandbawks, so the recipe
  exits non-zero there even when every test passes. That is the sweep's
  behaviour (`tests/live_sweep.rs`), outside this task; left for the
  coordinator to decide.

## Breaking changes

None. No Rust code changed; `Cargo.lock` is unchanged.
