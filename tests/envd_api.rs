//! envd process and filesystem calls against a mock server, including Connect
//! streaming frames, end-of-stream and error frames.

mod common;

use std::time::Duration;

use common::{
    ACCESS_TOKEN, SANDBOX_ID, decode_request_envelope, end_stream, envelope, message, sandbox,
    sandbox_from_json, sandbox_json, sandbox_with_version, stream_response,
};
use e2b_client::{
    CommandConnectOptions, CommandEvent, CommandOptions, ConnectCode, Error, FileOptions, FileType,
    ListOptions, Sandbox, Signal, WriteOptions,
};
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use wiremock::matchers::{
    body_json, body_string_contains, header, header_exists, method, path, query_param,
    query_param_is_missing,
};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

/// base64("hello\n") and base64("oops\n").
const HELLO_B64: &str = "aGVsbG8K";
const OOPS_B64: &str = "b29wcwo=";

fn start_frames(exit_code: i32) -> Vec<Vec<u8>> {
    vec![
        message(json!({"event": {"start": {"pid": 42}}})),
        message(json!({"event": {"keepalive": {}}})),
        message(json!({"event": {"data": {"stdout": HELLO_B64}}})),
        message(json!({"event": {"data": {"stderr": OOPS_B64}}})),
        message(
            json!({"event": {"end": {"exitCode": exit_code, "exited": true,
            "status": "exit status"}}}),
        ),
        end_stream(json!({})),
    ]
}

#[tokio::test]
async fn run_streams_output_to_exit_code_with_sdk_headers() {
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    Mock::given(method("POST"))
        .and(path("/process.Process/Start"))
        .and(header("content-type", "application/connect+json"))
        .and(header("connect-protocol-version", "1"))
        .and(header("connect-timeout-ms", "60000"))
        .and(header("keepalive-ping-interval", "50"))
        .and(header("e2b-sandbox-id", SANDBOX_ID))
        .and(header("e2b-sandbox-port", "49983"))
        .and(header("x-access-token", ACCESS_TOKEN))
        // base64("root:")
        .and(header("authorization", "Basic cm9vdDo="))
        .respond_with(stream_response(&start_frames(3)))
        .expect(1)
        .mount(&server)
        .await;

    let options = CommandOptions::default()
        .cwd("/work")
        .env("A", "1")
        .user("root");
    let result = sandbox
        .commands()
        .run("echo hello", &options)
        .await
        .expect("run");

    assert_eq!(result.stdout, "hello\n");
    assert_eq!(result.stderr, "oops\n");
    assert_eq!(result.exit_code, 3);
    assert!(!result.success());

    let requests = server.received_requests().await.expect("recording");
    let start = requests
        .iter()
        .find(|r| r.url.path() == "/process.Process/Start")
        .expect("start");
    assert_eq!(
        decode_request_envelope(&start.body),
        json!({
            "process": {
                "cmd": "/bin/bash",
                "args": ["-l", "-c", "echo hello"],
                "envs": {"A": "1"},
                "cwd": "/work"
            },
            "stdin": false
        })
    );
    // The API key must never be sent to envd.
    assert!(start.headers.get("x-api-key").is_none());
}

#[tokio::test]
async fn start_without_user_sends_no_authorization_on_current_envd() {
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    Mock::given(method("POST"))
        .and(path("/process.Process/Start"))
        .and(|request: &Request| request.headers.get("authorization").is_none())
        .respond_with(stream_response(&start_frames(0)))
        .expect(1)
        .mount(&server)
        .await;

    let result = sandbox
        .commands()
        .run("true", &CommandOptions::default())
        .await
        .expect("run");

    assert!(result.success());
}

#[tokio::test]
async fn legacy_envd_defaults_to_user_account() {
    let server = MockServer::start().await;
    let sandbox = sandbox_with_version(&server, "0.3.9").await;
    Mock::given(method("POST"))
        .and(path("/process.Process/Start"))
        // base64("user:")
        .and(header("authorization", "Basic dXNlcjo="))
        .respond_with(stream_response(&start_frames(0)))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/files"))
        .and(query_param("username", "user"))
        .respond_with(ResponseTemplate::new(200).set_body_string("x"))
        .expect(1)
        .mount(&server)
        .await;

    sandbox
        .commands()
        .run("true", &CommandOptions::default())
        .await
        .expect("run");
    sandbox
        .files()
        .read("a", &FileOptions::default())
        .await
        .expect("read");
}

#[tokio::test]
async fn next_event_yields_chunks_in_order_and_skips_keepalives() {
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    Mock::given(method("POST"))
        .and(path("/process.Process/Start"))
        .respond_with(stream_response(&start_frames(0)))
        .mount(&server)
        .await;

    let mut handle = sandbox
        .commands()
        .start("echo hello", &CommandOptions::default())
        .await
        .expect("start");
    let mut events = Vec::new();
    while let Some(event) = handle.next_event().await.expect("event") {
        events.push(event);
    }

    assert_eq!(handle.pid(), 42);
    assert_eq!(events.len(), 3);
    assert!(matches!(&events[0], CommandEvent::Stdout(chunk) if chunk.as_ref() == b"hello\n"));
    assert!(matches!(&events[1], CommandEvent::Stderr(chunk) if chunk.as_ref() == b"oops\n"));
    assert!(matches!(&events[2], CommandEvent::Exit(exit) if exit.exit_code == 0));
}

#[tokio::test]
async fn end_of_stream_error_frame_becomes_rpc_error() {
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    let frames = vec![
        message(json!({"event": {"start": {"pid": 7}}})),
        end_stream(json!({"error": {"code": "deadline_exceeded", "message": "too slow"}})),
    ];
    Mock::given(method("POST"))
        .and(path("/process.Process/Start"))
        .respond_with(stream_response(&frames))
        .mount(&server)
        .await;

    let err = sandbox
        .commands()
        .run("sleep 100", &CommandOptions::default())
        .await
        .expect_err("e");

    assert!(
        matches!(err, Error::Rpc { code: ConnectCode::DeadlineExceeded, ref message, .. }
            if message == "too slow"),
        "{err:?}"
    );
}

#[tokio::test]
async fn stream_without_end_of_stream_frame_is_protocol_error() {
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    let frames = vec![message(json!({"event": {"start": {"pid": 7}}}))];
    Mock::given(method("POST"))
        .and(path("/process.Process/Start"))
        .respond_with(stream_response(&frames))
        .mount(&server)
        .await;

    let err = sandbox
        .commands()
        .run("x", &CommandOptions::default())
        .await
        .expect_err("error");

    assert!(matches!(err, Error::Protocol(_)), "{err:?}");
}

#[tokio::test]
async fn truncated_envelope_is_protocol_error() {
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    let mut truncated = message(json!({"event": {"data": {"stdout": HELLO_B64}}}));
    truncated.truncate(truncated.len() - 2);
    let frames = vec![message(json!({"event": {"start": {"pid": 7}}})), truncated];
    Mock::given(method("POST"))
        .and(path("/process.Process/Start"))
        .respond_with(stream_response(&frames))
        .mount(&server)
        .await;

    let err = sandbox
        .commands()
        .run("x", &CommandOptions::default())
        .await
        .expect_err("error");

    assert!(
        matches!(err, Error::Protocol(ref m) if m.contains("middle")),
        "{err:?}"
    );
}

#[tokio::test]
async fn compressed_envelope_is_protocol_error() {
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    let frames = vec![
        message(json!({"event": {"start": {"pid": 7}}})),
        envelope(0b01, &json!({"event": {"data": {"stdout": HELLO_B64}}})),
    ];
    Mock::given(method("POST"))
        .and(path("/process.Process/Start"))
        .respond_with(stream_response(&frames))
        .mount(&server)
        .await;

    let err = sandbox
        .commands()
        .run("x", &CommandOptions::default())
        .await
        .expect_err("error");

    assert!(
        matches!(err, Error::Protocol(ref m) if m.contains("compressed")),
        "{err:?}"
    );
}

/// Serves one streaming response that sends `frames` and then stalls, which
/// wiremock cannot do: its delays apply before the response headers.
async fn stalling_envd(frames: Vec<Vec<u8>>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let address = listener.local_addr().expect("address");
    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("accept");
        let mut request = Vec::new();
        let mut buffer = [0_u8; 4096];
        while !request.windows(4).any(|window| window == b"\r\n\r\n") {
            let read = socket.read(&mut buffer).await.expect("read");
            if read == 0 {
                return;
            }
            request.extend_from_slice(&buffer[..read]);
        }
        let body = frames.concat();
        let mut response = b"HTTP/1.1 200 OK\r\ncontent-type: application/connect+json\r\n\
            transfer-encoding: chunked\r\n\r\n"
            .to_vec();
        // One chunk with every frame; a zero-length chunk would end the body.
        if !body.is_empty() {
            response.extend_from_slice(format!("{:x}\r\n", body.len()).as_bytes());
            response.extend_from_slice(&body);
            response.extend_from_slice(b"\r\n");
        }
        socket.write_all(&response).await.expect("write");
        // Hold the connection open, sending nothing more.
        tokio::time::sleep(Duration::from_secs(30)).await;
    });
    format!("http://{address}")
}

/// A sandbox whose REST API is a mock and whose envd is a [`stalling_envd`]
/// sending `frames`.
async fn sandbox_on_stalling_envd(
    frames: Vec<Vec<u8>>,
    request_timeout: Option<Duration>,
) -> Sandbox {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v2/sandboxes"))
        .respond_with(ResponseTemplate::new(201).set_body_json(sandbox_json("0.5.7")))
        .mount(&server)
        .await;
    let config = e2b_client::ConnectionConfig::builder()
        .api_key(common::API_KEY)
        .api_url(server.uri())
        .sandbox_url(stalling_envd(frames).await)
        .domain("example.test")
        .request_timeout(request_timeout)
        .build()
        .expect("config");
    e2b_client::Client::new(config)
        .expect("client")
        .create_sandbox(&e2b_client::CreateSandboxParams::default())
        .await
        .expect("create")
}

#[tokio::test]
async fn stream_stalling_after_start_times_out_with_command_timeout() {
    let start = message(json!({"event": {"start": {"pid": 7}}}));
    let sandbox = sandbox_on_stalling_envd(vec![start], None).await;
    let options = CommandOptions::default().timeout(Some(Duration::from_millis(300)));

    let err = sandbox
        .commands()
        .run("sleep 60", &options)
        .await
        .expect_err("error");

    assert!(matches!(err, Error::Timeout(_)), "{err:?}");
}

#[tokio::test]
async fn first_event_must_arrive_within_the_request_timeout() {
    // Arrange: response headers arrive at once, the start event never does.
    let request_timeout = Duration::from_millis(300);
    let sandbox = sandbox_on_stalling_envd(Vec::new(), Some(request_timeout)).await;

    // Act: no command timeout, so only the request timeout can end the wait.
    let options = CommandOptions::default().timeout(None);
    let err = sandbox
        .commands()
        .start("x", &options)
        .await
        .expect_err("error");

    // Assert
    assert!(
        matches!(err, Error::Timeout(limit) if limit == request_timeout),
        "{err:?}"
    );
}

#[tokio::test]
async fn clean_end_without_exit_event_fails_wait() {
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    let frames = vec![
        message(json!({"event": {"start": {"pid": 7}}})),
        end_stream(json!({})),
    ];
    Mock::given(method("POST"))
        .and(path("/process.Process/Start"))
        .respond_with(stream_response(&frames))
        .mount(&server)
        .await;

    let err = sandbox
        .commands()
        .run("x", &CommandOptions::default())
        .await
        .expect_err("error");

    assert!(
        matches!(err, Error::Protocol(ref m) if m.contains("end event")),
        "{err:?}"
    );
}

#[tokio::test]
async fn stream_not_opening_with_start_event_is_protocol_error() {
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    let frames = vec![
        message(json!({"event": {"data": {"stdout": HELLO_B64}}})),
        end_stream(json!({})),
    ];
    Mock::given(method("POST"))
        .and(path("/process.Process/Start"))
        .respond_with(stream_response(&frames))
        .mount(&server)
        .await;

    let err = sandbox
        .commands()
        .start("x", &CommandOptions::default())
        .await
        .expect_err("error");

    assert!(
        matches!(err, Error::Protocol(ref m) if m.contains("start event")),
        "{err:?}"
    );
}

#[tokio::test]
async fn stream_rejected_before_streaming_maps_connect_error() {
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    Mock::given(method("POST"))
        .and(path("/process.Process/Start"))
        .respond_with(
            ResponseTemplate::new(401)
                .set_body_json(json!({"code": "unauthenticated", "message": "bad token"})),
        )
        .mount(&server)
        .await;

    let err = sandbox
        .commands()
        .start("x", &CommandOptions::default())
        .await
        .expect_err("error");

    assert!(
        matches!(
            err,
            Error::Rpc {
                code: ConnectCode::Unauthenticated,
                ..
            }
        ),
        "{err:?}"
    );
}

#[tokio::test]
async fn slow_stream_open_times_out_with_request_timeout() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v2/sandboxes"))
        .respond_with(ResponseTemplate::new(201).set_body_json(common::sandbox_json("0.5.7")))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/process.Process/Start"))
        .respond_with(stream_response(&start_frames(0)).set_delay(Duration::from_secs(5)))
        .mount(&server)
        .await;
    let client = common::client_with_timeout(&server, Some(Duration::from_millis(200)));
    let sandbox = client
        .create_sandbox(&e2b_client::CreateSandboxParams::default())
        .await
        .expect("create");

    let err = sandbox
        .commands()
        .start("x", &CommandOptions::default())
        .await
        .expect_err("error");

    assert!(matches!(err, Error::Timeout(_)), "{err:?}");
}

#[tokio::test]
async fn connect_to_running_process_by_pid() {
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    Mock::given(method("POST"))
        .and(path("/process.Process/Connect"))
        .and(header("connect-timeout-ms", "5000"))
        .and(header("keepalive-ping-interval", "50"))
        .and(|request: &Request| {
            decode_request_envelope(&request.body) == json!({"process": {"pid": 42}})
        })
        .respond_with(stream_response(&start_frames(0)))
        .expect(1)
        .mount(&server)
        .await;

    let handle = sandbox
        .commands()
        .connect(
            42,
            &CommandConnectOptions::new().timeout(Some(Duration::from_secs(5))),
        )
        .await
        .expect("connect");
    let result = handle.wait().await.expect("wait");

    assert_eq!(result.stdout, "hello\n");
}

#[tokio::test]
async fn list_processes_unary() {
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    Mock::given(method("POST"))
        .and(path("/process.Process/List"))
        .and(header("content-type", "application/json"))
        .and(header("connect-protocol-version", "1"))
        .and(header_exists("connect-timeout-ms"))
        .and(body_json(json!({})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "processes": [
                {"pid": 1, "tag": "web",
                 "config": {"cmd": "/bin/bash", "args": ["-l", "-c", "serve"], "cwd": "/srv"}},
                {"pid": 2}
            ]
        })))
        .mount(&server)
        .await;

    let processes = sandbox.commands().list().await.expect("list");

    assert_eq!(processes.len(), 2);
    assert_eq!(processes[0].pid, 1);
    assert_eq!(processes[0].tag.as_deref(), Some("web"));
    assert_eq!(processes[0].args, ["-l", "-c", "serve"]);
    assert_eq!(processes[0].cwd.as_deref(), Some("/srv"));
    assert_eq!(processes[1].cmd, "");
}

#[tokio::test]
async fn stdin_and_signal_calls_send_proto_json() {
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    Mock::given(method("POST"))
        .and(path("/process.Process/SendInput"))
        .and(body_json(
            json!({"process": {"pid": 9}, "input": {"stdin": "aGkK"}}),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/process.Process/CloseStdin"))
        .and(body_json(json!({"process": {"pid": 9}})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/process.Process/SendSignal"))
        .and(body_json(
            json!({"process": {"pid": 9}, "signal": "SIGNAL_SIGTERM"}),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/process.Process/SendSignal"))
        .and(body_json(
            json!({"process": {"pid": 9}, "signal": "SIGNAL_SIGKILL"}),
        ))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;
    let commands = sandbox.commands();

    commands.send_stdin(9, b"hi\n").await.expect("stdin");
    commands.close_stdin(9).await.expect("close stdin");
    commands
        .send_signal(9, Signal::Sigterm)
        .await
        .expect("signal");
    assert!(commands.kill(9).await.expect("kill"));
}

#[tokio::test]
async fn close_stdin_is_refused_before_envd_0_5_2() {
    // Arrange: envd would accept the call, so only a client-side check fails it.
    let server = MockServer::start().await;
    let sandbox = sandbox_with_version(&server, "0.5.1").await;
    Mock::given(method("POST"))
        .and(path("/process.Process/CloseStdin"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .mount(&server)
        .await;

    // Act
    let result = sandbox.commands().close_stdin(9).await;

    // Assert: refused locally, like a depth-0 list, so no request is sent.
    // `InvalidArgument` is the crate's variant for a call refused before any
    // request; if the fix picks another, update this match with it.
    assert!(
        matches!(result, Err(Error::InvalidArgument(_))),
        "{result:?}"
    );
    let requests = server.received_requests().await.expect("recording");
    assert!(
        requests
            .iter()
            .all(|request| request.url.path() != "/process.Process/CloseStdin"),
        "close_stdin must not reach envd"
    );
}

#[tokio::test]
async fn kill_missing_process_returns_false() {
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    Mock::given(method("POST"))
        .and(path("/process.Process/SendSignal"))
        .respond_with(
            ResponseTemplate::new(404)
                .set_body_json(json!({"code": "not_found", "message": "process not found"})),
        )
        .mount(&server)
        .await;

    assert!(!sandbox.commands().kill(123).await.expect("kill"));
}

#[tokio::test]
async fn unary_plain_proxy_error_maps_status() {
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    Mock::given(method("POST"))
        .and(path("/process.Process/List"))
        .respond_with(ResponseTemplate::new(502).set_body_string("sandbox not running"))
        .mount(&server)
        .await;

    let err = sandbox.commands().list().await.expect_err("error");

    assert!(
        matches!(err, Error::Rpc { code: ConnectCode::Unavailable, ref message, .. }
            if message == "sandbox not running"),
        "{err:?}"
    );
}

#[tokio::test]
async fn read_file_downloads_bytes() {
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    Mock::given(method("GET"))
        .and(path("/files"))
        .and(query_param("path", "/tmp/a b.txt"))
        .and(query_param_is_missing("username"))
        .and(header("x-access-token", ACCESS_TOKEN))
        .and(header("e2b-sandbox-id", SANDBOX_ID))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"\x00binary\xff".to_vec()))
        .mount(&server)
        .await;

    let bytes = sandbox
        .files()
        .read("/tmp/a b.txt", &FileOptions::default())
        .await
        .expect("read");

    assert_eq!(bytes.as_ref(), b"\x00binary\xff");
}

#[tokio::test]
async fn read_missing_file_is_envd_not_found() {
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    Mock::given(method("GET"))
        .and(path("/files"))
        .respond_with(
            ResponseTemplate::new(404)
                .set_body_json(json!({"code": 404, "message": "file not found"})),
        )
        .mount(&server)
        .await;

    let err = sandbox
        .files()
        .read_to_string("/nope", &FileOptions::default())
        .await
        .expect_err("error");

    assert!(
        matches!(err, Error::Envd { status: 404, ref message, .. } if message == "file not found"),
        "{err:?}"
    );
    assert!(err.is_not_found());
}

#[tokio::test]
async fn write_file_uploads_multipart() {
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    Mock::given(method("POST"))
        .and(path("/files"))
        .and(query_param("path", "/tmp/out.txt"))
        .and(query_param("username", "root"))
        .and(header("x-access-token", ACCESS_TOKEN))
        .and(|request: &Request| {
            request
                .headers
                .get("content-type")
                .and_then(|value| value.to_str().ok())
                .is_some_and(|value| value.starts_with("multipart/form-data"))
        })
        .and(body_string_contains(
            "name=\"file\"; filename=\"/tmp/out.txt\"",
        ))
        .and(body_string_contains("file contents"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {"name": "out.txt", "type": "file", "path": "/tmp/out.txt"}
        ])))
        .expect(1)
        .mount(&server)
        .await;

    let info = sandbox
        .files()
        .write(
            "/tmp/out.txt",
            "file contents",
            &WriteOptions::default().user("root"),
        )
        .await
        .expect("write");

    assert_eq!(info.name, "out.txt");
    assert_eq!(info.path, "/tmp/out.txt");
    assert_eq!(info.file_type, FileType::File);
}

#[tokio::test]
async fn write_file_with_empty_response_is_protocol_error() {
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    Mock::given(method("POST"))
        .and(path("/files"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .mount(&server)
        .await;

    let err = sandbox
        .files()
        .write("/x", vec![1_u8, 2, 3], &WriteOptions::default())
        .await
        .expect_err("error");

    assert!(matches!(err, Error::Protocol(_)), "{err:?}");
}

#[tokio::test]
async fn write_file_disk_full_is_envd_error() {
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    Mock::given(method("POST"))
        .and(path("/files"))
        .respond_with(ResponseTemplate::new(507).set_body_string("no space"))
        .mount(&server)
        .await;

    let err = sandbox
        .files()
        .write("/x", "data", &WriteOptions::default())
        .await
        .expect_err("error");

    assert!(matches!(err, Error::Envd { status: 507, .. }), "{err:?}");
}

#[tokio::test]
async fn list_dir_and_stat_parse_entries() {
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    let entry = json!({
        "name": "a.txt", "type": "FILE_TYPE_FILE", "path": "/tmp/a.txt", "size": "5",
        "mode": 420, "permissions": "-rw-r--r--", "owner": "user", "group": "user",
        "modifiedTime": "2026-10-26T12:00:00Z"
    });
    Mock::given(method("POST"))
        .and(path("/filesystem.Filesystem/ListDir"))
        .and(body_json(json!({"path": "/tmp", "depth": 2})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "entries": [entry.clone(), {"name": "d", "type": "FILE_TYPE_DIRECTORY", "path": "/tmp/d"}]
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/filesystem.Filesystem/Stat"))
        .and(body_json(json!({"path": "/tmp/a.txt"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"entry": entry})))
        .mount(&server)
        .await;
    let files = sandbox.files();

    let entries = files
        .list("/tmp", &ListOptions::new().depth(2))
        .await
        .expect("list");
    let info = files
        .get_info("/tmp/a.txt", &FileOptions::default())
        .await
        .expect("stat");

    assert_eq!(entries.len(), 2);
    assert_eq!(entries[1].file_type, FileType::Directory);
    assert_eq!(info.size, 5);
    assert_eq!(info.mode, 0o644);
    assert_eq!(info.file_type, FileType::File);
    assert!(info.modified_time.is_some());
}

#[tokio::test]
async fn list_dir_with_zero_depth_is_rejected_locally() {
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;

    let err = sandbox
        .files()
        .list("/", &ListOptions::new().depth(0))
        .await
        .expect_err("error");

    assert!(matches!(err, Error::InvalidArgument(_)), "{err:?}");
    let requests = server.received_requests().await.expect("recording");
    assert!(
        requests
            .iter()
            .all(|request| request.url.path() == "/v2/sandboxes"),
        "only the create call may reach the server"
    );
}

/// The `username` query value of each `/files` request `server` received.
async fn file_api_usernames(server: &MockServer) -> Vec<Option<String>> {
    server
        .received_requests()
        .await
        .expect("recording")
        .iter()
        .filter(|request| request.url.path() == "/files")
        .map(|request| {
            request
                .url
                .query_pairs()
                .find(|(key, _)| key == "username")
                .map(|(_, value)| value.into_owned())
        })
        .collect()
}

#[tokio::test]
async fn file_api_selects_the_user_with_the_username_query() {
    // Arrange: no `/files` mocks; each call's 404 is irrelevant here.
    let current_server = MockServer::start().await;
    let current = sandbox(&current_server).await;
    let legacy_server = MockServer::start().await;
    let legacy = sandbox_with_version(&legacy_server, "0.3.9").await;

    // Act: read then write, as an explicit user and as no user.
    for (sandbox, user) in [(&current, Some("root")), (&current, None), (&legacy, None)] {
        let files = sandbox.files();
        // `map_or_else(default, f)`: the default when `None`, else `f(user)`.
        let read_options = user.map_or_else(FileOptions::new, |user| FileOptions::new().user(user));
        let write_options = user.map_or_else(WriteOptions::default, |user| {
            WriteOptions::default().user(user)
        });
        let _ = files.read("/a", &read_options).await;
        let _ = files.write("/a", "x", &write_options).await;
    }

    // Assert: the explicit user is sent; no user sends none on current envd
    // and `user` on envd older than 0.4.0, as RPCs do.
    let root = Some("root".to_owned());
    assert_eq!(
        file_api_usernames(&current_server).await,
        [root.clone(), root, None, None]
    );
    let user = Some("user".to_owned());
    assert_eq!(
        file_api_usernames(&legacy_server).await,
        [user.clone(), user]
    );
}

#[tokio::test]
async fn exists_maps_not_found_to_false() {
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    Mock::given(method("POST"))
        .and(path("/filesystem.Filesystem/Stat"))
        .and(body_json(json!({"path": "/present"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"entry": {"name": "p"}})))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/filesystem.Filesystem/Stat"))
        .and(body_json(json!({"path": "/absent"})))
        .respond_with(
            ResponseTemplate::new(404).set_body_json(json!({"code": "not_found", "message": "no"})),
        )
        .mount(&server)
        .await;
    let files = sandbox.files();

    assert!(
        files
            .exists("/present", &FileOptions::default())
            .await
            .expect("present")
    );
    assert!(
        !files
            .exists("/absent", &FileOptions::default())
            .await
            .expect("absent")
    );
}

#[tokio::test]
async fn make_dir_reports_already_exists_as_false() {
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    Mock::given(method("POST"))
        .and(path("/filesystem.Filesystem/MakeDir"))
        .and(body_json(json!({"path": "/new"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"entry": {"name": "new"}})))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/filesystem.Filesystem/MakeDir"))
        .and(body_json(json!({"path": "/old"})))
        .respond_with(
            ResponseTemplate::new(409)
                .set_body_json(json!({"code": "already_exists", "message": "exists"})),
        )
        .mount(&server)
        .await;
    let files = sandbox.files();

    assert!(
        files
            .make_dir("/new", &FileOptions::default())
            .await
            .expect("new")
    );
    assert!(
        !files
            .make_dir("/old", &FileOptions::default())
            .await
            .expect("old")
    );
}

#[tokio::test]
async fn rename_and_remove() {
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    Mock::given(method("POST"))
        .and(path("/filesystem.Filesystem/Move"))
        .and(body_json(json!({"source": "/a", "destination": "/b"})))
        // base64("root:")
        .and(header("authorization", "Basic cm9vdDo="))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "entry": {"name": "b", "type": "FILE_TYPE_FILE", "path": "/b"}
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/filesystem.Filesystem/Remove"))
        .and(body_json(json!({"path": "/b"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .expect(1)
        .mount(&server)
        .await;
    let files = sandbox.files();

    let moved = files
        .rename("/a", "/b", &FileOptions::new().user("root"))
        .await
        .expect("rename");
    files
        .remove("/b", &FileOptions::default())
        .await
        .expect("remove");

    assert_eq!(moved.path, "/b");
}

#[tokio::test]
async fn stat_without_entry_is_protocol_error() {
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    Mock::given(method("POST"))
        .and(path("/filesystem.Filesystem/Stat"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .mount(&server)
        .await;

    let err = sandbox
        .files()
        .get_info("/x", &FileOptions::default())
        .await
        .expect_err("error");

    assert!(matches!(err, Error::Protocol(_)), "{err:?}");
}

#[tokio::test]
async fn is_running_checks_envd_health() {
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    Mock::given(method("GET"))
        .and(path("/health"))
        .and(header("e2b-sandbox-id", SANDBOX_ID))
        .respond_with(ResponseTemplate::new(204))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/health"))
        .respond_with(ResponseTemplate::new(502))
        .mount(&server)
        .await;

    assert!(sandbox.is_running().await.expect("running"));
    assert!(!sandbox.is_running().await.expect("not running"));
}

/// Calls every kind of envd route once (process streams, unary RPCs, the
/// file API and health) and returns the requests that reached envd, that is,
/// all but the create call. Nothing is mocked, so each call fails on
/// wiremock's 404 after its request is recorded.
async fn envd_requests_for_every_route(server: &MockServer, sandbox: &Sandbox) -> Vec<Request> {
    let commands = sandbox.commands();
    let files = sandbox.files();
    // `let _ =` discards each call's result on purpose.
    let _ = commands.run("true", &CommandOptions::default()).await;
    let _ = commands.connect(1, &CommandConnectOptions::default()).await;
    let _ = commands.list().await;
    let _ = files.read("/a", &FileOptions::default()).await;
    let _ = files.write("/a", "x", &WriteOptions::default()).await;
    let _ = files.list("/", &ListOptions::default()).await;
    let _ = sandbox.is_running().await;
    let requests: Vec<Request> = server
        .received_requests()
        .await
        .expect("recording")
        .into_iter()
        .filter(|request| request.url.path() != "/v2/sandboxes")
        .collect();
    assert_eq!(requests.len(), 7, "one request per call");
    requests
}

#[tokio::test]
async fn every_envd_request_carries_the_sandbox_id_and_port() {
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;

    let requests = envd_requests_for_every_route(&server, &sandbox).await;

    for request in &requests {
        let header = |name: &str| request.headers.get(name).map(|value| value.as_bytes());
        assert_eq!(
            header("e2b-sandbox-id"),
            Some(SANDBOX_ID.as_bytes()),
            "{}",
            request.url
        );
        assert_eq!(
            header("e2b-sandbox-port"),
            Some(&b"49983"[..]),
            "{}",
            request.url
        );
    }
}

#[tokio::test]
async fn envd_requests_carry_the_access_token_only_when_issued_and_never_the_api_key() {
    // Arrange: one sandbox the API issued a token for, one without.
    let secured_server = MockServer::start().await;
    let secured = sandbox(&secured_server).await;
    let open_server = MockServer::start().await;
    let mut created = sandbox_json("0.5.7");
    created["envdAccessToken"] = Value::Null;
    let open = sandbox_from_json(&open_server, created).await;

    // Act
    let secured_requests = envd_requests_for_every_route(&secured_server, &secured).await;
    let open_requests = envd_requests_for_every_route(&open_server, &open).await;

    // Assert
    for request in &secured_requests {
        assert_eq!(
            request
                .headers
                .get("x-access-token")
                .map(|value| value.as_bytes()),
            Some(ACCESS_TOKEN.as_bytes()),
            "{}",
            request.url
        );
    }
    for request in &open_requests {
        assert!(
            !request.headers.contains_key("x-access-token"),
            "{}",
            request.url
        );
    }
    for request in secured_requests.iter().chain(&open_requests) {
        assert!(
            !request.headers.contains_key("x-api-key"),
            "{}",
            request.url
        );
    }
}

#[tokio::test]
async fn rpcs_post_uncompressed_connect_json_to_service_method_paths() {
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;

    let requests = envd_requests_for_every_route(&server, &sandbox).await;

    let rpcs: Vec<&Request> = requests
        .iter()
        .filter(|request| request.url.path().contains('.'))
        .collect();
    let paths: Vec<&str> = rpcs.iter().map(|request| request.url.path()).collect();
    assert_eq!(
        paths,
        [
            "/process.Process/Start",
            "/process.Process/Connect",
            "/process.Process/List",
            "/filesystem.Filesystem/ListDir"
        ]
    );
    for request in rpcs {
        assert_eq!(request.method.as_str(), "POST", "{}", request.url);
        assert!(
            !request.headers.contains_key("content-encoding"),
            "{}",
            request.url
        );
        assert!(
            !request.headers.contains_key("connect-content-encoding"),
            "{}",
            request.url
        );
        let content_type = request
            .headers
            .get("content-type")
            .map(|value| value.as_bytes());
        // A streaming body is one uncompressed (flag 0) JSON envelope, which
        // `decode_request_envelope` checks; a unary body is bare JSON.
        match content_type {
            Some(b"application/connect+json") => {
                decode_request_envelope(&request.body);
            }
            Some(b"application/json") => {
                serde_json::from_slice::<Value>(&request.body).expect("unary JSON body");
            }
            other => panic!("unexpected content type {other:?} on {}", request.url),
        }
    }
}

#[tokio::test]
async fn list_and_stat_report_symlinks_for_both_envd_shapes() {
    // Arrange: newer envd sets `isSymlink` with the target's type; older envd
    // puts FILE_TYPE_SYMLINK in `type`; a plain file has `isSymlink: false`.
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    let new_shape = json!({
        "name": "link", "type": "FILE_TYPE_FILE", "path": "/w/link",
        "isSymlink": true, "symlinkTarget": "/w/target"
    });
    Mock::given(method("POST"))
        .and(path("/filesystem.Filesystem/ListDir"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"entries": [
            new_shape.clone(),
            {"name": "old", "type": "FILE_TYPE_SYMLINK", "path": "/w/old"},
            {"name": "target", "type": "FILE_TYPE_FILE", "path": "/w/target", "isSymlink": false},
            {"name": "dirlink", "type": "FILE_TYPE_DIRECTORY", "path": "/w/dirlink", "isSymlink": true}
        ]})))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/filesystem.Filesystem/Stat"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"entry": new_shape})))
        .mount(&server)
        .await;
    let files = sandbox.files();

    // Act
    let entries = files
        .list("/w", &ListOptions::default())
        .await
        .expect("list");
    let info = files
        .get_info("/w/link", &FileOptions::default())
        .await
        .expect("stat");

    // Assert
    let types: Vec<(&str, FileType)> = entries
        .iter()
        .map(|entry| (entry.name.as_str(), entry.file_type))
        .collect();
    assert_eq!(
        types,
        [
            ("link", FileType::Symlink),
            ("old", FileType::Symlink),
            ("target", FileType::File),
            ("dirlink", FileType::Symlink),
        ]
    );
    assert_eq!(info.file_type, FileType::Symlink);
    assert_eq!(info.symlink_target.as_deref(), Some("/w/target"));
}
