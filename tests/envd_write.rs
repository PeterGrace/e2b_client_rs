//! Mock tests for `Filesystem::write_files` and its upload encodings
//! (`POST /files`: multipart, octet-stream and gzip).

mod common;

use std::time::{Duration, Instant};

use async_compression::tokio::bufread::GzipDecoder;
use common::{ACCESS_TOKEN, sandbox, sandbox_with_version};
use e2b_client::{Error, FileType, WriteEntry, WriteOptions};
use serde_json::json;
use tokio::io::AsyncReadExt;
use wiremock::matchers::{
    body_bytes, body_string_contains, header, method, path, query_param, query_param_is_missing,
};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

/// Whether a request carries no `path` query parameter.
fn has_no_path_param(request: &Request) -> bool {
    request.url.query_pairs().all(|(key, _)| key != "path")
}

/// Whether a request's body is `multipart/form-data`.
fn is_multipart(request: &Request) -> bool {
    request
        .headers
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.starts_with("multipart/form-data"))
}

/// envd's upload answer for `paths`, one `{name, type, path}` each.
fn written(paths: &[&str]) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(
        paths
            .iter()
            .map(|p| json!({"name": p.rsplit('/').next(), "type": "file", "path": p}))
            .collect::<Vec<_>>(),
    )
}

/// Decompresses a gzip request body.
async fn gunzip(body: &[u8]) -> Vec<u8> {
    let mut plain = Vec::new();
    GzipDecoder::new(body)
        .read_to_end(&mut plain)
        .await
        .expect("gunzip");
    plain
}

/// The `/files` requests `server` received, in arrival order.
async fn file_requests(server: &MockServer) -> Vec<Request> {
    server
        .received_requests()
        .await
        .expect("recording")
        .into_iter()
        .filter(|request| request.url.path() == "/files")
        .collect()
}

/// Two entries under `/w`, used by most tests.
fn two_entries() -> [WriteEntry; 2] {
    [
        WriteEntry::new("/w/a.txt", "alpha"),
        WriteEntry::new("/w/b.txt", "beta"),
    ]
}

#[tokio::test]
async fn write_files_multipart_sends_one_request_without_a_path_param() {
    // Arrange
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    Mock::given(method("POST"))
        .and(path("/files"))
        .and(has_no_path_param)
        .and(query_param("username", "root"))
        .and(header("x-access-token", ACCESS_TOKEN))
        .and(is_multipart)
        .and(body_string_contains("name=\"file\"; filename=\"/w/a.txt\""))
        .and(body_string_contains("name=\"file\"; filename=\"/w/b.txt\""))
        .and(body_string_contains("alpha"))
        .and(body_string_contains("beta"))
        .respond_with(written(&["/w/a.txt", "/w/b.txt"]))
        .expect(1)
        .mount(&server)
        .await;

    // Act
    let infos = sandbox
        .files()
        .write_files(&two_entries(), &WriteOptions::default().user("root"))
        .await
        .expect("write_files");

    // Assert: both files, in envd's order.
    let paths: Vec<&str> = infos.iter().map(|info| info.path.as_str()).collect();
    assert_eq!(paths, ["/w/a.txt", "/w/b.txt"]);
    assert_eq!(infos[0].name, "a.txt");
    assert_eq!(infos[1].file_type, FileType::File);
}

#[tokio::test]
async fn write_files_with_one_entry_sends_the_path_param() {
    // Arrange
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    Mock::given(method("POST"))
        .and(path("/files"))
        .and(query_param("path", "/w/a.txt"))
        .and(query_param_is_missing("username"))
        .and(is_multipart)
        .and(body_string_contains("name=\"file\"; filename=\"/w/a.txt\""))
        .respond_with(written(&["/w/a.txt"]))
        .expect(1)
        .mount(&server)
        .await;

    // Act
    let infos = sandbox
        .files()
        .write_files(
            &[WriteEntry::new("/w/a.txt", "alpha")],
            &WriteOptions::default(),
        )
        .await
        .expect("write_files");

    // Assert
    assert_eq!(infos.len(), 1);
    assert_eq!(infos[0].path, "/w/a.txt");
}

#[tokio::test]
async fn write_files_with_no_entries_sends_nothing() {
    // Arrange
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;

    // Act
    let infos = sandbox
        .files()
        .write_files(&[], &WriteOptions::default().gzip(true))
        .await
        .expect("write_files");

    // Assert: only the create call reached the server.
    assert!(infos.is_empty());
    let requests = server.received_requests().await.expect("recording");
    assert!(
        requests
            .iter()
            .all(|request| request.url.path() == "/v2/sandboxes"),
        "only the create call may reach the server"
    );
}

#[tokio::test]
async fn octet_stream_write_sends_one_request_per_file() {
    // Arrange: one mock per file, each matching only its own body.
    let server = MockServer::start().await;
    let sandbox = sandbox_with_version(&server, "0.5.7").await;
    for (target, data) in [("/w/a.txt", "alpha"), ("/w/b.txt", "beta")] {
        Mock::given(method("POST"))
            .and(path("/files"))
            .and(query_param("path", target))
            .and(header("content-type", "application/octet-stream"))
            .and(header("x-access-token", ACCESS_TOKEN))
            .and(body_bytes(data.as_bytes()))
            .respond_with(written(&[target]))
            .expect(1)
            .mount(&server)
            .await;
    }

    // Act
    let infos = sandbox
        .files()
        .write_files(&two_entries(), &WriteOptions::default().octet_stream(true))
        .await
        .expect("write_files");

    // Assert: results follow the entries' order; no request was compressed.
    let paths: Vec<&str> = infos.iter().map(|info| info.path.as_str()).collect();
    assert_eq!(paths, ["/w/a.txt", "/w/b.txt"]);
    assert!(
        file_requests(&server)
            .await
            .iter()
            .all(|request| !request.headers.contains_key("content-encoding")),
        "plain octet-stream must not set Content-Encoding"
    );
}

#[tokio::test]
async fn gzip_write_compresses_the_body_and_sets_content_encoding() {
    // Arrange: `gzip(true)` alone, which implies octet-stream.
    let server = MockServer::start().await;
    let sandbox = sandbox_with_version(&server, "0.5.7").await;
    let data: Vec<u8> = (0..=255_u8).cycle().take(100_000).collect();
    Mock::given(method("POST"))
        .and(path("/files"))
        .and(query_param("path", "/w/data.bin"))
        .and(header("content-type", "application/octet-stream"))
        .and(header("content-encoding", "gzip"))
        .respond_with(written(&["/w/data.bin"]))
        .expect(1)
        .mount(&server)
        .await;

    // Act
    sandbox
        .files()
        .write_files(
            &[WriteEntry::new("/w/data.bin", data.clone())],
            &WriteOptions::default().gzip(true),
        )
        .await
        .expect("write_files");

    // Assert: the body is gzip of the data.
    let requests = file_requests(&server).await;
    assert_eq!(requests.len(), 1);
    assert_ne!(requests[0].body, data, "the body must be compressed");
    assert_eq!(gunzip(&requests[0].body).await, data);
}

#[tokio::test]
async fn octet_stream_and_gzip_fall_back_to_plain_multipart_before_envd_0_5_7() {
    // Arrange
    let server = MockServer::start().await;
    let sandbox = sandbox_with_version(&server, "0.5.6").await;
    Mock::given(method("POST"))
        .and(path("/files"))
        .and(has_no_path_param)
        .and(is_multipart)
        .and(body_string_contains("name=\"file\"; filename=\"/w/a.txt\""))
        .and(body_string_contains("alpha"))
        .and(body_string_contains("beta"))
        .respond_with(written(&["/w/a.txt", "/w/b.txt"]))
        .expect(1)
        .mount(&server)
        .await;

    // Act
    let infos = sandbox
        .files()
        .write_files(
            &two_entries(),
            &WriteOptions::default().octet_stream(true).gzip(true),
        )
        .await
        .expect("write_files");

    // Assert: one multipart request, uncompressed.
    assert_eq!(infos.len(), 2);
    let requests = file_requests(&server).await;
    assert_eq!(requests.len(), 1);
    assert!(
        !requests[0].headers.contains_key("content-encoding"),
        "the multipart fallback must not be compressed"
    );
}

#[tokio::test]
async fn octet_stream_uploads_run_concurrently() {
    // Arrange: each upload takes 400 ms to answer.
    let server = MockServer::start().await;
    let sandbox = sandbox_with_version(&server, "0.5.7").await;
    let targets = ["/w/1", "/w/2", "/w/3"];
    for target in targets {
        Mock::given(method("POST"))
            .and(path("/files"))
            .and(query_param("path", target))
            .respond_with(written(&[target]).set_delay(Duration::from_millis(400)))
            .expect(1)
            .mount(&server)
            .await;
    }
    let entries = targets.map(|target| WriteEntry::new(target, "x"));

    // Act
    let started = Instant::now();
    let infos = sandbox
        .files()
        .write_files(&entries, &WriteOptions::default().octet_stream(true))
        .await
        .expect("write_files");
    let elapsed = started.elapsed();

    // Assert: one after another would take at least 1.2 s.
    assert_eq!(infos.len(), 3);
    assert!(
        elapsed < Duration::from_millis(1_000),
        "uploads ran one after another: {elapsed:?}"
    );
}

#[tokio::test]
async fn octet_stream_upload_errors_are_reported() {
    // Arrange: the first file fails with 507, the second answers no entry.
    let server = MockServer::start().await;
    let sandbox = sandbox_with_version(&server, "0.5.7").await;
    Mock::given(method("POST"))
        .and(path("/files"))
        .and(query_param("path", "/w/full"))
        .respond_with(
            ResponseTemplate::new(507).set_body_json(json!({"code": 507, "message": "no space"})),
        )
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/files"))
        .and(query_param("path", "/w/empty"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .mount(&server)
        .await;
    let files = sandbox.files();
    let options = WriteOptions::default().octet_stream(true);

    // Act
    let full = files
        .write_files(&[WriteEntry::new("/w/full", "x")], &options)
        .await
        .expect_err("507");
    let empty = files
        .write_files(&[WriteEntry::new("/w/empty", "x")], &options)
        .await
        .expect_err("empty answer");

    // Assert
    assert!(
        matches!(&full, Error::Envd { status: 507, message } if message == "no space"),
        "{full:?}"
    );
    assert!(matches!(empty, Error::Protocol(_)), "{empty:?}");
}
