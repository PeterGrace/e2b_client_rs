# M1 Task 7: live run on sandbawks

Issue #11, Task 7 of `docs/superpowers/plans/2026-10-26-milestone-1-verify-v0.1.md`.

## What changed

- `docs/verification/m1-live-results.md`: new section "sandbawks", with one
  row per `clm_*` test (46) and a column for the hosted E2B result. It
  classifies every failure and lists twelve sandbawks defects (S1-S12), each
  with the client call, the observed request and response, and the expected
  E2B behaviour with an SDK citation. The coordinator files them on
  gfd/sandbawks.
- `docs/verification/runs/`: the logs of all three runs, `20261026T225357Z-sandbawks-998331`,
  `20261026T225651Z-sandbawks-1078537` (the run of record) and
  `20261026T230103Z-sandbawks-rerun-1207087` (single-test reruns). Each was
  key-checked before it was copied: 0 matches for both keys.
- `justfile`: `test-live` now passes `--no-fail-fast`. Without it, cargo
  skipped `live_files` and `live_lifecycle` once `live_commands` had failed,
  so a single failure hid two thirds of the results. This happens on any
  server.
- `tests/live_sweep.rs`: the sweep now checks `metadata["e2b-client-test"]`
  against the run id itself before it kills a sandbox. sandbawks ignores the
  `metadata` and `state` filters of `GET /v2/sandboxes`, so the first run's
  sweep killed three of the server's warm-pool sandboxes, which it did not own.
  Nothing changes on hosted E2B, which honours the filter.
- Plan: a dated correction note in the Task 7 section only (the template, the
  two harness fixes, the capacity caveat, and 46 tests rather than 47).

## Why three runs

1. The first `just test-live sandbawks` used the default template `base`,
   which sandbawks doesn't have. Create still answered 201 `running`. All 13
   command tests failed against sandboxes that could never start, and cargo
   skipped the other two binaries. This run tested nothing about sandbawks.
2. The run of record repeated the run with
   `E2B_TEMPLATE_SANDBAWKS=compound-sandbox`, after the two harness fixes:
   17 passed, 29 failed.
3. Each of the 29 failures was then rerun on its own, to separate capacity
   from defects. The two-sandbox warm pool runs dry at four threads, and cold
   creates are unschedulable for lack of memory.

## Result

25 of 46 tests pass. Of the 21 failures, 16 are sandbawks defects that pass on
hosted E2B, and 5 are inconclusive because the sandbox never became ready. No
crate bugs and no test bugs were found. CLM-0014 passes vacuously on
sandbawks, because pause is a no-op there. The results file notes this.
