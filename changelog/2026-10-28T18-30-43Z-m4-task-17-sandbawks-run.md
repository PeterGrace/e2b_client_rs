# M4 Task 17: Live run on sandbawks, paced (issue #99)

Plan: `docs/superpowers/plans/2026-10-28-milestone-4-api-stability.md`, Task 17.

## What changed

- `docs/verification/runs/20261028T154205Z-sandbawks-paced-913873.log` (new):
  the run of record, `just test-live-paced sandbawks` at the default 120 s
  gap on `3cedbc3` (main with M4.01 to M4.16), template `compound-sandbox`,
  stamped `dirty: no`.
- `docs/verification/m4-live-results.md`: the "sandbawks" section (one row
  per `clm_*` test with its paced result, hosted result, class and first
  failure line; totals; sweeps; secret check; the CLM-0171 pause probe; new
  defect N1; the CLM-0118 statement; the M4 claims on sandbawks). The hosted
  section is unchanged.
- `docs/protocol-notes.md`, "Pause" row: sandbawks sends no 409 for a pause
  of a paused sandbox (200, `state: paused`, and the sandbox stays
  `running`, gfd/sandbawks#679), so its wording cannot be checked yet.
- `docs/verification/live-pacing.md`: `will not become ready` also marks an
  unknown template's `action_failure`, so a `FAIL-READINESS (rerun)` line is
  read before it counts as capacity.
- The plan, Task 17: two dated corrections (the missing 409, and `clm_0150`'s
  readiness label).

No code or test changed.

## Result

`summary: 29 pass, 0 pass on rerun, 49 fail, 1 still not ready` over 79
tests. M1 to M3's 75: 28 pass (M3 had 30 after its reruns); CLM-0010 and
CLM-0025 now fail on a new defect, N1: `startedAt` is when the warm-pool
sandbox started, not when create handed it out. Every other failure is the
one M3 recorded, against its gfd/sandbawks issue. M4's 4: `clm_0175` passes;
`clm_0171` (#679), `clm_0177` (#678) and `clm_0178` (#677) fail on known
defects. The one readiness result, `clm_0150`, is #680 (no `mcp-gateway`
template), not capacity.

The sandbox sweep killed 0 leftovers; the snapshot and volume sweeps failed
with 404 on their list calls (no such routes, #669, #671), so the recipe
exited 1. The secret check found 0 of each key and token.

CLM-0118: no v0.1 test ends at capacity; 24 of the 26 v0.1 failures are
filed, and the other 2 are N1. Once the coordinator files N1, CLM-0118 can
be decided (#27).

## Why

The milestone's run of record on sandbawks, in the paced form M4.15 built for
its two-sandbox warm pool. It is CLM-0181's demonstration, the sandbawks half
of the M4 claims' live evidence, and the run that can settle CLM-0118.
