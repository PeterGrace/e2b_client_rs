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
| Create body | `templateID`, `timeout`, `autoPause`, `metadata`, `envVars`, `allow_internet_access` (snake_case in the spec), `network` (see "Network") | `py: api/client/models/new_sandbox_v2.py:112-132, 145-171`; `py: sandbox_async/sandbox_api.py:237-251` |
| Created sandbox | `templateID`, `sandboxID`, `clientID`, `envdVersion`, `envdAccessToken?`, `trafficAccessToken?`, `domain?` | `py: api/client/models/sandbox.py:85-115` |
| Connect / resume | `POST /v2/sandboxes/{id}/connect` body `{timeout?}`, 200 or 201 → `Sandbox` | `py: api/client/api/sandboxes/post_v_2_sandboxes_sandbox_id_connect.py:22-23, 37-44`; `py: api/client/models/connect_sandbox_v2.py:25`; `py: sandbox_async/sandbox_api.py:511-520`; `js: 5617` |
| Connect v1 (fallback) | `POST /sandboxes/{id}/connect`, body `{timeout}` required | `py: api/client/api/sandboxes/post_sandboxes_sandbox_id_connect.py:22-23`; `py: api/client/models/connect_sandbox.py:24, 48` |
| Default sandbox timeout 300 s | | `py: sandbox/main.py:30` |
| Get | `GET /sandboxes/{id}` → `SandboxDetail`; 404 → not found | `py: api/client/api/sandboxes/get_sandboxes_sandbox_id.py:17-18`; `py: sandbox_async/sandbox_api.py:118-135`; `js: 5289` |
| Detail fields | `startedAt`, `endAt`, `cpuCount`, `memoryMB`, `diskSizeMB`, `state`, `metadata`, `domain`, ... | `py: api/client/models/sandbox_detail.py:174-218` |
| State values | `running`, `paused` | `py: api/client/models/sandbox_state.py:5-6` |
| Metrics | `GET /sandboxes/{id}/metrics?start&end`, both Unix seconds, each omitted when unset; 200 → `SandboxMetric[]`; 404 → not found | `py: api/client/api/sandboxes/get_sandboxes_sandbox_id_metrics.py:21-29, 39-60`; `py: sandbox_async/sandbox_api.py:280-336`; `js: 5346-5378` |
| Metric fields | `timestamp` (RFC 3339), `timestampUnix`, `cpuCount`, `cpuUsedPct` (float), `memUsed`, `memTotal`, `memCache`, `diskUsed`, `diskTotal` (bytes) | `py: api/client/models/sandbox_metric.py:28-36, 79-95` |
| List | `GET /v2/sandboxes?metadata&state&order&startedAfter&template&limit&nextToken` → `ListedSandbox[]`; `order` is `asc` or `desc` (by start time), `startedAfter` is `datetime.isoformat()` of an aware time (`2026-10-27T12:00:00+00:00`, or `...12:00:00.123456+00:00` when the microseconds are not zero; the crate formats it the same way and truncates nanoseconds, which Python cannot hold, rather than using chrono's `to_rfc3339`, which writes 3, 6 or 9 digits), `template` is a template ID or name; each filter is omitted when unset. Hosted E2B filters by the template ID that `get_info` reports (live, 2026-10-27) | `py: api/client/api/sandboxes/get_v2_sandboxes.py:14-61`; `py: api/client/models/listed_sandbox.py:115-137`; `py: sandbox/sandbox_api.py:1320-1340`; `py: sandbox_async/paginator.py:55-90`; `js: 5662` |
| List paging | next page token in response header `x-next-token` | `py: paginator.py:48-50` |
| List `state` | comma-joined | `py: api/client/api/sandboxes/get_v2_sandboxes.py:31-38`; `js: 1529-1532` (`explode: false`) |
| List `metadata` | each key and value `encodeURIComponent`-ed, then form-encoded into one string | `py: sandbox_async/paginator.py:50-57`; `js: 5656-5658` |
| Kill | `DELETE /sandboxes/{id}`, 204; 404 → `false` | `py: sandbox_async/sandbox_api.py:138-161`; `js: 5267` |
| Pause | `POST /sandboxes/{id}/pause`, body `{}` when no memory mode; 204; 404 → not found; 409 → already paused (`false`) | `py: sandbox_async/sandbox_api.py:399-432`; `py: sandbox/sandbox_api.py:639-662`; `js: 5442-5450` |
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
| Update: `PUT /sandboxes/{id}/network`, 204 → done; 404 → not found; other statuses are errors | `py: api/client/api/sandboxes/put_sandboxes_sandbox_id_network.py:21-55`; `py: sandbox_async/sandbox_api.py:190-209`; `js: 5420` |
| Update body: `allowOut`, `denyOut`, `dnsServers`, `egressProxy`, `rules` in camelCase, and `allow_internet_access` in snake_case; each is sent only when set, and an update with nothing set is sent as `{}` (not left out, as an empty create `network` is). The crate does not send `egressProxy` (deferred) | `py: api/client/models/sandbox_network_update_config.py:50-103`; `py: sandbox/sandbox_api.py:1128-1152` |
| The update replaces the whole egress configuration: a field it leaves out is cleared, so `dnsServers` goes back to `8.8.8.8` and an omitted `rules` or `egressProxy` is dropped. The sandbox is not restarted | `py: sandbox/sandbox_api.py:515-561` |
| Hosted E2B applies an update at once, without a restart: an update with only `allow_internet_access: true` clears a create-time `denyOut: ["0.0.0.0/0"]`, and `denyOut: ["0.0.0.0/0"]` closes egress again, each seen by the first probe after the 204, with a tmpfs file kept | live, hosted E2B, 2026-10-27 (`clm_0144`) |

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
| Timeouts sent as whole ms, at least 1 | `py: envd/utils.py:15-24` |
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
| Stream deadline defaults to 60 s (`STREAM_INPUT_TIMEOUT`), sent as `Connect-Timeout-Ms`; `None` disables it | `py: connection_config.py:33`; `py: sandbox_async/commands/command.py:150-174` |
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
| Headers: the user header and `Keepalive-Ping-Interval: 50`; the watch timeout (default 60 s; the SDK's `0`/`None` for none is only `None` here, since `Some(Duration::ZERO)` is sent as `Connect-Timeout-Ms: 1` and ends the watch at once, as for the other options structs) is sent as `Connect-Timeout-Ms` and bounds the whole stream | `py: sandbox_async/filesystem/filesystem.py:685, 723-740` |
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

## Deliberate differences from the SDKs

- **Connect v1 fallback.** If `POST /v2/sandboxes/{id}/connect` answers 404,
  the client retries the v1 route `POST /sandboxes/{id}/connect` (still in the
  E2B OpenAPI spec the SDK is generated from) before reporting the sandbox
  missing. Servers that implement only the v1 route keep working; against
  hosted E2B a missing sandbox costs one extra request.
- **Non-zero exit is not an error.** The SDKs raise `CommandExitException`
  from `run`; this crate returns `CommandResult` with `exit_code` and a
  `success()` helper, which is the idiomatic Rust shape.
- **`wait()` does not replay consumed output.** The SDK handle buffers all
  output; here `next_event()` hands chunks to the caller and `wait()` only
  collects what is left, so output is held once.
- **Not implemented yet.** Milestone 3: snapshots and fork, network
  updates, and the remaining SDK 2.61 create fields. Milestone 4:
  the `is_symlink` file type (#30) and dropping the v1 connect fallback.
  Unscheduled: sandbox health probing on transport errors. Out of scope by
  design: template builds and a sync API.

## Verification against a live server

Every v0.1 behaviour was verified live on 2026-10-26 by the M1 suite, and the
M2 behaviours on 2026-10-27 (`tests/live_*.rs`, `just test-live <target>`). Each test is named after the
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

Findings that affect how the crate talks to servers:

- **Hosted E2B and `allow_internet_access(false)`.** Outbound TCP handshakes
  still complete, but no data flows. A probe for this setting must send a
  request and check for a response, not just open a connection.
- **sandbawks and v2 connect.** The v2 connect route has been served since
  sandbawks 0.1.28. The v1 connect fallback (above) is therefore no longer
  needed by any known server, and M4 removes it.
- **sandbawks and envd version gates.** sandbawks reports its release number
  as `envdVersion` (gfd/sandbawks#675), so envd version gates such as
  `close_stdin`, the default user, octet-stream and gzip uploads, and file
  metadata take their old-envd paths there.
- **Unsecured sandboxes.** Without an `envdAccessToken`, `upload_url` and
  `download_url` return unsigned URLs and refuse an expiration, as the SDK
  does. A server that hands out no token must then guard `/files` some other
  way; sandbawks currently does not (gfd/sandbawks#683).

