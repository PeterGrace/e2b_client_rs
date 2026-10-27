//! Mock tests for streamed input (`process.Process/StreamInput`).

mod common;

use common::{
    ACCESS_TOKEN, decode_request_envelopes, end_stream, message, sandbox, stream_response,
};
use e2b_client::{ConnectCode, Error, StreamInputOptions};
use serde_json::json;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer};

const ROUTE: &str = "/process.Process/StreamInput";

#[tokio::test]
async fn stream_input_sends_start_then_one_data_envelope_per_chunk() {
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    Mock::given(method("POST"))
        .and(path(ROUTE))
        .and(header("content-type", "application/connect+json"))
        .and(header("connect-protocol-version", "1"))
        .and(header("x-access-token", ACCESS_TOKEN))
        .respond_with(stream_response(&[
            message(json!({})),
            end_stream(json!({})),
        ]))
        .expect(1)
        .mount(&server)
        .await;

    let mut input = sandbox
        .commands()
        .stream_input(7, &StreamInputOptions::default().timeout(None));
    input.send("a").await.expect("send a");
    input.send(b"b".to_vec()).await.expect("send b");
    // `close` returns only once envd has answered the finished stream.
    input.close().await.expect("close");

    let requests = server.received_requests().await.expect("requests");
    let stream = requests
        .iter()
        .find(|request| request.url.path() == ROUTE)
        .expect("StreamInput request");
    assert_eq!(
        decode_request_envelopes(&stream.body),
        vec![
            json!({"start": {"process": {"pid": 7}}}),
            json!({"data": {"input": {"stdin": "YQ=="}}}),
            json!({"data": {"input": {"stdin": "Yg=="}}}),
        ]
    );
    assert!(
        !stream.headers.contains_key("connect-timeout-ms"),
        "no deadline header when the timeout is None"
    );
}

#[tokio::test]
async fn stream_input_deadline_defaults_to_60_s() {
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    Mock::given(method("POST"))
        .and(path(ROUTE))
        .and(header("connect-timeout-ms", "60000"))
        .respond_with(stream_response(&[
            message(json!({})),
            end_stream(json!({})),
        ]))
        .expect(1)
        .mount(&server)
        .await;

    // No timeout given: the SDKs' 60 s default applies.
    let input = sandbox
        .commands()
        .stream_input(7, &StreamInputOptions::default());

    input.close().await.expect("close");
}

#[tokio::test]
async fn stream_input_error_is_reported_by_close() {
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    Mock::given(method("POST"))
        .and(path(ROUTE))
        .respond_with(stream_response(&[end_stream(
            json!({"error": {"code": "not_found", "message": "process 7 not found"}}),
        )]))
        .mount(&server)
        .await;

    let mut input = sandbox
        .commands()
        .stream_input(7, &StreamInputOptions::default().timeout(None));
    input
        .send("x")
        .await
        .expect("queued before the server answered");
    let err = input.close().await.expect_err("not found");

    assert!(
        matches!(
            err,
            Error::Rpc {
                code: ConnectCode::NotFound,
                ..
            }
        ),
        "{err:?}"
    );
}
