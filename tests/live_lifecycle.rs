//! Live tests for the REST lifecycle claims (S1). One test per ledger claim.

mod live_support;

use std::collections::HashSet;
use std::time::{Duration, Instant};

use e2b_client::{
    CommandOptions, EnvdVersion, Error, ListSandboxesParams, SandboxInfo, SandboxState,
};
use live_support::{LiveSandbox, RUN_METADATA_KEY, client, params, poll_until, run_id};

/// A sandbox id that cannot exist.
const MISSING_ID: &str = "i0000000000000000000z";

/// Every sandbox listed for `params`, following every page.
async fn list_all(params: &ListSandboxesParams) -> Vec<SandboxInfo> {
    let client = client();
    let mut sandboxes = Vec::new();
    let mut page_params = params.clone();
    loop {
        let page = client.list_sandboxes(&page_params).await.expect("list");
        sandboxes.extend(page.sandboxes);
        match page.next_token {
            Some(token) => page_params = params.clone().next_token(token),
            None => return sandboxes,
        }
    }
}

/// The ids of `sandboxes`, in listing order.
fn ids_of(sandboxes: &[SandboxInfo]) -> Vec<String> {
    sandboxes
        .iter()
        .map(|info| info.sandbox_id.clone())
        .collect()
}

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0007_create_returns_running_sandbox() {
    // Act
    let sandbox = LiveSandbox::new().await;

    // Assert
    assert!(!sandbox.sandbox_id().is_empty());
    assert!(!sandbox.template_id().is_empty());
    let info = sandbox.get_info().await.expect("info");
    assert_eq!(info.state, SandboxState::Running);
    assert_eq!(info.sandbox_id, sandbox.sandbox_id());
    // `parse_lenient` maps an unparseable version to 99.99.99, so compare the
    // parsed handle version with the raw string the API reports.
    assert!(
        !info.envd_version.is_empty(),
        "API reported no envd version"
    );
    assert_eq!(
        sandbox.envd_version(),
        EnvdVersion::parse_lenient(&info.envd_version)
    );
}

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0008_create_metadata_is_visible_in_info() {
    // Arrange
    let sandbox = LiveSandbox::create(&params().metadata("purpose", "clm-0008")).await;

    // Act
    let info = sandbox.get_info().await.expect("info");

    // Assert
    assert_eq!(
        info.metadata.get("purpose").map(String::as_str),
        Some("clm-0008")
    );
    assert_eq!(
        info.metadata.get(RUN_METADATA_KEY).map(String::as_str),
        Some(run_id())
    );
}

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0009_create_env_vars_reach_commands() {
    // Arrange
    let sandbox = LiveSandbox::create(&params().env("GREETING", "hi from create")).await;

    // Act
    let result = sandbox
        .commands()
        .run("echo \"$GREETING\"", &CommandOptions::default())
        .await
        .expect("run");

    // Assert
    assert_eq!(result.stdout, "hi from create\n");
}

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0010_create_timeout_sets_end_at() {
    // Arrange
    let sandbox = LiveSandbox::create(&params().timeout(Duration::from_secs(90))).await;

    // Act
    let info = sandbox.get_info().await.expect("info");

    // Assert
    let lifetime = info.end_at.timestamp() - info.started_at.timestamp();
    assert!(
        (80..=100).contains(&lifetime),
        "end_at - started_at = {lifetime}s, want about 90"
    );
}

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0011_auto_pause_pauses_at_timeout() {
    // Arrange
    let sandbox =
        LiveSandbox::create(&params().timeout(Duration::from_secs(15)).auto_pause(true)).await;

    // Act: wait well past the timeout. `async move` copies the reference into
    // each future, so the future does not borrow from the closure (which Rust
    // forbids for `FnMut` closures).
    let sandbox_ref = &sandbox;
    let state = poll_until(
        "auto-pause at timeout",
        Duration::from_secs(120),
        || async move {
            // Not found is the outcome this claim rules out, so fail at once.
            // Any other error may be transient and keeps polling.
            match sandbox_ref.get_info().await {
                Ok(info) => (info.state == SandboxState::Paused).then_some(info.state),
                Err(error) if error.is_not_found() => {
                    panic!("sandbox was killed, not paused, at its timeout: {error:?}")
                }
                Err(_) => None,
            }
        },
    )
    .await;

    // Assert: paused, not killed (a killed sandbox would be not found).
    assert_eq!(state, SandboxState::Paused);
}

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0012_internet_access_false_blocks_outbound() {
    // Arrange: a control sandbox proves the probe works when access is allowed.
    // The probe must see an HTTP reply: hosted E2B completes the TCP handshake
    // on the host even when egress is denied, so a bare connect proves nothing.
    let open = LiveSandbox::new().await;
    let closed = LiveSandbox::create(&params().allow_internet_access(false)).await;
    let probe = r#"timeout 10 bash -c 'exec 3<>/dev/tcp/1.1.1.1/80
printf "GET / HTTP/1.0\r\nHost: one.one.one.one\r\n\r\n" >&3
read -r status <&3
[[ $status == HTTP/* ]]'"#;

    // Act
    let open_result = open
        .commands()
        .run(probe, &CommandOptions::default())
        .await
        .expect("open probe");
    let closed_result = closed
        .commands()
        .run(probe, &CommandOptions::default())
        .await
        .expect("closed probe");

    // Assert
    assert_eq!(
        open_result.exit_code, 0,
        "control sandbox got no HTTP reply from 1.1.1.1:80: {open_result:?}"
    );
    assert_ne!(
        closed_result.exit_code, 0,
        "sandbox without internet access got an HTTP reply from 1.1.1.1:80: {closed_result:?}"
    );
}

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0013_connect_running_sandbox_runs_commands() {
    // Arrange
    let sandbox = LiveSandbox::new().await;

    // Act
    let connected = sandbox
        .client()
        .connect_sandbox(sandbox.sandbox_id(), None)
        .await
        .expect("connect");

    // Assert
    let result = connected
        .commands()
        .run("echo connected", &CommandOptions::default())
        .await
        .expect("run");
    assert_eq!(result.stdout, "connected\n");
}

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0014_connect_resumes_paused_sandbox_with_files() {
    // Arrange
    let sandbox = LiveSandbox::create(&params().timeout(Duration::from_secs(180))).await;
    sandbox
        .files()
        .write("/tmp/state.txt", "persisted", None)
        .await
        .expect("write");
    assert!(sandbox.pause().await.expect("pause"));
    // Without this, a server whose pause is a no-op passes the test vacuously.
    assert_eq!(
        sandbox.get_info().await.expect("info after pause").state,
        SandboxState::Paused
    );

    // Act
    let resumed = sandbox
        .client()
        .connect_sandbox(sandbox.sandbox_id(), Some(Duration::from_secs(120)))
        .await
        .expect("resume");

    // Assert
    let text = resumed
        .files()
        .read_to_string("/tmp/state.txt", None)
        .await
        .expect("read after resume");
    assert_eq!(text, "persisted");
    let result = resumed
        .commands()
        .run("echo resumed", &CommandOptions::default())
        .await
        .expect("run after resume");
    assert_eq!(result.stdout, "resumed\n");
    let info = sandbox
        .client()
        .get_sandbox_info(sandbox.sandbox_id())
        .await
        .expect("info after resume");
    assert_eq!(info.state, SandboxState::Running);
}

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0015_connect_unknown_sandbox_is_not_found() {
    // Act
    let error = client()
        .connect_sandbox(MISSING_ID, None)
        .await
        .expect_err("unknown sandbox must not connect");

    // Assert
    assert!(
        matches!(error, Error::SandboxNotFound(_)),
        "want SandboxNotFound, got {error:?}"
    );
}

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0016_get_info_returns_detail() {
    // Arrange
    let sandbox = LiveSandbox::new().await;

    // Act
    let info = sandbox
        .client()
        .get_sandbox_info(sandbox.sandbox_id())
        .await
        .expect("info");

    // Assert
    assert_eq!(info.sandbox_id, sandbox.sandbox_id());
    assert_eq!(info.state, SandboxState::Running);
    assert_eq!(info.template_id, sandbox.template_id());
    assert!(info.end_at > info.started_at);
    assert_eq!(
        info.metadata.get(RUN_METADATA_KEY).map(String::as_str),
        Some(run_id())
    );
}

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0017_get_info_unknown_sandbox_is_not_found() {
    // Act
    let error = client()
        .get_sandbox_info(MISSING_ID)
        .await
        .expect_err("unknown sandbox");

    // Assert
    assert!(
        matches!(error, Error::SandboxNotFound(_)),
        "want SandboxNotFound, got {error:?}"
    );
}

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0018_list_filters_by_metadata() {
    // Arrange: one sandbox carries a tag unique to this test, one does not.
    let tag = format!("clm-0018-{}", run_id());
    let tagged = LiveSandbox::create(&params().metadata("list-test", &tag)).await;
    let _untagged = LiveSandbox::new().await;
    let filter = ListSandboxesParams::new().metadata("list-test", &tag);
    let filter = &filter;

    // Act: poll, because list results may lag behind create.
    let listed = poll_until(
        "tagged sandbox to be listed",
        Duration::from_secs(30),
        || async move {
            let listed = list_all(filter).await;
            (!listed.is_empty()).then_some(listed)
        },
    )
    .await;

    // Assert
    assert_eq!(ids_of(&listed), vec![tagged.sandbox_id().to_owned()]);
}

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0019_list_filters_by_state() {
    // Arrange: one running and one paused sandbox, both carrying a test tag.
    let tag = format!("clm-0019-{}", run_id());
    let running = LiveSandbox::create(&params().metadata("list-test", &tag)).await;
    let paused = LiveSandbox::create(
        &params()
            .metadata("list-test", &tag)
            .timeout(Duration::from_secs(120)),
    )
    .await;
    assert!(paused.pause().await.expect("pause"));
    let base = ListSandboxesParams::new().metadata("list-test", &tag);
    let base = &base;

    // Act: poll, because list results may lag behind create and pause.
    let paused_listed = poll_until(
        "paused sandbox to be listed",
        Duration::from_secs(30),
        || async move {
            let listed = list_all(&base.clone().state(SandboxState::Paused)).await;
            (!listed.is_empty()).then_some(listed)
        },
    )
    .await;
    let running_listed = list_all(&base.clone().state(SandboxState::Running)).await;

    // Assert
    assert_eq!(ids_of(&paused_listed), vec![paused.sandbox_id().to_owned()]);
    assert!(
        paused_listed
            .iter()
            .all(|info| info.state == SandboxState::Paused)
    );
    assert_eq!(
        ids_of(&running_listed),
        vec![running.sandbox_id().to_owned()]
    );
    assert!(
        running_listed
            .iter()
            .all(|info| info.state == SandboxState::Running)
    );
}

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0020_list_pages_without_duplicates_or_omissions() {
    // Arrange: three tagged sandboxes, listed one per page.
    let tag = format!("clm-0020-{}", run_id());
    let mut sandboxes = Vec::with_capacity(3);
    for _ in 0..3 {
        sandboxes.push(LiveSandbox::create(&params().metadata("list-test", &tag)).await);
    }
    let created: HashSet<String> = sandboxes
        .iter()
        .map(|sandbox| sandbox.sandbox_id().to_owned())
        .collect();
    let filter = ListSandboxesParams::new()
        .metadata("list-test", &tag)
        .limit(1);
    let filter = &filter;

    // Act
    let listed = poll_until(
        "all three sandboxes to be listed",
        Duration::from_secs(30),
        || async move {
            let ids = ids_of(&list_all(filter).await);
            (ids.len() >= 3).then_some(ids)
        },
    )
    .await;

    // Assert
    let unique: HashSet<String> = listed.iter().cloned().collect();
    assert_eq!(
        listed.len(),
        unique.len(),
        "duplicate ids across pages: {listed:?}"
    );
    assert_eq!(unique, created);
}

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0021_kill_running_sandbox() {
    // Arrange
    let sandbox = LiveSandbox::new().await;

    // Act
    let killed = sandbox
        .client()
        .kill_sandbox(sandbox.sandbox_id())
        .await
        .expect("kill");

    // Assert
    assert!(killed);
    // The API may briefly still return the sandbox, so poll until `get_info`
    // fails, then require that failure to be not found.
    let sandbox_ref = &sandbox;
    let error = poll_until(
        "get_info to fail after kill",
        Duration::from_secs(30),
        || async move { sandbox_ref.get_info().await.err() },
    )
    .await;
    assert!(error.is_not_found(), "want not found, got {error:?}");
}

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0022_kill_unknown_sandbox_is_false() {
    // Act
    let killed = client()
        .kill_sandbox(MISSING_ID)
        .await
        .expect("kill unknown");

    // Assert
    assert!(!killed);
}

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0023_pause_running_sandbox() {
    // Arrange
    let sandbox = LiveSandbox::create(&params().timeout(Duration::from_secs(120))).await;

    // Act
    let paused = sandbox
        .client()
        .pause_sandbox(sandbox.sandbox_id())
        .await
        .expect("pause");

    // Assert
    assert!(paused);
    assert_eq!(
        sandbox.get_info().await.expect("info").state,
        SandboxState::Paused
    );
}

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0024_pause_paused_sandbox_is_false() {
    // Arrange
    let sandbox = LiveSandbox::create(&params().timeout(Duration::from_secs(120))).await;
    assert!(sandbox.pause().await.expect("first pause"));

    // Act
    let second = sandbox
        .client()
        .pause_sandbox(sandbox.sandbox_id())
        .await
        .expect("second pause");

    // Assert
    assert!(!second);
}

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0025_set_timeout_moves_end_at() {
    // Arrange: `Instant` is a monotonic clock, so the elapsed time below does
    // not depend on the local wall clock agreeing with the server's.
    let before_create = Instant::now();
    let sandbox = LiveSandbox::new().await;

    // Act
    sandbox
        .client()
        .set_sandbox_timeout(sandbox.sandbox_id(), Duration::from_secs(240))
        .await
        .expect("set timeout");
    let elapsed = i64::try_from(before_create.elapsed().as_secs()).expect("elapsed fits in i64");

    // Assert: the server set end_at to its own "now" plus 240 s, and that "now"
    // lies between started_at and started_at + elapsed. Measuring from the
    // server's started_at keeps both ends of the comparison on the server's
    // clock. The 2 s slack absorbs whole-second rounding.
    let info = sandbox.get_info().await.expect("info");
    let lifetime = info.end_at.timestamp() - info.started_at.timestamp();
    let window = (240 - 2)..=(240 + elapsed + 2);
    assert!(
        window.contains(&lifetime),
        "end_at - started_at = {lifetime}s, want {window:?} (240 s from the set_timeout call)"
    );
}

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0026_refresh_extends_end_at() {
    // Arrange
    let sandbox = LiveSandbox::new().await;
    let before = sandbox.get_info().await.expect("info before").end_at;

    // Act
    sandbox
        .client()
        .refresh_sandbox(sandbox.sandbox_id(), Some(Duration::from_secs(300)))
        .await
        .expect("refresh");

    // Assert
    let after = sandbox.get_info().await.expect("info after").end_at;
    assert!(
        after > before,
        "end_at did not move: before {before}, after {after}"
    );
}

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0027_is_running_tracks_kill() {
    // Arrange
    let sandbox = LiveSandbox::new().await;
    assert!(sandbox.is_running().await.expect("is_running while alive"));

    // Act
    assert!(sandbox.kill().await.expect("kill"));

    // Assert: envd may take a moment to go away, so poll for `Ok(false)` and
    // report the last answer (an `Err` included) if it never comes.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    let mut last = sandbox.is_running().await;
    while !matches!(last, Ok(false)) && tokio::time::Instant::now() < deadline {
        tokio::time::sleep(Duration::from_secs(2)).await;
        last = sandbox.is_running().await;
    }
    assert!(
        matches!(last, Ok(false)),
        "want Ok(false) after kill, got {last:?}"
    );
}

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0028_get_host_reaches_a_port_in_the_sandbox() {
    // Arrange: a web server on port 8000 serving one known file. The handle
    // is kept so the server stays attached for the whole test.
    let sandbox = LiveSandbox::new().await;
    sandbox
        .files()
        .write("/tmp/www/hello.txt", "hello from 8000", None)
        .await
        .expect("write");
    let _server = sandbox
        .commands()
        .start(
            "cd /tmp/www && python3 -m http.server 8000",
            &CommandOptions::default(),
        )
        .await
        .expect("start server");
    let url = format!("https://{}/hello.txt", sandbox.get_host(8000));
    let http = reqwest::Client::new();

    // Act: `async move` copies the two references into each future.
    let (http, url) = (&http, url.as_str());
    let body = poll_until(
        "the server on port 8000",
        Duration::from_secs(60),
        || async move {
            let response = http.get(url).send().await.ok()?;
            if !response.status().is_success() {
                return None;
            }
            response.text().await.ok()
        },
    )
    .await;

    // Assert
    assert_eq!(body, "hello from 8000");
}
