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
| Create body | `templateID`, `timeout`, `autoPause`, `metadata`, `envVars`, `allow_internet_access` (snake_case in the spec) | `py: api/client/models/new_sandbox_v2.py:112-132, 145-171` |
| Created sandbox | `templateID`, `sandboxID`, `clientID`, `envdVersion`, `envdAccessToken?`, `trafficAccessToken?`, `domain?` | `py: api/client/models/sandbox.py:85-115` |
| Connect / resume | `POST /v2/sandboxes/{id}/connect` body `{timeout?}`, 200 or 201 → `Sandbox` | `py: api/client/api/sandboxes/post_v_2_sandboxes_sandbox_id_connect.py:22-23, 37-44`; `py: api/client/models/connect_sandbox_v2.py:25`; `py: sandbox_async/sandbox_api.py:511-520`; `js: 5617` |
| Connect v1 (fallback) | `POST /sandboxes/{id}/connect`, body `{timeout}` required | `py: api/client/api/sandboxes/post_sandboxes_sandbox_id_connect.py:22-23`; `py: api/client/models/connect_sandbox.py:24, 48` |
| Default sandbox timeout 300 s | | `py: sandbox/main.py:30` |
| Get | `GET /sandboxes/{id}` → `SandboxDetail`; 404 → not found | `py: api/client/api/sandboxes/get_sandboxes_sandbox_id.py:17-18`; `py: sandbox_async/sandbox_api.py:118-135`; `js: 5289` |
| Detail fields | `startedAt`, `endAt`, `cpuCount`, `memoryMB`, `diskSizeMB`, `state`, `metadata`, `domain`, ... | `py: api/client/models/sandbox_detail.py:174-218` |
| State values | `running`, `paused` | `py: api/client/models/sandbox_state.py:5-6` |
| List | `GET /v2/sandboxes?metadata&state&limit&nextToken` → `ListedSandbox[]` | `py: api/client/api/sandboxes/get_v2_sandboxes.py:28-61`; `py: api/client/models/listed_sandbox.py:115-137`; `js: 5662` |
| List paging | next page token in response header `x-next-token` | `py: paginator.py:48-50` |
| List `state` | comma-joined | `py: api/client/api/sandboxes/get_v2_sandboxes.py:31-38`; `js: 1529-1532` (`explode: false`) |
| List `metadata` | each key and value `encodeURIComponent`-ed, then form-encoded into one string | `py: sandbox_async/paginator.py:50-57`; `js: 5656-5658` |
| Kill | `DELETE /sandboxes/{id}`, 204; 404 → `false` | `py: sandbox_async/sandbox_api.py:138-161`; `js: 5267` |
| Pause | `POST /sandboxes/{id}/pause`, body `{}` when no memory mode; 204; 404 → not found; 409 → already paused (`false`) | `py: sandbox_async/sandbox_api.py:399-432`; `py: sandbox/sandbox_api.py:639-662`; `js: 5442-5450` |
| Set timeout | `POST /sandboxes/{id}/timeout` body `{timeout}` (seconds); 404 → not found | `py: api/client/models/sandbox_timeout_request.py:17`; `py: sandbox_async/sandbox_api.py:164-187`; `js: 5396-5398` |
| Refresh | `POST /sandboxes/{id}/refreshes` body `{duration?}` (seconds), 204 | `py: api/client/api/sandboxes/post_sandboxes_sandbox_id_refreshes.py:21-38`; `py: api/client/models/sandbox_refresh_request.py:19` |
| Error body | `{code: int, message: string}`; the SDK surfaces `message` | `py: api/client/models/error.py:26-28`; `py: api/__init__.py:191-213` |

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
| `StartRequest` | `process`, `tag?`, `stdin?` | 217-238 |
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
| Write (default encoding): `POST /files?path=<p>[&username=<u>]`, `multipart/form-data`, field `file`, filename = path; response is a JSON array of `{name, type, path}` | `js: 2226-2249`; `py: sandbox_async/filesystem/filesystem.py:445-462`; `js: index.d.ts:4567-4580` |
| envd HTTP errors carry `{"message"}`; 400/401/404/429/502/507 are distinguished | `py: envd/api.py:24-33, 143-152` |
| `list` depth must be ≥ 1 | `py: sandbox_async/filesystem/filesystem.py:508-513` |
| `exists` is `Stat` with `not_found` → `false` | `py: sandbox_async/filesystem/filesystem.py:530-560` |
| `make_dir` returns `false` on `already_exists` | `py: sandbox_async/filesystem/filesystem.py:647-676` |
| `rename` is `Move` and returns the new entry | `py: sandbox_async/filesystem/filesystem.py:614-645` |
| Filesystem RPCs send the user via the `Authorization` header | `py: sandbox_async/filesystem/filesystem.py:517, 551, 583, 609, 640, 668` |
| `is_running`: `GET /health`, 502 → not running | `py: sandbox_async/main.py:134-168` |

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
- **Not implemented in v0.1:** PTY, `StreamInput`, `Update`, directory
  watching, signed file URLs, octet-stream and gzip uploads, sandbox health
  probing on transport errors, metrics, snapshots, fork, network updates,
  template builds, and the sync API.

## Verification against a live server

`https://sb-ghlw.gfd.dev` (a self-hosted E2B-compatible server) was probed
without credentials only, on 2026-10-26:

- `GET /health` without a sandbox ID answers 400 `missing_sandbox_id`, asking
  for `E2b-Sandbox-Id` or a `{port}-{sandbox_id}.{domain}` host: the same two
  addressing modes the SDKs use.
- `GET https://49983-x.sb-ghlw.gfd.dev/health` answers 404 "x not found", so the
  per-sandbox host form routes.
- `POST /v2/sandboxes`, `GET /v2/sandboxes`, `POST /sandboxes/x/connect`,
  `POST /sandboxes/x/refreshes` answer 401 (route exists, needs a key);
  `POST /v2/sandboxes/x/connect` answers 404 with an empty body (no v2
  connect route), which is what motivated the v1 fallback above.

No authenticated call was made; the integration tests have not been run.
