# M3 Task 3: List filters (issue #62)

Plan: `docs/superpowers/plans/2026-10-27-milestone-3-lifecycle-parity.md`, Task 3.
Claims: CLM-0151 (S1), CLM-0166 (S4).

## What changed

- New `SortOrder` enum (`Ascending` → `asc`, `Descending` → `desc`;
  `#[non_exhaustive]`), re-exported from the crate root.
- `ListSandboxesParams` gains `order(SortOrder)`, `started_after(DateTime<Utc>)`
  (sent as `startedAfter` in Python's `isoformat` form: `+00:00`, and six
  fractional digits only when the microseconds are not zero) and `template(..)` (an ID or name, sent as
  given). Each is left out of the query when unset. `query()` emits the pairs
  in the order `metadata, state, order, startedAfter, template, limit,
  nextToken`.
- `src/client.rs`: the `x-next-token` read moved out of `list_sandboxes` into
  the private helper `next_token_of(&reqwest::Response) -> Option<String>`,
  which Task 8 (snapshot listing) reuses.
- Tests: mock tests `list_sends_order_started_after_and_template` and
  `list_omits_unset_filters` (`tests/rest_list.rs`, new); unit test
  `list_query_orders_new_filters_like_the_sdk` (`src/models.rs`); the
  `ListSandboxesParams` doctest now uses the new setters; live test
  `clm_0151_list_filters_by_template_and_start_time_and_orders`.
- Docs: the "List" row in `docs/protocol-notes.md` names the three filters,
  and `docs/verification/s4-evidence-map.md` has the CLM-0166 row.

## Plan divergences

The plan's code compiled as written, apart from `rustfmt` line breaks. In
review (brooke-reviews), `startedAfter` moved from `to_rfc3339()` to a
private `isoformat` helper: chrono writes 3, 6 or 9 fractional digits
depending on the value, Python's `isoformat()` writes none or exactly 6, so a
millisecond or nanosecond time was sent differently from the SDK. The helper
truncates to microseconds as Python must. The plan has a dated correction
note at Task 3 and its chrono fact row is updated, and the unit test
`isoformat_matches_python_for_whole_milli_micro_and_nanosecond_times` pins
the form.
Hosted E2B accepted the template filter as the template ID that `get_info`
reports, which is what the live test passes.

## Verification

- `just lint`, `just test`, `just doc`: green (counts in the PR).
- Live, hosted E2B, run `dev-20261027T231745Z-1341794`:
  `clm_0151_list_filters_by_template_and_start_time_and_orders` passed in
  5.10 s.
- After the review change, run `dev-20261027T232232Z-1390098`: the same test
  passed in 3.64 s.
