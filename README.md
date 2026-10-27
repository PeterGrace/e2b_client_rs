# e2b_client_rs

An async Rust client for the [E2B](https://e2b.dev) sandbox API: the REST
sandbox lifecycle and envd's process and filesystem services. It speaks the
standard E2B API the official Python and JavaScript SDKs use, so it works with
hosted E2B and with E2B-compatible servers.

The crate is `e2b-client` (library `e2b_client`). It is not on crates.io yet;
depend on it from git:

```toml
[dependencies]
e2b-client = { git = "https://github.com/PeterGrace/e2b_client_rs" }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

## Quick start

```rust,no_run
use e2b_client::{Client, CommandOptions, CreateSandboxParams};

#[tokio::main]
async fn main() -> e2b_client::Result<()> {
    // Reads E2B_API_KEY, and optionally E2B_API_URL and E2B_DOMAIN.
    let client = Client::from_env()?;
    let sandbox = client.create_sandbox(&CreateSandboxParams::new("base")).await?;

    // Run a command to completion.
    let result = sandbox.commands().run("echo hello", &CommandOptions::default()).await?;
    assert_eq!(result.stdout, "hello\n");
    assert_eq!(result.exit_code, 0);

    // Files.
    sandbox.files().write("/tmp/hello.txt", "hi there", None).await?;
    let text = sandbox.files().read_to_string("/tmp/hello.txt", None).await?;
    assert_eq!(text, "hi there");

    // Pause, then resume by connecting again.
    sandbox.pause().await?;
    let sandbox = client.connect_sandbox(sandbox.sandbox_id(), None).await?;

    sandbox.kill().await?;
    Ok(())
}
```

## What it covers

| Area | API |
|---|---|
| Configuration | `ConnectionConfig::builder()` / `from_env()`: API key, API URL, domain, sandbox URL, request timeout (`E2B_API_KEY`, `E2B_API_URL`, `E2B_DOMAIN`, `E2B_SANDBOX_URL`) |
| Lifecycle (REST) | `Client::{create_sandbox, connect_sandbox, get_sandbox_info, list_sandboxes, pause_sandbox, kill_sandbox, set_sandbox_timeout, refresh_sandbox}` and the same on `Sandbox` |
| Processes (envd) | `Commands::{start, run, connect, list, send_stdin, close_stdin, stream_input, send_signal, kill}`; `CommandHandle::{next_event, wait}` streams stdout/stderr to an exit code; `InputStream::{send, close}` streams stdin in order over one call |
| PTY sessions (envd) | `Pty::{create, connect, send_stdin, stream_input, resize, kill}`; `PtyHandle::{next_event, wait}` streams terminal output to an exit code |
| Files (envd) | `Filesystem::{read, read_to_string, read_stream, write, write_files, write_stream, list, get_info, exists, make_dir, rename, remove, watch_dir}`; `WriteOptions` selects multipart, octet-stream or gzip uploads and attaches metadata; `FileStream::next_chunk` and `WatchHandle::next_event` stream downloads and directory events |
| Signed URLs | `Sandbox::{upload_url, download_url}` with `FileUrlOptions::{user, expires_in}`: plain HTTP links signed with the sandbox's envd access token |
| Errors | `Error` (`thiserror`): REST status + E2B message, Connect error codes, envd HTTP errors, protocol, timeout, transport, invalid argument |

Out of scope for now: code interpreter, template building, and a blocking API.
Sandbox-level `get_host`, metrics, snapshots and network updates come in
milestone 3.

## Streaming, PTYs, watching and signed URLs

```rust,no_run
use std::time::Duration;

use e2b_client::{
    CommandOptions, FileUrlOptions, PtyEvent, PtyOptions, PtySize, Sandbox,
    StreamInputOptions, WatchOptions, WriteEntry, WriteOptions,
};

async fn tour(sandbox: &Sandbox) -> e2b_client::Result<()> {
    // Stream stdin to a command in order, over one StreamInput call.
    let handle = sandbox
        .commands()
        .start("cat", &CommandOptions::default().stdin(true))
        .await?;
    let mut input = sandbox
        .commands()
        .stream_input(handle.pid(), &StreamInputOptions::default());
    input.send("one ").await?;
    input.send("two\n").await?;
    input.close().await?;
    sandbox.commands().close_stdin(handle.pid()).await?;
    assert_eq!(handle.wait().await?.stdout, "one two\n");

    // An interactive terminal.
    let mut pty = sandbox
        .pty()
        .create(PtySize::new(80, 24), &PtyOptions::default())
        .await?;
    pty.resize(PtySize::new(120, 40)).await?;
    pty.send_stdin("echo hi; exit\n").await?;
    // PtyEvent is #[non_exhaustive], so match it with `if let` or a `_` arm.
    while let Some(event) = pty.next_event().await? {
        if let PtyEvent::Data(bytes) = event {
            print!("{}", String::from_utf8_lossy(&bytes));
        }
    }

    // Watch a directory, then write several files in one call.
    let mut watch = sandbox.files().watch_dir("/tmp", &WatchOptions::default()).await?;
    let entries = [WriteEntry::new("/tmp/a.txt", "alpha"), WriteEntry::new("/tmp/b.txt", "beta")];
    sandbox
        .files()
        .write_files(&entries, &WriteOptions::default().metadata("team", "docs"))
        .await?;
    if let Some(event) = watch.next_event().await? {
        println!("{:?} {}", event.event_type, event.name);
    }

    // Stream an upload from any AsyncRead, and a download as chunks
    // (tokio::io::repeat and take need tokio's "io-util" feature).
    let reader = tokio::io::repeat(b'z');
    let reader = tokio::io::AsyncReadExt::take(reader, 64 << 20);
    sandbox
        .files()
        .write_stream("/tmp/big.bin", reader, &WriteOptions::default().gzip(true))
        .await?;
    let mut download = sandbox.files().read_stream("/tmp/big.bin", None).await?;
    while let Some(chunk) = download.next_chunk().await? {
        println!("{} bytes", chunk.len());
    }

    // A link anyone can use for the next five minutes, without an API key.
    let url = sandbox.download_url(
        "/tmp/a.txt",
        &FileUrlOptions::default().expires_in(Duration::from_secs(300)),
    )?;
    println!("{url}");
    Ok(())
}
```

## How it talks to E2B

- REST calls go to `E2B_API_URL` (default `https://api.<domain>`) with the
  `X-API-KEY` header.
- envd is reached at `https://49983-<sandbox-id>.<domain>` (or
  `https://sandbox.<domain>` for hosted E2B RPCs), with the `E2b-Sandbox-Id`,
  `E2b-Sandbox-Port` and `X-Access-Token` headers.
- envd's services are Connect-RPC with the JSON codec, as in both official SDKs.

Every wire detail, with citations into the official SDK sources, is in
[docs/protocol-notes.md](docs/protocol-notes.md).

## Verification status

Every behaviour listed above is backed by a claim in `claims.json`, the claims
ledger. Each claim has a recorded verdict and the evidence behind it.

As of 2026-10-27 (milestone 2):

- **Hosted E2B:** all 57 behaviours above passed in a live run (46 from
  milestone 1, 11 added in milestone 2).
- **Wire-protocol claims:** all 63 are backed by passing mock and unit tests, as is CLM-0122 (`Debug` output never shows secrets).
- **Refuted claims:** 0. Two were refuted during M1 and fixed: empty config
  values, and the envd version gate on `close_stdin`.
- **Not yet implemented:** sandbox-level parity (milestone 3) and
  publishability (milestones 4 and 5) are tracked as `not_enough_info` claims.
- **E2B-compatible servers:** a self-hosted sandbawks server passes 29 of the
  57; every failure there is a server defect, filed upstream.

Live results are in `docs/verification/`. The road to a crates.io release is
in `docs/superpowers/specs/2026-10-26-claims-audit-and-publishability-design.md`.

## Development

See [CONTRIBUTING.md](CONTRIBUTING.md). In short: `just lint` and `just test`.

## License

Licensed under either of [Apache License 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.
