# Milestone 5 live results

Results of the Milestone 5 live run (plan Task 9): hosted E2B, through the
live CI job.

## Hosted E2B (run 20261029T024539Z-hosted-2513, 2026-10-29T02:45:39Z, commit d9c6401)

Run by `.github/workflows/live.yml` (Actions run 40808242984, dispatched on
`main` at `d9c6401`: main with every M5 code task, M5.01 to M5.08, issues #125,
#120, #122, #126, #127, #128, #129 and #130). The log starts `commit:
d9c6401907cf561945b0a7810814fbea4779045d` and `dirty: no`. Log:
[`runs/20261029T024539Z-hosted-2513.log`](runs/20261029T024539Z-hosted-2513.log)
(the job's `live-hosted-log` artifact, byte for byte; the job sets
`CARGO_TERM_COLOR: always`, so cargo's status lines carry ANSI colour codes).
All 81 tests rerun as a regression check of the M5 API changes. Rows are
sorted by claim number, and a claim's tests keep their source order.

| Test | Claim | Result | Failure (first line) |
|---|---|---|---|
| clm_0007_create_returns_running_sandbox | CLM-0007 | pass | |
| clm_0008_create_metadata_is_visible_in_info | CLM-0008 | pass | |
| clm_0009_create_env_vars_reach_commands | CLM-0009 | pass | |
| clm_0010_create_timeout_sets_end_at | CLM-0010 | pass | |
| clm_0011_auto_pause_pauses_at_timeout | CLM-0011 | pass | |
| clm_0012_internet_access_false_blocks_outbound | CLM-0012 | pass | |
| clm_0013_connect_running_sandbox_runs_commands | CLM-0013 | pass | |
| clm_0014_connect_resumes_paused_sandbox_with_files | CLM-0014 | pass | |
| clm_0015_connect_unknown_sandbox_is_not_found | CLM-0015 | pass | |
| clm_0016_get_info_returns_detail | CLM-0016 | pass | |
| clm_0017_get_info_unknown_sandbox_is_not_found | CLM-0017 | pass | |
| clm_0018_list_filters_by_metadata | CLM-0018 | pass | |
| clm_0019_list_filters_by_state | CLM-0019 | pass | |
| clm_0020_list_pages_without_duplicates_or_omissions | CLM-0020 | pass | |
| clm_0021_kill_running_sandbox | CLM-0021 | pass | |
| clm_0022_kill_unknown_sandbox_is_false | CLM-0022 | pass | |
| clm_0023_pause_running_sandbox | CLM-0023 | pass | |
| clm_0024_pause_paused_sandbox_is_false | CLM-0024 | pass | |
| clm_0025_set_timeout_moves_end_at | CLM-0025 | pass | |
| clm_0026_refresh_extends_end_at | CLM-0026 | pass | |
| clm_0027_is_running_tracks_kill | CLM-0027 | pass | |
| clm_0028_get_host_reaches_a_port_in_the_sandbox | CLM-0028 | pass | |
| clm_0029_get_metrics_reports_cpu_memory_and_disk | CLM-0029 | pass | |
| clm_0030_snapshot_starts_sandboxes_with_its_files | CLM-0030 | pass | |
| clm_0030_fork_starts_running_copies | CLM-0030 | pass | |
| clm_0033_run_returns_output_and_exit_code | CLM-0033 | pass | |
| clm_0034_start_streams_chunks_then_exit | CLM-0034 | pass | |
| clm_0035_run_delivers_large_output_in_order | CLM-0035 | pass | |
| clm_0036_options_env_is_set | CLM-0036 | pass | |
| clm_0037_options_cwd_is_working_directory | CLM-0037 | pass | |
| clm_0038_options_user_runs_as_user | CLM-0038 | pass | |
| clm_0039_options_timeout_ends_long_command | CLM-0039 | pass | |
| clm_0040_connect_attaches_to_running_command | CLM-0040 | pass | |
| clm_0041_list_includes_background_command | CLM-0041 | pass | |
| clm_0042_send_stdin_reaches_command | CLM-0042 | pass | |
| clm_0043_close_stdin_sends_eof | CLM-0043 | pass | |
| clm_0044_sigterm_terminates_command | CLM-0044 | pass | |
| clm_0045_kill_reports_whether_pid_existed | CLM-0045 | pass | |
| clm_0046_stream_input_delivers_chunks_in_order | CLM-0046 | pass | |
| clm_0047_pty_create_streams_terminal_output | CLM-0047 | pass | |
| clm_0048_pty_resize_changes_terminal_size | CLM-0048 | pass | |
| clm_0049_pty_input_reaches_the_shell | CLM-0049 | pass | |
| clm_0050_write_read_round_trips_binary | CLM-0050 | pass | |
| clm_0051_read_to_string_returns_text | CLM-0051 | pass | |
| clm_0052_write_returns_write_info | CLM-0052 | pass | |
| clm_0053_read_missing_file_is_not_found | CLM-0053 | pass | |
| clm_0054_list_depth_one_lists_direct_entries | CLM-0054 | pass | |
| clm_0055_list_depth_two_recurses | CLM-0055 | pass | |
| clm_0056_get_info_reports_metadata | CLM-0056 | pass | |
| clm_0057_exists_true_and_false | CLM-0057 | pass | |
| clm_0058_make_dir_reports_creation | CLM-0058 | pass | |
| clm_0059_rename_moves_entry | CLM-0059 | pass | |
| clm_0060_remove_deletes_file_and_directory | CLM-0060 | pass | |
| clm_0061_user_selects_file_owner | CLM-0061 | pass | |
| clm_0062_write_files_writes_every_file | CLM-0062 | pass | |
| clm_0063_watch_dir_reports_a_created_file | CLM-0063 | pass | |
| clm_0064_upload_url_accepts_a_multipart_post | CLM-0064 | pass | |
| clm_0065_download_url_fetches_without_credentials | CLM-0065 | pass | |
| clm_0066_write_stream_uploads_more_than_it_buffers | CLM-0066 | pass | |
| clm_0128_read_stream_downloads_in_chunks | CLM-0128 | pass | |
| clm_0129_write_metadata_round_trips | CLM-0129 | pass | |
| clm_0143_create_network_allow_and_deny_lists_are_enforced | CLM-0143 | pass | |
| clm_0144_update_network_replaces_egress_without_restart | CLM-0144 | pass | |
| clm_0145_lifecycle_pause_with_full_snapshot_pauses_at_timeout | CLM-0145 | pass | |
| clm_0146_resume_restores_memory_or_reboots | CLM-0146 | pass | |
| clm_0146_filesystem_pause_cold_boots | CLM-0146 | pass | |
| clm_0147_list_and_delete_snapshots | CLM-0147 | pass | |
| clm_0148_volume_create_get_list_destroy | CLM-0148 | skip (environment: volumes not enabled) | |
| clm_0148_missing_volume_is_not_found_and_not_destroyed | CLM-0148 | pass | |
| clm_0149_volume_mount_persists_across_sandboxes | CLM-0149 | skip (environment: volumes not enabled) | |
| clm_0150_mcp_gateway_starts_and_exposes_url_and_token | CLM-0150 | pass | |
| clm_0151_list_filters_by_template_and_start_time_and_orders | CLM-0151 | pass | |
| clm_0153_get_info_reports_network | CLM-0153 | pass | |
| clm_0153_get_info_reports_lifecycle | CLM-0153 | pass | |
| clm_0153_get_info_reports_volume_mounts | CLM-0153 | skip (environment: volumes not enabled) | |
| clm_0171_moded_pause_of_a_paused_sandbox_is_false | CLM-0171 | pass | |
| clm_0175_pty_outlives_sixty_seconds | CLM-0175 | pass | |
| clm_0177_symlinks_report_symlink_type | CLM-0177 | pass | |
| clm_0178_pty_connect_to_exited_pid | CLM-0178 | pass | |
| clm_0182_a_one_millisecond_request_timeout_times_out_get_info | CLM-0182 | pass | |
| clm_0183_a_one_millisecond_request_timeout_times_out_list | CLM-0183 | pass | |

**Totals:** 81 `clm_*` tests (Task 9 Step 1's count: 41 in `live_lifecycle`,
20 in `live_commands`, 20 in `live_files`, as the plan expects): 78 passed,
3 skipped on an environment limit, 0 failed (`test result: ok` for all three
binaries: commands 20/20, files 20/20, lifecycle 41/41). That is M4's 79 plus
M5's `clm_0182` and `clm_0183`, both passed. The three sweep tests are not
claim tests and are not counted. One dispatch, no reruns, no test changed.

**Skips:** `clm_0148_volume_create_get_list_destroy`,
`clm_0149_volume_mount_persists_across_sandboxes` and
`clm_0153_get_info_reports_volume_mounts` each print `SKIPPED (environment):
volumes are not enabled for this team: use of volumes is not enabled` in
their output section and pass (Task 4's skip: `POST /volumes` answers 403
with that message, as in M3 and M4). They say nothing about the volume
claims. `clm_0148_missing_volume_is_not_found_and_not_destroyed` needs no
volume and passed.

**Sweeps:**

- `sweep: killed 0 leftover sandbox(es) of run 20261029T024539Z-hosted-2513`
- `sweep: deleted 0 leftover snapshot(s) of run 20261029T024539Z-hosted-2513`
- `sweep: destroyed 0 leftover volume(s) of run 20261029T024539Z-hosted-2513`

The log has no `could not kill` warning.

**Secret check (M4 Task 16 Step 3, against the main checkout's `.env`, on the
downloaded artifact):** `E2B_API_KEY_HOSTED: 0`, `E2B_API_KEY_SANDBAWKS: 0`,
`GATEWAY_ACCESS_TOKEN=: 0`, `"token": 0`. Both `.env` values were checked to
be unquoted (without printing them). The log also has no `signature=`, no
`v1_`, no `Bearer` followed by a UUID and no `access_token` or `accessToken`
(case-insensitive). The job's own check (the "Check the run log for the key"
step) passed before upload.

**Failures:** none, so nothing to classify and no crate bug to file. The other
known environment limits did not occur: `clm_0146_filesystem_pause_cold_boots`
passed (no refusing 400 or 409) and
`clm_0150_mcp_gateway_starts_and_exposes_url_and_token` passed (the
`mcp-gateway` template was available).

### The Actions run (CLM-0123)

- Run: <https://github.com/PeterGrace/e2b_client_rs/actions/runs/40808242984>,
  event `workflow_dispatch`, branch `main`, head
  `d9c6401907cf561945b0a7810814fbea4779045d`, conclusion `success`.
- Duration: 2 min 55 s from creation (02:45:30Z) to completion (02:48:25Z);
  the job ran 2 min 50 s, of which the "Live tests" step (build plus tests)
  took 2 min 38 s, well inside the 45-minute limit. Test time: 70.82 s
  (`live_commands`, `clm_0175` alone takes over 60 s by design), 11.56 s
  (`live_files`), 26.39 s (`live_lifecycle`) and 0.25 s (`live_sweep`),
  109 s in all, against M4's 110 s.
- The key reached only the three steps that set it; the job used the
  repository secret `E2B_API_KEY_HOSTED` and no `.env`.

This run is the evidence for CLM-0123 ("A CI job runs the live suite against
hosted E2B using repository secrets on a manual or nightly trigger").

### M5 claims backed by new tests

- **CLM-0182** (per-call request timeouts on REST calls):
  `clm_0182_a_one_millisecond_request_timeout_times_out_get_info` passed:
  `get_info` with `RequestOptions::new().request_timeout(Some(1 ms))` fails
  with an error whose `is_timeout()` is true, and the same call with
  `RequestOptions::new()` (the configured timeout) succeeds. The `None`, zero
  and unset semantics and the other REST calls rest on Task 1's mock tests.
- **CLM-0183** (per-call request timeouts on pid-only process and PTY calls):
  `clm_0183_a_one_millisecond_request_timeout_times_out_list` passed:
  `Commands::list` with a 1 ms request timeout times out (`is_timeout()`),
  and with `RequestOptions::new()` succeeds. The other pid-only calls rest on
  Task 2's mock tests.

The `live_lifecycle` and `live_commands` tests reach the calls Tasks 1 and 2
gave a `RequestOptions` argument through the new signatures (64
`RequestOptions::new()` arguments in the two files at `d9c6401`, 48 in
`live_lifecycle` and 16 in `live_commands`, counted with
`grep -o 'RequestOptions::new()'`; `live_files` calls none of them), and
every one passed.
