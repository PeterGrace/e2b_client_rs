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

## sandbawks (run 20261026T225651Z-sandbawks-1078537, 2026-10-26T22:56:51Z)

sandbawks 0.1.28 at `sb-ghlw.gfd.dev`, tested from `8cd5040` plus the two harness
fixes in this PR. Hosted E2B is the canonical server. A test that passes on
hosted E2B and fails here is a sandbawks defect.

There were four runs. Every log is checked in:

| Run id | What it was | Log |
|---|---|---|
| `20261026T225357Z-sandbawks-998331` | `just test-live sandbawks`, template `base` | [log](runs/20261026T225357Z-sandbawks-998331.log) |
| `20261026T225651Z-sandbawks-1078537` | `just test-live sandbawks` with `E2B_TEMPLATE_SANDBAWKS=compound-sandbox` (**the run of record**) | [log](runs/20261026T225651Z-sandbawks-1078537.log) |
| `20261026T230103Z-sandbawks-rerun-1207087` | each of the 29 failures from the run of record rerun on its own, one at a time, then the sweep | [log](runs/20261026T230103Z-sandbawks-rerun-1207087.log) |
| `20261026T233507Z-sandbawks-2244779` | second full `just test-live sandbawks` (coordinator, with `E2B_TEMPLATE_SANDBAWKS` in `.env`); see "Second full run" below | [log](runs/20261026T233507Z-sandbawks-2244779.log) |

**Why there are four runs.**

- **The first run tested nothing.** sandbawks has no `base` template. Its only
  `SandboxTemplate` is `compound-sandbox`. Create still answered 201 with state
  `running` (see S1), so every command test failed against a sandbox that
  would never start. Only `live_commands` ran. `just test-live` had no
  `--no-fail-fast`, so cargo stopped at the first failing test binary and
  `live_files` and `live_lifecycle` never ran (H1). The same run's sweep also
  killed three of pgrace's warm-pool sandboxes (H2, S3).
- **The run of record** is run 2, made after the H1 and H2 fixes, with the
  template that exists.
- **The reruns** separate capacity failures from real ones. In the run of
  record, four tests at a time drained the two-sandbox warm pool. The cold
  creates that followed were unschedulable (`Insufficient memory` on 17 of 25
  nodes), but create had already answered `running` (S2). Run alone, a test
  usually got a warm sandbox.
- **The second full run** was made by the coordinator after
  `E2B_TEMPLATE_SANDBAWKS=compound-sandbox` was added to `.env`. It has its
  own subsection below. The main table is still the run of record and the
  reruns.

**The Hosted column** is from run `20261026T225333Z-hosted-980838` (PR #22,
issue #10), in which all 46 tests passed. The per-PR hosted runs of #6, #7 and
#8 had passed them too.

**Classes:** **pass**; **sandbawks** (a sandbawks defect, S-number below);
**env** (the cluster or template, not sandbawks code: inconclusive);
**crate** (crate bug); **test** (test bug). This run found no crate bugs and no
test bugs. It found two harness bugs, which are fixed here (H1, H2).

| Test | Claim | Run of record | Rerun | Hosted | Class | Failure (first line, last attempt) |
|---|---|---|---|---|---|---|
| clm_0007_create_returns_running_sandbox | CLM-0007 | pass | | pass | pass | |
| clm_0008_create_metadata_is_visible_in_info | CLM-0008 | pass | | pass | pass | |
| clm_0009_create_env_vars_reach_commands | CLM-0009 | fail | fail | pass | sandbawks S6 ([sandbawks#681](https://git.gfpd.us/gfd/sandbawks/issues/681)) | `left: "\n"` `right: "hi from create\n"` |
| clm_0010_create_timeout_sets_end_at | CLM-0010 | pass | | pass | pass | |
| clm_0011_auto_pause_pauses_at_timeout | CLM-0011 | fail | fail | pass | sandbawks S11 ([sandbawks#679](https://git.gfpd.us/gfd/sandbawks/issues/679), [sandbawks#665](https://git.gfpd.us/gfd/sandbawks/issues/665)) | `timed out after 120s waiting for auto-pause at timeout` |
| clm_0012_internet_access_false_blocks_outbound | CLM-0012 | fail | pass | pass | pass (run of record: env S2) | |
| clm_0013_connect_running_sandbox_runs_commands | CLM-0013 | fail | pass | pass | pass (run of record: env S2) | |
| clm_0014_connect_resumes_paused_sandbox_with_files | CLM-0014 | fail | pass | pass | pass, vacuous (see notes) | |
| clm_0015_connect_unknown_sandbox_is_not_found | CLM-0015 | pass | | pass | pass | |
| clm_0016_get_info_returns_detail | CLM-0016 | pass | | pass | pass | |
| clm_0017_get_info_unknown_sandbox_is_not_found | CLM-0017 | pass | | pass | pass | |
| clm_0018_list_filters_by_metadata | CLM-0018 | fail | fail | pass | sandbawks S3 ([sandbawks#676](https://git.gfpd.us/gfd/sandbawks/issues/676)) | `left: [<own>, "sbx-compound-pool-k52zk", ...]` `right: [<own>]` |
| clm_0019_list_filters_by_state | CLM-0019 | fail | fail | pass | sandbawks S3 ([sandbawks#676](https://git.gfpd.us/gfd/sandbawks/issues/676)) | `left: [4 sandboxes]` `right: ["sbx-36eb867de991"]` |
| clm_0020_list_pages_without_duplicates_or_omissions | CLM-0020 | fail | fail | pass | sandbawks S3 ([sandbawks#676](https://git.gfpd.us/gfd/sandbawks/issues/676)) | page union holds 2 pool sandboxes besides the 3 of the test |
| clm_0021_kill_running_sandbox | CLM-0021 | pass | | pass | pass | |
| clm_0022_kill_unknown_sandbox_is_false | CLM-0022 | pass | | pass | pass | |
| clm_0023_pause_running_sandbox | CLM-0023 | fail | fail | pass | sandbawks S11 ([sandbawks#679](https://git.gfpd.us/gfd/sandbawks/issues/679), [sandbawks#665](https://git.gfpd.us/gfd/sandbawks/issues/665)) | `left: Running` `right: Paused` |
| clm_0024_pause_paused_sandbox_is_false | CLM-0024 | fail | fail | pass | sandbawks S11 ([sandbawks#679](https://git.gfpd.us/gfd/sandbawks/issues/679), [sandbawks#665](https://git.gfpd.us/gfd/sandbawks/issues/665)) | `assertion failed: !second` |
| clm_0025_set_timeout_moves_end_at | CLM-0025 | pass | | pass | pass | |
| clm_0026_refresh_extends_end_at | CLM-0026 | pass | | pass | pass | |
| clm_0027_is_running_tracks_kill | CLM-0027 | fail | fail | pass | sandbawks S12 ([sandbawks#679](https://git.gfpd.us/gfd/sandbawks/issues/679)) | `want Ok(false) after kill, got Err(Envd { status: 404, ... not found })` |
| clm_0033_run_returns_output_and_exit_code | CLM-0033 | pass | | pass | pass | |
| clm_0034_start_streams_chunks_then_exit | CLM-0034 | pass | | pass | pass | |
| clm_0035_run_delivers_large_output_in_order | CLM-0035 | pass | | pass | pass | |
| clm_0036_options_env_is_set | CLM-0036 | fail | pass | pass | pass (run of record: env S2) | |
| clm_0037_options_cwd_is_working_directory | CLM-0037 | fail | pass | pass | pass (run of record: env S2) | |
| clm_0038_options_user_runs_as_user | CLM-0038 | fail | fail | pass | sandbawks S5 (and env: no `user` account) | `left: "dev\n"` `right: "root\n"` |
| clm_0039_options_timeout_ends_long_command | CLM-0039 | pass | | pass | pass | |
| clm_0040_connect_attaches_to_running_command | CLM-0040 | fail | fail | pass | sandbawks S7 ([sandbawks#677](https://git.gfpd.us/gfd/sandbawks/issues/677)) | `connect: Protocol("Connect stream did not open with a start event")` |
| clm_0041_list_includes_background_command | CLM-0041 | fail | pass | pass | pass (run of record: env S2) | |
| clm_0042_send_stdin_reaches_command | CLM-0042 | fail | fail | pass | env S2 ([sandbawks#674](https://git.gfpd.us/gfd/sandbawks/issues/674), inconclusive) | `send_stdin: Rpc { code: Unavailable, message: "sandbox is still starting up (Pod is Pending (Unschedulable ...` |
| clm_0043_close_stdin_sends_eof | CLM-0043 | fail | fail | pass | sandbawks S4 ([sandbawks#675](https://git.gfpd.us/gfd/sandbawks/issues/675)) | run of record: `close_stdin: InvalidArgument("close_stdin needs envd 0.5.2 or newer; this sandbox runs envd 0.1.28")` |
| clm_0044_sigterm_terminates_command | CLM-0044 | fail | fail | pass | sandbawks S8 ([sandbawks#677](https://git.gfpd.us/gfd/sandbawks/issues/677)) | `pid 10 still listed after SIGTERM: [ProcessInfo { pid: 10, ... "sleep 30" ... }]` |
| clm_0045_kill_reports_whether_pid_existed | CLM-0045 | pass | | pass | pass | |
| clm_0050_write_read_round_trips_binary | CLM-0050 | pass | | pass | pass | |
| clm_0051_read_to_string_returns_text | CLM-0051 | fail | pass | pass | pass (run of record: env S2) | |
| clm_0052_write_returns_write_info | CLM-0052 | fail | fail | pass | env S2 ([sandbawks#674](https://git.gfpd.us/gfd/sandbawks/issues/674), inconclusive) | `write: Envd { status: 503, message: "sandbox is still starting up ...` |
| clm_0053_read_missing_file_is_not_found | CLM-0053 | pass | | pass | pass | |
| clm_0054_list_depth_one_lists_direct_entries | CLM-0054 | fail | fail | pass | sandbawks S9 ([sandbawks#678](https://git.gfpd.us/gfd/sandbawks/issues/678)) | `write a: Envd { status: 404, message: "No such file or directory (os error 2)" }` |
| clm_0055_list_depth_two_recurses | CLM-0055 | fail | fail | pass | sandbawks S9 ([sandbawks#678](https://git.gfpd.us/gfd/sandbawks/issues/678)) | `write a: Envd { status: 404, message: "No such file or directory (os error 2)" }` |
| clm_0056_get_info_reports_metadata | CLM-0056 | fail | fail | pass | env S2 ([sandbawks#674](https://git.gfpd.us/gfd/sandbawks/issues/674), inconclusive) | `write: Envd { status: 503, message: "sandbox is still starting up ...` |
| clm_0057_exists_true_and_false | CLM-0057 | fail | fail | pass | env S2 ([sandbawks#674](https://git.gfpd.us/gfd/sandbawks/issues/674), inconclusive) | `write: Envd { status: 503, message: "sandbox is still starting up (Pod is Pending (ContainerCreating)) ...` |
| clm_0058_make_dir_reports_creation | CLM-0058 | fail | fail | pass | sandbawks S10 ([sandbawks#678](https://git.gfpd.us/gfd/sandbawks/issues/678)) | `make_dir of an existing directory returned true` |
| clm_0059_rename_moves_entry | CLM-0059 | fail | pass | pass | pass (run of record: env S2) | |
| clm_0060_remove_deletes_file_and_directory | CLM-0060 | fail | fail | pass | sandbawks S9 ([sandbawks#678](https://git.gfpd.us/gfd/sandbawks/issues/678)) | `write file: Envd { status: 404, message: "No such file or directory (os error 2)" }` |
| clm_0061_user_selects_file_owner | CLM-0061 | fail | fail | pass | env S2 ([sandbawks#674](https://git.gfpd.us/gfd/sandbawks/issues/674), inconclusive; S5 and the missing `user` account would also fail it) | `write as root: Envd { status: 503, message: "sandbox is still starting up ...` |

**Totals.** 46 `clm_*` tests: lifecycle 21, commands 13, files 12. In the run
of record, 17 passed and 29 failed (lifecycle 10/21, commands 5/13, files
2/12). After the reruns, 25 pass and 21 fail. Of the 21 failures, 16 are
sandbawks defects, which hosted E2B passes, and 5 are inconclusive because the
sandbox never became ready (env, S2). None of the 21 fails on hosted E2B.

**Sweep.** In run 1, `sweep: killed 8 leftover sandbox(es) of run
20261026T225357Z-sandbawks-998331`. That count includes sandboxes the run did
not own (H2). In the run of record, `killed 0 leftover sandbox(es) of run
20261026T225651Z-sandbawks-1078537`. In the rerun, `killed 0 leftover
sandbox(es) of run 20261026T230103Z-sandbawks-rerun-1207087`. Since the H2 fix,
the sweep counts only sandboxes carrying this run's id. No log has a
`could not kill` warning. `kubectl -n sandbawks get sbx` after the runs and
probes showed only the two warm-pool sandboxes.

**Secret check** (plan Task 6 Step 3, on each log before it was copied here;
both `.env` values were confirmed unquoted without printing them):
`E2B_API_KEY_HOSTED: 0` and `E2B_API_KEY_SANDBAWKS: 0` for each of the run of record, run 1 and the rerun logs. The fourth log is
checked in its own subsection below.

### Second full run (run 20261026T233507Z-sandbawks-2244779, 2026-10-26T23:35:07Z)

The coordinator ran `just test-live sandbawks` from this branch (with H1 and
H2) after `E2B_TEMPLATE_SANDBAWKS=compound-sandbox` was added to `.env`. Log:
[`runs/20261026T233507Z-sandbawks-2244779.log`](runs/20261026T233507Z-sandbawks-2244779.log).
22 passed and 24 failed (lifecycle 12/21, commands 6/13, files 4/12). Sweep:
`killed 0 leftover sandbox(es) of run 20261026T233507Z-sandbawks-2244779`.
Secret check on the log before copying: `E2B_API_KEY_HOSTED: 0`,
`E2B_API_KEY_SANDBAWKS: 0` (values confirmed unquoted).

12 failures are capacity-masked: the sandbox answered "still starting up"
([sandbawks#674](https://git.gfpd.us/gfd/sandbawks/issues/674)), so these tests say nothing about the feature itself. Three more
(CLM-0027, 0038, 0040) failed with an empty or false result consistent with
the same cause; the single-test reruns above show the defect each of them
hits once the sandbox is ready. Every failing test passed on hosted E2B.

| Test | Claim | Result | gfd/sandbawks issue |
|---|---|---|---|
| clm_0007_create_returns_running_sandbox | CLM-0007 | pass |  |
| clm_0008_create_metadata_is_visible_in_info | CLM-0008 | pass |  |
| clm_0009_create_env_vars_reach_commands | CLM-0009 | fail | [sandbawks#681](https://git.gfpd.us/gfd/sandbawks/issues/681) |
| clm_0010_create_timeout_sets_end_at | CLM-0010 | pass |  |
| clm_0011_auto_pause_pauses_at_timeout | CLM-0011 | fail | [sandbawks#665](https://git.gfpd.us/gfd/sandbawks/issues/665) |
| clm_0012_internet_access_false_blocks_outbound | CLM-0012 | pass |  |
| clm_0013_connect_running_sandbox_runs_commands | CLM-0013 | pass |  |
| clm_0014_connect_resumes_paused_sandbox_with_files | CLM-0014 | fail | capacity-masked ([sandbawks#674](https://git.gfpd.us/gfd/sandbawks/issues/674)) |
| clm_0015_connect_unknown_sandbox_is_not_found | CLM-0015 | pass |  |
| clm_0016_get_info_returns_detail | CLM-0016 | pass |  |
| clm_0017_get_info_unknown_sandbox_is_not_found | CLM-0017 | pass |  |
| clm_0018_list_filters_by_metadata | CLM-0018 | fail | [sandbawks#676](https://git.gfpd.us/gfd/sandbawks/issues/676) |
| clm_0019_list_filters_by_state | CLM-0019 | fail | [sandbawks#676](https://git.gfpd.us/gfd/sandbawks/issues/676) |
| clm_0020_list_pages_without_duplicates_or_omissions | CLM-0020 | fail | [sandbawks#676](https://git.gfpd.us/gfd/sandbawks/issues/676) |
| clm_0021_kill_running_sandbox | CLM-0021 | pass |  |
| clm_0022_kill_unknown_sandbox_is_false | CLM-0022 | pass |  |
| clm_0023_pause_running_sandbox | CLM-0023 | fail | [sandbawks#679](https://git.gfpd.us/gfd/sandbawks/issues/679) |
| clm_0024_pause_paused_sandbox_is_false | CLM-0024 | fail | [sandbawks#679](https://git.gfpd.us/gfd/sandbawks/issues/679) |
| clm_0025_set_timeout_moves_end_at | CLM-0025 | pass |  |
| clm_0026_refresh_extends_end_at | CLM-0026 | pass |  |
| clm_0027_is_running_tracks_kill | CLM-0027 | fail | probably capacity-masked ([sandbawks#674](https://git.gfpd.us/gfd/sandbawks/issues/674)): `is_running` false while alive; once ready it fails on [sandbawks#679](https://git.gfpd.us/gfd/sandbawks/issues/679) (rerun above) |
| clm_0033_run_returns_output_and_exit_code | CLM-0033 | pass |  |
| clm_0034_start_streams_chunks_then_exit | CLM-0034 | pass |  |
| clm_0035_run_delivers_large_output_in_order | CLM-0035 | pass |  |
| clm_0036_options_env_is_set | CLM-0036 | pass |  |
| clm_0037_options_cwd_is_working_directory | CLM-0037 | pass |  |
| clm_0038_options_user_runs_as_user | CLM-0038 | fail | probably capacity-masked ([sandbawks#674](https://git.gfpd.us/gfd/sandbawks/issues/674)): empty stdout; once ready it fails on [sandbawks#682](https://git.gfpd.us/gfd/sandbawks/issues/682) (rerun above) |
| clm_0039_options_timeout_ends_long_command | CLM-0039 | fail | capacity-masked ([sandbawks#674](https://git.gfpd.us/gfd/sandbawks/issues/674)) |
| clm_0040_connect_attaches_to_running_command | CLM-0040 | fail | probably capacity-masked ([sandbawks#674](https://git.gfpd.us/gfd/sandbawks/issues/674)): empty stdout; once ready it fails on [sandbawks#677](https://git.gfpd.us/gfd/sandbawks/issues/677) (rerun above) |
| clm_0041_list_includes_background_command | CLM-0041 | pass |  |
| clm_0042_send_stdin_reaches_command | CLM-0042 | fail | capacity-masked ([sandbawks#674](https://git.gfpd.us/gfd/sandbawks/issues/674)) |
| clm_0043_close_stdin_sends_eof | CLM-0043 | fail | [sandbawks#677](https://git.gfpd.us/gfd/sandbawks/issues/677): `send_stdin: Rpc { code: NotFound, message: "no process with pid 0" }`, the start event of a `stdin: true` command carries pid 0 (second time, so not a one-off); [sandbawks#675](https://git.gfpd.us/gfd/sandbawks/issues/675) also blocks `close_stdin` |
| clm_0044_sigterm_terminates_command | CLM-0044 | fail | capacity-masked ([sandbawks#674](https://git.gfpd.us/gfd/sandbawks/issues/674)) |
| clm_0045_kill_reports_whether_pid_existed | CLM-0045 | fail | capacity-masked ([sandbawks#674](https://git.gfpd.us/gfd/sandbawks/issues/674)) |
| clm_0050_write_read_round_trips_binary | CLM-0050 | pass |  |
| clm_0051_read_to_string_returns_text | CLM-0051 | pass |  |
| clm_0052_write_returns_write_info | CLM-0052 | pass |  |
| clm_0053_read_missing_file_is_not_found | CLM-0053 | pass |  |
| clm_0054_list_depth_one_lists_direct_entries | CLM-0054 | fail | capacity-masked ([sandbawks#674](https://git.gfpd.us/gfd/sandbawks/issues/674)) |
| clm_0055_list_depth_two_recurses | CLM-0055 | fail | [sandbawks#678](https://git.gfpd.us/gfd/sandbawks/issues/678) |
| clm_0056_get_info_reports_metadata | CLM-0056 | fail | capacity-masked ([sandbawks#674](https://git.gfpd.us/gfd/sandbawks/issues/674)) |
| clm_0057_exists_true_and_false | CLM-0057 | fail | capacity-masked ([sandbawks#674](https://git.gfpd.us/gfd/sandbawks/issues/674)) |
| clm_0058_make_dir_reports_creation | CLM-0058 | fail | capacity-masked ([sandbawks#674](https://git.gfpd.us/gfd/sandbawks/issues/674)) |
| clm_0059_rename_moves_entry | CLM-0059 | fail | capacity-masked ([sandbawks#674](https://git.gfpd.us/gfd/sandbawks/issues/674)) |
| clm_0060_remove_deletes_file_and_directory | CLM-0060 | fail | capacity-masked ([sandbawks#674](https://git.gfpd.us/gfd/sandbawks/issues/674)) |
| clm_0061_user_selects_file_owner | CLM-0061 | fail | capacity-masked ([sandbawks#674](https://git.gfpd.us/gfd/sandbawks/issues/674)) |

### Harness bugs fixed in this PR

These are wrong for hosted E2B too, so they are fixed here.

- **H1. `just test-live` stopped at the first failing binary.** cargo runs the
  test binaries in turn and, without `--no-fail-fast`, skips the remaining ones
  once one fails. On any server, a single failure in `live_commands` would
  hide every `live_files` and `live_lifecycle` result. The recipe now passes
  `--no-fail-fast`.
- **H2. The sweep trusted the server's list filter.** It killed every sandbox
  `GET /v2/sandboxes?metadata=e2b-client-test%3D<run id>&state=running,paused`
  returned. sandbawks ignores the filter (S3), so in run 1 the sweep killed
  pgrace's `compound-pool` warm-pool sandboxes `sbx-compound-pool-ml7ls`,
  `-f8vpg` and `-j9cvx` (kubelet `Killing` events at 22:54:15Z, when the sweep
  ran). The warm pool replaced them within about two minutes. The sweep now also
  checks `metadata["e2b-client-test"] == run id` on its own side before it
  kills. On hosted, which honours the filter, it behaves as before.
  The check still finds this run's leftovers on sandbawks, because its list
  entries carry each sandbox's metadata. A probe at 2026-10-27T00:16:31Z
  created a sandbox tagged `e2b-client-test=probe-issue-11-listmeta`, and
  `GET /v2/sandboxes?metadata=...&state=running,paused` returned it with
  `"metadata":{"e2b-client-test":"probe-issue-11-listmeta"}`, beside two
  untagged pool sandboxes with `{}`. The sandbox was then deleted (204).

### Environment findings (not sandbawks code)

- **No `base` template.** The deployment has one `SandboxTemplate`,
  `compound-sandbox`. The plan's default template `base` doesn't exist there,
  so a sandbawks run needs `E2B_TEMPLATE_SANDBAWKS=compound-sandbox` (the
  harness already reads this variable). Consider adding it to `.env`.
- **`compound-sandbox` has no `user` account.** Its users are `root` and `dev`
  (uid 1000), and the default user is `dev`. CLM-0038 and CLM-0061 run commands
  and write files as `user`, the hosted `base` default. They would fail on this
  template even with S5 fixed.
- **Capacity.** A `compound-sandbox` asks for 2 CPU, 2 GiB and a 40 GiB disk.
  The warm pool `compound-pool` keeps 2 ready (max 5). Cold creates beyond the
  pool went unschedulable: `0/25 nodes are available: 17 Insufficient memory,
  ... 6 Insufficient ephemeral-storage, 7 Insufficient cpu`. That is why a
  four-thread run fails tests that pass alone.

### sandbawks defects, for issues on gfd/sandbawks

Each was observed through the crate and confirmed with a direct HTTP probe of
sandbawks 0.1.28 on 2026-10-26, between 22:54Z and 23:10Z. The probes are
quoted below. Probe requests carried `X-API-Key` (value omitted). envd
requests carried `E2b-Sandbox-Id: <id>`, `E2b-Sandbox-Port: 49983` and
`Connect-Protocol-Version: 1` and went to
`https://49983-<id>.sb-ghlw.gfd.dev`. No `envdAccessToken` was returned, so
none was sent. "SDK" citations are to `docs/protocol-notes.md` sources (e2b
2.61.0).

**S1. Create accepts an unknown template and reports the doomed sandbox as running.** Filed: [sandbawks#680](https://git.gfpd.us/gfd/sandbawks/issues/680).
- Client call: `Client::create_sandbox(CreateSandboxParams::new("base"))`.
- Request: `POST /sandboxes {"templateID":"no-such-template-issue-11","timeout":30,"metadata":{...}}`.
- Response: `201 {"sandboxID":"sbx-370bee272196","templateID":"no-such-template-issue-11",...,"envdVersion":"0.1.28","state":"running"}`.
  `GET /sandboxes/sbx-370bee272196` then answers `200 ... "state":"running"`.
  In run 1, with `"templateID":"base"`, the controller logged
  `action_failure: SandboxTemplate 'base' not found` for each sandbox.
  envd calls get `412 {"code":"sandbox_failed","message":"sandbox will not become ready without intervention: the sandbox failed (action_failure)"}`.
  Process Start through the gateway yields a synthetic result: stderr
  `ERROR: sandbox will not become ready ...`, exit code 69.
- Expected: create fails with an error status when the template doesn't
  exist, and a failed sandbox is never reported as `running`. The SDK treats any
  status other than 201 as an error (`py: api/client/api/sandboxes/post_v2_sandboxes.py:39-42`).
  State values are `running` and `paused` only (`py: api/client/models/sandbox_state.py:5-6`).

**S2. Create returns `running` before the sandbox can serve envd.** Filed: [sandbawks#674](https://git.gfpd.us/gfd/sandbawks/issues/674).
- Client call: any `create_sandbox` that misses the warm pool, then any
  `commands()` or `files()` call.
- Request: e.g. `POST /files?path=/tmp/x` or `POST /process.Process/SendInput` right after create.
- Response: `503` or Connect `unavailable`, message
  `sandbox is still starting up (Pod is Pending (Unschedulable: 0/25 nodes are available: 17 Insufficient memory, ...)); retry shortly [EAGAIN: resource temporarily unavailable]`.
  A process Start in that state gets a synthetic stream: stderr
  `NOTICE: sandbox is still starting up ...`, exit code 75. The start event
  apparently carries pid 0: the rerun's CLM-0043 then got
  `send_stdin: Rpc { code: NotFound, message: "no process with pid 0" }`.
  The pid 0 recurred in the second full run and is filed with S7 under
  [sandbawks#677](https://git.gfpd.us/gfd/sandbawks/issues/677).
  The REST state stays `running` throughout.
- Expected: when create returns 201, the sandbox is usable. Hosted E2B
  behaves this way: all 46 tests passed there with no readiness wait. The
  crate, like the SDK, runs commands straight after create. If capacity is
  short, create should block until the sandbox is ready or fail with an error
  status. It should not return a sandbox that envd cannot serve.
  Exit code 75 and the fake pid 0 look like real process results to clients.

**S3. `GET /v2/sandboxes` ignores the `metadata` and `state` filters.** Filed: [sandbawks#676](https://git.gfpd.us/gfd/sandbawks/issues/676).
- Client call: `list_sandboxes(&ListSandboxesParams::new().metadata(k, v).state(SandboxState::Paused))`.
- Requests and responses (identical bodies each time, 2 sandboxes, both
  `"metadata":{}`, `"state":"running"`):
  - `GET /v2/sandboxes`
  - `GET /v2/sandboxes?metadata=nokey%3Dnovalue`
  - `GET /v2/sandboxes?state=paused`
  - `GET /v2/sandboxes?metadata=e2b-client-test%3D20261026T225357Z-sandbawks-998331`
  - Each answered `200 [{"sandboxID":"sbx-compound-pool-dsxnz",...,"metadata":{}},{"sandboxID":"sbx-compound-pool-g6wx7",...}]`.
- Expected: only sandboxes whose metadata contains every pair, and only the
  listed states. `metadata` is the form-encoded `k=v` string
  (`py: sandbox_async/paginator.py:50-57`; `js: 5656-5658`). `state` is
  comma-joined (`py: api/client/api/sandboxes/get_v2_sandboxes.py:31-38`).
  Warm-pool sandboxes, which no client has claimed, also show up in an
  unfiltered list. Their `endAt` equals `startedAt`.
- Impact: any client that cleans up by metadata kills other tenants'
  sandboxes. This happened in run 1 (H2).

**S4. `envdVersion` is sandbawks' release version, not an envd protocol version.** Filed: [sandbawks#675](https://git.gfpd.us/gfd/sandbawks/issues/675).
- Client call: `commands().close_stdin(pid)`.
- Response: create and info carry `"envdVersion":"0.1.28"`. The crate follows
  the SDK and refuses `close_stdin` below envd 0.5.2:
  `InvalidArgument("close_stdin needs envd 0.5.2 or newer; this sandbox runs envd 0.1.28")`.
  sandbawks' envd does implement it: `POST /process.Process/CloseStdin {"process":{"pid":14}}`
  answered `200 {}`, and `wc -c` then printed `3`.
- Expected: `envdVersion` is the version of the E2B envd protocol the sandbox
  speaks. SDK clients gate features on it (`close_stdin` ≥ 0.5.2:
  `py: sandbox_async/commands/command.py:189-193`, `py: envd/versions.py:7`;
  the default user below 0.4.0: `py: envd/utils.py:44-57`). Every official
  SDK gates `close_stdin` off on sandbawks today.

**S5. The acting user (`Authorization: Basic`) is ignored by process Start.** Filed: [sandbawks#682](https://git.gfpd.us/gfd/sandbawks/issues/682).
- Client call: `commands().run("whoami", &CommandOptions::default().user("root"))`.
- Request: `POST /process.Process/Start` with `Authorization: Basic base64("root:")`
  and body `{"process":{"cmd":"/bin/bash","args":["-l","-c","whoami"],"envs":{}}}`.
- Response: `200` stream, `start {pid:13}`, stdout `dev\n`, `end {exitCode:0}`.
  The result was the same with `Basic base64("user:")`.
- Expected: the command runs as the named user (`py: envd/utils.py:44-57`;
  `js: 1737`).

**S6. Create `envVars` don't reach commands.** Filed: [sandbawks#681](https://git.gfpd.us/gfd/sandbawks/issues/681).
- Client call: `create_sandbox(&params.env("GREETING", "hi from create"))`, then `run("echo \"$GREETING\"")`.
- Request: `POST /sandboxes {"templateID":"compound-sandbox",...,"envVars":{"GREETING":"hi from create"}}` → 201.
  Then `POST /process.Process/Start` with `{"process":{"cmd":"/bin/bash","args":["-l","-c","echo \"$GREETING\""],"envs":{}}}`.
- Response: stdout `\n` (the variable is unset).
- Expected: sandbox-level `envVars` are in every process's environment
  (`py: api/client/models/new_sandbox_v2.py:112-132`, field `envVars`).

**S7. A `Connect` stream doesn't open with a `start` event.** Filed: [sandbawks#677](https://git.gfpd.us/gfd/sandbawks/issues/677).
- Client call: `commands().connect(pid, None)` on a running command.
- Request: `POST /process.Process/Connect` (`application/connect+json`), body `{"process":{"pid":10}}`.
- Response: `200` frames `data {stdout:"bGF0ZQo="}`, `end {exitCode:0,...}`, end-of-stream `{}`. There is no `start` frame.
- Expected: the first event is `start {pid}`, as on Start. The SDK waits for
  that first event and requires it to be `start`
  (`py: sandbox_async/commands/command.py:382-411`, `py: envd/utils.py:27-41`).

**S8. `Process/List` keeps processes that have exited.** Filed: [sandbawks#677](https://git.gfpd.us/gfd/sandbawks/issues/677).
- Client call: `send_signal(pid, Signal::Sigterm)`, `wait()`, then `list()`.
- Request: SendSignal `{"process":{"pid":16},"signal":"SIGNAL_SIGTERM"}` → `200 {}`.
  The stream ends `end {exitCode:-1, exited:false, status:"terminated by signal 15"}`.
  Then `POST /process.Process/List {}`.
- Response: `200 {"processes":[{"config":{...,"args":["-l","-c","sleep 30"]},"pid":16},{...pid 10 (exited)...},{...pid 14 (exited)...}]}`.
- Expected: List returns running processes only. Hosted envd drops a pid once
  its process has ended (CLM-0044 passed on hosted).

**S9. File upload doesn't create parent directories.** Filed: [sandbawks#678](https://git.gfpd.us/gfd/sandbawks/issues/678).
- Client call: `files().write("/tmp/tree/a.txt", "a", None)`.
- Request: `POST /files?path=/tmp/tree/a.txt`, `multipart/form-data`, field `file`, filename `/tmp/tree/a.txt`.
- Response: `404 {"code":"not_found","message":"No such file or directory (os error 2)"}`.
- Expected: envd creates the missing parent directories and returns the
  written entry (`js: 2226-2249`; `py: sandbox_async/filesystem/filesystem.py:445-462`).
  Hosted does this (CLM-0054, 0055 and 0060 passed on hosted).

**S10. `MakeDir` on an existing directory succeeds again.** Filed: [sandbawks#678](https://git.gfpd.us/gfd/sandbawks/issues/678).
- Client call: `files().make_dir("/tmp/newdir", None)` twice.
- Request: `POST /filesystem.Filesystem/MakeDir {"path":"/tmp/probe-dir"}`, twice.
- Response: both `200 {"entry":{"name":"probe-dir","type":"FILE_TYPE_DIRECTORY",...}}`.
- Expected: the second answers Connect `already_exists`, which the SDK maps
  to `false` (`py: sandbox_async/filesystem/filesystem.py:647-676`).

**S11. Pause is a no-op that reports success. Auto-pause never happens.** Filed: [sandbawks#679](https://git.gfpd.us/gfd/sandbawks/issues/679), [sandbawks#665](https://git.gfpd.us/gfd/sandbawks/issues/665).
- Client call: `pause_sandbox(id)`, then `get_sandbox_info(id)`, then `pause_sandbox(id)` again.
- Request: `POST /sandboxes/sbx-compound-pool-j9gf6/pause {}`.
- Response: `200 {"sandboxID":"sbx-compound-pool-j9gf6","state":"paused"}`.
  Every `GET /sandboxes/{id}` over the next 30 s still answered `"state":"running"`.
  No pause or checkpoint event appeared for the sandbox. A second pause
  again answered `200 {"state":"paused"}`.
  CLM-0011: a sandbox created with `autoPause: true` and a short timeout
  was never seen `paused` within 120 s.
- Expected: after a successful pause the sandbox is `paused`. Pausing a
  paused sandbox answers 409 (`py: sandbox_async/sandbox_api.py:399-432`;
  `js: 5442-5450`). The documented success status is 204. A 200 with a body is
  harmless to the crate.

**S12. `/health` on a killed sandbox answers 404 instead of 502.** Filed: [sandbawks#679](https://git.gfpd.us/gfd/sandbawks/issues/679).
- Client call: `kill()`, then `is_running()`.
- Request: `GET https://49983-sbx-compound-pool-pgwmv.sb-ghlw.gfd.dev/health` after `DELETE /sandboxes/sbx-compound-pool-pgwmv` → 204.
- Response: `404 {"code":"not_found","message":"sandboxes.sandbawks.dev \"sbx-compound-pool-pgwmv\" not found","request_id":"..."}`.
- Expected: `502` for a sandbox that is gone. The SDK maps exactly 502 to
  "not running" (`py: sandbox_async/main.py:134-168`). The message also leaks
  the Kubernetes resource name `sandboxes.sandbawks.dev`.

### Notes

- **CLM-0014 passed vacuously on sandbawks.** The test pauses, connects and
  reads the file back, but doesn't assert that the sandbox was `paused` before
  the connect. With S11, sandbawks never paused, so the test proves nothing
  about resume there. On hosted, pause is synchronous and the test is sound,
  so it was not changed. The coordinator may want a `Paused` assertion after
  the pause.
- **CLM-0015, 0017 and 0022 (unknown ids)** pass. sandbawks answers 404 for
  unknown sandbox ids on REST.
- **CLM-0039 (command timeout), 0045 (kill) and 0012 (internet off)** pass on
  a ready sandbox.
