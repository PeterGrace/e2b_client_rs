# M4 Task 14: An example on every public type and async method (issue #97)

Plan: `docs/superpowers/plans/2026-10-28-milestone-4-api-stability.md`, Task 14.
Claim: CLM-0120.

## The rule

Every public type re-exported from `src/lib.rs` and every `pub async fn`
has an `# Examples` section in its rustdoc. Builder setters and accessors
are covered by their type's example. Examples that need a sandbox or the
API are `no_run` with the crate's hidden `# async fn demo(..) ->
e2b_client::Result<()>` wrapper, so `cargo test` compiles them against the
current signatures without a network. Pure-value examples
(`ConnectionConfig`, `EnvdVersion`, `ConnectCode`, `OnTimeout`, `SortOrder`)
run.

## The test

`tests/doc_examples.rs` reads `src/` and fails, naming each item by file
and line, when one lacks an example:

- `every_public_type_and_async_method_has_an_example`: the rule.
- `reexported_names_reads_single_and_grouped_uses`: the `pub use` parser.
- `has_examples_looks_through_attributes_only`: the doc-comment search
  skips `#[...]` attributes and plain `//` comments, and stops at a blank
  line or code.

The plan's helper stopped at a `//` comment. `CreateSandboxParams`,
`PtyOptions` and `PtyConnectOptions` keep a `//` note on their
`#[derive]` below an existing example, so it reported them as missing. The
helper now looks through `//` lines, and the plan is corrected.

## Coverage

| | Before (89218d1) | After |
|---|---|---|
| Re-exported public types | 28 of 67 | 67 of 67 |
| Public async functions | 18 of 69 | 69 of 69 |

The first run of the new test listed 93 items: the 90 that lacked one (39
types, 51 async functions), plus the three false positives above. The numbers are appended to `docs/api-review.md`.

## Breaking changes

None: rustdoc and tests only.
