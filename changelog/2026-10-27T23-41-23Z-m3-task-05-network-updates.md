# M3 Task 5: Network updates (issue #64)

Plan: `docs/superpowers/plans/2026-10-27-milestone-3-lifecycle-parity.md`, Task 5.
Claims: CLM-0144 and the implemented part of CLM-0031 (S1); CLM-0157 (S4).

## What changed

- `src/network.rs`: `NetworkUpdate`, the body of `PUT /sandboxes/{id}/network`
  (`SandboxNetworkUpdateConfig`). It shares the egress setters (`allow_out`,
  `deny_out`, `dns_servers`, `rule`) with `NetworkConfig` through the
  `egress_setters!` macro and adds `allow_internet_access`, which the spec
  spells in snake_case while the egress fields are camelCase. Each field is
  sent only when set; an empty update is sent as `{}`, unlike an empty create
  `network`, which is left out. The module doc now links `NetworkUpdate` and
  `Client::update_sandbox_network`, as Task 4's correction asked.
- `Client::update_sandbox_network(id, &NetworkUpdate)`: 204 is success, 404 is
  `Error::SandboxNotFound`, anything else `Error::Api`.
- `Sandbox::update_network(&NetworkUpdate)` delegates to it.
- `NetworkUpdate` is re-exported from the crate root and named in the crate
  doc.
- Tests: 5 mock tests in `tests/rest_network.rs` (exact body, empty body,
  404, a 400 refusal, the `Sandbox` delegation), 2 unit tests in
  `src/network.rs` (snake_case next to camelCase, `Debug` hides rule header
  values), a doctest on `NetworkUpdate`, and the live test
  `clm_0144_update_network_replaces_egress_without_restart`.
- Docs: the update route, body and replace semantics, and the live
  observation, in the "Network" section of `docs/protocol-notes.md`; the
  CLM-0157 row in `docs/verification/s4-evidence-map.md`.

## Why

The update replaces the whole egress configuration, as the SDK documents:
anything the caller leaves out is cleared. `NetworkUpdate`'s doc says so,
because the natural reading of an "update" is a merge.

## Live result

`clm_0144` passed on hosted E2B as the plan wrote it: an update with only
`allow_internet_access(true)` cleared a create-time `deny_out([ALL_TRAFFIC])`,
`deny_out([ALL_TRAFFIC])` closed egress again, and a file in `/dev/shm` was
still there afterwards, so the sandbox was not restarted.

After review (PR #78): `clm_0144` treats egress as closed only after two
probes in a row get no reply (`live_support::no_http_reply_twice`), and passed
again on hosted E2B with that check.

## Plan divergences

None in the library code. A dated correction at Task 5 records the extra
tests, the module doc's wording and the two-probe check in `clm_0144`. No later task names the new symbols.
