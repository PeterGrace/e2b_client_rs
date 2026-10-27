//! The SDK 2.61 list filters against a mock server.

mod common;

use chrono::{DateTime, Utc};
use common::client_for;
use e2b_client::{ListSandboxesParams, SortOrder};
use serde_json::json;
use wiremock::matchers::{method, path, query_param, query_param_is_missing};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn list_sends_order_started_after_and_template() {
    // Arrange
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v2/sandboxes"))
        .and(query_param("order", "desc"))
        .and(query_param("startedAfter", "2026-10-27T12:00:00+00:00"))
        .and(query_param("template", "base"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .expect(1)
        .mount(&server)
        .await;
    let started_after: DateTime<Utc> = "2026-10-27T12:00:00Z".parse().expect("time");
    let params = ListSandboxesParams::new()
        .order(SortOrder::Descending)
        .started_after(started_after)
        .template("base");

    // Act
    let page = client_for(&server)
        .list_sandboxes(&params)
        .await
        .expect("list");

    // Assert
    assert!(page.sandboxes.is_empty());
}

#[tokio::test]
async fn list_omits_unset_filters() {
    // Arrange
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v2/sandboxes"))
        .and(query_param_is_missing("order"))
        .and(query_param_is_missing("startedAfter"))
        .and(query_param_is_missing("template"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .expect(1)
        .mount(&server)
        .await;

    // Act
    let page = client_for(&server)
        .list_sandboxes(&ListSandboxesParams::new())
        .await
        .expect("list");

    // Assert: `.expect(1)` on the mock checks the query when the server drops.
    assert!(page.sandboxes.is_empty());
}
