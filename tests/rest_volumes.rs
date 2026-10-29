//! Volumes and volume mounts against a mock server.

mod common;

use common::{API_KEY, SANDBOX_ID, client_for, detail_json, sandbox_json};
use e2b_client::{CreateSandboxParams, Error, ListSandboxesParams, RequestOptions};
use secrecy::ExposeSecret;
use serde_json::json;
use wiremock::matchers::{body_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const VOLUME_TOKEN: &str = "volume-token-secret";

fn volume_json() -> serde_json::Value {
    json!({"volumeID": "vol-1", "name": "data", "token": VOLUME_TOKEN})
}

fn error_json(code: u16, message: &str) -> serde_json::Value {
    json!({"code": code, "message": message})
}

#[tokio::test]
async fn create_volume_posts_its_name_and_keeps_the_token_secret() {
    // Arrange
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/volumes"))
        .and(header("x-api-key", API_KEY))
        .and(body_json(json!({"name": "data"})))
        .respond_with(ResponseTemplate::new(201).set_body_json(volume_json()))
        .expect(1)
        .mount(&server)
        .await;

    // Act
    let volume = client_for(&server)
        .create_volume("data", &RequestOptions::new())
        .await
        .expect("create");

    // Assert
    assert_eq!(volume.volume_id(), "vol-1");
    assert_eq!(volume.name(), "data");
    assert_eq!(volume.token().expose_secret(), VOLUME_TOKEN);
    assert_eq!(volume.domain(), None);
    assert!(!format!("{volume:?}").contains(VOLUME_TOKEN));
}

#[tokio::test]
async fn create_volume_refused_is_an_api_error() {
    // A 400 for a name the API refuses, and the 403 hosted E2B gives a team
    // without volumes (live, 2026-10-28), each with its own server.
    for (status, message) in [(400, "bad name"), (403, "use of volumes is not enabled")] {
        // Arrange
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/volumes"))
            .respond_with(ResponseTemplate::new(status).set_body_json(error_json(status, message)))
            .mount(&server)
            .await;

        // Act
        let error = client_for(&server)
            .create_volume("data", &RequestOptions::new())
            .await
            .expect_err("refused");

        // Assert
        assert!(
            matches!(&error, Error::Api { status: got, message: text, .. }
                if *got == status && text == message),
            "{status}: {error:?}"
        );
        assert!(!error.is_not_found(), "{status}");
    }
}

#[tokio::test]
async fn get_volume_reads_the_domain_and_maps_404() {
    // Arrange
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/volumes/vol-1"))
        .and(header("x-api-key", API_KEY))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "volumeID": "vol-1", "name": "data", "token": VOLUME_TOKEN,
            "domain": "byoc.example.test"
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/volumes/gone"))
        .respond_with(ResponseTemplate::new(404).set_body_json(error_json(404, "nope")))
        .mount(&server)
        .await;
    let client = client_for(&server);

    // Act
    let volume = client
        .get_volume("vol-1", &RequestOptions::new())
        .await
        .expect("get");
    let error = client
        .get_volume("gone", &RequestOptions::new())
        .await
        .expect_err("missing");

    // Assert
    assert_eq!(volume.volume_id(), "vol-1");
    assert_eq!(volume.token().expose_secret(), VOLUME_TOKEN);
    assert_eq!(volume.domain(), Some("byoc.example.test"));
    assert!(
        matches!(error, Error::VolumeNotFound(ref id) if id == "gone"),
        "{error:?}"
    );
    assert!(error.is_not_found());
}

#[tokio::test]
async fn get_volume_refused_is_an_api_error_not_not_found() {
    // Arrange
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/volumes/vol-1"))
        .respond_with(ResponseTemplate::new(500).set_body_json(error_json(500, "boom")))
        .mount(&server)
        .await;

    // Act
    let error = client_for(&server)
        .get_volume("vol-1", &RequestOptions::new())
        .await
        .expect_err("refused");

    // Assert
    assert!(
        matches!(&error, Error::Api { status: 500, message, .. } if message == "boom"),
        "{error:?}"
    );
    assert!(!error.is_not_found());
}

#[tokio::test]
async fn list_volumes_reads_ids_and_names() {
    // Arrange
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/volumes"))
        .and(header("x-api-key", API_KEY))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {"volumeID": "vol-1", "name": "data"}, {"volumeID": "vol-2", "name": "cache"}
        ])))
        .mount(&server)
        .await;

    // Act
    let volumes = client_for(&server)
        .list_volumes(&RequestOptions::new())
        .await
        .expect("list");

    // Assert
    let entries: Vec<(&str, &str)> = volumes
        .iter()
        .map(|volume| (volume.volume_id.as_str(), volume.name.as_str()))
        .collect();
    assert_eq!(entries, [("vol-1", "data"), ("vol-2", "cache")]);
}

#[tokio::test]
async fn list_volumes_reads_a_null_body_as_none_and_maps_errors() {
    // Arrange: a `null` body on the first call, a 401 on the second.
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/volumes"))
        .respond_with(ResponseTemplate::new(200).set_body_raw("null", "application/json"))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/volumes"))
        .respond_with(ResponseTemplate::new(401).set_body_json(error_json(401, "bad key")))
        .mount(&server)
        .await;
    let client = client_for(&server);

    // Act
    let empty = client
        .list_volumes(&RequestOptions::new())
        .await
        .expect("null body");
    let error = client
        .list_volumes(&RequestOptions::new())
        .await
        .expect_err("refused");

    // Assert
    assert!(empty.is_empty(), "{empty:?}");
    assert!(
        matches!(&error, Error::Api { status: 401, message, .. } if message == "bad key"),
        "{error:?}"
    );
}

#[tokio::test]
async fn destroy_volume_reports_whether_it_existed() {
    // Arrange
    let server = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path("/volumes/vol-1"))
        .and(header("x-api-key", API_KEY))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/volumes/gone"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/volumes/busy"))
        .respond_with(ResponseTemplate::new(500).set_body_json(error_json(500, "busy")))
        .mount(&server)
        .await;
    let client = client_for(&server);

    // Act
    let destroyed = client
        .destroy_volume("vol-1", &RequestOptions::new())
        .await
        .expect("destroy");
    let missing = client
        .destroy_volume("gone", &RequestOptions::new())
        .await
        .expect("destroy missing");
    let error = client
        .destroy_volume("busy", &RequestOptions::new())
        .await
        .expect_err("refused");

    // Assert
    assert!(destroyed);
    assert!(!missing);
    assert!(
        matches!(&error, Error::Api { status: 500, message, .. } if message == "busy"),
        "{error:?}"
    );
}

#[tokio::test]
async fn create_sends_volume_mounts_and_info_reads_them() {
    // Arrange
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v2/sandboxes"))
        .and(body_json(json!({
            "templateID": "base", "metadata": {}, "envVars": {},
            "volumeMounts": [
                {"name": "data", "path": "/mnt/data"},
                {"name": "cache", "path": "/mnt/cache"}
            ]
        })))
        .respond_with(ResponseTemplate::new(201).set_body_json(sandbox_json("0.5.7")))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/sandboxes/{SANDBOX_ID}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(detail_json(json!({
            "volumeMounts": [{"name": "data", "path": "/mnt/data"}]
        }))))
        .mount(&server)
        .await;
    let client = client_for(&server);
    let params = CreateSandboxParams::default()
        .volume_mount("data", "/mnt/data")
        .volume_mount("cache", "/mnt/cache");

    // Act
    let sandbox = client.create_sandbox(&params).await.expect("create");
    let info = sandbox
        .get_info(&RequestOptions::new())
        .await
        .expect("info");

    // Assert
    assert_eq!(info.volume_mounts.len(), 1);
    assert_eq!(info.volume_mounts[0].name, "data");
    assert_eq!(info.volume_mounts[0].path, "/mnt/data");
}

#[tokio::test]
async fn get_info_without_volume_mounts_reads_none() {
    // Arrange: one detail with `volumeMounts: null`, then one without it.
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/sandboxes/{SANDBOX_ID}")))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(detail_json(json!({"volumeMounts": null}))),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/sandboxes/{SANDBOX_ID}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(detail_json(json!({}))))
        .mount(&server)
        .await;
    let client = client_for(&server);

    // Act
    let null = client
        .get_sandbox_info(SANDBOX_ID, &RequestOptions::new())
        .await
        .expect("null");
    let absent = client
        .get_sandbox_info(SANDBOX_ID, &RequestOptions::new())
        .await
        .expect("absent");

    // Assert
    assert!(null.volume_mounts.is_empty(), "{:?}", null.volume_mounts);
    assert!(
        absent.volume_mounts.is_empty(),
        "{:?}",
        absent.volume_mounts
    );
}

#[tokio::test]
async fn list_entries_read_volume_mounts() {
    // Arrange
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v2/sandboxes"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!([detail_json(json!({
                "volumeMounts": [{"name": "data", "path": "/mnt/data"}]
            }))])),
        )
        .mount(&server)
        .await;

    // Act
    let page = client_for(&server)
        .list_sandboxes(&ListSandboxesParams::new())
        .await
        .expect("list");

    // Assert
    let mounts = &page.sandboxes[0].volume_mounts;
    assert_eq!(mounts.len(), 1);
    assert_eq!(
        (mounts[0].name.as_str(), mounts[0].path.as_str()),
        ("data", "/mnt/data")
    );
}
