# M5 Task 8: The README coverage table from the ledger (issue #130)

Plan: `docs/superpowers/plans/2026-10-28-milestone-5-release.md`, Task 8.
Part of #124. Backs CLM-0184.

## What changed

- `tests/readme_coverage.rs` (new): recomputes the per-area table from
  `claims.json` (active claims only, by area tag `s1`..`s5` or `root`, by
  the last verdict; no verdict counts as unverified) and fails, printing
  the table to paste, if the README's table between
  `<!-- coverage:begin -->` and `<!-- coverage:end -->` differs. A claim
  with no area tag, more than one area tag or an unknown verdict fails the
  test rather than dropping out of the counts or being counted under one
  area quietly; markers in the wrong order fail with a clear message.
- `README.md`, "Verification status": the generated table between the two
  markers. The "Ledger:" bullet points at the table instead of repeating
  its (stale, 179-claim) numbers. The line above the bullets says they are
  the runs as of milestone 4, with the M5 claims still unverified; the
  wire-protocol bullet says its 79 are CLM-0005's children, so the S4 row's
  80 is those plus CLM-0005.
- The plan: a dated correction under Task 8 Step 1 (the two review-added
  assertions), and Task 10's README step names the reworded line.

The table today: 184 active claims, 169 supported, 10 `not_enough_info`,
0 refuted, 5 unverified (the M5 claims CLM-0182 to CLM-0186, which get
verdicts in Task 11).

## Rule for later PRs

Every PR that changes `claims.json` must update the table in the same PR
(only the coordinator's PRs change the ledger). Run
`cargo test --test readme_coverage`, paste the table it prints between the
markers, and run it again.

## Verification

- Before the README change: FAIL, `README has the coverage:begin marker`.
- With the markers and no table: FAIL, printing the table.
- After pasting it: PASS.
- `just lint`, `just test`, `just doc`, `just test-recipes`,
  `just package-check`: green (counts in the PR).

## Why

M5 decision 10: the coverage table is generated, not hand-kept, so it
cannot drift from the ledger the way the "Ledger:" bullet had.
