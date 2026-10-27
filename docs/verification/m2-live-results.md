Results of the Milestone 2 live runs (plan Tasks 9 and 10), one section per target.

## Hosted E2B (run 20261027T125515Z-hosted-1922536, 2026-10-27T12:55:15Z, commit 9910325)

Run with `just test-live hosted` on `9910325` (main with every M2 feature task,
#26 and #31 to #37), template `base`. The log starts `commit:
9910325fd8d67203924e0841e445e35c17087bd7` and `dirty: no`. Log:
[`runs/20261027T125515Z-hosted-1922536.log`](runs/20261027T125515Z-hosted-1922536.log).
M1's 46 tests were rerun on purpose, as a regression check.

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

**Totals:** 57 `clm_*` tests: 57 passed, 0 failed (commands 17/17, files 19/19,
lifecycle 21/21). That is M1's 46 plus M2's 11 (0046, 0047, 0048, 0049, 0062,
0063, 0064, 0065, 0066, 0128, 0129), as the plan expects. The sweep test is not
a claim test and is not counted. This was one run, with no reruns.

**Sweep:** `sweep: killed 0 leftover sandbox(es) of run 20261027T125515Z-hosted-1922536`.
Every drop guard killed its sandbox, and the log has no `could not kill` warning.

**Secret check (M1 Task 6 Step 3, on the log before it was copied here):**
`E2B_API_KEY_HOSTED: 0`, `E2B_API_KEY_SANDBAWKS: 0`. Both `.env` values were
checked to be unquoted (without printing them), so those zeros are real.

**Signed-URL check:** the CLM-0064 and CLM-0065 tests make signed URLs, and a
signed URL is a bearer credential until it expires. The log has no
`signature=` and no `v1_`, and no URL at all. Its two `access` hits are both
the test name `clm_0012_internet_access_false_blocks_outbound`.

### Failures

None.

### Notes on passes

- **CLM-0129.** It passed, so the `base` template's envd is new enough (0.6.2 or
  later) for file metadata; there is no environment limit to record.

## sandbawks (run 20261027T130108Z-sandbawks-2101151, 2026-10-27T13:01:08Z, commit 0b9494b)

Run with `just test-live sandbawks` on `0b9494b` (main with every M2 feature
task and #38), template `compound-sandbox` (`E2B_TEMPLATE_SANDBAWKS` in `.env`),
against sandbawks at `sb-ghlw.gfd.dev`. Every sandbox reported
`"envdVersion":"0.1.28"`. The log starts `commit:
0b9494ba6f5c190554826e683d3c13f489e82910` and `dirty: no`. Hosted E2B is the
canonical server: every test that fails here passed on hosted in run
`20261027T125515Z-hosted-1922536` (above), so no failure is a crate bug.

There was one full run and four rounds of single-test reruns. Every log is
checked in, and every one starts with the same `commit:` line and `dirty: no`:

| Run id | What it was | Log |
|---|---|---|
| `20261027T130108Z-sandbawks-2101151` | `just test-live sandbawks` (**the run of record**) | [log](runs/20261027T130108Z-sandbawks-2101151.log) |
| `20261027T130557Z-sandbawks-rerun-2121210` | R1: each of the 43 failures on its own, 20 s apart, then the sweep | [log](runs/20261027T130557Z-sandbawks-rerun-2121210.log) |
| `20261027T133834Z-sandbawks-rerun2-2291013` | R2: the 30 failures R1 could not classify, after a 5 min pause, 120 s apart, then the sweep | [log](runs/20261027T133834Z-sandbawks-rerun2-2291013.log) |
| `20261027T144305Z-sandbawks-rerun3-2463003` | R3: the 6 still capacity-masked, plus 4 new failures to confirm they repeat (0012, 0056, 0061, 0128), 120 s apart, then the sweep | [log](runs/20261027T144305Z-sandbawks-rerun3-2463003.log) |
| `20261027T150453Z-sandbawks-rerun4-2799756` | R4: the 4 still capacity-masked (0035, 0036, 0037, 0039), 120 s apart, then the sweep | [log](runs/20261027T150453Z-sandbawks-rerun4-2799756.log) |

Each rerun ran `cargo test --test <binary> -- --ignored --exact <test>
--test-threads=1 --show-output` with `E2B_TEST_TARGET=sandbawks` and the
round's own `E2B_TEST_RUN_ID`, then `live_sweep` for that id.

**Why there are four rerun rounds.** The cluster had no room for cold
sandboxes. At `--test-threads=4` the run of record drained the two-sandbox warm
pool at once, and the cold creates that followed went unschedulable:
`Unschedulable: 0/29 nodes are available: 19 Insufficient memory, 2 node(s)
didn't match Pod's node affinity/selector, ... 5 Insufficient cpu, 7
Insufficient ephemeral-storage`. Create still answered `running`
([sandbawks#674](https://git.gfpd.us/gfd/sandbawks/issues/674)), so these tests
failed on envd's `sandbox is still starting up` (HTTP 503, Connect
`unavailable`, or a synthetic exit code 75). During R1 the cluster was adding
nodes (`0/35 nodes are available: ... 7 node(s) had untolerated taint
{node.kubernetes.io/not-ready: }, 6 node(s) had untolerated taint
{node.cilium.io/agent-not-ready: }`), and some sandboxes failed outright
(`412 sandbox will not become ready without intervention: the sandbox failed
(action_failure)`, or exit code 69). With 20 s between tests the pool never
refilled, so R1 settled only 13 tests. With 120 s gaps, most tests in R2 to R4
got a ready sandbox. Every test was rerun until it either passed or failed on
something other than readiness.

**Classes:** **pass**; **sandbawks** (a sandbawks defect, filed issue or new
N-number below); **capacity** (the sandbox never became ready; says nothing
about the feature). There are no **crate** or **test** classes: every failure
passes on hosted, and no test was found wrong. The last column quotes the
last attempt.

| Test | Claim | Run of record | Reruns | Hosted | Class | Failure (first line, last attempt) |
|---|---|---|---|---|---|---|
| clm_0007_create_returns_running_sandbox | CLM-0007 | pass | | pass | pass | |
| clm_0008_create_metadata_is_visible_in_info | CLM-0008 | pass | | pass | pass | |
| clm_0009_create_env_vars_reach_commands | CLM-0009 | fail | R1 fail, R2 fail | pass | sandbawks ([sandbawks#681](https://git.gfpd.us/gfd/sandbawks/issues/681)) | `left: "\n"` `right: "hi from create\n"` |
| clm_0010_create_timeout_sets_end_at | CLM-0010 | pass | | pass | pass | |
| clm_0011_auto_pause_pauses_at_timeout | CLM-0011 | fail | R1 fail | pass | sandbawks ([sandbawks#679](https://git.gfpd.us/gfd/sandbawks/issues/679), [sandbawks#665](https://git.gfpd.us/gfd/sandbawks/issues/665)) | `timed out after 120s waiting for auto-pause at timeout` |
| clm_0012_internet_access_false_blocks_outbound | CLM-0012 | fail | R1 fail, R2 fail, R3 fail | pass | sandbawks ([sandbawks#667](https://git.gfpd.us/gfd/sandbawks/issues/667)); passed in M1 | `sandbox without internet access got an HTTP reply from 1.1.1.1:80: CommandResult { stdout: "", stderr: "", exit_code: 0, ... }` |
| clm_0013_connect_running_sandbox_runs_commands | CLM-0013 | fail | R1 fail, R2 pass | pass | pass (run of record: capacity) | |
| clm_0014_connect_resumes_paused_sandbox_with_files | CLM-0014 | fail | R1 fail, R2 fail | pass | sandbawks ([sandbawks#679](https://git.gfpd.us/gfd/sandbawks/issues/679)) | `left: Running` `right: Paused` |
| clm_0015_connect_unknown_sandbox_is_not_found | CLM-0015 | pass | | pass | pass | |
| clm_0016_get_info_returns_detail | CLM-0016 | pass | | pass | pass | |
| clm_0017_get_info_unknown_sandbox_is_not_found | CLM-0017 | pass | | pass | pass | |
| clm_0018_list_filters_by_metadata | CLM-0018 | fail | R1 fail | pass | sandbawks ([sandbawks#676](https://git.gfpd.us/gfd/sandbawks/issues/676)) | `left: ["sbx-65e78e7479f0", "sbx-65e7b263645c", "sbx-compound-pool-8nxfb", "sbx-compound-pool-pjz7c"]` |
| clm_0019_list_filters_by_state | CLM-0019 | fail | R1 fail | pass | sandbawks ([sandbawks#676](https://git.gfpd.us/gfd/sandbawks/issues/676)) | `left: ["sbx-65ec9963dc05", "sbx-65ecacd2bd57", "sbx-compound-pool-8nxfb", "sbx-compound-pool-pjz7c"]` |
| clm_0020_list_pages_without_duplicates_or_omissions | CLM-0020 | fail | R1 fail | pass | sandbawks ([sandbawks#676](https://git.gfpd.us/gfd/sandbawks/issues/676)) | page union holds the 2 pool sandboxes besides the 3 of the test |
| clm_0021_kill_running_sandbox | CLM-0021 | pass | | pass | pass | |
| clm_0022_kill_unknown_sandbox_is_false | CLM-0022 | pass | | pass | pass | |
| clm_0023_pause_running_sandbox | CLM-0023 | fail | R1 fail | pass | sandbawks ([sandbawks#679](https://git.gfpd.us/gfd/sandbawks/issues/679)) | `left: Running` `right: Paused` |
| clm_0024_pause_paused_sandbox_is_false | CLM-0024 | fail | R1 fail | pass | sandbawks ([sandbawks#679](https://git.gfpd.us/gfd/sandbawks/issues/679)) | `assertion failed: !second` |
| clm_0025_set_timeout_moves_end_at | CLM-0025 | pass | | pass | pass | |
| clm_0026_refresh_extends_end_at | CLM-0026 | pass | | pass | pass | |
| clm_0027_is_running_tracks_kill | CLM-0027 | fail | R1 fail, R2 fail, R3 fail | pass | sandbawks ([sandbawks#679](https://git.gfpd.us/gfd/sandbawks/issues/679)); R1 and R2: capacity | `want Ok(false) after kill, got Err(Envd { status: 404, message: "sandboxes.sandbawks.dev \"sbx-compound-pool-9vnlw\" not found" })` |
| clm_0033_run_returns_output_and_exit_code | CLM-0033 | pass | | pass | pass | |
| clm_0034_start_streams_chunks_then_exit | CLM-0034 | pass | | pass | pass | |
| clm_0035_run_delivers_large_output_in_order | CLM-0035 | fail | R1 fail, R2 fail, R3 fail, R4 pass | pass | pass (earlier attempts: capacity) | |
| clm_0036_options_env_is_set | CLM-0036 | fail | R1 fail, R2 fail, R3 fail, R4 pass | pass | pass (earlier attempts: capacity) | |
| clm_0037_options_cwd_is_working_directory | CLM-0037 | fail | R1 fail, R2 fail, R3 fail, R4 pass | pass | pass (earlier attempts: capacity) | |
| clm_0038_options_user_runs_as_user | CLM-0038 | fail | R1 fail, R2 fail | pass | sandbawks ([sandbawks#682](https://git.gfpd.us/gfd/sandbawks/issues/682)) | `left: "dev\n"` `right: "root\n"` |
| clm_0039_options_timeout_ends_long_command | CLM-0039 | fail | R1 fail, R2 fail, R3 fail, R4 pass | pass | pass (earlier attempts: capacity) | |
| clm_0040_connect_attaches_to_running_command | CLM-0040 | fail | R1 fail, R2 fail, R3 fail | pass | sandbawks ([sandbawks#677](https://git.gfpd.us/gfd/sandbawks/issues/677)); R1 and R2: capacity | `connect: Protocol("Connect stream did not open with a start event")` |
| clm_0041_list_includes_background_command | CLM-0041 | pass | | pass | pass | |
| clm_0042_send_stdin_reaches_command | CLM-0042 | fail | R1 fail, R2 pass | pass | pass (earlier attempts: capacity) | |
| clm_0043_close_stdin_sends_eof | CLM-0043 | fail | R1 fail, R2 fail | pass | sandbawks ([sandbawks#677](https://git.gfpd.us/gfd/sandbawks/issues/677), [sandbawks#675](https://git.gfpd.us/gfd/sandbawks/issues/675)) | `send_stdin: Rpc { code: NotFound, message: "no process with pid 0" }` |
| clm_0044_sigterm_terminates_command | CLM-0044 | fail | R1 fail | pass | sandbawks ([sandbawks#677](https://git.gfpd.us/gfd/sandbawks/issues/677)) | `pid 10 still listed after SIGTERM: [ProcessInfo { pid: 10, ... args: ["-l", "-c", "sleep 30"] ... }]` |
| clm_0045_kill_reports_whether_pid_existed | CLM-0045 | fail | R1 fail, R2 pass | pass | pass (earlier attempts: capacity) | |
| clm_0046_stream_input_delivers_chunks_in_order | CLM-0046 | fail | R1 fail | pass | sandbawks ([sandbawks#664](https://git.gfpd.us/gfd/sandbawks/issues/664)) | `close input stream: Rpc { code: NotFound, message: "HTTP 404" }` |
| clm_0047_pty_create_streams_terminal_output | CLM-0047 | fail | R1 fail, R2 pass | pass | pass (earlier attempts: capacity) | |
| clm_0048_pty_resize_changes_terminal_size | CLM-0048 | pass | | pass | pass | |
| clm_0049_pty_input_reaches_the_shell | CLM-0049 | fail | R1 fail | pass | sandbawks ([sandbawks#664](https://git.gfpd.us/gfd/sandbawks/issues/664)) | `close input stream: Rpc { code: NotFound, message: "HTTP 404" }` |
| clm_0050_write_read_round_trips_binary | CLM-0050 | fail | R1 fail, R2 pass | pass | pass (earlier attempts: capacity) | |
| clm_0051_read_to_string_returns_text | CLM-0051 | fail | R1 pass | pass | pass (run of record: capacity) | |
| clm_0052_write_returns_write_info | CLM-0052 | fail | R1 fail, R2 pass | pass | pass (earlier attempts: capacity) | |
| clm_0053_read_missing_file_is_not_found | CLM-0053 | fail | R1 fail, R2 pass | pass | pass (earlier attempts: capacity) | |
| clm_0054_list_depth_one_lists_direct_entries | CLM-0054 | fail | R1 fail, R2 fail | pass | sandbawks ([sandbawks#678](https://git.gfpd.us/gfd/sandbawks/issues/678)) | `write a: Envd { status: 404, message: "No such file or directory (os error 2)" }` |
| clm_0055_list_depth_two_recurses | CLM-0055 | fail | R1 fail, R2 fail | pass | sandbawks ([sandbawks#678](https://git.gfpd.us/gfd/sandbawks/issues/678)) | `write a: Envd { status: 404, message: "No such file or directory (os error 2)" }` |
| clm_0056_get_info_reports_metadata | CLM-0056 | fail | R1 fail, R2 fail, R3 fail | pass | sandbawks, new N4 | ``want `-rw...` permissions: EntryInfo { ... mode: 420, permissions: "rw-r--r--", owner: "", group: "", ... }`` |
| clm_0057_exists_true_and_false | CLM-0057 | fail | R1 fail, R2 pass | pass | pass (earlier attempts: capacity) | |
| clm_0058_make_dir_reports_creation | CLM-0058 | fail | R1 fail, R2 fail | pass | sandbawks ([sandbawks#678](https://git.gfpd.us/gfd/sandbawks/issues/678)) | `make_dir of an existing directory returned true` |
| clm_0059_rename_moves_entry | CLM-0059 | fail | R1 fail, R2 pass | pass | pass (earlier attempts: capacity) | |
| clm_0060_remove_deletes_file_and_directory | CLM-0060 | fail | R1 fail, R2 fail | pass | sandbawks ([sandbawks#678](https://git.gfpd.us/gfd/sandbawks/issues/678)) | `write file: Envd { status: 404, message: "No such file or directory (os error 2)" }` |
| clm_0061_user_selects_file_owner | CLM-0061 | fail | R1 fail, R2 fail, R3 fail | pass | sandbawks, new N4 (also [sandbawks#682](https://git.gfpd.us/gfd/sandbawks/issues/682) and the missing `user` account) | `left: ""` `right: "root"` |
| clm_0062_write_files_writes_every_file | CLM-0062 | fail | R1 fail | pass | sandbawks, new N1 (reached through [sandbawks#675](https://git.gfpd.us/gfd/sandbawks/issues/675)) | ``write_files: Envd { status: 400, message: "Failed to deserialize query string: missing field `path`" }`` |
| clm_0063_watch_dir_reports_a_created_file | CLM-0063 | fail | R1 fail, R2 fail | pass | sandbawks ([sandbawks#664](https://git.gfpd.us/gfd/sandbawks/issues/664)) | `watch_dir: Rpc { code: NotFound, message: "HTTP 404" }` |
| clm_0064_upload_url_accepts_a_multipart_post | CLM-0064 | fail | R1 fail, R2 pass | pass | pass, but unsigned (see N5) | |
| clm_0065_download_url_fetches_without_credentials | CLM-0065 | fail | R1 fail, R2 fail | pass | sandbawks, new N5 | `download url: InvalidArgument("a URL expiration needs a secured sandbox (one with an envd access token)")` |
| clm_0066_write_stream_uploads_more_than_it_buffers | CLM-0066 | fail | R1 fail | pass | sandbawks, new N2 (multipart because of [sandbawks#675](https://git.gfpd.us/gfd/sandbawks/issues/675)) | `write_stream plain: Envd { status: 413, message: "<html>...<title>413 Request Entity Too Large</title>...<center>nginx</center>..." }` |
| clm_0128_read_stream_downloads_in_chunks | CLM-0128 | fail | R1 fail, R2 fail, R3 fail | pass | sandbawks, new N3 | `read_stream: Envd { status: 500, message: "an internal error occurred" }` |
| clm_0129_write_metadata_round_trips | CLM-0129 | fail | R1 fail | pass | sandbawks ([sandbawks#675](https://git.gfpd.us/gfd/sandbawks/issues/675)) | `file metadata needs envd 0.6.2 or newer; this template runs envd 0.1.28` |

**Totals.** 57 `clm_*` tests: lifecycle 21, commands 17, files 19. In the run
of record, 14 passed and 43 failed (lifecycle 10/21, commands 4/17, files
0/19). After the reruns, 29 pass and 28 fail (lifecycle 11/21, commands 11/17,
files 7/19). All 28 failures are sandbawks defects: 22 match an issue already
filed on gfd/sandbawks, and 6 are new (N1 to N5 below; N4 covers two tests).
No failure is left at capacity: each of the 15 tests that failed only on
readiness passed in a later round. None of the 28 fails on hosted E2B.

**M2's 11 new tests:** CLM-0047 passes (after capacity) and CLM-0048 passes.
CLM-0046 and 0063 hit [sandbawks#664](https://git.gfpd.us/gfd/sandbawks/issues/664),
CLM-0049 too (its `pty().stream_input` is the same RPC), and CLM-0129 hits
[sandbawks#675](https://git.gfpd.us/gfd/sandbawks/issues/675). CLM-0062, 0064 to 0066
and 0128 hit the new N1, N5, N2 and N3. CLM-0064 passes, but only because
sandbawks accepts unsigned, unauthenticated uploads (N5).

**Sweep.** `sweep: killed 0 leftover sandbox(es) of run <id>` for the run of
record and for each of R1, R2, R3 and R4. No log has a `could not kill`
warning, so no sandbox ids are left to report. The sweep only kills sandboxes
carrying the round's run id, so it left alone the warm-pool sandboxes that
the unfiltered list returns ([sandbawks#676](https://git.gfpd.us/gfd/sandbawks/issues/676)).

**Secret check (M1 Task 6 Step 3, on each of the five logs before it was
copied here):** `E2B_API_KEY_HOSTED: 0` and `E2B_API_KEY_SANDBAWKS: 0` for each.
Both `.env` values were checked to be unquoted (without printing them), so those
zeros are real.

**Signed-URL and token check:** none of the five logs has `signature=`, `v1_`,
`X-Access-Token` or `X-API-Key`. The only URL in any log is
`https://api.sb-ghlw.gfd.dev/v2/sandboxes` (a create that timed out in R1). The
only `token` hit is the CLM-0065 error text quoted above, and every `access`
hit is that text, the CLM-0012 test name or the CLM-0012 failure message.
sandbawks returned no `envdAccessToken` (N5), so no URL was signed anyway.

### sandbawks defects already filed

Each failure above that names a gfd/sandbawks issue shows the same behaviour
that the issue describes and that M1 recorded (see `m1-live-results.md`,
section "sandbawks", S1 to S12), with three notes:

- **CLM-0012 passed on sandbawks in M1** (its single-test rerun, 2026-10-26)
  and now fails three times out of three on a ready sandbox. The control
  sandbox reaches `1.1.1.1:80`, and so does the sandbox created with
  `"allowInternetAccess": false`. [sandbawks#667](https://git.gfpd.us/gfd/sandbawks/issues/667)
  says create ignores `allowInternetAccess` and that the default is deny; the
  default now looks like allow. That may be worth a comment on #667.
- **CLM-0014** now fails on [sandbawks#679](https://git.gfpd.us/gfd/sandbawks/issues/679)
  (pause is a no-op), as M1's note predicted once the test asserted the paused
  state.
- **CLM-0046, 0049 and 0063** are the M2 calls that
  [sandbawks#664](https://git.gfpd.us/gfd/sandbawks/issues/664) lists:
  `POST /process.Process/StreamInput` and `POST /filesystem.Filesystem/WatchDir`
  get `404` from the gateway (the crate reports a Connect error without a body
  as `NotFound, "HTTP 404"`). For CLM-0046 and 0049 the 404 comes back when the
  input stream is closed, because a client stream's answer is only read then.

### New sandbawks defects, for issues on gfd/sandbawks

Seen through the crate on 2026-10-27, each in at least two rounds except N5.
N5's error appeared once (R2; earlier attempts were capacity-masked), but it
follows from the create answer lacking `envdAccessToken`, which M1's probes
saw on every sandbox too. No direct
HTTP probes were made in this run; the requests are the ones the crate sends,
as described in `docs/protocol-notes.md` with its SDK citations (e2b 2.61.0).
envd requests go to `https://49983-<id>.sb-ghlw.gfd.dev` with the
`E2b-Sandbox-Id` and `E2b-Sandbox-Port` routing headers. sandbawks returned no
`envdAccessToken`, so none was sent.

**N1. A multi-file multipart upload without a `path` query is refused** (CLM-0062).
- Client call: `files().write_files(&[WriteEntry::new("/tmp/batch/a.txt", "alpha"), WriteEntry::new("/tmp/batch/sub/b.bin", <bytes 0..=255>)], &WriteOptions::default())`.
- Request: one `POST /files` with no `path` query, `multipart/form-data` with
  two parts, both named `file`, with filenames `/tmp/batch/a.txt` and
  `/tmp/batch/sub/b.bin`. This is the SDK's upload for envd older than 0.5.7: the
  `path` query is only sent for exactly one file
  (`py: sandbox_async/filesystem/filesystem.py:369-384, 445-462`). The crate
  takes it because sandbawks reports envd 0.1.28
  ([sandbawks#675](https://git.gfpd.us/gfd/sandbawks/issues/675)); on hosted
  envd 0.10.0 it sends one octet-stream `POST /files?path=...` per file instead.
- Response: ``400 {"message": "Failed to deserialize query string: missing field `path`"}``
  (axum's `Query` extractor message), in the run of record and R1.
- Expected: envd writes every part to the path in its filename and answers a
  JSON array with one entry per file. Even once #675 is fixed, a client that
  asks for multipart batch uploads will send this request.

**N2. Uploads are capped by an nginx body limit** (CLM-0066).
- Client call: `files().write_stream("/tmp/plain.bin", <64 MiB of 'z' from tokio::io::repeat>, &WriteOptions::default())`.
- Request: `POST /files?path=/tmp/plain.bin`, `multipart/form-data` with one
  streamed `file` part (filename `/tmp/plain.bin`), sent chunked. It is
  multipart, not octet-stream, because envd 0.1.28 is below 0.5.7
  ([sandbawks#675](https://git.gfpd.us/gfd/sandbawks/issues/675)).
- Response: `413` with nginx's HTML body (`<title>413 Request Entity Too
  Large</title>`, `<center>nginx</center>`), in the run of record and R1. The
  limit sits in front of the gateway: the page is nginx's, not a gateway JSON
  error. The size where it starts was not measured. ingress-nginx's default
  `proxy-body-size` is 1 MiB, which would also break much smaller uploads.
- Expected: envd accepts large uploads; hosted took both 64 MiB uploads
  (plain and gzip) in this suite. A deliberate cap should answer with an
  E2B-shaped error, not an HTML page.

**N3. Downloading an 8 MiB file fails with 500** (CLM-0128).
- Client call: `commands().run("yes 0123456789abcde | head -c 8388608 > /tmp/big.txt")`
  (exit code 0), then `files().read_stream("/tmp/big.txt", None)`.
- Request: `GET /files?path=/tmp/big.txt`.
- Response: `500 {"message": "an internal error occurred"}`, before any byte
  of the body, in R2 and R3 (both on a ready sandbox: the `yes` command had
  just succeeded). The run of record and R1 were capacity-masked.
- Expected: `200` with the 8 MiB body, streamed. CLM-0050 and 0051 read small
  files over the same route and pass, so the failure looks size-related.

**N4. `Stat` reports no owner or group, and permissions without the file-type character** (CLM-0056, CLM-0061).
- Client call: `files().write("/tmp/sized.txt", "12345", None)`, then `files().get_info("/tmp/sized.txt", None)`.
- Request: `POST /filesystem.Filesystem/Stat {"path":"/tmp/sized.txt"}`.
- Response (as decoded by the crate): `EntryInfo { name: "sized.txt", file_type: File, path: "/tmp/sized.txt", size: 5, mode: 420, permissions: "rw-r--r--", owner: "", group: "", ... }`,
  in R2 and R3. In CLM-0061 a file written as `root` also has owner `""`.
- Expected: `permissions` in `ls -l` form with the type character
  (`-rw-r--r--`; hosted passes the `starts_with("-rw")` check), and `owner` and
  `group` set to the file's user and group names (`EntryInfo` fields `owner`,
  `group`: `docs/protocol-notes.md`, section "Filesystem service and file API",
  from `py: envd/filesystem/filesystem_pb.py`). CLM-0061 would still fail on
  [sandbawks#682](https://git.gfpd.us/gfd/sandbawks/issues/682) and the
  template's missing `user` account once owners are reported.

**N5. Sandboxes are not secured: no `envdAccessToken`, and envd's file API answers without credentials** (CLM-0065; also explains the CLM-0064 pass).
- Client call: `create_sandbox(...)`, then `download_url("/tmp/shared.txt", &FileUrlOptions::default().expires_in(60 s))`.
- Response: the create answer has no `envdAccessToken`, so the crate has
  nothing to sign with and refuses an expiring URL, as the SDK does:
  `InvalidArgument("a URL expiration needs a secured sandbox (one with an envd access token)")` (R1 was capacity-masked, R2 shows this).
  CLM-0064 passed in R2 with an unsigned upload URL: a plain `reqwest` client
  with no API key, no `X-Access-Token` and no routing headers sent
  `POST https://49983-<id>.sb-ghlw.gfd.dev/files?path=/tmp/up.txt` and the file
  was written. So anyone who knows a sandbox id can read and write its files.
  The ids look partly time-ordered (`sbx-649...`, `sbx-65e...` within minutes).
- Expected: every sandbox is secured (`secure` is deprecated and ignored on
  hosted, so every hosted sandbox has an envd access token:
  `py: sandbox_async/main.py:177, 197`). envd requires `X-Access-Token` or a
  valid `signature` on `/files` (`py: sandbox_async/main.py:1229-1232`;
  `e2b/sandbox/signature.py:15-47`), and a URL with its `signature` removed is
  refused (hosted passes that part of CLM-0065).
