# M3 Task 4: Network settings at create (issue #63)

Plan: `docs/superpowers/plans/2026-10-27-milestone-3-lifecycle-parity.md`, Task 4.
Claims: CLM-0143 and the network part of CLM-0153 (S1); CLM-0156 and the
network part of CLM-0167 (S4).

## What changed

- New module `src/network.rs`, re-exported from the crate root:
  - `ALL_TRAFFIC` (`0.0.0.0/0`, the SDK's `ctx.all_traffic`);
  - `NetworkConfig`, a builder for the create body's `network` object:
    `allow_out`, `deny_out`, `dns_servers`, `rule(host, NetworkRule)`,
    `allow_public_traffic`, `mask_request_host` and `https_ports`, each sent
    only when set, in camelCase. The four egress setters live in a private
    `Egress` struct and the `egress_setters!` macro, so Task 5's
    `NetworkUpdate` can share them;
  - `NetworkRule`, headers set on outbound requests to one host. Its
    `Debug` prints header names only, since values are often credentials;
  - `NetworkInfo`, the `network` object `get_info` reports, with `null`
    lists and maps read as empty.
- `CreateSandboxParams::network(NetworkConfig)`. A `NetworkConfig` with
  nothing set is left out of the body, as the SDK does.
- `SandboxInfo` gains `allow_internet_access: Option<bool>` and
  `network: Option<NetworkInfo>`.
- `tests/live_support`: `http_reply(sandbox, ip)`, the HTTP-reply probe
  `clm_0012` used inline, now shared; `clm_0012` calls it.
- Tests: `tests/rest_network.rs` (6 mock tests), 7 unit tests in
  `src/network.rs`, doctests on `NetworkConfig`, `NetworkRule` and
  `CreateSandboxParams::network`, and live tests
  `clm_0143_create_network_allow_and_deny_lists_are_enforced` and
  `clm_0153_get_info_reports_network`.
- Docs: a "Network" section in `docs/protocol-notes.md`, `network` in its
  "Create body" row, and the CLM-0156 and CLM-0167 rows in
  `docs/verification/s4-evidence-map.md`.

## Plan divergences

Recorded in the plan as a dated correction at Task 4, with Tasks 5 and 6
updated:

- An empty `NetworkConfig` is not sent (`network_body if network_body else
  UNSET` in the SDK); the plan sent `"network": {}`.
- The module doc doesn't link `NetworkUpdate` or
  `Client::update_sandbox_network` yet: they are Task 5's, and the broken
  links would fail `just doc`.
- A `null` rule transform or header map reads as empty.
- Hosted E2B accepts one rule per host (400 for more); the `rule()` doc says
  so.

## Hosted E2B observations (2026-10-27)

- `allowInternetAccess: false` is reported as given, not as a `denyOut`
  entry, so `clm_0153` asserts as the plan wrote it.
- The detail echoes `allowOut`, `dnsServers` and `rules`, header values
  included, and adds `allowPublicTraffic: true` when it was not set.
- A sandbox created without network settings reports `allowInternetAccess:
  null` and `network: null`.

## Verification

- `just lint`, `just test`, `just doc`: green (counts in the PR).
- Live, hosted E2B, one test at a time: `clm_0143`, `clm_0153` and `clm_0012`
  passed (run ids in the PR).
- Review (brooke-reviews): `clm_0153` now also sends a rule for
  `one.one.one.one` and asserts that its header comes back in the detail.
  On hosted E2B, run `dev-20261027T233544Z-1510604`, it passed in 1.78 s.
