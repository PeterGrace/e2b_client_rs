Results of the Milestone 1 live runs (plan Tasks 6 and 7), one section per target.

## Hosted E2B (run 20261026T225333Z-hosted-980838, 2026-10-26T22:53:33Z)

Run with `just test-live hosted` on `8cd5040` (main with all M1 tests and the
CLM-0071 and CLM-0109 fixes), template `base`. Log:
[`runs/20261026T225333Z-hosted-980838.log`](runs/20261026T225333Z-hosted-980838.log).

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

**Totals:** 46 `clm_*` tests: 46 passed, 0 failed (lifecycle 21/21, commands 13/13,
files 12/12). This was one run, with no reruns.

**Sweep:** `sweep: killed 0 leftover sandbox(es) of run 20261026T225333Z-hosted-980838`.
Every drop guard killed its sandbox, and the log has no `could not kill` warning.

**Secret check (plan Task 6 Step 3, on the log before it was copied here):**
`E2B_API_KEY_HOSTED: 0`, `E2B_API_KEY_SANDBAWKS: 0`.

### Failures

None.

### Notes on passes

- **CLM-0012.** Hosted E2B completes the TCP handshake even when internet access
  is off, so a bare connect proves nothing. The probe sends `GET /` to
  `1.1.1.1:80` and needs an `HTTP/` status line back. A control sandbox with
  internet access proves the probe works. It passed: the control sandbox got
  a reply, and the closed sandbox got none.
- **CLM-0044.** The test assumes envd drops a pid from `commands().list()` once
  `wait()` has returned. Hosted envd did. It also met the 10 s SIGTERM budget,
  which is measured before the `list()` call.
- **CLM-0061.** Only the `root` half proves the `user` argument takes effect,
  because `user` is the `base` template's default user. That half passed.
- **CLM-0011.** It uses its own 15 s sandbox timeout, which is why the
  lifecycle binary finished in 17.8 s even though it includes an auto-pause wait.
