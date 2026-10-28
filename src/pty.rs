//! Pseudo-terminal (PTY) sessions in a sandbox, over envd's `process.Process`
//! service (`e2b/sandbox_async/commands/pty.py`).

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use reqwest::header::HeaderMap;

use crate::commands::{CommandExit, Commands, StreamTimeouts, open_process_stream};
use crate::config::{RequestTimeout, no_limit_if_zero};
use crate::envd::connect::{ConnectStream, Empty};
use crate::envd::process_pb::{
    self, ConnectRequest, EventResponse, ProcessConfig, ProcessInput, ProcessSelector, PtyConfig,
    PtySizeWire, SendInputRequest, StartRequest, UpdateRequest,
};
use crate::envd::{EnvdClient, keepalive_headers};
use crate::error::{Error, Result};
use crate::stream_input::{InputKind, InputStream, StreamInputOptions};

/// The SDKs' bound on a PTY stream (`timeout=60`,
/// `e2b/sandbox_async/commands/pty.py:146, 211`). This crate does not apply
/// it by default (a deadline ends the shell, see [`PtyOptions::timeout`]);
/// pass `Some(DEFAULT_PTY_TIMEOUT)` to [`PtyOptions::timeout`] or
/// [`PtyConnectOptions::timeout`] for the SDKs' behaviour.
pub const DEFAULT_PTY_TIMEOUT: Duration = Duration::from_secs(60);

/// The shell a PTY session runs: an interactive login bash
/// (`e2b/sandbox_async/commands/pty.py:169-172`).
const PTY_SHELL: &str = "/bin/bash";
const PTY_SHELL_ARGS: [&str; 2] = ["-i", "-l"];

/// Terminal environment the SDKs set unless the caller did
/// (`e2b/sandbox_async/commands/pty.py:162-165`).
const TERMINAL_ENV_DEFAULTS: [(&str, &str); 3] = [
    ("TERM", "xterm-256color"),
    ("LANG", "C.UTF-8"),
    ("LC_ALL", "C.UTF-8"),
];

/// A terminal size in character cells.
///
/// Deliberately exhaustive with public fields: a size is two numbers and
/// will not grow, so callers may build it as a literal or destructure it.
///
/// # Examples
///
/// ```
/// use e2b_client::PtySize;
///
/// let size = PtySize::new(80, 24);
/// assert_eq!((size.cols, size.rows), (80, 24));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PtySize {
    /// Columns (the terminal width).
    pub cols: u16,
    /// Rows (the terminal height).
    pub rows: u16,
}

impl PtySize {
    /// Creates a size.
    ///
    /// # Arguments
    ///
    /// * `cols` - the terminal width in columns.
    /// * `rows` - the terminal height in rows.
    ///
    /// # Returns
    ///
    /// The size.
    #[must_use]
    pub const fn new(cols: u16, rows: u16) -> Self {
        Self { cols, rows }
    }

    /// The `process.PTY` message for this size. `u16::into` widens to the
    /// proto's `uint32` losslessly.
    fn wire(self) -> PtyConfig {
        PtyConfig {
            size: PtySizeWire {
                cols: self.cols.into(),
                rows: self.rows.into(),
            },
        }
    }
}

/// Options for [`Pty::create`].
///
/// `TERM=xterm-256color`, `LANG=C.UTF-8` and `LC_ALL=C.UTF-8` are added to the
/// environment unless set here, as in the SDKs.
///
/// # Examples
///
/// ```
/// use std::time::Duration;
/// use e2b_client::PtyOptions;
///
/// let options = PtyOptions::default()
///     .cwd("/tmp")
///     .env("LANG", "en_US.UTF-8")
///     .user("root")
///     .timeout(Some(Duration::from_secs(3600)));
/// # let _ = options;
/// ```
// `Default` is derived because every field's own default is the option's
// default (no deadline, #55): an empty map, `None`s and the unset
// `RequestTimeout`, which means the configured request timeout.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PtyOptions {
    envs: HashMap<String, String>,
    cwd: Option<String>,
    user: Option<String>,
    timeout: Option<Duration>,
    request_timeout: RequestTimeout,
}

impl PtyOptions {
    /// The default options: no extra environment, the sandbox's default
    /// user and working directory, no deadline (#55) and the configured
    /// request timeout. The same as [`PtyOptions::default`].
    ///
    /// # Returns
    ///
    /// The options.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an environment variable for the shell.
    ///
    /// # Arguments
    ///
    /// * `key`, `value` - the variable.
    ///
    /// # Returns
    ///
    /// The options, for chaining.
    #[must_use]
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.envs.insert(key.into(), value.into());
        self
    }

    /// Sets the shell's working directory.
    ///
    /// # Arguments
    ///
    /// * `cwd` - the directory to start in.
    ///
    /// # Returns
    ///
    /// The options, for chaining.
    #[must_use]
    pub fn cwd(mut self, cwd: impl Into<String>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    /// Runs the shell as `user` instead of the sandbox's default user.
    ///
    /// # Arguments
    ///
    /// * `user` - the user name, e.g. `"root"`.
    ///
    /// # Returns
    ///
    /// The options, for chaining.
    #[must_use]
    pub fn user(mut self, user: impl Into<String>) -> Self {
        self.user = Some(user.into());
        self
    }

    /// Bounds the whole session; `None` or zero for no limit, which is the
    /// default. This differs from the SDKs, which default to 60 s
    /// ([`DEFAULT_PTY_TIMEOUT`]).
    ///
    /// A deadline, if you set one, counts from when the session opens, not
    /// from the last activity. When it passes, the output stream ends with
    /// the deadline error from [`PtyHandle::next_event`], even while the
    /// shell is in use. envd then ends the shell too, so it cannot be
    /// reattached with [`Pty::connect`] (observed on hosted E2B,
    /// 2026-10-27). Without a deadline, the session lives until the shell
    /// exits, you kill it, or the sandbox ends.
    ///
    /// # Arguments
    ///
    /// * `timeout` - the stream deadline.
    ///
    /// # Returns
    ///
    /// The options, for chaining.
    #[must_use]
    pub fn timeout(mut self, timeout: Option<Duration>) -> Self {
        self.timeout = no_limit_if_zero(timeout);
        self
    }

    /// Bounds the wait until envd confirms the shell started, for this call
    /// only; `None` or zero for no limit. When it is not set, the configured
    /// [`crate::ConnectionConfig::request_timeout`] applies. It matches the
    /// SDKs' `request_timeout` (`e2b/sandbox_async/commands/pty.py:139-147`).
    ///
    /// # Arguments
    ///
    /// * `timeout` - the bound on opening the stream.
    ///
    /// # Returns
    ///
    /// The options, for chaining.
    #[must_use]
    pub fn request_timeout(mut self, timeout: Option<Duration>) -> Self {
        self.request_timeout = RequestTimeout::set(timeout);
        self
    }

    /// The caller's environment plus the terminal defaults it did not set.
    fn envs_with_terminal_defaults(&self) -> HashMap<String, String> {
        let mut envs = self.envs.clone();
        for (key, value) in TERMINAL_ENV_DEFAULTS {
            // `entry().or_insert_with()` is Python's `dict.setdefault`: it
            // only inserts when the key is absent, and the closure builds the
            // `String` only in that case.
            envs.entry(key.to_owned())
                .or_insert_with(|| value.to_owned());
        }
        envs
    }
}

/// One event from a PTY session.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PtyEvent {
    /// Terminal output: stdout and stderr interleaved, with the terminal's
    /// control sequences.
    Data(Bytes),
    /// The shell exited. No further events follow.
    Exit(CommandExit),
}

/// Options for [`Pty::connect`]: the deadline of the reattached session, and
/// the request timeout for the wait until envd confirms the attach. They match
/// the SDKs' `pty.connect(pid, timeout=60, request_timeout=None)`
/// (`e2b/sandbox_async/commands/pty.py:207-212`), except that no deadline is
/// set by default (see [`PtyOptions::timeout`]).
///
/// # Examples
///
/// ```
/// use std::time::Duration;
/// use e2b_client::PtyConnectOptions;
///
/// let options = PtyConnectOptions::new()
///     .timeout(Some(Duration::from_secs(600)))
///     .request_timeout(Some(Duration::from_secs(10)));
/// # let _ = options;
/// ```
// Derived for the same reason as `PtyOptions`: no deadline and the
// configured request timeout are each field's own default.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PtyConnectOptions {
    timeout: Option<Duration>,
    request_timeout: RequestTimeout,
}

impl PtyConnectOptions {
    /// The default options: no stream deadline (the SDKs use 60 s,
    /// [`DEFAULT_PTY_TIMEOUT`]) and the configured request timeout.
    ///
    /// # Returns
    ///
    /// The default options.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Bounds the reattached session; `None` or zero for no limit. The
    /// deadline counts from the attach. No deadline by default; see
    /// [`PtyOptions::timeout`] for what a deadline does to the shell.
    ///
    /// # Arguments
    ///
    /// * `timeout` - the stream deadline.
    ///
    /// # Returns
    ///
    /// The options, for chaining.
    #[must_use]
    pub fn timeout(mut self, timeout: Option<Duration>) -> Self {
        self.timeout = no_limit_if_zero(timeout);
        self
    }

    /// Bounds the wait until envd confirms the attach, for this call only;
    /// `None` or zero for no limit. When it is not set, the configured
    /// [`crate::ConnectionConfig::request_timeout`] applies.
    ///
    /// # Arguments
    ///
    /// * `timeout` - the bound on opening the stream.
    ///
    /// # Returns
    ///
    /// The options, for chaining.
    #[must_use]
    pub fn request_timeout(mut self, timeout: Option<Duration>) -> Self {
        self.request_timeout = RequestTimeout::set(timeout);
        self
    }
}

/// The PTY service of one sandbox. Get it from [`crate::Sandbox::pty`].
///
/// A PTY session is an interactive login `bash` attached to a pseudo-terminal.
/// Its output is one stream of terminal bytes ([`PtyEvent::Data`]) rather
/// than separate stdout and stderr.
#[derive(Debug, Clone)]
pub struct Pty {
    envd: Arc<EnvdClient>,
}

impl Pty {
    pub(crate) fn new(envd: Arc<EnvdClient>) -> Self {
        Self { envd }
    }

    /// Starts an interactive login shell (`/bin/bash -i -l`) on a new PTY and
    /// returns once envd confirms it started.
    ///
    /// # Arguments
    ///
    /// * `size` - the initial terminal size.
    /// * `options` - environment, working directory, user, the session
    ///   deadline (none by default) and the request timeout.
    ///
    /// # Returns
    ///
    /// A [`PtyHandle`] to read terminal output from.
    ///
    /// # Errors
    ///
    /// [`Error::Rpc`] if envd rejects the request, [`Error::Timeout`] if the
    /// start confirmation does not arrive within the request timeout,
    /// [`Error::Protocol`] if the stream does not open with a start event,
    /// [`Error::InvalidArgument`] for a user name that is not a valid header,
    /// [`Error::Transport`] for connection failures.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use e2b_client::{PtyEvent, PtyOptions, PtySize, Sandbox};
    ///
    /// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
    /// let pty = sandbox.pty();
    /// let mut handle = pty.create(PtySize::new(80, 24), &PtyOptions::default()).await?;
    /// handle.send_stdin("echo hello; exit\n").await?;
    /// while let Some(event) = handle.next_event().await? {
    ///     match event {
    ///         PtyEvent::Data(chunk) => print!("{}", String::from_utf8_lossy(&chunk)),
    ///         PtyEvent::Exit(exit) => println!("exit code {}", exit.exit_code),
    ///         // `PtyEvent` is `#[non_exhaustive]`: a later version may add
    ///         // events, so a match outside the crate needs a catch-all arm.
    ///         _ => {}
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create(&self, size: PtySize, options: &PtyOptions) -> Result<PtyHandle> {
        let envs = options.envs_with_terminal_defaults();
        let request = StartRequest {
            process: ProcessConfig {
                cmd: PTY_SHELL,
                args: PTY_SHELL_ARGS.to_vec(),
                envs: &envs,
                cwd: options.cwd.as_deref(),
            },
            pty: Some(size.wire()),
            tag: None,
            stdin: None,
        };
        let mut headers = self.envd.user_headers(options.user.as_deref())?;
        headers.extend(keepalive_headers());
        let timeouts = StreamTimeouts {
            stream: options.timeout,
            request: options.request_timeout,
        };
        self.open("Start", &request, headers, timeouts).await
    }

    /// Attaches to a PTY session that is already running.
    ///
    /// # Arguments
    ///
    /// * `pid` - the shell's process ID.
    /// * `options` - the stream deadline and the request timeout for the
    ///   attach.
    ///
    /// # Returns
    ///
    /// A [`PtyHandle`] for the session's remaining output.
    ///
    /// # Errors
    ///
    /// [`Error::Rpc`] with [`crate::ConnectCode::NotFound`] if envd no longer
    /// knows the process, plus the errors of [`Pty::create`]. envd keeps a
    /// shell that has exited attachable for a short time (tens of seconds on
    /// hosted E2B; see `docs/protocol-notes.md`): the attach succeeds and
    /// the stream then delivers the shell's [`PtyEvent::Exit`] as its first
    /// event, as with the Python SDK's `pty.connect` (issue #56).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use e2b_client::{PtyConnectOptions, PtyOptions, PtySize, Sandbox};
    ///
    /// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
    /// let first = sandbox
    ///     .pty()
    ///     .create(PtySize::new(80, 24), &PtyOptions::default())
    ///     .await?;
    /// let pid = first.pid();
    /// drop(first); // the shell keeps running
    /// let mut again = sandbox
    ///     .pty()
    ///     .connect(pid, &PtyConnectOptions::default())
    ///     .await?;
    /// again.send_stdin("exit\n").await?;
    /// let exit = again.wait().await?;
    /// assert_eq!(exit.exit_code, 0);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn connect(&self, pid: u32, options: &PtyConnectOptions) -> Result<PtyHandle> {
        let request = ConnectRequest {
            process: ProcessSelector::Pid(pid),
        };
        let timeouts = StreamTimeouts {
            stream: options.timeout,
            request: options.request_timeout,
        };
        self.open("Connect", &request, keepalive_headers(), timeouts)
            .await
    }

    /// Opens a `Start`/`Connect` stream and wraps it in a [`PtyHandle`].
    async fn open<Req: serde::Serialize>(
        &self,
        method: &'static str,
        request: &Req,
        headers: HeaderMap,
        timeouts: StreamTimeouts,
    ) -> Result<PtyHandle> {
        let (pid, stream) =
            open_process_stream(&self.envd, method, request, headers, timeouts).await?;
        Ok(PtyHandle {
            pid,
            stream,
            pty: self.clone(),
        })
    }

    /// Writes to a PTY's input, as if typed at the terminal. End a command
    /// line with `"\n"` (or `"\r"`) to run it.
    ///
    /// # Arguments
    ///
    /// * `pid` - the shell's process ID.
    /// * `data` - the bytes to write.
    ///
    /// # Errors
    ///
    /// [`Error::Rpc`] (e.g. `not_found`) or [`Error::Transport`].
    pub async fn send_stdin(&self, pid: u32, data: impl AsRef<[u8]>) -> Result<()> {
        let request = SendInputRequest {
            process: ProcessSelector::Pid(pid),
            input: ProcessInput::Pty(data.as_ref()),
        };
        let _: Empty = self
            .envd
            .unary(
                process_pb::SERVICE,
                "SendInput",
                &request,
                HeaderMap::new(),
                RequestTimeout::default(),
            )
            .await?;
        Ok(())
    }

    /// Changes a PTY's terminal size (envd's `Update` RPC). Call it when the
    /// local terminal is resized.
    ///
    /// # Arguments
    ///
    /// * `pid` - the shell's process ID.
    /// * `size` - the new size.
    ///
    /// # Errors
    ///
    /// [`Error::Rpc`] (e.g. `not_found`) or [`Error::Transport`].
    pub async fn resize(&self, pid: u32, size: PtySize) -> Result<()> {
        let request = UpdateRequest {
            process: ProcessSelector::Pid(pid),
            pty: size.wire(),
        };
        let _: Empty = self
            .envd
            .unary(
                process_pb::SERVICE,
                "Update",
                &request,
                HeaderMap::new(),
                RequestTimeout::default(),
            )
            .await?;
        Ok(())
    }

    /// Kills a PTY session's shell with `SIGKILL`.
    ///
    /// # Arguments
    ///
    /// * `pid` - the shell's process ID.
    ///
    /// # Returns
    ///
    /// `true` if it was killed, `false` if no such process existed.
    ///
    /// # Errors
    ///
    /// As [`Commands::kill`].
    pub async fn kill(&self, pid: u32) -> Result<bool> {
        Commands::new(Arc::clone(&self.envd)).kill(pid).await
    }

    /// Opens an ordered input stream to a PTY over envd's client-streaming
    /// `StreamInput` RPC, as [`Commands::stream_input`] does for stdin.
    ///
    /// # Arguments
    ///
    /// * `pid` - the shell's process ID.
    /// * `options` - the stream deadline (60 s by default).
    ///
    /// # Returns
    ///
    /// An [`InputStream`] whose chunks go to the PTY's input. Errors such as
    /// a missing process surface from its [`send`](InputStream::send) and
    /// [`close`](InputStream::close).
    ///
    /// # Panics
    ///
    /// Outside a Tokio runtime, because the call runs on a spawned task.
    #[must_use]
    pub fn stream_input(&self, pid: u32, options: &StreamInputOptions) -> InputStream {
        InputStream::open(Arc::clone(&self.envd), pid, InputKind::Pty, options.timeout)
    }
}

/// A running PTY session. Read its output with
/// [`next_event`](Self::next_event).
#[derive(Debug)]
pub struct PtyHandle {
    pid: u32,
    stream: ConnectStream<EventResponse>,
    pty: Pty,
}

impl PtyHandle {
    /// The shell's process ID.
    ///
    /// # Returns
    ///
    /// The PID.
    #[must_use]
    pub fn pid(&self) -> u32 {
        self.pid
    }

    /// Waits for the next chunk of terminal output or the exit event.
    /// Keepalive events are skipped.
    ///
    /// # Returns
    ///
    /// `Some(event)` for each event, then `None` once envd cleanly ends the
    /// stream. [`PtyEvent::Exit`] normally comes last.
    ///
    /// # Errors
    ///
    /// As [`crate::CommandHandle::next_event`].
    pub async fn next_event(&mut self) -> Result<Option<PtyEvent>> {
        while let Some(response) = self.stream.next().await? {
            let Some(event) = response.event else {
                continue;
            };
            if let Some(chunk) = event.data.and_then(|data| data.pty) {
                return Ok(Some(PtyEvent::Data(chunk)));
            }
            if let Some(end) = event.end {
                return Ok(Some(PtyEvent::Exit(CommandExit {
                    exit_code: end.exit_code,
                    error: end.error,
                })));
            }
        }
        Ok(None)
    }

    /// Discards the remaining output and waits for the shell to exit.
    ///
    /// # Returns
    ///
    /// How the shell exited.
    ///
    /// # Errors
    ///
    /// [`Error::Protocol`] if the stream ends without an exit event, plus the
    /// errors of [`next_event`](Self::next_event).
    pub async fn wait(mut self) -> Result<CommandExit> {
        while let Some(event) = self.next_event().await? {
            if let PtyEvent::Exit(exit) = event {
                return Ok(exit);
            }
        }
        Err(Error::Protocol(
            "PTY stream ended without an end event".into(),
        ))
    }

    /// Writes to the session's input. See [`Pty::send_stdin`].
    ///
    /// # Arguments
    ///
    /// * `data` - the bytes to write.
    ///
    /// # Errors
    ///
    /// As [`Pty::send_stdin`].
    pub async fn send_stdin(&self, data: impl AsRef<[u8]>) -> Result<()> {
        self.pty.send_stdin(self.pid, data).await
    }

    /// Changes the session's terminal size. See [`Pty::resize`].
    ///
    /// # Arguments
    ///
    /// * `size` - the new size.
    ///
    /// # Errors
    ///
    /// As [`Pty::resize`].
    pub async fn resize(&self, size: PtySize) -> Result<()> {
        self.pty.resize(self.pid, size).await
    }

    /// Kills the session's shell with `SIGKILL`. See [`Pty::kill`].
    ///
    /// # Returns
    ///
    /// `true` if it was killed, `false` if it had already exited.
    ///
    /// # Errors
    ///
    /// As [`Pty::kill`].
    pub async fn kill(&self) -> Result<bool> {
        self.pty.kill(self.pid).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_defaults_fill_only_unset_keys() {
        let options = PtyOptions::default().env("LANG", "en_US.UTF-8");

        let envs = options.envs_with_terminal_defaults();

        assert_eq!(envs.len(), 3);
        assert_eq!(envs["TERM"], "xterm-256color");
        assert_eq!(envs["LANG"], "en_US.UTF-8");
        assert_eq!(envs["LC_ALL"], "C.UTF-8");
    }

    #[test]
    fn options_default_to_no_deadline() {
        assert_eq!(PtyOptions::default().timeout, None);
        assert_eq!(PtyConnectOptions::default().timeout, None);
        assert_eq!(PtyConnectOptions::new(), PtyConnectOptions::default());
        assert_eq!(
            PtyConnectOptions::default().request_timeout,
            RequestTimeout::default(),
            "unset: the configured request timeout applies"
        );
        // The SDKs' value stays available for callers who want parity.
        assert_eq!(DEFAULT_PTY_TIMEOUT, Duration::from_secs(60));
    }

    #[test]
    fn size_widens_to_the_wire_message() {
        let wire = PtySize::new(u16::MAX, 24).wire();

        assert_eq!((wire.size.cols, wire.size.rows), (u32::from(u16::MAX), 24));
    }
}
