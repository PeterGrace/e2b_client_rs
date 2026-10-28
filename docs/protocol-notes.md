# Protocol notes

Every wire fact this crate relies on, with the place in the official SDKs it
comes from. The SDK sources were read, not executed.

## Sources

| Short name | Package | How obtained |
|---|---|---|
| `py:` | `e2b` 2.61.0 Python SDK (`e2b-2.61.0-py3-none-any.whl`), paths relative to `e2b/` | `pip download e2b --no-deps` |
| `js:` | `e2b` 2.61.0 JS SDK, `package/dist/index.js` unless another file is named | `npm pack e2b` |
| `connect-es:` | `@connectrpc/connect` 2.2.0, `package/dist/cjs/` (the Connect implementation the JS SDK uses, `js: package.json` dependencies) | `npm pack @connectrpc/connect@2` |

Both SDK packages are MIT-licensed. No `.proto` files are vendored: the envd
messages are hand-written serde structs (`src/envd/process_pb.rs`,
`src/envd/filesystem_pb.rs`) that follow the proto definitions the Python SDK
embeds in its generated code, so there is no Apache-2.0 material to carry a
NOTICE for.

## Configuration and authentication

| Fact | Citation |
|---|---|
| `E2B_DOMAIN`, default `e2b.app` | `py: connection_config.py:221-223`; `js: 1107-1108` |
| `E2B_API_KEY` | `py: connection_config.py:229-231`; `js: 1119-1120` |
| `E2B_API_URL`, else `https://api.<domain>` | `py: connection_config.py:233-235, 310-314`; `js: 1110-1111` |
| `E2B_SANDBOX_URL` override for envd traffic | `py: connection_config.py:237-239, 341-342`; `js: 1113-1114, 1127` |
| Explicit value, then env var, then default (`value or env or default`, so empty counts as unset) | `py: connection_config.py:289-291, 310-318` |
| Default request timeout 60 s | `py: connection_config.py:32` |
| A zero request timeout disables it, as `request_timeout=0` | `py: api/__init__.py:280` |
| API key header `X-API-KEY` | `py: api/__init__.py:250-251`; `js: 1528` |
| envd port 49983 | `py: connection_config.py:201`; `js: 1143, 5784` |

## envd addressing

| Fact | Citation |
|---|---|
| RPC base URL: override, else `https://sandbox.<domain>` for hosted E2B domains, else `https://<port>-<sandbox_id>.<domain>` | `py: connection_config.py:336-351`; `js: 1125-1132` |
| Hosted domain list | `py: sandbox_domains.py:1-13` |
| Direct URL (file API): override, else always the per-sandbox host | `py: connection_config.py:353-364`; `js: 1133-1137` |
| Host format `<port>-<sandbox_id>.<domain>` | `py: connection_config.py:366-380`; `js: 1138-1141` |
| Sandbox domain: the `domain` the API returned, else the configured one | `py: sandbox/main.py:46`; `js: 5794` |
| `get_host(port)`: `<port>-<sandbox_id>.<sandbox domain or configured domain>`; not changed by `E2B_SANDBOX_URL` | `py: connection_config.py:366-379`; `py: sandbox/main.py:46, 213-224`; `js: 1138-1141` |

## envd headers

| Fact | Citation |
|---|---|
| `E2b-Sandbox-Id: <id>` and `E2b-Sandbox-Port: 49983` on envd requests | `py: sandbox_async/main.py:1234-1235`; `js: 5805-5808, 5816` |
| `X-Access-Token: <envdAccessToken>` when the API returned one | `py: sandbox_async/main.py:1229-1232`; `js: 5817` (RPC), `js: 1865` (file API) |
| Those headers ride on every RPC via a default-headers interceptor | `py: envd/interceptors.py:42-62, 263-272`; `py: connection_config.py:454-462` |
| The API key is not part of the envd headers (only `User-Agent` plus the extras) | `py: connection_config.py:454-462` |
| Acting user: `Authorization: Basic base64("<user>:")`; none when no user, except envd < 0.4.0 defaults to `user` | `py: envd/utils.py:44-57`; `js: 1737`; `py: envd/versions.py:6` |
| File API selects the user with the `username` query parameter, same defaulting | `py: sandbox_async/filesystem/filesystem.py:206-212`; `js: 2115-2116, 2127-2130` |
| `Keepalive-Ping-Interval: 50` on process streams | `py: connection_config.py:48-49`; `py: sandbox_async/commands/command.py:331-334, 371-373` |
| Unparseable/debug envd version falls back to 99.99.99 | `py: envd/versions.py:4`; `py: sandbox_async/main.py:1203` |

## REST endpoints

All JSON. Request/response models are the generated OpenAPI models under
`py: api/client/models/`.

| Call | Route and status handling | Citation |
|---|---|---|
| Create | `POST /v2/sandboxes`, 201 → `Sandbox` | `py: api/client/api/sandboxes/post_v2_sandboxes.py:24-25, 39-42`; `py: sandbox_async/sandbox_api.py:256-277`; `js: 5566` |
| Create body | `templateID`, `timeout`, `autoPause`, `autoPauseMemory`, `autoResume`, `metadata`, `envVars`, `allow_internet_access` (snake_case in the spec), `network` (see "Network"), `volumeMounts` (see "Volumes"), `mcp` (see "MCP gateway"); header `Idempotency-Key` when a key is given | `py: api/client/models/new_sandbox_v2.py:97-132, 145-171`; `py: sandbox_async/sandbox_api.py:237-260` |
| Create lifecycle | `autoPause` only when `on_timeout` is set (`true` for pause, `false` for kill); `autoPauseMemory` only for a pause with a mode (`full` → `true`, `filesystem` → `false`), else omitted and the API default (full) applies; `autoResume: {enabled}` only when `auto_resume` is set | `py: sandbox/sandbox_api.py:639-662, 1054-1125`; `py: api/client/models/new_sandbox_v2.py:113-118`; `py: api/client/models/sandbox_auto_resume_config.py:18, 28` |
| Lifecycle checks | before any request, `InvalidArgument`: `auto_resume: true` without `on_timeout` pause (unset counts as kill), and `auto_resume: true` with mode `filesystem`. The SDK's third check, a mode on a kill, cannot be written here: `OnTimeout::Kill` has no mode | `py: sandbox/sandbox_api.py:1095-1114` |
| Idempotency key | `Idempotency-Key` header on `POST /v2/sandboxes`, only when given; 1-255 characters, then only `[a-z0-9-]`, else `InvalidArgument` before any request; checked before the lifecycle | `py: api/client/api/sandboxes/post_v2_sandboxes.py:17-21`; `py: retry.py:29-44`; `py: sandbox_async/sandbox_api.py:228-232`; `js: 5568` |
| Created sandbox | `templateID`, `sandboxID`, `clientID`, `envdVersion`, `envdAccessToken?`, `trafficAccessToken?`, `domain?` | `py: api/client/models/sandbox.py:85-115` |
| Connect / resume | `POST /v2/sandboxes/{id}/connect` body `{timeout?, memory?}`, 200 or 201 → `Sandbox`; `memory: false` only for `on_resume` reboot (cold boot from disk, any memory snapshot ignored), and left out for restore, the API's default; 404 → not found, with no other route tried | `py: api/client/api/sandboxes/post_v_2_sandboxes_sandbox_id_connect.py:22-23, 37-44`; `py: api/client/models/connect_sandbox_v2.py:13-42`; `py: sandbox/sandbox_api.py:761-791`; `py: sandbox_async/sandbox_api.py:498-520`; `js: 5609-5621` |
| Default sandbox timeout 300 s | | `py: sandbox/main.py:30` |
| Get | `GET /sandboxes/{id}` → `SandboxDetail`; 404 → not found | `py: api/client/api/sandboxes/get_sandboxes_sandbox_id.py:17-18`; `py: sandbox_async/sandbox_api.py:118-135`; `js: 5289` |
| Detail fields | `startedAt`, `endAt`, `cpuCount`, `memoryMB`, `diskSizeMB`, `state`, `metadata`, `domain`, `volumeMounts` (see "Volumes"), ... | `py: api/client/models/sandbox_detail.py:174-250` |
| Detail `lifecycle` | `{onTimeout: pause\|kill, autoResume: bool}`; an unknown, missing or `null` `onTimeout` reads as `TimeoutAction::Unknown` (the SDK raises on these); a missing `autoResume` reads as `false`; `lifecycle` absent or `null` → `None` | `py: api/client/models/sandbox_lifecycle.py:44-46`; `py: api/client/models/sandbox_detail.py:43, 227-232` |
| Detail `pauseMode` | `full` or `filesystem`: what the latest pause saved; returned once a pause has finished and omitted while running or still pausing (hosted E2B adds it a poll after `state` turns `paused`, live 2026-10-27); unknown values read as `None` | `py: api/client/models/sandbox_detail.py:44-47, 234-239`; `py: api/client/models/sandbox_pause_mode.py:5-6` |
| State values | `running`, `paused` | `py: api/client/models/sandbox_state.py:5-6` |
| Metrics | `GET /sandboxes/{id}/metrics?start&end`, both Unix seconds, each omitted when unset; 200 → `SandboxMetric[]`; 404 → not found | `py: api/client/api/sandboxes/get_sandboxes_sandbox_id_metrics.py:21-29, 39-60`; `py: sandbox_async/sandbox_api.py:280-336`; `js: 5346-5378` |
| Metric fields | `timestamp` (RFC 3339), `timestampUnix`, `cpuCount`, `cpuUsedPct` (float), `memUsed`, `memTotal`, `memCache`, `diskUsed`, `diskTotal` (bytes) | `py: api/client/models/sandbox_metric.py:28-36, 79-95` |
| List | `GET /v2/sandboxes?metadata&state&order&startedAfter&template&limit&nextToken` → `ListedSandbox[]`; `order` is `asc` or `desc` (by start time), `startedAfter` is `datetime.isoformat()` of an aware time (`2026-10-27T12:00:00+00:00`, or `...12:00:00.123456+00:00` when the microseconds are not zero; the crate formats it the same way and truncates nanoseconds, which Python cannot hold, rather than using chrono's `to_rfc3339`, which writes 3, 6 or 9 digits), `template` is a template ID or name; each filter is omitted when unset. Hosted E2B filters by the template ID that `get_info` reports (live, 2026-10-27) | `py: api/client/api/sandboxes/get_v2_sandboxes.py:14-61`; `py: api/client/models/listed_sandbox.py:115-137`; `py: sandbox/sandbox_api.py:1320-1340`; `py: sandbox_async/paginator.py:55-90`; `js: 5662` |
| List paging | next page token in response header `x-next-token` | `py: paginator.py:48-50` |
| List `state` | comma-joined | `py: api/client/api/sandboxes/get_v2_sandboxes.py:31-38`; `js: 1529-1532` (`explode: false`) |
| List `metadata` | each key and value `encodeURIComponent`-ed, then form-encoded into one string | `py: sandbox_async/paginator.py:50-57`; `js: 5656-5658` |
| Kill | `DELETE /sandboxes/{id}`, 204; 404 → `false` | `py: sandbox_async/sandbox_api.py:138-161`; `js: 5267` |
| Pause | `POST /sandboxes/{id}/pause`, body `{memory?}` from the mode: `true` for `full`, `false` for `filesystem` (only the filesystem is kept, and resuming cold-boots), `{}` when no mode is set (the API default is full); 204; 404 → not found; 409 without a mode → already paused (`false`), as in the SDK; 409 with a mode → `false` only when the message says the sandbox "is already paused" (hosted E2B: `Error pausing sandbox - sandbox '<id>' is already paused`, live 2026-10-28; matched case-sensitively; sandbawks's wording not yet checked, see M4 Task 17), else `Error::Api` 409 (issue #81). The SDK documents a 400 or 409 refusal of filesystem-only for snapshots, not for pause; hosted E2B took a filesystem pause on 2026-10-28 | `py: sandbox_async/sandbox_api.py:399-432`; `py: sandbox/sandbox_api.py:639-662`; `py: api/client/models/sandbox_pause_request.py:13-34`; `py: sandbox_async/main.py:791-829`; `js: 5442-5450`; live probe, hosted E2B, 2026-10-28 |
| Set timeout | `POST /sandboxes/{id}/timeout` body `{timeout}` (seconds); 404 → not found | `py: api/client/models/sandbox_timeout_request.py:17`; `py: sandbox_async/sandbox_api.py:164-187`; `js: 5396-5398` |
| Refresh | `POST /sandboxes/{id}/refreshes` body `{duration?}` (seconds), 204 | `py: api/client/api/sandboxes/post_sandboxes_sandbox_id_refreshes.py:21-38`; `py: api/client/models/sandbox_refresh_request.py:19` |
| Error body | `{code: int, message: string}`; the SDK surfaces `message` | `py: api/client/models/error.py:26-28`; `py: api/__init__.py:191-213` |

## Network

The create body's `network` object (`SandboxNetworkConfig`), the network
update body (`SandboxNetworkUpdateConfig`), and what the sandbox detail reports
back. The crate builds them with `NetworkConfig`, `NetworkUpdate` and
`NetworkRule` (`src/network.rs`).

| Fact | Citation |
|---|---|
| Fields, camelCase: `allowOut`, `denyOut`, `dnsServers`, `egressProxy`, `allowPublicTraffic`, `maskRequestHost`, `httpsPorts`, `rules`; each is sent only when the caller set it. The crate does not send `egressProxy` (deferred) | `py: api/client/models/sandbox_network_config.py:51-120`; `py: sandbox/sandbox_api.py:928-996` |
| `dnsServers: []` is sent, and means "no DNS server"; unset, `8.8.8.8` is allowed when `allowOut` names a domain | `py: sandbox/sandbox_api.py:955-958`; `py: api/client/models/sandbox_network_config.py:30-32` |
| A `network` with nothing set is left out of the create body (`network_body if network_body else UNSET`) | `py: sandbox_async/sandbox_api.py:237, 250` |
| `rules` maps a host to a list of rules, each `{"transform": {"headers": {...}}}`; a rule without a transform, or with no headers, is sent as `{}` or `{"transform": {}}`. Registering a host doesn't allow egress to it | `py: sandbox/sandbox_api.py:149-224, 826-869`; `py: api/client/models/sandbox_network_rule.py:30-46`; `py: api/client/models/sandbox_network_transform.py:30-41` |
| Hosted E2B accepts at most one rule per host: two give 400 `Domain "<host>" can have at most 1 transform rule.` | live, hosted E2B, 2026-10-27 |
| `ALL_TRAFFIC` is `0.0.0.0/0`; `allow_internet_access: false` works like `denyOut: ["0.0.0.0/0"]`, and both may be sent | `py: sandbox/network.py:1-8`; `py: sandbox_async/main.py:197` |
| Detail: `allowInternetAccess` (bool or `null`) and `network` (the create shape, or `null`); list entries carry neither | `py: api/client/models/sandbox_detail.py:51-69, 174-250`; `py: api/client/models/listed_sandbox.py` |
| Hosted E2B reports `allowInternetAccess: false` as given, not as a `denyOut` entry; it echoes `allowOut`, `dnsServers` and `rules` (header values included) and adds `allowPublicTraffic: true` when unset. A sandbox created without either reports both as `null` | live, hosted E2B, 2026-10-27 |
| Update: `PUT /sandboxes/{id}/network`, 204 → done (the crate, as for the other routes, takes any 2xx as success); 404 → not found; other statuses are errors | `py: api/client/api/sandboxes/put_sandboxes_sandbox_id_network.py:21-55`; `py: sandbox_async/sandbox_api.py:190-209`; `js: 5420` |
| Update body: `allowOut`, `denyOut`, `dnsServers`, `egressProxy`, `rules` in camelCase, and `allow_internet_access` in snake_case; each is sent only when set, and an update with nothing set is sent as `{}` (not left out, as an empty create `network` is). The crate does not send `egressProxy` (deferred) | `py: api/client/models/sandbox_network_update_config.py:50-103`; `py: sandbox/sandbox_api.py:1128-1152` |
| The update replaces the whole egress configuration: a field it leaves out is cleared, so `dnsServers` goes back to `8.8.8.8` and an omitted `rules` or `egressProxy` is dropped. The sandbox is not restarted | `py: sandbox/sandbox_api.py:515-561` |
| Hosted E2B applies an update at once, without a restart: an update with only `allow_internet_access: true` clears a create-time `denyOut: ["0.0.0.0/0"]`, and `denyOut: ["0.0.0.0/0"]` closes egress again, each seen by the first probe after the 204, with a tmpfs file kept | live, hosted E2B, 2026-10-27 (`clm_0144`) |

## Snapshots and fork

Snapshots are persistent copies of a sandbox that new sandboxes start from;
a fork starts running copies of a sandbox from a snapshot taken for it.
The crate builds the requests with `SnapshotOptions` and
`ListSnapshotsParams` (`src/snapshots.rs`), and `ForkOptions`
(`src/models.rs`).

| Fact | Citation |
|---|---|
| Create: `POST /sandboxes/{id}/snapshots`, body `{name?, memory?}`; `name` is left out when unset or empty (`name if name else UNSET`), `memory` is `true` for `full`, `false` for `filesystem`, left out when no mode is set (the API default is full); 201 → `{snapshotID, names}`; 404 → not found; other statuses are errors | `py: api/client/api/sandboxes/post_sandboxes_sandbox_id_snapshots.py:22-57`; `py: api/client/models/sandbox_snapshot_request.py:24-48`; `py: sandbox_async/sandbox_api.py:339-374`; `js: 5484` |
| The source sandbox pauses while the snapshot is taken and keeps running afterwards; a snapshot outlives the sandbox. Naming a snapshot that exists adds a new build to it | `py: sandbox_async/main.py:886-906` |
| A filesystem-only snapshot can be refused: 400 `snapshot_filesystem_only_disabled` (not enabled for the team) or 409 `snapshot_filesystem_only_unsupported_node` (the node predates it; pause, resume and retry). The crate returns both as `Error::Api` | `py: sandbox_async/main.py:905` |
| `SnapshotInfo`: `snapshotID` (a template ID or namespaced name, with a tag) and `names` (full names, `team-slug/name:tag`); `names` missing or `null` reads as empty, as the SDK's `list(names) if names else []` | `py: api/client/models/snapshot_info.py:11-55`; `py: sandbox/sandbox_api.py:1364-1370`; `py: sandbox_async/sandbox_api.py:371-374` |
| A sandbox is created from a snapshot by passing its `snapshotID` as the template | `py: sandbox_async/main.py:886-900`; `py: sandbox/sandbox_api.py:1364-1370` |
| List: `GET /snapshots?sandboxID&name&limit&nextToken` → `SnapshotInfo[]`, a `null` body read as an empty page; each parameter is left out when unset, empty or (for `limit`) zero, as the paginator passes `x if x else UNSET`; the next page token is in `x-next-token`. The instance method fills `sandboxID` with the sandbox's own ID. `name` matches a name or ID, optionally tag-qualified | `py: api/client/api/snapshots/get_snapshots.py:14-34`; `py: sandbox_async/paginator.py:103-160`; `py: paginator.py:48-50`; `py: sandbox_async/main.py:965-1044`; `js: 5721` |
| Delete: `DELETE /templates/{id}` (a snapshot is a template), 204 → `true`, 404 → `false`, other statuses are errors. The id is one path segment: `/` is sent as `%2F`. The SDKs also encode `:` (`quote(value, safe="")`, `encodeURIComponent`) where the crate's URL builder leaves it as is; `:` is a legal path character with the same meaning either way, and hosted E2B deleted `<id>:<tag>` snapshots sent this way | `py: api/client/api/templates/delete_templates_template_id.py:16-37`; `py: sandbox_async/sandbox_api.py:377-396`; `py: api/__init__.py:32-40`; `js: 5520-5528`; live, hosted E2B, 2026-10-28 |
| Hosted E2B refuses to delete a snapshot while a sandbox created from it runs: 400 `cannot delete template '<id>' because there are running sandboxes using it`. Once that sandbox is killed, the delete succeeds at once | live, hosted E2B, 2026-10-28 (`clm_0030`) |
| Hosted E2B's names for a snapshot named `<name>` contain `<name>` as given (lowercase), and listing by that bare name, without the team prefix, finds it | live, hosted E2B, 2026-10-28 (`clm_0147`, the snapshot sweep) |
| Fork: `POST /sandboxes/{id}/fork`, body `{timeout?, count?}` (timeout in seconds), each left out when unset, so `{}` takes the API defaults (one fork, 15 s); `count` must be an integer 1 to 20 (`MAX_FORK_COUNT`), else `InvalidArgument` before any request; 201 → one `SandboxForkResult` per fork; 404 → not found; other statuses (the spec lists 401, 409, 429, 500, 503) are errors | `py: api/client/api/sandboxes/post_sandboxes_sandbox_id_fork.py:22-66`; `py: api/client/models/sandbox_fork_request.py:13-39`; `py: sandbox/sandbox_api.py:81, 103-118`; `py: sandbox_async/sandbox_api.py:435-472`; `js: 5137-5141, 5579-5604` |
| The source is briefly paused, snapshotted with its full memory and resumed, keeping its ID and expiry; every fork boots from that one snapshot, taken once whatever the count | `py: sandbox_async/main.py:400-420`; `py: api/client/models/sandbox_fork_request.py:17-19` |
| `SandboxForkResult` is `{sandbox?, error?: {code, message}}`; the API sets exactly one. An error wins over a sandbox, and an entry with neither is an error ("Failed to start forked sandbox" in the SDK, `Error::Protocol` here). A per-fork error is `Error::Api { status: code, message }`, a per-fork 404 included: it refers to something that fork needed, not the source. The SDK maps the code to an exception class; the crate keeps the code. The crate decodes each entry on its own, so a malformed one is that fork's `Error::Decode`, or `Error::Protocol` naming the sandbox when the entry has a `sandbox.sandboxID` (that fork may be running); the SDK fails the whole call on one. A per-fork 404 is still `is_not_found()`, as any API 404 is. A `null` body is `Error::Decode`, as the Python SDK raises on it (the JS SDK reads it as no forks) | `py: api/client/models/sandbox_fork_result.py:17-76`; `py: sandbox_async/sandbox_api.py:466-493`; `py: api/client/models/error.py:26-28`; `js: 5596-5604` |
| Hosted E2B starts forks with the source's metadata (a fork's `get_info().metadata` carries the source's keys) and in state `running`, with a `/tmp` file the source wrote before the fork | live, hosted E2B, 2026-10-28 (`clm_0030_fork_starts_running_copies`) |

## Volumes

Volumes are persistent storage that outlives sandboxes and is mounted into
them at create. The crate manages them with `Client::{create_volume,
get_volume, list_volumes, destroy_volume}` and mounts them with
`CreateSandboxParams::volume_mount` (`src/volumes.rs`, `src/client.rs`,
`src/models.rs`). The volume content API (files in a volume, reached with the
volume's token, without a sandbox) is not implemented.

| Fact | Citation |
|---|---|
| Create: `POST /volumes` body `{name}`, 201 → `VolumeAndToken` `{volumeID, name, token, domain?}`; other statuses are errors | `py: api/client/api/volumes/post_volumes.py:21-52`; `py: api/client/models/volume_and_token.py:50-63`; `py: volume/volume_async.py:134-173` |
| Get: `GET /volumes/{id}`, 200 → `VolumeAndToken`; 404 → not found (`Error::VolumeNotFound`, the SDK's `VolumeNotFoundException`); other statuses are errors. The ID is one path segment | `py: api/client/api/volumes/get_volumes_volume_id.py:17-43`; `py: volume/volume_async.py:197-237` |
| List: `GET /volumes`, 200 → `[{volumeID, name}]` (no tokens, no paging); a `null` body reads as none | `py: api/client/api/volumes/get_volumes.py:15-42`; `py: volume/volume_async.py:240-262`; `py: volume/types.py:11-18` |
| Destroy: `DELETE /volumes/{id}`, 204 → `true`, 404 → `false`; other statuses are errors | `py: api/client/api/volumes/delete_volumes_volume_id.py:16-41`; `py: volume/volume_async.py:265-285` |
| `domain` is set only for teams on a custom (BYOC) cluster; an empty or missing one reads as `None` (`domain if isinstance(domain, str) and domain else None`). The SDK then falls back to the configured domain for content requests; the crate, which has no content API, leaves that to the caller | `py: volume/types.py:21-30`; `py: volume/volume_async.py:159-163, 227-231` |
| The token is a secret: `Volume` keeps it in a `SecretString`, and its `Debug` prints the ID, name and domain only | crate rule (CLM-0122's) |
| Mounts at create: `volumeMounts: [{name, path}]` in the create body, left out when there are none (`volume_mounts if volume_mounts else UNSET`). The SDK takes a dict from path to volume (or volume name), so one path holds one mount; a second `volume_mount` at the same path replaces the first in its place, as a dict assignment does | `py: sandbox_async/main.py:203, 218-227`; `py: sandbox_async/sandbox_api.py:252`; `py: api/client/models/sandbox_volume_mount.py:10-43`; `py: api/client/models/new_sandbox_v2.py:97-102, 131-132` |
| Mounts read back: `volumeMounts` in the detail and in list entries; missing or `null` reads as none (`d.pop("volumeMounts", UNSET) or []`) | `py: api/client/models/sandbox_detail.py:241-246`; `py: api/client/models/listed_sandbox.py:139-144` |
| Hosted E2B, for a team without volumes: `POST /volumes` answers 403 `use of volumes is not enabled`, while `GET /volumes` answers 200 `[]`. A missing volume with a UUID-shaped ID is 404 `Volume not found` on get and destroy; an ID of another shape is 400 `Invalid volume ID` (an `Error::Api`, not `VolumeNotFound`, as in the SDK) | live, hosted E2B, 2026-10-28 (`clm_0148_missing_volume_is_not_found_and_not_destroyed`) |

## MCP gateway

A sandbox created with an MCP configuration runs an MCP gateway that serves
the configured MCP servers over HTTP. The crate takes the configuration
with `CreateSandboxParams::mcp` (`src/models.rs`), starts the gateway in
`Client::create_sandbox` (`src/client.rs`, `Sandbox::start_mcp_gateway` in
`src/sandbox.rs`), and exposes it through `Sandbox::{mcp_url, mcp_token}`;
the constants and the command are in `src/mcp.rs`.

| Fact | Citation |
|---|---|
| The configuration is a JSON object keyed by server name (e.g. `{"fetch": {}}`), sent as `mcp` in the create body. An empty object is left out of the body (`mcp or UNSET`; the JS SDK sends `{}`), but still starts the gateway. The crate takes a `serde_json::Map` and does not check the server names; the SDK's generated `McpServer` types are not ported (see the plan's deviations) | `py: sandbox_async/sandbox_api.py:219, 246`; `py: api/client/models/new_sandbox_v2.py:85-91, 127-128`; `js: 5550` |
| Template: the one given, else `mcp-gateway` when an MCP configuration is set, else `base`. Python also replaces an empty template (`if not template`); the crate, like the JS SDK's `??`, sends an explicit `""` as given | `py: sandbox/main.py:32-33`; `py: sandbox_async/main.py:214-217`; `js: 5873, 6324` |
| Start, after the create returns: `mcp-gateway --config <quoted JSON>` through the process service (`/bin/bash -l -c`), as `root`, with one environment variable, `GATEWAY_ACCESS_TOKEN`, set to a new random UUID v4 (lowercase, hyphenated). The token is only in the environment, never on the command line | `py: sandbox_async/main.py:245-256`; `js: 5883-5890` |
| Quoting is `shlex.quote` (the JS SDK ports it): the empty string is `''`; a string of only ASCII letters, digits and `_@%+=:,./-` is left as is; anything else is wrapped in `'...'` with each `'` written as `'"'"'`. A JSON object always contains `{`, so it is always quoted | Python 3 `shlex.quote`; `js: 268-281` |
| The JSON is compact, `{"fetch":{}}`, as the JS SDK's `JSON.stringify` writes it; Python's `json.dumps` writes `{"fetch": {}}` and escapes non-ASCII as `\uXXXX`. Keys come out sorted (`serde_json::Map` is a `BTreeMap`), where the SDKs keep insertion order. Each form is the same JSON object to the gateway | `py: sandbox_async/main.py:251`; `js: 5887` |
| Failure: if the gateway exits non-zero, or the command call itself fails, the sandbox is killed and the kill's own error is ignored. A non-zero exit is `Error::McpGateway(<stderr>)` (the SDKs' `SandboxException("Failed to start MCP gateway: <stderr>")`); any other error is returned as it is | `py: sandbox_async/main.py:257-266`; `js: 5891-5893` |
| URL: `https://<get_host(50005)>/mcp`, i.e. `https://50005-<sandbox id>.<sandbox domain>/mcp` | `py: sandbox/main.py:28, 226-232`; `js: 5785, 6201-6203` |
| Token: the one generated at create, kept on the handle; else read once from `/etc/mcp-gateway/.token` as `root` and kept (a failed read is not kept). The crate trims trailing whitespace from the file, where the SDKs return it as is (see the plan's deviations). Clones of a `Sandbox` share the kept token | `py: sandbox_async/main.py:1063-1073`; `js: 6209-6213` |
| The token is a secret: it is a `SecretString`, and `Sandbox`'s `Debug` does not print it | crate rule (CLM-0122's) |
| How the token is used (the SDKs don't say): hosted E2B's gateway answers an MCP `initialize` `POST` to the URL with 401 without an `Authorization` header and with `Authorization: Bearer <another UUID>`, and with 200 with `Authorization: Bearer <the token>`. The token from the file equals the one generated at create | live, hosted E2B, 2026-10-28 (`clm_0150`) |

## Connect-RPC framing

envd's process and filesystem services are Connect-RPC.

**Codec choice: JSON.** Both SDKs use Connect's JSON codec, not binary
protobuf: the JS SDK sets `useBinaryFormat: false` (`js: 5824-5828`) and the
Python SDK installs a JSON codec on purpose "matching the JS SDK's
`useBinaryFormat: false`" (`py: envd/client_shared.py:29-60`). So JSON is what
envd and E2B-compatible servers are known to accept, and it needs no protobuf
toolchain. Hence no prost: messages are serde structs following the proto3
JSON mapping (lowerCamelCase names, default values omitted, `bytes` as base64,
`int64` as a string, enums by name; parsers accept numbers and either base64
alphabet). Compression is off in both directions (`py: envd/client_shared.py:133-143`).

| Fact | Citation |
|---|---|
| Route: `POST <base>/<package.Service>/<Method>` | `py: envd/process/process_connect.py:80-150`; `py: envd/filesystem/filesystem_connect.py:79-159` |
| Unary methods: `process.Process/{List, SendInput, SendSignal, CloseStdin}`, `filesystem.Filesystem/{Stat, MakeDir, Move, ListDir, Remove}` | same lines |
| Server-streaming: `process.Process/{Start, Connect}` | `py: envd/process/process_connect.py:90, 100` |
| Client-streaming: `process.Process/StreamInput` (one envelope per request message; the answer is one message plus an end-of-stream envelope) | `py: envd/process/process_connect.py:110-128` |
| Unary `Content-Type: application/json`; streaming `application/connect+json` | `connect-es: protocol-connect/content-type.js:37-40`; `protocol-connect/request-header.js:31-37` |
| `Connect-Protocol-Version: 1` | `connect-es: protocol-connect/headers.js:27`; `protocol-connect/version.js:28`; `protocol-connect/request-header.js:38` |
| Deadline header `Connect-Timeout-Ms` | `connect-es: protocol-connect/headers.js:26`; `protocol-connect/request-header.js:28-30` |
| Timeouts sent as whole ms, at least 1; `None` or a zero duration sends no header and sets no client-side deadline (the SDK's `timeout_to_ms` maps `0` and `None` to no deadline) | `py: envd/utils.py:15-24` |
| Envelope: 1 flag byte + 4-byte big-endian length + payload | `connect-es: protocol/envelope.js:38` |
| End-of-stream flag `0x02`; payload `{"error"?: {...}, "metadata"?: {...}}` | `connect-es: protocol-connect/end-stream.js:30, 37-73` |
| Error JSON: `code` is a snake_case string, `message` a string | `connect-es: protocol-connect/error-json.js:42-60`; `protocol-connect/code-string.js:24-31` |
| Code list | `connect-es: code.js:31-91` |
| A non-Connect error body (proxy answering for envd) maps by HTTP status (404 → `not_found`, 409 → `already_exists`, 502/503 → `unavailable`, ...); an integer JSON `code` is taken as the status | `py: envd/client_shared.py:65-125` |

## Process service

Messages from `process/process.proto` as embedded in
`py: envd/process/process_pb.py`:

| Message | Fields used | Lines |
|---|---|---|
| `ProcessConfig` | `cmd`, `args`, `envs`, `cwd?` | 85-103 |
| `StartRequest` | `process`, `pty?`, `tag?`, `stdin?` | 217-260 |
| `ProcessEvent` oneof `start`/`data`/`end`/`keepalive` | | 318-340 |
| `StartEvent` | `pid` | 346-352 |
| `DataEvent` oneof `stdout`/`stderr`/`pty` (bytes) | | 374-394 |
| `EndEvent` | `exit_code` (sint32), `exited`, `status`, `error?` | 402-420 |
| `StartResponse` / `ConnectResponse` | `event` | 466-500 |
| `ListResponse` / `ProcessInfo` | `processes[]`: `config`, `pid`, `tag?` | 149-195 |
| `SendInputRequest` / `ProcessInput` oneof `stdin`/`pty` | | 522-596 |
| `SendSignalRequest` | `process`, `signal` | 722-732 |
| `CloseStdinRequest` | `process` | 774-780 |
| `ConnectRequest` | `process` | 820-826 |
| `ProcessSelector` oneof `pid`/`tag` | | 848-866 |
| `Signal` | `SIGNAL_SIGTERM = 15`, `SIGNAL_SIGKILL = 9` | 874-888 |

Behaviour:

| Fact | Citation |
|---|---|
| Commands run as `/bin/bash -l -c <cmd>` with `envs`, `cwd`, `stdin` (default `false`) | `py: sandbox_async/commands/command.py:291-292, 320-330` |
| Command stream default timeout 60 s, sent as the stream deadline | `py: sandbox_async/commands/command.py:218, 335-338` |
| Opening the stream waits for the first event within the request timeout; it must be `start` and carries the pid | `py: sandbox_async/commands/command.py:382-411`; `py: envd/utils.py:27-41` |
| Output is accumulated from `data.stdout`/`data.stderr`; `end` gives `exit_code` and `error` | `py: sandbox_async/commands/command_handle.py:154-183` |
| A stream that ends without an `end` event is an error | `py: sandbox_async/commands/command_handle.py:252-253` |
| `kill(pid)` is `SendSignal(SIGKILL)`; `not_found` → `false` | `py: envd/rpc.py:208-231` |
| `send_stdin` is `SendInput` with `input.stdin` bytes | `py: sandbox_async/commands/command.py:133-146` |
| `close_stdin` needs envd ≥ 0.5.2 | `py: sandbox_async/commands/command.py:189-193`; `py: envd/versions.py:7` |
| `list` maps `ProcessInfo` (`tag`, `cwd` optional) | `py: sandbox_async/commands/command.py:75-95` |

## Streamed input

`Commands::stream_input` returns an `InputStream` over the client-streaming
`StreamInput` RPC (`src/stream_input.rs`). The request body is streamed from a
bounded queue by a background task.

| Fact | Citation |
|---|---|
| `StreamInput` is client-streaming at `/process.Process/StreamInput`, sent as `application/connect+json` | `py: envd/process/process_connect.py:110-128` |
| `StreamInputRequest` is a oneof `start {process: ProcessSelector}` / `data {input: ProcessInput}` / `keepalive {}`; the response is empty | `py: envd/process/process_pb.py:597-716` |
| `ProcessInput` is a oneof `stdin` / `pty` (bytes, base64 in JSON) | `py: envd/process/process_pb.py:569-596` |
| The handle sends `start` first, then one `data` per queued chunk, and a `keepalive` after 50 s without input | `py: sandbox_async/commands/command_input_handle.py:91-131`; `py: sandbox/commands/stream_input.py:10-37`; `js: 2781-2870` |
| `close()` ends the request stream, then waits for the call to finish | `py: sandbox_async/commands/command_input_handle.py:71-83` |
| Keepalive interval 50 s (`KEEPALIVE_PING_INTERVAL_SEC`) | `py: connection_config.py:48-49` |
| Stream deadline defaults to 60 s (`STREAM_INPUT_TIMEOUT`), sent as `Connect-Timeout-Ms`; `None` or zero disables it | `py: connection_config.py:33`; `py: sandbox_async/commands/command.py:150-174` |
| No envd version gate | `py: sandbox_async/commands/command.py:150-174`; `py: envd/versions.py:1-11` |

The SDK's queue is unbounded; `InputStream::send` waits once 64 chunks are
queued. An error that ends the call early is returned by the next `send` or
by `close`.

## PTY

`Sandbox::pty()` returns a `Pty` (`src/pty.rs`). A session's output is read
from a `PtyHandle`, whose events are `PtyEvent::{Data, Exit}`, kept apart from
a command's `CommandEvent::{Stdout, Stderr}`. Both handles open their streams
through the same `open_process_stream` (`src/commands.rs`).

| Fact | Citation |
|---|---|
| `PTY {size: PTY.Size {cols: uint32 = 1, rows: uint32 = 2}}`; `StartRequest.pty` is optional field 2, `stdin` optional field 4 | `py: envd/process/process_pb.py:18-80, 212-260` |
| `create` sends `Start` with `cmd "/bin/bash"`, `args ["-i", "-l"]`, `envs`, `cwd`, `pty.size {cols, rows}` and no `stdin` field | `py: sandbox_async/commands/pty.py:166-185`; `js: 4682-4712` |
| `TERM=xterm-256color`, `LANG=C.UTF-8` and `LC_ALL=C.UTF-8` are added to `envs` only when the caller did not set them | `py: sandbox_async/commands/pty.py:162-165` |
| `create` sends the user header and `Keepalive-Ping-Interval: 50`; `connect` sends `Connect {process: {pid}}` with the keepalive header only; in the SDKs both default to a 60 s stream deadline (here `PtyOptions` defaults to it, and `Pty::connect` takes the deadline directly, so callers pass `Some(DEFAULT_PTY_TIMEOUT)` for the SDK value) | `py: sandbox_async/commands/pty.py:139-185, 207-234` |
| The deadline counts from when the stream opens. When a `Start` stream's deadline passes, the stream ends with `deadline_exceeded` and envd ends the shell: the pid leaves `List`, and `SendInput` answers `not_found` | observed on hosted E2B, 2026-10-27 (a 3 s PTY, checked 2 s after the deadline) |
| Both wait for the opening `start` event within the request timeout, as commands do | `py: sandbox_async/commands/pty.py:187-199, 236-248` |
| Output arrives as `DataEvent.pty` bytes, next to `stdout`/`stderr` in the same oneof | `py: envd/process/process_pb.py:369-394`; `py: sandbox_async/commands/command_handle.py:169, 221-231` |
| `send_stdin` is `SendInput` with `input.pty` bytes | `py: sandbox_async/commands/pty.py:85-111` |
| `stream_input` is the `StreamInput` handle with the `pty` member | `py: sandbox_async/commands/pty.py:113-137` |
| `resize` is unary `/process.Process/Update {process: {pid}, pty: {size: {cols, rows}}}`, answered by an empty `UpdateResponse` | `py: sandbox_async/commands/pty.py:256-283`; `py: envd/process/process_connect.py:110-115`; `py: envd/process/process_pb.py:261-296` |
| `kill` is the shared `SendSignal(SIGKILL)` with `not_found` → `false` | `py: sandbox_async/commands/pty.py:65-83`; `py: envd/rpc.py:208-248` |

`PtySize` holds `u16` cells (as terminals report them), widened to the
proto's `uint32` on the wire. The SDK's `on_data` callback is replaced by the
handle's `next_event` loop, as for commands.

## Filesystem service and file API

Messages from `filesystem/filesystem.proto` as embedded in
`py: envd/filesystem/filesystem_pb.py`:

| Message | Fields | Lines |
|---|---|---|
| `MoveRequest` / `MoveResponse` | `source`, `destination` / `entry` | 25-65 |
| `MakeDirRequest` / `MakeDirResponse` | `path` / `entry` | 87-121 |
| `RemoveRequest` | `path` | 143-149 |
| `StatRequest` / `StatResponse` | `path` / `entry` | 189-223 |
| `EntryInfo` | `name`, `type`, `path`, `size` (int64), `mode`, `permissions`, `owner`, `group`, `modified_time?`, `symlink_target?`, `metadata`, `is_symlink` | 245-303 |
| `ListDirRequest` / `ListDirResponse` | `path`, `depth` / `entries[]` | 347-387 |
| `FileType` | `FILE_TYPE_FILE = 1`, `FILE_TYPE_DIRECTORY = 2`, `FILE_TYPE_SYMLINK = 3` | 747-765 |

Behaviour:

| Fact | Citation |
|---|---|
| File routes `/files` and `/health` on the direct URL | `py: envd/api.py:21-22`; `py: sandbox/main.py:102-129` |
| Read: `GET /files?path=<p>[&username=<u>]`, body is the file | `py: sandbox_async/filesystem/filesystem.py:197-212`; `js: 2153-2170` |
| Streamed download (`read_stream`): the same `GET /files`, the body handed over chunk by chunk (`FileStream::next_chunk`); no whole-request timeout unless the caller sets one, and the status is checked before the first chunk, so a missing file fails `read_stream` itself | `py: sandbox_async/filesystem/filesystem.py:197-262` |
| Write (default encoding): `POST /files?path=<p>[&username=<u>]`, `multipart/form-data`, field `file`, filename = path; response is a JSON array of `{name, type, path}` | `js: 2226-2249`; `py: sandbox_async/filesystem/filesystem.py:445-462`; `js: index.d.ts:4567-4580` |
| envd HTTP errors carry `{"message"}`; 400/401/404/429/502/507 are distinguished | `py: envd/api.py:24-33, 143-152` |
| `list` depth must be ≥ 1 | `py: sandbox_async/filesystem/filesystem.py:508-513` |
| `exists` is `Stat` with `not_found` → `false` | `py: sandbox_async/filesystem/filesystem.py:530-560` |
| `make_dir` returns `false` on `already_exists` | `py: sandbox_async/filesystem/filesystem.py:647-676` |
| `rename` is `Move` and returns the new entry | `py: sandbox_async/filesystem/filesystem.py:614-645` |
| Filesystem RPCs send the user via the `Authorization` header | `py: sandbox_async/filesystem/filesystem.py:517, 551, 583, 609, 640, 668` |
| Every filesystem call takes `user` and a per-call `request_timeout` (`FileOptions`, `ListOptions`, `WriteOptions`, `WatchOptions`); unset uses the connection's request timeout, `None` or zero means no limit; an RPC sends it as `Connect-Timeout-Ms` and enforces it client-side, the file API as a whole-request timeout; `list` defaults to depth 1 | `py: sandbox_async/filesystem/filesystem.py:116-121, 491-496, 530-534, 562-566, 590-594, 614-619, 647-650, 684-685` |
| `is_running`: `GET /health`, 502 → not running | `py: sandbox_async/main.py:134-168` |

## Uploads

`Filesystem::write_files(&[WriteEntry], &WriteOptions)` writes a batch of
in-memory files (`src/filesystem.rs`). `Filesystem::write` is a one-entry
`write_files` with default options, so its wire request is unchanged.
`Filesystem::write_stream(path, impl AsyncRead, &WriteOptions)` uploads one
file from a reader, read as the body is sent.

| Fact | Citation |
|---|---|
| `write_files([])` returns `[]` without a request | `py: sandbox_async/filesystem/filesystem.py:360-361` |
| `write` is `write_files` with one entry | `py: sandbox_async/filesystem/filesystem.py:291-329` |
| Multipart (the default for in-memory data): one `POST /files` for all files, every part named `file` with filename = path; the `path` query is sent only for exactly one file; `username` as for reads | `py: sandbox_async/filesystem/filesystem.py:445-462`; `py: sandbox/filesystem/filesystem.py:267-280` |
| Octet-stream: one `POST /files?path=<p>[&username=<u>]` per file, `Content-Type: application/octet-stream`, the body is the file; the requests run concurrently (`asyncio.gather`) and the results are concatenated in entry order | `py: sandbox_async/filesystem/filesystem.py:400-444` |
| Gzip: the octet-stream request with `Content-Encoding: gzip` and a gzip body; gzip implies octet-stream, since the encoding applies to the whole request body | `py: sandbox_async/filesystem/filesystem.py:380-384, 409-411`; `py: sandbox/filesystem/filesystem.py:323-339` |
| Octet-stream and gzip need envd 0.5.7: `use_octet_stream = (use_octet_stream or gzip) and envd >= 0.5.7`, so older envd gets *uncompressed* multipart | `py: sandbox_async/filesystem/filesystem.py:369-384`; `py: envd/versions.py:8` |
| Every answer is a JSON array of `{name, type, path, metadata?}`; an empty array (or not an array) is an error | `py: sandbox_async/filesystem/filesystem.py:431-438, 480-487`; `py: sandbox/filesystem/filesystem.py:56-90` |
| An in-memory upload is bounded by the request timeout | `py: sandbox_async/filesystem/filesystem.py:386-393` |
| A streamed upload (`write_stream`) is an octet-stream `POST /files?path=<p>[&username=<u>]` whose body is read from the caller's reader as it is sent (chunked); with gzip, the reader is compressed on the fly and `Content-Encoding: gzip` is set | `py: sandbox_async/filesystem/filesystem.py:400-419`; `py: sandbox/filesystem/filesystem.py:323-339` |
| A streamed upload on envd older than 0.5.7 is a multipart upload with one streamed `file` part (filename = path, `path` query set), uncompressed even when gzip was asked for | `py: sandbox_async/filesystem/filesystem.py:369-384, 445-462` |
| A streamed upload carries no client-side timeout | `py: sandbox_async/filesystem/filesystem.py:386-393, 413-419` |
| Metadata (`WriteOptions::metadata`) goes as one `X-Metadata-<key>: <value>` header per pair on every upload request: the multipart request, each octet-stream request and the streamed one; in a multi-file upload every file gets the same map | `py: sandbox/filesystem/filesystem.py:342, 367-373`; `py: sandbox_async/filesystem/filesystem.py:394-396, 409, 462` |
| Metadata is validated before any request: a key is a non-empty HTTP token (`A-Za-z0-9!#$%&'*+-.^_`\|~`), a value printable US-ASCII (`0x20..=0x7e`, empty allowed); keys are checked before the envd version | `py: sandbox/filesystem/filesystem.py:344-364`; `py: sandbox_async/filesystem/filesystem.py:363-367` |
| Metadata needs envd 0.6.2; on older envd any metadata is refused before a request (the crate's `Error::InvalidArgument`, the SDK's `TemplateException`) | `py: sandbox_async/filesystem/filesystem.py:365-366`; `py: envd/versions.py:9` |
| `write_files([])` returns before the metadata is checked | `py: sandbox_async/filesystem/filesystem.py:360-363` |
| envd stores metadata as `user.e2b.*` extended attributes and lowercases the keys (the crate lowercases them already in `WriteOptions::metadata`, so the last call for a key wins whatever its case); the upload answer, `Stat`, `ListDir` and `include_entry` watch events report it as `metadata` (proto `map<string, string>` field 11, a JSON object); absent or `null` reads as an empty map | `py: sandbox/filesystem/filesystem.py:57-90, 130-148, 376-380`; `py: envd/filesystem/filesystem_pb.py:240, 291-297` |

Differences: the SDK compresses an in-memory gzip upload whole
(`gzip.compress`) and sends it with a `Content-Length`. This crate
compresses it as it is sent (`async-compression`'s `GzipEncoder` over the
buffer), so the request is chunked and the compressed copy is never held
whole. envd decodes both; hosted envd 0.10.0 accepted it live on 2026-10-27.
`write` returns the first entry of the answer, where the SDK errors unless
there is exactly one; that is the v0.1 behaviour, kept.
No metadata is reported as an empty map, where the SDK uses `None`. Because
the streamed multipart fallback only applies below envd 0.5.7, it never
carries metadata: metadata on such envd is refused first. Hosted envd 0.10.0
stored and reported metadata live on 2026-10-27.

## Directory watching

`Filesystem::watch_dir` returns a `WatchHandle` over the server-streaming
`WatchDir` RPC (`src/filesystem.rs`). Events are pulled with
`WatchHandle::next_event`, and dropping the handle closes the stream, which
is the SDK's `stop()`.

| Fact | Citation |
|---|---|
| `WatchDir` is server-streaming at `/filesystem.Filesystem/WatchDir` | `py: envd/filesystem/filesystem_connect.py:129-137` |
| `WatchDirRequest {path = 1, recursive = 2, include_entry = 3, allow_network_mounts = 4}`; proto JSON names `includeEntry`, `allowNetworkMounts` | `py: envd/filesystem/filesystem_pb.py:404-455, 816` |
| `WatchDirResponse` is a oneof `start {}` / `filesystem FilesystemEvent` / `keepalive {}` | `py: envd/filesystem/filesystem_pb.py:499-560` |
| `FilesystemEvent {name = 1, type = 2, entry = 3?}`; `name` is relative to the watched directory; `entry` only with `include_entry`, and may be absent for removals and renames away | `py: envd/filesystem/filesystem_pb.py:457-497`; `py: sandbox/filesystem/watch_handle.py:49-70` |
| `EventType`: `UNSPECIFIED = 0`, `CREATE = 1`, `WRITE = 2`, `REMOVE = 3`, `RENAME = 4`, `CHMOD = 5`; unspecified and unknown types are dropped, as are keepalives | `py: envd/filesystem/filesystem_pb.py:774-810`; `py: sandbox/filesystem/watch_handle.py:36-46`; `py: sandbox_async/filesystem/watch_handle.py:47-64` |
| Version gates, checked before any request: `recursive` needs envd 0.1.4, `include_entry` 0.6.3, `allow_network_mounts` 0.6.4 (the SDKs raise `TemplateException`; here `Error::InvalidArgument`) | `py: sandbox_async/filesystem/filesystem.py:705-721`; `py: envd/versions.py:3, 10, 11` |
| Headers: the user header and `Keepalive-Ping-Interval: 50`; the watch timeout (default 60 s; `None` or zero for no limit, as the SDK's `0`/`None`) is sent as `Connect-Timeout-Ms` and bounds the whole stream | `py: sandbox_async/filesystem/filesystem.py:685, 723-740` |
| The stream must open with `start`, awaited within the request timeout; anything else is an error | `py: sandbox_async/filesystem/filesystem.py:742-766` |

The SDK's `on_event`/`on_exit` callbacks are replaced by the handle's
`next_event` loop, as for commands and PTY sessions. A clean end of the
stream is `Ok(None)`; the error that ends a watch is returned by
`next_event`, where the SDK hands it to `on_exit`.

## Signed URLs

`Sandbox::download_url(path, &FileUrlOptions)` and
`Sandbox::upload_url(path, &FileUrlOptions)` build `/files` URLs that work
with no API key or envd headers (`src/sandbox.rs`). The caller `GET`s a
download URL, and `POST`s a file to an upload URL as `multipart/form-data`
in a part named `file`. `FileUrlOptions` carries the user and an optional
expiry (`expires_in`).

| Fact | Citation |
|---|---|
| The URL is `<envd direct url>/files?path=<p>[&username=<u>]&signature=<sig>[&signature_expiration=<unix s>]`; `path` is left out when empty, `username` when no user (or an empty one) applies | `py: sandbox/main.py:102-129`; `js: 6316-6321` |
| `sig = "v1_" + base64(sha256(raw))` with standard alphabet and the `=` padding stripped, `raw = "<path>:<read\|write>:<user or empty>:<token>"`, plus `":<expiration>"` when an expiry is set; download signs `read`, upload `write` | `py: sandbox/signature.py:15-47`; `py: sandbox/main.py:157-163, 199-205`; `js: 1538-1548` |
| The expiry is the current Unix time plus the requested seconds, sent as `signature_expiration` and included in the signed string | `py: sandbox/signature.py:28-42`; `js: 1541-1545` |
| With no user, the user is `"user"` on envd older than 0.4.0 (in both `username` and the signature), else empty in the signature and absent from the URL | `py: sandbox/main.py:153-155, 195-197`; `py: envd/versions.py:6`; `js: 6232, 6266` |
| A sandbox with no envd access token gets the unsigned URL; asking for an expiry on one is an error (the SDKs' `InvalidArgumentException`/`InvalidArgumentError`, the crate's `Error::InvalidArgument`), checked before anything else | `py: sandbox/main.py:147-151, 189-193`; `js: 6229-6230, 6263-6264` |
| Every hosted sandbox is secured, so hosted URLs are always signed | `py: sandbox_async/main.py:177, 197` |

The query is form-encoded by `url::Url::query_pairs_mut`, like the JS SDK's
`URLSearchParams` (`/` becomes `%2F`, a space `+`); the Python SDK's `quote`
leaves `/` alone and writes a space as `%20`. envd decodes both, and the
signature is over the raw path, so neither changes it. The URL carries the
signature, never the token, but it is a bearer credential until it expires.
`EnvdClient` keeps the token as a `secrecy::SecretString`, so `Debug` output
shows `[REDACTED]`. The expiry is counted in whole seconds, as in the SDKs: the current Unix
second plus `expires_in`'s whole seconds (any fraction is dropped). An
`expires_in` under one second is refused with `Error::InvalidArgument`,
where the SDKs would sign a URL that is already expired (`0` is still an
expiry in 2.61.0, `py: sandbox/signature.py:28-32`, `js: 1541`).
Hosted envd accepted a multipart `POST` to an upload URL and
served a download URL with no credentials live on 2026-10-27, and refused
the download URL with its `signature` pair removed with a 4xx.

## Error classification

The crate keeps one variant per protocol (`Api`, `Envd`, `Rpc`) and answers
the SDKs' exception classes with methods on `Error`.

| Fact | Citation |
|---|---|
| REST: 401 → `AuthenticationException`, 429 → `RateLimitException`, 503 → `ServiceBusyException`; other statuses keep `status_code`. Crate: `Error::Api { status }` with `is_auth`, `is_rate_limited`, `is_service_busy`, and `status()` | `py: api/__init__.py:155-213` |
| envd HTTP: 400 invalid argument, 401 authentication, 404 not found, 429 rate limit, 502 sandbox unavailable, 507 not enough space. Crate: `Error::Envd { status }` with `is_auth`, `is_not_found`, `is_rate_limited`, `is_sandbox_unreachable`, `is_not_enough_space`, and `status()` | `py: envd/api.py:24-33` |
| envd RPC: `unauthenticated` → authentication, `not_found`, `unavailable` → sandbox unavailable, `resource_exhausted` → rate limit, `canceled` and `deadline_exceeded` → timeout. Crate: `Error::Rpc { code }` with `is_auth`, `is_not_found`, `is_sandbox_unreachable`, `is_rate_limited`, `is_timeout` | `py: envd/rpc.py:26-40` |
| A 502 or `unavailable` from the proxy says "was not found" (the sandbox is gone) or "port is not open" (envd unreachable); both are `is_sandbox_unreachable`, and the message says which | `py: exceptions.py:4-36` |
| `is_retryable`: rate limited, a busy REST API, or a connection that was never made (`TransportError::is_connect`); not a timeout, which may have taken effect | crate decision (M4, decision 1) |
| `Error::Transport` holds an opaque `TransportError`: reqwest is not part of the public API | crate decision (M4, decision 4) |

## Deliberate differences from the SDKs

- **Classifier methods, not exception classes.** The SDKs raise one class per
  category; this crate keeps the protocol in the variant (`Api`, `Envd`,
  `Rpc`) and answers the category with `Error::is_*` (see "Error
  classification"). `is_sandbox_unreachable` stands for both
  `SandboxNotRunningException` and `SandboxUnreachableException`.
- **Non-zero exit is not an error.** The SDKs raise `CommandExitException`
  from `run`; this crate returns `CommandResult` with `exit_code` and a
  `success()` helper, which is the idiomatic Rust shape.
- **`wait()` does not replay consumed output.** The SDK handle buffers all
  output; here `next_event()` hands chunks to the caller and `wait()` only
  collects what is left, so output is held once.
- **A refused moded pause is an error.** SDK 2.61 returns `False` for every
  pause 409. With a `SnapshotMode`, this crate returns `false` only for the
  409 that says the sandbox "is already paused", and `Error::Api` 409 for any
  other, so a refused filesystem-only pause is not mistaken for a pause
  (issue #81). Without a mode it matches the SDK.
- **Not implemented yet.** Milestone 4: the `is_symlink` file type (#30)
  and the API review (#52 to #57).
  Deferred from milestone 3 and unscheduled: the SOCKS5 egress proxy, IAM
  workload identity (`iam.tokens` and identity placeholders in rule
  transforms), and the volume content API. Also unscheduled: sandbox health
  probing on transport errors, sandbox logs, secrets and git. Not planned:
  batch metrics, callable egress selectors and transforms, and the
  deprecated `secure` and `keep_memory` flags. Out of scope by design:
  template builds and a sync API.

## Verification against a live server

Every v0.1 behaviour was verified live on 2026-10-26 by the M1 suite, the
M2 behaviours on 2026-10-27, and the M3 behaviours on 2026-10-28 (`tests/live_*.rs`, `just test-live <target>`). Each test is named after the
claim it backs in `claims.json`. The wire details in this file are covered by
mock and unit tests instead; `docs/verification/s4-evidence-map.md` maps each
one to its tests.

- **Hosted E2B (canonical):** all 46 live tests passed in run
  `20261026T225333Z-hosted-980838` at `8cd5040`
  (`docs/verification/runs/`).
- **sandbawks (`sb-ghlw.gfd.dev`, 0.1.28):** 25 of 46 passed. Every failure
  either passed on hosted E2B and is filed as a sandbawks defect
  (gfd/sandbawks #674-#682, plus #665), or was caused by cluster capacity.
  None is a crate bug. Details and request/response evidence are in
  `docs/verification/m1-live-results.md`.
- **Hosted E2B, M2:** all 57 live tests (M1's 46 plus 11 for M2) passed in
  run `20261027T125515Z-hosted-1922536` at `9910325`.
- **sandbawks, M2:** 14 of 57 passed in run
  `20261027T130108Z-sandbawks-2101151` at `0b9494b`, and 29 after single-test
  reruns. Every remaining failure passed on hosted E2B and is a sandbawks
  defect. Most are the M1 defects still open (#665, #674 to #682, and #667,
  where `allowInternetAccess: false` no longer blocks the internet). The M2
  calls add these: StreamInput and the watcher RPCs are not routed (#664), batch
  multipart uploads are refused (#684), large uploads hit an nginx body limit
  (#685), large downloads fail (#686), `Stat` omits owner and group (#687),
  and sandboxes have no envd access token, so their file API takes
  unauthenticated requests (#683). Details are in
  `docs/verification/m2-live-results.md`.
- **Hosted E2B, M3:** 72 of 75 live tests (M1's and M2's 57, all passing,
  plus 18 for M3) passed in run `20261028T012033Z-hosted-2696474` at
  `ebccc15`. The 3 failures are the volume tests, stopped by an environment
  limit: `POST /volumes` answers 403 `use of volumes is not enabled` for the
  team.
- **sandbawks, M3:** 16 of 75 passed in run
  `20261028T012919Z-sandbawks-2784597` at `6871dbf`, and 30 after reruns
  120 s apart. M1's and M2's tests give the same 29 passes as in M2. Of the
  M3 tests only the missing-volume test passes, and only because no volume
  route exists. The rest hit routes sandbawks does not serve yet (metrics,
  snapshots, fork, network update and volumes, which answer a bare 404, read
  as `SandboxNotFound` on a sandbox route), create fields it ignores
  (`network`, `allowInternetAccess`, lifecycle; #665, #667), an unknown
  `mcp-gateway` template (#680), the template's missing `/home/user` (#688),
  or list parameters it ignores (#689). Details are in
  `docs/verification/m3-live-results.md`.

Findings that affect how the crate talks to servers:

- **Hosted E2B and `allow_internet_access(false)`.** Outbound TCP handshakes
  still complete, but no data flows. A probe for this setting must send a
  request and check for a response, not just open a connection.
- **sandbawks and v2 connect.** The v2 connect route has been served since
  sandbawks 0.1.28. The v1 connect fallback was therefore removed in M4
  (Task 1): a v2 404 is the sandbox missing.
- **sandbawks and envd version gates.** sandbawks reports its release number
  as `envdVersion` (gfd/sandbawks#675), so envd version gates such as
  `close_stdin`, the default user, octet-stream and gzip uploads, and file
  metadata take their old-envd paths there.
- **Unsecured sandboxes.** Without an `envdAccessToken`, `upload_url` and
  `download_url` return unsigned URLs and refuse an expiration, as the SDK
  does. A server that hands out no token must then guard `/files` some other
  way; sandbawks currently does not (gfd/sandbawks#683).

