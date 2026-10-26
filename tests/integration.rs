//! End-to-end tests against a real E2B (or E2B-compatible) deployment.
//!
//! Ignored by default. Run them with credentials in the environment:
//!
//! ```text
//! E2B_API_KEY=... [E2B_API_URL=...] [E2B_DOMAIN=...] [E2B_TEMPLATE=base] \
//!     cargo test --test integration -- --ignored --test-threads=1
//! ```

use std::time::Duration;

use e2b_client::{Client, CommandOptions, CreateSandboxParams, Sandbox, SandboxState};

fn template() -> String {
    std::env::var("E2B_TEMPLATE")
        .ok()
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| "base".into())
}

async fn create() -> (Client, Sandbox) {
    let client = Client::from_env().expect("E2B_API_KEY must be set for integration tests");
    let params = CreateSandboxParams::new(template())
        .timeout(Duration::from_secs(300))
        .metadata("suite", "e2b-client-rs-integration");
    let sandbox = client
        .create_sandbox(&params)
        .await
        .expect("create sandbox");
    (client, sandbox)
}

#[tokio::test]
#[ignore = "needs E2B_API_KEY and a reachable E2B deployment"]
async fn command_output_and_exit_code() {
    let (_client, sandbox) = create().await;

    let result = sandbox
        .commands()
        .run("echo out; echo err >&2; exit 3", &CommandOptions::default())
        .await;
    sandbox.kill().await.expect("kill");

    let result = result.expect("run");
    assert_eq!(result.stdout, "out\n");
    assert_eq!(result.stderr, "err\n");
    assert_eq!(result.exit_code, 3);
}

#[tokio::test]
#[ignore = "needs E2B_API_KEY and a reachable E2B deployment"]
async fn write_then_read_file() {
    let (_client, sandbox) = create().await;
    let files = sandbox.files();

    let outcome = async {
        let written = files
            .write("/tmp/e2b-client-rs.txt", "hello from rust", None)
            .await?;
        let text = files.read_to_string("/tmp/e2b-client-rs.txt", None).await?;
        let exists = files.exists("/tmp/e2b-client-rs.txt", None).await?;
        Ok::<_, e2b_client::Error>((written, text, exists))
    }
    .await;
    sandbox.kill().await.expect("kill");

    let (written, text, exists) = outcome.expect("file round trip");
    assert_eq!(written.path, "/tmp/e2b-client-rs.txt");
    assert_eq!(text, "hello from rust");
    assert!(exists);
}

#[tokio::test]
#[ignore = "needs E2B_API_KEY and a reachable E2B deployment"]
async fn pause_resume_and_kill() {
    let (client, sandbox) = create().await;
    let id = sandbox.sandbox_id().to_owned();
    sandbox
        .files()
        .write("/tmp/state.txt", "persisted", None)
        .await
        .expect("write");

    assert!(sandbox.pause().await.expect("pause"));
    let info = client
        .get_sandbox_info(&id)
        .await
        .expect("info after pause");
    assert_eq!(info.state, SandboxState::Paused);

    let resumed = client
        .connect_sandbox(&id, Some(Duration::from_secs(300)))
        .await
        .expect("resume");
    let text = resumed
        .files()
        .read_to_string("/tmp/state.txt", None)
        .await
        .expect("read");
    assert_eq!(text, "persisted");
    let result = resumed
        .commands()
        .run("echo resumed", &CommandOptions::default())
        .await;

    assert!(resumed.kill().await.expect("kill"));
    assert_eq!(result.expect("run after resume").stdout, "resumed\n");
}
