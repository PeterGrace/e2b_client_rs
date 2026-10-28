Results of the Milestone 3 live runs (plan Tasks 12 and 13), one section per target.

## Hosted E2B (run 20261028T012033Z-hosted-2696474, 2026-10-28T01:20:33Z, commit ebccc15)

Run with `just test-live hosted` on `ebccc15` (main with every M3 feature
task, M3.01 to M3.11, PRs #74 to #85), template `base` (and `mcp-gateway`
for CLM-0150, chosen by the crate). The log starts `commit:
ebccc152316ba2c44fefd4548bb9afc44685a266` and `dirty: no`. Log:
[`runs/20261028T012033Z-hosted-2696474.log`](runs/20261028T012033Z-hosted-2696474.log).
M1's and M2's 57 tests were rerun on purpose, as a regression check. Rows are
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
| clm_0148_volume_create_get_list_destroy | CLM-0148 | fail (environment limit) | `create: Api { status: 403, message: "use of volumes is not enabled" }` |
| clm_0148_missing_volume_is_not_found_and_not_destroyed | CLM-0148 | pass | |
| clm_0149_volume_mount_persists_across_sandboxes | CLM-0149 | fail (environment limit) | `create volume: Api { status: 403, message: "use of volumes is not enabled" }` |
| clm_0150_mcp_gateway_starts_and_exposes_url_and_token | CLM-0150 | pass | |
| clm_0151_list_filters_by_template_and_start_time_and_orders | CLM-0151 | pass | |
| clm_0153_get_info_reports_network | CLM-0153 | pass | |
| clm_0153_get_info_reports_lifecycle | CLM-0153 | pass | |
| clm_0153_get_info_reports_volume_mounts | CLM-0153 | fail (environment limit) | `create volume: Api { status: 403, message: "use of volumes is not enabled" }` |

**Totals:** 75 `clm_*` tests: 72 passed, 3 failed (commands 17/17, files 19/19,
lifecycle 36/39). That is M1's and M2's 57 (all passed) plus M3's 18 (15
passed): `clm_0028`, `0029`, `0030` (2), `0143`, `0144`, `0145`, `0146`
(2), `0147`, `0148` (2), `0149`, `0150`, `0151` and `0153` (3), as the
plan expects. The three sweep tests are not claim tests and are not counted.
This was one run, with no reruns: no failure was transient.

**Sweeps:**

- `sweep: killed 0 leftover sandbox(es) of run 20261028T012033Z-hosted-2696474`
- `sweep: deleted 0 leftover snapshot(s) of run 20261028T012033Z-hosted-2696474`
- `sweep: destroyed 0 leftover volume(s) of run 20261028T012033Z-hosted-2696474`

Every drop guard killed its sandbox, and the log has no `could not kill` warning.

**Secret check (M1 Task 6 Step 3, on the log before it was copied here):**
`E2B_API_KEY_HOSTED: 0`, `E2B_API_KEY_SANDBAWKS: 0`, `GATEWAY_ACCESS_TOKEN=: 0`,
`"token": 0`. Both `.env` values were checked to be unquoted (without printing
them), so those zeros are real. The log also has no `signature=`, no `v1_`
and no `Bearer` followed by a UUID, so neither a signed URL nor the MCP token
reached it.

### Failures

All three are the same environment limit: **volumes are not enabled for the
team on hosted E2B.** Each test fails at its first call, `POST /volumes`,
which answers 403 with the message `use of volumes is not enabled` (the crate's
`Error::Api` carries the response body's `message`; the same answer Task 10
recorded in `docs/protocol-notes.md`, "Volumes"). No volume is created, so
there is nothing for the crate's mount or read-back code to act on.

| Test | Classification | Response |
|---|---|---|
| clm_0148_volume_create_get_list_destroy | environment limit (403 on volumes) | `POST /volumes` 403, `use of volumes is not enabled` (`tests/live_lifecycle.rs:1120`) |
| clm_0149_volume_mount_persists_across_sandboxes | environment limit (403 on volumes) | `POST /volumes` 403, `use of volumes is not enabled` (`tests/live_lifecycle.rs:1179`) |
| clm_0153_get_info_reports_volume_mounts | environment limit (403 on volumes) | `POST /volumes` 403, `use of volumes is not enabled` (`tests/live_lifecycle.rs:1218`) |

CLM-0148 and CLM-0149 stay `not_enough_info` on this run. CLM-0153 has three
live tests: its network and lifecycle parts passed, and only its volume-mounts
part is limited. No failure is a crate bug or a test bug, so this PR changes no
test.

### Notes on passes

- **CLM-0148 (missing volume).** `clm_0148_missing_volume_is_not_found_and_not_destroyed`
  passed: for a UUID-shaped missing ID, `get_volume` is `Error::VolumeNotFound`
  and `destroy_volume` is `false` (both 404), even for a team without volumes.
- **CLM-0146 (filesystem pause).** `clm_0146_filesystem_pause_cold_boots`
  passed, so neither the team (400) nor the node (409) refused a
  filesystem-only pause; that environment limit did not occur.
- **CLM-0150.** The `mcp-gateway` template was available to the team, and the
  gateway started; that environment limit did not occur either.
- **The volume sweep** reported 0 with no volume ever created; it killed the
  run's sandboxes first (0 left by then), as Task 10 made it do.
