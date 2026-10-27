//! Live tests for the envd process claims (S2). One test per ledger claim.

mod live_support;

use std::time::Duration;

use e2b_client::{CommandEvent, CommandOptions, ConnectCode, Error, Signal, StreamInputOptions};
use live_support::LiveSandbox;

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0033_run_returns_output_and_exit_code() {
    let sandbox = LiveSandbox::new().await;

    let result = sandbox
        .commands()
        .run("echo out; echo err >&2; exit 3", &CommandOptions::default())
        .await
        .expect("a non-zero exit is not an Err");

    assert_eq!(result.stdout, "out\n");
    assert_eq!(result.stderr, "err\n");
    assert_eq!(result.exit_code, 3);
    assert!(!result.success());
}

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0034_start_streams_chunks_then_exit() {
    let sandbox = LiveSandbox::new().await;
    let mut handle = sandbox
        .commands()
        .start(
            "echo first; sleep 2; echo second >&2",
            &CommandOptions::default(),
        )
        .await
        .expect("start");

    // envd may split or merge output into chunks of any size, so join the
    // chunks per stream instead of asserting an exact event count. The time
    // of the first stdout chunk shows it arrived while the command was still
    // sleeping, i.e. as it was produced rather than buffered until exit.
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let mut first_stdout_at = None;
    let mut first_stderr_at = None;
    let mut exit = None;
    while let Some(event) = handle.next_event().await.expect("next_event") {
        assert!(exit.is_none(), "event after Exit: {event:?}");
        let now = tokio::time::Instant::now();
        match event {
            CommandEvent::Stdout(chunk) => {
                first_stdout_at.get_or_insert(now);
                stdout.extend_from_slice(&chunk);
            }
            CommandEvent::Stderr(chunk) => {
                first_stderr_at.get_or_insert(now);
                stderr.extend_from_slice(&chunk);
            }
            CommandEvent::Exit(command_exit) => exit = Some((now, command_exit)),
        }
    }

    let (exit_at, command_exit) = exit.expect("stream ended without Exit");
    let first_stdout_at = first_stdout_at.expect("no Stdout chunk");
    let first_stderr_at = first_stderr_at.expect("no Stderr chunk");
    assert_eq!(stdout, b"first\n");
    assert_eq!(stderr, b"second\n");
    assert_eq!(command_exit.exit_code, 0);
    assert!(
        first_stdout_at < first_stderr_at,
        "stdout chunk came after stderr"
    );
    assert!(
        exit_at.duration_since(first_stdout_at) >= Duration::from_secs(1),
        "stdout was not delivered before the 2 s sleep ended: {:?} before Exit",
        exit_at.duration_since(first_stdout_at)
    );
}

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0035_run_delivers_large_output_in_order() {
    let sandbox = LiveSandbox::new().await;
    let expected: String = (1..=200_000).map(|n| format!("{n}\n")).collect();

    let result = sandbox
        .commands()
        .run("seq 1 200000", &CommandOptions::default())
        .await
        .expect("run");

    assert!(expected.len() > 1024 * 1024);
    assert_eq!(result.exit_code, 0);
    assert_eq!(result.stdout.len(), expected.len());
    // `assert!` rather than `assert_eq!`: a failure would otherwise print
    // both 1.2 MB strings.
    assert!(
        result.stdout == expected,
        "output differs from seq 1 200000"
    );
}

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0036_options_env_is_set() {
    let sandbox = LiveSandbox::new().await;

    let result = sandbox
        .commands()
        .run(
            "echo \"$COLOR\"",
            &CommandOptions::default().env("COLOR", "teal"),
        )
        .await
        .expect("run");

    assert_eq!(result.stdout, "teal\n");
}

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0037_options_cwd_is_working_directory() {
    let sandbox = LiveSandbox::new().await;

    let result = sandbox
        .commands()
        .run("pwd", &CommandOptions::default().cwd("/tmp"))
        .await
        .expect("run");

    assert_eq!(result.stdout, "/tmp\n");
}

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0038_options_user_runs_as_user() {
    let sandbox = LiveSandbox::new().await;

    let root = sandbox
        .commands()
        .run("whoami", &CommandOptions::default().user("root"))
        .await
        .expect("run as root");
    let user = sandbox
        .commands()
        .run("whoami", &CommandOptions::default().user("user"))
        .await
        .expect("run as user");

    assert_eq!(root.stdout, "root\n");
    assert_eq!(user.stdout, "user\n");
}

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0039_options_timeout_ends_long_command() {
    let sandbox = LiveSandbox::new().await;
    let started = tokio::time::Instant::now();

    let error = sandbox
        .commands()
        .run(
            "sleep 30",
            &CommandOptions::default().timeout(Some(Duration::from_secs(3))),
        )
        .await
        .expect_err("sleep 30 must not finish within 3 s");

    assert!(
        started.elapsed() < Duration::from_secs(15),
        "the wait took {:?}",
        started.elapsed()
    );
    assert!(
        matches!(
            error,
            Error::Timeout(_)
                | Error::Rpc {
                    code: ConnectCode::DeadlineExceeded,
                    ..
                }
        ),
        "want a timeout, got {error:?}"
    );
}

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0040_connect_attaches_to_running_command() {
    let sandbox = LiveSandbox::new().await;
    let first = sandbox
        .commands()
        .start("sleep 3; echo late", &CommandOptions::default())
        .await
        .expect("start");
    let pid = first.pid();
    // Dropping the handle closes the Start stream; the command keeps running.
    drop(first);

    let attached = sandbox
        .commands()
        .connect(pid, None)
        .await
        .expect("connect");
    let result = attached.wait().await.expect("wait");

    assert_eq!(result.stdout, "late\n");
    assert_eq!(result.exit_code, 0);
}

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0041_list_includes_background_command() {
    let sandbox = LiveSandbox::new().await;
    let handle = sandbox
        .commands()
        .start("sleep 30", &CommandOptions::default())
        .await
        .expect("start");

    let processes = sandbox.commands().list().await.expect("list");

    let entry = processes
        .iter()
        .find(|process| process.pid == handle.pid())
        .unwrap_or_else(|| panic!("pid {} missing from {processes:?}", handle.pid()));
    assert_eq!(entry.cmd, "/bin/bash");
    assert!(
        entry.args.iter().any(|arg| arg.contains("sleep 30")),
        "args: {:?}",
        entry.args
    );
}

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0042_send_stdin_reaches_command() {
    let sandbox = LiveSandbox::new().await;
    let handle = sandbox
        .commands()
        .start("head -n 1", &CommandOptions::default().stdin(true))
        .await
        .expect("start");

    sandbox
        .commands()
        .send_stdin(handle.pid(), "typed line\n")
        .await
        .expect("send_stdin");
    let result = handle.wait().await.expect("wait");

    assert_eq!(result.stdout, "typed line\n");
}

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0043_close_stdin_sends_eof() {
    let sandbox = LiveSandbox::new().await;
    let handle = sandbox
        .commands()
        .start("wc -c", &CommandOptions::default().stdin(true))
        .await
        .expect("start");
    sandbox
        .commands()
        .send_stdin(handle.pid(), "abc")
        .await
        .expect("send_stdin");

    sandbox
        .commands()
        .close_stdin(handle.pid())
        .await
        .expect("close_stdin");
    let result = handle.wait().await.expect("wait");

    assert_eq!(result.stdout.trim(), "3");
    assert_eq!(result.exit_code, 0);
}

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0044_sigterm_terminates_command() {
    let sandbox = LiveSandbox::new().await;
    let handle = sandbox
        .commands()
        .start("sleep 30", &CommandOptions::default())
        .await
        .expect("start");
    let pid = handle.pid();
    let started = tokio::time::Instant::now();

    sandbox
        .commands()
        .send_signal(pid, Signal::Sigterm)
        .await
        .expect("signal");
    let result = handle.wait().await.expect("wait");
    // Measure before `list()`, so its round trip does not count against
    // the SIGTERM budget.
    let waited = started.elapsed();
    let processes = sandbox.commands().list().await.expect("list");

    assert!(
        waited < Duration::from_secs(10),
        "the command outlived SIGTERM by {waited:?}"
    );
    assert!(
        !result.success(),
        "SIGTERM'd command reported success: {result:?}"
    );
    assert!(
        processes.iter().all(|process| process.pid != pid),
        "pid {pid} still listed after SIGTERM: {processes:?}"
    );
}

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0045_kill_reports_whether_pid_existed() {
    let sandbox = LiveSandbox::new().await;
    let handle = sandbox
        .commands()
        .start("sleep 30", &CommandOptions::default())
        .await
        .expect("start");

    let killed = sandbox
        .commands()
        .kill(handle.pid())
        .await
        .expect("kill running");
    let missing = sandbox
        .commands()
        .kill(999_999)
        .await
        .expect("kill unknown");

    assert!(
        killed,
        "kill of running pid {} returned false",
        handle.pid()
    );
    assert!(!missing, "kill of unknown pid 999999 returned true");
}

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0046_stream_input_delivers_chunks_in_order() {
    let sandbox = LiveSandbox::new().await;
    let handle = sandbox
        .commands()
        .start("cat", &CommandOptions::default().stdin(true))
        .await
        .expect("start cat");
    let mut input = sandbox
        .commands()
        .stream_input(handle.pid(), &StreamInputOptions::default());

    for chunk in ["one ", "two ", "three\n"] {
        input.send(chunk).await.expect("send");
    }
    input.close().await.expect("close input stream");
    sandbox
        .commands()
        .close_stdin(handle.pid())
        .await
        .expect("close stdin");
    let result = handle.wait().await.expect("wait");

    assert_eq!(result.stdout, "one two three\n");
    assert_eq!(result.exit_code, 0);
}
