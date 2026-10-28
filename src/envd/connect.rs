//! A minimal Connect-protocol client: unary JSON calls and server-streaming
//! JSON calls, which is all envd's process and filesystem services need.
//!
//! Wire facts (cited in `docs/protocol-notes.md`):
//!
//! - Unary calls POST `application/json`; a non-200 status carries a JSON
//!   error `{"code": "<snake_case>", "message": "..."}`.
//! - Server-streaming calls POST `application/connect+json` with the request
//!   inside one envelope, and answer with a sequence of envelopes. An envelope
//!   is 1 flag byte plus a 4-byte big-endian length plus the payload. Flag bit
//!   `0x02` marks the final end-of-stream envelope, whose JSON payload may hold
//!   an `error`.
//! - `Connect-Protocol-Version: 1` is sent on every call and
//!   `Connect-Timeout-Ms` carries the deadline when there is one.

use std::marker::PhantomData;
use std::time::Duration;

use bytes::{Buf, Bytes, BytesMut};
use reqwest::header::{CONTENT_TYPE, HeaderMap, HeaderValue};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use tokio::time::Instant;

use crate::error::{ConnectCode, Error, Result};

/// `Content-Type` for unary JSON calls.
pub(crate) const CONTENT_TYPE_UNARY_JSON: &str = "application/json";
/// `Content-Type` for streaming JSON calls.
pub(crate) const CONTENT_TYPE_STREAM_JSON: &str = "application/connect+json";
/// Header carrying the protocol version.
pub(crate) const HEADER_PROTOCOL_VERSION: &str = "connect-protocol-version";
/// The only defined protocol version.
pub(crate) const PROTOCOL_VERSION: &str = "1";
/// Header carrying the call deadline in milliseconds.
pub(crate) const HEADER_TIMEOUT_MS: &str = "connect-timeout-ms";

/// Any message whose content this crate ignores (`SendInputResponse`,
/// `SendSignalResponse`, `RemoveResponse`, ...). Unknown fields are skipped.
#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct Empty {}

/// Envelope flag: the payload is compressed. This client never negotiates
/// compression, so seeing it is a protocol error.
const FLAG_COMPRESSED: u8 = 0b0000_0001;
/// Envelope flag: this is the end-of-stream envelope.
const FLAG_END_STREAM: u8 = 0b0000_0010;
/// Envelope header size: 1 flag byte plus a 4-byte length.
const ENVELOPE_HEADER_LEN: usize = 5;

/// Wraps one message in a Connect envelope with no flags set.
///
/// # Arguments
///
/// * `payload` - the serialized message.
///
/// # Returns
///
/// The envelope bytes.
///
/// # Errors
///
/// [`Error::InvalidArgument`] if the payload is larger than `u32::MAX` bytes.
pub(crate) fn encode_envelope(payload: &[u8]) -> Result<Vec<u8>> {
    let len = u32::try_from(payload.len())
        .map_err(|_| Error::InvalidArgument("Connect message larger than 4 GiB".into()))?;
    let mut envelope = Vec::with_capacity(ENVELOPE_HEADER_LEN + payload.len());
    envelope.push(0);
    envelope.extend_from_slice(&len.to_be_bytes());
    envelope.extend_from_slice(payload);
    Ok(envelope)
}

/// One decoded envelope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Envelope {
    /// The flag byte.
    pub(crate) flags: u8,
    /// The payload.
    pub(crate) data: Bytes,
}

impl Envelope {
    fn is_end_stream(&self) -> bool {
        self.flags & FLAG_END_STREAM != 0
    }
}

/// Incremental envelope decoder: push arbitrary byte chunks in, pull whole
/// envelopes out. Chunk boundaries need not line up with envelope boundaries.
#[derive(Debug, Default)]
pub(crate) struct EnvelopeDecoder {
    buffer: BytesMut,
}

impl EnvelopeDecoder {
    /// Appends bytes received from the network.
    pub(crate) fn push(&mut self, chunk: &[u8]) {
        self.buffer.extend_from_slice(chunk);
    }

    /// Whether undecoded bytes remain (a partial envelope).
    pub(crate) fn has_partial(&self) -> bool {
        !self.buffer.is_empty()
    }

    /// Takes the next complete envelope, if one is buffered.
    ///
    /// # Returns
    ///
    /// `Some(envelope)` when a whole envelope is available, `None` when more
    /// bytes are needed.
    pub(crate) fn next_envelope(&mut self) -> Option<Envelope> {
        if self.buffer.len() < ENVELOPE_HEADER_LEN {
            return None;
        }
        let flags = self.buffer[0];
        let len_bytes = [
            self.buffer[1],
            self.buffer[2],
            self.buffer[3],
            self.buffer[4],
        ];
        // `u32` always fits in `usize` on the 32- and 64-bit targets reqwest supports.
        let len = u32::from_be_bytes(len_bytes) as usize;
        if self.buffer.len() < ENVELOPE_HEADER_LEN + len {
            return None;
        }
        self.buffer.advance(ENVELOPE_HEADER_LEN);
        // `split_to` hands out the payload without copying it.
        let data = self.buffer.split_to(len).freeze();
        Some(Envelope { flags, data })
    }
}

/// A Connect error object, as found in unary error bodies and in the `error`
/// field of an end-of-stream message.
#[derive(Debug, Deserialize)]
struct WireError {
    #[serde(default)]
    code: Option<serde_json::Value>,
    #[serde(default)]
    message: Option<String>,
}

/// The end-of-stream message body.
#[derive(Debug, Deserialize)]
struct EndStream {
    #[serde(default)]
    error: Option<WireError>,
}

/// Converts a failed (non-200) Connect response into an [`Error::Rpc`].
///
/// A valid Connect error body (JSON with a known string `code`) is used as
/// is. Anything else is a plain HTTP error, e.g. a proxy answering for envd;
/// it is mapped the way the Python SDK does (`e2b/envd/client_shared.py:83-125`):
/// an integer `code` counts as the HTTP status, and the status picks the code.
///
/// # Arguments
///
/// * `status` - the HTTP status.
/// * `content_type` - the response `Content-Type`, if any.
/// * `body` - the response body.
///
/// # Returns
///
/// The error to surface.
pub(crate) fn error_from_response(status: u16, content_type: Option<&str>, body: &[u8]) -> Error {
    let is_json = content_type
        .and_then(|value| value.split(';').next())
        .is_some_and(|mime| mime.trim().eq_ignore_ascii_case(CONTENT_TYPE_UNARY_JSON));
    let parsed = if is_json {
        serde_json::from_slice::<WireError>(body).ok()
    } else {
        None
    };

    let mut effective_status = status;
    let mut message = None;
    if let Some(wire) = parsed {
        match &wire.code {
            Some(serde_json::Value::String(code)) => {
                if let Some(code) = ConnectCode::from_wire(code) {
                    return Error::Rpc {
                        code,
                        message: wire.message.unwrap_or_default(),
                    };
                }
            }
            Some(serde_json::Value::Number(number)) => {
                if let Some(code) = number.as_u64().and_then(|n| u16::try_from(n).ok()) {
                    effective_status = code;
                }
            }
            _ => {}
        }
        message = wire.message;
    }

    let message = message.unwrap_or_else(|| {
        let text = String::from_utf8_lossy(body);
        if text.is_empty() {
            format!("HTTP {status}")
        } else {
            text.into_owned()
        }
    });
    Error::Rpc {
        code: ConnectCode::from_plain_http_status(effective_status),
        message,
    }
}

/// Parses an end-of-stream payload.
///
/// # Returns
///
/// `Ok(())` for a clean end, or the error the server reported.
fn end_stream_result(data: &[u8]) -> Result<()> {
    // An empty payload is a valid, error-free end of stream.
    if data.is_empty() {
        return Ok(());
    }
    let end: EndStream = serde_json::from_slice(data)
        .map_err(|err| Error::Protocol(format!("invalid end-of-stream message: {err}")))?;
    match end.error {
        None => Ok(()),
        Some(wire) => {
            let code = match &wire.code {
                Some(serde_json::Value::String(code)) => {
                    ConnectCode::from_wire(code).unwrap_or(ConnectCode::Unknown)
                }
                _ => ConnectCode::Unknown,
            };
            Err(Error::Rpc {
                code,
                message: wire.message.unwrap_or_default(),
            })
        }
    }
}

/// Builds the Connect protocol headers for a call.
///
/// # Arguments
///
/// * `content_type` - unary or streaming JSON content type.
/// * `timeout` - the call deadline to advertise, if any.
///
/// # Returns
///
/// The headers to add to the request.
pub(crate) fn protocol_headers(content_type: &'static str, timeout: Option<Duration>) -> HeaderMap {
    let mut headers = HeaderMap::with_capacity(3);
    headers.insert(CONTENT_TYPE, HeaderValue::from_static(content_type));
    headers.insert(
        HEADER_PROTOCOL_VERSION,
        HeaderValue::from_static(PROTOCOL_VERSION),
    );
    if let Some(timeout) = timeout {
        // A positive timeout is sent as at least 1 ms, as the Python SDK does
        // (`e2b/envd/utils.py:15-24`), so it stays a deadline.
        let millis = timeout.as_millis().max(1);
        headers.insert(
            HEADER_TIMEOUT_MS,
            HeaderValue::from(u64::try_from(millis).unwrap_or(u64::MAX)),
        );
    }
    headers
}

/// Decodes a successful unary response body.
///
/// # Errors
///
/// [`Error::Decode`] if the body is not the expected JSON.
pub(crate) fn decode_unary<T: DeserializeOwned>(body: &[u8]) -> Result<T> {
    // An empty body is the canonical encoding of an empty message.
    let body: &[u8] = if body.is_empty() { b"{}" } else { body };
    Ok(serde_json::from_slice(body)?)
}

/// A server-streaming Connect response, yielding decoded messages of type `T`.
///
/// Python note: this is an async iterator. Call [`next`](Self::next) in a
/// `while let Some(msg) = stream.next().await? { ... }` loop.
#[derive(Debug)]
pub(crate) struct ConnectStream<T> {
    response: reqwest::Response,
    decoder: EnvelopeDecoder,
    deadline: Option<(Instant, Duration)>,
    finished: bool,
    // `PhantomData<fn() -> T>` records the message type without owning a `T`,
    // which keeps the stream `Send`/`Sync` regardless of `T`.
    _message: PhantomData<fn() -> T>,
}

impl<T: DeserializeOwned> ConnectStream<T> {
    /// Wraps a successful (HTTP 200) streaming response.
    ///
    /// # Arguments
    ///
    /// * `response` - the HTTP response whose body carries envelopes.
    /// * `timeout` - a client-side bound on the whole stream, if any.
    pub(crate) fn new(response: reqwest::Response, timeout: Option<Duration>) -> Self {
        Self {
            response,
            decoder: EnvelopeDecoder::default(),
            deadline: timeout.map(|timeout| (Instant::now() + timeout, timeout)),
            finished: false,
            _message: PhantomData,
        }
    }

    /// Waits for the next message.
    ///
    /// # Returns
    ///
    /// `Ok(Some(message))` for each message, then `Ok(None)` once the server
    /// sent a clean end-of-stream envelope.
    ///
    /// # Errors
    ///
    /// - [`Error::Rpc`] when the end-of-stream envelope carries an error.
    /// - [`Error::Protocol`] for a compressed or truncated envelope, or a body
    ///   that ends without an end-of-stream envelope.
    /// - [`Error::Decode`] when a message is not the expected JSON.
    /// - [`Error::Timeout`] when the stream deadline passes.
    /// - [`Error::Transport`] when the connection fails.
    pub(crate) async fn next(&mut self) -> Result<Option<T>> {
        loop {
            if self.finished {
                return Ok(None);
            }
            if let Some(envelope) = self.decoder.next_envelope() {
                return self.handle_envelope(&envelope);
            }
            match self.read_chunk().await? {
                Some(chunk) => self.decoder.push(&chunk),
                None => {
                    self.finished = true;
                    let reason = if self.decoder.has_partial() {
                        "stream ended in the middle of an envelope"
                    } else {
                        "stream ended without an end-of-stream message"
                    };
                    return Err(Error::Protocol(reason.into()));
                }
            }
        }
    }

    fn handle_envelope(&mut self, envelope: &Envelope) -> Result<Option<T>> {
        if envelope.flags & FLAG_COMPRESSED != 0 {
            self.finished = true;
            return Err(Error::Protocol("received a compressed envelope".into()));
        }
        if envelope.is_end_stream() {
            self.finished = true;
            return end_stream_result(&envelope.data).map(|()| None);
        }
        Ok(Some(serde_json::from_slice(&envelope.data)?))
    }

    async fn read_chunk(&mut self) -> Result<Option<Bytes>> {
        match self.deadline {
            None => self.response.chunk().await.map_err(Error::transport),
            Some((deadline, timeout)) => {
                match tokio::time::timeout_at(deadline, self.response.chunk()).await {
                    Ok(chunk) => chunk.map_err(Error::transport),
                    Err(_elapsed) => {
                        self.finished = true;
                        Err(Error::Timeout(timeout))
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_round_trip_across_arbitrary_chunk_boundaries() {
        let mut wire = encode_envelope(br#"{"a":1}"#).expect("encode");
        wire.extend(encode_envelope(br#"{"a":2}"#).expect("encode"));
        let mut decoder = EnvelopeDecoder::default();
        let mut decoded = Vec::new();
        // Feed one byte at a time: the worst case for boundary handling.
        for byte in &wire {
            decoder.push(std::slice::from_ref(byte));
            while let Some(envelope) = decoder.next_envelope() {
                decoded.push(envelope);
            }
        }
        assert_eq!(decoded.len(), 2);
        assert_eq!(decoded[0].data.as_ref(), br#"{"a":1}"#);
        assert_eq!(decoded[1].data.as_ref(), br#"{"a":2}"#);
        assert!(!decoder.has_partial());
    }

    #[test]
    fn envelope_header_layout_is_flag_then_big_endian_length() {
        let wire = encode_envelope(b"xyz").expect("encode");
        assert_eq!(wire, [0, 0, 0, 0, 3, b'x', b'y', b'z']);
    }

    #[test]
    fn partial_envelope_waits_for_more_bytes() {
        let mut decoder = EnvelopeDecoder::default();
        decoder.push(&[0, 0, 0, 0, 5, b'a']);
        assert!(decoder.next_envelope().is_none());
        assert!(decoder.has_partial());
    }

    #[test]
    fn end_stream_without_error_is_ok() {
        assert!(end_stream_result(b"{}").is_ok());
        assert!(end_stream_result(b"").is_ok());
        assert!(end_stream_result(br#"{"metadata":{"k":["v"]}}"#).is_ok());
    }

    #[test]
    fn end_stream_with_error_maps_code() {
        let err = end_stream_result(br#"{"error":{"code":"not_found","message":"gone"}}"#)
            .expect_err("error");
        assert!(matches!(
            err,
            Error::Rpc { code: ConnectCode::NotFound, ref message } if message == "gone"
        ));
    }

    #[test]
    fn end_stream_with_unknown_code_is_unknown() {
        let err = end_stream_result(br#"{"error":{"code":"weird"}}"#).expect_err("error");
        assert!(matches!(
            err,
            Error::Rpc {
                code: ConnectCode::Unknown,
                ..
            }
        ));
    }

    #[test]
    fn malformed_end_stream_is_protocol_error() {
        assert!(matches!(
            end_stream_result(b"not json"),
            Err(Error::Protocol(_))
        ));
    }

    #[test]
    fn connect_error_body_is_used_verbatim() {
        let err = error_from_response(
            404,
            Some("application/json; charset=utf-8"),
            br#"{"code":"already_exists","message":"dir exists"}"#,
        );
        assert!(matches!(
            err,
            Error::Rpc { code: ConnectCode::AlreadyExists, ref message } if message == "dir exists"
        ));
    }

    #[test]
    fn plain_http_error_maps_status() {
        let err = error_from_response(502, Some("text/plain"), b"bad gateway");
        assert!(matches!(
            err,
            Error::Rpc { code: ConnectCode::Unavailable, ref message } if message == "bad gateway"
        ));
    }

    #[test]
    fn plain_json_error_with_integer_code_uses_it_as_status() {
        let err = error_from_response(
            500,
            Some("application/json"),
            br#"{"code":429,"message":"slow down"}"#,
        );
        assert!(matches!(
            err,
            Error::Rpc { code: ConnectCode::ResourceExhausted, ref message } if message == "slow down"
        ));
    }

    #[test]
    fn empty_plain_error_gets_status_message() {
        let err = error_from_response(404, None, b"");
        assert!(matches!(
            err,
            Error::Rpc { code: ConnectCode::NotFound, ref message } if message == "HTTP 404"
        ));
    }

    #[test]
    fn protocol_headers_include_version_and_timeout() {
        let headers = protocol_headers(CONTENT_TYPE_STREAM_JSON, Some(Duration::from_micros(10)));
        assert_eq!(headers[CONTENT_TYPE], CONTENT_TYPE_STREAM_JSON);
        assert_eq!(headers[HEADER_PROTOCOL_VERSION], "1");
        assert_eq!(headers[HEADER_TIMEOUT_MS], "1");
        let headers = protocol_headers(CONTENT_TYPE_UNARY_JSON, None);
        assert!(!headers.contains_key(HEADER_TIMEOUT_MS));
    }

    #[test]
    fn empty_unary_body_decodes_as_empty_message() {
        #[derive(Deserialize)]
        struct Empty {}
        let _: Empty = decode_unary(b"").expect("empty");
    }
}
