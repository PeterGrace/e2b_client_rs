# M4 Task 6: one connect and one pause method (issue #94)

Plan: `docs/superpowers/plans/2026-10-28-milestone-4-api-stability.md`, Task 6.
No new claim: pure API reshaping with no wire change, so the existing
lifecycle claims and their tests cover the behaviour.

`Client::connect_sandbox`, `Client::pause_sandbox` and `Sandbox::pause` now
take their options structs, and the `_with` twins added in M3 are removed:
0.2 has one method per operation (decision 3).

## What changed

- `src/client.rs`: `connect_sandbox(id, &ConnectOptions)` and
  `pause_sandbox(id, &PauseOptions)` carry the bodies of the removed
  `connect_sandbox_with` and `pause_sandbox_with` unchanged (Task 1's v2-only
  connect, Task 3's moded 409 reading). Their rustdoc merges both forms and
  gains an `# Examples` section.
- `src/sandbox.rs`: `Sandbox::pause(&PauseOptions)` replaces `pause()` and
  `pause_with`, with an example.
- `src/lifecycle.rs`: the `PauseOptions` and `ConnectOptions` rustdoc links
  point at the surviving methods.
- `README.md`: the quick start pauses with `&PauseOptions::new()` and
  connects with `&ConnectOptions::new()`; the lifecycle row names the new
  signatures.
- `docs/verification/s4-evidence-map.md`: the CLM-0159 row names the renamed
  test and the new forms.
- The plan's Task 6 has a Corrected note (moved line numbers, the evidence-map
  row, the README `use` line, the changelog slug, the exact grep check).

## Breaking changes

| Old | New |
|---|---|
| `client.connect_sandbox(id, None)` | `client.connect_sandbox(id, &ConnectOptions::new())` |
| `client.connect_sandbox(id, Some(t))` | `client.connect_sandbox(id, &ConnectOptions::new().timeout(t))` |
| `client.connect_sandbox_with(id, &o)` | `client.connect_sandbox(id, &o)` |
| `client.pause_sandbox(id)` | `client.pause_sandbox(id, &PauseOptions::new())` |
| `client.pause_sandbox_with(id, &o)` | `client.pause_sandbox(id, &o)` |
| `sandbox.pause()` | `sandbox.pause(&PauseOptions::new())` |
| `sandbox.pause_with(&o)` | `sandbox.pause(&o)` |

`OnTimeout::pause_with(SnapshotMode)` is a different item and stays.

## Tests

Call sites updated in `tests/rest_api.rs`, `tests/rest_lifecycle.rs`,
`tests/rest_mcp.rs`, `tests/live_lifecycle.rs` and the README; renamed
`sandbox_pause_with_sends_the_mode` to `sandbox_pause_sends_the_mode`. No
test was deleted.

## Live check (hosted E2B)

One at a time, `cargo test --test live_lifecycle -- --ignored --exact`:

- `clm_0014_connect_resumes_paused_sandbox_with_files`: passed
  (run `dev-20261028T134749Z-3429924`).
- `clm_0146_resume_restores_memory_or_reboots`: passed
  (run `dev-20261028T134751Z-3429924`).
