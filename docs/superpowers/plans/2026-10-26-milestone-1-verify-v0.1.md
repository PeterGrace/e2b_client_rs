# Milestone 1 (Verify v0.1) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Prove or disprove, with live evidence from hosted E2B and sandbawks, every claim v0.1 makes, and record a verdict for each in the claims ledger.

**Architecture:** A live test harness (`tests/live_support/`) gives each test one throwaway sandbox, guarded so it is killed even on panic, tagged with a per-run metadata id, and swept at the end of each run. There is one `#[ignore]`d live test per claim, named `clm_NNNN_<slug>` after the ledger claim it backs. They are split by area into `tests/live_lifecycle.rs`, `tests/live_commands.rs` and `tests/live_files.rs`. `just test-live <target>` runs them against one target and writes a run log. The coordinator session alone writes the ledger (`claims.json`): it turns run logs and the S4 evidence map into evidence and verdicts.

**Tech Stack:** Rust 2024, tokio, the crate's own public API, `dotenvy` (dev-only), `just`, claims MCP.

**Design:** `docs/superpowers/specs/2026-10-26-claims-audit-and-publishability-design.md`, sections "Claim hierarchy", "M1 verification harness" and "Milestones".

---

## Scope

In:

- the claims ledger with all 126 claims, CLM-0001 to CLM-0126. It was populated while planning and is committed with this plan as `claims.json`;
- the live harness and one live test per v0.1 claim in S1 to S3;
- an S4 evidence map: one mock or unit test per S4 wire claim, plus CLM-0122 (Debug redaction), with tests written where none asserts the claim;
- a live run on hosted E2B and on sandbawks, with run logs checked in;
- verdicts for every `v0.1` claim, and fixes for every crate bug the runs refute (added as tasks while M1 runs, see Task 8);
- sandbawks-only failures filed on gfd/sandbawks (Forgejo).

Deferred:

| Item | Claims | Goes to |
|---|---|---|
| StreamInput, PTY, batch write, watch_dir, signed URLs, streaming upload | CLM-0046..0049, 0062..0066 | M2 |
| get_host, metrics, snapshots, network, extra create fields | CLM-0028..0032 | M3 |
| `#[non_exhaustive]` review, missing_docs, MSRV/semver, dropping the v1 fallback | CLM-0119..0121, 0087 | M4 |
| Live CI job, publish dry-run | CLM-0123, 0124 | M5 |

## Decisions taken while planning (2026-10-26)

1. **Merging.** pgrace authorized the coordinator to merge the plan PR and the M1 sub-issue PRs once CI is green and brooke-reviews' review has been answered.
2. **Ledger.** `claims.json` is committed to the repo. Only the coordinator session writes it, because the claims MCP server writes `claims.json` in the coordinator's own working directory. Subagents never call claims write tools. They hand over evidence as files (run logs, the S4 evidence map), and the coordinator records it. `.claims.json.lock` is gitignored.
3. **Keys.** These live in `.env` at the main checkout's root (the directory that contains `.worktrees/`): `E2B_API_KEY_HOSTED`, `E2B_API_KEY_SANDBAWKS` and `E2B_DOMAIN_SANDBAWKS`. `dotenvy::dotenv_iter` finds that file by walking up from any worktree under `.worktrees/`. Process environment variables override it, so CI can use secrets.
4. **The ledger was populated during planning**, not as a task, so the test names in this plan can cite real claim ids.
5. **sandbawks API URL.** Both `https://api.<domain>` and `https://<domain>` route on sandbawks (probed 2026-10-26: both answer 401 unauthenticated). So the default `https://api.<domain>` is used and no `E2B_API_URL_*` variable is needed.
6. **sandbawks has v2 routes.** `POST /v2/sandboxes/x/connect` now answers 401, not 404, so sandbawks ≥ 0.1.28 is deployed. CLM-0087 (v1 fallback) is still verified by its mock test only.

## Simplifications and deviations relative to the design

- **Live test parallelism.** Each test uses its own sandbox as designed, but the binaries run with `--test-threads=4` instead of serially, to keep a run to a few minutes. Tests share nothing except the run id.
- **Sandbox timeout.** The design says 60 s per sandbox. `live_support::params()` uses 60 s; the few tests that wait longer (pause/resume, auto-pause) set their own timeout explicitly.

## Verified facts the code relies on

| Fact | Source |
|---|---|
| `dotenvy::dotenv_iter()` exists, searches from the current dir upward, and does not modify the environment | `~/.cargo/registry/src/index.crates.io-*/dotenvy-0.15.7/src/lib.rs:370`, `src/find.rs:33-49` |
| tokio feature `rt-multi-thread` implies `rt`, so `tokio::runtime::Builder::new_current_thread` is available to tests | `tokio-1.54.1/Cargo.toml:119-120` |
| `#[tokio::test]` defaults to a current-thread runtime. A blocking join inside `Drop` would starve that runtime's own connection tasks, so the drop guard builds a fresh `Client` on its own thread and runtime | tokio `#[tokio::test]` docs (`flavor` defaults to `current_thread`) |
| Cargo test cwd is the package root, the worktree, so `dotenv_iter` starts there | cargo reference, "cargo test" (CWD is the package root) |
| `chrono` is built without the `clock` feature, so tests compare `end_at.timestamp()` with `SystemTime::now()` instead of `Utc::now()` | `Cargo.toml:23` |
| `Sandbox: Clone`, with a custom `Debug` that omits tokens | `src/sandbox.rs:41-60` |
| `Client: Debug + Clone`; `Client::new(ConnectionConfig) -> Result<Client>` | `src/client.rs:53-81` |
| `ConnectionConfig::builder().api_key(..).domain(..).build() -> Result<ConnectionConfig>`. Builder values beat env vars, which beat defaults | `src/config.rs:91-336` |
| `ListSandboxesParams: Clone + Default`; `.metadata(k,v)`, `.state(s)` (accumulates), `.limit(n)`, `.next_token(t)`; `new()` lists running and paused | `src/models.rs:300-380` |
| `SandboxPage { sandboxes: Vec<SandboxInfo>, next_token: Option<String> }` | `src/models.rs:432-440` |
| `SandboxInfo` fields: `sandbox_id`, `template_id`, `started_at`, `end_at: DateTime<Utc>`, `state`, `metadata: HashMap<String,String>` | `src/models.rs:255-300` |
| `SandboxState: Copy + Eq`, variants `Running`, `Paused`, `#[non_exhaustive]` | `src/models.rs:221-230` |
| `Error::is_not_found()` is true for `SandboxNotFound`, `Rpc{NotFound}`, and `Api`/`Envd` with status 404 | `src/error.rs:120-131` |
| `ConnectCode::DeadlineExceeded` exists | `src/error.rs:136-170` |
| `CommandEvent::{Stdout(Bytes), Stderr(Bytes), Exit(CommandExit)}`; `CommandExit { exit_code: i32, error: Option<String> }` | `src/commands.rs:166-186` |
| `CommandResult { stdout, stderr: String, exit_code: i32, error: Option<String> }` with `success()` | `src/commands.rs:189-210` |
| `ProcessInfo { pid: u32, tag, cmd: String, args: Vec<String>, envs, cwd }` | `src/commands.rs:213-230` |
| `Commands::{start, run}(&str, &CommandOptions)`, `connect(pid, Option<Duration>)`, `list()`, `send_stdin(pid, impl AsRef<[u8]>)`, `close_stdin(pid)`, `send_signal(pid, Signal)`, `kill(pid) -> bool`; `CommandHandle::{pid, next_event, wait}` | `src/commands.rs:279-641` |
| `CommandOptions` builders: `env`, `cwd`, `user`, `stdin(bool)`, `timeout(Option<Duration>)` | `src/commands.rs:79-141` |
| `Signal::{Sigterm, Sigkill}` | `src/commands.rs:149-163` |
| `Filesystem::write(&str, impl Into<Bytes>, Option<&str>) -> WriteInfo { name, path, file_type }`; `read -> Bytes`; `read_to_string`; `list(path, depth, user)`; `get_info`, `exists`, `make_dir -> bool`, `rename(src, dst, user) -> EntryInfo`, `remove` | `src/filesystem.rs:165-420` |
| `EntryInfo { name, file_type: FileType, path, size: i64, permissions, owner, .. }`; `FileType::{File, Directory, Symlink, Unknown}` | `src/filesystem.rs:20-100` |
| Hosted E2B `base` template has users `user` (default) and `root`, and ships `bash`, `coreutils` and `seq` | E2B docs, "Sandbox users"; refuted live if wrong |
| `.env` is gitignored | `.gitignore:2` |

## Conventions for every task

- **Gates:** `just lint` (fmt check plus `clippy --all-targets -D warnings`), `just test` and `just doc` must pass before pushing. Live tasks additionally run `just test-live <target>`.
- **Tests:** live tests are `#[tokio::test]` plus `#[ignore = "live: run with `just test-live <target>`"]`, named `clm_NNNN_<slug>`, one claim each, Arrange-Act-Assert. Use `.expect("<what>")` in tests, never `.unwrap()`. No `println!`/`dbg!`; the only output is `eprintln!` warnings in `live_support`.
- **Ledger:** subagents never call `mcp__claims__*` write tools (`add_*`, `record_verdict`, `update_claim`, `retire_claim`). Read tools are fine.
- **Changelog:** one entry per task, `changelog/<UTC ts>-m1-task-<NN>-<slug>.md`, with the timestamp from `date -u +%Y-%m-%dT%H-%M-%SZ`.
- **Logs:** live run logs go to `target/live-logs/` inside your own worktree. Never write them to a shared scratchpad.
- **Processes:** never kill processes by pattern; kill only your own PIDs.
- The code in this plan has not been compiled. If an implementation has to differ, fix this plan in the same PR (inline, or with a dated `> **Corrected (issue #N).**` note), then grep later tasks for the changed symbol.

## File map

| File | Task |
|---|---|
| `claims.json`, `.gitignore` (add `.claims.json.lock`) | plan PR; Task 8 |
| `Cargo.toml` (dev-dep `dotenvy`) | 1 |
| `justfile` (`test-live`, remove `test-integration`) | 1 |
| `CONTRIBUTING.md` (live test section) | 1 |
| `tests/integration.rs` (deleted) | 1 |
| `tests/live_support/mod.rs` | 1 |
| `tests/live_sweep.rs` | 1 |
| `tests/live_lifecycle.rs` | 1 (create), 2 |
| `tests/live_commands.rs` | 1 (create), 3 |
| `tests/live_files.rs` | 1 (create), 4 |
| `tests/rest_api.rs`, `tests/envd_api.rs`, `src/**` `#[cfg(test)]` modules | 5 |
| `docs/verification/s4-evidence-map.md` | 5 |
| `docs/verification/runs/<run id>.log` | 6, 7 |
| `docs/verification/m1-live-results.md` | 6, 7 (one section each) |
| `README.md`, `docs/protocol-notes.md` ("Verification against a live server"), milestone changelog | 8 |

## Task dependencies

```
1 -> 2
1 -> 3
1 -> 4
2, 3, 4 -> 6
2, 3, 4 -> 7
5 -> 8
6, 7 -> 8
```

Tasks 1 and 5 can start immediately. Tasks 2, 3 and 4 run in parallel, since each owns one test file. Tasks 6 and 7 run in parallel and both append a section to `docs/verification/m1-live-results.md`; on a conflict, keep both sections. Task 8 is the coordinator's.

---

### Task 1: Live test harness

**Files:**
- Modify: `Cargo.toml` (dev-dependencies)
- Modify: `justfile`
- Modify: `CONTRIBUTING.md` (Tooling table, Tests section)
- Delete: `tests/integration.rs`
- Create: `tests/live_support/mod.rs`, `tests/live_sweep.rs`, `tests/live_lifecycle.rs`, `tests/live_commands.rs`, `tests/live_files.rs`

- [ ] **Step 1: Add the dev-dependency**

In `Cargo.toml` under `[dev-dependencies]`, add:

```toml
# Reads the live-test keys from the nearest .env without touching the process environment.
dotenvy = "0.15.7"
```

- [ ] **Step 2: Write `tests/live_support/mod.rs`**

```rust
//! Shared helpers for the live test suites (`tests/live_*.rs`).
//!
//! Every live test is `#[ignore]`d. Run them with `just test-live <target>`.
//! Settings come from the process environment first, then from the nearest
//! `.env` found walking up from the package directory:
//!
//! | Variable | Meaning |
//! |---|---|
//! | `E2B_TEST_TARGET` | `hosted` or `sandbawks` (required) |
//! | `E2B_TEST_RUN_ID` | tags every sandbox of this run (default: clock seconds and pid) |
//! | `E2B_API_KEY_<TARGET>` | API key for the target (required) |
//! | `E2B_DOMAIN_<TARGET>` | domain (hosted default `e2b.app`; required for sandbawks) |
//! | `E2B_TEMPLATE_<TARGET>` | template (default `base`) |
//!
//! `<TARGET>` is `HOSTED` or `SANDBAWKS`. The standard `E2B_API_KEY`,
//! `E2B_API_URL`, `E2B_DOMAIN` and `E2B_SANDBOX_URL` variables must be unset,
//! so a run can never silently talk to the wrong server.

// Each test binary uses a different subset of these helpers; Rust compiles
// this module once per binary and would warn about the unused ones.
#![allow(dead_code)]

use std::collections::HashMap;
use std::future::Future;
use std::ops::Deref;
use std::sync::OnceLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use e2b_client::{Client, ConnectionConfig, CreateSandboxParams, Sandbox};

/// Metadata key carrying the run id on every sandbox a live test creates.
pub const RUN_METADATA_KEY: &str = "e2b-client-test";

/// Default lifetime of a test sandbox.
pub const SANDBOX_TIMEOUT: Duration = Duration::from_secs(60);

/// Which server a live run talks to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// Hosted E2B, the canonical server.
    Hosted,
    /// pgrace's E2B-compatible sandbawks gateway.
    Sandbawks,
}

impl Target {
    fn suffix(self) -> &'static str {
        match self {
            Self::Hosted => "HOSTED",
            Self::Sandbawks => "SANDBAWKS",
        }
    }
}

/// The `.env` values, read once. `dotenv_iter` only reads the file; unlike
/// `dotenvy::dotenv()` it never mutates the process environment (which is
/// `unsafe` in Rust 2024).
fn dotenv_values() -> &'static HashMap<String, String> {
    static VALUES: OnceLock<HashMap<String, String>> = OnceLock::new();
    VALUES.get_or_init(|| {
        dotenvy::dotenv_iter()
            .map(|entries| entries.filter_map(Result::ok).collect())
            .unwrap_or_default()
    })
}

/// A setting from the process environment, else `.env`; empty counts as unset.
fn setting(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .or_else(|| dotenv_values().get(name).cloned())
        .filter(|value| !value.is_empty())
}

/// The target selected by `E2B_TEST_TARGET`.
pub fn target() -> Target {
    match setting("E2B_TEST_TARGET").as_deref() {
        Some("hosted") => Target::Hosted,
        Some("sandbawks") => Target::Sandbawks,
        other => panic!("E2B_TEST_TARGET must be `hosted` or `sandbawks`, got {other:?}"),
    }
}

/// The id tagging every sandbox of this run.
pub fn run_id() -> &'static str {
    static RUN_ID: OnceLock<String> = OnceLock::new();
    RUN_ID.get_or_init(|| {
        setting("E2B_TEST_RUN_ID").unwrap_or_else(|| {
            let seconds = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|elapsed| elapsed.as_secs())
                .unwrap_or_default();
            format!("{seconds}-{}", std::process::id())
        })
    })
}

/// Seconds since the Unix epoch, for comparing with `SandboxInfo::end_at`.
pub fn unix_now() -> i64 {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after 1970")
        .as_secs();
    i64::try_from(seconds).expect("seconds fit in i64")
}

/// A client for the selected target.
pub fn client() -> Client {
    for name in [
        "E2B_API_KEY",
        "E2B_API_URL",
        "E2B_DOMAIN",
        "E2B_SANDBOX_URL",
    ] {
        assert!(
            std::env::var_os(name).is_none(),
            "unset {name}: live tests take their settings from E2B_*_<TARGET>"
        );
    }
    let target = target();
    let suffix = target.suffix();
    let api_key = setting(&format!("E2B_API_KEY_{suffix}"))
        .unwrap_or_else(|| panic!("E2B_API_KEY_{suffix} must be set"));
    let mut builder = ConnectionConfig::builder().api_key(api_key);
    match (target, setting(&format!("E2B_DOMAIN_{suffix}"))) {
        (_, Some(domain)) => builder = builder.domain(domain),
        (Target::Hosted, None) => {}
        (Target::Sandbawks, None) => panic!("E2B_DOMAIN_SANDBAWKS must be set"),
    }
    Client::new(builder.build().expect("live config")).expect("live client")
}

/// The template for the selected target.
pub fn template() -> String {
    setting(&format!("E2B_TEMPLATE_{}", target().suffix())).unwrap_or_else(|| "base".into())
}

/// Create parameters every live sandbox starts from: the target's template,
/// [`SANDBOX_TIMEOUT`] and the run-id metadata.
pub fn params() -> CreateSandboxParams {
    CreateSandboxParams::new(template())
        .timeout(SANDBOX_TIMEOUT)
        .metadata(RUN_METADATA_KEY, run_id())
}

/// A sandbox that is killed when dropped, even if the test panics.
#[derive(Debug)]
pub struct LiveSandbox {
    client: Client,
    sandbox: Sandbox,
}

impl LiveSandbox {
    /// Create a sandbox from `params` (which should come from [`params`]).
    pub async fn create(params: &CreateSandboxParams) -> Self {
        let client = client();
        let sandbox = client.create_sandbox(params).await.expect("create sandbox");
        Self { client, sandbox }
    }

    /// Create a sandbox with the default [`params`].
    pub async fn new() -> Self {
        Self::create(&params()).await
    }

    /// The client that created this sandbox.
    pub fn client(&self) -> &Client {
        &self.client
    }
}

// `Deref` lets a test call `Sandbox` methods directly on a `LiveSandbox`,
// the way a Python wrapper class would forward attribute access.
impl Deref for LiveSandbox {
    type Target = Sandbox;

    fn deref(&self) -> &Sandbox {
        &self.sandbox
    }
}

impl Drop for LiveSandbox {
    fn drop(&mut self) {
        // `Drop` cannot be async. Kill from a fresh thread with its own runtime
        // and its own `Client`: the test's current-thread runtime is blocked
        // in this `join`, so connections owned by it could not make progress.
        let sandbox_id = self.sandbox.sandbox_id().to_owned();
        let thread_sandbox_id = sandbox_id.clone();
        let outcome = std::thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map(|runtime| runtime.block_on(client().kill_sandbox(&thread_sandbox_id)))
        })
        .join();
        if !matches!(outcome, Ok(Ok(Ok(_)))) {
            eprintln!("warning: could not kill sandbox {sandbox_id}; the sweep will retry");
        }
    }
}

/// Poll `probe` every 2 s until it returns `Some`, panicking after `limit`.
pub async fn poll_until<T, F, Fut>(what: &str, limit: Duration, mut probe: F) -> T
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Option<T>>,
{
    let deadline = tokio::time::Instant::now() + limit;
    loop {
        if let Some(value) = probe().await {
            return value;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "timed out after {limit:?} waiting for {what}"
        );
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}
```

- [ ] **Step 3: Write `tests/live_sweep.rs`**

```rust
//! Kills every sandbox this run left behind. `just test-live` runs it last.

mod live_support;

use e2b_client::{ListSandboxesParams, SandboxState};
use live_support::{RUN_METADATA_KEY, client, run_id};

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn sweep_leftover_sandboxes_of_this_run() {
    let client = client();
    let base = ListSandboxesParams::new()
        .metadata(RUN_METADATA_KEY, run_id())
        .state(SandboxState::Running)
        .state(SandboxState::Paused);
    let mut leftovers = Vec::new();
    let mut params = base.clone();
    loop {
        let page = client.list_sandboxes(&params).await.expect("list run sandboxes");
        leftovers.extend(page.sandboxes.into_iter().map(|info| info.sandbox_id));
        match page.next_token {
            Some(token) => params = base.clone().next_token(token),
            None => break,
        }
    }
    for sandbox_id in &leftovers {
        client.kill_sandbox(sandbox_id).await.expect("kill leftover sandbox");
    }
    eprintln!("sweep: killed {} leftover sandbox(es) of run {}", leftovers.len(), run_id());
}
```

- [ ] **Step 4: Port the three existing integration tests**

Create `tests/live_lifecycle.rs`:

```rust
//! Live tests for the REST lifecycle claims (S1). One test per ledger claim.

mod live_support;

use std::time::Duration;

use e2b_client::{CommandOptions, SandboxState};
use live_support::{LiveSandbox, params};

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0014_connect_resumes_paused_sandbox_with_files() {
    // Arrange
    let sandbox = LiveSandbox::create(&params().timeout(Duration::from_secs(180))).await;
    sandbox
        .files()
        .write("/tmp/state.txt", "persisted", None)
        .await
        .expect("write");
    assert!(sandbox.pause().await.expect("pause"));

    // Act
    let resumed = sandbox
        .client()
        .connect_sandbox(sandbox.sandbox_id(), Some(Duration::from_secs(120)))
        .await
        .expect("resume");

    // Assert
    let text = resumed
        .files()
        .read_to_string("/tmp/state.txt", None)
        .await
        .expect("read after resume");
    assert_eq!(text, "persisted");
    let result = resumed
        .commands()
        .run("echo resumed", &CommandOptions::default())
        .await
        .expect("run after resume");
    assert_eq!(result.stdout, "resumed\n");
    let info = sandbox
        .client()
        .get_sandbox_info(sandbox.sandbox_id())
        .await
        .expect("info after resume");
    assert_eq!(info.state, SandboxState::Running);
}
```

Create `tests/live_commands.rs`:

```rust
//! Live tests for the envd process claims (S2). One test per ledger claim.

mod live_support;

use e2b_client::CommandOptions;
use live_support::LiveSandbox;

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0033_run_returns_output_and_exit_code() {
    let sandbox = LiveSandbox::new().await;

    let result = sandbox
        .commands()
        .run("echo out; echo err >&2; exit 3", &CommandOptions::default())
        .await
        .expect("a non-zero exit is not an Err");

    assert_eq!(result.stdout, "out\n");
    assert_eq!(result.stderr, "err\n");
    assert_eq!(result.exit_code, 3);
    assert!(!result.success());
}
```

Create `tests/live_files.rs`:

```rust
//! Live tests for the envd filesystem claims (S3). One test per ledger claim.

mod live_support;

use live_support::LiveSandbox;

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0051_read_to_string_returns_text() {
    let sandbox = LiveSandbox::new().await;
    sandbox
        .files()
        .write("/tmp/e2b-client-rs.txt", "hello from rust", None)
        .await
        .expect("write");

    let text = sandbox
        .files()
        .read_to_string("/tmp/e2b-client-rs.txt", None)
        .await
        .expect("read_to_string");

    assert_eq!(text, "hello from rust");
}
```

Delete `tests/integration.rs`.

- [ ] **Step 5: Replace the `test-integration` recipe in `justfile`**

Remove the `test-integration` recipe and add:

> **Corrected (issue #5, 2026-10-26).** Both `cargo test` calls pass
> `--show-output`. Without it libtest swallows stderr from passing tests, so the
> sweep's `killed N leftover sandbox(es)` line and the drop guard's warnings
> never reached the run log. The two comment lines are also swapped, because
> `just --list` shows only the last comment line above a recipe. The `test`
> recipe's comment now says "live tests" instead of "integration tests".
>
> **Corrected (issue #5, 2026-10-26, review).** The run id ends in the shell's
> pid (`-$$`). With second resolution alone, two runs against the same target
> started in the same second, for example from two worktrees, would share a tag,
> and each sweep would kill the other run's sandboxes.

```just
# Keys come from E2B_API_KEY_<TARGET> (env or the nearest .env). Writes target/live-logs/<run id>.log.
# Live tests against a real server: `just test-live hosted` or `just test-live sandbawks`.
test-live target:
    #!/usr/bin/env bash
    set -uo pipefail
    export E2B_TEST_TARGET="{{target}}"
    export E2B_TEST_RUN_ID="$(date -u +%Y%m%dT%H%M%SZ)-{{target}}-$$"
    mkdir -p target/live-logs
    log="target/live-logs/${E2B_TEST_RUN_ID}.log"
    cargo test --test live_lifecycle --test live_commands --test live_files \
        -- --ignored --test-threads=4 --show-output 2>&1 | tee "$log"
    status=${PIPESTATUS[0]}
    cargo test --test live_sweep -- --ignored --show-output 2>&1 | tee -a "$log"
    echo "run log: $log"
    exit "$status"
```

- [ ] **Step 6: Update `CONTRIBUTING.md`**

In the Tooling table, replace the `just test-integration` row with:

```markdown
| `just test-live <target>` | Live tests against `hosted` E2B or `sandbawks` (see below) |
```

Replace the "Integration tests" bullet in the Tests section with:

````markdown
- Live tests (`tests/live_*.rs`) are `#[ignore]`d. Each creates real
  sandboxes, killed on drop and swept at the end of the run. They read keys
  from the environment or the nearest `.env` (the main checkout's `.env` is
  found from any worktree):

  ```sh
  # .env
  E2B_API_KEY_HOSTED=...
  E2B_API_KEY_SANDBAWKS=...
  E2B_DOMAIN_SANDBAWKS=...
  ```

  Then run `just test-live hosted` or `just test-live sandbawks`. Each test is
  named `clm_NNNN_*` after the claim in `claims.json` it backs, and the run log
  in `target/live-logs/` is the evidence for that claim.
````

> **Corrected (issue #5, 2026-10-26).** The "Testing against a self-hosted
> E2B-compatible server" subsection also called `just test-integration` with the
> standard `E2B_*` variables, which the harness now refuses. It was rewritten to
> use the `sandbawks` target (`E2B_API_KEY_SANDBAWKS`, `E2B_DOMAIN_SANDBAWKS`,
> optional `E2B_TEMPLATE_SANDBAWKS`). It also notes that the harness does not
> support a single shared envd endpoint (`E2B_SANDBOX_URL`). See `CONTRIBUTING.md`.

- [ ] **Step 7: Run the gates and one live smoke run**

Run: `just lint && just test && just doc`
Expected: all pass. `cargo test` lists the live tests as ignored.

Run: `just test-live hosted`
Expected: 3 passed, and the sweep reports `killed 0 leftover sandbox(es)`. If hosted fails here, record the output in the PR and carry on: Task 6 owns the verdicts.

- [ ] **Step 8: Changelog and commit**

Write `changelog/<ts>-m1-task-01-live-harness.md` (what changed, how to run it, smoke-run result). Then:

```bash
git add Cargo.toml Cargo.lock justfile CONTRIBUTING.md tests/ changelog/
git commit -m "test: live harness with per-claim tests, drop guard and sweep"
```

---

### Task 2: Live lifecycle tests (S1)

**Files:**
- Modify: `tests/live_lifecycle.rs`

Add each test below to `tests/live_lifecycle.rs`, extend its `use` lines to `use std::collections::HashSet;`, `use std::time::Duration;`, `use e2b_client::{CommandOptions, EnvdVersion, Error, ListSandboxesParams, SandboxState};` and `use live_support::{LiveSandbox, RUN_METADATA_KEY, client, params, poll_until, run_id, unix_now};`. Every test carries `#[tokio::test]` and `#[ignore = "live: run with `just test-live <target>`"]`; they are shown once here and omitted below for brevity, but each test needs both.

A sandbox id that cannot exist: `const MISSING_ID: &str = "i0000000000000000000z";`.

- [ ] **Step 1: Create and metadata (CLM-0007, 0008, 0009, 0010)**

```rust
const MISSING_ID: &str = "i0000000000000000000z";

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0007_create_returns_running_sandbox() {
    let sandbox = LiveSandbox::new().await;

    assert!(!sandbox.sandbox_id().is_empty());
    assert!(!sandbox.template_id().is_empty());
    let info = sandbox.get_info().await.expect("info");
    assert_eq!(info.state, SandboxState::Running);
    assert_eq!(info.sandbox_id, sandbox.sandbox_id());
    // `parse_lenient` maps an unparseable version to 99.99.99, so compare the
    // parsed handle version with the raw string the API reports.
    assert!(!info.envd_version.is_empty(), "API reported no envd version");
    assert_eq!(sandbox.envd_version(), EnvdVersion::parse_lenient(&info.envd_version));
}

async fn clm_0008_create_metadata_is_visible_in_info() {
    let sandbox = LiveSandbox::create(&params().metadata("purpose", "clm-0008")).await;

    let info = sandbox.get_info().await.expect("info");

    assert_eq!(info.metadata.get("purpose").map(String::as_str), Some("clm-0008"));
    assert_eq!(info.metadata.get(RUN_METADATA_KEY).map(String::as_str), Some(run_id()));
}

async fn clm_0009_create_env_vars_reach_commands() {
    let sandbox = LiveSandbox::create(&params().env("GREETING", "hi from create")).await;

    let result = sandbox
        .commands()
        .run("echo \"$GREETING\"", &CommandOptions::default())
        .await
        .expect("run");

    assert_eq!(result.stdout, "hi from create\n");
}

async fn clm_0010_create_timeout_sets_end_at() {
    let sandbox = LiveSandbox::create(&params().timeout(Duration::from_secs(90))).await;

    let info = sandbox.get_info().await.expect("info");

    let lifetime = info.end_at.timestamp() - info.started_at.timestamp();
    assert!((80..=100).contains(&lifetime), "end_at - started_at = {lifetime}s, want about 90");
}
```

- [ ] **Step 2: Auto-pause and internet access (CLM-0011, 0012)**

```rust
async fn clm_0011_auto_pause_pauses_at_timeout() {
    let sandbox =
        LiveSandbox::create(&params().timeout(Duration::from_secs(15)).auto_pause(true)).await;

    // `async move` copies the reference into each future, so the future does
    // not borrow from the closure (which Rust forbids for FnMut closures).
    let sandbox_ref = &sandbox;
    let state = poll_until("auto-pause at timeout", Duration::from_secs(120), || async move {
        let info = sandbox_ref.get_info().await.ok()?;
        (info.state == SandboxState::Paused).then_some(info.state)
    })
    .await;

    assert_eq!(state, SandboxState::Paused);
}

async fn clm_0012_internet_access_false_blocks_outbound() {
    // Arrange: a control sandbox proves the probe works when access is allowed.
    let open = LiveSandbox::new().await;
    let closed = LiveSandbox::create(&params().allow_internet_access(false)).await;
    let probe = "timeout 5 bash -c '</dev/tcp/1.1.1.1/443'";

    // Act
    let open_result = open.commands().run(probe, &CommandOptions::default()).await.expect("open probe");
    let closed_result =
        closed.commands().run(probe, &CommandOptions::default()).await.expect("closed probe");

    // Assert
    assert_eq!(open_result.exit_code, 0, "control sandbox could not reach 1.1.1.1:443");
    assert_ne!(closed_result.exit_code, 0, "sandbox without internet access reached 1.1.1.1:443");
}
```

- [ ] **Step 3: Connect, info, not-found (CLM-0013, 0015, 0016, 0017)**

```rust
async fn clm_0013_connect_running_sandbox_runs_commands() {
    let sandbox = LiveSandbox::new().await;

    let connected = sandbox
        .client()
        .connect_sandbox(sandbox.sandbox_id(), None)
        .await
        .expect("connect");

    let result = connected
        .commands()
        .run("echo connected", &CommandOptions::default())
        .await
        .expect("run");
    assert_eq!(result.stdout, "connected\n");
}

async fn clm_0015_connect_unknown_sandbox_is_not_found() {
    let error = client()
        .connect_sandbox(MISSING_ID, None)
        .await
        .expect_err("unknown sandbox must not connect");

    assert!(matches!(error, Error::SandboxNotFound(_)), "want SandboxNotFound, got {error:?}");
}

async fn clm_0016_get_info_returns_detail() {
    let sandbox = LiveSandbox::new().await;

    let info = sandbox.client().get_sandbox_info(sandbox.sandbox_id()).await.expect("info");

    assert_eq!(info.sandbox_id, sandbox.sandbox_id());
    assert_eq!(info.state, SandboxState::Running);
    assert!(!info.template_id.is_empty());
    assert!(info.end_at > info.started_at);
    assert_eq!(info.metadata.get(RUN_METADATA_KEY).map(String::as_str), Some(run_id()));
}

async fn clm_0017_get_info_unknown_sandbox_is_not_found() {
    let error = client().get_sandbox_info(MISSING_ID).await.expect_err("unknown sandbox");

    assert!(matches!(error, Error::SandboxNotFound(_)), "want SandboxNotFound, got {error:?}");
}
```

- [ ] **Step 4: List filters and paging (CLM-0018, 0019, 0020)**

Each list test tags its sandboxes with a key unique to the test, and polls, because list results may lag behind create.

```rust
/// The ids listed for `params`, following every page.
async fn list_all(params: &ListSandboxesParams) -> Vec<String> {
    let client = client();
    let mut ids = Vec::new();
    let mut page_params = params.clone();
    loop {
        let page = client.list_sandboxes(&page_params).await.expect("list");
        ids.extend(page.sandboxes.into_iter().map(|info| info.sandbox_id));
        match page.next_token {
            Some(token) => page_params = params.clone().next_token(token),
            None => return ids,
        }
    }
}

async fn clm_0018_list_filters_by_metadata() {
    let tag = format!("clm-0018-{}", run_id());
    let tagged = LiveSandbox::create(&params().metadata("list-test", &tag)).await;
    let _untagged = LiveSandbox::new().await;
    let filter = ListSandboxesParams::new().metadata("list-test", &tag);
    let filter = &filter;

    let ids = poll_until("tagged sandbox to be listed", Duration::from_secs(30), || async move {
        let ids = list_all(filter).await;
        (!ids.is_empty()).then_some(ids)
    })
    .await;

    assert_eq!(ids, vec![tagged.sandbox_id().to_owned()]);
}

async fn clm_0019_list_filters_by_state() {
    let tag = format!("clm-0019-{}", run_id());
    let running = LiveSandbox::create(&params().metadata("list-test", &tag)).await;
    let paused =
        LiveSandbox::create(&params().metadata("list-test", &tag).timeout(Duration::from_secs(120)))
            .await;
    assert!(paused.pause().await.expect("pause"));
    let base = ListSandboxesParams::new().metadata("list-test", &tag);
    let base = &base;

    let paused_ids = poll_until("paused sandbox to be listed", Duration::from_secs(30), || async move {
        let ids = list_all(&base.clone().state(SandboxState::Paused)).await;
        (!ids.is_empty()).then_some(ids)
    })
    .await;
    let running_ids = list_all(&base.clone().state(SandboxState::Running)).await;

    assert_eq!(paused_ids, vec![paused.sandbox_id().to_owned()]);
    assert_eq!(running_ids, vec![running.sandbox_id().to_owned()]);
}

async fn clm_0020_list_pages_without_duplicates_or_omissions() {
    let tag = format!("clm-0020-{}", run_id());
    let mut sandboxes = Vec::new();
    for _ in 0..3 {
        sandboxes.push(LiveSandbox::create(&params().metadata("list-test", &tag)).await);
    }
    let created: HashSet<String> =
        sandboxes.iter().map(|sandbox| sandbox.sandbox_id().to_owned()).collect();
    let filter = ListSandboxesParams::new().metadata("list-test", &tag).limit(1);
    let filter = &filter;

    let listed = poll_until("all three sandboxes to be listed", Duration::from_secs(30), || async move {
        let ids = list_all(filter).await;
        (ids.len() >= 3).then_some(ids)
    })
    .await;

    let unique: HashSet<String> = listed.iter().cloned().collect();
    assert_eq!(listed.len(), unique.len(), "duplicate ids across pages: {listed:?}");
    assert_eq!(unique, created);
}
```

- [ ] **Step 5: Kill, pause, timeout, refresh, is_running (CLM-0021..0027)**

```rust
async fn clm_0021_kill_running_sandbox() {
    let sandbox = LiveSandbox::new().await;

    let killed = sandbox.kill().await.expect("kill");

    assert!(killed);
    let error = sandbox.get_info().await.expect_err("killed sandbox must be gone");
    assert!(error.is_not_found(), "want not found, got {error:?}");
}

async fn clm_0022_kill_unknown_sandbox_is_false() {
    assert!(!client().kill_sandbox(MISSING_ID).await.expect("kill unknown"));
}

async fn clm_0023_pause_running_sandbox() {
    let sandbox = LiveSandbox::create(&params().timeout(Duration::from_secs(120))).await;

    let paused = sandbox.pause().await.expect("pause");

    assert!(paused);
    assert_eq!(sandbox.get_info().await.expect("info").state, SandboxState::Paused);
}

async fn clm_0024_pause_paused_sandbox_is_false() {
    let sandbox = LiveSandbox::create(&params().timeout(Duration::from_secs(120))).await;
    assert!(sandbox.pause().await.expect("first pause"));

    let second = sandbox.pause().await.expect("second pause");

    assert!(!second);
}

async fn clm_0025_set_timeout_moves_end_at() {
    let sandbox = LiveSandbox::new().await;

    sandbox.set_timeout(Duration::from_secs(240)).await.expect("set timeout");

    let info = sandbox.get_info().await.expect("info");
    let remaining = info.end_at.timestamp() - unix_now();
    assert!((220..=250).contains(&remaining), "end_at is {remaining}s away, want about 240");
}

async fn clm_0026_refresh_extends_end_at() {
    let sandbox = LiveSandbox::new().await;
    let before = sandbox.get_info().await.expect("info before").end_at;

    sandbox.refresh(Some(Duration::from_secs(300))).await.expect("refresh");

    let after = sandbox.get_info().await.expect("info after").end_at;
    assert!(after > before, "end_at did not move: before {before}, after {after}");
}

async fn clm_0027_is_running_tracks_kill() {
    let sandbox = LiveSandbox::new().await;
    assert!(sandbox.is_running().await.expect("is_running while alive"));

    assert!(sandbox.kill().await.expect("kill"));

    let sandbox_ref = &sandbox;
    let still_running = poll_until("envd to go away", Duration::from_secs(30), || async move {
        match sandbox_ref.is_running().await {
            Ok(false) => Some(false),
            _ => None,
        }
    })
    .await;
    assert!(!still_running);
}
```

- [ ] **Step 6: Gates, live run, changelog, commit**

Run: `just lint && just test && just doc`. Expected: pass.
Run: `just test-live hosted`. Record pass/fail per test in the PR body; failures are not fixed here (Task 6).
Write `changelog/<ts>-m1-task-02-live-lifecycle.md`.

```bash
git add tests/live_lifecycle.rs changelog/
git commit -m "test: live lifecycle tests for S1 v0.1 claims"
```

---

### Task 3: Live command tests (S2)

**Files:**
- Modify: `tests/live_commands.rs`

Extend the `use` lines to `use std::time::Duration;`, `use e2b_client::{CommandEvent, CommandOptions, ConnectCode, Error, Signal};` and `use live_support::LiveSandbox;`. Every test carries `#[tokio::test]` and `#[ignore = "live: run with `just test-live <target>`"]`.

- [ ] **Step 1: Streaming and large output (CLM-0034, 0035)**

```rust
async fn clm_0034_start_streams_chunks_then_exit() {
    let sandbox = LiveSandbox::new().await;
    let mut handle = sandbox
        .commands()
        .start("echo first; sleep 1; echo second >&2", &CommandOptions::default())
        .await
        .expect("start");

    // envd may split or merge output into chunks of any size, so join the
    // chunks per stream instead of asserting an exact event count.
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let mut exit = None;
    while let Some(event) = handle.next_event().await.expect("next_event") {
        assert!(exit.is_none(), "event after Exit: {event:?}");
        match event {
            CommandEvent::Stdout(chunk) => stdout.extend_from_slice(&chunk),
            CommandEvent::Stderr(chunk) => stderr.extend_from_slice(&chunk),
            CommandEvent::Exit(command_exit) => exit = Some(command_exit),
        }
    }

    assert_eq!(stdout, b"first\n");
    assert_eq!(stderr, b"second\n");
    assert_eq!(exit.expect("stream ended without Exit").exit_code, 0);
}

async fn clm_0035_run_delivers_large_output_in_order() {
    let sandbox = LiveSandbox::new().await;
    let expected: String = (1..=200_000).map(|n| format!("{n}\n")).collect();

    let result = sandbox
        .commands()
        .run("seq 1 200000", &CommandOptions::default())
        .await
        .expect("run");

    assert!(expected.len() > 1024 * 1024);
    assert_eq!(result.stdout.len(), expected.len());
    assert!(result.stdout == expected, "output differs from seq 1 200000");
}
```

- [ ] **Step 2: Options (CLM-0036, 0037, 0038, 0039)**

```rust
async fn clm_0036_options_env_is_set() {
    let sandbox = LiveSandbox::new().await;

    let result = sandbox
        .commands()
        .run("echo \"$COLOR\"", &CommandOptions::default().env("COLOR", "teal"))
        .await
        .expect("run");

    assert_eq!(result.stdout, "teal\n");
}

async fn clm_0037_options_cwd_is_working_directory() {
    let sandbox = LiveSandbox::new().await;

    let result = sandbox
        .commands()
        .run("pwd", &CommandOptions::default().cwd("/tmp"))
        .await
        .expect("run");

    assert_eq!(result.stdout, "/tmp\n");
}

async fn clm_0038_options_user_runs_as_user() {
    let sandbox = LiveSandbox::new().await;

    let root = sandbox
        .commands()
        .run("whoami", &CommandOptions::default().user("root"))
        .await
        .expect("run as root");
    let user = sandbox
        .commands()
        .run("whoami", &CommandOptions::default().user("user"))
        .await
        .expect("run as user");

    assert_eq!(root.stdout, "root\n");
    assert_eq!(user.stdout, "user\n");
}

async fn clm_0039_options_timeout_ends_long_command() {
    let sandbox = LiveSandbox::new().await;
    let started = tokio::time::Instant::now();

    let error = sandbox
        .commands()
        .run("sleep 30", &CommandOptions::default().timeout(Some(Duration::from_secs(3))))
        .await
        .expect_err("sleep 30 must not finish within 3 s");

    assert!(started.elapsed() < Duration::from_secs(15));
    assert!(
        matches!(
            error,
            Error::Timeout(_) | Error::Rpc { code: ConnectCode::DeadlineExceeded, .. }
        ),
        "want a timeout, got {error:?}"
    );
}
```

- [ ] **Step 3: Connect and list (CLM-0040, 0041)**

```rust
async fn clm_0040_connect_attaches_to_running_command() {
    let sandbox = LiveSandbox::new().await;
    let first = sandbox
        .commands()
        .start("sleep 3; echo late", &CommandOptions::default())
        .await
        .expect("start");
    let pid = first.pid();
    drop(first);

    let attached = sandbox.commands().connect(pid, None).await.expect("connect");
    let result = attached.wait().await.expect("wait");

    assert_eq!(result.stdout, "late\n");
    assert_eq!(result.exit_code, 0);
}

async fn clm_0041_list_includes_background_command() {
    let sandbox = LiveSandbox::new().await;
    let handle = sandbox
        .commands()
        .start("sleep 30", &CommandOptions::default())
        .await
        .expect("start");

    let processes = sandbox.commands().list().await.expect("list");

    let entry = processes
        .iter()
        .find(|process| process.pid == handle.pid())
        .unwrap_or_else(|| panic!("pid {} missing from {processes:?}", handle.pid()));
    assert_eq!(entry.cmd, "/bin/bash");
    assert!(entry.args.iter().any(|arg| arg.contains("sleep 30")), "args: {:?}", entry.args);
}
```

- [ ] **Step 4: Stdin, signal, kill (CLM-0042..0045)**

```rust
async fn clm_0042_send_stdin_reaches_command() {
    let sandbox = LiveSandbox::new().await;
    let handle = sandbox
        .commands()
        .start("head -n 1", &CommandOptions::default().stdin(true))
        .await
        .expect("start");

    sandbox.commands().send_stdin(handle.pid(), "typed line\n").await.expect("send_stdin");
    let result = handle.wait().await.expect("wait");

    assert_eq!(result.stdout, "typed line\n");
}

async fn clm_0043_close_stdin_sends_eof() {
    let sandbox = LiveSandbox::new().await;
    let handle = sandbox
        .commands()
        .start("wc -c", &CommandOptions::default().stdin(true))
        .await
        .expect("start");
    sandbox.commands().send_stdin(handle.pid(), "abc").await.expect("send_stdin");

    sandbox.commands().close_stdin(handle.pid()).await.expect("close_stdin");
    let result = handle.wait().await.expect("wait");

    assert_eq!(result.stdout.trim(), "3");
}

async fn clm_0044_sigterm_terminates_command() {
    let sandbox = LiveSandbox::new().await;
    let handle = sandbox
        .commands()
        .start("sleep 30", &CommandOptions::default())
        .await
        .expect("start");
    let started = tokio::time::Instant::now();

    sandbox.commands().send_signal(handle.pid(), Signal::Sigterm).await.expect("signal");
    let result = handle.wait().await.expect("wait");

    assert!(started.elapsed() < Duration::from_secs(10));
    assert!(!result.success(), "SIGTERM'd command reported success: {result:?}");
}

async fn clm_0045_kill_reports_whether_pid_existed() {
    let sandbox = LiveSandbox::new().await;
    let handle = sandbox
        .commands()
        .start("sleep 30", &CommandOptions::default())
        .await
        .expect("start");

    let killed = sandbox.commands().kill(handle.pid()).await.expect("kill running");
    let missing = sandbox.commands().kill(999_999).await.expect("kill unknown");

    assert!(killed);
    assert!(!missing);
}
```

- [ ] **Step 5: Gates, live run, changelog, commit**

Run: `just lint && just test && just doc`, then `just test-live hosted`, and record per-test results in the PR body.
Write `changelog/<ts>-m1-task-03-live-commands.md`.

```bash
git add tests/live_commands.rs changelog/
git commit -m "test: live command tests for S2 v0.1 claims"
```

---

### Task 4: Live filesystem tests (S3)

**Files:**
- Modify: `tests/live_files.rs`

Extend the `use` lines to `use std::collections::HashSet;`, `use e2b_client::FileType;` and `use live_support::LiveSandbox;`. Every test carries `#[tokio::test]` and `#[ignore = "live: run with `just test-live <target>`"]`.

- [ ] **Step 1: Read and write (CLM-0050, 0052, 0053)**

```rust
async fn clm_0050_write_read_round_trips_binary() {
    let sandbox = LiveSandbox::new().await;
    let data: Vec<u8> = (0..=255u8).cycle().take(64 * 1024).collect();

    sandbox.files().write("/tmp/binary.bin", data.clone(), None).await.expect("write");
    let read = sandbox.files().read("/tmp/binary.bin", None).await.expect("read");

    assert!(read.as_ref() == data.as_slice(), "binary round trip changed the bytes");
}

async fn clm_0052_write_returns_write_info() {
    let sandbox = LiveSandbox::new().await;

    let info = sandbox.files().write("/tmp/info.txt", "x", None).await.expect("write");

    assert_eq!(info.path, "/tmp/info.txt");
    assert_eq!(info.name, "info.txt");
    assert_eq!(info.file_type, FileType::File);
}

async fn clm_0053_read_missing_file_is_not_found() {
    let sandbox = LiveSandbox::new().await;

    let error = sandbox
        .files()
        .read("/tmp/does-not-exist", None)
        .await
        .expect_err("missing file");

    assert!(error.is_not_found(), "want not found, got {error:?}");
}
```

- [ ] **Step 2: List and stat (CLM-0054, 0055, 0056, 0057)**

```rust
/// Creates /tmp/tree with a.txt and sub/b.txt.
async fn make_tree(sandbox: &LiveSandbox) {
    let files = sandbox.files();
    files.write("/tmp/tree/a.txt", "a", None).await.expect("write a");
    files.write("/tmp/tree/sub/b.txt", "b", None).await.expect("write b");
}

async fn clm_0054_list_depth_one_lists_direct_entries() {
    let sandbox = LiveSandbox::new().await;
    make_tree(&sandbox).await;

    let entries = sandbox.files().list("/tmp/tree", 1, None).await.expect("list");

    let found: HashSet<(String, FileType)> =
        entries.into_iter().map(|entry| (entry.name, entry.file_type)).collect();
    let want: HashSet<(String, FileType)> = [
        ("a.txt".to_owned(), FileType::File),
        ("sub".to_owned(), FileType::Directory),
    ]
    .into_iter()
    .collect();
    assert_eq!(found, want);
}

async fn clm_0055_list_depth_two_recurses() {
    let sandbox = LiveSandbox::new().await;
    make_tree(&sandbox).await;

    let entries = sandbox.files().list("/tmp/tree", 2, None).await.expect("list");

    assert!(
        entries.iter().any(|entry| entry.path == "/tmp/tree/sub/b.txt"),
        "depth 2 missing sub/b.txt: {entries:?}"
    );
}

async fn clm_0056_get_info_reports_metadata() {
    let sandbox = LiveSandbox::new().await;
    sandbox.files().write("/tmp/sized.txt", "12345", None).await.expect("write");

    let info = sandbox.files().get_info("/tmp/sized.txt", None).await.expect("get_info");

    assert_eq!(info.size, 5);
    assert_eq!(info.file_type, FileType::File);
    assert!(!info.permissions.is_empty());
    assert!(!info.owner.is_empty());
}

async fn clm_0057_exists_true_and_false() {
    let sandbox = LiveSandbox::new().await;
    sandbox.files().write("/tmp/present.txt", "x", None).await.expect("write");

    let present = sandbox.files().exists("/tmp/present.txt", None).await.expect("exists");
    let absent = sandbox.files().exists("/tmp/absent.txt", None).await.expect("exists");

    assert!(present);
    assert!(!absent);
}
```

- [ ] **Step 3: Mutations and users (CLM-0058..0061)**

```rust
async fn clm_0058_make_dir_reports_creation() {
    let sandbox = LiveSandbox::new().await;

    let first = sandbox.files().make_dir("/tmp/newdir", None).await.expect("first make_dir");
    let second = sandbox.files().make_dir("/tmp/newdir", None).await.expect("second make_dir");

    assert!(first);
    assert!(!second);
}

async fn clm_0059_rename_moves_entry() {
    let sandbox = LiveSandbox::new().await;
    sandbox.files().write("/tmp/old.txt", "moving", None).await.expect("write");

    let entry = sandbox
        .files()
        .rename("/tmp/old.txt", "/tmp/new.txt", None)
        .await
        .expect("rename");

    assert_eq!(entry.path, "/tmp/new.txt");
    assert!(!sandbox.files().exists("/tmp/old.txt", None).await.expect("exists old"));
    let text = sandbox.files().read_to_string("/tmp/new.txt", None).await.expect("read new");
    assert_eq!(text, "moving");
}

async fn clm_0060_remove_deletes_file_and_directory() {
    let sandbox = LiveSandbox::new().await;
    sandbox.files().write("/tmp/gone/file.txt", "x", None).await.expect("write");

    sandbox.files().remove("/tmp/gone/file.txt", None).await.expect("remove file");
    let file_exists = sandbox.files().exists("/tmp/gone/file.txt", None).await.expect("exists");
    sandbox.files().remove("/tmp/gone", None).await.expect("remove dir");
    let dir_exists = sandbox.files().exists("/tmp/gone", None).await.expect("exists");

    assert!(!file_exists);
    assert!(!dir_exists);
}

async fn clm_0061_user_selects_file_owner() {
    let sandbox = LiveSandbox::new().await;
    let files = sandbox.files();

    files.write("/tmp/as-root.txt", "r", Some("root")).await.expect("write as root");
    files.write("/tmp/as-user.txt", "u", Some("user")).await.expect("write as user");

    let root_owner = files.get_info("/tmp/as-root.txt", None).await.expect("stat root").owner;
    let user_owner = files.get_info("/tmp/as-user.txt", None).await.expect("stat user").owner;
    assert_eq!(root_owner, "root");
    assert_eq!(user_owner, "user");
}
```

- [ ] **Step 4: Gates, live run, changelog, commit**

Run: `just lint && just test && just doc`, then `just test-live hosted`, and record per-test results in the PR body.
Write `changelog/<ts>-m1-task-04-live-files.md`.

```bash
git add tests/live_files.rs changelog/
git commit -m "test: live filesystem tests for S3 v0.1 claims"
```

---

### Task 5: S4 evidence map and missing wire tests

**Files:**
- Create: `docs/verification/s4-evidence-map.md`
- Modify (only where a claim has no precise test): `tests/rest_api.rs`, `tests/envd_api.rs`, `#[cfg(test)]` modules in `src/**`

The S4 claims are CLM-0067 to CLM-0116, plus CLM-0122 (Debug redaction). Read each with `mcp__claims__get_claim`, which is read-only, or `list_claims {tag: "s4"}`.

- [ ] **Step 1: Map each claim to the test that proves it**

For every claim, find the existing test whose assertions establish the claim's whole statement, not just part of it. Existing tests live in `tests/rest_api.rs`, `tests/envd_api.rs` and the `#[cfg(test)]` modules of `src/config.rs`, `src/error.rs`, `src/client.rs`, `src/filesystem.rs`, `src/models.rs`, `src/wire.rs`, `src/envd/{connect,mod,process_pb,filesystem_pb}.rs`. Write `docs/verification/s4-evidence-map.md`:

```markdown
# S4 evidence map

Which test proves each S4 wire claim (and CLM-0122). The coordinator records
each row as `test` evidence in the claims ledger.

| Claim | Test (path::name) | Covers whole claim? | Notes |
|---|---|---|---|
| CLM-0067 | src/config.rs::tests::defaults_without_env | yes | |
```

Expected gaps (confirm or refute each by reading the tests): CLM-0076 (API-returned domain wins), CLM-0078 (the API key is never sent to envd), CLM-0080 (`username` query on read and write), CLM-0092 (several states comma-joined), CLM-0109 (close_stdin refused below envd 0.5.2), CLM-0115 (502 from /health means not running), CLM-0122 (Sandbox and Client Debug output omit tokens).

- [ ] **Step 2: For each gap, write the failing test first**

Follow the file's existing wiremock patterns. For example, a CLM-0078 test in `tests/envd_api.rs` asserts that no request reaching the envd mock carries `x-api-key`:

```rust
#[tokio::test]
async fn envd_requests_never_carry_the_api_key() {
    // Arrange: `sandbox` (tests/common/mod.rs) builds a Sandbox whose REST API
    // and envd are both this mock server; `start_frames` is this file's helper.
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    Mock::given(method("POST"))
        .and(path("/process.Process/Start"))
        .respond_with(stream_response(&start_frames(0)))
        .mount(&server)
        .await;

    // Act
    sandbox.commands().run("echo hi", &CommandOptions::default()).await.expect("run");

    // Assert
    let envd_requests: Vec<_> = server
        .received_requests()
        .await
        .expect("request recording is on by default")
        .into_iter()
        .filter(|request| request.url.path().starts_with("/process."))
        .collect();
    assert!(!envd_requests.is_empty());
    assert!(envd_requests.iter().all(|request| !request.headers.contains_key("x-api-key")));
}
```

Run each new test. A test that fails on current code means the claim is refuted: keep the test, mark it `#[ignore = "refutes CLM-NNNN, see #<issue>"]`, note it in the map, and report it. Don't fix the crate in this task.

- [ ] **Step 3: Gates, changelog, commit**

Run: `just lint && just test && just doc`. Expected: pass, with refuting tests ignored as described.
Write `changelog/<ts>-m1-task-05-s4-evidence-map.md`.

```bash
git add docs/verification/s4-evidence-map.md tests/ src/ changelog/
git commit -m "test: S4 wire-claim evidence map and missing wire tests"
```

---

### Task 6: Live run on hosted E2B

**Files:**
- Create: `docs/verification/runs/<run id>.log` (copied from `target/live-logs/`)
- Create or modify: `docs/verification/m1-live-results.md` (section "Hosted E2B")

- [ ] **Step 1: Run the suite on the merged main**

Run: `just test-live hosted`
Expected: one result line per `clm_*` test and a sweep line.

- [ ] **Step 2: Check for leaks**

The sweep must report 0 leftovers on a clean run. If it reports more, a drop guard failed: say so in the results.

- [ ] **Step 3: Check the log in and summarize**

Copy the log to `docs/verification/runs/`. First make sure it contains none of the configured keys. Check each actual value, not a key pattern. Nothing is printed except the counts, and every count must be `0`:

```bash
env_file="$(git rev-parse --path-format=absolute --git-common-dir)/../.env"
for name in E2B_API_KEY_HOSTED E2B_API_KEY_SANDBAWKS; do
    value="$(grep "^${name}=" "$env_file" | cut -d= -f2-)"
    [ -n "$value" ] && echo "$name: $(grep -cF -- "$value" <log>)"
done
``` Add to `docs/verification/m1-live-results.md`:

```markdown
## Hosted E2B (run <run id>, <UTC time>)

| Test | Claim | Result | Failure (first line) |
|---|---|---|---|
| clm_0007_create_returns_running_sandbox | CLM-0007 | pass | |
```

Every `clm_*` test gets a row. For each failure, add a short analysis below the table: is the crate wrong, is the test wrong, or is it the environment (for example, quota)? Test bugs are fixed in this PR, with a plan correction note. Crate bugs are only reported here. The coordinator files them as M1 bug tasks (see Task 8).

- [ ] **Step 4: Changelog and commit**

Write `changelog/<ts>-m1-task-06-hosted-run.md`.

```bash
git add docs/verification/ changelog/
git commit -m "docs: M1 live results on hosted E2B"
```

---

### Task 7: Live run on sandbawks

**Files:**
- Create: `docs/verification/runs/<run id>.log`
- Create or modify: `docs/verification/m1-live-results.md` (section "sandbawks")

Same steps as Task 6, with `just test-live sandbawks` and the section heading `## sandbawks (run <run id>, <UTC time>)`. For each failure, also state whether the same test passed on hosted E2B. A test that passes on hosted but fails on sandbawks is a sandbawks defect: describe the observed request and response so the coordinator can file it on gfd/sandbawks. Write `changelog/<ts>-m1-task-07-sandbawks-run.md` and commit `docs: M1 live results on sandbawks`.

---

### Task 8: Verdicts, bug tasks and milestone close (coordinator)

**Executor:** the coordinator session, because it owns `claims.json`. This isn't a subagent task.

**Files:**
- Modify: `claims.json`
- Modify: `README.md` (add a "Verification status" section)
- Modify: `docs/protocol-notes.md` (rewrite "Verification against a live server")
- Create: `changelog/<ts>-m1-milestone.md`

- [ ] **Step 1: Record S4 evidence.** For each row of `s4-evidence-map.md`, call `add_evidence` with kind `test`, locator `<path>::<name> @ <main commit>` and stance `supports`, or `refutes` for an ignored refuting test. Then `record_verdict`.
- [ ] **Step 2: Record live evidence.** For each row of both results tables, call `add_evidence` with kind `demonstration` and locator `<test> @ <target>, docs/verification/runs/<run id>.log`. Hosted rows decide the verdict. A sandbawks failure on a claim that passed on hosted is `neutral` evidence that names the gfd/sandbawks issue.
- [ ] **Step 3: Handle refuted claims.**
  1. File one sub-issue per crate bug, `M1.9+: fix <claim>`, under the M1 epic, blocked by nothing. Add a matching task to this plan with a dated `> **Corrected**` note.
  2. Run each one through the normal per-issue PR flow.
  3. Re-run the affected live test on hosted and record new evidence and a new verdict.
  4. File sandbawks defects on Forgejo `gfd/sandbawks` and link them from the evidence.
- [ ] **Step 4: Record the S5 and gap verdicts.**
  - CLM-0117 and CLM-0118 are judged from the results above.
  - CLM-0122 comes from its S4 test.
  - CLM-0125 and CLM-0126 are judged by inspection (`grep -ri sandbawks src/` and the license files).
  - Gap claims get `not_enough_info` with the rationale "not implemented; scheduled for M<k>".
- [ ] **Step 5: Docs.**
  - **README "Verification status":** live-verified on hosted E2B on <date>; the claim counts by verdict from `audit_report`; the pointer "see `claims.json` and `docs/verification/`".
  - **protocol-notes "Verification against a live server":** replace the unauthenticated-probe text with a summary of the authenticated runs.
  - **Milestone changelog.**
- [ ] **Step 6: Commit `claims.json` and the docs on branch `m1-verdicts` and open the PR.** The flow is the same as for every other PR: review, answer, merge. Then run `github-epic-close-out`.
