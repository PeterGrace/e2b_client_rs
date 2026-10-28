//! Creating a sandbox with an MCP configuration, against a mock server that
//! plays both the API and envd.

mod common;

use common::{
    SANDBOX_ID, client_for, decode_request_envelope, end_stream, message, sandbox_json,
    stream_response,
};
use e2b_client::{ConnectOptions, CreateSandboxParams, Error};
use secrecy::ExposeSecret;
use serde_json::{Map, Value, json};
use wiremock::matchers::{body_json, method, path, query_param};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

const START_PATH: &str = "/process.Process/Start";

fn fetch_config() -> Map<String, Value> {
    let Value::Object(config) = json!({"fetch": {}}) else {
        panic!("an object");
    };
    config
}

/// The frames of a process that writes `stderr_b64` (base64) to stderr and
/// exits with `exit_code`.
fn process_frames(exit_code: i32, stderr_b64: &str) -> Vec<Vec<u8>> {
    vec![
        message(json!({"event": {"start": {"pid": 7}}})),
        message(json!({"event": {"data": {"stderr": stderr_b64}}})),
        message(json!({"event": {"end": {"exitCode": exit_code, "exited": true}}})),
        end_stream(json!({})),
    ]
}

/// Mounts a create answering 201 for any body.
async fn mount_create(server: &MockServer) {
    Mock::given(method("POST"))
        .and(path("/v2/sandboxes"))
        .respond_with(ResponseTemplate::new(201).set_body_json(sandbox_json("0.5.7")))
        .mount(server)
        .await;
}

/// Mounts a gateway start that exits with `exit_code` and `stderr_b64`.
async fn mount_start(server: &MockServer, exit_code: i32, stderr_b64: &str) {
    Mock::given(method("POST"))
        .and(path(START_PATH))
        .respond_with(stream_response(&process_frames(exit_code, stderr_b64)))
        .expect(1)
        .mount(server)
        .await;
}

/// Mounts the kill of the test sandbox, answering `status`, expected once.
async fn mount_kill(server: &MockServer, status: u16) {
    Mock::given(method("DELETE"))
        .and(path(format!("/sandboxes/{SANDBOX_ID}")))
        .respond_with(ResponseTemplate::new(status))
        .expect(1)
        .mount(server)
        .await;
}

/// The decoded `process` of the one gateway start the server received.
async fn started_process(server: &MockServer) -> (Request, Value) {
    let requests = server.received_requests().await.expect("recording");
    let start = requests
        .into_iter()
        .find(|request| request.url.path() == START_PATH)
        .expect("start");
    let process = decode_request_envelope(&start.body)["process"].clone();
    (start, process)
}

#[tokio::test]
async fn create_with_mcp_uses_the_gateway_template_and_starts_it_as_root() {
    // Arrange
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v2/sandboxes"))
        .and(body_json(json!({
            "templateID": "mcp-gateway", "metadata": {}, "envVars": {}, "mcp": {"fetch": {}}
        })))
        .respond_with(ResponseTemplate::new(201).set_body_json(sandbox_json("0.5.7")))
        .expect(1)
        .mount(&server)
        .await;
    mount_start(&server, 0, "").await;

    // Act
    let sandbox = client_for(&server)
        .create_sandbox(&CreateSandboxParams::default().mcp(fetch_config()))
        .await
        .expect("create");

    // Assert: the command, its user and the token it was given.
    let (start, process) = started_process(&server).await;
    // base64("root:")
    assert_eq!(
        start
            .headers
            .get("authorization")
            .expect("user")
            .to_str()
            .expect("ascii"),
        "Basic cm9vdDo="
    );
    assert_eq!(
        process["args"],
        json!(["-l", "-c", r#"mcp-gateway --config '{"fetch":{}}'"#])
    );
    let envs = process["envs"].as_object().expect("envs");
    assert_eq!(envs.len(), 1, "only the token variable: {:?}", envs.keys());
    let sent_token = envs["GATEWAY_ACCESS_TOKEN"]
        .as_str()
        .expect("token env")
        .to_owned();
    let parsed = uuid::Uuid::parse_str(&sent_token).expect("a UUID");
    assert_eq!(parsed.get_version_num(), 4, "a random (v4) UUID");
    assert_eq!(
        sent_token,
        parsed.hyphenated().to_string(),
        "lowercase, hyphenated"
    );
    let token = sandbox.mcp_token().await.expect("cached token");
    assert_eq!(token.expose_secret(), sent_token);
    // A clone shares the cached token, so it reads no file either (none is
    // mocked: a read would fail).
    let clone = sandbox.clone();
    assert_eq!(
        clone.mcp_token().await.expect("shared").expose_secret(),
        sent_token
    );
    assert_eq!(
        sandbox.mcp_url(),
        format!("https://50005-{SANDBOX_ID}.example.test/mcp")
    );
}

#[tokio::test]
async fn an_explicit_template_wins_over_the_gateway_default() {
    // Arrange
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v2/sandboxes"))
        .and(body_json(json!({
            "templateID": "my-mcp", "metadata": {}, "envVars": {}, "mcp": {"fetch": {}}
        })))
        .respond_with(ResponseTemplate::new(201).set_body_json(sandbox_json("0.5.7")))
        .expect(1)
        .mount(&server)
        .await;
    mount_start(&server, 0, "").await;

    // Act
    let created = client_for(&server)
        .create_sandbox(&CreateSandboxParams::new("my-mcp").mcp(fetch_config()))
        .await;

    // Assert: the mock answers only the exact body, and the gateway starts.
    created.expect("create");
}

#[tokio::test]
async fn an_empty_mcp_config_is_left_out_of_the_body_but_starts_the_gateway() {
    // Arrange: the Python SDK sends `mcp or UNSET` but starts the gateway
    // whenever `mcp is not None`.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v2/sandboxes"))
        .and(body_json(json!({
            "templateID": "mcp-gateway", "metadata": {}, "envVars": {}
        })))
        .respond_with(ResponseTemplate::new(201).set_body_json(sandbox_json("0.5.7")))
        .expect(1)
        .mount(&server)
        .await;
    mount_start(&server, 0, "").await;

    // Act
    client_for(&server)
        .create_sandbox(&CreateSandboxParams::default().mcp(Map::new()))
        .await
        .expect("create");

    // Assert
    let (_, process) = started_process(&server).await;
    assert_eq!(
        process["args"],
        json!(["-l", "-c", "mcp-gateway --config '{}'"])
    );
}

#[tokio::test]
async fn create_without_mcp_starts_no_gateway() {
    // Arrange
    let server = MockServer::start().await;
    mount_create(&server).await;

    // Act
    client_for(&server)
        .create_sandbox(&CreateSandboxParams::default())
        .await
        .expect("create");

    // Assert
    let requests = server.received_requests().await.expect("recording");
    assert_eq!(requests.len(), 1, "only the create call");
}

#[tokio::test]
async fn a_failed_gateway_kills_the_sandbox_and_reports_stderr() {
    // Arrange
    let server = MockServer::start().await;
    mount_create(&server).await;
    // base64("bad config\n")
    mount_start(&server, 1, "YmFkIGNvbmZpZwo=").await;
    mount_kill(&server, 204).await;

    // Act
    let error = client_for(&server)
        .create_sandbox(&CreateSandboxParams::default().mcp(fetch_config()))
        .await
        .expect_err("gateway failure");

    // Assert: the kill is checked by `mount_kill`'s `.expect(1)` when the
    // server drops.
    assert!(
        matches!(error, Error::McpGateway(ref stderr) if stderr == "bad config\n"),
        "{error:?}"
    );
}

#[tokio::test]
async fn a_failed_gateway_call_kills_the_sandbox_and_a_failed_kill_is_ignored() {
    // Arrange: envd refuses the start, and the kill fails too.
    let server = MockServer::start().await;
    mount_create(&server).await;
    Mock::given(method("POST"))
        .and(path(START_PATH))
        .respond_with(ResponseTemplate::new(503).set_body_string("envd down"))
        .expect(1)
        .mount(&server)
        .await;
    mount_kill(&server, 500).await;

    // Act
    let error = client_for(&server)
        .create_sandbox(&CreateSandboxParams::default().mcp(fetch_config()))
        .await
        .expect_err("start failure");

    // Assert: the start's own error is reported, not the kill's.
    assert!(matches!(error, Error::Rpc { .. }), "{error:?}");
}

#[tokio::test]
async fn a_connected_handle_reads_the_token_file_as_root() {
    // Arrange
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/v2/sandboxes/{SANDBOX_ID}/connect")))
        .respond_with(ResponseTemplate::new(200).set_body_json(sandbox_json("0.5.7")))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/files"))
        .and(query_param("path", "/etc/mcp-gateway/.token"))
        .and(query_param("username", "root"))
        // The trailing newline is trimmed (see the plan's deviations).
        .respond_with(ResponseTemplate::new(200).set_body_string("tok-from-file\n"))
        .expect(1)
        .mount(&server)
        .await;
    let sandbox = client_for(&server)
        .connect_sandbox(SANDBOX_ID, &ConnectOptions::new())
        .await
        .expect("connect");

    // Act: two calls, one read: the token is cached.
    let first = sandbox
        .mcp_token()
        .await
        .expect("token")
        .expose_secret()
        .to_owned();
    let second = sandbox
        .mcp_token()
        .await
        .expect("token again")
        .expose_secret()
        .to_owned();

    // Assert
    assert_eq!(first, "tok-from-file");
    assert_eq!(second, first);
}

#[tokio::test]
async fn a_failed_token_read_is_not_cached() {
    // Arrange: the first read finds no file, the second does.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/v2/sandboxes/{SANDBOX_ID}/connect")))
        .respond_with(ResponseTemplate::new(200).set_body_json(sandbox_json("0.5.7")))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/files"))
        .respond_with(ResponseTemplate::new(404).set_body_string("no such file"))
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/files"))
        .respond_with(ResponseTemplate::new(200).set_body_string("tok"))
        .expect(1)
        .mount(&server)
        .await;
    let sandbox = client_for(&server)
        .connect_sandbox(SANDBOX_ID, &ConnectOptions::new())
        .await
        .expect("connect");

    // Act
    let missing = sandbox.mcp_token().await.map(|_| ()).expect_err("no file");
    let found = sandbox
        .mcp_token()
        .await
        .expect("read again")
        .expose_secret()
        .to_owned();

    // Assert
    assert!(
        matches!(missing, Error::Envd { status: 404, .. }),
        "{missing:?}"
    );
    assert_eq!(found, "tok");
}
