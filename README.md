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

## What it covers (v0.1)

| Area | API |
|---|---|
| Configuration | `ConnectionConfig::builder()` / `from_env()`: API key, API URL, domain, sandbox URL, request timeout (`E2B_API_KEY`, `E2B_API_URL`, `E2B_DOMAIN`, `E2B_SANDBOX_URL`) |
| Lifecycle (REST) | `Client::{create_sandbox, connect_sandbox, get_sandbox_info, list_sandboxes, pause_sandbox, kill_sandbox, set_sandbox_timeout, refresh_sandbox}` and the same on `Sandbox` |
| Processes (envd) | `Commands::{start, run, connect, list, send_stdin, close_stdin, send_signal, kill}`; `CommandHandle::{next_event, wait}` streams stdout/stderr to an exit code |
| Files (envd) | `Filesystem::{read, read_to_string, write, list, get_info, exists, make_dir, rename, remove}` |
| Errors | `Error` (`thiserror`): REST status + E2B message, Connect error codes, envd HTTP errors, protocol, timeout, transport |

Out of scope for now: code interpreter, PTY sessions, directory watching,
template building, and a blocking API.

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

As of 2026-10-26 (milestone 1):

- **Hosted E2B:** all 46 behaviours above passed in a live run.
- **Wire-protocol claims:** all 51 are backed by passing mock and unit tests.
- **Refuted claims:** 0. Two were refuted during M1 and fixed: empty config
  values, and the envd version gate on `close_stdin`.
- **Not yet implemented:** the features under "Out of scope for now" are
  tracked as `not_enough_info` claims for milestones 2 to 5.

Live results are in `docs/verification/`. The road to a crates.io release is
in `docs/superpowers/specs/2026-10-26-claims-audit-and-publishability-design.md`.

## Development

See [CONTRIBUTING.md](CONTRIBUTING.md). In short: `just lint` and `just test`.

## License

Licensed under either of [Apache License 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.
