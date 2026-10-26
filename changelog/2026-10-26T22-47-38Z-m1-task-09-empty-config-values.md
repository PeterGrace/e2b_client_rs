# M1 Task 9: an explicit empty config value counts as unset (issue #18)

- Fixes CLM-0071. `ConnectionConfigBuilder` now treats an explicit empty
  value as unset for `api_key`, `api_url`, `domain` and `sandbox_url`, so it
  falls through to the env var and then the default. Before, `.api_url("")`
  made `build()` fail with `InvalidConfig`, and `.domain("")` kept `""` as
  the domain. This matches the Python SDK's `value or env or default`
  (`py: connection_config.py:289-291, 310-318`).
- `src/config.rs`: a private `non_empty` helper filters the builder's string
  values in `build_with_env`, and the API key is filtered on its exposed
  value. The module docs, the builder rustdoc (with a new example) and each
  affected setter state the rule.
- `config::tests::explicit_empty_values_count_as_unset` is no longer
  ignored and passes.
- `docs/verification/s4-evidence-map.md`: CLM-0071 moves from refuted to
  covered (summary: 20 covered by new tests, 1 refuted).
- Plan: new Task 9 with a dated Corrected note, added under Task 8 Step 3.
