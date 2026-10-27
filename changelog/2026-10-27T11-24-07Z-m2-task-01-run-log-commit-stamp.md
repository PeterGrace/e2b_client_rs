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
- A full `just test-live hosted` run on the committed tree is reported in the
  PR.
