//! Mock tests for PTY sessions (`Sandbox::pty`).

mod common;

use bytes::Bytes;
use common::{
    decode_request_envelope, decode_request_envelopes, end_stream, message, sandbox,
    stream_response,
};
use e2b_client::{CommandEvent, CommandOptions, PtyEvent, PtyOptions, PtySize, StreamInputOptions};
use serde_json::json;
use wiremock::matchers::{body_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// A PTY stream: start, one chunk of terminal output (`"$ "`), a keepalive,
/// then the end event.
fn pty_frames() -> Vec<Vec<u8>> {
    vec![
        message(json!({"event": {"start": {"pid": 11}}})),
        message(json!({"event": {"data": {"pty": "JCA="}}})),
        message(json!({"event": {"keepalive": {}}})),
        message(json!({"event": {"end": {"exitCode": 0, "exited": true}}})),
        end_stream(json!({})),
    ]
}

#[tokio::test]
async fn create_sends_bash_with_pty_size_and_terminal_env_defaults() {
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    Mock::given(method("POST"))
        .and(path("/process.Process/Start"))
        .and(header("authorization", "Basic cm9vdDo="))
        .respond_with(stream_response(&pty_frames()))
        .expect(1)
        .mount(&server)
        .await;
    let options = PtyOptions::default()
        .env("LANG", "en_US.UTF-8")
        .cwd("/tmp")
        .user("root");

    let mut handle = sandbox
        .pty()
        .create(PtySize::new(80, 24), &options)
        .await
        .expect("create");
    let first = handle.next_event().await.expect("first event");
    let second = handle.next_event().await.expect("second event");
    let after_exit = handle.next_event().await.expect("after exit");

    assert_eq!(handle.pid(), 11);
    let requests = server.received_requests().await.expect("requests");
    let start = requests
        .iter()
        .find(|request| request.url.path() == "/process.Process/Start")
        .expect("Start request");
    // CLM-0132: bash -i -l, the PTY size, the terminal env defaults only
    // where the caller set nothing, and no `stdin` field.
    assert_eq!(
        decode_request_envelope(&start.body),
        json!({
            "process": {
                "cmd": "/bin/bash",
                "args": ["-i", "-l"],
                "envs": {"TERM": "xterm-256color", "LANG": "en_US.UTF-8", "LC_ALL": "C.UTF-8"},
                "cwd": "/tmp"
            },
            "pty": {"size": {"cols": 80, "rows": 24}}
        })
    );
    assert_eq!(
        start
            .headers
            .get("keepalive-ping-interval")
            .and_then(|value| value.to_str().ok()),
        Some("50")
    );
    assert_eq!(
        start
            .headers
            .get("connect-timeout-ms")
            .and_then(|value| value.to_str().ok()),
        Some("60000"),
        "the PTY stream deadline defaults to 60 s"
    );
    // CLM-0133: `data.pty` bytes come out as `Data`; the keepalive is skipped.
    assert_eq!(first, Some(PtyEvent::Data(Bytes::from_static(b"$ "))));
    assert!(
        matches!(&second, Some(PtyEvent::Exit(exit)) if exit.exit_code == 0),
        "expected Exit, got {second:?}"
    );
    assert_eq!(after_exit, None);
}

#[tokio::test]
async fn pty_handle_skips_stdout_and_command_handle_skips_pty_data() {
    // CLM-0133: PTY output and stdout/stderr are kept apart. The same stream
    // carries one of each; each handle surfaces only its own kind.
    let frames = vec![
        message(json!({"event": {"start": {"pid": 11}}})),
        message(json!({"event": {"data": {"stdout": "b3V0"}}})),
        message(json!({"event": {"data": {"pty": "JCA="}}})),
        message(json!({"event": {"end": {"exitCode": 0, "exited": true}}})),
        end_stream(json!({})),
    ];
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    Mock::given(method("POST"))
        .and(path("/process.Process/Start"))
        .respond_with(stream_response(&frames))
        .mount(&server)
        .await;

    let mut pty = sandbox
        .pty()
        .create(PtySize::new(80, 24), &PtyOptions::default())
        .await
        .expect("create");
    let pty_first = pty.next_event().await.expect("pty event");
    let mut command = sandbox
        .commands()
        .start("true", &CommandOptions::default())
        .await
        .expect("start");
    let command_first = command.next_event().await.expect("command event");
    let command_second = command.next_event().await.expect("command event");

    assert_eq!(pty_first, Some(PtyEvent::Data(Bytes::from_static(b"$ "))));
    assert_eq!(
        command_first,
        Some(CommandEvent::Stdout(Bytes::from_static(b"out")))
    );
    assert!(
        matches!(&command_second, Some(CommandEvent::Exit(_))),
        "the PTY chunk must not reach a CommandHandle, got {command_second:?}"
    );
}

#[tokio::test]
async fn connect_attaches_by_pid_with_the_keepalive_header() {
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    Mock::given(method("POST"))
        .and(path("/process.Process/Connect"))
        .and(header("keepalive-ping-interval", "50"))
        .respond_with(stream_response(&pty_frames()))
        .expect(1)
        .mount(&server)
        .await;

    let handle = sandbox.pty().connect(11, None).await.expect("connect");
    let pid = handle.pid();
    let exit = handle.wait().await.expect("wait");

    let requests = server.received_requests().await.expect("requests");
    let connect = requests
        .iter()
        .find(|request| request.url.path() == "/process.Process/Connect")
        .expect("Connect request");
    assert_eq!(
        decode_request_envelope(&connect.body),
        json!({"process": {"pid": 11}})
    );
    assert!(
        !connect.headers.contains_key("connect-timeout-ms"),
        "no deadline header when the timeout is None"
    );
    assert_eq!(pid, 11);
    assert_eq!(exit.exit_code, 0);
}

#[tokio::test]
async fn send_stdin_uses_the_pty_input_member() {
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    Mock::given(method("POST"))
        .and(path("/process.Process/SendInput"))
        .and(body_json(
            json!({"process": {"pid": 11}, "input": {"pty": "bHMN"}}),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .expect(1)
        .mount(&server)
        .await;

    sandbox.pty().send_stdin(11, "ls\r").await.expect("send");
}

#[tokio::test]
async fn resize_sends_update_with_the_new_size() {
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    Mock::given(method("POST"))
        .and(path("/process.Process/Update"))
        .and(body_json(
            json!({"process": {"pid": 11}, "pty": {"size": {"cols": 120, "rows": 40}}}),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .expect(1)
        .mount(&server)
        .await;

    sandbox
        .pty()
        .resize(11, PtySize::new(120, 40))
        .await
        .expect("resize");
}

#[tokio::test]
async fn kill_sends_sigkill_and_reports_a_missing_pid_as_false() {
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    Mock::given(method("POST"))
        .and(path("/process.Process/SendSignal"))
        .and(body_json(
            json!({"process": {"pid": 11}, "signal": "SIGNAL_SIGKILL"}),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/process.Process/SendSignal"))
        .and(body_json(
            json!({"process": {"pid": 12}, "signal": "SIGNAL_SIGKILL"}),
        ))
        .respond_with(
            ResponseTemplate::new(404)
                .set_body_json(json!({"code": "not_found", "message": "no process"})),
        )
        .expect(1)
        .mount(&server)
        .await;

    let killed = sandbox.pty().kill(11).await.expect("kill 11");
    let missing = sandbox.pty().kill(12).await.expect("kill 12");

    assert!(killed);
    assert!(!missing);
}

#[tokio::test]
async fn pty_stream_input_uses_the_pty_member() {
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    Mock::given(method("POST"))
        .and(path("/process.Process/StreamInput"))
        .respond_with(stream_response(&[
            message(json!({})),
            end_stream(json!({})),
        ]))
        .expect(1)
        .mount(&server)
        .await;

    let mut input = sandbox
        .pty()
        .stream_input(11, &StreamInputOptions::default());
    input.send("ls\r").await.expect("send");
    input.close().await.expect("close");

    let requests = server.received_requests().await.expect("requests");
    let stream = requests
        .iter()
        .find(|request| request.url.path() == "/process.Process/StreamInput")
        .expect("StreamInput request");
    assert_eq!(
        decode_request_envelopes(&stream.body),
        vec![
            json!({"start": {"process": {"pid": 11}}}),
            json!({"data": {"input": {"pty": "bHMN"}}}),
        ]
    );
}
