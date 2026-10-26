//! envd process and filesystem calls against a mock server, including Connect
//! streaming frames, end-of-stream and error frames.

mod common;

use std::time::Duration;

use common::{
    ACCESS_TOKEN, SANDBOX_ID, decode_request_envelope, end_stream, envelope, message, sandbox,
    sandbox_with_version, stream_response,
};
use e2b_client::{CommandEvent, CommandOptions, ConnectCode, Error, FileType, Signal};
use serde_json::json;
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
    sandbox.files().read("a", None).await.expect("read");
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
        matches!(err, Error::Rpc { code: ConnectCode::DeadlineExceeded, ref message }
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

/// Serves one streaming response that sends `start` and then stalls, which
/// wiremock cannot do: its delays apply before the response headers.
async fn stalling_envd() -> String {
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
        let start = message(json!({"event": {"start": {"pid": 7}}}));
        let mut response = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/connect+json\r\n\
             transfer-encoding: chunked\r\n\r\n{:x}\r\n",
            start.len()
        )
        .into_bytes();
        response.extend_from_slice(&start);
        response.extend_from_slice(b"\r\n");
        socket.write_all(&response).await.expect("write");
        // Hold the connection open, sending nothing more.
        tokio::time::sleep(Duration::from_secs(30)).await;
    });
    format!("http://{address}")
}

#[tokio::test]
async fn stream_stalling_after_start_times_out_with_command_timeout() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v2/sandboxes"))
        .respond_with(ResponseTemplate::new(201).set_body_json(common::sandbox_json("0.5.7")))
        .mount(&server)
        .await;
    let config = e2b_client::ConnectionConfig::builder()
        .api_key(common::API_KEY)
        .api_url(server.uri())
        .sandbox_url(stalling_envd().await)
        .domain("example.test")
        .request_timeout(None)
        .build()
        .expect("config");
    let sandbox = e2b_client::Client::new(config)
        .expect("client")
        .create_sandbox(&e2b_client::CreateSandboxParams::default())
        .await
        .expect("create");
    let options = CommandOptions::default().timeout(Some(Duration::from_millis(300)));

    let err = sandbox
        .commands()
        .run("sleep 60", &options)
        .await
        .expect_err("error");

    assert!(matches!(err, Error::Timeout(_)), "{err:?}");
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
        .and(|request: &Request| {
            decode_request_envelope(&request.body) == json!({"process": {"pid": 42}})
        })
        .respond_with(stream_response(&start_frames(0)))
        .expect(1)
        .mount(&server)
        .await;

    let handle = sandbox
        .commands()
        .connect(42, Some(Duration::from_secs(5)))
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
        matches!(err, Error::Rpc { code: ConnectCode::Unavailable, ref message }
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
        .read("/tmp/a b.txt", None)
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
        .read_to_string("/nope", None)
        .await
        .expect_err("error");

    assert!(
        matches!(err, Error::Envd { status: 404, ref message } if message == "file not found"),
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
        .write("/tmp/out.txt", "file contents", Some("root"))
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
        .write("/x", vec![1_u8, 2, 3], None)
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
        .write("/x", "data", None)
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

    let entries = files.list("/tmp", 2, None).await.expect("list");
    let info = files.get_info("/tmp/a.txt", None).await.expect("stat");

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

    let err = sandbox.files().list("/", 0, None).await.expect_err("error");

    assert!(matches!(err, Error::InvalidArgument(_)), "{err:?}");
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

    assert!(files.exists("/present", None).await.expect("present"));
    assert!(!files.exists("/absent", None).await.expect("absent"));
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

    assert!(files.make_dir("/new", None).await.expect("new"));
    assert!(!files.make_dir("/old", None).await.expect("old"));
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
        .rename("/a", "/b", Some("root"))
        .await
        .expect("rename");
    files.remove("/b", None).await.expect("remove");

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
        .get_info("/x", None)
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
