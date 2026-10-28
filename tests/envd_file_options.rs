//! Mock tests for the filesystem options (CLM-0173): `FileOptions`,
//! `ListOptions`, and the per-call request timeout on them, on
//! `WriteOptions` and on `WatchOptions`.

mod common;

use std::time::{Duration, Instant};

use common::{client_with_timeout, sandbox, sandbox_from_json_with_client, sandbox_json};
use e2b_client::{Error, FileOptions, ListOptions, WatchOptions, WriteOptions};
use serde_json::json;
use wiremock::matchers::{body_json, header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

const STAT: &str = "/filesystem.Filesystem/Stat";
const SHORT: Duration = Duration::from_millis(200);

/// A `Stat` answer for `/a` that arrives after `delay`.
fn slow_stat(delay: Duration) -> ResponseTemplate {
    ResponseTemplate::new(200)
        .set_body_json(json!({"entry": {"name": "a", "type": "FILE_TYPE_FILE", "path": "/a"}}))
        .set_delay(delay)
}

/// Whether `error` is a transport-level timeout (reqwest's own deadline).
fn is_transport_timeout(error: &Error) -> bool {
    matches!(error, Error::Transport(transport) if transport.is_timeout())
}

#[tokio::test]
async fn a_per_call_request_timeout_overrides_the_configured_one() {
    // Arrange: the configured timeout is 5 s; Stat answers after 2 s.
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    Mock::given(method("POST"))
        .and(path(STAT))
        .respond_with(slow_stat(Duration::from_secs(2)))
        .mount(&server)
        .await;
    let options = FileOptions::new().request_timeout(Some(SHORT));

    // Act
    let started = Instant::now();
    let error = sandbox
        .files()
        .get_info("/a", &options)
        .await
        .expect_err("the 200 ms per-call timeout passes first");

    // Assert: the call gave up near 200 ms, and envd was told the deadline.
    assert!(is_transport_timeout(&error), "{error:?}");
    assert!(
        started.elapsed() < Duration::from_secs(1),
        "{:?}",
        started.elapsed()
    );
    let requests = server.received_requests().await.expect("recording");
    let stat = requests
        .iter()
        .find(|r| r.url.path() == STAT)
        .expect("Stat request");
    assert_eq!(
        stat.headers
            .get("connect-timeout-ms")
            .and_then(|v| v.to_str().ok()),
        Some("200")
    );
}

#[tokio::test]
async fn a_per_call_none_lifts_a_short_configured_timeout() {
    // Arrange: the configured timeout is 200 ms; Stat answers after 600 ms.
    let server = MockServer::start().await;
    let client = client_with_timeout(&server, Some(SHORT));
    let sandbox = sandbox_from_json_with_client(&server, &client, sandbox_json("0.5.7")).await;
    Mock::given(method("POST"))
        .and(path(STAT))
        .respond_with(slow_stat(Duration::from_millis(600)))
        .mount(&server)
        .await;
    let files = sandbox.files();

    // Act
    let default = files.get_info("/a", &FileOptions::default()).await;
    let unbounded = files
        .get_info("/a", &FileOptions::new().request_timeout(None))
        .await;

    // Assert: the configured 200 ms applies by default; `None` lifts it.
    assert!(is_transport_timeout(
        &default.expect_err("default times out")
    ));
    assert_eq!(unbounded.expect("no limit").path, "/a");
}

#[tokio::test]
async fn a_zero_per_call_request_timeout_means_no_limit() {
    // Arrange: as above, with zero instead of `None` (CLM-0172).
    let server = MockServer::start().await;
    let client = client_with_timeout(&server, Some(SHORT));
    let sandbox = sandbox_from_json_with_client(&server, &client, sandbox_json("0.5.7")).await;
    Mock::given(method("POST"))
        .and(path(STAT))
        .respond_with(slow_stat(Duration::from_millis(600)))
        .mount(&server)
        .await;

    // Act
    let info = sandbox
        .files()
        .get_info(
            "/a",
            &FileOptions::new().request_timeout(Some(Duration::ZERO)),
        )
        .await;

    // Assert: no deadline at all, so no header either.
    assert_eq!(info.expect("zero is no limit").path, "/a");
    let requests = server.received_requests().await.expect("recording");
    let stat = requests
        .iter()
        .find(|r| r.url.path() == STAT)
        .expect("Stat request");
    assert!(!stat.headers.contains_key("connect-timeout-ms"));
}

#[tokio::test]
async fn read_and_write_use_the_per_call_request_timeout() {
    // Arrange: both `/files` routes answer after 2 s; the client allows 5 s.
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    Mock::given(method("GET"))
        .and(path("/files"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("x")
                .set_delay(Duration::from_secs(2)),
        )
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/files"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!([{"name": "a", "type": "file", "path": "/a"}]))
                .set_delay(Duration::from_secs(2)),
        )
        .mount(&server)
        .await;
    let files = sandbox.files();

    // Act
    let read = files
        .read("/a", &FileOptions::new().request_timeout(Some(SHORT)))
        .await;
    let write = files
        .write(
            "/a",
            "x",
            &WriteOptions::default().request_timeout(Some(SHORT)),
        )
        .await;

    // Assert
    assert!(is_transport_timeout(&read.expect_err("read")));
    assert!(is_transport_timeout(&write.expect_err("write")));
}

#[tokio::test]
async fn watch_dir_opens_within_the_per_call_request_timeout() {
    // Arrange: WatchDir answers after 2 s; the client allows 5 s.
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    Mock::given(method("POST"))
        .and(path("/filesystem.Filesystem/WatchDir"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(2)))
        .mount(&server)
        .await;
    let options = WatchOptions::default().request_timeout(Some(SHORT));

    // Act
    let error = sandbox
        .files()
        .watch_dir("/w", &options)
        .await
        .expect_err("the watch does not open within 200 ms");

    // Assert: opening is bounded client-side, which reports `Error::Timeout`.
    assert!(
        matches!(error, Error::Timeout(limit) if limit == SHORT),
        "{error:?}"
    );
}

#[tokio::test]
async fn file_options_select_the_user() {
    // Arrange: the RPC carries the user in Authorization, the file API in
    // the `username` query.
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    Mock::given(method("POST"))
        .and(path(STAT))
        // base64("root:")
        .and(header("authorization", "Basic cm9vdDo="))
        .respond_with(slow_stat(Duration::ZERO))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/files"))
        .and(query_param("path", "/a"))
        .and(query_param("username", "root"))
        .respond_with(ResponseTemplate::new(200).set_body_string("x"))
        .expect(1)
        .mount(&server)
        .await;
    let root = FileOptions::new().user("root");

    // Act
    let info = sandbox.files().get_info("/a", &root).await;
    let text = sandbox.files().read_to_string("/a", &root).await;

    // Assert: each matched only its user-carrying mock.
    assert_eq!(info.expect("stat as root").path, "/a");
    assert_eq!(text.expect("read as root"), "x");
}

#[tokio::test]
async fn list_options_default_to_depth_one_and_reject_zero() {
    // Arrange
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    Mock::given(method("POST"))
        .and(path("/filesystem.Filesystem/ListDir"))
        .and(body_json(json!({"path": "/w", "depth": 1})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"entries": []})))
        .expect(1)
        .mount(&server)
        .await;
    let files = sandbox.files();

    // Act
    let listed = files.list("/w", &ListOptions::default()).await;
    let zero = files.list("/w", &ListOptions::new().depth(0)).await;

    // Assert
    assert!(listed.expect("depth 1").is_empty());
    assert!(matches!(zero, Err(Error::InvalidArgument(_))), "{zero:?}");
}
