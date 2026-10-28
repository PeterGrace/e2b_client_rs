//! Mock tests for `Filesystem::write_files` and its upload encodings
//! (`POST /files`: multipart, octet-stream and gzip), and for file metadata
//! on writes (`X-Metadata-*` headers, `metadata` in answers).

mod common;

use std::collections::HashMap;
use std::time::{Duration, Instant};

use async_compression::tokio::bufread::GzipDecoder;
use common::{ACCESS_TOKEN, sandbox, sandbox_with_version};
use e2b_client::Sandbox;
use e2b_client::{Error, FileOptions, FileType, WriteEntry, WriteOptions};
use serde_json::json;
use tokio::io::AsyncReadExt;
use wiremock::matchers::{
    body_bytes, body_json, body_string_contains, header, method, path, query_param,
    query_param_is_missing,
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

    // Assert: one after another would take at least 3 x 400 ms = 1.2 s, so
    // anything below that sequential minimum means they overlapped.
    assert_eq!(infos.len(), 3);
    let sequential_minimum = Duration::from_millis(400) * 3;
    assert!(
        elapsed < sequential_minimum,
        "uploads ran one after another: {elapsed:?}"
    );
}

#[tokio::test]
async fn octet_stream_batch_fails_fast_without_waiting_for_the_other_uploads() {
    // Arrange: in one batch, one file fails at once with 507 while the other
    // would answer only after 5 s.
    let server = MockServer::start().await;
    let sandbox = sandbox_with_version(&server, "0.5.7").await;
    Mock::given(method("POST"))
        .and(path("/files"))
        .and(query_param("path", "/w/slow"))
        .respond_with(written(&["/w/slow"]).set_delay(Duration::from_secs(5)))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/files"))
        .and(query_param("path", "/w/full"))
        .respond_with(
            ResponseTemplate::new(507).set_body_json(json!({"code": 507, "message": "no space"})),
        )
        .mount(&server)
        .await;
    let entries = [
        WriteEntry::new("/w/slow", "x"),
        WriteEntry::new("/w/full", "x"),
    ];

    // Act
    let started = Instant::now();
    let err = sandbox
        .files()
        .write_files(&entries, &WriteOptions::default().octet_stream(true))
        .await
        .expect_err("507 in the batch");
    let elapsed = started.elapsed();

    // Assert: the failure is returned well before the slow upload would
    // answer, so the slow upload was dropped (cancelled), not awaited.
    assert!(matches!(err, Error::Envd { status: 507, .. }), "{err:?}");
    assert!(
        elapsed < Duration::from_secs(4),
        "write_files waited for the slow upload: {elapsed:?}"
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
        matches!(&full, Error::Envd { status: 507, message, .. } if message == "no space"),
        "{full:?}"
    );
    assert!(matches!(empty, Error::Protocol(_)), "{empty:?}");
}

/// The first envd version that stores file metadata.
const METADATA_ENVD: &str = "0.6.2";

/// Options carrying the two metadata pairs the metadata tests send.
fn metadata_options() -> WriteOptions {
    WriteOptions::default()
        .metadata("team", "m2")
        .metadata("Run", "1")
}

/// Whether a request carries both of [`metadata_options`]' headers. Header
/// names are case-insensitive, and `Run` goes out as `x-metadata-run`.
fn has_metadata_headers(request: &Request) -> bool {
    let value = |name: &str| {
        request
            .headers
            .get(name)
            .and_then(|value| value.to_str().ok())
    };
    value("x-metadata-team") == Some("m2") && value("x-metadata-run") == Some("1")
}

/// Asserts that only the create call reached `server`, so nothing was
/// uploaded.
async fn assert_no_file_requests(server: &MockServer) {
    assert!(
        file_requests(server).await.is_empty(),
        "no /files request may be sent"
    );
}

/// A `{"team": "m2"}` map, the metadata envd reports in these tests.
fn team_m2() -> HashMap<String, String> {
    HashMap::from([("team".to_owned(), "m2".to_owned())])
}

#[tokio::test]
async fn metadata_is_sent_as_x_metadata_headers_on_every_upload() {
    // Arrange: every /files request must carry both headers; any request
    // without them finds no mock and fails with 404.
    let server = MockServer::start().await;
    let sandbox = sandbox_with_version(&server, METADATA_ENVD).await;
    Mock::given(method("POST"))
        .and(path("/files"))
        .and(has_metadata_headers)
        .respond_with(written(&["/w/a.txt"]))
        .expect(5)
        .mount(&server)
        .await;
    let files = sandbox.files();
    let options = metadata_options();

    // Act: one multipart request, two octet-stream requests, one gzip
    // request and one streamed upload.
    files
        .write_files(&two_entries(), &options)
        .await
        .expect("multipart write_files");
    files
        .write_files(&two_entries(), &options.clone().octet_stream(true))
        .await
        .expect("octet-stream write_files");
    files
        .write_files(
            &[WriteEntry::new("/w/a.txt", "alpha")],
            &options.clone().gzip(true),
        )
        .await
        .expect("gzip write_files");
    let data: &'static [u8] = b"streamed";
    files
        .write_stream("/w/s.bin", data, &options)
        .await
        .expect("write_stream");

    // Assert: the expected mix of encodings, all with the headers (checked by
    // the mock's `expect(5)` when the server drops).
    let requests = file_requests(&server).await;
    assert_eq!(
        requests
            .iter()
            .filter(|request| is_multipart(request))
            .count(),
        1
    );
    let gzipped = |request: &&Request| request.headers.contains_key("content-encoding");
    assert_eq!(requests.iter().filter(gzipped).count(), 1);
    assert!(requests.iter().all(has_metadata_headers));
}

#[tokio::test]
async fn invalid_metadata_is_rejected_before_sending() {
    // Arrange
    let server = MockServer::start().await;
    let sandbox = sandbox_with_version(&server, METADATA_ENVD).await;
    let files = sandbox.files();
    let invalid = [
        WriteOptions::default().metadata("", "x"),
        WriteOptions::default().metadata("a b", "x"),
        WriteOptions::default().metadata("\u{e9}", "x"),
        WriteOptions::default().metadata("team", "tab\there"),
    ];

    for options in &invalid {
        // Act
        let batch = files
            .write_files(&two_entries(), options)
            .await
            .expect_err("invalid metadata in write_files");
        let data: &'static [u8] = b"x";
        let streamed = files
            .write_stream("/w/s.bin", data, options)
            .await
            .expect_err("invalid metadata in write_stream");

        // Assert
        for err in [batch, streamed] {
            assert!(
                matches!(&err, Error::InvalidArgument(message) if message.contains("metadata")),
                "{options:?}: {err:?}"
            );
        }
    }
    assert_no_file_requests(&server).await;
}

#[tokio::test]
async fn metadata_is_refused_before_envd_0_6_2() {
    // Arrange
    let server = MockServer::start().await;
    let sandbox = sandbox_with_version(&server, "0.6.1").await;
    let files = sandbox.files();
    let options = metadata_options().octet_stream(true);

    // Act
    let batch = files
        .write_files(&two_entries(), &options)
        .await
        .expect_err("metadata on 0.6.1");
    let data: &'static [u8] = b"x";
    let streamed = files
        .write_stream("/w/s.bin", data, &options)
        .await
        .expect_err("streamed metadata on 0.6.1");

    // Assert: the error names the version needed, and nothing was sent.
    for err in [batch, streamed] {
        assert!(
            matches!(&err, Error::InvalidArgument(message) if message.contains("0.6.2")),
            "{err:?}"
        );
    }
    assert_no_file_requests(&server).await;
}

/// Mounts an upload answer and a `Stat` answer for `/a`, each with `metadata`
/// as given (or without the field for `None`), then returns what `write`
/// and `get_info` report.
async fn reported_metadata(
    server: &MockServer,
    sandbox: &Sandbox,
    metadata: Option<serde_json::Value>,
) -> (HashMap<String, String>, HashMap<String, String>) {
    let mut entry = json!({"name": "a", "type": "file", "path": "/a"});
    if let Some(metadata) = metadata {
        entry["metadata"] = metadata;
    }
    server.reset().await;
    Mock::given(method("POST"))
        .and(path("/files"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([entry.clone()])))
        .mount(server)
        .await;
    Mock::given(method("POST"))
        .and(path("/filesystem.Filesystem/Stat"))
        .and(body_json(json!({"path": "/a"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"entry": entry})))
        .mount(server)
        .await;
    let files = sandbox.files();
    let written = files
        .write("/a", "x", &WriteOptions::default())
        .await
        .expect("write");
    let info = files
        .get_info("/a", &FileOptions::default())
        .await
        .expect("stat");
    (written.metadata, info.metadata)
}

#[tokio::test]
async fn upload_response_and_stat_report_metadata() {
    // Arrange
    let server = MockServer::start().await;
    let sandbox = sandbox_with_version(&server, METADATA_ENVD).await;

    // Act
    let with = reported_metadata(&server, &sandbox, Some(json!({"team": "m2"}))).await;
    let without = reported_metadata(&server, &sandbox, None).await;

    // Assert: the map when envd reports one, else empty in both.
    assert_eq!(with, (team_m2(), team_m2()));
    assert!(without.0.is_empty() && without.1.is_empty(), "{without:?}");
}
