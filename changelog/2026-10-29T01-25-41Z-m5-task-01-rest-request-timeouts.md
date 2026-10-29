# M5 Task 1: Per-call request timeouts on REST calls (issue #125)

Plan: `docs/superpowers/plans/2026-10-28-milestone-5-release.md`, Task 1.
Part of #120 (Task 2 closes it). Backs CLM-0182.

## What changed

- `src/config.rs`: a new public `RequestOptions` (re-exported from the
  crate root), holding only a per-call request timeout, for the calls that
  have nothing else to configure. Unset uses the configured timeout; `None`
  or zero means no limit (M4's `RequestTimeout` rules).
- `src/client.rs`: `Client::request` and `Client::send_json` take the
  call's `RequestTimeout` and resolve it against the configured one, as
  `EnvdClient::unary` already did. Every REST method passes its own.
- A `request_timeout(Option<Duration>)` setter, and a `pub(crate)` field
  that never reaches a body or query, on `CreateSandboxParams`,
  `ConnectOptions`, `ListSandboxesParams`, `MetricsParams`, `PauseOptions`,
  `SnapshotOptions`, `ListSnapshotsParams` and `ForkOptions`.
  `create_sandbox`'s bounds the `POST` only (decision 6); the list params
  keep it across `next_token`.
- `src/sandbox.rs`: the delegating methods take and pass the options;
  `Sandbox::is_running` passes its options' timeout to envd's health check.
  `create_sandbox`'s kill after a failed MCP gateway passes
  `&RequestOptions::new()`. `mcp_token` is unchanged (decision 7).
- Tests: `tests/rest_request_options.rs` (new: override, `None`, zero,
  unset, all 18 `Client` calls, all 11 `Sandbox` calls, next pages); unit
  tests in `src/{config,lifecycle,metrics,models,snapshots}.rs`;
  `RequestOptions` in `tests/api_traits.rs`; the live test
  `clm_0182_a_one_millisecond_request_timeout_times_out_get_info`; every
  caller in `tests/`.
- Docs: rustdoc (Arguments line and examples) on every changed method and
  on the examples in `src/{error,lifecycle,models,network,volumes}.rs`; the
  crate docs' options paragraph; README (Quick start, the snapshot tour,
  the Lifecycle and Network rows, the "Not implemented yet" sentence);
  `docs/api-review.md` (a `RequestOptions` row, "Method shape", "Known
  limits" narrowed to the pid-only calls); `docs/protocol-notes.md` (a
  per-call REST timeout row, the "Not implemented yet" line).
- The plan: a dated correction under Task 1's tests, the argument order of
  `sandbox_from_json_with_client` in Task 2's test, and Task 2's docs step
  now lists the three places Task 1 left saying "REST" only.

## Breaking changes

| Old | New |
|---|---|
| `client.get_sandbox_info(id)` | `client.get_sandbox_info(id, &RequestOptions::new())` |
| `client.kill_sandbox(id)` | `client.kill_sandbox(id, &RequestOptions::new())` |
| `client.set_sandbox_timeout(id, d)` | `client.set_sandbox_timeout(id, d, &RequestOptions::new())` |
| `client.update_sandbox_network(id, &update)` | `client.update_sandbox_network(id, &update, &RequestOptions::new())` |
| `client.refresh_sandbox(id, d)` | `client.refresh_sandbox(id, d, &RequestOptions::new())` |
| `client.delete_snapshot(id)` | `client.delete_snapshot(id, &RequestOptions::new())` |
| `client.create_volume(name)` | `client.create_volume(name, &RequestOptions::new())` |
| `client.get_volume(id)` | `client.get_volume(id, &RequestOptions::new())` |
| `client.list_volumes()` | `client.list_volumes(&RequestOptions::new())` |
| `client.destroy_volume(id)` | `client.destroy_volume(id, &RequestOptions::new())` |
| `sandbox.is_running()` | `sandbox.is_running(&RequestOptions::new())` |
| `sandbox.kill()` | `sandbox.kill(&RequestOptions::new())` |
| `sandbox.set_timeout(d)` | `sandbox.set_timeout(d, &RequestOptions::new())` |
| `sandbox.update_network(&update)` | `sandbox.update_network(&update, &RequestOptions::new())` |
| `sandbox.refresh(d)` | `sandbox.refresh(d, &RequestOptions::new())` |
| `sandbox.get_info()` | `sandbox.get_info(&RequestOptions::new())` |

Not breaking: `create_sandbox`, `connect_sandbox`, `list_sandboxes`,
`get_sandbox_metrics` / `Sandbox::get_metrics`, `pause_sandbox` /
`Sandbox::pause`, `create_snapshot` / `Sandbox::create_snapshot`,
`list_snapshots` / `Sandbox::list_snapshots` and `fork_sandbox` /
`Sandbox::fork` keep their signatures; their structs gain the
`request_timeout` setter.

## Verification

- `just lint`, `just test`, `just doc`: green.
- Hosted E2B, one at a time: `clm_0182`, `clm_0013`, `clm_0015`,
  `clm_0016`, `clm_0027` pass.

## Why

The SDK takes `request_timeout` on every REST call (`ApiParams`,
`e2b/connection_config.py:70-78`); M4 gave it only to the envd calls that
had an options struct (#120). It lands before 0.2.0 so the break is not a
0.3.0 one (M5 decision 4), in the shape of decision 5: one method per
call, a setter where there is already a struct, a trailing
`&RequestOptions` where there is not.
