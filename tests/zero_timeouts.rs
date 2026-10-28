//! A zero timeout means "no limit" on every stream deadline and request
//! timeout, as SDK 2.61's `timeout=0` (`py: envd/utils.py:15-24`) and
//! `request_timeout=0` (`py: api/__init__.py:280`) do (CLM-0172, #52).

mod common;

use std::time::Duration;

use common::{
    client_with_timeout, end_stream, message, sandbox, sandbox_from_json_with_client, sandbox_json,
    stream_response,
};
use e2b_client::{
    CommandOptions, ListOptions, PtyOptions, PtySize, StreamInputOptions, WatchOptions,
};
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

/// The one request `server` received on `route`.
async fn request_to(server: &MockServer, route: &str) -> Request {
    server
        .received_requests()
        .await
        .expect("recording")
        .into_iter()
        .find(|request| request.url.path() == route)
        .unwrap_or_else(|| panic!("no request to {route}"))
}

/// A process stream: start, then a clean exit.
fn process_frames() -> Vec<Vec<u8>> {
    vec![
        message(json!({"event": {"start": {"pid": 7}}})),
        message(json!({"event": {"end": {"exitCode": 0, "exited": true}}})),
        end_stream(json!({})),
    ]
}

#[tokio::test]
async fn a_zero_command_timeout_sends_no_deadline() {
    // Arrange
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    Mock::given(method("POST"))
        .and(path("/process.Process/Start"))
        .respond_with(stream_response(&process_frames()))
        .expect(1)
        .mount(&server)
        .await;
    let options = CommandOptions::default().timeout(Some(Duration::ZERO));

    // Act
    let result = sandbox.commands().run("true", &options).await;

    // Assert: the stream was not cut off, and no deadline was sent.
    assert_eq!(result.expect("run").exit_code, 0);
    let start = request_to(&server, "/process.Process/Start").await;
    assert!(
        !start.headers.contains_key("connect-timeout-ms"),
        "a zero timeout must send no Connect-Timeout-Ms"
    );
}

#[tokio::test]
async fn a_zero_pty_timeout_sends_no_deadline() {
    // Arrange
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    Mock::given(method("POST"))
        .and(path("/process.Process/Start"))
        .respond_with(stream_response(&process_frames()))
        .expect(1)
        .mount(&server)
        .await;
    let options = PtyOptions::default().timeout(Some(Duration::ZERO));

    // Act
    let handle = sandbox
        .pty()
        .create(PtySize::new(80, 24), &options)
        .await
        .expect("create");
    let exit = handle.wait().await;

    // Assert
    assert_eq!(exit.expect("wait").exit_code, 0);
    let start = request_to(&server, "/process.Process/Start").await;
    assert!(!start.headers.contains_key("connect-timeout-ms"));
}

#[tokio::test]
async fn a_zero_command_connect_timeout_sends_no_deadline() {
    // Arrange
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    Mock::given(method("POST"))
        .and(path("/process.Process/Connect"))
        .respond_with(stream_response(&process_frames()))
        .expect(1)
        .mount(&server)
        .await;

    // Act
    let handle = sandbox
        .commands()
        .connect(7, Some(Duration::ZERO))
        .await
        .expect("connect");
    let result = handle.wait().await;

    // Assert
    assert_eq!(result.expect("wait").exit_code, 0);
    let connect = request_to(&server, "/process.Process/Connect").await;
    assert!(!connect.headers.contains_key("connect-timeout-ms"));
}

#[tokio::test]
async fn a_zero_pty_connect_timeout_sends_no_deadline() {
    // Arrange
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    Mock::given(method("POST"))
        .and(path("/process.Process/Connect"))
        .respond_with(stream_response(&process_frames()))
        .expect(1)
        .mount(&server)
        .await;

    // Act
    let handle = sandbox
        .pty()
        .connect(7, Some(Duration::ZERO))
        .await
        .expect("connect");
    let exit = handle.wait().await;

    // Assert
    assert_eq!(exit.expect("wait").exit_code, 0);
    let connect = request_to(&server, "/process.Process/Connect").await;
    assert!(!connect.headers.contains_key("connect-timeout-ms"));
}

#[tokio::test]
async fn a_zero_watch_timeout_sends_no_deadline() {
    // Arrange: the watch opens, then envd ends it cleanly.
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    Mock::given(method("POST"))
        .and(path("/filesystem.Filesystem/WatchDir"))
        .respond_with(stream_response(&[
            message(json!({"start": {}})),
            end_stream(json!({})),
        ]))
        .expect(1)
        .mount(&server)
        .await;
    let options = WatchOptions::default().timeout(Some(Duration::ZERO));

    // Act
    let mut watch = sandbox
        .files()
        .watch_dir("/w", &options)
        .await
        .expect("watch");
    let end = watch.next_event().await;

    // Assert: a clean end, not a deadline error.
    assert!(matches!(end, Ok(None)), "{end:?}");
    let request = request_to(&server, "/filesystem.Filesystem/WatchDir").await;
    assert!(!request.headers.contains_key("connect-timeout-ms"));
}

#[tokio::test]
async fn a_zero_stream_input_timeout_sends_no_deadline() {
    // Arrange
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
    let options = StreamInputOptions::default().timeout(Some(Duration::ZERO));

    // Act
    let mut input = sandbox.commands().stream_input(7, &options);
    input.send("x").await.expect("send");
    let closed = input.close().await;

    // Assert
    assert!(closed.is_ok(), "{closed:?}");
    let request = request_to(&server, "/process.Process/StreamInput").await;
    assert!(!request.headers.contains_key("connect-timeout-ms"));
}

#[tokio::test]
async fn a_zero_request_timeout_in_the_config_means_no_limit() {
    // Arrange: a configured request timeout of zero, and a unary call that
    // answers after 300 ms.
    let server = MockServer::start().await;
    let client = client_with_timeout(&server, Some(Duration::ZERO));
    let sandbox = sandbox_from_json_with_client(&server, &client, sandbox_json("0.5.7")).await;
    Mock::given(method("POST"))
        .and(path("/filesystem.Filesystem/ListDir"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"entries": []}))
                .set_delay(Duration::from_millis(300)),
        )
        .expect(1)
        .mount(&server)
        .await;

    // Act: list with the default options (depth 1, configured timeout).
    let entries = sandbox.files().list("/w", &ListOptions::default()).await;

    // Assert: neither a deadline header nor a client-side timeout applied.
    assert!(entries.expect("list").is_empty());
    let request = request_to(&server, "/filesystem.Filesystem/ListDir").await;
    assert!(!request.headers.contains_key("connect-timeout-ms"));
    assert_eq!(client.config().request_timeout(), None);
}
