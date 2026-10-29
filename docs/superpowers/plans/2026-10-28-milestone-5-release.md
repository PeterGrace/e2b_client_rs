# Milestone 5 (Release) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Release `e2b-client` 0.2.0 on crates.io: finish the per-call request timeouts M4 deferred (#120), make the live suite able to pass on a team without volumes, fix the paced recipe's readiness marker (#122), add the live CI job (CLM-0123), make the package publishable and its contents deliberate (CLM-0124), write `CHANGELOG.md`, put a coverage table generated from the ledger in the README, run the live suite through the new CI job, and publish.

**Architecture:** One new public type, `RequestOptions` (in `src/config.rs`, next to `RequestTimeout`), for the calls that have no other options; every existing REST options or params struct gains a `request_timeout` setter. `Client::request` and `Client::send_json` take a `RequestTimeout`, as `EnvdClient::unary` already does. The live harness gains a skip marker for environment limits, the `justfile` gains a testable result classifier (`_classify`) and a recipe that tests it against fixtures (`test-recipes`). A second workflow, `.github/workflows/live.yml`, runs `just test-live hosted` on demand. `Cargo.toml` loses `publish = false` and gains an `include` list; CI checks the package list. A new test, `tests/readme_coverage.rs`, recomputes the README's coverage table from `claims.json`.

**Tech Stack:** Rust 2024 (MSRV 1.89), reqwest 0.13 (`RequestBuilder::timeout`), serde_json (dev, for the ledger test), wiremock, `just`, GitHub Actions (`workflow_dispatch`, `actions/upload-artifact@v4`), cargo `package`/`publish`, claims MCP.

**Design:** `docs/superpowers/specs/2026-10-26-claims-audit-and-publishability-design.md`, sections "Claim hierarchy" (S5) and "Milestones" (the M5 row: live CI job, hosted only, behind secrets, manual or nightly trigger; README coverage table from the ledger audit; CHANGELOG; `publish = true`; 0.2.0 on crates.io). As for M2 to M4, the spec's "short design" for this milestone is this plan's "Decisions", "API after M5" and "Verified facts" sections.

---

## Scope

In:

- **Per-call request timeouts on REST calls (#120, CLM-0182):** every `Client` and `Sandbox` call that reaches the REST API, and `Sandbox::is_running` (envd health), takes a per-call request timeout.
- **Per-call request timeouts on pid-only process and PTY calls (#120, CLM-0183):** `Commands::{list, send_stdin, close_stdin, send_signal, kill}`, `Pty::{send_stdin, resize, kill}`, and the same calls on `CommandHandle` and `PtyHandle`.
- **The readiness marker (#122):** `just test-live-paced` no longer counts a sandbox that failed outright (`action_failure`) as capacity; the classifier is tested.
- **Environment skips:** a volume test on a team without volumes prints a skip line and passes, so a hosted run can be green; the paced recipe reports it as `SKIP`.
- **The live CI job (CLM-0123):** `.github/workflows/live.yml`, manual trigger only, hosted only, behind the `E2B_API_KEY_HOSTED` repository secret.
- **Packaging (CLM-0124):** `publish = false` goes, an `include` list keeps the ledger, logs, plans and tests out of the package, CI checks the list and that the package builds.
- **`CHANGELOG.md` (CLM-0185):** the 0.2.0 entry, with every breaking change since 0.1.0.
- **README coverage table (CLM-0184):** per area, from `claims.json`, checked by a test.
- **The release (CLM-0186):** version 0.2.0, tag `v0.2.0`, GitHub release, `cargo publish` after pgrace's go-ahead, docs.rs build.
- **The five claims CLM-0182 to CLM-0186**, registered while planning and committed with this plan; CLM-0123 and CLM-0124 retagged `m5`.
- One hosted live run through the CI job, the verdicts and the milestone close.

Deferred:

| Item | Claims / issue | Goes to |
|---|---|---|
| The sweeps' 404 on sandbawks (the live recipes can't exit 0 there) | #121 | unscheduled (pgrace, 2026-10-28: not in M5) |
| Volume tests on a team with volumes enabled | #119, CLM-0148, CLM-0149, CLM-0153 | unscheduled; needs a team where `POST /volumes` is not 403 |
| A nightly or weekly schedule for the live CI job | CLM-0123 | not planned (pgrace, 2026-10-28: manual only, for cost) |
| A sandbawks run of record for M5 | none | not planned: hosted is canonical (spec decision 3), M5 changes no wire behaviour except the timeouts, and M4's paced run settled CLM-0118 |
| The SDK's control-plane `retries` (`ApiParams.retries`, `e2b/connection_config.py:80-91`) | none | unscheduled; recorded in `docs/api-review.md` "Known limits" |
| The SOCKS5 egress proxy and IAM tokens (CLM-0031, CLM-0032, and with them CLM-0002 and the root CLM-0001) | none | unscheduled (M3's decision) |

## Decisions taken while planning (2026-10-28)

1. **Live CI trigger: manual only** (`workflow_dispatch`), hosted only, behind the repository secret `E2B_API_KEY_HOSTED`. No schedule: a hosted run is about 80 billed sandboxes (pgrace).
2. **Volume tests skip on "not enabled".** A volume test whose `create_volume` answers 403 with a message containing `not enabled` prints `SKIPPED (environment): ...` and returns, so it passes. The run log shows the skip; the claims stay `not_enough_info` (pgrace).
3. **Publishing: the coordinator runs `cargo publish`, after pgrace's go-ahead.** The release PR merges and `v0.2.0` is tagged first; then the coordinator asks once, and publishes only on a yes (pgrace).
4. **Follow-ups carried in:** #120 (per-call REST and pid-only timeouts, before 0.2.0 so it isn't a 0.3.0 break) and #122 (the readiness marker). #121 and #119 stay out (pgrace).
5. **#120's API shape.** Calls that already take an options or params struct get a `request_timeout` setter on it: `CreateSandboxParams`, `ConnectOptions`, `ListSandboxesParams`, `MetricsParams`, `PauseOptions`, `SnapshotOptions`, `ListSnapshotsParams`, `ForkOptions`. Every other call gains a trailing `&RequestOptions`, a new type with only a request timeout. `NetworkUpdate` is a serialized request body, so `update_network` takes `&RequestOptions` as well rather than a skipped field on the body. This follows M4's decision 3 (one method per call, options by reference, no `_with` twins), and the handle methods (`CommandHandle::kill`, `PtyHandle::resize`, ...) follow it too, for one rule everywhere. The semantics are M4's: unset uses the configured timeout, `None` or zero means no limit. (Coordinator's decision, from M4's decision 3.)
6. **`create_sandbox`'s request timeout bounds the `POST` only.** Starting the MCP gateway is a command (`Commands::run`) with its own timeouts, as in the SDK (`e2b/sandbox_async/main.py:245-268`). `fork_sandbox`'s bounds each fork request.
7. **`Sandbox::mcp_token` keeps no options.** It reads the token file once and caches it; the read uses the configured timeout.
8. **#122: a sandbox that failed outright is its own class.** `will not become ready` leaves the readiness markers. Output with `the sandbox failed (action_failure)` (and none of the readiness markers) is `FAIL-SANDBOX-FAILED`: rerun once, like a readiness failure, and if the rerun fails the same way it ends `FAIL-SANDBOX-FAILED (rerun)`, counted apart in the summary. A person classifies it: M2 saw it on capacity, M1, M3 and M4 on an unknown template (gfd/sandbawks#680).
9. **Package contents.** `include = ["/src", "/README.md", "/CHANGELOG.md", "/LICENSE-APACHE", "/LICENSE-MIT"]`: the ledger, the run logs, the plans, the changelog directory, the tests and `.claude/` are not shipped. The README stays (its examples are doctests through `src/lib.rs`).
10. **The coverage table is generated, not hand-kept.** `tests/readme_coverage.rs` recomputes the per-area table from `claims.json` and fails, printing the expected table, if the README's differs. Every PR that changes the ledger (only the coordinator's) updates it.
11. **Version 0.2.0.** 0.1.0 was never published; the spec names 0.2.0. `CHANGELOG.md` follows Keep a Changelog, with one 0.2.0 entry and a short 0.1.0 entry (git only).
12. **Ledger.** As before, only the coordinator session writes `claims.json`. CLM-0182 to CLM-0186 were added while planning and are committed with this plan; CLM-0123 and CLM-0124 were retagged from `gap` to `m5`.
13. **Merging.** The standing authorization holds: the coordinator merges the plan PR and each sub-issue PR once CI is green and brooke-reviews' review has been answered. Publishing is decision 3.
14. **Execution order.** One issue at a time, oldest first (pgrace's standing rule). The graph below shows what could run in parallel.

## API after M5

Only #120 changes the API. Old call to new call:

| Old | New |
|---|---|
| `client.get_sandbox_info(id)` | `client.get_sandbox_info(id, &RequestOptions::new())` |
| `client.kill_sandbox(id)` | `client.kill_sandbox(id, &RequestOptions::new())` |
| `client.set_sandbox_timeout(id, d)` | `client.set_sandbox_timeout(id, d, &RequestOptions::new())` |
| `client.update_sandbox_network(id, &update)` | `client.update_sandbox_network(id, &update, &RequestOptions::new())` |
| `client.refresh_sandbox(id, d)` | `client.refresh_sandbox(id, d, &RequestOptions::new())` |
| `client.delete_snapshot(id)` | `client.delete_snapshot(id, &RequestOptions::new())` |
| `client.create_volume(name)` | `client.create_volume(name, &RequestOptions::new())` |
| `client.get_volume(id)` | `client.get_volume(id, &RequestOptions::new())` |
| `client.list_volumes()` | `client.list_volumes(&RequestOptions::new())` |
| `client.destroy_volume(id)` | `client.destroy_volume(id, &RequestOptions::new())` |
| `sandbox.is_running()` | `sandbox.is_running(&RequestOptions::new())` |
| `sandbox.kill()` | `sandbox.kill(&RequestOptions::new())` |
| `sandbox.set_timeout(d)` | `sandbox.set_timeout(d, &RequestOptions::new())` |
| `sandbox.update_network(&update)` | `sandbox.update_network(&update, &RequestOptions::new())` |
| `sandbox.refresh(d)` | `sandbox.refresh(d, &RequestOptions::new())` |
| `sandbox.get_info()` | `sandbox.get_info(&RequestOptions::new())` |
| `commands.list()` | `commands.list(&RequestOptions::new())` |
| `commands.send_stdin(pid, data)` | `commands.send_stdin(pid, data, &RequestOptions::new())` |
| `commands.close_stdin(pid)` | `commands.close_stdin(pid, &RequestOptions::new())` |
| `commands.send_signal(pid, signal)` | `commands.send_signal(pid, signal, &RequestOptions::new())` |
| `commands.kill(pid)` | `commands.kill(pid, &RequestOptions::new())` |
| `handle.send_stdin(data)` (`CommandHandle`) | `handle.send_stdin(data, &RequestOptions::new())` |
| `handle.kill()` (`CommandHandle`) | `handle.kill(&RequestOptions::new())` |
| `pty.send_stdin(pid, data)` | `pty.send_stdin(pid, data, &RequestOptions::new())` |
| `pty.resize(pid, size)` | `pty.resize(pid, size, &RequestOptions::new())` |
| `pty.kill(pid)` | `pty.kill(pid, &RequestOptions::new())` |
| `handle.send_stdin(data)` / `resize(size)` / `kill()` (`PtyHandle`) | the same with a trailing `&RequestOptions::new()` |

Unchanged signatures that gain a `request_timeout(Option<Duration>)` setter on their struct: `create_sandbox(&CreateSandboxParams)`, `connect_sandbox(id, &ConnectOptions)`, `list_sandboxes(&ListSandboxesParams)`, `get_sandbox_metrics(id, &MetricsParams)` and `Sandbox::get_metrics`, `pause_sandbox(id, &PauseOptions)` and `Sandbox::pause`, `create_snapshot(id, &SnapshotOptions)` and `Sandbox::create_snapshot`, `list_snapshots(&ListSnapshotsParams)` and `Sandbox::list_snapshots`, `fork_sandbox(id, &ForkOptions)` and `Sandbox::fork`.

## Verified facts the code relies on

SDK sources are e2b 2.61.0 (`py:` paths relative to `e2b/`). Crate sources are under `~/.cargo/registry/src/index.crates.io-*/`.

| Fact | Source |
|---|---|
| Every REST call on the SDK's `AsyncSandbox` and `SandboxApi` takes `**opts: Unpack[ApiParams]`, whose `request_timeout: Optional[float]` is "Timeout for the request in seconds" (`kill`, `set_timeout`, `get_info`, `get_metrics`, `connect`, ...) | `py: connection_config.py:70-78`; `py: sandbox_async/main.py:271-276, 523-552, 569-606, 677-705, 719-759` |
| Per-call params win over bound params, which win over the environment; a per-call `None` falls back | `py: connection_config.py:146-160` (`merge_api_params`) |
| `commands.list`, `kill`, `send_stdin`, `close_stdin` take `request_timeout: Optional[float] = None` | `py: sandbox_async/commands/command.py:64-71, 99-102, 120-124, 176-179` |
| `pty.kill`, `send_stdin`, `resize` take `request_timeout` | `py: sandbox_async/commands/pty.py:65-68, 85-89, 256-260` |
| `CommandHandle.send_stdin(data, request_timeout=None)`; `CommandHandle.kill()` takes none | `py: sandbox_async/commands/command_handle.py:265-280` |
| `ApiParams.retries` (control-plane retries, default 3) exists; the crate has no retries | `py: connection_config.py:80-91` |
| The crate: `Client::request(method, segments)` applies `config.request_timeout()` with `RequestBuilder::timeout`; the API `reqwest::Client` has no client-wide timeout; `send_json` goes through `request` | `src/client.rs:98-106, 1066-1089` |
| The crate: `EnvdClient::unary(service, method, request, headers, RequestTimeout)` and `http_request(method, path, RequestTimeout)` already resolve a per-call timeout (`timeout_for`); the pid-only calls pass `RequestTimeout::default()` | `src/envd/mod.rs:290-292, 346-354, 524-531`; `src/commands.rs:640-650, 693-705, 790-850`; `src/pty.rs:526-610`; `src/sandbox.rs:600-603` |
| `RequestTimeout(Option<Option<Duration>>)`, `RequestTimeout::set` (zero means no limit) and `resolve(configured)` | `src/config.rs:49-84` |
| `PauseOptions`, `ConnectOptions`, `MetricsParams`, `ForkOptions` derive `Copy`; `RequestTimeout` is `Copy`, so adding the field keeps them `Copy` | `src/lifecycle.rs:355, 447`; `src/metrics.rs:64`; `src/models.rs:496`; `src/config.rs:56` |
| `NetworkUpdate` derives `Serialize` and is the request body | `src/network.rs:277-278` |
| Test helpers: `common::client_with_timeout(&server, Option<Duration>)`, `common::sandbox(&server)`; `Error::Transport(t)` with `t.is_timeout()` | `tests/common/mod.rs:21-36, 94`; `tests/envd_file_options.rs:26-28` |
| Hosted E2B answers `POST /volumes` for this team with 403 `use of volumes is not enabled` (`Error::Api { status: 403, message: "use of volumes is not enabled" }`); the volume sweep's list call succeeds | `docs/verification/runs/20261028T153214Z-hosted-761165.log:193-204` and its sweep lines |
| `clm_0148_missing_volume_is_not_found_and_not_destroyed` passes on hosted without volumes; the other three volume tests create one | same log; `tests/live_lifecycle.rs:1161-1290` |
| sandbawks's message for a sandbox that failed: `sandbox will not become ready without intervention: the sandbox failed (action_failure)` (as `McpGateway("ERROR: ...")` or a 412 `sandbox_failed`); seen on capacity in M2 and on an unknown template in M1, M3, M4 | `docs/verification/runs/20261028T154205Z-sandbawks-paced-913873.log:952`; `docs/verification/m1-live-results.md:324`; `m2-live-results.md:133-136` |
| The paced recipe's readiness pattern and labels | `justfile:59-87` |
| `cargo publish --dry-run` with `publish = false` fails: "`package.publish` must be set to `true` or a non-empty list" | plan session, cargo 1.99.1, 2026-10-28 |
| With the `include` list of decision 9, `cargo package --list` gives `.cargo_vcs_info.json`, `Cargo.lock`, `Cargo.toml`, `Cargo.toml.orig`, the two licences, `README.md` and the 21 files under `src/`; a missing `CHANGELOG.md` is skipped silently; `cargo package --locked` builds the packaged crate (33 s) | plan session, 2026-10-28 |
| Today's package (no `include`) ships `claims.json` (464 KB), `docs/` (1.5 MB, with the run logs), `changelog/`, `tests/`, `.claude/`, `.github/`, `justfile` | plan session, `cargo package --list`, 2026-10-28 |
| crates.io has no crate `e2b-client` or `e2b_client` (the name is free) | `https://crates.io/api/v1/crates/e2b-client` → "crate `e2b-client` does not exist", 2026-10-28 |
| `~/.cargo/credentials.toml` exists on pgrace's machine (a crates.io token) | plan session, 2026-10-28 |
| The repository has no Actions secrets yet (`gh secret list` is empty) | plan session, 2026-10-28 |
| `claims.json`: `claims[]` with `id`, `tags` (one of `s1`..`s5`, or `root`), `status.state` (`active`/`retired`), `verdicts[]` (the last is current; none means unverified). Active today: s1 33 supported + 6 nei, s2 21, s3 23, s4 80, s5 12 + 3 nei, root 1 nei | plan session, 2026-10-28 |

## Conventions for every task

- **Gates:** `just lint` (fmt check plus `clippy --all-targets -D warnings`), `just test` and `just doc` must pass before pushing. Tasks 3 and 4 also run `just test-recipes`. A task that changes a live test's calls or adds one runs those tests against hosted E2B, one at a time:
  `E2B_TEST_TARGET=hosted E2B_TEST_RUN_ID=dev-$(date -u +%Y%m%dT%H%M%SZ)-$$ cargo test --test <live file> -- --ignored --exact <test name>`.
  Report the result in the PR. The recorded run is Task 9.
- **Breaking changes:** Tasks 1 and 2 change signatures. A signature change updates every caller in `src/`, `tests/`, `README.md` and the rustdoc examples in the same PR; `cargo test` compiles the README and doctests, so the gates catch a missed one. Each such task's changelog entry has a "Breaking changes" list (old call, new call), which Task 7 copies into `CHANGELOG.md`.
- **Tests:** mock tests use `wiremock` through `tests/common`. Live tests are `#[tokio::test]` plus `#[ignore = "live: run with `just test-live <target>`"]`, named `clm_NNNN_<slug>`, one claim each, Arrange-Act-Assert. Use `.expect("<what>")` in tests, never `.unwrap()`. No `println!`/`dbg!`; a live test's skip line is `eprintln!` (as the sweeps do).
- **Docs per task:** every new or changed public item keeps a rustdoc comment with Arguments/Returns/Errors and an `# Examples` section (`tests/doc_examples.rs` enforces it). Every wire fact a task changes is corrected in `docs/protocol-notes.md`. `docs/api-review.md` "Known limits" drops the request-timeout bullet when Task 2 lands (Task 1 narrows it to the pid-only calls).
- **Ledger:** subagents never call `mcp__claims__*` write tools. Read tools are fine. A PR that changes `claims.json` updates the README coverage table (Task 8's test fails otherwise); only the coordinator's do.
- **Changelog:** one entry per task, `changelog/<UTC ts>-m5-task-<NN>-<slug>.md`, timestamp from `date -u +%Y-%m-%dT%H-%M-%SZ`.
- **Logs:** live run logs go to `target/live-logs/` inside your own worktree. Never write them to a shared scratchpad.
- **Processes:** never kill processes by pattern; kill only your own PIDs.
- **`Cargo.lock`:** diff it before pushing; no task adds a dependency. Only Task 10's version bump changes it (the `e2b-client` version line).
- **Secrets:** never print, log or commit an API key. The CI job reads it from a secret; the release uses pgrace's existing `~/.cargo/credentials.toml` and never reads or prints it.
- The code in this plan has not been compiled. If an implementation has to differ, fix this plan in the same PR (inline, or with a dated `> **Corrected (issue #N).**` note), then grep later tasks for the changed symbol.

## File map

| File | Task |
|---|---|
| `claims.json` | plan PR; 10; 11 |
| `Cargo.toml` | 6 (`publish`, `include`), 10 (version) |
| `Cargo.lock` | 10 (the crate's own version line) |
| `.github/workflows/ci.yml` | 3 (`just test-recipes`), 6 (package check) |
| `.github/workflows/live.yml` (new) | 5 |
| `justfile` | 3 (`_classify`, `test-recipes`, the paced loop and summary), 4 (`SKIP`), 6 (`package-check`) |
| `tests/fixtures/live-pacing/*.txt` (new) | 3, 4 |
| `src/config.rs` | 1 (`RequestOptions`) |
| `src/lib.rs` | 1 (re-export, crate docs' options paragraph), 2 (crate docs) |
| `src/client.rs` | 1 |
| `src/sandbox.rs` | 1 |
| `src/models.rs` (`CreateSandboxParams`, `ListSandboxesParams`, `ForkOptions`) | 1 |
| `src/lifecycle.rs` (`ConnectOptions`, `PauseOptions`) | 1 |
| `src/metrics.rs` (`MetricsParams`) | 1 |
| `src/snapshots.rs` (`SnapshotOptions`, `ListSnapshotsParams`) | 1 |
| `src/volumes.rs` (rustdoc examples only) | 1 |
| `src/commands.rs` | 2 |
| `src/pty.rs` | 2 |
| `tests/rest_request_options.rs` (new) | 1 |
| `tests/envd_request_options.rs` (new) | 2 |
| `tests/readme_coverage.rs` (new) | 8 |
| `tests/rest_*.rs`, `tests/common/mod.rs` | 1 (callers) |
| `tests/envd_*.rs`, `tests/zero_timeouts.rs`, `tests/api_traits.rs` | 1, 2 (callers; `api_traits.rs` adds `RequestOptions`) |
| `tests/live_support/mod.rs` | 1 (callers), 4 (`SKIPPED`, `create_volume_or_skip`) |
| `tests/live_lifecycle.rs` | 1 (callers, `clm_0182`), 4 (the three volume tests) |
| `tests/live_commands.rs`, `tests/live_files.rs` | 1, 2 (callers); 2 (`clm_0183` in `live_commands.rs`) |
| `tests/live_sweep.rs` | 1 (callers) |
| `README.md` | 1, 2 (examples, coverage rows), 6 (nothing; checked), 8 (coverage table), 10 (install, verification status), 11 |
| `CHANGELOG.md` (new) | 7; 10 (release date) |
| `CONTRIBUTING.md` | 3 (`just test-recipes`), 5 (the live CI job), 6 (`just package-check`) |
| `docs/api-review.md` | 1, 2 ("Known limits") |
| `docs/protocol-notes.md` | 1, 2 (request timeouts), 11 ("Not implemented yet") |
| `docs/verification/live-pacing.md` | 3, 4 |
| `docs/verification/runs/<run id>.log`, `docs/verification/m5-live-results.md` (new) | 9 |
| the spec | 11 (the M5 row's outcome) |
| milestone changelog | 11 |

## Task dependencies

```
1 -> 2
1 -> 4
3 -> 4
6 -> 7
2 -> 7
1, 2, 3, 4, 5, 6, 7, 8 -> 9
9 -> 10
10 -> 11
```

Tasks 1, 3, 5, 6 and 8 can start straight away. Task 2 reuses Task 1's `RequestOptions`. Task 4 edits the volume tests whose `create_volume` calls Task 1 changes, and the paced loop Task 3 rewrites. Task 7 lists Tasks 1 and 2's breaking changes and puts `CHANGELOG.md` in Task 6's `include` list. Task 9 runs the CI job on the merged `main` and needs pgrace to have set the `E2B_API_KEY_HOSTED` secret (the coordinator asks before dispatching). Tasks 10 and 11 are the coordinator's.

Tasks 1, 2 and 8 all edit `README.md` (Tasks 1 and 2 the examples and the "What it covers" rows, Task 8 "Verification status"): keep both sides. Tasks 3 and 6 both edit `justfile` and `ci.yml` (different recipes and steps): keep both sides.

---

### Task 1: Per-call request timeouts on REST calls (#120, CLM-0182)

**Files:**
- Modify: `src/config.rs` (new `RequestOptions`), `src/lib.rs`, `src/client.rs`, `src/sandbox.rs`, `src/models.rs`, `src/lifecycle.rs`, `src/metrics.rs`, `src/snapshots.rs`, `src/volumes.rs` (examples)
- Create: `tests/rest_request_options.rs`
- Modify (callers): `tests/rest_*.rs`, `tests/common/mod.rs`, `tests/envd_*.rs` where they call a changed method, `tests/api_traits.rs`, `tests/live_*.rs`, `tests/live_support/mod.rs`, `README.md`
- Modify: `docs/api-review.md`, `docs/protocol-notes.md`
- Create: `changelog/<ts>-m5-task-01-rest-request-timeouts.md`

This PR references #120 but does not close it (Task 2 does): write "Part of #120" in the PR body, not "Closes".

- [ ] **Step 1: Write the failing tests**

Create `tests/rest_request_options.rs`:

```rust
//! Mock tests for the per-call request timeout on REST calls (CLM-0182):
//! `RequestOptions` on calls with no other options, and the
//! `request_timeout` setter on the REST options and params structs.

mod common;

use std::time::{Duration, Instant};

use common::{client_with_timeout, detail_json, sandbox};
use e2b_client::{
    ConnectOptions, CreateSandboxParams, Error, ListSandboxesParams, PauseOptions, RequestOptions,
};
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const SHORT: Duration = Duration::from_millis(200);

/// Whether `error` is a transport-level timeout (reqwest's own deadline).
fn is_transport_timeout(error: &Error) -> bool {
    matches!(error, Error::Transport(transport) if transport.is_timeout())
}

/// Mounts `GET /sandboxes/sbx-1` answering after `delay`.
async fn slow_detail(server: &MockServer, delay: Duration) {
    Mock::given(method("GET"))
        .and(path("/sandboxes/sbx-1"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(detail_json(json!({})))
                .set_delay(delay),
        )
        .mount(server)
        .await;
}

#[tokio::test]
async fn a_per_call_request_timeout_overrides_the_configured_one() {
    // Arrange: configured 5 s; the API answers after 2 s.
    let server = MockServer::start().await;
    slow_detail(&server, Duration::from_secs(2)).await;
    let client = client_with_timeout(&server, Some(Duration::from_secs(5)));
    let options = RequestOptions::new().request_timeout(Some(SHORT));

    // Act
    let started = Instant::now();
    let error = client
        .get_sandbox_info("sbx-1", &options)
        .await
        .expect_err("the 200 ms per-call timeout passes first");

    // Assert
    assert!(is_transport_timeout(&error), "{error:?}");
    assert!(started.elapsed() < Duration::from_secs(1), "{:?}", started.elapsed());
}

#[tokio::test]
async fn a_per_call_none_lifts_a_short_configured_timeout() {
    // Arrange: configured 200 ms; the API answers after 500 ms.
    let server = MockServer::start().await;
    slow_detail(&server, Duration::from_millis(500)).await;
    let client = client_with_timeout(&server, Some(SHORT));

    // Act
    let info = client
        .get_sandbox_info("sbx-1", &RequestOptions::new().request_timeout(None))
        .await;

    // Assert
    assert!(info.is_ok(), "{info:?}");
}

#[tokio::test]
async fn a_zero_per_call_request_timeout_means_no_limit() {
    let server = MockServer::start().await;
    slow_detail(&server, Duration::from_millis(500)).await;
    let client = client_with_timeout(&server, Some(SHORT));

    let info = client
        .get_sandbox_info("sbx-1", &RequestOptions::new().request_timeout(Some(Duration::ZERO)))
        .await;

    assert!(info.is_ok(), "{info:?}");
}

#[tokio::test]
async fn unset_options_keep_the_configured_timeout() {
    let server = MockServer::start().await;
    slow_detail(&server, Duration::from_millis(500)).await;
    let client = client_with_timeout(&server, Some(SHORT));

    let error = client
        .get_sandbox_info("sbx-1", &RequestOptions::new())
        .await
        .expect_err("the configured 200 ms applies");

    assert!(is_transport_timeout(&error), "{error:?}");
}

#[tokio::test]
async fn options_structs_carry_the_request_timeout() {
    // Arrange: every REST route answers after 2 s; configured 5 s.
    let server = MockServer::start().await;
    let slow = |body: serde_json::Value| {
        ResponseTemplate::new(200)
            .set_body_json(body)
            .set_delay(Duration::from_secs(2))
    };
    Mock::given(method("POST"))
        .and(path("/v2/sandboxes"))
        .respond_with(slow(common::sandbox_json("0.5.2")))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v2/sandboxes/sbx-1/connect"))
        .respond_with(slow(common::sandbox_json("0.5.2")))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v2/sandboxes"))
        .respond_with(slow(json!([])))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/sandboxes/sbx-1/pause"))
        .respond_with(ResponseTemplate::new(204).set_delay(Duration::from_secs(2)))
        .mount(&server)
        .await;
    let client = client_with_timeout(&server, Some(Duration::from_secs(5)));

    // Act
    let created = client
        .create_sandbox(&CreateSandboxParams::new("base").request_timeout(Some(SHORT)))
        .await;
    let connected = client
        .connect_sandbox("sbx-1", &ConnectOptions::new().request_timeout(Some(SHORT)))
        .await;
    let listed = client
        .list_sandboxes(&ListSandboxesParams::new().request_timeout(Some(SHORT)))
        .await;
    let paused = client
        .pause_sandbox("sbx-1", &PauseOptions::new().request_timeout(Some(SHORT)))
        .await;

    // Assert: each gave up on its own 200 ms.
    for (call, error) in [
        ("create", created.err()),
        ("connect", connected.err()),
        ("list", listed.err()),
        ("pause", paused.err()),
    ] {
        let error = error.unwrap_or_else(|| panic!("{call}: expected a timeout"));
        assert!(is_transport_timeout(&error), "{call}: {error:?}");
    }
}

#[tokio::test]
async fn sandbox_calls_pass_the_options_through() {
    // Arrange: a sandbox handle on the mock; its kill answers after 2 s.
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    Mock::given(method("DELETE"))
        .and(path(format!("/sandboxes/{}", sandbox.sandbox_id())))
        .respond_with(ResponseTemplate::new(204).set_delay(Duration::from_secs(2)))
        .mount(&server)
        .await;

    // Act
    let error = sandbox
        .kill(&RequestOptions::new().request_timeout(Some(SHORT)))
        .await
        .expect_err("the per-call timeout passes first");

    // Assert
    assert!(is_transport_timeout(&error), "{error:?}");
}
```

Check the route and body shapes against the existing tests before relying on them (`tests/rest_api.rs` and `tests/rest_lifecycle.rs` mount the same routes): if `list_sandboxes`, `pause_sandbox` or `common::sandbox_json` differ from the above, use what those tests use and note it.

> **Corrected (issue #125, 2026-10-29).** The routes and shapes above were right. What shipped (`tests/rest_request_options.rs`) keeps the first four tests and replaces the last two with wider ones: `every_client_call_takes_a_per_call_request_timeout` mounts one catch-all slow route (`wiremock::matchers::any()`) and checks all 18 `Client` REST calls; `every_sandbox_call_passes_its_request_timeout_through` checks the 11 `Sandbox` calls, `is_running` among them, on a sandbox made with `common::sandbox_from_json_with_client(&server, &client, created)` (the client is the second argument, by reference); `a_next_page_keeps_the_request_timeout` checks both list params after `next_token`. Unit tests in `src/{config,lifecycle,metrics,models,snapshots}.rs` check that the field never reaches a body or query. The call sites also included rustdoc examples in `src/error.rs` and `src/network.rs`, and `tests/error_taxonomy.rs`, which the file list above does not name.

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --test rest_request_options`
Expected: compile errors: `RequestOptions` is not exported, `get_sandbox_info` takes 1 argument, no `request_timeout` on `CreateSandboxParams`.

- [ ] **Step 3: `RequestOptions`**

In `src/config.rs`, after `impl RequestTimeout`:

```rust
/// Options for a call that has nothing else to configure: only its request
/// timeout. Calls that take another options struct have a
/// `request_timeout` setter on it instead.
///
/// Python note: this is the SDK's `request_timeout=` keyword argument
/// (`ApiParams`, `e2b/connection_config.py:70-78`) on calls such as
/// `Sandbox.kill` or `commands.send_stdin`.
///
/// # Examples
///
/// ```
/// use std::time::Duration;
/// use e2b_client::RequestOptions;
///
/// // Give up on this one call after five seconds.
/// let options = RequestOptions::new().request_timeout(Some(Duration::from_secs(5)));
/// # let _ = options;
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RequestOptions {
    pub(crate) request_timeout: RequestTimeout,
}

impl RequestOptions {
    /// The configured request timeout
    /// ([`ConnectionConfigBuilder::request_timeout`]).
    ///
    /// # Returns
    ///
    /// Empty options.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The request timeout for this call: `Some(d)` bounds it to `d`;
    /// `None`, or a zero duration (as SDK `request_timeout=0`), means no
    /// limit. Unset, the configured one applies.
    ///
    /// # Arguments
    ///
    /// * `timeout` - the per-call timeout, or `None` for no limit.
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

Re-export it from `src/lib.rs` (`pub use config::{..., RequestOptions};`) and add it to `tests/api_traits.rs`'s `Send`/`Sync`/`Debug` list.

- [ ] **Step 4: Resolve per call in `Client`**

In `src/client.rs`, `request` and `send_json` take the call's `RequestTimeout`:

```rust
    /// Builds a request to `api_url` plus the given path segments, each
    /// percent-encoded as a single segment, bounded by the call's request
    /// timeout (unset: the configured one).
    fn request(
        &self,
        method: Method,
        segments: &[&str],
        timeout: RequestTimeout,
    ) -> Result<RequestBuilder> {
        let mut url = self.inner.config.api_url().clone();
        url.path_segments_mut()
            .map_err(|()| Error::InvalidConfig("API URL cannot be a base URL".into()))?
            .pop_if_empty()
            .extend(segments);
        let builder = self.inner.api_http.request(method, url);
        Ok(match timeout.resolve(self.inner.config.request_timeout()) {
            Some(timeout) => builder.timeout(timeout),
            None => builder,
        })
    }

    async fn send_json<B: Serialize>(
        &self,
        method: Method,
        segments: &[&str],
        body: &B,
        timeout: RequestTimeout,
    ) -> Result<reqwest::Response> {
        self.request(method, segments, timeout)?
            .json(body)
            .send()
            .await
            .map_err(Error::transport)
    }
```

Then every public method passes its timeout. For a call without options, add `options: &RequestOptions` as the last parameter and pass `options.request_timeout`, for example:

```rust
    pub async fn get_sandbox_info(
        &self,
        sandbox_id: &str,
        options: &RequestOptions,
    ) -> Result<SandboxInfo> {
        let response = self
            .request(Method::GET, &["sandboxes", sandbox_id], options.request_timeout)?
            .send()
            .await
            .map_err(Error::transport)?;
        parse_sandbox_response(response, sandbox_id).await
    }
```

The full list is the "API after M5" table: `get_sandbox_info`, `kill_sandbox`, `set_sandbox_timeout`, `update_sandbox_network`, `refresh_sandbox`, `delete_snapshot`, `create_volume`, `get_volume`, `list_volumes`, `destroy_volume`. Each rustdoc gains the argument line "* `options` - the request timeout; [`RequestOptions::new`] for the configured one." and its example passes `&RequestOptions::new()`.

For a call with an options or params struct, give the struct a field and setter, and pass `params.request_timeout` (a `pub(crate)` field, as `FileOptions` does). For example, in `src/lifecycle.rs`:

```rust
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PauseOptions {
    mode: Option<SnapshotMode>,
    pub(crate) request_timeout: RequestTimeout,
}

impl PauseOptions {
    // ... `new` and `mode` unchanged ...

    /// The request timeout for this call: `Some(d)` bounds it to `d`;
    /// `None`, or a zero duration, means no limit. Unset, the configured
    /// one applies ([`crate::ConnectionConfigBuilder::request_timeout`]).
    ///
    /// # Arguments
    ///
    /// * `timeout` - the per-call timeout, or `None` for no limit.
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

The same field and setter go on `ConnectOptions`, `CreateSandboxParams`, `ListSandboxesParams`, `ForkOptions` (`src/models.rs`), `MetricsParams` (`src/metrics.rs`), `SnapshotOptions` and `ListSnapshotsParams` (`src/snapshots.rs`). Check each struct's body or query builder: the new field must not reach the wire (none of them derives `Serialize`; `CreateSandboxParams::body` and the list query builders pick their fields by name). The paging calls (`list_sandboxes`, `list_snapshots`) apply the timeout to each page request. `ListSandboxesParams::next_token` and `ListSnapshotsParams::next_token` must keep the timeout (callers build the next page from the same params). `create_sandbox` applies its timeout to the `POST` only (decision 6); `fork_sandbox` to each fork request.

- [ ] **Step 5: `Sandbox` delegates**

In `src/sandbox.rs`, the delegating methods take and pass the options: `kill(&RequestOptions)`, `set_timeout(Duration, &RequestOptions)`, `update_network(&NetworkUpdate, &RequestOptions)`, `refresh(Option<Duration>, &RequestOptions)`, `get_info(&RequestOptions)`; `pause`, `get_metrics`, `create_snapshot`, `list_snapshots` and `fork` pass their structs unchanged. `is_running` takes `&RequestOptions` and passes `options.request_timeout` to `http_request` instead of `RequestTimeout::default()`. Internal callers (`create_sandbox`'s kill after a failed MCP gateway, anything else `grep -n 'kill()\|get_info()' src/` finds) pass `&RequestOptions::new()`. `mcp_token` keeps its signature (decision 7).

- [ ] **Step 6: Callers**

Update every caller the compiler names in `tests/` (mock, live, sweep, `common`, `live_support`), `README.md` and the rustdoc examples. Run `just lint` and `just test` until both pass.

- [ ] **Step 7: A live check (hosted E2B)**

Add to `tests/live_lifecycle.rs`:

```rust
#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0182_a_one_millisecond_request_timeout_times_out_get_info() {
    // Arrange
    let sandbox = LiveSandbox::new().await;
    let tiny = RequestOptions::new().request_timeout(Some(Duration::from_millis(1)));

    // Act
    let timed_out = sandbox.get_info(&tiny).await;
    let configured = sandbox.get_info(&RequestOptions::new()).await;

    // Assert: 1 ms is too short for any real round trip; the configured
    // timeout is not.
    let error = timed_out.expect_err("1 ms passes before the API answers");
    assert!(error.is_timeout(), "{error:?}");
    assert!(configured.is_ok(), "{configured:?}");
}
```

Check `Error::is_timeout` covers a transport timeout (M4 Task 2: it does, through `TransportError::is_timeout`); if not, assert on `Error::Transport(t) if t.is_timeout()` and note it. Run it and the lifecycle tests whose calls changed (at least `clm_0013`, `clm_0015`, `clm_0016`), one at a time, as in the conventions.

- [ ] **Step 8: Docs**

- `docs/api-review.md`, "Known limits": the request-timeout bullet now says only the pid-only envd calls lack a per-call timeout (Task 2); add a "Method shape" note that calls with no other options take `&RequestOptions`.
- `docs/protocol-notes.md`: the request-timeout row says REST calls take a per-call timeout, citing `py: connection_config.py:70-78`.
- `README.md`: the Lifecycle row of "What it covers" mentions `RequestOptions`.
- `src/lib.rs`: the crate docs' options paragraph says that a call with no other options takes `&RequestOptions`.

- [ ] **Step 9: Changelog and commit**

The changelog entry lists every breaking change from the "API after M5" table that this task made.

```bash
git add src/ tests/ README.md docs/ changelog/
git commit -m "feat: per-call request timeouts on REST calls (part of #120)"
```

---

### Task 2: Per-call request timeouts on pid-only process and PTY calls (#120, CLM-0183)

**Files:**
- Modify: `src/commands.rs`, `src/pty.rs`, `src/lib.rs` (crate docs)
- Create: `tests/envd_request_options.rs`
- Modify (callers): `tests/envd_*.rs`, `tests/zero_timeouts.rs`, `tests/live_commands.rs`, `tests/live_support/mod.rs` where they call a changed method, `README.md`

> **Corrected (issue #120, 2026-10-29).** The callers that changed were `tests/envd_api.rs`, `tests/envd_pty.rs`, `tests/error_taxonomy.rs` (a `Commands::list`), `tests/live_commands.rs`, `README.md`, and the rustdoc examples in `src/commands.rs`, `src/pty.rs` and `src/stream_input.rs`; `tests/zero_timeouts.rs` and `tests/live_support/mod.rs` call none of the changed methods.
- Modify: `docs/api-review.md`, `docs/protocol-notes.md`
- Create: `changelog/<ts>-m5-task-02-pid-request-timeouts.md`

This PR closes #120.

- [ ] **Step 1: Write the failing tests**

Create `tests/envd_request_options.rs`:

```rust
//! Mock tests for the per-call request timeout on the pid-only process and
//! PTY calls (CLM-0183).

mod common;

use std::time::{Duration, Instant};

use common::{client_with_timeout, sandbox_from_json_with_client, sandbox_json};
use e2b_client::{Error, PtySize, RequestOptions, Signal};
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const SHORT: Duration = Duration::from_millis(200);

/// Whether `error` is a transport-level timeout (reqwest's own deadline).
fn is_transport_timeout(error: &Error) -> bool {
    matches!(error, Error::Transport(transport) if transport.is_timeout())
}

/// Mounts every unary process RPC answering `{}` after `delay`.
async fn slow_process_rpcs(server: &MockServer, delay: Duration) {
    for rpc in ["List", "SendInput", "CloseStdin", "SendSignal", "Update"] {
        Mock::given(method("POST"))
            .and(path(format!("/process.Process/{rpc}")))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({}))
                    .set_delay(delay),
            )
            .mount(server)
            .await;
    }
}

#[tokio::test]
async fn every_pid_only_call_takes_a_per_call_request_timeout() {
    // Arrange: configured 5 s; every RPC answers after 2 s.
    let server = MockServer::start().await;
    let client = client_with_timeout(&server, Some(Duration::from_secs(5)));
    let sandbox = sandbox_from_json_with_client(&server, &client, sandbox_json("0.5.2")).await;
    slow_process_rpcs(&server, Duration::from_secs(2)).await;
    let short = RequestOptions::new().request_timeout(Some(SHORT));
    let commands = sandbox.commands();
    let pty = sandbox.pty();

    // Act
    let started = Instant::now();
    let results = [
        ("list", commands.list(&short).await.err()),
        ("send_stdin", commands.send_stdin(7, "x", &short).await.err()),
        ("close_stdin", commands.close_stdin(7, &short).await.err()),
        ("send_signal", commands.send_signal(7, Signal::Sigterm, &short).await.err()),
        ("kill", commands.kill(7, &short).await.err()),
        ("pty send_stdin", pty.send_stdin(7, "x", &short).await.err()),
        ("pty resize", pty.resize(7, PtySize::new(80, 24), &short).await.err()),
        ("pty kill", pty.kill(7, &short).await.err()),
    ];

    // Assert: each gave up on its own 200 ms, so eight calls took well
    // under the 16 s the server's delay would need.
    for (call, error) in results {
        let error = error.unwrap_or_else(|| panic!("{call}: expected a timeout"));
        assert!(is_transport_timeout(&error), "{call}: {error:?}");
    }
    assert!(started.elapsed() < Duration::from_secs(5), "{:?}", started.elapsed());
}

#[tokio::test]
async fn a_per_call_none_lifts_a_short_configured_timeout() {
    // Arrange: configured 200 ms; the RPCs answer after 500 ms.
    let server = MockServer::start().await;
    let client = client_with_timeout(&server, Some(SHORT));
    let sandbox = sandbox_from_json_with_client(&server, &client, sandbox_json("0.5.2")).await;
    slow_process_rpcs(&server, Duration::from_millis(500)).await;

    // Act
    let listed = sandbox
        .commands()
        .list(&RequestOptions::new().request_timeout(None))
        .await;
    let short = sandbox.commands().list(&RequestOptions::new()).await;

    // Assert
    assert!(listed.is_ok(), "{listed:?}");
    let error = short.expect_err("the configured 200 ms applies when unset");
    assert!(is_transport_timeout(&error), "{error:?}");
}
```

Check against `tests/envd_api.rs` and `tests/envd_pty.rs`: the RPC paths (`/process.Process/<Rpc>`), whether `kill` maps a timeout through `send_signal` unchanged, and whether the base URL needs the `E2b-Sandbox-Id` routing those tests use. Use what they use. (Corrected in issue #125, 2026-10-29: `sandbox_from_json_with_client` is `(server, &client, created)`; the calls above now use that order.)

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --test envd_request_options`
Expected: compile errors: `list` takes no arguments, `send_stdin` takes 2.

- [ ] **Step 3: Pass the options through**

In `src/commands.rs`, each pid-only call takes `options: &RequestOptions` last and passes `options.request_timeout` to `unary` in place of `RequestTimeout::default()`, for example:

```rust
    pub async fn send_signal(
        &self,
        pid: u32,
        signal: Signal,
        options: &RequestOptions,
    ) -> Result<()> {
        // ... the request as today ...
        let _: Empty = self
            .envd
            .unary(
                process_pb::SERVICE,
                "SendSignal",
                &request,
                HeaderMap::new(),
                options.request_timeout,
            )
            .await?;
        Ok(())
    }

    pub async fn kill(&self, pid: u32, options: &RequestOptions) -> Result<bool> {
        match self.send_signal(pid, Signal::Sigkill, options).await {
            Ok(()) => Ok(true),
            Err(Error::Rpc {
                code: ConnectCode::NotFound,
                ..
            }) => Ok(false),
            Err(err) => Err(err),
        }
    }
```

The same for `list`, `send_stdin`, `close_stdin` (keep its envd version gate first). `CommandHandle::send_stdin(data, &RequestOptions)` and `CommandHandle::kill(&RequestOptions)` pass them to `Commands`. In `src/pty.rs`: `Pty::send_stdin(pid, data, &RequestOptions)`, `Pty::resize(pid, size, &RequestOptions)`, `Pty::kill(pid, &RequestOptions)`, and the `PtyHandle` methods. `RequestOptions::request_timeout` is a `pub(crate)` field, as on every other options struct, so `src/commands.rs` and `src/pty.rs` can read it. (Corrected in issue #125, 2026-10-29, per review: the `timeout()` accessor was dropped so every call site reads the field.) Each rustdoc gains the `options` argument line and its example passes `&RequestOptions::new()`.

- [ ] **Step 4: Callers**

Update every caller in `src/` (internal uses such as a kill in a drop path or a test helper), `tests/`, `README.md` and the rustdoc examples. Run `just lint` and `just test` until both pass.

- [ ] **Step 5: A live check (hosted E2B)**

Add to `tests/live_commands.rs`:

```rust
#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0183_a_one_millisecond_request_timeout_times_out_list() {
    // Arrange
    let sandbox = LiveSandbox::new().await;
    let tiny = RequestOptions::new().request_timeout(Some(Duration::from_millis(1)));

    // Act
    let timed_out = sandbox.commands().list(&tiny).await;
    let configured = sandbox.commands().list(&RequestOptions::new()).await;

    // Assert
    let error = timed_out.expect_err("1 ms passes before envd answers");
    assert!(error.is_timeout(), "{error:?}");
    assert!(configured.is_ok(), "{configured:?}");
}
```

Run it, and the `live_commands` tests whose calls changed (`clm_0041` to `clm_0048`, `clm_0175` and `clm_0178`, which covers the PTY tests), one at a time.

> **Corrected (issue #120, 2026-10-29).** The plan named `clm_0035` to `clm_0040`; those call only `run`, `start` and `connect`, which did not change. The tests that call a changed method are `clm_0041` (`list`), `clm_0042` to `clm_0046` (stdin, signals, kill), `clm_0047`, `clm_0048` and `clm_0175` (PTY input and resize) and `clm_0178` (through `attach_after_exit`'s `send_stdin`).

- [ ] **Step 6: Docs**

- `docs/api-review.md`, "Known limits": remove the request-timeout bullet; add one for the SDK's control-plane `retries` (not implemented, unscheduled).
- `docs/protocol-notes.md`: the pid-only calls' rows cite `py: sandbox_async/commands/command.py:64-71, 99-102, 120-124, 176-179` and `py: sandbox_async/commands/pty.py:65-68, 85-89, 256-260`; the "Not implemented yet" line about per-call timeouts goes.
- `README.md`: the Processes and PTY rows mention `RequestOptions`.
- (Added in issue #125, 2026-10-29.) `README.md`, the "Not implemented yet" sentence under "What it covers": Task 1 narrowed it to "per-call request timeouts on the pid-only process and PTY calls"; drop that clause. `src/lib.rs`: Task 1's options paragraph says "A REST call with nothing else to configure (and [`Sandbox::is_running`]) takes a [`RequestOptions`]"; widen it to the pid-only calls. `docs/api-review.md`: Task 1 added a `RequestOptions` row ("for the REST calls ... that have no other options") and a "Method shape" bullet; widen both to the pid-only calls.

- [ ] **Step 7: Changelog and commit**

```bash
git add src/ tests/ README.md docs/ changelog/
git commit -m "feat: per-call request timeouts on pid-only process and PTY calls (#120)"
```

---

### Task 3: A sandbox that failed is not a readiness result (#122)

**Files:**
- Modify: `justfile` (new private `_classify`, new `test-recipes`, the paced loop and summary)
- Create: `tests/fixtures/live-pacing/PASS--ok.txt`, `FAIL--assertion.txt`, `FAIL-READINESS--still-starting.txt`, `FAIL-READINESS--unschedulable.txt`, `FAIL-SANDBOX-FAILED--action-failure.txt`, `FAIL-READINESS--capacity-and-action-failure.txt`
- Modify: `.github/workflows/ci.yml` (a `just test-recipes` step), `CONTRIBUTING.md`, `docs/verification/live-pacing.md`
- Create: `changelog/<ts>-m5-task-03-readiness-marker.md`

- [ ] **Step 1: The fixtures (the failing test)**

Each fixture is a test's output; its name before `--` is the label the classifier must print. A `PASS` or `SKIP` fixture is classified with exit status 0, every other with 101. Contents (one or a few lines each, copied from real logs where there is one):

- `PASS--ok.txt`: `test clm_0007_create_returns_running_sandbox ... ok`
- `FAIL--assertion.txt`: `thread 'clm_0020' panicked at tests/live_lifecycle.rs:754:5:` and `assertion failed: ids.windows(2).all(|w| w[0] <= w[1])`
- `FAIL-READINESS--still-starting.txt`: `Rpc { code: Unavailable, message: "sandbox is still starting up" }`
- `FAIL-READINESS--unschedulable.txt`: `Unschedulable: 0/29 nodes are available: 19 Insufficient memory`
- `FAIL-SANDBOX-FAILED--action-failure.txt`: `create sandbox: McpGateway("ERROR: sandbox will not become ready without intervention: the sandbox failed (action_failure)\n")` (`docs/verification/runs/20261028T154205Z-sandbawks-paced-913873.log:952`)
- `FAIL-READINESS--capacity-and-action-failure.txt`: both the `still starting up` line and the `action_failure` line (a readiness marker wins: M2's capacity runs saw both)

- [ ] **Step 2: `test-recipes`**

Add to `justfile`, after `test`:

```just
# Tests the paced recipe's result classifier against tests/fixtures/live-pacing/.
test-recipes:
    #!/usr/bin/env bash
    set -uo pipefail
    status=0
    for fixture in tests/fixtures/live-pacing/*.txt; do
        name="$(basename "$fixture" .txt)"
        expected="${name%%--*}"
        code=101
        case "$expected" in PASS|SKIP) code=0 ;; esac
        actual="$({{just_executable()}} --justfile {{justfile()}} _classify "$code" "$fixture")"
        if [ "$actual" = "$expected" ]; then echo "ok   $name"
        else echo "FAIL $name: got $actual"; status=1; fi
    done
    exit "$status"
```

Run: `just test-recipes`
Expected: fails: there is no `_classify` recipe.

- [ ] **Step 3: `_classify`**

Add, next to `_stamp`:

```just
# Prints the paced result label for one test run, from its exit status and
# its output file: PASS, FAIL-READINESS (no ready sandbox: capacity),
# FAIL-SANDBOX-FAILED (the sandbox failed outright: capacity or, as with an
# unknown template, a server defect; a person classifies it) or FAIL.
# Used by `test-live-paced`; `test-recipes` tests it.
[private]
_classify status out:
    #!/usr/bin/env bash
    set -uo pipefail
    # Messages that mean the server had no ready sandbox, so the test says
    # nothing about the feature (docs/verification/live-pacing.md).
    readiness='still starting up|Service Temporarily Unavailable|Unschedulable'
    # A sandbox that failed outright (gfd/sandbawks#680 on an unknown
    # template; M2 also saw it on capacity).
    failed='the sandbox failed \(action_failure\)'
    if [ {{quote(status)}} -eq 0 ]; then echo PASS
    elif grep -Eq -- "$readiness" {{quote(out)}}; then echo FAIL-READINESS
    elif grep -Eq -- "$failed" {{quote(out)}}; then echo FAIL-SANDBOX-FAILED
    else echo FAIL; fi
```

Run: `just test-recipes`
Expected: six `ok` lines, exit 0.

- [ ] **Step 4: Use it in the paced loop**

In `test-live-paced`, remove the `readiness=` line and classify through `_classify`; both readiness and sandbox-failed results get the one rerun:

```bash
    # Classifies one run: `_classify` prints the label (see the recipe).
    classify() {
        {{just_executable()}} --justfile {{justfile()}} _classify "$1" "$2"
    }
```

```bash
            run_one "$bin" "$test" "$out"
            result="$(classify "$?" "$out")"
            case "$result" in
                PASS) ;;
                FAIL-READINESS|FAIL-SANDBOX-FAILED) retry+=("$bin $test") ;;
                *) status=1 ;;
            esac
```

and in the rerun loop:

```bash
        run_one "$bin" "$test" "$out"
        result="$(classify "$?" "$out")"
        case "$result" in
            PASS) result=PASS-ON-RERUN ;;
            *) status=1 ;;
        esac
```

> **Corrected (issue #122, 2026-10-29).** The rerun `case` shipped without the `SKIP) ;;` branch: `_classify` has no `SKIP` label until Task 4, which adds the branch back. The summary line was rewritten around a `count` helper (`count() { grep -Ec -- "^RESULT [^ ]+ ($1)\$" "$log"; }`), and its `fail` figure now counts `FAIL (rerun)` as well as `FAIL`, since a rerun can now end `FAIL`; the line reads `summary: N pass, N pass on rerun, N fail, N still not ready, N sandbox failed`. The first-pass `case` (above) ends in `*) status=1 ;;` rather than `FAIL) status=1 ;;`, so an unexpected or empty label fails the run instead of passing silently (review of PR #136). Read the `justfile`.

(the rerun's line stays `RESULT <test> <result> (rerun)`, so a rerun that fails the same way is `FAIL-SANDBOX-FAILED (rerun)` or `FAIL-READINESS (rerun)`, and a rerun that fails differently is `FAIL (rerun)`). The summary line counts `FAIL-SANDBOX-FAILED (rerun)` as its own figure: `..., N still not ready, M sandbox failed`. Keep `set -uo pipefail` semantics: `run_one` returning non-zero must not stop the loop (it does not under `-u` alone).

- [ ] **Step 5: Check it on sandbawks**

Run: `just test-live-paced sandbawks 30 'clm_0150|clm_0007'`
Expected: `clm_0150` ends `FAIL-SANDBOX-FAILED (rerun)` (the unknown `mcp-gateway` template, #680), `clm_0007` `PASS` or a known defect; the summary counts one sandbox-failed result. The sweeps' 404 still makes the recipe exit non-zero (#121, out of scope). Quote the `RESULT` and `summary:` lines in the PR.

- [ ] **Step 6: CI and docs**

- `.github/workflows/ci.yml`, job `check`, after `Test`: a step `Recipe tests` running `just test-recipes`.
- `CONTRIBUTING.md`: the tooling table gains `just test-recipes`.
- `docs/verification/live-pacing.md`: the result table gains `FAIL-SANDBOX-FAILED` and `FAIL-SANDBOX-FAILED (rerun)`; the readiness paragraph lists the three markers, says `will not become ready` is no longer one, and replaces "read a `FAIL-READINESS (rerun)` line before counting it" with the new class's meaning; mention `just test-recipes`.

- [ ] **Step 7: Changelog and commit**

```bash
git add justfile tests/fixtures/ .github/workflows/ci.yml CONTRIBUTING.md docs/verification/live-pacing.md changelog/
git commit -m "fix: count a failed sandbox apart from readiness in paced runs (#122)"
```

---

### Task 4: Volume tests skip when the team has no volumes

**Files:**
- Modify: `tests/live_support/mod.rs`, `tests/live_lifecycle.rs` (`clm_0148_volume_create_get_list_destroy`, `clm_0149_volume_mount_persists_across_sandboxes`, `clm_0153_get_info_reports_volume_mounts`)
- Modify: `justfile` (`_classify`'s `SKIP`, the summary), `docs/verification/live-pacing.md`, `CONTRIBUTING.md` (the live tests section)
- Create: `tests/fixtures/live-pacing/SKIP--volumes-not-enabled.txt`
- Create: `changelog/<ts>-m5-task-04-environment-skips.md`

- [ ] **Step 1: The fixture (the failing recipe test)**

`SKIP--volumes-not-enabled.txt`:

```text
SKIPPED (environment): volumes are not enabled for this team: use of volumes is not enabled
test clm_0148_volume_create_get_list_destroy ... ok
```

Run: `just test-recipes`
Expected: `FAIL SKIP--volumes-not-enabled: got PASS`.

- [ ] **Step 2: `SKIP` in the classifier**

In `_classify`, the first branch becomes:

```bash
    if [ {{quote(status)}} -eq 0 ]; then
        # A test that passed only because the environment can't run it
        # (tests/live_support: SKIPPED) says nothing about the feature.
        if grep -Fq 'SKIPPED (environment):' {{quote(out)}}; then echo SKIP; else echo PASS; fi
```

The paced summary counts `RESULT .* SKIP$` as `N skipped`: add `$(count SKIP) skipped` to the summary line, through Task 3's `count` helper. In both `case`s of `test-live-paced` (the first pass and the rerun loop), add `SKIP) ;;` before `*) status=1 ;;`, so a test that skips does not fail the run (Task 3 shipped without it; both end in `*) status=1 ;;`). Run `just test-recipes`: seven `ok` lines.

> **Corrected (issue #126, 2026-10-29).** The summary's figure is `$(count 'SKIP( \(rerun\))?') skipped`, not `RESULT .* SKIP$`: the rerun `case` leaves a skip's label `SKIP`, so its line is `RESULT <test> SKIP (rerun)`, and it is counted too (as `fail` counts `FAIL (rerun)`). The line reads `summary: N pass, N pass on rerun, N fail, N still not ready, N sandbox failed, N skipped`. `docs/verification/live-pacing.md`'s table gains `SKIP (rerun)` as well. Read the `justfile`.

- [ ] **Step 3: The harness helper**

In `tests/live_support/mod.rs`:

```rust
/// What a live test prints when the environment, not the crate, keeps it
/// from running; `just test-live-paced` reports such a test as `SKIP`, and
/// the results tables list it as an environment limit.
pub const SKIPPED: &str = "SKIPPED (environment):";

/// Creates a volume named `name`, or prints a [`SKIPPED`] line and returns
/// `None` when the team has volumes disabled: hosted E2B answers
/// `POST /volumes` with 403 `use of volumes is not enabled`.
///
/// # Panics
///
/// On any other error, so a real failure still fails the test.
pub async fn create_volume_or_skip(client: &Client, name: &str) -> Option<Volume> {
    match client.create_volume(name, &RequestOptions::new()).await {
        Ok(volume) => Some(volume),
        Err(Error::Api {
            status: 403,
            message,
            ..
        }) if message.contains("not enabled") => {
            eprintln!("{SKIPPED} volumes are not enabled for this team: {message}");
            None
        }
        Err(error) => panic!("create volume: {error:?}"),
    }
}
```

- [ ] **Step 4: Use it**

In the three tests, replace the `create_volume(...).await.expect(...)` with:

```rust
    let Some(volume) = create_volume_or_skip(&client, &resource_name("managed")).await else {
        return;
    };
```

(`let ... else` runs the `else` block, which must leave the function, when the pattern doesn't match: Python's `if volume is None: return`.) In `clm_0148_volume_create_get_list_destroy` the variable is `created`, named as today. `clm_0148_missing_volume_is_not_found_and_not_destroyed` is unchanged: it passes without volumes.

- [ ] **Step 5: Run them on hosted**

Run the three, one at a time (conventions). Expected: each passes with the `SKIPPED (environment): volumes are not enabled for this team: use of volumes is not enabled` line in its output. Quote one in the PR.

- [ ] **Step 6: Docs**

- `docs/verification/live-pacing.md`: the result table gains `SKIP`.
- `CONTRIBUTING.md`, "Tests": a sentence that a live test the environment can't run prints `SKIPPED (environment): <why>` and passes; a run's skips are environment limits, never evidence for a claim.

- [ ] **Step 7: Changelog and commit**

```bash
git add tests/ justfile docs/verification/live-pacing.md CONTRIBUTING.md changelog/
git commit -m "test: skip the volume live tests on a team without volumes"
```

---

### Task 5: The live CI job (CLM-0123)

**Files:**
- Create: `.github/workflows/live.yml`
- Modify: `CONTRIBUTING.md`
- Create: `changelog/<ts>-m5-task-05-live-ci-job.md`

- [ ] **Step 1: The workflow**

```yaml
name: Live (hosted E2B)

# Runs the live suite against hosted E2B (CLM-0123). Manual only: a run
# creates about 80 billed sandboxes (M5 decision 1). It needs the
# repository secret E2B_API_KEY_HOSTED. A workflow_dispatch run can only be
# started by someone with write access, and secrets never reach pull
# requests from forks, so the key stays with the repository.
on:
  workflow_dispatch:

permissions:
  contents: read

# One live run at a time: two would share the team's sandbox quota.
concurrency:
  group: live-hosted
  cancel-in-progress: false

env:
  CARGO_TERM_COLOR: always

jobs:
  live-hosted:
    name: just test-live hosted
    runs-on: ubuntu-latest
    timeout-minutes: 45
    # The key is set only on the three steps that need it, never job-wide,
    # so the third-party actions below can't read it; and those actions are
    # pinned to commit SHAs, since this job holds a billing key.
    steps:
      - uses: actions/checkout@<sha> # v4
      - uses: dtolnay/rust-toolchain@<sha> # stable
      - uses: Swatinem/rust-cache@<sha> # v2
      - uses: extractions/setup-just@<sha> # v2
      - name: Check the secret is set
        env:
          E2B_API_KEY_HOSTED: ${{ secrets.E2B_API_KEY_HOSTED }}
        run: |
          if [ -z "$E2B_API_KEY_HOSTED" ]; then
            echo "::error::Set the repository secret E2B_API_KEY_HOSTED"
            exit 1
          fi
      - name: Live tests
        env:
          E2B_API_KEY_HOSTED: ${{ secrets.E2B_API_KEY_HOSTED }}
        run: just test-live hosted
      # GitHub masks the secret in the job log, but not inside uploaded
      # files, so the run log is checked for it before upload and deleted
      # if it is there.
      - name: Check the run log for the key
        if: always()
        env:
          E2B_API_KEY_HOSTED: ${{ secrets.E2B_API_KEY_HOSTED }}
        run: |
          # An empty secret would match every line; the first step already failed.
          [ -n "$E2B_API_KEY_HOSTED" ] || exit 0
          shopt -s nullglob
          for log in target/live-logs/*.log; do
            if grep -qF -- "$E2B_API_KEY_HOSTED" "$log"; then
              echo "::error::the API key appears in $(basename "$log"); not uploading it"
              rm -f -- "$log"
              exit 1
            fi
          done
      - name: Upload the run log
        if: always()
        uses: actions/upload-artifact@<sha> # v4
        with:
          name: live-hosted-log
          path: target/live-logs/*.log
          if-no-files-found: warn
          retention-days: 30
```

Replace each `<sha>` with the full commit SHA the tag points to now, keeping the tag as the comment: `gh api repos/<owner>/<repo>/commits/<tag> --jq .sha` (for example `repos/actions/checkout/commits/v4`; `dtolnay/rust-toolchain`'s `stable` is a branch, so pin its current head). A pinned action does not move under the job; updating one is a deliberate edit. `ci.yml` keeps its tags: it holds no secret.

The harness refuses to run while a standard `E2B_API_KEY` is set (none is in CI) and reads `E2B_API_KEY_HOSTED` from the environment when there is no `.env` (`tests/live_support/mod.rs:55-73`). The `_stamp` header records the checked-out commit and `dirty: no`.

- [ ] **Step 2: Validate it**

There is no runner for a `workflow_dispatch` workflow before it is on `main`, so check the file statically: `python3 -c 'import sys, yaml; yaml.safe_load(open(sys.argv[1]))' .github/workflows/live.yml` (or `actionlint` if installed), and read it against the existing `ci.yml`. The first real run is Task 9's.

- [ ] **Step 3: Docs**

`CONTRIBUTING.md`: a "Live CI job" paragraph: the workflow, its manual trigger (`gh workflow run live.yml --ref main`, or the Actions tab), the secret it needs (`gh secret set E2B_API_KEY_HOSTED`, by a maintainer), the log artifact `live-hosted-log`, and that it costs about 80 sandboxes per run.

- [ ] **Step 4: Changelog and commit**

```bash
git add .github/workflows/live.yml CONTRIBUTING.md changelog/
git commit -m "ci: a manual live job against hosted E2B (CLM-0123)"
```

---

### Task 6: A publishable package (CLM-0124)

**Files:**
- Modify: `Cargo.toml`, `justfile` (`package-check`), `.github/workflows/ci.yml`, `CONTRIBUTING.md`
- Create: `changelog/<ts>-m5-task-06-publishable-package.md`

- [ ] **Step 1: The failing check**

Add to `justfile`, after `doc`:

```just
# Checks what `cargo publish` would upload: only the files listed in
# Cargo.toml's `include` (plus cargo's own), and that the package builds.
package-check:
    #!/usr/bin/env bash
    set -euo pipefail
    allowed='^(src/.+\.rs|README\.md|CHANGELOG\.md|LICENSE-APACHE|LICENSE-MIT|Cargo\.toml|Cargo\.toml\.orig|Cargo\.lock|\.cargo_vcs_info\.json)$'
    unexpected="$(cargo package --list --locked --allow-dirty | grep -Ev -- "$allowed" || true)"
    if [ -n "$unexpected" ]; then
        echo "files that should not be published:" >&2
        echo "$unexpected" >&2
        exit 1
    fi
    cargo package --locked --allow-dirty --quiet
    echo "package ok"
```

Run: `just package-check`
Expected: fails, listing `claims.json`, `docs/...`, `changelog/...`, `tests/...`, `.claude/...`, `.github/...`, `justfile`, `CONTRIBUTING.md`, `.gitignore`.

(`--allow-dirty` lets a contributor run it before committing; CI's checkout is clean.)

- [ ] **Step 2: `Cargo.toml`**

Replace:

```toml
# Not published to crates.io yet.
publish = false
```

with:

```toml
# What `cargo publish` uploads (M5 decision 9): the code, the README (its
# examples are doctests), the changelog and the licences. The claims
# ledger, run logs, plans and tests stay in the repository.
# `just package-check` (CI) fails if anything else would be shipped.
include = ["/src", "/README.md", "/CHANGELOG.md", "/LICENSE-APACHE", "/LICENSE-MIT"]
```

Without `publish = false` the crate can be published (the default); only Task 10 runs `cargo publish`.

Run: `just package-check`
Expected: `package ok`. Check `cargo publish --dry-run --locked --allow-dirty`: it packages, verifies, and stops with "aborting upload due to dry run". Quote its last lines in the PR.

- [ ] **Step 3: CI**

`.github/workflows/ci.yml`, job `check`, after `Docs`: a step `Package` running `just package-check`.

- [ ] **Step 4: Docs**

`CONTRIBUTING.md`: the tooling table gains `just package-check`; a sentence that a new top-level file is not shipped unless it is added to `include`.

- [ ] **Step 5: Changelog and commit**

```bash
git add Cargo.toml justfile .github/workflows/ci.yml CONTRIBUTING.md changelog/
git commit -m "build: make the crate publishable with a deliberate file list (CLM-0124)"
```

---

### Task 7: CHANGELOG.md (CLM-0185)

**Files:**
- Create: `CHANGELOG.md`
- Modify: `README.md` (a link in "Versioning")
- Create: `changelog/<ts>-m5-task-07-changelog.md`

- [ ] **Step 1: Gather**

Read the milestone changelogs (`changelog/*-m1-*.md` ... `changelog/2026-10-28T18-39-36Z-m4-milestone.md`) and Task 1 and 2's entries. 0.1.0 is commit `<sha of PR #2's merge>` (`git log --merges --format='%h %s' | grep '#2 '`).

- [ ] **Step 2: Write it**

```markdown
# Changelog

All notable changes to `e2b-client` are listed here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the crate
follows [Semantic Versioning](https://semver.org) (README, "Versioning").

## [0.2.0] - unreleased

The first release on crates.io. Parity with the official SDK's `Sandbox`
surface (e2b 2.61), live-verified on hosted E2B.

### Added
<one bullet per feature area, from M2 and M3: streamed stdin, PTY,
watch_dir, batch and streamed and gzip writes, streamed downloads, file
metadata, signed URLs; get_host, metrics, list filters, network rules,
lifecycle and idempotency key, pause and resume modes, snapshots, fork,
volumes, the MCP gateway; from M4: error classifiers, options structs with
per-call request timeouts; from M5: `RequestOptions`>

### Changed
<the behaviour changes: zero timeouts mean no limit (#52), no PTY deadline
by default (#55), at most 8 uploads in flight (#53), symlinks reported
(#30), a refused moded pause is an error (#81), MSRV 1.89>

### Removed
<the v1 connect fallback; the `_with` methods>

### Breaking changes since 0.1.0
<every row of the M4 milestone changelog's breaking-changes list and of
Tasks 1 and 2's entries, old call to new call, grouped by type>

## [0.1.0] - 2026-10-26

Git only, never published: the REST sandbox lifecycle and envd's process
and filesystem services (PR #2).

[0.2.0]: https://github.com/PeterGrace/e2b_client_rs/compare/<0.1.0 sha>...v0.2.0
[0.1.0]: https://github.com/PeterGrace/e2b_client_rs/tree/<0.1.0 sha>
```

Fill every `<...>` from the sources in Step 1; leave none. Each "Breaking changes" row names the issue or milestone task it came from.

- [ ] **Step 3: Check**

Every breaking-change row in the M4 milestone changelog and in Tasks 1 and 2's entries appears once (count them and say the count in the PR). `just package-check` now lists `CHANGELOG.md` among the packaged files. In `README.md`'s "Versioning", "Each release's breaking changes are listed in its changelog entry" links `CHANGELOG.md`.

- [ ] **Step 4: Changelog and commit**

```bash
git add CHANGELOG.md README.md changelog/
git commit -m "docs: CHANGELOG.md for 0.2.0 (CLM-0185)"
```

---

### Task 8: The README coverage table from the ledger (CLM-0184)

**Files:**
- Create: `tests/readme_coverage.rs`
- Modify: `README.md` ("Verification status")
- Create: `changelog/<ts>-m5-task-08-readme-coverage.md`

- [ ] **Step 1: Write the failing test**

```rust
//! The README's coverage table is computed from the claims ledger
//! (`claims.json`, CLM-0184): one row per area (the `s1`..`s5` tags), with
//! the active claims' current verdicts. When the ledger changes, this test
//! fails and prints the table to paste.

use std::collections::BTreeMap;

use serde_json::Value;

/// The README rows, in order: area tag and label.
const AREAS: [(&str, &str); 6] = [
    ("s1", "S1 Lifecycle"),
    ("s2", "S2 Commands and PTY"),
    ("s3", "S3 Filesystem"),
    ("s4", "S4 Wire protocol"),
    ("s5", "S5 Publishability"),
    ("root", "System (root)"),
];
const VERDICTS: [&str; 4] = ["supported", "not_enough_info", "refuted", "unverified"];
const BEGIN: &str = "<!-- coverage:begin -->";
const END: &str = "<!-- coverage:end -->";

/// The table for the ledger, as Markdown.
fn expected_table(ledger: &Value) -> String {
    // `BTreeMap` keeps the counts keyed by (area, verdict).
    let mut counts: BTreeMap<(&str, &str), usize> = BTreeMap::new();
    let claims = ledger["claims"].as_array().expect("claims array");
    for claim in claims {
        if claim["status"]["state"] != "active" {
            continue;
        }
        let verdict = claim["verdicts"]
            .as_array()
            .and_then(|verdicts| verdicts.last())
            .and_then(|last| last["verdict"].as_str())
            .unwrap_or("unverified");
        // An unknown verdict would drop out of every column, so fail instead.
        assert!(VERDICTS.contains(&verdict), "{}: unknown verdict {verdict}", claim["id"]);
        let tags = claim["tags"].as_array().expect("tags array");
        let area = AREAS
            .iter()
            .map(|(tag, _)| *tag)
            .find(|tag| tags.iter().any(|t| t == tag))
            .unwrap_or_else(|| panic!("{} has no area tag", claim["id"]));
        *counts.entry((area, verdict)).or_default() += 1;
    }
    let mut table = String::from(
        "| Area | Claims | Supported | Not enough info | Refuted | Unverified |\n\
         |---|---|---|---|---|---|\n",
    );
    let mut totals = [0usize; 4];
    for (tag, label) in AREAS {
        let row: Vec<usize> = VERDICTS
            .iter()
            .map(|verdict| counts.get(&(tag, *verdict)).copied().unwrap_or(0))
            .collect();
        for (total, count) in totals.iter_mut().zip(&row) {
            *total += count;
        }
        table.push_str(&format!(
            "| {label} | {} | {} | {} | {} | {} |\n",
            row.iter().sum::<usize>(),
            row[0],
            row[1],
            row[2],
            row[3]
        ));
    }
    table.push_str(&format!(
        "| **All** | **{}** | **{}** | **{}** | **{}** | **{}** |\n",
        totals.iter().sum::<usize>(),
        totals[0],
        totals[1],
        totals[2],
        totals[3]
    ));
    table
}

#[test]
fn readme_coverage_table_matches_the_ledger() {
    // Arrange
    let ledger: Value = serde_json::from_str(
        &std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/claims.json"))
            .expect("read claims.json"),
    )
    .expect("parse claims.json");
    let readme = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))
        .expect("read README.md");

    // Act
    let expected = expected_table(&ledger);
    let start = readme.find(BEGIN).expect("README has the coverage:begin marker") + BEGIN.len();
    let end = readme.find(END).expect("README has the coverage:end marker");
    let actual = readme[start..end].trim();

    // Assert
    assert_eq!(
        actual,
        expected.trim(),
        "README coverage table is stale; replace the lines between the markers with:\n\n{expected}"
    );
}
```

Run: `cargo test --test readme_coverage`
Expected: FAIL, "README has the coverage:begin marker".

- [ ] **Step 2: The README**

In "Verification status", after the intro sentence, add "Claims by area, from `claims.json` (`tests/readme_coverage.rs` keeps this table in step with the ledger):" and the two markers. Run the test, paste the table it prints between them, and run it again: PASS. The bullets below the table keep the per-run detail; the "Ledger:" bullet now points at the table instead of repeating its numbers.

- [ ] **Step 3: Changelog and commit**

The entry says that every PR that changes `claims.json` must update the table (the coordinator's).

```bash
git add tests/readme_coverage.rs README.md changelog/
git commit -m "docs: a README coverage table computed from the ledger (CLM-0184)"
```

---

### Task 9: Live run on hosted E2B through the CI job

**Files:**
- Create: `docs/verification/runs/<run id>.log` (from the job's artifact)
- Create: `docs/verification/m5-live-results.md`
- Create: `changelog/<ts>-m5-task-09-hosted-run.md`

**Before this task:** the repository secret `E2B_API_KEY_HOSTED` must be set. The coordinator asks pgrace to set it (`gh secret set E2B_API_KEY_HOSTED`) or to authorize the coordinator to set it from the main checkout's `.env`; the value is never printed.

- [ ] **Step 1: Count the tests**

As M4 Task 16 Step 1. Expected: 81: M4's 79 plus `clm_0182` (Task 1) and `clm_0183` (Task 2). If the code tasks added or removed others, use the real count and correct this step with a dated note.

- [ ] **Step 2: Dispatch the job on `main`**

```bash
gh workflow run live.yml --ref main
sleep 5
run_id="$(gh run list --workflow live.yml --branch main --limit 1 --json databaseId --jq '.[0].databaseId')"
gh run watch "$run_id" --exit-status
```

Run `gh run watch` in the background and wait for its exit, not a sleep loop. Expected: green, well inside the job's 45-minute limit (M4's hosted tests took about 110 s of test time, 72 + 27 + 9 s across the three binaries, plus the build; write the real duration in the results). Then download the log:

```bash
gh run download "$run_id" --name live-hosted-log --dir target/live-logs/ci-"$run_id"
```

- [ ] **Step 3: Check the log**

The log starts with `commit: <main's head>` and `dirty: no`. The three sweeps report 0 leftovers. The three volume tests each print `SKIPPED (environment): volumes are not enabled ...` and pass. The secret check (M4 Task 16 Step 3, against the main checkout's `.env`) gives 0 for both keys, `GATEWAY_ACCESS_TOKEN=` and `"token"`.

- [ ] **Step 4: Summarize**

Copy the log to `docs/verification/runs/`. Write `docs/verification/m5-live-results.md`:

```markdown
# Milestone 5 live results

Results of the Milestone 5 live run (plan Task 9): hosted E2B, through the
live CI job.

## Hosted E2B (run <run id>, <UTC time>, commit <sha>)

Run by `.github/workflows/live.yml` (Actions run <gh run id>, dispatched on
`main` at `<sha>`). Log: [`runs/<run id>.log`](runs/<run id>.log). All
<n> tests rerun as a regression check of the M5 API changes. Rows are
sorted by claim number.

| Test | Claim | Result | Failure (first line) |
|---|---|---|---|
```

Every `clm_*` test gets a row; skipped volume tests read `skip (environment: volumes not enabled)`. Classify each failure as a crate bug, a test bug or an environment limit (M4 Task 16 Step 4's list). Fix test bugs in this PR with a plan correction; report crate bugs for the coordinator to file as M5 bug tasks. Below the table: the Actions run URL (evidence for CLM-0123), and the claims the new tests back (CLM-0182 through `clm_0182`, CLM-0183 through `clm_0183`).

- [ ] **Step 5: Changelog and commit**

```bash
git add docs/verification/ changelog/
git commit -m "docs: M5 live results on hosted E2B through the CI job (#<issue>)"
```

---

### Task 10: Release 0.2.0 (coordinator)

Done by the coordinator session (it writes the ledger and runs the publish), on a branch in its own checkout.

- [ ] **Step 1: Evidence and verdicts for the claims a merged `main` shows.** Read each file before writing its summary (evidence can't be edited).

  | Claim | Kind | Locator and what it shows |
  |---|---|---|
  | CLM-0182 | test | `tests/rest_request_options.rs` @ Task 1's merge; plus demonstration `clm_0182_* @ hosted, <log>` |
  | CLM-0183 | test | `tests/envd_request_options.rs` @ Task 2's merge; plus demonstration `clm_0183_* @ hosted, <log>` |
  | CLM-0123 | demonstration | Actions run `<url>` of `live.yml` on `main` at `<sha>`: green, the log artifact, 0 leftovers, the secret check |
  | CLM-0124 | demonstration | `cargo publish --dry-run --locked` on the release branch: output ends "aborting upload due to dry run"; `just package-check` lists only the allowed files |
  | CLM-0184 | test | `tests/readme_coverage.rs` passing in CI @ Task 8's merge |
  | CLM-0185 | inspection | `CHANGELOG.md` @ Task 7's merge: the breaking-changes count against the M4 milestone changelog and Tasks 1 and 2 |

  Record `supported` for each, citing the evidence.
- [ ] **Step 2: The release branch** (`release-0.2.0`):
  - `Cargo.toml`: `version = "0.2.0"`; `cargo update -p e2b-client` (only that line of `Cargo.lock` changes; diff it).
  - `CHANGELOG.md`: `## [0.2.0] - <date -u +%Y-%m-%d>`.
  - `README.md`: the install snippet becomes `e2b-client = "0.2"` and "It is not on crates.io yet" goes; "Verification status" becomes "As of <date> (milestone 5)", with the Task 9 run and the coverage table updated (the test prints it).
  - `claims.json` with Step 1's records.
  - Gates, `just package-check`, then `cargo publish --dry-run --locked` (Step 1's CLM-0124 evidence comes from this run).
  - Open the PR (`docs: release 0.2.0`), get brooke's review, answer it, merge.
- [ ] **Step 3: Tag.** On the merge commit: `git tag -a v0.2.0 -m "e2b-client 0.2.0"` and `git push origin v0.2.0`.
- [ ] **Step 4: Ask, then publish.** Ask pgrace once: "Publish e2b-client 0.2.0 to crates.io from tag v0.2.0 now? It can be yanked but never deleted." Only on a yes: in a clean worktree of the tag, `cargo publish --locked`. Never read or print `~/.cargo/credentials.toml`. On a no, stop here and report the release as ready to publish.
- [ ] **Step 5: GitHub release.** `gh release create v0.2.0 --title "e2b-client 0.2.0" --notes-file <the 0.2.0 section of CHANGELOG.md>`.
- [ ] **Step 6: Check it landed.** `curl -s https://crates.io/api/v1/crates/e2b-client` shows `max_version` `0.2.0` (crates.io asks API clients for a descriptive User-Agent; use `-A 'e2b_client_rs release check (github.com/PeterGrace/e2b_client_rs)'`). docs.rs builds within about an hour: `https://docs.rs/crate/e2b-client/0.2.0` shows the docs, not a failed build. Record both for Task 11.

---

### Task 11: Verdicts, docs and milestone close (coordinator)

- [ ] **Step 1: CLM-0186.** Demonstration evidence: the crates.io API answer (version, publish time) and the docs.rs page for 0.2.0. Record `supported`. If the publish was declined (Task 10 Step 4), CLM-0186 stays `not_enough_info` and so does CLM-0006.
- [ ] **Step 2: Parents.**
  - **CLM-0006:** `supported` when all 17 children are (CLM-0117 to CLM-0126, CLM-0170, CLM-0179 to CLM-0181, CLM-0184 to CLM-0186). Read `derived_from` from the ledger first and list every child in the evidence.
  - **CLM-0002** stays `not_enough_info` (the egress proxy and IAM); its rationale names CLM-0182 as a new supported child.
  - **CLM-0003** is re-affirmed over its children, which now include CLM-0183.
  - **CLM-0001** stays `not_enough_info` while CLM-0002 is; the rationale says publishability (CLM-0006) is now met.
- [ ] **Step 3: Expected ledger counts.** Before M5: 179 active, 169 supported, 10 `not_enough_info`. M5 adds CLM-0182 to CLM-0186: 184 active. If all five, CLM-0123, CLM-0124 and CLM-0006 are supported: **177 supported, 7 `not_enough_info`** (CLM-0001, 0002, 0031, 0032, 0148, 0149, 0153), 0 refuted, 0 unverified. Run `mcp__claims__audit_report` and compare; explain any difference in the milestone changelog.
- [ ] **Step 4: Docs.**
  - `README.md`: "Verification status" says 0.2.0 is published, with the coverage table updated.
  - `docs/protocol-notes.md`, "Not implemented yet": drops milestone 5 and the per-call timeouts; keeps the egress proxy and IAM, the volume content API, the SDK's control-plane retries, the unscheduled and not-planned items.
  - The spec: under the Milestones table, "**M5 outcome (<date>).** Closed by epic #<n>: 0.2.0 is on crates.io (<link>); the live CI job is `.github/workflows/live.yml` (manual); the README coverage table is computed from the ledger."
- [ ] **Step 5: Milestone changelog.** `changelog/<ts>-m5-milestone.md`: what shipped, the breaking changes (a pointer to `CHANGELOG.md`), the ledger counts before and after, the Actions run, the crates.io and docs.rs links, the index of every plan correction by task, and the follow-ups (#119, #121, the egress proxy and IAM, retries).
- [ ] **Step 6:** Open the close PR, review, merge; then `github-epic-close-out`.
