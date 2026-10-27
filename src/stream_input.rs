//! Streamed input to a running process, over envd's client-streaming
//! `process.Process/StreamInput` RPC
//! (`e2b/sandbox_async/commands/command_input_handle.py`).

use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use futures_util::stream::{self, Stream};
use reqwest::header::HeaderMap;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::envd::EnvdClient;
use crate::envd::connect::encode_envelope;
use crate::envd::process_pb::{self, ProcessInput, ProcessSelector, StreamInputRequest};
use crate::error::{Error, Result};

/// The SDKs' default bound on an input stream (`STREAM_INPUT_TIMEOUT`,
/// `e2b/connection_config.py:33`).
pub const DEFAULT_STREAM_INPUT_TIMEOUT: Duration = Duration::from_secs(60);

/// How long the stream may sit idle before a keepalive message is sent
/// (`KEEPALIVE_PING_INTERVAL_SEC`, `e2b/connection_config.py:48`).
const KEEPALIVE_INTERVAL: Duration = Duration::from_secs(50);

/// How many chunks may wait to be sent before [`InputStream::send`] waits.
const QUEUE_DEPTH: usize = 64;

/// Options for [`crate::Commands::stream_input`].
///
/// # Examples
///
/// ```
/// use std::time::Duration;
/// use e2b_client::{DEFAULT_STREAM_INPUT_TIMEOUT, StreamInputOptions};
///
/// // The SDKs' 60 s deadline applies unless you change it.
/// assert_eq!(
///     StreamInputOptions::default(),
///     StreamInputOptions::default().timeout(Some(DEFAULT_STREAM_INPUT_TIMEOUT)),
/// );
/// let unbounded = StreamInputOptions::default().timeout(None);
/// let longer = StreamInputOptions::default().timeout(Some(Duration::from_secs(600)));
/// # let _ = (unbounded, longer);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StreamInputOptions {
    // `pub(crate)`: readable by the modules that open input streams, but not
    // by users of the crate, who go through the builder method.
    pub(crate) timeout: Option<Duration>,
}

impl Default for StreamInputOptions {
    fn default() -> Self {
        Self {
            timeout: Some(DEFAULT_STREAM_INPUT_TIMEOUT),
        }
    }
}

impl StreamInputOptions {
    /// Bounds the whole input stream; `None` for no limit. Defaults to
    /// [`DEFAULT_STREAM_INPUT_TIMEOUT`].
    ///
    /// Keepalives do not extend this deadline: they only keep idle proxies
    /// from dropping the connection.
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

/// Which `ProcessInput` member the data goes in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InputKind {
    /// A command's stdin.
    Stdin,
    /// A PTY's input.
    #[expect(dead_code, reason = "PTY sessions (M2 Task 3) open PTY input streams")]
    Pty,
}

/// An ordered, long-lived input stream to a running command.
///
/// Get one from [`crate::Commands::stream_input`]. Chunks reach the process in
/// the order they were sent. Call [`close`](Self::close) to end the stream and
/// learn whether envd accepted it. Dropping the stream without `close` also
/// ends it, but any error is then lost.
///
/// The stream's deadline ([`StreamInputOptions::timeout`], 60 s by default)
/// bounds the whole stream. The keepalive sent after 50 s without input stops
/// proxies from dropping the connection, but it does not extend the
/// deadline: with the default, the stream ends 60 s after it opens however
/// active it is, as in the SDKs. Pass `.timeout(None)` for a stream with no
/// limit.
///
/// Python note: this is the SDK's `AsyncCommandInputHandle`. `send` is
/// `async` because it waits when 64 chunks are already queued, instead of
/// buffering without bound.
#[derive(Debug)]
pub struct InputStream {
    // `Option` so `send` and `close` can drop the sender (ending the request
    // body) and take the task out exactly once.
    sender: Option<mpsc::Sender<Bytes>>,
    task: Option<JoinHandle<Result<()>>>,
}

impl InputStream {
    /// Starts the `StreamInput` call on a background task.
    ///
    /// # Arguments
    ///
    /// * `envd` - the sandbox's envd transport.
    /// * `pid` - the process to send input to.
    /// * `kind` - which `ProcessInput` member carries the data.
    /// * `timeout` - the stream deadline, or `None` for no limit.
    ///
    /// # Returns
    ///
    /// The open stream.
    ///
    /// # Panics
    ///
    /// Outside a Tokio runtime (`tokio::spawn` requires one).
    pub(crate) fn open(
        envd: Arc<EnvdClient>,
        pid: u32,
        kind: InputKind,
        timeout: Option<Duration>,
    ) -> Self {
        let (sender, receiver) = mpsc::channel(QUEUE_DEPTH);
        let body = reqwest::Body::wrap_stream(request_messages(pid, kind, receiver));
        // `async move` moves `envd` and `body` into the task, which may
        // outlive this call (like `asyncio.create_task`).
        let task = tokio::spawn(async move {
            envd.client_stream(
                process_pb::SERVICE,
                "StreamInput",
                body,
                HeaderMap::new(),
                timeout,
            )
            .await
        });
        Self {
            sender: Some(sender),
            task: Some(task),
        }
    }

    /// Queues `data` to be sent after everything sent before it.
    ///
    /// # Arguments
    ///
    /// * `data` - the bytes (`&'static str`, `String`, `Vec<u8>`, `Bytes`...).
    ///
    /// # Errors
    ///
    /// The error that ended the stream early ([`Error::Rpc`],
    /// [`Error::Timeout`], [`Error::Transport`]...), [`Error::Protocol`] if
    /// envd ended the call cleanly before it was closed, or
    /// [`Error::InvalidArgument`] once that error was already returned.
    pub async fn send(&mut self, data: impl Into<Bytes>) -> Result<()> {
        let Some(sender) = &self.sender else {
            return Err(closed());
        };
        if sender.send(data.into()).await.is_ok() {
            return Ok(());
        }
        // The receiver is gone, so the call has ended: report why.
        self.sender = None;
        match self.finish().await {
            Err(err) => Err(err),
            Ok(()) => Err(Error::Protocol(
                "input stream ended before it was closed".into(),
            )),
        }
    }

    /// Ends the input stream after the queued chunks and waits for envd to
    /// acknowledge it.
    ///
    /// This does not close the process's stdin; use
    /// [`crate::Commands::close_stdin`] for that.
    ///
    /// # Errors
    ///
    /// The error envd ended the call with ([`Error::Rpc`]), [`Error::Timeout`]
    /// when the stream deadline passed, [`Error::Transport`] for connection
    /// failures, or [`Error::InvalidArgument`] if `send` already returned the
    /// stream's error.
    pub async fn close(mut self) -> Result<()> {
        // Dropping the sender ends the request body after the queued chunks.
        self.sender = None;
        self.finish().await
    }

    /// Waits for the background call and returns its outcome.
    async fn finish(&mut self) -> Result<()> {
        let task = self.task.take().ok_or_else(closed)?;
        // The outer `Result` is the task's (a panic or cancellation), the
        // inner one the call's.
        task.await
            .map_err(|err| Error::Protocol(format!("input stream task failed: {err}")))?
    }
}

fn closed() -> Error {
    Error::InvalidArgument("the input stream is closed; open a new one with stream_input".into())
}

/// The request body: a start message, then one data message per chunk, with
/// a keepalive whenever [`KEEPALIVE_INTERVAL`] passes without a chunk. It ends
/// when every sender is dropped.
fn request_messages(
    pid: u32,
    kind: InputKind,
    receiver: mpsc::Receiver<Bytes>,
) -> impl Stream<Item = Result<Bytes>> + Send + 'static {
    // `unfold` threads the state (the receiver, and whether start was sent)
    // through each step, like a Python async generator's local variables.
    stream::unfold(
        (receiver, false),
        move |(mut receiver, started)| async move {
            let frame = if started {
                match tokio::time::timeout(KEEPALIVE_INTERVAL, receiver.recv()).await {
                    Ok(Some(chunk)) => {
                        let input = match kind {
                            InputKind::Stdin => ProcessInput::Stdin(&chunk),
                            InputKind::Pty => ProcessInput::Pty(&chunk),
                        };
                        envelope(&StreamInputRequest::Data { input })
                    }
                    // Every sender is gone: end the body.
                    Ok(None) => return None,
                    Err(_idle) => envelope(&StreamInputRequest::Keepalive {}),
                }
            } else {
                envelope(&StreamInputRequest::Start {
                    process: ProcessSelector::Pid(pid),
                })
            };
            Some((frame, (receiver, true)))
        },
    )
}

/// One request message, enveloped.
fn envelope(message: &StreamInputRequest<'_>) -> Result<Bytes> {
    Ok(Bytes::from(encode_envelope(&serde_json::to_vec(message)?)?))
}

#[cfg(test)]
mod tests {
    use futures_util::StreamExt;
    use serde_json::{Value, json};
    use tokio::time::Instant;

    use super::*;

    fn decode(frame: &Bytes) -> Value {
        serde_json::from_slice(&frame[5..]).expect("frame json")
    }

    // `start_paused` freezes Tokio's clock and jumps it forward whenever every
    // task is waiting on a timer, so the 50 s keepalive arrives at once.
    #[tokio::test(start_paused = true)]
    async fn messages_start_then_data_then_keepalive_when_idle() {
        let (sender, receiver) = mpsc::channel(4);
        // `Box::pin` gives the `Unpin` stream that `next()` needs.
        let mut messages = Box::pin(request_messages(9, InputKind::Stdin, receiver));

        let start = messages.next().await.expect("start").expect("start ok");
        sender
            .send(Bytes::from_static(b"hi"))
            .await
            .expect("queue a chunk");
        let data = messages.next().await.expect("data").expect("data ok");
        let idle_since = Instant::now();
        let keepalive = messages
            .next()
            .await
            .expect("keepalive")
            .expect("keepalive ok");
        let idle_for = idle_since.elapsed();
        drop(sender);
        let end = messages.next().await;

        assert_eq!(decode(&start), json!({"start": {"process": {"pid": 9}}}));
        assert_eq!(decode(&data), json!({"data": {"input": {"stdin": "aGk="}}}));
        assert_eq!(decode(&keepalive), json!({"keepalive": {}}));
        assert_eq!(idle_for, KEEPALIVE_INTERVAL);
        assert!(end.is_none(), "the body ends once the sender is dropped");
    }

    #[test]
    fn options_default_to_the_sdk_deadline() {
        assert_eq!(
            StreamInputOptions::default().timeout,
            Some(Duration::from_secs(60))
        );
        assert_eq!(StreamInputOptions::default().timeout(None).timeout, None);
    }
}
