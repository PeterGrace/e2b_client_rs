# Claims audit and road to a publishable crate

- **Date:** 2026-10-26
- **Status:** approved (brainstorming and spec review, 2026-10-26)
- **Scope:** how we decide whether `e2b-client` is a valid E2B client, and the
  milestones that take it from v0.1 to a crate published on crates.io.

## Background

v0.1 (PR #2, closes #1) was built ad hoc from issue #1 with no design doc. It
covers the REST sandbox lifecycle and envd's process and filesystem services.
Every wire detail is cited to the e2b 2.61 Python/JS SDK sources in
`docs/protocol-notes.md`, and `cargo test` passes: 111 tests pass and 3 are
ignored. But:

- it has never made an authenticated call to a live server. The 3 live
  integration tests are `#[ignore]` and have never been run;
- every test runs against wiremock, so the tests show that the crate matches
  the SDKs as read from their source, not that it works against a real server;
- it lacks parts of the official SDK's `Sandbox` surface (PTY, directory
  watching, `get_host`, signed URLs, streamed stdin, metrics, snapshots,
  network);
- the claims ledger is empty, so no requirement or claim has been written
  down or checked.

## Decisions

1. **The bar is "publishable crate":** parity with the official SDK's core
   `Sandbox` surface, live-verified, with a reviewed and stable public API, fit
   to publish on crates.io.
2. **Claims first.** We register what the crate claims and what parity needs
   as claims in the ledger, verify v0.1 live, and let the verdicts drive the
   plan. No new features until v0.1 has been verified (M1).
3. **Hosted E2B is canonical.** A claim is judged against hosted E2B. When
   sandbawks lacks or breaks something hosted E2B does, that goes on record
   as a sandbawks defect (gfd/sandbawks epic #673) and does not count against
   the crate.
4. **Both servers get tested.** pgrace provides a hosted E2B API key and a
   sandbawks team key.
5. **The v1 connect fallback goes away** in M4, unless verification shows
   something still needs it. sandbawks 0.1.28 serves the v2 connect route, so
   the fallback only made up for a non-canonical server.

## Claim hierarchy

All claims live in the claims MCP ledger. The tree:

- **System root:** "`e2b-client` is a publishable async Rust client for the
  standard E2B API, usable against hosted E2B and E2B-compatible servers."
  - **S1 Lifecycle parity:** one unit claim per SDK `Sandbox`/`SandboxApi`
    operation: create, connect/resume, get_info, list (with paging and
    filters), kill, pause, set_timeout, refresh, is_running, get_host,
    get_metrics, snapshots/fork, network updates, and the create fields the
    SDK sends.
  - **S2 Commands parity:** start, run, connect, list, send_stdin,
    close_stdin, send_signal, kill, output streaming, streamed stdin
    (`StreamInput`), and PTY (create, resize, send input, kill).
  - **S3 Filesystem parity:** read, write, list, get_info, exists, make_dir,
    rename, remove, batch write, `watch_dir`, and signed upload/download URLs.
  - **S4 Wire correctness:** one claim per fact row in
    `docs/protocol-notes.md`, for example "envd requests carry
    `E2b-Sandbox-Id`" or "list paging reads `x-next-token`".
  - **S5 Publishability:**
    - live-verified on hosted E2B and on sandbawks;
    - public enums and structs are `#[non_exhaustive]` where they may grow;
    - every public item has a rustdoc comment with an example where useful;
    - the semver and MSRV policy is stated;
    - no secret shows up in `Debug` output;
    - a live CI job runs behind repository secrets;
    - `publish = true`.

The ledger holds 126 claims (CLM-0001 to CLM-0126).

**Fields:**

- `level`: system for the root, software for S1 to S5, unit for the leaves.
- `kind`: functional for behaviour, interface for wire facts,
  documentation, constraint or security for S5 as fitting.
- `source`: the protocol-notes row, the SDK file and line, or this spec's
  section.
- `derived_from`: the parent claim.

**Tags:**

- `v0.1` for behaviour the crate claims today, and `gap` for behaviour it
  lacks;
- `s1` to `s5` for the parent area;
- `hosted` and `sandbawks` on live demonstration evidence summaries, not on
  the claims themselves.

**Verdict rules:**

- `supported` needs supporting evidence. For live behaviour (S1 to S3) that
  means demonstration evidence from hosted E2B. Supporting evidence from
  sandbawks is recorded too, and a sandbawks refutation is recorded as
  evidence that names gfd/sandbawks #673. It does not refute the claim.
- `refuted` needs evidence that refutes the claim on hosted E2B, or a failing
  mock test for an S4 wire claim.
- A `gap` claim gets `not_enough_info` until it is implemented.

## M1 verification harness

- `tests/integration.rs` grows into a suite that runs once per target.
  `E2B_TEST_TARGET=hosted|sandbawks` picks the target. Keys are
  `E2B_API_KEY_HOSTED` and `E2B_API_KEY_SANDBAWKS` in `.env` (already
  gitignored), loaded by `dotenvy` as a dev-dependency only. The library keeps
  reading only the standard `E2B_*` variables. The harness builds each
  target's `ConnectionConfig` explicitly from the per-target values, and it
  refuses to run while a standard `E2B_API_KEY`, `E2B_API_URL`,
  `E2B_DOMAIN` or `E2B_SANDBOX_URL` is set, so a run can't silently reach
  the wrong server.
- `just test-live <target>` replaces `just test-integration`. The tests stay
  `#[ignore]`, so `cargo test` never reaches the network.
- Each test is named after the claim it backs, for example
  `clm_0020_list_pages_without_duplicates_or_omissions`, so evidence locators are exact.
- Cost and leak control, since hosted E2B bills per sandbox-second:
  - each test uses its own sandbox with a 60 s timeout;
  - a drop guard kills the sandbox even if the test panics;
  - every sandbox carries the metadata `e2b-client-test=<run id>`;
  - a final sweep lists sandboxes by that metadata and kills any that are
    left.
- Evidence flow:
  1. Run the suite and save the output as a dated run log under
     `docs/verification/runs/`. The per-task changelog entry links to it.
  2. For each test, call `add_evidence` with kind `demonstration`, locator
     `<test name> @ <target>, <run log path>`, and an honest stance.
  3. Call `record_verdict`, citing that evidence.
  
  The wiremock tests become `test` evidence for the S4 claims.
- **Exit criterion:** every `v0.1` claim has a verdict on hosted E2B, and
  every refuted claim has a bug sub-issue that is fixed and re-verified
  within M1.

## Milestones

| Milestone | Contents | Gated on |
|---|---|---|
| M1 Verify v0.1 | Ledger populated; live harness; verdicts on both servers; fixes for refuted claims | none |
| M2 Command and file parity | Streamed stdin, PTY, batch write, `watch_dir`, signed upload/download URLs | M1 |
| M3 Lifecycle parity | `get_host(port)`, `get_metrics`, snapshots/fork, network updates, the remaining create fields from SDK 2.61 | M1 |
| M4 API stability | `#[non_exhaustive]` review, error taxonomy pass, drop the v1 connect fallback, naming and builder consistency (Rust API Guidelines), pinned MSRV, rustdoc examples | M2, M3 |
| M5 Release | Live CI job (hosted only, behind secrets, manual or nightly trigger), README coverage table from the ledger audit, CHANGELOG, `publish = true`, 0.2.0 on crates.io | M4 |

M2 and M3 are independent and may run in parallel. M4 waits for both, so
the API review happens once, after the surface stops growing. Each of M2 to
M5 gets its own short design and plan when it starts; this spec does not
design PTY, watching or the other gap features.

## Tracking

- M1 becomes a GitHub epic on `PeterGrace/e2b_client_rs` with one sub-issue
  per task: populate the ledger, build the harness, run on hosted E2B, run on
  sandbawks, and one per bug found.
- M2 to M5 get GitHub milestones now and sub-issues when their designs are
  done.
- The knowledge catalog concept `projects/e2b-client-rs.md` records the bar,
  the milestones and the ledger as the source of truth for completeness.

## Out of scope

- Code interpreter: a separate upstream package, and possibly a sibling crate
  later.
- Template building.
- A blocking (sync) API.
- Anything that only sandbawks serves. That belongs in a separate
  `sandbawks_client` crate.
- Regenerating models from the OpenAPI spec or protos. M4 may revisit this
  for the REST layer.
