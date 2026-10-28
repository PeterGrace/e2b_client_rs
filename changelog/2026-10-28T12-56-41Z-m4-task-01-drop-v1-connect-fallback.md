# M4 Task 1: drop the v1 connect fallback (issue #91)

`connect_sandbox` and `connect_sandbox_with` now make one request,
`POST /v2/sandboxes/{id}/connect`. A 404 there is `Error::SandboxNotFound`;
the crate no longer retries `POST /sandboxes/{id}/connect`. The fallback only
served sandbawks before 0.1.28 (spec decision 5). Backs CLM-0168, which
supersedes CLM-0087. `DEFAULT_SANDBOX_TIMEOUT`, used only for the v1 body, is
gone.

## Breaking changes

- The v1 fallback is gone. Against a server that serves only the v1 connect
  route, `connect_sandbox(id, timeout)` and `connect_sandbox_with(id, options)`
  now return `Error::SandboxNotFound` where they used to connect. No
  signature changes.
- Without a timeout, connect no longer resets the lifetime to 300 s on that
  route (it was a v1-only behaviour).

## Tests

- New: `tests/rest_api.rs::connect_sandbox_v2_404_is_not_found_without_a_v1_request`.
- Changed: `connect_missing_sandbox_is_not_found` (one request, not two),
  `every_rest_request_carries_the_api_key` (8 requests, not 9).
- Removed: `connect_sandbox_falls_back_to_v1_route`,
  `tests/rest_lifecycle.rs::the_v1_fallback_keeps_the_reboot_flag`.
- Live on hosted: `clm_0015_connect_unknown_sandbox_is_not_found` passed (run
  `dev-20261028T125655Z-2453847`).

## Docs

- `docs/protocol-notes.md`: the "Connect v1 (fallback)" row and the
  "Connect v1 fallback" difference are gone; the connect row says a 404 is
  not found with no other route tried.
- `docs/verification/s4-evidence-map.md`: new row CLM-0168; CLM-0087 marked
  superseded; CLM-0073 and CLM-0159 no longer cite the fallback.
