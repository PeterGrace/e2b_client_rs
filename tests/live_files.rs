//! Live tests for the envd filesystem claims (S3). One test per ledger claim.

mod live_support;

use std::collections::HashSet;

use e2b_client::FileType;
use live_support::LiveSandbox;

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0050_write_read_round_trips_binary() {
    let sandbox = LiveSandbox::new().await;
    // Every byte value, repeated to 64 KiB, so invalid UTF-8 and NULs are covered.
    let data: Vec<u8> = (0..=255u8).cycle().take(64 * 1024).collect();

    sandbox
        .files()
        .write("/tmp/binary.bin", data.clone(), None)
        .await
        .expect("write");
    let read = sandbox
        .files()
        .read("/tmp/binary.bin", None)
        .await
        .expect("read");

    assert!(
        read.as_ref() == data.as_slice(),
        "binary round trip changed the bytes"
    );
}

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

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0052_write_returns_write_info() {
    let sandbox = LiveSandbox::new().await;

    let written = sandbox
        .files()
        .write("/tmp/info.txt", "x", None)
        .await
        .expect("write");

    assert_eq!(written.path, "/tmp/info.txt");
    assert_eq!(written.name, "info.txt");
    assert_eq!(written.file_type, FileType::File);
}

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0053_read_missing_file_is_not_found() {
    let sandbox = LiveSandbox::new().await;

    let error = sandbox
        .files()
        .read("/tmp/does-not-exist", None)
        .await
        .expect_err("missing file");

    assert!(error.is_not_found(), "want not found, got {error:?}");
}

/// Creates `/tmp/tree` holding `a.txt` and `sub/b.txt` (envd creates the
/// parent directories on upload).
async fn make_tree(sandbox: &LiveSandbox) {
    let files = sandbox.files();
    files
        .write("/tmp/tree/a.txt", "a", None)
        .await
        .expect("write a");
    files
        .write("/tmp/tree/sub/b.txt", "b", None)
        .await
        .expect("write b");
}

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0054_list_depth_one_lists_direct_entries() {
    let sandbox = LiveSandbox::new().await;
    make_tree(&sandbox).await;

    let entries = sandbox
        .files()
        .list("/tmp/tree", 1, None)
        .await
        .expect("list");

    // `into_iter` consumes the Vec, so the names move into the set without cloning.
    let found: HashSet<(String, FileType)> = entries
        .into_iter()
        .map(|entry| (entry.name, entry.file_type))
        .collect();
    let want: HashSet<(String, FileType)> = [
        ("a.txt".to_owned(), FileType::File),
        ("sub".to_owned(), FileType::Directory),
    ]
    .into_iter()
    .collect();
    assert_eq!(found, want);
}

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0055_list_depth_two_recurses() {
    let sandbox = LiveSandbox::new().await;
    make_tree(&sandbox).await;

    let entries = sandbox
        .files()
        .list("/tmp/tree", 2, None)
        .await
        .expect("list");

    assert!(
        entries
            .iter()
            .any(|entry| entry.path == "/tmp/tree/sub/b.txt"),
        "depth 2 missing sub/b.txt: {entries:?}"
    );
}

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0056_get_info_reports_metadata() {
    let sandbox = LiveSandbox::new().await;
    sandbox
        .files()
        .write("/tmp/sized.txt", "12345", None)
        .await
        .expect("write");

    let info = sandbox
        .files()
        .get_info("/tmp/sized.txt", None)
        .await
        .expect("get_info");

    assert_eq!(info.size, 5);
    assert_eq!(info.file_type, FileType::File);
    assert!(!info.permissions.is_empty(), "no permissions: {info:?}");
    assert!(!info.owner.is_empty(), "no owner: {info:?}");
}

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0057_exists_true_and_false() {
    let sandbox = LiveSandbox::new().await;
    sandbox
        .files()
        .write("/tmp/present.txt", "x", None)
        .await
        .expect("write");

    let present = sandbox
        .files()
        .exists("/tmp/present.txt", None)
        .await
        .expect("exists present");
    let absent = sandbox
        .files()
        .exists("/tmp/absent.txt", None)
        .await
        .expect("exists absent");

    assert!(present, "exists is false for a written file");
    assert!(!absent, "exists is true for a missing file");
}

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0058_make_dir_reports_creation() {
    let sandbox = LiveSandbox::new().await;

    let first = sandbox
        .files()
        .make_dir("/tmp/newdir", None)
        .await
        .expect("first make_dir");
    let second = sandbox
        .files()
        .make_dir("/tmp/newdir", None)
        .await
        .expect("second make_dir");

    assert!(first, "make_dir of a new directory returned false");
    assert!(!second, "make_dir of an existing directory returned true");
}

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0059_rename_moves_entry() {
    let sandbox = LiveSandbox::new().await;
    let files = sandbox.files();
    files
        .write("/tmp/old.txt", "moving", None)
        .await
        .expect("write");

    let entry = files
        .rename("/tmp/old.txt", "/tmp/new.txt", None)
        .await
        .expect("rename");

    assert_eq!(entry.path, "/tmp/new.txt");
    assert!(
        !files
            .exists("/tmp/old.txt", None)
            .await
            .expect("exists old"),
        "the old path still exists"
    );
    let text = files
        .read_to_string("/tmp/new.txt", None)
        .await
        .expect("read new");
    assert_eq!(text, "moving");
}

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0060_remove_deletes_file_and_directory() {
    let sandbox = LiveSandbox::new().await;
    let files = sandbox.files();
    files
        .write("/tmp/gone/file.txt", "x", None)
        .await
        .expect("write");

    files
        .remove("/tmp/gone/file.txt", None)
        .await
        .expect("remove file");
    let file_exists = files
        .exists("/tmp/gone/file.txt", None)
        .await
        .expect("exists file");
    files.remove("/tmp/gone", None).await.expect("remove dir");
    let dir_exists = files.exists("/tmp/gone", None).await.expect("exists dir");

    assert!(!file_exists, "the removed file still exists");
    assert!(!dir_exists, "the removed directory still exists");
}

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0061_user_selects_file_owner() {
    let sandbox = LiveSandbox::new().await;
    let files = sandbox.files();

    files
        .write("/tmp/as-root.txt", "r", Some("root"))
        .await
        .expect("write as root");
    files
        .write("/tmp/as-user.txt", "u", Some("user"))
        .await
        .expect("write as user");

    let root_owner = files
        .get_info("/tmp/as-root.txt", None)
        .await
        .expect("stat root")
        .owner;
    let user_owner = files
        .get_info("/tmp/as-user.txt", None)
        .await
        .expect("stat user")
        .owner;
    assert_eq!(root_owner, "root");
    assert_eq!(user_owner, "user");
}
