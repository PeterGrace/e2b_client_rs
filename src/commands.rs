//! Running commands in a sandbox through envd's `process.Process` service.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use bytes::{Bytes, BytesMut};
use reqwest::header::{HeaderMap, HeaderValue};

use crate::envd::connect::{ConnectStream, Empty};
use crate::envd::process_pb::{
    self, CloseStdinRequest, ConnectRequest, EventResponse, ListRequest, ListResponse,
    ProcessConfig, ProcessInput, ProcessSelector, SendInputRequest, SendSignalRequest,
    StartRequest,
};
use crate::envd::{EnvdClient, EnvdVersion};
use crate::error::{ConnectCode, Error, Result};

/// The default bound on a command stream (`timeout=60` in the SDKs'
/// `commands.run`, `e2b/sandbox_async/commands/command.py:218`).
pub const DEFAULT_COMMAND_TIMEOUT: Duration = Duration::from_secs(60);

/// Asks envd to send keepalive events on long streams so idle proxies do
/// not drop them (`e2b/connection_config.py:48-49`).
const KEEPALIVE_PING_HEADER: &str = "keepalive-ping-interval";
const KEEPALIVE_PING_INTERVAL_SECS: u16 = 50;

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
}

impl Default for CommandOptions {
    fn default() -> Self {
        Self {
            envs: HashMap::new(),
            cwd: None,
            user: None,
            stdin: false,
            timeout: Some(DEFAULT_COMMAND_TIMEOUT),
        }
    }
}

impl CommandOptions {
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

    /// Bounds how long the command's output stream may stay open; `None` for
    /// no limit. Defaults to [`DEFAULT_COMMAND_TIMEOUT`].
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
        self.timeout = timeout;
        self
    }
}

/// A signal to send to a process (`process.Signal`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandEvent {
    /// A chunk of standard output.
    Stdout(Bytes),
    /// A chunk of standard error.
    Stderr(Bytes),
    /// The command finished. No further events follow.
    Exit(CommandExit),
}

/// How a command ended (`process.ProcessEvent.EndEvent`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct CommandExit {
    /// The exit code.
    pub exit_code: i32,
    /// envd's error message, if the command failed to run or was killed.
    pub error: Option<String>,
}

/// The collected outcome of a finished command.
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
            tag: None,
            stdin: options.stdin,
        };
        let mut headers = self.envd.user_headers(options.user.as_deref())?;
        headers.insert(
            KEEPALIVE_PING_HEADER,
            HeaderValue::from(KEEPALIVE_PING_INTERVAL_SECS),
        );
        self.open("Start", &request, headers, options.timeout).await
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
    pub async fn run(&self, cmd: &str, options: &CommandOptions) -> Result<CommandResult> {
        self.start(cmd, options).await?.wait().await
    }

    /// Attaches to a process that is already running.
    ///
    /// # Arguments
    ///
    /// * `pid` - the process ID.
    /// * `timeout` - the stream deadline, or `None` for no limit.
    ///
    /// # Returns
    ///
    /// A [`CommandHandle`] for the process's remaining output.
    ///
    /// # Errors
    ///
    /// [`Error::Rpc`] with [`ConnectCode::NotFound`] if no such process, plus
    /// the errors of [`Commands::start`].
    pub async fn connect(&self, pid: u32, timeout: Option<Duration>) -> Result<CommandHandle> {
        let request = ConnectRequest {
            process: ProcessSelector::Pid(pid),
        };
        let mut headers = HeaderMap::with_capacity(1);
        headers.insert(
            KEEPALIVE_PING_HEADER,
            HeaderValue::from(KEEPALIVE_PING_INTERVAL_SECS),
        );
        self.open("Connect", &request, headers, timeout).await
    }

    /// Opens a `Start`/`Connect` stream and waits, within the request
    /// timeout, for the start event that carries the PID
    /// (`e2b/sandbox_async/commands/command.py:382-411`).
    async fn open<Req: serde::Serialize>(
        &self,
        method: &'static str,
        request: &Req,
        headers: HeaderMap,
        timeout: Option<Duration>,
    ) -> Result<CommandHandle> {
        let opening = async {
            let mut stream: ConnectStream<EventResponse> = self
                .envd
                .server_stream(process_pb::SERVICE, method, request, headers, timeout)
                .await?;
            let first = stream.next().await?;
            let pid = first
                .and_then(|response| response.event)
                .and_then(|event| event.start)
                .map(|start| start.pid)
                .ok_or_else(|| {
                    Error::Protocol(format!("{method} stream did not open with a start event"))
                })?;
            Ok::<_, Error>((pid, stream))
        };
        let (pid, stream) = match self.envd.request_timeout() {
            Some(limit) => tokio::time::timeout(limit, opening)
                .await
                .map_err(|_| Error::Timeout(limit))??,
            None => opening.await?,
        };
        Ok(CommandHandle {
            pid,
            stream,
            commands: self.clone(),
        })
    }

    /// Lists the processes running in the sandbox.
    ///
    /// # Returns
    ///
    /// One [`ProcessInfo`] per process.
    ///
    /// # Errors
    ///
    /// [`Error::Rpc`], [`Error::Decode`] or [`Error::Transport`].
    pub async fn list(&self) -> Result<Vec<ProcessInfo>> {
        let response: ListResponse = self
            .envd
            .unary(
                process_pb::SERVICE,
                "List",
                &ListRequest {},
                HeaderMap::new(),
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
    ///
    /// # Errors
    ///
    /// [`Error::Rpc`] (e.g. `not_found`) or [`Error::Transport`].
    pub async fn send_stdin(&self, pid: u32, data: impl AsRef<[u8]>) -> Result<()> {
        let request = SendInputRequest {
            process: ProcessSelector::Pid(pid),
            input: ProcessInput {
                stdin: data.as_ref(),
            },
        };
        let _: Empty = self
            .envd
            .unary(process_pb::SERVICE, "SendInput", &request, HeaderMap::new())
            .await?;
        Ok(())
    }

    /// Closes a process's stdin, signalling end of input.
    ///
    /// # Arguments
    ///
    /// * `pid` - the process ID.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidArgument`] when the sandbox's envd is older than 0.5.2,
    /// which has no `CloseStdin`; nothing is sent in that case, matching the
    /// SDK (`e2b/sandbox_async/commands/command.py:189-193`). Otherwise
    /// [`Error::Rpc`] (e.g. `not_found`) or [`Error::Transport`].
    pub async fn close_stdin(&self, pid: u32) -> Result<()> {
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
    ///
    /// # Errors
    ///
    /// [`Error::Rpc`] (e.g. `not_found`) or [`Error::Transport`].
    pub async fn send_signal(&self, pid: u32, signal: Signal) -> Result<()> {
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
            )
            .await?;
        Ok(())
    }

    /// Kills a process with `SIGKILL`.
    ///
    /// # Arguments
    ///
    /// * `pid` - the process ID.
    ///
    /// # Returns
    ///
    /// `true` if it was killed, `false` if no such process existed
    /// (`e2b/envd/rpc.py:215-231`).
    ///
    /// # Errors
    ///
    /// [`Error::Rpc`] other than `not_found`, or [`Error::Transport`].
    pub async fn kill(&self, pid: u32) -> Result<bool> {
        match self.send_signal(pid, Signal::Sigkill).await {
            Ok(()) => Ok(true),
            Err(Error::Rpc {
                code: ConnectCode::NotFound,
                ..
            }) => Ok(false),
            Err(err) => Err(err),
        }
    }
}

/// A started command. Read its output with [`next_event`](Self::next_event)
/// or collect everything with [`wait`](Self::wait).
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
    /// events are skipped.
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
    ///
    /// # Errors
    ///
    /// As [`Commands::send_stdin`].
    pub async fn send_stdin(&self, data: impl AsRef<[u8]>) -> Result<()> {
        self.commands.send_stdin(self.pid, data).await
    }

    /// Kills the command with `SIGKILL`. See [`Commands::kill`].
    ///
    /// # Returns
    ///
    /// `true` if it was killed, `false` if it had already exited.
    ///
    /// # Errors
    ///
    /// As [`Commands::kill`].
    pub async fn kill(&self) -> Result<bool> {
        self.commands.kill(self.pid).await
    }
}
