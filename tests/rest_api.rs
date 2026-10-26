//! REST control-plane calls against a mock server.

mod common;

use std::time::Duration;

use common::{API_KEY, SANDBOX_ID, client_for, sandbox_json};
use e2b_client::{CreateSandboxParams, Error, ListSandboxesParams, SandboxState};
use serde_json::json;
use wiremock::matchers::{body_json, header, method, path, query_param, query_param_is_missing};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn detail_json(state: &str) -> serde_json::Value {
    json!({
        "templateID": "base",
        "sandboxID": SANDBOX_ID,
        "clientID": "client",
        "startedAt": "2026-10-26T12:00:00Z",
        "endAt": "2026-10-26T12:05:00Z",
        "envdVersion": "0.5.7",
        "cpuCount": 2,
        "memoryMB": 512,
        "diskSizeMB": 2048,
        "state": state,
        "metadata": {"owner": "tests"}
    })
}

#[tokio::test]
async fn create_sandbox_posts_v2_body_with_api_key() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v2/sandboxes"))
        .and(header("x-api-key", API_KEY))
        .and(header("content-type", "application/json"))
        .and(body_json(json!({
            "templateID": "my-template",
            "timeout": 600,
            "metadata": {"owner": "tests"},
            "envVars": {"A": "1"}
        })))
        .respond_with(ResponseTemplate::new(201).set_body_json(sandbox_json("0.5.7")))
        .expect(1)
        .mount(&server)
        .await;

    let params = CreateSandboxParams::new("my-template")
        .timeout(Duration::from_secs(600))
        .metadata("owner", "tests")
        .env("A", "1");
    let sandbox = client_for(&server)
        .create_sandbox(&params)
        .await
        .expect("create");

    assert_eq!(sandbox.sandbox_id(), SANDBOX_ID);
    assert_eq!(sandbox.template_id(), "base");
    assert_eq!(sandbox.envd_version().to_string(), "0.5.7");
}

#[tokio::test]
async fn create_sandbox_surfaces_e2b_error_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v2/sandboxes"))
        .respond_with(
            ResponseTemplate::new(400)
                .set_body_json(json!({"code": 400, "message": "template not found"})),
        )
        .mount(&server)
        .await;

    let err = client_for(&server)
        .create_sandbox(&CreateSandboxParams::default())
        .await
        .expect_err("error");

    assert!(
        matches!(err, Error::Api { status: 400, ref message } if message == "template not found"),
        "{err:?}"
    );
}

#[tokio::test]
async fn redirects_are_not_followed_so_the_api_key_stays_put() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v2/sandboxes"))
        .respond_with(
            ResponseTemplate::new(307)
                .insert_header("location", format!("{}/elsewhere", server.uri())),
        )
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/elsewhere"))
        .respond_with(ResponseTemplate::new(201).set_body_json(sandbox_json("0.5.7")))
        .expect(0)
        .mount(&server)
        .await;

    let err = client_for(&server)
        .create_sandbox(&CreateSandboxParams::default())
        .await
        .expect_err("error");

    assert!(matches!(err, Error::Api { status: 307, .. }), "{err:?}");
}

#[tokio::test]
async fn create_sandbox_rejects_unexpected_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v2/sandboxes"))
        .respond_with(ResponseTemplate::new(201).set_body_string("not json"))
        .mount(&server)
        .await;

    let err = client_for(&server)
        .create_sandbox(&CreateSandboxParams::default())
        .await
        .expect_err("error");

    assert!(matches!(err, Error::Decode(_)), "{err:?}");
}

#[tokio::test]
async fn connect_sandbox_uses_v2_with_timeout() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/v2/sandboxes/{SANDBOX_ID}/connect")))
        .and(body_json(json!({"timeout": 120})))
        .respond_with(ResponseTemplate::new(201).set_body_json(sandbox_json("0.5.7")))
        .expect(1)
        .mount(&server)
        .await;

    let sandbox = client_for(&server)
        .connect_sandbox(SANDBOX_ID, Some(Duration::from_secs(120)))
        .await
        .expect("connect");

    assert_eq!(sandbox.sandbox_id(), SANDBOX_ID);
}

#[tokio::test]
async fn connect_sandbox_without_timeout_sends_empty_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/v2/sandboxes/{SANDBOX_ID}/connect")))
        .and(body_json(json!({})))
        .respond_with(ResponseTemplate::new(200).set_body_json(sandbox_json("0.5.7")))
        .expect(1)
        .mount(&server)
        .await;

    client_for(&server)
        .connect_sandbox(SANDBOX_ID, None)
        .await
        .expect("connect");
}

#[tokio::test]
async fn connect_sandbox_falls_back_to_v1_route() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/v2/sandboxes/{SANDBOX_ID}/connect")))
        .respond_with(ResponseTemplate::new(404))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path(format!("/sandboxes/{SANDBOX_ID}/connect")))
        .and(body_json(json!({"timeout": 300})))
        .respond_with(ResponseTemplate::new(201).set_body_json(sandbox_json("0.5.7")))
        .expect(1)
        .mount(&server)
        .await;

    let sandbox = client_for(&server)
        .connect_sandbox(SANDBOX_ID, None)
        .await
        .expect("connect");

    assert_eq!(sandbox.sandbox_id(), SANDBOX_ID);
}

#[tokio::test]
async fn connect_missing_sandbox_is_not_found() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(404).set_body_json(json!({"code": 404, "message": "nope"})),
        )
        .expect(2)
        .mount(&server)
        .await;

    let err = client_for(&server)
        .connect_sandbox(SANDBOX_ID, None)
        .await
        .expect_err("error");

    assert!(
        matches!(err, Error::SandboxNotFound(ref id) if id == SANDBOX_ID),
        "{err:?}"
    );
}

#[tokio::test]
async fn get_sandbox_info_parses_detail() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/sandboxes/{SANDBOX_ID}")))
        .and(header("x-api-key", API_KEY))
        .respond_with(ResponseTemplate::new(200).set_body_json(detail_json("running")))
        .mount(&server)
        .await;

    let info = client_for(&server)
        .get_sandbox_info(SANDBOX_ID)
        .await
        .expect("info");

    assert_eq!(info.sandbox_id, SANDBOX_ID);
    assert_eq!(info.state, SandboxState::Running);
    assert_eq!(info.cpu_count, 2);
    assert_eq!(info.disk_size_mb, Some(2048));
    assert_eq!(info.end_at.to_rfc3339(), "2026-10-26T12:05:00+00:00");
}

#[tokio::test]
async fn get_sandbox_info_404_is_not_found() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let err = client_for(&server)
        .get_sandbox_info(SANDBOX_ID)
        .await
        .expect_err("error");

    assert!(
        matches!(err, Error::SandboxNotFound(ref id) if id == SANDBOX_ID),
        "{err:?}"
    );
}

#[tokio::test]
async fn list_sandboxes_sends_filters_and_reads_next_token() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v2/sandboxes"))
        .and(query_param("metadata", "owner=tests"))
        .and(query_param("state", "running,paused"))
        .and(query_param("limit", "1"))
        .and(query_param_is_missing("nextToken"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("x-next-token", "page-2")
                .set_body_json(json!([detail_json("paused")])),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v2/sandboxes"))
        .and(query_param("nextToken", "page-2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .expect(1)
        .mount(&server)
        .await;
    let client = client_for(&server);
    let params = ListSandboxesParams::new()
        .metadata("owner", "tests")
        .state(SandboxState::Running)
        .state(SandboxState::Paused)
        .limit(1);

    let first = client.list_sandboxes(&params).await.expect("page 1");
    let token = first.next_token.clone().expect("next token");
    let second = client
        .list_sandboxes(&params.next_token(token))
        .await
        .expect("page 2");

    assert_eq!(first.sandboxes.len(), 1);
    assert_eq!(first.sandboxes[0].state, SandboxState::Paused);
    assert!(second.sandboxes.is_empty());
    assert!(second.next_token.is_none());
}

#[tokio::test]
async fn kill_sandbox_reports_whether_it_existed() {
    let server = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path("/sandboxes/alive"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/sandboxes/gone"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;
    let client = client_for(&server);

    assert!(client.kill_sandbox("alive").await.expect("kill alive"));
    assert!(!client.kill_sandbox("gone").await.expect("kill gone"));
}

#[tokio::test]
async fn kill_sandbox_server_error_is_api_error() {
    let server = MockServer::start().await;
    Mock::given(method("DELETE"))
        .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
        .mount(&server)
        .await;

    let err = client_for(&server)
        .kill_sandbox("x")
        .await
        .expect_err("error");

    assert!(
        matches!(err, Error::Api { status: 500, ref message } if message == "boom"),
        "{err:?}"
    );
}

#[tokio::test]
async fn pause_sandbox_maps_status_codes() {
    let server = MockServer::start().await;
    for (id, status) in [("ok", 204), ("already", 409), ("missing", 404)] {
        Mock::given(method("POST"))
            .and(path(format!("/sandboxes/{id}/pause")))
            .and(body_json(json!({})))
            .respond_with(ResponseTemplate::new(status))
            .mount(&server)
            .await;
    }
    let client = client_for(&server);

    assert!(client.pause_sandbox("ok").await.expect("pause"));
    assert!(
        !client
            .pause_sandbox("already")
            .await
            .expect("already paused")
    );
    let err = client.pause_sandbox("missing").await.expect_err("missing");
    assert!(matches!(err, Error::SandboxNotFound(_)), "{err:?}");
}

#[tokio::test]
async fn set_timeout_and_refresh_send_seconds() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/sandboxes/{SANDBOX_ID}/timeout")))
        .and(body_json(json!({"timeout": 30})))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path(format!("/sandboxes/{SANDBOX_ID}/refreshes")))
        .and(body_json(json!({"duration": 60})))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path(format!("/sandboxes/{SANDBOX_ID}/refreshes")))
        .and(body_json(json!({})))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    let client = client_for(&server);

    client
        .set_sandbox_timeout(SANDBOX_ID, Duration::from_secs(30))
        .await
        .expect("timeout");
    client
        .refresh_sandbox(SANDBOX_ID, Some(Duration::from_secs(60)))
        .await
        .expect("refresh");
    client
        .refresh_sandbox(SANDBOX_ID, None)
        .await
        .expect("refresh default");
}

#[tokio::test]
async fn set_timeout_on_missing_sandbox_is_not_found() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let err = client_for(&server)
        .set_sandbox_timeout("missing", Duration::from_secs(1))
        .await
        .expect_err("error");

    assert!(matches!(err, Error::SandboxNotFound(_)), "{err:?}");
}

#[tokio::test]
async fn sandbox_handle_delegates_lifecycle_calls() {
    let server = MockServer::start().await;
    let sandbox = common::sandbox(&server).await;
    Mock::given(method("GET"))
        .and(path(format!("/sandboxes/{SANDBOX_ID}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(detail_json("running")))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path(format!("/sandboxes/{SANDBOX_ID}/pause")))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path(format!("/sandboxes/{SANDBOX_ID}/timeout")))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path(format!("/sandboxes/{SANDBOX_ID}/refreshes")))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path(format!("/sandboxes/{SANDBOX_ID}")))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;

    assert_eq!(
        sandbox.get_info().await.expect("info").sandbox_id,
        SANDBOX_ID
    );
    assert!(sandbox.pause().await.expect("pause"));
    sandbox
        .set_timeout(Duration::from_secs(10))
        .await
        .expect("timeout");
    sandbox.refresh(None).await.expect("refresh");
    assert!(sandbox.kill().await.expect("kill"));
}

#[tokio::test]
async fn every_rest_request_carries_the_api_key() {
    // Arrange: no mocks, so every call gets wiremock's 404. Only the requests
    // matter here, not how each call reports the 404.
    let server = MockServer::start().await;
    let client = client_for(&server);
    let timeout = Duration::from_secs(1);

    // Act: `let _ =` discards each call's result on purpose.
    let _ = client.create_sandbox(&CreateSandboxParams::default()).await;
    // v2 connect, then the v1 fallback after its 404.
    let _ = client.connect_sandbox(SANDBOX_ID, None).await;
    let _ = client.get_sandbox_info(SANDBOX_ID).await;
    let _ = client.list_sandboxes(&ListSandboxesParams::new()).await;
    let _ = client.kill_sandbox(SANDBOX_ID).await;
    let _ = client.pause_sandbox(SANDBOX_ID).await;
    let _ = client.set_sandbox_timeout(SANDBOX_ID, timeout).await;
    let _ = client.refresh_sandbox(SANDBOX_ID, None).await;

    // Assert
    let requests = server.received_requests().await.expect("recording");
    assert_eq!(requests.len(), 9, "one request per call, two for connect");
    for request in &requests {
        assert_eq!(
            request
                .headers
                .get("x-api-key")
                .map(|value| value.as_bytes()),
            Some(API_KEY.as_bytes()),
            "{} {}",
            request.method,
            request.url
        );
    }
}

#[tokio::test]
async fn request_timeout_applies_to_rest_calls() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(5)))
        .mount(&server)
        .await;
    let client = common::client_with_timeout(&server, Some(Duration::from_millis(100)));

    let err = client
        .get_sandbox_info(SANDBOX_ID)
        .await
        .expect_err("timeout");

    assert!(
        matches!(err, Error::Transport(ref inner) if inner.is_timeout()),
        "{err:?}"
    );
}
