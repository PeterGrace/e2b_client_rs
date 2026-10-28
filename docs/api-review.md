# API review (milestone 4)

A review of every public item of `e2b-client` against the
[Rust API Guidelines](https://rust-lang.github.io/api-guidelines/checklist.html),
done in M4 (plan `docs/superpowers/plans/2026-10-28-milestone-4-api-stability.md`,
Task 12, issue #95), after every API-changing task (1 to 11) and before the
0.2.0 release. It backs CLM-0119 (`#[non_exhaustive]` where types may grow),
CLM-0179 (`Send`/`Sync`/`Debug`, enforced by `tests/api_traits.rs`) and
CLM-0180 (this record). The item list is the re-exports in `src/lib.rs` as of
2026-10-28.

## Rules applied

| Guideline | Rule in this crate | Enforced by |
|---|---|---|
| C-CASE | `snake_case` functions, `PascalCase` types and variants, `SCREAMING_SNAKE_CASE` constants | rustc lints |
| C-CONV | `as_*` borrows or copies cheaply (`ConnectCode::as_wire`); `from_*` constructs (`ConnectCode::from_wire`, `ConnectCode::from_plain_http_status`, `Client::from_env`, `ConnectionConfig::from_env`); parsing is `FromStr` plus `EnvdVersion::parse_lenient` | review |
| C-GETTER | Field accessors have no `get_` prefix (`Sandbox::sandbox_id`, `Volume::token`, `CommandHandle::pid`, `CreateSandboxParams::template`). `get_` is kept only where it mirrors an SDK operation of the same name: the remote calls `Client::{get_sandbox_info, get_sandbox_metrics, get_volume}`, `Sandbox::{get_info, get_metrics}` and `Filesystem::get_info`, which make a request, and `Sandbox::get_host`, which builds a host name from a port (SDK `get_host`). None of them reads a field, so they are not getters in the guideline's sense, and the SDK names help porting | review |
| C-CTOR, C-COMMON-TRAITS | Every options or parameters type has `new()` and `Default`, and they agree; options derive `Debug, Clone, PartialEq, Eq` (plus `Copy` when every field is `Copy`). Two deliberate exceptions: `CreateSandboxParams::new(template)` takes the template, and `ConnectionConfigBuilder` is reached through `ConnectionConfig::builder()` (it is a builder, not an options type) | `tests/api_traits.rs` (`every_options_type_has_new_equal_to_default`) |
| C-BUILDER | Options are by-value builders (`fn x(self, ..) -> Self`) with `#[must_use]`; fields are private, so a setter can validate or normalise (zero timeouts, Task 4) | review; `clippy` |
| C-DEBUG | Every public type implements `Debug`; types holding secrets print `[REDACTED]` or leave the field out (CLM-0122) | `missing_debug_implementations` in `Cargo.toml`; `tests/api_traits.rs` |
| C-SEND-SYNC | Clients, service handles, options, results and errors are `Send + Sync`; stream handles are `Send` | `tests/api_traits.rs` |
| C-GOOD-ERR | One `Error` enum, `#[non_exhaustive]`, struct variants `#[non_exhaustive]`, `Send + Sync + 'static`, with classifiers instead of exception classes (decision 1). A variant that wraps a source error does not repeat it in its `Display` (see "Error display" below) | `tests/api_traits.rs`; `tests/error_taxonomy.rs` |
| C-NON-EXHAUSTIVE / C-STRUCT-PRIVATE | Options and handles have private fields. Result types have public fields and are `#[non_exhaustive]` (callers read them, the API may add fields). Enums that mirror an API or protobuf enum that can grow are `#[non_exhaustive]` | review |
| C-STABLE (public dependencies) | No reqwest type is public (`TransportError`); `secrecy` is re-exported; the public dependencies are listed in the README (Task 13) | review; CLM-0170; README 'Versioning' |
| C-FAILURE | Every fallible public function documents `# Errors` | `clippy::missing_errors_doc` and `missing_panics_doc`: no warnings on 2026-10-28 |
| C-EXAMPLE | Every public type and every public async method has an `# Examples` section | `tests/doc_examples.rs` (Task 14) |
| Docs | Every public item has rustdoc | `missing_docs` in `Cargo.toml` |

## Types

"NE" is `#[non_exhaustive]`. "Fields" is how the type is built: **private**
(constructors and setters), **public** (read the fields), or **variants**.
"Debug (manual)" is a hand-written impl that hides secrets.

| Item | Kind | Traits | Send/Sync | Fields | NE | Decision |
|---|---|---|---|---|---|---|
| `Client` | struct | Debug, Clone | both | private | no | Cheap to clone (shared state behind an `Arc`); built with `new`/`from_env` |
| `ConnectionConfig` | struct | Clone, Debug (manual, redacts the key) | both | private | no | Built by `ConnectionConfigBuilder`; accessors without `get_` |
| `ConnectionConfigBuilder` | struct | Default, Debug (manual) | both | private | no | Consuming builder from `ConnectionConfig::builder()`, ending in `build()`; no `new()`, on purpose |
| `Sandbox` | struct | Clone, Debug (manual, redacts tokens) | both | private | no | Handle; accessors without `get_`; remote calls keep SDK names |
| `Commands` | struct | Debug, Clone | both | private | no | Service handle from `Sandbox::commands` |
| `CommandHandle` | struct | Debug | Send | private | no | Stream handle; `next_event(&mut self)` |
| `CommandOptions` | struct | Debug, Clone, PartialEq, Eq, Default (manual: 60 s deadline) | both | private | no | `new()` added in M4 |
| `CommandConnectOptions` | struct | Debug, Clone, PartialEq, Eq, Default (manual: 60 s deadline) | both | private | no | New in M4 (#54); default deadline 60 s, as the SDK |
| `CommandEvent` | enum | Debug, Clone, PartialEq, Eq | both | variants | **yes (M4)** | envd's `process.ProcessEvent` can grow |
| `CommandExit` | struct | Debug, Clone, PartialEq, Eq | both | public | yes | Result type |
| `CommandResult` | struct | Debug, Clone, PartialEq, Eq | both | public | yes | Result type |
| `ProcessInfo` | struct | Debug, Clone, PartialEq, Eq | both | public | yes | Result type |
| `Signal` | enum | Debug, Clone, Copy, PartialEq, Eq, Hash | both | variants | **yes (M4)** | envd's `process.Signal` can grow |
| `Pty` | struct | Debug, Clone | both | private | no | Service handle |
| `PtyHandle` | struct | Debug | Send | private | no | Stream handle |
| `PtyOptions` | struct | Debug, Clone, PartialEq, Eq, Default | both | private | no | `new()` added in M4; no deadline by default (decision 6) |
| `PtyConnectOptions` | struct | Debug, Clone, PartialEq, Eq, Default | both | private | no | New in M4 (#54); no deadline by default |
| `PtySize` | struct | Debug, Clone, Copy, PartialEq, Eq, Hash | both | public (`cols`, `rows`) | **no, on purpose** | A terminal size is two numbers and will not grow; public fields let callers destructure it or write a literal. Built with `PtySize::new` or a literal |
| `PtyEvent` | enum | Debug, Clone, PartialEq, Eq | both | variants | yes | Can grow |
| `StreamInputOptions` | struct | Debug, Clone, Copy, PartialEq, Eq, Default (manual: 60 s deadline) | both | private | no | `new()` added in M4 |
| `InputStream` | struct | Debug | Send | private | no | Stream handle; `close(self)` consumes it |
| `Filesystem` | struct | Debug, Clone | both | private | no | Service handle; every call takes an options reference (decision 3) |
| `FileOptions` | struct | Debug, Clone, Default, PartialEq, Eq | both | private | no | New in M4: user and request timeout |
| `ListOptions` | struct | Debug, Clone, PartialEq, Eq, Default (manual: depth 1) | both | private | no | New in M4: depth, user, request timeout |
| `WriteOptions` | struct | Debug, Clone, PartialEq, Eq, Default (manual: cap 8) | both | private | no | `new()` added in M4; `max_concurrency` (#53), `request_timeout` |
| `WriteEntry` | struct | Debug, Clone, PartialEq, Eq | both | private | no | Built with `WriteEntry::new(path, data)` |
| `WriteInfo` | struct | Debug, Clone, PartialEq, Eq | both | public | yes | Result type |
| `WatchOptions` | struct | Debug, Clone, PartialEq, Eq, Default (manual: 60 s deadline) | both | private | no | `new()` added in M4 |
| `WatchHandle` | struct | Debug | Send | private | no | Stream handle |
| `FileStream` | struct | Debug | Send | private | no | Stream handle |
| `EntryInfo` | struct | Debug, Clone, PartialEq, Eq | both | public | yes | Result type; `file_type` reads `is_symlink` (#30) |
| `FileType` | enum | Debug, Clone, Copy, PartialEq, Eq, Hash | both | variants | yes | envd's `FileType` can grow |
| `FilesystemEvent` | struct | Debug, Clone, PartialEq, Eq | both | public | yes | Result type |
| `FilesystemEventType` | enum | Debug, Clone, Copy, PartialEq, Eq, Hash | both | variants | yes | Can grow |
| `FileUrlOptions` | struct | Debug, Clone, Default, PartialEq, Eq | both | private | no | `new()` added in M4 |
| `CreateSandboxParams` | struct | Debug, Clone, Default, PartialEq, Eq | both | private | no | `new(template)` takes the template, so it is not `Default::default()`; `Default` leaves the template unset and `template()` then reads `DEFAULT_TEMPLATE` (or `DEFAULT_MCP_TEMPLATE` with `mcp`) |
| `ListSandboxesParams` | struct | Debug, Clone, Default, PartialEq, Eq | both | private | no | |
| `SandboxInfo` | struct | Debug, Clone, PartialEq, Eq, Deserialize | both | public | yes | Result type |
| `SandboxPage` | struct | Debug, Clone, PartialEq, Eq | both | public | yes | Result type |
| `SandboxState` | enum | Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize | both | variants | yes | API enum that can grow |
| `SortOrder` | enum | Debug, Clone, Copy, PartialEq, Eq, Hash | both | variants | yes | |
| `ConnectOptions` | struct | Debug, Clone, Copy, Default, PartialEq, Eq | both | private | no | The only form of `connect_sandbox` since Task 6 |
| `PauseOptions` | struct | Debug, Clone, Copy, Default, PartialEq, Eq | both | private | no | The only form of `pause_sandbox`/`Sandbox::pause` since Task 6 |
| `Lifecycle` | struct | Debug, Clone, Copy, Default, PartialEq, Eq | both | private | no | |
| `LifecycleInfo` | struct | Debug, Clone, Copy, PartialEq, Eq, Deserialize | both | public | yes | Result type |
| `OnTimeout` | enum | Debug, Clone, Copy, PartialEq, Eq, Hash | both | variants | yes | `Pause { mode }` is built with `OnTimeout::pause`/`pause_with` |
| `OnResume` | enum | Debug, Clone, Copy, PartialEq, Eq, Hash | both | variants | yes | |
| `SnapshotMode` | enum | Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize | both | variants | yes | |
| `TimeoutAction` | enum | Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize | both | variants | yes | API enum that can grow |
| `MetricsParams` | struct | Debug, Clone, Copy, Default, PartialEq, Eq | both | private | no | |
| `SandboxMetrics` | struct | Debug, Clone, PartialEq, Deserialize | both | public | yes | No `Eq`: it holds `f64`s |
| `ForkOptions` | struct | Debug, Clone, Copy, Default, PartialEq, Eq | both | private | no | |
| `NetworkConfig` | struct | Debug, Clone, Default, PartialEq, Eq, Serialize | both | private | no | |
| `NetworkUpdate` | struct | Debug, Clone, Default, PartialEq, Eq, Serialize | both | private | no | |
| `NetworkRule` | struct | Debug (header values hidden), Clone, Default, PartialEq, Eq, Serialize, Deserialize | both | private | no | Header values are often credentials, so `Debug` prints only the names |
| `NetworkInfo` | struct | Debug, Clone, Default, PartialEq, Eq, Deserialize | both | public | yes | Result type |
| `SnapshotOptions` | struct | Debug, Clone, Default, PartialEq, Eq | both | private | no | |
| `SnapshotInfo` | struct | Debug, Clone, PartialEq, Eq, Deserialize | both | public | yes | Result type |
| `ListSnapshotsParams` | struct | Debug, Clone, Default, PartialEq, Eq | both | private | no | |
| `SnapshotPage` | struct | Debug, Clone, PartialEq, Eq | both | public | yes | Result type |
| `Volume` | struct | Clone, Debug (manual, leaves the token out) | both | private | no | Accessors without `get_` |
| `VolumeInfo` | struct | Debug, Clone, PartialEq, Eq, Hash, Deserialize | both | public | yes | Result type |
| `VolumeMount` | struct | Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize | both | public | yes | Built with `VolumeMount::new` |
| `EnvdVersion` | struct | Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Display, FromStr | both | private | no | Ordered, so version gates compare; `new` is `const` |
| `Error` | enum | Debug, Display, Error | both | variants | yes; struct variants too (M4) | Classifiers (decision 1); `TransportError` hides reqwest |
| `TransportError` | struct | Debug, Display, Error | both | private | no | New in M4: `is_timeout`, `is_connect` |
| `ConnectCode` | enum | Debug, Clone, Copy, PartialEq, Eq, Hash, Display | both | variants | **no, on purpose** | The Connect protocol's 16 codes are the closed set of gRPC status codes; a code the crate does not recognise is reported as one of them, so an exhaustive `match` is useful and stays valid |
| `Result<T>` | type alias | | | | | `std::result::Result<T, Error>` |
| `secrecy` | crate re-export | | | | | `pub use secrecy;` (decision 4), so callers name the version the crate uses |

## Constants

All `SCREAMING_SNAKE_CASE`, documented with their SDK source.

| Item | Type | Decision |
|---|---|---|
| `DEFAULT_DOMAIN` | `&str` | SDK default domain |
| `DEFAULT_REQUEST_TIMEOUT` | `Duration` | SDK default request timeout |
| `ENVD_PORT` | `u16` | envd's port |
| `MCP_PORT` | `u16` | MCP gateway port |
| `DEFAULT_MCP_TEMPLATE` | `&str` | Template used when `CreateSandboxParams::mcp` is set |
| `DEFAULT_TEMPLATE` | `&str` | SDK default template |
| `MAX_FORK_COUNT` | `u32` | API limit on one fork call |
| `ALL_TRAFFIC` | `&str` | CIDR for "every address" in network rules |
| `DEFAULT_COMMAND_TIMEOUT` | `Duration` | SDK `commands.run`/`connect` default |
| `DEFAULT_PTY_TIMEOUT` | `Duration` | Kept as the SDK's value for callers who want parity; not the crate's default (decision 6) |
| `DEFAULT_STREAM_INPUT_TIMEOUT` | `Duration` | SDK `STREAM_INPUT_TIMEOUT` |
| `DEFAULT_WATCH_TIMEOUT` | `Duration` | SDK `watch_dir` default |
| `DEFAULT_UPLOAD_CONCURRENCY` | `usize` | New in M4 (#53): `WriteOptions` default cap |

## Error display

A variant that wraps another error keeps the detail in
[`source()`](https://doc.rust-lang.org/std/error/trait.Error.html#method.source)
and leaves it out of its own `Display`, so a chain printer (anyhow's `{:#}`,
`tracing`'s error chains) prints it once:

| Variant | `Display` | `source()` | Decision |
|---|---|---|---|
| `Error::Transport` | `HTTP transport error` | `TransportError` | Task 2 |
| `Error::Decode` | `could not decode response` | `serde_json::Error` | **Changed in M4 (Task 12)**: it was `could not decode response: {0}` with a `#[from]` source, which printed the serde_json message twice in a chain (brooke-reviews, PR #102). Pinned by `tests/error_taxonomy.rs` (`a_decode_error_shows_its_detail_once`). Callers that showed only `err.to_string()` should print the chain (`{:#}` in anyhow, or walk `source()`) |

The other variants hold strings or numbers, not errors, so their `Display`
carries the message and `source()` is `None`.

## Method shape (decision 3)

- One method per operation. Calls that can grow take a trailing `&XOptions`;
  `connect_sandbox_with`, `pause_sandbox_with` and `Sandbox::pause_with` were
  removed in Task 6.
- `Client` and `Sandbox` both offer the sandbox operations (`Client` by id,
  `Sandbox` on itself), as the SDK's class and instance methods do.
- Booleans that report "did it exist" or "did it change" (`kill_sandbox`,
  `Sandbox::kill`, `Commands::kill`, `Pty::kill`, `pause_sandbox`,
  `delete_snapshot`, `destroy_volume`, `make_dir`) return `Result<bool>` as
  the SDK does.

## Known limits

From the plan's Scope ("Deferred") table and this review:

- **Per-call request timeouts** exist only on calls that take an options
  struct reaching envd. The REST calls (`Client::*`, the `Sandbox` lifecycle
  calls) and the pid-only envd calls (`send_stdin`, `close_stdin`,
  `send_signal`, `kill`, `resize`, `Commands::list`) use the configured
  request timeout. Adding options to them later is a breaking change, so it
  waits for a semver-breaking release (a 0.x minor bump). Unscheduled.
- **The REST models are hand-written**, cited row by row in
  `docs/protocol-notes.md` and live-verified; regenerating them from the
  OpenAPI spec is not planned.
- **No typed error variant per SDK exception class**: the classifier
  methods (`is_auth`, `is_rate_limited`, ...) answer the same questions
  (pgrace, 2026-10-28). Not planned.
- **No idle-based PTY keep-alive** (#55): a PTY has no deadline by default
  instead (pgrace, 2026-10-28). Not planned.
- **Release work** (live CI job, publish dry-run, `publish = true`,
  CHANGELOG, 0.2.0; CLM-0123, CLM-0124) is M5.
- **Volume operations are verified by mock only**: hosted E2B answers
  `POST /volumes` with 403 for this team, so CLM-0148, CLM-0149 and
  CLM-0153 wait for a team with volumes enabled. Unscheduled.
- **No SOCKS5 egress proxy and no IAM tokens** (CLM-0031, CLM-0032, and with
  them CLM-0002). Unscheduled (M3's decision).
- **Result types are `#[non_exhaustive]`**, so code outside the crate cannot
  construct them; tests that need one run against a mock server (as this
  crate's own tests do with wiremock).
- **`Error::Decode` holds a `serde_json::Error`**, so serde_json 1.x is a
  public dependency; the README lists it (Task 13).
- **`get_` names** on the SDK-mirroring operations (C-GETTER above) stay.

## Example coverage (Task 14)

`tests/doc_examples.rs` requires an `# Examples` section on every public type
re-exported from `lib.rs` and every `pub async fn`, and fails naming each
item that lacks one. Builder setters and accessors are covered by their
type's example. `missing_docs` (Task 12) keeps every public item documented.

| | Before (`origin/main` at 89218d1) | After (issue #97) |
|---|---|---|
| Re-exported public types | 28 of 67 | 67 of 67 |
| Public async functions | 18 of 69 | 69 of 69 |

Before M4 the plan session counted 13 of 72 async functions. The examples
that need a sandbox or the API are `no_run` (compiled by `cargo test`, never
run); the pure-value ones (options, `ConnectionConfig`, `EnvdVersion`,
`ConnectCode`, `CreateSandboxParams`, `OnTimeout`, `SortOrder`) run as
doctests.
