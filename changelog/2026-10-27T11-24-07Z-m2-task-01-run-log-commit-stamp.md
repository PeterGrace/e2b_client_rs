# M2 Task 1: stamp the commit in run logs

Issue #26, Task 1 of `docs/superpowers/plans/2026-10-27-milestone-2-command-and-file-parity.md`.

## What changed

- `justfile`: `test-live` now writes two lines before any test output:
  `commit: <git rev-parse HEAD>` and `dirty: yes|no` (from
  `git status --porcelain`, so untracked files count as dirty too). The claim
  tests' output is appended with `tee -a`, so the header survives. A checked-in
  log can now be traced to the exact code that produced it.
- `CONTRIBUTING.md`: the run-log paragraph in the Tests section describes the
  two header lines.

## Why

Ledger evidence named the commit in its locator, but the log itself could not
be traced to a commit. Raised in the PR #22 review and listed as an M1
follow-up.

## Not done

- The issue's optional item, printing paths relative to the repo root, was not
  done. The `/home/...` paths come from cargo's own `Compiling` lines, and
  filtering cargo's output would hide lines a reader may need. The plan's Task 1
  does not ask for it either.

## Validation

- `just test-live nosuchtarget` (plan Step 2): the tests fail fast on the
  unknown target, no sandbox is created, and the log starts with
  `commit: <40 hex>` and `dirty: yes` (the change was not yet committed).
- `just test-live hosted`, run `20261027T112435Z-hosted-554735` on commit
  `cc85e67` with the tree committed: the log starts with
  `commit: cc85e67fec62703ae8ef4f45218ddb65ac5d9621` and `dirty: no`. All 46
  claim tests passed (13 commands, 12 files, 21 lifecycle), and the sweep
  reported `killed 0 leftover sandbox(es)`. Neither key appears in the log.
  The log stays in `target/live-logs/`; Task 9 does the recorded run.

## Review follow-ups (PR #41)

- The header no longer stamps an empty commit and `dirty: no` when git can't
  answer (outside a checkout, or git missing). The commit line uses
  `git rev-parse --verify -q HEAD` and falls back to `unknown`; the dirty line
  prints `dirty: unknown` when `git status` fails. The plan's Task 1 code was
  corrected to match, and Task 9 now says `unknown` stamps don't count either.
