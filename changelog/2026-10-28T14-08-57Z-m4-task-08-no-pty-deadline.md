# M4 Task 8: no PTY deadline by default (issue #55)

Plan: `docs/superpowers/plans/2026-10-28-milestone-4-api-stability.md`, Task 8.
Claim: CLM-0175 (decision 6).

A PTY's stream deadline counts from when the session opens, not from the last
activity, and when it passes envd ends the shell, so it cannot be reattached
(observed on hosted E2B, 2026-10-27). With the SDKs' 60 s default, every
interactive session died after a minute. pgrace decided on 2026-10-28 that a
PTY has no deadline unless the caller sets one. This deliberately differs
from SDK 2.61 (`timeout=60` on `pty.create` and `pty.connect`).

## What changed

- `src/pty.rs`:
  - `PtyOptions::default()` and `PtyConnectOptions::default()` (and `new()`)
    set no stream deadline. With every field at its own default, both types
    now derive `Default` instead of writing it by hand.
  - `DEFAULT_PTY_TIMEOUT` stays exported as the SDKs' 60 s; its rustdoc says
    the crate does not apply it by default and how to opt in.
  - The rustdoc of `PtyOptions::timeout`, `PtyConnectOptions` (type, `new`,
    `timeout`) and `Pty::create`'s `options` argument describe the new
    default; the deadline-kills-the-shell note now applies "if you set one".
    The `PtyOptions` example sets a 1 h deadline instead of restating the
    default.
- `README.md`: the PTY part of the tour example says there is no deadline by
  default and how to set one.
- `docs/protocol-notes.md`: the PTY `create`/`connect` row and the deadline
  row; a new "No PTY deadline by default" bullet under "Deliberate
  differences from the SDKs".
- The plan's Task 8 has a Corrected note (derived `Default`, the changelog
  slug, the extra protocol-notes bullet, the live result).

## Breaking changes

| Old | New |
|---|---|
| `PtyOptions::default()` bounded the session to 60 s | No deadline; for the old behaviour use `PtyOptions::default().timeout(Some(DEFAULT_PTY_TIMEOUT))` |
| `PtyConnectOptions::default()` / `new()` bounded the attach to 60 s | No deadline; for the old behaviour use `PtyConnectOptions::new().timeout(Some(DEFAULT_PTY_TIMEOUT))` |

No signature changes; every caller in `src/`, `tests/`, `README.md` and the
doctests compiles unchanged.

## Tests

- Unit: `pty::tests::options_default_to_no_deadline` replaces
  `options_default_to_the_sdk_deadline` and
  `connect_options_default_to_the_sdk_deadline`.
- `tests/envd_connect_options.rs`: `pty_connect_has_no_deadline_by_default`
  replaces `pty_connect_defaults_to_the_sdk_sixty_seconds`;
  `pty_create_sends_a_deadline_only_when_set` sends `Connect-Timeout-Ms:
  60000` when `DEFAULT_PTY_TIMEOUT` is set explicitly.
- `tests/envd_pty.rs`: `create_sends_bash_with_pty_size_and_terminal_env_defaults`
  asserts no deadline header; the connect test uses
  `PtyConnectOptions::default()`, so the default path is the one under test.
- Live: new `clm_0175_pty_outlives_sixty_seconds` (a 150 s sandbox; after
  65 s the shell still answers `echo alive-$((40+2))` and exits 0).

## Live check (hosted E2B)

Each run one at a time, `E2B_TEST_TARGET=hosted`:

| Test | Run id | Result |
|---|---|---|
| `clm_0175_pty_outlives_sixty_seconds` | `dev-20261028T140645Z-3686094` | passed, 66.69 s |
| `clm_0047_pty_create_streams_terminal_output` | `dev-20261028T140752Z-3686094` | passed, 1.19 s |
| `clm_0048_pty_resize_changes_terminal_size` | `dev-20261028T140754Z-3686094` | passed, 0.87 s |
| `clm_0049_pty_input_reaches_the_shell` | `dev-20261028T140755Z-3686094` | passed, 1.00 s |

Closes #55.
