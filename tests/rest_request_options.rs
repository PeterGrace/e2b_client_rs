//! Mock tests for the per-call request timeout on REST calls (CLM-0182):
//! `RequestOptions` on calls with no other options, and the
//! `request_timeout` setter on the REST options and params structs.

mod common;

use std::time::{Duration, Instant};

use common::{client_with_timeout, detail_json, sandbox_from_json_with_client, sandbox_json};
use e2b_client::{
    ConnectOptions, CreateSandboxParams, Error, ForkOptions, ListSandboxesParams,
    ListSnapshotsParams, MetricsParams, NetworkUpdate, PauseOptions, RequestOptions,
    SnapshotOptions,
};
use serde_json::json;
use wiremock::matchers::{any, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// The per-call timeout the tests set: well under every server delay.
const SHORT: Duration = Duration::from_millis(200);
/// How long the "slow" routes take to answer: well over [`SHORT`], well
/// under the 5 s configured timeout.
const SLOW: Duration = Duration::from_secs(2);

/// Whether `error` is a transport-level timeout (reqwest's own deadline).
fn is_transport_timeout(error: &Error) -> bool {
    matches!(error, Error::Transport(transport) if transport.is_timeout())
}

/// Asserts that every labelled call failed with a transport timeout.
///
/// # Arguments
///
/// * `results` - each call's name and its error, `None` if it succeeded.
fn assert_all_timed_out(results: Vec<(&str, Option<Error>)>) {
    for (call, error) in results {
        let error = error.unwrap_or_else(|| panic!("{call}: expected a timeout"));
        assert!(is_transport_timeout(&error), "{call}: {error:?}");
    }
}

/// Mounts `GET /sandboxes/sbx-1` answering after `delay`.
async fn slow_detail(server: &MockServer, delay: Duration) {
    Mock::given(method("GET"))
        .and(path("/sandboxes/sbx-1"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(detail_json(json!({})))
                .set_delay(delay),
        )
        .mount(server)
        .await;
}

/// Mounts a route answering every request with `{}` after [`SLOW`]. The
/// bodies never matter: every call gives up first.
async fn everything_slow(server: &MockServer) {
    Mock::given(any())
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({}))
                .set_delay(SLOW),
        )
        .mount(server)
        .await;
}

#[tokio::test]
async fn a_per_call_request_timeout_overrides_the_configured_one() {
    // Arrange: configured 5 s; the API answers after 2 s.
    let server = MockServer::start().await;
    slow_detail(&server, SLOW).await;
    let client = client_with_timeout(&server, Some(Duration::from_secs(5)));
    let options = RequestOptions::new().request_timeout(Some(SHORT));

    // Act
    let started = Instant::now();
    let error = client
        .get_sandbox_info("sbx-1", &options)
        .await
        .expect_err("the 200 ms per-call timeout passes first");

    // Assert
    assert!(is_transport_timeout(&error), "{error:?}");
    assert!(
        started.elapsed() < Duration::from_secs(1),
        "{:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn a_per_call_none_lifts_a_short_configured_timeout() {
    // Arrange: configured 200 ms; the API answers after 500 ms.
    let server = MockServer::start().await;
    slow_detail(&server, Duration::from_millis(500)).await;
    let client = client_with_timeout(&server, Some(SHORT));

    // Act
    let info = client
        .get_sandbox_info("sbx-1", &RequestOptions::new().request_timeout(None))
        .await;

    // Assert
    assert!(info.is_ok(), "{info:?}");
}

#[tokio::test]
async fn a_zero_per_call_request_timeout_means_no_limit() {
    // Arrange: configured 200 ms; the API answers after 500 ms.
    let server = MockServer::start().await;
    slow_detail(&server, Duration::from_millis(500)).await;
    let client = client_with_timeout(&server, Some(SHORT));
    let options = RequestOptions::new().request_timeout(Some(Duration::ZERO));

    // Act
    let info = client.get_sandbox_info("sbx-1", &options).await;

    // Assert
    assert!(info.is_ok(), "{info:?}");
}

#[tokio::test]
async fn unset_options_keep_the_configured_timeout() {
    // Arrange: configured 200 ms; the API answers after 500 ms.
    let server = MockServer::start().await;
    slow_detail(&server, Duration::from_millis(500)).await;
    let client = client_with_timeout(&server, Some(SHORT));

    // Act
    let error = client
        .get_sandbox_info("sbx-1", &RequestOptions::new())
        .await
        .expect_err("the configured 200 ms applies");

    // Assert
    assert!(is_transport_timeout(&error), "{error:?}");
}

#[tokio::test]
async fn every_client_call_takes_a_per_call_request_timeout() {
    // Arrange: configured 5 s; every route answers after 2 s.
    let server = MockServer::start().await;
    everything_slow(&server).await;
    let client = client_with_timeout(&server, Some(Duration::from_secs(5)));
    let short = RequestOptions::new().request_timeout(Some(SHORT));
    let update = NetworkUpdate::new();

    // Act: `.err()` turns each `Result` into the `Option` of its error.
    let started = Instant::now();
    let results = vec![
        (
            "create_sandbox",
            client
                .create_sandbox(&CreateSandboxParams::new("base").request_timeout(Some(SHORT)))
                .await
                .err(),
        ),
        (
            "connect_sandbox",
            client
                .connect_sandbox("sbx-1", &ConnectOptions::new().request_timeout(Some(SHORT)))
                .await
                .err(),
        ),
        (
            "get_sandbox_info",
            client.get_sandbox_info("sbx-1", &short).await.err(),
        ),
        (
            "get_sandbox_metrics",
            client
                .get_sandbox_metrics("sbx-1", &MetricsParams::new().request_timeout(Some(SHORT)))
                .await
                .err(),
        ),
        (
            "list_sandboxes",
            client
                .list_sandboxes(&ListSandboxesParams::new().request_timeout(Some(SHORT)))
                .await
                .err(),
        ),
        (
            "kill_sandbox",
            client.kill_sandbox("sbx-1", &short).await.err(),
        ),
        (
            "pause_sandbox",
            client
                .pause_sandbox("sbx-1", &PauseOptions::new().request_timeout(Some(SHORT)))
                .await
                .err(),
        ),
        (
            "set_sandbox_timeout",
            client
                .set_sandbox_timeout("sbx-1", Duration::from_secs(60), &short)
                .await
                .err(),
        ),
        (
            "update_sandbox_network",
            client
                .update_sandbox_network("sbx-1", &update, &short)
                .await
                .err(),
        ),
        (
            "refresh_sandbox",
            client.refresh_sandbox("sbx-1", None, &short).await.err(),
        ),
        (
            "create_snapshot",
            client
                .create_snapshot(
                    "sbx-1",
                    &SnapshotOptions::new().request_timeout(Some(SHORT)),
                )
                .await
                .err(),
        ),
        (
            "list_snapshots",
            client
                .list_snapshots(&ListSnapshotsParams::new().request_timeout(Some(SHORT)))
                .await
                .err(),
        ),
        (
            "delete_snapshot",
            client.delete_snapshot("snap-1", &short).await.err(),
        ),
        (
            "fork_sandbox",
            client
                .fork_sandbox("sbx-1", &ForkOptions::new().request_timeout(Some(SHORT)))
                .await
                .err(),
        ),
        (
            "create_volume",
            client.create_volume("data", &short).await.err(),
        ),
        ("get_volume", client.get_volume("vol-1", &short).await.err()),
        ("list_volumes", client.list_volumes(&short).await.err()),
        (
            "destroy_volume",
            client.destroy_volume("vol-1", &short).await.err(),
        ),
    ];

    // Assert: each gave up on its own 200 ms, so the 18 calls took well
    // under the 36 s the server's delay would need.
    assert_all_timed_out(results);
    assert!(
        started.elapsed() < Duration::from_secs(15),
        "{:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn every_sandbox_call_passes_its_request_timeout_through() {
    // Arrange: a sandbox handle created quickly, then every route (REST and
    // envd's health check) answering after 2 s; configured 5 s.
    let server = MockServer::start().await;
    let client = client_with_timeout(&server, Some(Duration::from_secs(5)));
    let sandbox = sandbox_from_json_with_client(&server, &client, sandbox_json("0.5.7")).await;
    everything_slow(&server).await;
    let short = RequestOptions::new().request_timeout(Some(SHORT));
    let update = NetworkUpdate::new();

    // Act
    let results = vec![
        ("is_running", sandbox.is_running(&short).await.err()),
        ("kill", sandbox.kill(&short).await.err()),
        (
            "pause",
            sandbox
                .pause(&PauseOptions::new().request_timeout(Some(SHORT)))
                .await
                .err(),
        ),
        (
            "set_timeout",
            sandbox
                .set_timeout(Duration::from_secs(60), &short)
                .await
                .err(),
        ),
        (
            "update_network",
            sandbox.update_network(&update, &short).await.err(),
        ),
        ("refresh", sandbox.refresh(None, &short).await.err()),
        ("get_info", sandbox.get_info(&short).await.err()),
        (
            "get_metrics",
            sandbox
                .get_metrics(&MetricsParams::new().request_timeout(Some(SHORT)))
                .await
                .err(),
        ),
        (
            "create_snapshot",
            sandbox
                .create_snapshot(&SnapshotOptions::new().request_timeout(Some(SHORT)))
                .await
                .err(),
        ),
        (
            "list_snapshots",
            sandbox
                .list_snapshots(&ListSnapshotsParams::new().request_timeout(Some(SHORT)))
                .await
                .err(),
        ),
        (
            "fork",
            sandbox
                .fork(&ForkOptions::new().request_timeout(Some(SHORT)))
                .await
                .err(),
        ),
    ];

    // Assert
    assert_all_timed_out(results);
}

#[tokio::test]
async fn a_next_page_keeps_the_request_timeout() {
    // Arrange: configured 5 s; the list answers after 2 s.
    let server = MockServer::start().await;
    everything_slow(&server).await;
    let client = client_with_timeout(&server, Some(Duration::from_secs(5)));
    let sandboxes = ListSandboxesParams::new()
        .request_timeout(Some(SHORT))
        .next_token("page-2");
    let snapshots = ListSnapshotsParams::new()
        .request_timeout(Some(SHORT))
        .next_token("page-2");

    // Act
    let results = vec![
        (
            "list_sandboxes",
            client.list_sandboxes(&sandboxes).await.err(),
        ),
        (
            "list_snapshots",
            client.list_snapshots(&snapshots).await.err(),
        ),
    ];

    // Assert
    assert_all_timed_out(results);
}
