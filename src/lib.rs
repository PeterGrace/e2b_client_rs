//! An async Rust client for the [E2B](https://e2b.dev) sandbox API.
//!
//! It speaks the standard E2B API that the official Python and JavaScript
//! SDKs use, so it works against hosted E2B and any E2B-compatible server:
//!
//! - **REST control plane** ([`Client`]): create, connect/resume, inspect,
//!   list, pause, kill and extend sandboxes.
//! - **envd process service** ([`Commands`]): start commands, stream their
//!   stdout and stderr to an exit code, send stdin (one write at a time or
//!   as an ordered [`InputStream`]), signal, list processes.
//! - **envd PTY sessions** ([`Pty`]): interactive terminals with their own
//!   output stream ([`PtyHandle`]), input, resize and kill.
//! - **envd filesystem service** ([`Filesystem`]): read and write files
//!   (one at a time or in a batch, multipart, octet-stream or gzip, with
//!   optional metadata, see [`WriteOptions`]), stream uploads from an `AsyncRead` and downloads as
//!   chunks ([`FileStream`]), list directories, stat, make directories, move
//!   and remove, and watch a directory for changes ([`WatchHandle`]).
//! - **Signed file URLs** ([`Sandbox::upload_url`], [`Sandbox::download_url`]):
//!   plain HTTP upload and download links, signed with the sandbox's envd
//!   access token, optionally expiring ([`FileUrlOptions`]).
//!
//! # Quick start
//!
//! ```no_run
//! use e2b_client::{Client, CommandOptions, CreateSandboxParams};
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
//!     sandbox.files().write("/tmp/hello.txt", "hello", None).await?;
//!     let text = sandbox.files().read_to_string("/tmp/hello.txt", None).await?;
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

mod client;
mod commands;
mod config;
mod envd;
mod error;
mod filesystem;
mod models;
mod pty;
mod sandbox;
mod stream_input;
mod wire;

// Compiles the README's examples as doctests under `cargo test`, so they
// can't drift from the API. `cfg(doctest)` keeps this type out of normal
// builds and out of the docs.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;

pub use client::Client;
pub use commands::{
    CommandEvent, CommandExit, CommandHandle, CommandOptions, CommandResult, Commands,
    DEFAULT_COMMAND_TIMEOUT, ProcessInfo, Signal,
};
pub use config::{
    ConnectionConfig, ConnectionConfigBuilder, DEFAULT_DOMAIN, DEFAULT_REQUEST_TIMEOUT, ENVD_PORT,
};
pub use envd::EnvdVersion;
pub use error::{ConnectCode, Error, Result};
pub use filesystem::{
    DEFAULT_WATCH_TIMEOUT, EntryInfo, FileStream, FileType, Filesystem, FilesystemEvent,
    FilesystemEventType, WatchHandle, WatchOptions, WriteEntry, WriteInfo, WriteOptions,
};
pub use models::{
    CreateSandboxParams, DEFAULT_TEMPLATE, ListSandboxesParams, SandboxInfo, SandboxPage,
    SandboxState,
};
pub use pty::{DEFAULT_PTY_TIMEOUT, Pty, PtyEvent, PtyHandle, PtyOptions, PtySize};
pub use sandbox::{FileUrlOptions, Sandbox};
pub use stream_input::{DEFAULT_STREAM_INPUT_TIMEOUT, InputStream, StreamInputOptions};
