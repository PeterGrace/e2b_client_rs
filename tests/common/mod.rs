//! Shared helpers for the mock-server tests.
//!
//! Every test runs its own `wiremock` server that plays both the REST API and
//! envd: the client is configured with `sandbox_url` pointing at the same
//! server, the official SDKs' `E2B_SANDBOX_URL` override.

#![allow(dead_code)]

use std::time::Duration;

use e2b_client::{Client, ConnectionConfig, Sandbox};
use serde_json::{Value, json};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

pub const API_KEY: &str = "test-api-key";
pub const ACCESS_TOKEN: &str = "test-access-token";
pub const SANDBOX_ID: &str = "sbx-test";

/// A client whose REST and envd traffic both go to `server`.
pub fn client_for(server: &MockServer) -> Client {
    client_with_timeout(server, Some(Duration::from_secs(5)))
}

/// Like [`client_for`] with a specific request timeout.
pub fn client_with_timeout(server: &MockServer, timeout: Option<Duration>) -> Client {
    let config = ConnectionConfig::builder()
        .api_key(API_KEY)
        .api_url(server.uri())
        .sandbox_url(server.uri())
        .domain("example.test")
        .request_timeout(timeout)
        .build()
        .expect("config");
    Client::new(config).expect("client")
}

/// The `Sandbox` JSON the API returns from create/connect.
pub fn sandbox_json(envd_version: &str) -> Value {
    json!({
        "templateID": "base",
        "sandboxID": SANDBOX_ID,
        "clientID": "client",
        "envdVersion": envd_version,
        "envdAccessToken": ACCESS_TOKEN,
        "domain": null
    })
}

/// A running sandbox's `SandboxDetail` (`GET /sandboxes/{id}`) with the
/// fields of `extra` merged in, replacing any of the same name.
pub fn detail_json(extra: Value) -> Value {
    let mut detail = json!({
        "templateID": "base", "sandboxID": SANDBOX_ID, "clientID": "c",
        "startedAt": "2026-10-27T12:00:00Z", "endAt": "2026-10-27T12:05:00Z",
        "envdVersion": "0.5.7", "cpuCount": 2, "memoryMB": 512, "diskSizeMB": 2048,
        "state": "running"
    });
    if let (Some(detail), Value::Object(extra)) = (detail.as_object_mut(), extra) {
        detail.extend(extra);
    }
    detail
}

/// Mounts a create endpoint answering `created` and creates a sandbox from it.
pub async fn sandbox_from_json(server: &MockServer, created: Value) -> Sandbox {
    sandbox_from_json_with_client(server, &client_for(server), created).await
}

/// Like [`sandbox_from_json`], creating the sandbox through `client` (for
/// example one from [`client_with_timeout`]).
pub async fn sandbox_from_json_with_client(
    server: &MockServer,
    client: &Client,
    created: Value,
) -> Sandbox {
    Mock::given(method("POST"))
        .and(path("/v2/sandboxes"))
        .respond_with(ResponseTemplate::new(201).set_body_json(created))
        .mount(server)
        .await;
    client
        .create_sandbox(&e2b_client::CreateSandboxParams::default())
        .await
        .expect("create sandbox")
}

/// Mounts a create endpoint and creates a sandbox reporting `envd_version`.
pub async fn sandbox_with_version(server: &MockServer, envd_version: &str) -> Sandbox {
    sandbox_from_json(server, sandbox_json(envd_version)).await
}

/// A sandbox on a current envd.
pub async fn sandbox(server: &MockServer) -> Sandbox {
    sandbox_with_version(server, "0.5.7").await
}

/// One Connect envelope: flag byte, big-endian length, payload.
pub fn envelope(flags: u8, payload: &Value) -> Vec<u8> {
    let data = serde_json::to_vec(payload).expect("json");
    let mut out = Vec::with_capacity(5 + data.len());
    out.push(flags);
    out.extend_from_slice(&u32::try_from(data.len()).expect("len").to_be_bytes());
    out.extend_from_slice(&data);
    out
}

/// A data envelope.
pub fn message(payload: Value) -> Vec<u8> {
    envelope(0, &payload)
}

/// An end-of-stream envelope.
pub fn end_stream(payload: Value) -> Vec<u8> {
    envelope(0b10, &payload)
}

/// A 200 response carrying the concatenated envelopes.
pub fn stream_response(frames: &[Vec<u8>]) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_raw(frames.concat(), "application/connect+json")
}

/// Decodes the single envelope of a streaming request body.
pub fn decode_request_envelope(body: &[u8]) -> Value {
    assert!(body.len() >= 5, "request body too short for an envelope");
    assert_eq!(body[0], 0, "request envelope flags");
    let len = u32::from_be_bytes([body[1], body[2], body[3], body[4]]) as usize;
    assert_eq!(body.len(), 5 + len, "request envelope length");
    serde_json::from_slice(&body[5..]).expect("request envelope json")
}

/// Decodes every envelope of a streaming request body, in order.
pub fn decode_request_envelopes(mut body: &[u8]) -> Vec<Value> {
    let mut messages = Vec::new();
    while !body.is_empty() {
        assert!(body.len() >= 5, "request body too short for an envelope");
        assert_eq!(body[0], 0, "request envelope flags");
        let len = u32::from_be_bytes([body[1], body[2], body[3], body[4]]) as usize;
        assert!(body.len() >= 5 + len, "request envelope length");
        messages.push(serde_json::from_slice(&body[5..5 + len]).expect("request envelope json"));
        body = &body[5 + len..];
    }
    messages
}
