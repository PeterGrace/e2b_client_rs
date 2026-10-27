//! Mock tests for `Sandbox::upload_url` and `Sandbox::download_url`: the
//! `/files` URL, its query pairs and the SDKs' v1 signature over the envd
//! access token.

mod common;

use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::Engine;
use base64::engine::general_purpose::STANDARD_NO_PAD;
use common::{ACCESS_TOKEN, sandbox, sandbox_from_json, sandbox_json, sandbox_with_version};
use e2b_client::{Error, FileUrlOptions};
use sha2::{Digest, Sha256};
use url::Url;
use wiremock::MockServer;

/// The SDK's v1 signature of `raw`, recomputed independently of the crate.
fn expected_signature(raw: &str) -> String {
    format!(
        "v1_{}",
        STANDARD_NO_PAD.encode(Sha256::digest(raw.as_bytes()))
    )
}

/// Parses `url`, checks it is `<server>/files`, never carries the access
/// token, and returns its decoded query pairs.
fn query_of(server: &MockServer, url: &str) -> HashMap<String, String> {
    assert!(
        url.starts_with(&format!("{}/files?", server.uri())),
        "not a /files URL on the envd base"
    );
    assert!(
        !url.contains(ACCESS_TOKEN),
        "access token leaked into a URL"
    );
    let parsed = Url::parse(url).expect("parse url");
    // `into_owned` copies each decoded `Cow<str>` pair into a `String`.
    parsed.query_pairs().into_owned().collect()
}

/// The current Unix time in seconds.
fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after 1970")
        .as_secs()
}

#[tokio::test]
async fn download_url_is_signed_with_the_access_token() {
    // Arrange
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;

    // Act
    let url = sandbox
        .download_url("/home/user/a b.txt", &FileUrlOptions::default())
        .expect("download url");

    // Assert: the signature is over the raw path, whatever the encoding.
    let query = query_of(&server, &url);
    assert_eq!(query["path"], "/home/user/a b.txt");
    assert!(!query.contains_key("username"));
    assert!(!query.contains_key("signature_expiration"));
    assert_eq!(
        query["signature"],
        expected_signature(&format!("/home/user/a b.txt:read::{ACCESS_TOKEN}"))
    );
}

#[tokio::test]
async fn upload_url_with_user_and_expiration() {
    // Arrange
    let server = MockServer::start().await;
    let sandbox = sandbox(&server).await;
    let options = FileUrlOptions::default()
        .user("root")
        .expires_in(Duration::from_secs(120));

    // Act
    let url = sandbox
        .upload_url("/tmp/up.bin", &options)
        .expect("upload url");

    // Assert
    let query = query_of(&server, &url);
    assert_eq!(query["path"], "/tmp/up.bin");
    assert_eq!(query["username"], "root");
    let expiration: u64 = query["signature_expiration"]
        .parse()
        .expect("numeric expiration");
    assert!(
        expiration.abs_diff(unix_now() + 120) <= 5,
        "expiration {expiration} is not about now + 120 s"
    );
    assert_eq!(
        query["signature"],
        expected_signature(&format!(
            "/tmp/up.bin:write:root:{ACCESS_TOKEN}:{expiration}"
        ))
    );
}

#[tokio::test]
async fn legacy_envd_signs_for_the_default_user() {
    // Arrange: envd before 0.4.0 has no default user.
    let server = MockServer::start().await;
    let sandbox = sandbox_with_version(&server, "0.3.9").await;

    // Act
    let url = sandbox
        .download_url("/home/user/a.txt", &FileUrlOptions::default())
        .expect("download url");

    // Assert
    let query = query_of(&server, &url);
    assert_eq!(query["username"], "user");
    assert_eq!(
        query["signature"],
        expected_signature(&format!("/home/user/a.txt:read:user:{ACCESS_TOKEN}"))
    );
}

#[tokio::test]
async fn unsecured_sandbox_gets_an_unsigned_url_and_rejects_expiration() {
    // Arrange: no envd access token.
    let server = MockServer::start().await;
    let mut created = sandbox_json("0.5.7");
    created["envdAccessToken"] = serde_json::Value::Null;
    let sandbox = sandbox_from_json(&server, created).await;

    // Act
    let url = sandbox
        .upload_url("/tmp/up.bin", &FileUrlOptions::default())
        .expect("upload url");
    let with_expiry = sandbox.download_url(
        "/tmp/up.bin",
        &FileUrlOptions::default().expires_in(Duration::from_secs(60)),
    );

    // Assert
    let query = query_of(&server, &url);
    assert_eq!(query["path"], "/tmp/up.bin");
    assert!(!query.contains_key("signature"));
    assert!(!query.contains_key("signature_expiration"));
    let err = with_expiry.expect_err("expiry on an unsecured sandbox");
    assert!(
        matches!(&err, Error::InvalidArgument(message) if message.contains("secured")),
        "{err:?}"
    );
}
