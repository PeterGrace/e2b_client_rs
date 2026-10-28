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

## sandbawks (run 20261028T012919Z-sandbawks-2784597, 2026-10-28T01:29:19Z, commit 6871dbf)

Run with `just test-live sandbawks` on `6871dbf` (main with every M3 feature
task and the hosted results, PR #86), template `compound-sandbox`
(`E2B_TEMPLATE_SANDBAWKS` in `.env`), against sandbawks at `sb-ghlw.gfd.dev`.
The sandboxes report envd 0.1.28 (`envd_version` in CLM-0153's failure
message, and CLM-0043's and CLM-0129's errors). The log starts `commit:
6871dbfccd30862f458eb57a8772dae57bfa0560` and `dirty: no`. Hosted E2B is the
canonical server: every test that fails here, except the three volume tests,
passed on hosted in run `20261028T012033Z-hosted-2696474` (above), so no
failure is a crate bug. The volume tests fail on hosted on an environment
limit (403), and here on a missing route.

There was one full run and three rounds of single-test reruns. Every log is
checked in, and every one starts with the same `commit:` line and `dirty: no`:

| Run id | What it was | Log |
|---|---|---|
| `20261028T012919Z-sandbawks-2784597` | `just test-live sandbawks` (**the run of record**) | [log](runs/20261028T012919Z-sandbawks-2784597.log) |
| `20261028T013641Z-sandbawks-rerun-2828108` | R1: the 38 failures whose cause the run of record left open, 120 s apart, then the sweeps | [log](runs/20261028T013641Z-sandbawks-rerun-2828108.log) |
| `20261028T025532Z-sandbawks-rerun2-3256660` | R2: the 3 still not ready in R1 (0058, 0060, `clm_0146_resume_restores_memory_or_reboots`), 120 s apart, then the sweeps | [log](runs/20261028T025532Z-sandbawks-rerun2-3256660.log) |
| `20261028T030151Z-sandbawks-rerun3-3279120` | R3: CLM-0058, whose R2 create got nginx's 503, then the sweeps | [log](runs/20261028T030151Z-sandbawks-rerun3-3279120.log) |

Each rerun ran `cargo test --test <binary> -- --ignored --exact <test>
--test-threads=1 --show-output` with `E2B_TEST_TARGET=sandbawks` and the
round's own `E2B_TEST_RUN_ID`, then `live_sweep` for that id. The first rerun
round waited 60 s after the run of record, and each later round waited 120 s
after the one before.

**Which failures were rerun.** The 21 failures of the run of record that
failed on something other than readiness, at a call whose behaviour a filed
gfd/sandbawks issue describes, were not rerun: their result cannot change
while the issue is open. These are CLM-0011, 0018, 0019, 0020, 0023, 0024,
0027, 0043, 0046, 0049, 0055, 0062, 0066, 0129, 0144, 0145, the three volume
tests, and `clm_0153_get_info_reports_lifecycle` and `_network`. M2 saw the
same 14 M1 and M2 failures in at least two rounds. Every other failure was
rerun until it passed or failed on something other than readiness.

**Capacity.** As in M2, at `--test-threads=4` the run of record drained the
two-sandbox warm pool at once, and the cold creates that followed could not
be scheduled: `sandbox is still starting up (Pod is Pending (Unschedulable:
0/25 nodes are available: 1 node(s) had untolerated taint
{ToBeDeletedByClusterAutoscaler: 1793150557}, 10 Insufficient cpu, 18
Insufficient memory, 2 node(s) had untolerated taint
{node-role.kubernetes.io/control-plane: true}, 9 Insufficient
ephemeral-storage. ...)); retry shortly [EAGAIN: resource temporarily
unavailable]`. Create still answered `running`
([sandbawks#674](https://git.gfpd.us/gfd/sandbawks/issues/674)), so these
tests failed on envd's 503, Connect `unavailable`, exit code 75, or empty
output. With 120 s gaps most reruns got a ready sandbox; three did not on
their first try (two `still starting up`, one nginx `503 Service Temporarily
Unavailable` on create), and passed or failed on a defect in the next round.

**Classes:** **pass**; **sandbawks** (a sandbawks defect: a filed issue, or a
new N-number below); **missing route** (a route sandbawks 0.1.28 does not
serve, filed under epic
[sandbawks#673](https://git.gfpd.us/gfd/sandbawks/issues/673)); **capacity**
(the sandbox never became ready; says nothing about the feature). There are
no **crate** or **test** classes: every failure passes on hosted or, for the
volume tests, fails there on an environment limit, and no test was found
wrong. The last column quotes the last attempt.

| Test | Claim | Run of record | Reruns | Hosted | Class | Failure (first line, last attempt) |
|---|---|---|---|---|---|---|
| clm_0007_create_returns_running_sandbox | CLM-0007 | pass | | pass | pass | |
| clm_0008_create_metadata_is_visible_in_info | CLM-0008 | pass | | pass | pass | |
| clm_0009_create_env_vars_reach_commands | CLM-0009 | fail | R1 fail | pass | sandbawks ([sandbawks#681](https://git.gfpd.us/gfd/sandbawks/issues/681)); run of record: capacity | `left: "\n"` `right: "hi from create\n"` |
| clm_0010_create_timeout_sets_end_at | CLM-0010 | pass | | pass | pass | |
| clm_0011_auto_pause_pauses_at_timeout | CLM-0011 | fail | | pass | sandbawks ([sandbawks#679](https://git.gfpd.us/gfd/sandbawks/issues/679), [sandbawks#665](https://git.gfpd.us/gfd/sandbawks/issues/665)) | `timed out after 120s waiting for auto-pause at timeout` |
| clm_0012_internet_access_false_blocks_outbound | CLM-0012 | fail | R1 fail | pass | sandbawks ([sandbawks#667](https://git.gfpd.us/gfd/sandbawks/issues/667)); run of record: capacity | `sandbox without internet access got an HTTP reply from 1.1.1.1:80` |
| clm_0013_connect_running_sandbox_runs_commands | CLM-0013 | fail | R1 pass | pass | pass (run of record: capacity) | |
| clm_0014_connect_resumes_paused_sandbox_with_files | CLM-0014 | fail | R1 fail | pass | sandbawks ([sandbawks#679](https://git.gfpd.us/gfd/sandbawks/issues/679)); run of record: capacity | `left: Running` `right: Paused` |
| clm_0015_connect_unknown_sandbox_is_not_found | CLM-0015 | pass | | pass | pass | |
| clm_0016_get_info_returns_detail | CLM-0016 | pass | | pass | pass | |
| clm_0017_get_info_unknown_sandbox_is_not_found | CLM-0017 | pass | | pass | pass | |
| clm_0018_list_filters_by_metadata | CLM-0018 | fail | | pass | sandbawks ([sandbawks#676](https://git.gfpd.us/gfd/sandbawks/issues/676)) | `left: [<6 non-pool sandboxes>, "sbx-compound-pool-4mt6r", "sbx-compound-pool-bf94w"]` `right: ["sbx-8d6ff1ca9367"]` |
| clm_0019_list_filters_by_state | CLM-0019 | fail | | pass | sandbawks ([sandbawks#676](https://git.gfpd.us/gfd/sandbawks/issues/676)) | `left: [<6 non-pool sandboxes>, "sbx-compound-pool-4mt6r", "sbx-compound-pool-bf94w"]` |
| clm_0020_list_pages_without_duplicates_or_omissions | CLM-0020 | fail | | pass | sandbawks ([sandbawks#676](https://git.gfpd.us/gfd/sandbawks/issues/676)) | page union holds 5 other sandboxes besides the 3 of the test |
| clm_0021_kill_running_sandbox | CLM-0021 | pass | | pass | pass | |
| clm_0022_kill_unknown_sandbox_is_false | CLM-0022 | pass | | pass | pass | |
| clm_0023_pause_running_sandbox | CLM-0023 | fail | | pass | sandbawks ([sandbawks#679](https://git.gfpd.us/gfd/sandbawks/issues/679)) | `left: Running` `right: Paused` |
| clm_0024_pause_paused_sandbox_is_false | CLM-0024 | fail | | pass | sandbawks ([sandbawks#679](https://git.gfpd.us/gfd/sandbawks/issues/679)) | `assertion failed: !second` |
| clm_0025_set_timeout_moves_end_at | CLM-0025 | pass | | pass | pass | |
| clm_0026_refresh_extends_end_at | CLM-0026 | pass | | pass | pass | |
| clm_0027_is_running_tracks_kill | CLM-0027 | fail | | pass | sandbawks ([sandbawks#679](https://git.gfpd.us/gfd/sandbawks/issues/679)) | `want Ok(false) after kill, got Err(Envd { status: 404, message: "sandboxes.sandbawks.dev \"sbx-compound-pool-bf94w\" not found" })` |
| clm_0028_get_host_reaches_a_port_in_the_sandbox | CLM-0028 | fail | R1 fail | pass | sandbawks ([sandbawks#678](https://git.gfpd.us/gfd/sandbawks/issues/678)), before `get_host` is reached; run of record: capacity | `write: Envd { status: 404, message: "No such file or directory (os error 2)" }` |
| clm_0029_get_metrics_reports_cpu_memory_and_disk | CLM-0029 | fail | R1 fail | pass | missing route ([sandbawks#668](https://git.gfpd.us/gfd/sandbawks/issues/668)) | `sandbox vanished: SandboxNotFound("sbx-compound-pool-dmkk6")` |
| clm_0030_snapshot_starts_sandboxes_with_its_files | CLM-0030 | fail | R1 fail | pass | missing route ([sandbawks#669](https://git.gfpd.us/gfd/sandbawks/issues/669)); run of record: capacity | `snapshot: SandboxNotFound("sbx-compound-pool-f2xrw")` |
| clm_0030_fork_starts_running_copies | CLM-0030 | fail | R1 fail | pass | missing route ([sandbawks#670](https://git.gfpd.us/gfd/sandbawks/issues/670)); run of record: capacity | `fork: SandboxNotFound("sbx-compound-pool-gqjxj")` |
| clm_0033_run_returns_output_and_exit_code | CLM-0033 | pass | | pass | pass | |
| clm_0034_start_streams_chunks_then_exit | CLM-0034 | fail | R1 pass | pass | pass (run of record: capacity) | |
| clm_0035_run_delivers_large_output_in_order | CLM-0035 | fail | R1 pass | pass | pass (run of record: capacity, exit code 75) | |
| clm_0036_options_env_is_set | CLM-0036 | pass | | pass | pass | |
| clm_0037_options_cwd_is_working_directory | CLM-0037 | fail | R1 pass | pass | pass (run of record: capacity) | |
| clm_0038_options_user_runs_as_user | CLM-0038 | fail | R1 fail | pass | sandbawks ([sandbawks#682](https://git.gfpd.us/gfd/sandbawks/issues/682)); run of record: capacity | `left: "dev\n"` `right: "root\n"` |
| clm_0039_options_timeout_ends_long_command | CLM-0039 | pass | | pass | pass | |
| clm_0040_connect_attaches_to_running_command | CLM-0040 | fail | R1 fail | pass | sandbawks ([sandbawks#677](https://git.gfpd.us/gfd/sandbawks/issues/677)); run of record: capacity | `connect: Protocol("Connect stream did not open with a start event")` |
| clm_0041_list_includes_background_command | CLM-0041 | fail | R1 pass | pass | pass (run of record: capacity) | |
| clm_0042_send_stdin_reaches_command | CLM-0042 | fail | R1 pass | pass | pass (run of record: capacity) | |
| clm_0043_close_stdin_sends_eof | CLM-0043 | fail | | pass | sandbawks ([sandbawks#675](https://git.gfpd.us/gfd/sandbawks/issues/675); behind it [sandbawks#677](https://git.gfpd.us/gfd/sandbawks/issues/677)) | `close_stdin: InvalidArgument("close_stdin needs envd 0.5.2 or newer; this sandbox runs envd 0.1.28")` |
| clm_0044_sigterm_terminates_command | CLM-0044 | fail | R1 fail | pass | sandbawks ([sandbawks#677](https://git.gfpd.us/gfd/sandbawks/issues/677)); run of record: capacity | `pid 10 still listed after SIGTERM: [ProcessInfo { pid: 10, ... args: ["-l", "-c", "sleep 30"] ... }]` |
| clm_0045_kill_reports_whether_pid_existed | CLM-0045 | fail | R1 pass | pass | pass (run of record: capacity) | |
| clm_0046_stream_input_delivers_chunks_in_order | CLM-0046 | fail | | pass | sandbawks ([sandbawks#664](https://git.gfpd.us/gfd/sandbawks/issues/664)) | `close input stream: Rpc { code: NotFound, message: "HTTP 404" }` |
| clm_0047_pty_create_streams_terminal_output | CLM-0047 | pass | | pass | pass | |
| clm_0048_pty_resize_changes_terminal_size | CLM-0048 | fail | R1 pass | pass | pass (run of record: capacity) | |
| clm_0049_pty_input_reaches_the_shell | CLM-0049 | fail | | pass | sandbawks ([sandbawks#664](https://git.gfpd.us/gfd/sandbawks/issues/664)) | `close input stream: Rpc { code: NotFound, message: "HTTP 404" }` |
| clm_0050_write_read_round_trips_binary | CLM-0050 | fail | R1 pass | pass | pass (run of record: capacity) | |
| clm_0051_read_to_string_returns_text | CLM-0051 | fail | R1 pass | pass | pass (run of record: capacity) | |
| clm_0052_write_returns_write_info | CLM-0052 | fail | R1 pass | pass | pass (run of record: capacity) | |
| clm_0053_read_missing_file_is_not_found | CLM-0053 | fail | R1 pass | pass | pass (run of record: capacity) | |
| clm_0054_list_depth_one_lists_direct_entries | CLM-0054 | fail | R1 fail | pass | sandbawks ([sandbawks#678](https://git.gfpd.us/gfd/sandbawks/issues/678)); run of record: capacity | `write a: Envd { status: 404, message: "No such file or directory (os error 2)" }` |
| clm_0055_list_depth_two_recurses | CLM-0055 | fail | | pass | sandbawks ([sandbawks#678](https://git.gfpd.us/gfd/sandbawks/issues/678)) | `write a: Envd { status: 404, message: "No such file or directory (os error 2)" }` |
| clm_0056_get_info_reports_metadata | CLM-0056 | fail | R1 fail | pass | sandbawks ([sandbawks#687](https://git.gfpd.us/gfd/sandbawks/issues/687)); run of record: capacity | ``want `-rw...` permissions: EntryInfo { ... mode: 420, permissions: "rw-r--r--", owner: "", group: "", ... }`` |
| clm_0057_exists_true_and_false | CLM-0057 | fail | R1 pass | pass | pass (run of record: capacity) | |
| clm_0058_make_dir_reports_creation | CLM-0058 | fail | R1 fail, R2 fail, R3 fail | pass | sandbawks ([sandbawks#678](https://git.gfpd.us/gfd/sandbawks/issues/678)); run of record, R1 and R2: capacity | `make_dir of an existing directory returned true` |
| clm_0059_rename_moves_entry | CLM-0059 | pass | | pass | pass | |
| clm_0060_remove_deletes_file_and_directory | CLM-0060 | fail | R1 fail, R2 fail | pass | sandbawks ([sandbawks#678](https://git.gfpd.us/gfd/sandbawks/issues/678)); run of record and R1: capacity | `write file: Envd { status: 404, message: "No such file or directory (os error 2)" }` |
| clm_0061_user_selects_file_owner | CLM-0061 | fail | R1 fail | pass | sandbawks ([sandbawks#687](https://git.gfpd.us/gfd/sandbawks/issues/687), also [sandbawks#682](https://git.gfpd.us/gfd/sandbawks/issues/682) and N1); run of record: capacity | `left: ""` `right: "root"` |
| clm_0062_write_files_writes_every_file | CLM-0062 | fail | | pass | sandbawks ([sandbawks#684](https://git.gfpd.us/gfd/sandbawks/issues/684), reached through [sandbawks#675](https://git.gfpd.us/gfd/sandbawks/issues/675)) | ``write_files: Envd { status: 400, message: "Failed to deserialize query string: missing field `path`" }`` |
| clm_0063_watch_dir_reports_a_created_file | CLM-0063 | fail | R1 fail | pass | sandbawks ([sandbawks#664](https://git.gfpd.us/gfd/sandbawks/issues/664)); run of record: capacity | `watch_dir: Rpc { code: NotFound, message: "HTTP 404" }` |
| clm_0064_upload_url_accepts_a_multipart_post | CLM-0064 | fail | R1 pass | pass | pass, but unsigned ([sandbawks#683](https://git.gfpd.us/gfd/sandbawks/issues/683)); run of record: capacity | |
| clm_0065_download_url_fetches_without_credentials | CLM-0065 | fail | R1 fail | pass | sandbawks ([sandbawks#683](https://git.gfpd.us/gfd/sandbawks/issues/683)); run of record: capacity | `download url: InvalidArgument("a URL expiration needs a secured sandbox (one with an envd access token)")` |
| clm_0066_write_stream_uploads_more_than_it_buffers | CLM-0066 | fail | | pass | sandbawks ([sandbawks#685](https://git.gfpd.us/gfd/sandbawks/issues/685), multipart because of [sandbawks#675](https://git.gfpd.us/gfd/sandbawks/issues/675)) | `write_stream plain: Envd { status: 413, message: "<html>...<title>413 Request Entity Too Large</title>...<center>nginx</center>..." }` |
| clm_0128_read_stream_downloads_in_chunks | CLM-0128 | fail | R1 fail | pass | sandbawks ([sandbawks#686](https://git.gfpd.us/gfd/sandbawks/issues/686)); run of record: capacity | `read_stream: Envd { status: 500, message: "an internal error occurred" }` |
| clm_0129_write_metadata_round_trips | CLM-0129 | fail | | pass | sandbawks ([sandbawks#675](https://git.gfpd.us/gfd/sandbawks/issues/675)) | `file metadata needs envd 0.6.2 or newer; this template runs envd 0.1.28` |
| clm_0143_create_network_allow_and_deny_lists_are_enforced | CLM-0143 | fail | R1 fail | pass | missing feature ([sandbawks#667](https://git.gfpd.us/gfd/sandbawks/issues/667)); run of record: capacity | `1.0.0.1 is denied by ALL_TRAFFIC but replied` |
| clm_0144_update_network_replaces_egress_without_restart | CLM-0144 | fail | | pass | missing feature and route ([sandbawks#667](https://git.gfpd.us/gfd/sandbawks/issues/667)) | `egress was open before any update` |
| clm_0145_lifecycle_pause_with_full_snapshot_pauses_at_timeout | CLM-0145 | fail | | pass | missing feature ([sandbawks#665](https://git.gfpd.us/gfd/sandbawks/issues/665), also [sandbawks#679](https://git.gfpd.us/gfd/sandbawks/issues/679)) | `timed out after 120s waiting for lifecycle pause at timeout, with its pause mode` |
| clm_0146_resume_restores_memory_or_reboots | CLM-0146 | fail | R1 fail, R2 fail | pass | sandbawks N1, before the pause is reached (behind it [sandbawks#679](https://git.gfpd.us/gfd/sandbawks/issues/679), [sandbawks#666](https://git.gfpd.us/gfd/sandbawks/issues/666)); run of record and R1: capacity | `touch: cannot touch '/home/user/on-disk': No such file or directory` (exit code 1) |
| clm_0146_filesystem_pause_cold_boots | CLM-0146 | fail | R1 fail | pass | sandbawks N1, before the pause is reached (behind it [sandbawks#679](https://git.gfpd.us/gfd/sandbawks/issues/679)); run of record: capacity | `touch: cannot touch '/home/user/on-disk': No such file or directory` (exit code 1) |
| clm_0147_list_and_delete_snapshots | CLM-0147 | fail | R1 fail | pass | missing route ([sandbawks#669](https://git.gfpd.us/gfd/sandbawks/issues/669)) | `snapshot: SandboxNotFound("sbx-compound-pool-pc7rd")` |
| clm_0148_volume_create_get_list_destroy | CLM-0148 | fail | | fail (environment limit, 403) | missing route ([sandbawks#671](https://git.gfpd.us/gfd/sandbawks/issues/671)) | `create: Api { status: 404, message: "HTTP 404" }` |
| clm_0148_missing_volume_is_not_found_and_not_destroyed | CLM-0148 | pass | | pass | pass, but only because the whole route answers 404 ([sandbawks#671](https://git.gfpd.us/gfd/sandbawks/issues/671)) | |
| clm_0149_volume_mount_persists_across_sandboxes | CLM-0149 | fail | | fail (environment limit, 403) | missing route ([sandbawks#671](https://git.gfpd.us/gfd/sandbawks/issues/671)) | `create volume: Api { status: 404, message: "HTTP 404" }` |
| clm_0150_mcp_gateway_starts_and_exposes_url_and_token | CLM-0150 | fail | R1 fail | pass | sandbawks ([sandbawks#680](https://git.gfpd.us/gfd/sandbawks/issues/680), probably: no `mcp-gateway` template); the `mcp` create field is out of #673's scope and goes with [sandbawks#672](https://git.gfpd.us/gfd/sandbawks/issues/672) | `create sandbox: McpGateway("ERROR: sandbox will not become ready without intervention: the sandbox failed (action_failure)\n")` |
| clm_0151_list_filters_by_template_and_start_time_and_orders | CLM-0151 | fail | R1 fail | pass | sandbawks N2 (with [sandbawks#676](https://git.gfpd.us/gfd/sandbawks/issues/676)) | `["sbx-91d5e81e4a28", "sbx-compound-pool-5g966", "sbx-compound-pool-sw4nn"]` (desc order assertion, `tests/live_lifecycle.rs:718`) |
| clm_0153_get_info_reports_network | CLM-0153 | fail | | pass | missing feature ([sandbawks#667](https://git.gfpd.us/gfd/sandbawks/issues/667)) | `left: None` `right: Some(false)` (`allow_internet_access`) |
| clm_0153_get_info_reports_lifecycle | CLM-0153 | fail | | pass | missing feature ([sandbawks#665](https://git.gfpd.us/gfd/sandbawks/issues/665)) | `lifecycle in the detail` (the detail has no `lifecycle`) |
| clm_0153_get_info_reports_volume_mounts | CLM-0153 | fail | | fail (environment limit, 403) | missing route ([sandbawks#671](https://git.gfpd.us/gfd/sandbawks/issues/671)) | `create volume: Api { status: 404, message: "HTTP 404" }` |

**Totals.** 75 `clm_*` tests: lifecycle 39, commands 17, files 19. In the run
of record, 16 passed and 59 failed (lifecycle 11/39, commands 4/17, files
1/19). After the reruns, 30 pass and 45 fail (lifecycle 12/39, commands
11/17, files 7/19). No failure is left at capacity: each of the 14 tests
that failed only on readiness passed in a later round, and every other
capacity-masked test then failed on a defect.

- **M1's and M2's 57 tests:** 29 pass and 28 fail, the same two sets as in M2
  after its reruns (section "sandbawks" of `m2-live-results.md`). All 28
  failures match the gfd/sandbawks issue M2 recorded (or that M2's N1 to N5
  became, #683 to #687). One message changed: CLM-0043 now stops in the
  crate, which refuses `close_stdin` below envd 0.5.2 as the SDK does, so it
  fails on the reported envd version
  ([sandbawks#675](https://git.gfpd.us/gfd/sandbawks/issues/675)) before it
  reaches the `pid 0` of [sandbawks#677](https://git.gfpd.us/gfd/sandbawks/issues/677).
- **M3's 18 tests:** 1 passes (`clm_0148_missing_volume_is_not_found_and_not_destroyed`,
  for the wrong reason: every volume route answers 404) and 17 fail. 12 hit a
  known missing route or ignored field (#665, #667 to #671), 2 more
  (CLM-0028, CLM-0150) an M1-era issue (#678, #680), and 3 are new:
  CLM-0146 (2 tests, N1) and CLM-0151 (N2).

**Sweeps.** For the run of record and for each of R1, R2 and R3:

- `sweep: killed 0 leftover sandbox(es) of run <id>`.
- The snapshot sweep fails with `list snapshots: Api { status: 404, message:
  "HTTP 404" }`, and the volume sweep with `list volumes: Api { status: 404,
  message: "HTTP 404" }`. sandbawks serves neither `GET /snapshots` nor
  `GET /volumes` ([sandbawks#669](https://git.gfpd.us/gfd/sandbawks/issues/669),
  [sandbawks#671](https://git.gfpd.us/gfd/sandbawks/issues/671)), so this is
  noted, not counted as a leak: no snapshot or volume could have been
  created (every create in the run got 404 too). Both sweeps kill the run's
  sandboxes before they list, so the 404 skips no sandbox kill.

No log has a `could not kill` warning. After R3 the sandbox sweep was run
again for each of the four run ids, and for
`20261028T012856Z-sandbawks-2761559`, a first `just test-live` started from
a tree that was dirty (a stdout file inside it), stopped while it was still
compiling, before any test ran; its log is not kept. All five said `killed 0`.
An unfiltered `GET /v2/sandboxes` then listed only the two warm-pool
sandboxes, with no metadata.

**Secret check (M1 Task 6 Step 3, on each of the four logs before it was
copied here):** `E2B_API_KEY_HOSTED: 0`, `E2B_API_KEY_SANDBAWKS: 0`,
`GATEWAY_ACCESS_TOKEN=: 0` and `"token": 0` for each. Both `.env` values
were checked to be unquoted and non-empty (without printing them), so those
zeros are real. No log has `signature=`, `v1_`, `X-Access-Token`, `Bearer`
or any `https://` URL. sandbawks returned no `envdAccessToken`
([sandbawks#683](https://git.gfpd.us/gfd/sandbawks/issues/683)), and the MCP
gateway never started, so there was no token to leak.

### Known missing routes and fields (epic sandbawks#673)

Every M3 failure in this group shows what its sub-issue describes:

- **A missing route reads as "sandbox not found".** sandbawks answers
  `GET /sandboxes/{id}/metrics`, `POST /sandboxes/{id}/snapshots`,
  `POST /sandboxes/{id}/fork`, `GET /snapshots` and `POST`/`GET /volumes`
  with a bare 404 and an empty body. A direct probe after R1 (2026-10-28,
  read-only, on a warm-pool sandbox) got `404` with no body for
  `GET /sandboxes/sbx-compound-pool-czcwt/metrics`, `GET /snapshots` and
  `GET /volumes`, while `GET /sandboxes/sbx-compound-pool-czcwt` answered 200.
  On a sandbox route, a 404 is the API's "no such sandbox" answer, so the
  crate reports `SandboxNotFound` (CLM-0029, CLM-0030, CLM-0147), as the SDK
  raises `NotFoundException`. Epic #673's goal ("nothing should come back as
  a bare 404") covers this.
- **CLM-0143, CLM-0144, CLM-0153 (network):** create ignores `network` and
  `allowInternetAccess`
  ([sandbawks#667](https://git.gfpd.us/gfd/sandbawks/issues/667)). With
  `denyOut: ["0.0.0.0/0"]` and `allowOut: ["1.1.1.1"]`, 1.0.0.1 replies; with
  `denyOut: ["0.0.0.0/0"]` alone, 1.1.1.1 replies before any update, so CLM-0144
  never reached `PUT /sandboxes/{id}/network`. The detail has no
  `allowInternetAccess` and no `network`: the probe above saw only `alias`,
  `clientID`, `cpuCount`, `diskSizeMB`, `endAt`, `envdVersion`, `memoryMB`,
  `metadata`, `sandboxID`, `startedAt`, `state` and `templateID`.
- **CLM-0145, CLM-0153 (lifecycle):** create ignores the lifecycle fields
  ([sandbawks#665](https://git.gfpd.us/gfd/sandbawks/issues/665)), and pause
  is a no-op ([sandbawks#679](https://git.gfpd.us/gfd/sandbawks/issues/679)),
  so the sandbox never turns `paused` at its 15 s timeout, and the detail
  has no `lifecycle` or `pauseMode`.
- **CLM-0148, CLM-0149, CLM-0153 (volumes):** `POST /volumes` gets a bare
  404 ([sandbawks#671](https://git.gfpd.us/gfd/sandbawks/issues/671)). On
  hosted the same three tests stop at the same call with 403 (volumes not
  enabled for the team), so neither server has exercised volumes yet.
- **CLM-0028** stops at the test's setup, writing `/tmp/www/hello.txt`,
  because envd does not create parent directories
  ([sandbawks#678](https://git.gfpd.us/gfd/sandbawks/issues/678)); `get_host`
  itself is untested here.
- **CLM-0150** asks for the `mcp-gateway` template, which hosted E2B provides.
  Create answers `running`, and the gateway's start command then gets
  `the sandbox failed (action_failure)` in both attempts. That is the
  signature of an unknown template
  ([sandbawks#680](https://git.gfpd.us/gfd/sandbawks/issues/680)); the
  template list cannot be checked, since `GET /templates` is 404 too. Epic
  #673 puts the `mcp` create field out of its scope, with #672's secrets
  decision.

### New sandbawks defects, for issues on gfd/sandbawks

Seen through the crate on 2026-10-28. envd requests go to
`https://49983-<id>.sb-ghlw.gfd.dev` with the `E2b-Sandbox-Id` and
`E2b-Sandbox-Port` routing headers; API requests go to
`https://api.sb-ghlw.gfd.dev` with `X-API-Key`. The requests are the ones
the crate sends, as `docs/protocol-notes.md` describes them with their SDK
citations (e2b 2.61.0); N2 was also probed directly.

**N1. The `compound-sandbox` template has no `/home/user`** (CLM-0146, both tests; also part of CLM-0061's failure).
- Client call: `create_sandbox(...)` on `compound-sandbox`, then
  `commands().run("touch /dev/shm/in-memory /home/user/on-disk && sync", &CommandOptions::default())`.
- Request: `POST /process.Process/Start` with `cmd: "/bin/bash", args: ["-l",
  "-c", "touch /dev/shm/in-memory /home/user/on-disk && sync"]` and
  `Authorization: Basic base64("user:")`: with no user given, the crate
  sends `user` for envd older than 0.4.0, as the SDK does (`py:
  envd/utils.py:44-57`; `py: envd/versions.py:6`), and sandbawks reports
  0.1.28 ([sandbawks#675](https://git.gfpd.us/gfd/sandbawks/issues/675)).
- Response: the command ends with exit code 1 and stderr `touch: cannot
  touch '/home/user/on-disk': No such file or directory`, in R1 and R2 for
  `clm_0146_resume_restores_memory_or_reboots`, and in R1 for
  `clm_0146_filesystem_pause_cold_boots` (each on a ready sandbox).
- Expected: E2B templates have a `user` account with home `/home/user`,
  the user the SDKs name by default for old envd (above), and hosted's
  `base` has it (both CLM-0146 tests pass there). Commands here run as `dev` ([sandbawks#682](https://git.gfpd.us/gfd/sandbawks/issues/682)),
  and M2 already saw the missing `user` account through CLM-0061. This may be
  a property of the template rather than of the gateway; either way it keeps
  both CLM-0146 tests from reaching the pause, so their real blockers
  ([sandbawks#679](https://git.gfpd.us/gfd/sandbawks/issues/679), and
  [sandbawks#666](https://git.gfpd.us/gfd/sandbawks/issues/666) for
  `memory: false` on connect) stay unseen.

**N2. `GET /v2/sandboxes` ignores `order`, `template` and `startedAfter`** (CLM-0151).
- Client call: `list_sandboxes` with `.metadata(RUN_METADATA_KEY, run_id())`,
  `.template("compound-sandbox")` and `.order(SortOrder::Descending)`, then
  the same with `Ascending`, then with `.started_after(<second sandbox's
  startedAt>)`.
- Request: `GET /v2/sandboxes?metadata=...&template=compound-sandbox&order=desc`
  (then `order=asc`, then `startedAfter=<isoformat time>`).
- Response: the same list in the same order for `desc` and `asc`, so the
  test's descending check fails (`tests/live_lifecycle.rs:718`, R1). A direct
  probe after R1 (read-only) confirmed it with nothing of ours running:
  `order=asc`, `order=desc`, `template=compound-sandbox&order=desc`,
  `startedAfter=2030-01-01T00:00:00+00:00` and `template=nonexistent` each
  answered 200 with the same two warm-pool sandboxes, in the same order
  (`sbx-compound-pool-czcwt` started 02:52:57, then
  `sbx-compound-pool-sw4nn` started 02:48:46).
- Expected: `order` sorts by start time, `asc` or `desc`; `startedAfter`
  keeps only sandboxes started after the time; `template` keeps only
  sandboxes of that template ID or name (`py:
  api/client/api/sandboxes/get_v2_sandboxes.py:14-61`; hosted passes
  CLM-0151). [sandbawks#676](https://git.gfpd.us/gfd/sandbawks/issues/676)
  covers the ignored `metadata` filter (and, through CLM-0019, `state`); these
  three are the SDK 2.61 filters it does not name.
