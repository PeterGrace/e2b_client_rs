//! Sandbox network settings against a mock server.

mod common;

use common::{SANDBOX_ID, client_for, sandbox_json};
use e2b_client::{ALL_TRAFFIC, CreateSandboxParams, NetworkConfig, NetworkRule};
use serde_json::{Value, json};
use wiremock::matchers::{body_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// A `SandboxDetail` with `extra` merged in.
fn detail_json(extra: Value) -> Value {
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

/// Mounts a create endpoint that only matches `body`, once.
async fn expect_create_body(server: &MockServer, body: Value) {
    Mock::given(method("POST"))
        .and(path("/v2/sandboxes"))
        .and(body_json(body))
        .respond_with(ResponseTemplate::new(201).set_body_json(sandbox_json("0.5.7")))
        .expect(1)
        .mount(server)
        .await;
}

#[tokio::test]
async fn create_sends_the_network_object_in_camel_case() {
    // Arrange
    let server = MockServer::start().await;
    expect_create_body(
        &server,
        json!({
            "templateID": "base",
            "metadata": {},
            "envVars": {},
            "network": {
                "allowOut": ["1.1.1.1", "api.example.test"],
                "denyOut": ["0.0.0.0/0"],
                "dnsServers": [],
                "rules": {"api.example.test": [{"transform": {"headers": {"X-Key": "v"}}}]},
                "allowPublicTraffic": false,
                "maskRequestHost": "${PORT}-app.example.test",
                "httpsPorts": [3000]
            }
        }),
    )
    .await;
    let network = NetworkConfig::new()
        .allow_out(["1.1.1.1", "api.example.test"])
        .deny_out([ALL_TRAFFIC])
        .dns_servers(Vec::<String>::new())
        .rule("api.example.test", NetworkRule::new().header("X-Key", "v"))
        .allow_public_traffic(false)
        .mask_request_host("${PORT}-app.example.test")
        .https_ports([3000]);

    // Act
    let sandbox = client_for(&server)
        .create_sandbox(&CreateSandboxParams::default().network(network))
        .await
        .expect("create");

    // Assert: the mock only matches the exact body.
    assert_eq!(sandbox.sandbox_id(), SANDBOX_ID);
}

#[tokio::test]
async fn create_sends_network_next_to_allow_internet_access() {
    // Arrange
    let server = MockServer::start().await;
    expect_create_body(
        &server,
        json!({
            "templateID": "base",
            "metadata": {},
            "envVars": {},
            "allow_internet_access": false,
            "network": {"allowOut": ["1.1.1.1"]}
        }),
    )
    .await;
    let params = CreateSandboxParams::default()
        .allow_internet_access(false)
        .network(NetworkConfig::new().allow_out(["1.1.1.1"]));

    // Act
    let sandbox = client_for(&server)
        .create_sandbox(&params)
        .await
        .expect("create");

    // Assert: the mock only matches the exact body.
    assert_eq!(sandbox.sandbox_id(), SANDBOX_ID);
}

#[tokio::test]
async fn create_leaves_out_a_network_with_nothing_set() {
    // Arrange: the SDK sends no `network` for an empty one.
    let server = MockServer::start().await;
    expect_create_body(
        &server,
        json!({"templateID": "base", "metadata": {}, "envVars": {}}),
    )
    .await;

    // Act
    let sandbox = client_for(&server)
        .create_sandbox(&CreateSandboxParams::default().network(NetworkConfig::new()))
        .await
        .expect("create");

    // Assert: the mock only matches the exact body.
    assert_eq!(sandbox.sandbox_id(), SANDBOX_ID);
}

#[tokio::test]
async fn get_info_reads_network_and_internet_access() {
    // Arrange
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/sandboxes/{SANDBOX_ID}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(detail_json(json!({
            "allowInternetAccess": false,
            "network": {
                "allowOut": ["1.1.1.1"], "denyOut": null, "dnsServers": ["8.8.8.8"],
                "allowPublicTraffic": true, "maskRequestHost": "${PORT}-app.example.test",
                "httpsPorts": [3000],
                "rules": {"api.example.test": [{"transform": {"headers": {"X-Key": "v"}}}]},
                "egressProxy": null
            }
        }))))
        .mount(&server)
        .await;

    // Act
    let info = client_for(&server)
        .get_sandbox_info(SANDBOX_ID)
        .await
        .expect("info");

    // Assert
    assert_eq!(info.allow_internet_access, Some(false));
    let network = info.network.expect("network");
    assert_eq!(network.allow_out, ["1.1.1.1"]);
    assert!(network.deny_out.is_empty());
    assert_eq!(
        network.dns_servers.as_deref(),
        Some(&["8.8.8.8".to_owned()][..])
    );
    assert_eq!(network.allow_public_traffic, Some(true));
    assert_eq!(
        network.mask_request_host.as_deref(),
        Some("${PORT}-app.example.test")
    );
    assert_eq!(network.https_ports, [3000]);
    let rule = &network.rules["api.example.test"][0];
    assert_eq!(rule.headers().get("X-Key").map(String::as_str), Some("v"));
}

#[tokio::test]
async fn get_info_reads_null_network_fields_as_none() {
    // Arrange: `allowInternetAccess` is nullable in `SandboxDetail`.
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/sandboxes/{SANDBOX_ID}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(detail_json(json!({
            "allowInternetAccess": null,
            "network": null
        }))))
        .mount(&server)
        .await;

    // Act
    let info = client_for(&server)
        .get_sandbox_info(SANDBOX_ID)
        .await
        .expect("info");

    // Assert
    assert_eq!(info.allow_internet_access, None);
    assert_eq!(info.network, None);
}

#[tokio::test]
async fn get_info_without_network_fields_reads_none() {
    // Arrange: a server that predates the fields.
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/sandboxes/{SANDBOX_ID}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(detail_json(json!({}))))
        .mount(&server)
        .await;

    // Act
    let info = client_for(&server)
        .get_sandbox_info(SANDBOX_ID)
        .await
        .expect("info");

    // Assert
    assert_eq!(info.allow_internet_access, None);
    assert_eq!(info.network, None);
}
