# Milestone 3 (Lifecycle parity) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the S1 gaps between `e2b-client` and the official SDK 2.61 `Sandbox`/`SandboxApi`/`Volume` surface: `get_host`, metrics, the new list filters, network settings at create and on a running sandbox, the lifecycle and idempotency create options, memory modes for pause and connect, snapshots, fork, volumes and volume mounts, and the MCP gateway. Each is backed by mock wire tests and a live test on hosted E2B, and gets a verdict in the claims ledger.

**Architecture:** Every feature is a REST call on `Client` (`src/client.rs`) with a delegating method on `Sandbox` (`src/sandbox.rs`), as in v0.1. New request and response types live in one module per area: `src/metrics.rs`, `src/network.rs`, `src/lifecycle.rs`, `src/snapshots.rs`, `src/volumes.rs` and `src/mcp.rs`. New create options go on `CreateSandboxParams` (`src/models.rs`), and new detail fields on `SandboxInfo`. The MCP gateway is started through the existing `Commands::run`. Each task adds its mock tests in a test file of its own, its live tests to `tests/live_lifecycle.rs`, and its wire facts to `docs/protocol-notes.md` and `docs/verification/s4-evidence-map.md`.

**Tech Stack:** Rust 2024, reqwest 0.13, serde/serde_json, chrono, secrecy, tokio (`sync` for `OnceCell`), uuid 1.29 (`v4`, new in Task 11), wiremock, `just`, claims MCP.

**Design:** `docs/superpowers/specs/2026-10-26-claims-audit-and-publishability-design.md`, sections "Claim hierarchy" (S1, S4) and "Milestones" (M3). As for M2, the spec's "short design" for this milestone is this plan's "Decisions" and "Verified facts" sections: every feature copies an SDK behaviour cited there.

---

## Scope

In:

- **`get_host(port)`** (CLM-0028, 0154): `Sandbox::get_host` and `ConnectionConfig::sandbox_host`.
- **Metrics** (CLM-0029, 0155): `Client::get_sandbox_metrics` and `Sandbox::get_metrics`, with optional `start` and `end`.
- **List filters** (CLM-0151, 0166): `ListSandboxesParams::{order, started_after, template}`.
- **Network at create** (CLM-0143, 0156): `NetworkConfig` with `allow_out`, `deny_out`, `dns_servers`, `allow_public_traffic`, `mask_request_host`, `https_ports` and per-host header rules.
- **Network updates** (CLM-0031, 0144, 0157): `Client::update_sandbox_network` and `Sandbox::update_network` with `NetworkUpdate`.
- **Lifecycle and idempotency at create** (CLM-0145, 0152, 0158): `Lifecycle` (`on_timeout` kill or pause with a `SnapshotMode`, `auto_resume`), with the SDK's validation; `CreateSandboxParams::idempotency_key`.
- **Pause and resume modes** (CLM-0146, 0159): `PauseOptions` (full or filesystem snapshot) and `ConnectOptions` (`OnResume::Restore` or `Reboot`).
- **Snapshots** (CLM-0030, 0147, 0160, 0161): create, list (paged, by sandbox or name) and delete, and creating a sandbox from a snapshot.
- **Fork** (CLM-0030, 0162): `Client::fork_sandbox`, one result per fork.
- **Volumes** (CLM-0148, 0149, 0163, 0164): create, look up, list and destroy volumes; `CreateSandboxParams::volume_mount`.
- **MCP** (CLM-0150, 0165): `CreateSandboxParams::mcp`, the `mcp-gateway` default template, starting the gateway, `Sandbox::mcp_url` and `Sandbox::mcp_token`.
- **Detail read-back** (CLM-0153, 0167): `SandboxInfo` gains `allow_internet_access`, `network`, `lifecycle`, `pause_mode` and `volume_mounts`; each is added by the task that adds the matching create option.
- **The 25 claims CLM-0143 to CLM-0167**, registered while planning and committed with this plan.
- Live runs on hosted E2B and sandbawks, verdicts and the milestone close.

Deferred:

| Item | Claims / issue | Goes to |
|---|---|---|
| The SOCKS5 egress proxy (`egress_proxy`, with its credentials and TLS options) | part of CLM-0031, CLM-0032 | unscheduled (pgrace, 2026-10-27: can't be tested live without a proxy) |
| IAM workload identity (`iam.tokens`) and `${e2b.identity.tokens.<name>}` placeholders in rule transforms | part of CLM-0032 | unscheduled (needs workload identity enabled on the team) |
| Callable `allow_out`/`deny_out` selectors and callable `transform` rules | none | not planned: a Python idiom; callers build the lists themselves, with `ALL_TRAFFIC` for `ctx.all_traffic` |
| The volume content API (list, read, write, stat and remove files in a volume without a sandbox, through the volume token) | none | unscheduled |
| Batch metrics (`GET /sandboxes/metrics?sandbox_ids=`): the SDK has the generated call but no public method uses it | none | not planned |
| Sandbox logs, secrets, templates and git | none | logs, secrets and git unscheduled; template builds are out of scope by design |
| The deprecated `secure` create flag and `keep_memory` aliases | none | not planned: the SDK ignores `secure`, and `keep_memory` is only an alias of `mode` |
| `#[non_exhaustive]` review, error taxonomy, dropping the v1 fallback, naming (including whether the `*_with` options methods of Task 7 should replace the v0.1 signatures, with #54), MSRV | CLM-0119..0121, #30, #52 to #57 | M4 |
| Live CI job, publish dry-run | CLM-0123, 0124 | M5 |
| Re-running the full sandbawks suite with cluster headroom (decides CLM-0118) | #27 | its own issue, not code |

Because the egress proxy and IAM are deferred, CLM-0031 (update network "as the SDK does") and CLM-0032 (every create field SDK 2.61 sends) cannot be fully supported at the end of M3. Task 14 records their implemented parts through the child claims (CLM-0143, 0144, 0145, 0149, 0150, 0152) and leaves the two parents, and with them CLM-0002, at `not_enough_info` with the deferral as the rationale.

## Decisions taken while planning (2026-10-27)

1. **Network surface.** The core lists and flags plus static per-host header rules, at create and in `update_network`. The egress proxy and IAM tokens are deferred: neither can be tested live without a SOCKS5 proxy and workload identity on the team.
2. **MCP.** The full SDK behaviour: send `mcp`, default to the `mcp-gateway` template, start `mcp-gateway --config ...` as root with a generated token (killing the sandbox if that fails), and add `mcp_url` and `mcp_token`.
3. **Memory modes.** All of them: `SnapshotMode` on `create_snapshot`, on pause and in the timeout lifecycle; `OnResume` on connect; `auto_resume`; with the SDK's validation.
4. **Volumes.** Volume mounts and the volume management API (create, get, list, destroy), so the live test can make its own volume. The volume content API is deferred.
5. **Extras found while planning.** SDK 2.61's list filters (`order`, `startedAfter`, `template`) and the detail fields (`network`, `lifecycle`, `pauseMode`, `volumeMounts`, `allowInternetAccess`) are in, as new claims, because S1 covers "list (with paging and filters)" and `get_info`.
6. **Follow-ups.** None carried in. #27 stays out (it needs cluster headroom, not code). #30 and #52 to #57 are M4's.
7. **Ledger.** As in M1 and M2, only the coordinator session writes `claims.json`. The 25 new claims (CLM-0143 to CLM-0167) were added while planning and are committed with this plan.
8. **Merging.** The standing authorization holds: the coordinator merges the plan PR and the sub-issue PRs once CI is green and brooke-reviews' review has been answered.
9. **Execution order.** One issue at a time, oldest first (pgrace's standing rule since 2026-10-27). The graph below shows what could run in parallel if pgrace asks for it.

## Simplifications and deviations relative to the SDK

- **Options methods next to the v0.1 signatures.** `pause_sandbox(id)` and `connect_sandbox(id, timeout)` keep their v0.1 signatures and call the new `pause_sandbox_with(id, &PauseOptions)` and `connect_sandbox_with(id, &ConnectOptions)`. M4 decides whether the options forms replace them (with #54).
- **`auto_pause` stays** as shorthand: `auto_pause(true)` is `on_timeout(OnTimeout::pause())` and `auto_pause(false)` is `on_timeout(OnTimeout::Kill)`. Both write the same `Lifecycle` field, so the last call wins.
- **Lists, not selector callables.** `allow_out` and `deny_out` take lists. `ALL_TRAFFIC` is the SDK's `ctx.all_traffic`.
- **The MCP configuration is a JSON object** (`serde_json::Map`), not the SDK's 2,538-line generated `McpServer` TypedDict. The server sends it on; the crate does not validate server names.
- **`mcp_token` is cached in a `tokio::sync::OnceCell`**, as the SDK caches `_mcp_token`. It is set when `create_sandbox` starts the gateway, and read from `/etc/mcp-gateway/.token` the first time on a handle from `connect_sandbox`.
- **The token read from the file is trimmed at the end** (`trim_end`), where the SDK returns the file as is. The token is a UUID, so trailing whitespace can only be a newline the file was written with, and a token with a newline is unusable in a header and would not equal the token `create_sandbox` generated.
- **Fork returns `Vec<Result<Sandbox>>`.** A per-fork error is `Error::Api { status: code, message }`; the SDK maps the code to an exception class, and a per-fork 404 stays generic in both.
- **No debug mode.** The SDK's `E2B_DEBUG` (`localhost:<port>` hosts, empty metrics) has no equivalent in the crate, so `get_host` and `get_metrics` have no debug branch.
- **Pages, not paginator objects.** `list_snapshots` returns a `SnapshotPage` with `next_token`, like `list_sandboxes` returns a `SandboxPage`.
- **Header values in rules are secrets.** `NetworkTransform`'s `Debug` prints header names only, so an `Authorization` header in a rule never reaches a log (CLM-0122's rule).
- **A volume's token is a `SecretString`**, and `Volume`'s `Debug` leaves it out.
- **An unknown `pauseMode` reads as `None`**, so a value added to the API later doesn't make `get_info` fail. An unknown `onTimeout` reads as `TimeoutAction::Unknown`, like `SandboxState::Unknown`.

## Verified facts the code relies on

SDK sources are e2b 2.61.0, the same packages as `docs/protocol-notes.md` (`py:` paths relative to `e2b/`, `js:` lines of `package/dist/index.js`). Crate sources are under `~/.cargo/registry/src/index.crates.io-*/`.

| Fact | Source |
|---|---|
| `get_host(port)` is `<port>-<sandbox_id>.<sandbox_domain>`; `sandbox_domain` is the API's `domain` or the configured domain; `E2B_SANDBOX_URL` doesn't affect it | `py: connection_config.py:366-379`; `py: sandbox/main.py:46, 213-224`; `js: 1138-1141, 6045-6047` |
| MCP: port 50005, default template `mcp-gateway` (used only when no template is given), URL `https://<get_host(50005)>/mcp` | `py: sandbox/main.py:28-33, 226-232`; `js: 5873, 6201-6203, 6324` |
| MCP start: after create, token = a random UUID v4, then `commands.run("mcp-gateway --config " + shlex.quote(json.dumps(mcp)), user="root", envs={"GATEWAY_ACCESS_TOKEN": token})`; on any failure the sandbox is killed (kill errors ignored); a non-zero exit becomes "Failed to start MCP gateway: <stderr>" | `py: sandbox_async/main.py:244-268`; `js: 5884-5893` |
| Shell quoting (both SDKs): empty → `''`; a string of only `[A-Za-z0-9_@%+=:,./-]` stays as is; otherwise `'` + the string with each `'` replaced by `'"'"'` + `'` | Python 3 `shlex.quote` source; `js: 268, 277-281` |
| `get_mcp_token`: returns the cached token, else reads `/etc/mcp-gateway/.token` as root | `py: sandbox_async/main.py:1063-1073`; `js: 6211` |
| Metrics: `GET /sandboxes/{id}/metrics?start&end`, both Unix seconds, each omitted when unset; 200 → array; 404 → not found; fields `timestamp` (RFC 3339), `timestampUnix`, `cpuCount`, `cpuUsedPct` (float), `memUsed`, `memTotal`, `memCache`, `diskUsed`, `diskTotal` (bytes) | `py: api/client/api/sandboxes/get_sandboxes_sandbox_id_metrics.py:21-29, 39-60`; `py: api/client/models/sandbox_metric.py:28-36, 79-95`; `py: sandbox_async/sandbox_api.py:280-336`; `js: 5346-5378` |
| List filters: `order` (`asc`/`desc`), `startedAfter` (`datetime.isoformat()` of an aware time, e.g. `2026-10-27T12:00:00+00:00`), `template` (ID or name); each omitted when unset | `py: api/client/api/sandboxes/get_v2_sandboxes.py:14-58`; `py: sandbox/sandbox_api.py:1320-1340`; `py: sandbox_async/paginator.py:55-90` |
| Create network object: `allowOut`, `denyOut`, `dnsServers`, `egressProxy`, `allowPublicTraffic`, `maskRequestHost`, `httpsPorts`, `rules` (camelCase); `dnsServers: []` is sent (it means "no DNS server"); a field is sent only when the caller set it; a `network` with nothing set is left out of the body (corrected, issue #63) | `py: api/client/models/sandbox_network_config.py:51-120`; `py: sandbox/sandbox_api.py:928-996`; `py: sandbox_async/sandbox_api.py:250` |
| A rule list per host: `rules: {"<host>": [{"transform": {"headers": {...}}}]}`; registering a host doesn't allow egress to it | `py: sandbox/sandbox_api.py:149-224`; `py: api/client/models/sandbox_network_rule.py:46`; `py: api/client/models/sandbox_network_transform.py:51` |
| `ALL_TRAFFIC` is `0.0.0.0/0`; `allow_internet_access: false` is the same as `deny_out: ["0.0.0.0/0"]` | `py: sandbox/network.py:1-8`; `py: sandbox_async/main.py:197` |
| Network update: `PUT /sandboxes/{id}/network`, 204; 404 → not found; body `allowOut`, `denyOut`, `dnsServers`, `egressProxy`, `rules`, `allow_internet_access` (snake_case); the update replaces the whole egress configuration, so omitted fields are cleared | `py: api/client/api/sandboxes/put_sandboxes_sandbox_id_network.py:21-55`; `py: api/client/models/sandbox_network_update_config.py:50-103`; `py: sandbox/sandbox_api.py:515-561, 1128-1152`; `py: sandbox_async/sandbox_api.py:190-209`; `js: 5420` |
| Create lifecycle: `autoPause` only when `on_timeout` is set (true for pause); `autoPauseMemory` only for a pause with a mode (`full` → true, `filesystem` → false); `autoResume: {enabled}` when `auto_resume` is set | `py: sandbox/sandbox_api.py:1046-1125`; `py: api/client/models/new_sandbox_v2.py:112-132`; `py: api/client/models/sandbox_auto_resume_config.py:18, 37` |
| Lifecycle validation, all before the request: `auto_resume: true` needs `on_timeout` pause; `auto_resume: true` is refused with mode `filesystem`; mode only with pause | `py: sandbox/sandbox_api.py:1095-1114` |
| `SnapshotMode` is `full` or `filesystem`; `memory = (mode == "full")`; unset → field omitted, the API default (full) applies | `py: sandbox/sandbox_api.py:627-662` |
| Pause: `POST /sandboxes/{id}/pause` body `{memory?}`; 404 → not found; 409 → already paused | `py: sandbox_async/sandbox_api.py:399-432`; `py: api/client/models/sandbox_pause_request.py:22-39`; `js: 5442-5446` |
| Connect v2 body `{timeout?, memory?}`; `on_resume` `restore` (default, omitted) or `reboot` (`memory: false`); the v1 body also has an optional `memory` | `py: sandbox/sandbox_api.py:761-791`; `py: api/client/models/connect_sandbox_v2.py:25-49`; `py: api/client/models/connect_sandbox.py:48-50`; `js: 5609-5621` |
| Idempotency key: `Idempotency-Key` header on `POST /v2/sandboxes`, only when given; 1-255 chars of `[a-z0-9-]`, else `InvalidArgument` before any request | `py: api/client/api/sandboxes/post_v2_sandboxes.py:17-25`; `py: retry.py:29-44`; `js: 5568` |
| Snapshot create: `POST /sandboxes/{id}/snapshots` body `{name?, memory?}`; 201 → `{snapshotID, names}`; 404 → not found; a filesystem-only snapshot can be refused with 400 (`snapshot_filesystem_only_disabled`) or 409 (`snapshot_filesystem_only_unsupported_node`) | `py: api/client/api/sandboxes/post_sandboxes_sandbox_id_snapshots.py:22-57`; `py: api/client/models/sandbox_snapshot_request.py:24-48`; `py: api/client/models/snapshot_info.py:21-46`; `py: sandbox_async/sandbox_api.py:339-374`; `py: sandbox_async/main.py:886-933`; `js: 5484` |
| A sandbox is created from a snapshot by passing its `snapshot_id` as the template | `py: sandbox_async/main.py:886-900`; `py: sandbox/sandbox_api.py:1364-1370` |
| Snapshot list: `GET /snapshots?sandboxID&name&limit&nextToken` (each omitted when unset), 200 → `[{snapshotID, names}]`, next page in `x-next-token` | `py: api/client/api/snapshots/get_snapshots.py:22-34`; `py: sandbox_async/paginator.py:103-160`; `py: paginator.py:48-50`; `js: 5721` |
| Snapshot delete: `DELETE /templates/{id}` with the id percent-encoded as one path segment; 204 → true; 404 → false | `py: api/client/api/templates/delete_templates_template_id.py:16-37`; `py: sandbox_async/sandbox_api.py:377-396`; `py: api/__init__.py:32-40` |
| Fork: `POST /sandboxes/{id}/fork` body `{timeout?, count?}`; `count` an integer 1 to 20 (`MAX_FORK_COUNT`), checked before the request; 201 → `[{sandbox?, error?: {code, message}}]`, one per fork; a whole-request 404 → not found | `py: api/client/api/sandboxes/post_sandboxes_sandbox_id_fork.py:22-66`; `py: api/client/models/sandbox_fork_request.py:22-46`; `py: api/client/models/sandbox_fork_result.py:27-63`; `py: sandbox/sandbox_api.py:81-118`; `py: sandbox_async/sandbox_api.py:435-495`; `js: 5137-5141, 5579-5604` |
| Volumes: `POST /volumes {name}` 201 → `{volumeID, name, token, domain?}`; `GET /volumes` 200 → `[{volumeID, name}]`; `GET /volumes/{id}` 200 → `{volumeID, name, token, domain?}`, 404 → not found; `DELETE /volumes/{id}` 204 → true, 404 → false | `py: api/client/api/volumes/post_volumes.py:21-52`; `py: api/client/api/volumes/get_volumes.py:15-42`; `py: api/client/api/volumes/get_volumes_volume_id.py:17-43`; `py: api/client/api/volumes/delete_volumes_volume_id.py:16-41`; `py: api/client/models/volume_and_token.py:50-63`; `py: volume/volume_async.py:133-285` |
| Volume mounts: `volumeMounts: [{name, path}]` in the create body, omitted when empty; returned in detail and list entries | `py: sandbox_async/main.py:213-222`; `py: sandbox_async/sandbox_api.py:252`; `py: api/client/models/sandbox_volume_mount.py:18-43`; `py: api/client/models/sandbox_detail.py:242`; `py: api/client/models/listed_sandbox.py:140` |
| Detail extras: `allowInternetAccess` (bool or null), `network` (the create network shape), `lifecycle {onTimeout: pause\|kill, autoResume: bool}`, `pauseMode` (`full`\|`filesystem`), `volumeMounts` | `py: api/client/models/sandbox_detail.py:51-69, 174-250`; `py: api/client/models/sandbox_lifecycle.py:44-46`; `py: api/client/models/sandbox_pause_mode.py:5-6` |
| `secure` is accepted and ignored (every sandbox is secured) | `py: sandbox_async/main.py:177, 197` |
| reqwest: `RequestBuilder::header(name, value)` and `query(&T: Serialize)` (feature `query`, already enabled) | `reqwest-0.13.5/src/async_impl/request.rs:197, 366`; `Cargo.toml` features |
| `url::Url::path_segments_mut().extend(..)` percent-encodes `/`, `%`, space and `?` in a segment but not `:`; servers decode both (the crate's `Client::request` already relies on it; `tests/rest_api.rs` has no `:` case) | `url-2.5.8/src/parser.rs:20-38, 1210-1212`; `url-2.5.8/src/path_segments.rs:236-256`; `src/client.rs:401-413` |
| chrono 0.4.45 with `std`: `DateTime::timestamp() -> i64`, `timestamp_subsec_micros()` and `to_rfc3339_opts(SecondsFormat, use_z) -> String` (`use_z: false` gives `+00:00` for UTC, as Python's `isoformat`; `Secs` or `Micros` matches its fraction, where plain `to_rfc3339` may write 3 or 9 digits) | `chrono-0.4.45/src/datetime/mod.rs:200, 634`; `Cargo.toml:59, 96` |
| tokio 1.54.1 `sync::OnceCell` (feature `sync`, already enabled): `get`, `set`, `get_or_try_init` | `tokio-1.54.1/src/sync/mod.rs:452-492`; `tokio-1.54.1/src/sync/once_cell.rs:277, 400` |
| uuid 1.29.0: `Uuid::new_v4()` behind feature `v4`; `Display` is the lowercase hyphenated form; needs rustc 1.89 (here 1.97.1) | `uuid-1.29.0/src/v4.rs:33`; `uuid-1.29.0/src/fmt.rs:31-35`; `uuid-1.29.0/Cargo.toml:14, 113, 134` |
| secrecy 0.10.3: `SecretString: From<String>`, `Debug` prints `[REDACTED]` (already used for the envd token) | `secrecy-0.10.3/src/lib.rs:141-145, 214`; `src/sandbox.rs:212-224` |
| The crate today: `Client::{request, send_json}` (private), `parse_success`, `parse_sandbox_response`, `ensure_found_and_success`, `HEADER_NEXT_TOKEN`; `CreateSandboxParams::body()` is infallible and `auto_pause` is an `Option<bool>`; `SandboxInfo` derives `Eq` and is `#[non_exhaustive]`; `Sandbox` is not `Clone`; `Error` is `#[non_exhaustive]` with `is_not_found()`; `CommandResult { stdout, stderr, exit_code, error }`; `CommandOptions::{env, user}` | `src/client.rs:14-27, 400-470`; `src/models.rs:31-200, 255-290`; `src/sandbox.rs:187-230`; `src/error.rs:25-127`; `src/commands.rs:40-120, 185-194` |
| Mock helpers: `client_for`, `sandbox_json`, `sandbox_from_json`, `stream_response`, `message`, `end_stream`, `decode_request_envelope`, `API_KEY`, `SANDBOX_ID`; live helpers `LiveSandbox::{create, new, client}`, `params`, `template`, `run_id`, `poll_until`, `RUN_METADATA_KEY` | `tests/common/mod.rs:15-125`; `tests/live_support/mod.rs:30-225` |
| sandbawks 0.1.28 serves none of network update, metrics, snapshots, fork or volumes (tracked in gfd/sandbawks #664 to #672 under #673) | catalog `projects/sandbawks.md`, 2026-10-26 |

## Conventions for every task

- **Gates:** `just lint` (fmt check plus `clippy --all-targets -D warnings`), `just test` and `just doc` must pass before pushing. Feature tasks (1 to 11) also run their own new live tests against hosted E2B before pushing, one test at a time:
  `E2B_TEST_TARGET=hosted E2B_TEST_RUN_ID=dev-$(date -u +%Y%m%dT%H%M%SZ)-$$ cargo test --test live_lifecycle -- --ignored --exact <test name>`.
  Report the result in the PR. The recorded runs are Tasks 12 and 13.
- **Tests:** mock tests use `wiremock` through `tests/common`. Each feature task creates its own mock test file (named in its Files list), to keep conflicts out of `tests/rest_api.rs`. Each new type a live test uses goes into the `use e2b_client::{...}` list at the top of `tests/live_lifecycle.rs`. Live tests are `#[tokio::test]` plus `#[ignore = "live: run with `just test-live <target>`"]`, named `clm_NNNN_<slug>`, one claim each, Arrange-Act-Assert. Use `.expect("<what>")` in tests, never `.unwrap()`. No `println!`/`dbg!`.
- **Cost and leaks:** every sandbox a live test makes is a `LiveSandbox` (forks are adopted into one, Task 9). Snapshots and volumes are named with `live_support::resource_name(slug)` (Task 8 adds it), deleted by the test, and swept by `tests/live_sweep.rs` (Tasks 8 and 10 extend it).
- **Docs per task:** every new public item gets a rustdoc comment with Arguments/Returns/Errors and an example where useful (CLAUDE.md). Every wire fact the task implements gets a row in `docs/protocol-notes.md`: REST rows go in the "REST endpoints" table; Tasks 4, 5, 8, 10 and 11 add the sections "Network", "Snapshots and fork", "Volumes" and "MCP gateway". Each S4 claim (CLM-0154 to CLM-0167) gets a row in `docs/verification/s4-evidence-map.md`, and so does CLM-0152: it is an S1 claim (a child of CLM-0032), but it is verified by mock tests, as CLM-0122 is. Append rows, and extend the intro line to "CLM-0152 (an S1 claim verified by tests), and CLM-0154 to CLM-0167".
- **Ledger:** subagents never call `mcp__claims__*` write tools. Read tools are fine.
- **Changelog:** one entry per task, `changelog/<UTC ts>-m3-task-<NN>-<slug>.md`, timestamp from `date -u +%Y-%m-%dT%H-%M-%SZ`.
- **Logs:** live run logs go to `target/live-logs/` inside your own worktree. Never write them to a shared scratchpad.
- **Processes:** never kill processes by pattern; kill only your own PIDs.
- **`Cargo.lock`:** diff it before pushing; a dependency change must not downgrade unrelated crates (an M2 lesson).
- The code in this plan has not been compiled. If an implementation has to differ, fix this plan in the same PR (inline, or with a dated `> **Corrected (issue #N).**` note), then grep later tasks for the changed symbol.

## File map

| File | Task |
|---|---|
| `claims.json` | plan PR; Task 14 |
| `Cargo.toml`, `Cargo.lock` | 11 (uuid) |
| `src/config.rs` | 1 (`sandbox_host`) |
| `src/sandbox.rs` | 1 (`get_host`), 2 (`get_metrics`), 5 (`update_network`), 7 (`pause_with`), 8 (`create_snapshot`, `list_snapshots`), 9 (`fork`), 11 (`mcp_url`, `mcp_token`, the token cell) |
| `src/client.rs` | 2, 3 (`next_token_of`), 5, 6 (idempotency header, fallible body), 7, 8, 9, 10, 11 |
| `src/models.rs` | 3 (list params), 4 (`network`, info fields), 6 (`lifecycle`, `idempotency_key`, info fields), 7 (connect and pause bodies), 9 (fork wire types), 10 (`volume_mount`, info field), 11 (`mcp`, optional template) |
| `src/metrics.rs` (new) | 2 |
| `src/network.rs` (new) | 4, 5 |
| `src/lifecycle.rs` (new) | 6, 7 |
| `src/snapshots.rs` (new) | 8 |
| `src/volumes.rs` (new) | 10 |
| `src/mcp.rs` (new) | 11 |
| `src/error.rs` | 10 (`VolumeNotFound`), 11 (`McpGateway`) |
| `src/lib.rs` (modules, re-exports, crate docs) | 2, 3, 4, 5, 6, 7, 8, 9, 10, 11 |
| `tests/rest_metrics.rs` (new) | 2 |
| `tests/rest_list.rs` (new) | 3 |
| `tests/rest_network.rs` (new) | 4, 5 |
| `tests/rest_lifecycle.rs` (new) | 6, 7 |
| `tests/rest_snapshots.rs` (new) | 8 |
| `tests/rest_fork.rs` (new) | 9 |
| `tests/rest_volumes.rs` (new) | 10 |
| `tests/rest_mcp.rs` (new) | 11 |
| `tests/rest_api.rs` (existing assertions touched by refactors only) | 6, 7 |
| `tests/live_support/mod.rs` | 4 (`http_reply`), 8 (`resource_name`, `params_from`), 9 (`LiveSandbox::adopt`) |
| `tests/live_lifecycle.rs` | 1 to 11 |
| `tests/live_sweep.rs` | 8 (snapshots), 10 (volumes) |
| `docs/protocol-notes.md`, `docs/verification/s4-evidence-map.md` | 1 to 11, 14 |
| `docs/verification/runs/<run id>.log`, `docs/verification/m3-live-results.md` | 12, 13 |
| `README.md`, milestone changelog | 14 |

## Task dependencies

```
4 -> 5
6 -> 7
6 -> 8
1 -> 11
1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11 -> 12
1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11 -> 13
12, 13 -> 14
```

Tasks 1, 2, 3, 4, 6, 9 and 10 can start straight away. Task 5 needs Task 4's `network.rs` (the shared egress fields and the live probe); Tasks 7 and 8 need Task 6's `SnapshotMode`; Task 11 needs Task 1's `get_host` for `mcp_url`. Every feature task touches `src/client.rs`, `src/models.rs`, `src/lib.rs`, `tests/live_lifecycle.rs` and the two docs files. Each of those changes is an append, or a new field and its setter: on a conflict, keep both sides. Tasks 6 and 11 both rework `CreateSandboxParams` (the fallible body, the optional template); whichever lands second merges carefully and re-runs `tests/rest_api.rs`. Tasks 12 and 13 run in parallel and each adds one section to `docs/verification/m3-live-results.md`; keep both. Task 14 is the coordinator's.

---
### Task 1: `get_host` (CLM-0028, 0154)

**Files:**
- Modify: `src/config.rs` (`sandbox_host` free function, new `ConnectionConfig::sandbox_host`)
- Modify: `src/sandbox.rs` (`Sandbox::get_host`, unit tests)
- Modify: `tests/live_lifecycle.rs`
- Modify: `docs/protocol-notes.md` ("envd addressing" gains a `get_host` row), `docs/verification/s4-evidence-map.md` (CLM-0154)

- [ ] **Step 1: Write the failing unit tests**

In `src/config.rs`'s `mod tests`:

```rust
    #[test]
    fn sandbox_host_uses_the_sandbox_domain_then_the_configured_one() {
        let config = ConnectionConfig::builder()
            .api_key("k")
            .domain("example.com")
            .sandbox_url("https://ignored.test")
            .build_with_env(|_| None)
            .expect("config");

        // E2B_SANDBOX_URL does not change get_host, as in the SDKs.
        assert_eq!(config.sandbox_host("sbx1", None, 3000), "3000-sbx1.example.com");
        assert_eq!(config.sandbox_host("sbx1", Some("e2b.app"), 8080), "8080-sbx1.e2b.app");
    }
```

In `src/sandbox.rs`'s `mod tests` (it already has `sandbox_on(domain)` on the client domain `configured.test`):

```rust
    #[test]
    fn get_host_follows_the_reported_domain() {
        assert_eq!(sandbox_on(Some("e2b.app")).get_host(3000), "3000-sbx.e2b.app");
        assert_eq!(sandbox_on(None).get_host(8080), "8080-sbx.configured.test");
    }
```

- [ ] **Step 2: Run them to verify they fail**

> **Corrected (issue #60, 2026-10-27).** Cargo takes one positional test filter, so `cargo test --lib sandbox_host get_host` is rejected with a usage error; several filters go after `--` (fixed here, in Step 4 and in Task 7 Step 2). The "envd addressing" table has two columns (Fact, Citation), so the Step 6 row puts the format in the Fact cell after a colon.

Run: `cargo test --lib -- sandbox_host get_host`
Expected: compile error, no method `sandbox_host` / `get_host`.

- [ ] **Step 3: Implement**

In `src/config.rs`, replace the free function at the end of the `impl ConnectionConfig` section:

```rust
/// `<port>-<sandbox_id>.<domain>` (`e2b/connection_config.py:366-379`).
fn host_name(port: u16, sandbox_id: &str, domain: &str) -> String {
    format!("{port}-{sandbox_id}.{domain}")
}
```

and change its two callers in `envd_rpc_url` and `envd_direct_url` to `host_name(ENVD_PORT, sandbox_id, domain)`. Add to `impl ConnectionConfig`:

```rust
    /// The host name that reaches `port` inside a sandbox from outside it,
    /// for an HTTP or WebSocket client.
    ///
    /// Mirrors `ConnectionConfig.get_host` (`e2b/connection_config.py:366-379`;
    /// JS `dist/index.js:1138-1141`): `<port>-<sandbox_id>.<domain>`, with the
    /// domain the API reported for the sandbox, else this configuration's
    /// domain. `E2B_SANDBOX_URL` does not change it, as in the SDKs.
    ///
    /// # Arguments
    ///
    /// * `sandbox_id` - the sandbox ID.
    /// * `sandbox_domain` - the domain the API returned for the sandbox, if any.
    /// * `port` - the port a service listens on inside the sandbox.
    ///
    /// # Returns
    ///
    /// The host name, without a scheme.
    ///
    /// # Examples
    ///
    /// ```
    /// use e2b_client::ConnectionConfig;
    ///
    /// let config = ConnectionConfig::builder().api_key("k").domain("example.com").build()?;
    /// assert_eq!(config.sandbox_host("sbx1", None, 3000), "3000-sbx1.example.com");
    /// # Ok::<(), e2b_client::Error>(())
    /// ```
    #[must_use]
    pub fn sandbox_host(&self, sandbox_id: &str, sandbox_domain: Option<&str>, port: u16) -> String {
        host_name(port, sandbox_id, sandbox_domain.unwrap_or(&self.domain))
    }
```

In `src/sandbox.rs`, after `traffic_access_token`:

```rust
    /// The host name that reaches `port` inside this sandbox, for example to
    /// open `https://<host>/` on a web server the sandbox runs. The SDKs'
    /// `get_host(port)`.
    ///
    /// When the sandbox was created with public traffic disallowed, requests
    /// to the host also need [`Sandbox::traffic_access_token`].
    ///
    /// # Arguments
    ///
    /// * `port` - the port the service listens on inside the sandbox.
    ///
    /// # Returns
    ///
    /// The host name, without a scheme, e.g. `3000-<sandbox id>.e2b.app`.
    #[must_use]
    pub fn get_host(&self, port: u16) -> String {
        self.client
            .config()
            .sandbox_host(&self.sandbox_id, self.domain.as_deref(), port)
    }
```

- [ ] **Step 4: Run the tests**

Run: `cargo test --lib -- sandbox_host get_host` and `cargo test --doc sandbox_host`
Expected: PASS.

- [ ] **Step 5: Live test**

Add to `tests/live_lifecycle.rs`:

```rust
#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0028_get_host_reaches_a_port_in_the_sandbox() {
    // Arrange: a web server on port 8000 serving one known file. The handle
    // is kept so the server stays attached for the whole test.
    let sandbox = LiveSandbox::new().await;
    sandbox
        .files()
        .write("/tmp/www/hello.txt", "hello from 8000", None)
        .await
        .expect("write");
    let _server = sandbox
        .commands()
        .start(
            "cd /tmp/www && python3 -m http.server 8000",
            &CommandOptions::default(),
        )
        .await
        .expect("start server");
    let url = format!("https://{}/hello.txt", sandbox.get_host(8000));
    let http = reqwest::Client::new();

    // Act: `async move` copies the two references into each future.
    let (http, url) = (&http, url.as_str());
    let body = poll_until("the server on port 8000", Duration::from_secs(60), || async move {
        let response = http.get(url).send().await.ok()?;
        if !response.status().is_success() {
            return None;
        }
        response.text().await.ok()
    })
    .await;

    // Assert
    assert_eq!(body, "hello from 8000");
}
```

Run it on hosted (Conventions). Expected: PASS.

- [ ] **Step 6: Docs**

`docs/protocol-notes.md`, "envd addressing" table: a row "`get_host(port)`: `<port>-<sandbox_id>.<sandbox domain or configured domain>`; not changed by `E2B_SANDBOX_URL` | `py: connection_config.py:366-379`; `py: sandbox/main.py:46, 213-224`; `js: 1138-1141`". Evidence map row: `| CLM-0154 | src/config.rs::tests::sandbox_host_uses_the_sandbox_domain_then_the_configured_one; src/sandbox.rs::tests::get_host_follows_the_reported_domain | yes | new | The config test pins `<port>-<id>.<configured domain>` with no sandbox domain, the sandbox domain when given, and that `sandbox_url` does not change the host; the sandbox test checks the same through `Sandbox::get_host` for a handle with and without a reported domain. |`.

- [ ] **Step 7: Gates, changelog, commit**

Run `just lint`, `just test`, `just doc`. Write `changelog/<ts>-m3-task-01-get-host.md`.

```bash
git add src/config.rs src/sandbox.rs tests/live_lifecycle.rs docs/ changelog/
git commit -m "feat: Sandbox::get_host and ConnectionConfig::sandbox_host"
```

---

### Task 2: Metrics (CLM-0029, 0155)

**Files:**
- Create: `src/metrics.rs`
- Modify: `src/client.rs` (`get_sandbox_metrics`), `src/sandbox.rs` (`get_metrics`), `src/lib.rs` (module, re-exports)
- Create: `tests/rest_metrics.rs`
- Modify: `tests/live_lifecycle.rs`, `docs/protocol-notes.md` ("REST endpoints"), `docs/verification/s4-evidence-map.md`

- [ ] **Step 1: Write the failing mock tests**

`tests/rest_metrics.rs`:

```rust
//! `GET /sandboxes/{id}/metrics` against a mock server.

mod common;

use chrono::{DateTime, Utc};
use common::{SANDBOX_ID, client_for};
use e2b_client::{Error, MetricsParams};
use serde_json::json;
use wiremock::matchers::{method, path, query_param, query_param_is_missing};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn metric_json() -> serde_json::Value {
    json!({
        "timestamp": "2026-10-27T12:00:05Z",
        "timestampUnix": 1_793_102_405,
        "cpuCount": 2,
        "cpuUsedPct": 12.5,
        "memUsed": 104_857_600,
        "memTotal": 536_870_912,
        "memCache": 1_048_576,
        "diskUsed": 1_073_741_824,
        "diskTotal": 2_147_483_648_u64
    })
}

#[tokio::test]
async fn get_metrics_sends_start_and_end_as_unix_seconds() {
    // Arrange
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/sandboxes/{SANDBOX_ID}/metrics")))
        .and(query_param("start", "1793102400"))
        .and(query_param("end", "1793102460"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([metric_json()])))
        .expect(1)
        .mount(&server)
        .await;
    let start: DateTime<Utc> = "2026-10-27T12:00:00Z".parse().expect("start");
    let end: DateTime<Utc> = "2026-10-27T12:01:00Z".parse().expect("end");

    // Act
    let metrics = client_for(&server)
        .get_sandbox_metrics(SANDBOX_ID, &MetricsParams::new().start(start).end(end))
        .await
        .expect("metrics");

    // Assert
    assert_eq!(metrics.len(), 1);
    let sample = &metrics[0];
    assert_eq!(sample.timestamp, "2026-10-27T12:00:05Z".parse::<DateTime<Utc>>().expect("ts"));
    assert_eq!(sample.cpu_count, 2);
    assert!((sample.cpu_used_pct - 12.5).abs() < f64::EPSILON);
    assert_eq!(sample.mem_used, 104_857_600);
    assert_eq!(sample.mem_total, 536_870_912);
    assert_eq!(sample.mem_cache, 1_048_576);
    assert_eq!(sample.disk_used, 1_073_741_824);
    assert_eq!(sample.disk_total, 2_147_483_648);
}

#[tokio::test]
async fn get_metrics_without_a_range_sends_no_query() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/sandboxes/{SANDBOX_ID}/metrics")))
        .and(query_param_is_missing("start"))
        .and(query_param_is_missing("end"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .expect(1)
        .mount(&server)
        .await;

    let metrics = client_for(&server)
        .get_sandbox_metrics(SANDBOX_ID, &MetricsParams::default())
        .await
        .expect("metrics");

    assert!(metrics.is_empty());
}

#[tokio::test]
async fn get_metrics_of_a_missing_sandbox_is_not_found() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/sandboxes/{SANDBOX_ID}/metrics")))
        .respond_with(ResponseTemplate::new(404).set_body_json(json!({"code": 404, "message": "nope"})))
        .mount(&server)
        .await;

    let error = client_for(&server)
        .get_sandbox_metrics(SANDBOX_ID, &MetricsParams::default())
        .await
        .expect_err("not found");

    assert!(matches!(error, Error::SandboxNotFound(ref id) if id == SANDBOX_ID), "{error:?}");
}
```

`tests/rest_metrics.rs` uses `chrono`, which is a normal dependency of the crate, so integration tests can use it.

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test --test rest_metrics`
Expected: compile error, `MetricsParams` and `get_sandbox_metrics` not found.

- [ ] **Step 3: Implement**

> **Corrected (issue #61, 2026-10-27).** The `MetricsParams` doctest called
> `Utc::now()`, which needs chrono's `clock` feature; the crate builds chrono
> with `serde` and `std` only, so the doctest failed to compile (E0599). It now
> parses a fixed time, as below. Any later task that wants the current time in
> library code or a doctest must not use `Utc::now()` either (use
> `std::time::SystemTime`, or add `clock` deliberately). Additions beyond this
> step: unit tests `query_without_a_range_is_empty` and
> `query_sends_each_set_bound_as_unix_seconds` in `src/metrics.rs`, and a
> `no_run` example on `Client::get_sandbox_metrics`, and a fourth mock test,
> `get_metrics_with_a_null_body_is_empty` (a 200 with `null` is an empty
> list); the CLM-0155 row names them with the mock tests. "Oldest first" became "in the order the API
> returns them": neither SDK documents an order.

`src/metrics.rs`:

```rust
//! Sandbox resource metrics (`GET /sandboxes/{id}/metrics`).

use chrono::{DateTime, Utc};
use serde::Deserialize;

/// One sample of a sandbox's CPU, memory and disk use
/// (`SandboxMetric`, `e2b/api/client/models/sandbox_metric.py:28-36`).
///
/// This is a plain data record, so its fields are public. It is
/// `#[non_exhaustive]` so fields can be added without a breaking change.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct SandboxMetrics {
    /// When the sample was taken.
    pub timestamp: DateTime<Utc>,
    /// Number of vCPUs.
    pub cpu_count: u32,
    /// CPU use, in percent.
    pub cpu_used_pct: f64,
    /// Memory in use, in bytes.
    pub mem_used: u64,
    /// Total memory, in bytes.
    pub mem_total: u64,
    /// Page cache, in bytes.
    pub mem_cache: u64,
    /// Disk in use, in bytes.
    pub disk_used: u64,
    /// Total disk, in bytes.
    pub disk_total: u64,
}

/// The time range for [`crate::Client::get_sandbox_metrics`]. With neither
/// bound set, the API returns samples from the sandbox's start to now.
///
/// # Examples
///
/// ```
/// use chrono::{DateTime, Duration, Utc};
/// use e2b_client::MetricsParams;
///
/// // The crate builds chrono without its `clock` feature, so this example
/// // parses a fixed time rather than calling `Utc::now()`.
/// let end: DateTime<Utc> = "2026-10-27T12:05:00Z".parse().expect("a valid time");
/// let params = MetricsParams::new().start(end - Duration::minutes(5)).end(end);
/// # let _ = params;
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MetricsParams {
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
}

impl MetricsParams {
    /// No range: the sandbox's whole life so far.
    ///
    /// # Returns
    ///
    /// Empty parameters.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Only samples at or after `start`. Sent in whole Unix seconds.
    ///
    /// # Arguments
    ///
    /// * `start` - the earliest sample time.
    ///
    /// # Returns
    ///
    /// The parameters, for chaining.
    #[must_use]
    pub fn start(mut self, start: DateTime<Utc>) -> Self {
        self.start = Some(start);
        self
    }

    /// Only samples at or before `end`. Sent in whole Unix seconds.
    ///
    /// # Arguments
    ///
    /// * `end` - the latest sample time.
    ///
    /// # Returns
    ///
    /// The parameters, for chaining.
    #[must_use]
    pub fn end(mut self, end: DateTime<Utc>) -> Self {
        self.end = Some(end);
        self
    }

    /// The query pairs (`get_sandboxes_sandbox_id_metrics.py:21-23`).
    pub(crate) fn query(&self) -> Vec<(&'static str, i64)> {
        // `filter_map` drops the unset bounds, like the SDK's `UNSET`.
        [("start", self.start), ("end", self.end)]
            .into_iter()
            .filter_map(|(name, time)| time.map(|time| (name, time.timestamp())))
            .collect()
    }
}
```

In `src/client.rs`, after `get_sandbox_info`:

```rust
    /// Reads a sandbox's CPU, memory and disk samples
    /// (`GET /sandboxes/{id}/metrics`).
    ///
    /// # Arguments
    ///
    /// * `sandbox_id` - the sandbox.
    /// * `params` - the optional time range.
    ///
    /// # Returns
    ///
    /// The samples, in the order the API returns them; empty if there are
    /// none yet.
    ///
    /// # Errors
    ///
    /// [`Error::SandboxNotFound`] for HTTP 404, [`Error::Api`] for other
    /// non-2xx answers, [`Error::Decode`] for an unexpected body,
    /// [`Error::Transport`] for connection failures.
    pub async fn get_sandbox_metrics(
        &self,
        sandbox_id: &str,
        params: &MetricsParams,
    ) -> Result<Vec<SandboxMetrics>> {
        let response = self
            .request(Method::GET, &["sandboxes", sandbox_id, "metrics"])?
            .query(&params.query())
            .send()
            .await?;
        let metrics: Option<Vec<SandboxMetrics>> =
            parse_sandbox_response(response, sandbox_id).await?;
        Ok(metrics.unwrap_or_default())
    }
```

with `use crate::metrics::{MetricsParams, SandboxMetrics};`. In `src/sandbox.rs`, after `get_info`:

```rust
    /// Reads this sandbox's metrics. See [`Client::get_sandbox_metrics`].
    ///
    /// # Arguments
    ///
    /// * `params` - the optional time range.
    ///
    /// # Returns
    ///
    /// The samples.
    ///
    /// # Errors
    ///
    /// As [`Client::get_sandbox_metrics`].
    pub async fn get_metrics(&self, params: &MetricsParams) -> Result<Vec<SandboxMetrics>> {
        self.client.get_sandbox_metrics(&self.sandbox_id, params).await
    }
```

`src/lib.rs`: `mod metrics;` and `pub use metrics::{MetricsParams, SandboxMetrics};`.

- [ ] **Step 4: Run the tests**

Run: `cargo test --test rest_metrics` and `cargo test --doc MetricsParams`
Expected: PASS.

- [ ] **Step 5: Live test**

```rust
#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0029_get_metrics_reports_cpu_memory_and_disk() {
    // Arrange
    let sandbox = LiveSandbox::new().await;
    let started_at = sandbox.get_info().await.expect("info").started_at;

    // Act: the API samples every few seconds, so wait for the first sample.
    // Not found means the sandbox is gone, so fail at once; any other error
    // may be transient and keeps polling.
    let sandbox_ref = &sandbox;
    let metrics = poll_until("a first metrics sample", Duration::from_secs(90), || async move {
        match sandbox_ref.get_metrics(&MetricsParams::default()).await {
            Ok(metrics) => (!metrics.is_empty()).then_some(metrics),
            Err(error) if error.is_not_found() => panic!("sandbox vanished: {error:?}"),
            Err(_) => None,
        }
    })
    .await;

    // Assert
    let sample = metrics.last().expect("a sample");
    assert!(sample.cpu_count >= 1, "{sample:?}");
    assert!(sample.mem_total > 0 && sample.mem_used <= sample.mem_total, "{sample:?}");
    assert!(sample.disk_total > 0 && sample.disk_used <= sample.disk_total, "{sample:?}");
    assert!(sample.timestamp >= started_at - chrono::Duration::seconds(5), "{sample:?}");
}
```

Run it on hosted. Expected: PASS.

- [ ] **Step 6: Docs, gates, changelog, commit**

`docs/protocol-notes.md` "REST endpoints": rows "Metrics" and "Metric fields" from the Verified facts. Evidence map: CLM-0155 → the three `tests/rest_metrics.rs` tests. Gates, `changelog/<ts>-m3-task-02-metrics.md`.

```bash
git add src/metrics.rs src/client.rs src/sandbox.rs src/lib.rs tests/rest_metrics.rs tests/live_lifecycle.rs docs/ changelog/
git commit -m "feat: sandbox metrics (get_sandbox_metrics, Sandbox::get_metrics)"
```

---

### Task 3: List filters (CLM-0151, 0166)

**Files:**
- Modify: `src/models.rs` (`ListSandboxesParams`, new `SortOrder`)
- Modify: `src/client.rs` (factor `next_token_of`), `src/lib.rs` (re-export `SortOrder`)
- Create: `tests/rest_list.rs`
- Modify: `tests/live_lifecycle.rs`, `docs/protocol-notes.md` (the "List" row), `docs/verification/s4-evidence-map.md`

- [ ] **Step 1: Write the failing mock test**

`tests/rest_list.rs`:

```rust
//! The SDK 2.61 list filters against a mock server.

mod common;

use chrono::{DateTime, Utc};
use common::client_for;
use e2b_client::{ListSandboxesParams, SortOrder};
use serde_json::json;
use wiremock::matchers::{method, path, query_param, query_param_is_missing};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn list_sends_order_started_after_and_template() {
    // Arrange
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v2/sandboxes"))
        .and(query_param("order", "desc"))
        .and(query_param("startedAfter", "2026-10-27T12:00:00+00:00"))
        .and(query_param("template", "base"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .expect(1)
        .mount(&server)
        .await;
    let started_after: DateTime<Utc> = "2026-10-27T12:00:00Z".parse().expect("time");
    let params = ListSandboxesParams::new()
        .order(SortOrder::Descending)
        .started_after(started_after)
        .template("base");

    // Act
    let page = client_for(&server).list_sandboxes(&params).await.expect("list");

    // Assert
    assert!(page.sandboxes.is_empty());
}

#[tokio::test]
async fn list_omits_unset_filters() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v2/sandboxes"))
        .and(query_param_is_missing("order"))
        .and(query_param_is_missing("startedAfter"))
        .and(query_param_is_missing("template"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .expect(1)
        .mount(&server)
        .await;

    client_for(&server)
        .list_sandboxes(&ListSandboxesParams::new())
        .await
        .expect("list");
}
```

And in `src/models.rs`'s tests, a unit test that `SortOrder::Ascending` gives `("order", "asc")` and that the pairs keep the order `metadata, state, order, startedAfter, template, limit, nextToken`.

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --test rest_list`
Expected: compile error, `SortOrder` not found.

- [ ] **Step 3: Implement**

In `src/models.rs`:

```rust
/// Sort order for [`ListSandboxesParams::order`]: by start time, across the
/// whole result set before paging (`e2b/sandbox_async/sandbox_api.py:91`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SortOrder {
    /// Oldest first (`asc`).
    Ascending,
    /// Newest first (`desc`).
    Descending,
}

impl SortOrder {
    fn as_wire(self) -> &'static str {
        match self {
            Self::Ascending => "asc",
            Self::Descending => "desc",
        }
    }
}
```

Add `order: Option<SortOrder>`, `started_after: Option<DateTime<Utc>>` and `template: Option<String>` to `ListSandboxesParams`, with the setters:

```rust
    /// Sort by start time. Unset, the server's order applies.
    ///
    /// # Arguments
    ///
    /// * `order` - oldest or newest first.
    ///
    /// # Returns
    ///
    /// The parameters, for chaining.
    #[must_use]
    pub fn order(mut self, order: SortOrder) -> Self {
        self.order = Some(order);
        self
    }

    /// Only sandboxes started at or after `time`.
    ///
    /// # Arguments
    ///
    /// * `time` - the earliest start time.
    ///
    /// # Returns
    ///
    /// The parameters, for chaining.
    #[must_use]
    pub fn started_after(mut self, time: DateTime<Utc>) -> Self {
        self.started_after = Some(time);
        self
    }

    /// Only sandboxes from `template` (a template ID or name).
    ///
    /// # Arguments
    ///
    /// * `template` - the template ID or name.
    ///
    /// # Returns
    ///
    /// The parameters, for chaining.
    #[must_use]
    pub fn template(mut self, template: impl Into<String>) -> Self {
        self.template = Some(template.into());
        self
    }
```

In `query()`, after the `state` block and before `limit`, with the capacity raised to 7:

```rust
        if let Some(order) = self.order {
            query.push(("order", order.as_wire().to_owned()));
        }
        if let Some(time) = self.started_after {
            query.push(("startedAfter", isoformat(time)));
        }
        if let Some(template) = &self.template {
            query.push(("template", template.clone()));
        }
```

> **Corrected (issue #62, 2026-10-27).** `to_rfc3339()` writes 3, 6 or 9 fractional digits depending on the value, while Python's `isoformat()` writes none for a whole second and otherwise exactly 6 (Python has no nanoseconds). `startedAfter` now goes through a private `fn isoformat(time: DateTime<Utc>) -> String` in `src/models.rs`: `to_rfc3339_opts(SecondsFormat::Secs, false)` when `timestamp_subsec_micros()` is 0, else `SecondsFormat::Micros` (which truncates nanoseconds). The unit test `isoformat_matches_python_for_whole_milli_micro_and_nanosecond_times` pins it.

Update the doc comment on `query()` to name the new pairs. In `src/client.rs`, move the header read out of `list_sandboxes` into a helper that Task 8 reuses:

```rust
/// The next page token from `x-next-token` (`e2b/paginator.py:48-50`), or
/// `None` on the last page.
fn next_token_of(response: &reqwest::Response) -> Option<String> {
    response
        .headers()
        .get(HEADER_NEXT_TOKEN)
        .and_then(|value| value.to_str().ok())
        .filter(|token| !token.is_empty())
        .map(str::to_owned)
}
```

`list_sandboxes` then uses `let next_token = next_token_of(&response);`. `src/lib.rs`: add `SortOrder` to the `models` re-export.

- [ ] **Step 4: Run the tests**

Run: `cargo test --test rest_list`, `cargo test --test rest_api list_` and `cargo test --lib list_query`
Expected: PASS (the existing list tests unchanged).

- [ ] **Step 5: Live test**

```rust
#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0151_list_filters_by_template_and_start_time_and_orders() {
    // Arrange: two sandboxes started at least 2 s apart.
    let first = LiveSandbox::new().await;
    tokio::time::sleep(Duration::from_secs(2)).await;
    let second = LiveSandbox::new().await;
    let second_info = second.get_info().await.expect("second info");
    let first_info = first.get_info().await.expect("first info");
    let ours = ListSandboxesParams::new()
        .metadata(RUN_METADATA_KEY, run_id())
        .template(first_info.template_id.clone());

    // Act
    let newest_first = ids_of(&list_all(&ours.clone().order(SortOrder::Descending)).await);
    let oldest_first = ids_of(&list_all(&ours.clone().order(SortOrder::Ascending)).await);
    let since_second = ids_of(&list_all(&ours.clone().started_after(second_info.started_at)).await);

    // Assert: other tests of this run may also be listed; compare our two.
    let position = |ids: &[String], id: &str| ids.iter().position(|each| each == id);
    let (first_id, second_id) = (first.sandbox_id(), second.sandbox_id());
    for (ids, label) in [(&newest_first, "desc"), (&oldest_first, "asc")] {
        assert!(
            position(ids, first_id).is_some() && position(ids, second_id).is_some(),
            "{label} list is missing one of ours: {ids:?}"
        );
    }
    assert!(position(&newest_first, second_id) < position(&newest_first, first_id), "{newest_first:?}");
    assert!(position(&oldest_first, first_id) < position(&oldest_first, second_id), "{oldest_first:?}");
    assert!(since_second.iter().any(|id| id == second_id), "{since_second:?}");
    assert!(!since_second.iter().any(|id| id == first_id), "{since_second:?}");
}
```

`position(...) < position(...)` compares `Option<usize>`, where `None` sorts first, so the loop first makes sure both ids are in both lists. Run it on hosted. Expected: PASS.

- [ ] **Step 6: Docs, gates, changelog, commit**

Extend the "List" row in `docs/protocol-notes.md` with `order`, `startedAfter` and `template`. Evidence map: CLM-0166 → both `tests/rest_list.rs` tests and the unit test. `changelog/<ts>-m3-task-03-list-filters.md`.

```bash
git add src/models.rs src/client.rs src/lib.rs tests/rest_list.rs tests/live_lifecycle.rs docs/ changelog/
git commit -m "feat: list filters order, started_after and template"
```

---

### Task 4: Network settings at create (CLM-0143, 0156; the network part of CLM-0153, 0167)

**Files:**
- Create: `src/network.rs` (`ALL_TRAFFIC`, the shared egress fields, `NetworkConfig`, `NetworkRule`, `NetworkInfo`)
- Modify: `src/models.rs` (`CreateSandboxParams::network`, the body field, `SandboxInfo::{allow_internet_access, network}`), `src/lib.rs`
- Create: `tests/rest_network.rs`
- Modify: `tests/live_support/mod.rs` (`http_reply`), `tests/live_lifecycle.rs` (new tests; `clm_0012` uses `http_reply`)
- Modify: `docs/protocol-notes.md` (new section "Network"), `docs/verification/s4-evidence-map.md`

> **Corrected (issue #63, 2026-10-27).** What shipped differs from the code below in these points; read `src/network.rs` and `tests/rest_network.rs`:
> - **An empty `NetworkConfig` is not sent.** The SDK leaves `network` out when nothing is set (`network_body if network_body else UNSET`, `py: sandbox_async/sandbox_api.py:250`), so `NetworkConfig` has a private `is_empty()` and `body()` sends `network: self.network.as_ref().filter(|network| !network.is_empty())`. The mock test `create_leaves_out_a_network_with_nothing_set` pins it. Task 6's `body()` below is updated to match.
> - **The module doc doesn't link `NetworkUpdate` or `Client::update_sandbox_network`**, which don't exist yet (`just doc` fails on a broken intra-doc link). Task 5 adds both links when it adds the items.
> - **`null` reads as empty in rules too:** `NetworkRule::transform` and `NetworkTransform::headers` use `deserialize_null_as_default` (alongside `default`), as `NetworkInfo`'s lists do.
> - **Hosted E2B accepts at most one rule per host:** two give 400 `Domain "<host>" can have at most 1 transform rule.` (live, 2026-10-27). The `rule()` doc says so; the crate still sends what the caller built, as the SDK does.
> - **Hosted E2B reports `allowInternetAccess: false` as given**, not as a `deny_out` entry, so `clm_0153` asserts as written. A sandbox created without network settings reports `allowInternetAccess: null` and `network: null`.
> - **Extra tests:** mock tests `create_sends_network_next_to_allow_internet_access`, `create_leaves_out_a_network_with_nothing_set`, `get_info_reads_null_network_fields_as_none` and `get_info_without_network_fields_reads_none`; unit tests `config_debug_hides_rule_header_values`, `rules_for_one_host_keep_their_order_and_an_empty_rule_is_an_empty_object`, `only_a_config_with_nothing_set_is_empty`, `rule_reads_null_transform_and_null_headers_as_empty` and `info_reads_null_fields_as_empty_or_none`.
> - **`clm_0012`'s failure messages** no longer print the `CommandResult`, since `http_reply` returns a `bool`.
> - **`clm_0153` also sends a rule** (`one.one.one.one` with header `X-Probe: v`) and asserts that hosted E2B reports it back (review of PR #77).

- [ ] **Step 1: Write the failing mock tests**

`tests/rest_network.rs`:

```rust
//! Sandbox network settings against a mock server.

mod common;

use common::{SANDBOX_ID, client_for, sandbox_json};
use e2b_client::{ALL_TRAFFIC, CreateSandboxParams, NetworkConfig, NetworkRule};
use serde_json::json;
use wiremock::matchers::{body_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn create_sends_the_network_object_in_camel_case() {
    // Arrange
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v2/sandboxes"))
        .and(body_json(json!({
            "templateID": "base",
            "metadata": {},
            "envVars": {},
            "network": {
                "allowOut": ["1.1.1.1", "api.example.test"],
                "denyOut": ["0.0.0.0/0"],
                "dnsServers": [],
                "rules": {"api.example.test": [{"transform": {"headers": {"X-Key": "v"}}}]},
                "allowPublicTraffic": false,
                "maskRequestHost": "${PORT}-app.example.test",
                "httpsPorts": [3000]
            }
        })))
        .respond_with(ResponseTemplate::new(201).set_body_json(sandbox_json("0.5.7")))
        .expect(1)
        .mount(&server)
        .await;
    let network = NetworkConfig::new()
        .allow_out(["1.1.1.1", "api.example.test"])
        .deny_out([ALL_TRAFFIC])
        .dns_servers(Vec::<String>::new())
        .rule("api.example.test", NetworkRule::new().header("X-Key", "v"))
        .allow_public_traffic(false)
        .mask_request_host("${PORT}-app.example.test")
        .https_ports([3000]);

    // Act
    let sandbox = client_for(&server)
        .create_sandbox(&CreateSandboxParams::default().network(network))
        .await
        .expect("create");

    // Assert: the mock only matches the exact body.
    assert_eq!(sandbox.sandbox_id(), SANDBOX_ID);
}

#[tokio::test]
async fn get_info_reads_network_and_internet_access() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/sandboxes/{SANDBOX_ID}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "templateID": "base", "sandboxID": SANDBOX_ID, "clientID": "c",
            "startedAt": "2026-10-27T12:00:00Z", "endAt": "2026-10-27T12:05:00Z",
            "envdVersion": "0.5.7", "cpuCount": 2, "memoryMB": 512, "diskSizeMB": 2048,
            "state": "running",
            "allowInternetAccess": false,
            "network": {
                "allowOut": ["1.1.1.1"], "denyOut": null, "dnsServers": ["8.8.8.8"],
                "allowPublicTraffic": true, "httpsPorts": [3000],
                "rules": {"api.example.test": [{"transform": {"headers": {"X-Key": "v"}}}]},
                "egressProxy": null
            }
        })))
        .mount(&server)
        .await;

    let info = client_for(&server).get_sandbox_info(SANDBOX_ID).await.expect("info");

    assert_eq!(info.allow_internet_access, Some(false));
    let network = info.network.expect("network");
    assert_eq!(network.allow_out, ["1.1.1.1"]);
    assert!(network.deny_out.is_empty());
    assert_eq!(network.dns_servers.as_deref(), Some(&["8.8.8.8".to_owned()][..]));
    assert_eq!(network.allow_public_traffic, Some(true));
    assert_eq!(network.https_ports, [3000]);
    let rule = &network.rules["api.example.test"][0];
    assert_eq!(rule.headers().get("X-Key").map(String::as_str), Some("v"));
}
```

In `src/network.rs`'s own `mod tests` (written with the module in Step 3), two unit tests: `NetworkConfig::new().deny_out([ALL_TRAFFIC])` serializes to exactly `{"denyOut": ["0.0.0.0/0"]}`; and `format!("{:?}", NetworkRule::new().header("Authorization", "Bearer s3cret"))` contains `Authorization` but not `s3cret`.

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --test rest_network`
Expected: compile error, `NetworkConfig` not found.

- [ ] **Step 3: Implement `src/network.rs`**

```rust
//! Sandbox network settings: outbound allow and deny lists, DNS servers,
//! public traffic, and per-host header rules, set at create through
//! [`crate::CreateSandboxParams::network`] and read back in
//! [`crate::SandboxInfo::network`].
//!
//! The JSON shapes are `SandboxNetworkConfig` and
//! `SandboxNetworkUpdateConfig` (`e2b/api/client/models/`); the SDK builds
//! them in `e2b/sandbox/sandbox_api.py:928-996, 1128-1152`.

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::wire::deserialize_null_as_default;

/// The CIDR block that matches all IPv4 traffic, the SDKs' `ALL_TRAFFIC`
/// (`e2b/sandbox/network.py:8`). `deny_out([ALL_TRAFFIC])` blocks all
/// egress, which is what `allow_internet_access(false)` does.
pub const ALL_TRAFFIC: &str = "0.0.0.0/0";

/// The egress fields that create and update share. Each is sent only when
/// set; an empty list is sent as `[]` (for `dnsServers` it means "no DNS
/// server", `e2b/sandbox/sandbox_api.py:955-958`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct Egress {
    #[serde(skip_serializing_if = "Option::is_none")]
    allow_out: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    deny_out: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dns_servers: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rules: Option<BTreeMap<String, Vec<NetworkRule>>>,
}

fn strings<I, S>(items: I) -> Vec<String>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    items.into_iter().map(Into::into).collect()
}

/// The egress setters, for a builder with an `egress: Egress` field.
///
/// Rust note: `macro_rules!` pastes these methods into each `impl` block
/// that invokes it, so `NetworkConfig` and `NetworkUpdate` share one copy of
/// the code without a trait.
macro_rules! egress_setters {
    () => {
        /// Allows outbound traffic to these CIDR blocks, IP addresses or
        /// host names, overriding `deny_out` for them. Without `allow_out`
        /// everything not denied is allowed.
        ///
        /// # Arguments
        ///
        /// * `targets` - the allowed destinations.
        ///
        /// # Returns
        ///
        /// The settings, for chaining.
        #[must_use]
        pub fn allow_out<I, S>(mut self, targets: I) -> Self
        where
            I: IntoIterator<Item = S>,
            S: Into<String>,
        {
            self.egress.allow_out = Some(strings(targets));
            self
        }

        /// Denies outbound traffic to these CIDR blocks, IP addresses or
        /// host names. `[ALL_TRAFFIC]` denies everything not allowed.
        ///
        /// # Arguments
        ///
        /// * `targets` - the denied destinations.
        ///
        /// # Returns
        ///
        /// The settings, for chaining.
        #[must_use]
        pub fn deny_out<I, S>(mut self, targets: I) -> Self
        where
            I: IntoIterator<Item = S>,
            S: Into<String>,
        {
            self.egress.deny_out = Some(strings(targets));
            self
        }

        /// The IPv4 DNS servers the sandbox may reach when `allow_out`
        /// names a domain (up to 10). Unset, `8.8.8.8` is allowed; an empty
        /// list allows none. This only opens the firewall; it does not
        /// change the sandbox's `/etc/resolv.conf`.
        ///
        /// # Arguments
        ///
        /// * `servers` - the DNS server addresses.
        ///
        /// # Returns
        ///
        /// The settings, for chaining.
        #[must_use]
        pub fn dns_servers<I, S>(mut self, servers: I) -> Self
        where
            I: IntoIterator<Item = S>,
            S: Into<String>,
        {
            self.egress.dns_servers = Some(strings(servers));
            self
        }

        /// Adds a rule for outbound HTTP and HTTPS requests to `host`.
        /// Rules for one host apply in the order added. A rule does not
        /// allow egress by itself: the host must also be in `allow_out`.
        ///
        /// # Arguments
        ///
        /// * `host` - the domain (or IP or CIDR block) the rule applies to.
        /// * `rule` - the rule.
        ///
        /// # Returns
        ///
        /// The settings, for chaining.
        #[must_use]
        pub fn rule(mut self, host: impl Into<String>, rule: NetworkRule) -> Self {
            self.egress
                .rules
                .get_or_insert_with(BTreeMap::new)
                .entry(host.into())
                .or_default()
                .push(rule);
            self
        }
    };
}

/// Network settings for a new sandbox ([`crate::CreateSandboxParams::network`]).
///
/// # Examples
///
/// ```
/// use e2b_client::{ALL_TRAFFIC, CreateSandboxParams, NetworkConfig};
///
/// // Deny all egress except one API, and keep the sandbox's URLs private.
/// let network = NetworkConfig::new()
///     .deny_out([ALL_TRAFFIC])
///     .allow_out(["api.example.com"])
///     .allow_public_traffic(false);
/// let params = CreateSandboxParams::new("base").network(network);
/// # let _ = params;
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkConfig {
    // Rust note: `flatten` writes the egress fields at this level of the
    // JSON object, as if they were declared here.
    #[serde(flatten)]
    egress: Egress,
    #[serde(skip_serializing_if = "Option::is_none")]
    allow_public_traffic: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mask_request_host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    https_ports: Option<Vec<u16>>,
}

impl NetworkConfig {
    /// No settings: the server's defaults apply.
    ///
    /// # Returns
    ///
    /// Empty settings.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    egress_setters!();

    /// Whether the sandbox's URLs ([`crate::Sandbox::get_host`]) are public.
    /// When `false`, requests to them need the sandbox's traffic access
    /// token. Unset, the server's default applies.
    ///
    /// # Arguments
    ///
    /// * `allow` - whether public traffic is allowed.
    ///
    /// # Returns
    ///
    /// The settings, for chaining.
    #[must_use]
    pub fn allow_public_traffic(mut self, allow: bool) -> Self {
        self.allow_public_traffic = Some(allow);
        self
    }

    /// A custom host mask for requests to the sandbox, with `${PORT}` for the
    /// port, e.g. `"${PORT}-myapp.example.com"`.
    ///
    /// # Arguments
    ///
    /// * `mask` - the host mask.
    ///
    /// # Returns
    ///
    /// The settings, for chaining.
    #[must_use]
    pub fn mask_request_host(mut self, mask: impl Into<String>) -> Self {
        self.mask_request_host = Some(mask.into());
        self
    }

    /// Ports whose public URLs reach the sandbox over HTTPS, for services
    /// that serve TLS themselves. The proxy does not verify their
    /// certificates, so self-signed ones work.
    ///
    /// # Arguments
    ///
    /// * `ports` - the ports.
    ///
    /// # Returns
    ///
    /// The settings, for chaining.
    #[must_use]
    pub fn https_ports(mut self, ports: impl IntoIterator<Item = u16>) -> Self {
        self.https_ports = Some(ports.into_iter().collect());
        self
    }
}

/// A rule for outbound HTTP and HTTPS requests to one host: headers to set
/// on each request (`SandboxNetworkRule`, `e2b/sandbox/sandbox_api.py:149-224`).
///
/// # Examples
///
/// ```
/// use e2b_client::NetworkRule;
///
/// let rule = NetworkRule::new().header("X-Api-Key", "secret");
/// assert_eq!(rule.headers()["X-Api-Key"], "secret");
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkRule {
    #[serde(default, skip_serializing_if = "NetworkTransform::is_empty")]
    transform: NetworkTransform,
}

impl NetworkRule {
    /// A rule that changes nothing yet.
    ///
    /// # Returns
    ///
    /// An empty rule.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets `name: value` on every matching request, replacing a header of
    /// that name the request already has.
    ///
    /// # Arguments
    ///
    /// * `name`, `value` - the header.
    ///
    /// # Returns
    ///
    /// The rule, for chaining.
    #[must_use]
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.transform.headers.insert(name.into(), value.into());
        self
    }

    /// The headers this rule sets.
    ///
    /// # Returns
    ///
    /// Header names and values.
    #[must_use]
    pub fn headers(&self) -> &BTreeMap<String, String> {
        &self.transform.headers
    }
}

/// `SandboxNetworkTransform`: what a rule changes.
#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
struct NetworkTransform {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    headers: BTreeMap<String, String>,
}

impl NetworkTransform {
    fn is_empty(&self) -> bool {
        self.headers.is_empty()
    }
}

// Header values are often credentials (`Authorization`), so `Debug` shows
// only the header names.
impl fmt::Debug for NetworkTransform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NetworkTransform")
            .field("headers", &self.headers.keys().collect::<Vec<_>>())
            .finish()
    }
}

/// A sandbox's network settings as [`crate::Client::get_sandbox_info`]
/// reports them (`SandboxNetworkConfig` in `SandboxDetail`). A missing or
/// `null` list reads as empty.
///
/// This is a plain data record, so its fields are public. It is
/// `#[non_exhaustive]` so fields can be added without a breaking change.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct NetworkInfo {
    /// Allowed outbound destinations.
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub allow_out: Vec<String>,
    /// Denied outbound destinations.
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub deny_out: Vec<String>,
    /// The DNS servers the sandbox may reach; `None` means the default,
    /// `8.8.8.8`.
    #[serde(default)]
    pub dns_servers: Option<Vec<String>>,
    /// Whether the sandbox's URLs are public, when reported.
    #[serde(default)]
    pub allow_public_traffic: Option<bool>,
    /// The custom host mask, if any.
    #[serde(default)]
    pub mask_request_host: Option<String>,
    /// Ports served over HTTPS.
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub https_ports: Vec<u16>,
    /// Per-host rules.
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub rules: BTreeMap<String, Vec<NetworkRule>>,
}
```

plus the `mod tests` from Step 1. `src/models.rs`: a `network: Option<NetworkConfig>` field on `CreateSandboxParams` (initialised to `None` in `new`), the setter

```rust
    /// Sets the sandbox's network settings.
    ///
    /// # Arguments
    ///
    /// * `network` - the settings.
    ///
    /// # Returns
    ///
    /// The parameters, for chaining.
    #[must_use]
    pub fn network(mut self, network: NetworkConfig) -> Self {
        self.network = Some(network);
        self
    }
```

`network: self.network.as_ref()` in `body()`, and on `NewSandboxBody`:

```rust
    #[serde(skip_serializing_if = "Option::is_none")]
    network: Option<&'a NetworkConfig>,
```

On `SandboxInfo`:

```rust
    /// Whether the sandbox may reach the internet, when reported.
    #[serde(default)]
    pub allow_internet_access: Option<bool>,
    /// The sandbox's network settings, when reported.
    #[serde(default)]
    pub network: Option<NetworkInfo>,
```

`src/lib.rs`: `mod network;` and `pub use network::{ALL_TRAFFIC, NetworkConfig, NetworkInfo, NetworkRule};`.

- [ ] **Step 4: Run the tests**

Run: `cargo test --test rest_network`, `cargo test --lib network` and `cargo test --test rest_api`
Expected: PASS; the existing create bodies are unchanged.

- [ ] **Step 5: The live probe helper and live tests**

In `tests/live_support/mod.rs` (import `e2b_client::{CommandOptions, Sandbox}`):

```rust
/// Whether `sandbox` gets an HTTP reply from port 80 of `ip`, within 10 s.
///
/// It needs a reply, not just a connection: hosted E2B completes the TCP
/// handshake on the host even when egress is denied (M1, CLM-0012). The
/// targets are Cloudflare's resolvers (1.1.1.1, 1.0.0.1), which answer HTTP.
pub async fn http_reply(sandbox: &Sandbox, ip: &str) -> bool {
    let probe = format!(
        r#"timeout 10 bash -c 'exec 3<>/dev/tcp/{ip}/80
printf "GET / HTTP/1.0\r\nHost: one.one.one.one\r\n\r\n" >&3
read -r status <&3
[[ $status == HTTP/* ]]'"#
    );
    sandbox
        .commands()
        .run(&probe, &CommandOptions::default())
        .await
        .expect("probe")
        .exit_code
        == 0
}
```

`format!` treats `{` and `}` as placeholders; the probe has none besides `{ip}`. Change `clm_0012_internet_access_false_blocks_outbound` to call `http_reply(&open, "1.1.1.1")` and `http_reply(&closed, "1.1.1.1")` and assert on the two booleans with the same messages. Then add to `tests/live_lifecycle.rs`:

```rust
#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0143_create_network_allow_and_deny_lists_are_enforced() {
    // Arrange: deny everything except 1.1.1.1.
    let network = NetworkConfig::new().deny_out([ALL_TRAFFIC]).allow_out(["1.1.1.1"]);
    let sandbox = LiveSandbox::create(&params().network(network)).await;

    // Act
    let allowed = http_reply(&sandbox, "1.1.1.1").await;
    let denied = http_reply(&sandbox, "1.0.0.1").await;

    // Assert
    assert!(allowed, "1.1.1.1 is in allow_out but gave no HTTP reply");
    assert!(!denied, "1.0.0.1 is denied by ALL_TRAFFIC but replied");
}

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0153_get_info_reports_network() {
    // Arrange
    let network = NetworkConfig::new().allow_out(["1.1.1.1"]).dns_servers(["1.1.1.1"]);
    let sandbox =
        LiveSandbox::create(&params().allow_internet_access(false).network(network)).await;

    // Act
    let info = sandbox.get_info().await.expect("info");

    // Assert
    assert_eq!(info.allow_internet_access, Some(false), "{info:?}");
    let network = info.network.expect("network in the detail");
    assert!(network.allow_out.iter().any(|target| target == "1.1.1.1"), "{network:?}");
    assert_eq!(network.dns_servers.as_deref(), Some(&["1.1.1.1".to_owned()][..]), "{network:?}");
}
```

Run both new tests and `clm_0012` on hosted. Expected: PASS. If the server reports `allow_internet_access: false` as a `deny_out` entry instead, record what it returns, adjust the assertion to it, and add a plan correction.

- [ ] **Step 6: Docs, gates, changelog, commit**

`docs/protocol-notes.md`: a new section "Network" after "REST endpoints", with rows for the create network object, rules, `ALL_TRAFFIC`, and the detail fields `allowInternetAccess` and `network`, from the Verified facts; the "Create body" row gains `network`. Evidence map: CLM-0156 → `tests/rest_network.rs::create_sends_the_network_object_in_camel_case` and the serialization unit test; CLM-0167 (network part) → `tests/rest_network.rs::get_info_reads_network_and_internet_access`. Note in CLM-0167's row that Tasks 6 and 10 add the other parts. `changelog/<ts>-m3-task-04-network-at-create.md`.

```bash
git add src/network.rs src/models.rs src/lib.rs tests/rest_network.rs tests/live_support/mod.rs tests/live_lifecycle.rs docs/ changelog/
git commit -m "feat: network settings at sandbox create"
```

---

### Task 5: Network updates (CLM-0031, 0144, 0157)

**Files:**
- Modify: `src/network.rs` (`NetworkUpdate`)
- Modify: `src/client.rs` (`update_sandbox_network`), `src/sandbox.rs` (`update_network`), `src/lib.rs`
- Modify: `tests/rest_network.rs`, `tests/live_lifecycle.rs`, `docs/protocol-notes.md` ("Network"), `docs/verification/s4-evidence-map.md`

> **Corrected (issue #63, 2026-10-27).** Task 4 shipped `src/network.rs` without links to `NetworkUpdate` and `Client::update_sandbox_network` in its module doc, because they didn't exist yet. When adding them, extend the module doc's first sentence with "and, through [`NetworkUpdate`], by [`crate::Client::update_sandbox_network`]". `NetworkConfig::is_empty` is private to create; don't use it for updates, which send `{}` when empty.
>
> **Corrected (issue #64, 2026-10-27).** What shipped matches the code below, with these additions; read `src/network.rs` and `tests/rest_network.rs`:
> - **The module doc** also names `SandboxNetworkUpdateConfig` among the JSON shapes and cites `e2b/sandbox/sandbox_api.py:1128-1152` (`build_network_update_body`). The crate doc's feature list names `NetworkUpdate` next to `NetworkConfig`.
> - **Extra tests:** mock tests `update_network_refused_by_the_api_is_an_api_error` (a 400, hosted E2B's answer to two rules for one host, is `Error::Api` with status and message) and `sandbox_update_network_puts_to_its_own_id` (`Sandbox::update_network` delegates); unit tests `update_sends_allow_internet_access_in_snake_case_next_to_camel_case_egress` and `update_debug_hides_rule_header_values`. The not-found test also checks the id `SandboxNotFound` carries.
> - **`clm_0144` checks "closed" on two probes in a row** (`live_support::no_http_reply_twice`), both before the first update and in the "egress to close" poll, so one transient failure doesn't pass for a deny rule (review of PR #78).
> - **Hosted E2B behaves as planned:** `clm_0144` passed (live, 2026-10-27). An update with only `allow_internet_access(true)` clears the create-time `deny_out`, and each update took effect by the first probe after the 204.

- [ ] **Step 1: Write the failing mock tests**

Append to `tests/rest_network.rs` (add `Error` and `NetworkUpdate` to its imports):

```rust
#[tokio::test]
async fn update_network_puts_the_egress_fields() {
    // Arrange
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(path(format!("/sandboxes/{SANDBOX_ID}/network")))
        .and(body_json(json!({
            "denyOut": ["0.0.0.0/0"],
            "allowOut": ["1.1.1.1"],
            "dnsServers": ["1.1.1.1"],
            "rules": {"api.example.test": [{"transform": {"headers": {"X-Key": "v"}}}]},
            "allow_internet_access": true
        })))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    let update = NetworkUpdate::new()
        .deny_out([ALL_TRAFFIC])
        .allow_out(["1.1.1.1"])
        .dns_servers(["1.1.1.1"])
        .rule("api.example.test", NetworkRule::new().header("X-Key", "v"))
        .allow_internet_access(true);

    // Act and assert: the mock only matches the exact body.
    client_for(&server)
        .update_sandbox_network(SANDBOX_ID, &update)
        .await
        .expect("update");
}

#[tokio::test]
async fn an_empty_update_sends_an_empty_object() {
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(path(format!("/sandboxes/{SANDBOX_ID}/network")))
        .and(body_json(json!({})))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;

    client_for(&server)
        .update_sandbox_network(SANDBOX_ID, &NetworkUpdate::new())
        .await
        .expect("update");
}

#[tokio::test]
async fn update_network_of_a_missing_sandbox_is_not_found() {
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(path(format!("/sandboxes/{SANDBOX_ID}/network")))
        .respond_with(ResponseTemplate::new(404).set_body_json(json!({"code": 404, "message": "nope"})))
        .mount(&server)
        .await;

    let error = client_for(&server)
        .update_sandbox_network(SANDBOX_ID, &NetworkUpdate::new())
        .await
        .expect_err("not found");

    assert!(matches!(error, Error::SandboxNotFound(_)), "{error:?}");
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --test rest_network update_`
Expected: compile error, `NetworkUpdate` not found.

- [ ] **Step 3: Implement**

`src/network.rs`:

```rust
/// A replacement for a running sandbox's egress settings
/// ([`crate::Client::update_sandbox_network`], `SandboxNetworkUpdate`,
/// `e2b/sandbox/sandbox_api.py:515-561`).
///
/// The update replaces the whole egress configuration: every field left
/// unset is cleared on the server (DNS servers go back to `8.8.8.8`). Repeat
/// every setting that should stay.
///
/// # Examples
///
/// ```
/// use e2b_client::{ALL_TRAFFIC, NetworkUpdate};
///
/// let lock_down = NetworkUpdate::new().deny_out([ALL_TRAFFIC]).allow_out(["1.1.1.1"]);
/// # let _ = lock_down;
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct NetworkUpdate {
    #[serde(flatten)]
    egress: Egress,
    // The spec spells this one field in snake_case
    // (`sandbox_network_update_config.py:103`).
    #[serde(skip_serializing_if = "Option::is_none")]
    allow_internet_access: Option<bool>,
}

impl NetworkUpdate {
    /// An update that clears every egress setting.
    ///
    /// # Returns
    ///
    /// An empty update.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    egress_setters!();

    /// Allows or denies all internet access; `false` is the same as
    /// `deny_out([ALL_TRAFFIC])`.
    ///
    /// # Arguments
    ///
    /// * `allow` - whether the sandbox may reach the internet.
    ///
    /// # Returns
    ///
    /// The update, for chaining.
    #[must_use]
    pub fn allow_internet_access(mut self, allow: bool) -> Self {
        self.allow_internet_access = Some(allow);
        self
    }
}
```

`src/client.rs`, after `set_sandbox_timeout`:

```rust
    /// Replaces a running sandbox's egress settings, without a restart
    /// (`PUT /sandboxes/{id}/network`). Fields the update leaves unset are
    /// cleared; see [`NetworkUpdate`].
    ///
    /// # Arguments
    ///
    /// * `sandbox_id` - the sandbox.
    /// * `update` - the new settings.
    ///
    /// # Errors
    ///
    /// [`Error::SandboxNotFound`] for HTTP 404, [`Error::Api`] for other
    /// non-2xx answers, [`Error::Transport`] for connection failures.
    pub async fn update_sandbox_network(
        &self,
        sandbox_id: &str,
        update: &NetworkUpdate,
    ) -> Result<()> {
        let response = self
            .send_json(Method::PUT, &["sandboxes", sandbox_id, "network"], update)
            .await?;
        ensure_found_and_success(response, sandbox_id).await
    }
```

`src/sandbox.rs`:

```rust
    /// Replaces this sandbox's egress settings. See
    /// [`Client::update_sandbox_network`].
    ///
    /// # Arguments
    ///
    /// * `update` - the new settings.
    ///
    /// # Errors
    ///
    /// As [`Client::update_sandbox_network`].
    pub async fn update_network(&self, update: &NetworkUpdate) -> Result<()> {
        self.client
            .update_sandbox_network(&self.sandbox_id, update)
            .await
    }
```

`src/lib.rs`: add `NetworkUpdate` to the `network` re-export.

- [ ] **Step 4: Run the tests**

Run: `cargo test --test rest_network` and `cargo test --doc NetworkUpdate`
Expected: PASS.

- [ ] **Step 5: Live test**

```rust
#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0144_update_network_replaces_egress_without_restart() {
    // Arrange: all egress denied, and a file that a restart would not keep
    // in memory (tmpfs).
    let network = NetworkConfig::new().deny_out([ALL_TRAFFIC]);
    let sandbox = LiveSandbox::create(&params().network(network)).await;
    sandbox
        .commands()
        .run("echo kept > /dev/shm/marker", &CommandOptions::default())
        .await
        .expect("marker");
    assert!(!http_reply(&sandbox, "1.1.1.1").await, "egress was open before any update");

    // Act 1: an update with only allow_internet_access(true) clears deny_out.
    sandbox
        .update_network(&NetworkUpdate::new().allow_internet_access(true))
        .await
        .expect("open egress");
    let sandbox_ref = &sandbox;
    poll_until("egress to open", Duration::from_secs(30), || async move {
        http_reply(sandbox_ref, "1.1.1.1").await.then_some(())
    })
    .await;

    // Act 2: deny everything again.
    sandbox
        .update_network(&NetworkUpdate::new().deny_out([ALL_TRAFFIC]))
        .await
        .expect("close egress");
    poll_until("egress to close", Duration::from_secs(30), || async move {
        (!http_reply(sandbox_ref, "1.1.1.1").await).then_some(())
    })
    .await;

    // Assert: no restart, so the in-memory file is still there.
    let marker = sandbox
        .commands()
        .run("cat /dev/shm/marker", &CommandOptions::default())
        .await
        .expect("read marker");
    assert_eq!(marker.stdout, "kept\n");
}
```

Run it on hosted. Expected: PASS.

- [ ] **Step 6: Docs, gates, changelog, commit**

"Network" section: rows for the update route, its body (note the snake_case `allow_internet_access`) and the replace semantics. Evidence map: CLM-0157 → the three new `tests/rest_network.rs` tests. `changelog/<ts>-m3-task-05-network-updates.md`.

```bash
git add src/network.rs src/client.rs src/sandbox.rs src/lib.rs tests/rest_network.rs tests/live_lifecycle.rs docs/ changelog/
git commit -m "feat: update_network on a running sandbox"
```

---

### Task 6: Lifecycle and idempotency key at create (CLM-0145, 0152, 0158; the lifecycle part of CLM-0153, 0167)

**Files:**
- Create: `src/lifecycle.rs` (`SnapshotMode`, `OnTimeout`, `Lifecycle`, `TimeoutAction`, `LifecycleInfo`)
- Modify: `src/models.rs` (`CreateSandboxParams::{lifecycle, idempotency_key}`, `auto_pause` rewritten, fallible `body()`, `SandboxInfo::{lifecycle, pause_mode}`)
- Modify: `src/client.rs` (`create_sandbox` validates first and sends `Idempotency-Key`), `src/lib.rs`
- Create: `tests/rest_lifecycle.rs`
- Modify: `tests/live_lifecycle.rs`, `docs/protocol-notes.md` ("REST endpoints": "Create body", "Create lifecycle", "Idempotency key", "Detail fields"), `docs/verification/s4-evidence-map.md`

- [ ] **Step 1: Write the failing mock tests**

`tests/rest_lifecycle.rs`:

```rust
//! Lifecycle create options, the idempotency key, and the pause and resume
//! modes (Task 7) against a mock server.

mod common;

use common::{SANDBOX_ID, client_for, sandbox_json};
use e2b_client::{
    CreateSandboxParams, Error, Lifecycle, OnTimeout, SnapshotMode, TimeoutAction,
};
use serde_json::json;
use wiremock::matchers::{body_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Mounts a create endpoint that answers 201 to any body.
async fn create_endpoint(server: &MockServer) {
    Mock::given(method("POST"))
        .and(path("/v2/sandboxes"))
        .respond_with(ResponseTemplate::new(201).set_body_json(sandbox_json("0.5.7")))
        .mount(server)
        .await;
}

/// The JSON body of the only create request `server` received.
async fn create_body(server: &MockServer) -> serde_json::Value {
    let requests = server.received_requests().await.expect("recording");
    let [request] = requests.as_slice() else {
        panic!("expected exactly one request, got {}", requests.len());
    };
    serde_json::from_slice(&request.body).expect("json body")
}

#[tokio::test]
async fn pause_with_a_filesystem_snapshot_sends_auto_pause_memory_false() {
    // Arrange
    let server = MockServer::start().await;
    create_endpoint(&server).await;
    let lifecycle = Lifecycle::new()
        .on_timeout(OnTimeout::pause_with(SnapshotMode::Filesystem))
        .auto_resume(false);

    // Act
    client_for(&server)
        .create_sandbox(&CreateSandboxParams::default().lifecycle(lifecycle))
        .await
        .expect("create");

    // Assert
    assert_eq!(
        create_body(&server).await,
        json!({
            "templateID": "base", "metadata": {}, "envVars": {},
            "autoPause": true, "autoPauseMemory": false, "autoResume": {"enabled": false}
        })
    );
}

#[tokio::test]
async fn pause_without_a_mode_omits_auto_pause_memory() {
    let server = MockServer::start().await;
    create_endpoint(&server).await;
    let lifecycle = Lifecycle::new().on_timeout(OnTimeout::pause()).auto_resume(true);

    client_for(&server)
        .create_sandbox(&CreateSandboxParams::default().lifecycle(lifecycle))
        .await
        .expect("create");

    assert_eq!(
        create_body(&server).await,
        json!({
            "templateID": "base", "metadata": {}, "envVars": {},
            "autoPause": true, "autoResume": {"enabled": true}
        })
    );
}

#[tokio::test]
async fn kill_sends_auto_pause_false_only() {
    let server = MockServer::start().await;
    create_endpoint(&server).await;

    client_for(&server)
        .create_sandbox(
            &CreateSandboxParams::default().lifecycle(Lifecycle::new().on_timeout(OnTimeout::Kill)),
        )
        .await
        .expect("create");

    assert_eq!(
        create_body(&server).await,
        json!({"templateID": "base", "metadata": {}, "envVars": {}, "autoPause": false})
    );
}

#[tokio::test]
async fn invalid_lifecycles_are_refused_before_any_request() {
    let server = MockServer::start().await;
    create_endpoint(&server).await;
    let client = client_for(&server);
    let invalid = [
        // auto_resume needs a pause on timeout; unset means kill.
        Lifecycle::new().auto_resume(true),
        Lifecycle::new().on_timeout(OnTimeout::Kill).auto_resume(true),
        // A filesystem-only snapshot cannot be resumed by traffic.
        Lifecycle::new()
            .on_timeout(OnTimeout::pause_with(SnapshotMode::Filesystem))
            .auto_resume(true),
    ];

    for lifecycle in invalid {
        let error = client
            .create_sandbox(&CreateSandboxParams::default().lifecycle(lifecycle))
            .await
            .expect_err("invalid lifecycle");
        assert!(matches!(error, Error::InvalidArgument(_)), "{lifecycle:?}: {error:?}");
    }
    assert!(server.received_requests().await.expect("recording").is_empty());
}

#[tokio::test]
async fn the_idempotency_key_is_sent_as_a_header() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v2/sandboxes"))
        .and(header("idempotency-key", "retry-0a1b"))
        .and(body_json(json!({"templateID": "base", "metadata": {}, "envVars": {}})))
        .respond_with(ResponseTemplate::new(201).set_body_json(sandbox_json("0.5.7")))
        .expect(1)
        .mount(&server)
        .await;

    client_for(&server)
        .create_sandbox(&CreateSandboxParams::default().idempotency_key("retry-0a1b"))
        .await
        .expect("create");
}

#[tokio::test]
async fn no_idempotency_key_sends_no_header() {
    let server = MockServer::start().await;
    create_endpoint(&server).await;

    client_for(&server)
        .create_sandbox(&CreateSandboxParams::default())
        .await
        .expect("create");

    let requests = server.received_requests().await.expect("recording");
    assert!(requests[0].headers.get("idempotency-key").is_none());
}

#[tokio::test]
async fn invalid_idempotency_keys_are_refused_before_any_request() {
    let server = MockServer::start().await;
    create_endpoint(&server).await;
    let client = client_for(&server);

    for key in [String::new(), "Upper".to_owned(), "under_score".to_owned(), "a".repeat(256)] {
        let error = client
            .create_sandbox(&CreateSandboxParams::default().idempotency_key(key.clone()))
            .await
            .expect_err("invalid key");
        assert!(matches!(error, Error::InvalidArgument(_)), "{key:?}: {error:?}");
    }
    assert!(server.received_requests().await.expect("recording").is_empty());
}

#[tokio::test]
async fn get_info_reads_lifecycle_and_pause_mode() {
    let server = MockServer::start().await;
    let detail = |lifecycle: serde_json::Value, pause_mode: serde_json::Value| {
        json!({
            "templateID": "base", "sandboxID": SANDBOX_ID, "clientID": "c",
            "startedAt": "2026-10-27T12:00:00Z", "endAt": "2026-10-27T12:05:00Z",
            "envdVersion": "0.5.7", "cpuCount": 2, "memoryMB": 512, "diskSizeMB": 2048,
            "state": "paused", "lifecycle": lifecycle, "pauseMode": pause_mode
        })
    };
    Mock::given(method("GET"))
        .and(path(format!("/sandboxes/{SANDBOX_ID}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(detail(
            json!({"onTimeout": "pause", "autoResume": true}),
            json!("filesystem"),
        )))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/sandboxes/{SANDBOX_ID}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(detail(
            json!({"onTimeout": "hibernate", "autoResume": false}),
            json!("someday"),
        )))
        .mount(&server)
        .await;
    let client = client_for(&server);

    let known = client.get_sandbox_info(SANDBOX_ID).await.expect("known");
    let unknown = client.get_sandbox_info(SANDBOX_ID).await.expect("unknown");

    let lifecycle = known.lifecycle.expect("lifecycle");
    assert_eq!(lifecycle.on_timeout, TimeoutAction::Pause);
    assert!(lifecycle.auto_resume);
    assert_eq!(known.pause_mode, Some(SnapshotMode::Filesystem));
    assert_eq!(unknown.lifecycle.expect("lifecycle").on_timeout, TimeoutAction::Unknown);
    assert_eq!(unknown.pause_mode, None);
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --test rest_lifecycle`
Expected: compile error, `Lifecycle` not found.

- [ ] **Step 3: Implement `src/lifecycle.rs`**

```rust
//! What a sandbox does when its timeout expires, and what a pause or a
//! snapshot keeps (`e2b/sandbox/sandbox_api.py:627-791, 1046-1125`).

use serde::{Deserialize, Deserializer, Serialize};

use crate::error::{Error, Result};

/// What a pause or snapshot persists (`SnapshotMode`,
/// `e2b/sandbox/sandbox_api.py:627-636`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum SnapshotMode {
    /// The filesystem and the memory: running processes, in-memory state
    /// and open connections come back on resume.
    Full,
    /// Only the filesystem: smaller and faster, and resuming cold-boots the
    /// sandbox from disk.
    Filesystem,
}

impl SnapshotMode {
    /// The API's `memory` flag for this mode (`resolve_snapshot_memory`).
    pub(crate) fn keeps_memory(self) -> bool {
        matches!(self, Self::Full)
    }
}

/// What happens when a sandbox's timeout expires.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum OnTimeout {
    /// Kill the sandbox (the API's default).
    Kill,
    /// Pause it, so [`crate::Client::connect_sandbox`] can resume it.
    Pause {
        /// What the pause keeps; `None` leaves it to the API (currently
        /// [`SnapshotMode::Full`]).
        mode: Option<SnapshotMode>,
    },
}

impl OnTimeout {
    /// Pause on timeout, with the API's default snapshot.
    ///
    /// # Returns
    ///
    /// [`OnTimeout::Pause`] with no mode.
    #[must_use]
    pub fn pause() -> Self {
        Self::Pause { mode: None }
    }

    /// Pause on timeout, keeping what `mode` says.
    ///
    /// # Arguments
    ///
    /// * `mode` - full or filesystem-only.
    ///
    /// # Returns
    ///
    /// [`OnTimeout::Pause`] with that mode.
    #[must_use]
    pub fn pause_with(mode: SnapshotMode) -> Self {
        Self::Pause { mode: Some(mode) }
    }
}

/// Lifecycle settings for a new sandbox ([`crate::CreateSandboxParams::lifecycle`],
/// the SDK's `lifecycle`). Unset fields are left to the API.
///
/// # Examples
///
/// ```
/// use e2b_client::{CreateSandboxParams, Lifecycle, OnTimeout, SnapshotMode};
///
/// let lifecycle = Lifecycle::new()
///     .on_timeout(OnTimeout::pause_with(SnapshotMode::Full))
///     .auto_resume(true);
/// let params = CreateSandboxParams::new("base").lifecycle(lifecycle);
/// # let _ = params;
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Lifecycle {
    on_timeout: Option<OnTimeout>,
    auto_resume: Option<bool>,
}

impl Lifecycle {
    /// Nothing set: the API's defaults (kill on timeout).
    ///
    /// # Returns
    ///
    /// Empty settings.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// What happens at the timeout.
    ///
    /// # Arguments
    ///
    /// * `on_timeout` - kill, or pause with an optional mode.
    ///
    /// # Returns
    ///
    /// The settings, for chaining.
    #[must_use]
    pub fn on_timeout(mut self, on_timeout: OnTimeout) -> Self {
        self.on_timeout = Some(on_timeout);
        self
    }

    /// Whether traffic to a paused sandbox resumes it. `true` needs
    /// [`OnTimeout::Pause`] without [`SnapshotMode::Filesystem`]; `false`
    /// opts out even if the API's default changes.
    ///
    /// # Arguments
    ///
    /// * `auto_resume` - whether activity resumes the sandbox.
    ///
    /// # Returns
    ///
    /// The settings, for chaining.
    #[must_use]
    pub fn auto_resume(mut self, auto_resume: bool) -> Self {
        self.auto_resume = Some(auto_resume);
        self
    }

    /// The create-body fields, after the SDK's checks
    /// (`build_lifecycle_config`, `e2b/sandbox/sandbox_api.py:1054-1125`).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidArgument`] for `auto_resume(true)` without a pause on
    /// timeout, or with a filesystem-only pause.
    pub(crate) fn body(&self) -> Result<LifecycleBody> {
        // `Some(mode)` for a pause (whose own mode may be `None`), `None` otherwise.
        let pause_mode = match self.on_timeout {
            Some(OnTimeout::Pause { mode }) => Some(mode),
            Some(OnTimeout::Kill) | None => None,
        };
        if self.auto_resume == Some(true) {
            if pause_mode.is_none() {
                return Err(Error::InvalidArgument(
                    "auto_resume can only be true when on_timeout is a pause".into(),
                ));
            }
            if pause_mode == Some(Some(SnapshotMode::Filesystem)) {
                return Err(Error::InvalidArgument(
                    "auto_resume cannot be true with a filesystem-only pause: such a \
                     sandbox must be resumed with connect_sandbox"
                        .into(),
                ));
            }
        }
        Ok(LifecycleBody {
            auto_pause: self.on_timeout.map(|on_timeout| on_timeout != OnTimeout::Kill),
            // `flatten` turns `Option<Option<_>>` into `Option<_>`.
            auto_pause_memory: pause_mode.flatten().map(SnapshotMode::keeps_memory),
            auto_resume: self.auto_resume.map(|enabled| AutoResume { enabled }),
        })
    }
}

/// The lifecycle fields of `NewSandboxV2`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LifecycleBody {
    pub(crate) auto_pause: Option<bool>,
    pub(crate) auto_pause_memory: Option<bool>,
    pub(crate) auto_resume: Option<AutoResume>,
}

/// `SandboxAutoResumeConfig`: `{enabled}`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub(crate) struct AutoResume {
    enabled: bool,
}

/// The timeout action a sandbox reports (`SandboxLifecycle.onTimeout`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum TimeoutAction {
    /// Killed at the timeout.
    Kill,
    /// Paused at the timeout.
    Pause,
    /// An action this version of the crate does not know.
    #[serde(other)]
    Unknown,
}

/// A sandbox's lifecycle as [`crate::Client::get_sandbox_info`] reports it.
///
/// This is a plain data record, so its fields are public. It is
/// `#[non_exhaustive]` so fields can be added without a breaking change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct LifecycleInfo {
    /// What happens at the timeout.
    pub on_timeout: TimeoutAction,
    /// Whether activity resumes the paused sandbox.
    #[serde(default)]
    pub auto_resume: bool,
}

/// Reads `pauseMode`, treating `null` and values this crate does not know as
/// `None`, so a mode the API adds later does not fail the whole detail.
pub(crate) fn deserialize_known_mode<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Option<SnapshotMode>, D::Error> {
    let raw = Option::<String>::deserialize(deserializer)?;
    Ok(match raw.as_deref() {
        Some("full") => Some(SnapshotMode::Full),
        Some("filesystem") => Some(SnapshotMode::Filesystem),
        _ => None,
    })
}
```

(`std::result::Result` is spelled out because `crate::error::Result` is in scope.) Add a `mod tests` with unit tests of `Lifecycle::body()`: the four valid shapes the mock tests use, and the three refusals.

- [ ] **Step 4: Rework `CreateSandboxParams`**

In `src/models.rs`, replace the field `auto_pause: Option<bool>` with `lifecycle: Lifecycle` (initialised to `Lifecycle::new()` in `new`), and add `idempotency_key: Option<String>` (initialised to `None`). `auto_pause` keeps its signature:

```rust
    /// Pauses instead of killing the sandbox when its timeout expires.
    /// Shorthand for [`CreateSandboxParams::lifecycle`] with
    /// `on_timeout(OnTimeout::pause())` (`true`) or `on_timeout(OnTimeout::Kill)`
    /// (`false`); the later of the two calls wins.
    ///
    /// # Arguments
    ///
    /// * `auto_pause` - whether to auto-pause.
    ///
    /// # Returns
    ///
    /// The parameters, for chaining.
    #[must_use]
    pub fn auto_pause(mut self, auto_pause: bool) -> Self {
        let on_timeout = if auto_pause { OnTimeout::pause() } else { OnTimeout::Kill };
        self.lifecycle = self.lifecycle.on_timeout(on_timeout);
        self
    }

    /// Sets the lifecycle: what happens at the timeout, and auto-resume.
    /// Replaces any earlier [`CreateSandboxParams::auto_pause`].
    ///
    /// # Arguments
    ///
    /// * `lifecycle` - the settings.
    ///
    /// # Returns
    ///
    /// The parameters, for chaining.
    #[must_use]
    pub fn lifecycle(mut self, lifecycle: Lifecycle) -> Self {
        self.lifecycle = lifecycle;
        self
    }

    /// Makes the create request idempotent: a retry with the same key gets
    /// the same sandbox instead of a second one. Sent as the
    /// `Idempotency-Key` header.
    ///
    /// # Arguments
    ///
    /// * `key` - 1 to 255 lowercase ASCII letters, digits or hyphens, for
    ///   example a lowercase UUID. Checked by [`crate::Client::create_sandbox`].
    ///
    /// # Returns
    ///
    /// The parameters, for chaining.
    #[must_use]
    pub fn idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }

    /// The idempotency key, if set and valid (`e2b/retry.py:29-44`).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidArgument`] for a key that is empty, longer than 255
    /// characters, or has a character outside `a-z`, `0-9` and `-`.
    pub(crate) fn checked_idempotency_key(&self) -> Result<Option<&str>> {
        let Some(key) = self.idempotency_key.as_deref() else {
            return Ok(None);
        };
        let valid = (1..=255).contains(&key.len())
            && key
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-');
        if valid {
            Ok(Some(key))
        } else {
            Err(Error::InvalidArgument(format!(
                "idempotency key must be 1-255 lowercase ASCII letters, digits or hyphens \
                 (got {} characters)",
                key.len()
            )))
        }
    }
```

`body()` becomes fallible:

```rust
    /// The request body (`NewSandboxV2`, `e2b/api/client/models/new_sandbox_v2.py`).
    ///
    /// # Errors
    ///
    /// As [`Lifecycle`]'s checks: [`Error::InvalidArgument`].
    pub(crate) fn body(&self) -> Result<NewSandboxBody<'_>> {
        let lifecycle = self.lifecycle.body()?;
        Ok(NewSandboxBody {
            template_id: &self.template,
            timeout: self.timeout.map(|timeout| timeout.as_secs()),
            metadata: &self.metadata,
            env_vars: &self.envs,
            auto_pause: lifecycle.auto_pause,
            auto_pause_memory: lifecycle.auto_pause_memory,
            auto_resume: lifecycle.auto_resume,
            allow_internet_access: self.allow_internet_access,
            network: self.network.as_ref().filter(|network| !network.is_empty()),
        })
    }
```

`NewSandboxBody` gains, after `auto_pause`:

```rust
    #[serde(rename = "autoPauseMemory", skip_serializing_if = "Option::is_none")]
    auto_pause_memory: Option<bool>,
    #[serde(rename = "autoResume", skip_serializing_if = "Option::is_none")]
    auto_resume: Option<AutoResume>,
```

The `network` line is Task 4's; keep whatever fields `body()` has on your base. Change the two `models.rs` unit tests that call `params.body()` to `params.body().expect("body")`. `SandboxInfo` gains:

```rust
    /// The sandbox's lifecycle settings, when reported.
    #[serde(default)]
    pub lifecycle: Option<LifecycleInfo>,
    /// What the sandbox's pause keeps, when reported and known.
    #[serde(default, deserialize_with = "deserialize_known_mode")]
    pub pause_mode: Option<SnapshotMode>,
```

- [ ] **Step 5: Send it from `create_sandbox`**

In `src/client.rs`, a constant next to the others, `const HEADER_IDEMPOTENCY_KEY: &str = "Idempotency-Key";` (`post_v2_sandboxes.py:21`), and:

```rust
    pub async fn create_sandbox(&self, params: &CreateSandboxParams) -> Result<Sandbox> {
        // Every check runs before any request, as in the SDK.
        let body = params.body()?;
        let idempotency_key = params.checked_idempotency_key()?;
        let mut request = self.request(Method::POST, &["v2", "sandboxes"])?.json(&body);
        if let Some(key) = idempotency_key {
            request = request.header(HEADER_IDEMPOTENCY_KEY, key);
        }
        let created: SandboxCreated = parse_success(request.send().await?).await?;
        Sandbox::from_created(self.clone(), created)
    }
```

Add `[`Error::InvalidArgument`] for an invalid lifecycle or idempotency key, before any request` to its `# Errors`. `src/lib.rs`: `mod lifecycle;` and `pub use lifecycle::{Lifecycle, LifecycleInfo, OnTimeout, SnapshotMode, TimeoutAction};`.

- [ ] **Step 6: Run the tests**

Run: `cargo test --test rest_lifecycle`, `cargo test --lib`, `cargo test --test rest_api`
Expected: PASS, with `create_sandbox_posts_v2_body_with_api_key` and the `auto_pause` unit test unchanged.

- [ ] **Step 7: Live tests**

```rust
#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0145_lifecycle_pause_with_full_snapshot_pauses_at_timeout() {
    // Arrange
    let lifecycle = Lifecycle::new()
        .on_timeout(OnTimeout::pause_with(SnapshotMode::Full))
        .auto_resume(false);
    let sandbox = LiveSandbox::create(
        &params().timeout(Duration::from_secs(15)).lifecycle(lifecycle),
    )
    .await;

    // Act: as clm_0011, but with the lifecycle form.
    let sandbox_ref = &sandbox;
    let info = poll_until("auto-pause at timeout", Duration::from_secs(120), || async move {
        match sandbox_ref.get_info().await {
            Ok(info) => (info.state == SandboxState::Paused).then_some(info),
            Err(error) if error.is_not_found() => {
                panic!("sandbox was killed, not paused, at its timeout: {error:?}")
            }
            Err(_) => None,
        }
    })
    .await;

    // Assert
    let reported = info.lifecycle.expect("lifecycle in the detail");
    assert_eq!(reported.on_timeout, TimeoutAction::Pause);
    assert!(!reported.auto_resume);
    assert_eq!(info.pause_mode, Some(SnapshotMode::Full), "{info:?}");
}

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0153_get_info_reports_lifecycle() {
    // Arrange
    let lifecycle = Lifecycle::new().on_timeout(OnTimeout::Kill).auto_resume(false);
    let sandbox = LiveSandbox::create(&params().lifecycle(lifecycle)).await;

    // Act
    let info = sandbox.get_info().await.expect("info");

    // Assert
    let reported = info.lifecycle.expect("lifecycle in the detail");
    assert_eq!(reported.on_timeout, TimeoutAction::Kill);
    assert!(!reported.auto_resume);
}
```

Run both, and `clm_0011` (which uses `auto_pause`), on hosted. Expected: PASS. If hosted leaves `pauseMode` out of the detail, record what it returns and assert on that, with a plan correction.

- [ ] **Step 8: Docs, gates, changelog, commit**

`docs/protocol-notes.md`: the "Create body" row gains `autoPauseMemory`, `autoResume` and the header (Task 4 added `network`); new rows "Create lifecycle", "Lifecycle checks", "Idempotency key", and the detail fields `lifecycle` and `pauseMode`. Evidence map: CLM-0158 → the first four `tests/rest_lifecycle.rs` tests and the `lifecycle.rs` unit tests; CLM-0152 → the three idempotency tests; CLM-0167 (lifecycle part) → `get_info_reads_lifecycle_and_pause_mode`. `changelog/<ts>-m3-task-06-lifecycle-and-idempotency.md`.

```bash
git add src/lifecycle.rs src/models.rs src/client.rs src/lib.rs tests/rest_lifecycle.rs tests/live_lifecycle.rs docs/ changelog/
git commit -m "feat: lifecycle and idempotency key options at sandbox create"
```

---

### Task 7: Pause and resume modes (CLM-0146, 0159)

**Files:**
- Modify: `src/lifecycle.rs` (`PauseOptions`, `OnResume`, `ConnectOptions`)
- Modify: `src/models.rs` (`PauseBody` and `ConnectBody` replace `EmptyBody`), `src/client.rs` (`pause_sandbox_with`, `connect_sandbox_with`), `src/sandbox.rs` (`pause_with`), `src/lib.rs`
- Modify: `tests/rest_lifecycle.rs`, `tests/live_lifecycle.rs`, `docs/protocol-notes.md` (the "Pause" and "Connect" rows), `docs/verification/s4-evidence-map.md`

- [ ] **Step 1: Write the failing mock tests**

Append to `tests/rest_lifecycle.rs` (add `ConnectOptions`, `OnResume`, `PauseOptions` and `std::time::Duration` to its imports):

```rust
#[tokio::test]
async fn pause_sends_memory_from_the_mode() {
    // Arrange
    let server = MockServer::start().await;
    for (body, label) in [
        (json!({"memory": true}), "full"),
        (json!({"memory": false}), "filesystem"),
        (json!({}), "default"),
    ] {
        Mock::given(method("POST"))
            .and(path(format!("/sandboxes/{SANDBOX_ID}/pause")))
            .and(body_json(body))
            .respond_with(ResponseTemplate::new(204))
            .expect(1)
            .named(label)
            .mount(&server)
            .await;
    }
    let client = client_for(&server);

    // Act and assert: each call matches only its own mock.
    let full = PauseOptions::new().mode(SnapshotMode::Full);
    let filesystem = PauseOptions::new().mode(SnapshotMode::Filesystem);
    assert!(client.pause_sandbox_with(SANDBOX_ID, &full).await.expect("full"));
    assert!(client.pause_sandbox_with(SANDBOX_ID, &filesystem).await.expect("filesystem"));
    assert!(client.pause_sandbox(SANDBOX_ID).await.expect("default"));
}

#[tokio::test]
async fn connect_sends_memory_false_only_for_a_reboot() {
    let server = MockServer::start().await;
    for (body, label) in [
        (json!({"timeout": 60, "memory": false}), "reboot"),
        (json!({}), "restore"),
    ] {
        Mock::given(method("POST"))
            .and(path(format!("/v2/sandboxes/{SANDBOX_ID}/connect")))
            .and(body_json(body))
            .respond_with(ResponseTemplate::new(200).set_body_json(sandbox_json("0.5.7")))
            .expect(1)
            .named(label)
            .mount(&server)
            .await;
    }
    let client = client_for(&server);
    let reboot = ConnectOptions::new()
        .timeout(Duration::from_secs(60))
        .on_resume(OnResume::Reboot);
    let restore = ConnectOptions::new().on_resume(OnResume::Restore);

    client.connect_sandbox_with(SANDBOX_ID, &reboot).await.expect("reboot");
    client.connect_sandbox_with(SANDBOX_ID, &restore).await.expect("restore");
}

#[tokio::test]
async fn the_v1_fallback_keeps_the_reboot_flag() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/v2/sandboxes/{SANDBOX_ID}/connect")))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path(format!("/sandboxes/{SANDBOX_ID}/connect")))
        .and(body_json(json!({"timeout": 300, "memory": false})))
        .respond_with(ResponseTemplate::new(200).set_body_json(sandbox_json("0.5.7")))
        .expect(1)
        .mount(&server)
        .await;

    client_for(&server)
        .connect_sandbox_with(SANDBOX_ID, &ConnectOptions::new().on_resume(OnResume::Reboot))
        .await
        .expect("connect");
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --test rest_lifecycle -- pause_ connect_ v1_`
Expected: compile error, `PauseOptions` not found.

- [ ] **Step 3: Implement**

`src/lifecycle.rs`:

```rust
/// Options for [`crate::Client::pause_sandbox_with`].
///
/// # Examples
///
/// ```
/// use e2b_client::{PauseOptions, SnapshotMode};
///
/// let options = PauseOptions::new().mode(SnapshotMode::Filesystem);
/// # let _ = options;
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PauseOptions {
    mode: Option<SnapshotMode>,
}

impl PauseOptions {
    /// The API's default pause (currently [`SnapshotMode::Full`]).
    ///
    /// # Returns
    ///
    /// Empty options.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// What the pause keeps. The API may refuse [`SnapshotMode::Filesystem`]
    /// with HTTP 400 (not enabled for the team) or 409 (the sandbox's node is
    /// too old); a full pause works in both cases.
    ///
    /// # Arguments
    ///
    /// * `mode` - full or filesystem-only.
    ///
    /// # Returns
    ///
    /// The options, for chaining.
    #[must_use]
    pub fn mode(mut self, mode: SnapshotMode) -> Self {
        self.mode = Some(mode);
        self
    }

    /// The body's `memory` flag; `None` leaves it out.
    pub(crate) fn memory(&self) -> Option<bool> {
        self.mode.map(SnapshotMode::keeps_memory)
    }
}

/// How a paused sandbox comes back (`SandboxOnResume`,
/// `e2b/sandbox/sandbox_api.py:783-791`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum OnResume {
    /// Restore the memory snapshot, so processes and connections survive
    /// (the API's default).
    Restore,
    /// Ignore any memory in the snapshot and cold-boot from disk: the rescue
    /// path for a snapshot whose memory image wedges the guest.
    Reboot,
}

/// Options for [`crate::Client::connect_sandbox_with`].
///
/// # Examples
///
/// ```
/// use std::time::Duration;
/// use e2b_client::{ConnectOptions, OnResume};
///
/// let options = ConnectOptions::new()
///     .timeout(Duration::from_secs(600))
///     .on_resume(OnResume::Reboot);
/// # let _ = options;
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ConnectOptions {
    timeout: Option<Duration>,
    on_resume: Option<OnResume>,
}

impl ConnectOptions {
    /// No new timeout, and the API's default resume.
    ///
    /// # Returns
    ///
    /// Empty options.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A new sandbox lifetime from now, sent in whole seconds.
    ///
    /// # Arguments
    ///
    /// * `timeout` - the lifetime.
    ///
    /// # Returns
    ///
    /// The options, for chaining.
    #[must_use]
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// How a paused sandbox resumes. Ignored for a running sandbox.
    ///
    /// # Arguments
    ///
    /// * `on_resume` - restore or reboot.
    ///
    /// # Returns
    ///
    /// The options, for chaining.
    #[must_use]
    pub fn on_resume(mut self, on_resume: OnResume) -> Self {
        self.on_resume = Some(on_resume);
        self
    }

    pub(crate) fn timeout_value(&self) -> Option<Duration> {
        self.timeout
    }

    /// `memory: false` for a reboot; restore is the API's default and is
    /// left out (`resolve_connect_memory`, `e2b/sandbox/sandbox_api.py:761-780`).
    pub(crate) fn memory(&self) -> Option<bool> {
        (self.on_resume == Some(OnResume::Reboot)).then_some(false)
    }
}
```

with `use std::time::Duration;` at the top of `lifecycle.rs`. In `src/models.rs`, replace `EmptyBody` with:

```rust
/// `SandboxPauseRequest` body: `{memory?}`.
#[derive(Debug, Serialize)]
pub(crate) struct PauseBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) memory: Option<bool>,
}

/// `ConnectSandboxV2` and v1 `ConnectSandbox` body: `{timeout?, memory?}`.
#[derive(Debug, Serialize)]
pub(crate) struct ConnectBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) timeout: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) memory: Option<bool>,
}
```

In `src/client.rs`, `connect_sandbox` keeps its doc comment and signature and delegates; the body moves to `connect_sandbox_with`:

```rust
    pub async fn connect_sandbox(
        &self,
        sandbox_id: &str,
        timeout: Option<Duration>,
    ) -> Result<Sandbox> {
        let options = match timeout {
            Some(timeout) => ConnectOptions::new().timeout(timeout),
            None => ConnectOptions::new(),
        };
        self.connect_sandbox_with(sandbox_id, &options).await
    }

    /// [`Client::connect_sandbox`] with the SDK 2.61 options: the timeout and
    /// how a paused sandbox resumes.
    ///
    /// # Arguments
    ///
    /// * `sandbox_id` - the sandbox to connect to.
    /// * `options` - the timeout and the resume mode.
    ///
    /// # Returns
    ///
    /// A [`Sandbox`] handle.
    ///
    /// # Errors
    ///
    /// As [`Client::connect_sandbox`].
    pub async fn connect_sandbox_with(
        &self,
        sandbox_id: &str,
        options: &ConnectOptions,
    ) -> Result<Sandbox> {
        let memory = options.memory();
        let v2_body = ConnectBody {
            timeout: options.timeout_value().map(|timeout| timeout.as_secs()),
            memory,
        };
        let response = self
            .send_json(Method::POST, &["v2", "sandboxes", sandbox_id, "connect"], &v2_body)
            .await?;
        let response = if response.status() == StatusCode::NOT_FOUND {
            // The v1 body requires a timeout (`connect_sandbox.py:48`).
            let v1_body = ConnectBody {
                timeout: Some(
                    options
                        .timeout_value()
                        .unwrap_or(DEFAULT_SANDBOX_TIMEOUT)
                        .as_secs(),
                ),
                memory,
            };
            self.send_json(Method::POST, &["sandboxes", sandbox_id, "connect"], &v1_body)
                .await?
        } else {
            response
        };
        let created: SandboxCreated = parse_sandbox_response(response, sandbox_id).await?;
        Sandbox::from_created(self.clone(), created)
    }
```

`pause_sandbox` likewise keeps its signature and calls `self.pause_sandbox_with(sandbox_id, &PauseOptions::new()).await`; `pause_sandbox_with(&self, sandbox_id: &str, options: &PauseOptions) -> Result<bool>` holds the existing body with `&PauseBody { memory: options.memory() }` in place of `&EmptyBody {}`, and the `# Errors` gain the 400/409 note for filesystem pauses. In `src/sandbox.rs`:

```rust
    /// Pauses the sandbox with options. See [`Client::pause_sandbox_with`].
    ///
    /// # Arguments
    ///
    /// * `options` - what the pause keeps.
    ///
    /// # Returns
    ///
    /// `true` if it was paused, `false` if it was already paused.
    ///
    /// # Errors
    ///
    /// As [`Client::pause_sandbox_with`].
    pub async fn pause_with(&self, options: &PauseOptions) -> Result<bool> {
        self.client.pause_sandbox_with(&self.sandbox_id, options).await
    }
```

`src/lib.rs`: add `ConnectOptions`, `OnResume`, `PauseOptions` to the `lifecycle` re-export.

- [ ] **Step 4: Run the tests**

Run: `cargo test --test rest_lifecycle` and `cargo test --test rest_api`
Expected: PASS, including the existing `connect_sandbox_*` and `pause_sandbox_maps_status_codes` tests.

- [ ] **Step 5: Live tests**

Add a helper to `tests/live_lifecycle.rs`:

```rust
/// Whether `path` exists in `sandbox`.
async fn exists(sandbox: &Sandbox, path: &str) -> bool {
    sandbox
        .commands()
        .run(&format!("test -e {path}"), &CommandOptions::default())
        .await
        .expect("test -e")
        .exit_code
        == 0
}
```

`/dev/shm` is a tmpfs, so a file there lives in memory: a memory restore keeps it and a cold boot loses it. A file in `/tmp` is on disk and survives both.

```rust
#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0146_resume_restores_memory_or_reboots() {
    // Arrange
    let sandbox = LiveSandbox::create(&params().timeout(Duration::from_secs(240))).await;
    let client = sandbox.client();
    sandbox
        .commands()
        .run("touch /dev/shm/in-memory /tmp/on-disk", &CommandOptions::default())
        .await
        .expect("markers");
    let full = PauseOptions::new().mode(SnapshotMode::Full);
    let resume = |on_resume| ConnectOptions::new().timeout(Duration::from_secs(120)).on_resume(on_resume);

    // Act 1: a full pause, then a restore.
    assert!(sandbox.pause_with(&full).await.expect("first pause"));
    let restored = client
        .connect_sandbox_with(sandbox.sandbox_id(), &resume(OnResume::Restore))
        .await
        .expect("restore");
    let kept_after_restore = exists(&restored, "/dev/shm/in-memory").await;

    // Act 2: another full pause, then a reboot.
    assert!(restored.pause_with(&full).await.expect("second pause"));
    let rebooted = client
        .connect_sandbox_with(sandbox.sandbox_id(), &resume(OnResume::Reboot))
        .await
        .expect("reboot");

    // Assert
    assert!(kept_after_restore, "a restore lost the in-memory file");
    assert!(!exists(&rebooted, "/dev/shm/in-memory").await, "a reboot kept the in-memory file");
    assert!(exists(&rebooted, "/tmp/on-disk").await, "a reboot lost the on-disk file");
}

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0146_filesystem_pause_cold_boots() {
    // Arrange
    let sandbox = LiveSandbox::create(&params().timeout(Duration::from_secs(180))).await;
    sandbox
        .commands()
        .run("touch /dev/shm/in-memory /tmp/on-disk", &CommandOptions::default())
        .await
        .expect("markers");

    // Act: hosted E2B may refuse this for the team (400) or node (409); that
    // is an environment limit, recorded as such in the results (Task 12).
    let paused = sandbox
        .pause_with(&PauseOptions::new().mode(SnapshotMode::Filesystem))
        .await
        .expect("filesystem pause (400 or 409 means the team or node lacks it)");
    let resumed = sandbox
        .client()
        .connect_sandbox(sandbox.sandbox_id(), Some(Duration::from_secs(120)))
        .await
        .expect("resume");

    // Assert
    assert!(paused);
    assert!(!exists(&resumed, "/dev/shm/in-memory").await, "a filesystem pause kept memory");
    assert!(exists(&resumed, "/tmp/on-disk").await, "a filesystem pause lost the disk");
}
```

Run both on hosted. Expected: PASS, or for the second a 400/409 refusal, which is reported in the PR as an environment limit and not worked around.

- [ ] **Step 6: Docs, gates, changelog, commit**

`docs/protocol-notes.md`: the "Pause" row gains `{memory}` from the mode and the 400/409 refusals; the "Connect / resume" and "Connect v1" rows gain `memory: false` for a reboot. Evidence map: CLM-0159 → the three new `tests/rest_lifecycle.rs` tests. `changelog/<ts>-m3-task-07-pause-and-resume-modes.md`.

```bash
git add src/lifecycle.rs src/models.rs src/client.rs src/sandbox.rs src/lib.rs tests/rest_lifecycle.rs tests/live_lifecycle.rs docs/ changelog/
git commit -m "feat: snapshot mode on pause and reboot on resume"
```

---

### Task 8: Snapshots (CLM-0030, 0147, 0160, 0161)

**Files:**
- Create: `src/snapshots.rs` (`SnapshotOptions`, `SnapshotInfo`, `ListSnapshotsParams`, `SnapshotPage`)
- Modify: `src/client.rs` (`create_snapshot`, `list_snapshots`, `delete_snapshot`), `src/sandbox.rs` (`create_snapshot`, `list_snapshots`), `src/lib.rs`
- Create: `tests/rest_snapshots.rs`
- Modify: `tests/live_support/mod.rs` (`resource_name`, `params_from`), `tests/live_sweep.rs` (delete the run's snapshots), `tests/live_lifecycle.rs`
- Modify: `docs/protocol-notes.md` (new section "Snapshots and fork"), `docs/verification/s4-evidence-map.md`

- [ ] **Step 1: Write the failing mock tests**

`tests/rest_snapshots.rs`:

```rust
//! Snapshots against a mock server.

mod common;

use common::{SANDBOX_ID, client_for, sandbox_json};
use e2b_client::{Error, ListSnapshotsParams, SnapshotMode, SnapshotOptions};
use serde_json::json;
use wiremock::matchers::{body_json, method, path, query_param, query_param_is_missing};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn create_snapshot_posts_name_and_memory() {
    // Arrange
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/sandboxes/{SANDBOX_ID}/snapshots")))
        .and(body_json(json!({"name": "my-snap", "memory": false})))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({
            "snapshotID": "tpl123:v1", "names": ["team/my-snap:v1"]
        })))
        .expect(1)
        .mount(&server)
        .await;
    let options = SnapshotOptions::new().name("my-snap").mode(SnapshotMode::Filesystem);

    // Act
    let snapshot = client_for(&server)
        .create_snapshot(SANDBOX_ID, &options)
        .await
        .expect("snapshot");

    // Assert
    assert_eq!(snapshot.snapshot_id, "tpl123:v1");
    assert_eq!(snapshot.names, ["team/my-snap:v1"]);
}

#[tokio::test]
async fn create_snapshot_with_defaults_sends_an_empty_object() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/sandboxes/{SANDBOX_ID}/snapshots")))
        .and(body_json(json!({})))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({"snapshotID": "t:1", "names": []})))
        .expect(1)
        .mount(&server)
        .await;

    client_for(&server)
        .create_snapshot(SANDBOX_ID, &SnapshotOptions::default())
        .await
        .expect("snapshot");
}

#[tokio::test]
async fn create_snapshot_of_a_missing_sandbox_is_not_found() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/sandboxes/{SANDBOX_ID}/snapshots")))
        .respond_with(ResponseTemplate::new(404).set_body_json(json!({"code": 404, "message": "nope"})))
        .mount(&server)
        .await;

    let error = client_for(&server)
        .create_snapshot(SANDBOX_ID, &SnapshotOptions::default())
        .await
        .expect_err("not found");

    assert!(matches!(error, Error::SandboxNotFound(_)), "{error:?}");
}

#[tokio::test]
async fn list_snapshots_sends_filters_and_reads_the_next_token() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/snapshots"))
        .and(query_param("sandboxID", SANDBOX_ID))
        .and(query_param("name", "my-snap"))
        .and(query_param("limit", "2"))
        .and(query_param("nextToken", "page-1"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("x-next-token", "page-2")
                .set_body_json(json!([{"snapshotID": "t:1", "names": ["team/my-snap:v1"]}])),
        )
        .expect(1)
        .mount(&server)
        .await;
    let params = ListSnapshotsParams::new()
        .sandbox_id(SANDBOX_ID)
        .name("my-snap")
        .limit(2)
        .next_token("page-1");

    let page = client_for(&server).list_snapshots(&params).await.expect("list");

    assert_eq!(page.snapshots.len(), 1);
    assert_eq!(page.next_token.as_deref(), Some("page-2"));
}

#[tokio::test]
async fn list_snapshots_without_filters_sends_no_query() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/snapshots"))
        .and(query_param_is_missing("sandboxID"))
        .and(query_param_is_missing("name"))
        .and(query_param_is_missing("limit"))
        .and(query_param_is_missing("nextToken"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .expect(1)
        .mount(&server)
        .await;

    let page = client_for(&server)
        .list_snapshots(&ListSnapshotsParams::new())
        .await
        .expect("list");

    assert!(page.snapshots.is_empty() && page.next_token.is_none());
}

#[tokio::test]
async fn delete_snapshot_encodes_the_id_as_one_segment() {
    let server = MockServer::start().await;
    // `/` is percent-encoded so `team/my-snap:v1` stays one path segment.
    Mock::given(method("DELETE"))
        .and(path("/templates/team%2Fmy-snap:v1"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/templates/gone"))
        .respond_with(ResponseTemplate::new(404))
        .expect(1)
        .mount(&server)
        .await;
    let client = client_for(&server);

    assert!(client.delete_snapshot("team/my-snap:v1").await.expect("delete"));
    assert!(!client.delete_snapshot("gone").await.expect("delete missing"));
}

#[tokio::test]
async fn sandbox_list_snapshots_filters_by_its_own_id() {
    let server = MockServer::start().await;
    let sandbox = common::sandbox_from_json(&server, sandbox_json("0.5.7")).await;
    Mock::given(method("GET"))
        .and(path("/snapshots"))
        .and(query_param("sandboxID", SANDBOX_ID))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .expect(1)
        .mount(&server)
        .await;

    sandbox
        .list_snapshots(&ListSnapshotsParams::new())
        .await
        .expect("list");
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --test rest_snapshots`
Expected: compile error, `SnapshotOptions` not found. If wiremock's `path` matcher compares the decoded path, change the first delete matcher to `path("/templates/team/my-snap:v1")` and assert the raw request URL contains `team%2Fmy-snap:v1` instead (record it as a plan correction).

- [ ] **Step 3: Implement `src/snapshots.rs`**

```rust
//! Snapshots: persistent copies of a sandbox that new sandboxes can start
//! from (`e2b/sandbox_async/main.py:886-1060`).

use serde::{Deserialize, Serialize};

use crate::lifecycle::SnapshotMode;

/// Options for [`crate::Client::create_snapshot`].
///
/// # Examples
///
/// ```
/// use e2b_client::{SnapshotMode, SnapshotOptions};
///
/// let options = SnapshotOptions::new().name("before-upgrade").mode(SnapshotMode::Full);
/// # let _ = options;
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SnapshotOptions {
    name: Option<String>,
    mode: Option<SnapshotMode>,
}

impl SnapshotOptions {
    /// An unnamed snapshot with the API's default mode (full).
    ///
    /// # Returns
    ///
    /// Empty options.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Names the snapshot. If a snapshot with this name exists, the new one
    /// becomes a new build of it instead of a new snapshot.
    ///
    /// # Arguments
    ///
    /// * `name` - the snapshot name.
    ///
    /// # Returns
    ///
    /// The options, for chaining.
    #[must_use]
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// What the snapshot keeps. Sandboxes made from a
    /// [`SnapshotMode::Filesystem`] snapshot cold-boot. The API may refuse
    /// that mode with 400 or 409, as for [`crate::PauseOptions::mode`].
    ///
    /// # Arguments
    ///
    /// * `mode` - full or filesystem-only.
    ///
    /// # Returns
    ///
    /// The options, for chaining.
    #[must_use]
    pub fn mode(mut self, mode: SnapshotMode) -> Self {
        self.mode = Some(mode);
        self
    }

    /// `SandboxSnapshotRequest`: `{name?, memory?}`.
    pub(crate) fn body(&self) -> SnapshotRequest<'_> {
        SnapshotRequest {
            name: self.name.as_deref(),
            memory: self.mode.map(SnapshotMode::keeps_memory),
        }
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct SnapshotRequest<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    memory: Option<bool>,
}

/// A snapshot (`SnapshotInfo`, `e2b/sandbox/sandbox_api.py:1364-1370`).
/// Create a sandbox from it with `CreateSandboxParams::new(snapshot_id)`.
///
/// This is a plain data record, so its fields are public. It is
/// `#[non_exhaustive]` so fields can be added without a breaking change.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[non_exhaustive]
pub struct SnapshotInfo {
    /// The snapshot's ID: a template ID with a tag, or a namespaced name with
    /// a tag, e.g. `my-snapshot:latest`.
    #[serde(rename = "snapshotID")]
    pub snapshot_id: String,
    /// The snapshot's full names, e.g. `project-slug/my-snapshot:v2`.
    #[serde(default)]
    pub names: Vec<String>,
}

/// Filters and paging for [`crate::Client::list_snapshots`].
///
/// # Examples
///
/// ```
/// use e2b_client::ListSnapshotsParams;
///
/// let params = ListSnapshotsParams::new().name("before-upgrade").limit(10);
/// # let _ = params;
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ListSnapshotsParams {
    sandbox_id: Option<String>,
    name: Option<String>,
    limit: Option<u32>,
    next_token: Option<String>,
}

impl ListSnapshotsParams {
    /// No filters: every snapshot of the team.
    ///
    /// # Returns
    ///
    /// Empty parameters.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Only snapshots taken from this sandbox.
    ///
    /// # Arguments
    ///
    /// * `sandbox_id` - the source sandbox.
    ///
    /// # Returns
    ///
    /// The parameters, for chaining.
    #[must_use]
    pub fn sandbox_id(mut self, sandbox_id: impl Into<String>) -> Self {
        self.sandbox_id = Some(sandbox_id.into());
        self
    }

    /// Only snapshots with this name or ID, optionally tag-qualified
    /// (`my-snapshot`, `my-project/my-snapshot` or `my-snapshot:v1`).
    ///
    /// # Arguments
    ///
    /// * `name` - the name or ID.
    ///
    /// # Returns
    ///
    /// The parameters, for chaining.
    #[must_use]
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Maximum number of snapshots per page.
    ///
    /// # Arguments
    ///
    /// * `limit` - the page size.
    ///
    /// # Returns
    ///
    /// The parameters, for chaining.
    #[must_use]
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Continue from a previous page's [`SnapshotPage::next_token`].
    ///
    /// # Arguments
    ///
    /// * `token` - the pagination token.
    ///
    /// # Returns
    ///
    /// The parameters, for chaining.
    #[must_use]
    pub fn next_token(mut self, token: impl Into<String>) -> Self {
        self.next_token = Some(token.into());
        self
    }

    /// The query pairs (`get_snapshots.py:22-28`), each left out when unset.
    pub(crate) fn query(&self) -> Vec<(&'static str, String)> {
        [
            ("sandboxID", self.sandbox_id.clone()),
            ("name", self.name.clone()),
            ("limit", self.limit.map(|limit| limit.to_string())),
            ("nextToken", self.next_token.clone()),
        ]
        .into_iter()
        .filter_map(|(key, value)| value.map(|value| (key, value)))
        .collect()
    }
}

/// One page of [`crate::Client::list_snapshots`] results.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SnapshotPage {
    /// The snapshots on this page.
    pub snapshots: Vec<SnapshotInfo>,
    /// The token for the next page (from `x-next-token`), or `None` on the
    /// last page.
    pub next_token: Option<String>,
}
```

`src/client.rs` (with `use crate::snapshots::{ListSnapshotsParams, SnapshotInfo, SnapshotOptions, SnapshotPage};`):

```rust
    /// Snapshots a sandbox (`POST /sandboxes/{id}/snapshots`). The sandbox
    /// pauses while the snapshot is taken, then keeps running. Snapshots
    /// outlive the sandbox.
    ///
    /// # Arguments
    ///
    /// * `sandbox_id` - the sandbox.
    /// * `options` - the name and mode.
    ///
    /// # Returns
    ///
    /// The snapshot; start sandboxes from it with
    /// `CreateSandboxParams::new(snapshot.snapshot_id)`.
    ///
    /// # Errors
    ///
    /// [`Error::SandboxNotFound`] for HTTP 404, [`Error::Api`] for other
    /// non-2xx answers (400 or 409 for a refused filesystem-only snapshot),
    /// [`Error::Decode`], [`Error::Transport`].
    pub async fn create_snapshot(
        &self,
        sandbox_id: &str,
        options: &SnapshotOptions,
    ) -> Result<SnapshotInfo> {
        let response = self
            .send_json(Method::POST, &["sandboxes", sandbox_id, "snapshots"], &options.body())
            .await?;
        parse_sandbox_response(response, sandbox_id).await
    }

    /// Lists snapshots, one page at a time (`GET /snapshots`).
    ///
    /// # Arguments
    ///
    /// * `params` - filters and the page token.
    ///
    /// # Returns
    ///
    /// One page, with the token for the next page if there is one.
    ///
    /// # Errors
    ///
    /// [`Error::Api`] for a non-2xx answer, [`Error::Decode`],
    /// [`Error::Transport`].
    pub async fn list_snapshots(&self, params: &ListSnapshotsParams) -> Result<SnapshotPage> {
        let response = self
            .request(Method::GET, &["snapshots"])?
            .query(&params.query())
            .send()
            .await?;
        let next_token = next_token_of(&response);
        let snapshots: Option<Vec<SnapshotInfo>> = parse_success(response).await?;
        Ok(SnapshotPage {
            snapshots: snapshots.unwrap_or_default(),
            next_token,
        })
    }

    /// Deletes a snapshot (`DELETE /templates/{id}`; a snapshot is a
    /// template).
    ///
    /// # Arguments
    ///
    /// * `snapshot_id` - the snapshot ID or name.
    ///
    /// # Returns
    ///
    /// `true` if it was deleted, `false` if it did not exist.
    ///
    /// # Errors
    ///
    /// [`Error::Api`] for a non-2xx answer other than 404,
    /// [`Error::Transport`].
    pub async fn delete_snapshot(&self, snapshot_id: &str) -> Result<bool> {
        let response = self
            .request(Method::DELETE, &["templates", snapshot_id])?
            .send()
            .await?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(false);
        }
        ensure_success(response).await.map(|_| true)
    }
```

`next_token_of` is Task 3's helper. If Task 3 has not merged, add it here as Task 3 describes. `src/sandbox.rs`:

```rust
    /// Snapshots this sandbox. See [`Client::create_snapshot`].
    ///
    /// # Arguments
    ///
    /// * `options` - the name and mode.
    ///
    /// # Returns
    ///
    /// The snapshot.
    ///
    /// # Errors
    ///
    /// As [`Client::create_snapshot`].
    pub async fn create_snapshot(&self, options: &SnapshotOptions) -> Result<SnapshotInfo> {
        self.client.create_snapshot(&self.sandbox_id, options).await
    }

    /// Lists the snapshots taken from this sandbox. See
    /// [`Client::list_snapshots`]; any `sandbox_id` in `params` is replaced
    /// by this sandbox's.
    ///
    /// # Arguments
    ///
    /// * `params` - the other filters and the page token.
    ///
    /// # Returns
    ///
    /// One page.
    ///
    /// # Errors
    ///
    /// As [`Client::list_snapshots`].
    pub async fn list_snapshots(&self, params: &ListSnapshotsParams) -> Result<SnapshotPage> {
        let params = params.clone().sandbox_id(self.sandbox_id.clone());
        self.client.list_snapshots(&params).await
    }
```

`src/lib.rs`: `mod snapshots;` and `pub use snapshots::{ListSnapshotsParams, SnapshotInfo, SnapshotOptions, SnapshotPage};`.

- [ ] **Step 4: Run the tests**

Run: `cargo test --test rest_snapshots`
Expected: PASS.

- [ ] **Step 5: Live helpers and the sweep**

`tests/live_support/mod.rs`:

```rust
/// A name for a snapshot or volume this run creates: lowercase, since the
/// API's names are, and with the run id, so the sweep finds it.
pub fn resource_name(slug: &str) -> String {
    format!("e2b-client-test-{}-{slug}", run_id().to_ascii_lowercase())
}

/// Like [`params`], from another template (for example a snapshot ID).
pub fn params_from(template: &str) -> CreateSandboxParams {
    CreateSandboxParams::new(template)
        .timeout(SANDBOX_TIMEOUT)
        .metadata(RUN_METADATA_KEY, run_id())
}
```

and `params()` becomes `params_from(&template())`. In `tests/live_sweep.rs`, a second test after the sandbox sweep (it doesn't depend on test order; each sweep is independent):

```rust
#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn sweep_leftover_snapshots_of_this_run() {
    let client = client();
    let marker = resource_name("");
    let mut params = ListSnapshotsParams::new();
    let mut leftovers = Vec::new();
    loop {
        let page = client.list_snapshots(&params).await.expect("list snapshots");
        leftovers.extend(
            page.snapshots
                .into_iter()
                .filter(|snapshot| snapshot.names.iter().any(|name| name.contains(&marker)))
                .map(|snapshot| snapshot.snapshot_id),
        );
        match page.next_token {
            Some(token) => params = ListSnapshotsParams::new().next_token(token),
            None => break,
        }
    }
    for snapshot_id in &leftovers {
        client.delete_snapshot(snapshot_id).await.expect("delete leftover snapshot");
    }
    eprintln!("sweep: deleted {} leftover snapshot(s) of run {}", leftovers.len(), run_id());
}
```

`resource_name("")` is the shared prefix, ending in `-`. The `just test-live` recipe runs `cargo test --test live_sweep`, so both sweeps run with no recipe change.

- [ ] **Step 6: Live tests**

```rust
#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0030_snapshot_starts_sandboxes_with_its_files() {
    // Arrange
    let source = LiveSandbox::new().await;
    source
        .files()
        .write("/tmp/snapshot.txt", "from the snapshot", None)
        .await
        .expect("write");
    let options = SnapshotOptions::new().name(resource_name("starts"));
    let snapshot = source.create_snapshot(&options).await.expect("snapshot");

    // Act
    let copy = LiveSandbox::create(&params_from(&snapshot.snapshot_id)).await;
    let text = copy
        .files()
        .read_to_string("/tmp/snapshot.txt", None)
        .await
        .expect("read in the copy");

    // Assert
    assert_eq!(text, "from the snapshot");
    assert_eq!(source.get_info().await.expect("source info").state, SandboxState::Running);
    assert!(source.client().delete_snapshot(&snapshot.snapshot_id).await.expect("delete"));
}

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0147_list_and_delete_snapshots() {
    // Arrange
    let source = LiveSandbox::new().await;
    let name = resource_name("listed");
    let snapshot = source
        .create_snapshot(&SnapshotOptions::new().name(name.clone()))
        .await
        .expect("snapshot");
    let client = source.client();
    let listed = |page: SnapshotPage| page.snapshots.iter().any(|each| each.snapshot_id == snapshot.snapshot_id);

    // Act
    let by_sandbox = source.list_snapshots(&ListSnapshotsParams::new()).await.expect("by sandbox");
    let by_name = client
        .list_snapshots(&ListSnapshotsParams::new().name(name))
        .await
        .expect("by name");
    let deleted = client.delete_snapshot(&snapshot.snapshot_id).await.expect("delete");
    let deleted_again = client.delete_snapshot(&snapshot.snapshot_id).await.expect("delete again");

    // Assert
    assert!(listed(by_sandbox), "not listed for its source sandbox");
    assert!(listed(by_name), "not listed by its name");
    assert!(deleted);
    assert!(!deleted_again);
}
```

The source sandbox has one snapshot, so one page is enough here; paging is covered by the mock test. Run both, then `cargo test --test live_sweep -- --ignored` with the same run id, on hosted. Expected: PASS, and the sweep deletes 0 snapshots.

- [ ] **Step 7: Docs, gates, changelog, commit**

`docs/protocol-notes.md`: a new section "Snapshots and fork", with rows for create, list, delete (note the `DELETE /templates/{id}` route and the one-segment encoding, and that `:` stays unencoded while `/` becomes `%2F`) and create-from-snapshot. Evidence map: CLM-0160 → the three create tests; CLM-0161 → the list, delete and `sandbox_list_snapshots_filters_by_its_own_id` tests. `changelog/<ts>-m3-task-08-snapshots.md`.

```bash
git add src/snapshots.rs src/client.rs src/sandbox.rs src/lib.rs tests/rest_snapshots.rs tests/live_support/mod.rs tests/live_sweep.rs tests/live_lifecycle.rs docs/ changelog/
git commit -m "feat: create, list and delete snapshots"
```

---

### Task 9: Fork (CLM-0030, 0162)

**Files:**
- Modify: `src/models.rs` (`ForkOptions`, `MAX_FORK_COUNT`, the fork wire types), `src/client.rs` (`fork_sandbox`), `src/sandbox.rs` (`fork`), `src/lib.rs`
- Create: `tests/rest_fork.rs`
- Modify: `tests/live_support/mod.rs` (`LiveSandbox::adopt`), `tests/live_lifecycle.rs`
- Modify: `docs/protocol-notes.md` ("Snapshots and fork", or "REST endpoints" if Task 8 has not merged), `docs/verification/s4-evidence-map.md`

- [ ] **Step 1: Write the failing mock tests**

`tests/rest_fork.rs`:

```rust
//! Fork against a mock server.

mod common;

use std::time::Duration;

use common::{SANDBOX_ID, client_for, sandbox_json};
use e2b_client::{Error, ForkOptions};
use serde_json::json;
use wiremock::matchers::{body_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn fork_returns_one_result_per_fork() {
    // Arrange
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/sandboxes/{SANDBOX_ID}/fork")))
        .and(body_json(json!({"timeout": 60, "count": 3})))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!([
            {"sandbox": sandbox_json("0.5.7")},
            {"error": {"code": 429, "message": "rate limited"}},
            {}
        ])))
        .expect(1)
        .mount(&server)
        .await;
    let options = ForkOptions::new().timeout(Duration::from_secs(60)).count(3);

    // Act
    let results = client_for(&server)
        .fork_sandbox(SANDBOX_ID, &options)
        .await
        .expect("fork");

    // Assert
    assert_eq!(results.len(), 3);
    assert_eq!(results[0].as_ref().expect("first fork").sandbox_id(), SANDBOX_ID);
    assert!(
        matches!(&results[1], Err(Error::Api { status: 429, message }) if message == "rate limited"),
        "{:?}",
        results[1]
    );
    assert!(matches!(&results[2], Err(Error::Protocol(_))), "{:?}", results[2]);
}

#[tokio::test]
async fn fork_with_defaults_sends_an_empty_object() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/sandboxes/{SANDBOX_ID}/fork")))
        .and(body_json(json!({})))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!([{"sandbox": sandbox_json("0.5.7")}])))
        .expect(1)
        .mount(&server)
        .await;

    let results = client_for(&server)
        .fork_sandbox(SANDBOX_ID, &ForkOptions::default())
        .await
        .expect("fork");

    assert_eq!(results.len(), 1);
}

#[tokio::test]
async fn fork_counts_outside_1_to_20_are_refused_before_any_request() {
    let server = MockServer::start().await;
    let client = client_for(&server);

    for count in [0, 21] {
        let error = client
            .fork_sandbox(SANDBOX_ID, &ForkOptions::new().count(count))
            .await
            .expect_err("invalid count");
        assert!(matches!(error, Error::InvalidArgument(_)), "{count}: {error:?}");
    }
    assert!(server.received_requests().await.expect("recording").is_empty());
}

#[tokio::test]
async fn fork_of_a_missing_sandbox_is_not_found() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/sandboxes/{SANDBOX_ID}/fork")))
        .respond_with(ResponseTemplate::new(404).set_body_json(json!({"code": 404, "message": "nope"})))
        .mount(&server)
        .await;

    let error = client_for(&server)
        .fork_sandbox(SANDBOX_ID, &ForkOptions::default())
        .await
        .expect_err("not found");

    assert!(matches!(error, Error::SandboxNotFound(_)), "{error:?}");
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --test rest_fork`
Expected: compile error, `ForkOptions` not found.

- [ ] **Step 3: Implement**

`src/models.rs`:

```rust
/// The most forks one request may ask for (`MAX_FORK_COUNT`,
/// `e2b/sandbox/sandbox_api.py:81`).
pub const MAX_FORK_COUNT: u32 = 20;

/// Options for [`crate::Client::fork_sandbox`].
///
/// # Examples
///
/// ```
/// use std::time::Duration;
/// use e2b_client::ForkOptions;
///
/// let options = ForkOptions::new().count(2).timeout(Duration::from_secs(300));
/// # let _ = options;
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ForkOptions {
    timeout: Option<Duration>,
    count: Option<u32>,
}

impl ForkOptions {
    /// One fork with the API's default timeout.
    ///
    /// # Returns
    ///
    /// Empty options.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The forks' lifetime, sent in whole seconds.
    ///
    /// # Arguments
    ///
    /// * `timeout` - the lifetime.
    ///
    /// # Returns
    ///
    /// The options, for chaining.
    #[must_use]
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// How many forks to start, 1 to [`MAX_FORK_COUNT`]. All boot from one
    /// snapshot of the source. Checked by [`crate::Client::fork_sandbox`].
    ///
    /// # Arguments
    ///
    /// * `count` - the number of forks.
    ///
    /// # Returns
    ///
    /// The options, for chaining.
    #[must_use]
    pub fn count(mut self, count: u32) -> Self {
        self.count = Some(count);
        self
    }

    /// The request body, after the SDK's count check
    /// (`validate_fork_count`, `e2b/sandbox/sandbox_api.py:103-118`).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidArgument`] for a count outside 1 to 20.
    pub(crate) fn body(&self) -> Result<ForkBody> {
        if let Some(count) = self.count
            && !(1..=MAX_FORK_COUNT).contains(&count)
        {
            return Err(Error::InvalidArgument(format!(
                "fork count must be between 1 and {MAX_FORK_COUNT} (got {count})"
            )));
        }
        Ok(ForkBody {
            timeout: self.timeout.map(|timeout| timeout.as_secs()),
            count: self.count,
        })
    }
}

/// `SandboxForkRequest`: `{timeout?, count?}`.
#[derive(Debug, Serialize)]
pub(crate) struct ForkBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    timeout: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    count: Option<u32>,
}

/// `SandboxForkResult`: a started fork or why it failed.
#[derive(Debug, Deserialize)]
pub(crate) struct ForkResultWire {
    #[serde(default)]
    pub(crate) sandbox: Option<SandboxCreated>,
    #[serde(default)]
    pub(crate) error: Option<ErrorBody>,
}

/// The E2B error object, `{code, message}` (`e2b/api/client/models/error.py`).
#[derive(Debug, Deserialize)]
pub(crate) struct ErrorBody {
    pub(crate) code: u16,
    pub(crate) message: String,
}
```

(`if let ... && ...` is a let chain, stable in Rust 2024.) `models.rs` needs `use crate::error::{Error, Result};` if Task 6 has not added it. `src/client.rs`:

```rust
    /// Forks a running sandbox (`POST /sandboxes/{id}/fork`). The source is
    /// briefly paused, snapshotted with its memory and resumed, keeping its
    /// ID and expiry; each fork boots from that snapshot.
    ///
    /// # Arguments
    ///
    /// * `sandbox_id` - the sandbox to fork.
    /// * `options` - the forks' timeout and count.
    ///
    /// # Returns
    ///
    /// One result per requested fork: a handle, or why that fork failed
    /// ([`Error::Api`] with the per-fork code). Forks succeed or fail
    /// independently.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidArgument`] for a count outside 1 to 20, before any
    /// request; [`Error::SandboxNotFound`] for HTTP 404; [`Error::Api`] for
    /// other non-2xx answers; [`Error::Decode`]; [`Error::Transport`].
    pub async fn fork_sandbox(
        &self,
        sandbox_id: &str,
        options: &ForkOptions,
    ) -> Result<Vec<Result<Sandbox>>> {
        let body = options.body()?;
        let response = self
            .send_json(Method::POST, &["sandboxes", sandbox_id, "fork"], &body)
            .await?;
        let results: Vec<ForkResultWire> = parse_sandbox_response(response, sandbox_id).await?;
        Ok(results
            .into_iter()
            .map(|result| self.fork_result(result))
            .collect())
    }

    /// One fork's outcome; an error wins over a sandbox, as in the SDK
    /// (`e2b/sandbox_async/sandbox_api.py:476-493`).
    fn fork_result(&self, result: ForkResultWire) -> Result<Sandbox> {
        match (result.sandbox, result.error) {
            (_, Some(error)) => Err(Error::Api {
                status: error.code,
                message: error.message,
            }),
            (Some(created), None) => Sandbox::from_created(self.clone(), created),
            (None, None) => Err(Error::Protocol(
                "a fork result has neither a sandbox nor an error".into(),
            )),
        }
    }
```

`src/sandbox.rs`:

```rust
    /// Forks this sandbox. See [`Client::fork_sandbox`].
    ///
    /// # Arguments
    ///
    /// * `options` - the forks' timeout and count.
    ///
    /// # Returns
    ///
    /// One result per fork.
    ///
    /// # Errors
    ///
    /// As [`Client::fork_sandbox`].
    pub async fn fork(&self, options: &ForkOptions) -> Result<Vec<Result<Sandbox>>> {
        self.client.fork_sandbox(&self.sandbox_id, options).await
    }
```

`src/lib.rs`: add `ForkOptions` and `MAX_FORK_COUNT` to the `models` re-export.

- [ ] **Step 4: Run the tests**

Run: `cargo test --test rest_fork`
Expected: PASS.

- [ ] **Step 5: Live test**

`tests/live_support/mod.rs`, in `impl LiveSandbox`:

```rust
    /// Takes ownership of a sandbox this test did not create with
    /// [`LiveSandbox::create`] (a fork), so it is killed on drop too.
    pub fn adopt(sandbox: Sandbox) -> Self {
        Self { client: client(), sandbox }
    }
```

```rust
#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0030_fork_starts_running_copies() {
    // Arrange
    let source = LiveSandbox::new().await;
    source
        .files()
        .write("/tmp/fork.txt", "from the source", None)
        .await
        .expect("write");
    let options = ForkOptions::new().count(2).timeout(Duration::from_secs(60));

    // Act: adopt every fork that started before asserting anything, so each
    // is killed on drop even if another failed.
    let results = source.fork(&options).await.expect("fork");
    let (started, failed): (Vec<_>, Vec<_>) = results.into_iter().partition(Result::is_ok);
    let forks: Vec<LiveSandbox> = started
        .into_iter()
        .map(|result| LiveSandbox::adopt(result.expect("partitioned as Ok")))
        .collect();

    // Assert
    assert!(failed.is_empty(), "forks that failed to start: {failed:?}");
    assert_eq!(forks.len(), 2);
    for fork in &forks {
        assert_ne!(fork.sandbox_id(), source.sandbox_id());
        let text = fork
            .files()
            .read_to_string("/tmp/fork.txt", None)
            .await
            .expect("read in the fork");
        assert_eq!(text, "from the source");
    }
    assert_eq!(source.get_info().await.expect("source info").state, SandboxState::Running);
}
```

Record in the PR whether a fork's `get_info().metadata` has `RUN_METADATA_KEY`: it decides whether the sandbox sweep can also catch a leaked fork. Run it on hosted. Expected: PASS.

- [ ] **Step 6: Docs, gates, changelog, commit**

Fork rows in "Snapshots and fork". Evidence map: CLM-0162 → the four `tests/rest_fork.rs` tests. `changelog/<ts>-m3-task-09-fork.md`.

```bash
git add src/models.rs src/client.rs src/sandbox.rs src/lib.rs tests/rest_fork.rs tests/live_support/mod.rs tests/live_lifecycle.rs docs/ changelog/
git commit -m "feat: fork a sandbox"
```

---

### Task 10: Volumes and volume mounts (CLM-0148, 0149, 0163, 0164; the mounts part of CLM-0153, 0167)

**Files:**
- Create: `src/volumes.rs` (`Volume`, `VolumeInfo`, `VolumeMount`)
- Modify: `src/error.rs` (`VolumeNotFound`, and `is_not_found`), `src/client.rs` (`create_volume`, `get_volume`, `list_volumes`, `destroy_volume`), `src/models.rs` (`CreateSandboxParams::volume_mount`, the body field, `SandboxInfo::volume_mounts`), `src/lib.rs`
- Create: `tests/rest_volumes.rs`
- Modify: `tests/live_sweep.rs` (destroy the run's volumes), `tests/live_lifecycle.rs`
- Modify: `docs/protocol-notes.md` (new section "Volumes"), `docs/verification/s4-evidence-map.md`

- [ ] **Step 1: Write the failing mock tests**

`tests/rest_volumes.rs`:

```rust
//! Volumes and volume mounts against a mock server.

mod common;

use common::{API_KEY, SANDBOX_ID, client_for, sandbox_json};
use e2b_client::{CreateSandboxParams, Error, ListSandboxesParams};
use secrecy::ExposeSecret;
use serde_json::json;
use wiremock::matchers::{body_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const VOLUME_TOKEN: &str = "volume-token-secret";

fn volume_json() -> serde_json::Value {
    json!({"volumeID": "vol-1", "name": "data", "token": VOLUME_TOKEN})
}

#[tokio::test]
async fn create_volume_posts_its_name_and_keeps_the_token_secret() {
    // Arrange
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/volumes"))
        .and(header("x-api-key", API_KEY))
        .and(body_json(json!({"name": "data"})))
        .respond_with(ResponseTemplate::new(201).set_body_json(volume_json()))
        .expect(1)
        .mount(&server)
        .await;

    // Act
    let volume = client_for(&server).create_volume("data").await.expect("create");

    // Assert
    assert_eq!(volume.volume_id(), "vol-1");
    assert_eq!(volume.name(), "data");
    assert_eq!(volume.token().expose_secret(), VOLUME_TOKEN);
    assert_eq!(volume.domain(), None);
    assert!(!format!("{volume:?}").contains(VOLUME_TOKEN));
}

#[tokio::test]
async fn get_volume_reads_the_domain_and_maps_404() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/volumes/vol-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "volumeID": "vol-1", "name": "data", "token": VOLUME_TOKEN, "domain": "byoc.example.test"
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/volumes/gone"))
        .respond_with(ResponseTemplate::new(404).set_body_json(json!({"code": 404, "message": "nope"})))
        .mount(&server)
        .await;
    let client = client_for(&server);

    let volume = client.get_volume("vol-1").await.expect("get");
    let error = client.get_volume("gone").await.expect_err("missing");

    assert_eq!(volume.domain(), Some("byoc.example.test"));
    assert!(matches!(error, Error::VolumeNotFound(ref id) if id == "gone"), "{error:?}");
    assert!(error.is_not_found());
}

#[tokio::test]
async fn list_volumes_reads_ids_and_names() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/volumes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {"volumeID": "vol-1", "name": "data"}, {"volumeID": "vol-2", "name": "cache"}
        ])))
        .mount(&server)
        .await;

    let volumes = client_for(&server).list_volumes().await.expect("list");

    let names: Vec<&str> = volumes.iter().map(|volume| volume.name.as_str()).collect();
    assert_eq!(names, ["data", "cache"]);
    assert_eq!(volumes[0].volume_id, "vol-1");
}

#[tokio::test]
async fn destroy_volume_reports_whether_it_existed() {
    let server = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path("/volumes/vol-1"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/volumes/gone"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;
    let client = client_for(&server);

    assert!(client.destroy_volume("vol-1").await.expect("destroy"));
    assert!(!client.destroy_volume("gone").await.expect("destroy missing"));
}

#[tokio::test]
async fn create_sends_volume_mounts_and_info_reads_them() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v2/sandboxes"))
        .and(body_json(json!({
            "templateID": "base", "metadata": {}, "envVars": {},
            "volumeMounts": [{"name": "data", "path": "/mnt/data"}]
        })))
        .respond_with(ResponseTemplate::new(201).set_body_json(sandbox_json("0.5.7")))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/sandboxes/{SANDBOX_ID}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "templateID": "base", "sandboxID": SANDBOX_ID, "clientID": "c",
            "startedAt": "2026-10-27T12:00:00Z", "endAt": "2026-10-27T12:05:00Z",
            "envdVersion": "0.5.7", "cpuCount": 2, "memoryMB": 512, "diskSizeMB": 2048,
            "state": "running", "volumeMounts": [{"name": "data", "path": "/mnt/data"}]
        })))
        .mount(&server)
        .await;
    let client = client_for(&server);

    let sandbox = client
        .create_sandbox(&CreateSandboxParams::default().volume_mount("data", "/mnt/data"))
        .await
        .expect("create");
    let info = sandbox.get_info().await.expect("info");

    assert_eq!(info.volume_mounts.len(), 1);
    assert_eq!(info.volume_mounts[0].name, "data");
    assert_eq!(info.volume_mounts[0].path, "/mnt/data");
}

#[tokio::test]
async fn list_entries_read_volume_mounts() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v2/sandboxes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{
            "templateID": "base", "sandboxID": SANDBOX_ID, "clientID": "c",
            "startedAt": "2026-10-27T12:00:00Z", "endAt": "2026-10-27T12:05:00Z",
            "envdVersion": "0.5.7", "cpuCount": 2, "memoryMB": 512, "diskSizeMB": 2048,
            "state": "running", "volumeMounts": [{"name": "data", "path": "/mnt/data"}]
        }])))
        .mount(&server)
        .await;

    let page = client_for(&server)
        .list_sandboxes(&ListSandboxesParams::new())
        .await
        .expect("list");

    let mounts = &page.sandboxes[0].volume_mounts;
    assert_eq!(mounts.len(), 1);
    assert_eq!((mounts[0].name.as_str(), mounts[0].path.as_str()), ("data", "/mnt/data"));
}
```

The test uses `secrecy` (a normal dependency) for `ExposeSecret`. The existing "omits unset optionals" unit test in `models.rs` already proves that no mounts means no `volumeMounts`.

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --test rest_volumes`
Expected: compile error, `create_volume` not found.

- [ ] **Step 3: Implement**

`src/volumes.rs`:

```rust
//! Volumes: persistent storage that outlives sandboxes and can be mounted
//! into them (`e2b/volume/volume_async.py`). This module covers managing
//! volumes; reading and writing a volume's files without a sandbox (the
//! volume content API) is not implemented.

use std::fmt;

use secrecy::SecretString;
use serde::{Deserialize, Serialize};

/// A volume, with the token for its content API
/// (`VolumeAndToken`, `e2b/volume/types.py:21-30`).
pub struct Volume {
    volume_id: String,
    name: String,
    token: SecretString,
    domain: Option<String>,
}

impl Volume {
    /// The volume ID.
    ///
    /// # Returns
    ///
    /// The ID.
    #[must_use]
    pub fn volume_id(&self) -> &str {
        &self.volume_id
    }

    /// The volume name, which [`crate::CreateSandboxParams::volume_mount`] takes.
    ///
    /// # Returns
    ///
    /// The name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The token for the volume content API.
    ///
    /// # Returns
    ///
    /// The token, wrapped so it is not printed by accident.
    #[must_use]
    pub fn token(&self) -> &SecretString {
        &self.token
    }

    /// The host for volume content requests, set only for teams on a custom
    /// (BYOC) cluster.
    ///
    /// # Returns
    ///
    /// The domain, if any.
    #[must_use]
    pub fn domain(&self) -> Option<&str> {
        self.domain.as_deref()
    }
}

// Not derived, so the token can never reach a log.
impl fmt::Debug for Volume {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Volume")
            .field("volume_id", &self.volume_id)
            .field("name", &self.name)
            .field("domain", &self.domain)
            .finish_non_exhaustive()
    }
}

/// `VolumeAndToken` as the API sends it.
#[derive(Deserialize)]
pub(crate) struct VolumeWire {
    #[serde(rename = "volumeID")]
    volume_id: String,
    name: String,
    token: String,
    #[serde(default)]
    domain: Option<String>,
}

impl From<VolumeWire> for Volume {
    fn from(wire: VolumeWire) -> Self {
        Self {
            volume_id: wire.volume_id,
            name: wire.name,
            token: SecretString::from(wire.token),
            // An empty domain means none, as in the SDK (`volume_async.py:160-164`).
            domain: wire.domain.filter(|domain| !domain.is_empty()),
        }
    }
}

/// `NewVolume`: `{name}`.
#[derive(Serialize)]
pub(crate) struct NewVolumeBody<'a> {
    pub(crate) name: &'a str,
}

/// One entry of [`crate::Client::list_volumes`] (`VolumeInfo`).
///
/// This is a plain data record, so its fields are public. It is
/// `#[non_exhaustive]` so fields can be added without a breaking change.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[non_exhaustive]
pub struct VolumeInfo {
    /// The volume ID.
    #[serde(rename = "volumeID")]
    pub volume_id: String,
    /// The volume name.
    pub name: String,
}

/// A volume mounted into a sandbox (`SandboxVolumeMount`).
///
/// This is a plain data record, so its fields are public. It is
/// `#[non_exhaustive]` so fields can be added without a breaking change.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct VolumeMount {
    /// The volume name.
    pub name: String,
    /// Where it is mounted inside the sandbox.
    pub path: String,
}

impl VolumeMount {
    /// A mount of volume `name` at `path`.
    ///
    /// # Arguments
    ///
    /// * `name` - the volume name.
    /// * `path` - the mount path inside the sandbox.
    ///
    /// # Returns
    ///
    /// The mount.
    #[must_use]
    pub fn new(name: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            path: path.into(),
        }
    }
}
```

`src/error.rs`, after `SandboxNotFound`:

```rust
    /// The REST API reported that the volume does not exist (HTTP 404).
    #[error("volume {0} not found")]
    VolumeNotFound(String),
```

and `is_not_found` matches `Self::SandboxNotFound(_) | Self::VolumeNotFound(_) => true`. `src/client.rs`:

```rust
    /// Creates a volume (`POST /volumes`).
    ///
    /// # Arguments
    ///
    /// * `name` - the volume name.
    ///
    /// # Returns
    ///
    /// The volume, with its content token.
    ///
    /// # Errors
    ///
    /// [`Error::Api`] for a non-2xx answer, [`Error::Decode`],
    /// [`Error::Transport`].
    pub async fn create_volume(&self, name: &str) -> Result<Volume> {
        let response = self
            .send_json(Method::POST, &["volumes"], &NewVolumeBody { name })
            .await?;
        let wire: VolumeWire = parse_success(response).await?;
        Ok(wire.into())
    }

    /// Looks up a volume (`GET /volumes/{id}`), the SDK's `Volume.get_info`
    /// and `Volume.connect`.
    ///
    /// # Arguments
    ///
    /// * `volume_id` - the volume ID.
    ///
    /// # Returns
    ///
    /// The volume, with its content token.
    ///
    /// # Errors
    ///
    /// [`Error::VolumeNotFound`] for HTTP 404, [`Error::Api`] for other
    /// non-2xx answers, [`Error::Decode`], [`Error::Transport`].
    pub async fn get_volume(&self, volume_id: &str) -> Result<Volume> {
        let response = self.request(Method::GET, &["volumes", volume_id])?.send().await?;
        if response.status() == StatusCode::NOT_FOUND {
            return Err(Error::VolumeNotFound(volume_id.to_owned()));
        }
        let wire: VolumeWire = parse_success(response).await?;
        Ok(wire.into())
    }

    /// Lists the team's volumes (`GET /volumes`).
    ///
    /// # Returns
    ///
    /// Every volume's ID and name.
    ///
    /// # Errors
    ///
    /// [`Error::Api`] for a non-2xx answer, [`Error::Decode`],
    /// [`Error::Transport`].
    pub async fn list_volumes(&self) -> Result<Vec<VolumeInfo>> {
        let response = self.request(Method::GET, &["volumes"])?.send().await?;
        let volumes: Option<Vec<VolumeInfo>> = parse_success(response).await?;
        Ok(volumes.unwrap_or_default())
    }

    /// Destroys a volume and its data (`DELETE /volumes/{id}`).
    ///
    /// # Arguments
    ///
    /// * `volume_id` - the volume ID.
    ///
    /// # Returns
    ///
    /// `true` if it was destroyed, `false` if it did not exist.
    ///
    /// # Errors
    ///
    /// [`Error::Api`] for a non-2xx answer other than 404,
    /// [`Error::Transport`].
    pub async fn destroy_volume(&self, volume_id: &str) -> Result<bool> {
        let response = self
            .request(Method::DELETE, &["volumes", volume_id])?
            .send()
            .await?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(false);
        }
        ensure_success(response).await.map(|_| true)
    }
```

`src/models.rs`: `volume_mounts: Vec<VolumeMount>` on `CreateSandboxParams` (initialised to `Vec::new()`), with

```rust
    /// Mounts volume `name` at `path` in the sandbox (call again for more).
    ///
    /// # Arguments
    ///
    /// * `name` - the volume name ([`crate::Volume::name`]).
    /// * `path` - where to mount it.
    ///
    /// # Returns
    ///
    /// The parameters, for chaining.
    #[must_use]
    pub fn volume_mount(mut self, name: impl Into<String>, path: impl Into<String>) -> Self {
        self.volume_mounts.push(VolumeMount::new(name, path));
        self
    }
```

`volume_mounts: &self.volume_mounts` in `body()`, and on `NewSandboxBody`:

```rust
    // `<[_]>::is_empty` is the slice method, so an empty list is left out,
    // as the SDK does (`sandbox_async/sandbox_api.py:252`).
    #[serde(rename = "volumeMounts", skip_serializing_if = "<[_]>::is_empty")]
    volume_mounts: &'a [VolumeMount],
```

`SandboxInfo`:

```rust
    /// The volumes mounted into the sandbox.
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub volume_mounts: Vec<VolumeMount>,
```

`src/lib.rs`: `mod volumes;` and `pub use volumes::{Volume, VolumeInfo, VolumeMount};`.

- [ ] **Step 4: Run the tests**

Run: `cargo test --test rest_volumes`, `cargo test --lib`, `cargo test --test rest_api`
Expected: PASS.

- [ ] **Step 5: The sweep and live tests**

`tests/live_sweep.rs`:

```rust
#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn sweep_leftover_volumes_of_this_run() {
    let client = client();
    let marker = resource_name("");
    let leftovers: Vec<String> = client
        .list_volumes()
        .await
        .expect("list volumes")
        .into_iter()
        .filter(|volume| volume.name.contains(&marker))
        .map(|volume| volume.volume_id)
        .collect();
    for volume_id in &leftovers {
        client.destroy_volume(volume_id).await.expect("destroy leftover volume");
    }
    eprintln!("sweep: destroyed {} leftover volume(s) of run {}", leftovers.len(), run_id());
}
```

`resource_name` is Task 8's; if Task 8 has not merged, add it to `tests/live_support/mod.rs` as Task 8 Step 5 shows. Then in `tests/live_lifecycle.rs`:

```rust
#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0148_volume_create_get_list_destroy() {
    // Arrange
    let client = client();
    let name = resource_name("managed");

    // Act
    let created = client.create_volume(&name).await.expect("create");
    let fetched = client.get_volume(created.volume_id()).await.expect("get");
    let listed = client.list_volumes().await.expect("list");
    let destroyed = client.destroy_volume(created.volume_id()).await.expect("destroy");
    let after = client.get_volume(created.volume_id()).await;
    let destroyed_again = client.destroy_volume(created.volume_id()).await.expect("destroy again");

    // Assert
    assert_eq!(created.name(), name);
    assert_eq!(fetched.volume_id(), created.volume_id());
    assert_eq!(fetched.name(), name);
    assert!(listed.iter().any(|volume| volume.volume_id == created.volume_id()), "{listed:?}");
    assert!(destroyed);
    assert!(matches!(after, Err(Error::VolumeNotFound(_))), "{after:?}");
    assert!(!destroyed_again);
}

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0149_volume_mount_persists_across_sandboxes() {
    // Arrange: write through the mount in one sandbox, then kill it.
    let client = client();
    let volume = client.create_volume(&resource_name("mounted")).await.expect("create volume");
    let mount = params().volume_mount(volume.name(), "/mnt/data");
    {
        let writer = LiveSandbox::create(&mount).await;
        writer
            .files()
            .write("/mnt/data/kept.txt", "on the volume", None)
            .await
            .expect("write through the mount");
        // Dropping `writer` at the end of this block kills it.
    }

    // Act
    let reader = LiveSandbox::create(&mount).await;
    let text = reader
        .files()
        .read_to_string("/mnt/data/kept.txt", None)
        .await
        .expect("read through the mount");

    // Assert
    assert_eq!(text, "on the volume");
    drop(reader);
    assert!(client.destroy_volume(volume.volume_id()).await.expect("destroy volume"));
}

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0153_get_info_reports_volume_mounts() {
    // Arrange
    let client = client();
    let volume = client.create_volume(&resource_name("info")).await.expect("create volume");
    let sandbox = LiveSandbox::create(&params().volume_mount(volume.name(), "/mnt/info")).await;

    // Act
    let info = sandbox.get_info().await.expect("info");

    // Assert
    let mounted = info
        .volume_mounts
        .iter()
        .any(|mount| mount.name == volume.name() && mount.path == "/mnt/info");
    assert!(mounted, "{:?}", info.volume_mounts);
    drop(sandbox);
    assert!(client.destroy_volume(volume.volume_id()).await.expect("destroy volume"));
}
```

A volume may refuse to be destroyed while a sandbox still mounts it, hence the explicit `drop` first (the drop guard kills synchronously). If hosted E2B answers the volume calls with 403 or 404 for the team, stop and report it: the claims stay `not_enough_info`, and the task finishes with the mock tests only. Run the three tests and the volume sweep on hosted. Expected: PASS, and the sweep destroys 0 volumes.

- [ ] **Step 6: Docs, gates, changelog, commit**

`docs/protocol-notes.md`: a new section "Volumes" with the four routes and the mount body; the "Create body" and "Detail fields" rows gain `volumeMounts`. Evidence map: CLM-0163 → the first four `tests/rest_volumes.rs` tests; CLM-0164 → `create_sends_volume_mounts_and_info_reads_them` (create body and detail), `list_entries_read_volume_mounts` (list entries) and the omit unit test; CLM-0167 (mounts part) → the same mock test. `changelog/<ts>-m3-task-10-volumes.md`.

```bash
git add src/volumes.rs src/error.rs src/client.rs src/models.rs src/lib.rs tests/rest_volumes.rs tests/live_sweep.rs tests/live_lifecycle.rs docs/ changelog/
git commit -m "feat: volumes and volume mounts"
```

---

### Task 11: The MCP gateway (CLM-0150, 0165)

**Files:**
- Create: `src/mcp.rs` (constants, shell quoting, the gateway command)
- Modify: `Cargo.toml`, `Cargo.lock` (uuid)
- Modify: `src/models.rs` (`CreateSandboxParams::mcp`, an optional template, `#[derive(Default)]`), `src/client.rs` (start the gateway after create), `src/sandbox.rs` (the token cell, `start_mcp_gateway`, `mcp_url`, `mcp_token`), `src/error.rs` (`McpGateway`), `src/lib.rs`
- Create: `tests/rest_mcp.rs`
- Modify: `tests/live_lifecycle.rs`, `docs/protocol-notes.md` (new section "MCP gateway"), `docs/verification/s4-evidence-map.md`

- [ ] **Step 1: Write the failing tests**

Unit tests in `src/mcp.rs`'s `mod tests` (written with the module in Step 3):

```rust
    #[test]
    fn shell_quote_matches_shlex_quote() {
        assert_eq!(shell_quote(""), "''");
        assert_eq!(shell_quote("abc-1.2/x:y,z@%+="), "abc-1.2/x:y,z@%+=");
        assert_eq!(shell_quote("a b"), "'a b'");
        assert_eq!(shell_quote("it's"), r#"'it'"'"'s'"#);
    }

    #[test]
    fn gateway_command_quotes_the_json_config() {
        let config = serde_json::json!({"fetch": {}});
        let serde_json::Value::Object(config) = config else {
            panic!("an object");
        };
        assert_eq!(
            gateway_command(&config).expect("command"),
            r#"mcp-gateway --config '{"fetch":{}}'"#
        );
    }
```

`tests/rest_mcp.rs`:

```rust
//! Creating a sandbox with an MCP configuration, against a mock server that
//! plays both the API and envd.

mod common;

use common::{SANDBOX_ID, client_for, decode_request_envelope, end_stream, message, sandbox_json, stream_response};
use e2b_client::{CreateSandboxParams, Error};
use secrecy::ExposeSecret;
use serde_json::{Map, Value, json};
use wiremock::matchers::{body_json, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn fetch_config() -> Map<String, Value> {
    let Value::Object(config) = json!({"fetch": {}}) else {
        panic!("an object");
    };
    config
}

/// The frames of a process that writes `stderr` and exits with `exit_code`.
fn process_frames(exit_code: i32, stderr_b64: &str) -> Vec<Vec<u8>> {
    vec![
        message(json!({"event": {"start": {"pid": 7}}})),
        message(json!({"event": {"data": {"stderr": stderr_b64}}})),
        message(json!({"event": {"end": {"exitCode": exit_code, "exited": true}}})),
        end_stream(json!({})),
    ]
}

#[tokio::test]
async fn create_with_mcp_uses_the_gateway_template_and_starts_it_as_root() {
    // Arrange
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v2/sandboxes"))
        .and(body_json(json!({
            "templateID": "mcp-gateway", "metadata": {}, "envVars": {}, "mcp": {"fetch": {}}
        })))
        .respond_with(ResponseTemplate::new(201).set_body_json(sandbox_json("0.5.7")))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/process.Process/Start"))
        .respond_with(stream_response(&process_frames(0, "")))
        .expect(1)
        .mount(&server)
        .await;

    // Act
    let sandbox = client_for(&server)
        .create_sandbox(&CreateSandboxParams::default().mcp(fetch_config()))
        .await
        .expect("create");

    // Assert: the command, its user and the token it was given.
    let requests = server.received_requests().await.expect("recording");
    let start = requests
        .iter()
        .find(|request| request.url.path() == "/process.Process/Start")
        .expect("start");
    // base64("root:")
    assert_eq!(start.headers.get("authorization").expect("user").to_str().expect("ascii"), "Basic cm9vdDo=");
    let process = &decode_request_envelope(&start.body)["process"];
    assert_eq!(process["args"], json!(["-l", "-c", r#"mcp-gateway --config '{"fetch":{}}'"#]));
    let sent_token = process["envs"]["GATEWAY_ACCESS_TOKEN"].as_str().expect("token env").to_owned();
    assert_eq!(sent_token.len(), 36, "a hyphenated UUID: {sent_token}");
    let token = sandbox.mcp_token().await.expect("cached token");
    assert_eq!(token.expose_secret(), sent_token);
    assert_eq!(sandbox.mcp_url(), format!("https://50005-{SANDBOX_ID}.example.test/mcp"));
}

#[tokio::test]
async fn an_explicit_template_wins_over_the_gateway_default() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v2/sandboxes"))
        .and(body_json(json!({
            "templateID": "my-mcp", "metadata": {}, "envVars": {}, "mcp": {"fetch": {}}
        })))
        .respond_with(ResponseTemplate::new(201).set_body_json(sandbox_json("0.5.7")))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/process.Process/Start"))
        .respond_with(stream_response(&process_frames(0, "")))
        .mount(&server)
        .await;

    client_for(&server)
        .create_sandbox(&CreateSandboxParams::new("my-mcp").mcp(fetch_config()))
        .await
        .expect("create");
}

#[tokio::test]
async fn a_failed_gateway_kills_the_sandbox_and_reports_stderr() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v2/sandboxes"))
        .respond_with(ResponseTemplate::new(201).set_body_json(sandbox_json("0.5.7")))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/process.Process/Start"))
        // base64("bad config\n")
        .respond_with(stream_response(&process_frames(1, "YmFkIGNvbmZpZwo=")))
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path(format!("/sandboxes/{SANDBOX_ID}")))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;

    let error = client_for(&server)
        .create_sandbox(&CreateSandboxParams::default().mcp(fetch_config()))
        .await
        .expect_err("gateway failure");

    assert!(matches!(error, Error::McpGateway(ref stderr) if stderr.contains("bad config")), "{error:?}");
}

#[tokio::test]
async fn a_connected_handle_reads_the_token_file_as_root() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/v2/sandboxes/{SANDBOX_ID}/connect")))
        .respond_with(ResponseTemplate::new(200).set_body_json(sandbox_json("0.5.7")))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/files"))
        .and(query_param("path", "/etc/mcp-gateway/.token"))
        .and(query_param("username", "root"))
        // The trailing newline is trimmed (see the plan's deviations).
        .respond_with(ResponseTemplate::new(200).set_body_string("tok-from-file\n"))
        .expect(1)
        .mount(&server)
        .await;
    let sandbox = client_for(&server).connect_sandbox(SANDBOX_ID, None).await.expect("connect");

    // Two calls, one read: the token is cached.
    let first = sandbox.mcp_token().await.expect("token").expose_secret().to_owned();
    let second = sandbox.mcp_token().await.expect("token again").expose_secret().to_owned();

    assert_eq!(first, "tok-from-file");
    assert_eq!(second, first);
}
```

The `/files` query keys `path` and `username` are the ones envd's read already uses (`tests/envd_api.rs:702-703`).

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --test rest_mcp`
Expected: compile error, no method `mcp`.

- [ ] **Step 3: Implement**

`Cargo.toml`, in `[dependencies]` (alphabetical):

```toml
# Random v4 UUIDs for the MCP gateway's access token, as the SDKs make it.
uuid = { version = "1.29.0", features = ["v4"] }
```

`src/mcp.rs`:

```rust
//! The MCP gateway that SDK 2.61 starts in a sandbox created with an MCP
//! configuration (`e2b/sandbox_async/main.py:244-268`).

use std::borrow::Cow;

use serde_json::{Map, Value};

use crate::error::Result;

/// The port the gateway listens on (`e2b/sandbox/main.py:28`).
pub const MCP_PORT: u16 = 50005;
/// The template used when an MCP configuration is given without a template
/// (`e2b/sandbox/main.py:33`).
pub const DEFAULT_MCP_TEMPLATE: &str = "mcp-gateway";
/// Where the gateway keeps its token (`e2b/sandbox_async/main.py:1063-1073`).
pub(crate) const MCP_TOKEN_PATH: &str = "/etc/mcp-gateway/.token";
/// The variable the gateway reads its token from.
pub(crate) const MCP_TOKEN_ENV: &str = "GATEWAY_ACCESS_TOKEN";

/// `mcp-gateway --config '<json>'`, as both SDKs build it.
///
/// # Errors
///
/// [`crate::Error::Decode`] if the configuration cannot be serialized (it
/// can't: every `Value` is valid JSON).
pub(crate) fn gateway_command(config: &Map<String, Value>) -> Result<String> {
    let json = serde_json::to_string(config)?;
    Ok(format!("mcp-gateway --config {}", shell_quote(&json)))
}

/// POSIX shell quoting, as Python's `shlex.quote` and the JS SDK's
/// `shellQuote` (`dist/index.js:268, 277-281`) do: safe strings stay as they
/// are, anything else is single-quoted with each `'` written as `'"'"'`.
///
/// Rust note: `Cow` (clone on write) returns the input unchanged without
/// copying it, and allocates only when quoting is needed.
fn shell_quote(raw: &str) -> Cow<'_, str> {
    if raw.is_empty() {
        return Cow::Borrowed("''");
    }
    let safe = raw
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || b"_@%+=:,./-".contains(&byte));
    if safe {
        Cow::Borrowed(raw)
    } else {
        Cow::Owned(format!("'{}'", raw.replace('\'', r#"'"'"'"#)))
    }
}
```

plus the Step 1 unit tests. `src/error.rs`:

```rust
    /// The MCP gateway did not start in a sandbox created with an MCP
    /// configuration; the sandbox was killed. Holds the gateway's stderr.
    #[error("failed to start the MCP gateway: {0}")]
    McpGateway(String),
```

`src/models.rs`: `CreateSandboxParams` now derives `Default`, its `template` becomes `Option<String>`, and it gains `mcp: Option<Map<String, Value>>`. Remove the hand-written `impl Default`, and write `new` as

```rust
    pub fn new(template: impl Into<String>) -> Self {
        Self {
            template: Some(template.into()),
            ..Self::default()
        }
    }
```

(`..Self::default()` fills every other field with its default, like `dataclasses.replace`.) Update `new`'s `# Returns` to "Parameters for `template` with nothing else set", and `Default`'s rustdoc to say it uses [`DEFAULT_TEMPLATE`], or [`DEFAULT_MCP_TEMPLATE`] once [`CreateSandboxParams::mcp`] is set. Then replace the existing `template()` accessor (`src/models.rs:75`, which returns `&self.template`) with the version below, which resolves the optional template, and add `mcp` and `mcp_config` after it:

```rust
    /// The template ID or alias the sandbox is created from: the one given
    /// to [`CreateSandboxParams::new`], else [`DEFAULT_MCP_TEMPLATE`] when an
    /// MCP configuration is set, else [`DEFAULT_TEMPLATE`]
    /// (`e2b/sandbox_async/main.py:224-227`).
    ///
    /// # Returns
    ///
    /// The template.
    #[must_use]
    pub fn template(&self) -> &str {
        match (&self.template, &self.mcp) {
            (Some(template), _) => template,
            (None, Some(_)) => DEFAULT_MCP_TEMPLATE,
            (None, None) => DEFAULT_TEMPLATE,
        }
    }

    /// Starts an MCP gateway in the sandbox with this configuration: a JSON
    /// object keyed by server name, e.g. `{"fetch": {}}`, as the SDK's
    /// `mcp=` takes it. [`crate::Client::create_sandbox`] then runs the
    /// gateway as root and kills the sandbox if it fails to start; reach it
    /// at [`crate::Sandbox::mcp_url`] with [`crate::Sandbox::mcp_token`].
    ///
    /// # Arguments
    ///
    /// * `config` - the MCP server configuration.
    ///
    /// # Returns
    ///
    /// The parameters, for chaining.
    #[must_use]
    pub fn mcp(mut self, config: Map<String, Value>) -> Self {
        self.mcp = Some(config);
        self
    }

    pub(crate) fn mcp_config(&self) -> Option<&Map<String, Value>> {
        self.mcp.as_ref()
    }
```

In `body()`, `template_id: self.template()` and `mcp: self.mcp.as_ref()`; on `NewSandboxBody`:

```rust
    #[serde(skip_serializing_if = "Option::is_none")]
    mcp: Option<&'a Map<String, Value>>,
```

`src/sandbox.rs`: a field `mcp_token: OnceCell<SecretString>` (`tokio::sync::OnceCell`, initialised with `OnceCell::new()` in `from_created`), and:

```rust
    /// The URL of the sandbox's MCP gateway, `https://<get_host(50005)>/mcp`
    /// (`e2b/sandbox/main.py:226-232`).
    ///
    /// # Returns
    ///
    /// The URL.
    #[must_use]
    pub fn mcp_url(&self) -> String {
        format!("https://{}/mcp", self.get_host(MCP_PORT))
    }

    /// The MCP gateway's access token: the one [`Client::create_sandbox`]
    /// generated, or, on a handle from [`Client::connect_sandbox`], read
    /// once from `/etc/mcp-gateway/.token` as root (without trailing
    /// whitespace) and then cached.
    ///
    /// # Returns
    ///
    /// The token, wrapped so it is not printed by accident.
    ///
    /// # Errors
    ///
    /// The errors of [`Filesystem::read_to_string`], for example
    /// [`Error::Envd`] with status 404 when the sandbox has no gateway.
    pub async fn mcp_token(&self) -> Result<&SecretString> {
        self.mcp_token
            .get_or_try_init(|| async {
                let token = self
                    .files()
                    .read_to_string(MCP_TOKEN_PATH, Some("root"))
                    .await?;
                // Trimmed, unlike the SDK: a trailing newline would make the
                // token unusable in a header (see the plan's deviations).
                Ok(SecretString::from(token.trim_end().to_owned()))
            })
            .await
    }

    /// Starts the MCP gateway as the SDKs do, and caches its token.
    ///
    /// # Errors
    ///
    /// [`Error::McpGateway`] with the gateway's stderr if it exits non-zero;
    /// the errors of [`Commands::run`].
    pub(crate) async fn start_mcp_gateway(&self, config: &Map<String, Value>) -> Result<()> {
        let token = uuid::Uuid::new_v4().to_string();
        let options = CommandOptions::default()
            .user("root")
            .env(MCP_TOKEN_ENV, token.clone());
        let result = self.commands().run(&gateway_command(config)?, &options).await?;
        if !result.success() {
            return Err(Error::McpGateway(result.stderr));
        }
        // `set` fails only when the cell is already filled, which a handle
        // fresh from `create_sandbox` never is.
        let _ = self.mcp_token.set(SecretString::from(token));
        Ok(())
    }
```

In `src/client.rs`, `create_sandbox` ends with:

```rust
        let sandbox = Sandbox::from_created(self.clone(), created)?;
        if let Some(config) = params.mcp_config()
            && let Err(error) = sandbox.start_mcp_gateway(config).await
        {
            // As in the SDKs, a sandbox whose gateway failed is killed. The
            // kill's own result is ignored so the gateway error is reported.
            let _ = sandbox.kill().await;
            return Err(error);
        }
        Ok(sandbox)
```

and its `# Errors` gain [`Error::McpGateway`]. `src/lib.rs`: `mod mcp;` and `pub use mcp::{DEFAULT_MCP_TEMPLATE, MCP_PORT};`.

- [ ] **Step 4: Run the tests**

Run: `cargo test --test rest_mcp`, `cargo test --lib`, `cargo test --test rest_api` and `cargo test --doc`
Expected: PASS. Diff `Cargo.lock`: only `uuid` (and its `getrandom` if a new version is needed) should be added.

- [ ] **Step 5: Live test**

```rust
#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0150_mcp_gateway_starts_and_exposes_url_and_token() {
    // Arrange: the `fetch` server needs no credentials. `params()` would set
    // the run's template, which wins over the gateway default, so the
    // gateway template is named explicitly.
    let serde_json::Value::Object(config) = serde_json::json!({"fetch": {}}) else {
        panic!("an object");
    };
    let create = params_from(DEFAULT_MCP_TEMPLATE).mcp(config);

    // Act
    let sandbox = LiveSandbox::create(&create).await;
    let token = sandbox.mcp_token().await.expect("token").expose_secret().to_owned();
    let connected = sandbox
        .client()
        .connect_sandbox(sandbox.sandbox_id(), None)
        .await
        .expect("connect");
    let from_file = connected.mcp_token().await.expect("token file").expose_secret().to_owned();
    let response = reqwest::Client::new()
        .get(sandbox.mcp_url())
        .send()
        .await
        .map_err(|error| error.without_url())
        .expect("the gateway answers HTTP");

    // Assert
    assert_eq!(from_file, token, "the token file holds the token create generated");
    assert!(sandbox.mcp_url().starts_with(&format!("https://50005-{}.", sandbox.sandbox_id())));
    assert!(response.status().as_u16() < 500, "gateway status {}", response.status());
}
```

`params_from` is Task 8's; add it as Task 8 Step 5 shows if Task 8 has not merged. Record in the PR the status the gateway answers with and without `Authorization: Bearer <token>`, so the protocol notes can state how the token is used (the SDKs don't say). Run it on hosted. Expected: PASS. If the `mcp-gateway` template isn't available to the team, stop and report it: the claim stays `not_enough_info`.

- [ ] **Step 6: Docs, gates, changelog, commit**

`docs/protocol-notes.md`: a new section "MCP gateway" (port, default template, the command and its quoting, user and env, kill on failure, the URL, the token file, and what Step 5 found about the token's use); the "Create body" row gains `mcp`. Evidence map: CLM-0165 → the four `tests/rest_mcp.rs` tests and the two `mcp.rs` unit tests. `changelog/<ts>-m3-task-11-mcp-gateway.md`.

```bash
git add Cargo.toml Cargo.lock src/mcp.rs src/models.rs src/client.rs src/sandbox.rs src/error.rs src/lib.rs tests/rest_mcp.rs tests/live_lifecycle.rs docs/ changelog/
git commit -m "feat: MCP gateway at sandbox create, mcp_url and mcp_token"
```

---

### Task 12: Live run on hosted E2B

**Files:**
- Create: `docs/verification/runs/<run id>.log` (copied from `target/live-logs/`)
- Create or modify: `docs/verification/m3-live-results.md` (section "Hosted E2B")

- [ ] **Step 1: Run the full suite on the merged main**

Run: `just test-live hosted`
Expected: the log starts with `commit:` and `dirty: no`, then one result line per `clm_*` test. That is 57 from M1 and M2, plus M3's 17 (74 in all): `clm_0028`, `0029`, `0030` (snapshot and fork, 2), `0143`, `0144`, `0145`, `0146` (2), `0147`, `0148`, `0149`, `0150`, `0151` and `0153` (3: network, lifecycle, mounts). Then come the three sweep lines (sandboxes, snapshots, volumes). The earlier milestones' tests rerun as a regression check.

- [ ] **Step 2: Check for leaks, secrets and the stamp**

All three sweeps must report 0 leftovers. Check the log for both API keys exactly as in M1 Task 6 Step 3, and also for the strings `GATEWAY_ACCESS_TOKEN=` and `"token"`. Every count must be `0`: a volume or MCP token in a log is a leak. A log with `dirty: yes`, `dirty: unknown` or `commit: unknown` doesn't count; commit or clean the tree, then rerun.

- [ ] **Step 3: Summarize**

Copy the log to `docs/verification/runs/`. Start `docs/verification/m3-live-results.md` with a one-line intro, then:

```markdown
## Hosted E2B (run <run id>, <UTC time>, commit <sha>)

| Test | Claim | Result | Failure (first line) |
|---|---|---|---|
```

Every `clm_*` test gets a row. Classify each failure as a crate bug, a test bug or an environment limit. Fix test bugs in this PR, with a plan correction note. Only report crate bugs here; the coordinator files them as M3 bug tasks (Task 14). These are environment limits, not bugs: a 400 or 409 on the filesystem-only pause (the team or node lacks it); a 403 or 404 on volumes; and a missing `mcp-gateway` template. Record each with the response body.

- [ ] **Step 4: Changelog and commit**

```bash
git add docs/verification/ changelog/
git commit -m "docs: M3 live results on hosted E2B"
```

---

### Task 13: Live run on sandbawks

As Task 12, with `just test-live sandbawks` (`.env` must have `E2B_TEMPLATE_SANDBAWKS=compound-sandbox`) and a "sandbawks" section in `docs/verification/m3-live-results.md`.

- **Pacing:** rerun failures one at a time, 120 s apart, so the two-sandbox warm pool can refill (M2 Task 10).
- **Expected misses:** sandbawks 0.1.28 serves none of network update, metrics, snapshots, fork or volumes (gfd/sandbawks #664 to #672).
  - A test that fails because its route is missing is recorded against the existing sub-issue, not filed again.
  - Every other failure that passes on hosted is a new sandbawks defect. Write its request and response in the results, and list it for the coordinator to file on gfd/sandbawks under epic #673.
- **Capacity:** failures from unschedulable Pods are recorded as capacity, not as defects.
- **The sweeps** must still report 0 leftovers. On a server without the snapshot or volume routes, a sweep that fails with 404 is noted, not counted as a leak.

Changelog `changelog/<ts>-m3-task-13-sandbawks-run.md`; commit `docs: M3 live results on sandbawks`.

---

### Task 14: Verdicts, docs and milestone close (coordinator)

Done by the coordinator session, which alone writes the ledger. The ledger work is on a branch in that session's own checkout (an M2 lesson: the claims server writes `claims.json` in its start directory).

- [ ] **Step 1: Evidence and verdicts.**
  - **Live evidence:** for each M3 live test, `add_evidence` (kind `demonstration`, locator `<test> @ hosted, docs/verification/runs/<log>`, stance from the result), and the same from the sandbawks run. A sandbawks failure names its gfd/sandbawks issue, has stance `neutral`, and does not refute the claim.
  - **Test evidence:** for each S4 claim (CLM-0154 to CLM-0167), and for CLM-0152 (S1, under CLM-0032, verified by test), `test` evidence from its evidence-map row at the merge commit.
  - **Before writing any summary,** read the test body it describes; evidence can't be edited (an M2 lesson).
  - **Verdicts** for CLM-0028 to CLM-0030 and CLM-0143 to CLM-0167. An environment limit (Task 12) leaves its claim `not_enough_info`, with the limit as the rationale.
  - **The parents:**
    - CLM-0031, CLM-0032 and CLM-0002 stay `not_enough_info`, because the egress proxy and IAM are deferred (see Scope). Their rationale names the supported children.
    - CLM-0005 is re-affirmed over its wire claims, which now include CLM-0154 to CLM-0167. CLM-0152 is not one of them.
  - **Retag:** CLM-0028 to CLM-0032 lose `gap` and carry `m3`; keep the history.
- [ ] **Step 2: Bugs.** Each refuted claim gets a bug task (Task 15, 16, ...) added under this step with a dated note, a sub-issue, and a fix and re-verification within M3, as in M1.
- [ ] **Step 3: Docs.**
  - **`README.md`:** the coverage table and the feature list (`get_host`, metrics, network, lifecycle, snapshots, fork, volumes, MCP), with one example that creates a sandbox with network settings and a lifecycle, snapshots it, and starts a second sandbox from the snapshot. README examples are doctests, so the example must compile.
  - **`docs/protocol-notes.md`:**
    - "Not implemented yet" lists the M4 items, the deferred M3 items (egress proxy, IAM, the volume content API), the unscheduled ones, and the design's out-of-scope items.
    - "Verification against a live server" gains the M3 runs.
  - **`src/lib.rs`:** the crate docs' bullets cover the new surface.
- [ ] **Step 4: Milestone changelog.** `changelog/<ts>-m3-milestone.md`: what shipped, the ledger counts before and after, the run ids, the sandbawks defects filed, the index of every plan correction by task (as in M2's), and the follow-ups.
