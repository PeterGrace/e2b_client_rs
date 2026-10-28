Results of the Milestone 4 live runs (plan Tasks 16 and 17), one section per target.

## Hosted E2B (run 20261028T153214Z-hosted-761165, 2026-10-28T15:32:14Z, commit bf65ca8)

Run with `just test-live hosted` on `bf65ca8` (main with every M4 code task,
M4.01 to M4.15, PRs #101 to #115), template `base` (and `mcp-gateway` for
CLM-0150, chosen by the crate). The log starts `commit:
bf65ca89f2e093d73f6ca84b57e6902fe2100d2b` and `dirty: no`. Log:
[`runs/20261028T153214Z-hosted-761165.log`](runs/20261028T153214Z-hosted-761165.log). The 75 tests from M1 to M3 were rerun on
purpose, as a regression check of the M4 API changes. Rows are sorted by claim
number, and a claim's tests keep their source order.

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
| clm_0171_moded_pause_of_a_paused_sandbox_is_false | CLM-0171 | pass | |
| clm_0175_pty_outlives_sixty_seconds | CLM-0175 | pass | |
| clm_0177_symlinks_report_symlink_type | CLM-0177 | pass | |
| clm_0178_pty_connect_to_exited_pid | CLM-0178 | pass | |

**Totals:** 79 `clm_*` tests (Task 16 Step 1's count: 40 in `live_lifecycle`,
19 in `live_commands`, 20 in `live_files`, as the plan expects): 76 passed,
3 failed (commands 19/19, files 20/20, lifecycle 37/40). That is M1 to M3's 75
(72 passed, the same three volume tests failing as in M3's run
`20261028T012033Z-hosted-2696474`) plus M4's 4 (all passed): `clm_0171`,
`clm_0175`, `clm_0177` and `clm_0178`. The three sweep tests are not claim
tests and are not counted. This was one run, with no reruns: no failure was
transient, and no test was changed.

**Sweeps:**

- `sweep: killed 0 leftover sandbox(es) of run 20261028T153214Z-hosted-761165`
- `sweep: deleted 0 leftover snapshot(s) of run 20261028T153214Z-hosted-761165`
- `sweep: destroyed 0 leftover volume(s) of run 20261028T153214Z-hosted-761165`

Every drop guard killed its sandbox, and the log has no `could not kill` warning.

**Secret check (Task 16 Step 3, on the log before it was copied here):**
`E2B_API_KEY_HOSTED: 0`, `E2B_API_KEY_SANDBAWKS: 0`, `GATEWAY_ACCESS_TOKEN=: 0`,
`"token": 0`. Both `.env` values were checked to be unquoted (without printing
them), so those zeros are real. The log also has no `signature=`, no `v1_`,
no `Bearer` followed by a UUID and no `access_token` or `accessToken`
(case-insensitive), so no signed URL, envd access token, volume token or MCP
token reached it.

### Failures

All three are the same environment limit as in M3: **volumes are not enabled
for the team on hosted E2B.** Each test fails at its first call, `POST
/volumes`, which answers 403 with the response body's message `use of volumes
is not enabled` (the crate's `Error::Api` carries it; `docs/protocol-notes.md`,
"Volumes"). No volume is created, so there is nothing for the crate's mount or
read-back code to act on.

| Test | Classification | Response |
|---|---|---|
| clm_0148_volume_create_get_list_destroy | environment limit (403 on volumes) | `POST /volumes` 403, `use of volumes is not enabled` (`tests/live_lifecycle.rs:1167`) |
| clm_0149_volume_mount_persists_across_sandboxes | environment limit (403 on volumes) | `POST /volumes` 403, `use of volumes is not enabled` (`tests/live_lifecycle.rs:1226`) |
| clm_0153_get_info_reports_volume_mounts | environment limit (403 on volumes) | `POST /volumes` 403, `use of volumes is not enabled` (`tests/live_lifecycle.rs:1269`) |

No failure is a crate bug or a test bug, so there is nothing for the
coordinator to file and this PR changes no test.

The other two known environment limits did not occur:

- **Filesystem-only pause.** `clm_0146_filesystem_pause_cold_boots` passed, so
  neither the team (400) nor the node (409) refused a filesystem-only pause.
  Since issue #81 a refusing 409 would be an `Error::Api`, not `false`.
- **`mcp-gateway` template.** `clm_0150_mcp_gateway_starts_and_exposes_url_and_token`
  passed: the template was available and the gateway started.

### M4 claims backed by new or changed tests

- **CLM-0168** (one connect request, a v2 404 is `SandboxNotFound`):
  `clm_0015_connect_unknown_sandbox_is_not_found` passed through the new
  `connect_sandbox(id, &ConnectOptions)` with no v1 fallback;
  `clm_0013`, `clm_0014`, `clm_0146` (both) and `clm_0150` connect through
  the same single request and passed. Counting requests is the mock test's job
  (`tests/rest_api.rs`); the live test shows hosted E2B's v2 404 alone gives
  `Error::SandboxNotFound`.
- **CLM-0171** (a moded pause reads only "already paused" as `false`):
  `clm_0171_moded_pause_of_a_paused_sandbox_is_false` passed. The log does
  not record the 409 itself; that its message contains `already paused` is
  inferred from the crate's matching (`ALREADY_PAUSED` in `src/client.rs`:
  with a mode, any other 409 would have been an `Error::Api` and failed the
  test's `expect`). The full message, `Error pausing sandbox - sandbox '<id>'
  is already paused`, is quoted in that constant's rustdoc (seen live on
  2026-10-28, issue #81).
  `clm_0146_filesystem_pause_cold_boots` passed too (no refusing 409).
- **CLM-0173** (filesystem options): all 20 `live_files` tests passed through
  `FileOptions`, `ListOptions`, `WriteOptions` and `WatchOptions`, and
  `clm_0061_user_selects_file_owner` through `WriteOptions::user`. No live
  test sets a per-call request timeout; that half rests on the mock tests.
- **CLM-0174** (connect options): `clm_0040_connect_attaches_to_running_command`
  passed through `CommandConnectOptions`, and `clm_0178` through
  `PtyConnectOptions`. As for CLM-0173, the request-timeout override and the
  60 s default deadline are not exercised live.
- **CLM-0175** (no PTY deadline by default):
  `clm_0175_pty_outlives_sixty_seconds` passed; the shell echoed `alive-42`
  65 s after the PTY opened, in a sandbox with a 150 s timeout.
- **CLM-0177** (symlinks): `clm_0177_symlinks_report_symlink_type` passed:
  `list` reports both links (to a file and to a directory) as
  `FileType::Symlink` and their targets as `File` and `Directory`, and
  `get_info` reports the file link as `Symlink`.
- **CLM-0178** (`Pty::connect` to an exited pid):
  `clm_0178_pty_connect_to_exited_pid` passed: straight after the exit event
  and 5 s later, envd still attaches and the first event replays the exit
  (code 3); a pid that never existed is refused with `ConnectCode::NotFound`.
  That is what `Pty::connect`'s rustdoc states.

In M4.01 to M4.15 (PRs #101 to #115) the other M1 to M3 tests changed only
their call forms (options structs in place of `None` and `Option` arguments,
`pause` and `connect_sandbox` in place of the `_with` twins, and a catch-all
arm on the `#[non_exhaustive]` `CommandEvent` in `clm_0034`) and, in
`clm_0146_filesystem_pause_cold_boots`, a comment and two messages. That
comes from `git diff 3808f92 bf65ca8 -- tests/live_*.rs` (from the M4 plan's
merge, before any M4 code task, to this run's commit), not from this log,
which shows only that every one of them passed. CLM-0169, CLM-0170,
CLM-0172, CLM-0176, CLM-0179, CLM-0180 and CLM-0119 to CLM-0121 have no live
test; CLM-0181 is Task 17's paced sandbawks run.

### Evidence for the ledger (coordinator)

Every row is from run `20261028T153214Z-hosted-761165` (log `docs/verification/runs/20261028T153214Z-hosted-761165.log`, commit
`bf65ca89f2e093d73f6ca84b57e6902fe2100d2b`, `dirty: no`), target hosted E2B.

M4 claims with a live test:

| Claim | Live test(s) and result |
|---|---|
| CLM-0168 | `clm_0015_connect_unknown_sandbox_is_not_found` pass |
| CLM-0171 | `clm_0171_moded_pause_of_a_paused_sandbox_is_false` pass; `clm_0146_filesystem_pause_cold_boots` pass |
| CLM-0173 | **partly live-backed:** every `live_files` test pass (20/20), user option via `clm_0061_user_selects_file_owner` pass; the per-call request timeout is not live-tested (mock tests only), so record this run as partial evidence, not full verification |
| CLM-0174 | **partly live-backed:** `clm_0040_connect_attaches_to_running_command` pass; `clm_0178_pty_connect_to_exited_pid` pass; the per-call request timeout and the 60 s default deadline are not live-tested (mock tests only), so record this run as partial evidence, not full verification |
| CLM-0175 | `clm_0175_pty_outlives_sixty_seconds` pass |
| CLM-0177 | `clm_0177_symlinks_report_symlink_type` pass |
| CLM-0178 | `clm_0178_pty_connect_to_exited_pid` pass |

M1 to M3 claims (regression check):

| Claim | Live test(s) and result |
|---|---|
| CLM-0007 | `clm_0007_create_returns_running_sandbox` pass |
| CLM-0008 | `clm_0008_create_metadata_is_visible_in_info` pass |
| CLM-0009 | `clm_0009_create_env_vars_reach_commands` pass |
| CLM-0010 | `clm_0010_create_timeout_sets_end_at` pass |
| CLM-0011 | `clm_0011_auto_pause_pauses_at_timeout` pass |
| CLM-0012 | `clm_0012_internet_access_false_blocks_outbound` pass |
| CLM-0013 | `clm_0013_connect_running_sandbox_runs_commands` pass |
| CLM-0014 | `clm_0014_connect_resumes_paused_sandbox_with_files` pass |
| CLM-0015 | `clm_0015_connect_unknown_sandbox_is_not_found` pass |
| CLM-0016 | `clm_0016_get_info_returns_detail` pass |
| CLM-0017 | `clm_0017_get_info_unknown_sandbox_is_not_found` pass |
| CLM-0018 | `clm_0018_list_filters_by_metadata` pass |
| CLM-0019 | `clm_0019_list_filters_by_state` pass |
| CLM-0020 | `clm_0020_list_pages_without_duplicates_or_omissions` pass |
| CLM-0021 | `clm_0021_kill_running_sandbox` pass |
| CLM-0022 | `clm_0022_kill_unknown_sandbox_is_false` pass |
| CLM-0023 | `clm_0023_pause_running_sandbox` pass |
| CLM-0024 | `clm_0024_pause_paused_sandbox_is_false` pass |
| CLM-0025 | `clm_0025_set_timeout_moves_end_at` pass |
| CLM-0026 | `clm_0026_refresh_extends_end_at` pass |
| CLM-0027 | `clm_0027_is_running_tracks_kill` pass |
| CLM-0028 | `clm_0028_get_host_reaches_a_port_in_the_sandbox` pass |
| CLM-0029 | `clm_0029_get_metrics_reports_cpu_memory_and_disk` pass |
| CLM-0030 | `clm_0030_snapshot_starts_sandboxes_with_its_files` pass; `clm_0030_fork_starts_running_copies` pass |
| CLM-0033 | `clm_0033_run_returns_output_and_exit_code` pass |
| CLM-0034 | `clm_0034_start_streams_chunks_then_exit` pass |
| CLM-0035 | `clm_0035_run_delivers_large_output_in_order` pass |
| CLM-0036 | `clm_0036_options_env_is_set` pass |
| CLM-0037 | `clm_0037_options_cwd_is_working_directory` pass |
| CLM-0038 | `clm_0038_options_user_runs_as_user` pass |
| CLM-0039 | `clm_0039_options_timeout_ends_long_command` pass |
| CLM-0040 | `clm_0040_connect_attaches_to_running_command` pass |
| CLM-0041 | `clm_0041_list_includes_background_command` pass |
| CLM-0042 | `clm_0042_send_stdin_reaches_command` pass |
| CLM-0043 | `clm_0043_close_stdin_sends_eof` pass |
| CLM-0044 | `clm_0044_sigterm_terminates_command` pass |
| CLM-0045 | `clm_0045_kill_reports_whether_pid_existed` pass |
| CLM-0046 | `clm_0046_stream_input_delivers_chunks_in_order` pass |
| CLM-0047 | `clm_0047_pty_create_streams_terminal_output` pass |
| CLM-0048 | `clm_0048_pty_resize_changes_terminal_size` pass |
| CLM-0049 | `clm_0049_pty_input_reaches_the_shell` pass |
| CLM-0050 | `clm_0050_write_read_round_trips_binary` pass |
| CLM-0051 | `clm_0051_read_to_string_returns_text` pass |
| CLM-0052 | `clm_0052_write_returns_write_info` pass |
| CLM-0053 | `clm_0053_read_missing_file_is_not_found` pass |
| CLM-0054 | `clm_0054_list_depth_one_lists_direct_entries` pass |
| CLM-0055 | `clm_0055_list_depth_two_recurses` pass |
| CLM-0056 | `clm_0056_get_info_reports_metadata` pass |
| CLM-0057 | `clm_0057_exists_true_and_false` pass |
| CLM-0058 | `clm_0058_make_dir_reports_creation` pass |
| CLM-0059 | `clm_0059_rename_moves_entry` pass |
| CLM-0060 | `clm_0060_remove_deletes_file_and_directory` pass |
| CLM-0061 | `clm_0061_user_selects_file_owner` pass |
| CLM-0062 | `clm_0062_write_files_writes_every_file` pass |
| CLM-0063 | `clm_0063_watch_dir_reports_a_created_file` pass |
| CLM-0064 | `clm_0064_upload_url_accepts_a_multipart_post` pass |
| CLM-0065 | `clm_0065_download_url_fetches_without_credentials` pass |
| CLM-0066 | `clm_0066_write_stream_uploads_more_than_it_buffers` pass |
| CLM-0128 | `clm_0128_read_stream_downloads_in_chunks` pass |
| CLM-0129 | `clm_0129_write_metadata_round_trips` pass |
| CLM-0143 | `clm_0143_create_network_allow_and_deny_lists_are_enforced` pass |
| CLM-0144 | `clm_0144_update_network_replaces_egress_without_restart` pass |
| CLM-0145 | `clm_0145_lifecycle_pause_with_full_snapshot_pauses_at_timeout` pass |
| CLM-0146 | `clm_0146_resume_restores_memory_or_reboots` pass; `clm_0146_filesystem_pause_cold_boots` pass |
| CLM-0147 | `clm_0147_list_and_delete_snapshots` pass |
| CLM-0148 | `clm_0148_volume_create_get_list_destroy` fail; `clm_0148_missing_volume_is_not_found_and_not_destroyed` pass |
| CLM-0149 | `clm_0149_volume_mount_persists_across_sandboxes` fail |
| CLM-0150 | `clm_0150_mcp_gateway_starts_and_exposes_url_and_token` pass |
| CLM-0151 | `clm_0151_list_filters_by_template_and_start_time_and_orders` pass |
| CLM-0153 | `clm_0153_get_info_reports_network` pass; `clm_0153_get_info_reports_lifecycle` pass; `clm_0153_get_info_reports_volume_mounts` fail |

The three failures are the volume environment limit above; CLM-0148's other
test, `clm_0148_missing_volume_is_not_found_and_not_destroyed`, passed.

## sandbawks (run 20261028T154205Z-sandbawks-paced-913873, 2026-10-28T15:42:05Z, commit 3cedbc3)

Run with `just test-live-paced sandbawks` (M4.15) at the default 120 s gap,
on `3cedbc3` (main with every M4 code task, M4.01 to M4.15, and M4.16's
hosted results, PR #116), template `compound-sandbox`
(`E2B_TEMPLATE_SANDBAWKS`), against `sb-ghlw.gfd.dev` (sandbawks 0.1.28). The
log starts `commit: 3cedbc3e91533db5e6a183fe636d97f9be0c5a0b`, `dirty: no` and
`paced: one test at a time, 120s apart; readiness failures rerun once`. Log:
[`runs/20261028T154205Z-sandbawks-paced-913873.log`](runs/20261028T154205Z-sandbawks-paced-913873.log).
It ran from 15:42:05Z to 18:27:16Z (the log's last write). This one paced run replaces M2's and
M3's hand-paced rerun rounds: every test ran once, alone, after a 120 s gap,
and the one readiness failure was rerun once by the recipe. Rows are sorted
by claim number, and a claim's tests keep their source order.

**Classes:** **pass**; **sandbawks** (a sandbawks defect or missing route:
the filed gfd/sandbawks issue, or **new** with an N-number below);
**capacity** (`FAIL-READINESS (rerun)`: says nothing about the feature). No
failure is a crate bug or a test bug: every one passed on hosted (above) or,
for the volume tests, failed there on the 403 environment limit.

| Test | Claim | Paced result | Hosted | Class | Failure (first line) |
|---|---|---|---|---|---|
| clm_0007_create_returns_running_sandbox | CLM-0007 | PASS | pass | pass | |
| clm_0008_create_metadata_is_visible_in_info | CLM-0008 | PASS | pass | pass | |
| clm_0009_create_env_vars_reach_commands | CLM-0009 | FAIL | pass | sandbawks ([sandbawks#681](https://git.gfpd.us/gfd/sandbawks/issues/681)) | `left: "\n"` `right: "hi from create\n"` |
| clm_0010_create_timeout_sets_end_at | CLM-0010 | FAIL | pass | sandbawks, **new N1** | `end_at - started_at = 211s, want about 90` |
| clm_0011_auto_pause_pauses_at_timeout | CLM-0011 | FAIL | pass | sandbawks ([sandbawks#679](https://git.gfpd.us/gfd/sandbawks/issues/679), [sandbawks#665](https://git.gfpd.us/gfd/sandbawks/issues/665)) | `timed out after 120s waiting for auto-pause at timeout` |
| clm_0012_internet_access_false_blocks_outbound | CLM-0012 | FAIL | pass | sandbawks ([sandbawks#667](https://git.gfpd.us/gfd/sandbawks/issues/667)) | `sandbox without internet access got an HTTP reply from 1.1.1.1:80` |
| clm_0013_connect_running_sandbox_runs_commands | CLM-0013 | PASS | pass | pass | |
| clm_0014_connect_resumes_paused_sandbox_with_files | CLM-0014 | FAIL | pass | sandbawks ([sandbawks#679](https://git.gfpd.us/gfd/sandbawks/issues/679)) | `left: Running` `right: Paused` |
| clm_0015_connect_unknown_sandbox_is_not_found | CLM-0015 | PASS | pass | pass | |
| clm_0016_get_info_returns_detail | CLM-0016 | PASS | pass | pass | |
| clm_0017_get_info_unknown_sandbox_is_not_found | CLM-0017 | PASS | pass | pass | |
| clm_0018_list_filters_by_metadata | CLM-0018 | FAIL | pass | sandbawks ([sandbawks#676](https://git.gfpd.us/gfd/sandbawks/issues/676)) | `left: ["sbx-compound-pool-k9ltg", "sbx-compound-pool-khhvd", "sbx-compound-pool-lsxmr", "sbx-compound-pool-pmkth"]` `right: ["sbx-compound-pool-k9ltg"]` |
| clm_0019_list_filters_by_state | CLM-0019 | FAIL | pass | sandbawks ([sandbawks#676](https://git.gfpd.us/gfd/sandbawks/issues/676)) | `left: [<4 sandboxes>]` `right: ["sbx-compound-pool-pmkth"]` |
| clm_0020_list_pages_without_duplicates_or_omissions | CLM-0020 | FAIL | pass | sandbawks ([sandbawks#676](https://git.gfpd.us/gfd/sandbawks/issues/676)) | page union holds 2 other sandboxes besides the 3 of the test |
| clm_0021_kill_running_sandbox | CLM-0021 | PASS | pass | pass | |
| clm_0022_kill_unknown_sandbox_is_false | CLM-0022 | PASS | pass | pass | |
| clm_0023_pause_running_sandbox | CLM-0023 | FAIL | pass | sandbawks ([sandbawks#679](https://git.gfpd.us/gfd/sandbawks/issues/679)) | `left: Running` `right: Paused` |
| clm_0024_pause_paused_sandbox_is_false | CLM-0024 | FAIL | pass | sandbawks ([sandbawks#679](https://git.gfpd.us/gfd/sandbawks/issues/679)) | `assertion failed: !second` |
| clm_0025_set_timeout_moves_end_at | CLM-0025 | FAIL | pass | sandbawks, **new N1** | `end_at - started_at = 843s, want 238..=242 (240 s from the set_timeout call)` |
| clm_0026_refresh_extends_end_at | CLM-0026 | PASS | pass | pass | |
| clm_0027_is_running_tracks_kill | CLM-0027 | FAIL | pass | sandbawks ([sandbawks#679](https://git.gfpd.us/gfd/sandbawks/issues/679)) | `want Ok(false) after kill, got Err(Envd { status: 404, message: "sandboxes.sandbawks.dev \"sbx-compound-pool-hsgkl\" not found" })` |
| clm_0028_get_host_reaches_a_port_in_the_sandbox | CLM-0028 | FAIL | pass | sandbawks ([sandbawks#678](https://git.gfpd.us/gfd/sandbawks/issues/678)), before `get_host` is reached | `write: Envd { status: 404, message: "No such file or directory (os error 2)" }` |
| clm_0029_get_metrics_reports_cpu_memory_and_disk | CLM-0029 | FAIL | pass | missing route ([sandbawks#668](https://git.gfpd.us/gfd/sandbawks/issues/668)) | `sandbox vanished: SandboxNotFound("sbx-compound-pool-4spn9")` |
| clm_0030_snapshot_starts_sandboxes_with_its_files | CLM-0030 | FAIL | pass | missing route ([sandbawks#669](https://git.gfpd.us/gfd/sandbawks/issues/669)) | `snapshot: SandboxNotFound("sbx-compound-pool-jldfk")` |
| clm_0030_fork_starts_running_copies | CLM-0030 | FAIL | pass | missing route ([sandbawks#670](https://git.gfpd.us/gfd/sandbawks/issues/670)) | `fork: SandboxNotFound("sbx-compound-pool-7zxmc")` |
| clm_0033_run_returns_output_and_exit_code | CLM-0033 | PASS | pass | pass | |
| clm_0034_start_streams_chunks_then_exit | CLM-0034 | PASS | pass | pass | |
| clm_0035_run_delivers_large_output_in_order | CLM-0035 | PASS | pass | pass | |
| clm_0036_options_env_is_set | CLM-0036 | PASS | pass | pass | |
| clm_0037_options_cwd_is_working_directory | CLM-0037 | PASS | pass | pass | |
| clm_0038_options_user_runs_as_user | CLM-0038 | FAIL | pass | sandbawks ([sandbawks#682](https://git.gfpd.us/gfd/sandbawks/issues/682)) | `left: "dev\n"` `right: "root\n"` |
| clm_0039_options_timeout_ends_long_command | CLM-0039 | PASS | pass | pass | |
| clm_0040_connect_attaches_to_running_command | CLM-0040 | FAIL | pass | sandbawks ([sandbawks#677](https://git.gfpd.us/gfd/sandbawks/issues/677)) | `connect: Protocol("Connect stream did not open with a start event")` |
| clm_0041_list_includes_background_command | CLM-0041 | PASS | pass | pass | |
| clm_0042_send_stdin_reaches_command | CLM-0042 | PASS | pass | pass | |
| clm_0043_close_stdin_sends_eof | CLM-0043 | FAIL | pass | sandbawks ([sandbawks#675](https://git.gfpd.us/gfd/sandbawks/issues/675); behind it [sandbawks#677](https://git.gfpd.us/gfd/sandbawks/issues/677)) | `close_stdin: InvalidArgument("close_stdin needs envd 0.5.2 or newer; this sandbox runs envd 0.1.28")` |
| clm_0044_sigterm_terminates_command | CLM-0044 | FAIL | pass | sandbawks ([sandbawks#677](https://git.gfpd.us/gfd/sandbawks/issues/677)) | `pid 10 still listed after SIGTERM: [ProcessInfo { pid: 10, ... args: ["-l", "-c", "sleep 30"] ... }]` |
| clm_0045_kill_reports_whether_pid_existed | CLM-0045 | PASS | pass | pass | |
| clm_0046_stream_input_delivers_chunks_in_order | CLM-0046 | FAIL | pass | sandbawks ([sandbawks#664](https://git.gfpd.us/gfd/sandbawks/issues/664)) | `close input stream: Rpc { code: NotFound, message: "HTTP 404" }` |
| clm_0047_pty_create_streams_terminal_output | CLM-0047 | PASS | pass | pass | |
| clm_0048_pty_resize_changes_terminal_size | CLM-0048 | PASS | pass | pass | |
| clm_0049_pty_input_reaches_the_shell | CLM-0049 | FAIL | pass | sandbawks ([sandbawks#664](https://git.gfpd.us/gfd/sandbawks/issues/664)) | `close input stream: Rpc { code: NotFound, message: "HTTP 404" }` |
| clm_0050_write_read_round_trips_binary | CLM-0050 | PASS | pass | pass | |
| clm_0051_read_to_string_returns_text | CLM-0051 | PASS | pass | pass | |
| clm_0052_write_returns_write_info | CLM-0052 | PASS | pass | pass | |
| clm_0053_read_missing_file_is_not_found | CLM-0053 | PASS | pass | pass | |
| clm_0054_list_depth_one_lists_direct_entries | CLM-0054 | FAIL | pass | sandbawks ([sandbawks#678](https://git.gfpd.us/gfd/sandbawks/issues/678)) | `write a: Envd { status: 404, message: "No such file or directory (os error 2)" }` |
| clm_0055_list_depth_two_recurses | CLM-0055 | FAIL | pass | sandbawks ([sandbawks#678](https://git.gfpd.us/gfd/sandbawks/issues/678)) | `write a: Envd { status: 404, message: "No such file or directory (os error 2)" }` |
| clm_0056_get_info_reports_metadata | CLM-0056 | FAIL | pass | sandbawks ([sandbawks#687](https://git.gfpd.us/gfd/sandbawks/issues/687)) | ``want `-rw...` permissions: EntryInfo { ... mode: 420, permissions: "rw-r--r--", owner: "", group: "", ... }`` |
| clm_0057_exists_true_and_false | CLM-0057 | PASS | pass | pass | |
| clm_0058_make_dir_reports_creation | CLM-0058 | FAIL | pass | sandbawks ([sandbawks#678](https://git.gfpd.us/gfd/sandbawks/issues/678)) | `make_dir of an existing directory returned true` |
| clm_0059_rename_moves_entry | CLM-0059 | PASS | pass | pass | |
| clm_0060_remove_deletes_file_and_directory | CLM-0060 | FAIL | pass | sandbawks ([sandbawks#678](https://git.gfpd.us/gfd/sandbawks/issues/678)) | `write file: Envd { status: 404, message: "No such file or directory (os error 2)" }` |
| clm_0061_user_selects_file_owner | CLM-0061 | FAIL | pass | sandbawks ([sandbawks#687](https://git.gfpd.us/gfd/sandbawks/issues/687), also [sandbawks#682](https://git.gfpd.us/gfd/sandbawks/issues/682) and [sandbawks#688](https://git.gfpd.us/gfd/sandbawks/issues/688)) | `left: ""` `right: "root"` |
| clm_0062_write_files_writes_every_file | CLM-0062 | FAIL | pass | sandbawks ([sandbawks#684](https://git.gfpd.us/gfd/sandbawks/issues/684), reached through [sandbawks#675](https://git.gfpd.us/gfd/sandbawks/issues/675)) | ``write_files: Envd { status: 400, message: "Failed to deserialize query string: missing field `path`" }`` |
| clm_0063_watch_dir_reports_a_created_file | CLM-0063 | FAIL | pass | sandbawks ([sandbawks#664](https://git.gfpd.us/gfd/sandbawks/issues/664)) | `watch_dir: Rpc { code: NotFound, message: "HTTP 404" }` |
| clm_0064_upload_url_accepts_a_multipart_post | CLM-0064 | PASS | pass | pass, but unsigned ([sandbawks#683](https://git.gfpd.us/gfd/sandbawks/issues/683)) | |
| clm_0065_download_url_fetches_without_credentials | CLM-0065 | FAIL | pass | sandbawks ([sandbawks#683](https://git.gfpd.us/gfd/sandbawks/issues/683)) | `download url: InvalidArgument("a URL expiration needs a secured sandbox (one with an envd access token)")` |
| clm_0066_write_stream_uploads_more_than_it_buffers | CLM-0066 | FAIL | pass | sandbawks ([sandbawks#685](https://git.gfpd.us/gfd/sandbawks/issues/685), multipart because of [sandbawks#675](https://git.gfpd.us/gfd/sandbawks/issues/675)) | `write_stream plain: Envd { status: 413, message: "<html>...<title>413 Request Entity Too Large</title>...<center>nginx</center>..." }` |
| clm_0128_read_stream_downloads_in_chunks | CLM-0128 | FAIL | pass | sandbawks ([sandbawks#686](https://git.gfpd.us/gfd/sandbawks/issues/686)) | `read_stream: Envd { status: 500, message: "an internal error occurred" }` |
| clm_0129_write_metadata_round_trips | CLM-0129 | FAIL | pass | sandbawks ([sandbawks#675](https://git.gfpd.us/gfd/sandbawks/issues/675)) | `file metadata needs envd 0.6.2 or newer; this template runs envd 0.1.28` |
| clm_0143_create_network_allow_and_deny_lists_are_enforced | CLM-0143 | FAIL | pass | missing feature ([sandbawks#667](https://git.gfpd.us/gfd/sandbawks/issues/667)) | `1.0.0.1 is denied by ALL_TRAFFIC but replied` |
| clm_0144_update_network_replaces_egress_without_restart | CLM-0144 | FAIL | pass | missing feature and route ([sandbawks#667](https://git.gfpd.us/gfd/sandbawks/issues/667)) | `egress was open before any update` |
| clm_0145_lifecycle_pause_with_full_snapshot_pauses_at_timeout | CLM-0145 | FAIL | pass | missing feature ([sandbawks#665](https://git.gfpd.us/gfd/sandbawks/issues/665), also [sandbawks#679](https://git.gfpd.us/gfd/sandbawks/issues/679)) | `timed out after 120s waiting for lifecycle pause at timeout, with its pause mode` |
| clm_0146_resume_restores_memory_or_reboots | CLM-0146 | FAIL | pass | sandbawks ([sandbawks#688](https://git.gfpd.us/gfd/sandbawks/issues/688)), before the pause is reached (behind it [sandbawks#679](https://git.gfpd.us/gfd/sandbawks/issues/679), [sandbawks#666](https://git.gfpd.us/gfd/sandbawks/issues/666)) | `touch: cannot touch '/home/user/on-disk': No such file or directory` (exit code 1) |
| clm_0146_filesystem_pause_cold_boots | CLM-0146 | FAIL | pass | sandbawks ([sandbawks#688](https://git.gfpd.us/gfd/sandbawks/issues/688)), before the pause is reached (behind it [sandbawks#679](https://git.gfpd.us/gfd/sandbawks/issues/679)) | `touch: cannot touch '/home/user/on-disk': No such file or directory` (exit code 1) |
| clm_0147_list_and_delete_snapshots | CLM-0147 | FAIL | pass | missing route ([sandbawks#669](https://git.gfpd.us/gfd/sandbawks/issues/669)) | `snapshot: SandboxNotFound("sbx-compound-pool-n2fpl")` |
| clm_0148_volume_create_get_list_destroy | CLM-0148 | FAIL | fail (environment limit, 403) | missing route ([sandbawks#671](https://git.gfpd.us/gfd/sandbawks/issues/671)) | `create: Api { status: 404, message: "HTTP 404" }` |
| clm_0148_missing_volume_is_not_found_and_not_destroyed | CLM-0148 | PASS | pass | pass, but only because the whole route answers 404 ([sandbawks#671](https://git.gfpd.us/gfd/sandbawks/issues/671)) | |
| clm_0149_volume_mount_persists_across_sandboxes | CLM-0149 | FAIL | fail (environment limit, 403) | missing route ([sandbawks#671](https://git.gfpd.us/gfd/sandbawks/issues/671)) | `create volume: Api { status: 404, message: "HTTP 404" }` |
| clm_0150_mcp_gateway_starts_and_exposes_url_and_token | CLM-0150 | FAIL-READINESS (rerun) | pass | sandbawks ([sandbawks#680](https://git.gfpd.us/gfd/sandbawks/issues/680), probably: no `mcp-gateway` template), **not capacity**: see below | `create sandbox: McpGateway("ERROR: sandbox will not become ready without intervention: the sandbox failed (action_failure)\n")` |
| clm_0151_list_filters_by_template_and_start_time_and_orders | CLM-0151 | FAIL | pass | sandbawks ([sandbawks#689](https://git.gfpd.us/gfd/sandbawks/issues/689), with [sandbawks#676](https://git.gfpd.us/gfd/sandbawks/issues/676)) | `["sbx-compound-pool-jtf9q", "sbx-compound-pool-mtdz8", "sbx-compound-pool-v5gfg", "sbx-compound-pool-v6xmm"]` (`tests/live_lifecycle.rs:754`) |
| clm_0153_get_info_reports_network | CLM-0153 | FAIL | pass | missing feature ([sandbawks#667](https://git.gfpd.us/gfd/sandbawks/issues/667)) | `left: None` `right: Some(false)` (`allow_internet_access`) |
| clm_0153_get_info_reports_lifecycle | CLM-0153 | FAIL | pass | missing feature ([sandbawks#665](https://git.gfpd.us/gfd/sandbawks/issues/665)) | `lifecycle in the detail` (the detail has no `lifecycle`) |
| clm_0153_get_info_reports_volume_mounts | CLM-0153 | FAIL | fail (environment limit, 403) | missing route ([sandbawks#671](https://git.gfpd.us/gfd/sandbawks/issues/671)) | `create volume: Api { status: 404, message: "HTTP 404" }` |
| clm_0171_moded_pause_of_a_paused_sandbox_is_false | CLM-0171 | FAIL | pass | sandbawks ([sandbawks#679](https://git.gfpd.us/gfd/sandbawks/issues/679)): no 409 at all, see below | `assertion failed: !second` |
| clm_0175_pty_outlives_sixty_seconds | CLM-0175 | PASS | pass | pass | |
| clm_0177_symlinks_report_symlink_type | CLM-0177 | FAIL | pass | sandbawks ([sandbawks#678](https://git.gfpd.us/gfd/sandbawks/issues/678)), at the setup write, before any link is made | `write target: Envd { status: 404, message: "No such file or directory (os error 2)" }` (writing `/tmp/sym/target.txt`) |
| clm_0178_pty_connect_to_exited_pid | CLM-0178 | FAIL | pass | sandbawks ([sandbawks#677](https://git.gfpd.us/gfd/sandbawks/issues/677)) | `connect to an exited pid gave, at once: Refused(Protocol("Connect stream did not open with a start event")); after 5 s: Refused(Protocol("Connect stream did not open with a start event"))` |

**Totals.** 79 `clm_*` tests (40 in `live_lifecycle`, 19 in `live_commands`,
20 in `live_files`, as in Task 16 Step 1), and 80 `RESULT` lines (one test
was rerun). The recipe's summary line: `summary: 29 pass, 0 pass on rerun, 49
fail, 1 still not ready`. By binary: lifecycle 10/40, commands 12/19, files
7/20.

- **M1 to M3's 75 tests:** 28 pass, against 30 after M3's reruns. The two
  that passed in M3 and fail now are CLM-0010 and CLM-0025, both on the new
  defect N1 (below). Every other failure is the one M3 recorded, with the
  same first line, against the same gfd/sandbawks issue.
  `clm_0150` ended `FAIL-READINESS (rerun)` here and `fail` (#680) in M3:
  the message is the same in both.
- **M4's 4 tests:** 1 passes (`clm_0175`) and 3 fail on known defects:
  `clm_0171` on #679, `clm_0177` on #678, `clm_0178` on #677.

**The one readiness result is not capacity.** `clm_0150` failed twice with
`the sandbox failed (action_failure)`, inside the gateway's own message
`sandbox will not become ready without intervention`. The recipe's readiness
markers include `will not become ready`, so it was labelled `FAIL-READINESS`
and rerun, and labelled `FAIL-READINESS (rerun)` after the rerun failed the
same way. That is M3's #680 signature (the `mcp-gateway` template does not
exist on sandbawks, and create answers `running` regardless), not an
unschedulable sandbox: no test in this run saw `still starting up`,
`Service Temporarily Unavailable` or `Unschedulable`. `docs/verification/live-pacing.md`
now says to read a `FAIL-READINESS (rerun)` line before counting it as
capacity.

**Sweeps.**

- `sweep: killed 0 leftover sandbox(es) of run 20261028T154205Z-sandbawks-paced-913873`.
- The snapshot sweep failed with `list snapshots: Api { status: 404,
  message: "HTTP 404" }` and the volume sweep with `list volumes: Api {
  status: 404, message: "HTTP 404" }`: sandbawks serves neither `GET
  /snapshots` nor `GET /volumes`
  ([sandbawks#669](https://git.gfpd.us/gfd/sandbawks/issues/669),
  [sandbawks#671](https://git.gfpd.us/gfd/sandbawks/issues/671)). That is not
  a leak: every snapshot and volume create in the run got a 404 too. These two
  failures made the recipe exit 1, as `live-pacing.md` and the PR #115 review
  expect.

The log has no `could not kill` warning.

**Secret check (Task 16 Step 3, on the log before it was copied here):**
`E2B_API_KEY_HOSTED: 0`, `E2B_API_KEY_SANDBAWKS: 0`, `GATEWAY_ACCESS_TOKEN=: 0`,
`"token": 0`. Both `.env` values were checked to be unquoted and non-empty
(without printing them), so those zeros are real. The log also has no
`signature=`, `v1_`, `X-Access-Token`, `Bearer`, `https://`, `access_token`
or `accessToken` (case-insensitive). sandbawks returns no `envdAccessToken`
([sandbawks#683](https://git.gfpd.us/gfd/sandbawks/issues/683)) and the MCP
gateway never started, so there was no token to leak.

### The moded already-paused pause (CLM-0171, issue #81)

sandbawks sends **no 409** for a pause of a paused sandbox, with or without a
mode, so there is no message to match. A direct probe after the run
(2026-10-28T18:27:47Z, `https://api.sb-ghlw.gfd.dev`, `X-API-KEY`, one
`compound-sandbox` sandbox created with a 120 s timeout and killed at the
end, 204) got:

```text
POST /sandboxes/sbx-compound-pool-26v87/pause  {}                -> 200 {"sandboxID":"sbx-compound-pool-26v87","state":"paused"}
POST /sandboxes/sbx-compound-pool-26v87/pause  {"memory":false}  -> 200 {"sandboxID":"sbx-compound-pool-26v87","state":"paused"}
POST /sandboxes/sbx-compound-pool-26v87/pause  {"memory":true}   -> 200 {"sandboxID":"sbx-compound-pool-26v87","state":"paused"}
GET  /sandboxes/sbx-compound-pool-26v87                          -> 200, "state": "running"
```

Every pause answers 200 (hosted E2B: 204, then 409), and the sandbox stays
`running`: the no-op pause of
[sandbawks#679](https://git.gfpd.us/gfd/sandbawks/issues/679), which also
fails `clm_0023` and `clm_0024`. The crate reads the 200 as `true`, so
`clm_0171`'s second, moded pause returned `true` and the test failed on
`assert!(!second)`, not on an `Error::Api` 409. The `ALREADY_PAUSED` wording
was never reached; whether sandbawks's 409 will contain `already paused` can
only be checked once #679 makes it send one. `docs/protocol-notes.md`'s
"Pause" row now says so.

### New sandbawks defects, for the coordinator to file

One, under epic [sandbawks#673](https://git.gfpd.us/gfd/sandbawks/issues/673).

**N1. `startedAt` is when the warm-pool sandbox started, not when create
handed it out** (CLM-0010, CLM-0025). Both tests passed on hosted in this
milestone and on sandbawks in M3.

- Client calls: `create_sandbox(&SandboxParams::new("compound-sandbox").timeout(90 s))`
  then `get_info(id)` (CLM-0010); `create_sandbox` with the default timeout,
  `set_sandbox_timeout(id, 240 s)`, then `get_info(id)` (CLM-0025). The tests
  check `endAt - startedAt` against the timeout, both on the server's clock.
- Seen in the run: `end_at - started_at = 211s, want about 90` and
  `end_at - started_at = 843s, want 238..=242`. The excess (121 s and about
  600 s) is how long the claimed `sbx-compound-pool-*` sandbox had sat in the
  warm pool: at a 120 s gap the pool is no longer drained at once, as it was
  in M3's `--test-threads=4` run, so a claimed sandbox is minutes old.
- Probe after the run (the same one as above), request:

  ```text
  POST https://api.sb-ghlw.gfd.dev/sandboxes
  X-API-KEY: <key>
  Content-Type: application/json

  {"templateID":"compound-sandbox","timeout":120,"metadata":{"probe":"i99-clm0171"}}
  ```

  Response: `201`, `date: Wed, 28 Oct 2026 18:27:47 GMT` (the local clock
  said 18:27:47Z too), body keys `alias`, `clientID`, `cpuCount`,
  `diskSizeMB`, `endAt`, `envdVersion`, `memoryMB`, `metadata`, `sandboxID`,
  `startedAt`, `state`, `templateID`, with `"sandboxID":
  "sbx-compound-pool-26v87"`, `"startedAt": "2026-10-28T18:25:15+00:00"` and
  `"endAt": "2026-10-28T18:29:47.210354169+00:00"`. `GET
  /sandboxes/sbx-compound-pool-26v87` straight after gave the same two
  times.
- So `endAt` is create time plus the timeout (correct), but `startedAt` is
  152 s before the create. On hosted E2B `startedAt` is the create time:
  CLM-0010 and CLM-0025 pass there, measuring `endAt - startedAt` against
  the timeout (the field is in the detail, `docs/protocol-notes.md` "REST
  endpoints", row "Detail fields"). A client that computes a sandbox's age
  or remaining lifetime from `startedAt` gets it wrong by however long the
  sandbox was pooled.
- Expected: `startedAt` is the time the create (claim) happened, so `endAt -
  startedAt` equals the requested timeout on a new sandbox.

### CLM-0118 (v0.1 on sandbawks, #27)

The v0.1 tests are CLM-0007 to CLM-0027 and CLM-0033 to CLM-0063: 52 tests
(`clm_0028` to `clm_0030` are M3's). In this run 26 pass and 26 fail, and
**none ends at capacity**. 24 of the 26 failures are filed sandbawks defects
(#664, #665, #667, #675 to #679, #681, #682, #684, #687, #688; the table above).
The other 2, CLM-0010 and CLM-0025, are N1, which is not filed yet.

So the run settles CLM-0118 **once N1 is filed**: then every v0.1 test ends
as pass or as a filed sandbawks defect, with no capacity result, and the
coordinator can decide CLM-0118 and close #27 (Task 18 Step 5). Until N1 is
filed, two v0.1 failures have no issue.

### M4 claims on sandbawks

- **CLM-0168:** `clm_0015_connect_unknown_sandbox_is_not_found` passed: a
  v2 404 alone is `SandboxNotFound` here too; `clm_0013` passed through the
  same single connect request.
- **CLM-0171:** `clm_0171` failed on #679 (no 409; above). It says nothing
  about the crate's 409 handling, which only the mock tests and hosted cover.
- **CLM-0173:** 7 of 20 `live_files` tests passed through the new options
  structs; the 13 failures are the known envd defects in the table.
- **CLM-0174:** `clm_0040` and `clm_0178` both failed at `connect` on #677
  (no start event), so `CommandConnectOptions` and `PtyConnectOptions` are not
  exercised here.
- **CLM-0175:** `clm_0175_pty_outlives_sixty_seconds` passed: the PTY
  outlived 60 s on sandbawks too.
- **CLM-0177:** `clm_0177` failed at its setup write (#678), before any
  symlink was made.
- **CLM-0178:** `clm_0178` failed: envd refuses `Connect` to an exited pid
  with a stream that has no start event (#677), at once and after 5 s.
- **CLM-0181** (paced runs): this run is its demonstration: one `RESULT`
  line per test (80 in all), one readiness rerun (`clm_0150`, labelled
  `FAIL-READINESS` then `FAIL-READINESS (rerun)`), the summary line, and the
  last line `run log: target/live-logs/20261028T154205Z-sandbawks-paced-913873.log`.

Per Task 18 Step 1, each sandbawks failure has stance `neutral` and names its
gfd/sandbawks issue (N1 once filed); none refutes its claim.
