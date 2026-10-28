//! An async Rust client for the [E2B](https://e2b.dev) sandbox API.
//!
//! It speaks the standard E2B API that the official Python and JavaScript
//! SDKs use, so it works against hosted E2B and any E2B-compatible server:
//!
//! - **REST control plane** ([`Client`]): create, connect/resume, inspect,
//!   list (filtered by metadata, state, template or start time, and ordered,
//!   see [`ListSandboxesParams`]), pause, kill and extend sandboxes, find the
//!   host that reaches a port inside one ([`Sandbox::get_host`]), read their
//!   CPU, memory and disk metrics ([`MetricsParams`]), and set their network
//!   egress rules
//!   at create ([`NetworkConfig`]) or on a running sandbox
//!   ([`NetworkUpdate`]). At create, choose what happens at the timeout
//!   ([`Lifecycle`]: kill, or pause with a [`SnapshotMode`]) and make the
//!   request idempotent ([`CreateSandboxParams::idempotency_key`]). Choose
//!   what a pause keeps ([`PauseOptions`]) and whether a resume restores the
//!   memory or reboots ([`ConnectOptions`], [`OnResume`]). Snapshot a
//!   sandbox ([`SnapshotOptions`]), list ([`ListSnapshotsParams`]) and
//!   delete snapshots, and start new sandboxes from one. Fork a running
//!   sandbox into up to [`MAX_FORK_COUNT`] copies ([`ForkOptions`]).
//!   Create, look up, list and destroy persistent volumes ([`Volume`],
//!   [`VolumeInfo`]) and mount them into new sandboxes
//!   ([`CreateSandboxParams::volume_mount`]). Start an MCP gateway in a new
//!   sandbox ([`CreateSandboxParams::mcp`]) and reach it at
//!   [`Sandbox::mcp_url`] with [`Sandbox::mcp_token`].
//! - **envd process service** ([`Commands`]): start commands
//!   ([`CommandOptions`]), stream their stdout and stderr to an exit code,
//!   attach to a running one ([`CommandConnectOptions`], with the SDKs' 60 s
//!   stream deadline by default), send stdin (one write at a time or as an
//!   ordered [`InputStream`]), signal, list processes.
//! - **envd PTY sessions** ([`Pty`]): interactive terminals with their own
//!   output stream ([`PtyHandle`]), input, resize and kill. A PTY has no
//!   deadline unless you set one ([`PtyOptions`], [`PtyConnectOptions`]).
//! - **envd filesystem service** ([`Filesystem`]): read and write files
//!   (one at a time or in a batch, multipart, octet-stream or gzip, with
//!   optional metadata and at most [`DEFAULT_UPLOAD_CONCURRENCY`] uploads in
//!   flight by default, see [`WriteOptions`]), stream uploads from an
//!   `AsyncRead` and downloads as chunks ([`FileStream`]), list directories
//!   ([`ListOptions`]), stat, make directories, move and remove
//!   ([`FileOptions`]), and watch a directory for changes ([`WatchHandle`]).
//! - **Signed file URLs** ([`Sandbox::upload_url`], [`Sandbox::download_url`]):
//!   plain HTTP upload and download links, signed with the sandbox's envd
//!   access token, optionally expiring ([`FileUrlOptions`]).
//! - **Errors** ([`Error`]): one variant per protocol, and the SDKs'
//!   categories as classifiers: [`Error::status`], [`Error::is_not_found`],
//!   [`Error::is_auth`], [`Error::is_rate_limited`],
//!   [`Error::is_service_busy`], [`Error::is_not_enough_space`],
//!   [`Error::is_sandbox_unreachable`], [`Error::is_timeout`] and
//!   [`Error::is_retryable`]. Transport failures are an opaque
//!   [`TransportError`], so no HTTP client type is part of the API.
//!
//! Every call that takes a `*Options` type takes it by reference, and each
//! `*Options` type has `new()`, `Default` and builder setters, so adding an
//! option later is not a breaking change. Their `request_timeout` setters
//! bound that one call. A zero stream deadline or request timeout means no limit,
//! as in the SDKs.
//!
//! # Quick start
//!
//! ```no_run
//! use e2b_client::{Client, CommandOptions, CreateSandboxParams, FileOptions, WriteOptions};
//!
//! #[tokio::main]
//! async fn main() -> e2b_client::Result<()> {
//!     // Reads E2B_API_KEY (and optionally E2B_API_URL / E2B_DOMAIN).
//!     let client = Client::from_env()?;
//!     let sandbox = client.create_sandbox(&CreateSandboxParams::new("base")).await?;
//!
//!     let result = sandbox.commands().run("uname -a", &CommandOptions::default()).await?;
//!     println!("exit {}: {}", result.exit_code, result.stdout);
//!
//!     let files = sandbox.files();
//!     files.write("/tmp/hello.txt", "hello", &WriteOptions::default()).await?;
//!     let text = files.read_to_string("/tmp/hello.txt", &FileOptions::default()).await?;
//!     assert_eq!(text, "hello");
//!
//!     sandbox.kill().await?;
//!     Ok(())
//! }
//! ```
//!
//! # Python-to-Rust notes
//!
//! Every network call is an `async fn`: it returns a future that does nothing
//! until you `.await` it inside an async runtime such as tokio (the
//! equivalent of `asyncio.run`). Fallible calls return [`Result`]; the `?`
//! operator propagates the error the way an uncaught exception would.
//!
//! # Versioning
//!
//! The crate follows semver, needs Rust 1.89 or newer, and lists the
//! dependencies whose types appear in its API; see the README's
//! [Versioning](https://github.com/PeterGrace/e2b_client_rs#versioning)
//! section.

mod client;
mod commands;
mod config;
mod envd;
mod error;
mod filesystem;
mod lifecycle;
mod mcp;
mod metrics;
mod models;
mod network;
mod pty;
mod sandbox;
mod snapshots;
mod stream_input;
mod volumes;
mod wire;

// Compiles the README's examples as doctests under `cargo test`, so they
// can't drift from the API. `cfg(doctest)` keeps this type out of normal
// builds and out of the docs.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;

pub use client::Client;
pub use commands::{
    CommandConnectOptions, CommandEvent, CommandExit, CommandHandle, CommandOptions, CommandResult,
    Commands, DEFAULT_COMMAND_TIMEOUT, ProcessInfo, Signal,
};
pub use config::{
    ConnectionConfig, ConnectionConfigBuilder, DEFAULT_DOMAIN, DEFAULT_REQUEST_TIMEOUT, ENVD_PORT,
};
pub use envd::EnvdVersion;
pub use error::{ConnectCode, Error, Result, TransportError};
pub use filesystem::{
    DEFAULT_UPLOAD_CONCURRENCY, DEFAULT_WATCH_TIMEOUT, EntryInfo, FileOptions, FileStream,
    FileType, Filesystem, FilesystemEvent, FilesystemEventType, ListOptions, WatchHandle,
    WatchOptions, WriteEntry, WriteInfo, WriteOptions,
};
pub use lifecycle::{
    ConnectOptions, Lifecycle, LifecycleInfo, OnResume, OnTimeout, PauseOptions, SnapshotMode,
    TimeoutAction,
};
pub use mcp::{DEFAULT_MCP_TEMPLATE, MCP_PORT};
pub use metrics::{MetricsParams, SandboxMetrics};
pub use models::{
    CreateSandboxParams, DEFAULT_TEMPLATE, ForkOptions, ListSandboxesParams, MAX_FORK_COUNT,
    SandboxInfo, SandboxPage, SandboxState, SortOrder,
};
pub use network::{ALL_TRAFFIC, NetworkConfig, NetworkInfo, NetworkRule, NetworkUpdate};
pub use pty::{
    DEFAULT_PTY_TIMEOUT, Pty, PtyConnectOptions, PtyEvent, PtyHandle, PtyOptions, PtySize,
};
pub use sandbox::{FileUrlOptions, Sandbox};
pub use snapshots::{ListSnapshotsParams, SnapshotInfo, SnapshotOptions, SnapshotPage};
pub use stream_input::{DEFAULT_STREAM_INPUT_TIMEOUT, InputStream, StreamInputOptions};
pub use volumes::{Volume, VolumeInfo, VolumeMount};

/// The `secrecy` crate, at the version this crate's API uses: the envd and
/// MCP tokens are [`secrecy::SecretString`]s, and `secrecy::ExposeSecret`
/// reads them.
pub use secrecy;
