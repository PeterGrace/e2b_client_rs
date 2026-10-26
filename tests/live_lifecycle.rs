//! Live tests for the REST lifecycle claims (S1). One test per ledger claim.

mod live_support;

use std::time::Duration;

use e2b_client::{CommandOptions, SandboxState};
use live_support::{LiveSandbox, params};

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
