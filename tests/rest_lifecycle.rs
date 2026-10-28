//! Lifecycle create options, the idempotency key, and the pause and resume
//! modes against a mock server.

mod common;

use std::time::Duration;

use common::{SANDBOX_ID, client_for, detail_json, sandbox_json};
use e2b_client::{
    ConnectOptions, CreateSandboxParams, Error, Lifecycle, OnResume, OnTimeout, PauseOptions,
    SnapshotMode, TimeoutAction,
};
use serde_json::{Value, json};
use wiremock::matchers::{body_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Mounts a create endpoint that answers 201 to any body.
async fn create_endpoint(server: &MockServer) {
    Mock::given(method("POST"))
        .and(path("/v2/sandboxes"))
        .respond_with(ResponseTemplate::new(201).set_body_json(sandbox_json("0.5.7")))
        .mount(server)
        .await;
}

/// The JSON body of the only create request `server` received.
async fn create_body(server: &MockServer) -> Value {
    let requests = server.received_requests().await.expect("recording");
    let [request] = requests.as_slice() else {
        panic!("expected exactly one request, got {}", requests.len());
    };
    serde_json::from_slice(&request.body).expect("json body")
}

/// Creates a sandbox from `params` against `server`, expecting success.
async fn create(server: &MockServer, params: &CreateSandboxParams) {
    client_for(server)
        .create_sandbox(params)
        .await
        .expect("create");
}

/// Mounts a detail endpoint answering `detail` once.
async fn detail_once(server: &MockServer, detail: Value) {
    Mock::given(method("GET"))
        .and(path(format!("/sandboxes/{SANDBOX_ID}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(detail))
        .up_to_n_times(1)
        .mount(server)
        .await;
}

#[tokio::test]
async fn pause_with_a_filesystem_snapshot_sends_auto_pause_memory_false() {
    // Arrange
    let server = MockServer::start().await;
    create_endpoint(&server).await;
    let lifecycle = Lifecycle::new()
        .on_timeout(OnTimeout::pause_with(SnapshotMode::Filesystem))
        .auto_resume(false);

    // Act
    create(
        &server,
        &CreateSandboxParams::default().lifecycle(lifecycle),
    )
    .await;

    // Assert
    assert_eq!(
        create_body(&server).await,
        json!({
            "templateID": "base", "metadata": {}, "envVars": {},
            "autoPause": true, "autoPauseMemory": false, "autoResume": {"enabled": false}
        })
    );
}

#[tokio::test]
async fn pause_with_a_full_snapshot_sends_auto_pause_memory_true() {
    // Arrange
    let server = MockServer::start().await;
    create_endpoint(&server).await;
    let lifecycle = Lifecycle::new()
        .on_timeout(OnTimeout::pause_with(SnapshotMode::Full))
        .auto_resume(true);

    // Act
    create(
        &server,
        &CreateSandboxParams::default().lifecycle(lifecycle),
    )
    .await;

    // Assert
    assert_eq!(
        create_body(&server).await,
        json!({
            "templateID": "base", "metadata": {}, "envVars": {},
            "autoPause": true, "autoPauseMemory": true, "autoResume": {"enabled": true}
        })
    );
}

#[tokio::test]
async fn pause_without_a_mode_omits_auto_pause_memory() {
    // Arrange
    let server = MockServer::start().await;
    create_endpoint(&server).await;
    let lifecycle = Lifecycle::new()
        .on_timeout(OnTimeout::pause())
        .auto_resume(true);

    // Act
    create(
        &server,
        &CreateSandboxParams::default().lifecycle(lifecycle),
    )
    .await;

    // Assert
    assert_eq!(
        create_body(&server).await,
        json!({
            "templateID": "base", "metadata": {}, "envVars": {},
            "autoPause": true, "autoResume": {"enabled": true}
        })
    );
}

#[tokio::test]
async fn kill_sends_auto_pause_false_only() {
    // Arrange
    let server = MockServer::start().await;
    create_endpoint(&server).await;
    let params =
        CreateSandboxParams::default().lifecycle(Lifecycle::new().on_timeout(OnTimeout::Kill));

    // Act
    create(&server, &params).await;

    // Assert
    assert_eq!(
        create_body(&server).await,
        json!({"templateID": "base", "metadata": {}, "envVars": {}, "autoPause": false})
    );
}

#[tokio::test]
async fn invalid_lifecycles_are_refused_before_any_request() {
    // Arrange
    let server = MockServer::start().await;
    create_endpoint(&server).await;
    let client = client_for(&server);
    let invalid = [
        // auto_resume needs a pause on timeout; unset means kill.
        Lifecycle::new().auto_resume(true),
        Lifecycle::new()
            .on_timeout(OnTimeout::Kill)
            .auto_resume(true),
        // A filesystem-only snapshot cannot be resumed by traffic.
        Lifecycle::new()
            .on_timeout(OnTimeout::pause_with(SnapshotMode::Filesystem))
            .auto_resume(true),
    ];

    for lifecycle in invalid {
        // Act
        let error = client
            .create_sandbox(&CreateSandboxParams::default().lifecycle(lifecycle))
            .await
            .expect_err("invalid lifecycle");

        // Assert
        assert!(
            matches!(error, Error::InvalidArgument(_)),
            "{lifecycle:?}: {error:?}"
        );
    }
    assert!(
        server
            .received_requests()
            .await
            .expect("recording")
            .is_empty()
    );
}

#[tokio::test]
async fn the_idempotency_key_is_sent_as_a_header() {
    // Arrange: the longest valid key, next to a lifecycle.
    let server = MockServer::start().await;
    let key = format!("retry-{}z", "0a1b".repeat(62));
    assert_eq!(key.len(), 255);
    Mock::given(method("POST"))
        .and(path("/v2/sandboxes"))
        .and(header("idempotency-key", key.as_str()))
        .and(body_json(json!({
            "templateID": "base", "metadata": {}, "envVars": {}, "autoPause": true
        })))
        .respond_with(ResponseTemplate::new(201).set_body_json(sandbox_json("0.5.7")))
        .expect(1)
        .mount(&server)
        .await;

    // Act and Assert: `expect(1)` is checked when the server is dropped.
    create(
        &server,
        &CreateSandboxParams::default()
            .auto_pause(true)
            .idempotency_key(key.as_str()),
    )
    .await;
}

#[tokio::test]
async fn no_idempotency_key_sends_no_header() {
    // Arrange
    let server = MockServer::start().await;
    create_endpoint(&server).await;

    // Act
    create(&server, &CreateSandboxParams::default()).await;

    // Assert
    let requests = server.received_requests().await.expect("recording");
    let [request] = requests.as_slice() else {
        panic!("expected exactly one request, got {}", requests.len());
    };
    assert!(request.headers.get("idempotency-key").is_none());
}

#[tokio::test]
async fn invalid_idempotency_keys_are_refused_before_any_request() {
    // Arrange
    let server = MockServer::start().await;
    create_endpoint(&server).await;
    let client = client_for(&server);

    for key in [
        String::new(),
        "Upper".to_owned(),
        "under_score".to_owned(),
        "a".repeat(256),
    ] {
        // Act
        let error = client
            .create_sandbox(&CreateSandboxParams::default().idempotency_key(key.clone()))
            .await
            .expect_err("invalid key");

        // Assert
        assert!(
            matches!(error, Error::InvalidArgument(_)),
            "{key:?}: {error:?}"
        );
    }
    assert!(
        server
            .received_requests()
            .await
            .expect("recording")
            .is_empty()
    );
}

#[tokio::test]
async fn the_idempotency_key_is_checked_before_the_lifecycle() {
    // Arrange: both are invalid; the SDK resolves the key first.
    let server = MockServer::start().await;
    let params = CreateSandboxParams::default()
        .lifecycle(Lifecycle::new().auto_resume(true))
        .idempotency_key("Upper");

    // Act
    let error = client_for(&server)
        .create_sandbox(&params)
        .await
        .expect_err("invalid");

    // Assert
    assert!(
        matches!(&error, Error::InvalidArgument(message) if message.contains("idempotency")),
        "{error:?}"
    );
}

#[tokio::test]
async fn get_info_reads_lifecycle_and_pause_mode() {
    // Arrange: a known lifecycle and mode, then values the crate does not know.
    let server = MockServer::start().await;
    detail_once(
        &server,
        detail_json(json!({
            "state": "paused",
            "lifecycle": {"onTimeout": "pause", "autoResume": true},
            "pauseMode": "filesystem"
        })),
    )
    .await;
    detail_once(
        &server,
        detail_json(json!({
            "state": "paused",
            "lifecycle": {"onTimeout": "hibernate", "autoResume": false},
            "pauseMode": "someday"
        })),
    )
    .await;
    detail_once(
        &server,
        detail_json(json!({
            "lifecycle": {"onTimeout": "kill", "autoResume": false},
            "pauseMode": "full"
        })),
    )
    .await;
    let client = client_for(&server);

    // Act
    let known = client.get_sandbox_info(SANDBOX_ID).await.expect("known");
    let unknown = client.get_sandbox_info(SANDBOX_ID).await.expect("unknown");
    let kill = client.get_sandbox_info(SANDBOX_ID).await.expect("kill");

    // Assert
    let lifecycle = known.lifecycle.expect("lifecycle");
    assert_eq!(lifecycle.on_timeout, TimeoutAction::Pause);
    assert!(lifecycle.auto_resume);
    assert_eq!(known.pause_mode, Some(SnapshotMode::Filesystem));
    let lifecycle = unknown.lifecycle.expect("lifecycle");
    assert_eq!(lifecycle.on_timeout, TimeoutAction::Unknown);
    assert!(!lifecycle.auto_resume);
    assert_eq!(unknown.pause_mode, None);
    assert_eq!(
        kill.lifecycle.expect("lifecycle").on_timeout,
        TimeoutAction::Kill
    );
    assert_eq!(kill.pause_mode, Some(SnapshotMode::Full));
}

#[tokio::test]
async fn get_info_without_lifecycle_fields_reads_none() {
    // Arrange: `null` for both, then a server that predates the fields.
    let server = MockServer::start().await;
    detail_once(
        &server,
        detail_json(json!({"lifecycle": null, "pauseMode": null})),
    )
    .await;
    detail_once(&server, detail_json(json!({}))).await;
    let client = client_for(&server);

    // Act
    let null = client.get_sandbox_info(SANDBOX_ID).await.expect("null");
    let absent = client.get_sandbox_info(SANDBOX_ID).await.expect("absent");

    // Assert
    for info in [null, absent] {
        assert_eq!(info.lifecycle, None, "{info:?}");
        assert_eq!(info.pause_mode, None, "{info:?}");
    }
}

/// Mounts a mock answering `template` to one `POST` on `route` whose JSON body
/// is exactly `body`, named `label` so a failed expectation says which.
async fn expect_post(
    server: &MockServer,
    route: String,
    body: Value,
    template: ResponseTemplate,
    label: &str,
) {
    Mock::given(method("POST"))
        .and(path(route))
        .and(body_json(body))
        .respond_with(template)
        .expect(1)
        .named(label)
        .mount(server)
        .await;
}

#[tokio::test]
async fn pause_sends_memory_from_the_mode() {
    // Arrange: one mock per body; a request with any other body gets
    // wiremock's 404, which `pause_sandbox` reports as not found.
    let server = MockServer::start().await;
    for (body, label) in [
        (json!({"memory": true}), "full"),
        (json!({"memory": false}), "filesystem"),
        (json!({}), "default"),
    ] {
        expect_post(
            &server,
            format!("/sandboxes/{SANDBOX_ID}/pause"),
            body,
            ResponseTemplate::new(204),
            label,
        )
        .await;
    }
    let client = client_for(&server);
    let full = PauseOptions::new().mode(SnapshotMode::Full);
    let filesystem = PauseOptions::new().mode(SnapshotMode::Filesystem);

    // Act and Assert: each call matches only its own mock.
    assert!(client.pause_sandbox(SANDBOX_ID, &full).await.expect("full"));
    assert!(
        client
            .pause_sandbox(SANDBOX_ID, &filesystem)
            .await
            .expect("filesystem")
    );
    assert!(
        client
            .pause_sandbox(SANDBOX_ID, &PauseOptions::new())
            .await
            .expect("default")
    );
}

/// Hosted E2B's 409 for pausing a sandbox that is already paused (live,
/// 2026-10-28), with the sandbox id it names.
fn already_paused_body(id: &str) -> Value {
    json!({
        "code": 409,
        "message": format!("Error pausing sandbox - sandbox '{id}' is already paused"),
    })
}

#[tokio::test]
async fn a_moded_pause_reads_only_an_already_paused_409_as_false() {
    // Arrange: per sandbox id, the answer the API gives.
    let server = MockServer::start().await;
    let answers = [
        (
            "already",
            ResponseTemplate::new(409).set_body_json(already_paused_body("already")),
        ),
        (
            "refused",
            ResponseTemplate::new(409).set_body_json(json!({
                "code": 409,
                "message": "snapshot_filesystem_only_unsupported_node",
            })),
        ),
        ("empty", ResponseTemplate::new(409)),
        (
            "disabled",
            ResponseTemplate::new(400).set_body_json(
                json!({"code": 400, "message": "snapshot_filesystem_only_disabled"}),
            ),
        ),
        ("missing", ResponseTemplate::new(404)),
    ];
    for (id, template) in answers {
        Mock::given(method("POST"))
            .and(path(format!("/sandboxes/{id}/pause")))
            .respond_with(template)
            .expect(1)
            .mount(&server)
            .await;
    }
    let client = client_for(&server);
    let filesystem = PauseOptions::new().mode(SnapshotMode::Filesystem);

    // Act
    let already = client.pause_sandbox("already", &filesystem).await;
    let refused = client.pause_sandbox("refused", &filesystem).await;
    let empty = client.pause_sandbox("empty", &filesystem).await;
    let disabled = client.pause_sandbox("disabled", &filesystem).await;
    let missing = client.pause_sandbox("missing", &filesystem).await;

    // Assert
    assert!(!already.expect("an already-paused 409 is Ok(false)"));
    assert!(
        matches!(refused, Err(Error::Api { status: 409, ref message, .. })
            if message == "snapshot_filesystem_only_unsupported_node"),
        "{refused:?}"
    );
    assert!(
        matches!(empty, Err(Error::Api { status: 409, .. })),
        "a 409 without the message is a refusal: {empty:?}"
    );
    assert!(
        matches!(disabled, Err(Error::Api { status: 400, .. })),
        "{disabled:?}"
    );
    assert!(
        matches!(missing, Err(Error::SandboxNotFound(ref id)) if id == "missing"),
        "{missing:?}"
    );
}

#[tokio::test]
async fn a_full_pause_also_reads_the_409_body() {
    // Arrange: `Full` is a mode too, so it gets the same check.
    let server = MockServer::start().await;
    for (id, template) in [
        (
            "already",
            ResponseTemplate::new(409).set_body_json(already_paused_body("already")),
        ),
        (
            "refused",
            ResponseTemplate::new(409).set_body_json(json!({"code": 409, "message": "busy"})),
        ),
    ] {
        Mock::given(method("POST"))
            .and(path(format!("/sandboxes/{id}/pause")))
            .and(body_json(json!({"memory": true})))
            .respond_with(template)
            .expect(1)
            .mount(&server)
            .await;
    }
    let client = client_for(&server);
    let full = PauseOptions::new().mode(SnapshotMode::Full);

    // Act
    let already = client.pause_sandbox("already", &full).await;
    let refused = client.pause_sandbox("refused", &full).await;

    // Assert
    assert!(!already.expect("already paused"));
    assert!(
        matches!(refused, Err(Error::Api { status: 409, ref message, .. }) if message == "busy"),
        "{refused:?}"
    );
}

#[tokio::test]
async fn a_pause_without_a_mode_reads_every_409_as_already_paused() {
    // Arrange: SDK parity. Any 409, whatever its message, is `Ok(false)`.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/sandboxes/{SANDBOX_ID}/pause")))
        .and(body_json(json!({})))
        .respond_with(
            ResponseTemplate::new(409).set_body_json(json!({"code": 409, "message": "busy"})),
        )
        .expect(1)
        .mount(&server)
        .await;

    // Act
    let paused = client_for(&server)
        .pause_sandbox(SANDBOX_ID, &PauseOptions::new())
        .await;

    // Assert
    assert!(!paused.expect("SDK parity: 409 is already paused"));
}

#[tokio::test]
async fn sandbox_pause_sends_the_mode() {
    let server = MockServer::start().await;
    let sandbox = common::sandbox(&server).await;
    expect_post(
        &server,
        format!("/sandboxes/{SANDBOX_ID}/pause"),
        json!({"memory": false}),
        ResponseTemplate::new(204),
        "filesystem",
    )
    .await;

    let paused = sandbox
        .pause(&PauseOptions::new().mode(SnapshotMode::Filesystem))
        .await
        .expect("pause");

    assert!(paused);
}

#[tokio::test]
async fn connect_sends_memory_false_only_for_a_reboot() {
    // Arrange
    let server = MockServer::start().await;
    for (body, label) in [
        (json!({"timeout": 60, "memory": false}), "reboot"),
        (json!({"timeout": 90}), "restore with a timeout"),
        (json!({}), "restore"),
    ] {
        expect_post(
            &server,
            format!("/v2/sandboxes/{SANDBOX_ID}/connect"),
            body,
            ResponseTemplate::new(200).set_body_json(sandbox_json("0.5.7")),
            label,
        )
        .await;
    }
    let client = client_for(&server);
    let reboot = ConnectOptions::new()
        .timeout(Duration::from_secs(60))
        .on_resume(OnResume::Reboot);
    let restore_with_timeout = ConnectOptions::new()
        .timeout(Duration::from_secs(90))
        .on_resume(OnResume::Restore);
    let restore = ConnectOptions::new().on_resume(OnResume::Restore);

    // Act and Assert: each call matches only its own mock.
    let sandbox = client
        .connect_sandbox(SANDBOX_ID, &reboot)
        .await
        .expect("reboot");
    assert_eq!(sandbox.sandbox_id(), SANDBOX_ID);
    client
        .connect_sandbox(SANDBOX_ID, &restore_with_timeout)
        .await
        .expect("restore with a timeout");
    client
        .connect_sandbox(SANDBOX_ID, &restore)
        .await
        .expect("restore");
}
