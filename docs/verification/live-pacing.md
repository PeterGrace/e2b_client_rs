# Paced live runs

`just test-live-paced <target> [gap] [filter]` runs the live suite one test at
a time, `gap` seconds apart (default 120), for servers whose warm pool refills
slowly. sandbawks keeps two warm sandboxes; at `--test-threads=4` a run drains
them at once and the cold creates that follow time out or go unschedulable
(`docs/verification/m2-live-results.md`, "Why there are four rerun rounds").
Hosted E2B has no such limit and keeps `just test-live`.

## What it does

1. Writes the same `commit:` and `dirty:` header as `just test-live` (both
   recipes call the private `_stamp` recipe), then a `paced:` line with the gap.
2. Builds the three live binaries once, so a compile error stops the run
   instead of failing every test.
3. Lists each binary's ignored tests (`cargo test --test <bin> -- --ignored
   --list`) and runs them one by one with `--exact --test-threads=1`, sleeping
   `gap` seconds between tests. A `filter` (an extended regex on the test
   name) runs a subset. Every test shares the run's `E2B_TEST_RUN_ID`, so the
   sweep finds everything the run created.
4. Classifies each result and writes one line per test:

   | Line | Meaning |
   |---|---|
   | `RESULT <test> PASS` | passed on the first try |
   | `RESULT <test> FAIL` | failed for a reason other than readiness: a real result |
   | `RESULT <test> FAIL-READINESS` | failed while the server had no ready sandbox; rerun below |
   | `RESULT <test> PASS-ON-RERUN (rerun)` | passed on its one rerun |
   | `RESULT <test> FAIL-READINESS (rerun)` | still no ready sandbox after the rerun: a capacity result, says nothing about the feature |

   A failure is a readiness failure when its output contains `still starting
   up`, `Service Temporarily Unavailable`, `Unschedulable` or `will not become
   ready`, the messages the M2 and M3 sandbawks runs saw while no sandbox was
   ready. A failure reported only as an exit code (M2 saw a synthetic 75) is
   left as `FAIL` for a person to classify. A test that failed and was rerun
   has two lines: `FAIL-READINESS`, then the rerun's result.
5. Reruns each readiness failure once, after another gap.
6. Runs `live_sweep` for the run id and prints a summary line that counts the
   final results.

The log is `target/live-logs/<run id>.log`, with each test's full output above
its `RESULT` line. The run fails if any test ends `FAIL` or
`FAIL-READINESS (rerun)`, or if the sweep fails. On a server without the
snapshot or volume routes (sandbawks today) those two sweeps fail with 404 on
their list call and the recipe exits non-zero; that is not a leak.

## Cost and time

A full run is about (number of tests) x (gap + test time): 79 tests at 120 s
is roughly three hours. Run it in the background and wait for its log to print
`run log:`. A narrower filter and gap are fine for checking a fix.

## Validation (M4 Task 15)

[`runs/20261028T152320Z-sandbawks-paced-608823.log`](runs/20261028T152320Z-sandbawks-paced-608823.log): three tests, 30 s apart, against
sandbawks (template `compound-sandbox`), on commit `a43141d` with a clean
tree:

```text
RESULT clm_0007_create_returns_running_sandbox PASS
RESULT clm_0033_run_returns_output_and_exit_code PASS
RESULT clm_0050_write_read_round_trips_binary PASS
summary: 3 pass, 0 pass on rerun, 0 fail, 0 still not ready
```

The sandbox sweep killed 0 leftovers. The snapshot and volume sweeps failed
with `Api { status: 404, message: "HTTP 404" }` on their list call (sandbawks
has no such routes, gfd/sandbawks #664 to #672), so the recipe exited 101.
No test hit a readiness failure, so the rerun path was not exercised live; the
marker regex was checked against the four messages (each classified as
readiness) and against `exit code 75` and `HTTP 404` (each left as `FAIL`).
Neither API key appears in the log.
