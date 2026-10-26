# S4 evidence map

Which tests prove each S4 wire claim (CLM-0067 to CLM-0116) and CLM-0122
(Debug redaction). The coordinator records each test in a row as `test`
evidence in the claims ledger (Task 8): stance `supports`, or `refutes` for a
test marked `#[ignore = "refutes CLM-NNNN: ..."]`.

How to read it:

- **Test** names each test as `path::name`. When a row names several tests,
  each one asserts a different part of the claim. Together they establish the
  whole statement, and none of them does alone.
- **Covers whole claim?** is `yes` when those tests' assertions establish the
  whole statement, `partial` when part of it has no test (the Notes say which
  part), and `refuted` when a test of the claim fails on current code. A
  refuting test is kept, ignored and listed first.
- **Source** is `existing` when tests already in `main` at 5b134d0 cover the
  claim, and `new` when this task (issue #9) had to add a test or tighten an
  existing assertion.

Mapped against main at 5b134d0. To run the remaining refuting test:
`cargo test -- --ignored close_stdin_is_refused_before_envd_0_5_2`. CLM-0071
was fixed by issue #18, and its test is no longer ignored.

## Summary

| Covered by existing tests | Covered by new or tightened tests | Partial | Refuted |
|---|---|---|---|
| 29 | 20 | 1 (CLM-0085) | 1 (CLM-0109) |

## Map

| Claim | Test (path::name) | Covers whole claim? | Source | Notes |
|---|---|---|---|---|
| CLM-0067 | src/config.rs::tests::defaults_without_env; src/config.rs::tests::env_values_are_used_when_builder_is_empty | yes | new | Tightened: the default is asserted as the literal `"e2b.app"` and the env case now asserts `domain()`. Before, the env case was only implied by the API URL. |
| CLM-0068 | src/config.rs::tests::env_values_are_used_when_builder_is_empty | yes | existing | |
| CLM-0069 | src/config.rs::tests::each_setting_resolves_explicit_then_env_then_default; src/config.rs::tests::defaults_without_env | yes | new | No test read `E2B_API_URL` from the environment: the only test that set it also overrode it with a builder value. |
| CLM-0070 | src/config.rs::tests::env_values_are_used_when_builder_is_empty | yes | existing | Both envd URLs equal the `E2B_SANDBOX_URL` value. |
| CLM-0071 | src/config.rs::tests::explicit_empty_values_count_as_unset; src/config.rs::tests::each_setting_resolves_explicit_then_env_then_default; src/config.rs::tests::empty_env_values_count_as_unset | yes | new | Explicit, then env, then default holds for all four settings, and an empty value counts as unset whether it is an env var or an explicit builder value, as in the SDKs' `value or env or default` (py `connection_config.py:289-291, 310-318`). Refuted at 5b134d0, when an explicit empty builder value was kept; fixed by issue #18, which un-ignored the first test. |
| CLM-0072 | src/config.rs::tests::defaults_without_env | yes | new | Tightened: the test compared against the `DEFAULT_REQUEST_TIMEOUT` constant, so it could not catch a wrong value. It now asserts `Duration::from_secs(60)`. |
| CLM-0073 | tests/rest_api.rs::every_rest_request_carries_the_api_key | yes | new | Covers all nine REST requests: create, v2 connect, the v1 connect fallback, info, list, kill, pause, timeout and refresh. Before, only create and info asserted `x-api-key`. |
| CLM-0074 | src/config.rs::tests::env_values_are_used_when_builder_is_empty; src/config.rs::tests::every_hosted_domain_routes_rpc_through_the_shared_sandbox_host; src/config.rs::tests::envd_urls_for_custom_domain_use_per_sandbox_host | yes | new | Before, only `e2b.app` was tested of the seven hosted domains in `sandbox_domains.py:1-9`. The new test spells out all seven. |
| CLM-0075 | src/config.rs::tests::env_values_are_used_when_builder_is_empty; src/config.rs::tests::envd_urls_for_hosted_domain_split_rpc_and_direct; src/config.rs::tests::envd_urls_for_custom_domain_use_per_sandbox_host | yes | existing | The override wins; otherwise the per-sandbox host is used for hosted and custom domains alike. |
| CLM-0076 | src/sandbox.rs::tests::envd_addressing_uses_the_api_domain_else_the_configured_one; src/config.rs::tests::envd_urls_for_custom_domain_use_per_sandbox_host | yes | new | The config test only covered the URL function. The new test builds a `Sandbox` from a create response with and without `domain`. It reads the envd base URLs through test-only (`#[cfg(test)]`) accessors on `EnvdClient`, because a wire test cannot resolve real sandbox hosts. |
| CLM-0077 | tests/envd_api.rs::every_envd_request_carries_the_sandbox_id_and_port | yes | new | Covers one request on each envd path: Start and Connect streams, List and ListDir unary calls, file read, file write, and health. Before, the port header was asserted only on Start. |
| CLM-0078 | tests/envd_api.rs::envd_requests_carry_the_access_token_only_when_issued_and_never_the_api_key | yes | new | Covers the same routes, for a sandbox with an access token and one without. Before, the token was checked on Start and the file API, and the API key's absence on Start only. |
| CLM-0079 | tests/envd_api.rs::run_streams_output_to_exit_code_with_sdk_headers; tests/envd_api.rs::start_without_user_sends_no_authorization_on_current_envd; tests/envd_api.rs::legacy_envd_defaults_to_user_account | yes | existing | Covers an explicit user (`Basic cm9vdDo=`), no header on envd 0.5.7, and `user` on envd 0.3.9. |
| CLM-0080 | tests/envd_api.rs::file_api_selects_the_user_with_the_username_query | yes | new | Covers reads and writes with an explicit user, with no user on current envd, and with no user on envd 0.3.9. Before, only an explicit write, a user-less read and a legacy read were tested. |
| CLM-0081 | tests/envd_api.rs::run_streams_output_to_exit_code_with_sdk_headers; tests/envd_api.rs::connect_to_running_process_by_pid | yes | new | Tightened: the Connect stream mock now requires `keepalive-ping-interval: 50`. Before, the header was asserted only on Start. |
| CLM-0082 | src/envd/mod.rs::tests::unparseable_or_debug_versions_are_99_99_99 | yes | new | Before, only the `parse_lenient` doc test covered this (`"dev"`), and it compared against the `UNKNOWN_LATEST` constant. |
| CLM-0083 | tests/rest_api.rs::create_sandbox_posts_v2_body_with_api_key | yes | existing | |
| CLM-0084 | src/models.rs::tests::create_body_uses_spec_field_names; src/models.rs::tests::create_body_omits_unset_optionals | yes | existing | `metadata` and `envVars` are always sent, as `{}` when unset. The SDK does the same (`metadata or {}`, `env_vars or {}`: py `sandbox_async/sandbox_api.py:187-189`, e2b 2.21.1), so only the optional fields are ever omitted. |
| CLM-0085 | src/models.rs::tests::sandbox_created_parses_fields_and_optional_tokens; tests/rest_api.rs::create_sandbox_posts_v2_body_with_api_key | partial | new | `templateID`, `sandboxID`, `envdVersion` and the three optional fields are parsed and asserted. `clientID` is accepted but not parsed: `SandboxCreated` has no field for it, so no test can assert it. For the coordinator: either the claim drops `clientID`, or the crate parses it. |
| CLM-0086 | tests/rest_api.rs::connect_sandbox_uses_v2_with_timeout; tests/rest_api.rs::connect_sandbox_without_timeout_sends_empty_body | yes | existing | Covers `{"timeout":120}` answered with 201, and `{}` answered with 200. |
| CLM-0087 | tests/rest_api.rs::connect_sandbox_falls_back_to_v1_route; tests/rest_api.rs::connect_missing_sandbox_is_not_found | yes | existing | Without a timeout, the v1 body is `{"timeout":300}`. When both routes answer 404 (`expect(2)`), the result is `SandboxNotFound`. |
| CLM-0088 | tests/rest_api.rs::get_sandbox_info_parses_detail; tests/rest_api.rs::get_sandbox_info_404_is_not_found | yes | new | Tightened: the 404 test asserted `is_not_found()`, which `Error::Api { status: 404 }` would also pass. It now matches `SandboxNotFound(id)`. |
| CLM-0089 | src/models.rs::tests::sandbox_info_parses_detail; tests/rest_api.rs::get_sandbox_info_parses_detail | yes | existing | One parses `paused`, the other `running`. |
| CLM-0090 | tests/rest_api.rs::list_sandboxes_sends_filters_and_reads_next_token | yes | existing | |
| CLM-0091 | tests/rest_api.rs::list_sandboxes_sends_filters_and_reads_next_token | yes | existing | |
| CLM-0092 | src/models.rs::tests::list_query_matches_sdk_encoding | yes | existing | The plan expected a gap here. There is none: the test asserts the whole query vector, with exactly one `("state", "running,paused")` pair. |
| CLM-0093 | src/models.rs::tests::list_query_matches_sdk_encoding; src/models.rs::tests::list_query_form_encodes_several_metadata_pairs_into_one_string | yes | new | The existing test had a single pair. The new one checks that two pairs are form-encoded into one `metadata` string. |
| CLM-0094 | tests/rest_api.rs::kill_sandbox_reports_whether_it_existed | yes | existing | |
| CLM-0095 | tests/rest_api.rs::pause_sandbox_maps_status_codes | yes | existing | |
| CLM-0096 | tests/rest_api.rs::set_timeout_and_refresh_send_seconds; tests/rest_api.rs::set_timeout_on_missing_sandbox_is_not_found | yes | existing | |
| CLM-0097 | tests/rest_api.rs::set_timeout_and_refresh_send_seconds | yes | existing | Covers `{"duration":60}` and `{}`, both answered with 204. |
| CLM-0098 | tests/rest_api.rs::create_sandbox_surfaces_e2b_error_body; tests/rest_api.rs::kill_sandbox_server_error_is_api_error | yes | existing | A JSON body's `message` is used; a plain-text body is used as is. |
| CLM-0099 | tests/envd_api.rs::rpcs_post_uncompressed_connect_json_to_service_method_paths | yes | new | Each RPC is a POST to `/<package.Service>/<Method>` with a JSON content type and no `content-encoding` or `connect-content-encoding`. Its body is bare JSON (unary) or one JSON envelope with flag 0 (streaming). |
| CLM-0100 | tests/envd_api.rs::run_streams_output_to_exit_code_with_sdk_headers; tests/envd_api.rs::list_processes_unary; src/envd/connect.rs::tests::protocol_headers_include_version_and_timeout | yes | existing | |
| CLM-0101 | src/envd/connect.rs::tests::protocol_headers_include_version_and_timeout; tests/envd_api.rs::run_streams_output_to_exit_code_with_sdk_headers; tests/envd_api.rs::connect_to_running_process_by_pid | yes | existing | Covers 10 µs sent as `1`, 60 s as `60000` and 5 s as `5000`. |
| CLM-0102 | src/envd/connect.rs::tests::envelope_header_layout_is_flag_then_big_endian_length; src/envd/connect.rs::tests::envelope_round_trip_across_arbitrary_chunk_boundaries; src/envd/connect.rs::tests::partial_envelope_waits_for_more_bytes | yes | existing | |
| CLM-0103 | src/envd/connect.rs::tests::end_stream_with_error_maps_code; tests/envd_api.rs::end_of_stream_error_frame_becomes_rpc_error; tests/envd_api.rs::next_event_yields_chunks_in_order_and_skips_keepalives | yes | existing | The last test's stream ends with a flag `0x02` envelope, after which `next_event` returns `None`. |
| CLM-0104 | src/error.rs::tests::wire_spelling_round_trips_for_every_code; src/envd/connect.rs::tests::connect_error_body_is_used_verbatim | yes | new | Tightened: the round trip compared `as_wire` against `from_wire`, so a wrong spelling could round-trip unnoticed. It now checks all 16 snake_case spellings, written out. |
| CLM-0105 | src/envd/connect.rs::tests::plain_http_error_maps_status; src/envd/connect.rs::tests::plain_json_error_with_integer_code_uses_it_as_status; src/error.rs::tests::plain_http_status_mapping_matches_python_sdk_table; tests/envd_api.rs::unary_plain_proxy_error_maps_status | yes | existing | |
| CLM-0106 | tests/envd_api.rs::run_streams_output_to_exit_code_with_sdk_headers | yes | existing | The decoded Start body is asserted in full. |
| CLM-0107 | tests/envd_api.rs::stream_not_opening_with_start_event_is_protocol_error; tests/envd_api.rs::first_event_must_arrive_within_the_request_timeout; tests/envd_api.rs::next_event_yields_chunks_in_order_and_skips_keepalives | yes | new | `slow_stream_open_times_out_with_request_timeout` delays only the response headers. The new test sends headers at once and then withholds the start event. The last test asserts that the pid comes from the start event. |
| CLM-0108 | tests/envd_api.rs::clean_end_without_exit_event_fails_wait | yes | existing | |
| CLM-0109 | **tests/envd_api.rs::close_stdin_is_refused_before_envd_0_5_2 (ignored, refutes)** | refuted | new | `Commands::close_stdin` has no version check, so it succeeds on envd 0.5.1. The SDK refuses it below `ENVD_ENVD_CLOSE = 0.5.2` (py `envd/versions.py`). |
| CLM-0110 | tests/envd_api.rs::read_file_downloads_bytes; tests/envd_api.rs::legacy_envd_defaults_to_user_account | yes | existing | Covers `path` without `username` returning the raw bytes, and `path` with `username`. |
| CLM-0111 | tests/envd_api.rs::write_file_uploads_multipart | yes | existing | |
| CLM-0112 | tests/envd_api.rs::read_missing_file_is_envd_not_found; tests/envd_api.rs::write_file_disk_full_is_envd_error | yes | existing | |
| CLM-0113 | tests/envd_api.rs::list_dir_with_zero_depth_is_rejected_locally | yes | new | Tightened: the test now also asserts that no request besides the create call reached the server. |
| CLM-0114 | src/envd/filesystem_pb.rs::tests::entry_info_parses_proto_json; src/filesystem.rs::tests::file_type_mapping; tests/envd_api.rs::list_dir_and_stat_parse_entries | yes | existing | |
| CLM-0115 | tests/envd_api.rs::is_running_checks_envd_health | yes | existing | The plan expected a gap here. There is none: `GET /health` answered with 204 gives `true`, and answered with 502 gives `false`. |
| CLM-0116 | tests/envd_api.rs::next_event_yields_chunks_in_order_and_skips_keepalives; src/envd/process_pb.rs::tests::end_event_parses_exit_code_and_error; src/envd/process_pb.rs::tests::end_event_defaults_apply_when_fields_are_omitted | yes | new | No test parsed an end event's `error` field. The new test parses `exitCode` and `error` together. |
| CLM-0122 | src/sandbox.rs::tests::debug_output_omits_the_api_key_and_access_tokens; src/config.rs::tests::debug_output_redacts_api_key | yes | new | Before, only the `Debug` output of `ConnectionConfig` was tested. The new test renders `Client`, `ConnectionConfig` and `Sandbox`, built with a real API key, envd access token and traffic access token. It asserts that none of the three appears. |
