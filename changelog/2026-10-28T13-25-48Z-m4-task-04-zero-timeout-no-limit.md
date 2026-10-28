# M4 Task 4: A zero timeout means no limit (issue #52)

Plan: `docs/superpowers/plans/2026-10-28-milestone-4-api-stability.md`, Task 4.
Claim: CLM-0172 (S4), `partial` until Task 5 adds the per-call request timeout.

## What changed

- `src/config.rs`: `no_limit_if_zero`, the one place the rule lives.
  `ConnectionConfigBuilder::request_timeout(Some(Duration::ZERO))` now means
  no request timeout, as the SDK's `request_timeout=0`.
- `CommandOptions::timeout`, `PtyOptions::timeout`, `WatchOptions::timeout`
  and `StreamInputOptions::timeout` store `None` for a zero duration, so
  no `Connect-Timeout-Ms` header and no client-side deadline are set.
- `Commands::connect(pid, timeout)` and `Pty::connect(pid, timeout)` apply
  the same rule to their deadline argument, so every stream deadline agrees
  until Task 7 moves those deadlines into options structs.
- Tests: `tests/zero_timeouts.rs` (7) and the `src/config.rs` unit test
  `zero_means_no_limit`.
- Docs: the rustdoc of the five setters and the two `connect` methods (the
  `WatchOptions` "Python note" that called zero an expired deadline is gone),
  four rows of `docs/protocol-notes.md`, and the CLM-0172 row of the S4
  evidence map. The plan gains a Corrected note for the two `connect` tests,
  the shifted line numbers, and Task 7's new callers.

## Breaking changes

- `Some(Duration::ZERO)` passed to `CommandOptions::timeout`,
  `PtyOptions::timeout`, `WatchOptions::timeout`,
  `StreamInputOptions::timeout`, `ConnectionConfigBuilder::request_timeout`,
  `Commands::connect` or `Pty::connect` was a deadline that had already
  passed (sent as `Connect-Timeout-Ms: 1`; for the request timeout, every
  request failed at once). It is now no limit. Code that used zero to make
  a call fail at once has to use `Some(Duration::from_millis(1))`.

## Why

pgrace chose the SDK's meaning on 2026-10-28 (plan decision 5), so code
ported from Python with `timeout=0` keeps working.
