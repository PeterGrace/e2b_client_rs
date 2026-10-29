//! Mock tests for the per-call request timeout on the pid-only process and
//! PTY calls (CLM-0183).

mod common;

use std::time::{Duration, Instant};

use common::{client_with_timeout, sandbox_from_json_with_client, sandbox_json};
use e2b_client::{Error, PtySize, RequestOptions, Signal};
use serde_json::json;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const SHORT: Duration = Duration::from_millis(200);

/// Whether `error` is a transport-level timeout (reqwest's own deadline).
fn is_transport_timeout(error: &Error) -> bool {
    matches!(error, Error::Transport(transport) if transport.is_timeout())
}

/// Mounts every unary process RPC answering `{}` after `delay`.
async fn slow_process_rpcs(server: &MockServer, delay: Duration) {
    for rpc in ["List", "SendInput", "CloseStdin", "SendSignal", "Update"] {
        Mock::given(method("POST"))
            .and(path(format!("/process.Process/{rpc}")))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({}))
                    .set_delay(delay),
            )
            .mount(server)
            .await;
    }
}

#[tokio::test]
async fn every_pid_only_call_takes_a_per_call_request_timeout() {
    // Arrange: configured 5 s; every RPC answers after 2 s.
    let server = MockServer::start().await;
    let client = client_with_timeout(&server, Some(Duration::from_secs(5)));
    let sandbox = sandbox_from_json_with_client(&server, &client, sandbox_json("0.5.2")).await;
    slow_process_rpcs(&server, Duration::from_secs(2)).await;
    let short = RequestOptions::new().request_timeout(Some(SHORT));
    let commands = sandbox.commands();
    let pty = sandbox.pty();

    // Act
    let started = Instant::now();
    let results = [
        ("list", commands.list(&short).await.err()),
        (
            "send_stdin",
            commands.send_stdin(7, "x", &short).await.err(),
        ),
        ("close_stdin", commands.close_stdin(7, &short).await.err()),
        (
            "send_signal",
            commands.send_signal(7, Signal::Sigterm, &short).await.err(),
        ),
        ("kill", commands.kill(7, &short).await.err()),
        ("pty send_stdin", pty.send_stdin(7, "x", &short).await.err()),
        (
            "pty resize",
            pty.resize(7, PtySize::new(80, 24), &short).await.err(),
        ),
        ("pty kill", pty.kill(7, &short).await.err()),
    ];

    // Assert: each gave up on its own 200 ms, so eight calls took well
    // under the 16 s the server's delay would need.
    for (call, error) in results {
        let error = error.unwrap_or_else(|| panic!("{call}: expected a timeout"));
        assert!(is_transport_timeout(&error), "{call}: {error:?}");
    }
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "{:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn a_per_call_none_lifts_a_short_configured_timeout() {
    // Arrange: configured 200 ms; the RPCs answer after 500 ms.
    let server = MockServer::start().await;
    let client = client_with_timeout(&server, Some(SHORT));
    let sandbox = sandbox_from_json_with_client(&server, &client, sandbox_json("0.5.2")).await;
    slow_process_rpcs(&server, Duration::from_millis(500)).await;

    // Act
    let unbounded = sandbox
        .commands()
        .list(&RequestOptions::new().request_timeout(None))
        .await;
    let configured = sandbox.commands().list(&RequestOptions::new()).await;

    // Assert
    assert!(unbounded.is_ok(), "{unbounded:?}");
    let error = configured.expect_err("the configured 200 ms applies when unset");
    assert!(is_transport_timeout(&error), "{error:?}");
}

#[tokio::test]
async fn the_per_call_timeout_is_sent_as_connect_timeout_ms() {
    // Arrange: configured 5 s; `SendSignal` answers only a request that
    // carries the per-call 200 ms, so any other header misses the mock.
    let server = MockServer::start().await;
    let client = client_with_timeout(&server, Some(Duration::from_secs(5)));
    let sandbox = sandbox_from_json_with_client(&server, &client, sandbox_json("0.5.2")).await;
    Mock::given(method("POST"))
        .and(path("/process.Process/SendSignal"))
        .and(header("connect-timeout-ms", "200"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .expect(1)
        .mount(&server)
        .await;
    let short = RequestOptions::new().request_timeout(Some(SHORT));

    // Act: `kill` reaches envd through `send_signal`.
    let killed = sandbox.commands().kill(7, &short).await;

    // Assert
    assert!(killed.expect("kill with the 200 ms header"), "SIGKILL sent");
}
