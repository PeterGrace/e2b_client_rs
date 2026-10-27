//! Live tests for the envd filesystem claims (S3). One test per ledger claim.

mod live_support;

use std::collections::HashSet;
use std::time::Duration;

use e2b_client::{
    CommandOptions, FileType, FilesystemEventType, WatchOptions, WriteEntry, WriteOptions,
};
use live_support::LiveSandbox;
use tokio::io::AsyncReadExt;

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
    // `ls -l` form: a regular file its writer (the default user) can read and write.
    assert!(
        info.permissions.starts_with("-rw"),
        "want `-rw...` permissions: {info:?}"
    );
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
        .expect("write file");
    // Keeps `/tmp/gone` non-empty, so removing it shows the documented recursion.
    files
        .write("/tmp/gone/kept.txt", "y", None)
        .await
        .expect("write kept");

    files
        .remove("/tmp/gone/file.txt", None)
        .await
        .expect("remove file");
    let file_exists = files
        .exists("/tmp/gone/file.txt", None)
        .await
        .expect("exists file");
    files
        .remove("/tmp/gone", None)
        .await
        .expect("remove non-empty dir");
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
    // `user` is also the template's default user, so this half would pass even
    // if the argument were ignored. The `root` half proves the parameter takes
    // effect; keep it.
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

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0062_write_files_writes_every_file() {
    let sandbox = LiveSandbox::new().await;
    let files = sandbox.files();
    // Every byte value, so a binary file in a new subdirectory is covered.
    let binary: Vec<u8> = (0..=255_u8).collect();
    let entries = [
        WriteEntry::new("/tmp/batch/a.txt", "alpha"),
        WriteEntry::new("/tmp/batch/sub/b.bin", binary.clone()),
    ];

    let written = files
        .write_files(&entries, &WriteOptions::default())
        .await
        .expect("write_files");

    let mut paths: Vec<&str> = written.iter().map(|info| info.path.as_str()).collect();
    paths.sort_unstable();
    assert_eq!(paths, ["/tmp/batch/a.txt", "/tmp/batch/sub/b.bin"]);
    let text = files
        .read("/tmp/batch/a.txt", None)
        .await
        .expect("read a.txt");
    let bytes = files
        .read("/tmp/batch/sub/b.bin", None)
        .await
        .expect("read b.bin");
    assert_eq!(text.as_ref(), b"alpha");
    assert!(bytes.as_ref() == binary.as_slice(), "b.bin changed");
}

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0063_watch_dir_reports_a_created_file() {
    let sandbox = LiveSandbox::new().await;
    let files = sandbox.files();
    files
        .make_dir("/tmp/watched", None)
        .await
        .expect("make_dir");
    let mut watch = files
        .watch_dir("/tmp/watched", &WatchOptions::default())
        .await
        .expect("watch_dir");

    files
        .write("/tmp/watched/new.txt", "x", None)
        .await
        .expect("write");
    // Events can arrive in a burst (create, write, chmod...), so look for
    // the create of this file rather than taking the first event. `seen`
    // keeps everything else for the failure message.
    let mut seen = Vec::new();
    let found = tokio::time::timeout(Duration::from_secs(20), async {
        while let Some(event) = watch.next_event().await.expect("next_event") {
            if event.event_type == FilesystemEventType::Create && event.name == "new.txt" {
                return true;
            }
            seen.push(event);
        }
        false
    })
    .await;

    assert!(
        matches!(found, Ok(true)),
        "no create event for new.txt within 20 s (envd {}, watch ended: {}); other events: {seen:?}",
        sandbox.envd_version(),
        matches!(found, Ok(false)),
    );
}

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0066_write_stream_uploads_more_than_it_buffers() {
    let sandbox = LiveSandbox::new().await;
    let files = sandbox.files();
    // 64 MiB generated on the fly by `repeat(..).take(..)`: the data never
    // exists in memory as a whole, only in the reader's small buffers.
    const SIZE: u64 = 64 << 20;
    let plain_source = tokio::io::repeat(b'z').take(SIZE);
    let gzip_source = tokio::io::repeat(b'z').take(SIZE);

    let plain = files
        .write_stream("/tmp/plain.bin", plain_source, &WriteOptions::default())
        .await
        .expect("write_stream plain");
    let gzipped = files
        .write_stream(
            "/tmp/gz.bin",
            gzip_source,
            &WriteOptions::default().gzip(true),
        )
        .await
        .expect("write_stream gzip");

    assert_eq!(plain.path, "/tmp/plain.bin");
    assert_eq!(gzipped.path, "/tmp/gz.bin");
    let expected_size = i64::try_from(SIZE).expect("64 MiB fits i64");
    for target in ["/tmp/plain.bin", "/tmp/gz.bin"] {
        let info = files.get_info(target, None).await.expect("get_info");
        assert_eq!(info.size, expected_size, "{target} size");
    }
    // Only `z` bytes, so envd stored the decompressed data, not the gzip.
    let others = sandbox
        .commands()
        .run("tr -d z < /tmp/gz.bin | wc -c", &CommandOptions::default())
        .await
        .expect("run tr");
    assert_eq!(others.stdout.trim(), "0", "non-z bytes in /tmp/gz.bin");
}

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn clm_0128_read_stream_downloads_in_chunks() {
    let sandbox = LiveSandbox::new().await;
    // `yes` repeats the 16-byte line "0123456789abcde\n"; 8 MiB of it.
    const PATTERN: &[u8; 16] = b"0123456789abcde\n";
    const SIZE: usize = 8 << 20;
    let made = sandbox
        .commands()
        .run(
            "yes 0123456789abcde | head -c 8388608 > /tmp/big.txt",
            &CommandOptions::default(),
        )
        .await
        .expect("run yes");
    assert_eq!(made.exit_code, 0, "making /tmp/big.txt: {}", made.stderr);

    let mut stream = sandbox
        .files()
        .read_stream("/tmp/big.txt", None)
        .await
        .expect("read_stream");
    // Check each chunk against the pattern at its offset, then drop it, so
    // the file is never held whole.
    let mut offset = 0_usize;
    let mut chunks = 0_usize;
    while let Some(chunk) = stream.next_chunk().await.expect("next_chunk") {
        let matches = chunk
            .iter()
            .enumerate()
            .all(|(index, byte)| *byte == PATTERN[(offset + index) % PATTERN.len()]);
        assert!(
            matches,
            "chunk {chunks} at offset {offset} breaks the pattern"
        );
        offset += chunk.len();
        chunks += 1;
    }

    assert_eq!(offset, SIZE, "total bytes");
    assert!(chunks > 1, "expected several chunks, got {chunks}");
}
