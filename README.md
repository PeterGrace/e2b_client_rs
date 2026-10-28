# e2b_client_rs

An async Rust client for the [E2B](https://e2b.dev) sandbox API: the REST
sandbox lifecycle and envd's process and filesystem services. It speaks the
standard E2B API the official Python and JavaScript SDKs use, so it works with
hosted E2B and with E2B-compatible servers.

The crate is `e2b-client` (library `e2b_client`). It is not on crates.io yet,
and needs Rust 1.89 or newer; depend on it from git:

```toml
[dependencies]
e2b-client = { git = "https://github.com/PeterGrace/e2b_client_rs" }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

## Quick start

```rust,no_run
use e2b_client::{
    Client, CommandOptions, ConnectOptions, CreateSandboxParams, FileOptions, PauseOptions,
    WriteOptions,
};

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
    let files = sandbox.files();
    files.write("/tmp/hello.txt", "hi there", &WriteOptions::default()).await?;
    let text = files.read_to_string("/tmp/hello.txt", &FileOptions::default()).await?;
    assert_eq!(text, "hi there");

    // Pause, then resume by connecting again.
    sandbox.pause(&PauseOptions::new()).await?;
    let sandbox = client
        .connect_sandbox(sandbox.sandbox_id(), &ConnectOptions::new())
        .await?;

    sandbox.kill().await?;
    Ok(())
}
```

## What it covers

| Area | API |
|---|---|
| Configuration | `ConnectionConfig::builder()` / `from_env()`: API key, API URL, domain, sandbox URL, request timeout (`E2B_API_KEY`, `E2B_API_URL`, `E2B_DOMAIN`, `E2B_SANDBOX_URL`) |
| Lifecycle (REST) | `Client::{create_sandbox, connect_sandbox, get_sandbox_info, list_sandboxes, pause_sandbox, kill_sandbox, set_sandbox_timeout, refresh_sandbox}` and the same on `Sandbox`; one method per operation: `connect_sandbox(id, &ConnectOptions)` (v2 connect only: a 404 is `Error::SandboxNotFound`) and `pause_sandbox(id, &PauseOptions)` / `Sandbox::pause(&PauseOptions)`; `ListSandboxesParams::{metadata, state, template, started_after, order}` |
| Hosts and metrics | `Sandbox::get_host(port)` gives the host name that reaches a port inside the sandbox; `Sandbox::get_metrics` / `Client::get_sandbox_metrics` read CPU, memory and disk samples (`MetricsParams::{start, end}`) |
| Network | `CreateSandboxParams::network(NetworkConfig)`: `allow_out`, `deny_out` (`ALL_TRAFFIC`), `dns_servers`, per-host header `rule`s, `allow_public_traffic`, `mask_request_host`, `https_ports`; `Sandbox::update_network(NetworkUpdate)` and `Client::update_sandbox_network` replace a running sandbox's egress without a restart |
| Lifecycle settings | `CreateSandboxParams::lifecycle(Lifecycle)`: `on_timeout` (`OnTimeout::Kill`, or a pause with a `SnapshotMode`) and `auto_resume`; `idempotency_key`; `Client::pause_sandbox(id, &PauseOptions)` / `Sandbox::pause(&PauseOptions)` and `Client::connect_sandbox(id, &ConnectOptions)` choose a full or filesystem-only snapshot and a restore or a reboot (`OnResume`) |
| Snapshots and fork | `Sandbox::{create_snapshot, list_snapshots, fork}`, `Client::{create_snapshot, list_snapshots, delete_snapshot, fork_sandbox}`; a snapshot ID is a template, so `CreateSandboxParams::new(snapshot_id)` starts a sandbox from it; `fork` returns one result per copy (up to `MAX_FORK_COUNT`) |
| Volumes | `Client::{create_volume, get_volume, list_volumes, destroy_volume}` (the volume token stays a `SecretString`); `CreateSandboxParams::volume_mount(name, path)` |
| MCP gateway | `CreateSandboxParams::mcp(config)` starts the gateway (on the `mcp-gateway` template unless one is given); `Sandbox::{mcp_url, mcp_token}` |
| Processes (envd) | `Commands::{start, run, connect, list, send_stdin, close_stdin, stream_input, send_signal, kill}`; `Commands::connect(pid, &CommandConnectOptions)` with the SDKs' 60 s stream deadline by default; `request_timeout` on `CommandOptions` and `CommandConnectOptions`; a zero timeout means no limit; `CommandHandle::{next_event, wait}` streams stdout/stderr to an exit code; `InputStream::{send, close}` streams stdin in order over one call |
| PTY sessions (envd) | `Pty::{create, connect, send_stdin, stream_input, resize, kill}`; `Pty::connect(pid, &PtyConnectOptions)`; `PtyOptions` and `PtyConnectOptions` set no deadline by default (`DEFAULT_PTY_TIMEOUT` is the SDKs' 60 s, to opt in) and take a `request_timeout`; `PtyHandle::{next_event, wait}` streams terminal output to an exit code |
| Files (envd) | `Filesystem::{read, read_to_string, read_stream, write, write_files, write_stream, list, get_info, exists, make_dir, rename, remove, watch_dir}`; every call takes an options value: `FileOptions::{user, request_timeout}`, `ListOptions::{depth, user, request_timeout}`, and `request_timeout` on `WriteOptions` and `WatchOptions`; `WriteOptions` selects multipart, octet-stream or gzip uploads, attaches metadata and caps uploads in flight (`max_concurrency`, `DEFAULT_UPLOAD_CONCURRENCY` 8); a symlink is `FileType::Symlink` on every envd version; `FileStream::next_chunk` and `WatchHandle::next_event` stream downloads and directory events |
| Signed URLs | `Sandbox::{upload_url, download_url}` with `FileUrlOptions::{user, expires_in}`: plain HTTP links signed with the sandbox's envd access token |
| Errors | `Error` (`thiserror`): REST status + E2B message, Connect error codes, envd HTTP errors, protocol, timeout, transport (`TransportError`, with `is_timeout`/`is_connect`), invalid argument; `status()` and the SDK's categories as `is_not_found`, `is_auth`, `is_rate_limited`, `is_service_busy`, `is_not_enough_space`, `is_sandbox_unreachable`, `is_timeout`, `is_retryable` |

Out of scope by design: code interpreter, template building, and a blocking
API. Not implemented yet: per-call request timeouts on REST calls and on
pid-only process calls, the SOCKS5 egress proxy and IAM workload identity in
network settings, the volume content API (files in a volume without a
sandbox), sandbox logs, secrets and git.

## Network, lifecycle and snapshots

```rust,no_run
use std::time::Duration;

use e2b_client::{
    ALL_TRAFFIC, Client, CreateSandboxParams, FileOptions, Lifecycle, NetworkConfig, OnTimeout,
    SnapshotMode, SnapshotOptions, WriteOptions,
};

async fn snapshot_tour(client: &Client) -> e2b_client::Result<()> {
    // Deny all egress except one address, and pause (keeping memory) instead
    // of being killed at the timeout; a later connect resumes it.
    let network = NetworkConfig::new().deny_out([ALL_TRAFFIC]).allow_out(["1.1.1.1"]);
    let lifecycle = Lifecycle::new()
        .on_timeout(OnTimeout::pause_with(SnapshotMode::Full))
        .auto_resume(true);
    let params = CreateSandboxParams::new("base")
        .timeout(Duration::from_secs(600))
        .network(network)
        .lifecycle(lifecycle);
    let source = client.create_sandbox(&params).await?;
    source
        .files()
        .write("/tmp/state.txt", "warm", &WriteOptions::default())
        .await?;

    // A snapshot ID works as a template: start a second sandbox from it.
    let snapshot = source
        .create_snapshot(&SnapshotOptions::new().name("warm-start"))
        .await?;
    let copy = client
        .create_sandbox(&CreateSandboxParams::new(snapshot.snapshot_id.clone()))
        .await?;
    let state = copy
        .files()
        .read_to_string("/tmp/state.txt", &FileOptions::default())
        .await?;
    assert_eq!(state, "warm");

    // A snapshot cannot be deleted while a sandbox started from it runs.
    copy.kill().await?;
    client.delete_snapshot(&snapshot.snapshot_id).await?;
    source.kill().await?;
    Ok(())
}
```

## Streaming, PTYs, watching and signed URLs

```rust,no_run
use std::time::Duration;

use e2b_client::{
    CommandOptions, FileOptions, FileUrlOptions, PtyEvent, PtyOptions, PtySize, Sandbox,
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

    // An interactive terminal. It has no deadline by default (the SDKs use
    // 60 s, which ends the shell mid-session); set one with
    // `PtyOptions::default().timeout(Some(..))`.
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
    let mut download = sandbox
        .files()
        .read_stream("/tmp/big.bin", &FileOptions::default())
        .await?;
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

## Versioning

- **Semver.** `e2b-client` follows [Semantic Versioning](https://semver.org).
  Before 1.0, a minor release (0.2 to 0.3) may break the API and a patch
  release (0.2.0 to 0.2.1) never does. Each release's breaking changes are
  listed in its changelog entry.
- **Minimum supported Rust version: 1.89.** It is `rust-version` in
  `Cargo.toml` and is checked in CI. Raising it is a minor release, never a
  patch release. CI checks this repository's `Cargo.lock`, which Cargo
  ignores when the crate is a dependency: your own lockfile picks the
  transitive versions. On Rust 1.89 exactly, a newer transitive release may
  need a newer Rust. Cargo's MSRV-aware resolver (resolver 3, the default for
  edition 2024) avoids that by preferring versions that build on your
  `rust-version`, or on your toolchain when none is set. With an older
  resolver, pin the offending crate with `cargo update <crate> --precise <version>`.
- **Public dependencies.** These crates' types or traits appear in the API,
  so a breaking release of one of them (a new major version, or a new minor
  version before 1.0) is a breaking release of this crate too:

  | Crate | Where it appears |
  |---|---|
  | `bytes` 1 | `Bytes` in `CommandEvent`, `PtyEvent`, `FileStream::next_chunk`, `WriteEntry::new`, `InputStream::send`, `Filesystem::read` |
  | `chrono` 0.4 | `DateTime<Utc>` in `SandboxInfo`, `EntryInfo`, `SandboxMetrics`, `MetricsParams`, `ListSandboxesParams::started_after` |
  | `secrecy` 0.10 | `SecretString` from `Sandbox::traffic_access_token`, `Sandbox::mcp_token` and `Volume::token`; re-exported as `e2b_client::secrecy`, so use that path to name the matching version |
  | `serde` 1 | `Serialize`/`Deserialize` impls on `SandboxState`, `SnapshotMode`, `NetworkConfig`, `NetworkUpdate`, `NetworkRule`, `VolumeMount` and the result types |
  | `serde_json` 1 | `Error::Decode`, `CreateSandboxParams::mcp` (`serde_json::Map`) |
  | `tokio` 1 | `tokio::io::AsyncRead` in `Filesystem::write_stream`; every future needs a tokio runtime |
  | `url` 2 | `ConnectionConfig::api_url` |

  Every other dependency (reqwest, base64, sha2, async-compression, ...) is
  private: upgrading it is not a breaking change. That is why
  `Error::Transport` holds this crate's `TransportError` rather than a
  `reqwest::Error`.

## Verification status

Every behaviour listed above is backed by a claim in `claims.json`, the claims
ledger. Each claim has a recorded verdict and the evidence behind it.

As of 2026-10-28 (milestone 4):

- **Hosted E2B:** 76 of the 79 live tests passed (46 from milestone 1, 11
  from milestone 2, 18 from milestone 3, 4 from milestone 4) in run
  `20261028T153214Z-hosted-761165`. The 3 that did not are the volume
  tests: volumes are not enabled for the test team (403), so those claims
  stay `not_enough_info` rather than refuted.
- **Wire-protocol claims:** all 79 are backed by passing mock and unit tests,
  as are CLM-0122 (`Debug` output never shows secrets) and CLM-0152 (the
  idempotency key).
- **Ledger:** 179 active claims: 169 supported, 10 `not_enough_info`, 0
  refuted, 0 unverified. <!-- LEDGER: confirm -->
- **Refuted claims:** 0. Two were refuted during M1 and fixed: empty config
  values, and the envd version gate on `close_stdin`.
- **Not yet shown:** the egress proxy and IAM (unscheduled), volumes on a
  team that has them, and the release itself (milestone 5: a live CI job and
  a publish dry run) are tracked as `not_enough_info` claims.
- **E2B-compatible servers:** a self-hosted sandbawks server passes 29 of the
  79 in one paced run, `20261028T154205Z-sandbawks-paced-913873`, with no
  failure on capacity; every failure there is a server defect or a route it
  does not serve yet, filed upstream.
- **API:** the API milestone 4 settled is the one 0.2.0 will ship. Its
  breaking changes, old call to new call, are listed in
  `changelog/2026-10-28T18-39-36Z-m4-milestone.md`.

Live results are in `docs/verification/`. The road to a crates.io release is
in `docs/superpowers/specs/2026-10-26-claims-audit-and-publishability-design.md`.

## Development

See [CONTRIBUTING.md](CONTRIBUTING.md). In short: `just lint` and `just test`.

## License

Licensed under either of [Apache License 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.
