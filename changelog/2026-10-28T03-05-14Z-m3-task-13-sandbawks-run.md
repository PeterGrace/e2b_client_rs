# M3 Task 13: Live run on sandbawks (issue #72)

Plan: `docs/superpowers/plans/2026-10-27-milestone-3-lifecycle-parity.md`, Task 13.

## What changed

- `docs/verification/runs/`: four logs, all from `6871dbf` with `dirty: no`,
  copied from `target/live-logs/` after the secret check:
  - `20261028T012919Z-sandbawks-2784597.log`, the run of record
    (`just test-live sandbawks`, template `compound-sandbox`);
  - `20261028T013641Z-sandbawks-rerun-2828108.log` (R1, 38 tests),
    `20261028T025532Z-sandbawks-rerun2-3256660.log` (R2, 3 tests) and
    `20261028T030151Z-sandbawks-rerun3-3279120.log` (R3, 1 test): single-test
    reruns, 120 s apart, each followed by the sweeps.
- `docs/verification/m3-live-results.md`: the "sandbawks" section, in M2's
  form: the runs, one row per `clm_*` test (75) with its run-of-record and
  rerun results, hosted result, class and gfd/sandbawks issue, the totals,
  the sweeps, the secret check, the known missing routes, and two new
  defects (N1, N2) with their requests and responses.
- The plan's Task 13 gets a correction note: which failures were rerun, and
  that the snapshot and volume sweeps fail with 404 on sandbawks.

## Results

- Run of record: 16 of 75 passed. After the reruns: 30 pass, 45 fail; none
  is left at capacity.
- M1's and M2's 57: 29 pass, 28 fail, the same sets as in M2, each against
  the gfd/sandbawks issue M2 recorded.
- M3's 18: 1 passes (the missing-volume test, only because every volume
  route is 404). 12 fail on the known missing routes and fields (#665,
  #667 to #671), CLM-0028 on #678, CLM-0150 on #680 (probably no
  `mcp-gateway` template), and 3 on new defects: N1, the template has no
  `/home/user` (both CLM-0146 tests), and N2, `GET /v2/sandboxes` ignores
  `order`, `template` and `startedAfter` (CLM-0151). The coordinator filed
  them as gfd/sandbawks #688 and #689, and the results link them.
- Sweeps: 0 leftover sandboxes for every run id, the aborted first attempt's
  included; the snapshot and volume sweeps fail with 404 (routes missing),
  noted, not leaks.
- Secret check: both API keys, `GATEWAY_ACCESS_TOKEN=` and `"token"` are 0
  in every log.

## Why

Task 14 needs one recorded sandbawks run of the merged M3 code, with every
failure classified, so that the coordinator can file the new sandbawks
defects under epic gfd/sandbawks#673. Hosted E2B is canonical, so a failure
that passes there is a sandbawks defect, not a crate bug.
