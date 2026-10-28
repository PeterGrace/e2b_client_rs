# Milestone 4 (API stability) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the public API of `e2b-client` the one 0.2.0 will ship and keep: drop the v1 connect fallback, give errors the SDK's categories without leaking reqwest, give every call an options struct with a per-call request timeout, settle the eight follow-ups carried from M2 and M3 (#30, #52 to #57, #81), review the whole surface against the Rust API Guidelines, pin the MSRV, put an example on every public type and async method, and re-verify live on both servers.

**Architecture:** No new modules for features. The changes are signature and behaviour changes inside the existing modules: `src/error.rs` (classifiers, the opaque `TransportError`), `src/client.rs` and `src/sandbox.rs` (one connect and one pause method each), `src/filesystem.rs` (`FileOptions`, `ListOptions`, the upload cap, `is_symlink`), `src/commands.rs` and `src/pty.rs` (connect options, request timeouts, the PTY default), and `src/config.rs` (the shared timeout helpers). Every call that takes options resolves its request timeout through one helper, and every timeout setter treats zero as "no limit" through another. The API review is recorded in a new `docs/api-review.md`, enforced by a new trait test (`tests/api_traits.rs`), a new doc-example test (`tests/doc_examples.rs`) and a `[lints]` table in `Cargo.toml`.

**Tech Stack:** Rust 2024 (MSRV 1.89, CI through `dtolnay/rust-toolchain@master` with `toolchain: "1.89"`), reqwest 0.13 (now private), serde/serde_json, secrecy 0.10 (re-exported), futures-util 0.3 (`TryStreamExt::try_buffer_unordered`), tokio, wiremock, `just`, claims MCP.

**Design:** `docs/superpowers/specs/2026-10-26-claims-audit-and-publishability-design.md`, sections "Decisions" (item 5, the v1 fallback), "Claim hierarchy" (S5) and "Milestones" (M4). As for M2 and M3, the spec's "short design" for this milestone is this plan's "Decisions", "API after M4" and "Verified facts" sections.

---

## Scope

In:

- **The v1 connect fallback goes** (CLM-0168, replacing CLM-0087): `connect_sandbox` makes one request; a v2 404 is `Error::SandboxNotFound`. The spec's decision 5.
- **Error taxonomy** (CLM-0169, CLM-0170): `Error::status()` and the classifiers `is_auth`, `is_rate_limited`, `is_service_busy`, `is_not_enough_space`, `is_sandbox_unreachable`, `is_timeout`, `is_retryable`, next to `is_not_found`; `#[non_exhaustive]` on the struct variants; `Error::Transport` holds an opaque `TransportError`, so no reqwest type is public; `pub use secrecy`.
- **#81, a refused moded pause** (CLM-0171): with a `SnapshotMode`, only an "already paused" 409 is `Ok(false)`; any other 409 is `Error::Api`.
- **#52, zero timeouts** (CLM-0172): `Some(Duration::ZERO)` means no limit on every stream deadline and request timeout, as SDK `timeout=0`.
- **Options structs everywhere** (CLM-0173, CLM-0174): `FileOptions` and `ListOptions` for the filesystem; `WriteOptions` and `WatchOptions` gain a request timeout; `CommandConnectOptions` and `PtyConnectOptions` for the two `connect`s (#54); `CommandOptions` and `PtyOptions` gain a request timeout; `connect_sandbox` and `pause_sandbox` take their options structs and the `_with` twins go.
- **#55, the PTY default** (CLM-0175): no deadline by default.
- **#53, the upload cap** (CLM-0176): `WriteOptions::max_concurrency`, default 8.
- **#30, symlinks** (CLM-0177): `is_symlink` is read.
- **#56, `Pty::connect` to an exited pid** (CLM-0178): investigated on hosted E2B and pinned by a test.
- **API review** (CLM-0119, CLM-0179, CLM-0180): `#[non_exhaustive]` where types may grow, `Send`/`Sync`/`Debug` asserted by a test, `new()` and `Default` on every options type, the review recorded in `docs/api-review.md`, the lints in `Cargo.toml`.
- **MSRV and semver** (CLM-0121): `rust-version = "1.89"`, a CI job on 1.89, the semver and public-dependency policy in the README.
- **Rustdoc examples** (CLM-0120): an example on every public type and every public async method, enforced by `tests/doc_examples.rs`; the `missing_docs` lint.
- **#57, paced live runs** (CLM-0181): `just test-live-paced`.
- **The 14 claims CLM-0168 to CLM-0181**, registered while planning and committed with this plan; CLM-0119 to CLM-0121 retagged `m4`.
- Live runs on hosted E2B and on sandbawks (paced), verdicts and the milestone close.

Deferred:

| Item | Claims / issue | Goes to |
|---|---|---|
| Per-call request timeouts on the REST calls (`Client::*`, `Sandbox::{kill, pause, get_info, ...}`) and on the pid-only envd calls (`send_stdin`, `close_stdin`, `send_signal`, `kill`, `resize`, `Commands::list`) | none | unscheduled: they keep the configured request timeout; options structs can be added later without a breaking change only where one exists, so this is recorded in `docs/api-review.md` as a known limit |
| Regenerating the REST models from the OpenAPI spec (the spec's "M4 may revisit") | none | not planned: the hand-written models are cited row by row in `docs/protocol-notes.md` and live-verified |
| Typed error variants per SDK exception class | none | not planned (pgrace, 2026-10-28: classifier methods instead) |
| An idle-based PTY keep-alive | #55 | not planned (pgrace, 2026-10-28: no deadline by default instead) |
| Live CI job, publish dry-run, `publish = true`, CHANGELOG, 0.2.0 | CLM-0123, CLM-0124 | M5 |
| Volume tests on a team with volumes enabled (decides CLM-0148, CLM-0149, CLM-0153) | none | unscheduled; needs a team where `POST /volumes` is not 403 |
| The SOCKS5 egress proxy and IAM tokens (CLM-0031, CLM-0032, and with them CLM-0002) | none | unscheduled (M3's decision) |

## Decisions taken while planning (2026-10-28)

1. **Error taxonomy: classifier methods.** The `Api`, `Envd` and `Rpc` variants stay, because they say which protocol failed. `Error::status()` and seven classifiers give the SDK's exception categories. Struct variants become `#[non_exhaustive]` so fields can be added later.
2. **#81: an error when a mode was asked.** With a `SnapshotMode`, only the 409 whose message says "already paused" is `Ok(false)`; any other 409 is `Error::Api`. A pause without a mode keeps SDK parity (`Ok(false)` for every 409). Verified by mock only: hosted E2B accepts the filesystem-only pause.
3. **Options structs everywhere.** One method per operation, taking `&XOptions`. The filesystem calls take `FileOptions` (or `ListOptions`, which adds the depth); the two `connect`s take `CommandConnectOptions` and `PtyConnectOptions` (#54); `connect_sandbox` and `pause_sandbox` take `ConnectOptions` and `PauseOptions`, and the `_with` twins are removed. Every options struct that reaches envd carries a per-call request timeout, for SDK parity.
4. **Public dependencies: wrap reqwest, re-export the rest.** `Error::Transport` holds an opaque `TransportError` (`Display` and `source()` keep the detail; `is_timeout()` and `is_connect()` classify it). `pub use secrecy`, so callers name the version the crate uses. bytes 1, chrono 0.4, serde 1 (the derived impls on public types), serde_json 1, tokio 1 (`write_stream` takes `impl tokio::io::AsyncRead`) and url 2 stay in signatures, and the README's semver policy lists them with secrecy 0.10.
5. **#52: zero means no limit**, on every stream deadline and request timeout, as SDK `timeout=0` and `request_timeout=0`.
6. **#55: no PTY deadline by default.** `PtyOptions::default()` and `PtyConnectOptions::default()` set no deadline. `DEFAULT_PTY_TIMEOUT` stays exported as the SDK's value, for callers who want parity.
7. **#53: a default cap of 8 uploads in flight**, set with `WriteOptions::max_concurrency`; results stay in input order and the batch still fails fast.
8. **#57: a paced `just` recipe**, `just test-live-paced <target> [gap]`; hosted keeps `just test-live`.
9. **Follow-ups carried in:** #30, #52, #53, #54, #55, #56, #57 and #81, each its own task named with its issue number.
10. **Ledger.** As before, only the coordinator session writes `claims.json`. CLM-0168 to CLM-0181 were added while planning and are committed with this plan; CLM-0119 to CLM-0121 were retagged from `gap` to `m4`. Task 18 retires CLM-0087 (the v1 fallback), superseded by CLM-0168.
11. **Merging.** The standing authorization holds: the coordinator merges the plan PR and each sub-issue PR once CI is green and brooke-reviews' review has been answered.
12. **Execution order.** One issue at a time, oldest first (pgrace's standing rule). The graph below shows what could run in parallel if pgrace asks for it.

## Simplifications and deviations relative to the SDK

- **Classifier methods, not exception classes** (decision 1). `is_auth` is true for 401 on either HTTP API and for `Unauthenticated`; the SDK raises `AuthenticationException` for the same cases. `is_service_busy` is the SDK's `ServiceBusyException`, which only REST 503 raises. `is_sandbox_unreachable` covers the SDK's `SandboxNotRunningException` and `SandboxUnreachableException` together (envd 502 or `Unavailable`); the message, kept in the error, says which.
- **A moded pause refused with 409 is an error** (decision 2). SDK 2.61 returns `False` for every 409.
- **No PTY deadline by default** (decision 6). SDK 2.61 defaults `pty.create(timeout=60)` and `pty.connect(timeout=60)`.
- **At most 8 octet-stream uploads in flight by default** (decision 7). SDK 2.61 starts them all with `asyncio.gather`.
- **Per-call request timeouts only where an options struct exists** (the Scope table). The SDK takes `request_timeout` on every call.
- **No v1 connect route.** SDK 2.61 never had the fallback: it was the crate's own, for sandbawks before 0.1.28.
- **`Commands::connect` keeps the SDK's 60 s default**; only the PTY default changes.

## API after M4

Every task below writes against these names. A task that has to change one corrects this section in its own PR (see the Conventions).

**Errors (Task 2).**

```rust
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    MissingApiKey,
    InvalidConfig(String),
    InvalidArgument(String),
    #[non_exhaustive] Api { status: u16, message: String },
    SandboxNotFound(String),
    VolumeNotFound(String),
    McpGateway(String),
    #[non_exhaustive] Rpc { code: ConnectCode, message: String },
    #[non_exhaustive] Envd { status: u16, message: String },
    Protocol(String),
    Decode(#[from] serde_json::Error),
    Timeout(Duration),
    Transport(#[source] TransportError),   // was Transport(#[from] reqwest::Error)
}

impl Error {
    pub fn status(&self) -> Option<u16>;           // Api, Envd
    pub fn is_not_found(&self) -> bool;            // unchanged
    pub fn is_auth(&self) -> bool;                 // Api/Envd 401, Rpc Unauthenticated
    pub fn is_rate_limited(&self) -> bool;         // Api/Envd 429, Rpc ResourceExhausted
    pub fn is_service_busy(&self) -> bool;         // Api 503
    pub fn is_not_enough_space(&self) -> bool;     // Envd 507
    pub fn is_sandbox_unreachable(&self) -> bool;  // Envd 502, Rpc Unavailable
    pub fn is_timeout(&self) -> bool;              // Timeout, Rpc DeadlineExceeded/Canceled, Transport timeout
    pub fn is_retryable(&self) -> bool;            // rate limited, service busy, Transport connect failure
    pub(crate) fn transport(error: reqwest::Error) -> Self;
}

pub struct TransportError(reqwest::Error);       // opaque; derives Debug; Display and std::error::Error by hand (source() is reqwest's source)
impl TransportError {
    pub fn is_timeout(&self) -> bool;
    pub fn is_connect(&self) -> bool;
}
```

`From<reqwest::Error> for Error` is removed (a trait impl on a public type is public API); reqwest results are converted with `.map_err(Error::transport)?`. `lib.rs` adds `pub use error::TransportError;` and `pub use secrecy;`.

**Timeout helpers (Task 4, Task 5), in `src/config.rs`, crate-private.**

```rust
/// `Some(Duration::ZERO)` means no limit, as SDK `timeout=0` (#52).
pub(crate) fn no_limit_if_zero(timeout: Option<Duration>) -> Option<Duration>;

/// A per-call request timeout: unset uses the configured one; `Some(None)` is no limit.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct RequestTimeout(Option<Option<Duration>>);
impl RequestTimeout {
    pub(crate) fn set(timeout: Option<Duration>) -> Self;          // applies no_limit_if_zero
    pub(crate) fn resolve(self, configured: Option<Duration>) -> Option<Duration>;
}
```

Every public `timeout(..)` and `request_timeout(..)` setter that takes an `Option<Duration>` stores its value through these helpers, `ConnectionConfigBuilder::request_timeout` included.

**envd plumbing (Task 5), crate-private.** The transport takes the unresolved `RequestTimeout` and resolves it itself:

```rust
impl EnvdClient {
    pub(crate) fn timeout_for(&self, request_timeout: RequestTimeout) -> Option<Duration>;  // replaces request_timeout()
    pub(crate) async fn unary<Req, Res>(&self, service, method, request, extra_headers,
                                        request_timeout: RequestTimeout) -> Result<Res>;
    pub(crate) async fn within_request_timeout<T>(&self, request_timeout: RequestTimeout, opening) -> Result<T>;
    pub(crate) fn http_request(&self, method: Method, path: &str, request_timeout: RequestTimeout) -> RequestBuilder;
}   // server_stream, client_stream and http_request_with_timeout are unchanged
```

Callers without options (`Commands`, `Pty`, `Sandbox::is_running`) pass `RequestTimeout::default()`, and `Sandbox::mcp_token` passes `FileOptions::new().user(MCP_USER)`, the configured timeout, until Task 7 gives the commands and PTY calls their own. `read_stream` and `write_stream` resolve the per-call value against "no limit", so they stay unbounded unless the caller sets one.

**Filesystem (Tasks 5, 9, 10).** Every call ends with an options reference:

| Call | Options |
|---|---|
| `read`, `read_to_string`, `read_stream`, `get_info`, `exists`, `make_dir`, `remove` | `(path, &FileOptions)` |
| `rename` | `(source, destination, &FileOptions)` |
| `list` | `(path, &ListOptions)` |
| `write` | `(path, data, &WriteOptions)` (was `user: Option<&str>`) |
| `write_files`, `write_stream`, `watch_dir` | unchanged shape; `WriteOptions` and `WatchOptions` gain `request_timeout` |

```rust
pub struct FileOptions { /* user, request_timeout */ }
impl FileOptions { pub fn new() -> Self; pub fn user(self, impl Into<String>) -> Self;
                   pub fn request_timeout(self, Option<Duration>) -> Self; }
pub struct ListOptions { /* depth (default 1), user, request_timeout */ }
impl ListOptions { pub fn new() -> Self; pub fn depth(self, u32) -> Self;
                   pub fn user(self, impl Into<String>) -> Self;
                   pub fn request_timeout(self, Option<Duration>) -> Self; }
impl WriteOptions { /* existing setters */ pub fn request_timeout(self, Option<Duration>) -> Self;
                    pub fn max_concurrency(self, usize) -> Self; }   // Task 9; 0 is InvalidArgument; Default becomes hand-written
pub const DEFAULT_UPLOAD_CONCURRENCY: usize = 8;                      // Task 9
impl WatchOptions { /* existing setters */ pub fn request_timeout(self, Option<Duration>) -> Self; }
```

**Commands and PTY (Tasks 7, 8).**

```rust
impl Commands { pub async fn connect(&self, pid: u32, options: &CommandConnectOptions) -> Result<CommandHandle>; }
pub struct CommandConnectOptions { /* timeout: Some(DEFAULT_COMMAND_TIMEOUT), request_timeout */ }
impl CommandConnectOptions { pub fn new() -> Self; pub fn timeout(self, Option<Duration>) -> Self;
                             pub fn request_timeout(self, Option<Duration>) -> Self; }
impl CommandOptions { /* existing setters */ pub fn request_timeout(self, Option<Duration>) -> Self; }

impl Pty { pub async fn connect(&self, pid: u32, options: &PtyConnectOptions) -> Result<PtyHandle>; }
pub struct PtyConnectOptions { /* timeout: None after Task 8, request_timeout */ }
impl PtyConnectOptions { pub fn new() -> Self; pub fn timeout(self, Option<Duration>) -> Self;
                         pub fn request_timeout(self, Option<Duration>) -> Self; }
impl PtyOptions { /* existing setters */ pub fn request_timeout(self, Option<Duration>) -> Self; }
```

**Sandbox lifecycle (Task 6).**

```rust
impl Client {
    pub async fn connect_sandbox(&self, sandbox_id: &str, options: &ConnectOptions) -> Result<Sandbox>;
    pub async fn pause_sandbox(&self, sandbox_id: &str, options: &PauseOptions) -> Result<bool>;
}   // connect_sandbox_with and pause_sandbox_with are removed
impl Sandbox { pub async fn pause(&self, options: &PauseOptions) -> Result<bool>; }   // pause_with removed
```

**API review (Task 12).** `CommandEvent` and `Signal` become `#[non_exhaustive]`; `ConnectCode` (the closed set of gRPC codes) and `PtySize` (two public fields, built with `new`) stay exhaustive, with the reason in `docs/api-review.md`. Every options type has both `new()` and `Default`. `Cargo.toml` gains:

```toml
[lints.rust]
missing_docs = "warn"
missing_debug_implementations = "warn"
```

## Verified facts the code relies on

SDK sources are e2b 2.61.0, the same packages as `docs/protocol-notes.md` (`py:` paths relative to `e2b/`, `js:` lines of `package/dist/index.js`). Crate sources are under `~/.cargo/registry/src/index.crates.io-*/`.

| Fact | Source |
|---|---|
| sandbawks serves `POST /v2/sandboxes/{id}/connect` from 0.1.28, so the v1 fallback only covered older sandbawks | spec "Decisions" item 5; catalog `projects/e2b-client-rs.md` (gfd/sandbawks#662, 2026-10-26) |
| The crate today tries v1 after a v2 404 (`DEFAULT_SANDBOX_TIMEOUT` exists only for the v1 body); tests `connect_sandbox_falls_back_to_v1_route` and `the_v1_fallback_keeps_the_reboot_flag` pin it | `src/client.rs:35-37, 266-310`; `tests/rest_api.rs:164`; `tests/rest_lifecycle.rs:523` |
| REST errors: 401 → `AuthenticationException`, 429 → `RateLimitException`, 503 → `ServiceBusyException`, others → the call's default class with `status_code` | `py: api/__init__.py:155-213` |
| envd HTTP errors: 400 invalid argument, 401 authentication, 404 not found, 429 rate limit, 502 sandbox unavailable, 507 not enough space | `py: envd/api.py:24-33` |
| envd RPC errors: `INVALID_ARGUMENT`, `UNAUTHENTICATED` → authentication, `NOT_FOUND`, `UNAVAILABLE` → sandbox unavailable, `RESOURCE_EXHAUSTED` → rate limit, `CANCELED` and `DEADLINE_EXCEEDED` → timeout | `py: envd/rpc.py:26-40` |
| A 502 or `Unavailable` from the proxy says "was not found" (sandbox gone) or "port is not open" (envd unreachable) | `py: exceptions.py:4-36` |
| Pause: SDK 2.61 returns `False` for every 409 | `py: sandbox_async/sandbox_api.py:399-432` |
| Hosted E2B answers a pause of an already-paused sandbox with 409 `{"code":409,"message":"Error pausing sandbox - sandbox '<id>' is already paused"}`, with or without `memory: false` | live probe, hosted E2B, 2026-10-28T11:52Z (plan session) |
| A refused filesystem-only snapshot is 400 `snapshot_filesystem_only_disabled` or 409 `snapshot_filesystem_only_unsupported_node` | `py: sandbox_async/main.py:905` |
| `timeout_to_ms`: `None` and `0` → no deadline; positive → `max(1, round(ms))` | `py: envd/utils.py:15-24` |
| `request_timeout=0` disables the HTTP timeout | `py: api/__init__.py:280` |
| Filesystem calls take `user` and `request_timeout`; `list` takes `depth` (default 1) | `py: sandbox_async/filesystem/filesystem.py:116-121, 491-496, 530-534, 562-566, 590-594, 614-619, 647-650` |
| `commands.connect(pid, timeout=60, request_timeout)`, `pty.connect(pid, timeout=60, request_timeout)`, `pty.create(..., timeout=60, request_timeout)`, `watch_dir(..., request_timeout, timeout=60)` | `py: sandbox_async/commands/command.py:346-350`; `py: sandbox_async/commands/pty.py:139-147, 207-212`; `py: sandbox_async/filesystem/filesystem.py:684-685` |
| `EntryInfo.is_symlink` is proto field 12; the SDK maps `is_symlink` to `SYMLINK`, else maps `type` | `py: envd/filesystem/filesystem_pb.py:240, 299-303`; `py: sandbox/filesystem/filesystem.py:130-148` |
| reqwest 0.13.5 `Error::{is_timeout, is_connect, is_request, is_body, is_decode, status, url}` | `reqwest-0.13.5/src/error.rs:63, 119, 152, 158, 191, 196, 201` |
| futures-util 0.3.34 `TryStreamExt::try_buffered(n)` (keeps order, but holds a later error behind an earlier upload that has not answered: its queue is a `FuturesOrdered`) and `try_buffer_unordered(n)` (yields as each finishes), both behind feature `alloc` (already enabled); Task 9 uses `try_buffer_unordered` (corrected in issue #53) | `futures-util-0.3.34/src/stream/try_stream/mod.rs:1035-1037, 1111-1113`; `try_buffered.rs` |
| secrecy 0.10.3: `pub type SecretString = SecretBox<str>`; `ExposeSecret` | `secrecy-0.10.3/src/lib.rs:214, 262` |
| The crate builds with `cargo +1.89 check --all-targets`; the highest dependency `rust-version` is uuid 1.29.0's 1.89 | plan session, 2026-10-28 (`cargo metadata`, `uuid-1.29.0/Cargo.toml:14`) |
| `RUSTFLAGS="-W missing_docs -W missing_debug_implementations" cargo check --lib` gives no warnings today; `clippy::missing_errors_doc` and `missing_panics_doc` give none | plan session, 2026-10-28 |
| Today 25 of 194 public functions (13 of 72 async) have an `# Examples` section | plan session, 2026-10-28 (a scan of `src/*.rs`) |
| Every builder setter already carries `#[must_use]` | `src/config.rs:390`; the setters in `src/*.rs` |
| The crate today: zero is sent as `Connect-Timeout-Ms: 1` (`protocol_headers`); `EnvdClient::{unary, server_stream, client_stream, within_request_timeout, http_request}` use `self.request_timeout`; `write_files` starts all octet-stream uploads with `try_join_all`; `Commands::connect(pid, Option<Duration>)`, `Pty::connect(pid, Option<Duration>)`; `CommandEvent` and `Signal` are exhaustive | `src/envd/connect.rs:254-271`; `src/envd/mod.rs:267-275, 328-495`; `src/filesystem.rs:1029-1051`; `src/commands.rs:143-171, 328`; `src/pty.rs:309` |

## Conventions for every task

- **Gates:** `just lint` (fmt check plus `clippy --all-targets -D warnings`), `just test` and `just doc` must pass before pushing. A task that changes live behaviour or a live test's calls also runs the live tests it touched against hosted E2B, one at a time:
  `E2B_TEST_TARGET=hosted E2B_TEST_RUN_ID=dev-$(date -u +%Y%m%dT%H%M%SZ)-$$ cargo test --test <live file> -- --ignored --exact <test name>`.
  Report the result in the PR. The recorded runs are Tasks 16 and 17.
- **Breaking changes are the point of this milestone.** A signature change updates every caller in `src/`, `tests/`, `README.md` and the rustdoc examples in the same PR; `cargo test` compiles the README and doctests, so the gates catch a missed one. Each task's changelog entry has a "Breaking changes" list (old call, new call).
- **Tests:** mock tests use `wiremock` through `tests/common`. New mock tests go in the test file each task names. Live tests are `#[tokio::test]` plus `#[ignore = "live: run with `just test-live <target>`"]`, named `clm_NNNN_<slug>`, one claim each, Arrange-Act-Assert. Use `.expect("<what>")` in tests, never `.unwrap()`. No `println!`/`dbg!`.
- **Docs per task:** every new or changed public item keeps a rustdoc comment with Arguments/Returns/Errors, and an `# Examples` section for a new type or async method (Task 14 enforces it for all). Every wire fact a task changes is corrected in `docs/protocol-notes.md`; each S4 claim (CLM-0168, CLM-0169, CLM-0172) gets a row in `docs/verification/s4-evidence-map.md`, and the task adds its claim to the intro line's M4 list, which names only claims that have rows (Task 1: "and, from M4, CLM-0168"; Task 2: "CLM-0168 and CLM-0169"; Task 4: "CLM-0168, CLM-0169 and CLM-0172"; corrected in issue #91, 2026-10-28).
- **Ledger:** subagents never call `mcp__claims__*` write tools. Read tools are fine.
- **Changelog:** one entry per task, `changelog/<UTC ts>-m4-task-<NN>-<slug>.md`, timestamp from `date -u +%Y-%m-%dT%H-%M-%SZ`.
- **Logs:** live run logs go to `target/live-logs/` inside your own worktree. Never write them to a shared scratchpad.
- **Processes:** never kill processes by pattern; kill only your own PIDs.
- **`Cargo.lock`:** diff it before pushing; no task in this plan adds a dependency, so `Cargo.lock` should not change.
- The code in this plan has not been compiled. If an implementation has to differ, fix this plan in the same PR (inline, or with a dated `> **Corrected (issue #N).**` note), then grep later tasks for the changed symbol.

## File map

| File | Task |
|---|---|
| `claims.json` | plan PR; Task 18 |
| `Cargo.toml` | 12 (`[lints]`), 13 (`rust-version`) |
| `.github/workflows/ci.yml` | 13 (MSRV job) |
| `justfile` | 15 (`test-live-paced`) |
| `src/error.rs` | 2 |
| `src/config.rs` | 4 (`no_limit_if_zero`, `request_timeout` setter), 5 (`RequestTimeout`) |
| `src/client.rs` | 1 (connect), 2 (`map_err`), 3 (pause 409), 5 (a rustdoc example), 6 (signatures) |
| `src/sandbox.rs` | 2 (`map_err`), 5 (`is_running` and `mcp_token` callers, rustdoc examples), 6 (`pause`), 12 |
| `src/lifecycle.rs` | 6 (rustdoc of `ConnectOptions`, `PauseOptions`), 12 |
| `src/envd/connect.rs` | 2 (`map_err`), 4 (zero in `protocol_headers` callers) |
| `src/envd/mod.rs` | 2 (`map_err`), 5 (`timeout_for`; the `RequestTimeout` parameter on `unary`, `within_request_timeout`, `http_request`) |
| `src/envd/filesystem_pb.rs` | 10 (`is_symlink`) |
| `src/filesystem.rs` | 2 (`map_err`), 4 (`WatchOptions::timeout`), 5 (`FileOptions`, `ListOptions`, signatures), 9 (`max_concurrency`), 10 (`EntryInfo::from`) |
| `src/commands.rs` | 2 (`map_err`), 4 (`CommandOptions::timeout`), 5 (`RequestTimeout::default()` callers), 7 (`CommandConnectOptions`, `request_timeout`), 12 (`#[non_exhaustive]`) |
| `src/pty.rs` | 4 (`PtyOptions::timeout`), 5 (callers), 7 (`PtyConnectOptions`, `request_timeout`), 8 (defaults), 11 (rustdoc and behaviour of `connect`) |
| `src/stream_input.rs` | 4 (`StreamInputOptions::timeout`) |
| `src/lib.rs` (re-exports, crate docs) | 2, 5, 7, 9, 12, 14 |
| every `src/*.rs` (examples) | 14 |
| `tests/error_taxonomy.rs` (new) | 2 |
| `tests/zero_timeouts.rs` (new) | 4 |
| `tests/envd_file_options.rs` (new) | 5 |
| `tests/envd_connect_options.rs` (new) | 7, 8 |
| `tests/api_traits.rs` (new) | 12 |
| `tests/doc_examples.rs` (new) | 14 |
| `tests/rest_api.rs` | 1, 2 (`..` in patterns), 3 (a renamed test), 6 |
| `tests/rest_lifecycle.rs` | 1, 2 (`..`), 3, 6 |
| `tests/rest_fork.rs`, `tests/rest_network.rs`, `tests/rest_snapshots.rs`, `tests/rest_volumes.rs` | 2 (`..` in the `Error::{Api, Rpc, Envd}` patterns) |
| `tests/rest_mcp.rs` | 6 (`connect_sandbox` callers) |
| `tests/envd_api.rs`, `tests/envd_stream_files.rs`, `tests/envd_watch.rs`, `tests/envd_write.rs` | 2 (`..` in patterns: `envd_api.rs`, `envd_stream_files.rs`, `envd_write.rs`); 5 (call sites); 7 (`envd_api.rs` connect callers); 9 (`envd_write.rs`); 10 (`envd_api.rs`) |
| `tests/envd_pty.rs` | 7, 8 |
| `tests/live_lifecycle.rs` | 1, 3, 6 |
| `tests/live_files.rs` | 5, 10 |
| `tests/live_commands.rs` | 7, 8, 11 |
| `tests/live_support/mod.rs`, `tests/common/mod.rs` | callers only, as signatures change (2, 5, 6, 7) |
| `README.md` | 2 (the Errors row), 5, 6, 7, 8 (examples), 13 (semver and MSRV), 18 (coverage, verification status) |
| `CONTRIBUTING.md` | 13 |
| `docs/api-review.md` (new) | 12; 13 and 14 append |
| `docs/protocol-notes.md` | 1, 2, 3, 4, 5, 7, 8, 9, 10, 11 |
| `docs/verification/s4-evidence-map.md` | 1, 2, 4 |
| `docs/verification/live-pacing.md` (new) | 15 |
| `docs/verification/runs/<run id>.log`, `docs/verification/m4-live-results.md` | 16, 17 |
| the spec | 18 (the M4 row's outcome) |
| milestone changelog | 18 |

## Task dependencies

```
1 -> 6
3 -> 6
4 -> 5
5 -> 7
5 -> 9
5 -> 10
7 -> 8
8 -> 11
2 -> 13
1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11 -> 12
12, 13 -> 14
1, 2, ..., 15 -> 16
1, 2, ..., 15 -> 17
16, 17 -> 18
```

Tasks 1, 2, 3, 4 and 15 can start straight away. Task 6 folds the `_with` twins, so it waits for Task 1 (connect) and Task 3 (pause) to avoid rework. Task 5 needs Task 4's `no_limit_if_zero` for its `request_timeout` setters, and Tasks 7, 9 and 10 build on Task 5's `RequestTimeout` plumbing in `src/envd/mod.rs` and its new filesystem signatures. Task 8 flips the defaults of Task 7's `PtyConnectOptions`; Task 11 investigates the final `Pty::connect`. Task 12 reviews the surface once every API-changing task has landed, and Task 14 adds the examples to that final surface. Task 13 lists Task 2's public dependencies. Tasks 16 and 17 can run in parallel; each adds one section to `docs/verification/m4-live-results.md`, so keep both. Task 18 is the coordinator's.

Task 2 touches every file that converts a reqwest error (`.map_err(Error::transport)`), so any task open at the same time will conflict in a few lines of `src/client.rs`, `src/filesystem.rs`, `src/envd/*.rs` and `src/sandbox.rs`: keep both sides, and keep the `map_err`.

---

### Task 1: Drop the v1 connect fallback (CLM-0168)

**Files:**
- Modify: `src/client.rs` (`connect_sandbox` and `connect_sandbox_with` rustdoc and body; remove `DEFAULT_SANDBOX_TIMEOUT`, lines 35-37)
- Modify: `tests/rest_api.rs` (replace `connect_sandbox_falls_back_to_v1_route`, lines 163-187; `connect_missing_sandbox_is_not_found`, lines 189-209; `every_rest_request_carries_the_api_key`, lines 460-494)
- Modify: `tests/rest_lifecycle.rs` (remove `the_v1_fallback_keeps_the_reboot_flag`, lines 522-565)
- Modify: `docs/protocol-notes.md` (the "Connect v1 (fallback)" row, line 71; "Deliberate differences", lines 419-423 and 430-432; "Verification against a live server", lines 491-493)
- Modify: `docs/verification/s4-evidence-map.md` (intro line; rows CLM-0087, CLM-0159; new row CLM-0168)
- Create: `changelog/<ts>-m4-task-01-drop-v1-connect-fallback.md`

> **Corrected (issue #91, 2026-10-28).** Row CLM-0073 also named the v1 connect fallback ("all nine REST requests"); Task 1 corrects it to eight. `src/models.rs` (`ConnectBody` rustdoc, which cited the v1 `ConnectSandbox` body) is modified too. Step 5's live check ran `clm_0015` only, at the coordinator's direction; `clm_0014` is left to Task 16's recorded run. The evidence-map intro names only the claims that have rows: Task 1 adds CLM-0168, and Tasks 2 and 4 add their own (see the Conventions).

The spec's decision 5: the fallback only made up for sandbawks before 0.1.28 (Verified facts). After this task, a v2 404 is `Error::SandboxNotFound` straight away, with no second request. CLM-0087 is retired in Task 18 and superseded by CLM-0168.

- [ ] **Step 1: Write the failing mock test**

In `tests/rest_api.rs`, replace `connect_sandbox_falls_back_to_v1_route` (lines 163-187) with:

```rust
#[tokio::test]
async fn connect_sandbox_v2_404_is_not_found_without_a_v1_request() {
    // Arrange: the v1 route would succeed, so any request to it shows up as
    // a wrong `Ok` and as a broken `.expect(0)`.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/v2/sandboxes/{SANDBOX_ID}/connect")))
        .respond_with(
            ResponseTemplate::new(404).set_body_json(json!({"code": 404, "message": "nope"})),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path(format!("/sandboxes/{SANDBOX_ID}/connect")))
        .respond_with(ResponseTemplate::new(201).set_body_json(sandbox_json("0.5.7")))
        .expect(0)
        .named("v1 connect")
        .mount(&server)
        .await;

    // Act
    let err = client_for(&server)
        .connect_sandbox(SANDBOX_ID, None)
        .await
        .expect_err("a v2 404 is not found");

    // Assert
    assert!(
        matches!(err, Error::SandboxNotFound(ref id) if id == SANDBOX_ID),
        "{err:?}"
    );
    let requests = server.received_requests().await.expect("recording");
    assert_eq!(requests.len(), 1, "one request, no v1 retry");
}
```

In `connect_missing_sandbox_is_not_found` (lines 189-209), change `.expect(2)` to `.expect(1)`.

In `every_rest_request_carries_the_api_key` (lines 460-494), replace the comment `// v2 connect, then the v1 fallback after its 404.` with `// v2 connect only: a 404 is not found, with no v1 retry.`, and the assertion with:

```rust
    assert_eq!(requests.len(), 8, "one request per call");
```

In `tests/rest_lifecycle.rs`, delete `the_v1_fallback_keeps_the_reboot_flag` (lines 522-565). `connect_sends_memory_false_only_for_a_reboot` still covers the v2 body for a reboot.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --test rest_api connect_`
Expected: FAIL. `connect_sandbox_v2_404_is_not_found_without_a_v1_request` panics at `expect_err("a v2 404 is not found")`, because the v1 mock answers 201. `connect_missing_sandbox_is_not_found` fails its `.expect(1)` verification (2 calls received). `every_rest_request_carries_the_api_key` fails with `left: 9, right: 8`.

- [ ] **Step 3: Remove the fallback**

In `src/client.rs`, delete lines 35-37:

```rust
/// The sandbox timeout the SDKs fall back to (`e2b/sandbox/main.py:30`); the
/// v1 connect body requires a timeout.
const DEFAULT_SANDBOX_TIMEOUT: Duration = Duration::from_secs(300);
```

`Duration` is still imported for `set_sandbox_timeout` and friends.

Replace the rustdoc of `connect_sandbox` (lines 204-229) with:

```rust
    /// Connects to a sandbox, resuming it first if it is paused
    /// (`POST /v2/sandboxes/{id}/connect`). A paused sandbox is restored
    /// with the API's default; [`Client::connect_sandbox_with`] can reboot it
    /// instead.
    ///
    /// # Arguments
    ///
    /// * `sandbox_id` - the sandbox to connect to.
    /// * `timeout` - a new sandbox lifetime from now; `None` sends none, so
    ///   the server keeps the sandbox's current lifetime.
    ///
    /// # Returns
    ///
    /// A [`Sandbox`] handle.
    ///
    /// # Errors
    ///
    /// [`Error::SandboxNotFound`] for HTTP 404, [`Error::Api`] for other
    /// non-2xx answers, [`Error::Transport`] for connection failures.
```

Replace the rustdoc and body of `connect_sandbox_with` (lines 242-310) with:

```rust
    /// [`Client::connect_sandbox`] with the SDK 2.61 options: the timeout and
    /// how a paused sandbox resumes (`connect(timeout, on_resume)`,
    /// `e2b/sandbox_async/sandbox_api.py:498-520`).
    ///
    /// [`OnResume::Reboot`](crate::OnResume::Reboot) sends `memory: false`;
    /// a restore, the API's default, sends no `memory` field. The timeout
    /// behaves as in [`Client::connect_sandbox`].
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
    /// As [`Client::connect_sandbox`]. The API documents that it refuses a
    /// reboot with an error status ([`Error::Api`]) where that capability is
    /// not enabled, rather than falling back to a memory restore.
    pub async fn connect_sandbox_with(
        &self,
        sandbox_id: &str,
        options: &ConnectOptions,
    ) -> Result<Sandbox> {
        let body = ConnectBody {
            timeout: options.timeout_value().map(|timeout| timeout.as_secs()),
            memory: options.memory(),
        };
        let response = self
            .send_json(
                Method::POST,
                &["v2", "sandboxes", sandbox_id, "connect"],
                &body,
            )
            .await?;
        // A 404 is the sandbox missing: there is no other route to try.
        let created: SandboxCreated = parse_sandbox_response(response, sandbox_id).await?;
        Sandbox::from_created(self.clone(), created)
    }
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --test rest_api --test rest_lifecycle`
Expected: PASS.

- [ ] **Step 5: Live check**

The live tests of connect are unchanged. Run `clm_0015_connect_unknown_sandbox_is_not_found` (`tests/live_lifecycle.rs:285`) and `clm_0014_connect_resumes_paused_sandbox_with_files` (line 240) on hosted, one at a time, as in the Conventions. Expected: PASS. Report both in the PR.

- [ ] **Step 6: Docs**

`docs/protocol-notes.md`:
- Delete the "Connect v1 (fallback)" row (line 71). Keep "Default sandbox timeout 300 s" (line 72): it is still the SDK's create default.
- In "Deliberate differences from the SDKs", delete the "Connect v1 fallback" bullet (lines 419-423).
- In the "Not implemented yet" bullet (lines 430-432), delete `dropping the v1 connect fallback, ` from the milestone 4 list.
- In "Verification against a live server", replace the last sentence of the "sandbawks and v2 connect" bullet (lines 492-493) with: `The v1 connect fallback was therefore removed in M4 (Task 1): a v2 404 is the sandbox missing.`

`docs/verification/s4-evidence-map.md`:
- Intro (line 5): replace `and CLM-0154 to CLM-0167.` with `CLM-0154 to CLM-0167, and, from M4, CLM-0168.` (Corrected in issue #91: only claims that have rows; Tasks 2 and 4 extend it.)
- Row CLM-0087: replace its Notes with `Superseded by CLM-0168 in M4 (Task 1 removed the fallback and its test; Task 18 retires the claim).`, its Test column with `none (removed)`, and "Covers whole claim?" with `no`.
- Row CLM-0159: in the Test column, delete `tests/rest_lifecycle.rs::the_v1_fallback_keeps_the_reboot_flag; `. In the Notes, delete the sentence that begins `With the v2 route answering 404, a reboot falls back`.
- Append after the CLM-0167 row:

```markdown
| CLM-0168 | tests/rest_api.rs::connect_sandbox_v2_404_is_not_found_without_a_v1_request; tests/rest_api.rs::connect_missing_sandbox_is_not_found; tests/rest_api.rs::every_rest_request_carries_the_api_key | yes | new | A v2 connect answering 404 is `Error::SandboxNotFound` carrying the id, with exactly one request recorded; a v1 mock that would answer 201 has `.expect(0)`. A catch-all 404 mock sees exactly one connect request (`.expect(1)`), and eight calls, connect included, make eight requests. |
```

- [ ] **Step 7: Gates, changelog, commit**

Run: `just lint && just test && just doc`
Expected: all green. `git diff Cargo.lock` is empty.

Create `changelog/<date -u +%Y-%m-%dT%H-%M-%SZ>-m4-task-01-drop-v1-connect-fallback.md`:

```markdown
# M4 Task 1: drop the v1 connect fallback (issue #<N>)

`connect_sandbox` and `connect_sandbox_with` now make one request,
`POST /v2/sandboxes/{id}/connect`. A 404 there is `Error::SandboxNotFound`;
the crate no longer retries `POST /sandboxes/{id}/connect`. The fallback only
served sandbawks before 0.1.28 (spec decision 5). Backs CLM-0168, which
supersedes CLM-0087.

## Breaking changes

- Against a server that serves only the v1 connect route,
  `connect_sandbox(id, timeout)` now returns `Error::SandboxNotFound` where it
  used to connect. No signature changes.
- Without a timeout, connect no longer resets the lifetime to 300 s on that
  route (it was a v1-only behaviour).

## Tests

- New: `tests/rest_api.rs::connect_sandbox_v2_404_is_not_found_without_a_v1_request`.
- Changed: `connect_missing_sandbox_is_not_found` (one request, not two),
  `every_rest_request_carries_the_api_key` (8 requests, not 9).
- Removed: `connect_sandbox_falls_back_to_v1_route`,
  `tests/rest_lifecycle.rs::the_v1_fallback_keeps_the_reboot_flag`.
- Live on hosted: <results of clm_0014 and clm_0015>.
```

```bash
git add src/client.rs tests/rest_api.rs tests/rest_lifecycle.rs docs/protocol-notes.md docs/verification/s4-evidence-map.md changelog/
git commit -m "feat!: drop the v1 connect fallback"
```


---

### Task 2: Error taxonomy and an opaque transport error (CLM-0169, CLM-0170)

**Files:**
- Modify: `src/error.rs` (`TransportError`, `Error::Transport`, `#[non_exhaustive]` on `Api`, `Rpc` and `Envd`, the classifiers, `Error::transport`, the `is_not_found` example, unit tests)
- Modify: `src/lib.rs` (line 104: `TransportError`; `pub use secrecy;`)
- Modify: `src/client.rs`, `src/envd/connect.rs`, `src/envd/mod.rs`, `src/filesystem.rs`, `src/sandbox.rs` (`.map_err(Error::transport)` at every reqwest conversion; the list is in Step 3)
- Modify: `src/filesystem.rs:1083-1084` (the `write_stream` rustdoc names reqwest; Step 6)
- Modify, adding `..` to the patterns: `tests/envd_api.rs`, `tests/envd_stream_files.rs`, `tests/envd_write.rs`, `tests/rest_api.rs`, `tests/rest_fork.rs`, `tests/rest_lifecycle.rs`, `tests/rest_network.rs`, `tests/rest_snapshots.rs`, `tests/rest_volumes.rs` (the list is in Step 4)
- Create: `tests/error_taxonomy.rs`
- Modify: `README.md` (the "Errors" row, line 63), `docs/protocol-notes.md` (new section "Error classification"), `docs/verification/s4-evidence-map.md` (row CLM-0169)
- Create: `changelog/<ts>-m4-task-02-error-taxonomy.md`

> **Corrected (issue #92, 2026-10-28).** On 88184fb (after Task 1) the reqwest conversions are 30, not 32: Step 3's table lists 30 lines, and the line numbers in `src/client.rs` moved up once Task 1 removed the v1 fallback (by 3 above `connect_sandbox`, so `:105` is `:102`; by 32 below it, so `:325` is `:293` and `:1044` is `:1012`). Follow the compiler's list. Step 4's `tests/rest_api.rs:328` is now `:339`; the 19 pattern sites are otherwise as listed. No later task changes: their code already converts with `.map_err(Error::transport)` and matches the struct variants with `..`.

Decisions 1 and 4. The `Api`, `Envd` and `Rpc` variants stay. `status()` and seven classifiers map them onto SDK 2.61's exception classes (Verified facts: `py: api/__init__.py:155-213`, `py: envd/api.py:24-33`, `py: envd/rpc.py:26-40`). `Error::Transport` now holds a `TransportError`, so no reqwest type is public (CLM-0170). Because `From<reqwest::Error> for Error` is a public trait impl, it goes, and every reqwest result is converted with `.map_err(Error::transport)`.

- [ ] **Step 1: Write the failing integration tests**

Create `tests/error_taxonomy.rs`. It uses only the public API. The struct variants can't be built outside the crate once they are `#[non_exhaustive]`, so every error comes from a wiremock answer:

```rust
//! The error classifiers (CLM-0169) and the opaque transport error
//! (CLM-0170), each error produced by a real call against a mock server.

mod common;

use std::error::Error as _;
use std::time::Duration;

use common::{SANDBOX_ID, client_for, client_with_timeout, sandbox};
use e2b_client::{Client, ConnectionConfig, Error, TransportError};
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// What each classifier said about one error, in a fixed order so a failed
/// assertion prints every flag at once.
#[derive(Debug, PartialEq, Eq)]
struct Flags {
    auth: bool,
    rate_limited: bool,
    service_busy: bool,
    not_enough_space: bool,
    sandbox_unreachable: bool,
    timeout: bool,
    retryable: bool,
}

impl Flags {
    fn of(error: &Error) -> Self {
        Self {
            auth: error.is_auth(),
            rate_limited: error.is_rate_limited(),
            service_busy: error.is_service_busy(),
            not_enough_space: error.is_not_enough_space(),
            sandbox_unreachable: error.is_sandbox_unreachable(),
            timeout: error.is_timeout(),
            retryable: error.is_retryable(),
        }
    }

    const NONE: Self = Self {
        auth: false,
        rate_limited: false,
        service_busy: false,
        not_enough_space: false,
        sandbox_unreachable: false,
        timeout: false,
        retryable: false,
    };
}

#[tokio::test]
async fn rest_statuses_classify_as_the_sdk_does() {
    // Arrange: one sandbox id per status, each answering with an E2B error body.
    let server = MockServer::start().await;
    let cases = [
        (401, Flags { auth: true, ..Flags::NONE }),
        (429, Flags { rate_limited: true, retryable: true, ..Flags::NONE }),
        (503, Flags { service_busy: true, retryable: true, ..Flags::NONE }),
        (500, Flags::NONE),
    ];
    for (status, _) in &cases {
        Mock::given(method("GET"))
            .and(path(format!("/sandboxes/s{status}")))
            .respond_with(
                ResponseTemplate::new(*status)
                    .set_body_json(json!({"code": status, "message": "nope"})),
            )
            .expect(1)
            .mount(&server)
            .await;
    }
    let client = client_for(&server);

    for (status, expected) in cases {
        // Act
        let err = client
            .get_sandbox_info(&format!("s{status}"))
            .await
            .expect_err("an error status");

        // Assert
        assert!(matches!(err, Error::Api { .. }), "{status}: {err:?}");
        assert_eq!(err.status(), Some(status), "{err:?}");
        assert_eq!(Flags::of(&err), expected, "{status}: {err:?}");
    }
}

#[tokio::test]
async fn envd_http_statuses_classify_as_the_sdk_does() {
    // Arrange: `is_running` reads envd's plain HTTP `/health`. A 502 there
    // means "not running" (`Ok(false)`), so the 502 case is a unit test in
    // `src/error.rs`.
    let cases = [
        (401, Flags { auth: true, ..Flags::NONE }),
        (429, Flags { rate_limited: true, retryable: true, ..Flags::NONE }),
        (507, Flags { not_enough_space: true, ..Flags::NONE }),
        // An envd 503 is not the SDK's ServiceBusy, which only REST raises.
        (503, Flags::NONE),
    ];
    for (status, expected) in cases {
        let server = MockServer::start().await;
        let sandbox = sandbox(&server).await;
        Mock::given(method("GET"))
            .and(path("/health"))
            .respond_with(
                ResponseTemplate::new(status)
                    .set_body_json(json!({"code": status, "message": "nope"})),
            )
            .expect(1)
            .mount(&server)
            .await;

        // Act
        let err = sandbox.is_running().await.expect_err("an error status");

        // Assert
        assert!(matches!(err, Error::Envd { .. }), "{status}: {err:?}");
        assert_eq!(err.status(), Some(status), "{err:?}");
        assert_eq!(Flags::of(&err), expected, "{status}: {err:?}");
    }
}

#[tokio::test]
async fn rpc_codes_classify_as_the_sdk_does() {
    // Arrange: `Commands::list` is a unary RPC; each answer is a Connect
    // error body.
    let cases = [
        (401, "unauthenticated", Flags { auth: true, ..Flags::NONE }),
        (
            429,
            "resource_exhausted",
            Flags { rate_limited: true, retryable: true, ..Flags::NONE },
        ),
        (503, "unavailable", Flags { sandbox_unreachable: true, ..Flags::NONE }),
        (504, "deadline_exceeded", Flags { timeout: true, ..Flags::NONE }),
        (499, "canceled", Flags { timeout: true, ..Flags::NONE }),
        (500, "internal", Flags::NONE),
    ];
    for (status, code, expected) in cases {
        let server = MockServer::start().await;
        let sandbox = sandbox(&server).await;
        Mock::given(method("POST"))
            .and(path("/process.Process/List"))
            .respond_with(
                ResponseTemplate::new(status)
                    .set_body_json(json!({"code": code, "message": "nope"})),
            )
            .expect(1)
            .mount(&server)
            .await;

        // Act
        let err = sandbox.commands().list().await.expect_err("an RPC error");

        // Assert
        assert!(matches!(err, Error::Rpc { .. }), "{code}: {err:?}");
        assert_eq!(err.status(), None, "an RPC error has a code, not a status");
        assert_eq!(Flags::of(&err), expected, "{code}: {err:?}");
    }
}

#[tokio::test]
async fn a_request_timeout_is_a_timeout_and_not_retryable() {
    // Arrange: the answer comes after the client's request timeout.
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/sandboxes/{SANDBOX_ID}")))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(5)))
        .mount(&server)
        .await;
    let client = client_with_timeout(&server, Some(Duration::from_millis(100)));

    // Act
    let err = client
        .get_sandbox_info(SANDBOX_ID)
        .await
        .expect_err("timeout");

    // Assert
    let Error::Transport(ref transport) = err else {
        panic!("expected a transport error, got {err:?}");
    };
    assert!(transport.is_timeout(), "{transport:?}");
    assert!(!transport.is_connect(), "{transport:?}");
    assert_eq!(err.status(), None);
    assert_eq!(Flags::of(&err), Flags { timeout: true, ..Flags::NONE });
    // The detail is in `source()`, not repeated in `Display`.
    assert_eq!(err.to_string(), "HTTP transport error");
    let source = err.source().expect("a source");
    assert!(!source.to_string().is_empty(), "{source:?}");
}

#[tokio::test]
async fn a_refused_connection_is_retryable_and_keeps_its_cause() {
    // Arrange: reserve a port with a socket that is bound but never listens.
    // The kernel refuses connections to it, and because the socket stays
    // bound until the end of the test, no other process can take the port
    // in between (binding and then freeing a port would leave that race).
    let reserved = tokio::net::TcpSocket::new_v4().expect("socket");
    reserved.bind("127.0.0.1:0".parse().expect("addr")).expect("bind");
    let port = reserved.local_addr().expect("addr").port();
    let config = ConnectionConfig::builder()
        .api_key("test-api-key")
        .api_url(format!("http://127.0.0.1:{port}"))
        .domain("example.test")
        .request_timeout(Some(Duration::from_secs(5)))
        .build()
        .expect("config");
    let client = Client::new(config).expect("client");

    // Act
    let err = client
        .get_sandbox_info(SANDBOX_ID)
        .await
        .expect_err("connection refused");

    // Assert
    let Error::Transport(ref transport) = err else {
        panic!("expected a transport error, got {err:?}");
    };
    assert!(transport.is_connect(), "{transport:?}");
    assert!(err.is_retryable(), "{err:?}");
    assert!(!err.is_timeout(), "{err:?}");
    // The cause chain survives the wrapper: `Error` -> `TransportError`.
    let source = err.source().expect("a source");
    assert!(source.downcast_ref::<TransportError>().is_some(), "{source:?}");
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --test error_taxonomy`
Expected: FAIL to compile, with ``unresolved import `e2b_client::TransportError` `` and ``no method named `is_auth` found for enum `Error` `` (and likewise for `status`, `is_rate_limited` and the rest).

- [ ] **Step 3: Implement the taxonomy**

In `src/error.rs`, extend the module doc (after line 14) with:

```rust
//!
//! On top of the variants, [`Error::status`] and the `is_*` classifiers
//! answer the questions the official SDKs answer with exception classes
//! (`AuthenticationException`, `RateLimitException`, `ServiceBusyException`,
//! `NotEnoughSpaceException`, `SandboxNotRunningException`,
//! `TimeoutException`), whichever protocol the error came from.
```

Replace the `Api`, `Rpc` and `Envd` variants (lines 50-57, 75-82 and 84-91) so each carries `#[non_exhaustive]`:

```rust
    /// The E2B REST API answered with a non-success HTTP status.
    ///
    /// `#[non_exhaustive]`: match it with `Error::Api { status, .. }`, so a
    /// field added later is not a breaking change.
    #[error("E2B API error (HTTP {status}): {message}")]
    #[non_exhaustive]
    Api {
        /// The HTTP status code.
        status: u16,
        /// The `message` from the E2B error body, or the raw body when it was
        /// not an E2B error object.
        message: String,
    },
```

```rust
    /// An envd Connect-RPC call failed.
    ///
    /// `#[non_exhaustive]`: match it with `Error::Rpc { code, .. }`.
    #[error("envd RPC error ({code}): {message}")]
    #[non_exhaustive]
    Rpc {
        /// The Connect error code.
        code: ConnectCode,
        /// The human-readable error message.
        message: String,
    },

    /// envd's plain HTTP file API answered with a non-success status.
    ///
    /// `#[non_exhaustive]`: match it with `Error::Envd { status, .. }`.
    #[error("envd HTTP error (HTTP {status}): {message}")]
    #[non_exhaustive]
    Envd {
        /// The HTTP status code.
        status: u16,
        /// The `message` from the JSON error body, or the raw body.
        message: String,
    },
```

Replace the `Transport` variant (lines 107-109) with:

```rust
    /// The HTTP transport failed (connection refused, TLS failure, reset,
    /// request timeout, ...). [`TransportError`] says which.
    ///
    /// The `Display` is only "HTTP transport error"; the detail is the
    /// [`std::error::Error::source`], so a chain printer (anyhow's `{:#}`,
    /// for example) shows it once rather than twice.
    #[error("HTTP transport error")]
    Transport(#[source] TransportError),
```

Replace `impl Error` (lines 112-140) with:

```rust
impl Error {
    /// Wraps a reqwest error. reqwest stays private: callers see only
    /// [`TransportError`].
    ///
    /// Python note: Rust's `?` converts errors with a `From` impl. A
    /// `From<reqwest::Error>` impl would be public API (it would tie this
    /// crate's semver to reqwest's), so call sites convert explicitly with
    /// `.map_err(Error::transport)?`.
    pub(crate) fn transport(error: reqwest::Error) -> Self {
        Self::Transport(TransportError(error))
    }

    /// The HTTP status of a REST or envd HTTP error.
    ///
    /// # Returns
    ///
    /// The status for [`Error::Api`] and [`Error::Envd`], else `None`.
    /// [`Error::SandboxNotFound`] and [`Error::VolumeNotFound`] are 404s by
    /// definition and return `None`; an [`Error::Rpc`] has a code instead.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn demo(client: &e2b_client::Client) {
    /// if let Err(err) = client.get_sandbox_info("sbx-1").await {
    ///     if err.status() == Some(403) {
    ///         eprintln!("this key may not read that sandbox");
    ///     }
    /// }
    /// # }
    /// ```
    #[must_use]
    pub fn status(&self) -> Option<u16> {
        match self {
            Self::Api { status, .. } | Self::Envd { status, .. } => Some(*status),
            _ => None,
        }
    }

    /// The Connect code of an [`Error::Rpc`], else `None`.
    fn rpc_code(&self) -> Option<ConnectCode> {
        match self {
            Self::Rpc { code, .. } => Some(*code),
            _ => None,
        }
    }

    /// Whether this error means "the thing you asked about does not exist":
    /// a missing sandbox or volume, an RPC `not_found`, or an HTTP 404 from
    /// the REST or envd file API.
    ///
    /// # Returns
    ///
    /// `true` for [`Error::SandboxNotFound`], [`Error::VolumeNotFound`],
    /// [`Error::Rpc`] with [`ConnectCode::NotFound`], and
    /// [`Error::Api`]/[`Error::Envd`] with status 404.
    ///
    /// # Examples
    ///
    /// ```
    /// use e2b_client::Error;
    ///
    /// let err = Error::SandboxNotFound("sbx-1".into());
    /// assert!(err.is_not_found());
    /// ```
    #[must_use]
    pub fn is_not_found(&self) -> bool {
        match self {
            Self::SandboxNotFound(_) | Self::VolumeNotFound(_) => true,
            _ => self.status() == Some(404) || self.rpc_code() == Some(ConnectCode::NotFound),
        }
    }

    /// Whether the server refused the credentials: HTTP 401 from the REST
    /// or envd HTTP API, or an RPC `unauthenticated` (the SDKs'
    /// `AuthenticationException`, `py: api/__init__.py:164-168`,
    /// `py: envd/api.py:26`, `py: envd/rpc.py:28`).
    ///
    /// # Returns
    ///
    /// `true` for those errors. [`Error::MissingApiKey`] is a local
    /// configuration error, not a refusal, and returns `false`.
    #[must_use]
    pub fn is_auth(&self) -> bool {
        self.status() == Some(401) || self.rpc_code() == Some(ConnectCode::Unauthenticated)
    }

    /// Whether the server is rate limiting: HTTP 429, or an RPC
    /// `resource_exhausted` (the SDKs' `RateLimitException`,
    /// `py: api/__init__.py:170-174`, `py: envd/api.py:28-30`,
    /// `py: envd/rpc.py:31-33`).
    ///
    /// # Returns
    ///
    /// `true` for those errors.
    #[must_use]
    pub fn is_rate_limited(&self) -> bool {
        self.status() == Some(429) || self.rpc_code() == Some(ConnectCode::ResourceExhausted)
    }

    /// Whether the REST API refused the call as temporarily busy (HTTP 503):
    /// nothing was changed, so the same call can be retried after a short
    /// wait (the SDKs' `ServiceBusyException`, `py: api/__init__.py:176-180`).
    /// An envd 503 is not this: the SDKs raise it only for REST.
    ///
    /// # Returns
    ///
    /// `true` for [`Error::Api`] with status 503.
    #[must_use]
    pub fn is_service_busy(&self) -> bool {
        matches!(self, Self::Api { status: 503, .. })
    }

    /// Whether the sandbox's disk is full: envd answered HTTP 507 (the SDKs'
    /// `NotEnoughSpaceException`, `py: envd/api.py:32`).
    ///
    /// # Returns
    ///
    /// `true` for [`Error::Envd`] with status 507.
    #[must_use]
    pub fn is_not_enough_space(&self) -> bool {
        matches!(self, Self::Envd { status: 507, .. })
    }

    /// Whether envd could not be reached through the proxy: envd HTTP 502 or
    /// an RPC `unavailable`. The message says whether the sandbox is gone
    /// ("was not found") or running with envd's port closed ("port is not
    /// open") (the SDKs' `SandboxNotRunningException` and
    /// `SandboxUnreachableException`, `py: exceptions.py:4-36`,
    /// `py: envd/api.py:31`, `py: envd/rpc.py:30`).
    ///
    /// # Returns
    ///
    /// `true` for those errors.
    #[must_use]
    pub fn is_sandbox_unreachable(&self) -> bool {
        matches!(self, Self::Envd { status: 502, .. })
            || self.rpc_code() == Some(ConnectCode::Unavailable)
    }

    /// Whether a deadline ended the call: a client-side deadline
    /// ([`Error::Timeout`]), an RPC `deadline_exceeded` or `canceled` (the
    /// SDKs' `TimeoutException`, `py: envd/rpc.py:34-39`), or a request
    /// timeout in the transport.
    ///
    /// # Returns
    ///
    /// `true` for those errors.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Duration;
    /// use e2b_client::Error;
    ///
    /// assert!(Error::Timeout(Duration::from_secs(60)).is_timeout());
    /// ```
    #[must_use]
    pub fn is_timeout(&self) -> bool {
        match self {
            Self::Timeout(_) => true,
            Self::Rpc { code, .. } => {
                matches!(code, ConnectCode::DeadlineExceeded | ConnectCode::Canceled)
            }
            Self::Transport(transport) => transport.is_timeout(),
            _ => false,
        }
    }

    /// Whether retrying the same call later can succeed without changing
    /// it: the server is rate limiting ([`Error::is_rate_limited`]), the REST
    /// API is busy ([`Error::is_service_busy`]), or the connection was never
    /// made ([`TransportError::is_connect`]), so the request never reached
    /// the server. Timeouts are not included: a timed-out request may have
    /// taken effect.
    ///
    /// # Returns
    ///
    /// `true` for those errors.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::time::Duration;
    ///
    /// # async fn demo(client: &e2b_client::Client) -> e2b_client::Result<()> {
    /// let info = match client.get_sandbox_info("sbx-1").await {
    ///     Err(err) if err.is_retryable() => {
    ///         tokio::time::sleep(Duration::from_secs(1)).await;
    ///         client.get_sandbox_info("sbx-1").await?
    ///     }
    ///     other => other?,
    /// };
    /// println!("{:?}", info.state);
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn is_retryable(&self) -> bool {
        self.is_rate_limited()
            || self.is_service_busy()
            || matches!(self, Self::Transport(transport) if transport.is_connect())
    }
}

/// A failure of the HTTP transport under a call: the connection could not be
/// made, TLS failed, the connection dropped, or the request timed out.
///
/// The underlying HTTP client is an implementation detail, so this type is
/// opaque. Its `Display` is the transport's message, and
/// [`std::error::Error::source`] gives the cause below it (for example the
/// I/O error).
///
/// # Examples
///
/// ```no_run
/// use e2b_client::Error;
///
/// # async fn demo(client: &e2b_client::Client) {
/// if let Err(Error::Transport(transport)) = client.get_sandbox_info("sbx-1").await {
///     if transport.is_connect() {
///         eprintln!("could not reach the API: {transport}");
///     }
/// }
/// # }
/// ```
#[derive(Debug)]
pub struct TransportError(reqwest::Error);

impl TransportError {
    /// Whether the request timed out (the configured request timeout or a
    /// per-call one).
    ///
    /// # Returns
    ///
    /// `true` for a timeout.
    #[must_use]
    pub fn is_timeout(&self) -> bool {
        self.0.is_timeout()
    }

    /// Whether the connection could not be made (refused, DNS failure, TLS
    /// handshake), so the request never reached the server.
    ///
    /// # Returns
    ///
    /// `true` for a connection failure.
    #[must_use]
    pub fn is_connect(&self) -> bool {
        self.0.is_connect()
    }
}

impl fmt::Display for TransportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl std::error::Error for TransportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        // reqwest's own cause (an I/O or hyper error), not reqwest's error
        // itself, whose message `Display` already shows.
        self.0.source()
    }
}
```

`std::error::Error` is written out in full in this impl because the module's own `Error` enum shadows the name.

In `src/lib.rs`, replace line 104 with the line below and add the `secrecy` re-export after the `pub use` block (after line 124):

```rust
pub use error::{ConnectCode, Error, Result, TransportError};
```

```rust
/// The `secrecy` crate, at the version this crate's API uses: the envd and
/// MCP tokens are [`secrecy::SecretString`]s, and `secrecy::ExposeSecret`
/// reads them.
pub use secrecy;
```

Now convert every reqwest result. With `#[from]` gone, `cargo check --all-targets` lists each remaining site as ``E0277 `?` couldn't convert the error to `error::Error` ``. Re-run it after each batch, because one error can hide the next. On 7ee3eab the list is these 32 lines:

| File:line | Before | After |
|---|---|---|
| `src/client.rs:105` | `.build()?;` | `.build().map_err(Error::transport)?;` |
| `src/client.rs:110` | `.build()?;` | `.build().map_err(Error::transport)?;` |
| `src/client.rs:189` | `parse_success(request.send().await?).await?` | `parse_success(request.send().await.map_err(Error::transport)?).await?` |
| `src/client.rs:325`, `:372`, `:420`, `:447`, `:683`, `:717`, `:911`, `:959` | `.await?;` after `.send()` | `.await` then `.map_err(Error::transport)?;` |
| `src/client.rs:932` | `.send().await?;` | `.send().await.map_err(Error::transport)?;` |
| `src/client.rs:987` | `Ok(self.request(method, segments)?.json(body).send().await?)` | `self.request(method, segments)?.json(body).send().await.map_err(Error::transport)` |
| `src/client.rs:1024` | `Err(err) => Error::Transport(err),` | `Err(err) => Error::transport(err),` |
| `src/client.rs:1044` | `.bytes().await?;` | `.bytes().await.map_err(Error::transport)?;` |
| `src/envd/connect.rs:368` | `None => Ok(self.response.chunk().await?),` | `None => self.response.chunk().await.map_err(Error::transport),` |
| `src/envd/connect.rs:371` | `Ok(chunk) => Ok(chunk?),` | `Ok(chunk) => chunk.map_err(Error::transport),` |
| `src/envd/mod.rs:349` | `builder.send().await?;` | `builder.send().await.map_err(Error::transport)?;` |
| `src/envd/mod.rs:352` | `response.bytes().await?;` | `response.bytes().await.map_err(Error::transport)?;` |
| `src/envd/mod.rs:478` | `.await?;` after `.send()` | `.await.map_err(Error::transport)?;` |
| `src/envd/mod.rs:482` | `response.bytes().await?;` | `response.bytes().await.map_err(Error::transport)?;` |
| `src/envd/mod.rs:580` | `Err(err) => Error::Transport(err),` | `Err(err) => Error::transport(err),` |
| `src/filesystem.rs:712` | `Ok(self.response.chunk().await?)` | `self.response.chunk().await.map_err(Error::transport)` |
| `src/filesystem.rs:816` | `written_entries(request.body(body).send().await?).await` | `written_entries(request.body(body).send().await.map_err(Error::transport)?).await` |
| `src/filesystem.rs:854`, `:879`, `:936` | `.await?;` after `.send()` | `.await.map_err(Error::transport)?;` |
| `src/filesystem.rs:883` | `Ok(response.bytes().await?)` | `response.bytes().await.map_err(Error::transport)` |
| `src/filesystem.rs:1421` | `response.bytes().await?;` | `response.bytes().await.map_err(Error::transport)?;` |
| `src/sandbox.rs:568` | `.await?;` after `.send()` | `.await.map_err(Error::transport)?;` |

Each file already imports `crate::error::Error` . No other `Error::Transport(` construction exists in `src/` (`grep -rn "Error::Transport(" src` shows only the docs afterwards).

Add unit tests to `src/error.rs`'s `tests` module, for the cases a public caller cannot produce through `is_running` or the RPC mock (an envd 502, a REST 503 compared with an envd 503, `Timeout`, a 404 status):

```rust
    #[test]
    fn status_is_the_http_status_of_api_and_envd_only() {
        let api = Error::Api { status: 403, message: String::new() };
        let envd = Error::Envd { status: 507, message: String::new() };
        let rpc = Error::Rpc { code: ConnectCode::NotFound, message: String::new() };
        assert_eq!(api.status(), Some(403));
        assert_eq!(envd.status(), Some(507));
        assert_eq!(rpc.status(), None);
        assert_eq!(Error::SandboxNotFound("sbx".into()).status(), None);
        assert_eq!(Error::Timeout(Duration::from_secs(1)).status(), None);
    }

    #[test]
    fn service_busy_is_a_rest_503_only() {
        assert!(Error::Api { status: 503, message: String::new() }.is_service_busy());
        assert!(!Error::Envd { status: 503, message: String::new() }.is_service_busy());
        assert!(
            !Error::Rpc { code: ConnectCode::Unavailable, message: String::new() }
                .is_service_busy()
        );
    }

    #[test]
    fn an_envd_502_and_an_rpc_unavailable_are_sandbox_unreachable() {
        let proxy = Error::Envd {
            status: 502,
            message: "The sandbox was not found".into(),
        };
        let rpc = Error::Rpc { code: ConnectCode::Unavailable, message: String::new() };
        assert!(proxy.is_sandbox_unreachable());
        assert!(rpc.is_sandbox_unreachable());
        assert!(!Error::Api { status: 502, message: String::new() }.is_sandbox_unreachable());
        assert!(!proxy.is_retryable());
    }

    #[test]
    fn timeouts_cover_the_client_deadline_and_the_rpc_codes() {
        assert!(Error::Timeout(Duration::from_millis(1)).is_timeout());
        for code in [ConnectCode::DeadlineExceeded, ConnectCode::Canceled] {
            assert!(Error::Rpc { code, message: String::new() }.is_timeout(), "{code}");
        }
        assert!(!Error::Rpc { code: ConnectCode::Internal, message: String::new() }.is_timeout());
        assert!(!Error::Api { status: 504, message: String::new() }.is_timeout());
    }

    #[test]
    fn auth_and_rate_limits_cover_both_http_apis_and_rpc() {
        for error in [
            Error::Api { status: 401, message: String::new() },
            Error::Envd { status: 401, message: String::new() },
            Error::Rpc { code: ConnectCode::Unauthenticated, message: String::new() },
        ] {
            assert!(error.is_auth(), "{error:?}");
            assert!(!error.is_retryable(), "{error:?}");
        }
        for error in [
            Error::Api { status: 429, message: String::new() },
            Error::Envd { status: 429, message: String::new() },
            Error::Rpc { code: ConnectCode::ResourceExhausted, message: String::new() },
        ] {
            assert!(error.is_rate_limited(), "{error:?}");
            assert!(error.is_retryable(), "{error:?}");
        }
        assert!(!Error::MissingApiKey.is_auth());
        assert!(!Error::Api { status: 403, message: String::new() }.is_auth());
    }
```

The existing `is_not_found_covers_each_family` test (lines 351-392) stays as it is. It builds the variants inside the crate, where `#[non_exhaustive]` does not apply.

- [ ] **Step 4: Fix the patterns outside the crate**

Outside the crate, `#[non_exhaustive]` struct variants must be matched with `..`. `cargo test --no-run` reports each site as ``E0638 `..` required with variant marked as non-exhaustive``. On 7ee3eab there are 19. Add `, ..` before the closing brace of each:

| File:line | Pattern |
|---|---|
| `tests/envd_api.rs:193` | `Error::Rpc { code: ConnectCode::DeadlineExceeded, ref message }` |
| `tests/envd_api.rs:641` | `Error::Rpc { code: ConnectCode::Unavailable, ref message }` |
| `tests/envd_api.rs:690` | `Error::Envd { status: 404, ref message }` |
| `tests/envd_stream_files.rs:325` | `Error::Envd { status: 404, message }` |
| `tests/envd_write.rs:384` | `Error::Envd { status: 507, message }` |
| `tests/rest_api.rs:79` | `Error::Api { status: 400, ref message }` |
| `tests/rest_api.rs:328` | `Error::Api { status: 500, ref message }` |
| `tests/rest_fork.rs:52` | `Error::Api { status: 429, message }` |
| `tests/rest_fork.rs:62` | `Error::Api { status: 500, message }` |
| `tests/rest_fork.rs:166` | `Error::Api { status: 409, message }` |
| `tests/rest_lifecycle.rs:449` | `Error::Api { status: 400, ref message }` |
| `tests/rest_network.rs:304` | `Error::Api { status: 400, ref message }` |
| `tests/rest_snapshots.rs:118` | `Error::Api { status: got, message }` |
| `tests/rest_snapshots.rs:228` | `Error::Api { status: 401, message }` |
| `tests/rest_snapshots.rs:288` | `Error::Api { status: 500, message }` |
| `tests/rest_volumes.rs:70` | `Error::Api { status: got, message: text }` |
| `tests/rest_volumes.rs:131` | `Error::Api { status: 500, message }` |
| `tests/rest_volumes.rs:185` | `Error::Api { status: 401, message }` |
| `tests/rest_volumes.rs:225` | `Error::Api { status: 500, message }` |

For example, `Error::Api { status: 400, ref message }` becomes `Error::Api { status: 400, ref message, .. }`. No live test and no README example names every field (`grep -n "Error::\(Api\|Rpc\|Envd\) {" tests/live_*.rs README.md` finds nothing). The three transport tests keep compiling unchanged: `tests/rest_api.rs:512` and `tests/envd_stream_files.rs:64` call `inner.is_timeout()`, now on `TransportError`, and `tests/envd_stream_files.rs:242` matches `Error::Transport(_)`.

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test --test error_taxonomy && cargo test --lib error && cargo test --doc error`
Expected: PASS (5 integration tests, the 5 new unit tests plus the 4 existing ones, and the doctests on `status`, `is_not_found`, `is_timeout`, `is_retryable` and `TransportError`).

Then check CLM-0170 by inspection. `cargo doc --no-deps` must show no `reqwest` path anywhere in the public docs:

```bash
cargo doc --no-deps && ! grep -rl "reqwest" target/doc/e2b_client/ --include=*.html
```

Expected: the command exits 0 and prints nothing. It relies on Step 6's rewording of the one public rustdoc line that names reqwest in prose (`src/filesystem.rs:1083-1084`; `grep -rn "^\s*///.*reqwest" src` finds no other public one today, `gzip_body` at `src/filesystem.rs:1435` being private). If a page still names reqwest, look at what names it: a type path (`reqwest::...` in a signature, a field or an impl) means the API still exposes it, so fix the exposure, don't hide it; prose means a doc comment to reword the same way.

- [ ] **Step 6: Docs**

`src/filesystem.rs:1083-1084`, the `reader` argument of `write_stream`, stops naming the HTTP client, so the public docs no longer say "reqwest" (Step 5's check):

```rust
    /// * `reader` - the content, read to its end (a socket, a byte slice, a
    ///   `tokio::fs::File`...). It must be `Send + 'static` because the HTTP
    ///   client may poll the body on another task, so it cannot borrow local data.
```

`README.md`, replace the "Errors" row (line 63) with:

```markdown
| Errors | `Error` (`thiserror`): REST status + E2B message, Connect error codes, envd HTTP errors, protocol, timeout, transport (`TransportError`, with `is_timeout`/`is_connect`), invalid argument; `status()` and the SDK's categories as `is_not_found`, `is_auth`, `is_rate_limited`, `is_service_busy`, `is_not_enough_space`, `is_sandbox_unreachable`, `is_timeout`, `is_retryable` |
```

`docs/protocol-notes.md`, insert before `## Deliberate differences from the SDKs` (line 417):

```markdown
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
```

In "Deliberate differences from the SDKs", add this bullet first:

```markdown
- **Classifier methods, not exception classes.** The SDKs raise one class per
  category; this crate keeps the protocol in the variant (`Api`, `Envd`,
  `Rpc`) and answers the category with `Error::is_*` (see "Error
  classification"). `is_sandbox_unreachable` stands for both
  `SandboxNotRunningException` and `SandboxUnreachableException`.
```

`docs/verification/s4-evidence-map.md`, append after the CLM-0168 row (Task 1; if Task 1 has not landed, after CLM-0167):

```markdown
| CLM-0169 | tests/error_taxonomy.rs::rest_statuses_classify_as_the_sdk_does; tests/error_taxonomy.rs::envd_http_statuses_classify_as_the_sdk_does; tests/error_taxonomy.rs::rpc_codes_classify_as_the_sdk_does; tests/error_taxonomy.rs::a_request_timeout_is_a_timeout_and_not_retryable; tests/error_taxonomy.rs::a_refused_connection_is_retryable_and_keeps_its_cause; src/error.rs::tests::status_is_the_http_status_of_api_and_envd_only; src/error.rs::tests::service_busy_is_a_rest_503_only; src/error.rs::tests::an_envd_502_and_an_rpc_unavailable_are_sandbox_unreachable; src/error.rs::tests::timeouts_cover_the_client_deadline_and_the_rpc_codes; src/error.rs::tests::auth_and_rate_limits_cover_both_http_apis_and_rpc | yes | new | REST answers 401, 429, 503 and 500 to `get_sandbox_info` give `Error::Api` with that `status()` and exactly the SDK's flags (all seven classifiers compared at once): auth; rate limited and retryable; service busy and retryable; none. envd `/health` answers 401, 429, 507 and 503 give `Error::Envd` with auth, rate limited plus retryable, not enough space, and none (an envd 503 is not service busy). RPC answers `unauthenticated`, `resource_exhausted`, `unavailable`, `deadline_exceeded`, `canceled` and `internal` to `Commands::list` give `Error::Rpc` with `status() == None` and auth, rate limited plus retryable, unreachable, timeout, timeout and none. A request timeout is `Error::Transport` with `is_timeout`, not retryable; a refused connection is `is_connect` and retryable, and its `source()` is the `TransportError`. The unit tests pin the envd 502 (unreachable, not retryable), REST against envd 503, the client `Timeout`, `status()` for the non-HTTP variants, and that `MissingApiKey` and a 403 are not `is_auth`. |
```

- [ ] **Step 7: Gates, changelog, commit**

Run: `just lint && just test && just doc`
Expected: all green. `git diff Cargo.lock` is empty.

No live test changes. Run `clm_0015_connect_unknown_sandbox_is_not_found` on hosted once, as a smoke test of the converted transport path. Expected: PASS.

Create `changelog/<date -u +%Y-%m-%dT%H-%M-%SZ>-m4-task-02-error-taxonomy.md`:

```markdown
# M4 Task 2: error taxonomy and an opaque transport error (issue #<N>)

`Error` gains `status()` and the SDK 2.61 categories as classifiers:
`is_auth`, `is_rate_limited`, `is_service_busy`, `is_not_enough_space`,
`is_sandbox_unreachable`, `is_timeout` and `is_retryable`, next to
`is_not_found` (CLM-0169). `Error::Transport` holds the new opaque
`TransportError` (`is_timeout`, `is_connect`, `Display`, `source()`), so no
reqwest type is public, and `secrecy` is re-exported (CLM-0170).

## Breaking changes

- `Error::Transport(reqwest::Error)` is now `Error::Transport(TransportError)`.
  Code that called reqwest methods on it uses `TransportError::is_timeout` /
  `is_connect`, or `Error::is_timeout` / `is_retryable`.
- `impl From<reqwest::Error> for Error` is gone.
- `Error::Api`, `Error::Rpc` and `Error::Envd` are `#[non_exhaustive]`:
  patterns need `..` (`Error::Api { status, .. }`), and the variants can no
  longer be built outside the crate.

## Tests

- New: `tests/error_taxonomy.rs` (5 tests), 5 unit tests in `src/error.rs`,
  doctests on the classifiers and `TransportError`.
- Changed: 19 patterns in 9 test files gained `..`.
- Inspection: `target/doc/e2b_client/` names no reqwest type.
```

```bash
git add src/ tests/ README.md docs/protocol-notes.md docs/verification/s4-evidence-map.md changelog/
git commit -m "feat!: classify errors as the SDK does and keep reqwest private"
```


---

### Task 3: A refused moded pause is an error (#81, CLM-0171)

**Files:**
- Modify: `src/client.rs` (`ALREADY_PAUSED`; the 409 arm of `pause_sandbox_with`, lines 501-517; rustdoc of `pause_sandbox`, lines 454-469, and `pause_sandbox_with`, lines 475-500)
- Modify: `src/sandbox.rs` (rustdoc of `pause_with`, lines 604-617)
- Modify: `tests/rest_lifecycle.rs` (replace `pause_with_maps_status_codes_as_pause_does`, lines 422-456; new test)
- Modify: `tests/rest_api.rs` (rename `pause_sandbox_maps_status_codes`, line 334)
- Modify: `tests/live_lifecycle.rs` (the comment in `clm_0146_filesystem_pause_cold_boots`, lines 949-952 and 963-966; new `clm_0171_moded_pause_of_a_paused_sandbox_is_false`)
- Modify: `docs/protocol-notes.md` (the "Pause" row, line 85; the "Not implemented yet" bullet, lines 430-432; a "Deliberate differences" bullet)
- Create: `changelog/<ts>-m4-task-03-refused-moded-pause.md`

> **Corrected (issue #81, 2026-10-28).** SDK 2.61 reads the pause 409 at `py: sandbox_async/sandbox_api.py:421-423`, not 420-422; this task and Task 6 now cite 421-423. In `docs/protocol-notes.md` the "Pause" row is line 84 and the "Not implemented yet" bullet lines 443-445. `docs/verification/s4-evidence-map.md` row CLM-0095 also names `pause_sandbox_maps_status_codes`, so it takes the new name too. The new `clm_0171` sits after `clm_0024` and shifts `tests/live_lifecycle.rs` below it by 22 lines (21 from `clm_0146`'s Act comment down); Task 6's line numbers there, and in `src/client.rs` and `src/sandbox.rs`, are corrected to the post-Task-3 file.

Decision 2. SDK 2.61 reads every pause 409 as "already paused" (`py: sandbox_async/sandbox_api.py:421-423`), so a refused filesystem-only pause looks like success. Hosted E2B's "already paused" 409 says so in its message (Verified facts: `Error pausing sandbox - sandbox '<id>' is already paused`, with or without `memory: false`). So when a mode was asked, the crate reads the 409 body: that message is `Ok(false)`, and anything else is `Error::Api { status: 409, .. }`. A pause without a mode keeps SDK parity.

- [ ] **Step 1: Write the failing mock tests**

In `tests/rest_lifecycle.rs`, replace `pause_with_maps_status_codes_as_pause_does` (lines 422-456) with the two tests below. Use the hosted message verbatim:

```rust
/// Hosted E2B's 409 for pausing a sandbox that is already paused (live,
/// 2026-10-28), with the sandbox id it names.
fn already_paused_body(id: &str) -> Value {
    json!({
        "code": 409,
        "message": format!("Error pausing sandbox - sandbox '{id}' is already paused"),
    })
}

#[tokio::test]
async fn a_moded_pause_reads_only_an_already_paused_409_as_false() {
    // Arrange: per sandbox id, the answer the API gives.
    let server = MockServer::start().await;
    let answers = [
        ("already", ResponseTemplate::new(409).set_body_json(already_paused_body("already"))),
        (
            "refused",
            ResponseTemplate::new(409).set_body_json(json!({
                "code": 409,
                "message": "snapshot_filesystem_only_unsupported_node",
            })),
        ),
        ("empty", ResponseTemplate::new(409)),
        (
            "disabled",
            ResponseTemplate::new(400)
                .set_body_json(json!({"code": 400, "message": "snapshot_filesystem_only_disabled"})),
        ),
        ("missing", ResponseTemplate::new(404)),
    ];
    for (id, template) in answers {
        Mock::given(method("POST"))
            .and(path(format!("/sandboxes/{id}/pause")))
            .respond_with(template)
            .expect(1)
            .mount(&server)
            .await;
    }
    let client = client_for(&server);
    let filesystem = PauseOptions::new().mode(SnapshotMode::Filesystem);

    // Act
    let already = client.pause_sandbox_with("already", &filesystem).await;
    let refused = client.pause_sandbox_with("refused", &filesystem).await;
    let empty = client.pause_sandbox_with("empty", &filesystem).await;
    let disabled = client.pause_sandbox_with("disabled", &filesystem).await;
    let missing = client.pause_sandbox_with("missing", &filesystem).await;

    // Assert
    assert!(!already.expect("an already-paused 409 is Ok(false)"));
    assert!(
        matches!(refused, Err(Error::Api { status: 409, ref message, .. })
            if message == "snapshot_filesystem_only_unsupported_node"),
        "{refused:?}"
    );
    assert!(
        matches!(empty, Err(Error::Api { status: 409, .. })),
        "a 409 without the message is a refusal: {empty:?}"
    );
    assert!(
        matches!(disabled, Err(Error::Api { status: 400, .. })),
        "{disabled:?}"
    );
    assert!(
        matches!(missing, Err(Error::SandboxNotFound(ref id)) if id == "missing"),
        "{missing:?}"
    );
}

#[tokio::test]
async fn a_full_pause_also_reads_the_409_body() {
    // Arrange: `Full` is a mode too, so it gets the same check.
    let server = MockServer::start().await;
    for (id, template) in [
        ("already", ResponseTemplate::new(409).set_body_json(already_paused_body("already"))),
        (
            "refused",
            ResponseTemplate::new(409).set_body_json(json!({"code": 409, "message": "busy"})),
        ),
    ] {
        Mock::given(method("POST"))
            .and(path(format!("/sandboxes/{id}/pause")))
            .and(body_json(json!({"memory": true})))
            .respond_with(template)
            .expect(1)
            .mount(&server)
            .await;
    }
    let client = client_for(&server);
    let full = PauseOptions::new().mode(SnapshotMode::Full);

    // Act
    let already = client.pause_sandbox_with("already", &full).await;
    let refused = client.pause_sandbox_with("refused", &full).await;

    // Assert
    assert!(!already.expect("already paused"));
    assert!(
        matches!(refused, Err(Error::Api { status: 409, ref message, .. }) if message == "busy"),
        "{refused:?}"
    );
}

#[tokio::test]
async fn a_pause_without_a_mode_reads_every_409_as_already_paused() {
    // Arrange: SDK parity. Any 409, whatever its message, is `Ok(false)`.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/sandboxes/{SANDBOX_ID}/pause")))
        .and(body_json(json!({})))
        .respond_with(
            ResponseTemplate::new(409).set_body_json(json!({"code": 409, "message": "busy"})),
        )
        .expect(1)
        .mount(&server)
        .await;

    // Act
    let paused = client_for(&server)
        .pause_sandbox_with(SANDBOX_ID, &PauseOptions::new())
        .await;

    // Assert
    assert!(!paused.expect("SDK parity: 409 is already paused"));
}
```

In `tests/rest_api.rs`, rename `pause_sandbox_maps_status_codes` (line 334) to `pause_sandbox_without_a_mode_maps_status_codes`. Its body stays: an empty-bodied 409 with no mode is still `Ok(false)`.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --test rest_lifecycle pause`
Expected: FAIL. In `a_moded_pause_reads_only_an_already_paused_409_as_false`, `refused` is `Ok(false)` (the assertion prints `Ok(false)`). In `a_full_pause_also_reads_the_409_body`, likewise. `a_pause_without_a_mode_reads_every_409_as_already_paused` passes already.

- [ ] **Step 3: Implement the check**

In `src/client.rs`, add after `HEADER_IDEMPOTENCY_KEY` (line 34):

```rust
/// What hosted E2B's 409 for a pause says when the sandbox is already paused
/// (`Error pausing sandbox - sandbox '<id>' is already paused`, live
/// 2026-10-28). Used only for a pause that asked for a mode (issue #81).
const ALREADY_PAUSED: &str = "already paused";
```

Replace the body of `pause_sandbox_with` (lines 506-517) with:

```rust
        let body = PauseBody {
            memory: options.memory(),
        };
        let response = self
            .send_json(Method::POST, &["sandboxes", sandbox_id, "pause"], &body)
            .await?;
        match response.status() {
            StatusCode::NOT_FOUND => Err(Error::SandboxNotFound(sandbox_id.to_owned())),
            // No mode: every 409 is "already paused", as in the SDK.
            StatusCode::CONFLICT if body.memory.is_none() => Ok(false),
            // A mode was asked: only the "already paused" message is; any
            // other 409 is the API refusing that mode (issue #81).
            StatusCode::CONFLICT => match api_error(response).await {
                Error::Api { ref message, .. } if message.contains(ALREADY_PAUSED) => Ok(false),
                error => Err(error),
            },
            _ => ensure_success(response).await.map(|_| true),
        }
```

`api_error` (line 1012) turns the 409 into `Error::Api { status: 409, message }`, using the body's `message` or `HTTP 409` for an empty body. If the body can't be read it returns `Error::Transport`. Either way the result goes to the caller unchanged.

Replace the `# Returns` and `# Errors` of `pause_sandbox` (lines 462-468) with:

```rust
    /// # Returns
    ///
    /// `true` if it was paused, `false` if it was already paused (any HTTP
    /// 409, as in the SDK).
    ///
    /// # Errors
    ///
    /// [`Error::SandboxNotFound`] for HTTP 404, [`Error::Api`] for other
    /// non-2xx answers, [`Error::Transport`] for connection failures.
```

Replace the `# Returns` and `# Errors` of `pause_sandbox_with` (lines 485-497) with:

```rust
    /// # Returns
    ///
    /// `true` if it was paused, `false` if it was already paused. Without a
    /// mode any HTTP 409 means already paused, as in the SDK. With a mode,
    /// only a 409 whose message says the sandbox "is already paused" does.
    ///
    /// # Errors
    ///
    /// As [`Client::pause_sandbox`], and with a mode: an [`Error::Api`] with
    /// status 409 for any other 409, which is the API refusing that mode
    /// (for example `snapshot_filesystem_only_unsupported_node`, the node
    /// predates filesystem-only snapshots). SDK 2.61 returns `false` for
    /// every 409 (`e2b/sandbox_async/sandbox_api.py:421-423`); the crate
    /// departs from it here so a refusal is not mistaken for a pause. A
    /// filesystem-only pause refused for the team is an [`Error::Api`] with
    /// status 400.
```

In `src/sandbox.rs`, replace the `# Returns` of `pause_with` (lines 610-612) with:

```rust
    /// # Returns
    ///
    /// `true` if it was paused, `false` if it was already paused (see
    /// [`Client::pause_sandbox_with`] for how a 409 is read when a mode is
    /// set).
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --test rest_lifecycle --test rest_api pause`
Expected: PASS.

- [ ] **Step 5: The live test and the clm_0146 comment**

In `tests/live_lifecycle.rs`, `clm_0146_filesystem_pause_cold_boots`: replace the Act comment (lines 949-952) with:

```rust
    // Act: hosted E2B may refuse a filesystem-only pause for the team (400)
    // or the node (409, an error since issue #81); either is an environment
    // limit, to be recorded as such, not worked around.
```

and the first assertion (lines 963-966) with:

```rust
    assert!(
        paused,
        "a freshly created, running sandbox's filesystem pause returned false: since issue \
         #81 that means the server answered 409 \"already paused\", contradicting itself"
    );
```

Change the `.expect` message on the pause (line 956) to `"filesystem pause (HTTP 400 or 409 means the team or node lacks it)"`.

Append after `clm_0024_pause_paused_sandbox_is_false`, so the test pins the hosted message the check relies on:

```rust
#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0171_moded_pause_of_a_paused_sandbox_is_false() {
    // Arrange
    let sandbox = LiveSandbox::create(&params().timeout(Duration::from_secs(120))).await;
    assert!(sandbox.pause().await.expect("first pause"));

    // Act: with a mode, the 409 body is read; hosted E2B's says "already
    // paused", which must still be `false` rather than an error.
    let second = sandbox
        .client()
        .pause_sandbox_with(
            sandbox.sandbox_id(),
            &PauseOptions::new().mode(SnapshotMode::Filesystem),
        )
        .await
        .expect("a moded pause of a paused sandbox");

    // Assert
    assert!(!second);
}
```

Run it and `clm_0024_pause_paused_sandbox_is_false` on hosted, one at a time. Expected: PASS. Report both in the PR. If `clm_0171` fails with `Error::Api { status: 409 }`, the hosted message changed: correct `ALREADY_PAUSED` and the Verified facts row, then rerun.

- [ ] **Step 6: Docs**

`docs/protocol-notes.md`, the "Pause" row (line 85). Replace `409 → already paused (`false`), for every mode. The SDK documents a 400 or 409 refusal of filesystem-only for snapshots, not for pause; hosted E2B took a filesystem pause on 2026-10-28` with:

`409 without a mode → already paused (`false`), as in the SDK; 409 with a mode → `false` only when the message says the sandbox "is already paused" (hosted E2B: `Error pausing sandbox - sandbox '<id>' is already paused`, live 2026-10-28), else `Error::Api` 409 (issue #81). The SDK documents a 400 or 409 refusal of filesystem-only for snapshots, not for pause; hosted E2B took a filesystem pause on 2026-10-28`

In the citation cell, append `; live probe, hosted E2B, 2026-10-28`.

In the "Not implemented yet" bullet (lines 430-432), delete `a refused filesystem-only pause (409) that reads as "already paused" (#81), `.

In "Deliberate differences from the SDKs", add:

```markdown
- **A refused moded pause is an error.** SDK 2.61 returns `False` for every
  pause 409. With a `SnapshotMode`, this crate returns `false` only for the
  409 that says the sandbox "is already paused", and `Error::Api` 409 for any
  other, so a refused filesystem-only pause is not mistaken for a pause
  (issue #81). Without a mode it matches the SDK.
```

`docs/verification/s4-evidence-map.md`, row CLM-0159: in the Test column, replace `tests/rest_lifecycle.rs::pause_with_maps_status_codes_as_pause_does` with `tests/rest_lifecycle.rs::a_moded_pause_reads_only_an_already_paused_409_as_false`. In the Notes, replace `For a filesystem pause, 409 is `Ok(false)` as for every pause (the SDK reads every 409 as already paused), 400 is` with `For a filesystem pause, the already-paused 409 is `Ok(false)` and any other 409 an `Error::Api` (CLM-0171), 400 is`. CLM-0171 is an S1 claim backed by tests, and Task 18 records its evidence from `tests/rest_lifecycle.rs` directly. It gets no map row, as CLM-0146 has none.

- [ ] **Step 7: Gates, changelog, commit**

Run: `just lint && just test && just doc`
Expected: all green.

Create `changelog/<date -u +%Y-%m-%dT%H-%M-%SZ>-m4-task-03-refused-moded-pause.md`:

```markdown
# M4 Task 3: a refused moded pause is an error (issue #81)

A pause that asks for a `SnapshotMode` now reads the 409 body: hosted E2B's
"... is already paused" is `Ok(false)`, and any other 409 is `Error::Api`
with status 409, so a refused filesystem-only pause is no longer mistaken
for success (CLM-0171). A pause without a mode keeps SDK 2.61's reading of
every 409 as "already paused". Decided by pgrace while planning M4.

## Breaking changes

- `pause_sandbox_with(id, &PauseOptions::new().mode(..))` and
  `Sandbox::pause_with` return `Err(Error::Api { status: 409, .. })` for a
  409 that is not "already paused", where they returned `Ok(false)`.

## Tests

- New: `a_moded_pause_reads_only_an_already_paused_409_as_false`,
  `a_full_pause_also_reads_the_409_body`,
  `a_pause_without_a_mode_reads_every_409_as_already_paused`
  (`tests/rest_lifecycle.rs`); live `clm_0171_moded_pause_of_a_paused_sandbox_is_false`.
- Replaced: `pause_with_maps_status_codes_as_pause_does`. Renamed:
  `tests/rest_api.rs::pause_sandbox_without_a_mode_maps_status_codes`.
- Live on hosted: <results of clm_0171 and clm_0024>.
```

```bash
git add src/client.rs src/sandbox.rs tests/rest_lifecycle.rs tests/rest_api.rs tests/live_lifecycle.rs docs/ changelog/
git commit -m "feat!: report a refused moded pause instead of reading it as already paused"
```


---

### Task 4: A zero timeout means no limit (#52, CLM-0172)

**Files:**
- Modify: `src/config.rs` (`no_limit_if_zero`; `ConnectionConfigBuilder::request_timeout` at `src/config.rs:380-394`)
- Modify: `src/commands.rs` (`CommandOptions::timeout`, `src/commands.rs:126-140`)
- Modify: `src/pty.rs` (`PtyOptions::timeout`, `src/pty.rs:167-189`)
- Modify: `src/filesystem.rs` (`WatchOptions::timeout`, `src/filesystem.rs:548-567`)
- Modify: `src/stream_input.rs` (`StreamInputOptions::timeout`, `src/stream_input.rs:64-81`)
- Create: `tests/zero_timeouts.rs`
- Modify: `docs/protocol-notes.md` (the "Timeouts sent as whole ms" row at line 202, the StreamInput deadline row at line 259, the WatchDir headers row at line 376), `docs/verification/s4-evidence-map.md` (CLM-0172 row)
- Create: `changelog/<ts>-m4-task-04-zero-timeout-no-limit.md`

> **Corrected (issue #52, 2026-10-28).** The rule also covers the two deadlines that are not set through an options struct yet: `Commands::connect(pid, timeout)` and `Pty::connect(pid, timeout)` pass `no_limit_if_zero(timeout)` to `open`, so "every stream deadline" holds between this task and Task 7. `tests/zero_timeouts.rs` therefore has seven tests, not five: `a_zero_command_connect_timeout_sends_no_deadline` and `a_zero_pty_connect_timeout_sends_no_deadline` join the plan's five, and the CLM-0172 row names them too. Task 7 converts their `connect(7, Some(Duration::ZERO))` calls (see its Step 5). In `docs/protocol-notes.md` the rows are one line higher than Step 6 says: "Timeouts sent as whole ms" is line 201, the StreamInput deadline row 258 and the WatchDir headers row 375 (the new request-timeout row goes after line 29 as planned). This task shifts `src/commands.rs` and `src/pty.rs` down by 8 lines below their `connect`, and `src/filesystem.rs` up by 2 below `WatchOptions::timeout`; the line numbers that Tasks 5 and 12 cite in those files are corrected to the post-Task-4 files. The changelog slug is `zero-timeout-no-limit`.

The rule lives in one helper. Every public setter that takes an `Option<Duration>` deadline or request timeout stores `no_limit_if_zero(timeout)`. The fields are private, so a setter is the only way in, and `protocol_headers`, `within` and `ConnectStream` never see a zero. `Connect-Timeout-Ms` keeps "at least 1 ms" for positive sub-millisecond values (`src/envd/connect.rs:261-268`), as the SDK's `max(1, round(ms))` does (`py: envd/utils.py:15-24`).

- [ ] **Step 1: Write the failing tests**

Create `tests/zero_timeouts.rs`:

```rust
//! A zero timeout means "no limit" on every stream deadline and request
//! timeout, as SDK 2.61's `timeout=0` (`py: envd/utils.py:15-24`) and
//! `request_timeout=0` (`py: api/__init__.py:280`) do (CLM-0172, #52).

mod common;

use std::time::Duration;

use common::{
    client_with_timeout, end_stream, message, sandbox, sandbox_from_json_with_client,
    sandbox_json, stream_response,
};
use e2b_client::{CommandOptions, PtyOptions, PtySize, StreamInputOptions, WatchOptions};
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

/// The one request `server` received on `route`.
async fn request_to(server: &MockServer, route: &str) -> Request {
    server
        .received_requests()
        .await
        .expect("recording")
        .into_iter()
        .find(|request| request.url.path() == route)
        .unwrap_or_else(|| panic!("no request to {route}"))
}

/// A process stream: start, then a clean exit.
fn process_frames() -> Vec<Vec<u8>> {
    vec![
        message(json!({"event": {"start": {"pid": 7}}})),
        message(json!({"event": {"end": {"exitCode": 0, "exited": true}}})),
        end_stream(json!({})),
    ]
}

#[tokio::test]
async fn a_zero_command_timeout_sends_no_deadline() {
    // Arrange
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    Mock::given(method("POST"))
        .and(path("/process.Process/Start"))
        .respond_with(stream_response(&process_frames()))
        .expect(1)
        .mount(&server)
        .await;
    let options = CommandOptions::default().timeout(Some(Duration::ZERO));

    // Act
    let result = sandbox.commands().run("true", &options).await;

    // Assert: the stream was not cut off, and no deadline was sent.
    assert_eq!(result.expect("run").exit_code, 0);
    let start = request_to(&server, "/process.Process/Start").await;
    assert!(
        !start.headers.contains_key("connect-timeout-ms"),
        "a zero timeout must send no Connect-Timeout-Ms"
    );
}

#[tokio::test]
async fn a_zero_pty_timeout_sends_no_deadline() {
    // Arrange
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    Mock::given(method("POST"))
        .and(path("/process.Process/Start"))
        .respond_with(stream_response(&process_frames()))
        .expect(1)
        .mount(&server)
        .await;
    let options = PtyOptions::default().timeout(Some(Duration::ZERO));

    // Act
    let handle = sandbox
        .pty()
        .create(PtySize::new(80, 24), &options)
        .await
        .expect("create");
    let exit = handle.wait().await;

    // Assert
    assert_eq!(exit.expect("wait").exit_code, 0);
    let start = request_to(&server, "/process.Process/Start").await;
    assert!(!start.headers.contains_key("connect-timeout-ms"));
}

#[tokio::test]
async fn a_zero_watch_timeout_sends_no_deadline() {
    // Arrange: the watch opens, then envd ends it cleanly.
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    Mock::given(method("POST"))
        .and(path("/filesystem.Filesystem/WatchDir"))
        .respond_with(stream_response(&[
            message(json!({"start": {}})),
            end_stream(json!({})),
        ]))
        .expect(1)
        .mount(&server)
        .await;
    let options = WatchOptions::default().timeout(Some(Duration::ZERO));

    // Act
    let mut watch = sandbox
        .files()
        .watch_dir("/w", &options)
        .await
        .expect("watch");
    let end = watch.next_event().await;

    // Assert: a clean end, not a deadline error.
    assert!(matches!(end, Ok(None)), "{end:?}");
    let request = request_to(&server, "/filesystem.Filesystem/WatchDir").await;
    assert!(!request.headers.contains_key("connect-timeout-ms"));
}

#[tokio::test]
async fn a_zero_stream_input_timeout_sends_no_deadline() {
    // Arrange
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    Mock::given(method("POST"))
        .and(path("/process.Process/StreamInput"))
        .respond_with(stream_response(&[message(json!({})), end_stream(json!({}))]))
        .expect(1)
        .mount(&server)
        .await;
    let options = StreamInputOptions::default().timeout(Some(Duration::ZERO));

    // Act
    let mut input = sandbox.commands().stream_input(7, &options);
    input.send("x").await.expect("send");
    let closed = input.close().await;

    // Assert
    assert!(closed.is_ok(), "{closed:?}");
    let request = request_to(&server, "/process.Process/StreamInput").await;
    assert!(!request.headers.contains_key("connect-timeout-ms"));
}

#[tokio::test]
async fn a_zero_request_timeout_in_the_config_means_no_limit() {
    // Arrange: a configured request timeout of zero, and a unary call that
    // answers after 300 ms.
    let server = MockServer::start().await;
    let client = client_with_timeout(&server, Some(Duration::ZERO));
    let sandbox = sandbox_from_json_with_client(&server, &client, sandbox_json("0.5.7")).await;
    Mock::given(method("POST"))
        .and(path("/filesystem.Filesystem/ListDir"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"entries": []}))
                .set_delay(Duration::from_millis(300)),
        )
        .expect(1)
        .mount(&server)
        .await;

    // Act: `list(path, 1, None)` until Task 5 changes the signature.
    let entries = sandbox.files().list("/w", 1, None).await;

    // Assert: neither a deadline header nor a client-side timeout applied.
    assert!(entries.expect("list").is_empty());
    let request = request_to(&server, "/filesystem.Filesystem/ListDir").await;
    assert!(!request.headers.contains_key("connect-timeout-ms"));
    assert_eq!(client.config().request_timeout(), None);
}
```

Append to the `tests` module of `src/config.rs`:

```rust
    #[test]
    fn zero_means_no_limit() {
        assert_eq!(no_limit_if_zero(Some(Duration::ZERO)), None);
        assert_eq!(no_limit_if_zero(None), None);
        assert_eq!(
            no_limit_if_zero(Some(Duration::from_millis(1))),
            Some(Duration::from_millis(1))
        );
        let config = ConnectionConfig::builder()
            .request_timeout(Some(Duration::ZERO))
            .build_with_env(env_of(&[]))
            .expect("config");
        assert_eq!(config.request_timeout(), None);
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --test zero_timeouts` and `cargo test --lib config::tests::zero_means_no_limit`
Expected:
- the unit test fails to compile with "cannot find function `no_limit_if_zero`";
- the four stream tests fail, with either the `Connect-Timeout-Ms` assertion (the header is `1`) or an `Error::Timeout` from the 1 ms client-side deadline;
- the config test panics at `create sandbox`, because reqwest's zero timeout fails the create call at once.

- [ ] **Step 3: Add the helper**

In `src/config.rs`, after `DEFAULT_REQUEST_TIMEOUT` (line 25):

```rust
/// A timeout as the setters store it: `Some(Duration::ZERO)` means no limit,
/// as SDK 2.61's `timeout=0` and `request_timeout=0` do
/// (`e2b/envd/utils.py:15-24`, `e2b/api/__init__.py:280`).
///
/// Python note: in the SDK `0` is falsy, so `timeout or None` turns it into
/// "no timeout". Rust has no truthiness, so the rule is spelled out here once
/// and every setter calls it.
///
/// # Arguments
///
/// * `timeout` - the caller's value.
///
/// # Returns
///
/// `None` for `None` or zero, else the value unchanged.
pub(crate) fn no_limit_if_zero(timeout: Option<Duration>) -> Option<Duration> {
    // `filter` keeps `Some(d)` only when the closure returns true, like
    // `d if d else None` in Python.
    timeout.filter(|timeout| !timeout.is_zero())
}
```

`ConnectionConfigBuilder::request_timeout` (`src/config.rs:380-394`) becomes:

```rust
    /// Sets the per-request timeout. `None`, or a zero duration (the SDK's
    /// `request_timeout=0`), disables it.
    ///
    /// # Arguments
    ///
    /// * `timeout` - the timeout, or `None` (or zero) for no timeout.
    ///   Defaults to [`DEFAULT_REQUEST_TIMEOUT`].
    ///
    /// # Returns
    ///
    /// The builder, for chaining.
    #[must_use]
    pub fn request_timeout(mut self, timeout: Option<Duration>) -> Self {
        self.request_timeout = Some(no_limit_if_zero(timeout));
        self
    }
```

- [ ] **Step 4: Apply it in the four stream setters**

Each module imports `use crate::config::no_limit_if_zero;`, next to its other `crate::` imports.

`CommandOptions::timeout` (`src/commands.rs:126-140`):

```rust
    /// Bounds how long the command's output stream may stay open; `None`, or
    /// a zero duration (the SDK's `timeout=0`), for no limit. Defaults to
    /// [`DEFAULT_COMMAND_TIMEOUT`].
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
        self.timeout = no_limit_if_zero(timeout);
        self
    }
```

`PtyOptions::timeout` (`src/pty.rs:167-189`): replace the first sentence of the rustdoc with "Bounds the whole session; `None`, or a zero duration (the SDK's `timeout=0`), for no limit. Defaults to [`DEFAULT_PTY_TIMEOUT`] (60 s), as in the SDKs.". Keep the paragraph about the deadline counting from open; Task 8 rewrites it. Set the body to:

```rust
        self.timeout = no_limit_if_zero(timeout);
        self
```

`WatchOptions::timeout` (`src/filesystem.rs:548-567`): replace the doc and body with:

```rust
    /// Bounds the whole watch; `None`, or a zero duration (the SDK's
    /// `timeout=0`), for no limit. Defaults to [`DEFAULT_WATCH_TIMEOUT`]
    /// (60 s), as in the SDKs. The deadline counts from when the watch opens,
    /// not from the last event.
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
        self.timeout = no_limit_if_zero(timeout);
        self
    }
```

`StreamInputOptions::timeout` (`src/stream_input.rs:64-81`): change the first sentence to "Bounds the whole input stream; `None`, or a zero duration (the SDK's `timeout=0`), for no limit. Defaults to [`DEFAULT_STREAM_INPUT_TIMEOUT`].", and the body to `self.timeout = no_limit_if_zero(timeout); self`.

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test --test zero_timeouts && cargo test --lib config::tests`
Expected: PASS (5 + the config unit tests). Then run `just test`. `src/envd/connect.rs::tests::protocol_headers_include_version_and_timeout` still passes: 10 µs is positive, so it is still sent as 1 ms.

- [ ] **Step 6: Docs**

In `docs/protocol-notes.md`:
- line 202 becomes `| Timeouts sent as whole ms, at least 1; `None` or a zero duration sends no header and sets no client-side deadline (the SDK's `timeout_to_ms` maps `0` and `None` to no deadline) | `py: envd/utils.py:15-24` |`;
- line 259 ends "`None` or zero disables it";
- line 376's parenthesis becomes "(default 60 s; `None` or zero for no limit, as the SDK's `0`/`None`)";
- add a row after line 29: `| A zero request timeout disables it, as `request_timeout=0` | `py: api/__init__.py:280` |`.

In `docs/verification/s4-evidence-map.md`, add this row, and extend the intro line as the Conventions say:

```
| CLM-0172 | tests/zero_timeouts.rs::a_zero_command_timeout_sends_no_deadline; tests/zero_timeouts.rs::a_zero_pty_timeout_sends_no_deadline; tests/zero_timeouts.rs::a_zero_watch_timeout_sends_no_deadline; tests/zero_timeouts.rs::a_zero_stream_input_timeout_sends_no_deadline; tests/zero_timeouts.rs::a_zero_request_timeout_in_the_config_means_no_limit; src/config.rs::tests::zero_means_no_limit | partial | new | Each stream test sets `Some(Duration::ZERO)` on its options, checks the call completes (no 1 ms client-side deadline), and checks the request has no `Connect-Timeout-Ms`. The config test sets a zero request timeout: the create call and a `ListDir` that answers after 300 ms both succeed, with no header, and `request_timeout()` reads `None`. The per-call request timeout part is covered by Task 5's `tests/envd_file_options.rs::a_zero_per_call_request_timeout_means_no_limit`, which turns the row to `yes`. |
```

Task 5 changes `partial` to `yes` and appends its test name.

- [ ] **Step 7: Gates, changelog, commit**

Run `just lint`, `just test` and `just doc`. No live behaviour changes for non-zero values, and no live test calls these setters with zero, so no live run is needed.

Write `changelog/<ts>-m4-task-04-zero-timeout-no-limit.md`:

```markdown
# M4 Task 4: A zero timeout means no limit (issue #52)

Plan: `docs/superpowers/plans/2026-10-28-milestone-4-api-stability.md`, Task 4.
Claim: CLM-0172 (S4).

## What changed

- `src/config.rs`: `no_limit_if_zero`, the one place the rule lives.
  `ConnectionConfigBuilder::request_timeout(Some(Duration::ZERO))` now means
  no request timeout, as the SDK's `request_timeout=0`.
- `CommandOptions::timeout`, `PtyOptions::timeout`, `WatchOptions::timeout`
  and `StreamInputOptions::timeout` store `None` for a zero duration, so
  no `Connect-Timeout-Ms` header and no client-side deadline are set.
- Tests: `tests/zero_timeouts.rs` (5) and a `src/config.rs` unit test.
- Docs: the rustdoc of the five setters (the `WatchOptions` "Python note"
  that called zero an expired deadline is gone), four rows of
  `docs/protocol-notes.md`, and the CLM-0172 row of the S4 evidence map.

## Breaking changes

- `Some(Duration::ZERO)` on any of the five setters was a deadline that had
  already passed; it is now no limit. Code that used zero to make a call fail
  at once has to use `Some(Duration::from_millis(1))`.

## Why

pgrace chose the SDK's meaning on 2026-10-28 (plan decision 5), so code
ported from Python with `timeout=0` keeps working.
```

```bash
git add src/config.rs src/commands.rs src/pty.rs src/filesystem.rs src/stream_input.rs tests/zero_timeouts.rs docs/ changelog/
git commit -m "feat: a zero timeout means no limit, as the SDK's timeout=0"
```


---

### Task 5: Filesystem options and per-call request timeouts (CLM-0173)

**Files:**
- Modify: `src/config.rs` (`RequestTimeout`)
- Modify: `src/envd/mod.rs`:
  - `timeout_for` replaces `request_timeout()` at lines 267-275;
  - new parameters on `unary` (328-361), `within_request_timeout` (454-459) and `http_request` (494-496)
- Modify: `src/filesystem.rs`:
  - `FileOptions` and `ListOptions`;
  - `request_timeout` on `WriteOptions` (234-241) and `WatchOptions` (464-481);
  - the call signatures (873-1393);
  - the struct-literal unit test at 1543-1557
- Modify: `src/commands.rs`, `src/pty.rs` and `src/sandbox.rs`:
  - the `unary`, `within_request_timeout` and `http_request` callers: `src/commands.rs:373, 415, 497, 524, 591`; `src/pty.rs:360, 383`; `src/sandbox.rs:566`;
  - `mcp_token` at `src/sandbox.rs:351`
- Modify: `src/lib.rs` (re-exports and the crate-doc example at lines 54-55)
- Modify: the rustdoc examples at `src/client.rs:875`, `src/sandbox.rs:188`, `src/filesystem.rs:700` and `src/filesystem.rs:1354`
- Create: `tests/envd_file_options.rs`
- Modify (call sites): `tests/envd_api.rs`, `tests/envd_stream_files.rs`, `tests/envd_write.rs`, `tests/zero_timeouts.rs` (Task 4's `list` call), `tests/live_files.rs`, `tests/live_lifecycle.rs`
- Modify: `README.md` (lines 20, 34-35, 92, 101, 115-118, 169; the Files row)
- Modify: `docs/protocol-notes.md`, `docs/verification/s4-evidence-map.md` (the CLM-0172 row)
- Create: `changelog/<ts>-m4-task-05-filesystem-options.md`

> **Corrected (issue #93, 2026-10-28).** `src/stream_input.rs` needs no change: `InputStream` sends through `client_stream`, never `unary`, `within_request_timeout` or `http_request`, so "API after M4" no longer lists it among the `RequestTimeout::default()` callers. `tests/envd_watch.rs` has no call site to convert (`watch_dir` keeps its shape). `mcp_token`'s `read_to_string` is at `src/sandbox.rs:352`. The changelog slug is `filesystem-options`. The live check ran `live_files` with `--test-threads=1` and then `live_sweep` with the same `E2B_TEST_RUN_ID`, so the sweep removes only that run's leftovers; Step 10 now says so. In `tests/envd_file_options.rs`, `list_options_default_to_depth_one_and_reject_zero` builds its sandbox with `sandbox(&server)` (so `client_for` is not imported), and `is_transport_timeout` drops its pre-Task-2 comment. `Filesystem::write`'s rustdoc also lists `Error::InvalidArgument` for invalid metadata, since `write` now honours `WriteOptions::metadata`. No later task changes: Tasks 7, 9, 10, 12 and 14 already use these names and call shapes.

**Design:**
- One `RequestTimeout` value per options struct, resolved in exactly one place, `EnvdClient::timeout_for`.
- Unset means the configured request timeout. `request_timeout(None)`, or zero (via Task 4's `no_limit_if_zero`), means no limit. `request_timeout(Some(d))` means `d`.
- The filesystem calls pass their options' value. Commands, Pty and `is_running` pass `RequestTimeout::default()` until Task 7 gives them options.
- `server_stream` and `client_stream` are unchanged: their deadline is the stream timeout, not the request timeout.

- [ ] **Step 1: Write the failing tests**

Create `tests/envd_file_options.rs`:

```rust
//! Mock tests for the filesystem options (CLM-0173): `FileOptions`,
//! `ListOptions`, and the per-call request timeout on them, on
//! `WriteOptions` and on `WatchOptions`.

mod common;

use std::time::{Duration, Instant};

use common::{client_for, client_with_timeout, sandbox, sandbox_from_json_with_client, sandbox_json};
use e2b_client::{Error, FileOptions, ListOptions, WatchOptions, WriteOptions};
use serde_json::json;
use wiremock::matchers::{body_json, header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

const STAT: &str = "/filesystem.Filesystem/Stat";
const SHORT: Duration = Duration::from_millis(200);

/// A `Stat` answer for `/a` that arrives after `delay`.
fn slow_stat(delay: Duration) -> ResponseTemplate {
    ResponseTemplate::new(200)
        .set_body_json(json!({"entry": {"name": "a", "type": "FILE_TYPE_FILE", "path": "/a"}}))
        .set_delay(delay)
}

/// Whether `error` is a transport-level timeout (reqwest's own deadline).
fn is_transport_timeout(error: &Error) -> bool {
    // `TransportError::is_timeout` after Task 2, `reqwest::Error::is_timeout`
    // before it: the same method name, so this compiles either way.
    matches!(error, Error::Transport(transport) if transport.is_timeout())
}

#[tokio::test]
async fn a_per_call_request_timeout_overrides_the_configured_one() {
    // Arrange: the configured timeout is 5 s; Stat answers after 2 s.
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    Mock::given(method("POST"))
        .and(path(STAT))
        .respond_with(slow_stat(Duration::from_secs(2)))
        .mount(&server)
        .await;
    let options = FileOptions::new().request_timeout(Some(SHORT));

    // Act
    let started = Instant::now();
    let error = sandbox
        .files()
        .get_info("/a", &options)
        .await
        .expect_err("the 200 ms per-call timeout passes first");

    // Assert: the call gave up near 200 ms, and envd was told the deadline.
    assert!(is_transport_timeout(&error), "{error:?}");
    assert!(started.elapsed() < Duration::from_secs(1), "{:?}", started.elapsed());
    let requests = server.received_requests().await.expect("recording");
    let stat = requests.iter().find(|r| r.url.path() == STAT).expect("Stat request");
    assert_eq!(
        stat.headers.get("connect-timeout-ms").and_then(|v| v.to_str().ok()),
        Some("200")
    );
}

#[tokio::test]
async fn a_per_call_none_lifts_a_short_configured_timeout() {
    // Arrange: the configured timeout is 200 ms; Stat answers after 600 ms.
    let server = MockServer::start().await;
    let client = client_with_timeout(&server, Some(SHORT));
    let sandbox = sandbox_from_json_with_client(&server, &client, sandbox_json("0.5.7")).await;
    Mock::given(method("POST"))
        .and(path(STAT))
        .respond_with(slow_stat(Duration::from_millis(600)))
        .mount(&server)
        .await;
    let files = sandbox.files();

    // Act
    let default = files.get_info("/a", &FileOptions::default()).await;
    let unbounded = files
        .get_info("/a", &FileOptions::new().request_timeout(None))
        .await;

    // Assert: the configured 200 ms applies by default; `None` lifts it.
    assert!(is_transport_timeout(&default.expect_err("default times out")));
    assert_eq!(unbounded.expect("no limit").path, "/a");
}

#[tokio::test]
async fn a_zero_per_call_request_timeout_means_no_limit() {
    // Arrange: as above, with zero instead of `None` (CLM-0172).
    let server = MockServer::start().await;
    let client = client_with_timeout(&server, Some(SHORT));
    let sandbox = sandbox_from_json_with_client(&server, &client, sandbox_json("0.5.7")).await;
    Mock::given(method("POST"))
        .and(path(STAT))
        .respond_with(slow_stat(Duration::from_millis(600)))
        .mount(&server)
        .await;

    // Act
    let info = sandbox
        .files()
        .get_info("/a", &FileOptions::new().request_timeout(Some(Duration::ZERO)))
        .await;

    // Assert: no deadline at all, so no header either.
    assert_eq!(info.expect("zero is no limit").path, "/a");
    let requests = server.received_requests().await.expect("recording");
    let stat = requests.iter().find(|r| r.url.path() == STAT).expect("Stat request");
    assert!(!stat.headers.contains_key("connect-timeout-ms"));
}

#[tokio::test]
async fn read_and_write_use_the_per_call_request_timeout() {
    // Arrange: both `/files` routes answer after 2 s; the client allows 5 s.
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    Mock::given(method("GET"))
        .and(path("/files"))
        .respond_with(ResponseTemplate::new(200).set_body_string("x").set_delay(Duration::from_secs(2)))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/files"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!([{"name": "a", "type": "file", "path": "/a"}]))
                .set_delay(Duration::from_secs(2)),
        )
        .mount(&server)
        .await;
    let files = sandbox.files();

    // Act
    let read = files
        .read("/a", &FileOptions::new().request_timeout(Some(SHORT)))
        .await;
    let write = files
        .write("/a", "x", &WriteOptions::default().request_timeout(Some(SHORT)))
        .await;

    // Assert
    assert!(is_transport_timeout(&read.expect_err("read")));
    assert!(is_transport_timeout(&write.expect_err("write")));
}

#[tokio::test]
async fn watch_dir_opens_within_the_per_call_request_timeout() {
    // Arrange: WatchDir answers after 2 s; the client allows 5 s.
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    Mock::given(method("POST"))
        .and(path("/filesystem.Filesystem/WatchDir"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(2)))
        .mount(&server)
        .await;
    let options = WatchOptions::default().request_timeout(Some(SHORT));

    // Act
    let error = sandbox
        .files()
        .watch_dir("/w", &options)
        .await
        .expect_err("the watch does not open within 200 ms");

    // Assert: opening is bounded client-side, which reports `Error::Timeout`.
    assert!(matches!(error, Error::Timeout(limit) if limit == SHORT), "{error:?}");
}

#[tokio::test]
async fn file_options_select_the_user() {
    // Arrange: the RPC carries the user in Authorization, the file API in
    // the `username` query.
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    Mock::given(method("POST"))
        .and(path(STAT))
        // base64("root:")
        .and(header("authorization", "Basic cm9vdDo="))
        .respond_with(slow_stat(Duration::ZERO))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/files"))
        .and(query_param("path", "/a"))
        .and(query_param("username", "root"))
        .respond_with(ResponseTemplate::new(200).set_body_string("x"))
        .expect(1)
        .mount(&server)
        .await;
    let root = FileOptions::new().user("root");

    // Act
    let info = sandbox.files().get_info("/a", &root).await;
    let text = sandbox.files().read_to_string("/a", &root).await;

    // Assert: each matched only its user-carrying mock.
    assert_eq!(info.expect("stat as root").path, "/a");
    assert_eq!(text.expect("read as root"), "x");
}

#[tokio::test]
async fn list_options_default_to_depth_one_and_reject_zero() {
    // Arrange
    let server = MockServer::start().await;
    let client = client_for(&server);
    let sandbox = sandbox_from_json_with_client(&server, &client, sandbox_json("0.5.7")).await;
    Mock::given(method("POST"))
        .and(path("/filesystem.Filesystem/ListDir"))
        .and(body_json(json!({"path": "/w", "depth": 1})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"entries": []})))
        .expect(1)
        .mount(&server)
        .await;
    let files = sandbox.files();

    // Act
    let listed = files.list("/w", &ListOptions::default()).await;
    let zero = files.list("/w", &ListOptions::new().depth(0)).await;

    // Assert
    assert!(listed.expect("depth 1").is_empty());
    assert!(matches!(zero, Err(Error::InvalidArgument(_))), "{zero:?}");
}
```

Append to the `tests` module of `src/config.rs`:

```rust
    #[test]
    fn request_timeout_resolves_unset_none_zero_and_a_value() {
        let configured = Some(Duration::from_secs(60));
        let short = Duration::from_millis(200);
        assert_eq!(RequestTimeout::default().resolve(configured), configured);
        assert_eq!(RequestTimeout::set(None).resolve(configured), None);
        assert_eq!(RequestTimeout::set(Some(Duration::ZERO)).resolve(configured), None);
        assert_eq!(RequestTimeout::set(Some(short)).resolve(configured), Some(short));
        assert_eq!(RequestTimeout::default().resolve(None), None);
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --test envd_file_options`
Expected: FAIL to compile: "unresolved imports `e2b_client::FileOptions`, `e2b_client::ListOptions`" and "no method named `request_timeout` found for struct `WriteOptions`".

- [ ] **Step 3: `RequestTimeout` in `src/config.rs`**

After `no_limit_if_zero`:

```rust
/// A per-call request timeout, as an options struct stores it: unset uses
/// the configured [`ConnectionConfig::request_timeout`]; set to `None` (or
/// zero) means no limit; set to a duration means that duration. Only the
/// options structs' `request_timeout` setters make one.
///
/// Python note: this is the SDK's `request_timeout: Optional[float] = None`
/// argument, where `None` means "use the connection's". Rust needs the
/// extra `Option` level to tell "not given" from "given as no limit".
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct RequestTimeout(Option<Option<Duration>>);

impl RequestTimeout {
    /// A timeout the caller set explicitly; zero counts as no limit.
    ///
    /// # Arguments
    ///
    /// * `timeout` - the per-call timeout, or `None` for no limit.
    ///
    /// # Returns
    ///
    /// The set value.
    pub(crate) fn set(timeout: Option<Duration>) -> Self {
        Self(Some(no_limit_if_zero(timeout)))
    }

    /// The timeout to apply.
    ///
    /// # Arguments
    ///
    /// * `configured` - what applies when the caller set nothing.
    ///
    /// # Returns
    ///
    /// The caller's value when set, else `configured`.
    pub(crate) fn resolve(self, configured: Option<Duration>) -> Option<Duration> {
        // `unwrap_or` on the outer `Option`: "not set" falls back.
        self.0.unwrap_or(configured)
    }
}
```

- [ ] **Step 4: The envd plumbing (`src/envd/mod.rs`)**

Import `use crate::config::{ConnectionConfig, ENVD_PORT, RequestTimeout};`.

Replace `request_timeout()` (lines 267-275) with:

```rust
    /// The request timeout for one call: the caller's override, else the
    /// client's configured request timeout.
    ///
    /// # Arguments
    ///
    /// * `request_timeout` - the call's override, or `RequestTimeout::default()`
    ///   for the configured one.
    ///
    /// # Returns
    ///
    /// The timeout, or `None` when the call is unbounded.
    pub(crate) fn timeout_for(&self, request_timeout: RequestTimeout) -> Option<Duration> {
        request_timeout.resolve(self.request_timeout)
    }
```

`unary` (lines 328-361) gains a last parameter. Its rustdoc gains `/// * `request_timeout` - the per-call override of the request timeout.`, and the head of the body becomes:

```rust
    pub(crate) async fn unary<Req: Serialize, Res: DeserializeOwned>(
        &self,
        service: &str,
        method: &str,
        request: &Req,
        extra_headers: HeaderMap,
        request_timeout: RequestTimeout,
    ) -> Result<Res> {
        let timeout = self.timeout_for(request_timeout);
        let body = serde_json::to_vec(request)?;
        let mut builder = self
            .http
            .post(self.rpc_url(service, method))
            .headers(self.headers.clone())
            .headers(extra_headers)
            .headers(protocol_headers(CONTENT_TYPE_UNARY_JSON, timeout))
            .body(body);
        if let Some(timeout) = timeout {
            builder = builder.timeout(timeout);
        }
        // ... the rest unchanged.
```

`within_request_timeout` (lines 437-459):

```rust
    /// Runs `opening` (open a stream and read its first event) within the
    /// request timeout, as the SDKs' `first_event` does
    /// (`e2b/envd/client_async/__init__.py:113-149`). After that only the
    /// stream's own deadline applies.
    ///
    /// # Arguments
    ///
    /// * `request_timeout` - the per-call override of the request timeout.
    /// * `opening` - the future that opens the stream and reads its first
    ///   event.
    ///
    /// # Returns
    ///
    /// What `opening` returns.
    ///
    /// # Errors
    ///
    /// [`Error::Timeout`] when the request timeout passes first, else the
    /// error of `opening`.
    pub(crate) async fn within_request_timeout<T>(
        &self,
        request_timeout: RequestTimeout,
        opening: impl Future<Output = Result<T>>,
    ) -> Result<T> {
        within(self.timeout_for(request_timeout), opening).await
    }
```

`http_request` (lines 492-496):

```rust
    /// Starts a request to envd's plain HTTP API at `path`, with the sandbox
    /// headers and the call's request timeout applied.
    ///
    /// # Arguments
    ///
    /// * `method` - the HTTP method.
    /// * `path` - the route, e.g. `/files`.
    /// * `request_timeout` - the per-call override of the request timeout.
    ///
    /// # Returns
    ///
    /// The request builder.
    pub(crate) fn http_request(
        &self,
        method: Method,
        path: &str,
        request_timeout: RequestTimeout,
    ) -> RequestBuilder {
        self.http_request_with_timeout(method, path, self.timeout_for(request_timeout))
    }
```

Callers without options pass `RequestTimeout::default()`, imported as `use crate::config::RequestTimeout;`. Task 7 replaces these in `src/commands.rs` and `src/pty.rs`:
- `src/commands.rs:373`, `:415`, `:497`, `:524`: append `RequestTimeout::default()` as the last argument of each `.unary(...)`. For example, line 415 becomes `.unary(process_pb::SERVICE, "SendInput", &request, HeaderMap::new(), RequestTimeout::default())`.
- `src/commands.rs:591` (`open_process_stream`): `envd.within_request_timeout(RequestTimeout::default(), async { ... })`.
- `src/pty.rs:360` and `:383`: append `RequestTimeout::default()` to each `.unary(...)`.
- `src/sandbox.rs:566` (`is_running`): `.http_request(Method::GET, ENVD_HEALTH_ROUTE, RequestTimeout::default())`.

- [ ] **Step 5: `FileOptions`, `ListOptions` and the new fields (`src/filesystem.rs`)**

Import `use crate::config::RequestTimeout;`. Insert after `WriteEntry` (before `WriteOptions`, line 213):

```rust
/// Options for the filesystem calls that take only a user and a request
/// timeout: [`Filesystem::read`], [`read_to_string`](Filesystem::read_to_string),
/// [`read_stream`](Filesystem::read_stream), [`get_info`](Filesystem::get_info),
/// [`exists`](Filesystem::exists), [`make_dir`](Filesystem::make_dir),
/// [`rename`](Filesystem::rename) and [`remove`](Filesystem::remove).
///
/// Python note: these are the SDK's `user=` and `request_timeout=` keyword
/// arguments, gathered in one value so more can be added without breaking
/// callers.
///
/// # Examples
///
/// ```
/// use std::time::Duration;
/// use e2b_client::FileOptions;
///
/// let defaults = FileOptions::default();
/// let as_root = FileOptions::new()
///     .user("root")
///     .request_timeout(Some(Duration::from_secs(5)));
/// # let _ = (defaults, as_root);
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileOptions {
    user: Option<String>,
    request_timeout: RequestTimeout,
}

impl FileOptions {
    /// The defaults: the sandbox's default user and the configured request
    /// timeout.
    ///
    /// # Returns
    ///
    /// The options.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Acts as `user` instead of the sandbox's default user, which also
    /// decides how relative paths resolve.
    ///
    /// # Arguments
    ///
    /// * `user` - the user name, e.g. `"root"`.
    ///
    /// # Returns
    ///
    /// The options, for chaining.
    #[must_use]
    pub fn user(mut self, user: impl Into<String>) -> Self {
        self.user = Some(user.into());
        self
    }

    /// Bounds this call in place of the configured request timeout
    /// ([`crate::ConnectionConfigBuilder::request_timeout`]); `None`, or a
    /// zero duration, for no limit. For [`Filesystem::read_stream`], which
    /// has no timeout by default, it bounds the whole download.
    ///
    /// # Arguments
    ///
    /// * `timeout` - the per-call request timeout.
    ///
    /// # Returns
    ///
    /// The options, for chaining.
    #[must_use]
    pub fn request_timeout(mut self, timeout: Option<Duration>) -> Self {
        self.request_timeout = RequestTimeout::set(timeout);
        self
    }
}

/// Options for [`Filesystem::list`]: how deep to list, as which user, and a
/// per-call request timeout.
///
/// # Examples
///
/// ```
/// use e2b_client::ListOptions;
///
/// // Direct children and grandchildren, as root.
/// let options = ListOptions::new().depth(2).user("root");
/// # let _ = options;
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListOptions {
    depth: u32,
    file: FileOptions,
}

impl Default for ListOptions {
    fn default() -> Self {
        Self {
            // The SDK's `depth: Optional[int] = 1`
            // (`e2b/sandbox_async/filesystem/filesystem.py:494`).
            depth: 1,
            file: FileOptions::default(),
        }
    }
}

impl ListOptions {
    /// The defaults: depth 1 (direct children), the sandbox's default user
    /// and the configured request timeout.
    ///
    /// # Returns
    ///
    /// The options.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// How many levels to descend; 1 lists only direct children. 0 is
    /// refused by [`Filesystem::list`] before any request.
    ///
    /// # Arguments
    ///
    /// * `depth` - the depth, at least 1.
    ///
    /// # Returns
    ///
    /// The options, for chaining.
    #[must_use]
    pub fn depth(mut self, depth: u32) -> Self {
        self.depth = depth;
        self
    }

    /// Lists as `user` instead of the sandbox's default user.
    ///
    /// # Arguments
    ///
    /// * `user` - the user name, e.g. `"root"`.
    ///
    /// # Returns
    ///
    /// The options, for chaining.
    #[must_use]
    pub fn user(mut self, user: impl Into<String>) -> Self {
        self.file = self.file.user(user);
        self
    }

    /// Bounds this call in place of the configured request timeout; `None`,
    /// or a zero duration, for no limit.
    ///
    /// # Arguments
    ///
    /// * `timeout` - the per-call request timeout.
    ///
    /// # Returns
    ///
    /// The options, for chaining.
    #[must_use]
    pub fn request_timeout(mut self, timeout: Option<Duration>) -> Self {
        self.file = self.file.request_timeout(timeout);
        self
    }
}
```

`WriteOptions` (lines 234-241) gains a field `request_timeout: RequestTimeout,`, after `user`. Add this setter after `user`, and add to the struct rustdoc: "`request_timeout` bounds each in-memory upload request; [`Filesystem::write_stream`] has no timeout unless one is set here.":

```rust
    /// Bounds each upload request in place of the configured request
    /// timeout; `None`, or a zero duration, for no limit. For
    /// [`Filesystem::write_stream`], which has no timeout by default, it
    /// bounds the whole transfer.
    ///
    /// # Arguments
    ///
    /// * `timeout` - the per-call request timeout.
    ///
    /// # Returns
    ///
    /// The options, for chaining.
    #[must_use]
    pub fn request_timeout(mut self, timeout: Option<Duration>) -> Self {
        self.request_timeout = RequestTimeout::set(timeout);
        self
    }
```

`WatchOptions` (lines 464-481) gains a field `request_timeout: RequestTimeout,`; the `Default` impl sets `request_timeout: RequestTimeout::default()`. Add this setter after `timeout`:

```rust
    /// Bounds how long the watch may take to open (envd's start event) in
    /// place of the configured request timeout; `None`, or a zero duration,
    /// for no limit. The watch itself is bounded by
    /// [`timeout`](Self::timeout).
    ///
    /// # Arguments
    ///
    /// * `timeout` - the per-call request timeout.
    ///
    /// # Returns
    ///
    /// The options, for chaining.
    #[must_use]
    pub fn request_timeout(mut self, timeout: Option<Duration>) -> Self {
        self.request_timeout = RequestTimeout::set(timeout);
        self
    }
```

The `Filesystem` struct doc (lines 715-720) becomes: "Every method takes an options value with the sandbox user to act as (which also decides how relative paths resolve; unset uses the sandbox default) and a per-call request timeout ([`FileOptions`], [`ListOptions`], [`WriteOptions`], [`WatchOptions`])."

The unit test `write_options_default_to_multipart_and_chain` (lines 1543-1557) compares against a struct literal. Add the new field to it:

```rust
            WriteOptions {
                user: None,
                request_timeout: RequestTimeout::default(),
                octet_stream: false,
                gzip: false,
                metadata: BTreeMap::new(),
            }
```

- [ ] **Step 6: The new signatures (`src/filesystem.rs`)**

Replace each method's signature and the lines that use `user`. Task 2 has landed by now, so every reqwest result converts with `.map_err(Error::transport)`, as below; `?` on a reqwest error no longer compiles. The rustdoc `# Arguments` entry `* `user` - ...` becomes `* `options` - the user and the request timeout.` (`list`: "the depth, the user and the request timeout"; `write`: "the user, encoding, metadata and request timeout").

```rust
    pub async fn read(&self, path: &str, options: &FileOptions) -> Result<Bytes> {
        let response = self
            .envd
            .http_request(Method::GET, FILES_ROUTE, options.request_timeout)
            .query(&self.file_query(path, options.user.as_deref()))
            .send()
            .await
            .map_err(Error::transport)?;
        if !response.status().is_success() {
            return Err(envd_http_error(response).await);
        }
        response.bytes().await.map_err(Error::transport)
    }

    pub async fn read_to_string(&self, path: &str, options: &FileOptions) -> Result<String> {
        let bytes = self.read(path, options).await?;
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }

    pub async fn read_stream(&self, path: &str, options: &FileOptions) -> Result<FileStream> {
        // No timeout unless the caller set one: `resolve(None)`.
        let timeout = options.request_timeout.resolve(None);
        let response = self
            .envd
            .http_request_with_timeout(Method::GET, FILES_ROUTE, timeout)
            .query(&self.file_query(path, options.user.as_deref()))
            .send()
            .await
            .map_err(Error::transport)?;
        if !response.status().is_success() {
            return Err(envd_http_error(response).await);
        }
        Ok(FileStream { response })
    }

    pub async fn write(
        &self,
        path: &str,
        data: impl Into<Bytes>,
        options: &WriteOptions,
    ) -> Result<WriteInfo> {
        self.write_files(&[WriteEntry::new(path, data)], options)
            .await?
            .into_iter()
            .next()
            .ok_or_else(no_written_file)
    }
```

In the `read_stream` rustdoc, the paragraph starting "No timeout applies at all" becomes: "No timeout applies by default, including the wait for envd's response headers, because a deadline would cut off any download that outlasts it; this is the SDK's default. Set [`FileOptions::request_timeout`] to bound the whole download, or wrap this call and [`FileStream::next_chunk`] in `tokio::time::timeout`."

The `write` rustdoc's first paragraph ends "It is a one-entry [`Filesystem::write_files`] with `options`, so octet-stream, gzip and metadata apply to it too."

In `write_files` (line 1038), replace `let timeout = self.envd.request_timeout();` with:

```rust
        let timeout = self.envd.timeout_for(options.request_timeout);
```

Replace its rustdoc sentence "Every request is bounded by the client's request timeout." with "Every request is bounded by the request timeout ([`WriteOptions::request_timeout`], else the configured one)."

In `write_stream` (lines 1111-1139), replace both `None` timeout arguments with `timeout`, after adding at the top of the body:

```rust
        // No timeout unless the caller set one: `resolve(None)`.
        let timeout = options.request_timeout.resolve(None);
```

Its rustdoc's "No request timeout applies, because..." becomes "No request timeout applies unless [`WriteOptions::request_timeout`] sets one, because...".

The RPC calls:

```rust
    pub async fn list(&self, path: &str, options: &ListOptions) -> Result<Vec<EntryInfo>> {
        if options.depth == 0 {
            return Err(Error::InvalidArgument("depth should be at least 1".into()));
        }
        let response: ListDirResponse = self
            .envd
            .unary(
                filesystem_pb::SERVICE,
                "ListDir",
                &ListDirRequest {
                    path,
                    depth: options.depth,
                },
                self.envd.user_headers(options.file.user.as_deref())?,
                options.file.request_timeout,
            )
            .await?;
        Ok(response.entries.into_iter().map(EntryInfo::from).collect())
    }

    pub async fn get_info(&self, path: &str, options: &FileOptions) -> Result<EntryInfo> {
        let response: EntryResponse = self
            .envd
            .unary(
                filesystem_pb::SERVICE,
                "Stat",
                &PathRequest { path },
                self.envd.user_headers(options.user.as_deref())?,
                options.request_timeout,
            )
            .await?;
        entry_of(response)
    }

    pub async fn exists(&self, path: &str, options: &FileOptions) -> Result<bool> {
        match self.get_info(path, options).await {
            Ok(_) => Ok(true),
            Err(Error::Rpc {
                code: ConnectCode::NotFound,
                ..
            }) => Ok(false),
            Err(err) => Err(err),
        }
    }

    pub async fn make_dir(&self, path: &str, options: &FileOptions) -> Result<bool> {
        let result: Result<EntryResponse> = self
            .envd
            .unary(
                filesystem_pb::SERVICE,
                "MakeDir",
                &PathRequest { path },
                self.envd.user_headers(options.user.as_deref())?,
                options.request_timeout,
            )
            .await;
        match result {
            Ok(_) => Ok(true),
            Err(Error::Rpc {
                code: ConnectCode::AlreadyExists,
                ..
            }) => Ok(false),
            Err(err) => Err(err),
        }
    }

    pub async fn rename(
        &self,
        source: &str,
        destination: &str,
        options: &FileOptions,
    ) -> Result<EntryInfo> {
        let response: EntryResponse = self
            .envd
            .unary(
                filesystem_pb::SERVICE,
                "Move",
                &MoveRequest {
                    source,
                    destination,
                },
                self.envd.user_headers(options.user.as_deref())?,
                options.request_timeout,
            )
            .await?;
        entry_of(response)
    }

    pub async fn remove(&self, path: &str, options: &FileOptions) -> Result<()> {
        let _: Empty = self
            .envd
            .unary(
                filesystem_pb::SERVICE,
                "Remove",
                &PathRequest { path },
                self.envd.user_headers(options.user.as_deref())?,
                options.request_timeout,
            )
            .await?;
        Ok(())
    }
```

The `exists` arm stays `Error::Rpc { code: ConnectCode::NotFound, .. }`, which still compiles once Task 2 makes the variant `#[non_exhaustive]`.

`watch_dir` (line 1374): `.within_request_timeout(options.request_timeout, async { ... })`. Its rustdoc's "awaited within the request timeout" becomes "awaited within the request timeout ([`WatchOptions::request_timeout`], else the configured one)".

`src/lib.rs` re-exports:

```rust
pub use filesystem::{
    DEFAULT_WATCH_TIMEOUT, EntryInfo, FileOptions, FileStream, FileType, Filesystem,
    FilesystemEvent, FilesystemEventType, ListOptions, WatchHandle, WatchOptions, WriteEntry,
    WriteInfo, WriteOptions,
};
```

- [ ] **Step 7: Update every caller**

These rules convert every call in `src/`, `tests/`, the doctests and `README.md`. `cargo test` compiles them all, so a missed one fails the gates.

| Old call | New call |
|---|---|
| `read(p, None)`, `read_to_string(p, None)`, `read_stream(p, None)`, `get_info(p, None)`, `exists(p, None)`, `make_dir(p, None)`, `remove(p, None)` | the same with `&FileOptions::default()` |
| any of those with `Some("u")` | the same with `&FileOptions::new().user("u")` |
| `rename(a, b, None)` / `rename(a, b, Some("u"))` | `rename(a, b, &FileOptions::default())` / `rename(a, b, &FileOptions::new().user("u"))` |
| `write(p, data, None)` | `write(p, data, &WriteOptions::default())` |
| `write(p, data, Some("u"))` | `write(p, data, &WriteOptions::default().user("u"))` |
| `list(p, 1, None)` | `list(p, &ListOptions::default())` |
| `list(p, d, None)`, d > 1 or 0 | `list(p, &ListOptions::new().depth(d))` |

Sites (from `grep -nP '\.(read|read_to_string|read_stream|write|list|get_info|exists|make_dir|rename|remove)\(' src tests README.md` at 7ee3eab):
- **`src/`:**
  - `src/sandbox.rs:351` becomes `.read_to_string(MCP_TOKEN_PATH, &FileOptions::new().user(MCP_USER))`, with `use crate::filesystem::FileOptions;`;
  - doc examples at `src/sandbox.rs:188`, `src/client.rs:875`, `src/filesystem.rs:700` and `src/filesystem.rs:1354`, which become `&WriteOptions::default()` and `&FileOptions::default()` (each example adds the type to its `use e2b_client::{...}` line, or names it as `e2b_client::FileOptions`);
  - `src/lib.rs:54-55`, whose `use` line becomes `use e2b_client::{Client, CommandOptions, CreateSandboxParams, FileOptions, WriteOptions};`.
- **`tests/envd_api.rs`:**
  - lines 142, 663, 685, 725, 746, 765, 797-798, 813, 892-893, 917-918, 945, 948, 965, 1004-1006;
  - import `FileOptions, ListOptions, WriteOptions`;
  - line 813 is `list("/", &ListOptions::new().depth(0))` and line 797 is `list("/tmp", &ListOptions::new().depth(2))`;
  - line 725 is `write("/tmp/out.txt", "file contents", &WriteOptions::default().user("root"))`;
  - line 945 is `rename("/a", "/b", &FileOptions::new().user("root"))`;
  - the loop at lines 852-856 (its `user` is an `Option<&str>`) becomes:

  ```rust
    for (sandbox, user) in [(&current, Some("root")), (&current, None), (&legacy, None)] {
        let files = sandbox.files();
        // `map_or_else(default, f)`: the default when `None`, else `f(user)`.
        let read_options = user.map_or_else(FileOptions::new, |user| FileOptions::new().user(user));
        let write_options =
            user.map_or_else(WriteOptions::default, |user| WriteOptions::default().user(user));
        let _ = files.read("/a", &read_options).await;
        let _ = files.write("/a", "x", &write_options).await;
    }
  ```
- **`tests/envd_stream_files.rs`:** lines 214, 263 (`&FileOptions::new().user("root")`), 292, 300, 320.
- **`tests/envd_write.rs`:** lines 570-571.
- **`tests/zero_timeouts.rs`:** Task 4's `list("/w", 1, None)` becomes `list("/w", &ListOptions::default())`.
- **`tests/live_files.rs`:**
  - lines 24-554 as listed by the grep;
  - import `FileOptions, ListOptions`;
  - lines 110 and 136 are `&ListOptions::default()` and `&ListOptions::new().depth(2)`;
  - lines 289 and 296 are `&WriteOptions::default().user("root")` and `&WriteOptions::default().user("user")`.
- **`tests/live_lifecycle.rs`:** lines 245, 265, 611, 986, 996, 1072, 1093, 1185 and 1195; import `FileOptions, WriteOptions`.
- **`README.md`:**
  - the Quick start `use` (line 20) adds `FileOptions, WriteOptions`, and lines 34-35 use them;
  - the snapshot example's `use` (lines 76-79) adds `FileOptions, WriteOptions`, and lines 92 and 101 use them;
  - the tour's `use` (lines 115-118) adds `FileOptions`, and line 169 uses it.

In the "What it covers" Files row, after the method list, insert: "; every call takes an options value: `FileOptions::{user, request_timeout}`, `ListOptions::{depth, user, request_timeout}`, and `request_timeout` on `WriteOptions` and `WatchOptions`".

- [ ] **Step 8: Run the tests to verify they pass**

Run: `cargo test --test envd_file_options && cargo test --lib config::tests`
Expected: PASS (7 + the config unit tests).
Then: `just test`. Expected: PASS, including the README and rustdoc doctests.

- [ ] **Step 9: Docs**

`docs/protocol-notes.md`, "Filesystem" behaviour table: after the "Filesystem RPCs send the user via the `Authorization` header" row, add:

```
| Every filesystem call takes `user` and a per-call `request_timeout` (`FileOptions`, `ListOptions`, `WriteOptions`, `WatchOptions`); unset uses the connection's request timeout, `None` or zero means no limit; an RPC sends it as `Connect-Timeout-Ms` and enforces it client-side, the file API as a whole-request timeout; `list` defaults to depth 1 | `py: sandbox_async/filesystem/filesystem.py:116-121, 491-496, 530-534, 562-566, 590-594, 614-619, 647-650, 684-685` |
```

In `docs/verification/s4-evidence-map.md`, the CLM-0172 row from Task 4 gets `tests/envd_file_options.rs::a_zero_per_call_request_timeout_means_no_limit` appended to its Test cell. Its "Covers whole claim?" becomes `yes`, and its Notes lose the last sentence.

- [ ] **Step 10: Gates, live check, changelog, commit**

Run `just lint`, `just test` and `just doc`. This task rewrites every call in `tests/live_files.rs`, so run that binary once on hosted, then the sweep:

```bash
export E2B_TEST_TARGET=hosted E2B_TEST_RUN_ID=dev-$(date -u +%Y%m%dT%H%M%SZ)-$$
cargo test --test live_files -- --ignored --test-threads=1 2>&1 | tee target/live-logs/task-05-live-files.log
cargo test --test live_sweep -- --ignored
```

Report the pass count in the PR. `tests/live_lifecycle.rs` only changes argument shapes, so `cargo build --tests` covers it.

Write `changelog/<ts>-m4-task-05-filesystem-options.md`:

```markdown
# M4 Task 5: Filesystem options and per-call request timeouts

Plan: `docs/superpowers/plans/2026-10-28-milestone-4-api-stability.md`, Task 5.
Claim: CLM-0173 (S3); completes the per-call part of CLM-0172.

## What changed

- `FileOptions { user, request_timeout }` and `ListOptions { depth, user,
  request_timeout }`, re-exported from the crate root.
- `request_timeout` on `WriteOptions` (each in-memory upload; the whole
  transfer for `write_stream`) and `WatchOptions` (opening the watch).
- One `RequestTimeout` per options value, resolved by
  `EnvdClient::timeout_for`: unset is the configured request timeout, `None`
  or zero is no limit. `EnvdClient::{unary, within_request_timeout,
  http_request}` take it; Commands, Pty and `is_running` pass the default
  until Task 7.
- Tests: `tests/envd_file_options.rs` (7), a `src/config.rs` unit test, and
  every existing caller converted.

## Breaking changes

| Old | New |
|---|---|
| `files.read(p, None)` (also `read_to_string`, `read_stream`, `get_info`, `exists`, `make_dir`, `remove`) | `files.read(p, &FileOptions::default())` |
| `files.read(p, Some("root"))` | `files.read(p, &FileOptions::new().user("root"))` |
| `files.rename(a, b, user)` | `files.rename(a, b, &FileOptions...)` |
| `files.write(p, data, user)` | `files.write(p, data, &WriteOptions::default()[.user(..)])`; `write` now honours every `WriteOptions` field |
| `files.list(p, depth, user)` | `files.list(p, &ListOptions::new().depth(depth)[.user(..)])` |

## Live check (hosted E2B)

`cargo test --test live_files -- --ignored`: <n>/<n> passed (fill in).
```

```bash
git add src/ tests/ README.md docs/ changelog/
git commit -m "feat!: filesystem options structs with a per-call request timeout"
```


---

### Task 6: One connect and one pause method (connect_sandbox, pause_sandbox, Sandbox::pause)

**Files:**
- Modify: `src/client.rs` (`connect_sandbox` takes `&ConnectOptions`, `pause_sandbox` takes `&PauseOptions`; `connect_sandbox_with` and `pause_sandbox_with` are removed; rustdoc links at lines 210 and 435, after Task 3)
- Modify: `src/sandbox.rs` (`pause` takes `&PauseOptions`; `pause_with` is removed, lines 591-625, after Task 3)
- Modify: `src/lifecycle.rs` (rustdoc links at lines 294-295, 325 and 370)
- Modify: `tests/rest_api.rs` (lines 139, 158, 181, 200, 346-353, 452, 472, 476), `tests/rest_lifecycle.rs` (every `pause_sandbox`, `pause_sandbox_with`, `pause_with` and `connect_sandbox_with` call), `tests/rest_mcp.rs` (lines 277, 323), `tests/live_lifecycle.rs` (lines 225, 248, 258, 288, 376, 498, 515, 520, 533, 539, 938-948, 975, 980, 1319; 533 and 539 are Task 3's `clm_0171`; corrected in issue #81)
- Modify: `README.md` (quick start, lines 20 and 39-40; rows at lines 52 and 55)
- Create: `changelog/<ts>-m4-task-06-one-connect-one-pause.md`
- Modify: `docs/verification/s4-evidence-map.md` (row CLM-0159; corrected in issue #94)

> **Corrected (issue #94, 2026-10-28).** After Task 5 the call sites moved: `tests/rest_api.rs` `connect_sandbox` at 139, 158, 186, 211, 483, `pause_sandbox` at 357, 360, 364, 487, `sandbox.pause()` at 463; `tests/live_lifecycle.rs` `connect_sandbox` at 226, 259, 289, 985, 1333, `pause_sandbox`/`pause_sandbox_with` at 499, 521, 540, `.pause()` at 249, 377, 516, 534, `.pause_with(` at 943, 951, 980, `connect_sandbox_with` at 945, 953; `src/sandbox.rs` `pause`/`pause_with` at 593-627. `tests/live_support/mod.rs` has no call to change. The rename of `sandbox_pause_with_sends_the_mode` also touches the CLM-0159 row of `docs/verification/s4-evidence-map.md`: its Test cell names the new test, and its Notes say `{}` comes from `PauseOptions::new()` and `Sandbox::pause` posts to its own id. The README quick start's `use` keeps Task 5's `FileOptions` and `WriteOptions` beside `ConnectOptions` and `PauseOptions`, wrapped over four lines, and its lifecycle row is line 56. The changelog slug is `one-connect-one-pause`. The issue's check `grep -rn "connect_sandbox_with\|pause_sandbox_with\|pause_with" src tests README.md` still matches `OnTimeout::pause_with`, the `pause_with_a_*` lifecycle tests and the `connect_sandbox_without_*`/`pause_sandbox_without_*` test names, all different items; the exact check is `grep -rnw "connect_sandbox_with\|pause_sandbox_with" src tests README.md` plus `grep -rn "\.pause_with(\|Sandbox::pause_with" src tests README.md`, both empty.

Decision 3. The `_with` twins become the only methods. They take their options structs, and the v0.1 short forms go, so 0.2 has one method per operation. Tasks 1 and 3 have landed, so the bodies moved here are their final ones.

- [ ] **Step 1: Change the tests to the new signatures (they fail to compile)**

Rewrite every caller. These are the call-site changes; each test body is otherwise unchanged:

| Old call | New call |
|---|---|
| `client.connect_sandbox(id, None)` | `client.connect_sandbox(id, &ConnectOptions::new())` |
| `client.connect_sandbox(id, Some(Duration::from_secs(N)))` | `client.connect_sandbox(id, &ConnectOptions::new().timeout(Duration::from_secs(N)))` |
| `client.connect_sandbox_with(id, &options)` | `client.connect_sandbox(id, &options)` |
| `client.pause_sandbox(id)` | `client.pause_sandbox(id, &PauseOptions::new())` |
| `client.pause_sandbox_with(id, &options)` | `client.pause_sandbox(id, &options)` |
| `sandbox.pause()` | `sandbox.pause(&PauseOptions::new())` |
| `sandbox.pause_with(&options)` | `sandbox.pause(&options)` |

The sites (`grep -rn "connect_sandbox_with\|pause_sandbox_with\|\.pause_with(\|\.pause()\|connect_sandbox(\|pause_sandbox(" src tests README.md`, ignoring `OnTimeout::pause_with`, which stays):

- `tests/rest_api.rs:139, 158, 181, 200, 472` (`connect_sandbox`); `:346, 349, 353, 476` (`pause_sandbox`); `:452` (`sandbox.pause()`). Add `ConnectOptions, PauseOptions` to its `use e2b_client::{...}` (line 8).
- `tests/rest_lifecycle.rs`: every `pause_sandbox_with` (`pause_sends_memory_from_the_mode` and Task 3's two moded-pause tests), `pause_sandbox(SANDBOX_ID)` (line 419), `.pause_with(` (`sandbox_pause_with_sends_the_mode`, renamed to `sandbox_pause_sends_the_mode`), `connect_sandbox_with` (`connect_sends_memory_false_only_for_a_reboot`). In `pause_sends_memory_from_the_mode`, change the comment on line 386 from ``which `pause_sandbox_with` reports as not found`` to ``which `pause_sandbox` reports as not found``.
- `tests/rest_mcp.rs:277, 323` (`connect_sandbox(SANDBOX_ID, None)`). Add `ConnectOptions` to its imports.
- `tests/live_lifecycle.rs:225, 258, 288, 980, 1319` (`connect_sandbox`); `:498, 520, 539` (`pause_sandbox`/`pause_sandbox_with`, 539 in `clm_0171`); `:248, 376, 515, 533` (`.pause()`, 533 in `clm_0171`); `:938, 946, 975` (`.pause_with(`); `:940, 948` (`connect_sandbox_with`). (Line numbers after Task 3, corrected in issue #81.)

For example, `tests/live_lifecycle.rs:256-260` becomes:

```rust
    let resumed = sandbox
        .client()
        .connect_sandbox(
            sandbox.sandbox_id(),
            &ConnectOptions::new().timeout(Duration::from_secs(120)),
        )
        .await
        .expect("resume");
```

and `tests/live_lifecycle.rs:515` becomes:

```rust
    assert!(sandbox.pause(&PauseOptions::new()).await.expect("first pause"));
```

Run: `cargo test --no-run`
Expected: FAIL to compile with ``this method takes 1 argument but 2 arguments were supplied`` (`pause_sandbox`), ``this method takes 0 arguments but 1 argument was supplied`` (`Sandbox::pause`) and ``mismatched types: expected `Option<Duration>`, found `&ConnectOptions` `` (`connect_sandbox`).

- [ ] **Step 2: Fold the twins in `src/client.rs`**

Delete `connect_sandbox` (lines 204-240 after Task 1) and rename `connect_sandbox_with` to `connect_sandbox`, with this rustdoc:

```rust
    /// Connects to a sandbox, resuming it first if it is paused
    /// (`POST /v2/sandboxes/{id}/connect`, the SDK's
    /// `connect(timeout, on_resume)`, `e2b/sandbox_async/sandbox_api.py:498-520`).
    ///
    /// [`ConnectOptions::timeout`](crate::ConnectOptions::timeout) sets a new
    /// lifetime from now; without one the sandbox keeps its current lifetime.
    /// [`OnResume::Reboot`](crate::OnResume::Reboot) sends `memory: false`;
    /// a restore, the API's default, sends no `memory` field.
    ///
    /// # Arguments
    ///
    /// * `sandbox_id` - the sandbox to connect to.
    /// * `options` - the timeout and the resume mode;
    ///   [`ConnectOptions::new`](crate::ConnectOptions::new) for neither.
    ///
    /// # Returns
    ///
    /// A [`Sandbox`] handle.
    ///
    /// # Errors
    ///
    /// [`Error::SandboxNotFound`] for HTTP 404, [`Error::Api`] for other
    /// non-2xx answers (the API documents refusing a reboot this way where
    /// that capability is not enabled), [`Error::Transport`] for connection
    /// failures.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::time::Duration;
    /// use e2b_client::{Client, ConnectOptions, OnResume};
    ///
    /// # async fn demo(client: &Client) -> e2b_client::Result<()> {
    /// // Resume from the memory snapshot, with ten minutes to live.
    /// let options = ConnectOptions::new().timeout(Duration::from_secs(600));
    /// let sandbox = client.connect_sandbox("sbx-1", &options).await?;
    /// println!("connected to {}", sandbox.sandbox_id());
    /// // Or boot it cold from its filesystem.
    /// let reboot = ConnectOptions::new().on_resume(OnResume::Reboot);
    /// let rebooted = client.connect_sandbox("sbx-2", &reboot).await?;
    /// println!("rebooted {}", rebooted.sandbox_id());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn connect_sandbox(
        &self,
        sandbox_id: &str,
        options: &ConnectOptions,
    ) -> Result<Sandbox> {
```

Keep the body from Task 1, Step 3.

Delete `pause_sandbox` (the v0.1 form) and rename `pause_sandbox_with` to `pause_sandbox`, with this rustdoc:

```rust
    /// Pauses a sandbox (`POST /sandboxes/{id}/pause`, the SDK's
    /// `pause(mode)`, `e2b/sandbox_async/sandbox_api.py:399-432`). Resume it
    /// with [`Client::connect_sandbox`].
    ///
    /// The body is `{"memory": true}` for [`SnapshotMode::Full`],
    /// `{"memory": false}` for [`SnapshotMode::Filesystem`], and `{}` with
    /// [`PauseOptions::new`](crate::PauseOptions::new), the API's default.
    ///
    /// # Arguments
    ///
    /// * `sandbox_id` - the sandbox to pause.
    /// * `options` - what the pause keeps.
    ///
    /// # Returns
    ///
    /// `true` if it was paused, `false` if it was already paused. Without a
    /// mode any HTTP 409 means already paused, as in the SDK. With a mode,
    /// only a 409 whose message says the sandbox "is already paused" does.
    ///
    /// # Errors
    ///
    /// [`Error::SandboxNotFound`] for HTTP 404, [`Error::Transport`] for
    /// connection failures, and [`Error::Api`] for other non-2xx answers,
    /// including, with a mode, any 409 that is not "already paused": the
    /// API refusing that mode (for example
    /// `snapshot_filesystem_only_unsupported_node`). SDK 2.61 returns
    /// `false` for every 409 (`e2b/sandbox_async/sandbox_api.py:421-423`);
    /// the crate departs from it here so a refusal is not mistaken for a
    /// pause. A filesystem-only pause refused for the team is a 400.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use e2b_client::{Client, PauseOptions, SnapshotMode};
    ///
    /// # async fn demo(client: &Client) -> e2b_client::Result<()> {
    /// let options = PauseOptions::new().mode(SnapshotMode::Filesystem);
    /// if !client.pause_sandbox("sbx-1", &options).await? {
    ///     println!("it was already paused");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`SnapshotMode::Full`]: crate::SnapshotMode::Full
    /// [`SnapshotMode::Filesystem`]: crate::SnapshotMode::Filesystem
    pub async fn pause_sandbox(&self, sandbox_id: &str, options: &PauseOptions) -> Result<bool> {
```

Keep the body from Task 3, Step 3.

Search `src/client.rs` for `pause_sandbox_with` and `connect_sandbox_with`. The intra-doc links at the old lines 210 and 435 went with the deleted methods, and any left in other rustdoc become `pause_sandbox` / `connect_sandbox`. `just doc` (`-D warnings`) flags a broken link.

- [ ] **Step 3: Fold `Sandbox::pause`**

In `src/sandbox.rs`, replace `pause` and `pause_with` (lines 591-625) with:

```rust
    /// Pauses the sandbox, choosing what the pause keeps. See
    /// [`Client::pause_sandbox`].
    ///
    /// # Arguments
    ///
    /// * `options` - what the pause keeps; [`PauseOptions::new`] for the
    ///   API's default.
    ///
    /// # Returns
    ///
    /// `true` if it was paused, `false` if it was already paused (see
    /// [`Client::pause_sandbox`] for how a 409 is read when a mode is set).
    ///
    /// # Errors
    ///
    /// As [`Client::pause_sandbox`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use e2b_client::{PauseOptions, Sandbox};
    ///
    /// # async fn demo(sandbox: &Sandbox) -> e2b_client::Result<()> {
    /// sandbox.pause(&PauseOptions::new()).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn pause(&self, options: &PauseOptions) -> Result<bool> {
        self.client.pause_sandbox(&self.sandbox_id, options).await
    }
```

`PauseOptions` is already imported in `src/sandbox.rs` for `pause_with`.

- [ ] **Step 4: Fix the rustdoc links in `src/lifecycle.rs`**

- Lines 294-295: `/// Options for [`crate::Client::pause_sandbox_with`] and` / `/// [`crate::Sandbox::pause_with`]: what the pause keeps` become `/// Options for [`crate::Client::pause_sandbox`] and` / `/// [`crate::Sandbox::pause`]: what the pause keeps`.
- Line 325: `See [`crate::Client::pause_sandbox_with`] for` becomes `See [`crate::Client::pause_sandbox`] for`.
- Line 370: `/// Options for [`crate::Client::connect_sandbox_with`]: a new timeout and` becomes `/// Options for [`crate::Client::connect_sandbox`]: a new timeout and`.

- [ ] **Step 5: README**

Quick start (lines 19-45). Replace line 20 with:

```rust
use e2b_client::{Client, CommandOptions, ConnectOptions, CreateSandboxParams, PauseOptions};
```

and lines 38-40 with:

```rust
    // Pause, then resume by connecting again.
    sandbox.pause(&PauseOptions::new()).await?;
    let sandbox = client
        .connect_sandbox(sandbox.sandbox_id(), &ConnectOptions::new())
        .await?;
```

Row at line 55: replace `` `Sandbox::pause_with(PauseOptions)` and `Client::connect_sandbox_with(ConnectOptions)` choose `` with `` `Client::pause_sandbox(id, &PauseOptions)` / `Sandbox::pause(&PauseOptions)` and `Client::connect_sandbox(id, &ConnectOptions)` choose ``.

The row at line 52 lists method names only, so it stays.

- [ ] **Step 6: Run the tests to verify they pass**

Run: `cargo test`
Expected: PASS, the README doctests included. `grep -rn "connect_sandbox_with\|pause_sandbox_with\|\.pause_with(" src tests README.md docs/protocol-notes.md` finds nothing. Only `OnTimeout::pause_with` remains, and it is a different item.

- [ ] **Step 7: Live check**

Run on hosted, one at a time: `clm_0014_connect_resumes_paused_sandbox_with_files`, `clm_0024_pause_paused_sandbox_is_false`, `clm_0146_resume_restores_memory_or_reboots`, `clm_0146_filesystem_pause_cold_boots` and `clm_0171_moded_pause_of_a_paused_sandbox_is_false`. Expected: PASS. Report each in the PR.

- [ ] **Step 8: Gates, changelog, commit**

Run: `just lint && just test && just doc`
Expected: all green.

Create `changelog/<date -u +%Y-%m-%dT%H-%M-%SZ>-m4-task-06-one-connect-one-pause.md`:

```markdown
# M4 Task 6: one connect and one pause method (issue #<N>)

`Client::connect_sandbox`, `Client::pause_sandbox` and `Sandbox::pause` now
take their options structs, and the `_with` twins added in M3 are removed:
0.2 has one method per operation (decision 3).

## Breaking changes

| Old | New |
|---|---|
| `client.connect_sandbox(id, None)` | `client.connect_sandbox(id, &ConnectOptions::new())` |
| `client.connect_sandbox(id, Some(t))` | `client.connect_sandbox(id, &ConnectOptions::new().timeout(t))` |
| `client.connect_sandbox_with(id, &o)` | `client.connect_sandbox(id, &o)` |
| `client.pause_sandbox(id)` | `client.pause_sandbox(id, &PauseOptions::new())` |
| `client.pause_sandbox_with(id, &o)` | `client.pause_sandbox(id, &o)` |
| `sandbox.pause()` | `sandbox.pause(&PauseOptions::new())` |
| `sandbox.pause_with(&o)` | `sandbox.pause(&o)` |

## Tests

Call sites updated in `tests/rest_api.rs`, `tests/rest_lifecycle.rs`,
`tests/rest_mcp.rs`, `tests/live_lifecycle.rs` and the README; renamed
`sandbox_pause_with_sends_the_mode` to `sandbox_pause_sends_the_mode`.
Live on hosted: <results>.
```

```bash
git add src/client.rs src/sandbox.rs src/lifecycle.rs tests/ README.md changelog/
git commit -m "feat!: fold the _with twins into connect_sandbox and pause"
```

---

### Task 7: Connect options for commands and PTYs, and request timeouts (#54, CLM-0174)

**Files:**
- Modify: `src/commands.rs`. It gains `CommandConnectOptions` and `CommandOptions::request_timeout`, `Commands::connect` changes signature, and the crate-private `StreamTimeouts` is added for `open_process_stream`.
- Modify: `src/pty.rs`. It gains `PtyConnectOptions` and `PtyOptions::request_timeout`, and `Pty::connect` changes signature.
- Modify: `src/lib.rs`, for the re-exports.
- Create: `tests/envd_connect_options.rs`.
- Modify: `tests/envd_pty.rs`, `tests/envd_api.rs`, `tests/live_commands.rs` and `tests/zero_timeouts.rs`, which are callers.
- Modify: `docs/protocol-notes.md`, the "Commands" and "PTY" rows.
- Create: `changelog/<ts>-m4-task-07-connect-options.md`.

> **Corrected (issue #54, 2026-10-28).** The test file's `wiremock` import is `{Mock, MockServer, Request}` (`ResponseTemplate` is unused there and would fail `-D warnings`), and its `common` import is wrapped by rustfmt. `command_options_request_timeout_bounds_start` and `pty_options_request_timeout_bounds_create` assert the exact limit, `Error::Timeout(limit) if limit == Duration::from_millis(200)`, as the connect test does, so they show the call's own value bounded the wait rather than any timeout. The "not evidence for CLM-0174" note sits on `the_stream_deadline_is_not_the_request_timeout` as its doc comment. `tests/live_commands.rs` imports only `CommandConnectOptions`: no live test there calls `Pty::connect`, so `PtyConnectOptions` would be an unused import; Task 11 adds it. The commit also stages `tests/zero_timeouts.rs` and `tests/envd_connect_options.rs`. The rustdoc examples of `Commands::connect` and `Pty::connect` are rustfmt-wrapped but otherwise as given, so Task 14 skips both methods and both connect options types.

**What the SDK does.**
- `commands.connect(pid, timeout=60, request_timeout=None)` sends `timeout_to_ms(timeout)` as the stream deadline. It then waits for the start event within `get_request_timeout(request_timeout)` (`py: sandbox_async/commands/command.py:346-380, 382-411`; `py: envd/client_async/__init__.py:113-149`).
- `pty.create(..., timeout=60, request_timeout=None)` and `pty.connect(pid, timeout=60, request_timeout=None)` do the same (`py: sandbox_async/commands/pty.py:139-199, 207-248`).
- `commands.run`/`start` take `request_timeout` for the same wait (`py: sandbox_async/commands/command.py:218, 335-344`).

The crate already waits for the start event inside `open_process_stream` (`src/commands.rs`). After Task 5, that wait goes through `EnvdClient::within_request_timeout(&self, request_timeout: RequestTimeout, opening)`, and the Commands/Pty callers pass `RequestTimeout::default()`, meaning "use the configured timeout". `EnvdClient::timeout_for(RequestTimeout)` resolves it against the configured value. This task passes each options struct's own `RequestTimeout` instead. Nothing here resolves a timeout itself, and `server_stream` is unchanged.

- [ ] **Step 1: Write the failing mock tests**

Create `tests/envd_connect_options.rs`:

```rust
//! Mock tests for the options of `Commands::connect` and `Pty::connect`, and
//! for the per-call request timeout on every Commands/Pty call that takes
//! options (CLM-0174; zero as "no limit", CLM-0172).

mod common;

use std::time::Duration;

use common::{client_with_timeout, message, end_stream, sandbox, sandbox_from_json_with_client, sandbox_json, stream_response};
use e2b_client::{
    CommandConnectOptions, CommandOptions, Error, PtyConnectOptions, PtyOptions, PtySize, Sandbox,
};
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request};

/// A process stream: start (pid 42), one stdout chunk, then the end event.
fn frames() -> Vec<Vec<u8>> {
    vec![
        message(json!({"event": {"start": {"pid": 42}}})),
        message(json!({"event": {"data": {"stdout": "aGVsbG8K"}}})),
        message(json!({"event": {"end": {"exitCode": 0, "exited": true}}})),
        end_stream(json!({})),
    ]
}

/// Mounts `route` answering `frames()` after `delay`.
async fn mount_stream(server: &MockServer, route: &str, delay: Duration) {
    Mock::given(method("POST"))
        .and(path(route))
        .respond_with(stream_response(&frames()).set_delay(delay))
        .mount(server)
        .await;
}

/// The `Connect-Timeout-Ms` header of the one request sent to `route`.
async fn deadline_header(server: &MockServer, route: &str) -> Option<String> {
    let requests: Vec<Request> = server.received_requests().await.expect("requests");
    let request = requests
        .iter()
        .find(|request| request.url.path() == route)
        .expect("request to route");
    request
        .headers
        .get("connect-timeout-ms")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
}

/// A sandbox whose client has the given configured request timeout.
async fn sandbox_with_request_timeout(server: &MockServer, timeout: Duration) -> Sandbox {
    let client = client_with_timeout(server, Some(timeout));
    sandbox_from_json_with_client(server, &client, sandbox_json("0.5.7")).await
}

#[tokio::test]
async fn command_connect_defaults_to_the_sdk_sixty_seconds() {
    // Arrange
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    mount_stream(&server, "/process.Process/Connect", Duration::ZERO).await;

    // Act
    let handle = sandbox
        .commands()
        .connect(42, &CommandConnectOptions::default())
        .await
        .expect("connect");

    // Assert
    assert_eq!(handle.pid(), 42);
    assert_eq!(
        deadline_header(&server, "/process.Process/Connect").await.as_deref(),
        Some("60000")
    );
}

#[tokio::test]
async fn command_connect_sends_the_chosen_deadline() {
    // Arrange
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    mount_stream(&server, "/process.Process/Connect", Duration::ZERO).await;

    // Act
    let options = CommandConnectOptions::new().timeout(Some(Duration::from_secs(5)));
    sandbox.commands().connect(42, &options).await.expect("connect");

    // Assert
    assert_eq!(
        deadline_header(&server, "/process.Process/Connect").await.as_deref(),
        Some("5000")
    );
}

#[tokio::test]
async fn command_connect_without_a_deadline_sends_no_header() {
    // Arrange
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    mount_stream(&server, "/process.Process/Connect", Duration::ZERO).await;

    // Act
    let options = CommandConnectOptions::new().timeout(None);
    sandbox.commands().connect(42, &options).await.expect("connect");

    // Assert
    assert_eq!(deadline_header(&server, "/process.Process/Connect").await, None);
}

#[tokio::test]
async fn pty_connect_defaults_to_the_sdk_sixty_seconds() {
    // Arrange
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    mount_stream(&server, "/process.Process/Connect", Duration::ZERO).await;

    // Act
    sandbox
        .pty()
        .connect(42, &PtyConnectOptions::default())
        .await
        .expect("connect");

    // Assert
    assert_eq!(
        deadline_header(&server, "/process.Process/Connect").await.as_deref(),
        Some("60000")
    );
}

#[tokio::test]
async fn a_shorter_request_timeout_bounds_the_wait_for_the_start_event() {
    // Arrange: the client allows 5 s, the call only 200 ms, and envd takes 2 s.
    let server = MockServer::start().await;
    let sandbox = sandbox_with_request_timeout(&server, Duration::from_secs(5)).await;
    mount_stream(&server, "/process.Process/Connect", Duration::from_secs(2)).await;

    // Act
    let options = CommandConnectOptions::new().request_timeout(Some(Duration::from_millis(200)));
    let error = sandbox
        .commands()
        .connect(42, &options)
        .await
        .expect_err("the per-call request timeout must expire");

    // Assert
    assert!(matches!(error, Error::Timeout(limit) if limit == Duration::from_millis(200)), "{error:?}");
}

#[tokio::test]
async fn a_longer_request_timeout_overrides_the_configured_one() {
    // Arrange: the client allows 200 ms, the call 5 s, and envd takes 1 s.
    let server = MockServer::start().await;
    let sandbox = sandbox_with_request_timeout(&server, Duration::from_millis(200)).await;
    mount_stream(&server, "/process.Process/Connect", Duration::from_secs(1)).await;

    // Act
    let options = PtyConnectOptions::new().request_timeout(Some(Duration::from_secs(5)));
    let handle = sandbox.pty().connect(42, &options).await;

    // Assert
    assert_eq!(handle.expect("connect within the per-call timeout").pid(), 42);
}

#[tokio::test]
async fn a_zero_request_timeout_means_no_limit() {
    // Arrange: the client allows 200 ms; zero on the call lifts the bound.
    let server = MockServer::start().await;
    let sandbox = sandbox_with_request_timeout(&server, Duration::from_millis(200)).await;
    mount_stream(&server, "/process.Process/Connect", Duration::from_secs(1)).await;

    // Act
    let options = CommandConnectOptions::new().request_timeout(Some(Duration::ZERO));
    let handle = sandbox.commands().connect(42, &options).await;

    // Assert
    assert_eq!(handle.expect("connect with no request timeout").pid(), 42);
}

#[tokio::test]
async fn command_options_request_timeout_bounds_start() {
    // Arrange
    let server = MockServer::start().await;
    let sandbox = sandbox_with_request_timeout(&server, Duration::from_secs(5)).await;
    mount_stream(&server, "/process.Process/Start", Duration::from_secs(2)).await;

    // Act
    let options = CommandOptions::default().request_timeout(Some(Duration::from_millis(200)));
    let error = sandbox
        .commands()
        .start("true", &options)
        .await
        .expect_err("the per-call request timeout must expire");

    // Assert
    assert!(matches!(error, Error::Timeout(limit) if limit == Duration::from_millis(200)), "{error:?}");
}

#[tokio::test]
async fn pty_options_request_timeout_bounds_create() {
    // Arrange
    let server = MockServer::start().await;
    let sandbox = sandbox_with_request_timeout(&server, Duration::from_secs(5)).await;
    mount_stream(&server, "/process.Process/Start", Duration::from_secs(2)).await;

    // Act
    let options = PtyOptions::default().request_timeout(Some(Duration::from_millis(200)));
    let error = sandbox
        .pty()
        .create(PtySize::new(80, 24), &options)
        .await
        .expect_err("the per-call request timeout must expire");

    // Assert
    assert!(matches!(error, Error::Timeout(limit) if limit == Duration::from_millis(200)), "{error:?}");
}

#[tokio::test]
async fn the_stream_deadline_is_not_the_request_timeout() {
    // Arrange: the request timeout is 200 ms and the stream deadline 5 s. A
    // stream that opens at once and delivers after 1 s must still be read in
    // full, because after the start event only the stream deadline applies.
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    mount_stream(&server, "/process.Process/Connect", Duration::ZERO).await;

    // Act
    let options = CommandConnectOptions::new()
        .timeout(Some(Duration::from_secs(5)))
        .request_timeout(Some(Duration::from_millis(200)));
    let result = sandbox
        .commands()
        .connect(42, &options)
        .await
        .expect("connect")
        .wait()
        .await
        .expect("wait");

    // Assert
    assert_eq!(result.stdout, "hello\n");
    assert_eq!(
        deadline_header(&server, "/process.Process/Connect").await.as_deref(),
        Some("5000")
    );
}
```

The last test cannot delay part of a wiremock body, so it only checks that the two values travel separately: the header carries the stream deadline, and the 200 ms request timeout does not end the read. It is not evidence for CLM-0174: Task 18 cites the tests that prove the claim (the default and chosen deadlines, and the per-call request timeout bounding `connect`, `start` and `create`), not this one (brooke-reviews, PR #89). The existing `slow_stream_open_times_out_with_request_timeout` (`tests/envd_api.rs`) keeps covering the configured timeout.

Add unit tests for the defaults. `src/commands.rs` has no test module yet; add one at the end of the file:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connect_options_default_to_the_sdk_deadline() {
        assert_eq!(CommandConnectOptions::default().timeout, Some(DEFAULT_COMMAND_TIMEOUT));
        assert_eq!(CommandConnectOptions::new(), CommandConnectOptions::default());
        assert_eq!(
            CommandConnectOptions::default().request_timeout,
            RequestTimeout::default(),
            "unset: the configured request timeout applies"
        );
    }

    #[test]
    fn a_zero_connect_deadline_means_no_limit() {
        let options = CommandConnectOptions::new().timeout(Some(Duration::ZERO));

        assert_eq!(options.timeout, None);
    }
}
```

In the `tests` module of `src/pty.rs`, add:

```rust
    #[test]
    fn connect_options_default_to_the_sdk_deadline() {
        assert_eq!(PtyConnectOptions::default().timeout, Some(DEFAULT_PTY_TIMEOUT));
        assert_eq!(PtyConnectOptions::new(), PtyConnectOptions::default());
        assert_eq!(
            PtyConnectOptions::default().request_timeout,
            RequestTimeout::default(),
            "unset: the configured request timeout applies"
        );
    }
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --test envd_connect_options`
Expected: compile errors: there are no `CommandConnectOptions` or `PtyConnectOptions`, no `request_timeout` method on `CommandOptions`/`PtyOptions`, and `connect` expects an `Option<Duration>`.

- [ ] **Step 3: Implement the commands side**

In `src/commands.rs`, extend the imports:

```rust
use crate::config::{RequestTimeout, no_limit_if_zero};
```

Add the field and its setter to `CommandOptions`, and update its `Default`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOptions {
    envs: HashMap<String, String>,
    cwd: Option<String>,
    user: Option<String>,
    stdin: bool,
    timeout: Option<Duration>,
    request_timeout: RequestTimeout,
}

impl Default for CommandOptions {
    fn default() -> Self {
        Self {
            envs: HashMap::new(),
            cwd: None,
            user: None,
            stdin: false,
            timeout: Some(DEFAULT_COMMAND_TIMEOUT),
            request_timeout: RequestTimeout::default(),
        }
    }
}
```

In `impl CommandOptions`, after `timeout`:

```rust
    /// Bounds the wait until envd confirms the command started, for this
    /// call only; `None` or zero for no limit. When it is not set, the
    /// configured [`crate::ConnectionConfig::request_timeout`] applies. It
    /// matches the SDKs' `request_timeout` (`e2b/sandbox_async/commands/command.py:335-344`).
    /// After the start event only [`CommandOptions::timeout`] bounds the
    /// stream.
    ///
    /// # Arguments
    ///
    /// * `timeout` - the bound on opening the stream.
    ///
    /// # Returns
    ///
    /// The options, for chaining.
    #[must_use]
    pub fn request_timeout(mut self, timeout: Option<Duration>) -> Self {
        self.request_timeout = RequestTimeout::set(timeout);
        self
    }
```

Add the connect options after `CommandOptions`:

```rust
/// Options for [`Commands::connect`]: the deadline of the attached stream,
/// and the request timeout for the wait until envd confirms the attach. They
/// match the SDKs' `commands.connect(pid, timeout=60, request_timeout=None)`
/// (`e2b/sandbox_async/commands/command.py:346-350`).
///
/// # Examples
///
/// ```
/// use std::time::Duration;
/// use e2b_client::CommandConnectOptions;
///
/// // Follow a long-running process with no deadline, but give up if envd
/// // does not confirm the attach within 10 s.
/// let options = CommandConnectOptions::new()
///     .timeout(None)
///     .request_timeout(Some(Duration::from_secs(10)));
/// # let _ = options;
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandConnectOptions {
    timeout: Option<Duration>,
    request_timeout: RequestTimeout,
}

impl Default for CommandConnectOptions {
    fn default() -> Self {
        Self {
            timeout: Some(DEFAULT_COMMAND_TIMEOUT),
            request_timeout: RequestTimeout::default(),
        }
    }
}

impl CommandConnectOptions {
    /// The SDKs' defaults: a 60 s stream deadline
    /// ([`DEFAULT_COMMAND_TIMEOUT`]) and the configured request timeout.
    ///
    /// # Returns
    ///
    /// The default options.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Bounds how long the attached stream may stay open; `None` or zero for
    /// no limit (the SDKs' `timeout=0`). Defaults to
    /// [`DEFAULT_COMMAND_TIMEOUT`]. The deadline counts from the attach.
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
        self.timeout = no_limit_if_zero(timeout);
        self
    }

    /// Bounds the wait until envd confirms the attach, for this call only;
    /// `None` or zero for no limit. When it is not set, the configured
    /// [`crate::ConnectionConfig::request_timeout`] applies.
    ///
    /// # Arguments
    ///
    /// * `timeout` - the bound on opening the stream.
    ///
    /// # Returns
    ///
    /// The options, for chaining.
    #[must_use]
    pub fn request_timeout(mut self, timeout: Option<Duration>) -> Self {
        self.request_timeout = RequestTimeout::set(timeout);
        self
    }
}
```

Add the crate-private timeouts pair before `open_process_stream`:

```rust
/// The two bounds on a process stream: its own deadline, and the request
/// timeout for the wait until envd confirms it with a start event. They are
/// kept together so `open_process_stream` stays within five parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StreamTimeouts {
    /// The stream deadline (`Connect-Timeout-Ms`), or `None` for no limit.
    pub(crate) stream: Option<Duration>,
    /// The bound on opening the stream: the call's override, or
    /// `RequestTimeout::default()` for the configured one. `EnvdClient`
    /// resolves it (`timeout_for`).
    pub(crate) request: RequestTimeout,
}
```

Change `open_process_stream` to take it (the doc comment's `timeout` bullet becomes `timeouts`):

```rust
/// * `timeouts` - the stream deadline and the bound on opening the stream.
pub(crate) async fn open_process_stream<Req: serde::Serialize>(
    envd: &EnvdClient,
    method: &'static str,
    request: &Req,
    headers: HeaderMap,
    timeouts: StreamTimeouts,
) -> Result<(u32, ConnectStream<EventResponse>)> {
    // The async block's error type is inferred from `within_request_timeout`,
    // whose argument must yield the crate's `Result`.
    envd.within_request_timeout(timeouts.request, async {
        let mut stream: ConnectStream<EventResponse> = envd
            .server_stream(process_pb::SERVICE, method, request, headers, timeouts.stream)
            .await?;
        let pid = stream
            .next()
            .await?
            .and_then(|response| response.event)
            .and_then(|event| event.start)
            .map(|start| start.pid)
            .ok_or_else(|| {
                Error::Protocol(format!("{method} stream did not open with a start event"))
            })?;
        Ok((pid, stream))
    })
    .await
}
```

In `impl Commands`, `start` ends with:

```rust
        let timeouts = StreamTimeouts {
            stream: options.timeout,
            request: options.request_timeout,
        };
        self.open("Start", &request, headers, timeouts).await
```

`connect` becomes:

```rust
    /// Attaches to a process that is already running.
    ///
    /// # Arguments
    ///
    /// * `pid` - the process ID.
    /// * `options` - the stream deadline (60 s by default, as in the SDKs)
    ///   and the request timeout for the attach.
    ///
    /// # Returns
    ///
    /// A [`CommandHandle`] for the process's remaining output.
    ///
    /// # Errors
    ///
    /// [`Error::Rpc`] with [`ConnectCode::NotFound`] if there is no such
    /// process, plus the errors of [`Commands::start`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use e2b_client::{CommandConnectOptions, CommandOptions, Sandbox};
    ///
    /// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
    /// let started = sandbox.commands().start("sleep 5; echo done", &CommandOptions::default()).await?;
    /// let pid = started.pid();
    /// drop(started); // the process keeps running
    /// let attached = sandbox.commands().connect(pid, &CommandConnectOptions::default()).await?;
    /// assert_eq!(attached.wait().await?.stdout, "done\n");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn connect(&self, pid: u32, options: &CommandConnectOptions) -> Result<CommandHandle> {
        let request = ConnectRequest {
            process: ProcessSelector::Pid(pid),
        };
        let timeouts = StreamTimeouts {
            stream: options.timeout,
            request: options.request_timeout,
        };
        self.open("Connect", &request, keepalive_headers(), timeouts)
            .await
    }
```

`Commands::open` takes `timeouts: StreamTimeouts` in place of `timeout: Option<Duration>` and passes it through unchanged.

- [ ] **Step 4: Implement the PTY side**

In `src/pty.rs`, the imports become:

```rust
use crate::commands::{CommandExit, Commands, StreamTimeouts, open_process_stream};
use crate::config::{RequestTimeout, no_limit_if_zero};
```

`PtyOptions` gains `request_timeout: RequestTimeout` (in the struct and in `Default`, as `RequestTimeout::default()`), and this setter after `timeout`:

```rust
    /// Bounds the wait until envd confirms the shell started, for this call
    /// only; `None` or zero for no limit. When it is not set, the configured
    /// [`crate::ConnectionConfig::request_timeout`] applies. It matches the
    /// SDKs' `request_timeout` (`e2b/sandbox_async/commands/pty.py:139-147`).
    ///
    /// # Arguments
    ///
    /// * `timeout` - the bound on opening the stream.
    ///
    /// # Returns
    ///
    /// The options, for chaining.
    #[must_use]
    pub fn request_timeout(mut self, timeout: Option<Duration>) -> Self {
        self.request_timeout = RequestTimeout::set(timeout);
        self
    }
```

Add the connect options after `PtyOptions`:

```rust
/// Options for [`Pty::connect`]: the deadline of the reattached session, and
/// the request timeout for the wait until envd confirms the attach. They match
/// the SDKs' `pty.connect(pid, timeout=60, request_timeout=None)`
/// (`e2b/sandbox_async/commands/pty.py:207-212`).
///
/// # Examples
///
/// ```
/// use std::time::Duration;
/// use e2b_client::PtyConnectOptions;
///
/// let options = PtyConnectOptions::new()
///     .timeout(Some(Duration::from_secs(600)))
///     .request_timeout(Some(Duration::from_secs(10)));
/// # let _ = options;
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PtyConnectOptions {
    timeout: Option<Duration>,
    request_timeout: RequestTimeout,
}

impl Default for PtyConnectOptions {
    fn default() -> Self {
        Self {
            timeout: Some(DEFAULT_PTY_TIMEOUT),
            request_timeout: RequestTimeout::default(),
        }
    }
}

impl PtyConnectOptions {
    /// The default options: a stream deadline of [`DEFAULT_PTY_TIMEOUT`] and
    /// the configured request timeout.
    ///
    /// # Returns
    ///
    /// The default options.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Bounds the reattached session; `None` or zero for no limit. The
    /// deadline counts from the attach.
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
        self.timeout = no_limit_if_zero(timeout);
        self
    }

    /// Bounds the wait until envd confirms the attach, for this call only;
    /// `None` or zero for no limit. When it is not set, the configured
    /// [`crate::ConnectionConfig::request_timeout`] applies.
    ///
    /// # Arguments
    ///
    /// * `timeout` - the bound on opening the stream.
    ///
    /// # Returns
    ///
    /// The options, for chaining.
    #[must_use]
    pub fn request_timeout(mut self, timeout: Option<Duration>) -> Self {
        self.request_timeout = RequestTimeout::set(timeout);
        self
    }
}
```

`Pty::create` ends with:

```rust
        let timeouts = StreamTimeouts {
            stream: options.timeout,
            request: options.request_timeout,
        };
        self.open("Start", &request, headers, timeouts).await
```

`Pty::connect` becomes:

```rust
    /// Attaches to a PTY session that is already running.
    ///
    /// # Arguments
    ///
    /// * `pid` - the shell's process ID.
    /// * `options` - the stream deadline and the request timeout for the
    ///   attach.
    ///
    /// # Returns
    ///
    /// A [`PtyHandle`] for the session's remaining output.
    ///
    /// # Errors
    ///
    /// [`Error::Rpc`] with [`crate::ConnectCode::NotFound`] if there is no
    /// such process, plus the errors of [`Pty::create`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use e2b_client::{PtyConnectOptions, PtyOptions, PtySize, Sandbox};
    ///
    /// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
    /// let first = sandbox.pty().create(PtySize::new(80, 24), &PtyOptions::default()).await?;
    /// let pid = first.pid();
    /// drop(first); // the shell keeps running
    /// let mut again = sandbox.pty().connect(pid, &PtyConnectOptions::default()).await?;
    /// again.send_stdin("exit\n").await?;
    /// let exit = again.wait().await?;
    /// assert_eq!(exit.exit_code, 0);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn connect(&self, pid: u32, options: &PtyConnectOptions) -> Result<PtyHandle> {
        let request = ConnectRequest {
            process: ProcessSelector::Pid(pid),
        };
        let timeouts = StreamTimeouts {
            stream: options.timeout,
            request: options.request_timeout,
        };
        self.open("Connect", &request, keepalive_headers(), timeouts)
            .await
    }
```

`Pty::open` takes `timeouts: StreamTimeouts`, as `Commands::open` does. In the `PtyOptions::timeout` rustdoc, the sentence "it cannot be reattached with [`Pty::connect`]" stays; Task 8 rewrites the paragraph.

`src/lib.rs`:

```rust
pub use commands::{
    CommandConnectOptions, CommandEvent, CommandExit, CommandHandle, CommandOptions,
    CommandResult, Commands, DEFAULT_COMMAND_TIMEOUT, ProcessInfo, Signal,
};
```

```rust
pub use pty::{
    DEFAULT_PTY_TIMEOUT, Pty, PtyConnectOptions, PtyEvent, PtyHandle, PtyOptions, PtySize,
};
```

- [ ] **Step 5: Update the callers**

The old calls passed `None` (no limit) or `Some(d)`. The new default is 60 s, so a caller that passed `None` must say `.timeout(None)` to keep its behaviour:

- `tests/envd_pty.rs:152`: `sandbox.pty().connect(11, &PtyConnectOptions::new().timeout(None))`, and add `PtyConnectOptions` to the `use e2b_client::{...}` list. The test's "no deadline header when the timeout is None" assertion stays.
- `tests/envd_api.rs:495`: `.connect(42, &CommandConnectOptions::new().timeout(Some(Duration::from_secs(5))))`.
- `tests/envd_api.rs:1002`: `let _ = commands.connect(1, &CommandConnectOptions::default()).await;`. Add `CommandConnectOptions` to that file's import list.
- `tests/live_commands.rs:207` (`clm_0040`): `.connect(pid, &CommandConnectOptions::new().timeout(None))`, and add `CommandConnectOptions` to the imports.
- `tests/zero_timeouts.rs` (Task 4's `a_zero_command_connect_timeout_sends_no_deadline` and `a_zero_pty_connect_timeout_sends_no_deadline`): `.connect(7, &CommandConnectOptions::new().timeout(Some(Duration::ZERO)))` and `.connect(7, &PtyConnectOptions::new().timeout(Some(Duration::ZERO)))`, and add both types to the imports. `Commands::connect` and `Pty::connect` no longer call `no_limit_if_zero` themselves: the new setters do (corrected in issue #52, 2026-10-28).

Then `grep -rn "\.connect(" src tests README.md | grep -v connect_sandbox` must show no `Option<Duration>` argument left.

- [ ] **Step 6: Run the tests**

Run: `cargo test --test envd_connect_options --test envd_pty --test envd_api`, then `cargo test --lib -- connect_options a_zero_connect_deadline`
Expected: PASS.

Run the touched live test on hosted:
`E2B_TEST_TARGET=hosted E2B_TEST_RUN_ID=dev-$(date -u +%Y%m%dT%H%M%SZ)-$$ cargo test --test live_commands -- --ignored --exact clm_0040_connect_attaches_to_running_command`
Expected: PASS. Report it in the PR.

- [ ] **Step 7: Docs, gates, changelog, commit**

`docs/protocol-notes.md`, "Commands" section:
- Replace the row "Opening the stream waits for the first event within the request timeout; it must be `start` and carries the pid" with "Opening the stream waits for the first event within the call's request timeout (`CommandOptions::request_timeout` / `CommandConnectOptions::request_timeout`, else the configured one; `None` or zero for no limit); it must be `start` and carries the pid | `py: sandbox_async/commands/command.py:335-344, 382-411`; `py: envd/client_async/__init__.py:113-149`".
- Add the row "`connect(pid, timeout=60, request_timeout)`: the attach defaults to a 60 s stream deadline (`CommandConnectOptions::default()`) | `py: sandbox_async/commands/command.py:346-380`".

"PTY" section: the `create`/`connect` row's parenthesis becomes "(here `PtyOptions` and `PtyConnectOptions` default to it)". The "Both wait for the opening `start` event within the request timeout" row gains ", the call's own when `PtyOptions`/`PtyConnectOptions::request_timeout` is set".

Gates: `just lint`, `just test`, `just doc`.

`changelog/<ts>-m4-task-07-connect-options.md` lists:
- the new types and setters;
- the per-call request timeout;
- the tests;
- "Breaking changes":
  - `Commands::connect(pid, Some(d))` becomes `Commands::connect(pid, &CommandConnectOptions::new().timeout(Some(d)))`;
  - `Commands::connect(pid, None)` becomes `Commands::connect(pid, &CommandConnectOptions::new().timeout(None))`, because `&CommandConnectOptions::default()` is now 60 s, the SDK's value;
  - `Pty::connect(pid, t)` becomes `Pty::connect(pid, &PtyConnectOptions::new().timeout(t))`.
- It closes #54.

```bash
git add src/commands.rs src/pty.rs src/lib.rs tests/envd_connect_options.rs tests/envd_pty.rs tests/envd_api.rs tests/live_commands.rs tests/zero_timeouts.rs docs/protocol-notes.md docs/superpowers/plans/ changelog/
git commit -m "feat!: options structs for Commands::connect and Pty::connect, per-call request timeouts (#54)"
```


---

### Task 8: No PTY deadline by default (#55, CLM-0175)

**Files:**
- Modify: `src/pty.rs`, for the defaults of `PtyOptions` and `PtyConnectOptions`, their rustdoc, the `DEFAULT_PTY_TIMEOUT` doc and the unit tests.
- Modify: `tests/envd_pty.rs` and `tests/envd_connect_options.rs`, the default-header assertions.
- Modify: `tests/live_commands.rs`, which gains `clm_0175_pty_outlives_sixty_seconds`.
- Modify: `README.md`, the PTY example.
- Modify: `docs/protocol-notes.md`, the PTY rows.
- Create: `changelog/<ts>-m4-task-08-no-pty-deadline.md`.

> **Corrected (issue #55, 2026-10-28).** With no deadline, every field of `PtyOptions` and `PtyConnectOptions` takes its own type's default, so both derive `Default` (`#[derive(Debug, Clone, Default, PartialEq, Eq)]`) instead of the hand-written `impl Default` blocks Step 3 gave; Step 3 now says so. The changelog slug is `no-pty-deadline`. `docs/protocol-notes.md` also gains a "No PTY deadline by default" bullet under "Deliberate differences from the SDKs". The `PtyConnectOptions` type-level rustdoc says its defaults match the SDK's except the deadline. The live check ran `clm_0175`, `clm_0047`, `clm_0048` and `clm_0049` on hosted E2B, one at a time; all passed. No later task changes: Tasks 11, 12 and 14 already use `PtyOptions::default()`/`new()` and `PtyConnectOptions::default()`/`new()`, which `Self::default()` still serves.

**Why.** The deadline counts from open, not from activity. When it passes, envd ends the shell, and the session can't be reattached (observed on hosted E2B on 2026-10-27; `docs/protocol-notes.md`, the PTY section). A 60 s default therefore kills interactive sessions. pgrace decided on 2026-10-28 that a PTY has no deadline unless the caller sets one (decision 6). `DEFAULT_PTY_TIMEOUT` stays exported as the SDK's value.

- [ ] **Step 1: Flip the default assertions (the failing tests)**

In `src/pty.rs`, the `tests` module: replace `options_default_to_the_sdk_deadline` and Task 7's `connect_options_default_to_the_sdk_deadline` with:

```rust
    #[test]
    fn options_default_to_no_deadline() {
        assert_eq!(PtyOptions::default().timeout, None);
        assert_eq!(PtyConnectOptions::default().timeout, None);
        assert_eq!(PtyConnectOptions::new(), PtyConnectOptions::default());
        assert_eq!(
            PtyConnectOptions::default().request_timeout,
            RequestTimeout::default(),
            "unset: the configured request timeout applies"
        );
        // The SDKs' value stays available for callers who want parity.
        assert_eq!(DEFAULT_PTY_TIMEOUT, Duration::from_secs(60));
    }
```

In `tests/envd_pty.rs`, `create_sends_bash_with_pty_size_and_terminal_env_defaults`: replace the `connect-timeout-ms` assertion with:

```rust
    assert!(
        !start.headers.contains_key("connect-timeout-ms"),
        "a PTY has no stream deadline by default (CLM-0175)"
    );
```

In `tests/envd_connect_options.rs`, replace `pty_connect_defaults_to_the_sdk_sixty_seconds` with:

```rust
#[tokio::test]
async fn pty_connect_has_no_deadline_by_default() {
    // Arrange
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    mount_stream(&server, "/process.Process/Connect", Duration::ZERO).await;

    // Act
    sandbox
        .pty()
        .connect(42, &PtyConnectOptions::default())
        .await
        .expect("connect");

    // Assert
    assert_eq!(deadline_header(&server, "/process.Process/Connect").await, None);
}

#[tokio::test]
async fn pty_create_sends_a_deadline_only_when_set() {
    // Arrange
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    mount_stream(&server, "/process.Process/Start", Duration::ZERO).await;

    // Act
    let options = PtyOptions::default().timeout(Some(e2b_client::DEFAULT_PTY_TIMEOUT));
    sandbox
        .pty()
        .create(PtySize::new(80, 24), &options)
        .await
        .expect("create");

    // Assert
    assert_eq!(
        deadline_header(&server, "/process.Process/Start").await.as_deref(),
        Some("60000")
    );
}
```

`tests/envd_pty.rs:152` (Task 7 made it `PtyConnectOptions::new().timeout(None)`) can now use `&PtyConnectOptions::default()`; its "no deadline header" assertion holds either way. Change it, so the default path is the one under test.

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --lib -- options_default_to_no_deadline`, then `cargo test --test envd_pty --test envd_connect_options`
Expected: FAIL. The defaults are still `Some(60 s)`, so the header is `60000`.

- [ ] **Step 3: Change the defaults and their docs**

`src/pty.rs`:

```rust
/// The SDKs' bound on a PTY stream (`timeout=60`,
/// `e2b/sandbox_async/commands/pty.py:146, 211`). This crate does not apply
/// it by default (a deadline ends the shell, see [`PtyOptions::timeout`]);
/// pass `Some(DEFAULT_PTY_TIMEOUT)` to [`PtyOptions::timeout`] or
/// [`PtyConnectOptions::timeout`] for the SDKs' behaviour.
pub const DEFAULT_PTY_TIMEOUT: Duration = Duration::from_secs(60);
```

Remove `impl Default for PtyOptions` and derive it instead, since every field now takes its own type's default:

```rust
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PtyOptions {
```

The `PtyOptions::timeout` rustdoc becomes:

```rust
    /// Bounds the whole session; `None` or zero for no limit, which is the
    /// default. This differs from the SDKs, which default to 60 s
    /// ([`DEFAULT_PTY_TIMEOUT`]).
    ///
    /// A deadline, if you set one, counts from when the session opens, not
    /// from the last activity. When it passes, the output stream ends with
    /// the deadline error from [`PtyHandle::next_event`], even while the
    /// shell is in use. envd then ends the shell too, so it cannot be
    /// reattached with [`Pty::connect`] (observed on hosted E2B,
    /// 2026-10-27). Without a deadline, the session lives until the shell
    /// exits, you kill it, or the sandbox ends.
    ///
    /// # Arguments
    ///
    /// * `timeout` - the stream deadline.
    ///
    /// # Returns
    ///
    /// The options, for chaining.
```

`PtyOptions`' type-level example keeps `.timeout(None)`, which now restates the default; change it to `.timeout(Some(Duration::from_secs(3600)))` so the example shows a real choice.

`PtyConnectOptions` derives `Default` the same way (remove its `impl Default`):

```rust
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PtyConnectOptions {
```

Its `new()` doc becomes "The default options: no stream deadline (the SDKs use 60 s, [`DEFAULT_PTY_TIMEOUT`]) and the configured request timeout." Its `timeout` doc gains "No deadline by default; see [`PtyOptions::timeout`] for what a deadline does to the shell."

The `Pty::create` rustdoc's `options` bullet becomes "environment, working directory, user, the session deadline (none by default) and the request timeout."

- [ ] **Step 4: Run the tests**

Run: `cargo test --lib -- options_default_to_no_deadline`, then `cargo test --test envd_pty --test envd_connect_options`
Expected: PASS.

- [ ] **Step 5: The live test**

In `tests/live_commands.rs`, import `live_support::{LiveSandbox, params}`. Add after `clm_0049`:

```rust
#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0175_pty_outlives_sixty_seconds() {
    // Arrange: the sandbox must outlive the 65 s wait (the default test
    // sandbox lives 60 s).
    let sandbox = LiveSandbox::create(&params().timeout(Duration::from_secs(150))).await;
    let handle = sandbox
        .pty()
        .create(PtySize::new(80, 24), &PtyOptions::default())
        .await
        .expect("create pty");

    // Act: wait past the SDKs' 60 s default, then use the shell. The
    // terminal echoes the input, so the marker is one only the shell can
    // produce: the echo reads `alive-$((40+2))`, the output `alive-42`.
    tokio::time::sleep(Duration::from_secs(65)).await;
    handle
        .send_stdin("echo alive-$((40+2))\nexit\n")
        .await
        .expect("send_stdin after 65 s: the shell must still be running");
    let (transcript, exit_code) = pty_transcript(handle).await;

    // Assert
    assert!(transcript.contains("alive-42"), "no alive-42 in {transcript:?}");
    assert_eq!(exit_code, 0);
}
```

It ends the session with `exit` rather than a kill. `pty_transcript` reads until the `Exit` event, so a clean exit after 65 s shows both that input still reaches the shell and that the stream outlived 60 s.

Run on hosted:
`E2B_TEST_TARGET=hosted E2B_TEST_RUN_ID=dev-$(date -u +%Y%m%dT%H%M%SZ)-$$ cargo test --test live_commands -- --ignored --exact clm_0175_pty_outlives_sixty_seconds`
Expected: PASS, in about 70 s. Also rerun `clm_0047`, `clm_0048` and `clm_0049`, which now run without a deadline, and report all four in the PR.

- [ ] **Step 6: README, docs, gates, changelog, commit**

`README.md`, the PTY part of the tour example:

```rust
    // An interactive terminal. It has no deadline by default (the SDKs use
    // 60 s, which ends the shell mid-session); set one with
    // `PtyOptions::default().timeout(Some(..))`.
    let mut pty = sandbox
        .pty()
        .create(PtySize::new(80, 24), &PtyOptions::default())
        .await?;
```

`docs/protocol-notes.md`, "PTY" section:
- The `create`/`connect` row's parenthesis becomes "(here neither `PtyOptions` nor `PtyConnectOptions` sets a deadline by default, a deliberate difference, since a deadline ends the shell; `Some(DEFAULT_PTY_TIMEOUT)` gives the SDK value)".
- The deadline row begins "When a deadline is set, it counts from when the stream opens.".

Gates: `just lint`, `just test`, `just doc`.

`changelog/<ts>-m4-task-08-no-pty-deadline.md` lists:
- the new defaults;
- the live result;
- "Breaking changes": `PtyOptions::default()` and `PtyConnectOptions::default()` no longer set a 60 s deadline. A caller who relied on it adds `.timeout(Some(DEFAULT_PTY_TIMEOUT))`.
- It closes #55.

```bash
git add src/pty.rs tests/envd_pty.rs tests/envd_connect_options.rs tests/live_commands.rs README.md docs/protocol-notes.md changelog/
git commit -m "feat!: no PTY deadline by default (#55)"
```


---

### Task 9: Cap concurrent uploads in write_files (#53, CLM-0176)

**Files:**
- Modify: `src/filesystem.rs`:
  - `DEFAULT_UPLOAD_CONCURRENCY`;
  - `WriteOptions::max_concurrency`, with a hand-written `Default`;
  - the octet-stream branch of `write_files` (lines 1040-1052 at 7ee3eab, moved by Task 5);
  - the unit test from Task 5's Step 5
- Modify: `src/lib.rs` (re-export `DEFAULT_UPLOAD_CONCURRENCY`)
- Modify: `Cargo.toml` (the comment on futures-util only; the dependency line and `Cargo.lock` stay unchanged)
- Modify: `tests/envd_write.rs` (four new tests)
- Modify: `docs/protocol-notes.md` (the octet-stream row, line 335)
- Create: `changelog/<ts>-m4-task-09-upload-cap.md`

> **Corrected (issue #53, 2026-10-28).** `try_buffered` does not fail fast: `TryBuffered` keeps its in-flight uploads in a `FuturesOrdered`, which yields only in input order, so an error from entry 2 waits until entry 1 answers. With it, the existing `octet_stream_batch_fails_fast_without_waiting_for_the_other_uploads` (a 5 s upload first, a 507 second) failed after 5.05 s. The shipped code tags each upload with its entry index (`TryFutureExt::map_ok`), runs them through `TryStreamExt::try_buffer_unordered(n)` (same cap, results as they finish) and stores each result at its index with `try_for_each`, which stops at the first error and drops the rest; Step 3's code and the Cargo.toml comment below now say so (read `src/filesystem.rs`, `write_files`). The imports are `use futures_util::{TryFutureExt, TryStreamExt};`. `docs/protocol-notes.md` also gains an "At most 8 octet-stream uploads in flight" bullet under "Deliberate differences from the SDKs", and the `write_files` rustdoc says an empty `entries` skips the cap check too. The changelog is `changelog/2026-10-28T14-17-25Z-m4-task-09-upload-cap.md`. No later task changes: Tasks 12 and 14 name only `max_concurrency`, `DEFAULT_UPLOAD_CONCURRENCY` and `WriteOptions::new`/`default`, which are as planned; the Task 12 table's `Default` for `WriteOptions` is still implemented, by hand.

`futures_util::stream::iter` turns the uploads into a stream. `TryStreamExt::try_buffer_unordered(n)` polls at most `n` of them at once and yields each result as it finishes (behind `alloc`, which is already enabled); each result carries its entry index and is stored at that index, so the results keep input order. `try_for_each` returns at the first `Err` and drops the stream, so uploads still in flight are cancelled, as `try_join_all` did. (`try_buffered(n)` keeps order itself but holds an error behind every earlier upload, so it does not fail fast.)

- [ ] **Step 1: Write the failing tests**

Append to `tests/envd_write.rs`. Import `DEFAULT_UPLOAD_CONCURRENCY` from `e2b_client`, `std::sync::{Arc, Mutex}`, and add `Respond` to the `wiremock::{Mock, MockServer, Request, ResponseTemplate}` import (`tests/envd_write.rs:20`; `written` at line 37 and `Instant` at line 8 already exist). The two cap tests count uploads in flight from their arrival times (brooke-reviews, PR #89), so they have no wall-clock ceiling to flake on a busy runner:

```rust
/// Answers every octet-stream upload after `delay`, recording when each one
/// arrived. wiremock calls `respond` as a request arrives and only then
/// waits out the delay, so each arrival opens a window of `delay` during
/// which that upload is in flight.
struct GaugedUploads {
    delay: Duration,
    arrivals: Arc<Mutex<Vec<Instant>>>,
}

impl Respond for GaugedUploads {
    fn respond(&self, request: &Request) -> ResponseTemplate {
        self.arrivals.lock().expect("arrivals lock").push(Instant::now());
        let target = request
            .url
            .query_pairs()
            .find(|(key, _)| key == "path")
            .map(|(_, value)| value.into_owned())
            .unwrap_or_default();
        written(&[target.as_str()]).set_delay(self.delay)
    }
}

/// The largest number of uploads whose `[arrival, arrival + delay)` windows
/// overlap. Under a cap of `n` the client sends upload `n + 1` only after a
/// response, which is at least `delay` after the first arrival, so this is
/// exactly the number in flight, with no wall-clock ceiling to flake on.
fn peak_in_flight(arrivals: &[Instant], delay: Duration) -> usize {
    arrivals
        .iter()
        .map(|&start| {
            arrivals
                .iter()
                .filter(|&&other| other <= start && start < other + delay)
                .count()
        })
        .max()
        .unwrap_or(0)
}

/// Uploads `count` files as octet-stream with `options`, each answered after
/// `delay`, and returns the peak number of uploads in flight.
async fn gauged_batch(count: usize, delay: Duration, options: WriteOptions) -> usize {
    let server = MockServer::start().await;
    let sandbox = sandbox_with_version(&server, "0.5.7").await;
    let arrivals = Arc::new(Mutex::new(Vec::with_capacity(count)));
    Mock::given(method("POST"))
        .and(path("/files"))
        .respond_with(GaugedUploads { delay, arrivals: Arc::clone(&arrivals) })
        .expect(u64::try_from(count).expect("count fits u64"))
        .mount(&server)
        .await;
    let entries: Vec<WriteEntry> = (0..count)
        .map(|index| WriteEntry::new(format!("/w/{index}"), "x"))
        .collect();
    let written = sandbox
        .files()
        .write_files(&entries, &options.octet_stream(true))
        .await
        .expect("write_files");
    assert_eq!(written.len(), count);
    let arrivals = arrivals.lock().expect("arrivals lock");
    assert_eq!(arrivals.len(), count);
    peak_in_flight(&arrivals, delay)
}

#[tokio::test]
async fn octet_stream_uploads_are_capped_at_max_concurrency() {
    // Arrange and act: 6 uploads of 500 ms each, at most 2 at a time.
    let peak = gauged_batch(6, Duration::from_millis(500), WriteOptions::default().max_concurrency(2)).await;

    // Assert: exactly 2 were ever in flight together.
    assert_eq!(peak, 2);
}

#[tokio::test]
async fn the_default_cap_is_eight_uploads_in_flight() {
    // Arrange and act: 9 uploads of 500 ms each with the default options.
    let peak = gauged_batch(9, Duration::from_millis(500), WriteOptions::default()).await;

    // Assert: the 9th waited for a slot, so exactly 8 overlapped.
    assert_eq!(DEFAULT_UPLOAD_CONCURRENCY, 8);
    assert_eq!(peak, 8);
}

#[tokio::test]
async fn capped_uploads_keep_the_entry_order() {
    // Arrange: the first file answers last, the last file first.
    let server = MockServer::start().await;
    let sandbox = sandbox_with_version(&server, "0.5.7").await;
    for (target, delay_ms) in [("/w/1", 600), ("/w/2", 300), ("/w/3", 0)] {
        Mock::given(method("POST"))
            .and(path("/files"))
            .and(query_param("path", target))
            .respond_with(written(&[target]).set_delay(Duration::from_millis(delay_ms)))
            .expect(1)
            .mount(&server)
            .await;
    }
    let entries = ["/w/1", "/w/2", "/w/3"].map(|target| WriteEntry::new(target, "x"));

    // Act
    let written = sandbox
        .files()
        .write_files(&entries, &WriteOptions::default().octet_stream(true).max_concurrency(3))
        .await
        .expect("write_files");

    // Assert: results follow `entries`, not completion order.
    let paths: Vec<&str> = written.iter().map(|info| info.path.as_str()).collect();
    assert_eq!(paths, ["/w/1", "/w/2", "/w/3"]);
}

#[tokio::test]
async fn a_zero_max_concurrency_is_rejected_before_sending() {
    // Arrange
    let server = MockServer::start().await;
    let sandbox = sandbox_with_version(&server, "0.5.7").await;

    // Act
    let error = sandbox
        .files()
        .write_files(&two_entries(), &WriteOptions::default().octet_stream(true).max_concurrency(0))
        .await
        .expect_err("zero concurrency");

    // Assert
    assert!(
        matches!(&error, Error::InvalidArgument(message) if message.contains("max_concurrency")),
        "{error:?}"
    );
    assert_no_file_requests(&server).await;
}
```

The existing `octet_stream_uploads_run_concurrently` (3 uploads) and `octet_stream_batch_fails_fast_without_waiting_for_the_other_uploads` (2 uploads) stay unchanged. Both batches are under the default cap, so both must still pass.

In the `src/filesystem.rs` unit tests, `write_options_default_to_multipart_and_chain` (as left by Task 5) gains `max_concurrency: DEFAULT_UPLOAD_CONCURRENCY,` in its struct literal, and this test is added:

```rust
    #[test]
    fn write_options_default_to_eight_uploads_in_flight() {
        assert_eq!(WriteOptions::default().max_concurrency, 8);
        assert_eq!(WriteOptions::default().max_concurrency(3).max_concurrency, 3);
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --test envd_write`
Expected: FAIL to compile: "no method named `max_concurrency` found for struct `WriteOptions`" and "unresolved import `e2b_client::DEFAULT_UPLOAD_CONCURRENCY`".

- [ ] **Step 3: Implement**

`src/filesystem.rs`, after `DEFAULT_WATCH_TIMEOUT`:

```rust
/// How many octet-stream or gzip uploads [`Filesystem::write_files`] keeps in
/// flight by default ([`WriteOptions::max_concurrency`]). The SDK starts them
/// all at once (`asyncio.gather`); a cap keeps a large batch from opening
/// one connection per file.
pub const DEFAULT_UPLOAD_CONCURRENCY: usize = 8;
```

`WriteOptions`: drop `Default` from its derive (`#[derive(Debug, Clone, PartialEq, Eq)]`), add the field `max_concurrency: usize,` after `metadata`, and add:

```rust
impl Default for WriteOptions {
    fn default() -> Self {
        Self {
            user: None,
            request_timeout: RequestTimeout::default(),
            octet_stream: false,
            gzip: false,
            metadata: BTreeMap::new(),
            max_concurrency: DEFAULT_UPLOAD_CONCURRENCY,
        }
    }
}
```

The setter, after `gzip`:

```rust
    /// How many octet-stream or gzip uploads [`Filesystem::write_files`]
    /// keeps in flight at once; the rest start as earlier ones finish.
    /// Defaults to [`DEFAULT_UPLOAD_CONCURRENCY`] (8). Multipart sends one
    /// request for the whole batch, so the cap does not apply to it.
    ///
    /// # Arguments
    ///
    /// * `max_concurrency` - at least 1; 0 makes the write fail with
    ///   [`Error::InvalidArgument`] before any request.
    ///
    /// # Returns
    ///
    /// The options, for chaining.
    #[must_use]
    pub fn max_concurrency(mut self, max_concurrency: usize) -> Self {
        self.max_concurrency = max_concurrency;
        self
    }
```

In `write_files`, after `let metadata = options.metadata_headers(self.envd.version())?;`, add:

```rust
        if options.max_concurrency == 0 {
            return Err(Error::InvalidArgument(
                "max_concurrency should be at least 1".into(),
            ));
        }
```

Replace the octet-stream branch with:

```rust
        if self.uses_octet_stream(options) {
            // `Bytes::clone` is a reference-count bump, not a copy of the data.
            // `stream::iter` yields one not-yet-started upload per entry, and
            // `try_buffer_unordered(n)` runs at most `n` of them at once, like a
            // bounded `asyncio.gather`. It yields each result as it finishes,
            // so an error surfaces at once even while an earlier entry is
            // still uploading (`try_buffered` would hold it back until every
            // earlier entry answered). `try_for_each` stops at the first error
            // and drops the stream, which cancels the uploads still in flight;
            // each result is stored at its entry's index to keep entry order.
            let uploads =
                futures_util::stream::iter(entries.iter().enumerate().map(|(index, entry)| {
                    let body = if options.gzip {
                        gzip_body(std::io::Cursor::new(entry.data.clone()))
                    } else {
                        reqwest::Body::from(entry.data.clone())
                    };
                    let upload =
                        self.post_octet_stream(&entry.path, body, options, &metadata, timeout);
                    // The stream must yield `Result<future, error>`; `map_ok` tags
                    // the upload's answer with its entry index. `move` copies
                    // `index` (a `usize`) into the closure.
                    Ok(upload.map_ok(move |infos| (index, infos)))
                }));
            // `Vec::new` does not allocate, so this costs one slot per entry.
            let mut written: Vec<Vec<WriteInfo>> = std::iter::repeat_with(Vec::new)
                .take(entries.len())
                .collect();
            uploads
                .try_buffer_unordered(options.max_concurrency)
                .try_for_each(|(index, infos)| {
                    written[index] = infos;
                    std::future::ready(Ok(()))
                })
                .await?;
            return Ok(written.into_iter().flatten().collect());
        }
```

Add `use futures_util::{TryFutureExt, TryStreamExt};` to the imports. The old `futures_util::future::try_join_all` use goes; if it was the last user of the path in the crate, nothing else changes, because `alloc` stays enabled for `try_buffer_unordered`. Update the Cargo.toml comment on futures-util to "`alloc` for `TryStreamExt::try_buffer_unordered`, which runs capped concurrent uploads", without changing the dependency line.

The `write_files` rustdoc paragraph "The octet-stream requests all start at once..." becomes:

```rust
    /// The octet-stream requests run concurrently, at most
    /// [`WriteOptions::max_concurrency`] at a time (8 by default), where the
    /// SDK's `asyncio.gather` starts them all at once. Results come back in
    /// the order of `entries` whatever order the uploads finish in.
```

`# Errors` gains "[`Error::InvalidArgument`] for a zero `max_concurrency`, before any request."

`src/lib.rs`: add `DEFAULT_UPLOAD_CONCURRENCY` to the `pub use filesystem::{...}` list.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --test envd_write && cargo test --lib filesystem::tests`
Expected: PASS, including the two existing concurrency tests.

- [ ] **Step 5: Docs, gates, changelog, commit**

The octet-stream row of `docs/protocol-notes.md` (line 335) becomes:

```
| Octet-stream: one `POST /files?path=<p>[&username=<u>]` per file, `Content-Type: application/octet-stream`, the body is the file; the SDK runs all requests concurrently (`asyncio.gather`), the crate at most `WriteOptions::max_concurrency` at a time (default 8, a deliberate deviation, #53); results are in entry order and the first error fails the batch | `py: sandbox_async/filesystem/filesystem.py:400-444` |
```

Run `just lint`, `just test` and `just doc`. The wire format is unchanged, and so is any batch of 8 files or fewer, so no live run is needed.

`changelog/<ts>-m4-task-09-upload-cap.md`:

```markdown
# M4 Task 9: Cap concurrent uploads in write_files (issue #53)

Plan: `docs/superpowers/plans/2026-10-28-milestone-4-api-stability.md`, Task 9.
Claim: CLM-0176 (S3).

## What changed

- `WriteOptions::max_concurrency(n)` and `DEFAULT_UPLOAD_CONCURRENCY` (8).
  Octet-stream and gzip uploads run through `try_buffer_unordered(n)`: at most `n`
  in flight, results in entry order, the first error fails the batch and
  cancels the rest. `max_concurrency(0)` is `Error::InvalidArgument`
  before any request.
- `WriteOptions` has a hand-written `Default` (the cap is not zero).
- Tests: 4 in `tests/envd_write.rs`, 1 unit test.

## Breaking changes

- A batch of more than 8 octet-stream or gzip files no longer starts every
  upload at once. `max_concurrency(entries.len())` restores the old
  behaviour.

## Why

pgrace chose a default cap of 8 on 2026-10-28 (plan decision 7): a large
batch opened one connection per file.
```

```bash
git add src/filesystem.rs src/lib.rs Cargo.toml tests/envd_write.rs docs/protocol-notes.md changelog/
git commit -m "feat: cap concurrent octet-stream uploads at 8 by default"
```


---

### Task 10: Report symlinks from is_symlink (#30, CLM-0177)

**Files:**
- Modify: `src/envd/filesystem_pb.rs` (`EntryInfoWire`, lines 56-82)
- Modify: `src/filesystem.rs` (`EntryInfo::from`, lines 109-128; a unit test)
- Modify: `tests/envd_api.rs` (a mock test for both envd shapes)
- Modify: `tests/live_files.rs` (`clm_0177_symlinks_report_symlink_type`)
- Modify: `docs/protocol-notes.md` (the `EntryInfo` message row, line 302; a behaviour row)
- Create: `changelog/<ts>-m4-task-10-is-symlink.md`

> **Corrected (issue #30, 2026-10-28).** From review, Steps 1 and 3 now decode an explicit `isSymlink: null` as `false` (`deserialize_with = "deserialize_null_as_default"`, as `metadata` does), and the protocol-notes row reads "A symlink is `FileType::Symlink` on every envd version" (the wire `type` is not `FILE_TYPE_SYMLINK` on newer envd). The changelog slug is `is-symlink`, not `symlinks`. Hosted E2B runs envd 0.10.0, which uses the new shape: with the fix disabled the live test saw `file-link` as `File` and `dir-link` as `Directory`; with it, the test passes. No later task changes.

Newer envd reports a symlink's *target* type in `type` and sets `is_symlink` (proto field 12, `py: envd/filesystem/filesystem_pb.py:240, 299-303`). Older envd reports `FILE_TYPE_SYMLINK` in `type`. The SDK maps `FileType.SYMLINK if entry.is_symlink else map_file_type(entry.type)` (`py: sandbox/filesystem/filesystem.py:130-148`). The crate reads only `type` today. Watch events build their entry through the same `From`, so they get the fix too.

- [ ] **Step 1: Write the failing tests**

Append to `tests/envd_api.rs`:

```rust
#[tokio::test]
async fn list_and_stat_report_symlinks_for_both_envd_shapes() {
    // Arrange: newer envd sets `isSymlink` with the target's type; older envd
    // puts FILE_TYPE_SYMLINK in `type`; a plain file has `isSymlink: false`.
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    let new_shape = json!({
        "name": "link", "type": "FILE_TYPE_FILE", "path": "/w/link",
        "isSymlink": true, "symlinkTarget": "/w/target"
    });
    Mock::given(method("POST"))
        .and(path("/filesystem.Filesystem/ListDir"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"entries": [
            new_shape.clone(),
            {"name": "old", "type": "FILE_TYPE_SYMLINK", "path": "/w/old"},
            {"name": "target", "type": "FILE_TYPE_FILE", "path": "/w/target", "isSymlink": false},
            {"name": "dirlink", "type": "FILE_TYPE_DIRECTORY", "path": "/w/dirlink", "isSymlink": true}
        ]})))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/filesystem.Filesystem/Stat"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"entry": new_shape})))
        .mount(&server)
        .await;
    let files = sandbox.files();

    // Act
    let entries = files.list("/w", &ListOptions::default()).await.expect("list");
    let info = files.get_info("/w/link", &FileOptions::default()).await.expect("stat");

    // Assert
    let types: Vec<(&str, FileType)> = entries
        .iter()
        .map(|entry| (entry.name.as_str(), entry.file_type))
        .collect();
    assert_eq!(
        types,
        [
            ("link", FileType::Symlink),
            ("old", FileType::Symlink),
            ("target", FileType::File),
            ("dirlink", FileType::Symlink),
        ]
    );
    assert_eq!(info.file_type, FileType::Symlink);
    assert_eq!(info.symlink_target.as_deref(), Some("/w/target"));
}
```

Append to the `tests` module of `src/filesystem.rs`:

```rust
    #[test]
    fn is_symlink_wins_over_the_reported_type() {
        let decode = |raw: &str| {
            EntryInfo::from(serde_json::from_str::<EntryInfoWire>(raw).expect("entry json"))
        };
        assert_eq!(
            decode(r#"{"type":"FILE_TYPE_DIRECTORY","isSymlink":true}"#).file_type,
            FileType::Symlink
        );
        // The proto field name is accepted too, as Connect's JSON codec allows.
        assert_eq!(
            decode(r#"{"type":"FILE_TYPE_FILE","is_symlink":true}"#).file_type,
            FileType::Symlink
        );
        assert_eq!(decode(r#"{"type":"FILE_TYPE_FILE"}"#).file_type, FileType::File);
        // An explicit null is proto3 JSON's default, not a decode error.
        assert_eq!(
            decode(r#"{"type":"FILE_TYPE_FILE","isSymlink":null}"#).file_type,
            FileType::File
        );
        assert_eq!(decode(r#"{"type":3}"#).file_type, FileType::Symlink);
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --test envd_api list_and_stat_report_symlinks && cargo test --lib filesystem::tests::is_symlink`
Expected: FAIL. The mock test fails with `left: [("link", File), ("old", Symlink), ("target", File), ("dirlink", Directory)]`. The unit test fails on its first assertion (`Directory` != `Symlink`).

- [ ] **Step 3: Implement**

`src/envd/filesystem_pb.rs`, `EntryInfoWire`, after `metadata`:

```rust
    /// Whether the entry is a symbolic link, `bool` field 12
    /// (`e2b/envd/filesystem/filesystem_pb.py:240, 299-303`). Newer envd sets
    /// it and reports the *target's* type in `type`; older envd omits it and
    /// reports `FILE_TYPE_SYMLINK` in `type`. The struct's camelCase renaming
    /// makes the JSON name `isSymlink`; the alias also accepts the proto
    /// name, which Connect's JSON codec allows. An explicit `null` decodes
    /// as `false`, as proto3 JSON treats it as the default.
    #[serde(
        default,
        alias = "is_symlink",
        deserialize_with = "deserialize_null_as_default"
    )]
    pub(crate) is_symlink: bool,
```

`src/filesystem.rs`, in `impl From<EntryInfoWire> for EntryInfo`, replace the `file_type` line with:

```rust
            // A link's own type, as the SDK reports it
            // (`e2b/sandbox/filesystem/filesystem.py:130-148`): `is_symlink`
            // wins, else the reported type, which is how older envd says it.
            file_type: if wire.is_symlink {
                FileType::Symlink
            } else {
                FileType::from_wire(wire.file_type.as_ref())
            },
```

The `EntryInfo::file_type` field doc becomes "File, directory or symlink. A symbolic link is `Symlink` on every envd version, never its target's type."

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --test envd_api && cargo test --lib filesystem::tests`
Expected: PASS.

- [ ] **Step 5: The live test**

Append to `tests/live_files.rs`:

```rust
#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0177_symlinks_report_symlink_type() {
    // Arrange: a file, a directory, and a link to each.
    let sandbox = LiveSandbox::new().await;
    let files = sandbox.files();
    files
        .write("/tmp/sym/target.txt", "t", &WriteOptions::default())
        .await
        .expect("write target");
    files
        .make_dir("/tmp/sym/dir", &FileOptions::default())
        .await
        .expect("make dir");
    let linked = sandbox
        .commands()
        .run(
            "ln -s /tmp/sym/target.txt /tmp/sym/file-link && ln -s /tmp/sym/dir /tmp/sym/dir-link",
            &CommandOptions::default(),
        )
        .await
        .expect("run ln");
    assert_eq!(linked.exit_code, 0, "ln failed: {}", linked.stderr);

    // Act
    let entries = files
        .list("/tmp/sym", &ListOptions::default())
        .await
        .expect("list");
    let file_link = files
        .get_info("/tmp/sym/file-link", &FileOptions::default())
        .await
        .expect("get_info file-link");

    // Assert: each link is a symlink whatever it points at; the targets keep
    // their own types.
    let found: HashSet<(String, FileType)> = entries
        .into_iter()
        .map(|entry| (entry.name, entry.file_type))
        .collect();
    let want: HashSet<(String, FileType)> = [
        ("target.txt".to_owned(), FileType::File),
        ("dir".to_owned(), FileType::Directory),
        ("file-link".to_owned(), FileType::Symlink),
        ("dir-link".to_owned(), FileType::Symlink),
    ]
    .into_iter()
    .collect();
    assert_eq!(found, want, "envd {}", sandbox.envd_version());
    assert_eq!(file_link.file_type, FileType::Symlink);
}
```

Run it on hosted:

```bash
E2B_TEST_TARGET=hosted E2B_TEST_RUN_ID=dev-$(date -u +%Y%m%dT%H%M%SZ)-$$ \
  cargo test --test live_files -- --ignored --exact clm_0177_symlinks_report_symlink_type
```

Expected: PASS. If `get_info` on the link reports the target (envd's `Stat` following the link without setting `is_symlink`), record the observed envd version and response in the PR, and correct this task with a dated note before changing the assertion. Don't weaken it silently.

- [ ] **Step 6: Docs, gates, changelog, commit**

`docs/protocol-notes.md`:
- the `EntryInfo` message row (line 302) is unchanged, since it already lists `is_symlink`;
- add a "Filesystem" behaviour row after the `list` depth row:

```
| A symlink is `FileType::Symlink` on every envd version: newer envd sets `is_symlink` (JSON `isSymlink`) and puts the target's type in `type`, so `is_symlink` wins; older envd reports `FILE_TYPE_SYMLINK` in `type` | `py: sandbox/filesystem/filesystem.py:130-148`; `py: envd/filesystem/filesystem_pb.py:240, 299-303` |
```

The "Not implemented yet" paragraph (line 430) is rewritten by Task 18, not here, to avoid conflicts with Tasks 1 and 3.

Run `just lint`, `just test` and `just doc`.

`changelog/<ts>-m4-task-10-is-symlink.md`:

```markdown
# M4 Task 10: Report symlinks from is_symlink (issue #30)

Plan: `docs/superpowers/plans/2026-10-28-milestone-4-api-stability.md`, Task 10.
Claim: CLM-0177 (S3).

## What changed

- `EntryInfoWire` reads `isSymlink` (also accepted as `is_symlink`), and
  `EntryInfo::from` reports `FileType::Symlink` when it is set, as the SDK
  does. `list`, `get_info`, `rename` and watch-event entries all go through
  that conversion.
- Tests: a mock test for both envd shapes in `tests/envd_api.rs`, a unit test,
  and the live test `clm_0177_symlinks_report_symlink_type`.

## Breaking changes

- On newer envd a symlink used to report its target's type (`File` or
  `Directory`); it now reports `Symlink`. Code that relied on following the
  type through a link should read `symlink_target` or stat the target.

## Live result (hosted E2B)

`clm_0177_symlinks_report_symlink_type`: <pass|fail> on envd <version> (fill in).
```

```bash
git add src/envd/filesystem_pb.rs src/filesystem.rs tests/envd_api.rs tests/live_files.rs docs/protocol-notes.md changelog/
git commit -m "fix: report symlinks from envd's is_symlink (#30)"
```


---

### Task 11: Investigate Pty::connect to an exited pid (#56, CLM-0178)

**Files:**
- Modify: `tests/live_commands.rs`, which gains `clm_0178_pty_connect_to_exited_pid`.
- Create: `target/i<N>-logs/sdk_pty_connect.py`, an investigation script. It lives in the worktree's `target/` and is not committed.
- Modify, depending on the outcome:
  - `src/pty.rs` and `src/commands.rs` in branch (a);
  - `tests/envd_pty.rs` in branch (a);
  - the rustdoc of `Pty::connect` in branch (b);
  - `docs/protocol-notes.md`, the PTY section, in all branches.
- Create: `changelog/<ts>-m4-task-11-pty-connect-exited-pid.md`.

> **Corrected (issue #56, 2026-10-28).** Branch (b) applied: on hosted E2B and with SDK 2.61.0, `connect` to a just-exited pid succeeds and the first event is the shell's exit; by 30 s envd answers `not_found`. What shipped differs from Steps 1 and 4 (b) as follows. The enum is `PtyAttach` and the helper `attach(sandbox, pid)` is shared by `attach_after_exit`. The shell exits with code 3, so the replayed exit is identifiably its own. The test also connects to pid 999999, which never existed, and asserts `not_found`; this pins the rustdoc's first sentence and reads the `Refused` field, which would otherwise be a dead-code warning under `-D warnings`. The assertion checks both cases in one `assert!` whose message prints both outcomes, because Step 1's loop stops at the first failing case and hides the 5 s outcome. `Commands::connect` behaves the same (one probe run), so its `# Errors` got the matching correction, as Step 4 (b) asks, with no live test of its own. Read `tests/live_commands.rs` and `changelog/2026-10-28T14-41-37Z-m4-task-11-pty-connect-exited-pid.md`.

**The finding.** During M2 Task 3 (#32, PR #43), `Pty::connect` with the pid of a shell that had already exited returned `Ok`. The rustdoc says a missing process gives `Error::Rpc` with `ConnectCode::NotFound`. Nothing was investigated.

What the code does today:
- `Pty::connect` returns `Ok` only once envd has sent a `start` event on the `Connect` stream, because `open_process_stream` waits for it (`src/commands.rs`).
- The SDK waits for the same event (`py: sandbox_async/commands/pty.py:236-248`, through `first_event`, `py: envd/client_async/__init__.py:113-149`).

So an `Ok` means envd confirmed the attach. Candidates:
- envd keeps an exited process attachable for a short time and then sends its end event;
- the exit had not finished when connect ran.

**Investigate first.** If the finding is refuted (connect does fail with `not_found`), the work is branch (c): pin it with the test and report. No speculative fix.

- [ ] **Step 1: Write the repro as a live test of the documented behaviour**

In `tests/live_commands.rs` (`ConnectCode`, `Error` and `PtyEvent` are already imported; add `PtyConnectOptions` to the `use e2b_client::{...}` list, since Task 7 imported only `CommandConnectOptions` there; corrected in issue #54, 2026-10-28), add:

```rust
/// What a `Pty::connect` to the pid of an exited shell gave: the `connect`
/// result, and the first event when it succeeded.
#[derive(Debug)]
enum ExitedPidAttach {
    /// `connect` failed.
    Refused(Error),
    /// `connect` succeeded; the first event (or error) on the stream.
    Attached(Result<Option<PtyEvent>, Error>),
}

/// Starts a PTY, makes its shell exit and waits for the exit event, then
/// connects to the same pid after `delay`.
async fn attach_after_exit(sandbox: &LiveSandbox, delay: Duration) -> ExitedPidAttach {
    let handle = sandbox
        .pty()
        .create(PtySize::new(80, 24), &PtyOptions::default())
        .await
        .expect("create pty");
    let pid = handle.pid();
    handle.send_stdin("exit 0\n").await.expect("send exit");
    let exit = handle.wait().await.expect("wait for the shell to exit");
    assert_eq!(exit.exit_code, 0);
    tokio::time::sleep(delay).await;
    match sandbox.pty().connect(pid, &PtyConnectOptions::default()).await {
        Err(error) => ExitedPidAttach::Refused(error),
        Ok(mut attached) => ExitedPidAttach::Attached(attached.next_event().await),
    }
}

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0178_pty_connect_to_exited_pid() {
    // Arrange
    let sandbox = LiveSandbox::new().await;

    // Act: straight after the exit event, and again after 5 s, in case envd
    // keeps an exited process attachable for a grace period.
    let at_once = attach_after_exit(&sandbox, Duration::ZERO).await;
    let later = attach_after_exit(&sandbox, Duration::from_secs(5)).await;

    // Assert: the documented behaviour, `not_found` from `connect` itself.
    // Step 3 pins this assertion to the behaviour actually observed.
    for (when, outcome) in [("at once", &at_once), ("after 5 s", &later)] {
        assert!(
            matches!(
                outcome,
                ExitedPidAttach::Refused(Error::Rpc { code: ConnectCode::NotFound, .. })
            ),
            "{when}: connect to an exited pid gave {outcome:?}"
        );
    }
}
```

When the assertion fails, the panic message carries what happened, either `Attached(Ok(Some(Exit(..))))` or `Attached(Err(..))`, so the run log is the record.

- [ ] **Step 2: Reproduce on hosted E2B, three times**

```bash
mkdir -p target/i<N>-logs
for attempt in 1 2 3; do
  E2B_TEST_TARGET=hosted E2B_TEST_RUN_ID=dev-$(date -u +%Y%m%dT%H%M%SZ)-$$ \
    cargo test --test live_commands -- --ignored --exact --nocapture \
    clm_0178_pty_connect_to_exited_pid 2>&1 | tee -a target/i<N>-logs/repro.log
done
```

Record each attempt's outcome, `at once` and `after 5 s`, in the changelog entry. Three attempts separate a race (outcomes differ between runs) from a fixed behaviour.

- [ ] **Step 3: Compare with the Python SDK 2.61.0**

The SDK installed under `~/.local/lib/python3.12/site-packages` is e2b 2.21.1, so use a throwaway uv environment pinned to 2.61.0, the version every citation in this plan uses. Write `target/i<N>-logs/sdk_pty_connect.py`:

```python
"""Pty connect to an exited pid with e2b 2.61.0 (issue #56). Prints outcomes only, never the key."""

import asyncio
import pathlib
import sys

from e2b import AsyncSandbox, PtySize


def hosted_key() -> str:
    for line in pathlib.Path("/home/pgrace/repos/e2b_client_rs/.env").read_text().splitlines():
        if line.startswith("E2B_API_KEY_HOSTED="):
            return line.split("=", 1)[1].strip().strip('"')
    sys.exit("E2B_API_KEY_HOSTED missing from .env")


async def attach_after_exit(sandbox: AsyncSandbox, delay: float) -> str:
    first = await sandbox.pty.create(PtySize(rows=24, cols=80), on_data=lambda _: None)
    await sandbox.pty.send_stdin(first.pid, b"exit 0\n")
    await first.wait()
    await asyncio.sleep(delay)
    try:
        attached = await sandbox.pty.connect(first.pid, on_data=lambda _: None)
    except Exception as error:  # the outcome is the point
        return f"connect raised {type(error).__name__}: {error}"
    try:
        result = await attached.wait()
        return f"connect ok; wait -> exit_code={result.exit_code}"
    except Exception as error:
        return f"connect ok; wait raised {type(error).__name__}: {error}"


async def main() -> None:
    import e2b

    print(f"e2b {e2b.__version__}" if hasattr(e2b, "__version__") else "e2b (version attribute missing)")
    sandbox = await AsyncSandbox.create(
        timeout=60, metadata={"e2b-client-test": "m4-task-11-sdk"}, api_key=hosted_key()
    )
    try:
        print("at once:", await attach_after_exit(sandbox, 0))
        print("after 5 s:", await attach_after_exit(sandbox, 5))
    finally:
        await sandbox.kill()


asyncio.run(main())
```

Run it isolated (`-I`), from outside the repo so no local module shadows the SDK, and pinned to the release:

```bash
cd /tmp && uv run --no-project --with e2b==2.61.0 python -I \
  "$OLDPWD/target/i<N>-logs/sdk_pty_connect.py" 2>&1 | tee -a "$OLDPWD/target/i<N>-logs/sdk.log"; cd "$OLDPWD"
```

Expected: one line for each case. Copy both into the changelog entry. If `e2b.__version__` is missing, confirm the version with `uv run --no-project --with e2b==2.61.0 python -I -c "import importlib.metadata as m; print(m.version('e2b'))"`.

- [ ] **Step 4: Decide, then make the matching change**

Compare Step 2 with Step 3. Exactly one branch applies.

**(a) The crate differs from the SDK** (for example, the SDK raises `NotFoundException` where the crate returns `Ok`). Find where the code paths diverge; they are `open_process_stream` versus `first_event` plus `extract_start_pid`.
- Fix the crate to match.
- Add a mock test to `tests/envd_pty.rs` that replays the wire sequence the hosted run showed, for example a `Connect` stream that opens with a `start` event and then ends with a `not_found` end-of-stream error. The test asserts the SDK's outcome.
- The live test's assertion stays as written in Step 1.

**(b) The crate and the SDK agree, and envd attaches to the exited pid** (the connect succeeds, and the first event is the exit or an error).
- Rewrite the `# Errors` section of `Pty::connect`:

```rust
    /// # Errors
    ///
    /// [`Error::Rpc`] with [`crate::ConnectCode::NotFound`] if envd no longer
    /// knows the process, plus the errors of [`Pty::create`]. A shell that
    /// has only just exited can still be attached: envd confirms the attach
    /// and the stream then delivers <the observed first event>, as with the
    /// SDKs' `pty.connect` (observed on hosted E2B, <date>, issue #56).
```

  Replace each `<...>` with the observed fact. Make the same change to `Commands::connect` if the shared path shows the same behaviour; check that with one extra run of the Step 1 test changed to `commands().start("true", ..)`, recorded in the changelog.
- Change the live test's `Assert` to the observed behaviour. For example, for "attached, then the exit event":

```rust
    for (when, outcome) in [("at once", &at_once), ("after 5 s", &later)] {
        assert!(
            matches!(
                outcome,
                ExitedPidAttach::Attached(Ok(Some(PtyEvent::Exit(_))))
                    | ExitedPidAttach::Refused(Error::Rpc { code: ConnectCode::NotFound, .. })
            ),
            "{when}: connect to an exited pid gave {outcome:?}"
        );
    }
```

  Allow both arms only if Step 2 showed both; otherwise assert the single observed arm, so the test pins the behaviour.

**(c) Neither: connect fails with `not_found` every time** (the M2 observation does not reproduce). The rustdoc is already right, and the Step 1 test passes as written and pins it. Report in the PR that the finding is refuted, with the three attempts and the SDK's output. Then close #56 with that evidence.

In every branch, add a row to the "PTY" section of `docs/protocol-notes.md`: "`Connect` to the pid of an exited PTY: <observed outcome, at once and after 5 s>; SDK 2.61.0: <its outcome> | observed on hosted E2B, <date> (issue #56)". The run's UTC date comes from `date -u +%Y-%m-%d`.

- [ ] **Step 5: Run the pinned test, the gates and commit**

Run the live test once more on hosted with the final assertion:
`E2B_TEST_TARGET=hosted E2B_TEST_RUN_ID=dev-$(date -u +%Y%m%dT%H%M%SZ)-$$ cargo test --test live_commands -- --ignored --exact clm_0178_pty_connect_to_exited_pid`
Expected: PASS. In branch (a), also run `cargo test --test envd_pty`.

Gates: `just lint`, `just test`, `just doc`.

`changelog/<ts>-m4-task-11-pty-connect-exited-pid.md` contains:
- the three hosted attempts and the SDK 2.61.0 output, verbatim from `target/i<N>-logs/`, with no key or token in them (grep the logs for `e2b_` and `sk_` before copying);
- which branch applied and why;
- the change made;
- "Breaking changes": none in branches (b) and (c); in branch (a), the changed result.

It closes #56.

```bash
git add tests/live_commands.rs src/pty.rs src/commands.rs tests/envd_pty.rs docs/protocol-notes.md changelog/
git commit -m "fix: pin what Pty::connect returns for an exited pid (#56)"
```

Use `docs:` instead of `fix:` in the message for branch (b), and `test:` for branch (c). Stage only the files the branch changed.

---

### Task 12: API review: non_exhaustive, common traits, lints and the review record (CLM-0119, CLM-0179, CLM-0180)

**Files:**
- Create: `tests/api_traits.rs`
- Create: `docs/api-review.md`
- Modify: `Cargo.toml` (`[lints.rust]`)
- Modify: `src/commands.rs` (`#[non_exhaustive]` on `CommandEvent` and `Signal`; `CommandOptions::new`; the `start` example's `match`)
- Modify: `src/filesystem.rs` (`WatchOptions::new`, `WriteOptions::new`)
- Modify: `src/pty.rs` (`PtyOptions::new`)
- Modify: `src/stream_input.rs` (`StreamInputOptions::new`)
- Modify: `src/sandbox.rs` (`FileUrlOptions::new`)
- Modify: `tests/live_commands.rs:55-65` (catch-all arm)
- Create: `changelog/<ts>-m4-task-12-api-review.md`

> **Corrected (issue #95, 2026-10-28).** Shipped as planned, plus three changes. (1) `Error::Decode`'s `Display` is now only `could not decode response`; the serde_json detail stays in `source()`, as `Error::Transport` does since Task 2, so a chain printer shows it once (brooke-reviews on PR #102). It is pinned by `a_decode_error_shows_its_detail_once` in `tests/error_taxonomy.rs`, and it is a breaking `Display` change. (2) `src/error.rs` (`ConnectCode`) and `src/pty.rs` (`PtySize`) say in their rustdoc why they stay exhaustive. (3) `docs/api-review.md` differs from the Step 6 draft: a Constants table (one row per constant), an "Error display" section with the `Decode` decision, every deferred item of the Scope table under "Known limits", and rows corrected against the source (manual `Default` impls, `CreateSandboxParams`'s unset template, `ConnectionConfigBuilder` reached through `ConnectionConfig::builder()`, `Sandbox::get_host` as a non-remote SDK name). Read `docs/api-review.md`, not the Step 6 draft. Tasks 13 and 14 need no change: Task 13's C-STABLE edit matches the row as shipped, and Task 14's `CommandEvent` example already has a `_` arm.

This task runs after every API-changing task (1 to 11), so it reviews the surface 0.2.0 will ship. It adds no behaviour.

- [ ] **Step 1: Write the failing trait test**

Create `tests/api_traits.rs`:

```rust
//! Compile-time checks of the traits the public API promises (CLM-0179):
//! Rust API Guidelines C-SEND-SYNC and C-DEBUG, plus `new()` and `Default`
//! agreeing on every options type (C-COMMON-TRAITS).
//!
//! A missing trait is a compile error naming the type, so this file fails
//! to build rather than failing at run time.
//!
//! Python note: Rust checks these "interfaces" at compile time. `T: Send`
//! means a value can move to another thread; `T: Sync` means `&T` can be
//! shared between threads. tokio's multi-threaded runtime needs both for
//! anything a spawned task holds.

use std::fmt::Debug;

use e2b_client::{
    Client, CommandConnectOptions, CommandEvent, CommandExit, CommandHandle, CommandOptions,
    CommandResult, Commands, ConnectCode, ConnectOptions, ConnectionConfig,
    ConnectionConfigBuilder, CreateSandboxParams, EntryInfo, EnvdVersion, Error, FileOptions,
    FileStream, FileType, FileUrlOptions, Filesystem, FilesystemEvent, FilesystemEventType,
    ForkOptions, InputStream, Lifecycle, LifecycleInfo, ListOptions, ListSandboxesParams,
    ListSnapshotsParams, MetricsParams, NetworkConfig, NetworkInfo, NetworkRule, NetworkUpdate,
    OnResume, OnTimeout, PauseOptions, ProcessInfo, Pty, PtyConnectOptions, PtyEvent, PtyHandle,
    PtyOptions, PtySize, Sandbox, SandboxInfo, SandboxMetrics, SandboxPage, SandboxState, Signal,
    SnapshotInfo, SnapshotMode, SnapshotOptions, SnapshotPage, SortOrder, StreamInputOptions,
    TimeoutAction, TransportError, Volume, VolumeInfo, VolumeMount, WatchHandle, WatchOptions,
    WriteEntry, WriteInfo, WriteOptions,
};

/// Compiles only if `T` is `Send` and `Sync`.
fn assert_send_sync<T: Send + Sync>() {}

/// Compiles only if `T` is `Send`.
fn assert_send<T: Send>() {}

/// Compiles only if `T` implements `Debug`.
fn assert_debug<T: Debug>() {}

/// Compiles only if `T` is an error type that `anyhow` and `Box<dyn Error>`
/// accept: `Error + Send + Sync + 'static`.
fn assert_error<T: std::error::Error + Send + Sync + 'static>() {}

#[test]
fn clients_handles_options_and_results_are_send_and_sync() {
    // Clients and service handles.
    assert_send_sync::<Client>();
    assert_send_sync::<ConnectionConfig>();
    assert_send_sync::<ConnectionConfigBuilder>();
    assert_send_sync::<Sandbox>();
    assert_send_sync::<Commands>();
    assert_send_sync::<Pty>();
    assert_send_sync::<Filesystem>();
    assert_send_sync::<Volume>();
    // Options and parameters.
    assert_send_sync::<CommandOptions>();
    assert_send_sync::<CommandConnectOptions>();
    assert_send_sync::<PtyOptions>();
    assert_send_sync::<PtyConnectOptions>();
    assert_send_sync::<PtySize>();
    assert_send_sync::<StreamInputOptions>();
    assert_send_sync::<FileOptions>();
    assert_send_sync::<ListOptions>();
    assert_send_sync::<WriteOptions>();
    assert_send_sync::<WriteEntry>();
    assert_send_sync::<WatchOptions>();
    assert_send_sync::<FileUrlOptions>();
    assert_send_sync::<CreateSandboxParams>();
    assert_send_sync::<ListSandboxesParams>();
    assert_send_sync::<ConnectOptions>();
    assert_send_sync::<PauseOptions>();
    assert_send_sync::<Lifecycle>();
    assert_send_sync::<OnTimeout>();
    assert_send_sync::<OnResume>();
    assert_send_sync::<SnapshotMode>();
    assert_send_sync::<MetricsParams>();
    assert_send_sync::<ForkOptions>();
    assert_send_sync::<NetworkConfig>();
    assert_send_sync::<NetworkUpdate>();
    assert_send_sync::<NetworkRule>();
    assert_send_sync::<SnapshotOptions>();
    assert_send_sync::<ListSnapshotsParams>();
    assert_send_sync::<VolumeMount>();
    assert_send_sync::<SortOrder>();
    assert_send_sync::<Signal>();
    // Results and events.
    assert_send_sync::<CommandEvent>();
    assert_send_sync::<CommandExit>();
    assert_send_sync::<CommandResult>();
    assert_send_sync::<ProcessInfo>();
    assert_send_sync::<PtyEvent>();
    assert_send_sync::<EntryInfo>();
    assert_send_sync::<FileType>();
    assert_send_sync::<WriteInfo>();
    assert_send_sync::<FilesystemEvent>();
    assert_send_sync::<FilesystemEventType>();
    assert_send_sync::<SandboxInfo>();
    assert_send_sync::<SandboxPage>();
    assert_send_sync::<SandboxState>();
    assert_send_sync::<SandboxMetrics>();
    assert_send_sync::<LifecycleInfo>();
    assert_send_sync::<TimeoutAction>();
    assert_send_sync::<NetworkInfo>();
    assert_send_sync::<SnapshotInfo>();
    assert_send_sync::<SnapshotPage>();
    assert_send_sync::<VolumeInfo>();
    assert_send_sync::<EnvdVersion>();
    assert_send_sync::<ConnectCode>();
}

#[test]
fn stream_handles_are_send() {
    // A handle is driven by one task at a time (`&mut self`), so `Send` is
    // the promise; it may hold a boxed stream that is not `Sync`.
    assert_send::<CommandHandle>();
    assert_send::<PtyHandle>();
    assert_send::<WatchHandle>();
    assert_send::<FileStream>();
    assert_send::<InputStream>();
}

#[test]
fn errors_work_with_boxed_and_anyhow_errors() {
    assert_error::<Error>();
    assert_error::<TransportError>();
}

#[test]
fn every_public_type_is_debug() {
    assert_debug::<Client>();
    assert_debug::<ConnectionConfig>();
    assert_debug::<ConnectionConfigBuilder>();
    assert_debug::<Sandbox>();
    assert_debug::<Commands>();
    assert_debug::<CommandHandle>();
    assert_debug::<Pty>();
    assert_debug::<PtyHandle>();
    assert_debug::<Filesystem>();
    assert_debug::<WatchHandle>();
    assert_debug::<FileStream>();
    assert_debug::<InputStream>();
    assert_debug::<Volume>();
    assert_debug::<Error>();
    assert_debug::<TransportError>();
    assert_debug::<ConnectCode>();
    assert_debug::<EnvdVersion>();
    assert_debug::<CommandOptions>();
    assert_debug::<CommandConnectOptions>();
    assert_debug::<CommandEvent>();
    assert_debug::<CommandExit>();
    assert_debug::<CommandResult>();
    assert_debug::<ProcessInfo>();
    assert_debug::<Signal>();
    assert_debug::<PtyOptions>();
    assert_debug::<PtyConnectOptions>();
    assert_debug::<PtySize>();
    assert_debug::<PtyEvent>();
    assert_debug::<StreamInputOptions>();
    assert_debug::<FileOptions>();
    assert_debug::<ListOptions>();
    assert_debug::<WriteOptions>();
    assert_debug::<WriteEntry>();
    assert_debug::<WriteInfo>();
    assert_debug::<WatchOptions>();
    assert_debug::<EntryInfo>();
    assert_debug::<FileType>();
    assert_debug::<FilesystemEvent>();
    assert_debug::<FilesystemEventType>();
    assert_debug::<FileUrlOptions>();
    assert_debug::<CreateSandboxParams>();
    assert_debug::<ListSandboxesParams>();
    assert_debug::<SandboxInfo>();
    assert_debug::<SandboxPage>();
    assert_debug::<SandboxState>();
    assert_debug::<SortOrder>();
    assert_debug::<ConnectOptions>();
    assert_debug::<PauseOptions>();
    assert_debug::<Lifecycle>();
    assert_debug::<LifecycleInfo>();
    assert_debug::<OnTimeout>();
    assert_debug::<OnResume>();
    assert_debug::<SnapshotMode>();
    assert_debug::<TimeoutAction>();
    assert_debug::<MetricsParams>();
    assert_debug::<SandboxMetrics>();
    assert_debug::<ForkOptions>();
    assert_debug::<NetworkConfig>();
    assert_debug::<NetworkUpdate>();
    assert_debug::<NetworkRule>();
    assert_debug::<NetworkInfo>();
    assert_debug::<SnapshotOptions>();
    assert_debug::<SnapshotInfo>();
    assert_debug::<ListSnapshotsParams>();
    assert_debug::<SnapshotPage>();
    assert_debug::<VolumeInfo>();
    assert_debug::<VolumeMount>();
}

#[test]
fn every_options_type_has_new_equal_to_default() {
    // Arrange-Act-Assert collapses here: each line builds both forms and
    // compares them. C-COMMON-TRAITS: a no-argument `new()` and `Default`
    // must agree, so callers can use either.
    assert_eq!(CommandOptions::new(), CommandOptions::default());
    assert_eq!(CommandConnectOptions::new(), CommandConnectOptions::default());
    assert_eq!(PtyOptions::new(), PtyOptions::default());
    assert_eq!(PtyConnectOptions::new(), PtyConnectOptions::default());
    assert_eq!(StreamInputOptions::new(), StreamInputOptions::default());
    assert_eq!(FileOptions::new(), FileOptions::default());
    assert_eq!(ListOptions::new(), ListOptions::default());
    assert_eq!(WriteOptions::new(), WriteOptions::default());
    assert_eq!(WatchOptions::new(), WatchOptions::default());
    assert_eq!(FileUrlOptions::new(), FileUrlOptions::default());
    assert_eq!(ListSandboxesParams::new(), ListSandboxesParams::default());
    assert_eq!(ConnectOptions::new(), ConnectOptions::default());
    assert_eq!(PauseOptions::new(), PauseOptions::default());
    assert_eq!(Lifecycle::new(), Lifecycle::default());
    assert_eq!(MetricsParams::new(), MetricsParams::default());
    assert_eq!(ForkOptions::new(), ForkOptions::default());
    assert_eq!(NetworkConfig::new(), NetworkConfig::default());
    assert_eq!(NetworkUpdate::new(), NetworkUpdate::default());
    assert_eq!(NetworkRule::new(), NetworkRule::default());
    assert_eq!(SnapshotOptions::new(), SnapshotOptions::default());
    assert_eq!(ListSnapshotsParams::new(), ListSnapshotsParams::default());
}
```

- [ ] **Step 2: Run it to see it fail**

Run: `cargo test --test api_traits`
Expected: FAIL to compile with `no function or associated item named 'new' found` for `CommandOptions`, `PtyOptions`, `StreamInputOptions`, `WriteOptions`, `WatchOptions` and `FileUrlOptions` (the six options types that have only `Default` on `origin/main`). Any other error is a real finding:
- a type missing from `e2b_client` means a re-export is missing in `src/lib.rs`: add it;
- a `Send`/`Sync`/`Debug` failure is a real gap: fix the type (for example an `Rc` that should be an `Arc`), or, if the type cannot reasonably be `Sync`, move it to `stream_handles_are_send` and record why in `docs/api-review.md`. Never delete an assertion without a recorded reason;
- a `PartialEq` failure on a Task 5 or Task 7 options type: add `PartialEq, Eq` to its derive (C-COMMON-TRAITS), as every other options type has.

- [ ] **Step 3: Add `new()` to the six options types**

Each gets the same constructor, placed first in its inherent `impl` block. In `src/commands.rs` (`impl CommandOptions`):

```rust
    /// The default options: no extra environment, the sandbox's default
    /// user and working directory, no stdin, the
    /// [`DEFAULT_COMMAND_TIMEOUT`] deadline and the configured request
    /// timeout. The same as [`CommandOptions::default`].
    ///
    /// # Returns
    ///
    /// The options.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
```

In `src/pty.rs` (`impl PtyOptions`):

```rust
    /// The default options: no extra environment, the sandbox's default
    /// user and working directory, no deadline (decision 6) and the
    /// configured request timeout. The same as [`PtyOptions::default`].
    ///
    /// # Returns
    ///
    /// The options.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
```

In `src/stream_input.rs` (`impl StreamInputOptions`):

```rust
    /// The default options: the [`DEFAULT_STREAM_INPUT_TIMEOUT`] deadline.
    /// The same as [`StreamInputOptions::default`].
    ///
    /// # Returns
    ///
    /// The options.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
```

In `src/filesystem.rs` (`impl WriteOptions`):

```rust
    /// The default options: the sandbox's default user, multipart upload
    /// (octet-stream where envd supports it), no gzip, no metadata, at
    /// most [`DEFAULT_UPLOAD_CONCURRENCY`] uploads in flight and the
    /// configured request timeout. The same as [`WriteOptions::default`].
    ///
    /// # Returns
    ///
    /// The options.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
```

In `src/filesystem.rs` (`impl WatchOptions`):

```rust
    /// The default options: not recursive, no entries in events, no
    /// network mounts, the sandbox's default user, the
    /// [`DEFAULT_WATCH_TIMEOUT`] deadline and the configured request
    /// timeout. The same as [`WatchOptions::default`].
    ///
    /// # Returns
    ///
    /// The options.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
```

In `src/sandbox.rs` (`impl FileUrlOptions`):

```rust
    /// The default options: the sandbox's default user and no expiry.
    /// The same as [`FileUrlOptions::default`].
    ///
    /// # Returns
    ///
    /// The options.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
```

Before writing each doc sentence, read the type's `Default` impl (or derive) and make the sentence say exactly what it sets after Tasks 4 to 9; the wording above assumes the "API after M4" defaults.

- [ ] **Step 4: Mark `CommandEvent` and `Signal` `#[non_exhaustive]`**

envd's `process.ProcessEvent` and `process.Signal` are protobuf enums that can grow (the SDK already maps more signals than `SIGTERM`/`SIGKILL` in other versions), so both may gain variants. In `src/commands.rs`, add the attribute and a note to each doc comment:

```rust
/// A signal to send to a process (`process.Signal`).
///
/// `#[non_exhaustive]`: envd may support more signals later, so a `match`
/// outside this crate needs a `_` arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Signal {
```

```rust
/// One event from a running command.
///
/// `#[non_exhaustive]`: envd may add event kinds, so a `match` outside this
/// crate needs a catch-all arm (`_` or `other => ...`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CommandEvent {
```

Matches inside the crate (`CommandHandle::wait`, `src/commands.rs:686-697`) stay exhaustive: `#[non_exhaustive]` only applies to other crates. Matches outside must gain an arm:

`src/commands.rs`, the `Commands::start` example (doctests are another crate):

```rust
    /// while let Some(event) = handle.next_event().await? {
    ///     match event {
    ///         CommandEvent::Stdout(chunk) => print!("{}", String::from_utf8_lossy(&chunk)),
    ///         CommandEvent::Stderr(chunk) => eprint!("{}", String::from_utf8_lossy(&chunk)),
    ///         CommandEvent::Exit(exit) => println!("exit code {}", exit.exit_code),
    ///         // `CommandEvent` is `#[non_exhaustive]`.
    ///         _ => {}
    ///     }
    /// }
```

`tests/live_commands.rs:55-65`, in `clm_0034_start_streams_chunks_then_exit`:

```rust
        match event {
            CommandEvent::Stdout(chunk) => {
                first_stdout_at.get_or_insert(now);
                stdout.extend_from_slice(&chunk);
            }
            CommandEvent::Stderr(chunk) => {
                first_stderr_at.get_or_insert(now);
                stderr.extend_from_slice(&chunk);
            }
            CommandEvent::Exit(command_exit) => exit = Some((now, command_exit)),
            // `CommandEvent` is `#[non_exhaustive]`, so a catch-all arm is required.
            other => panic!("unexpected command event {other:?}"),
        }
```

Then find any other exhaustive match outside `src/`:

Run: `grep -rn "CommandEvent::\|Signal::" tests README.md src | grep -v "^src/commands.rs:[0-9]*:\s*[A-Z]"`
and build: `cargo test --no-run --all-targets && cargo test --doc`.
Expected: a compile error (`non-exhaustive patterns`) for each match still missing an arm; add `other => panic!("unexpected ... {other:?}")` in tests and `_ => {}` in examples. `matches!` uses (`tests/envd_api.rs:167-169`, `tests/envd_pty.rs:135`) need nothing.

- [ ] **Step 5: Add the lints table**

In `Cargo.toml`, after the `[lib]` table:

```toml
# Every public item has rustdoc (CLM-0120) and a Debug impl (CLM-0179).
# CI's `clippy -D warnings` turns these warnings into errors.
[lints.rust]
missing_docs = "warn"
missing_debug_implementations = "warn"
```

Run: `just lint`
Expected: PASS with no new warnings (the plan session found none on `origin/main`; Tasks 1 to 11 document their new items). rustc skips `missing_docs` when building a test harness, so the integration tests need no `//!` docs. If a doctest or test target does warn, fix the item rather than allowing the lint.

- [ ] **Step 6: Write `docs/api-review.md`**

Create it with this content. Before committing, check every row against the source: the "Traits" column comes from the derive lists on `origin/main` (plan session, 2026-10-28) plus Tasks 2, 5 and 7; correct any row that differs.

````markdown
# API review (milestone 4)

A review of every public item of `e2b-client` against the
[Rust API Guidelines](https://rust-lang.github.io/api-guidelines/checklist.html),
done in M4 (plan `docs/superpowers/plans/2026-10-28-milestone-4-api-stability.md`,
Task 12), before the 0.2.0 release. It backs CLM-0119 (`#[non_exhaustive]`
where types may grow), CLM-0179 (`Send`/`Sync`/`Debug`, enforced by
`tests/api_traits.rs`) and CLM-0180 (this record).

## Rules applied

| Guideline | Rule in this crate | Enforced by |
|---|---|---|
| C-CASE | `snake_case` functions, `PascalCase` types, `SCREAMING_SNAKE_CASE` constants | rustc lints |
| C-CONV | `as_*` borrow or copy cheaply (`ConnectCode::as_wire`), `from_*` construct (`ConnectCode::from_wire`, `from_plain_http_status`), `parse_*`/`FromStr` parse (`EnvdVersion`) | review |
| C-GETTER | Field accessors have no `get_` prefix (`Sandbox::sandbox_id`, `Volume::token`, `CommandHandle::pid`). `get_` is kept only on **remote operations** that mirror SDK names: `Client::{get_sandbox_info, get_sandbox_metrics, get_volume}`, `Sandbox::{get_info, get_metrics, get_host}`, `Filesystem::get_info`. They make a request (or, for `get_host`, mirror the SDK's `get_host`), so they are not getters in the guideline's sense, and the SDK names help porting | review |
| C-CTOR, C-COMMON-TRAITS | Every options or parameters type has `new()` and `Default`, and they agree; options derive `Debug, Clone, PartialEq, Eq` (plus `Copy` when every field is `Copy`) | `tests/api_traits.rs` |
| C-BUILDER | Options are by-value builders (`fn x(self, ..) -> Self`) with `#[must_use]`; fields are private, so a setter can validate or normalise (zero timeouts, Task 4) | review; `clippy` |
| C-DEBUG | Every public type implements `Debug`; types holding secrets print `[REDACTED]` or leave the field out (CLM-0122) | `missing_debug_implementations`; `tests/api_traits.rs` |
| C-SEND-SYNC | Clients, service handles, options, results and errors are `Send + Sync`; stream handles are `Send` | `tests/api_traits.rs` |
| C-GOOD-ERR | One `Error` enum, `#[non_exhaustive]`, struct variants `#[non_exhaustive]`, `Send + Sync + 'static`, with classifiers instead of exception classes (decision 1) | `tests/api_traits.rs`; `tests/error_taxonomy.rs` |
| C-NON-EXHAUSTIVE / C-STRUCT-PRIVATE | Options and handles have private fields. Result types have public fields and are `#[non_exhaustive]` (callers read them, the API may add fields). Enums that mirror an API or protobuf enum that can grow are `#[non_exhaustive]` | review |
| C-STABLE (public dependencies) | No reqwest type is public (`TransportError`); the public dependencies are listed in the README's "Versioning" section | review; CLM-0170 |
| C-FAILURE | Every fallible public function documents `# Errors` | `clippy::missing_errors_doc` (none found) |
| C-EXAMPLE | Every public type and every public async method has an `# Examples` section | `tests/doc_examples.rs` (Task 14) |

## Types

"NE" is `#[non_exhaustive]`. "Fields" is how the type is built: **private**
(constructors and setters), **public** (read the fields), or **variants**.

| Item | Kind | Traits | Send/Sync | Fields | NE | Decision |
|---|---|---|---|---|---|---|
| `Client` | struct | Debug, Clone | both | private | no | Cheap to clone (an `Arc`); built with `new`/`from_env` |
| `ConnectionConfig` | struct | Clone, Debug (manual, redacts the key) | both | private | no | Built by `ConnectionConfigBuilder`; accessors without `get_` |
| `ConnectionConfigBuilder` | struct | Default, Debug (manual) | both | private | no | Consuming builder ending in `build()` |
| `Sandbox` | struct | Clone, Debug (manual, redacts tokens) | both | private | no | Handle; accessors without `get_`; remote calls keep SDK names |
| `Commands` | struct | Debug, Clone | both | private | no | Service handle from `Sandbox::commands` |
| `CommandHandle` | struct | Debug | Send | private | no | Stream handle; `next_event(&mut self)` |
| `CommandOptions` | struct | Debug, Clone, PartialEq, Eq, Default | both | private | no | `new()` added in M4 |
| `CommandConnectOptions` | struct | Debug, Clone, PartialEq, Eq, Default | both | private | no | New in M4 (#54); default deadline 60 s, as the SDK |
| `CommandEvent` | enum | Debug, Clone, PartialEq, Eq | both | variants | **yes (M4)** | envd's `ProcessEvent` can grow |
| `CommandExit` | struct | Debug, Clone, PartialEq, Eq | both | public | yes | Result type |
| `CommandResult` | struct | Debug, Clone, PartialEq, Eq | both | public | yes | Result type |
| `ProcessInfo` | struct | Debug, Clone, PartialEq, Eq | both | public | yes | Result type |
| `Signal` | enum | Debug, Clone, Copy, PartialEq, Eq, Hash | both | variants | **yes (M4)** | envd's `process.Signal` can grow |
| `Pty` | struct | Debug, Clone | both | private | no | Service handle |
| `PtyHandle` | struct | Debug | Send | private | no | Stream handle |
| `PtyOptions` | struct | Debug, Clone, PartialEq, Eq, Default | both | private | no | `new()` added in M4; no deadline by default (decision 6) |
| `PtyConnectOptions` | struct | Debug, Clone, PartialEq, Eq, Default | both | private | no | New in M4 (#54); no deadline by default |
| `PtySize` | struct | Debug, Clone, Copy, PartialEq, Eq, Hash | both | public (`cols`, `rows`) | **no, on purpose** | A terminal size is two numbers and will not grow; public fields let callers destructure it. Built with `PtySize::new` or a literal |
| `PtyEvent` | enum | Debug, Clone, PartialEq, Eq | both | variants | yes | Can grow |
| `StreamInputOptions` | struct | Debug, Clone, Copy, PartialEq, Eq, Default | both | private | no | `new()` added in M4 |
| `InputStream` | struct | Debug | Send | private | no | Stream handle; `close(self)` consumes it |
| `Filesystem` | struct | Debug, Clone | both | private | no | Service handle; every call takes an options reference (decision 3) |
| `FileOptions` | struct | Debug, Clone, PartialEq, Eq, Default | both | private | no | New in M4: user and request timeout |
| `ListOptions` | struct | Debug, Clone, PartialEq, Eq, Default | both | private | no | New in M4: depth (default 1), user, request timeout |
| `WriteOptions` | struct | Debug, Clone, PartialEq, Eq, Default | both | private | no | `new()` added in M4; `max_concurrency` (#53), `request_timeout` |
| `WriteEntry` | struct | Debug, Clone, PartialEq, Eq | both | private | no | Built with `WriteEntry::new(path, data)` |
| `WriteInfo` | struct | Debug, Clone, PartialEq, Eq | both | public | yes | Result type |
| `WatchOptions` | struct | Debug, Clone, PartialEq, Eq, Default | both | private | no | `new()` added in M4 |
| `WatchHandle` | struct | Debug | Send | private | no | Stream handle |
| `FileStream` | struct | Debug | Send | private | no | Stream handle |
| `EntryInfo` | struct | Debug, Clone, PartialEq, Eq | both | public | yes | Result type; `file_type` reads `is_symlink` (#30) |
| `FileType` | enum | Debug, Clone, Copy, PartialEq, Eq, Hash | both | variants | yes | envd's `FileType` can grow |
| `FilesystemEvent` | struct | Debug, Clone, PartialEq, Eq | both | public | yes | Result type |
| `FilesystemEventType` | enum | Debug, Clone, Copy, PartialEq, Eq, Hash | both | variants | yes | Can grow |
| `FileUrlOptions` | struct | Debug, Clone, PartialEq, Eq, Default | both | private | no | `new()` added in M4 |
| `CreateSandboxParams` | struct | Debug, Clone, PartialEq, Eq, Default | both | private | no | `new(template)` takes the template, so it is not `Default::default()`; `Default` uses `DEFAULT_TEMPLATE` |
| `ListSandboxesParams` | struct | Debug, Clone, PartialEq, Eq, Default | both | private | no | |
| `SandboxInfo` | struct | Debug, Clone, PartialEq, Eq, Deserialize | both | public | yes | Result type |
| `SandboxPage` | struct | Debug, Clone, PartialEq, Eq | both | public | yes | Result type |
| `SandboxState` | enum | Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize | both | variants | yes | Has `Unknown` for values added later |
| `SortOrder` | enum | Debug, Clone, Copy, PartialEq, Eq, Hash | both | variants | yes | |
| `ConnectOptions` | struct | Debug, Clone, Copy, PartialEq, Eq, Default | both | private | no | Sandbox connect (timeout, resume mode); the only form of `connect_sandbox` since Task 6 |
| `PauseOptions` | struct | Debug, Clone, Copy, PartialEq, Eq, Default | both | private | no | The only form of `pause_sandbox`/`Sandbox::pause` since Task 6 |
| `Lifecycle` | struct | Debug, Clone, Copy, PartialEq, Eq, Default | both | private | no | |
| `LifecycleInfo` | struct | Debug, Clone, Copy, PartialEq, Eq, Deserialize | both | public | yes | Result type |
| `OnTimeout` | enum | Debug, Clone, Copy, PartialEq, Eq, Hash | both | variants | yes | `Pause { mode }` is built with `OnTimeout::pause`/`pause_with` |
| `OnResume` | enum | Debug, Clone, Copy, PartialEq, Eq, Hash | both | variants | yes | |
| `SnapshotMode` | enum | Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize | both | variants | yes | |
| `TimeoutAction` | enum | Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize | both | variants | yes | Has `Unknown` |
| `MetricsParams` | struct | Debug, Clone, Copy, PartialEq, Eq, Default | both | private | no | |
| `SandboxMetrics` | struct | Debug, Clone, PartialEq, Deserialize | both | public | yes | No `Eq`: `cpu_used_pct` is an `f64` |
| `ForkOptions` | struct | Debug, Clone, Copy, PartialEq, Eq, Default | both | private | no | |
| `NetworkConfig` | struct | Debug, Clone, PartialEq, Eq, Default, Serialize | both | private | no | |
| `NetworkUpdate` | struct | Debug, Clone, PartialEq, Eq, Default, Serialize | both | private | no | |
| `NetworkRule` | struct | Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize | both | private | no | Header values are secrets in `Debug` |
| `NetworkInfo` | struct | Debug, Clone, PartialEq, Eq, Default, Deserialize | both | public | yes | Result type |
| `SnapshotOptions` | struct | Debug, Clone, PartialEq, Eq, Default | both | private | no | |
| `SnapshotInfo` | struct | Debug, Clone, PartialEq, Eq, Deserialize | both | public | yes | Result type |
| `ListSnapshotsParams` | struct | Debug, Clone, PartialEq, Eq, Default | both | private | no | |
| `SnapshotPage` | struct | Debug, Clone, PartialEq, Eq | both | public | yes | Result type |
| `Volume` | struct | Clone, Debug (manual, leaves the token out) | both | private | no | Accessors without `get_` |
| `VolumeInfo` | struct | Debug, Clone, PartialEq, Eq, Hash, Deserialize | both | public | yes | Result type |
| `VolumeMount` | struct | Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize | both | public | yes | Built with `VolumeMount::new` |
| `EnvdVersion` | struct | Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Display, FromStr | both | private | no | Ordered, so version gates compare |
| `Error` | enum | Debug, Display, Error | both | variants | yes; struct variants too (M4) | Classifiers (decision 1); `TransportError` hides reqwest |
| `TransportError` | struct | Debug, Display, Error | both | private | no | New in M4: `is_timeout`, `is_connect` |
| `ConnectCode` | enum | Debug, Clone, Copy, PartialEq, Eq, Hash, Display | both | variants | **no, on purpose** | The Connect protocol's 16 codes are the closed set of gRPC status codes; an exhaustive `match` is useful and no code will be added |
| `Result<T>` | type alias | | | | | `std::result::Result<T, Error>` |
| `secrecy` | re-export | | | | | `pub use secrecy;` (decision 4) |

Constants (`DEFAULT_*`, `ENVD_PORT`, `MCP_PORT`, `MAX_FORK_COUNT`,
`ALL_TRAFFIC`, `DEFAULT_TEMPLATE`, `DEFAULT_MCP_TEMPLATE`,
`DEFAULT_UPLOAD_CONCURRENCY`) are `SCREAMING_SNAKE_CASE` and documented.

## Method shape (decision 3)

- One method per operation. Calls that can grow take a trailing `&XOptions`;
  `connect_sandbox_with`, `pause_sandbox_with` and `Sandbox::pause_with` were
  removed in Task 6.
- `Client` and `Sandbox` both offer the sandbox operations (`Client` by id,
  `Sandbox` on itself), as the SDK's class and instance methods do.
- Booleans that report "did it exist" (`kill`, `delete_snapshot`,
  `destroy_volume`, `make_dir`) return `Result<bool>` as the SDK does.

## Known limits

- **Per-call request timeouts** exist only on calls that take an options
  struct reaching envd. The REST calls (`Client::*`, the `Sandbox` lifecycle
  calls) and the pid-only envd calls (`send_stdin`, `close_stdin`,
  `send_signal`, `kill`, `resize`, `Commands::list`) use the configured
  request timeout. Adding options to them later is a breaking change, so it
  is planned only with a minor release.
- **Result types are `#[non_exhaustive]`**, so code outside the crate cannot
  construct them; tests that need one run against a mock server (as this
  crate's own tests do with wiremock).
- **`Error::Decode` holds a `serde_json::Error`**; serde_json is 1.x, so this
  is a public dependency the README lists.
- **`get_` names** on remote operations (above) stay, for SDK parity.
````

- [ ] **Step 7: Run the gates**

Run: `cargo test --test api_traits`, then `just lint`, `just test`, `just doc`.
Expected: all PASS; `api_traits` reports 5 passed.

- [ ] **Step 8: Changelog and commit**

`changelog/<ts>-m4-task-12-api-review.md`: what was reviewed, the two `#[non_exhaustive]` additions and the two deliberate exceptions, the six new `new()` constructors, the lints, and the breaking changes ("a `match` on `CommandEvent` or `Signal` outside the crate needs a `_` arm").

```bash
git add Cargo.toml src/ tests/api_traits.rs tests/live_commands.rs docs/api-review.md changelog/
git commit -m "feat: API review: non_exhaustive events and signals, new() on every options type, lints (#<issue>)"
```


---

### Task 13: MSRV 1.89 and the semver policy (CLM-0121)

**Files:**
- Modify: `Cargo.toml` (`rust-version`)
- Modify: `.github/workflows/ci.yml` (job `msrv`)
- Modify: `README.md` (section "Versioning", and the MSRV in the dependency snippet's intro)
- Modify: `CONTRIBUTING.md` (tooling table and a "Versioning" pointer)
- Modify: `docs/api-review.md` (link the policy), if Task 12 has landed; otherwise Task 12 adds the link
- Create: `changelog/<ts>-m4-task-13-msrv-and-semver.md`

- [ ] **Step 1: Confirm the floor before pinning it**

Run: `cargo +1.89 check --all-targets --locked`
Expected: `Finished`. (Install the toolchain with `rustup toolchain install 1.89 --profile minimal` if `rustup toolchain list` lacks it.) Then confirm 1.88 cannot build it, so 1.89 is the real floor:

Run: `cargo +1.88 check --all-targets --locked 2>&1 | grep -m1 "requires rustc"`
Expected: a line naming `uuid` (`rust-version` 1.89, plan "Verified facts"). If 1.88 builds, keep 1.89 anyway (uuid's declared floor) and say so in the changelog.

- [ ] **Step 2: Pin it**

In `Cargo.toml`, under `edition`:

```toml
edition = "2024"
# Minimum supported Rust version (CLM-0121): uuid 1.29 needs 1.89. CI checks
# it (`msrv` job); raising it is a minor release (README, "Versioning").
rust-version = "1.89"
```

Run: `cargo +1.89 check --all-targets --locked && cargo check --all-targets`
Expected: both `Finished`, no warnings.

- [ ] **Step 3: Add the CI job**

In `.github/workflows/ci.yml`, after the `check` job:

```yaml
  msrv:
    # The minimum supported Rust version in Cargo.toml (`rust-version`).
    # `--locked` checks the committed Cargo.lock, which is what users of a
    # git dependency get.
    name: MSRV (Rust 1.89)
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@master
        with:
          toolchain: "1.89"
      - uses: Swatinem/rust-cache@v2
      - name: Check on the MSRV
        run: cargo check --all-targets --locked
```

The pinned toolchain must equal `rust-version`; a later MSRV change edits both lines together.

- [ ] **Step 4: Write the policy**

In `README.md`, after the "How it talks to E2B" section and before "Verification status", add:

```markdown
## Versioning

- **Semver.** `e2b-client` follows [Semantic Versioning](https://semver.org).
  Before 1.0, a minor release (0.2 to 0.3) may break the API and a patch
  release (0.2.0 to 0.2.1) never does. Each release's breaking changes are
  listed in its changelog entry.
- **Minimum supported Rust version: 1.89.** It is `rust-version` in
  `Cargo.toml` and is checked in CI. Raising it is a minor release, never a
  patch release.
- **Public dependencies.** These crates' types or traits appear in the API,
  so a breaking release of one of them (a new major version, or a new minor
  version before 1.0) is a breaking release of this crate too:

  | Crate | Where it appears |
  |---|---|
  | `bytes` 1 | `Bytes` in `CommandEvent`, `PtyEvent`, `FileStream::next_chunk`, `WriteEntry::new`, `InputStream::send`, `Filesystem::read` |
  | `chrono` 0.4 | `DateTime<Utc>` in `SandboxInfo`, `EntryInfo`, `SandboxMetrics`, `MetricsParams`, `ListSandboxesParams::started_after` |
  | `secrecy` 0.10 | `SecretString` from `Sandbox::traffic_access_token`, `Sandbox::mcp_token` and `Volume::token`; re-exported as `e2b_client::secrecy`, so use that path to name the matching version |
  | `serde` 1 | `Serialize`/`Deserialize` impls on `SandboxState`, `SnapshotMode`, `NetworkConfig`, `NetworkUpdate`, `NetworkRule`, `VolumeMount` and the result types |
  | `serde_json` 1 | `Error::Decode`, `CreateSandboxParams::mcp` (`serde_json::Map`) |
  | `tokio` 1 | `tokio::io::AsyncRead` in `Filesystem::write_stream`; every future needs a tokio runtime |
  | `url` 2 | `ConnectionConfig::api_url` |

  Every other dependency (reqwest, base64, sha2, async-compression, ...) is
  private: upgrading it is not a breaking change. That is why
  `Error::Transport` holds this crate's `TransportError` rather than a
  `reqwest::Error`.
```

Before committing, confirm each "Where it appears" cell against `src/lib.rs`'s re-exports and the signatures (`grep -n "Bytes\|DateTime\|SecretString\|Serialize\|Map<\|AsyncRead\|Url" src/*.rs | grep "pub "`), and drop or add entries to match.

In `README.md`'s install paragraph ("It is not on crates.io yet; depend on it from git:"), change the sentence to: "It is not on crates.io yet, and needs Rust 1.89 or newer; depend on it from git:".

In `CONTRIBUTING.md`, add a row to the tooling table and a sentence under it:

```markdown
| `cargo +1.89 check --all-targets --locked` | Check the minimum supported Rust version (CI job `msrv`) |
```

```markdown
The crate's semver, MSRV and public-dependency policy is in the README's
"Versioning" section. A change that adds a public dependency, changes a
public signature or raises `rust-version` must say so in its changelog entry.
```

If `docs/api-review.md` exists, change its C-STABLE row's "Enforced by" cell to "review; CLM-0170; README 'Versioning'".

- [ ] **Step 5: Run the gates**

Run: `just lint`, `just test`, `just doc`, and `cargo +1.89 check --all-targets --locked`.
Expected: all PASS (the README is a doctest, so a broken code fence fails `just test`).

- [ ] **Step 6: Changelog and commit**

`changelog/<ts>-m4-task-13-msrv-and-semver.md`: the MSRV and how it was found (Step 1's output), the CI job, the policy text, and the public-dependency list.

```bash
git add Cargo.toml .github/workflows/ci.yml README.md CONTRIBUTING.md docs/api-review.md changelog/
git commit -m "build: pin MSRV 1.89 with a CI check, and state the semver policy (#<issue>)"
```

After pushing, confirm the new job ran on the PR: `gh pr checks <PR>` lists `MSRV (Rust 1.89)` as `pass`.


---

### Task 14: An example on every public type and async method (CLM-0120)

**Files:**
- Create: `tests/doc_examples.rs`
- Modify: `src/client.rs`, `src/commands.rs`, `src/config.rs`, `src/envd/mod.rs`, `src/error.rs`, `src/filesystem.rs`, `src/lifecycle.rs`, `src/metrics.rs`, `src/models.rs`, `src/network.rs`, `src/pty.rs`, `src/sandbox.rs`, `src/snapshots.rs`, `src/stream_input.rs`, `src/volumes.rs` (examples only)
- Modify: `docs/api-review.md` (coverage numbers)
- Create: `changelog/<ts>-m4-task-14-rustdoc-examples.md`

"Non-trivial use" in CLM-0120 is made concrete: **every public type re-exported from `lib.rs`, and every `pub async fn`**, has an `# Examples` section. Builder setters and accessors are covered by their type's example. The test makes the rule permanent.

- [ ] **Step 1: Write the failing test**

Create `tests/doc_examples.rs`:

```rust
//! Every public type and every public async method has an `# Examples`
//! section in its rustdoc (CLM-0120).
//!
//! The test reads the crate's source, so it needs no network. "Public type"
//! means a `pub struct` or `pub enum` at the top level of a file whose name
//! `src/lib.rs` re-exports; "public async method" means any `pub async fn`
//! (crate-private helpers are `pub(crate)` and are skipped).

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// The names `lib.rs` re-exports with `pub use <path>;` or
/// `pub use <path>::{A, B};`.
///
/// # Arguments
///
/// * `lib` - the text of `src/lib.rs`.
///
/// # Returns
///
/// The re-exported names (modules too, which never match a type).
fn reexported_names(lib: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut rest = lib;
    while let Some(start) = rest.find("pub use ") {
        let after = &rest[start + "pub use ".len()..];
        let end = after.find(';').expect("a `pub use` ends with `;`");
        let path = &after[..end];
        let items = match (path.find('{'), path.rfind('}')) {
            (Some(open), Some(close)) => &path[open + 1..close],
            _ => path
                .rsplit("::")
                .next()
                .expect("`rsplit` yields at least one item"),
        };
        names.extend(
            items
                .split(',')
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .map(str::to_owned),
        );
        rest = &after[end..];
    }
    names
}

/// Every `.rs` file under `dir`, recursively.
///
/// # Arguments
///
/// * `dir` - the directory to walk.
/// * `out` - where the paths are collected.
fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("read a source directory") {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

/// The kind and name of the item a line declares, if it declares a public
/// type at the top level or a public async function at any depth.
///
/// # Arguments
///
/// * `line` - one source line.
///
/// # Returns
///
/// `Some(("type" | "async fn", name))`, or `None`.
fn declared_item(line: &str) -> Option<(&'static str, &str)> {
    let trimmed = line.trim_start();
    let (kind, rest) = if let Some(rest) = line.strip_prefix("pub struct ") {
        ("type", rest)
    } else if let Some(rest) = line.strip_prefix("pub enum ") {
        ("type", rest)
    } else if let Some(rest) = trimmed.strip_prefix("pub async fn ") {
        ("async fn", rest)
    } else {
        return None;
    };
    let end = rest
        .find(|c: char| !(c.is_alphanumeric() || c == '_'))
        .unwrap_or(rest.len());
    Some((kind, &rest[..end]))
}

/// Whether the doc comment directly above line `item` has `# Examples`.
/// Attributes (`#[...]`) between the comment and the item are skipped.
///
/// # Arguments
///
/// * `lines` - the file's lines.
/// * `item` - the index of the declaring line.
fn has_examples(lines: &[&str], item: usize) -> bool {
    lines[..item]
        .iter()
        .rev()
        .map(|line| line.trim())
        .take_while(|line| line.starts_with("///") || line.starts_with("#["))
        .any(|line| line == "/// # Examples")
}

#[test]
fn reexported_names_reads_single_and_grouped_uses() {
    // Arrange
    let lib = "pub use client::Client;\npub use error::{ConnectCode, Error,\n    Result};\npub use secrecy;\n";

    // Act
    let names = reexported_names(lib);

    // Assert
    let expected: BTreeSet<String> = ["Client", "ConnectCode", "Error", "Result", "secrecy"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    assert_eq!(names, expected);
}

#[test]
fn has_examples_looks_through_attributes_only() {
    // Arrange
    let lines = [
        "/// A thing.",
        "///",
        "/// # Examples",
        "#[derive(Debug)]",
        "pub struct Thing;",
        "",
        "/// No example.",
        "pub struct Other;",
    ];

    // Act and Assert
    assert!(has_examples(&lines, 4));
    assert!(!has_examples(&lines, 7));
}

#[test]
fn every_public_type_and_async_method_has_an_example() {
    // Arrange
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let lib = fs::read_to_string(src.join("lib.rs")).expect("read src/lib.rs");
    let exported = reexported_names(&lib);
    let mut files = Vec::new();
    rust_files(&src, &mut files);
    files.sort();

    // Act
    let mut missing = Vec::new();
    for file in &files {
        let text = fs::read_to_string(file).expect("read a source file");
        let lines: Vec<&str> = text.lines().collect();
        for (index, line) in lines.iter().enumerate() {
            let Some((kind, name)) = declared_item(line) else {
                continue;
            };
            let checked = kind == "async fn" || exported.contains(name);
            if checked && !has_examples(&lines, index) {
                let shown = file.strip_prefix(&src).unwrap_or(file);
                missing.push(format!("src/{}:{} {kind} {name}", shown.display(), index + 1));
            }
        }
    }

    // Assert
    assert!(
        missing.is_empty(),
        "{} item(s) lack an `# Examples` section:\n{}",
        missing.len(),
        missing.join("\n")
    );
}
```

- [ ] **Step 2: Run it to see it fail**

Run: `cargo test --test doc_examples`
Expected: the two helper tests pass, and `every_public_type_and_async_method_has_an_example` FAILS, listing the items Step 3 covers (on `origin/main` before M4 there were 101; after Tasks 1 to 13 the list differs by the methods those tasks removed or added). Save the count for the changelog.

- [ ] **Step 3: Add the examples**

Each block below is inserted at the **end of the item's existing doc comment** (after its `# Errors` section, before any `#[...]` attribute). All network examples are `no_run`, take a `Sandbox` or `Client` through a hidden `demo` function, and use the M4 signatures. Pure-value examples run. An item that already has an example after Tasks 1 to 13 is skipped; a new M4 type without one (`TransportError`, `FileOptions`, `ListOptions`, `CommandConnectOptions`, `PtyConnectOptions`) gets the block given here.

**`src/client.rs`**

`Client::create_sandbox`:

```rust
    /// # Examples
    ///
    /// ```no_run
    /// use std::time::Duration;
    ///
    /// use e2b_client::{Client, CreateSandboxParams};
    ///
    /// # async fn demo(client: Client) -> e2b_client::Result<()> {
    /// let params = CreateSandboxParams::new("base")
    ///     .timeout(Duration::from_secs(120))
    ///     .metadata("purpose", "docs");
    /// let sandbox = client.create_sandbox(&params).await?;
    /// println!("{}", sandbox.sandbox_id());
    /// # Ok(())
    /// # }
    /// ```
```

`Client::connect_sandbox`:

```rust
    /// # Examples
    ///
    /// ```no_run
    /// use std::time::Duration;
    ///
    /// use e2b_client::{Client, ConnectOptions, OnResume};
    ///
    /// # async fn demo(client: Client, sandbox_id: &str) -> e2b_client::Result<()> {
    /// // Resume a paused sandbox from its memory snapshot, for ten minutes.
    /// let options = ConnectOptions::new().timeout(Duration::from_secs(600));
    /// let sandbox = client.connect_sandbox(sandbox_id, &options).await?;
    /// // Or boot it cold from its filesystem.
    /// let rebooted = client
    ///     .connect_sandbox(sandbox_id, &ConnectOptions::new().on_resume(OnResume::Reboot))
    ///     .await?;
    /// # let _ = (sandbox, rebooted);
    /// # Ok(())
    /// # }
    /// ```
```

`Client::get_sandbox_info`:

```rust
    /// # Examples
    ///
    /// ```no_run
    /// # async fn demo(client: e2b_client::Client, sandbox_id: &str) -> e2b_client::Result<()> {
    /// let info = client.get_sandbox_info(sandbox_id).await?;
    /// println!("{:?} until {}", info.state, info.end_at);
    /// # Ok(())
    /// # }
    /// ```
```

`Client::kill_sandbox`:

```rust
    /// # Examples
    ///
    /// ```no_run
    /// # async fn demo(client: e2b_client::Client, sandbox_id: &str) -> e2b_client::Result<()> {
    /// if !client.kill_sandbox(sandbox_id).await? {
    ///     println!("{sandbox_id} was already gone");
    /// }
    /// # Ok(())
    /// # }
    /// ```
```

`Client::pause_sandbox`:

```rust
    /// # Examples
    ///
    /// ```no_run
    /// use e2b_client::{Client, PauseOptions, SnapshotMode};
    ///
    /// # async fn demo(client: Client, sandbox_id: &str) -> e2b_client::Result<()> {
    /// // A filesystem-only snapshot: the next resume boots cold.
    /// let options = PauseOptions::new().mode(SnapshotMode::Filesystem);
    /// let paused = client.pause_sandbox(sandbox_id, &options).await?;
    /// println!("paused now: {paused}");
    /// # Ok(())
    /// # }
    /// ```
```

`Client::set_sandbox_timeout`:

```rust
    /// # Examples
    ///
    /// ```no_run
    /// use std::time::Duration;
    ///
    /// # async fn demo(client: e2b_client::Client, sandbox_id: &str) -> e2b_client::Result<()> {
    /// client.set_sandbox_timeout(sandbox_id, Duration::from_secs(900)).await?;
    /// # Ok(())
    /// # }
    /// ```
```

`Client::update_sandbox_network`:

```rust
    /// # Examples
    ///
    /// ```no_run
    /// use e2b_client::{Client, NetworkUpdate};
    ///
    /// # async fn demo(client: Client, sandbox_id: &str) -> e2b_client::Result<()> {
    /// // Cut the sandbox off from the internet without restarting it.
    /// let update = NetworkUpdate::new().allow_internet_access(false);
    /// client.update_sandbox_network(sandbox_id, &update).await?;
    /// # Ok(())
    /// # }
    /// ```
```

`Client::refresh_sandbox`:

```rust
    /// # Examples
    ///
    /// ```no_run
    /// use std::time::Duration;
    ///
    /// # async fn demo(client: e2b_client::Client, sandbox_id: &str) -> e2b_client::Result<()> {
    /// client.refresh_sandbox(sandbox_id, Some(Duration::from_secs(60))).await?;
    /// # Ok(())
    /// # }
    /// ```
```

`Client::delete_snapshot`:

```rust
    /// # Examples
    ///
    /// ```no_run
    /// # async fn demo(client: e2b_client::Client) -> e2b_client::Result<()> {
    /// let deleted = client.delete_snapshot("team/my-snapshot:v1").await?;
    /// println!("deleted: {deleted}");
    /// # Ok(())
    /// # }
    /// ```
```

`Client::get_volume`:

```rust
    /// # Examples
    ///
    /// ```no_run
    /// # async fn demo(client: e2b_client::Client, volume_id: &str) -> e2b_client::Result<()> {
    /// let volume = client.get_volume(volume_id).await?;
    /// println!("{} is {}", volume.volume_id(), volume.name());
    /// # Ok(())
    /// # }
    /// ```
```

`Client::list_volumes`:

```rust
    /// # Examples
    ///
    /// ```no_run
    /// # async fn demo(client: e2b_client::Client) -> e2b_client::Result<()> {
    /// for volume in client.list_volumes().await? {
    ///     println!("{} {}", volume.volume_id, volume.name);
    /// }
    /// # Ok(())
    /// # }
    /// ```
```

`Client::destroy_volume`:

```rust
    /// # Examples
    ///
    /// ```no_run
    /// # async fn demo(client: e2b_client::Client, volume_id: &str) -> e2b_client::Result<()> {
    /// if !client.destroy_volume(volume_id).await? {
    ///     println!("no volume {volume_id}");
    /// }
    /// # Ok(())
    /// # }
    /// ```
```

**`src/commands.rs`**

`Signal` (type):

```rust
/// # Examples
///
/// ```no_run
/// use e2b_client::{CommandOptions, Sandbox, Signal};
///
/// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
/// let handle = sandbox.commands().start("sleep 600", &CommandOptions::new()).await?;
/// sandbox.commands().send_signal(handle.pid(), Signal::Sigterm).await?;
/// # Ok(())
/// # }
/// ```
```

`CommandEvent` (type):

```rust
/// # Examples
///
/// ```no_run
/// use e2b_client::{CommandEvent, CommandOptions, Sandbox};
///
/// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
/// let mut handle = sandbox.commands().start("make", &CommandOptions::new()).await?;
/// while let Some(event) = handle.next_event().await? {
///     match event {
///         CommandEvent::Stdout(chunk) => print!("{}", String::from_utf8_lossy(&chunk)),
///         CommandEvent::Stderr(chunk) => eprint!("{}", String::from_utf8_lossy(&chunk)),
///         CommandEvent::Exit(exit) => println!("exit {}", exit.exit_code),
///         _ => {}
///     }
/// }
/// # Ok(())
/// # }
/// ```
```

`CommandExit` (type):

```rust
/// # Examples
///
/// ```no_run
/// use e2b_client::{CommandEvent, CommandOptions, Sandbox};
///
/// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
/// let mut handle = sandbox.commands().start("false", &CommandOptions::new()).await?;
/// while let Some(event) = handle.next_event().await? {
///     if let CommandEvent::Exit(exit) = event {
///         println!("code {}, error {:?}", exit.exit_code, exit.error);
///     }
/// }
/// # Ok(())
/// # }
/// ```
```

`CommandResult` (type):

```rust
/// # Examples
///
/// ```no_run
/// # async fn demo(sandbox: e2b_client::Sandbox) -> e2b_client::Result<()> {
/// let result = sandbox.commands().run("ls /", &e2b_client::CommandOptions::new()).await?;
/// if result.success() {
///     print!("{}", result.stdout);
/// } else {
///     eprintln!("exit {}: {}", result.exit_code, result.stderr);
/// }
/// # Ok(())
/// # }
/// ```
```

`ProcessInfo` (type):

```rust
/// # Examples
///
/// ```no_run
/// # async fn demo(sandbox: e2b_client::Sandbox) -> e2b_client::Result<()> {
/// for process in sandbox.commands().list().await? {
///     println!("{} {} {:?}", process.pid, process.cmd, process.args);
/// }
/// # Ok(())
/// # }
/// ```
```

`Commands` (type):

```rust
/// # Examples
///
/// ```no_run
/// # async fn demo(sandbox: e2b_client::Sandbox) -> e2b_client::Result<()> {
/// let commands = sandbox.commands();
/// let result = commands.run("echo hi", &e2b_client::CommandOptions::new()).await?;
/// assert_eq!(result.stdout, "hi\n");
/// # Ok(())
/// # }
/// ```
```

`Commands::run`:

```rust
    /// # Examples
    ///
    /// ```no_run
    /// use std::time::Duration;
    ///
    /// use e2b_client::{CommandOptions, Sandbox};
    ///
    /// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
    /// let options = CommandOptions::new()
    ///     .cwd("/home/user")
    ///     .env("RUST_LOG", "debug")
    ///     .timeout(Some(Duration::from_secs(300)));
    /// let result = sandbox.commands().run("cargo --version", &options).await?;
    /// println!("{}", result.stdout);
    /// # Ok(())
    /// # }
    /// ```
```

`Commands::connect`:

```rust
    /// # Examples
    ///
    /// ```no_run
    /// use e2b_client::{CommandConnectOptions, CommandOptions, Sandbox};
    ///
    /// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
    /// let started = sandbox.commands().start("sleep 5; echo done", &CommandOptions::new()).await?;
    /// // Attach a second handle to the same process, with no deadline.
    /// let attached = sandbox
    ///     .commands()
    ///     .connect(started.pid(), &CommandConnectOptions::new().timeout(None))
    ///     .await?;
    /// assert_eq!(attached.wait().await?.stdout, "done\n");
    /// # Ok(())
    /// # }
    /// ```
```

`Commands::list`:

```rust
    /// # Examples
    ///
    /// ```no_run
    /// # async fn demo(sandbox: e2b_client::Sandbox) -> e2b_client::Result<()> {
    /// let running = sandbox.commands().list().await?;
    /// println!("{} processes", running.len());
    /// # Ok(())
    /// # }
    /// ```
```

`Commands::send_stdin`:

```rust
    /// # Examples
    ///
    /// ```no_run
    /// use e2b_client::{CommandOptions, Sandbox};
    ///
    /// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
    /// let handle = sandbox.commands().start("cat", &CommandOptions::new().stdin(true)).await?;
    /// sandbox.commands().send_stdin(handle.pid(), "hello\n").await?;
    /// sandbox.commands().close_stdin(handle.pid()).await?;
    /// assert_eq!(handle.wait().await?.stdout, "hello\n");
    /// # Ok(())
    /// # }
    /// ```
```

`Commands::close_stdin`:

```rust
    /// # Examples
    ///
    /// ```no_run
    /// use e2b_client::{CommandOptions, Sandbox};
    ///
    /// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
    /// let handle = sandbox.commands().start("wc -c", &CommandOptions::new().stdin(true)).await?;
    /// sandbox.commands().send_stdin(handle.pid(), "abc").await?;
    /// // EOF: `wc` prints its count and exits.
    /// sandbox.commands().close_stdin(handle.pid()).await?;
    /// assert_eq!(handle.wait().await?.stdout.trim(), "3");
    /// # Ok(())
    /// # }
    /// ```
```

`Commands::send_signal`:

```rust
    /// # Examples
    ///
    /// ```no_run
    /// use e2b_client::{CommandOptions, Sandbox, Signal};
    ///
    /// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
    /// let handle = sandbox.commands().start("sleep 600", &CommandOptions::new()).await?;
    /// sandbox.commands().send_signal(handle.pid(), Signal::Sigterm).await?;
    /// # Ok(())
    /// # }
    /// ```
```

`Commands::kill`:

```rust
    /// # Examples
    ///
    /// ```no_run
    /// use e2b_client::{CommandOptions, Sandbox};
    ///
    /// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
    /// let handle = sandbox.commands().start("sleep 600", &CommandOptions::new()).await?;
    /// assert!(sandbox.commands().kill(handle.pid()).await?);
    /// # Ok(())
    /// # }
    /// ```
```

`CommandHandle` (type):

```rust
/// # Examples
///
/// ```no_run
/// # async fn demo(sandbox: e2b_client::Sandbox) -> e2b_client::Result<()> {
/// let handle = sandbox
///     .commands()
///     .start("sleep 1; echo ok", &e2b_client::CommandOptions::new())
///     .await?;
/// println!("pid {}", handle.pid());
/// let result = handle.wait().await?;
/// assert_eq!(result.stdout, "ok\n");
/// # Ok(())
/// # }
/// ```
```

`CommandHandle::next_event`:

```rust
    /// # Examples
    ///
    /// ```no_run
    /// use e2b_client::{CommandEvent, CommandOptions, Sandbox};
    ///
    /// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
    /// let mut handle = sandbox.commands().start("seq 3", &CommandOptions::new()).await?;
    /// while let Some(event) = handle.next_event().await? {
    ///     if let CommandEvent::Stdout(chunk) = event {
    ///         print!("{}", String::from_utf8_lossy(&chunk));
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
```

`CommandHandle::wait`:

```rust
    /// # Examples
    ///
    /// ```no_run
    /// # async fn demo(sandbox: e2b_client::Sandbox) -> e2b_client::Result<()> {
    /// let handle = sandbox
    ///     .commands()
    ///     .start("echo out; echo err >&2", &e2b_client::CommandOptions::new())
    ///     .await?;
    /// let result = handle.wait().await?;
    /// assert_eq!((result.stdout.as_str(), result.stderr.as_str()), ("out\n", "err\n"));
    /// # Ok(())
    /// # }
    /// ```
```

`CommandHandle::send_stdin`:

```rust
    /// # Examples
    ///
    /// ```no_run
    /// use e2b_client::{CommandOptions, Sandbox};
    ///
    /// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
    /// let handle = sandbox.commands().start("head -n1", &CommandOptions::new().stdin(true)).await?;
    /// handle.send_stdin("first line\n").await?;
    /// assert_eq!(handle.wait().await?.stdout, "first line\n");
    /// # Ok(())
    /// # }
    /// ```
```

`CommandHandle::kill`:

```rust
    /// # Examples
    ///
    /// ```no_run
    /// # async fn demo(sandbox: e2b_client::Sandbox) -> e2b_client::Result<()> {
    /// let handle = sandbox
    ///     .commands()
    ///     .start("sleep 600", &e2b_client::CommandOptions::new())
    ///     .await?;
    /// assert!(handle.kill().await?);
    /// # Ok(())
    /// # }
    /// ```
```

`CommandConnectOptions` (type; only if Task 7 left it without one):

```rust
/// # Examples
///
/// ```
/// use std::time::Duration;
///
/// use e2b_client::CommandConnectOptions;
///
/// // The SDK's 60 s deadline is the default; this one has none, and a
/// // 10 s limit on opening the stream.
/// let options = CommandConnectOptions::new()
///     .timeout(None)
///     .request_timeout(Some(Duration::from_secs(10)));
/// # let _ = options;
/// ```
```

**`src/config.rs`**

`ConnectionConfig` (type):

```rust
/// # Examples
///
/// ```
/// use std::time::Duration;
///
/// use e2b_client::ConnectionConfig;
///
/// let config = ConnectionConfig::builder()
///     .api_key("e2b_example_key")
///     .domain("e2b.example.com")
///     .request_timeout(Some(Duration::from_secs(30)))
///     .build()?;
/// assert_eq!(config.domain(), "e2b.example.com");
/// assert_eq!(config.api_url().as_str(), "https://api.e2b.example.com/");
/// # Ok::<(), e2b_client::Error>(())
/// ```
```

(`build()` reads the environment only for unset values; every value used here is set, so the example is deterministic. If `api_url()`'s display differs, correct the assertion to what `just test` reports, since this example runs.)

**`src/envd/mod.rs`**

`EnvdVersion` (type):

```rust
/// # Examples
///
/// ```
/// use e2b_client::EnvdVersion;
///
/// let version: EnvdVersion = "0.5.7".parse()?;
/// assert!(version >= EnvdVersion::new(0, 4, 0));
/// assert_eq!(version.to_string(), "0.5.7");
/// # Ok::<(), e2b_client::Error>(())
/// ```
```

**`src/error.rs`**

`Error` (type):

```rust
/// # Examples
///
/// ```no_run
/// use e2b_client::{Client, Error};
///
/// # async fn demo(client: Client) -> e2b_client::Result<()> {
/// match client.get_sandbox_info("sbx-maybe-gone").await {
///     Ok(info) => println!("{:?}", info.state),
///     Err(error) if error.is_not_found() => println!("gone"),
///     Err(error) if error.is_retryable() => println!("try again later: {error}"),
///     Err(Error::Api { status, message, .. }) => println!("HTTP {status}: {message}"),
///     Err(error) => return Err(error),
/// }
/// # Ok(())
/// # }
/// ```
```

`ConnectCode` (type):

```rust
/// # Examples
///
/// ```
/// use e2b_client::ConnectCode;
///
/// let code = ConnectCode::from_wire("deadline_exceeded").expect("a known code");
/// assert_eq!(code, ConnectCode::DeadlineExceeded);
/// assert_eq!(code.as_wire(), "deadline_exceeded");
/// assert_eq!(ConnectCode::from_plain_http_status(404), ConnectCode::NotFound);
/// ```
```

(Check `from_plain_http_status(404)` against `src/error.rs:268`; it follows connect-es's map, where 404 is `Unimplemented` in some versions. Use the code the function returns, since this example runs.)

`TransportError` (type; only if Task 2 left it without one):

```rust
/// # Examples
///
/// ```no_run
/// use e2b_client::{Client, Error};
///
/// # async fn demo(client: Client) -> e2b_client::Result<()> {
/// if let Err(Error::Transport(transport)) = client.list_volumes().await {
///     if transport.is_connect() {
///         eprintln!("could not reach the API: {transport}");
///     }
/// }
/// # Ok(())
/// # }
/// ```
```

**`src/filesystem.rs`**

`FileType` (type):

```rust
/// # Examples
///
/// ```no_run
/// use e2b_client::{FileType, ListOptions, Sandbox};
///
/// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
/// for entry in sandbox.files().list("/home/user", &ListOptions::new()).await? {
///     match entry.file_type {
///         FileType::Directory => println!("{}/", entry.name),
///         FileType::Symlink => println!("{} -> {:?}", entry.name, entry.symlink_target),
///         _ => println!("{}", entry.name),
///     }
/// }
/// # Ok(())
/// # }
/// ```
```

`EntryInfo` (type):

```rust
/// # Examples
///
/// ```no_run
/// use e2b_client::{FileOptions, Sandbox};
///
/// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
/// let entry = sandbox.files().get_info("/etc/hostname", &FileOptions::new()).await?;
/// println!("{} bytes, {} {}, mode {:o}", entry.size, entry.owner, entry.group, entry.mode);
/// # Ok(())
/// # }
/// ```
```

`WriteInfo` (type):

```rust
/// # Examples
///
/// ```no_run
/// use e2b_client::{Sandbox, WriteOptions};
///
/// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
/// let written = sandbox.files().write("/tmp/a.txt", "alpha", &WriteOptions::new()).await?;
/// println!("wrote {} ({:?})", written.path, written.file_type);
/// # Ok(())
/// # }
/// ```
```

`FilesystemEventType` (type):

```rust
/// # Examples
///
/// ```no_run
/// use e2b_client::{FilesystemEventType, Sandbox, WatchOptions};
///
/// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
/// let mut watch = sandbox.files().watch_dir("/tmp", &WatchOptions::new()).await?;
/// while let Some(event) = watch.next_event().await? {
///     if event.event_type == FilesystemEventType::Create {
///         println!("created {}", event.name);
///     }
/// }
/// # Ok(())
/// # }
/// ```
```

`FilesystemEvent` (type):

```rust
/// # Examples
///
/// ```no_run
/// use e2b_client::{Sandbox, WatchOptions};
///
/// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
/// let options = WatchOptions::new().recursive(true).include_entry(true);
/// let mut watch = sandbox.files().watch_dir("/home/user", &options).await?;
/// if let Some(event) = watch.next_event().await? {
///     println!("{:?} {} {:?}", event.event_type, event.name, event.entry.map(|e| e.size));
/// }
/// # Ok(())
/// # }
/// ```
```

`WatchHandle` (type):

```rust
/// # Examples
///
/// ```no_run
/// use e2b_client::{Sandbox, WatchOptions, WriteOptions};
///
/// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
/// let mut watch = sandbox.files().watch_dir("/tmp", &WatchOptions::new()).await?;
/// sandbox.files().write("/tmp/new.txt", "x", &WriteOptions::new()).await?;
/// let event = watch.next_event().await?.expect("an event before the deadline");
/// println!("{:?} {}", event.event_type, event.name);
/// # Ok(())
/// # }
/// ```
```

`FileStream` (type):

```rust
/// # Examples
///
/// ```no_run
/// use e2b_client::{FileOptions, Sandbox};
///
/// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
/// let mut download = sandbox.files().read_stream("/var/log/big.log", &FileOptions::new()).await?;
/// let mut total = 0;
/// while let Some(chunk) = download.next_chunk().await? {
///     total += chunk.len();
/// }
/// println!("{total} bytes");
/// # Ok(())
/// # }
/// ```
```

`Filesystem` (type):

```rust
/// # Examples
///
/// ```no_run
/// use e2b_client::{FileOptions, Sandbox, WriteOptions};
///
/// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
/// let files = sandbox.files();
/// files.write("/tmp/greeting.txt", "hello", &WriteOptions::new()).await?;
/// assert_eq!(files.read_to_string("/tmp/greeting.txt", &FileOptions::new()).await?, "hello");
/// # Ok(())
/// # }
/// ```
```

`Filesystem::read`:

```rust
    /// # Examples
    ///
    /// ```no_run
    /// use e2b_client::{FileOptions, Sandbox};
    ///
    /// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
    /// let bytes = sandbox.files().read("/bin/true", &FileOptions::new().user("root")).await?;
    /// assert_eq!(&bytes[..4], b"\x7fELF");
    /// # Ok(())
    /// # }
    /// ```
```

`Filesystem::read_to_string`:

```rust
    /// # Examples
    ///
    /// ```no_run
    /// use std::time::Duration;
    ///
    /// use e2b_client::{FileOptions, Sandbox};
    ///
    /// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
    /// let options = FileOptions::new().request_timeout(Some(Duration::from_secs(5)));
    /// let hostname = sandbox.files().read_to_string("/etc/hostname", &options).await?;
    /// println!("{}", hostname.trim());
    /// # Ok(())
    /// # }
    /// ```
```

`Filesystem::read_stream`:

```rust
    /// # Examples
    ///
    /// ```no_run
    /// use e2b_client::{FileOptions, Sandbox};
    ///
    /// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
    /// let mut stream = sandbox.files().read_stream("/tmp/big.bin", &FileOptions::new()).await?;
    /// while let Some(chunk) = stream.next_chunk().await? {
    ///     println!("{} bytes", chunk.len());
    /// }
    /// # Ok(())
    /// # }
    /// ```
```

`Filesystem::write`:

```rust
    /// # Examples
    ///
    /// ```no_run
    /// use e2b_client::{Sandbox, WriteOptions};
    ///
    /// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
    /// let options = WriteOptions::new().user("root");
    /// sandbox.files().write("/etc/motd", "welcome\n", &options).await?;
    /// # Ok(())
    /// # }
    /// ```
```

`Filesystem::list`:

```rust
    /// # Examples
    ///
    /// ```no_run
    /// use e2b_client::{ListOptions, Sandbox};
    ///
    /// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
    /// // Two levels deep: the directory's entries and theirs.
    /// let entries = sandbox.files().list("/home/user", &ListOptions::new().depth(2)).await?;
    /// for entry in entries {
    ///     println!("{}", entry.path);
    /// }
    /// # Ok(())
    /// # }
    /// ```
```

`Filesystem::get_info`:

```rust
    /// # Examples
    ///
    /// ```no_run
    /// use e2b_client::{FileOptions, FileType, Sandbox};
    ///
    /// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
    /// let info = sandbox.files().get_info("/home/user", &FileOptions::new()).await?;
    /// assert_eq!(info.file_type, FileType::Directory);
    /// # Ok(())
    /// # }
    /// ```
```

`Filesystem::exists`:

```rust
    /// # Examples
    ///
    /// ```no_run
    /// use e2b_client::{FileOptions, Sandbox};
    ///
    /// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
    /// if !sandbox.files().exists("/home/user/.cache", &FileOptions::new()).await? {
    ///     println!("no cache yet");
    /// }
    /// # Ok(())
    /// # }
    /// ```
```

`Filesystem::make_dir`:

```rust
    /// # Examples
    ///
    /// ```no_run
    /// use e2b_client::{FileOptions, Sandbox};
    ///
    /// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
    /// let created = sandbox.files().make_dir("/home/user/out/logs", &FileOptions::new()).await?;
    /// println!("created now: {created}");
    /// # Ok(())
    /// # }
    /// ```
```

`Filesystem::rename`:

```rust
    /// # Examples
    ///
    /// ```no_run
    /// use e2b_client::{FileOptions, Sandbox};
    ///
    /// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
    /// let moved = sandbox
    ///     .files()
    ///     .rename("/tmp/report.txt", "/home/user/report.txt", &FileOptions::new())
    ///     .await?;
    /// assert_eq!(moved.path, "/home/user/report.txt");
    /// # Ok(())
    /// # }
    /// ```
```

`Filesystem::remove`:

```rust
    /// # Examples
    ///
    /// ```no_run
    /// use e2b_client::{FileOptions, Sandbox};
    ///
    /// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
    /// // Removes a file, or a directory with everything in it.
    /// sandbox.files().remove("/tmp/scratch", &FileOptions::new()).await?;
    /// # Ok(())
    /// # }
    /// ```
```

`FileOptions` (type; only if Task 5 left it without one):

```rust
/// # Examples
///
/// ```
/// use std::time::Duration;
///
/// use e2b_client::FileOptions;
///
/// let options = FileOptions::new()
///     .user("root")
///     .request_timeout(Some(Duration::from_secs(10)));
/// # let _ = options;
/// ```
```

`ListOptions` (type; only if Task 5 left it without one):

```rust
/// # Examples
///
/// ```
/// use e2b_client::ListOptions;
///
/// // Recurse three levels, as the user `root`.
/// let options = ListOptions::new().depth(3).user("root");
/// # let _ = options;
/// ```
```

**`src/lifecycle.rs`**

`SnapshotMode` (type):

```rust
/// # Examples
///
/// ```no_run
/// use e2b_client::{PauseOptions, Sandbox, SnapshotMode};
///
/// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
/// // Keep memory, so the next resume continues where it left off.
/// sandbox.pause(&PauseOptions::new().mode(SnapshotMode::Full)).await?;
/// # Ok(())
/// # }
/// ```
```

`OnTimeout` (type):

```rust
/// # Examples
///
/// ```
/// use e2b_client::{CreateSandboxParams, Lifecycle, OnTimeout, SnapshotMode};
///
/// // Pause with a filesystem-only snapshot when the timeout passes.
/// let lifecycle = Lifecycle::new().on_timeout(OnTimeout::pause_with(SnapshotMode::Filesystem));
/// let params = CreateSandboxParams::new("base").lifecycle(lifecycle);
/// # let _ = params;
/// ```
```

`TimeoutAction` (type):

```rust
/// # Examples
///
/// ```no_run
/// use e2b_client::{Sandbox, TimeoutAction};
///
/// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
/// let info = sandbox.get_info().await?;
/// if let Some(lifecycle) = info.lifecycle {
///     if lifecycle.on_timeout == TimeoutAction::Pause {
///         println!("pauses at its timeout; auto-resume: {}", lifecycle.auto_resume);
///     }
/// }
/// # Ok(())
/// # }
/// ```
```

`LifecycleInfo` (type):

```rust
/// # Examples
///
/// ```no_run
/// # async fn demo(sandbox: e2b_client::Sandbox) -> e2b_client::Result<()> {
/// let lifecycle = sandbox.get_info().await?.lifecycle;
/// println!("{lifecycle:?}");
/// # Ok(())
/// # }
/// ```
```

`OnResume` (type):

```rust
/// # Examples
///
/// ```no_run
/// use e2b_client::{Client, ConnectOptions, OnResume};
///
/// # async fn demo(client: Client, sandbox_id: &str) -> e2b_client::Result<()> {
/// let options = ConnectOptions::new().on_resume(OnResume::Reboot);
/// let sandbox = client.connect_sandbox(sandbox_id, &options).await?;
/// # let _ = sandbox;
/// # Ok(())
/// # }
/// ```
```

**`src/metrics.rs`**

`SandboxMetrics` (type):

```rust
/// # Examples
///
/// ```no_run
/// use e2b_client::{MetricsParams, Sandbox};
///
/// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
/// for sample in sandbox.get_metrics(&MetricsParams::new()).await? {
///     println!(
///         "{} cpu {:.1}% mem {}/{}",
///         sample.timestamp, sample.cpu_used_pct, sample.mem_used, sample.mem_total
///     );
/// }
/// # Ok(())
/// # }
/// ```
```

**`src/models.rs`**

`CreateSandboxParams` (type):

```rust
/// # Examples
///
/// ```
/// use std::time::Duration;
///
/// use e2b_client::{CreateSandboxParams, NetworkConfig};
///
/// let params = CreateSandboxParams::new("base")
///     .timeout(Duration::from_secs(600))
///     .metadata("owner", "docs")
///     .env("LANG", "C.UTF-8")
///     .network(NetworkConfig::new().allow_public_traffic(false));
/// assert_eq!(params.template(), "base");
/// ```
```

`SandboxState` (type):

```rust
/// # Examples
///
/// ```no_run
/// use e2b_client::{ListSandboxesParams, SandboxState};
///
/// # async fn demo(client: e2b_client::Client) -> e2b_client::Result<()> {
/// let paused = client
///     .list_sandboxes(&ListSandboxesParams::new().state(SandboxState::Paused))
///     .await?;
/// println!("{} paused", paused.sandboxes.len());
/// # Ok(())
/// # }
/// ```
```

`SandboxInfo` (type):

```rust
/// # Examples
///
/// ```no_run
/// # async fn demo(sandbox: e2b_client::Sandbox) -> e2b_client::Result<()> {
/// let info = sandbox.get_info().await?;
/// println!(
///     "{} ({}): {} CPUs, {} MiB, ends {}",
///     info.sandbox_id, info.template_id, info.cpu_count, info.memory_mb, info.end_at
/// );
/// # Ok(())
/// # }
/// ```
```

`SortOrder` (type):

```rust
/// # Examples
///
/// ```
/// use e2b_client::{ListSandboxesParams, SortOrder};
///
/// let newest_first = ListSandboxesParams::new().order(SortOrder::Descending).limit(10);
/// # let _ = newest_first;
/// ```
```

`SandboxPage` (type):

```rust
/// # Examples
///
/// ```no_run
/// use e2b_client::{Client, ListSandboxesParams};
///
/// # async fn demo(client: Client) -> e2b_client::Result<()> {
/// let mut params = ListSandboxesParams::new().limit(100);
/// loop {
///     let page = client.list_sandboxes(&params).await?;
///     for sandbox in &page.sandboxes {
///         println!("{}", sandbox.sandbox_id);
///     }
///     match page.next_token {
///         Some(token) => params = params.next_token(token),
///         None => break,
///     }
/// }
/// # Ok(())
/// # }
/// ```
```

**`src/network.rs`**

`NetworkInfo` (type):

```rust
/// # Examples
///
/// ```no_run
/// # async fn demo(sandbox: e2b_client::Sandbox) -> e2b_client::Result<()> {
/// if let Some(network) = sandbox.get_info().await?.network {
///     println!("allow {:?}, deny {:?}", network.allow_out, network.deny_out);
/// }
/// # Ok(())
/// # }
/// ```
```

**`src/pty.rs`**

`PtyEvent` (type):

```rust
/// # Examples
///
/// ```no_run
/// use e2b_client::{PtyEvent, PtyOptions, PtySize, Sandbox};
///
/// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
/// let mut pty = sandbox.pty().create(PtySize::new(80, 24), &PtyOptions::new()).await?;
/// pty.send_stdin("exit\n").await?;
/// while let Some(event) = pty.next_event().await? {
///     match event {
///         PtyEvent::Data(bytes) => print!("{}", String::from_utf8_lossy(&bytes)),
///         PtyEvent::Exit(exit) => println!("[exit {}]", exit.exit_code),
///         _ => {}
///     }
/// }
/// # Ok(())
/// # }
/// ```
```

`Pty` (type):

```rust
/// # Examples
///
/// ```no_run
/// use e2b_client::{PtyOptions, PtySize, Sandbox};
///
/// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
/// let terminals = sandbox.pty();
/// let handle = terminals.create(PtySize::new(120, 40), &PtyOptions::new()).await?;
/// terminals.resize(handle.pid(), PtySize::new(100, 30)).await?;
/// terminals.kill(handle.pid()).await?;
/// # Ok(())
/// # }
/// ```
```

`Pty::connect`:

```rust
    /// # Examples
    ///
    /// ```no_run
    /// use e2b_client::{PtyConnectOptions, PtyOptions, PtySize, Sandbox};
    ///
    /// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
    /// let first = sandbox.pty().create(PtySize::new(80, 24), &PtyOptions::new()).await?;
    /// let pid = first.pid();
    /// drop(first); // the shell keeps running
    /// let again = sandbox.pty().connect(pid, &PtyConnectOptions::new()).await?;
    /// again.send_stdin("echo back\n").await?;
    /// # Ok(())
    /// # }
    /// ```
```

(If Task 11 found that a dropped handle or an exited shell behaves differently, keep this example consistent with the rustdoc Task 11 wrote.)

`Pty::send_stdin`:

```rust
    /// # Examples
    ///
    /// ```no_run
    /// use e2b_client::{PtyOptions, PtySize, Sandbox};
    ///
    /// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
    /// let handle = sandbox.pty().create(PtySize::new(80, 24), &PtyOptions::new()).await?;
    /// sandbox.pty().send_stdin(handle.pid(), "ls -la\n").await?;
    /// # Ok(())
    /// # }
    /// ```
```

`Pty::resize`:

```rust
    /// # Examples
    ///
    /// ```no_run
    /// use e2b_client::{PtyOptions, PtySize, Sandbox};
    ///
    /// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
    /// let handle = sandbox.pty().create(PtySize::new(80, 24), &PtyOptions::new()).await?;
    /// sandbox.pty().resize(handle.pid(), PtySize::new(132, 43)).await?;
    /// # Ok(())
    /// # }
    /// ```
```

`Pty::kill`:

```rust
    /// # Examples
    ///
    /// ```no_run
    /// use e2b_client::{PtyOptions, PtySize, Sandbox};
    ///
    /// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
    /// let handle = sandbox.pty().create(PtySize::new(80, 24), &PtyOptions::new()).await?;
    /// assert!(sandbox.pty().kill(handle.pid()).await?);
    /// # Ok(())
    /// # }
    /// ```
```

`PtyHandle` (type):

```rust
/// # Examples
///
/// ```no_run
/// use e2b_client::{PtyOptions, PtySize, Sandbox};
///
/// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
/// let pty = sandbox.pty().create(PtySize::new(80, 24), &PtyOptions::new()).await?;
/// pty.send_stdin("exit 7\n").await?;
/// assert_eq!(pty.wait().await?.exit_code, 7);
/// # Ok(())
/// # }
/// ```
```

`PtyHandle::next_event`:

```rust
    /// # Examples
    ///
    /// ```no_run
    /// use e2b_client::{PtyEvent, PtyOptions, PtySize, Sandbox};
    ///
    /// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
    /// let mut pty = sandbox.pty().create(PtySize::new(80, 24), &PtyOptions::new()).await?;
    /// pty.send_stdin("echo hi; exit\n").await?;
    /// while let Some(event) = pty.next_event().await? {
    ///     if let PtyEvent::Data(bytes) = event {
    ///         print!("{}", String::from_utf8_lossy(&bytes));
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
```

`PtyHandle::wait`:

```rust
    /// # Examples
    ///
    /// ```no_run
    /// use e2b_client::{PtyOptions, PtySize, Sandbox};
    ///
    /// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
    /// let pty = sandbox.pty().create(PtySize::new(80, 24), &PtyOptions::new()).await?;
    /// pty.send_stdin("exit\n").await?;
    /// let exit = pty.wait().await?;
    /// println!("shell exited with {}", exit.exit_code);
    /// # Ok(())
    /// # }
    /// ```
```

`PtyHandle::send_stdin`:

```rust
    /// # Examples
    ///
    /// ```no_run
    /// use e2b_client::{PtyOptions, PtySize, Sandbox};
    ///
    /// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
    /// let pty = sandbox.pty().create(PtySize::new(80, 24), &PtyOptions::new()).await?;
    /// // Ctrl-C, as a terminal sends it.
    /// pty.send_stdin([0x03]).await?;
    /// # Ok(())
    /// # }
    /// ```
```

`PtyHandle::resize`:

```rust
    /// # Examples
    ///
    /// ```no_run
    /// use e2b_client::{PtyOptions, PtySize, Sandbox};
    ///
    /// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
    /// let pty = sandbox.pty().create(PtySize::new(80, 24), &PtyOptions::new()).await?;
    /// pty.resize(PtySize::new(200, 50)).await?;
    /// # Ok(())
    /// # }
    /// ```
```

`PtyHandle::kill`:

```rust
    /// # Examples
    ///
    /// ```no_run
    /// use e2b_client::{PtyOptions, PtySize, Sandbox};
    ///
    /// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
    /// let pty = sandbox.pty().create(PtySize::new(80, 24), &PtyOptions::new()).await?;
    /// assert!(pty.kill().await?);
    /// # Ok(())
    /// # }
    /// ```
```

`PtyConnectOptions` (type; only if Task 7 or 8 left it without one):

```rust
/// # Examples
///
/// ```
/// use std::time::Duration;
///
/// use e2b_client::PtyConnectOptions;
///
/// // No deadline is the default; this one ends the stream after an hour.
/// let options = PtyConnectOptions::new().timeout(Some(Duration::from_secs(3600)));
/// # let _ = options;
/// ```
```

**`src/sandbox.rs`**

`Sandbox::mcp_token`:

```rust
    /// # Examples
    ///
    /// ```no_run
    /// use e2b_client::secrecy::ExposeSecret;
    ///
    /// # async fn demo(sandbox: e2b_client::Sandbox) -> e2b_client::Result<()> {
    /// let token = sandbox.mcp_token().await?;
    /// // Send it as `Authorization: Bearer <token>` to `sandbox.mcp_url()`.
    /// let header = format!("Bearer {}", token.expose_secret());
    /// # let _ = header;
    /// # Ok(())
    /// # }
    /// ```
```

`Sandbox::is_running`:

```rust
    /// # Examples
    ///
    /// ```no_run
    /// # async fn demo(sandbox: e2b_client::Sandbox) -> e2b_client::Result<()> {
    /// if !sandbox.is_running().await? {
    ///     println!("{} has stopped", sandbox.sandbox_id());
    /// }
    /// # Ok(())
    /// # }
    /// ```
```

`Sandbox::kill`:

```rust
    /// # Examples
    ///
    /// ```no_run
    /// # async fn demo(sandbox: e2b_client::Sandbox) -> e2b_client::Result<()> {
    /// sandbox.kill().await?;
    /// # Ok(())
    /// # }
    /// ```
```

`Sandbox::pause`:

```rust
    /// # Examples
    ///
    /// ```no_run
    /// use e2b_client::{Client, ConnectOptions, PauseOptions, Sandbox};
    ///
    /// # async fn demo(client: Client, sandbox: Sandbox) -> e2b_client::Result<()> {
    /// sandbox.pause(&PauseOptions::new()).await?;
    /// // Later: connecting resumes it.
    /// let resumed = client.connect_sandbox(sandbox.sandbox_id(), &ConnectOptions::new()).await?;
    /// # let _ = resumed;
    /// # Ok(())
    /// # }
    /// ```
```

`Sandbox::set_timeout`:

```rust
    /// # Examples
    ///
    /// ```no_run
    /// use std::time::Duration;
    ///
    /// # async fn demo(sandbox: e2b_client::Sandbox) -> e2b_client::Result<()> {
    /// sandbox.set_timeout(Duration::from_secs(30 * 60)).await?;
    /// # Ok(())
    /// # }
    /// ```
```

`Sandbox::update_network`:

```rust
    /// # Examples
    ///
    /// ```no_run
    /// use e2b_client::NetworkUpdate;
    ///
    /// # async fn demo(sandbox: e2b_client::Sandbox) -> e2b_client::Result<()> {
    /// sandbox.update_network(&NetworkUpdate::new().allow_internet_access(true)).await?;
    /// # Ok(())
    /// # }
    /// ```
```

`Sandbox::refresh`:

```rust
    /// # Examples
    ///
    /// ```no_run
    /// # async fn demo(sandbox: e2b_client::Sandbox) -> e2b_client::Result<()> {
    /// // Keep the sandbox alive for the API's default refresh period.
    /// sandbox.refresh(None).await?;
    /// # Ok(())
    /// # }
    /// ```
```

`Sandbox::get_info`:

```rust
    /// # Examples
    ///
    /// ```no_run
    /// # async fn demo(sandbox: e2b_client::Sandbox) -> e2b_client::Result<()> {
    /// let info = sandbox.get_info().await?;
    /// println!("{:?}, metadata {:?}", info.state, info.metadata);
    /// # Ok(())
    /// # }
    /// ```
```

`Sandbox::get_metrics`:

```rust
    /// # Examples
    ///
    /// ```no_run
    /// use e2b_client::MetricsParams;
    ///
    /// # async fn demo(sandbox: e2b_client::Sandbox) -> e2b_client::Result<()> {
    /// let samples = sandbox.get_metrics(&MetricsParams::new()).await?;
    /// if let Some(last) = samples.last() {
    ///     println!("disk {}/{}", last.disk_used, last.disk_total);
    /// }
    /// # Ok(())
    /// # }
    /// ```
```

`Sandbox::create_snapshot`:

```rust
    /// # Examples
    ///
    /// ```no_run
    /// use e2b_client::{CreateSandboxParams, SnapshotOptions};
    ///
    /// # async fn demo(client: e2b_client::Client, sandbox: e2b_client::Sandbox) -> e2b_client::Result<()> {
    /// let snapshot = sandbox.create_snapshot(&SnapshotOptions::new().name("warm-cache")).await?;
    /// // A new sandbox starts from the snapshot as from a template.
    /// let copy = client.create_sandbox(&CreateSandboxParams::new(&snapshot.snapshot_id)).await?;
    /// # let _ = copy;
    /// # Ok(())
    /// # }
    /// ```
```

`Sandbox::list_snapshots`:

```rust
    /// # Examples
    ///
    /// ```no_run
    /// use e2b_client::ListSnapshotsParams;
    ///
    /// # async fn demo(sandbox: e2b_client::Sandbox) -> e2b_client::Result<()> {
    /// let page = sandbox.list_snapshots(&ListSnapshotsParams::new()).await?;
    /// for snapshot in page.snapshots {
    ///     println!("{} {:?}", snapshot.snapshot_id, snapshot.names);
    /// }
    /// # Ok(())
    /// # }
    /// ```
```

`Sandbox::fork`:

```rust
    /// # Examples
    ///
    /// ```no_run
    /// use e2b_client::ForkOptions;
    ///
    /// # async fn demo(sandbox: e2b_client::Sandbox) -> e2b_client::Result<()> {
    /// for fork in sandbox.fork(&ForkOptions::new().count(3)).await? {
    ///     match fork {
    ///         Ok(copy) => println!("forked {}", copy.sandbox_id()),
    ///         Err(error) => eprintln!("one fork failed: {error}"),
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
```

**`src/snapshots.rs`**

`SnapshotInfo` (type):

```rust
/// # Examples
///
/// ```no_run
/// use e2b_client::SnapshotOptions;
///
/// # async fn demo(sandbox: e2b_client::Sandbox) -> e2b_client::Result<()> {
/// let snapshot = sandbox.create_snapshot(&SnapshotOptions::new()).await?;
/// println!("{} also known as {:?}", snapshot.snapshot_id, snapshot.names);
/// # Ok(())
/// # }
/// ```
```

`SnapshotPage` (type):

```rust
/// # Examples
///
/// ```no_run
/// use e2b_client::{Client, ListSnapshotsParams};
///
/// # async fn demo(client: Client) -> e2b_client::Result<()> {
/// let mut params = ListSnapshotsParams::new().limit(50);
/// loop {
///     let page = client.list_snapshots(&params).await?;
///     println!("{} snapshots", page.snapshots.len());
///     match page.next_token {
///         Some(token) => params = params.next_token(token),
///         None => break,
///     }
/// }
/// # Ok(())
/// # }
/// ```
```

**`src/stream_input.rs`**

`InputStream` (type):

```rust
/// # Examples
///
/// ```no_run
/// use e2b_client::{CommandOptions, Sandbox, StreamInputOptions};
///
/// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
/// let handle = sandbox.commands().start("cat", &CommandOptions::new().stdin(true)).await?;
/// let mut input = sandbox.commands().stream_input(handle.pid(), &StreamInputOptions::new());
/// input.send("a").await?;
/// input.send("b\n").await?;
/// input.close().await?;
/// sandbox.commands().close_stdin(handle.pid()).await?;
/// assert_eq!(handle.wait().await?.stdout, "ab\n");
/// # Ok(())
/// # }
/// ```
```

`InputStream::send`:

```rust
    /// # Examples
    ///
    /// ```no_run
    /// use e2b_client::{CommandOptions, Sandbox, StreamInputOptions};
    ///
    /// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
    /// let handle = sandbox.commands().start("wc -l", &CommandOptions::new().stdin(true)).await?;
    /// let mut input = sandbox.commands().stream_input(handle.pid(), &StreamInputOptions::new());
    /// for line in ["one\n", "two\n", "three\n"] {
    ///     input.send(line).await?;
    /// }
    /// input.close().await?;
    /// # Ok(())
    /// # }
    /// ```
```

`InputStream::close`:

```rust
    /// # Examples
    ///
    /// ```no_run
    /// use e2b_client::{CommandOptions, Sandbox, StreamInputOptions};
    ///
    /// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
    /// let handle = sandbox.commands().start("cat", &CommandOptions::new().stdin(true)).await?;
    /// let mut input = sandbox.commands().stream_input(handle.pid(), &StreamInputOptions::new());
    /// input.send("done\n").await?;
    /// // Waits for envd to accept every chunk; stdin itself stays open.
    /// input.close().await?;
    /// # Ok(())
    /// # }
    /// ```
```

**`src/volumes.rs`**

`Volume` (type):

```rust
/// # Examples
///
/// ```no_run
/// use e2b_client::CreateSandboxParams;
///
/// # async fn demo(client: e2b_client::Client) -> e2b_client::Result<()> {
/// let volume = client.create_volume("datasets").await?;
/// let params = CreateSandboxParams::new("base").volume_mount(volume.name(), "/mnt/data");
/// let sandbox = client.create_sandbox(&params).await?;
/// # let _ = sandbox;
/// # Ok(())
/// # }
/// ```
```

`VolumeInfo` (type):

```rust
/// # Examples
///
/// ```no_run
/// # async fn demo(client: e2b_client::Client) -> e2b_client::Result<()> {
/// let names: Vec<String> = client.list_volumes().await?.into_iter().map(|v| v.name).collect();
/// println!("{names:?}");
/// # Ok(())
/// # }
/// ```
```

`VolumeMount` (type):

```rust
/// # Examples
///
/// ```no_run
/// # async fn demo(sandbox: e2b_client::Sandbox) -> e2b_client::Result<()> {
/// for mount in sandbox.get_info().await?.volume_mounts {
///     println!("{} at {}", mount.name, mount.path);
/// }
/// # Ok(())
/// # }
/// ```
```

The examples that **run** (no `no_run`): `CommandConnectOptions`, `ConnectionConfig`, `EnvdVersion`, `ConnectCode`, `FileOptions`, `ListOptions`, `OnTimeout`, `CreateSandboxParams`, `SortOrder`, `PtyConnectOptions`. Each asserts only what the code itself guarantees; check every assertion against the source before running.

- [ ] **Step 4: Run the test and the doctests**

Run: `cargo test --test doc_examples`
Expected: PASS (3 passed). Any item still listed gets an example in the same style.

Run: `cargo test --doc`
Expected: PASS. A `no_run` example still compiles, so a wrong signature fails here; fix the example, not the API.

- [ ] **Step 5: Record the coverage**

Run the plan session's count again:

```bash
python3 - <<'EOF'
import re, glob
types = asyncs = types_ex = asyncs_ex = 0
for path in sorted(glob.glob('src/**/*.rs', recursive=True)):
    lines = open(path).read().split('\n')
    for i, line in enumerate(lines):
        m = re.match(r'(pub (struct|enum) \w+)|(\s*pub async fn \w+)', line)
        if not m:
            continue
        j, doc = i - 1, []
        while j >= 0 and lines[j].strip().startswith(('///', '#[')):
            doc.append(lines[j].strip()); j -= 1
        has = '/// # Examples' in doc
        if m.group(1):
            types += 1; types_ex += has
        else:
            asyncs += 1; asyncs_ex += has
print(f"pub types {types_ex}/{types}, pub async fns {asyncs_ex}/{asyncs}")
EOF
```

Expected: every async function has one (`N/N`); the type count includes non-re-exported `pub` types in `src/envd/*_pb.rs`, which the test does not require. Append to `docs/api-review.md`:

```markdown
## Example coverage (Task 14)

`tests/doc_examples.rs` requires an `# Examples` section on every public type
re-exported from `lib.rs` and every `pub async fn`. At <merge commit>: <X>/<X>
public async functions and every re-exported type have one, up from 13 of 72
async functions on `origin/main` before M4. `missing_docs` (Task 12) keeps
every public item documented.
```

(Fill `<merge commit>` and `<X>` from Step 5's output when committing; they are measured values, not placeholders in the code.)

- [ ] **Step 6: Run the gates**

Run: `just lint`, `just test`, `just doc`.
Expected: all PASS.

- [ ] **Step 7: Changelog and commit**

`changelog/<ts>-m4-task-14-rustdoc-examples.md`: the rule, the test, the count before (Step 2) and after (Step 5).

```bash
git add src/ tests/doc_examples.rs docs/api-review.md changelog/
git commit -m "docs: an example on every public type and async method, enforced by a test (#<issue>)"
```


---

### Task 15: Paced live runs for sandbawks (#57, CLM-0181)

**Files:**
- Modify: `justfile` (recipe `test-live-paced`; the commit stamp moves to a shared private recipe `_stamp`)
- Create: `docs/verification/live-pacing.md`
- Create: `docs/verification/runs/<run id>.log` (the validation run)
- Modify: `CONTRIBUTING.md` (tooling table)
- Create: `changelog/<ts>-m4-task-15-paced-live-runs.md`

sandbawks' two-sandbox warm pool needs about 120 s to refill. `just test-live` runs four tests at a time with no gap, so on sandbawks most tests fail on readiness and M2 and M3 needed hand-paced rerun rounds (`docs/verification/m2-live-results.md:120-145`, `m3-live-results.md:167-203`). The new recipe does those rounds itself. Hosted runs keep `just test-live`.

**Readiness markers.** A failure counts as readiness, not as a result, when its output contains one of the messages the M2 and M3 runs saw while no sandbox was ready: `still starting up` (envd's 503), `Service Temporarily Unavailable` (nginx's 503), `Unschedulable` (the Pod could not be placed) or `will not become ready` (sandbawks' 412 `action_failure`). The synthetic exit code 75 seen in M2 has no message of its own, so it is reported as `FAIL` and left to the person reading the log.

- [ ] **Step 1: Move the commit stamp into a shared recipe**

The header lines are needed by both live recipes, so they move to one place (DRY). In `justfile`, replace the header block inside `test-live` (from `# The first two lines say exactly which code produced this log.` to `} | tee "$log"`) with:

```bash
    {{just_executable()}} --justfile {{justfile()}} _stamp "$log"
```

and add, at the end of the file:

```just
# Writes the two lines that say which code produced a live log: the commit and
# whether the tree was dirty. Outside a git checkout, or without git, both say
# `unknown` rather than lying. Used by `test-live` and `test-live-paced`.
[private]
_stamp log:
    #!/usr/bin/env bash
    set -uo pipefail
    {
        echo "commit: $(git rev-parse --verify -q HEAD 2>/dev/null || echo unknown)"
        if ! porcelain="$(git status --porcelain 2>/dev/null)"; then echo "dirty: unknown"
        elif [ -n "$porcelain" ]; then echo "dirty: yes"
        else echo "dirty: no"; fi
    } | tee "{{log}}"
```

Run: `just --list`
Expected: `test-live` and `test-live-paced` listed (after Step 2), `_stamp` not listed.

- [ ] **Step 2: Add the paced recipe**

In `justfile`, after `test-live`:

```just
# Live tests one at a time with a gap between them, for a server whose warm pool
# refills slowly (sandbawks): `just test-live-paced sandbawks`, or with a gap in
# seconds and a test-name filter (an extended regex):
# `just test-live-paced sandbawks 30 'clm_0007|clm_0033'`. A test that fails on
# server readiness is rerun once, after the gap. Writes target/live-logs/<run id>.log.
test-live-paced target gap="120" filter="":
    #!/usr/bin/env bash
    set -uo pipefail
    export E2B_TEST_TARGET="{{target}}"
    export E2B_TEST_RUN_ID="$(date -u +%Y%m%dT%H%M%SZ)-{{target}}-paced-$$"
    gap="{{gap}}"
    filter="{{filter}}"
    mkdir -p target/live-logs
    log="target/live-logs/${E2B_TEST_RUN_ID}.log"
    {{just_executable()}} --justfile {{justfile()}} _stamp "$log"
    echo "paced: one test at a time, ${gap}s apart; readiness failures rerun once" | tee -a "$log"
    # Messages that mean the server had no ready sandbox, so the test says
    # nothing about the feature (docs/verification/live-pacing.md).
    readiness='still starting up|Service Temporarily Unavailable|Unschedulable|will not become ready'
    binaries=(live_lifecycle live_commands live_files)
    # Build once up front, so a compile error stops the run instead of
    # showing up as a failure of every test.
    if ! cargo test --no-run "${binaries[@]/#/--test=}" >>"$log" 2>&1; then
        echo "build failed; see $log" >&2
        exit 1
    fi
    # Runs one test; its output goes to the file named by $3.
    run_one() {
        cargo test --test "$1" -- --ignored --exact "$2" --test-threads=1 --show-output >"$3" 2>&1
    }
    status=0
    first=1
    retry=()
    for bin in "${binaries[@]}"; do
        mapfile -t tests < <(cargo test --test "$bin" -- --ignored --list 2>/dev/null | sed -n 's/: test$//p')
        for test in "${tests[@]}"; do
            if [ -n "$filter" ] && ! grep -Eq -- "$filter" <<<"$test"; then continue; fi
            [ "$first" -eq 1 ] || sleep "$gap"
            first=0
            out="$(mktemp -p target/live-logs paced.XXXXXX)"
            if run_one "$bin" "$test" "$out"; then result=PASS
            elif grep -Eq -- "$readiness" "$out"; then result=FAIL-READINESS; retry+=("$bin $test")
            else result=FAIL; status=1; fi
            cat "$out" >>"$log"
            rm -f "$out"
            echo "RESULT $test $result" | tee -a "$log"
        done
    done
    # One rerun for each readiness failure, after the gap, so the pool can refill.
    for entry in "${retry[@]}"; do
        bin="${entry%% *}"
        test="${entry#* }"
        sleep "$gap"
        out="$(mktemp -p target/live-logs paced.XXXXXX)"
        if run_one "$bin" "$test" "$out"; then result=PASS-ON-RERUN
        else result=FAIL-READINESS; status=1; fi
        cat "$out" >>"$log"
        rm -f "$out"
        echo "RESULT $test $result (rerun)" | tee -a "$log"
    done
    cargo test --test live_sweep -- --ignored --show-output 2>&1 | tee -a "$log"
    sweep_status=${PIPESTATUS[0]}
    echo "summary: $(grep -c '^RESULT .* PASS$' "$log") pass, $(grep -c 'PASS-ON-RERUN' "$log") pass on rerun, $(grep -c '^RESULT .* FAIL$' "$log") fail, $(grep -c '^RESULT .* FAIL-READINESS (rerun)$' "$log") still not ready" | tee -a "$log"
    echo "run log: $log"
    # A failed sweep means leaked sandboxes, so it fails the run too.
    if [ "$status" -ne 0 ]; then exit "$status"; fi
    exit "$sweep_status"
```

Notes for the implementer:
- `${binaries[@]/#/--test=}` prefixes each name, giving `--test=live_lifecycle --test=live_commands --test=live_files`.
- `cargo test -- --ignored --list` prints `<name>: test` for each ignored test; the `sed` keeps the names. Check the output format with `cargo test --test live_files -- --ignored --list` before relying on it.
- Each test gets the run's `E2B_TEST_RUN_ID`, so the final sweep finds everything the run created.
- `"${retry[@]}"` on an empty array is fine under `set -u` in bash 4.4 and later; check `bash --version`.
- The `RESULT` line of a test that failed and was rerun appears twice: `FAIL-READINESS`, then the rerun's result with ` (rerun)`. The summary counts final results.

- [ ] **Step 3: Check the recipe without a server**

Run: `just test-live-paced nosuchtarget 1 clm_0007`. The harness reads keys from `.env`, so an unknown target is the reliable way to run the recipe without reaching a server (`tests/live_support/mod.rs`, `target()`, rejects it; check that it panics rather than defaulting, and pick another dry-run method if it doesn't).
Expected: the log starts with `commit:` and `dirty:`, then `paced: ...`; one `RESULT clm_0007_create_returns_running_sandbox FAIL` line (the harness panics on the unknown target, which is not a readiness message); the sweep fails the same way; exit status non-zero. No sandbox is created.

- [ ] **Step 4: Validate on sandbawks**

Run three cheap tests with a short gap:

```bash
just test-live-paced sandbawks 30 'clm_0007_create_returns_running_sandbox|clm_0033_run_returns_output_and_exit_code|clm_0050_write_read_round_trips_binary'
```

Expected: three `RESULT` lines (each `PASS`, `FAIL-READINESS` followed by a rerun line, or `FAIL` with a known sandbawks issue), the sweep lines with 0 leftovers, and the summary line. Check the log for both API keys exactly as in Task 16 Step 2 (every count `0`), then copy it to `docs/verification/runs/`.

- [ ] **Step 5: Document it**

Create `docs/verification/live-pacing.md`:

```markdown
# Paced live runs

`just test-live-paced <target> [gap] [filter]` runs the live suite one test at
a time, `gap` seconds apart (default 120), for servers whose warm pool refills
slowly. sandbawks keeps two warm sandboxes; at `--test-threads=4` a run drains
them at once and the cold creates that follow time out or go unschedulable
(`docs/verification/m2-live-results.md`, "Why there are four rerun rounds").
Hosted E2B has no such limit and keeps `just test-live`.

## What it does

1. Writes the same `commit:` and `dirty:` header as `just test-live`.
2. Builds the three live binaries once.
3. Lists each binary's ignored tests and runs them one by one with
   `--exact --test-threads=1`, sleeping `gap` seconds between tests. A
   `filter` (an extended regex on the test name) runs a subset.
4. Classifies each result and writes one line per test:

   | Line | Meaning |
   |---|---|
   | `RESULT <test> PASS` | passed on the first try |
   | `RESULT <test> FAIL` | failed for a reason other than readiness: a real result |
   | `RESULT <test> FAIL-READINESS` | failed while the server had no ready sandbox; rerun below |
   | `RESULT <test> PASS-ON-RERUN (rerun)` | passed on its one rerun |
   | `RESULT <test> FAIL-READINESS (rerun)` | still no ready sandbox after the rerun: a capacity result, says nothing about the feature |

   A failure is a readiness failure when its output contains `still starting
   up`, `Service Temporarily Unavailable`, `Unschedulable` or `will not become
   ready`, the messages the M2 and M3 sandbawks runs saw while no sandbox was
   ready. A failure reported only as an exit code (M2 saw a synthetic 75) is
   left as `FAIL` for a person to classify.
5. Reruns each readiness failure once, after another gap.
6. Runs `live_sweep` for the run id and prints a summary line.

The run fails if any test ends `FAIL` or `FAIL-READINESS (rerun)`, or if the
sweep fails.

## Cost and time

A full run is about (number of tests) × (gap + test time): 79 tests at 120 s
is roughly three hours. Run it in the background and wait for its log. A
narrower filter and gap are fine for checking a fix.

## Validation (M4 Task 15)

[`runs/<run id>.log`](runs/<run id>.log): three tests, 30 s apart, against
sandbawks, with the results <as recorded>.
```

(Fill the run id and the three results from Step 4 when committing.)

In `CONTRIBUTING.md`'s tooling table, add:

```markdown
| `just test-live-paced <target> [gap] [filter]` | Live tests one at a time with a gap, rerunning readiness failures once (see `docs/verification/live-pacing.md`) |
```

- [ ] **Step 6: Run the gates**

Run: `just lint`, `just test`.
Expected: PASS (no Rust code changed).

- [ ] **Step 7: Changelog and commit**

`changelog/<ts>-m4-task-15-paced-live-runs.md`: the recipe, the markers and where they came from, the validation run's id and results.

```bash
git add justfile CONTRIBUTING.md docs/verification/ changelog/
git commit -m "build: just test-live-paced, one live test at a time with readiness reruns (#57)"
```


---

### Task 16: Live run on hosted E2B

**Files:**
- Create: `docs/verification/runs/<run id>.log` (copied from `target/live-logs/`)
- Create or modify: `docs/verification/m4-live-results.md` (section "Hosted E2B")
- Create: `changelog/<ts>-m4-task-16-hosted-run.md`

- [ ] **Step 1: Count the tests**

Run:

```bash
for bin in live_lifecycle live_commands live_files; do
    printf '%s %s\n' "$bin" "$(cargo test --test "$bin" -- --ignored --list 2>/dev/null | grep -c ': test$')"
done
```

Expected: 79 in all: M3's 75 (39 in `live_lifecycle`, 17 in `live_commands`, 19 in `live_files`) plus M4's `clm_0171` (a moded pause of a paused sandbox, Task 3), `clm_0175` (PTY without a deadline, Task 8), `clm_0177` (symlinks, Task 10) and `clm_0178` (PTY connect to an exited pid, Task 11). If Tasks 1 to 11 added or removed others, use the real count and correct this step with a dated note.

- [ ] **Step 2: Run the full suite on the merged main**

Run: `just test-live hosted`
Expected: the log starts with `commit:` and `dirty: no`, then one result line per `clm_*` test (the earlier milestones' tests rerun as a regression check, now through the M4 signatures), then the three sweep lines (sandboxes, snapshots, volumes). `clm_0175` takes over 60 s by design.

- [ ] **Step 3: Check for leaks, secrets and the stamp**

All three sweeps must report 0 leftovers. Check the log for both API keys (each actual value, not a pattern; nothing printed but counts):

```bash
log=target/live-logs/<run id>.log
env_file="$(git rev-parse --path-format=absolute --git-common-dir)/../.env"
for name in E2B_API_KEY_HOSTED E2B_API_KEY_SANDBAWKS; do
    value="$(grep "^${name}=" "$env_file" | cut -d= -f2-)"
    [ -n "$value" ] && echo "$name: $(grep -cF -- "$value" "$log")"
done
echo "GATEWAY_ACCESS_TOKEN=: $(grep -cF 'GATEWAY_ACCESS_TOKEN=' "$log")"
echo '"token": '"$(grep -cF '"token"' "$log")"
```

Every count must be `0` (the `.env` values are unquoted, M1's note). A log with `dirty: yes`, `dirty: unknown` or `commit: unknown` doesn't count; commit or clean the tree, then rerun.

- [ ] **Step 4: Summarize**

Copy the log to `docs/verification/runs/`. Start `docs/verification/m4-live-results.md` with a one-line intro ("Results of the Milestone 4 live runs (plan Tasks 16 and 17), one section per target."), then:

```markdown
## Hosted E2B (run <run id>, <UTC time>, commit <sha>)

Run with `just test-live hosted` on `<sha>` (main with every M4 code task,
M4.01 to M4.15). Log: [`runs/<run id>.log`](runs/<run id>.log). The 75
tests from M1 to M3 were rerun on purpose, as a regression check of the M4
API changes. Rows are sorted by claim number.

| Test | Claim | Result | Failure (first line) |
|---|---|---|---|
```

Every `clm_*` test gets a row. Classify each failure as a crate bug, a test bug or an environment limit. Fix test bugs in this PR, with a plan correction note. Only report crate bugs here; the coordinator files them as M4 bug tasks (Task 18). Environment limits, not bugs: the volume tests' 403 (`use of volumes is not enabled`, as in M3: `clm_0148` twice, `clm_0149`, `clm_0153_get_info_reports_volume_mounts`), a 400 or 409 on the filesystem-only pause, and a missing `mcp-gateway` template. Record each with the response body. Below the table, list the M4 claims each new or changed test backs (CLM-0168 through `clm_0015`, CLM-0173 through the files tests, CLM-0174 through `clm_0040`, CLM-0175, CLM-0177, CLM-0178).

- [ ] **Step 5: Changelog and commit**

```bash
git add docs/verification/ changelog/
git commit -m "docs: M4 live results on hosted E2B (#<issue>)"
```


---

### Task 17: Live run on sandbawks (paced)

As Task 16, with these differences:

- **The run:** `just test-live-paced sandbawks` (Task 15), with `E2B_TEMPLATE_SANDBAWKS=compound-sandbox` in `.env`. It takes about three hours at the default 120 s gap: start it in the background and wait for its own log (`target/live-logs/<run id>.log`) to print `run log:`; never poll with `pgrep` patterns. The log's `RESULT` lines and summary replace M3's hand-paced rerun rounds, so this is one run of record, not several.
- **The section** is "sandbawks" in `docs/verification/m4-live-results.md`, with the same table plus a column for the paced result (`PASS`, `PASS-ON-RERUN`, `FAIL`, `FAIL-READINESS (rerun)`).
- **Classes:** pass; **sandbawks** (a sandbawks defect: name the filed issue, or list it as new); **capacity** (`FAIL-READINESS (rerun)`: says nothing about the feature).
  - Known missing routes or behaviour, recorded against their existing issues, not filed again: network update, metrics, snapshots, fork and volumes (gfd/sandbawks #664 to #672); no envd access token (#683); #674 to #682 and #684 to #687 from M1 and M2; the template's missing `/home/user` (#688); list filters ignored (#689).
  - Every other failure that passes on hosted is a new sandbawks defect. Write its request and response in the results, and list it for the coordinator to file on gfd/sandbawks under epic #673.
- **The moded already-paused 409 (Task 3, added in issue #81):** record in the section the 409 body sandbawks sends to `clm_0171_moded_pause_of_a_paused_sandbox_is_false`, and correct the "Pause" row of `docs/protocol-notes.md` to say whether sandbawks's message contains `already paused` (case-sensitive). If `clm_0171` fails with `Error::Api` 409 there, the cause is the wording: it is a sandbawks defect (or, if the coordinator prefers, a reason to widen `ALREADY_PAUSED`), not capacity.
- **Sweeps:** the sandbox sweep must report 0 leftovers. On a server without the snapshot or volume routes, those two sweeps fail with 404 on their list call and make the recipe exit non-zero (M3 Task 13's note); record that, it is not a leak.
- **CLM-0118:** state in the section whether every v0.1 test (CLM-0007 to CLM-0027 and CLM-0033 to CLM-0063) now ends as pass or as a filed sandbawks defect with no capacity result. If so, the coordinator can decide CLM-0118 (open since M1 on capacity, #27).
- **Secret check:** as Task 16 Step 3.

Changelog `changelog/<ts>-m4-task-17-sandbawks-run.md`; commit `docs: M4 live results on sandbawks (#<issue>)`.


---

### Task 18: Verdicts, docs and milestone close (coordinator)

Done by the coordinator session, which alone writes the ledger. The ledger work is on a branch in that session's own checkout (an M2 lesson: the claims server writes `claims.json` in its start directory).

- [ ] **Step 1: Evidence.** Before writing any summary, read the test body or file it describes; evidence can't be edited (an M2 lesson). For each claim, `add_evidence` with the kind and locator below (`<sha>` is the merge commit of the PR named, `<hosted log>` and `<sandbawks log>` the Task 16 and 17 run logs):

  | Claim | Kind | Locator and what it shows |
  |---|---|---|
  | CLM-0119 | inspection | `docs/api-review.md` @ Task 12's merge: every type's NE decision; `CommandEvent` and `Signal` marked; `ConnectCode`, `PtySize` exceptions with reasons |
  | CLM-0120 | test | `tests/doc_examples.rs` @ Task 14's merge, passing in CI; `missing_docs = "warn"` in `Cargo.toml` with `just lint` (`-D warnings`) green |
  | CLM-0121 | inspection | `Cargo.toml` `rust-version = "1.89"`, `.github/workflows/ci.yml` job `msrv` green on main (`gh run list --branch main --limit 1`), README "Versioning" |
  | CLM-0168 | test | Task 1's mock test in `tests/rest_api.rs` (the evidence-map row); plus demonstration `clm_0015_connect_unknown_sandbox_is_not_found @ hosted, <hosted log>` |
  | CLM-0169 | test | `tests/error_taxonomy.rs` @ Task 2's merge (the evidence-map row) |
  | CLM-0170 | inspection | after `just doc`, `grep -rl reqwest target/doc/e2b_client/ \| wc -l` is `0` (rustdoc pages of the public API, not `target/doc/src/`); `pub use secrecy` in `src/lib.rs`; README "Versioning" list |
  | CLM-0171 | test | Task 3's mock tests in `tests/rest_lifecycle.rs` |
  | CLM-0172 | test | `tests/zero_timeouts.rs` @ Task 4's merge |
  | CLM-0173 | test | `tests/envd_file_options.rs` @ Task 5's merge; plus the `live_files` tests @ hosted (regression through the new signatures) |
  | CLM-0174 | test | `tests/envd_connect_options.rs` @ Task 7's merge: `command_connect_defaults_to_the_sdk_sixty_seconds`, `command_connect_sends_the_chosen_deadline`, `command_connect_without_a_deadline_sends_no_header`, `pty_connect_has_no_deadline_by_default` (Task 8's replacement of `pty_connect_defaults_to_the_sdk_sixty_seconds`), `a_shorter_request_timeout_bounds_the_wait_for_the_start_event`, `a_longer_request_timeout_overrides_the_configured_one`, `command_options_request_timeout_bounds_start`, `pty_options_request_timeout_bounds_create` (not `the_stream_deadline_is_not_the_request_timeout`, which only shows the two values travel separately); plus demonstration `clm_0040_connect_attaches_to_running_command @ hosted` |
  | CLM-0175 | demonstration | `clm_0175_* @ hosted, <hosted log>`; plus Task 8's mock test for the default |
  | CLM-0176 | test | Task 9's tests in `tests/envd_write.rs` |
  | CLM-0177 | test | Task 10's two mock tests in `tests/envd_api.rs`; plus demonstration `clm_0177_* @ hosted` |
  | CLM-0178 | demonstration | `clm_0178_* @ hosted, <hosted log>`; plus inspection of `Pty::connect`'s rustdoc @ Task 11's merge |
  | CLM-0179 | test | `tests/api_traits.rs` @ Task 12's merge |
  | CLM-0180 | inspection | `docs/api-review.md` @ Task 14's merge (with the coverage section) |
  | CLM-0181 | demonstration | Task 15's validation log and Task 17's run log: one `RESULT` line per test, the readiness reruns, the summary |

  Add the sandbawks demonstrations for CLM-0168, CLM-0173 to CLM-0175, CLM-0177 and CLM-0178 from `<sandbawks log>`. A sandbawks failure names its gfd/sandbawks issue, has stance `neutral`, and does not refute the claim.
- [ ] **Step 2: Verdicts** for CLM-0119, CLM-0120, CLM-0121 and CLM-0168 to CLM-0181, citing Step 1's evidence. An environment limit leaves its claim `not_enough_info`, with the limit as the rationale.
- [ ] **Step 3: Retire CLM-0087.** `mcp__claims__retire_claim` on CLM-0087 ("When v2 connect answers 404, connect_sandbox retries POST /sandboxes/{id}/connect ..."), with the reason "The v1 connect fallback was removed in M4 (Task 1, PR #<n>), per the spec's decision 5; superseded by CLM-0168." Its evidence and verdicts stay in the history.
- [ ] **Step 4: The parents.**
  - **CLM-0003 and CLM-0004** are re-affirmed `supported` over their children, which now include CLM-0174, CLM-0175, CLM-0178 (S2) and CLM-0173, CLM-0176, CLM-0177 (S3).
  - **CLM-0005** is re-affirmed over its wire claims, which now include CLM-0168, CLM-0169 and CLM-0172, and no longer CLM-0087.
  - **CLM-0002** stays `not_enough_info` (the egress proxy and IAM, unchanged since M3); its rationale names CLM-0171 as a new supported child.
  - **CLM-0006** stays `not_enough_info`: its M4 children (CLM-0119 to CLM-0121, CLM-0170, CLM-0179 to CLM-0181) are supported, but CLM-0123 (live CI job) and CLM-0124 (publish dry-run) are M5's, and CLM-0118 is open unless Step 5 decides it. The rationale lists them.
  - **CLM-0001** stays `not_enough_info` while CLM-0002 and CLM-0006 are.
- [ ] **Step 5: CLM-0118.** If Task 17 reports that every v0.1 test ended as pass or as a filed sandbawks defect with no capacity result, add its log as demonstration evidence and record `supported`; then comment on #27 with the run id and close it as completed. Otherwise leave CLM-0118 `not_enough_info`, with the capacity results as the rationale, and leave #27 open.
- [ ] **Step 6: Bugs.** Each refuted claim gets a bug task (Task 19, 20, ...) added under this step with a dated note, a sub-issue, and a fix and re-verification within M4, as in M1. New sandbawks defects from Task 17 are filed on gfd/sandbawks under epic #673.
- [ ] **Step 7: Docs.**
  - **`README.md`:** the coverage table rows that M4 changed: Lifecycle (`connect_sandbox(id, &ConnectOptions)` and `pause_sandbox(id, &PauseOptions)`, the `_with` forms gone), Processes and PTY (`CommandConnectOptions`, `PtyConnectOptions`, request timeouts, no PTY deadline by default), Files (`FileOptions`, `ListOptions`, `max_concurrency`, symlinks), Errors (the classifiers and `TransportError`). "Verification status" becomes "As of <date> (milestone 4)", with the hosted and sandbawks counts, the new ledger counts (Step 9), and a line that the M4 API is the one 0.2.0 will ship. README examples are doctests, so they must compile.
  - **`docs/protocol-notes.md`:** "Not implemented yet" drops the M4 items and lists: milestone 5 (live CI job, publish); per-call request timeouts on REST and pid-only calls; the deferred M3 items (egress proxy, IAM, the volume content API); the unscheduled ones (health probing, logs, secrets, git); the not-planned ones (batch metrics, callable selectors, `secure`, `keep_memory`, OpenAPI regeneration, typed error classes, a PTY keep-alive); and the design's out-of-scope items. "Verification against a live server" gains the M4 runs. The v1 connect row and the "Connect v1 fallback" paragraph are gone (Task 1); check.
  - **`src/lib.rs`:** the crate docs' bullets mention the options structs, the error classifiers and the versioning policy (a link to the README section).
  - **The spec** (`docs/superpowers/specs/2026-10-26-claims-audit-and-publishability-design.md`): under the Milestones table, add "**M4 outcome (<date>).** Closed by epic #<n>: the v1 connect fallback is gone (decision 5 done); the API review is recorded in `docs/api-review.md`; MSRV 1.89. Not revisited: regenerating the REST models from OpenAPI (kept hand-written and cited)." Decision 5's text stays as written.
- [ ] **Step 8: Milestone changelog.** `changelog/<ts>-m4-milestone.md`: what shipped; **every breaking change, old call to new call** (gathered from the per-task entries; this is the list 0.2.0's release notes will use); the ledger counts before and after; the run ids; the sandbawks defects filed; the index of every plan correction by task (as in M2's and M3's); and the follow-ups.
- [ ] **Step 9: Expected ledger counts.** Before M4: 166 active claims, 152 supported, 14 `not_enough_info`. M4 adds 14 claims (CLM-0168 to CLM-0181) and retires CLM-0087 (supported), so 179 active. If every M4 claim and CLM-0119 to CLM-0121 are supported and CLM-0118 is not decided: **168 supported, 11 `not_enough_info`** (CLM-0001, 0002, 0006, 0031, 0032, 0118, 0123, 0124, 0148, 0149, 0153), 0 refuted, 0 unverified. If Step 5 supports CLM-0118: 169 and 10. Run `mcp__claims__audit_report` and compare; explain any difference in the milestone changelog.

> Follow-ups to expect: the volume tests on a team with volumes (CLM-0148, CLM-0149, CLM-0153); per-call request timeouts on REST and pid-only calls; M5 (CLM-0123, CLM-0124).
