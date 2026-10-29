# Changelog

All notable changes to `e2b-client` are listed here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the crate
follows [Semantic Versioning](https://semver.org) (README, "Versioning").

## [0.2.0] - 2026-10-29

The first release on crates.io. It covers the official SDK's `Sandbox`
surface (e2b 2.61) except what the README lists as "Not implemented yet",
and is live-verified on hosted E2B (volumes excepted: they are not enabled
for the test team).

### Added

- Streamed stdin: `Commands::stream_input` returns an `InputStream`
  (`send`, `close`), bounded by `StreamInputOptions` (M2, #31).
- PTYs: `Sandbox::pty()`, with `Pty::{create, connect, send_stdin,
  stream_input, resize, kill}` and a `PtyHandle` with its own output stream
  (M2, #32).
- `Filesystem::watch_dir` with `WatchOptions` (recursive, include_entry,
  allow_network_mounts) and `WatchHandle::next_event` (M2, #33).
- Batch writes: `Filesystem::write_files` and `WriteOptions` (multipart,
  octet-stream and gzip, with the envd 0.5.7 fallback) (M2, #34).
- Streamed files: `Filesystem::write_stream` from any `AsyncRead`, and
  `read_stream` returning a `FileStream` of chunks (M2, #35).
- File metadata: `WriteOptions::metadata` and `metadata` on `WriteInfo`
  and `EntryInfo` (envd 0.6.2+) (M2, #36).
- Signed URLs: `Sandbox::{upload_url, download_url}` with `FileUrlOptions`
  (M2, #37).
- `Sandbox::get_host(port)` (M3, #60).
- Metrics: `Sandbox::get_metrics` and `Client::get_sandbox_metrics` with
  `MetricsParams`, returning `SandboxMetrics` (M3, #61).
- List filters: `ListSandboxesParams::{template, started_after, order}`
  and `SortOrder` (M3, #62).
- Network rules at create: `CreateSandboxParams::network(NetworkConfig)`,
  `ALL_TRAFFIC`, and `SandboxInfo::{allow_internet_access, network}`
  (M3, #63); updates with `Sandbox::update_network` and
  `Client::update_sandbox_network` (`NetworkUpdate`) (M3, #64).
- Lifecycle and idempotency: `CreateSandboxParams::{lifecycle,
  idempotency_key}` with `Lifecycle` (`OnTimeout`, `SnapshotMode`,
  `auto_resume`), and `SandboxInfo::{lifecycle, pause_mode}` (M3, #65).
- Pause and resume modes: `PauseOptions` and `ConnectOptions` with
  `OnResume::{Restore, Reboot}` (M3, #66).
- Snapshots: `Sandbox::{create_snapshot, list_snapshots}` and
  `Client::{create_snapshot, list_snapshots, delete_snapshot}`; a snapshot
  ID starts new sandboxes as a template (M3, #67).
- Fork: `Sandbox::fork` and `Client::fork_sandbox` with `ForkOptions`
  (1 to `MAX_FORK_COUNT` forks) (M3, #68).
- Volumes: `Client::{create_volume, get_volume, list_volumes,
  destroy_volume}`, `CreateSandboxParams::volume_mount` and
  `SandboxInfo::volume_mounts` (M3, #69).
- The MCP gateway: `CreateSandboxParams::mcp` and `Sandbox::{mcp_url,
  mcp_token}` (M3, #70).
- Error classifiers: `Error::status()`, `is_auth`, `is_rate_limited`,
  `is_service_busy`, `is_not_enough_space`, `is_sandbox_unreachable`,
  `is_timeout` and `is_retryable`; an opaque `TransportError`; the
  `secrecy` re-export (M4, #92).
- Options structs: `FileOptions` and `ListOptions` on every filesystem call
  (M4, #93), `CommandConnectOptions` and `PtyConnectOptions` (M4, #54), and
  `new()` on every options type (M4, #95).
- Per-call request timeouts: a `request_timeout` setter on `WriteOptions`,
  `WatchOptions`, `CommandOptions` and `PtyOptions` (M4, #93, #54), and on
  `CreateSandboxParams`, `ConnectOptions`, `ListSandboxesParams`,
  `MetricsParams`, `PauseOptions`, `SnapshotOptions`, `ListSnapshotsParams`
  and `ForkOptions` (M5, #125).
- `WriteOptions::max_concurrency` and `DEFAULT_UPLOAD_CONCURRENCY` (8)
  (M4, #53).
- `RequestOptions`, a per-call request timeout for the REST and pid-only
  process and PTY calls that take no other options (M5, #125, #120).

### Changed

- An explicit empty config value counts as unset (M1, #18).
- `Commands::close_stdin` is refused before envd 0.5.2 (M1, #19).
- A zero timeout or deadline means no limit, as in the SDKs (M4, #52).
- A PTY has no deadline by default; `DEFAULT_PTY_TIMEOUT` stays the SDKs'
  60 s (M4, #55).
- At most 8 octet-stream or gzip uploads are in flight at once (M4, #53).
- A symlink is reported as `FileType::Symlink` on every envd version
  (M4, #30).
- A pause with a mode that the server refuses with a 409 other than
  "already paused" is `Error::Api`, not `Ok(false)` (M4, #81).
- `CommandEvent` and `Signal` are `#[non_exhaustive]`; `Error::Decode`'s
  and `Error::Transport`'s detail is in `source()` (M4, #95, #92).
- Minimum supported Rust version 1.89 (M4, #96).

### Removed

- The v1 connect fallback: connect makes one request,
  `POST /v2/sandboxes/{id}/connect` (M4, #91).
- The `_with` twins: `Client::connect_sandbox_with`,
  `Client::pause_sandbox_with` and `Sandbox::pause_with`; the plain methods
  take the options (M4, #94).
- `impl From<reqwest::Error> for Error`: no reqwest type is public
  (M4, #92).

### Breaking changes since 0.1.0

Old call to new call. `(after 0.1.0)` marks a change to an API added in
M2 or M3, so it breaks code written against `main` between the two
releases, not against 0.1.0.

#### Lifecycle (REST)

| Old | New | Source |
|---|---|---|
| `client.connect_sandbox(id, None)` | `client.connect_sandbox(id, &ConnectOptions::new())` | M4 Task 6 (#94) |
| `client.connect_sandbox(id, Some(t))` | `client.connect_sandbox(id, &ConnectOptions::new().timeout(t))` | M4 Task 6 (#94) |
| `client.connect_sandbox_with(id, &o)` | `client.connect_sandbox(id, &o)` | M4 Task 6 (#94), (after 0.1.0) |
| `client.pause_sandbox(id)` | `client.pause_sandbox(id, &PauseOptions::new())` | M4 Task 6 (#94) |
| `client.pause_sandbox_with(id, &o)` | `client.pause_sandbox(id, &o)` | M4 Task 6 (#94), (after 0.1.0) |
| `sandbox.pause()` | `sandbox.pause(&PauseOptions::new())` | M4 Task 6 (#94) |
| `sandbox.pause_with(&o)` | `sandbox.pause(&o)` | M4 Task 6 (#94), (after 0.1.0) |
| `client.get_sandbox_info(id)` | `client.get_sandbox_info(id, &RequestOptions::new())` | M5 Task 1 (#125) |
| `client.kill_sandbox(id)` | `client.kill_sandbox(id, &RequestOptions::new())` | M5 Task 1 (#125) |
| `client.set_sandbox_timeout(id, d)` | `client.set_sandbox_timeout(id, d, &RequestOptions::new())` | M5 Task 1 (#125) |
| `client.update_sandbox_network(id, &update)` | `client.update_sandbox_network(id, &update, &RequestOptions::new())` | M5 Task 1 (#125), (after 0.1.0) |
| `client.refresh_sandbox(id, d)` | `client.refresh_sandbox(id, d, &RequestOptions::new())` | M5 Task 1 (#125) |
| `client.delete_snapshot(id)` | `client.delete_snapshot(id, &RequestOptions::new())` | M5 Task 1 (#125), (after 0.1.0) |
| `client.create_volume(name)` | `client.create_volume(name, &RequestOptions::new())` | M5 Task 1 (#125), (after 0.1.0) |
| `client.get_volume(id)` | `client.get_volume(id, &RequestOptions::new())` | M5 Task 1 (#125), (after 0.1.0) |
| `client.list_volumes()` | `client.list_volumes(&RequestOptions::new())` | M5 Task 1 (#125), (after 0.1.0) |
| `client.destroy_volume(id)` | `client.destroy_volume(id, &RequestOptions::new())` | M5 Task 1 (#125), (after 0.1.0) |
| `sandbox.is_running()` | `sandbox.is_running(&RequestOptions::new())` | M5 Task 1 (#125) |
| `sandbox.kill()` | `sandbox.kill(&RequestOptions::new())` | M5 Task 1 (#125) |
| `sandbox.set_timeout(d)` | `sandbox.set_timeout(d, &RequestOptions::new())` | M5 Task 1 (#125) |
| `sandbox.update_network(&update)` | `sandbox.update_network(&update, &RequestOptions::new())` | M5 Task 1 (#125), (after 0.1.0) |
| `sandbox.refresh(d)` | `sandbox.refresh(d, &RequestOptions::new())` | M5 Task 1 (#125) |
| `sandbox.get_info()` | `sandbox.get_info(&RequestOptions::new())` | M5 Task 1 (#125) |

`OnTimeout::pause_with(SnapshotMode)` is a different item and stays.
`create_sandbox`, `list_sandboxes`, `get_sandbox_metrics` /
`Sandbox::get_metrics`, `create_snapshot`, `list_snapshots` and
`fork_sandbox` / `Sandbox::fork` keep their signatures; their structs gain
the `request_timeout` setter.

Behaviour, with no signature change:

- Against a server that serves only the v1 connect route,
  `connect_sandbox` returns `Error::SandboxNotFound` where it used to
  connect through `POST /sandboxes/{id}/connect`. Without a timeout,
  connect no longer resets the lifetime to 300 s (a v1-only behaviour).
  M4 Task 1 (#91).
- A pause with a mode (`&PauseOptions::new().mode(..)`) returns
  `Err(Error::Api { status: 409, .. })` for a 409 that does not say
  "already paused", where it returned `Ok(false)`. A pause without a mode
  still reads every 409 as `Ok(false)`, as SDK 2.61 does. M4 Task 3 (#81),
  (after 0.1.0).

#### Errors

| Old | New | Source |
|---|---|---|
| `Error::Transport(reqwest::Error)` | `Error::Transport(TransportError)`; use `TransportError::{is_timeout, is_connect}` or `Error::{is_timeout, is_retryable}` | M4 Task 2 (#92) |
| `?` on a `reqwest::Error` into `Error` (`impl From<reqwest::Error> for Error`) | Gone; no reqwest type is public | M4 Task 2 (#92) |
| `Error::Api { status, message }` (also `Rpc`, `Envd`) in a pattern | `Error::Api { status, message, .. }`: the three variants are `#[non_exhaustive]` and cannot be built outside the crate | M4 Task 2 (#92) |
| `{err}` of `Error::Transport` showed the transport's message | `HTTP transport error`; the detail is `source()` (print the chain, `{:#}` in anyhow) | M4 Task 2 (#92) |
| `{err}` of `Error::Decode` was `could not decode response: <serde_json message>` | `could not decode response`; the detail is `source()` | M4 Task 12 (#95) |

#### Processes and PTY (envd)

| Old | New | Source |
|---|---|---|
| `commands.connect(pid, Some(d))` | `commands.connect(pid, &CommandConnectOptions::new().timeout(Some(d)))` | M4 Task 7 (#54) |
| `commands.connect(pid, None)` | `commands.connect(pid, &CommandConnectOptions::new().timeout(None))`; `&CommandConnectOptions::default()` is 60 s, the SDKs' value | M4 Task 7 (#54) |
| `pty.connect(pid, t)` | `pty.connect(pid, &PtyConnectOptions::new().timeout(t))` | M4 Task 7 (#54), (after 0.1.0) |
| `PtyOptions::default()` bounded the session to 60 s | No deadline; for the old behaviour, `PtyOptions::default().timeout(Some(DEFAULT_PTY_TIMEOUT))` | M4 Task 8 (#55), (after 0.1.0) |
| A `match` on `CommandEvent` or `Signal` listing every variant | Needs a `_` (or `other => ...`) arm: both are `#[non_exhaustive]` | M4 Task 12 (#95) |
| `commands.list()` | `commands.list(&RequestOptions::new())` | M5 Task 2 (#120) |
| `commands.send_stdin(pid, data)` | `commands.send_stdin(pid, data, &RequestOptions::new())` | M5 Task 2 (#120) |
| `commands.close_stdin(pid)` | `commands.close_stdin(pid, &RequestOptions::new())` | M5 Task 2 (#120) |
| `commands.send_signal(pid, signal)` | `commands.send_signal(pid, signal, &RequestOptions::new())` | M5 Task 2 (#120) |
| `commands.kill(pid)` | `commands.kill(pid, &RequestOptions::new())` | M5 Task 2 (#120) |
| `handle.send_stdin(data)` (`CommandHandle`) | `handle.send_stdin(data, &RequestOptions::new())` | M5 Task 2 (#120) |
| `handle.kill()` (`CommandHandle`) | `handle.kill(&RequestOptions::new())` | M5 Task 2 (#120) |
| `pty.send_stdin(pid, data)` | `pty.send_stdin(pid, data, &RequestOptions::new())` | M5 Task 2 (#120), (after 0.1.0) |
| `pty.resize(pid, size)` | `pty.resize(pid, size, &RequestOptions::new())` | M5 Task 2 (#120), (after 0.1.0) |
| `pty.kill(pid)` | `pty.kill(pid, &RequestOptions::new())` | M5 Task 2 (#120), (after 0.1.0) |
| `handle.send_stdin(data)` (`PtyHandle`) | `handle.send_stdin(data, &RequestOptions::new())` | M5 Task 2 (#120), (after 0.1.0) |
| `handle.resize(size)` (`PtyHandle`) | `handle.resize(size, &RequestOptions::new())` | M5 Task 2 (#120), (after 0.1.0) |
| `handle.kill()` (`PtyHandle`) | `handle.kill(&RequestOptions::new())` | M5 Task 2 (#120), (after 0.1.0) |

`stream_input` keeps its `&StreamInputOptions` (its bound is a stream
deadline, not a request timeout).

#### Files (envd)

| Old | New | Source |
|---|---|---|
| `files.read(p, None)` (also `read_to_string`, `read_stream`, `get_info`, `exists`, `make_dir`, `remove`) | `files.read(p, &FileOptions::default())` | M4 Task 5 (#93) |
| `files.read(p, Some("root"))` (and the same calls) | `files.read(p, &FileOptions::new().user("root"))` | M4 Task 5 (#93) |
| `files.rename(a, b, None)` / `files.rename(a, b, Some("root"))` | `files.rename(a, b, &FileOptions::default())` / `files.rename(a, b, &FileOptions::new().user("root"))` | M4 Task 5 (#93) |
| `files.write(p, data, None)` | `files.write(p, data, &WriteOptions::default())` | M4 Task 5 (#93) |
| `files.write(p, data, Some("root"))` | `files.write(p, data, &WriteOptions::default().user("root"))`; `write` now honours every `WriteOptions` field (octet-stream, gzip, metadata) | M4 Task 5 (#93) |
| `files.list(p, 1, None)` | `files.list(p, &ListOptions::default())` | M4 Task 5 (#93) |
| `files.list(p, depth, Some("root"))` | `files.list(p, &ListOptions::new().depth(depth).user("root"))` | M4 Task 5 (#93) |

`write_files`, `write_stream` and `watch_dir` keep their shape.
(`read_stream` was added after 0.1.0.)

Behaviour, with no signature change:

- A batch of more than 8 octet-stream or gzip files no longer starts every
  upload at once; `max_concurrency(entries.len())` restores it. M4 Task 9
  (#53), (after 0.1.0).
- On newer envd a symlink reported its target's type (`File` or
  `Directory`); it now reports `FileType::Symlink`. Read `symlink_target`
  or stat the target to follow it. M4 Task 10 (#30).

#### Timeouts

- `Some(Duration::ZERO)` passed to `CommandOptions::timeout`,
  `PtyOptions::timeout`, `WatchOptions::timeout`,
  `StreamInputOptions::timeout`, `ConnectionConfigBuilder::request_timeout`,
  any `request_timeout` setter, or the `timeout` of `CommandConnectOptions`
  and `PtyConnectOptions` was a deadline that had already passed. It is now
  no limit. Code that used zero to fail at once uses
  `Some(Duration::from_millis(1))`. M4 Task 4 (#52).

#### Toolchain

- Rust 1.89 or newer (`rust-version = "1.89"`). Builds on older toolchains
  already failed on `uuid@1.29.0`; cargo now names this crate first.
  M4 Task 13 (#96).

## [0.1.0] - 2026-10-26

Git only, never published: the REST sandbox lifecycle and envd's process
and filesystem services (PR #2).

[0.2.0]: https://github.com/PeterGrace/e2b_client_rs/compare/9e9bbd91aa2a9402fd4ecf94d32ce03f4f516fc0...v0.2.0
[0.1.0]: https://github.com/PeterGrace/e2b_client_rs/tree/9e9bbd91aa2a9402fd4ecf94d32ce03f4f516fc0
