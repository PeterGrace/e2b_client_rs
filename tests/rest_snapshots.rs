//! Snapshots against a mock server.

mod common;

use common::{SANDBOX_ID, client_for, sandbox_json};
use e2b_client::{Error, ListSnapshotsParams, SnapshotMode, SnapshotOptions};
use serde_json::json;
use wiremock::matchers::{body_json, method, path, query_param, query_param_is_missing};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn create_snapshot_posts_name_and_memory() {
    // Arrange
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/sandboxes/{SANDBOX_ID}/snapshots")))
        .and(body_json(json!({"name": "my-snap", "memory": false})))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({
            "snapshotID": "tpl123:v1", "names": ["team/my-snap:v1"]
        })))
        .expect(1)
        .mount(&server)
        .await;
    let options = SnapshotOptions::new()
        .name("my-snap")
        .mode(SnapshotMode::Filesystem);

    // Act
    let snapshot = client_for(&server)
        .create_snapshot(SANDBOX_ID, &options)
        .await
        .expect("snapshot");

    // Assert
    assert_eq!(snapshot.snapshot_id, "tpl123:v1");
    assert_eq!(snapshot.names, ["team/my-snap:v1"]);
}

#[tokio::test]
async fn create_snapshot_with_defaults_sends_an_empty_object() {
    // Arrange
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/sandboxes/{SANDBOX_ID}/snapshots")))
        .and(body_json(json!({})))
        .respond_with(
            ResponseTemplate::new(201).set_body_json(json!({"snapshotID": "t:1", "names": []})),
        )
        .expect(1)
        .mount(&server)
        .await;

    // Act
    let snapshot = client_for(&server)
        .create_snapshot(SANDBOX_ID, &SnapshotOptions::default())
        .await
        .expect("snapshot");

    // Assert
    assert_eq!(snapshot.snapshot_id, "t:1");
    assert!(snapshot.names.is_empty());
}

#[tokio::test]
async fn create_snapshot_of_a_missing_sandbox_is_not_found() {
    // Arrange
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/sandboxes/{SANDBOX_ID}/snapshots")))
        .respond_with(
            ResponseTemplate::new(404).set_body_json(json!({"code": 404, "message": "nope"})),
        )
        .mount(&server)
        .await;

    // Act
    let error = client_for(&server)
        .create_snapshot(SANDBOX_ID, &SnapshotOptions::default())
        .await
        .expect_err("not found");

    // Assert
    assert!(
        matches!(&error, Error::SandboxNotFound(id) if id == SANDBOX_ID),
        "{error:?}"
    );
}

#[tokio::test]
async fn create_snapshot_refused_is_an_api_error() {
    // Arrange: the SDK's two documented refusals of a filesystem-only
    // snapshot (`py: sandbox_async/main.py:905`).
    let server = MockServer::start().await;
    for (status, code) in [
        (400, "snapshot_filesystem_only_disabled"),
        (409, "snapshot_filesystem_only_unsupported_node"),
    ] {
        server.reset().await;
        Mock::given(method("POST"))
            .and(path(format!("/sandboxes/{SANDBOX_ID}/snapshots")))
            .respond_with(
                ResponseTemplate::new(status)
                    .set_body_json(json!({"code": status, "message": code})),
            )
            .expect(1)
            .mount(&server)
            .await;
        let options = SnapshotOptions::new().mode(SnapshotMode::Filesystem);

        // Act
        let error = client_for(&server)
            .create_snapshot(SANDBOX_ID, &options)
            .await
            .expect_err("refused");

        // Assert
        assert!(
            matches!(&error, Error::Api { status: got, message, .. } if *got == status && message == code),
            "{status}: {error:?}"
        );
    }
}

#[tokio::test]
async fn list_snapshots_sends_filters_and_reads_the_next_token() {
    // Arrange
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/snapshots"))
        .and(query_param("sandboxID", SANDBOX_ID))
        .and(query_param("name", "my-snap"))
        .and(query_param("limit", "2"))
        .and(query_param("nextToken", "page-1"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("x-next-token", "page-2")
                .set_body_json(json!([{"snapshotID": "t:1", "names": ["team/my-snap:v1"]}])),
        )
        .expect(1)
        .mount(&server)
        .await;
    let params = ListSnapshotsParams::new()
        .sandbox_id(SANDBOX_ID)
        .name("my-snap")
        .limit(2)
        .next_token("page-1");

    // Act
    let page = client_for(&server)
        .list_snapshots(&params)
        .await
        .expect("list");

    // Assert
    assert_eq!(page.snapshots.len(), 1);
    assert_eq!(page.snapshots[0].snapshot_id, "t:1");
    assert_eq!(page.snapshots[0].names, ["team/my-snap:v1"]);
    assert_eq!(page.next_token.as_deref(), Some("page-2"));
}

#[tokio::test]
async fn list_snapshots_without_filters_sends_no_query() {
    // Arrange
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/snapshots"))
        .and(query_param_is_missing("sandboxID"))
        .and(query_param_is_missing("name"))
        .and(query_param_is_missing("limit"))
        .and(query_param_is_missing("nextToken"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .expect(1)
        .mount(&server)
        .await;

    // Act
    let page = client_for(&server)
        .list_snapshots(&ListSnapshotsParams::new())
        .await
        .expect("list");

    // Assert
    assert!(page.snapshots.is_empty() && page.next_token.is_none());
}

#[tokio::test]
async fn list_snapshots_reads_a_null_body_as_an_empty_page() {
    // Arrange
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/snapshots"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!(null)))
        .expect(1)
        .mount(&server)
        .await;

    // Act
    let page = client_for(&server)
        .list_snapshots(&ListSnapshotsParams::new())
        .await
        .expect("list");

    // Assert
    assert!(page.snapshots.is_empty() && page.next_token.is_none());
}

#[tokio::test]
async fn list_snapshots_refused_is_an_api_error() {
    // Arrange
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/snapshots"))
        .respond_with(
            ResponseTemplate::new(401).set_body_json(json!({"code": 401, "message": "bad key"})),
        )
        .expect(1)
        .mount(&server)
        .await;

    // Act
    let error = client_for(&server)
        .list_snapshots(&ListSnapshotsParams::new())
        .await
        .expect_err("refused");

    // Assert
    assert!(
        matches!(&error, Error::Api { status: 401, message, .. } if message == "bad key"),
        "{error:?}"
    );
}

#[tokio::test]
async fn delete_snapshot_encodes_the_id_as_one_segment() {
    // Arrange: `/` is percent-encoded so `team/my-snap:v1` stays one path
    // segment; wiremock compares the path as sent, still encoded.
    let server = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path("/templates/team%2Fmy-snap:v1"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/templates/gone"))
        .respond_with(ResponseTemplate::new(404))
        .expect(1)
        .mount(&server)
        .await;
    let client = client_for(&server);

    // Act
    let deleted = client
        .delete_snapshot("team/my-snap:v1")
        .await
        .expect("delete");
    let deleted_missing = client
        .delete_snapshot("gone")
        .await
        .expect("delete missing");

    // Assert
    assert!(deleted);
    assert!(!deleted_missing);
}

#[tokio::test]
async fn delete_snapshot_refused_is_an_api_error() {
    // Arrange
    let server = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path("/templates/t:1"))
        .respond_with(
            ResponseTemplate::new(500).set_body_json(json!({"code": 500, "message": "boom"})),
        )
        .expect(1)
        .mount(&server)
        .await;

    // Act
    let error = client_for(&server)
        .delete_snapshot("t:1")
        .await
        .expect_err("refused");

    // Assert
    assert!(
        matches!(&error, Error::Api { status: 500, message, .. } if message == "boom"),
        "{error:?}"
    );
}

#[tokio::test]
async fn sandbox_create_snapshot_posts_to_its_own_id() {
    // Arrange
    let server = MockServer::start().await;
    let sandbox = common::sandbox_from_json(&server, sandbox_json("0.5.7")).await;
    Mock::given(method("POST"))
        .and(path(format!("/sandboxes/{SANDBOX_ID}/snapshots")))
        .and(body_json(json!({"name": "mine"})))
        .respond_with(
            ResponseTemplate::new(201).set_body_json(json!({"snapshotID": "t:2", "names": null})),
        )
        .expect(1)
        .mount(&server)
        .await;

    // Act
    let snapshot = sandbox
        .create_snapshot(&SnapshotOptions::new().name("mine"))
        .await
        .expect("snapshot");

    // Assert
    assert_eq!(snapshot.snapshot_id, "t:2");
    assert!(snapshot.names.is_empty());
}

#[tokio::test]
async fn sandbox_list_snapshots_filters_by_its_own_id() {
    // Arrange: a different `sandbox_id` in the parameters is replaced.
    let server = MockServer::start().await;
    let sandbox = common::sandbox_from_json(&server, sandbox_json("0.5.7")).await;
    Mock::given(method("GET"))
        .and(path("/snapshots"))
        .and(query_param("sandboxID", SANDBOX_ID))
        .and(query_param("name", "my-snap"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .expect(1)
        .mount(&server)
        .await;
    let params = ListSnapshotsParams::new()
        .sandbox_id("someone-else")
        .name("my-snap");

    // Act
    let page = sandbox.list_snapshots(&params).await.expect("list");

    // Assert
    assert!(page.snapshots.is_empty());
}
