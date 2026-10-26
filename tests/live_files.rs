//! Live tests for the envd filesystem claims (S3). One test per ledger claim.

mod live_support;

use live_support::LiveSandbox;

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0051_read_to_string_returns_text() {
    let sandbox = LiveSandbox::new().await;
    sandbox
        .files()
        .write("/tmp/e2b-client-rs.txt", "hello from rust", None)
        .await
        .expect("write");

    let text = sandbox
        .files()
        .read_to_string("/tmp/e2b-client-rs.txt", None)
        .await
        .expect("read_to_string");

    assert_eq!(text, "hello from rust");
}
