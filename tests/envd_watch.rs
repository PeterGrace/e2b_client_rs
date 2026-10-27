//! Mock tests for `Filesystem::watch_dir` (`filesystem.Filesystem/WatchDir`).

mod common;

use std::time::Duration;

use common::{
    decode_request_envelope, end_stream, message, sandbox, sandbox_with_version, stream_response,
};
use e2b_client::{Error, FilesystemEventType, WatchOptions};
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const ROUTE: &str = "/filesystem.Filesystem/WatchDir";

#[tokio::test]
async fn watch_dir_sends_options_and_maps_events() {
    // Arrange: a start event, a create, a keepalive, an unspecified type
    // (dropped), a write by number with its entry, then a clean end.
    let server = MockServer::start().await;
    let sandbox = sandbox_with_version(&server, "0.6.4").await;
    Mock::given(method("POST"))
        .and(path(ROUTE))
        .respond_with(stream_response(&[
            message(json!({"start": {}})),
            message(json!({"filesystem": {"name": "a.txt", "type": "EVENT_TYPE_CREATE"}})),
            message(json!({"keepalive": {}})),
            message(json!({"filesystem": {"name": "a.txt", "type": "EVENT_TYPE_UNSPECIFIED"}})),
            message(json!({"filesystem": {"name": "a.txt", "type": 2,
                "entry": {"name": "a.txt", "type": "FILE_TYPE_FILE", "path": "/w/a.txt",
                    "size": "3"}}})),
            end_stream(json!({})),
        ]))
        .expect(1)
        .mount(&server)
        .await;
    let options = WatchOptions::default()
        .recursive(true)
        .include_entry(true)
        .allow_network_mounts(true)
        .user("root");

    // Act
    let mut watch = sandbox
        .files()
        .watch_dir("/w", &options)
        .await
        .expect("watch");
    let create = watch
        .next_event()
        .await
        .expect("first event")
        .expect("create");
    let write = watch
        .next_event()
        .await
        .expect("second event")
        .expect("write");
    let end = watch.next_event().await.expect("clean end");

    // Assert
    assert_eq!(
        (create.name.as_str(), create.event_type),
        ("a.txt", FilesystemEventType::Create)
    );
    assert!(create.entry.is_none());
    assert_eq!(write.event_type, FilesystemEventType::Write);
    let entry = write.entry.expect("entry");
    assert_eq!((entry.path.as_str(), entry.size), ("/w/a.txt", 3));
    assert!(end.is_none());
    let requests = server.received_requests().await.expect("requests");
    let watch_request = requests
        .iter()
        .find(|request| request.url.path() == ROUTE)
        .expect("WatchDir request");
    assert_eq!(
        decode_request_envelope(&watch_request.body),
        json!({"path": "/w", "recursive": true, "includeEntry": true, "allowNetworkMounts": true})
    );
    let header = |name: &str| {
        watch_request
            .headers
            .get(name)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned)
    };
    assert_eq!(
        header("content-type").as_deref(),
        Some("application/connect+json")
    );
    assert_eq!(header("keepalive-ping-interval").as_deref(), Some("50"));
    assert_eq!(header("connect-timeout-ms").as_deref(), Some("60000"));
    // base64("root:")
    assert_eq!(header("authorization").as_deref(), Some("Basic cm9vdDo="));
}

#[tokio::test]
async fn watch_dir_options_are_refused_on_old_envd_without_a_request() {
    for (version, options) in [
        ("0.1.3", WatchOptions::default().recursive(true)),
        ("0.6.2", WatchOptions::default().include_entry(true)),
        ("0.6.3", WatchOptions::default().allow_network_mounts(true)),
    ] {
        // Arrange: one server per version, so each request log starts empty.
        let server = MockServer::start().await;
        let sandbox = sandbox_with_version(&server, version).await;

        // Act
        let err = sandbox
            .files()
            .watch_dir("/w", &options)
            .await
            .expect_err(version);

        // Assert
        assert!(
            matches!(err, Error::InvalidArgument(_)),
            "{version}: {err:?}"
        );
        let requests = server.received_requests().await.expect("requests");
        assert!(
            requests.iter().all(|request| request.url.path() != ROUTE),
            "{version}: a WatchDir request was sent"
        );
    }
}

#[tokio::test]
async fn watch_dir_must_open_with_a_start_event() {
    // Arrange: the stream's first message is an event, not `start`.
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    Mock::given(method("POST"))
        .and(path(ROUTE))
        .respond_with(stream_response(&[
            message(json!({"filesystem": {"name": "x", "type": "EVENT_TYPE_WRITE"}})),
            end_stream(json!({})),
        ]))
        .mount(&server)
        .await;

    // Act
    let err = sandbox
        .files()
        .watch_dir("/w", &WatchOptions::default())
        .await
        .expect_err("no start");

    // Assert
    assert!(matches!(err, Error::Protocol(_)), "{err:?}");
}

#[tokio::test]
async fn watch_dir_open_is_bounded_by_the_request_timeout() {
    // Arrange: envd takes 5 s to answer, the request timeout is 200 ms and
    // the watch timeout keeps its 60 s default.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v2/sandboxes"))
        .respond_with(ResponseTemplate::new(201).set_body_json(common::sandbox_json("0.6.4")))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path(ROUTE))
        .respond_with(
            stream_response(&[message(json!({"start": {}})), end_stream(json!({}))])
                .set_delay(Duration::from_secs(5)),
        )
        .mount(&server)
        .await;
    let client = common::client_with_timeout(&server, Some(Duration::from_millis(200)));
    let sandbox = client
        .create_sandbox(&e2b_client::CreateSandboxParams::default())
        .await
        .expect("create sandbox");

    // Act
    let err = sandbox
        .files()
        .watch_dir("/w", &WatchOptions::default())
        .await
        .expect_err("slow open");

    // Assert
    assert!(
        matches!(err, Error::Timeout(limit) if limit == Duration::from_millis(200)),
        "{err:?}"
    );
}
