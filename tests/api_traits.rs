//! Compile-time checks of the traits the public API promises (CLM-0179):
//! Rust API Guidelines C-SEND-SYNC and C-DEBUG, plus `new()` and `Default`
//! agreeing on every options type (C-COMMON-TRAITS).
//!
//! A missing trait is a compile error naming the type, so this file fails
//! to build rather than failing at run time.
//!
//! Python note: Rust checks these "interfaces" at compile time. `T: Send`
//! means a value can move to another thread; `T: Sync` means `&T` can be
//! shared between threads. tokio's multi-threaded runtime needs both for
//! anything a spawned task holds.

use std::fmt::Debug;

use e2b_client::{
    Client, CommandConnectOptions, CommandEvent, CommandExit, CommandHandle, CommandOptions,
    CommandResult, Commands, ConnectCode, ConnectOptions, ConnectionConfig,
    ConnectionConfigBuilder, CreateSandboxParams, EntryInfo, EnvdVersion, Error, FileOptions,
    FileStream, FileType, FileUrlOptions, Filesystem, FilesystemEvent, FilesystemEventType,
    ForkOptions, InputStream, Lifecycle, LifecycleInfo, ListOptions, ListSandboxesParams,
    ListSnapshotsParams, MetricsParams, NetworkConfig, NetworkInfo, NetworkRule, NetworkUpdate,
    OnResume, OnTimeout, PauseOptions, ProcessInfo, Pty, PtyConnectOptions, PtyEvent, PtyHandle,
    PtyOptions, PtySize, RequestOptions, Sandbox, SandboxInfo, SandboxMetrics, SandboxPage,
    SandboxState, Signal, SnapshotInfo, SnapshotMode, SnapshotOptions, SnapshotPage, SortOrder,
    StreamInputOptions, TimeoutAction, TransportError, Volume, VolumeInfo, VolumeMount,
    WatchHandle, WatchOptions, WriteEntry, WriteInfo, WriteOptions,
};

/// Compiles only if `T` is `Send` and `Sync`.
fn assert_send_sync<T: Send + Sync>() {}

/// Compiles only if `T` is `Send`.
fn assert_send<T: Send>() {}

/// Compiles only if `T` implements `Debug`.
fn assert_debug<T: Debug>() {}

/// Compiles only if `T` is an error type that `anyhow` and `Box<dyn Error>`
/// accept: `Error + Send + Sync + 'static`.
fn assert_error<T: std::error::Error + Send + Sync + 'static>() {}

#[test]
fn clients_handles_options_and_results_are_send_and_sync() {
    // Clients and service handles.
    assert_send_sync::<Client>();
    assert_send_sync::<ConnectionConfig>();
    assert_send_sync::<ConnectionConfigBuilder>();
    assert_send_sync::<Sandbox>();
    assert_send_sync::<Commands>();
    assert_send_sync::<Pty>();
    assert_send_sync::<Filesystem>();
    assert_send_sync::<Volume>();
    // Options and parameters.
    assert_send_sync::<CommandOptions>();
    assert_send_sync::<CommandConnectOptions>();
    assert_send_sync::<PtyOptions>();
    assert_send_sync::<PtyConnectOptions>();
    assert_send_sync::<PtySize>();
    assert_send_sync::<StreamInputOptions>();
    assert_send_sync::<FileOptions>();
    assert_send_sync::<ListOptions>();
    assert_send_sync::<WriteOptions>();
    assert_send_sync::<WriteEntry>();
    assert_send_sync::<WatchOptions>();
    assert_send_sync::<FileUrlOptions>();
    assert_send_sync::<CreateSandboxParams>();
    assert_send_sync::<ListSandboxesParams>();
    assert_send_sync::<ConnectOptions>();
    assert_send_sync::<PauseOptions>();
    assert_send_sync::<RequestOptions>();
    assert_send_sync::<Lifecycle>();
    assert_send_sync::<OnTimeout>();
    assert_send_sync::<OnResume>();
    assert_send_sync::<SnapshotMode>();
    assert_send_sync::<MetricsParams>();
    assert_send_sync::<ForkOptions>();
    assert_send_sync::<NetworkConfig>();
    assert_send_sync::<NetworkUpdate>();
    assert_send_sync::<NetworkRule>();
    assert_send_sync::<SnapshotOptions>();
    assert_send_sync::<ListSnapshotsParams>();
    assert_send_sync::<VolumeMount>();
    assert_send_sync::<SortOrder>();
    assert_send_sync::<Signal>();
    // Results and events.
    assert_send_sync::<CommandEvent>();
    assert_send_sync::<CommandExit>();
    assert_send_sync::<CommandResult>();
    assert_send_sync::<ProcessInfo>();
    assert_send_sync::<PtyEvent>();
    assert_send_sync::<EntryInfo>();
    assert_send_sync::<FileType>();
    assert_send_sync::<WriteInfo>();
    assert_send_sync::<FilesystemEvent>();
    assert_send_sync::<FilesystemEventType>();
    assert_send_sync::<SandboxInfo>();
    assert_send_sync::<SandboxPage>();
    assert_send_sync::<SandboxState>();
    assert_send_sync::<SandboxMetrics>();
    assert_send_sync::<LifecycleInfo>();
    assert_send_sync::<TimeoutAction>();
    assert_send_sync::<NetworkInfo>();
    assert_send_sync::<SnapshotInfo>();
    assert_send_sync::<SnapshotPage>();
    assert_send_sync::<VolumeInfo>();
    assert_send_sync::<EnvdVersion>();
    assert_send_sync::<ConnectCode>();
}

#[test]
fn stream_handles_are_send() {
    // A handle is driven by one task at a time (`&mut self`), so `Send` is
    // the promise; it may hold a boxed stream that is not `Sync`.
    assert_send::<CommandHandle>();
    assert_send::<PtyHandle>();
    assert_send::<WatchHandle>();
    assert_send::<FileStream>();
    assert_send::<InputStream>();
}

#[test]
fn errors_work_with_boxed_and_anyhow_errors() {
    assert_error::<Error>();
    assert_error::<TransportError>();
}

#[test]
fn every_public_type_is_debug() {
    assert_debug::<Client>();
    assert_debug::<ConnectionConfig>();
    assert_debug::<ConnectionConfigBuilder>();
    assert_debug::<Sandbox>();
    assert_debug::<Commands>();
    assert_debug::<CommandHandle>();
    assert_debug::<Pty>();
    assert_debug::<PtyHandle>();
    assert_debug::<Filesystem>();
    assert_debug::<WatchHandle>();
    assert_debug::<FileStream>();
    assert_debug::<InputStream>();
    assert_debug::<Volume>();
    assert_debug::<Error>();
    assert_debug::<TransportError>();
    assert_debug::<ConnectCode>();
    assert_debug::<EnvdVersion>();
    assert_debug::<CommandOptions>();
    assert_debug::<CommandConnectOptions>();
    assert_debug::<CommandEvent>();
    assert_debug::<CommandExit>();
    assert_debug::<CommandResult>();
    assert_debug::<ProcessInfo>();
    assert_debug::<Signal>();
    assert_debug::<PtyOptions>();
    assert_debug::<PtyConnectOptions>();
    assert_debug::<PtySize>();
    assert_debug::<PtyEvent>();
    assert_debug::<StreamInputOptions>();
    assert_debug::<FileOptions>();
    assert_debug::<ListOptions>();
    assert_debug::<WriteOptions>();
    assert_debug::<WriteEntry>();
    assert_debug::<WriteInfo>();
    assert_debug::<WatchOptions>();
    assert_debug::<EntryInfo>();
    assert_debug::<FileType>();
    assert_debug::<FilesystemEvent>();
    assert_debug::<FilesystemEventType>();
    assert_debug::<FileUrlOptions>();
    assert_debug::<CreateSandboxParams>();
    assert_debug::<ListSandboxesParams>();
    assert_debug::<SandboxInfo>();
    assert_debug::<SandboxPage>();
    assert_debug::<SandboxState>();
    assert_debug::<SortOrder>();
    assert_debug::<ConnectOptions>();
    assert_debug::<PauseOptions>();
    assert_debug::<RequestOptions>();
    assert_debug::<Lifecycle>();
    assert_debug::<LifecycleInfo>();
    assert_debug::<OnTimeout>();
    assert_debug::<OnResume>();
    assert_debug::<SnapshotMode>();
    assert_debug::<TimeoutAction>();
    assert_debug::<MetricsParams>();
    assert_debug::<SandboxMetrics>();
    assert_debug::<ForkOptions>();
    assert_debug::<NetworkConfig>();
    assert_debug::<NetworkUpdate>();
    assert_debug::<NetworkRule>();
    assert_debug::<NetworkInfo>();
    assert_debug::<SnapshotOptions>();
    assert_debug::<SnapshotInfo>();
    assert_debug::<ListSnapshotsParams>();
    assert_debug::<SnapshotPage>();
    assert_debug::<VolumeInfo>();
    assert_debug::<VolumeMount>();
}

#[test]
fn every_options_type_has_new_equal_to_default() {
    // Arrange-Act-Assert collapses here: each line builds both forms and
    // compares them. C-COMMON-TRAITS: a no-argument `new()` and `Default`
    // must agree, so callers can use either.
    assert_eq!(CommandOptions::new(), CommandOptions::default());
    assert_eq!(
        CommandConnectOptions::new(),
        CommandConnectOptions::default()
    );
    assert_eq!(PtyOptions::new(), PtyOptions::default());
    assert_eq!(PtyConnectOptions::new(), PtyConnectOptions::default());
    assert_eq!(StreamInputOptions::new(), StreamInputOptions::default());
    assert_eq!(FileOptions::new(), FileOptions::default());
    assert_eq!(ListOptions::new(), ListOptions::default());
    assert_eq!(WriteOptions::new(), WriteOptions::default());
    assert_eq!(WatchOptions::new(), WatchOptions::default());
    assert_eq!(FileUrlOptions::new(), FileUrlOptions::default());
    assert_eq!(ListSandboxesParams::new(), ListSandboxesParams::default());
    assert_eq!(ConnectOptions::new(), ConnectOptions::default());
    assert_eq!(PauseOptions::new(), PauseOptions::default());
    assert_eq!(RequestOptions::new(), RequestOptions::default());
    assert_eq!(Lifecycle::new(), Lifecycle::default());
    assert_eq!(MetricsParams::new(), MetricsParams::default());
    assert_eq!(ForkOptions::new(), ForkOptions::default());
    assert_eq!(NetworkConfig::new(), NetworkConfig::default());
    assert_eq!(NetworkUpdate::new(), NetworkUpdate::default());
    assert_eq!(NetworkRule::new(), NetworkRule::default());
    assert_eq!(SnapshotOptions::new(), SnapshotOptions::default());
    assert_eq!(ListSnapshotsParams::new(), ListSnapshotsParams::default());
}
