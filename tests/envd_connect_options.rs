//! Mock tests for the options of `Commands::connect` and `Pty::connect`, and
//! for the per-call request timeout on every Commands/Pty call that takes
//! options (CLM-0174; zero as "no limit", CLM-0172).

mod common;

use std::time::Duration;

use common::{
    client_with_timeout, end_stream, message, sandbox, sandbox_from_json_with_client, sandbox_json,
    stream_response,
};
use e2b_client::{
    CommandConnectOptions, CommandOptions, Error, PtyConnectOptions, PtyOptions, PtySize, Sandbox,
};
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request};

/// A process stream: start (pid 42), one stdout chunk, then the end event.
fn frames() -> Vec<Vec<u8>> {
    vec![
        message(json!({"event": {"start": {"pid": 42}}})),
        message(json!({"event": {"data": {"stdout": "aGVsbG8K"}}})),
        message(json!({"event": {"end": {"exitCode": 0, "exited": true}}})),
        end_stream(json!({})),
    ]
}

/// Mounts `route` answering `frames()` after `delay`.
async fn mount_stream(server: &MockServer, route: &str, delay: Duration) {
    Mock::given(method("POST"))
        .and(path(route))
        .respond_with(stream_response(&frames()).set_delay(delay))
        .mount(server)
        .await;
}

/// The `Connect-Timeout-Ms` header of the one request sent to `route`.
async fn deadline_header(server: &MockServer, route: &str) -> Option<String> {
    let requests: Vec<Request> = server.received_requests().await.expect("requests");
    let request = requests
        .iter()
        .find(|request| request.url.path() == route)
        .expect("request to route");
    request
        .headers
        .get("connect-timeout-ms")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
}

/// A sandbox whose client has the given configured request timeout.
async fn sandbox_with_request_timeout(server: &MockServer, timeout: Duration) -> Sandbox {
    let client = client_with_timeout(server, Some(timeout));
    sandbox_from_json_with_client(server, &client, sandbox_json("0.5.7")).await
}

#[tokio::test]
async fn command_connect_defaults_to_the_sdk_sixty_seconds() {
    // Arrange
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    mount_stream(&server, "/process.Process/Connect", Duration::ZERO).await;

    // Act
    let handle = sandbox
        .commands()
        .connect(42, &CommandConnectOptions::default())
        .await
        .expect("connect");

    // Assert
    assert_eq!(handle.pid(), 42);
    assert_eq!(
        deadline_header(&server, "/process.Process/Connect")
            .await
            .as_deref(),
        Some("60000")
    );
}

#[tokio::test]
async fn command_connect_sends_the_chosen_deadline() {
    // Arrange
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    mount_stream(&server, "/process.Process/Connect", Duration::ZERO).await;

    // Act
    let options = CommandConnectOptions::new().timeout(Some(Duration::from_secs(5)));
    sandbox
        .commands()
        .connect(42, &options)
        .await
        .expect("connect");

    // Assert
    assert_eq!(
        deadline_header(&server, "/process.Process/Connect")
            .await
            .as_deref(),
        Some("5000")
    );
}

#[tokio::test]
async fn command_connect_without_a_deadline_sends_no_header() {
    // Arrange
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    mount_stream(&server, "/process.Process/Connect", Duration::ZERO).await;

    // Act
    let options = CommandConnectOptions::new().timeout(None);
    sandbox
        .commands()
        .connect(42, &options)
        .await
        .expect("connect");

    // Assert
    assert_eq!(
        deadline_header(&server, "/process.Process/Connect").await,
        None
    );
}

#[tokio::test]
async fn pty_connect_has_no_deadline_by_default() {
    // Arrange
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    mount_stream(&server, "/process.Process/Connect", Duration::ZERO).await;

    // Act
    sandbox
        .pty()
        .connect(42, &PtyConnectOptions::default())
        .await
        .expect("connect");

    // Assert
    assert_eq!(
        deadline_header(&server, "/process.Process/Connect").await,
        None
    );
}

#[tokio::test]
async fn pty_create_sends_a_deadline_only_when_set() {
    // Arrange
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    mount_stream(&server, "/process.Process/Start", Duration::ZERO).await;

    // Act
    let options = PtyOptions::default().timeout(Some(e2b_client::DEFAULT_PTY_TIMEOUT));
    sandbox
        .pty()
        .create(PtySize::new(80, 24), &options)
        .await
        .expect("create");

    // Assert
    assert_eq!(
        deadline_header(&server, "/process.Process/Start")
            .await
            .as_deref(),
        Some("60000")
    );
}

#[tokio::test]
async fn a_shorter_request_timeout_bounds_the_wait_for_the_start_event() {
    // Arrange: the client allows 5 s, the call only 200 ms, and envd takes 2 s.
    let server = MockServer::start().await;
    let sandbox = sandbox_with_request_timeout(&server, Duration::from_secs(5)).await;
    mount_stream(&server, "/process.Process/Connect", Duration::from_secs(2)).await;

    // Act
    let options = CommandConnectOptions::new().request_timeout(Some(Duration::from_millis(200)));
    let error = sandbox
        .commands()
        .connect(42, &options)
        .await
        .expect_err("the per-call request timeout must expire");

    // Assert
    assert!(
        matches!(error, Error::Timeout(limit) if limit == Duration::from_millis(200)),
        "{error:?}"
    );
}

#[tokio::test]
async fn a_longer_request_timeout_overrides_the_configured_one() {
    // Arrange: the client allows 200 ms, the call 5 s, and envd takes 1 s.
    let server = MockServer::start().await;
    let sandbox = sandbox_with_request_timeout(&server, Duration::from_millis(200)).await;
    mount_stream(&server, "/process.Process/Connect", Duration::from_secs(1)).await;

    // Act
    let options = PtyConnectOptions::new().request_timeout(Some(Duration::from_secs(5)));
    let handle = sandbox.pty().connect(42, &options).await;

    // Assert
    assert_eq!(
        handle.expect("connect within the per-call timeout").pid(),
        42
    );
}

#[tokio::test]
async fn a_zero_request_timeout_means_no_limit() {
    // Arrange: the client allows 200 ms; zero on the call lifts the bound.
    let server = MockServer::start().await;
    let sandbox = sandbox_with_request_timeout(&server, Duration::from_millis(200)).await;
    mount_stream(&server, "/process.Process/Connect", Duration::from_secs(1)).await;

    // Act
    let options = CommandConnectOptions::new().request_timeout(Some(Duration::ZERO));
    let handle = sandbox.commands().connect(42, &options).await;

    // Assert
    assert_eq!(handle.expect("connect with no request timeout").pid(), 42);
}

#[tokio::test]
async fn command_options_request_timeout_bounds_start() {
    // Arrange
    let server = MockServer::start().await;
    let sandbox = sandbox_with_request_timeout(&server, Duration::from_secs(5)).await;
    mount_stream(&server, "/process.Process/Start", Duration::from_secs(2)).await;

    // Act
    let options = CommandOptions::default().request_timeout(Some(Duration::from_millis(200)));
    let error = sandbox
        .commands()
        .start("true", &options)
        .await
        .expect_err("the per-call request timeout must expire");

    // Assert
    assert!(
        matches!(error, Error::Timeout(limit) if limit == Duration::from_millis(200)),
        "{error:?}"
    );
}

#[tokio::test]
async fn pty_options_request_timeout_bounds_create() {
    // Arrange
    let server = MockServer::start().await;
    let sandbox = sandbox_with_request_timeout(&server, Duration::from_secs(5)).await;
    mount_stream(&server, "/process.Process/Start", Duration::from_secs(2)).await;

    // Act
    let options = PtyOptions::default().request_timeout(Some(Duration::from_millis(200)));
    let error = sandbox
        .pty()
        .create(PtySize::new(80, 24), &options)
        .await
        .expect_err("the per-call request timeout must expire");

    // Assert
    assert!(
        matches!(error, Error::Timeout(limit) if limit == Duration::from_millis(200)),
        "{error:?}"
    );
}

/// Not evidence for CLM-0174: wiremock cannot delay part of a body, so this
/// only checks that the two values travel separately (the header carries the
/// stream deadline, and the 200 ms request timeout does not end the read).
#[tokio::test]
async fn the_stream_deadline_is_not_the_request_timeout() {
    // Arrange: the request timeout is 200 ms and the stream deadline 5 s.
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    mount_stream(&server, "/process.Process/Connect", Duration::ZERO).await;

    // Act
    let options = CommandConnectOptions::new()
        .timeout(Some(Duration::from_secs(5)))
        .request_timeout(Some(Duration::from_millis(200)));
    let result = sandbox
        .commands()
        .connect(42, &options)
        .await
        .expect("connect")
        .wait()
        .await
        .expect("wait");

    // Assert
    assert_eq!(result.stdout, "hello\n");
    assert_eq!(
        deadline_header(&server, "/process.Process/Connect")
            .await
            .as_deref(),
        Some("5000")
    );
}
