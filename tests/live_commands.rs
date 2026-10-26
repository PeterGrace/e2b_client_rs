//! Live tests for the envd process claims (S2). One test per ledger claim.

mod live_support;

use e2b_client::CommandOptions;
use live_support::LiveSandbox;

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0033_run_returns_output_and_exit_code() {
    let sandbox = LiveSandbox::new().await;

    let result = sandbox
        .commands()
        .run("echo out; echo err >&2; exit 3", &CommandOptions::default())
        .await
        .expect("a non-zero exit is not an Err");

    assert_eq!(result.stdout, "out\n");
    assert_eq!(result.stderr, "err\n");
    assert_eq!(result.exit_code, 3);
    assert!(!result.success());
}
