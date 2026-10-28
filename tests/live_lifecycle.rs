//! Live tests for the REST lifecycle claims (S1). One test per ledger claim.

mod live_support;

use std::collections::HashSet;
use std::time::{Duration, Instant};

use e2b_client::{
    ALL_TRAFFIC, CommandOptions, ConnectOptions, EnvdVersion, Error, Lifecycle,
    ListSandboxesParams, ListSnapshotsParams, MetricsParams, NetworkConfig, NetworkRule,
    NetworkUpdate, OnResume, OnTimeout, PauseOptions, Sandbox, SandboxInfo, SandboxState,
    SnapshotMode, SnapshotOptions, SnapshotPage, SortOrder, TimeoutAction,
};
use live_support::{
    LiveSandbox, RUN_METADATA_KEY, client, http_reply, no_http_reply_twice, params, params_from,
    poll_until, resource_name, run_id,
};

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

/// A file on `/dev/shm`, a tmpfs: it lives in memory, so a memory restore
/// keeps it and a cold boot loses it.
const IN_MEMORY_MARKER: &str = "/dev/shm/in-memory";
/// A file in the user's home, on the sandbox's disk: it survives a restore
/// and a cold boot alike. Not `/tmp`: that is on the same disk, but the
/// template's systemd `tmpfiles.d/tmp.conf` (`D /tmp`) empties it at every
/// boot (hosted E2B, 2026-10-28).
const ON_DISK_MARKER: &str = "/home/user/on-disk";

/// Writes both markers in `sandbox`, then `sync`s so the on-disk one is
/// flushed: the API says a cold boot may lose writes not flushed before the
/// pause (`e2b/api/client/models/connect_sandbox_v2.py:18-22`).
async fn write_markers(sandbox: &Sandbox) {
    let result = sandbox
        .commands()
        .run(
            &format!("touch {IN_MEMORY_MARKER} {ON_DISK_MARKER} && sync"),
            &CommandOptions::default(),
        )
        .await
        .expect("markers");
    assert_eq!(result.exit_code, 0, "{result:?}");
}

/// Whether `path` exists in `sandbox`.
async fn exists(sandbox: &Sandbox, path: &str) -> bool {
    // `run` reports a non-zero exit in the result, not as an error.
    sandbox
        .commands()
        .run(&format!("test -e {path}"), &CommandOptions::default())
        .await
        .expect("test -e")
        .exit_code
        == 0
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
    let open = LiveSandbox::new().await;
    let closed = LiveSandbox::create(&params().allow_internet_access(false)).await;

    // Act
    let open_reply = http_reply(&open, "1.1.1.1").await;
    let closed_reply = http_reply(&closed, "1.1.1.1").await;

    // Assert
    assert!(
        open_reply,
        "control sandbox got no HTTP reply from 1.1.1.1:80"
    );
    assert!(
        !closed_reply,
        "sandbox without internet access got an HTTP reply from 1.1.1.1:80"
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

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0029_get_metrics_reports_cpu_memory_and_disk() {
    // Arrange
    let sandbox = LiveSandbox::new().await;
    let started_at = sandbox.get_info().await.expect("info").started_at;

    // Act: the API samples every few seconds, so wait for the first sample.
    // Not found means the sandbox is gone, so fail at once; any other error
    // may be transient and keeps polling.
    let sandbox_ref = &sandbox;
    let metrics = poll_until(
        "a first metrics sample",
        Duration::from_secs(90),
        || async move {
            match sandbox_ref.get_metrics(&MetricsParams::default()).await {
                Ok(metrics) => (!metrics.is_empty()).then_some(metrics),
                Err(error) if error.is_not_found() => panic!("sandbox vanished: {error:?}"),
                Err(_) => None,
            }
        },
    )
    .await;

    // Assert
    let sample = metrics.last().expect("a sample");
    assert!(sample.cpu_count >= 1, "{sample:?}");
    assert!(
        sample.mem_total > 0 && sample.mem_used <= sample.mem_total,
        "{sample:?}"
    );
    assert!(
        sample.disk_total > 0 && sample.disk_used <= sample.disk_total,
        "{sample:?}"
    );
    assert!(
        sample.timestamp >= started_at - chrono::Duration::seconds(5),
        "{sample:?}"
    );
}

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0151_list_filters_by_template_and_start_time_and_orders() {
    // Arrange: two sandboxes started at least 2 s apart.
    let first = LiveSandbox::new().await;
    tokio::time::sleep(Duration::from_secs(2)).await;
    let second = LiveSandbox::new().await;
    let second_info = second.get_info().await.expect("second info");
    let first_info = first.get_info().await.expect("first info");
    let ours = ListSandboxesParams::new()
        .metadata(RUN_METADATA_KEY, run_id())
        .template(first_info.template_id.clone());

    // Act
    let newest_first = ids_of(&list_all(&ours.clone().order(SortOrder::Descending)).await);
    let oldest_first = ids_of(&list_all(&ours.clone().order(SortOrder::Ascending)).await);
    let since_second = ids_of(&list_all(&ours.clone().started_after(second_info.started_at)).await);

    // Assert: other tests of this run may also be listed; compare our two.
    let position = |ids: &[String], id: &str| ids.iter().position(|each| each == id);
    let (first_id, second_id) = (first.sandbox_id(), second.sandbox_id());
    for (ids, label) in [(&newest_first, "desc"), (&oldest_first, "asc")] {
        assert!(
            position(ids, first_id).is_some() && position(ids, second_id).is_some(),
            "{label} list is missing one of ours: {ids:?}"
        );
    }
    // `Option<usize>` compares with `None` first, which is why the loop above
    // checks that both ids are present before these comparisons.
    assert!(
        position(&newest_first, second_id) < position(&newest_first, first_id),
        "{newest_first:?}"
    );
    assert!(
        position(&oldest_first, first_id) < position(&oldest_first, second_id),
        "{oldest_first:?}"
    );
    assert!(
        since_second.iter().any(|id| id == second_id),
        "{since_second:?}"
    );
    assert!(
        !since_second.iter().any(|id| id == first_id),
        "{since_second:?}"
    );
}

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0143_create_network_allow_and_deny_lists_are_enforced() {
    // Arrange: deny everything except 1.1.1.1.
    let network = NetworkConfig::new()
        .deny_out([ALL_TRAFFIC])
        .allow_out(["1.1.1.1"]);
    let sandbox = LiveSandbox::create(&params().network(network)).await;

    // Act
    let allowed = http_reply(&sandbox, "1.1.1.1").await;
    let denied = http_reply(&sandbox, "1.0.0.1").await;

    // Assert
    assert!(allowed, "1.1.1.1 is in allow_out but gave no HTTP reply");
    assert!(!denied, "1.0.0.1 is denied by ALL_TRAFFIC but replied");
}

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0153_get_info_reports_network() {
    // Arrange: rules are keyed by DNS name; hosted E2B echoes them back.
    let network = NetworkConfig::new()
        .allow_out(["1.1.1.1"])
        .dns_servers(["1.1.1.1"])
        .rule("one.one.one.one", NetworkRule::new().header("X-Probe", "v"));
    let sandbox =
        LiveSandbox::create(&params().allow_internet_access(false).network(network)).await;

    // Act
    let info = sandbox.get_info().await.expect("info");

    // Assert
    assert_eq!(info.allow_internet_access, Some(false), "{info:?}");
    let network = info.network.expect("network in the detail");
    assert!(
        network.allow_out.iter().any(|target| target == "1.1.1.1"),
        "{network:?}"
    );
    assert_eq!(
        network.dns_servers.as_deref(),
        Some(&["1.1.1.1".to_owned()][..]),
        "{network:?}"
    );
    let rules = network
        .rules
        .get("one.one.one.one")
        .expect("the rule in the detail");
    assert_eq!(rules.len(), 1, "{network:?}");
    assert_eq!(
        rules[0].headers().get("X-Probe").map(String::as_str),
        Some("v"),
        "{network:?}"
    );
}

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0144_update_network_replaces_egress_without_restart() {
    // Arrange: all egress denied, and a file in memory (tmpfs) that a
    // restart would not keep.
    let network = NetworkConfig::new().deny_out([ALL_TRAFFIC]);
    let sandbox = LiveSandbox::create(&params().network(network)).await;
    sandbox
        .commands()
        .run("echo kept > /dev/shm/marker", &CommandOptions::default())
        .await
        .expect("marker");
    assert!(
        no_http_reply_twice(&sandbox, "1.1.1.1").await,
        "egress was open before any update"
    );

    // Act 1: an update with only allow_internet_access(true) clears deny_out.
    sandbox
        .update_network(&NetworkUpdate::new().allow_internet_access(true))
        .await
        .expect("open egress");
    // Rust note: `async move` closures move what they capture, so they take
    // a copy of this reference rather than the sandbox itself.
    let sandbox_ref = &sandbox;
    poll_until("egress to open", Duration::from_secs(30), || async move {
        http_reply(sandbox_ref, "1.1.1.1").await.then_some(())
    })
    .await;

    // Act 2: deny everything again.
    sandbox
        .update_network(&NetworkUpdate::new().deny_out([ALL_TRAFFIC]))
        .await
        .expect("close egress");
    poll_until("egress to close", Duration::from_secs(30), || async move {
        no_http_reply_twice(sandbox_ref, "1.1.1.1")
            .await
            .then_some(())
    })
    .await;

    // Assert: no restart, so the in-memory file is still there.
    let marker = sandbox
        .commands()
        .run("cat /dev/shm/marker", &CommandOptions::default())
        .await
        .expect("read marker");
    assert_eq!(marker.stdout, "kept\n");
}

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0145_lifecycle_pause_with_full_snapshot_pauses_at_timeout() {
    // Arrange
    let lifecycle = Lifecycle::new()
        .on_timeout(OnTimeout::pause_with(SnapshotMode::Full))
        .auto_resume(false);
    let sandbox = LiveSandbox::create(
        &params()
            .timeout(Duration::from_secs(15))
            .lifecycle(lifecycle),
    )
    .await;

    // Act: as clm_0011, but with the lifecycle form. Hosted E2B leaves
    // `pauseMode` out while the sandbox runs and adds it once the pause has
    // been taken, which can be a poll after `state` turns `paused` (live,
    // 2026-10-27), so wait for both. Not found means the sandbox was killed,
    // which this claim rules out; other errors may be transient and keep
    // polling.
    let sandbox_ref = &sandbox;
    let info = poll_until(
        "lifecycle pause at timeout, with its pause mode",
        Duration::from_secs(120),
        || async move {
            match sandbox_ref.get_info().await {
                Ok(info) => (info.state == SandboxState::Paused && info.pause_mode.is_some())
                    .then_some(info),
                Err(error) if error.is_not_found() => {
                    panic!("sandbox was killed, not paused, at its timeout: {error:?}")
                }
                Err(_) => None,
            }
        },
    )
    .await;

    // Assert
    let reported = info.lifecycle.expect("lifecycle in the detail");
    assert_eq!(reported.on_timeout, TimeoutAction::Pause, "{info:?}");
    assert!(!reported.auto_resume, "{info:?}");
    assert_eq!(info.pause_mode, Some(SnapshotMode::Full), "{info:?}");
}

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0153_get_info_reports_lifecycle() {
    // Arrange
    let lifecycle = Lifecycle::new()
        .on_timeout(OnTimeout::Kill)
        .auto_resume(false);
    let sandbox = LiveSandbox::create(&params().lifecycle(lifecycle)).await;

    // Act
    let info = sandbox.get_info().await.expect("info");

    // Assert
    let reported = info.lifecycle.expect("lifecycle in the detail");
    assert_eq!(reported.on_timeout, TimeoutAction::Kill, "{info:?}");
    assert!(!reported.auto_resume, "{info:?}");
}

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0146_resume_restores_memory_or_reboots() {
    // Arrange
    let sandbox = LiveSandbox::create(&params().timeout(Duration::from_secs(240))).await;
    let client = sandbox.client();
    write_markers(&sandbox).await;
    let full = PauseOptions::new().mode(SnapshotMode::Full);
    let resume = |on_resume| {
        ConnectOptions::new()
            .timeout(Duration::from_secs(120))
            .on_resume(on_resume)
    };

    // Act 1: a full pause, then a restore.
    assert!(sandbox.pause_with(&full).await.expect("first pause"));
    let restored = client
        .connect_sandbox_with(sandbox.sandbox_id(), &resume(OnResume::Restore))
        .await
        .expect("restore");
    let kept_after_restore = exists(&restored, IN_MEMORY_MARKER).await;

    // Act 2: another full pause, then a reboot.
    assert!(restored.pause_with(&full).await.expect("second pause"));
    let rebooted = client
        .connect_sandbox_with(sandbox.sandbox_id(), &resume(OnResume::Reboot))
        .await
        .expect("reboot");

    // Assert
    assert!(kept_after_restore, "a restore lost the in-memory file");
    assert!(
        !exists(&rebooted, IN_MEMORY_MARKER).await,
        "a reboot kept the in-memory file"
    );
    assert!(
        exists(&rebooted, ON_DISK_MARKER).await,
        "a reboot lost the on-disk file"
    );
}

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0146_filesystem_pause_cold_boots() {
    // Arrange
    let sandbox = LiveSandbox::create(&params().timeout(Duration::from_secs(180))).await;
    write_markers(&sandbox).await;

    // Act: hosted E2B may refuse a filesystem-only pause for the team (400,
    // an error here) or the node (409, which reads as "already paused", so
    // `false` on this running sandbox); that is an environment limit, to be
    // recorded as such, not worked around.
    let paused = sandbox
        .pause_with(&PauseOptions::new().mode(SnapshotMode::Filesystem))
        .await
        .expect("filesystem pause (HTTP 400 means the team lacks it)");
    let resumed = sandbox
        .client()
        .connect_sandbox(sandbox.sandbox_id(), Some(Duration::from_secs(120)))
        .await
        .expect("resume");

    // Assert
    assert!(
        paused,
        "a running sandbox's pause answered 409: the node refused a filesystem-only pause"
    );
    assert!(
        !exists(&resumed, IN_MEMORY_MARKER).await,
        "a filesystem pause kept memory"
    );
    assert!(
        exists(&resumed, ON_DISK_MARKER).await,
        "a filesystem pause lost the disk"
    );
}

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0030_snapshot_starts_sandboxes_with_its_files() {
    // Arrange: a full snapshot (the default) restores memory, so `/tmp`,
    // which the `base` template empties at every boot, keeps its file.
    let source = LiveSandbox::new().await;
    source
        .files()
        .write("/tmp/snapshot.txt", "from the snapshot", None)
        .await
        .expect("write");
    let options = SnapshotOptions::new().name(resource_name("starts"));
    let snapshot = source.create_snapshot(&options).await.expect("snapshot");

    // Act
    let copy = LiveSandbox::create(&params_from(&snapshot.snapshot_id)).await;
    let text = copy
        .files()
        .read_to_string("/tmp/snapshot.txt", None)
        .await
        .expect("read in the copy");

    // Assert
    assert_eq!(text, "from the snapshot");
    assert_eq!(
        source.get_info().await.expect("source info").state,
        SandboxState::Running
    );
    // Hosted E2B refuses to delete a snapshot while a sandbox made from it
    // runs (400 "cannot delete template ... because there are running
    // sandboxes using it"), so the copy goes first.
    assert!(copy.kill().await.expect("kill the copy"));
    assert!(
        source
            .client()
            .delete_snapshot(&snapshot.snapshot_id)
            .await
            .expect("delete")
    );
}

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0147_list_and_delete_snapshots() {
    // Arrange
    let source = LiveSandbox::new().await;
    let name = resource_name("listed");
    let snapshot = source
        .create_snapshot(&SnapshotOptions::new().name(name.clone()))
        .await
        .expect("snapshot");
    let client = source.client();
    let listed = |page: &SnapshotPage| {
        page.snapshots
            .iter()
            .any(|each| each.snapshot_id == snapshot.snapshot_id)
    };

    // Act
    let by_sandbox = source
        .list_snapshots(&ListSnapshotsParams::new())
        .await
        .expect("by sandbox");
    let by_name = client
        .list_snapshots(&ListSnapshotsParams::new().name(name))
        .await
        .expect("by name");
    let deleted = client
        .delete_snapshot(&snapshot.snapshot_id)
        .await
        .expect("delete");
    let deleted_again = client
        .delete_snapshot(&snapshot.snapshot_id)
        .await
        .expect("delete again");

    // Assert
    assert!(
        listed(&by_sandbox),
        "not listed for its source sandbox: {by_sandbox:?}"
    );
    assert!(listed(&by_name), "not listed by its name: {by_name:?}");
    assert!(deleted);
    assert!(!deleted_again);
}
