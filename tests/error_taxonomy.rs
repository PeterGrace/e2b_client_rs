//! The error classifiers (CLM-0169) and the opaque transport error
//! (CLM-0170), each error produced by a real call against a mock server.

mod common;

use std::error::Error as _;
use std::time::Duration;

use common::{SANDBOX_ID, client_for, client_with_timeout, sandbox};
use e2b_client::{Client, ConnectionConfig, Error, RequestOptions, TransportError};
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// What each classifier said about one error, in a fixed order so a failed
/// assertion prints every flag at once.
#[derive(Debug, PartialEq, Eq)]
struct Flags {
    auth: bool,
    rate_limited: bool,
    service_busy: bool,
    not_enough_space: bool,
    sandbox_unreachable: bool,
    timeout: bool,
    retryable: bool,
}

impl Flags {
    fn of(error: &Error) -> Self {
        Self {
            auth: error.is_auth(),
            rate_limited: error.is_rate_limited(),
            service_busy: error.is_service_busy(),
            not_enough_space: error.is_not_enough_space(),
            sandbox_unreachable: error.is_sandbox_unreachable(),
            timeout: error.is_timeout(),
            retryable: error.is_retryable(),
        }
    }

    const NONE: Self = Self {
        auth: false,
        rate_limited: false,
        service_busy: false,
        not_enough_space: false,
        sandbox_unreachable: false,
        timeout: false,
        retryable: false,
    };
}

#[tokio::test]
async fn rest_statuses_classify_as_the_sdk_does() {
    // Arrange: one sandbox id per status, each answering with an E2B error body.
    let server = MockServer::start().await;
    let cases = [
        (
            401,
            Flags {
                auth: true,
                ..Flags::NONE
            },
        ),
        (
            429,
            Flags {
                rate_limited: true,
                retryable: true,
                ..Flags::NONE
            },
        ),
        (
            503,
            Flags {
                service_busy: true,
                retryable: true,
                ..Flags::NONE
            },
        ),
        (500, Flags::NONE),
    ];
    for (status, _) in &cases {
        Mock::given(method("GET"))
            .and(path(format!("/sandboxes/s{status}")))
            .respond_with(
                ResponseTemplate::new(*status)
                    .set_body_json(json!({"code": status, "message": "nope"})),
            )
            .expect(1)
            .mount(&server)
            .await;
    }
    let client = client_for(&server);

    for (status, expected) in cases {
        // Act
        let err = client
            .get_sandbox_info(&format!("s{status}"), &RequestOptions::new())
            .await
            .expect_err("an error status");

        // Assert
        assert!(matches!(err, Error::Api { .. }), "{status}: {err:?}");
        assert_eq!(err.status(), Some(status), "{err:?}");
        assert_eq!(Flags::of(&err), expected, "{status}: {err:?}");
    }
}

#[tokio::test]
async fn envd_http_statuses_classify_as_the_sdk_does() {
    // Arrange: `is_running` reads envd's plain HTTP `/health`. A 502 there
    // means "not running" (`Ok(false)`), so the 502 case is a unit test in
    // `src/error.rs`.
    let cases = [
        (
            401,
            Flags {
                auth: true,
                ..Flags::NONE
            },
        ),
        (
            429,
            Flags {
                rate_limited: true,
                retryable: true,
                ..Flags::NONE
            },
        ),
        (
            507,
            Flags {
                not_enough_space: true,
                ..Flags::NONE
            },
        ),
        // An envd 503 is not the SDK's ServiceBusy, which only REST raises.
        (503, Flags::NONE),
    ];
    for (status, expected) in cases {
        let server = MockServer::start().await;
        let sandbox = sandbox(&server).await;
        Mock::given(method("GET"))
            .and(path("/health"))
            .respond_with(
                ResponseTemplate::new(status)
                    .set_body_json(json!({"code": status, "message": "nope"})),
            )
            .expect(1)
            .mount(&server)
            .await;

        // Act
        let err = sandbox
            .is_running(&RequestOptions::new())
            .await
            .expect_err("an error status");

        // Assert
        assert!(matches!(err, Error::Envd { .. }), "{status}: {err:?}");
        assert_eq!(err.status(), Some(status), "{err:?}");
        assert_eq!(Flags::of(&err), expected, "{status}: {err:?}");
    }
}

#[tokio::test]
async fn rpc_codes_classify_as_the_sdk_does() {
    // Arrange: `Commands::list` is a unary RPC; each answer is a Connect
    // error body.
    let cases = [
        (
            401,
            "unauthenticated",
            Flags {
                auth: true,
                ..Flags::NONE
            },
        ),
        (
            429,
            "resource_exhausted",
            Flags {
                rate_limited: true,
                retryable: true,
                ..Flags::NONE
            },
        ),
        (
            503,
            "unavailable",
            Flags {
                sandbox_unreachable: true,
                ..Flags::NONE
            },
        ),
        (
            504,
            "deadline_exceeded",
            Flags {
                timeout: true,
                ..Flags::NONE
            },
        ),
        (
            499,
            "canceled",
            Flags {
                timeout: true,
                ..Flags::NONE
            },
        ),
        (500, "internal", Flags::NONE),
    ];
    for (status, code, expected) in cases {
        let server = MockServer::start().await;
        let sandbox = sandbox(&server).await;
        Mock::given(method("POST"))
            .and(path("/process.Process/List"))
            .respond_with(
                ResponseTemplate::new(status)
                    .set_body_json(json!({"code": code, "message": "nope"})),
            )
            .expect(1)
            .mount(&server)
            .await;

        // Act
        let err = sandbox.commands().list().await.expect_err("an RPC error");

        // Assert
        assert!(matches!(err, Error::Rpc { .. }), "{code}: {err:?}");
        assert_eq!(err.status(), None, "an RPC error has a code, not a status");
        assert_eq!(Flags::of(&err), expected, "{code}: {err:?}");
    }
}

#[tokio::test]
async fn a_request_timeout_is_a_timeout_and_not_retryable() {
    // Arrange: the answer comes after the client's request timeout.
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/sandboxes/{SANDBOX_ID}")))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(5)))
        .mount(&server)
        .await;
    let client = client_with_timeout(&server, Some(Duration::from_millis(100)));

    // Act
    let err = client
        .get_sandbox_info(SANDBOX_ID, &RequestOptions::new())
        .await
        .expect_err("timeout");

    // Assert
    let Error::Transport(ref transport) = err else {
        panic!("expected a transport error, got {err:?}");
    };
    assert!(transport.is_timeout(), "{transport:?}");
    assert!(!transport.is_connect(), "{transport:?}");
    assert_eq!(err.status(), None);
    assert_eq!(
        Flags::of(&err),
        Flags {
            timeout: true,
            ..Flags::NONE
        }
    );
    // The detail is in `source()`, not repeated in `Display`.
    assert_eq!(err.to_string(), "HTTP transport error");
    let source = err.source().expect("a source");
    assert!(!source.to_string().is_empty(), "{source:?}");
}

#[tokio::test]
async fn a_refused_connection_is_retryable_and_keeps_its_cause() {
    // Arrange: reserve a port with a socket that is bound but never listens.
    // The kernel refuses connections to it, and because the socket stays
    // bound until the end of the test, no other process can take the port
    // in between (binding and then freeing a port would leave that race).
    let reserved = tokio::net::TcpSocket::new_v4().expect("socket");
    reserved
        .bind("127.0.0.1:0".parse().expect("addr"))
        .expect("bind");
    let port = reserved.local_addr().expect("addr").port();
    let config = ConnectionConfig::builder()
        .api_key("test-api-key")
        .api_url(format!("http://127.0.0.1:{port}"))
        .domain("example.test")
        .request_timeout(Some(Duration::from_secs(5)))
        .build()
        .expect("config");
    let client = Client::new(config).expect("client");

    // Act
    let err = client
        .get_sandbox_info(SANDBOX_ID, &RequestOptions::new())
        .await
        .expect_err("connection refused");

    // Assert
    let Error::Transport(ref transport) = err else {
        panic!("expected a transport error, got {err:?}");
    };
    assert!(transport.is_connect(), "{transport:?}");
    assert!(err.is_retryable(), "{err:?}");
    assert!(!err.is_timeout(), "{err:?}");
    // The cause chain survives the wrapper: `Error` -> `TransportError`.
    let source = err.source().expect("a source");
    assert!(
        source.downcast_ref::<TransportError>().is_some(),
        "{source:?}"
    );
}

#[tokio::test]
async fn a_decode_error_shows_its_detail_once() {
    // Arrange: a 200 whose body is not JSON.
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/sandboxes/{SANDBOX_ID}")))
        .respond_with(ResponseTemplate::new(200).set_body_string("not json"))
        .mount(&server)
        .await;
    let client = client_for(&server);

    // Act
    let err = client
        .get_sandbox_info(SANDBOX_ID, &RequestOptions::new())
        .await
        .expect_err("undecodable body");

    // Assert: the serde_json detail is in `source()`, not repeated in
    // `Display`, so `{:#}` in anyhow prints it once (as for `Transport`).
    assert!(matches!(err, Error::Decode(_)), "{err:?}");
    assert_eq!(err.to_string(), "could not decode response");
    let source = err.source().expect("a source");
    assert!(!source.to_string().is_empty(), "{source:?}");
    assert!(
        source.downcast_ref::<serde_json::Error>().is_some(),
        "{source:?}"
    );
}
