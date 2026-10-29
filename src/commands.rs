//! Running commands in a sandbox through envd's `process.Process` service.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use bytes::{Bytes, BytesMut};
use reqwest::header::HeaderMap;

use crate::config::{RequestOptions, RequestTimeout, no_limit_if_zero};
use crate::envd::connect::{ConnectStream, Empty};
use crate::envd::process_pb::{
    self, CloseStdinRequest, ConnectRequest, EventResponse, ListRequest, ListResponse,
    ProcessConfig, ProcessInput, ProcessSelector, SendInputRequest, SendSignalRequest,
    StartRequest,
};
use crate::envd::{EnvdClient, EnvdVersion, keepalive_headers};
use crate::error::{ConnectCode, Error, Result};
use crate::stream_input::{InputKind, InputStream, StreamInputOptions};

/// The default bound on a command stream (`timeout=60` in the SDKs'
/// `commands.run`, `e2b/sandbox_async/commands/command.py:218`).
pub const DEFAULT_COMMAND_TIMEOUT: Duration = Duration::from_secs(60);

/// The shell the SDKs run commands with: `/bin/bash -l -c <cmd>`
/// (`e2b/sandbox_async/commands/command.py:323-327`).
const SHELL: &str = "/bin/bash";

/// Options for starting a command.
///
/// # Examples
///
/// ```
/// use std::time::Duration;
/// use e2b_client::CommandOptions;
///
/// let options = CommandOptions::default()
///     .cwd("/tmp")
///     .env("RUST_LOG", "debug")
///     .user("root")
///     .timeout(Some(Duration::from_secs(300)));
/// # let _ = options;
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOptions {
    envs: HashMap<String, String>,
    cwd: Option<String>,
    user: Option<String>,
    stdin: bool,
    timeout: Option<Duration>,
    request_timeout: RequestTimeout,
}

impl Default for CommandOptions {
    fn default() -> Self {
        Self {
            envs: HashMap::new(),
            cwd: None,
            user: None,
            stdin: false,
            timeout: Some(DEFAULT_COMMAND_TIMEOUT),
            request_timeout: RequestTimeout::default(),
        }
    }
}

impl CommandOptions {
    /// The default options: no extra environment, the sandbox's default
    /// user and working directory, no stdin, the
    /// [`DEFAULT_COMMAND_TIMEOUT`] deadline and the configured request
    /// timeout. The same as [`CommandOptions::default`].
    ///
    /// # Returns
    ///
    /// The options.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an environment variable for the command.
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

    /// Sets the working directory.
    ///
    /// # Arguments
    ///
    /// * `cwd` - the directory to run in.
    ///
    /// # Returns
    ///
    /// The options, for chaining.
    #[must_use]
    pub fn cwd(mut self, cwd: impl Into<String>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    /// Runs the command as `user` instead of the sandbox's default user.
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

    /// Keeps the command's stdin open so [`Commands::send_stdin`] can write
    /// to it. Off by default, matching the SDKs.
    ///
    /// # Arguments
    ///
    /// * `stdin` - whether to open stdin.
    ///
    /// # Returns
    ///
    /// The options, for chaining.
    #[must_use]
    pub fn stdin(mut self, stdin: bool) -> Self {
        self.stdin = stdin;
        self
    }

    /// Bounds how long the command's output stream may stay open; `None`, or
    /// a zero duration (the SDK's `timeout=0`), for no limit. Defaults to
    /// [`DEFAULT_COMMAND_TIMEOUT`].
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

    /// Bounds the wait until envd confirms the command started, for this
    /// call only; `None` or zero for no limit. When it is not set, the
    /// configured [`crate::ConnectionConfig::request_timeout`] applies. It
    /// matches the SDKs' `request_timeout`
    /// (`e2b/sandbox_async/commands/command.py:335-344`). After the start
    /// event only [`CommandOptions::timeout`] bounds the stream.
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

/// Options for [`Commands::connect`]: the deadline of the attached stream,
/// and the request timeout for the wait until envd confirms the attach. They
/// match the SDKs' `commands.connect(pid, timeout=60, request_timeout=None)`
/// (`e2b/sandbox_async/commands/command.py:346-350`).
///
/// # Examples
///
/// ```
/// use std::time::Duration;
/// use e2b_client::CommandConnectOptions;
///
/// // Follow a long-running process with no deadline, but give up if envd
/// // does not confirm the attach within 10 s.
/// let options = CommandConnectOptions::new()
///     .timeout(None)
///     .request_timeout(Some(Duration::from_secs(10)));
/// # let _ = options;
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandConnectOptions {
    timeout: Option<Duration>,
    request_timeout: RequestTimeout,
}

impl Default for CommandConnectOptions {
    fn default() -> Self {
        Self {
            timeout: Some(DEFAULT_COMMAND_TIMEOUT),
            request_timeout: RequestTimeout::default(),
        }
    }
}

impl CommandConnectOptions {
    /// The SDKs' defaults: a 60 s stream deadline
    /// ([`DEFAULT_COMMAND_TIMEOUT`]) and the configured request timeout.
    ///
    /// # Returns
    ///
    /// The default options.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Bounds how long the attached stream may stay open; `None` or zero for
    /// no limit (the SDKs' `timeout=0`). Defaults to
    /// [`DEFAULT_COMMAND_TIMEOUT`]. The deadline counts from the attach.
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

/// A signal to send to a process (`process.Signal`).
///
/// `#[non_exhaustive]`: envd's `process.Signal` may gain signals, so a
/// `match` outside this crate needs a `_` arm.
///
/// # Examples
///
/// ```no_run
/// use e2b_client::{CommandOptions, RequestOptions, Sandbox, Signal};
///
/// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
/// let handle = sandbox.commands().start("sleep 600", &CommandOptions::new()).await?;
/// let request = RequestOptions::new();
/// sandbox.commands().send_signal(handle.pid(), Signal::Sigterm, &request).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Signal {
    /// `SIGTERM`: ask the process to exit.
    Sigterm,
    /// `SIGKILL`: kill the process.
    Sigkill,
}

impl Signal {
    fn as_wire(self) -> &'static str {
        match self {
            Self::Sigterm => "SIGNAL_SIGTERM",
            Self::Sigkill => "SIGNAL_SIGKILL",
        }
    }
}

/// One event from a running command.
///
/// `#[non_exhaustive]`: envd's `process.ProcessEvent` may gain event kinds,
/// so a `match` outside this crate needs a catch-all arm (`_` or
/// `other => ...`).
///
/// # Examples
///
/// ```no_run
/// use e2b_client::{CommandEvent, CommandOptions, Sandbox};
///
/// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
/// let mut handle = sandbox.commands().start("make", &CommandOptions::new()).await?;
/// while let Some(event) = handle.next_event().await? {
///     match event {
///         CommandEvent::Stdout(chunk) => print!("{}", String::from_utf8_lossy(&chunk)),
///         CommandEvent::Stderr(chunk) => eprint!("{}", String::from_utf8_lossy(&chunk)),
///         CommandEvent::Exit(exit) => println!("exit {}", exit.exit_code),
///         _ => {}
///     }
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CommandEvent {
    /// A chunk of standard output.
    Stdout(Bytes),
    /// A chunk of standard error.
    Stderr(Bytes),
    /// The command finished. No further events follow.
    Exit(CommandExit),
}

/// How a command ended (`process.ProcessEvent.EndEvent`).
///
/// # Examples
///
/// ```no_run
/// use e2b_client::{CommandEvent, CommandOptions, Sandbox};
///
/// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
/// let mut handle = sandbox.commands().start("false", &CommandOptions::new()).await?;
/// while let Some(event) = handle.next_event().await? {
///     if let CommandEvent::Exit(exit) = event {
///         println!("code {}, error {:?}", exit.exit_code, exit.error);
///     }
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct CommandExit {
    /// The exit code.
    pub exit_code: i32,
    /// envd's error message, if the command failed to run or was killed.
    pub error: Option<String>,
}

/// The collected outcome of a finished command.
///
/// # Examples
///
/// ```no_run
/// # async fn demo(sandbox: e2b_client::Sandbox) -> e2b_client::Result<()> {
/// let result = sandbox.commands().run("ls /", &e2b_client::CommandOptions::new()).await?;
/// if result.success() {
///     print!("{}", result.stdout);
/// } else {
///     eprintln!("exit {}: {}", result.exit_code, result.stderr);
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct CommandResult {
    /// Standard output, decoded as UTF-8 (invalid sequences replaced).
    pub stdout: String,
    /// Standard error, decoded as UTF-8 (invalid sequences replaced).
    pub stderr: String,
    /// The exit code.
    pub exit_code: i32,
    /// envd's error message, if any.
    pub error: Option<String>,
}

impl CommandResult {
    /// Whether the command exited with code 0.
    ///
    /// # Returns
    ///
    /// `true` on success.
    #[must_use]
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }
}

/// A process running in the sandbox, from [`Commands::list`].
///
/// # Examples
///
/// ```no_run
/// # async fn demo(sandbox: e2b_client::Sandbox) -> e2b_client::Result<()> {
/// use e2b_client::RequestOptions;
///
/// for process in sandbox.commands().list(&RequestOptions::new()).await? {
///     println!("{} {} {:?}", process.pid, process.cmd, process.args);
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ProcessInfo {
    /// The process ID.
    pub pid: u32,
    /// The tag it was started with, if any.
    pub tag: Option<String>,
    /// The executable.
    pub cmd: String,
    /// The arguments.
    pub args: Vec<String>,
    /// The environment variables it was started with.
    pub envs: HashMap<String, String>,
    /// The working directory, if one was set.
    pub cwd: Option<String>,
}

/// The process service of one sandbox. Get it from
/// [`crate::Sandbox::commands`].
///
/// # Examples
///
/// ```no_run
/// # async fn demo(sandbox: e2b_client::Sandbox) -> e2b_client::Result<()> {
/// let commands = sandbox.commands();
/// let result = commands.run("echo hi", &e2b_client::CommandOptions::new()).await?;
/// assert_eq!(result.stdout, "hi\n");
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct Commands {
    envd: Arc<EnvdClient>,
}

impl Commands {
    pub(crate) fn new(envd: Arc<EnvdClient>) -> Self {
        Self { envd }
    }

    /// Starts `cmd` with `/bin/bash -l -c` and returns once envd confirms it
    /// started.
    ///
    /// # Arguments
    ///
    /// * `cmd` - the shell command line.
    /// * `options` - environment, working directory, user, stdin and timeout.
    ///
    /// # Returns
    ///
    /// A [`CommandHandle`] to stream output from and wait on.
    ///
    /// # Errors
    ///
    /// [`Error::Rpc`] if envd rejects the command, [`Error::Timeout`] if the
    /// start confirmation does not arrive within the request timeout,
    /// [`Error::Protocol`] if the stream does not open with a start event,
    /// [`Error::Transport`] for connection failures.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use e2b_client::{CommandEvent, CommandOptions, Sandbox};
    ///
    /// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
    /// let mut handle = sandbox.commands().start("for i in 1 2 3; do echo $i; done",
    ///     &CommandOptions::default()).await?;
    /// while let Some(event) = handle.next_event().await? {
    ///     match event {
    ///         CommandEvent::Stdout(chunk) => print!("{}", String::from_utf8_lossy(&chunk)),
    ///         CommandEvent::Stderr(chunk) => eprint!("{}", String::from_utf8_lossy(&chunk)),
    ///         CommandEvent::Exit(exit) => println!("exit code {}", exit.exit_code),
    ///         // `CommandEvent` is `#[non_exhaustive]`, so a `_` arm is required.
    ///         _ => {}
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn start(&self, cmd: &str, options: &CommandOptions) -> Result<CommandHandle> {
        let request = StartRequest {
            process: ProcessConfig {
                cmd: SHELL,
                args: vec!["-l", "-c", cmd],
                envs: &options.envs,
                cwd: options.cwd.as_deref(),
            },
            pty: None,
            tag: None,
            stdin: Some(options.stdin),
        };
        let mut headers = self.envd.user_headers(options.user.as_deref())?;
        headers.extend(keepalive_headers());
        let timeouts = StreamTimeouts {
            stream: options.timeout,
            request: options.request_timeout,
        };
        self.open("Start", &request, headers, timeouts).await
    }

    /// Starts `cmd` and waits for it to finish, collecting its output.
    ///
    /// A non-zero exit code is not an error: inspect
    /// [`CommandResult::exit_code`] or [`CommandResult::success`].
    ///
    /// # Arguments
    ///
    /// * `cmd` - the shell command line.
    /// * `options` - environment, working directory, user, stdin and timeout.
    ///
    /// # Returns
    ///
    /// The command's output and exit code.
    ///
    /// # Errors
    ///
    /// As [`Commands::start`] and [`CommandHandle::wait`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::time::Duration;
    ///
    /// use e2b_client::{CommandOptions, Sandbox};
    ///
    /// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
    /// let options = CommandOptions::new()
    ///     .cwd("/home/user")
    ///     .env("RUST_LOG", "debug")
    ///     .timeout(Some(Duration::from_secs(300)));
    /// let result = sandbox.commands().run("cargo --version", &options).await?;
    /// println!("{}", result.stdout);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn run(&self, cmd: &str, options: &CommandOptions) -> Result<CommandResult> {
        self.start(cmd, options).await?.wait().await
    }

    /// Attaches to a process that is already running.
    ///
    /// # Arguments
    ///
    /// * `pid` - the process ID.
    /// * `options` - the stream deadline (60 s by default, as in the SDKs)
    ///   and the request timeout for the attach.
    ///
    /// # Returns
    ///
    /// A [`CommandHandle`] for the process's remaining output.
    ///
    /// # Errors
    ///
    /// [`Error::Rpc`] with [`ConnectCode::NotFound`] if envd no longer knows
    /// the process, plus the errors of [`Commands::start`]. envd keeps a
    /// process that has exited attachable for a short time (tens of seconds
    /// on hosted E2B; see `docs/protocol-notes.md`): the attach succeeds and
    /// the stream then delivers the process's [`CommandEvent::Exit`] as its
    /// first event, until envd forgets the pid (seen in a probe run on
    /// hosted E2B, 2026-10-28, issue #56).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use e2b_client::{CommandConnectOptions, CommandOptions, Sandbox};
    ///
    /// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
    /// let started = sandbox
    ///     .commands()
    ///     .start("sleep 5; echo done", &CommandOptions::default())
    ///     .await?;
    /// let pid = started.pid();
    /// drop(started); // the process keeps running
    /// let attached = sandbox
    ///     .commands()
    ///     .connect(pid, &CommandConnectOptions::default())
    ///     .await?;
    /// assert_eq!(attached.wait().await?.stdout, "done\n");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn connect(
        &self,
        pid: u32,
        options: &CommandConnectOptions,
    ) -> Result<CommandHandle> {
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

    /// Opens a `Start`/`Connect` stream and wraps it in a [`CommandHandle`].
    async fn open<Req: serde::Serialize>(
        &self,
        method: &'static str,
        request: &Req,
        headers: HeaderMap,
        timeouts: StreamTimeouts,
    ) -> Result<CommandHandle> {
        let (pid, stream) =
            open_process_stream(&self.envd, method, request, headers, timeouts).await?;
        Ok(CommandHandle {
            pid,
            stream,
            commands: self.clone(),
        })
    }

    /// Lists the processes running in the sandbox.
    ///
    /// # Arguments
    ///
    /// * `options` - the request timeout; [`RequestOptions::new`] for the
    ///   configured one.
    ///
    /// # Returns
    ///
    /// One [`ProcessInfo`] per process.
    ///
    /// # Errors
    ///
    /// [`Error::Rpc`], [`Error::Decode`] or [`Error::Transport`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn demo(sandbox: e2b_client::Sandbox) -> e2b_client::Result<()> {
    /// use e2b_client::RequestOptions;
    ///
    /// let running = sandbox.commands().list(&RequestOptions::new()).await?;
    /// println!("{} processes", running.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list(&self, options: &RequestOptions) -> Result<Vec<ProcessInfo>> {
        let response: ListResponse = self
            .envd
            .unary(
                process_pb::SERVICE,
                "List",
                &ListRequest {},
                HeaderMap::new(),
                options.request_timeout,
            )
            .await?;
        Ok(response
            .processes
            .into_iter()
            .map(|process| {
                let config = process.config.unwrap_or_default();
                ProcessInfo {
                    pid: process.pid,
                    tag: process.tag,
                    cmd: config.cmd,
                    args: config.args,
                    envs: config.envs,
                    cwd: config.cwd,
                }
            })
            .collect())
    }

    /// Writes to a process's stdin. The process must have been started with
    /// [`CommandOptions::stdin`] set.
    ///
    /// # Arguments
    ///
    /// * `pid` - the process ID.
    /// * `data` - the bytes to write.
    /// * `options` - the request timeout; [`RequestOptions::new`] for the
    ///   configured one.
    ///
    /// # Errors
    ///
    /// [`Error::Rpc`] (e.g. `not_found`) or [`Error::Transport`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use e2b_client::{CommandOptions, RequestOptions, Sandbox};
    ///
    /// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
    /// let handle = sandbox.commands().start("cat", &CommandOptions::new().stdin(true)).await?;
    /// let request = RequestOptions::new();
    /// sandbox.commands().send_stdin(handle.pid(), "hello\n", &request).await?;
    /// sandbox.commands().close_stdin(handle.pid(), &request).await?;
    /// assert_eq!(handle.wait().await?.stdout, "hello\n");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn send_stdin(
        &self,
        pid: u32,
        data: impl AsRef<[u8]>,
        options: &RequestOptions,
    ) -> Result<()> {
        let request = SendInputRequest {
            process: ProcessSelector::Pid(pid),
            input: ProcessInput::Stdin(data.as_ref()),
        };
        let _: Empty = self
            .envd
            .unary(
                process_pb::SERVICE,
                "SendInput",
                &request,
                HeaderMap::new(),
                options.request_timeout,
            )
            .await?;
        Ok(())
    }

    /// Opens an ordered input stream to a running command's stdin over envd's
    /// client-streaming `StreamInput` RPC. The command must have been started
    /// with [`CommandOptions::stdin`] set.
    ///
    /// Unlike [`send_stdin`](Self::send_stdin), which makes one request per
    /// write, the stream keeps one request open and delivers its chunks in
    /// order. No request is made until a Tokio task picks the call up, so
    /// this returns at once.
    ///
    /// # Arguments
    ///
    /// * `pid` - the process ID.
    /// * `options` - the stream deadline (60 s by default; keepalives do not
    ///   extend it).
    ///
    /// # Returns
    ///
    /// An [`InputStream`]. Errors such as a missing process surface from its
    /// [`send`](InputStream::send) and [`close`](InputStream::close).
    ///
    /// # Panics
    ///
    /// Outside a Tokio runtime, because the call runs on a spawned task.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use e2b_client::{CommandOptions, RequestOptions, Sandbox, StreamInputOptions};
    ///
    /// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
    /// let commands = sandbox.commands();
    /// let handle = commands.start("cat", &CommandOptions::default().stdin(true)).await?;
    /// let mut input = commands.stream_input(handle.pid(), &StreamInputOptions::default());
    /// input.send("line 1\n").await?;
    /// input.send("line 2\n").await?;
    /// input.close().await?;
    /// commands.close_stdin(handle.pid(), &RequestOptions::new()).await?;
    /// let result = handle.wait().await?;
    /// assert_eq!(result.stdout, "line 1\nline 2\n");
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn stream_input(&self, pid: u32, options: &StreamInputOptions) -> InputStream {
        InputStream::open(
            Arc::clone(&self.envd),
            pid,
            InputKind::Stdin,
            options.timeout,
        )
    }

    /// Closes a process's stdin, signalling end of input.
    ///
    /// # Arguments
    ///
    /// * `pid` - the process ID.
    /// * `options` - the request timeout; [`RequestOptions::new`] for the
    ///   configured one.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidArgument`] when the sandbox's envd is older than 0.5.2,
    /// which has no `CloseStdin`; nothing is sent in that case, matching the
    /// SDK (`e2b/sandbox_async/commands/command.py:189-193`). Otherwise
    /// [`Error::Rpc`] (e.g. `not_found`) or [`Error::Transport`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use e2b_client::{CommandOptions, RequestOptions, Sandbox};
    ///
    /// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
    /// let handle = sandbox.commands().start("wc -c", &CommandOptions::new().stdin(true)).await?;
    /// let request = RequestOptions::new();
    /// sandbox.commands().send_stdin(handle.pid(), "abc", &request).await?;
    /// // EOF: `wc` prints its count and exits.
    /// sandbox.commands().close_stdin(handle.pid(), &request).await?;
    /// assert_eq!(handle.wait().await?.stdout.trim(), "3");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn close_stdin(&self, pid: u32, options: &RequestOptions) -> Result<()> {
        let version = self.envd.version();
        if version < EnvdVersion::CLOSE_STDIN {
            return Err(Error::InvalidArgument(format!(
                "close_stdin needs envd {} or newer; this sandbox runs envd {version}",
                EnvdVersion::CLOSE_STDIN
            )));
        }
        let request = CloseStdinRequest {
            process: ProcessSelector::Pid(pid),
        };
        let _: Empty = self
            .envd
            .unary(
                process_pb::SERVICE,
                "CloseStdin",
                &request,
                HeaderMap::new(),
                options.request_timeout,
            )
            .await?;
        Ok(())
    }

    /// Sends a signal to a process.
    ///
    /// # Arguments
    ///
    /// * `pid` - the process ID.
    /// * `signal` - the signal.
    /// * `options` - the request timeout; [`RequestOptions::new`] for the
    ///   configured one.
    ///
    /// # Errors
    ///
    /// [`Error::Rpc`] (e.g. `not_found`) or [`Error::Transport`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use e2b_client::{CommandOptions, RequestOptions, Sandbox, Signal};
    ///
    /// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
    /// let handle = sandbox.commands().start("sleep 600", &CommandOptions::new()).await?;
    /// let request = RequestOptions::new();
    /// sandbox.commands().send_signal(handle.pid(), Signal::Sigterm, &request).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn send_signal(
        &self,
        pid: u32,
        signal: Signal,
        options: &RequestOptions,
    ) -> Result<()> {
        let request = SendSignalRequest {
            process: ProcessSelector::Pid(pid),
            signal: signal.as_wire(),
        };
        let _: Empty = self
            .envd
            .unary(
                process_pb::SERVICE,
                "SendSignal",
                &request,
                HeaderMap::new(),
                options.request_timeout,
            )
            .await?;
        Ok(())
    }

    /// Kills a process with `SIGKILL`.
    ///
    /// # Arguments
    ///
    /// * `pid` - the process ID.
    /// * `options` - the request timeout; [`RequestOptions::new`] for the
    ///   configured one.
    ///
    /// # Returns
    ///
    /// `true` if it was killed, `false` if no such process existed
    /// (`e2b/envd/rpc.py:215-231`).
    ///
    /// # Errors
    ///
    /// [`Error::Rpc`] other than `not_found`, or [`Error::Transport`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use e2b_client::{CommandOptions, RequestOptions, Sandbox};
    ///
    /// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
    /// let handle = sandbox.commands().start("sleep 600", &CommandOptions::new()).await?;
    /// assert!(sandbox.commands().kill(handle.pid(), &RequestOptions::new()).await?);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn kill(&self, pid: u32, options: &RequestOptions) -> Result<bool> {
        match self.send_signal(pid, Signal::Sigkill, options).await {
            Ok(()) => Ok(true),
            Err(Error::Rpc {
                code: ConnectCode::NotFound,
                ..
            }) => Ok(false),
            Err(err) => Err(err),
        }
    }
}

/// The two bounds on a process stream: its own deadline, and the request
/// timeout for the wait until envd confirms it with a start event. They are
/// kept together so `open_process_stream` stays within five parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StreamTimeouts {
    /// The stream deadline (`Connect-Timeout-Ms`), or `None` for no limit.
    pub(crate) stream: Option<Duration>,
    /// The bound on opening the stream: the call's override, or
    /// `RequestTimeout::default()` for the configured one. `EnvdClient`
    /// resolves it (`timeout_for`).
    pub(crate) request: RequestTimeout,
}

/// Opens a `Start`/`Connect` stream and waits, within the request timeout,
/// for the start event that carries the PID
/// (`e2b/sandbox_async/commands/command.py:382-411`). Commands and PTY
/// sessions both open their streams here.
///
/// # Arguments
///
/// * `envd` - the sandbox's envd transport.
/// * `method` - `"Start"` or `"Connect"`.
/// * `request` - the request message.
/// * `headers` - extra headers (user, keepalive).
/// * `timeouts` - the stream deadline and the bound on opening the stream.
///
/// # Returns
///
/// The PID and the open stream, positioned after the start event.
///
/// # Errors
///
/// [`Error::Timeout`] if the start event does not arrive within the request
/// timeout, [`Error::Protocol`] if the stream opens with another event, plus
/// the errors of the stream call itself.
pub(crate) async fn open_process_stream<Req: serde::Serialize>(
    envd: &EnvdClient,
    method: &'static str,
    request: &Req,
    headers: HeaderMap,
    timeouts: StreamTimeouts,
) -> Result<(u32, ConnectStream<EventResponse>)> {
    // The async block's error type is inferred from `within_request_timeout`,
    // whose argument must yield the crate's `Result`.
    envd.within_request_timeout(timeouts.request, async {
        let mut stream: ConnectStream<EventResponse> = envd
            .server_stream(
                process_pb::SERVICE,
                method,
                request,
                headers,
                timeouts.stream,
            )
            .await?;
        let pid = stream
            .next()
            .await?
            .and_then(|response| response.event)
            .and_then(|event| event.start)
            .map(|start| start.pid)
            .ok_or_else(|| {
                Error::Protocol(format!("{method} stream did not open with a start event"))
            })?;
        Ok((pid, stream))
    })
    .await
}

/// A started command. Read its output with [`next_event`](Self::next_event)
/// or collect everything with [`wait`](Self::wait).
///
/// # Examples
///
/// ```no_run
/// # async fn demo(sandbox: e2b_client::Sandbox) -> e2b_client::Result<()> {
/// let handle = sandbox
///     .commands()
///     .start("sleep 1; echo ok", &e2b_client::CommandOptions::new())
///     .await?;
/// println!("pid {}", handle.pid());
/// let result = handle.wait().await?;
/// assert_eq!(result.stdout, "ok\n");
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct CommandHandle {
    pid: u32,
    stream: ConnectStream<EventResponse>,
    commands: Commands,
}

impl CommandHandle {
    /// The process ID.
    ///
    /// # Returns
    ///
    /// The PID.
    #[must_use]
    pub fn pid(&self) -> u32 {
        self.pid
    }

    /// Waits for the next stdout/stderr chunk or the exit event. Keepalive
    /// events, and PTY output (which [`crate::PtyHandle`] reads), are skipped.
    ///
    /// # Returns
    ///
    /// `Some(event)` for each event, then `None` once envd cleanly ends the
    /// stream. [`CommandEvent::Exit`] normally comes last, but a clean end
    /// without one also gives `None`; [`CommandHandle::wait`] reports that
    /// case as [`Error::Protocol`].
    ///
    /// # Errors
    ///
    /// [`Error::Rpc`] if envd ends the stream with an error,
    /// [`Error::Protocol`] for a compressed or truncated envelope, or a body
    /// that ends without an end-of-stream envelope,
    /// [`Error::Timeout`] when the command timeout passes,
    /// [`Error::Transport`] for connection failures.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use e2b_client::{CommandEvent, CommandOptions, Sandbox};
    ///
    /// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
    /// let mut handle = sandbox.commands().start("seq 3", &CommandOptions::new()).await?;
    /// while let Some(event) = handle.next_event().await? {
    ///     if let CommandEvent::Stdout(chunk) = event {
    ///         print!("{}", String::from_utf8_lossy(&chunk));
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn next_event(&mut self) -> Result<Option<CommandEvent>> {
        while let Some(response) = self.stream.next().await? {
            let Some(event) = response.event else {
                continue;
            };
            if let Some(data) = event.data {
                if let Some(chunk) = data.stdout {
                    return Ok(Some(CommandEvent::Stdout(chunk)));
                }
                if let Some(chunk) = data.stderr {
                    return Ok(Some(CommandEvent::Stderr(chunk)));
                }
            } else if let Some(end) = event.end {
                return Ok(Some(CommandEvent::Exit(CommandExit {
                    exit_code: end.exit_code,
                    error: end.error,
                })));
            }
        }
        Ok(None)
    }

    /// Consumes the remaining events and returns the collected result.
    ///
    /// Output already taken with [`next_event`](Self::next_event) is not
    /// included.
    ///
    /// # Returns
    ///
    /// The output and exit code.
    ///
    /// # Errors
    ///
    /// [`Error::Protocol`] if the stream ends without an exit event
    /// (`e2b/sandbox_async/commands/command_handle.py:252-253`), plus the errors
    /// of [`next_event`](Self::next_event).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn demo(sandbox: e2b_client::Sandbox) -> e2b_client::Result<()> {
    /// let handle = sandbox
    ///     .commands()
    ///     .start("echo out; echo err >&2", &e2b_client::CommandOptions::new())
    ///     .await?;
    /// let result = handle.wait().await?;
    /// assert_eq!((result.stdout.as_str(), result.stderr.as_str()), ("out\n", "err\n"));
    /// # Ok(())
    /// # }
    /// ```
    pub async fn wait(mut self) -> Result<CommandResult> {
        let mut stdout = BytesMut::new();
        let mut stderr = BytesMut::new();
        while let Some(event) = self.next_event().await? {
            match event {
                CommandEvent::Stdout(chunk) => stdout.extend_from_slice(&chunk),
                CommandEvent::Stderr(chunk) => stderr.extend_from_slice(&chunk),
                CommandEvent::Exit(exit) => {
                    return Ok(CommandResult {
                        stdout: String::from_utf8_lossy(&stdout).into_owned(),
                        stderr: String::from_utf8_lossy(&stderr).into_owned(),
                        exit_code: exit.exit_code,
                        error: exit.error,
                    });
                }
            }
        }
        Err(Error::Protocol(
            "command stream ended without an end event".into(),
        ))
    }

    /// Writes to the command's stdin. See [`Commands::send_stdin`].
    ///
    /// # Arguments
    ///
    /// * `data` - the bytes to write.
    /// * `options` - the request timeout; [`RequestOptions::new`] for the
    ///   configured one.
    ///
    /// # Errors
    ///
    /// As [`Commands::send_stdin`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use e2b_client::{CommandOptions, RequestOptions, Sandbox};
    ///
    /// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
    /// let options = CommandOptions::new().stdin(true);
    /// let handle = sandbox.commands().start("head -n1", &options).await?;
    /// handle.send_stdin("first line\n", &RequestOptions::new()).await?;
    /// assert_eq!(handle.wait().await?.stdout, "first line\n");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn send_stdin(&self, data: impl AsRef<[u8]>, options: &RequestOptions) -> Result<()> {
        self.commands.send_stdin(self.pid, data, options).await
    }

    /// Kills the command with `SIGKILL`. See [`Commands::kill`].
    ///
    /// # Arguments
    ///
    /// * `options` - the request timeout; [`RequestOptions::new`] for the
    ///   configured one.
    ///
    /// # Returns
    ///
    /// `true` if it was killed, `false` if it had already exited.
    ///
    /// # Errors
    ///
    /// As [`Commands::kill`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn demo(sandbox: e2b_client::Sandbox) -> e2b_client::Result<()> {
    /// use e2b_client::{CommandOptions, RequestOptions};
    ///
    /// let handle = sandbox.commands().start("sleep 600", &CommandOptions::new()).await?;
    /// assert!(handle.kill(&RequestOptions::new()).await?);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn kill(&self, options: &RequestOptions) -> Result<bool> {
        self.commands.kill(self.pid, options).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connect_options_default_to_the_sdk_deadline() {
        assert_eq!(
            CommandConnectOptions::default().timeout,
            Some(DEFAULT_COMMAND_TIMEOUT)
        );
        assert_eq!(
            CommandConnectOptions::new(),
            CommandConnectOptions::default()
        );
        assert_eq!(
            CommandConnectOptions::default().request_timeout,
            RequestTimeout::default(),
            "unset: the configured request timeout applies"
        );
    }

    #[test]
    fn a_zero_connect_deadline_means_no_limit() {
        let options = CommandConnectOptions::new().timeout(Some(Duration::ZERO));

        assert_eq!(options.timeout, None);
    }
}
