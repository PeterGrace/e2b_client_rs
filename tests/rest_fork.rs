//! Fork against a mock server.

mod common;

use std::time::Duration;

use common::{SANDBOX_ID, client_for, sandbox_from_json, sandbox_json};
use e2b_client::{Error, ForkOptions};
use serde_json::json;
use wiremock::matchers::{body_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// The fork route of the mock sandbox.
fn fork_path() -> String {
    format!("/sandboxes/{SANDBOX_ID}/fork")
}

#[tokio::test]
async fn fork_returns_one_result_per_fork() {
    // Arrange: a started fork, a per-fork error, an entry with neither, and
    // one with both (the error wins, as in the SDK).
    let server = MockServer::start().await;
    let mut fork = sandbox_json("0.5.7");
    fork["sandboxID"] = json!("sbx-fork");
    Mock::given(method("POST"))
        .and(path(fork_path()))
        .and(body_json(json!({"timeout": 60, "count": 4})))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!([
            {"sandbox": fork},
            {"error": {"code": 429, "message": "rate limited", "error_code": "x"}},
            {},
            {"sandbox": sandbox_json("0.5.7"), "error": {"code": 500, "message": "boom"}}
        ])))
        .expect(1)
        .mount(&server)
        .await;
    let options = ForkOptions::new().timeout(Duration::from_secs(60)).count(4);

    // Act
    let results = client_for(&server)
        .fork_sandbox(SANDBOX_ID, &options)
        .await
        .expect("fork");

    // Assert
    assert_eq!(results.len(), 4);
    assert_eq!(
        results[0].as_ref().expect("first fork").sandbox_id(),
        "sbx-fork"
    );
    assert!(
        matches!(&results[1], Err(Error::Api { status: 429, message }) if message == "rate limited"),
        "{:?}",
        results[1]
    );
    assert!(
        matches!(&results[2], Err(Error::Protocol(_))),
        "{:?}",
        results[2]
    );
    assert!(
        matches!(&results[3], Err(Error::Api { status: 500, message }) if message == "boom"),
        "{:?}",
        results[3]
    );
}

#[tokio::test]
async fn fork_with_defaults_sends_an_empty_object() {
    // Arrange
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(fork_path()))
        .and(body_json(json!({})))
        .respond_with(
            ResponseTemplate::new(201).set_body_json(json!([{"sandbox": sandbox_json("0.5.7")}])),
        )
        .expect(1)
        .mount(&server)
        .await;

    // Act
    let results = client_for(&server)
        .fork_sandbox(SANDBOX_ID, &ForkOptions::default())
        .await
        .expect("fork");

    // Assert
    assert_eq!(results.len(), 1);
    assert!(results[0].is_ok(), "{:?}", results[0]);
}

#[tokio::test]
async fn fork_counts_outside_1_to_20_are_refused_before_any_request() {
    // Arrange
    let server = MockServer::start().await;
    let client = client_for(&server);

    for count in [0, 21] {
        // Act
        let error = client
            .fork_sandbox(SANDBOX_ID, &ForkOptions::new().count(count))
            .await
            .expect_err("invalid count");

        // Assert
        assert!(
            matches!(error, Error::InvalidArgument(_)),
            "{count}: {error:?}"
        );
    }
    assert!(
        server
            .received_requests()
            .await
            .expect("recording")
            .is_empty()
    );
}

#[tokio::test]
async fn fork_of_a_missing_sandbox_is_not_found() {
    // Arrange
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(fork_path()))
        .respond_with(
            ResponseTemplate::new(404).set_body_json(json!({"code": 404, "message": "nope"})),
        )
        .mount(&server)
        .await;

    // Act
    let error = client_for(&server)
        .fork_sandbox(SANDBOX_ID, &ForkOptions::default())
        .await
        .expect_err("not found");

    // Assert
    assert!(
        matches!(&error, Error::SandboxNotFound(id) if id == SANDBOX_ID),
        "{error:?}"
    );
}

#[tokio::test]
async fn fork_refused_is_an_api_error() {
    // Arrange: the spec lists 401, 409, 429, 500 and 503 for the request.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(fork_path()))
        .respond_with(
            ResponseTemplate::new(409).set_body_json(json!({"code": 409, "message": "paused"})),
        )
        .mount(&server)
        .await;

    // Act
    let error = client_for(&server)
        .fork_sandbox(SANDBOX_ID, &ForkOptions::default())
        .await
        .expect_err("refused");

    // Assert
    assert!(
        matches!(&error, Error::Api { status: 409, message } if message == "paused"),
        "{error:?}"
    );
}

#[tokio::test]
async fn fork_reads_a_malformed_entry_as_that_forks_error_and_a_null_body_as_decode() {
    // Arrange: the second entry's sandbox has no `sandboxID`.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(fork_path()))
        .and(body_json(json!({"count": 2})))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!([
            {"sandbox": sandbox_json("0.5.7")},
            {"sandbox": {"templateID": "base"}}
        ])))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path(fork_path()))
        .and(body_json(json!({"count": 1})))
        .respond_with(ResponseTemplate::new(201).set_body_string("null"))
        .mount(&server)
        .await;
    let client = client_for(&server);

    // Act
    let results = client
        .fork_sandbox(SANDBOX_ID, &ForkOptions::new().count(2))
        .await
        .expect("fork");
    let null_body = client
        .fork_sandbox(SANDBOX_ID, &ForkOptions::new().count(1))
        .await
        .expect_err("null body");

    // Assert: the started fork is still handed back, so it can be killed.
    assert_eq!(results.len(), 2);
    assert!(results[0].is_ok(), "{:?}", results[0]);
    assert!(
        matches!(&results[1], Err(Error::Decode(_))),
        "{:?}",
        results[1]
    );
    assert!(matches!(null_body, Error::Decode(_)), "{null_body:?}");
}

#[tokio::test]
async fn sandbox_fork_posts_to_its_own_id() {
    // Arrange
    let server = MockServer::start().await;
    let sandbox = sandbox_from_json(&server, sandbox_json("0.5.7")).await;
    Mock::given(method("POST"))
        .and(path(fork_path()))
        .and(body_json(json!({"count": 1})))
        .respond_with(
            ResponseTemplate::new(201).set_body_json(json!([{"sandbox": sandbox_json("0.5.7")}])),
        )
        .expect(1)
        .mount(&server)
        .await;

    // Act
    let results = sandbox
        .fork(&ForkOptions::new().count(1))
        .await
        .expect("fork");

    // Assert
    assert_eq!(results.len(), 1);
    assert!(results[0].is_ok(), "{:?}", results[0]);
}
