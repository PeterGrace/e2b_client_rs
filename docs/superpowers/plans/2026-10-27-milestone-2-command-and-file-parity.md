# Milestone 2 (Command and file parity) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the S2 and S3 gaps between `e2b-client` and the official SDK's `Commands`, `Pty` and `Filesystem` surface: streamed stdin, PTY sessions, batch writes, directory watching, signed file URLs, streamed and gzip uploads, streamed downloads and file metadata. Each one is backed by mock wire tests and a live test on hosted E2B, and has a verdict in the claims ledger.

**Architecture:** Two new transport primitives in `src/envd/mod.rs`: a client-streaming Connect call (`client_stream`, used by `StreamInput`) and a shared "wait for the opening event within the request timeout" helper. Everything else is built on primitives the crate already has: `server_stream` (PTY, `WatchDir`), `unary` (PTY input, resize) and `http_request` (the `/files` API). New public modules: `src/stream_input.rs` (`InputStream`) and `src/pty.rs` (`Pty`, `PtyHandle`). Batch, streamed, gzip and metadata uploads all go through one `WriteOptions` and one upload path in `src/filesystem.rs`. Signed URLs live on `Sandbox`, as in the SDK. Each feature task adds its mock tests in a test file of its own, its live tests to `tests/live_commands.rs` or `tests/live_files.rs`, and its wire facts to `docs/protocol-notes.md` and `docs/verification/s4-evidence-map.md`.

**Tech Stack:** Rust 2024, tokio (`time`, `rt`, `sync`, `io-util`), reqwest 0.13 (`stream`), futures-util, tokio-util (`io`), async-compression (`tokio`, `gzip`), sha2, wiremock, `just`, claims MCP.

**Design:** `docs/superpowers/specs/2026-10-26-claims-audit-and-publishability-design.md`, sections "Claim hierarchy" (S2, S3, S4) and "Milestones" (M2). The spec says each milestone gets "its own short design and plan"; for M2 the design is the "Decisions" and "Verified facts" sections below, since every feature copies an SDK behaviour that is cited there.

---

## Scope

In:

- **Streamed stdin** (CLM-0046): `Commands::stream_input(pid, &StreamInputOptions) -> InputStream` over envd's client-streaming `StreamInput` RPC, with the SDK's 50 s keepalive and 60 s default deadline.
- **PTY** (CLM-0047, 0048, 0049): `Sandbox::pty()` with `create`, `connect`, `send_stdin`, `resize`, `kill` and `stream_input`, returning a `PtyHandle` whose events are `PtyEvent::{Data, Exit}`.
- **Batch write** (CLM-0062): `Filesystem::write_files(&[WriteEntry], &WriteOptions)`. `write` becomes a one-entry `write_files`.
- **Directory watching** (CLM-0063): `Filesystem::watch_dir(path, &WatchOptions) -> WatchHandle`, with `recursive`, `include_entry` and `allow_network_mounts` and their envd version gates.
- **Signed URLs** (CLM-0064, 0065): `Sandbox::upload_url` and `Sandbox::download_url`, with an optional expiration.
- **Streamed and gzip uploads** (CLM-0066): `Filesystem::write_stream(path, impl AsyncRead, &WriteOptions)`, plus the `octet_stream` and `gzip` options for every write. Both need envd ≥ 0.5.7 and fall back to multipart, as in the SDK.
- **Streamed download** (new, CLM-0128): `Filesystem::read_stream(path, user) -> FileStream`.
- **File metadata** (new, CLM-0129): `WriteOptions::metadata`, and a `metadata` map on `WriteInfo` and `EntryInfo` (envd ≥ 0.6.2).
- **S4 wire claims for all of the above** (CLM-0130 to CLM-0142), registered while planning.
- **Follow-up #26:** `just test-live` stamps the commit and a dirty-tree flag at the top of each run log (Task 1).
- Live runs on hosted E2B and sandbawks, verdicts and the milestone close.

Deferred:

| Item | Claims / issue | Goes to |
|---|---|---|
| get_host, metrics, snapshots, network, extra create fields | CLM-0028..0032 | M3 |
| `#[non_exhaustive]` review (including `CommandEvent`, `Signal`), missing_docs, MSRV/semver, dropping the v1 fallback, naming consistency of the `user: Option<&str>` arguments against the new options structs | CLM-0119..0121, 0087 | M4 |
| Live CI job, publish dry-run | CLM-0123, 0124 | M5 |
| Re-running the full sandbawks suite with cluster headroom (decides CLM-0118) | #27 | its own issue, not code |
| `EntryInfo` ignores envd's `is_symlink` field, so a symlink on newer envd shows its target's type (found while planning, see Verified facts) | new issue, filed with the epic | M4, or earlier if it bites |
| Per-call `request_timeout` overrides (the SDK takes one on most calls; the crate has one per `Client`) | none | M4 API review |
| Streamed download with `Accept-Encoding: gzip`, and the `stream_idle_timeout` knob | none | not planned |
| `Filesystem::watch_dir` callbacks (`on_event`, `on_exit`) | none | not planned: the handle's `next_event` loop is the Rust idiom |

## Decisions taken while planning (2026-10-27)

1. **Next milestone.** pgrace chose M2 before M3. M3 can run afterwards or in parallel, because its files barely overlap with M2's (`src/client.rs`, `src/models.rs`).
2. **PTY API.** A separate `PtyHandle` with `PtyEvent::{Data(Bytes), Exit(CommandExit)}`, not `CommandHandle` with a new `CommandEvent::Pty` variant. The types keep PTY output and stdout/stderr apart. It shares the stream-opening code with `CommandHandle`.
3. **Gzip.** Streamed uploads support both octet-stream and gzip, through `async-compression` (a new dependency), so CLM-0066 is covered whole.
4. **Extras beyond the gap claims.** Streamed download (CLM-0128) and file metadata on write (CLM-0129) are in. Both were registered as claims while planning.
5. **Follow-ups.** #26 is Task 1, so M2's own run logs carry the stamp. #27 stays out, because it needs cluster headroom, not code.
6. **Ledger.** As in M1, only the coordinator session writes `claims.json`. The 15 new claims (CLM-0128 to CLM-0142) were added while planning and are committed with this plan.
7. **Merging.** The M1 authorization stands: the coordinator merges the plan PR and the sub-issue PRs once CI is green and brooke-reviews' review has been answered.
8. **Execution order.** One issue at a time, oldest first (pgrace's standing rule since 2026-10-27). The dependency graph below shows what could run in parallel if pgrace asks for it.

## Simplifications and deviations relative to the SDK

- **Handles, not callbacks.** The SDK's `pty.create(on_data=...)` and `watch_dir(on_event=..., on_exit=...)` take callbacks. Here `PtyHandle::next_event` and `WatchHandle::next_event` are pulled in a `while let` loop, like `CommandHandle`. Dropping a `WatchHandle` stops the watch, which is the SDK's `stop()`.
- **Bounded input queue.** The SDK's input handle queues without limit. `InputStream::send` waits when 64 chunks are queued, so a fast producer can't buffer without bound.
- **The input deadline is a `StreamInputOptions` field** (default 60 s), like the other options structs, rather than the SDK's keyword argument.
- **`InputStream::send` is async and takes `&mut self`.** It is what reports a stream that already failed (the SDK raises the stored failure the same way).
- **`write` keeps its v0.1 signature** (`path, data, user`). The new options are on `write_files` and `write_stream`. M4 reviews whether `write` should take `&WriteOptions`.
- **Metadata is a `HashMap`, empty when none is set.** The SDK uses `None` for no metadata. An empty map is the Rust idiom and loses nothing.
- **Signed URL query encoding.** The URL is built with `url::Url::query_pairs_mut`, which form-encodes like the JS SDK's `URLSearchParams` (`/` becomes `%2F`, a space becomes `+`). The Python SDK uses `quote`, which leaves `/` alone and writes a space as `%20`. envd decodes both, and the signature is computed over the raw path, so neither choice changes it.
- **No streamed-download timeout knob.** `read_stream` sets no request timeout, which is the SDK's default. Callers who want a bound wrap `next_chunk` in `tokio::time::timeout`.

## Verified facts the code relies on

SDK sources are e2b 2.61.0, the same packages as `docs/protocol-notes.md` (`py:` paths relative to `e2b/`, `js:` lines of `package/dist/index.js`). Crate sources are under `~/.cargo/registry/src/index.crates.io-*/`.

| Fact | Source |
|---|---|
| `StreamInput` is a client-streaming RPC at `/process.Process/StreamInput`; `Update` is unary at `/process.Process/Update` | `py: envd/process/process_connect.py:110-128` |
| `StreamInputRequest` is a oneof `start {process: ProcessSelector}` / `data {input: ProcessInput}` / `keepalive {}`; the response is empty | `py: envd/process/process_pb.py:597-716` |
| The input handle sends `start` first, then one `data` per queued chunk, a `keepalive` after 50 s without input, and ends the request stream on `close()`, which then waits for the call to finish | `py: sandbox_async/commands/command_input_handle.py:71-83, 91-131`; `py: sandbox/commands/stream_input.py:10-37`; `js: 2781-2870` |
| Keepalive interval 50 s (`KEEPALIVE_PING_INTERVAL_SEC`), header `Keepalive-Ping-Interval` | `py: connection_config.py:48-49` |
| A stream's opening event is awaited within the request timeout, then only the stream's own deadline applies (the basis of `within_request_timeout`) | `py: envd/client_async/__init__.py:113-149` |
| Input stream timeout defaults to 60 s (`STREAM_INPUT_TIMEOUT`); `0`/`None` disables it | `py: connection_config.py:33`; `py: sandbox_async/commands/command.py:150-174` |
| SDKs gate `StreamInput` on no envd version | `py: sandbox_async/commands/command.py:150-174`; `py: envd/versions.py:1-11` (no constant for it) |
| `ProcessInput` is a oneof `stdin` / `pty` (bytes) | `py: envd/process/process_pb.py:569-596` |
| PTY create: `cmd "/bin/bash"`, `args ["-i", "-l"]`, `envs` with `TERM=xterm-256color`, `LANG=C.UTF-8`, `LC_ALL=C.UTF-8` set only when absent, `cwd`, `pty.size {cols, rows}`, no `stdin` field; user header and `Keepalive-Ping-Interval: 50`; default timeout 60 s | `py: sandbox_async/commands/pty.py:139-205`; `js: 4682-4712` |
| `PTY {size: PTY.Size {cols: uint32 = 1, rows: uint32 = 2}}`; `StartRequest.pty` is field 2, `stdin` is optional field 4 | `py: envd/process/process_pb.py:18-80, 212-260` |
| PTY output is `DataEvent.pty` bytes, next to `stdout`/`stderr` | `py: envd/process/process_pb.py:369-394`; `py: sandbox_async/commands/command_handle.py:169, 221-231` |
| PTY input is `SendInput` with `input.pty`; resize is `Update {process, pty: {size}}`; kill is the shared SIGKILL `kill_process`; PTY connect is `Connect` with the keepalive header | `py: sandbox_async/commands/pty.py:65-137, 207-283` |
| `WatchDirRequest {path = 1, recursive = 2, include_entry = 3, allow_network_mounts = 4}`; response oneof `start {}` / `filesystem FilesystemEvent` / `keepalive {}`; `FilesystemEvent {name = 1, type = 2, entry = 3?}` | `py: envd/filesystem/filesystem_pb.py:404-560` |
| `EventType`: `EVENT_TYPE_UNSPECIFIED = 0`, `CREATE = 1`, `WRITE = 2`, `REMOVE = 3`, `RENAME = 4`, `CHMOD = 5`; unknown types are dropped | `py: envd/filesystem/filesystem_pb.py:774-810`; `py: sandbox/filesystem/watch_handle.py:36-46`; `py: sandbox_async/filesystem/watch_handle.py:47-64` |
| `watch_dir` gates: recursive needs envd 0.1.4, include_entry 0.6.3, allow_network_mounts 0.6.4; checked before any request; the stream must open with `start`, awaited within the request timeout; default watch timeout 60 s; the user header and keepalive header are sent | `py: sandbox_async/filesystem/filesystem.py:678-766`; `py: envd/versions.py:3, 10, 11` |
| `write_files([])` returns `[]` without a request. Multipart: one POST, every part named `file` with filename = path, `path` query only for exactly one file. Octet-stream: one POST per file, `Content-Type: application/octet-stream`, `Content-Encoding: gzip` when gzip, sent concurrently. Both answer a JSON array of `{name, type, path, metadata?}` | `py: sandbox_async/filesystem/filesystem.py:331-489`; `py: sandbox/filesystem/filesystem.py:56-90, 267-280` |
| `use_octet_stream = (use_octet_stream or gzip) and envd >= 0.5.7`; so gzip falls back to *uncompressed* multipart on older envd | `py: sandbox_async/filesystem/filesystem.py:369-384`; `py: envd/versions.py:8` |
| A streamed upload carries no client-side timeout; an in-memory one uses the request timeout | `py: sandbox_async/filesystem/filesystem.py:386-393, 413-419` |
| Metadata: validated before sending (key = HTTP token chars `A-Za-z0-9!#$%&'*+-.^_`\|~`, non-empty; value = printable US-ASCII `0x20..=0x7e`); refused on envd < 0.6.2; sent as `X-Metadata-<key>: <value>` on every upload request; envd lowercases keys | `py: sandbox/filesystem/filesystem.py:342-380`; `py: sandbox_async/filesystem/filesystem.py:291-329, 360-367, 394-396`; `py: envd/versions.py:9` |
| `EntryInfo.metadata` is `map<string, string>` field 11 (proto JSON: an object) | `py: envd/filesystem/filesystem_pb.py:240-307` |
| `EntryInfo.is_symlink` exists, and the SDK reports `SYMLINK` when it is set (the crate does not read it; deferred above) | `py: envd/filesystem/filesystem_pb.py:240, 307`; `py: sandbox/filesystem/filesystem.py:130-148` |
| Streamed download: the same `GET /files` with the body read incrementally; no whole-request timeout unless the caller sets one | `py: sandbox_async/filesystem/filesystem.py:197-262` |
| Signed URL: `<envd direct url>/files?path=<p>[&username=<u>]&signature=v1_<b64nopad(sha256(raw))>[&signature_expiration=<unix s>]`, `raw = "<path>:<read\|write>:<user or empty>:<token>[:<expiration>]"`, expiration = now + seconds; user defaults to `"user"` below envd 0.4.0; without a token the URL is unsigned and an expiration is an `InvalidArgument` error | `py: sandbox/signature.py:15-47`; `py: sandbox/main.py:102-213`; `js: 6225-6290` |
| Every hosted sandbox is secured, so it always has an envd access token (`secure` is deprecated and ignored) | `py: sandbox_async/main.py:177, 197` |
| Known signature vectors, computed with the SDK's algorithm: `"/home/user/a.txt:read::tok"` gives `v1_Zkbb3914oLJLvMBzCAHQhm2ydBLxbSx2eQUR/kLdvIY`; `"/tmp/up.bin:write:root:tok:1700000000"` gives `v1_Y0uzWP5ST4jxa8RLcFIf65h1K6NEbMc0oox+Mgq89DY` | Python `hashlib.sha256` + `base64.b64encode(..).rstrip(b"=")`, run while planning |
| `reqwest::Body::wrap_stream<S: TryStream + Send + 'static>` with `S::Error: Into<Box<dyn Error + Send + Sync>>` and `Bytes: From<S::Ok>`; needs feature `stream` | `reqwest-0.13.5/src/async_impl/body.rs:83-92`; `Cargo.toml:179-182` |
| `reqwest::multipart::Part::stream(impl Into<Body>)` (unknown length) and `Part::stream_with_length` | `reqwest-0.13.5/src/async_impl/multipart.rs:256-266` |
| `reqwest::Response::chunk()` reads the body incrementally with no extra feature (the crate already uses it in `ConnectStream`) | `src/envd/connect.rs:365-379` |
| `futures_util::stream::unfold` is available with `default-features = false`; `future::try_join_all` needs the `alloc` feature | `futures-util-0.3.34/src/stream/mod.rs:105-106`; `Cargo.toml:38-43` |
| `tokio_util::io::ReaderStream::new(R: AsyncRead)` yields `io::Result<Bytes>`; feature `io` | `tokio-util-0.7.20/src/io/reader_stream.rs:45-82`; `Cargo.toml:65` |
| `async_compression::tokio::bufread::GzipEncoder::new(R: AsyncBufRead)` and `GzipDecoder`; features `tokio`, `gzip` | `async-compression-0.4.42/src/macros.rs:1-30, 120-133`; `Cargo.toml:43-68` |
| `secrecy::SecretString` is `SecretBox<str>`, whose `Debug` prints `SecretBox<str>([REDACTED])`, so a derived `Debug` on `EnvdClient` holding one does not leak the token | `secrecy-0.10.3/src/lib.rs:141-145, 214` |
| `url::Url::query_pairs_mut` returns a `form_urlencoded::Serializer`: bytes other than `*-._` and ASCII alphanumerics are percent-encoded (so `/` becomes `%2F`) and a space becomes `+` | `url-2.5.8/src/lib.rs:1701`; `form_urlencoded-1.2.2/src/lib.rs:121-145` |
| `sha2::Sha256::digest(data) -> Output` through `sha2::Digest` (re-exported from digest 0.11) | `sha2-0.11.1/src/lib.rs:19`; `digest-0.11.3/src/digest.rs:56` |
| tokio `AsyncRead` is always available; `BufReader`, `AsyncReadExt` and `io::repeat` need `io-util`; `tokio::spawn` needs `rt`; `tokio::sync::mpsc` needs `sync` | `tokio-1.54.1/src/io/mod.rs:209-210, 280`; `Cargo.toml:94, 119` |
| rustc here is 1.97.1; sha2 0.11.1 needs 1.88, tokio-util 0.7.20 needs 1.85, async-compression 0.4.42 needs 1.83 | `rustc --version`; each crate's `Cargo.toml` `rust-version` |
| The crate today: `ConnectStream::new(response, timeout)` and `next()`; `EnvdClient::{unary, server_stream, http_request, user_headers, effective_user, version, request_timeout}`; `Commands::open` waits for the start event within the request timeout; `ProcessInput { stdin }` is a struct; `StartRequest.stdin` is a plain `bool`; `FILES_ROUTE` is private to `filesystem.rs`; `EnvdClient::direct_base` is `#[cfg(test)]` only; the access token is kept only inside a header | `src/envd/connect.rs:289-379`; `src/envd/mod.rs:134-370`; `src/commands.rs:346-382`; `src/envd/process_pb.rs:31-151`; `src/filesystem.rs:21` |
| Mock-test helpers: `sandbox`, `sandbox_with_version`, `sandbox_from_json`, `sandbox_json`, `message`, `end_stream`, `stream_response`, `decode_request_envelope`, `ACCESS_TOKEN`, `client_with_timeout`; Task 6 adds `sandbox_from_json_with_client(server, &client, created)` | `tests/common/mod.rs:15-105` |
| Live helpers: `LiveSandbox::new()` (Deref to `Sandbox`), `poll_until`, `unix_now` | `tests/live_support/mod.rs:140-225` |

## Conventions for every task

- **Gates:** `just lint` (fmt check plus `clippy --all-targets -D warnings`), `just test` and `just doc` must pass before pushing. Feature tasks (2 to 8) also run their own new live tests against hosted E2B before pushing, one test at a time:
  `E2B_TEST_TARGET=hosted E2B_TEST_RUN_ID=dev-$(date -u +%Y%m%dT%H%M%SZ)-$$ cargo test --test <live_commands|live_files> -- --ignored --exact <test name>`.
  Report the result in the PR. The recorded runs are Tasks 9 and 10.
- **Tests:** mock tests use `wiremock` through `tests/common`. Each feature task creates its own mock test file (named in its Files list), to keep conflicts out of `tests/envd_api.rs`. Live tests are `#[tokio::test]` plus `#[ignore = "live: run with `just test-live <target>`"]`, named `clm_NNNN_<slug>`, one claim each, Arrange-Act-Assert. Use `.expect("<what>")` in tests, never `.unwrap()`. No `println!`/`dbg!`.
- **Docs per task:** every new public item gets a rustdoc comment with Arguments/Returns/Errors and an example where useful (CLAUDE.md). Every wire fact the task implements gets a row in `docs/protocol-notes.md` (new sections "Streamed input", "PTY", "Directory watching", "Uploads", "Signed URLs") and a row per S4 claim in `docs/verification/s4-evidence-map.md` (append rows; extend the intro line to "and CLM-0130 to CLM-0142").
- **Ledger:** subagents never call `mcp__claims__*` write tools. Read tools are fine.
- **Changelog:** one entry per task, `changelog/<UTC ts>-m2-task-<NN>-<slug>.md`, timestamp from `date -u +%Y-%m-%dT%H-%M-%SZ`.
- **Logs:** live run logs go to `target/live-logs/` inside your own worktree. Never write them to a shared scratchpad.
- **Processes:** never kill processes by pattern; kill only your own PIDs.
- The code in this plan has not been compiled. If an implementation has to differ, fix this plan in the same PR (inline, or with a dated `> **Corrected (issue #N).**` note), then grep later tasks for the changed symbol.

## File map

| File | Task |
|---|---|
| `claims.json` | plan PR; Task 11 |
| `justfile`, `CONTRIBUTING.md` (run-log header) | 1 |
| `Cargo.toml` | 2 (futures-util, reqwest `stream`, tokio `rt`/`sync`), 5 (async-compression, tokio-util, futures-util `alloc`), 6 (tokio `io-util`), 8 (sha2) |
| `src/envd/mod.rs` | 2 (`client_stream`, shared stream POST, keepalive constants, `within_request_timeout`), 4 (watch version constants), 5 (`OCTET_STREAM_UPLOAD`, untimed requests), 7 (`FILE_METADATA`), 8 (keep the access token, `direct_base`) |
| `src/envd/process_pb.rs` | 2 (`ProcessInput` oneof, `StreamInputRequest`), 3 (`PtyConfig`, `StartRequest.pty`/`stdin`, `DataEvent.pty`, `UpdateRequest`) |
| `src/envd/filesystem_pb.rs` | 4 (`WatchDirRequest`, `WatchDirResponse`, `FilesystemEventWire`), 7 (`EntryInfoWire.metadata`) |
| `src/stream_input.rs` (new) | 2 |
| `src/commands.rs` | 2 (`stream_input`, uses the shared helpers), 3 (shared `open_process_stream`, `StartRequest` fields) |
| `src/pty.rs` (new) | 3 |
| `src/filesystem.rs` | 4 (watch), 5 (`WriteEntry`, `WriteOptions`, `write_files`), 6 (`write_stream`, `read_stream`, `FileStream`), 7 (metadata), 8 (`FILES_ROUTE` made `pub(crate)`) |
| `src/sandbox.rs` | 3 (`pty()`), 8 (`FileUrlOptions`, `upload_url`, `download_url`) |
| `src/lib.rs` (re-exports, crate docs) | 2, 3, 4, 5, 6, 8 |
| `tests/common/mod.rs` (`decode_request_envelopes`) | 2 |
| `tests/envd_stream_input.rs` (new) | 2 |
| `tests/envd_pty.rs` (new) | 3 |
| `tests/envd_watch.rs` (new) | 4 |
| `tests/envd_write.rs` (new) | 5, 7 |
| `tests/envd_stream_files.rs` (new) | 6 |
| `tests/envd_file_urls.rs` (new) | 8 |
| `tests/envd_api.rs` (existing assertions touched by refactors only) | 2, 3, 5 |
| `tests/live_commands.rs` | 2, 3 |
| `tests/live_files.rs` | 4, 5, 6, 7, 8 |
| `docs/protocol-notes.md`, `docs/verification/s4-evidence-map.md` | 2 to 8 (one section each), 11 |
| `docs/verification/runs/<run id>.log`, `docs/verification/m2-live-results.md` | 9, 10 |
| `README.md`, milestone changelog | 11 |

## Task dependencies

```
2 -> 3
2 -> 4
5 -> 6
5 -> 7
1, 2, 3, 4, 5, 6, 7, 8 -> 9
1, 2, 3, 4, 5, 6, 7, 8 -> 10
9, 10 -> 11
```

Tasks 1, 2, 5 and 8 can start immediately. Task 3 needs Task 2's `InputStream` and process wire changes; Task 4 needs Task 2's keepalive constants and `within_request_timeout`. Tasks 6 and 7 build on Task 5's `WriteOptions` and upload path. If tasks run side by side, the expected conflicts are in `Cargo.toml`, `src/lib.rs`, `src/envd/mod.rs`, `docs/protocol-notes.md`, the evidence map and the live test files. Every one of them is an append: keep both sides. Tasks 9 and 10 run in parallel and each adds one section to `docs/verification/m2-live-results.md`; keep both. Task 11 is the coordinator's.

---

### Task 1: Stamp the commit in run logs (#26)

**Files:**
- Modify: `justfile` (`test-live`)
- Modify: `CONTRIBUTING.md` (Tests section, the run-log paragraph)

- [ ] **Step 1: Write the header before the test output**

In `test-live`, replace the first `cargo test ... | tee "$log"` so the log starts with two lines, then the test output is appended:

```bash
    log="target/live-logs/${E2B_TEST_RUN_ID}.log"
    # The first two lines say exactly which code produced this log.
    # Outside a git checkout, or without git, both say `unknown` rather than lying.
    {
        echo "commit: $(git rev-parse --verify -q HEAD 2>/dev/null || echo unknown)"
        if ! porcelain="$(git status --porcelain 2>/dev/null)"; then echo "dirty: unknown"
        elif [ -n "$porcelain" ]; then echo "dirty: yes"
        else echo "dirty: no"; fi
    } | tee "$log"
    # --no-fail-fast: a failing binary must not stop the others from running.
    cargo test --no-fail-fast --test live_lifecycle --test live_commands --test live_files \
        -- --ignored --test-threads=4 --show-output 2>&1 | tee -a "$log"
```

> **Corrected (issue #26, 2026-10-27).** The header guards against running outside a git checkout or without git: the commit line falls back to `unknown` and the dirty line prints `dirty: unknown` when `git status` fails, instead of an empty commit with `dirty: no` (PR #41 review). `dirty: unknown` doesn't count as a recorded run, as with `dirty: yes`.

`git status --porcelain` lists untracked files too, so a stray file also marks the tree dirty. That is intended: the log can't tell which files mattered.

- [ ] **Step 2: Check it without a live run**

Run: `just test-live nosuchtarget; head -2 target/live-logs/*-nosuchtarget-*.log`
Expected: the tests fail fast, because `live_support::target()` panics on an unknown target, and the log starts with `commit: <40 hex>` and `dirty: no` (or `yes` while the change is uncommitted). No sandbox is created, so this costs nothing.

- [ ] **Step 3: Document it**

In `CONTRIBUTING.md`, where it describes the run log, add: "The first two lines are the commit (`git rev-parse HEAD`) and whether the working tree had changes (`dirty: yes|no`)."

- [ ] **Step 4: Gates, changelog, commit**

Run `just lint && just test`. Write `changelog/<ts>-m2-task-01-run-log-commit-stamp.md`.

```bash
git add justfile CONTRIBUTING.md changelog/
git commit -m "test-live: stamp the commit and dirty flag at the top of each run log (#26)"
```

---

### Task 2: Client-streaming transport and streamed stdin (CLM-0046, 0130, 0131)

**Files:**
- Modify: `Cargo.toml`
- Modify: `src/envd/mod.rs`, `src/envd/process_pb.rs`, `src/commands.rs`, `src/lib.rs`
- Create: `src/stream_input.rs`
- Modify: `tests/common/mod.rs`
- Create: `tests/envd_stream_input.rs`
- Modify: `tests/live_commands.rs`, `docs/protocol-notes.md`, `docs/verification/s4-evidence-map.md`

- [ ] **Step 1: Dependencies**

```toml
# Stream combinators for request bodies (`stream::unfold`).
futures-util = { version = "0.3.34", default-features = false }
reqwest = { version = "0.13.5", default-features = false, features = ["json", "multipart", "http2", "rustls", "query", "stream"] }
# Deadlines on streams; a background task drives client-streaming calls.
tokio = { version = "1.54.1", features = ["time", "rt", "sync"] }

[dev-dependencies]
# `test-util` gives tests paused time (`start_paused`), so timer tests do not wait.
tokio = { version = "1.54.1", features = ["rt-multi-thread", "macros", "net", "io-util", "test-util"] }
```

- [ ] **Step 2: Write the failing mock tests**

Add to `tests/common/mod.rs`:

```rust
/// Decodes every envelope of a streaming request body, in order.
pub fn decode_request_envelopes(mut body: &[u8]) -> Vec<Value> {
    let mut messages = Vec::new();
    while !body.is_empty() {
        assert!(body.len() >= 5, "request body too short for an envelope");
        assert_eq!(body[0], 0, "request envelope flags");
        let len = u32::from_be_bytes([body[1], body[2], body[3], body[4]]) as usize;
        assert!(body.len() >= 5 + len, "request envelope length");
        messages.push(serde_json::from_slice(&body[5..5 + len]).expect("request envelope json"));
        body = &body[5 + len..];
    }
    messages
}
```

Create `tests/envd_stream_input.rs`:

```rust
//! Mock tests for streamed input (`process.Process/StreamInput`).

mod common;

use common::{
    ACCESS_TOKEN, decode_request_envelopes, end_stream, message, sandbox, stream_response,
};
use e2b_client::{ConnectCode, Error, StreamInputOptions};
use serde_json::json;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer};

const ROUTE: &str = "/process.Process/StreamInput";

#[tokio::test]
async fn stream_input_sends_start_then_one_data_envelope_per_chunk() {
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    Mock::given(method("POST"))
        .and(path(ROUTE))
        .and(header("content-type", "application/connect+json"))
        .and(header("connect-protocol-version", "1"))
        .and(header("x-access-token", ACCESS_TOKEN))
        .respond_with(stream_response(&[message(json!({})), end_stream(json!({}))]))
        .expect(1)
        .mount(&server)
        .await;

    let mut input = sandbox
        .commands()
        .stream_input(7, &StreamInputOptions::default().timeout(None));
    input.send("a").await.expect("send a");
    input.send(b"b".to_vec()).await.expect("send b");
    input.close().await.expect("close");

    let requests = server.received_requests().await.expect("requests");
    let stream = requests
        .iter()
        .find(|request| request.url.path() == ROUTE)
        .expect("StreamInput request");
    assert_eq!(
        decode_request_envelopes(&stream.body),
        vec![
            json!({"start": {"process": {"pid": 7}}}),
            json!({"data": {"input": {"stdin": "YQ=="}}}),
            json!({"data": {"input": {"stdin": "Yg=="}}}),
        ]
    );
}

#[tokio::test]
async fn stream_input_deadline_defaults_to_60_s() {
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    Mock::given(method("POST"))
        .and(path(ROUTE))
        .and(header("connect-timeout-ms", "60000"))
        .respond_with(stream_response(&[message(json!({})), end_stream(json!({}))]))
        .expect(1)
        .mount(&server)
        .await;

    // No timeout given: the SDKs' 60 s default applies.
    let input = sandbox
        .commands()
        .stream_input(7, &StreamInputOptions::default());
    input.close().await.expect("close");
}

#[tokio::test]
async fn stream_input_error_is_reported_by_close() {
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    Mock::given(method("POST"))
        .and(path(ROUTE))
        .respond_with(stream_response(&[end_stream(
            json!({"error": {"code": "not_found", "message": "process 7 not found"}}),
        )]))
        .mount(&server)
        .await;

    let mut input = sandbox
        .commands()
        .stream_input(7, &StreamInputOptions::default().timeout(None));
    input.send("x").await.expect("queued before the server answered");
    let err = input.close().await.expect_err("not found");

    assert!(
        matches!(err, Error::Rpc { code: ConnectCode::NotFound, .. }),
        "{err:?}"
    );
}
```

The keepalive is tested without a server, as a unit test in `src/stream_input.rs` (Step 5), on Tokio's paused clock (`#[tokio::test(start_paused = true)]`), so it uses the real 50 s interval and still finishes at once.

Run: `cargo test --test envd_stream_input`
Expected: compile errors (`stream_input` does not exist).

- [ ] **Step 3: Wire types**

In `src/envd/process_pb.rs`, turn `ProcessInput` into the oneof (this changes `send_input_base64_encodes_stdin` to `ProcessInput::Stdin(b"hi\n")`), and add the request:

```rust
/// `process.ProcessInput`: a oneof of `stdin` or `pty` bytes.
///
/// Serde's externally tagged enum gives the proto3 JSON shape of a oneof:
/// `{"stdin": "<base64>"}` or `{"pty": "<base64>"}`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ProcessInput<'a> {
    Stdin(#[serde(serialize_with = "serialize_base64")] &'a [u8]),
    Pty(#[serde(serialize_with = "serialize_base64")] &'a [u8]),
}

/// `process.StreamInputRequest`: a oneof of `start`, `data` and `keepalive`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum StreamInputRequest<'a> {
    Start { process: ProcessSelector },
    Data { input: ProcessInput<'a> },
    Keepalive {},
}
```

Unit tests next to the existing ones:

```rust
#[test]
fn stream_input_messages_match_proto_json() {
    let start = StreamInputRequest::Start { process: ProcessSelector::Pid(3) };
    let data = StreamInputRequest::Data { input: ProcessInput::Pty(b"ls\r") };
    assert_eq!(serde_json::to_value(start).expect("ser"), json!({"start": {"process": {"pid": 3}}}));
    assert_eq!(serde_json::to_value(data).expect("ser"), json!({"data": {"input": {"pty": "bHMN"}}}));
    assert_eq!(serde_json::to_value(StreamInputRequest::Keepalive {}).expect("ser"), json!({"keepalive": {}}));
}
```

(`use serde_json::json;` in the test module.)

- [ ] **Step 4: Transport**

In `src/envd/mod.rs`:

1. Move `KEEPALIVE_PING_HEADER` and `KEEPALIVE_PING_INTERVAL_SECS` from `src/commands.rs` here, private (callers go through `keepalive_headers()`), and add:

   ```rust
   /// The keepalive header envd honours on long streams.
   pub(crate) fn keepalive_headers() -> HeaderMap {
       let mut headers = HeaderMap::with_capacity(1);
       headers.insert(KEEPALIVE_PING_HEADER, HeaderValue::from(KEEPALIVE_PING_INTERVAL_SECS));
       headers
   }
   ```

   `Commands::start` and `connect` then use it (`start` extends the user headers with it).

2. Extract the POST-and-check-status part of `server_stream` into one private helper, and use it from `server_stream` and the new `client_stream`:

   ```rust
   /// POSTs a streaming Connect call and returns the response once it is 200.
   async fn post_stream(
       &self,
       service: &str,
       method: &str,
       body: reqwest::Body,
       extra_headers: HeaderMap,
       timeout: Option<Duration>,
   ) -> Result<reqwest::Response> {
       let response = self
           .http
           .post(self.rpc_url(service, method))
           .headers(self.headers.clone())
           .headers(extra_headers)
           .headers(protocol_headers(CONTENT_TYPE_STREAM_JSON, timeout))
           .body(body)
           .send()
           .await?;
       let status = response.status();
       if status != reqwest::StatusCode::OK {
           let content_type = content_type_of(&response);
           let bytes = response.bytes().await?;
           return Err(error_from_response(status.as_u16(), content_type.as_deref(), &bytes));
       }
       Ok(response)
   }

   /// Makes a client-streaming Connect call: `body` yields the request
   /// envelopes, and the call finishes when envd answers after the body ends.
   ///
   /// # Arguments
   ///
   /// * `service`, `method` - the RPC to call.
   /// * `body` - the enveloped request messages, streamed.
   /// * `extra_headers` - per-call headers.
   /// * `timeout` - the call deadline (sent as `Connect-Timeout-Ms` and
   ///   enforced client-side), or `None` for no limit.
   ///
   /// # Errors
   ///
   /// [`Error::Rpc`] when envd rejects the call or ends it with an error,
   /// [`Error::Timeout`] when the deadline passes, [`Error::Protocol`] for a
   /// malformed response, [`Error::Transport`] for connection failures.
   pub(crate) async fn client_stream(
       &self,
       service: &str,
       method: &str,
       body: reqwest::Body,
       extra_headers: HeaderMap,
       timeout: Option<Duration>,
   ) -> Result<()> {
       let call = async {
           let response = self.post_stream(service, method, body, extra_headers, timeout).await?;
           // Connect answers a client stream with one message and an
           // end-of-stream envelope; drain both so an error is surfaced.
           let mut stream: ConnectStream<Empty> = ConnectStream::new(response, None);
           while stream.next().await?.is_some() {}
           Ok(())
       };
       within(timeout, call).await
   }
   ```

3. Add the shared opening-deadline helper (Task 4 and Task 3 reuse it), and make `Commands::open` use it:

   ```rust
   /// Runs `opening` (open a stream and read its first event) within the
   /// request timeout, as the SDKs' `first_event` does
   /// (`e2b/envd/client_async/__init__.py:113-149`).
   ///
   /// # Errors
   ///
   /// [`Error::Timeout`] when the request timeout passes first, else the
   /// error of `opening`.
   pub(crate) async fn within_request_timeout<T>(
       &self,
       opening: impl Future<Output = Result<T>>,
   ) -> Result<T> {
       within(self.request_timeout, opening).await
   }
   ```

   Both use one private free function in `src/envd/mod.rs`,
   `async fn within<T>(limit: Option<Duration>, future: impl Future<Output = Result<T>>) -> Result<T>`,
   which maps an elapsed deadline to `Error::Timeout(limit)`.

   > **Corrected (issue #31, 2026-10-27).** With `Commands::open` on `within_request_timeout`, nothing called
   `EnvdClient::request_timeout()` any more, so it was removed (no dead code).
   Task 5 re-adds it where it needs it (see the note there).

- [ ] **Step 5: `src/stream_input.rs`**

```rust
//! Streamed input to a command or PTY, over envd's client-streaming
//! `process.Process/StreamInput` RPC (`e2b/sandbox_async/commands/command_input_handle.py`).

use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use futures_util::stream::{self, Stream};
use reqwest::header::HeaderMap;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::envd::EnvdClient;
use crate::envd::connect::encode_envelope;
use crate::envd::process_pb::{self, ProcessInput, ProcessSelector, StreamInputRequest};
use crate::error::{Error, Result};

/// The SDKs' default bound on an input stream (`STREAM_INPUT_TIMEOUT`,
/// `e2b/connection_config.py:33`).
pub const DEFAULT_STREAM_INPUT_TIMEOUT: Duration = Duration::from_secs(60);

/// Options for [`crate::Commands::stream_input`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StreamInputOptions {
    // `pub(crate)`: `commands.rs` (and Task 3's `pty.rs`) read it.
    pub(crate) timeout: Option<Duration>,
}

impl Default for StreamInputOptions {
    fn default() -> Self {
        Self {
            timeout: Some(DEFAULT_STREAM_INPUT_TIMEOUT),
        }
    }
}

impl StreamInputOptions {
    /// Bounds the whole input stream; `None` for no limit. Defaults to
    /// [`DEFAULT_STREAM_INPUT_TIMEOUT`].
    ///
    /// Keepalives do not extend this deadline: they only keep idle proxies
    /// from dropping the connection.
    ///
    /// # Arguments
    ///
    /// * `timeout` - the stream deadline.
    ///
    /// # Returns
    ///
    /// The options, for chaining.
    #[must_use]
    pub fn timeout(mut self, timeout: Option<Duration>) -> Self {
        self.timeout = timeout;
        self
    }
}

/// How long the stream may sit idle before a keepalive message is sent
/// (`KEEPALIVE_PING_INTERVAL_SEC`, `e2b/connection_config.py:48`).
const KEEPALIVE_INTERVAL: Duration = Duration::from_secs(50);

/// How many chunks may wait to be sent before [`InputStream::send`] waits.
const QUEUE_DEPTH: usize = 64;

/// Which `ProcessInput` member the data goes in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InputKind {
    /// A command's stdin.
    Stdin,
    /// A PTY's input.
    #[expect(dead_code, reason = "PTY sessions (M2 Task 3) open PTY input streams")]
    Pty,
}

/// An ordered, long-lived input stream to a running command or PTY.
///
/// Get one from [`crate::Commands::stream_input`]. Chunks reach the process in the order they
/// were sent. Call [`close`](Self::close) to end the stream and learn whether
/// envd accepted it. Dropping the stream without `close` also ends it, but any
/// error is then lost.
///
/// The stream's deadline ([`StreamInputOptions::timeout`], 60 s by default)
/// bounds the whole stream. The keepalive sent every 50 s while idle stops
/// proxies from dropping the connection, but it does not extend the
/// deadline: with the default, the stream ends 60 s after it opens however
/// active it is, as in the SDKs. Pass `.timeout(None)` for a stream with no
/// limit.
///
/// Python note: this is the SDK's `AsyncCommandInputHandle`. `send` is
/// `async` because it waits when the queue is full instead of buffering
/// without bound.
#[derive(Debug)]
pub struct InputStream {
    sender: Option<mpsc::Sender<Bytes>>,
    task: Option<JoinHandle<Result<()>>>,
}

impl InputStream {
    /// Starts the `StreamInput` call on a background task.
    ///
    /// # Panics
    ///
    /// Outside a Tokio runtime (`tokio::spawn` requires one).
    pub(crate) fn open(
        envd: Arc<EnvdClient>,
        pid: u32,
        kind: InputKind,
        timeout: Option<Duration>,
    ) -> Self {
        let (sender, receiver) = mpsc::channel(QUEUE_DEPTH);
        let body = reqwest::Body::wrap_stream(request_messages(pid, kind, receiver));
        let task = tokio::spawn(async move {
            envd.client_stream(process_pb::SERVICE, "StreamInput", body, HeaderMap::new(), timeout)
                .await
        });
        Self { sender: Some(sender), task: Some(task) }
    }

    /// Queues `data` to be sent after everything sent before it.
    ///
    /// # Arguments
    ///
    /// * `data` - the bytes (`&'static str`, `String`, `Vec<u8>`, `Bytes`...).
    ///
    /// # Errors
    ///
    /// The error that ended the stream early ([`Error::Rpc`],
    /// [`Error::Timeout`], [`Error::Transport`]...), or
    /// [`Error::InvalidArgument`] once the stream has failed and that error
    /// was already returned.
    pub async fn send(&mut self, data: impl Into<Bytes>) -> Result<()> {
        let Some(sender) = &self.sender else {
            return Err(closed());
        };
        if sender.send(data.into()).await.is_ok() {
            return Ok(());
        }
        // The receiver is gone, so the call has ended: report why.
        self.sender = None;
        match self.finish().await {
            Err(err) => Err(err),
            Ok(()) => Err(Error::Protocol("input stream ended before it was closed".into())),
        }
    }

    /// Ends the input stream and waits for envd to acknowledge it.
    ///
    /// This does not close the process's stdin; use
    /// [`crate::Commands::close_stdin`] for that.
    ///
    /// # Errors
    ///
    /// The error envd ended the call with, [`Error::Timeout`] when the stream
    /// deadline passed, or [`Error::Transport`].
    pub async fn close(mut self) -> Result<()> {
        // Dropping the sender ends the request body after the queued chunks.
        self.sender = None;
        self.finish().await
    }

    async fn finish(&mut self) -> Result<()> {
        let task = self.task.take().ok_or_else(closed)?;
        task.await
            .map_err(|err| Error::Protocol(format!("input stream task failed: {err}")))?
    }
}

fn closed() -> Error {
    Error::InvalidArgument("the input stream is closed; open a new one with stream_input".into())
}

/// The request body: a start message, then one data message per chunk, with a
/// keepalive whenever [`KEEPALIVE_INTERVAL`] passes without a chunk. It ends
/// when every sender is dropped.
fn request_messages(
    pid: u32,
    kind: InputKind,
    receiver: mpsc::Receiver<Bytes>,
) -> impl Stream<Item = Result<Bytes>> + Send + 'static {
    // `unfold` threads the state (receiver, whether start was sent) through
    // each step, like a Python async generator's local variables.
    stream::unfold((receiver, false), move |(mut receiver, started)| async move {
        let frame = if started {
            match tokio::time::timeout(KEEPALIVE_INTERVAL, receiver.recv()).await {
                Ok(Some(chunk)) => {
                    let input = match kind {
                        InputKind::Stdin => ProcessInput::Stdin(&chunk),
                        InputKind::Pty => ProcessInput::Pty(&chunk),
                    };
                    envelope(&StreamInputRequest::Data { input })
                }
                Ok(None) => return None,
                Err(_idle) => envelope(&StreamInputRequest::Keepalive {}),
            }
        } else {
            envelope(&StreamInputRequest::Start { process: ProcessSelector::Pid(pid) })
        };
        Some((frame, (receiver, true)))
    })
}

fn envelope(message: &StreamInputRequest<'_>) -> Result<Bytes> {
    Ok(Bytes::from(encode_envelope(&serde_json::to_vec(message)?)?))
}
```

Unit tests in the same file (`#[cfg(test)] mod tests`):

```rust
use futures_util::StreamExt;

fn decode(frame: &Bytes) -> serde_json::Value {
    serde_json::from_slice(&frame[5..]).expect("json")
}

// Paused time: the clock jumps to the next timer when every task waits, so
// the real 50 s keepalive arrives at once.
#[tokio::test(start_paused = true)]
async fn messages_start_then_data_then_keepalive_when_idle() {
    let (sender, receiver) = mpsc::channel(4);
    // `Box::pin` gives the `Unpin` stream `next()` needs.
    let mut messages = Box::pin(request_messages(9, InputKind::Stdin, receiver));

    let start = messages.next().await.expect("start").expect("ok");
    sender.send(Bytes::from_static(b"hi")).await.expect("send");
    let data = messages.next().await.expect("data").expect("ok");
    let idle_since = tokio::time::Instant::now();
    let keepalive = messages.next().await.expect("keepalive").expect("ok");
    let idle_for = idle_since.elapsed();
    drop(sender);
    let end = messages.next().await;

    assert_eq!(decode(&start), serde_json::json!({"start": {"process": {"pid": 9}}}));
    assert_eq!(decode(&data), serde_json::json!({"data": {"input": {"stdin": "aGk="}}}));
    assert_eq!(decode(&keepalive), serde_json::json!({"keepalive": {}}));
    assert_eq!(idle_for, KEEPALIVE_INTERVAL);
    assert!(end.is_none());
}
```

- [ ] **Step 6: `Commands::stream_input`**

```rust
    /// Opens an ordered input stream to a running command's stdin
    /// (`StreamInput`). The command must have been started with
    /// [`CommandOptions::stdin`] set.
    ///
    /// # Arguments
    ///
    /// * `pid` - the process ID.
    /// * `options` - the stream deadline (60 s by default; keepalives do not
    ///   extend it).
    ///
    /// # Returns
    ///
    /// An [`InputStream`]; errors such as a missing process surface from its
    /// `send` and `close`.
    ///
    /// # Panics
    ///
    /// Outside a Tokio runtime.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use e2b_client::{CommandOptions, Sandbox, StreamInputOptions};
    ///
    /// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
    /// let handle = sandbox.commands().start("cat", &CommandOptions::default().stdin(true)).await?;
    /// let mut input = sandbox.commands().stream_input(handle.pid(), &StreamInputOptions::default());
    /// input.send("line 1\n").await?;
    /// input.send("line 2\n").await?;
    /// input.close().await?;
    /// sandbox.commands().close_stdin(handle.pid()).await?;
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn stream_input(&self, pid: u32, options: &StreamInputOptions) -> InputStream {
        InputStream::open(Arc::clone(&self.envd), pid, InputKind::Stdin, options.timeout)
    }
```

`send_stdin` builds `ProcessInput::Stdin(data.as_ref())`. In `src/lib.rs`: `mod stream_input;` and `pub use stream_input::{DEFAULT_STREAM_INPUT_TIMEOUT, InputStream, StreamInputOptions};` (the `timeout` field of `StreamInputOptions` is `pub(crate)`, so `Commands::stream_input` and Task 3's `Pty::stream_input` pass `options.timeout` to `open`, which keeps taking an `Option<Duration>`); mention streamed stdin in the crate docs' process bullet.

- [ ] **Step 7: Live test**

In `tests/live_commands.rs`:

```rust
#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0046_stream_input_delivers_chunks_in_order() {
    let sandbox = LiveSandbox::new().await;
    let handle = sandbox
        .commands()
        .start("cat", &CommandOptions::default().stdin(true))
        .await
        .expect("start cat");

    let mut input = sandbox
        .commands()
        .stream_input(handle.pid(), &e2b_client::StreamInputOptions::default());
    for chunk in ["one ", "two ", "three\n"] {
        input.send(chunk).await.expect("send");
    }
    input.close().await.expect("close input stream");
    sandbox.commands().close_stdin(handle.pid()).await.expect("close stdin");
    let result = handle.wait().await.expect("wait");

    assert_eq!(result.stdout, "one two three\n");
    assert_eq!(result.exit_code, 0);
}
```

- [ ] **Step 8: Docs, gates, live check, changelog, commit**

Add a "Streamed input" section to `docs/protocol-notes.md` (the StreamInput rows from Verified facts) and remove `StreamInput` from "Not implemented in v0.1". Add evidence-map rows: CLM-0130 → `tests/envd_stream_input.rs::stream_input_sends_start_then_one_data_envelope_per_chunk`, `src/envd/process_pb.rs::tests::stream_input_messages_match_proto_json`; CLM-0131 → `src/stream_input.rs::tests::messages_start_then_data_then_keepalive_when_idle`, `tests/envd_stream_input.rs::stream_input_deadline_defaults_to_60_s`, `tests/envd_stream_input.rs::stream_input_sends_start_then_one_data_envelope_per_chunk` (the close-then-acknowledge part). Run the gates and the live test on hosted. Write `changelog/<ts>-m2-task-02-stream-input.md`.

```bash
git add Cargo.toml Cargo.lock src/ tests/ docs/ changelog/
git commit -m "feat: streamed stdin over envd's client-streaming StreamInput (CLM-0046)"
```

---

### Task 3: PTY sessions (CLM-0047, 0048, 0049, 0132, 0133, 0134)

**Files:**
- Modify: `src/envd/process_pb.rs`, `src/commands.rs`, `src/sandbox.rs`, `src/lib.rs`
- Create: `src/pty.rs`
- Create: `tests/envd_pty.rs`
- Modify: `tests/envd_api.rs` (only if a refactor changes an assertion), `tests/live_commands.rs`, `docs/protocol-notes.md`, `docs/verification/s4-evidence-map.md`

- [ ] **Step 1: Write the failing mock tests**

Create `tests/envd_pty.rs`:

```rust
//! Mock tests for PTY sessions.

mod common;

use common::{decode_request_envelope, end_stream, message, sandbox, stream_response};
use e2b_client::{PtyEvent, PtyOptions, PtySize};
use serde_json::json;
use wiremock::matchers::{body_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn pty_frames() -> Vec<Vec<u8>> {
    vec![
        message(json!({"event": {"start": {"pid": 11}}})),
        message(json!({"event": {"data": {"pty": "JCA="}}})),
        message(json!({"event": {"keepalive": {}}})),
        message(json!({"event": {"end": {"exitCode": 0, "exited": true}}})),
        end_stream(json!({})),
    ]
}

#[tokio::test]
async fn create_sends_bash_with_pty_size_and_terminal_env_defaults() {
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    Mock::given(method("POST"))
        .and(path("/process.Process/Start"))
        .respond_with(stream_response(&pty_frames()))
        .mount(&server)
        .await;

    let options = PtyOptions::default().env("LANG", "en_US.UTF-8").cwd("/tmp");
    let mut handle = sandbox
        .pty()
        .create(PtySize::new(80, 24), &options)
        .await
        .expect("create");

    assert_eq!(handle.pid(), 11);
    let requests = server.received_requests().await.expect("requests");
    let start = requests
        .iter()
        .find(|request| request.url.path() == "/process.Process/Start")
        .expect("Start request");
    assert_eq!(
        decode_request_envelope(&start.body),
        json!({
            "process": {
                "cmd": "/bin/bash",
                "args": ["-i", "-l"],
                "envs": {"TERM": "xterm-256color", "LANG": "en_US.UTF-8", "LC_ALL": "C.UTF-8"},
                "cwd": "/tmp"
            },
            "pty": {"size": {"cols": 80, "rows": 24}}
        })
    );
    assert_eq!(
        start.headers.get("keepalive-ping-interval").and_then(|v| v.to_str().ok()),
        Some("50")
    );
    // CLM-0133: PTY bytes come out as Data, keepalives are skipped.
    assert_eq!(
        handle.next_event().await.expect("event"),
        Some(PtyEvent::Data(bytes::Bytes::from_static(b"$ ")))
    );
    assert!(matches!(handle.next_event().await.expect("event"), Some(PtyEvent::Exit(exit)) if exit.exit_code == 0));
}

#[tokio::test]
async fn send_stdin_uses_the_pty_input_member() {
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    Mock::given(method("POST"))
        .and(path("/process.Process/SendInput"))
        .and(body_json(json!({"process": {"pid": 11}, "input": {"pty": "bHMN"}})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .expect(1)
        .mount(&server)
        .await;

    sandbox.pty().send_stdin(11, "ls\r").await.expect("send");
}

#[tokio::test]
async fn resize_sends_update_with_the_new_size() {
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    Mock::given(method("POST"))
        .and(path("/process.Process/Update"))
        .and(body_json(json!({"process": {"pid": 11}, "pty": {"size": {"cols": 120, "rows": 40}}})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .expect(1)
        .mount(&server)
        .await;

    sandbox.pty().resize(11, PtySize::new(120, 40)).await.expect("resize");
}

#[tokio::test]
async fn pty_stream_input_uses_the_pty_member() {
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    Mock::given(method("POST"))
        .and(path("/process.Process/StreamInput"))
        .respond_with(stream_response(&[message(json!({})), end_stream(json!({}))]))
        .mount(&server)
        .await;

    let mut input = sandbox
        .pty()
        .stream_input(11, &e2b_client::StreamInputOptions::default());
    input.send("ls\r").await.expect("send");
    input.close().await.expect("close");

    let requests = server.received_requests().await.expect("requests");
    let stream = requests
        .iter()
        .find(|request| request.url.path() == "/process.Process/StreamInput")
        .expect("StreamInput request");
    assert_eq!(
        common::decode_request_envelopes(&stream.body)[1],
        json!({"data": {"input": {"pty": "bHMN"}}})
    );
}
```

> **Corrected (issue #32, 2026-10-27).** The shipped `tests/envd_pty.rs` is a superset of the code above; read that file. The create test also sets `.user("root")` (matched as `Authorization: Basic cm9vdDo=`), asserts `Connect-Timeout-Ms: 60000` and that the stream returns `None` after `Exit`. `pty_stream_input_uses_the_pty_member` compares the whole envelope list (`start`, then `data` with `input.pty`) instead of index `[1]`. Three tests were added: `pty_handle_skips_stdout_and_command_handle_skips_pty_data` (CLM-0133's "separately from stdout and stderr"), `connect_attaches_by_pid_with_the_keepalive_header` and `kill_sends_sigkill_and_reports_a_missing_pid_as_false`. `src/envd/process_pb.rs` gained unit tests `pty_start_request_has_pty_size_and_no_stdin`, `update_request_matches_proto_json` and `data_event_parses_pty_bytes`; `src/pty.rs` has `terminal_defaults_fill_only_unset_keys`. The existing `run_streams_output_to_exit_code_with_sdk_headers` in `tests/envd_api.rs` already asserts `"stdin": false`, so no test was added for it.

Add one test to `tests/envd_api.rs`'s command tests, or to `tests/envd_pty.rs`, that `Commands::start` still sends `"stdin": false` (the existing `run_streams_output_to_exit_code_with_sdk_headers` may already assert the body; if so, leave it).

Run: `cargo test --test envd_pty` — Expected: compile errors.

- [ ] **Step 2: Wire types**

In `src/envd/process_pb.rs`:

```rust
/// `process.PTY`.
#[derive(Debug, Clone, Copy, Serialize)]
pub(crate) struct PtyConfig {
    pub(crate) size: PtySizeWire,
}

/// `process.PTY.Size`.
#[derive(Debug, Clone, Copy, Serialize)]
pub(crate) struct PtySizeWire {
    pub(crate) cols: u32,
    pub(crate) rows: u32,
}

/// `process.UpdateRequest`.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct UpdateRequest {
    pub(crate) process: ProcessSelector,
    pub(crate) pty: PtyConfig,
}
```

`StartRequest` gains `#[serde(skip_serializing_if = "Option::is_none")] pub(crate) pty: Option<PtyConfig>`, and `stdin` becomes `#[serde(skip_serializing_if = "Option::is_none")] pub(crate) stdin: Option<bool>`, because the SDKs send no `stdin` for a PTY. `Commands::start` passes `pty: None, stdin: Some(options.stdin)`, so its wire stays the same, and `start_request_matches_proto_json` is updated to match. `DataEvent` gains `#[serde(default, deserialize_with = "deserialize_base64_opt")] pub(crate) pty: Option<Bytes>`.

- [ ] **Step 3: Share stream opening**

In `src/commands.rs`, turn `Commands::open` into a crate-level function that both handles use:

```rust
/// Opens a `Start`/`Connect` stream and waits, within the request timeout,
/// for the start event that carries the PID
/// (`e2b/sandbox_async/commands/command.py:382-411`).
pub(crate) async fn open_process_stream<Req: serde::Serialize>(
    envd: &EnvdClient,
    method: &'static str,
    request: &Req,
    headers: HeaderMap,
    timeout: Option<Duration>,
) -> Result<(u32, ConnectStream<EventResponse>)> {
    envd.within_request_timeout(async {
        let mut stream: ConnectStream<EventResponse> = envd
            .server_stream(process_pb::SERVICE, method, request, headers, timeout)
            .await?;
        let pid = stream
            .next()
            .await?
            .and_then(|response| response.event)
            .and_then(|event| event.start)
            .map(|start| start.pid)
            .ok_or_else(|| Error::Protocol(format!("{method} stream did not open with a start event")))?;
        Ok((pid, stream))
    })
    .await
}
```

`Commands::start`/`connect` call it and wrap the result in `CommandHandle`. `CommandHandle::next_event` ignores `data.pty`.

- [ ] **Step 4: `src/pty.rs`**

```rust
//! Pseudo-terminal sessions in a sandbox (`e2b/sandbox_async/commands/pty.py`).

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use reqwest::header::HeaderMap;

use crate::commands::{CommandExit, Commands, open_process_stream};
use crate::envd::connect::{ConnectStream, Empty};
use crate::envd::process_pb::{
    self, ConnectRequest, EventResponse, ProcessConfig, ProcessInput, ProcessSelector, PtyConfig,
    PtySizeWire, SendInputRequest, StartRequest, UpdateRequest,
};
use crate::envd::{EnvdClient, keepalive_headers};
use crate::error::{Error, Result};
use crate::stream_input::{InputKind, InputStream, StreamInputOptions};

/// The SDKs' default bound on a PTY stream (`timeout=60`).
pub const DEFAULT_PTY_TIMEOUT: Duration = Duration::from_secs(60);

/// Terminal environment the SDKs set unless the caller did
/// (`e2b/sandbox_async/commands/pty.py:162-165`).
const TERMINAL_ENV_DEFAULTS: [(&str, &str); 3] =
    [("TERM", "xterm-256color"), ("LANG", "C.UTF-8"), ("LC_ALL", "C.UTF-8")];

/// A terminal size in character cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PtySize {
    /// Columns.
    pub cols: u16,
    /// Rows.
    pub rows: u16,
}

impl PtySize {
    /// Creates a size.
    ///
    /// # Arguments
    ///
    /// * `cols`, `rows` - the terminal width and height.
    ///
    /// # Returns
    ///
    /// The size.
    #[must_use]
    pub const fn new(cols: u16, rows: u16) -> Self {
        Self { cols, rows }
    }

    fn wire(self) -> PtyConfig {
        PtyConfig { size: PtySizeWire { cols: self.cols.into(), rows: self.rows.into() } }
    }
}
```

Then `PtyOptions` (fields `envs: HashMap<String, String>`, `cwd`, `user`, `timeout`, defaulting to `Some(DEFAULT_PTY_TIMEOUT)`; builders `env`, `cwd`, `user`, `timeout`, documented like `CommandOptions`). Then:

```rust
/// One event from a PTY session.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PtyEvent {
    /// Terminal output (stdout and stderr interleaved, with control sequences).
    Data(Bytes),
    /// The shell exited. No further events follow.
    Exit(CommandExit),
}

/// The PTY service of one sandbox. Get it from [`crate::Sandbox::pty`].
#[derive(Debug, Clone)]
pub struct Pty {
    envd: Arc<EnvdClient>,
}
```

Methods, all documented with Arguments/Returns/Errors:

- `pub(crate) fn new(envd: Arc<EnvdClient>) -> Self`, used by `Sandbox::pty`.
- `create(&self, size: PtySize, options: &PtyOptions) -> Result<PtyHandle>`: copy `options.envs`, insert each `TERMINAL_ENV_DEFAULTS` pair with `entry(..).or_insert_with(..)`, build `StartRequest { process: ProcessConfig { cmd: "/bin/bash", args: vec!["-i", "-l"], envs: &envs, cwd }, pty: Some(size.wire()), tag: None, stdin: None }`, headers = `user_headers(user)` extended with `keepalive_headers()`, then `open_process_stream(&self.envd, "Start", ...)`.
- `connect(&self, pid, timeout) -> Result<PtyHandle>`: `ConnectRequest`, `keepalive_headers()`.
- `send_stdin(&self, pid, data: impl AsRef<[u8]>) -> Result<()>`: `SendInput` with `ProcessInput::Pty`.
- `resize(&self, pid, size) -> Result<()>`: unary `Update` with `UpdateRequest`, response `Empty`.
- `kill(&self, pid) -> Result<bool>`: `Commands::new(Arc::clone(&self.envd)).kill(pid)`.
- `stream_input(&self, pid, options: &StreamInputOptions) -> InputStream`: `InputStream::open(.., InputKind::Pty, options.timeout)`.

> **Corrected (issue #31, 2026-10-27).** Task 2 shipped `InputKind::Pty` with `#[expect(dead_code, reason = ...)]`, because nothing built it yet. Remove that attribute here (the `expect` fails the build once `Pty::stream_input` uses the variant). Task 2 also left `crate::Pty::stream_input` out of the rustdoc of `StreamInputOptions` and `InputStream` (the link would not resolve); add it back to both.

```rust
/// A running PTY session. Read output with [`next_event`](Self::next_event).
#[derive(Debug)]
pub struct PtyHandle {
    pid: u32,
    stream: ConnectStream<EventResponse>,
    pty: Pty,
}

impl PtyHandle {
    /// The shell's process ID.
    #[must_use]
    pub fn pid(&self) -> u32 { self.pid }

    /// Waits for the next output chunk or the exit event; keepalives are
    /// skipped.
    ///
    /// # Errors
    ///
    /// As [`crate::CommandHandle::next_event`].
    pub async fn next_event(&mut self) -> Result<Option<PtyEvent>> {
        while let Some(response) = self.stream.next().await? {
            let Some(event) = response.event else { continue };
            if let Some(chunk) = event.data.and_then(|data| data.pty) {
                return Ok(Some(PtyEvent::Data(chunk)));
            }
            if let Some(end) = event.end {
                return Ok(Some(PtyEvent::Exit(CommandExit { exit_code: end.exit_code, error: end.error })));
            }
        }
        Ok(None)
    }

    /// Discards remaining output and waits for the shell to exit.
    ///
    /// # Errors
    ///
    /// [`Error::Protocol`] if the stream ends without an exit event, plus the
    /// errors of [`next_event`](Self::next_event).
    pub async fn wait(mut self) -> Result<CommandExit> {
        while let Some(event) = self.next_event().await? {
            if let PtyEvent::Exit(exit) = event {
                return Ok(exit);
            }
        }
        Err(Error::Protocol("PTY stream ended without an end event".into()))
    }
}
```

Plus `send_stdin`, `resize` and `kill` forwarding to `self.pty` with `self.pid`. `pty.rs` can build `CommandExit` with a struct literal: `#[non_exhaustive]` only binds other crates.

`src/sandbox.rs`: `pub fn pty(&self) -> Pty` next to `commands()`. `src/lib.rs`: `mod pty;`, `pub use pty::{DEFAULT_PTY_TIMEOUT, Pty, PtyEvent, PtyHandle, PtyOptions, PtySize};`, and a crate-docs bullet.

- [ ] **Step 5: Live tests**

In `tests/live_commands.rs`, collect PTY output with a helper:

```rust
/// Reads PTY events until exit; returns the output (lossy UTF-8) and exit code.
async fn pty_transcript(mut handle: e2b_client::PtyHandle) -> (String, i32) {
    let mut output = Vec::new();
    while let Some(event) = handle.next_event().await.expect("pty event") {
        match event {
            e2b_client::PtyEvent::Data(chunk) => output.extend_from_slice(&chunk),
            e2b_client::PtyEvent::Exit(exit) => {
                return (String::from_utf8_lossy(&output).into_owned(), exit.exit_code);
            }
            other => panic!("unexpected PTY event {other:?}"),
        }
    }
    panic!("PTY stream ended without Exit");
}
```

- `clm_0047_pty_create_streams_terminal_output`: create 80x24, then send all input in one `send_stdin(pid, "echo pty-$((40+2))\necho term-$TERM\nexit 3\n")`, so both checks run before `exit`. The transcript contains `pty-42` (only the shell can produce it: the echoed input reads `pty-$((40+2))`) and `term-xterm-256color` (the TERM default reached the shell), and the exit code is 3.
- `clm_0048_pty_resize_changes_terminal_size`: create 80x24; `resize(pid, PtySize::new(132, 43))`; `send_stdin("stty size; exit\n")`; transcript contains `43 132`.
- `clm_0049_pty_input_reaches_the_shell`: create; open `stream_input`, send `"echo in-"` then `"$((6*7))\n"` then `"exit\n"`, close it; transcript contains `in-42`. This covers both `send_stdin` (CLM-0047) and streamed PTY input.

- [ ] **Step 6: Docs, gates, live check, changelog, commit**

> **Corrected (issue #32, 2026-10-27).** The evidence rows also name the unit tests listed in the Step 1 note: CLM-0132 adds `src/envd/process_pb.rs::tests::pty_start_request_has_pty_size_and_no_stdin` and `src/pty.rs::tests::terminal_defaults_fill_only_unset_keys`; CLM-0133 adds `tests/envd_pty.rs::pty_handle_skips_stdout_and_command_handle_skips_pty_data` and `src/envd/process_pb.rs::tests::data_event_parses_pty_bytes`; CLM-0134 adds `src/envd/process_pb.rs::tests::update_request_matches_proto_json`.

Add a "PTY" section to `docs/protocol-notes.md`; drop PTY and `Update` from "Not implemented". Evidence-map rows: CLM-0132 → `tests/envd_pty.rs::create_sends_bash_with_pty_size_and_terminal_env_defaults`; CLM-0133 → the same test's event assertions (say so in Notes); CLM-0134 → `send_stdin_uses_the_pty_input_member`, `resize_sends_update_with_the_new_size`. Add `pty_stream_input_uses_the_pty_member` to the CLM-0130 row, and change that row's "Covers whole claim?" from `partial` to `yes` (Task 2 marked it partial because no stream sent `input.pty` yet). Run the gates and the three live tests on hosted. Write `changelog/<ts>-m2-task-03-pty.md`.

```bash
git add src/ tests/ docs/ changelog/
git commit -m "feat: PTY sessions: create, connect, input, resize, kill (CLM-0047..0049)"
```

---

### Task 4: Directory watching (CLM-0063, 0135, 0136)

**Files:**
- Modify: `src/envd/mod.rs` (version constants), `src/envd/filesystem_pb.rs`, `src/filesystem.rs`, `src/lib.rs`
- Create: `tests/envd_watch.rs`
- Modify: `tests/live_files.rs`, `docs/protocol-notes.md`, `docs/verification/s4-evidence-map.md`

- [ ] **Step 1: Write the failing mock tests**

Create `tests/envd_watch.rs`:

```rust
//! Mock tests for `Filesystem::watch_dir` (`filesystem.Filesystem/WatchDir`).

mod common;

use common::{decode_request_envelope, end_stream, message, sandbox, sandbox_with_version, stream_response};
use e2b_client::{Error, FilesystemEventType, WatchOptions};
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer};

const ROUTE: &str = "/filesystem.Filesystem/WatchDir";

#[tokio::test]
async fn watch_dir_sends_options_and_maps_events() {
    let server = MockServer::start().await;
    let sandbox = sandbox_with_version(&server, "0.6.4").await;
    Mock::given(method("POST"))
        .and(path(ROUTE))
        .respond_with(stream_response(&[
            message(json!({"start": {}})),
            message(json!({"filesystem": {"name": "a.txt", "type": "EVENT_TYPE_CREATE"}})),
            message(json!({"keepalive": {}})),
            message(json!({"filesystem": {"name": "a.txt", "type": "EVENT_TYPE_UNSPECIFIED"}})),
            message(json!({"filesystem": {"name": "a.txt", "type": 2,
                "entry": {"name": "a.txt", "type": "FILE_TYPE_FILE", "path": "/w/a.txt", "size": "3"}}})),
            end_stream(json!({})),
        ]))
        .mount(&server)
        .await;

    let options = WatchOptions::default().recursive(true).include_entry(true).allow_network_mounts(true);
    let mut watch = sandbox.files().watch_dir("/w", &options).await.expect("watch");
    let create = watch.next_event().await.expect("event").expect("create");
    let write = watch.next_event().await.expect("event").expect("write");
    let end = watch.next_event().await.expect("clean end");

    assert_eq!((create.name.as_str(), create.event_type), ("a.txt", FilesystemEventType::Create));
    assert!(create.entry.is_none());
    assert_eq!(write.event_type, FilesystemEventType::Write);
    assert_eq!(write.entry.expect("entry").size, 3);
    assert!(end.is_none());
    let requests = server.received_requests().await.expect("requests");
    let watch_request = requests.iter().find(|r| r.url.path() == ROUTE).expect("WatchDir request");
    assert_eq!(
        decode_request_envelope(&watch_request.body),
        json!({"path": "/w", "recursive": true, "includeEntry": true, "allowNetworkMounts": true})
    );
}

#[tokio::test]
async fn watch_dir_options_are_refused_on_old_envd_without_a_request() {
    for (version, options) in [
        ("0.1.3", WatchOptions::default().recursive(true)),
        ("0.6.2", WatchOptions::default().include_entry(true)),
        ("0.6.3", WatchOptions::default().allow_network_mounts(true)),
    ] {
        // One server per version: wiremock answers with the first matching
        // mock, so a second create mock on the same server would never be used.
        let server = MockServer::start().await;
        let sandbox = sandbox_with_version(&server, version).await;
        let err = sandbox.files().watch_dir("/w", &options).await.expect_err(version);
        assert!(matches!(err, Error::InvalidArgument(_)), "{version}: {err:?}");
        let requests = server.received_requests().await.expect("requests");
        assert!(requests.iter().all(|r| r.url.path() != ROUTE), "{version}: a WatchDir request was sent");
    }
}

#[tokio::test]
async fn watch_dir_must_open_with_a_start_event() {
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    Mock::given(method("POST"))
        .and(path(ROUTE))
        .respond_with(stream_response(&[
            message(json!({"filesystem": {"name": "x", "type": "EVENT_TYPE_WRITE"}})),
            end_stream(json!({})),
        ]))
        .mount(&server)
        .await;

    let err = sandbox.files().watch_dir("/w", &WatchOptions::default()).await.expect_err("no start");
    assert!(matches!(err, Error::Protocol(_)), "{err:?}");
}
```

Run: `cargo test --test envd_watch` — Expected: compile errors.

> **Corrected (issue #33, 2026-10-27).** The shipped `tests/envd_watch.rs` is a superset of the code above. The first test also sets `.user("root")`, mounts the mock with `expect(1)`, checks the write's `entry.path`, and asserts the `application/connect+json` content type, `Authorization: Basic cm9vdDo=`, `Keepalive-Ping-Interval: 50` and `Connect-Timeout-Ms: 60000`. A fourth test, `watch_dir_open_is_bounded_by_the_request_timeout`, delays the answer 5 s under a 200 ms request timeout and expects `Error::Timeout(200 ms)`. Unit tests were added too: `src/envd/filesystem_pb.rs::tests::{watch_dir_request_serializes_camel_case, watch_dir_response_members_parse}` and `src/filesystem.rs::tests::{event_type_mapping_drops_unspecified_and_unknown, watch_options_default_and_version_gates}`. Read the file rather than this snippet.

- [ ] **Step 2: Wire types and version gates**

`src/envd/mod.rs`, next to `CLOSE_STDIN`:

```rust
    /// envd 0.1.4 added recursive watching (`e2b/envd/versions.py:3`).
    pub(crate) const RECURSIVE_WATCH: Self = Self::new(0, 1, 4);
    /// envd 0.6.3 added entry info on watch events (`e2b/envd/versions.py:10`).
    pub(crate) const FS_EVENT_ENTRY_INFO: Self = Self::new(0, 6, 3);
    /// envd 0.6.4 allowed watching network mounts (`e2b/envd/versions.py:11`).
    pub(crate) const WATCH_NETWORK_MOUNTS: Self = Self::new(0, 6, 4);
```

`src/envd/filesystem_pb.rs`:

```rust
/// `filesystem.WatchDirRequest`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WatchDirRequest<'a> {
    pub(crate) path: &'a str,
    pub(crate) recursive: bool,
    pub(crate) include_entry: bool,
    pub(crate) allow_network_mounts: bool,
}

/// `filesystem.WatchDirResponse`: a oneof of `start`, `filesystem`, `keepalive`.
#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct WatchDirResponse {
    /// Present (as an empty object) on the opening event. The crate's
    /// `connect::Empty` (`use crate::envd::connect::Empty;`), which is
    /// `Clone` and skips unknown fields.
    #[serde(default)]
    pub(crate) start: Option<Empty>,
    #[serde(default)]
    pub(crate) filesystem: Option<FilesystemEventWire>,
}

/// `filesystem.FilesystemEvent`.
#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct FilesystemEventWire {
    #[serde(default)]
    pub(crate) name: String,
    #[serde(default, rename = "type")]
    pub(crate) event_type: Option<EnumValue>,
    #[serde(default)]
    pub(crate) entry: Option<EntryInfoWire>,
}
```

- [ ] **Step 3: Public API in `src/filesystem.rs`**

```rust
/// The SDKs' default bound on a watch (`timeout=60`).
pub const DEFAULT_WATCH_TIMEOUT: Duration = Duration::from_secs(60);

/// What happened to a watched entry (`filesystem.EventType`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FilesystemEventType {
    /// Created.
    Create,
    /// Written to.
    Write,
    /// Removed.
    Remove,
    /// Renamed.
    Rename,
    /// Permissions changed.
    Chmod,
}

impl FilesystemEventType {
    /// Maps the proto enum by name or number; `None` for unspecified or
    /// unknown types, which the SDKs drop (`e2b/sandbox/filesystem/watch_handle.py:36-46`).
    fn from_wire(value: Option<&EnumValue>) -> Option<Self> {
        match value? {
            EnumValue::Name(name) => match name.as_str() {
                "EVENT_TYPE_CREATE" => Some(Self::Create),
                "EVENT_TYPE_WRITE" => Some(Self::Write),
                "EVENT_TYPE_REMOVE" => Some(Self::Remove),
                "EVENT_TYPE_RENAME" => Some(Self::Rename),
                "EVENT_TYPE_CHMOD" => Some(Self::Chmod),
                _ => None,
            },
            EnumValue::Number(1) => Some(Self::Create),
            EnumValue::Number(2) => Some(Self::Write),
            EnumValue::Number(3) => Some(Self::Remove),
            EnumValue::Number(4) => Some(Self::Rename),
            EnumValue::Number(5) => Some(Self::Chmod),
            EnumValue::Number(_) => None,
        }
    }
}

/// One change in a watched directory.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct FilesystemEvent {
    /// The entry's path relative to the watched directory.
    pub name: String,
    /// What happened.
    pub event_type: FilesystemEventType,
    /// The entry's info, when the watch asked for it and envd had it
    /// (often `None` for removals and renames away).
    pub entry: Option<EntryInfo>,
}
```

`WatchOptions` with private fields `recursive`, `include_entry`, `allow_network_mounts`, `user: Option<String>`, `timeout: Option<Duration>` (default `Some(DEFAULT_WATCH_TIMEOUT)`), a builder method per field and a doc example. Then:

```rust
    /// Watches a directory for changes (`WatchDir`).
    ///
    /// The watch lasts until the handle is dropped or `options`' timeout
    /// passes. Python note: dropping the [`WatchHandle`] is the SDK's
    /// `stop()`.
    ///
    /// # Arguments
    ///
    /// * `path` - the directory.
    /// * `options` - recursion, entry info, network mounts, user and timeout.
    ///
    /// # Returns
    ///
    /// A [`WatchHandle`] to pull events from.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidArgument`] when an option needs a newer envd (nothing
    /// is sent), [`Error::Timeout`] if the watch does not open within the
    /// request timeout, [`Error::Protocol`] if it does not open with a start
    /// event, [`Error::Rpc`] (`not_found` for a missing directory) or
    /// [`Error::Transport`].
    pub async fn watch_dir(&self, path: &str, options: &WatchOptions) -> Result<WatchHandle> {
        // Private `WatchOptions::check_supported(&self, version) -> Result<()>`:
        // the three (wanted, needed version, what) gates, and the first one
        // that fails gives `Error::InvalidArgument("{what} needs envd {needs}
        // or newer; this sandbox runs envd {version}")`.
        options.check_supported(self.envd.version())?;
        let request = WatchDirRequest {
            path,
            recursive: options.recursive,
            include_entry: options.include_entry,
            allow_network_mounts: options.allow_network_mounts,
        };
        let mut headers = self.envd.user_headers(options.user.as_deref())?;
        headers.extend(keepalive_headers());
        let stream = self
            .envd
            .within_request_timeout(async {
                let mut stream: ConnectStream<WatchDirResponse> = self
                    .envd
                    .server_stream(filesystem_pb::SERVICE, "WatchDir", &request, headers, options.timeout)
                    .await?;
                match stream.next().await? {
                    Some(WatchDirResponse { start: Some(_), .. }) => Ok(stream),
                    _ => Err(Error::Protocol("WatchDir stream did not open with a start event".into())),
                }
            })
            .await?;
        Ok(WatchHandle { stream })
    }
```

```rust
/// An open directory watch. Pull events with [`next_event`](Self::next_event);
/// drop it to stop watching.
#[derive(Debug)]
pub struct WatchHandle {
    stream: ConnectStream<WatchDirResponse>,
}

impl WatchHandle {
    /// Waits for the next filesystem event. Keepalives and event types this
    /// crate does not know are skipped.
    ///
    /// # Returns
    ///
    /// `Some(event)` per change, `None` once envd ends the watch cleanly.
    ///
    /// # Errors
    ///
    /// [`Error::Timeout`] when the watch timeout passes, [`Error::Rpc`] if
    /// envd ends the watch with an error, [`Error::Protocol`],
    /// [`Error::Decode`] or [`Error::Transport`].
    pub async fn next_event(&mut self) -> Result<Option<FilesystemEvent>> {
        while let Some(response) = self.stream.next().await? {
            let Some(event) = response.filesystem else { continue };
            let Some(event_type) = FilesystemEventType::from_wire(event.event_type.as_ref()) else {
                continue;
            };
            return Ok(Some(FilesystemEvent {
                name: event.name,
                event_type,
                entry: event.entry.map(EntryInfo::from),
            }));
        }
        Ok(None)
    }
}
```

Re-export `DEFAULT_WATCH_TIMEOUT, FilesystemEvent, FilesystemEventType, WatchHandle, WatchOptions` from `src/lib.rs`, and mention watching in the crate docs.

- [ ] **Step 4: Live test**

```rust
#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0063_watch_dir_reports_a_created_file() {
    let sandbox = LiveSandbox::new().await;
    let files = sandbox.files();
    files.make_dir("/tmp/watched", None).await.expect("make_dir");
    let mut watch = files
        .watch_dir("/tmp/watched", &WatchOptions::default())
        .await
        .expect("watch_dir");

    files.write("/tmp/watched/new.txt", "x", None).await.expect("write");
    // Events can arrive in a burst (create, write, chmod...), so look for
    // the create of this file rather than taking the first event.
    let mut seen = Vec::new();
    let found = tokio::time::timeout(Duration::from_secs(20), async {
        while let Some(event) = watch.next_event().await.expect("next_event") {
            if event.event_type == FilesystemEventType::Create && event.name == "new.txt" {
                return true;
            }
            seen.push(event);
        }
        false
    })
    .await;

    assert!(
        matches!(found, Ok(true)),
        "no create event for new.txt within 20 s (envd {}, watch ended: {}); other events: {seen:?}",
        sandbox.envd_version(),
        matches!(found, Ok(false)),
    );
}
```

- [ ] **Step 5: Docs, gates, live check, changelog, commit**

Add a "Directory watching" section to protocol-notes; remove watching from "Not implemented". Evidence-map rows: CLM-0135 → `watch_dir_sends_options_and_maps_events`, `watch_dir_must_open_with_a_start_event`; CLM-0136 → `watch_dir_options_are_refused_on_old_envd_without_a_request`.

> **Corrected (issue #33, 2026-10-27).** The rows also name the unit tests from the Step 1 note: CLM-0135 the two `filesystem_pb` tests and `event_type_mapping_drops_unspecified_and_unknown`, CLM-0136 `watch_options_default_and_version_gates`. Gates, the live test on hosted, `changelog/<ts>-m2-task-04-watch-dir.md`.

```bash
git add src/ tests/ docs/ changelog/
git commit -m "feat: Filesystem::watch_dir with recursive, entry and network-mount options (CLM-0063)"
```

---

### Task 5: Batch write and the write options (CLM-0062, 0137, 0138)

**Files:**
- Modify: `Cargo.toml`, `src/envd/mod.rs`, `src/filesystem.rs`, `src/lib.rs`
- Create: `tests/envd_write.rs`
- Modify: `tests/envd_api.rs` (only if `write`'s refactor changes an assertion), `tests/live_files.rs`, `docs/protocol-notes.md`, `docs/verification/s4-evidence-map.md`

- [ ] **Step 1: Dependencies**

```toml
# Gzip-compresses uploads on the fly (the SDKs' `gzip=True` upload).
async-compression = { version = "0.4.42", features = ["tokio", "gzip"] }
futures-util = { version = "0.3.34", default-features = false, features = ["alloc"] }
# Turns an AsyncRead into a byte stream for request bodies.
tokio-util = { version = "0.7.20", default-features = false, features = ["io"] }
```

> **Corrected (issue #34, 2026-10-27).** Task 5 does not add tokio's `io-util` feature: nothing in it needs one. `std::io::Cursor<Bytes>` is `AsyncRead` and `AsyncBufRead` without it (`tokio-1.54.1/src/io/async_buf_read.rs:111`). Task 6 adds `io-util`, for `BufReader`. `Cargo.lock` pins async-compression 0.4.42 (with compression-codecs 0.4.38 and compression-core 0.4.32), the versions the Verified facts cite; a plain `cargo add` resolves 0.4.51.

- [ ] **Step 2: Write the failing mock tests**

Create `tests/envd_write.rs`. Helpers:

```rust
mod common;

use async_compression::tokio::bufread::GzipDecoder;
use common::{sandbox, sandbox_with_version};
use e2b_client::{WriteEntry, WriteOptions};
use serde_json::json;
use tokio::io::AsyncReadExt;
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

fn has_no_path_param(request: &Request) -> bool {
    request.url.query_pairs().all(|(key, _)| key != "path")
}

fn written(paths: &[&str]) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(
        paths
            .iter()
            .map(|p| json!({"name": p.rsplit('/').next(), "type": "file", "path": p}))
            .collect::<Vec<_>>(),
    )
}

async fn gunzip(body: &[u8]) -> Vec<u8> {
    let mut plain = Vec::new();
    GzipDecoder::new(body).read_to_end(&mut plain).await.expect("gunzip");
    plain
}
```

Tests:

- `write_files_multipart_sends_one_request_without_a_path_param` (CLM-0137): two entries; one mock `POST /files` matching `has_no_path_param`, `body_string_contains("name=\"file\"; filename=\"/w/a.txt\"")` and the same for `/w/b.txt`, `.expect(1)`, answering `written(&["/w/a.txt", "/w/b.txt"])`; the result has two `WriteInfo`s in order.
- `write_files_with_one_entry_sends_the_path_param` (CLM-0137): `query_param("path", "/w/a.txt")`.
- `write_files_with_no_entries_sends_nothing`: returns an empty `Vec`; only the create request reached the server.
- `octet_stream_write_sends_one_request_per_file` (CLM-0138): envd 0.5.7, `WriteOptions::default().octet_stream(true)`, two mocks with `query_param("path", ..)` and `header("content-type", "application/octet-stream")`, `.expect(1)` each; the body equals the entry's bytes.
- `gzip_write_compresses_the_body_and_sets_content_encoding` (CLM-0138): `gzip(true)` alone (it implies octet-stream); `header("content-encoding", "gzip")`; then `gunzip(&request.body)` equals the data.
- `octet_stream_and_gzip_fall_back_to_plain_multipart_before_envd_0_5_7` (CLM-0138): envd 0.5.6, `gzip(true)`; the request is multipart, has no `content-encoding`, and contains the plain data.

Run: `cargo test --test envd_write` — Expected: compile errors.

- [ ] **Step 3: Implementation**

`src/envd/mod.rs`: `pub(crate) const OCTET_STREAM_UPLOAD: Self = Self::new(0, 5, 7);` (`e2b/envd/versions.py:8`).

> **Corrected (issue #31, 2026-10-27).** Task 2 removed `EnvdClient::request_timeout()`, because its only caller moved to `within_request_timeout`. The upload code below calls `self.envd.request_timeout()`, so re-add the accessor to `EnvdClient` in `src/envd/mod.rs`: `pub(crate) fn request_timeout(&self) -> Option<Duration> { self.request_timeout }`, with a doc comment.

`src/filesystem.rs`:

```rust
/// One file for [`Filesystem::write_files`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteEntry {
    path: String,
    data: Bytes,
}

impl WriteEntry {
    /// Creates an entry.
    ///
    /// # Arguments
    ///
    /// * `path` - where to write.
    /// * `data` - the content.
    ///
    /// # Returns
    ///
    /// The entry.
    #[must_use]
    pub fn new(path: impl Into<String>, data: impl Into<Bytes>) -> Self {
        Self { path: path.into(), data: data.into() }
    }
}

/// How to upload: as which user, and with which encoding.
///
/// By default uploads are `multipart/form-data`, the SDKs' default for
/// in-memory data.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WriteOptions {
    user: Option<String>,
    octet_stream: bool,
    gzip: bool,
}
```

Builder methods `user`, `octet_stream(bool)` and `gzip(bool)` (gzip implies octet-stream; both need envd ≥ 0.5.7, and fall back to uncompressed multipart on older envd, as the SDKs do), each documented.

```rust
    /// Writes several files (`POST /files`), creating parent directories and
    /// overwriting existing files.
    ///
    /// Multipart (the default) sends all files in one request; octet-stream
    /// and gzip send one request per file, concurrently.
    ///
    /// # Arguments
    ///
    /// * `entries` - the files.
    /// * `options` - user and encoding.
    ///
    /// # Returns
    ///
    /// What envd reports about each written file, in envd's order. An empty
    /// `entries` returns an empty list without a request.
    ///
    /// # Errors
    ///
    /// As [`Filesystem::write`].
    pub async fn write_files(&self, entries: &[WriteEntry], options: &WriteOptions) -> Result<Vec<WriteInfo>> {
        if entries.is_empty() {
            return Ok(Vec::new());
        }
        if self.uses_octet_stream(options) {
            let uploads = entries.iter().map(|entry| {
                let body = if options.gzip {
                    gzip_body(std::io::Cursor::new(entry.data.clone()))
                } else {
                    reqwest::Body::from(entry.data.clone())
                };
                self.post_octet_stream(&entry.path, body, options, self.envd.request_timeout())
            });
            let written = futures_util::future::try_join_all(uploads).await?;
            return Ok(written.into_iter().flatten().collect());
        }
        let form = entries.iter().fold(Form::new(), |form, entry| {
            // `u64::try_from` cannot fail for an in-memory buffer on 64-bit targets.
            let length = u64::try_from(entry.data.len()).unwrap_or(u64::MAX);
            form.part("file", Part::stream_with_length(entry.data.clone(), length).file_name(entry.path.clone()))
        });
        let single_path = match entries {
            [only] => Some(only.path.as_str()),
            _ => None,
        };
        self.post_multipart(form, single_path, options, self.envd.request_timeout()).await
    }
```

(`Bytes::clone` is a reference-count bump, not a copy.)

Private helpers shared by Tasks 5 to 7:

- `fn uses_octet_stream(&self, options: &WriteOptions) -> bool { (options.octet_stream || options.gzip) && self.envd.version() >= EnvdVersion::OCTET_STREAM_UPLOAD }`
- `fn upload_query<'a>(&self, path: Option<&'a str>, user: Option<&'a str>) -> Vec<(&'static str, &'a str)>`: `path` when given, `username` from `effective_user`, borrowed rather than copied. `file_query(path, user)` stays for reads and is `upload_query(Some(path), user)`.
- `async fn post_octet_stream(&self, path: &str, body: reqwest::Body, options: &WriteOptions, timeout: Option<Duration>) -> Result<Vec<WriteInfo>>`: `Content-Type: application/octet-stream`, and `Content-Encoding: gzip` when `options.gzip`.
- `async fn post_multipart(&self, form: Form, path: Option<&str>, options: &WriteOptions, timeout: Option<Duration>) -> Result<Vec<WriteInfo>>`.
- `async fn written_entries(response: reqwest::Response) -> Result<Vec<WriteInfo>>`: a non-2xx status gives `envd_http_error`; an empty array gives `no_written_file()`, that is `Error::Protocol("envd reported no written file")`. Each entry converts through `impl From<UploadedEntry> for WriteInfo`.
- `fn no_written_file() -> Error`: the shared "no written file" error, also used by `write` (and by Task 6's `write_stream`).
- `fn gzip_body(source: impl AsyncBufRead + Send + 'static) -> reqwest::Body { reqwest::Body::wrap_stream(ReaderStream::new(GzipEncoder::new(source))) }`.

The `timeout` parameter exists for Task 6. Add `EnvdClient::http_request_with_timeout(method, path, timeout: Option<Duration>)`, and make `http_request` call it with `self.request_timeout`.

`write` becomes:

```rust
    pub async fn write(&self, path: &str, data: impl Into<Bytes>, user: Option<&str>) -> Result<WriteInfo> {
        let options = WriteOptions {
            user: user.map(str::to_owned),
            ..WriteOptions::default()
        };
        self.write_files(&[WriteEntry::new(path, data)], &options)
            .await?
            .into_iter()
            .next()
            .ok_or_else(no_written_file)
    }
```

Re-export `WriteEntry, WriteOptions`.

- [ ] **Step 4: Live test**

`clm_0062_write_files_writes_every_file`: `write_files` of `/tmp/batch/a.txt` = `"alpha"` and `/tmp/batch/sub/b.bin` = bytes `0..=255`, with default options; two `WriteInfo`s come back; reading each file returns its content.

- [ ] **Step 5: Docs, gates, live check, changelog, commit**

Add an "Uploads" section to protocol-notes (multipart batch, octet-stream, gzip, the 0.5.7 fallback), and change "octet-stream and gzip uploads" in "Not implemented" to say what remains (streamed, Task 6). Evidence-map rows for CLM-0137 and CLM-0138 (CLM-0138 is `partial` until Task 6 adds the streamed path; say so). `changelog/<ts>-m2-task-05-write-files.md`.

```bash
git add Cargo.toml Cargo.lock src/ tests/ docs/ changelog/
git commit -m "feat: Filesystem::write_files with octet-stream and gzip encodings (CLM-0062)"
```

---

### Task 6: Streamed upload and download (CLM-0066, 0128, 0142)

**Files:**
- Modify: `Cargo.toml` (tokio `io-util`, see below), `src/filesystem.rs`, `src/lib.rs`
- Create: `tests/envd_stream_files.rs`
- Modify: `tests/live_files.rs`, `docs/protocol-notes.md`, `docs/verification/s4-evidence-map.md`

- [ ] **Step 1: Write the failing mock tests**

`tests/envd_stream_files.rs`:

- `write_stream_sends_an_octet_stream_body` (CLM-0138 streamed path): envd 0.5.7; `write_stream("/w/big.bin", std::io::Cursor::new(vec![7u8; 300_000]), &WriteOptions::default())`; the mock matches `content-type: application/octet-stream`, and its body equals the 300 000 bytes.
- `write_stream_with_gzip_compresses_on_the_fly`: `gzip(true)`; `content-encoding: gzip`; the gunzipped body equals the input.
- `write_stream_falls_back_to_multipart_before_envd_0_5_7`: envd 0.5.6; multipart, field `file`, filename the path, with the data in the body.
- `write_stream_is_not_bounded_by_the_request_timeout` (CLM-0066, 0142): `common::client_with_timeout(&server, Some(Duration::from_secs(1)))` builds the sandbox (via `common::sandbox_from_json_with_client(server, &client, created)`, which `sandbox_from_json` now calls with `client_for`). The mock answers after `set_delay(Duration::from_secs(3))`; `write_stream` succeeds, and `write` of the same data fails with a timeout transport error.
- `read_stream_yields_the_body_in_chunks` (CLM-0142): `GET /files` with `query_param("path", "/w/big.bin")` answers 1 MiB; collecting `next_chunk` until `None` gives the 1 MiB.
- `read_stream_is_not_bounded_by_the_request_timeout` (CLM-0142): as above, with a 3 s delay and a 1 s request timeout: `read_stream` succeeds and `read` fails.

> **Corrected (issue #35, 2026-10-27).** The timeout tests use a 1 s request timeout and a 3 s delay instead of 100 ms and 300 ms, so creating the sandbox stays well inside the timeout on a slow CI runner. One more test shipped: `write_stream_surfaces_a_reader_error_as_transport` (a reader that fails on its first read gives `Error::Transport`, as the rustdoc promises).
- `read_stream_of_a_missing_file_is_envd_not_found`: 404 with `{"message": "no such file"}` gives `Error::Envd { status: 404, .. }` from `read_stream` itself, not from the first chunk.

Run: `cargo test --test envd_stream_files` — Expected: compile errors.

- [ ] **Step 2: Implementation**

```rust
    /// Uploads a file from a reader without loading it into memory
    /// (`POST /files`, `application/octet-stream`).
    ///
    /// No request timeout applies, because a deadline would cut off any
    /// transfer that outlasts it; this matches the SDKs. On envd older than
    /// 0.5.7 the upload falls back to a streamed multipart body, uncompressed.
    ///
    /// # Arguments
    ///
    /// * `path` - where to write.
    /// * `reader` - the content, read to its end (a `tokio::fs::File`, a
    ///   socket, `&'static [u8]`...).
    /// * `options` - user and gzip (`octet_stream` is implied).
    ///
    /// # Returns
    ///
    /// What envd reports about the written file.
    ///
    /// # Errors
    ///
    /// As [`Filesystem::write`]; a read error from `reader` surfaces as
    /// [`Error::Transport`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use e2b_client::{Sandbox, WriteOptions};
    ///
    /// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
    /// // Any `AsyncRead` works: a `tokio::fs::File`, a socket, a byte slice.
    /// let data: &'static [u8] = b"a,b\n1,2\n";
    /// sandbox.files().write_stream("/home/user/data.csv", data, &WriteOptions::default().gzip(true)).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn write_stream(
        &self,
        path: &str,
        reader: impl AsyncRead + Send + 'static,
        options: &WriteOptions,
    ) -> Result<WriteInfo> {
        let reader = BufReader::new(reader);
        let written = if self.envd.version() >= EnvdVersion::OCTET_STREAM_UPLOAD {
            let body = if options.gzip {
                gzip_body(reader)
            } else {
                reqwest::Body::wrap_stream(ReaderStream::new(reader))
            };
            self.post_octet_stream(path, body, options, None).await?
        } else {
            let part = Part::stream(reqwest::Body::wrap_stream(ReaderStream::new(reader))).file_name(path.to_owned());
            // `post_multipart` reads only the user from `options`, so gzip is
            // dropped on this path, as in the SDKs: multipart is never compressed.
            self.post_multipart(Form::new().part("file", part), Some(path), options, None).await?
        };
        written.into_iter().next().ok_or_else(no_written_file)
    }
```

> **Corrected (issue #34, 2026-10-27).** `BufReader` needs tokio's `io-util` feature, which Task 5 did not add. Add it to the `[dependencies]` tokio line in `Cargo.toml`: `features = ["time", "rt", "sync", "io-util"]`, and import `tokio::io::{AsyncRead, BufReader}` (`AsyncBufRead`, `ReaderStream` and `GzipEncoder` are already imported). `gzip_body` takes `impl AsyncBufRead + Send + 'static`, so `BufReader<impl AsyncRead + Send + 'static>` fits it directly.

The doc example uses a byte slice rather than `tokio::fs::File`, because the library does not enable tokio's `fs` feature.

> **Corrected (issue #35, 2026-10-27).** The fallback passes `options` to `post_multipart` unchanged instead of a clone with `gzip: false`: `post_multipart` never reads `gzip`, so the clone was dead weight. Both branches pass `options` through, so Task 7's metadata headers, added inside `post_multipart` and `post_octet_stream`, reach `write_stream` with no change there beyond the `metadata_headers` check at its top. Shipped code: `src/filesystem.rs::Filesystem::write_stream`.

`post_octet_stream` must set `Content-Encoding: gzip` only from `options.gzip`. Taking `octet_stream` as implied, `write_stream` always uses octet-stream when envd allows it.

```rust
    /// Downloads a file as a stream of chunks (`GET /files`), without holding
    /// the whole file in memory.
    ///
    /// No request timeout applies to the transfer. Bound it yourself with
    /// `tokio::time::timeout` around [`FileStream::next_chunk`] if needed.
    ///
    /// # Arguments
    ///
    /// * `path` - the file to read.
    /// * `user` - the user to read as.
    ///
    /// # Returns
    ///
    /// A [`FileStream`] positioned at the start of the file.
    ///
    /// # Errors
    ///
    /// [`Error::Envd`] (404 when the file does not exist) or
    /// [`Error::Transport`].
    pub async fn read_stream(&self, path: &str, user: Option<&str>) -> Result<FileStream> {
        let response = self
            .envd
            .http_request_with_timeout(Method::GET, FILES_ROUTE, None)
            .query(&self.file_query(path, user))
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(envd_http_error(response).await);
        }
        Ok(FileStream { response })
    }
```

```rust
/// A file being downloaded, from [`Filesystem::read_stream`].
///
/// Python note: the SDK's `AsyncFileStreamReader`. Pull chunks with
/// `while let Some(chunk) = stream.next_chunk().await? { ... }`; dropping the
/// stream closes the connection.
#[derive(Debug)]
pub struct FileStream {
    response: reqwest::Response,
}

impl FileStream {
    /// Waits for the next chunk of the file.
    ///
    /// # Returns
    ///
    /// `Some(chunk)` until the file ends, then `None`.
    ///
    /// # Errors
    ///
    /// [`Error::Transport`] if the connection fails mid-file.
    pub async fn next_chunk(&mut self) -> Result<Option<Bytes>> {
        Ok(self.response.chunk().await?)
    }
}
```

Re-export `FileStream`.

- [ ] **Step 3: Live tests**

- `clm_0066_write_stream_uploads_more_than_it_buffers`: upload 64 MiB from `tokio::io::repeat(b'z').take(64 << 20)` (never in memory) twice: to `/tmp/plain.bin` with default options, and to `/tmp/gz.bin` with `gzip(true)`. `get_info(..).size` is `64 << 20` for both, and `run("tr -d z < /tmp/gz.bin | wc -c")` prints `0`, so the server stored the decompressed bytes.
- `clm_0128_read_stream_downloads_in_chunks`: `run("yes 0123456789abcde | head -c 8388608 > /tmp/big.txt")`. Read with `read_stream`, counting chunks and checking each chunk against the repeating 16-byte pattern at its offset, without keeping them. The total is 8 MiB, and more than one chunk arrived.

- [ ] **Step 4: Docs, gates, live check, changelog, commit**

protocol-notes: "Uploads" gains the streamed rows, and a "Streamed download" row goes under the file API; remove the streaming upload from "Not implemented". Evidence-map: CLM-0138 becomes `yes`, plus a CLM-0142 row. `changelog/<ts>-m2-task-06-streamed-files.md`.

```bash
git add src/ tests/ docs/ changelog/
git commit -m "feat: streamed and gzip uploads from AsyncRead, streamed downloads (CLM-0066, CLM-0128)"
```

---

### Task 7: File metadata (CLM-0129, 0139)

**Files:**
- Modify: `src/envd/mod.rs`, `src/envd/filesystem_pb.rs`, `src/filesystem.rs`
- Modify: `tests/envd_write.rs`, `tests/live_files.rs`, `docs/protocol-notes.md`, `docs/verification/s4-evidence-map.md`

- [ ] **Step 1: Write the failing mock tests (in `tests/envd_write.rs`)**

- `metadata_is_sent_as_x_metadata_headers_on_every_upload` (CLM-0139): envd 0.6.2, `WriteOptions::default().metadata("team", "m2").metadata("Run", "1")`. A multipart `write_files` and an octet-stream `write_files` both carry `x-metadata-team: m2` and `x-metadata-run: 1` (HTTP header names are case-insensitive, so match lowercase).
- `invalid_metadata_is_rejected_before_sending` (CLM-0139): the keys `""`, `"a b"` and `"é"`, and the value `"tab\there"`, each give `Error::InvalidArgument`, and no `/files` request is made.
- `metadata_is_refused_before_envd_0_6_2` (CLM-0139): envd 0.6.1 gives `InvalidArgument` with no request.
- `upload_response_and_stat_report_metadata` (CLM-0129 wire part): the upload response `[{"name":"a","type":"file","path":"/a","metadata":{"team":"m2"}}]` gives `WriteInfo.metadata == {"team": "m2"}`; a `Stat` response with `"metadata": {"team": "m2"}` gives the same in `EntryInfo.metadata`; and without the field, both maps are empty.

- [ ] **Step 2: Implementation**

- `EnvdVersion::FILE_METADATA = 0.6.2` (`e2b/envd/versions.py:9`).
- `EntryInfoWire` and `UploadedEntry` gain `#[serde(default)] metadata: HashMap<String, String>` (carry `UploadedEntry`'s into `WriteInfo` in the `impl From<UploadedEntry> for WriteInfo` that Task 5 added). (Note from issue #33: `FilesystemEventWire.entry` is an `EntryInfoWire` mapped through `EntryInfo::from`, so `include_entry` watch events carry the metadata with no further change; extend `src/envd/filesystem_pb.rs::tests::watch_dir_response_members_parse` if you want that pinned.) `EntryInfo` and `WriteInfo` gain `pub metadata: HashMap<String, String>` ("user metadata stored as `user.e2b.*` extended attributes; empty when none is set").
- `WriteOptions` gains `metadata: BTreeMap<String, String>` (sorted, so the headers come out in a stable order for tests), set with `pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self`.
- `fn metadata_headers(&self, options: &WriteOptions) -> Result<HeaderMap>`: empty map gives empty headers; else check the version, then validate each pair (a key is non-empty and every byte is in `b"!#$%&'*+-.^_`|~"` or ASCII alphanumeric; a value has every byte in `0x20..=0x7e`), and build `x-metadata-<key>` headers with `HeaderName::from_bytes` / `HeaderValue::from_str`, mapping failures to `InvalidArgument`. It is called once at the top of `write_files` and `write_stream`, before any request, and the headers are added in `post_multipart` and `post_octet_stream`.

- [ ] **Step 3: Live test**

`clm_0129_write_metadata_round_trips`: if `sandbox.envd_version() < EnvdVersion::new(0, 6, 2)`, fail with a message naming the version, which tells Task 9 the template is too old (not a crate bug). Otherwise write with `metadata("team", "m2")` and check that `WriteInfo.metadata`, `get_info(..).metadata` and the `list` entry all contain `("team", "m2")`.

- [ ] **Step 4: Docs, gates, live check, changelog, commit**

Add metadata rows to protocol-notes' "Uploads" section, and a CLM-0139 evidence-map row. `changelog/<ts>-m2-task-07-file-metadata.md`.

```bash
git add src/ tests/ docs/ changelog/
git commit -m "feat: file metadata on writes, reported on WriteInfo and EntryInfo (CLM-0129)"
```

---

### Task 8: Signed upload and download URLs (CLM-0064, 0065, 0140, 0141)

**Files:**
- Modify: `Cargo.toml`, `src/envd/mod.rs`, `src/filesystem.rs` (`FILES_ROUTE` becomes `pub(crate)`), `src/sandbox.rs`, `src/lib.rs`
- Create: `tests/envd_file_urls.rs`
- Modify: `tests/live_files.rs`, `docs/protocol-notes.md`, `docs/verification/s4-evidence-map.md`

- [ ] **Step 1: Dependency**

```toml
# SHA-256 for the signatures on file URLs.
sha2 = "0.11.1"
```

- [ ] **Step 2: Write the failing tests**

Unit test in `src/sandbox.rs` (the vectors come from Verified facts):

```rust
#[test]
fn file_signature_matches_the_sdk_algorithm() {
    assert_eq!(
        file_signature("/home/user/a.txt", FileOperation::Read, None, "tok", None),
        "v1_Zkbb3914oLJLvMBzCAHQhm2ydBLxbSx2eQUR/kLdvIY"
    );
    assert_eq!(
        file_signature("/tmp/up.bin", FileOperation::Write, Some("root"), "tok", Some(1_700_000_000)),
        "v1_Y0uzWP5ST4jxa8RLcFIf65h1K6NEbMc0oox+Mgq89DY"
    );
}
```

`tests/envd_file_urls.rs`:

- `download_url_is_signed_with_the_access_token` (CLM-0140): `sandbox(&server)` (envd 0.5.7, token `ACCESS_TOKEN`). `download_url("/home/user/a b.txt", &FileUrlOptions::default())` parses with `url::Url`. It starts with `server.uri() + "/files"`, its `path` pair is `/home/user/a b.txt`, it has no `username` and no `signature_expiration`, and its `signature` equals the SDK algorithm computed in the test from `ACCESS_TOKEN` (the test computes it with `sha2` and `base64`: integration tests may use the package's regular dependencies, so no dev-dependency is needed).
- `upload_url_with_user_and_expiration` (CLM-0140): `FileUrlOptions::default().user("root").expires_in(Duration::from_secs(120))` gives `username=root` and a `signature_expiration` within 5 s of now + 120, and the signature covers `...:write:root:<token>:<expiration>`.
- `legacy_envd_signs_for_the_default_user` (CLM-0140): envd 0.3.9 without a user gives `username=user`, and the signature uses `user`.
- `unsecured_sandbox_gets_an_unsigned_url_and_rejects_expiration` (CLM-0141): `sandbox_from_json` with `"envdAccessToken": null` gives a URL with no `signature`; with `expires_in` it is `Error::InvalidArgument`.

- [ ] **Step 3: Implementation**

`src/envd/mod.rs`: `EnvdClient` keeps `access_token: Option<SecretString>` (cloned from the target), with `pub(crate) fn access_token(&self) -> Option<&SecretString>`. `direct_base` moves out of the `#[cfg(test)]` block. Check that the `Debug` derive on `EnvdClient` still hides the token: `SecretString`'s `Debug` prints `[REDACTED]`, and `debug_output_omits_the_api_key_and_access_tokens` must still pass.

`src/sandbox.rs`:

```rust
/// Options for [`Sandbox::upload_url`] and [`Sandbox::download_url`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileUrlOptions {
    user: Option<String>,
    expires_in: Option<Duration>,
}
```

Its builders are `user(impl Into<String>)` and `expires_in(Duration)` ("the URL stops working this long after it is made; only for secured sandboxes").

```rust
/// What a signed URL lets its holder do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileOperation {
    Read,
    Write,
}

impl FileOperation {
    fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
        }
    }
}

/// `v1_` + unpadded base64 of SHA-256 over
/// `<path>:<operation>:<user>:<token>[:<expiration>]` (`e2b/sandbox/signature.py:15-47`).
fn file_signature(
    path: &str,
    operation: FileOperation,
    user: Option<&str>,
    token: &str,
    expiration: Option<i64>,
) -> String {
    let user = user.unwrap_or_default();
    let raw = match expiration {
        Some(expiration) => format!("{path}:{}:{user}:{token}:{expiration}", operation.as_str()),
        None => format!("{path}:{}:{user}:{token}", operation.as_str()),
    };
    format!("v1_{}", STANDARD_NO_PAD.encode(Sha256::digest(raw.as_bytes())))
}
```

```rust
    /// A URL that downloads `path` with a plain `GET`, with no API key or
    /// headers needed. On a secured sandbox (every hosted one) it is signed
    /// with the sandbox's envd access token.
    ///
    /// # Arguments
    ///
    /// * `path` - the file.
    /// * `options` - the user to read as, and an optional expiry.
    ///
    /// # Returns
    ///
    /// The URL.
    ///
    /// # Errors
    ///
    /// [`crate::Error::InvalidArgument`] if an expiry is asked for on a
    /// sandbox without an access token, or the system clock is before 1970.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::time::Duration;
    /// use e2b_client::{FileUrlOptions, Sandbox};
    ///
    /// # fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
    /// let url = sandbox.download_url("/home/user/report.pdf",
    ///     &FileUrlOptions::default().expires_in(Duration::from_secs(600)))?;
    /// # let _ = url;
    /// # Ok(())
    /// # }
    /// ```
    pub fn download_url(&self, path: &str, options: &FileUrlOptions) -> Result<String> {
        self.file_url(path, FileOperation::Read, options)
    }
```

`upload_url` is the same with `FileOperation::Write`; its doc says to POST the file as `multipart/form-data`, field `file`.

```rust
    fn file_url(&self, path: &str, operation: FileOperation, options: &FileUrlOptions) -> Result<String> {
        let user = self.envd.effective_user(options.user.as_deref());
        let token = self.envd.access_token();
        if token.is_none() && options.expires_in.is_some() {
            return Err(Error::InvalidArgument(
                "a signature expiration needs a secured sandbox (one with an envd access token)".into(),
            ));
        }
        let mut url = Url::parse(&format!("{}{FILES_ROUTE}", self.envd.direct_base()))
            .map_err(|err| Error::InvalidConfig(format!("envd URL: {err}")))?;
        {
            // `query_pairs_mut` borrows `url` mutably until this block ends.
            let mut query = url.query_pairs_mut();
            if !path.is_empty() {
                query.append_pair("path", path);
            }
            if let Some(user) = user {
                query.append_pair("username", user);
            }
            if let Some(token) = token {
                let expiration = options.expires_in.map(expiration_from_now).transpose()?;
                query.append_pair(
                    "signature",
                    &file_signature(path, operation, user, token.expose_secret(), expiration),
                );
                if let Some(expiration) = expiration {
                    query.append_pair("signature_expiration", &expiration.to_string());
                }
            }
        }
        Ok(url.into())
    }
```

`expiration_from_now(Duration) -> Result<i64>` uses `SystemTime::now().duration_since(UNIX_EPOCH)` plus the duration, and maps a pre-1970 clock or an overflow to `InvalidArgument`. Re-export `FileUrlOptions`.

- [ ] **Step 4: Live tests**

- `clm_0065_download_url_fetches_without_credentials`: write `/tmp/shared.txt` = `"signed read"`; `reqwest::get(download_url(.., &FileUrlOptions::default().expires_in(60 s)))` gives status 200 and body `"signed read"`. Also check that the same URL with the `signature` pair removed is refused (a 4xx), which shows the signature is what grants access.
- `clm_0064_upload_url_accepts_a_multipart_post`: POST `reqwest::multipart::Form::new().part("file", Part::bytes(b"signed write".to_vec()).file_name("up.txt"))` to `upload_url("/tmp/up.txt", ..)` with a plain `reqwest::Client`; the status is a success; then `files().read_to_string("/tmp/up.txt")` is `"signed write"`.

- [ ] **Step 5: Docs, gates, live check, changelog, commit**

Add a "Signed URLs" section to protocol-notes; remove "signed file URLs" from "Not implemented". Evidence-map rows: CLM-0140 → `src/sandbox.rs::tests::file_signature_matches_the_sdk_algorithm` plus the three URL tests; CLM-0141 → `unsecured_sandbox_gets_an_unsigned_url_and_rejects_expiration`. `changelog/<ts>-m2-task-08-signed-urls.md`.

```bash
git add Cargo.toml Cargo.lock src/ tests/ docs/ changelog/
git commit -m "feat: signed upload and download URLs (CLM-0064, CLM-0065)"
```

---

### Task 9: Live run on hosted E2B

**Files:**
- Create: `docs/verification/runs/<run id>.log` (copied from `target/live-logs/`)
- Create or modify: `docs/verification/m2-live-results.md` (section "Hosted E2B")

- [ ] **Step 1: Run the full suite on the merged main**

Run: `just test-live hosted`
Expected: the log starts with `commit:` and `dirty: no` (Task 1), then one result line per `clm_*` test (57: M1's 46 plus M2's 11: 0046, 0047, 0048, 0049, 0062, 0063, 0064, 0065, 0066, 0128, 0129), then the sweep line. M1's tests are rerun on purpose, as a regression check.

- [ ] **Step 2: Check for leaks, secrets and the stamp**

The sweep must report 0 leftovers. Check the log for both keys exactly as in M1 Task 6 Step 3, including its note on unquoted `.env` values; every count must be `0`. `dirty: yes` (or `dirty: unknown`, or `commit: unknown`) means the run doesn't count: commit or clean the tree, then rerun.

- [ ] **Step 3: Summarize**

Copy the log to `docs/verification/runs/`. Start `docs/verification/m2-live-results.md` with a one-line intro, then:

```markdown
## Hosted E2B (run <run id>, <UTC time>, commit <sha>)

| Test | Claim | Result | Failure (first line) |
|---|---|---|---|
```

Every `clm_*` test gets a row. Analyse each failure: crate, test or environment. Test bugs are fixed in this PR, with a plan correction note. Crate bugs are only reported here, and the coordinator files them as M2 bug tasks (Task 11). If `clm_0129` fails because the template's envd is older than 0.6.2, record the envd version; that is an environment limit, not a crate bug.

- [ ] **Step 4: Changelog and commit**

```bash
git add docs/verification/ changelog/
git commit -m "docs: M2 live results on hosted E2B"
```

---

### Task 10: Live run on sandbawks

As Task 9, with `just test-live sandbawks` (`.env` must have `E2B_TEMPLATE_SANDBAWKS=compound-sandbox`) and a "sandbawks" section in `docs/verification/m2-live-results.md`. Use M1 Task 7's corrections: `--no-fail-fast` is already in the recipe; when the 2-sandbox warm pool runs out at `--test-threads=4`, rerun the failures one at a time. Every failure that passes on hosted is a sandbawks defect. Write its request and response evidence in the results, and list it for the coordinator to file on gfd/sandbawks under epic #673. Capacity failures (unschedulable Pods) are recorded as such, not as defects. Changelog `changelog/<ts>-m2-task-10-sandbawks-run.md`; commit `docs: M2 live results on sandbawks`.

---

### Task 11: Verdicts, docs and milestone close (coordinator)

Done by the coordinator session, which alone writes the ledger.

- [ ] **Step 1: Evidence and verdicts.** For each M2 live test, `add_evidence` (kind `demonstration`, locator `<test> @ hosted, docs/verification/runs/<log>`, stance from the result), and the same from the sandbawks run (a sandbawks refutation names gfd/sandbawks #673 and does not refute the claim). For each S4 claim CLM-0130 to CLM-0142, add `test` evidence from its evidence-map row at the merge commit. Record verdicts for CLM-0046..0049, 0062..0066, 0128..0142, then the parents CLM-0003, CLM-0004 and CLM-0005, and retag the implemented `gap` claims `m2` (keep the history; remove `gap`).
- [ ] **Step 2: Bugs.** Each refuted claim gets a bug task (Task 12, 13, ...) added under this step with a dated note, a sub-issue, and a fix and re-verification within M2, as in M1.
- [ ] **Step 3: Docs.** `README.md`: the feature list and examples (PTY, streamed input, watch, signed URLs, streamed files). `docs/protocol-notes.md`: "Not implemented" now lists only M3/M4 items, and "Verification against a live server" gains the M2 runs. `src/lib.rs`: the crate docs' bullets cover the new surface.
- [ ] **Step 4: Milestone changelog.** `changelog/<ts>-m2-milestone.md`: what shipped, the ledger counts before and after, the run ids, and the index of every plan correction by task (as in M1's).
- [ ] **Step 5: Catalog and close.** Update the knowledge-catalog concept `projects/e2b-client-rs.md` (M2 state and findings), file any sandbawks defects, then run `github-epic-close-out` on the M2 epic.
