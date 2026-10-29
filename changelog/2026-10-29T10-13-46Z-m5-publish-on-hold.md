# M5: the 0.2.0 publish is on hold

PR #143 tagged `v0.2.0` at `3855cc8`. A clean `cargo publish --dry-run
--locked` from a worktree of the tag packaged and verified `e2b-client
v0.2.0`, then stopped at "aborting upload due to dry run".

pgrace chose not to publish yet (2026-10-29). Until 0.2.0 is published:

- the README installs from the `v0.2.0` tag through git, not from
  crates.io, so `main` doesn't point at a release that doesn't exist;
- CLM-0186, and with it CLM-0006, stay undecided;
- M5.11 (#133) and epic #124 stay open.

When the publish goes ahead, the README goes back to `e2b-client = "0.2"`.
