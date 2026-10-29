# M5 Task 6: A publishable package (issue #128)

Plan: `docs/superpowers/plans/2026-10-28-milestone-5-release.md`, Task 6
(decision 9). Backs CLM-0124. Closes #128.

## Why

`Cargo.toml` had `publish = false`, so `cargo publish` refused outright, and
without an `include` list the package would have shipped everything git
tracks: `claims.json` (464 KB), `docs/` with the run logs (1.5 MB),
`changelog/`, `tests/`, `.claude/`, `.github/` and the `justfile`. A crate
user needs none of it.

## What changed

- `Cargo.toml`: `publish = false` is gone (the crate can now be published;
  only Task 10 does it) and `include = ["/src", "/README.md",
  "/CHANGELOG.md", "/LICENSE-APACHE", "/LICENSE-MIT"]` names what ships. The
  README stays because `src/lib.rs` includes it under `cfg(doctest)`, so its
  examples are doctests, and because `readme = "README.md"` names it.
  `CHANGELOG.md` does not exist yet (Task 7); cargo skips a missing include
  path silently.
- `justfile`: `package-check` fails, listing the files, if
  `cargo package --list --locked --allow-dirty` would ship anything outside
  `src/**.rs`, the README, `CHANGELOG.md`, the licences and cargo's own
  files (`Cargo.toml`, `Cargo.toml.orig`, `Cargo.lock`,
  `.cargo_vcs_info.json`). The list is taken into a variable first, so a
  failing `cargo package --list` stops the recipe there instead of being
  hidden by the grep's `|| true` (review of PR #139). Then it builds the
  packaged crate (`cargo package --locked --allow-dirty`) and prints
  `package ok`.
  `--allow-dirty` lets a contributor run it before committing.
- `.github/workflows/ci.yml`: job `check` gains a `Package` step after
  `Docs`.
- `CONTRIBUTING.md`: the tooling table and the CI sentence name
  `just package-check`; a paragraph says a new top-level file is not shipped
  unless it is added to `include` and to the recipe's allowed pattern.
- The plan: a dated correction in Task 6 (the recipe does not print the file
  list; the dry run's per-test "ignoring test" warnings are expected), and
  the same wording fixed in Task 7's check and Task 10's CLM-0124 row.

## The package

`cargo package --list --locked --allow-dirty` (28 files):
`.cargo_vcs_info.json`, `Cargo.lock`, `Cargo.toml`, `Cargo.toml.orig`,
`LICENSE-APACHE`, `LICENSE-MIT`, `README.md` and the 21 files under `src/`.
579.1 KiB, 137.9 KiB compressed.

## Validation

- `just package-check` before the `Cargo.toml` change: exit 1, listing 141
  files (60 under `changelog/`, 37 under `docs/`, 37 under `tests/`, two
  under `.github/`, `.claude/...`, `claims.json`, `CONTRIBUTING.md`,
  `.gitignore`, `justfile`). After: `package ok`.
- `cargo publish --dry-run --locked --allow-dirty`: packages, verifies
  (compiles the packaged crate), and ends `warning: aborting upload due to
  dry run`. Before packaging it prints one `warning: ignoring test ...`
  per test target (28), because `tests/` is not shipped; that is expected.
- `Cargo.lock` unchanged.
- Gates: `just lint`, `just test` (522 passed, 0 failed, 84 ignored),
  `just doc`, `just test-recipes` (7 of 7).
