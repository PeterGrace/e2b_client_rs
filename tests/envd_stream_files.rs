//! Mock tests for `Filesystem::write_stream` and `Filesystem::read_stream`
//! (`POST /files` from an `AsyncRead`, `GET /files` as chunks).

mod common;

use std::io::{self, Cursor};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use async_compression::tokio::bufread::GzipDecoder;
use common::{
    client_with_timeout, sandbox, sandbox_from_json_with_client, sandbox_json, sandbox_with_version,
};
use e2b_client::{Error, FileOptions, Sandbox, WriteOptions};
use serde_json::json;
use tokio::io::{AsyncRead, AsyncReadExt, ReadBuf};
use wiremock::matchers::{body_string_contains, header, method, path, query_param};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

/// The client request timeout in the timeout tests. Generous, so that
/// creating the sandbox on a slow CI runner stays well inside it.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(1);

/// How long the mock waits before answering in the timeout tests: three
/// times [`REQUEST_TIMEOUT`], so a bounded call always times out first.
const SLOW_ANSWER: Duration = Duration::from_secs(3);

/// envd's upload answer for one written `path`.
fn written(path: &str) -> ResponseTemplate {
    ResponseTemplate::new(200)
        .set_body_json(json!([{"name": path.rsplit('/').next(), "type": "file", "path": path}]))
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

/// A sandbox on envd 0.5.7 whose client has [`REQUEST_TIMEOUT`].
async fn sandbox_with_short_timeout(server: &MockServer) -> Sandbox {
    let client = client_with_timeout(server, Some(REQUEST_TIMEOUT));
    sandbox_from_json_with_client(server, &client, sandbox_json("0.5.7")).await
}

/// Whether `err` is reqwest's whole-request timeout.
fn is_request_timeout(err: &Error) -> bool {
    matches!(err, Error::Transport(inner) if inner.is_timeout())
}

/// A reader that fails on its first read.
struct FailingReader;

impl AsyncRead for FailingReader {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Poll::Ready(Err(io::Error::other("disk on fire")))
    }
}

/// `len` bytes of a repeating pattern, so misordered chunks would show.
fn patterned(len: usize) -> Vec<u8> {
    (0..=250_u8).cycle().take(len).collect()
}

#[tokio::test]
async fn write_stream_sends_an_octet_stream_body() {
    // Arrange
    let server = MockServer::start().await;
    let sandbox = sandbox_with_version(&server, "0.5.7").await;
    let data = patterned(300_000);
    Mock::given(method("POST"))
        .and(path("/files"))
        .and(query_param("path", "/w/big.bin"))
        .and(query_param("username", "root"))
        .and(header("content-type", "application/octet-stream"))
        .respond_with(written("/w/big.bin"))
        .expect(1)
        .mount(&server)
        .await;
    let reader = Cursor::new(data.clone());

    // Act: through `tokio::spawn`, which only compiles if the future is `Send`.
    let info = tokio::spawn(async move {
        sandbox
            .files()
            .write_stream("/w/big.bin", reader, &WriteOptions::default().user("root"))
            .await
    })
    .await
    .expect("join")
    .expect("write_stream");

    // Assert
    assert_eq!(info.path, "/w/big.bin");
    let requests = file_requests(&server).await;
    assert_eq!(requests.len(), 1);
    assert!(requests[0].headers.get("content-encoding").is_none());
    assert!(
        requests[0].body == data,
        "the body must be the reader's bytes"
    );
}

#[tokio::test]
async fn write_stream_with_gzip_compresses_on_the_fly() {
    // Arrange
    let server = MockServer::start().await;
    let sandbox = sandbox_with_version(&server, "0.5.7").await;
    let data = patterned(300_000);
    Mock::given(method("POST"))
        .and(path("/files"))
        .and(query_param("path", "/w/big.bin"))
        .and(header("content-type", "application/octet-stream"))
        .and(header("content-encoding", "gzip"))
        .respond_with(written("/w/big.bin"))
        .expect(1)
        .mount(&server)
        .await;

    // Act
    sandbox
        .files()
        .write_stream(
            "/w/big.bin",
            Cursor::new(data.clone()),
            &WriteOptions::default().gzip(true),
        )
        .await
        .expect("write_stream");

    // Assert
    let requests = file_requests(&server).await;
    assert_eq!(requests.len(), 1);
    assert!(requests[0].body != data, "the body must be compressed");
    assert!(gunzip(&requests[0].body).await == data, "gunzip differs");
}

#[tokio::test]
async fn write_stream_falls_back_to_multipart_before_envd_0_5_7() {
    // Arrange: gzip asked for, which the multipart fallback drops.
    let server = MockServer::start().await;
    let sandbox = sandbox_with_version(&server, "0.5.6").await;
    Mock::given(method("POST"))
        .and(path("/files"))
        .and(query_param("path", "/w/data.csv"))
        .and(body_string_contains(
            "name=\"file\"; filename=\"/w/data.csv\"",
        ))
        .and(body_string_contains("a,b\n1,2\n"))
        .respond_with(written("/w/data.csv"))
        .expect(1)
        .mount(&server)
        .await;
    let data: &'static [u8] = b"a,b\n1,2\n";

    // Act
    sandbox
        .files()
        .write_stream("/w/data.csv", data, &WriteOptions::default().gzip(true))
        .await
        .expect("write_stream");

    // Assert
    let requests = file_requests(&server).await;
    assert_eq!(requests.len(), 1);
    let content_type = requests[0]
        .headers
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    assert!(
        content_type.starts_with("multipart/form-data"),
        "{content_type}"
    );
    assert!(requests[0].headers.get("content-encoding").is_none());
}

#[tokio::test]
async fn write_stream_is_not_bounded_by_the_request_timeout() {
    // Arrange
    let server = MockServer::start().await;
    let sandbox = sandbox_with_short_timeout(&server).await;
    Mock::given(method("POST"))
        .and(path("/files"))
        .respond_with(written("/w/slow.bin").set_delay(SLOW_ANSWER))
        .mount(&server)
        .await;

    // Act
    let streamed = sandbox
        .files()
        .write_stream("/w/slow.bin", &b"slow"[..], &WriteOptions::default())
        .await;
    let buffered = sandbox
        .files()
        .write("/w/slow.bin", "slow", &WriteOptions::default())
        .await;

    // Assert: the streamed upload outlasts the request timeout; the
    // in-memory one, which the timeout bounds, does not.
    assert_eq!(streamed.expect("write_stream").path, "/w/slow.bin");
    let err = buffered.expect_err("write must time out");
    assert!(is_request_timeout(&err), "{err:?}");
}

#[tokio::test]
async fn write_stream_surfaces_a_reader_error_as_transport() {
    // Arrange
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    Mock::given(method("POST"))
        .and(path("/files"))
        .respond_with(written("/w/x"))
        .mount(&server)
        .await;

    // Act
    let err = sandbox
        .files()
        .write_stream("/w/x", FailingReader, &WriteOptions::default())
        .await
        .expect_err("the reader fails");

    // Assert
    assert!(matches!(err, Error::Transport(_)), "{err:?}");
}

#[tokio::test]
async fn read_stream_yields_the_body_in_chunks() {
    // Arrange
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    let data = patterned(1 << 20);
    Mock::given(method("GET"))
        .and(path("/files"))
        .and(query_param("path", "/w/big.bin"))
        .and(query_param("username", "root"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(data.clone()))
        .expect(1)
        .mount(&server)
        .await;

    // Act
    let mut stream = sandbox
        .files()
        .read_stream("/w/big.bin", &FileOptions::new().user("root"))
        .await
        .expect("read_stream");
    let mut received = Vec::with_capacity(data.len());
    while let Some(chunk) = stream.next_chunk().await.expect("next_chunk") {
        received.extend_from_slice(&chunk);
    }

    // Assert
    assert!(received == data, "the chunks must concatenate to the file");
}

#[tokio::test]
async fn read_stream_is_not_bounded_by_the_request_timeout() {
    // Arrange
    let server = MockServer::start().await;
    let sandbox = sandbox_with_short_timeout(&server).await;
    Mock::given(method("GET"))
        .and(path("/files"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(&b"slow"[..])
                .set_delay(SLOW_ANSWER),
        )
        .mount(&server)
        .await;

    // Act
    let streamed = async {
        let mut stream = sandbox
            .files()
            .read_stream("/w/slow", &FileOptions::default())
            .await?;
        let mut received = Vec::new();
        while let Some(chunk) = stream.next_chunk().await? {
            received.extend_from_slice(&chunk);
        }
        Ok::<_, Error>(received)
    }
    .await;
    let buffered = sandbox
        .files()
        .read("/w/slow", &FileOptions::default())
        .await;

    // Assert
    assert_eq!(streamed.expect("read_stream"), b"slow");
    let err = buffered.expect_err("read must time out");
    assert!(is_request_timeout(&err), "{err:?}");
}

#[tokio::test]
async fn read_stream_of_a_missing_file_is_envd_not_found() {
    // Arrange
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    Mock::given(method("GET"))
        .and(path("/files"))
        .respond_with(ResponseTemplate::new(404).set_body_json(json!({"message": "no such file"})))
        .mount(&server)
        .await;

    // Act: `read_stream` itself must fail, before any chunk is pulled.
    let result = sandbox
        .files()
        .read_stream("/w/missing", &FileOptions::default())
        .await;

    // Assert
    let err = result.expect_err("missing file");
    assert!(
        matches!(&err, Error::Envd { status: 404, message, .. } if message == "no such file"),
        "{err:?}"
    );
    assert!(err.is_not_found());
}
