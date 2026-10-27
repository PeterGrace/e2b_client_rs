# M2 Task 10: live run on sandbawks

Issue #39, Task 10 of `docs/superpowers/plans/2026-10-27-milestone-2-command-and-file-parity.md`.

## What changed

- `docs/verification/runs/`: five logs, all from `0b9494b` with `dirty: no`:
  the run of record `20261027T130108Z-sandbawks-2101151` (`just test-live
  sandbawks`, template `compound-sandbox`) and four rounds of single-test
  reruns (`...-rerun-2121210`, `...-rerun2-2291013`, `...-rerun3-2463003`,
  `...-rerun4-2799756`).
- `docs/verification/m2-live-results.md`: a "sandbawks" section below the
  hosted one, with one row per `clm_*` test, the totals, the sweep result, the
  key and signed-URL checks, notes on the defects already filed, and five new
  sandbawks defects (N1 to N5) with their request and response evidence.

## Result

In the run of record 14 of 57 passed (lifecycle 10/21, commands 4/17, files
0/19). Most failures were capacity: four tests at a time drained the
two-sandbox warm pool, and cold sandboxes went unschedulable or failed while
create still answered `running` (gfd/sandbawks#674). After the reruns 29 pass
and 28 fail (lifecycle 11/21, commands 11/17, files 7/19). All 28 failures are
sandbawks defects. 22 match gfd/sandbawks #664, #665, #667, #675 to #679,
#681 or #682, and 6 tests show 5 new defects:

- N1: a multi-file multipart `POST /files` without `path` gets 400 (CLM-0062).
- N2: nginx answers 413 to a 64 MiB upload (CLM-0066).
- N3: `GET /files` of an 8 MiB file gets 500 (CLM-0128).
- N4: `Stat` gives empty owner and group, and permissions without the type
  character (CLM-0056, CLM-0061).
- N5: sandboxes have no `envdAccessToken`, and envd's file API accepts
  requests without credentials (CLM-0065; CLM-0064 passes only because of it).

Every sweep killed 0 leftover sandboxes. The key-value grep counted 0 for both
keys in every log, with both `.env` values checked to be unquoted, and no log
has `signature=`, `v1_` or an access token.

## Why there were four rerun rounds

The first round ran the 43 failures 20 s apart. That was too fast for the
warm pool to refill, while the cluster was adding nodes, so only 13 failures
were settled. Later rounds waited 120 s between tests and reran only the tests
still failing on readiness, until each one either passed or failed on
something else. No test was left classed as capacity.

## Why there are no test, crate or plan fixes

Every failing test passes on hosted E2B (run `20261027T125515Z-hosted-1922536`),
which is canonical. No test was found wrong, and none was loosened. The run
followed the plan as written, so the plan needs no correction.
