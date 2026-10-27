//! The envd side of a sandbox: addressing, auth headers, and the transport
//! for the Connect-RPC services and the plain HTTP file API.

pub(crate) mod connect;
pub(crate) mod filesystem_pb;
pub(crate) mod process_pb;

use std::fmt;
use std::str::FromStr;
use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue};
use reqwest::{Method, RequestBuilder};
use secrecy::{ExposeSecret, SecretString};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::config::{ConnectionConfig, ENVD_PORT};
use crate::error::{Error, Result};
use connect::{
    CONTENT_TYPE_STREAM_JSON, CONTENT_TYPE_UNARY_JSON, ConnectStream, Empty, decode_unary,
    encode_envelope, error_from_response, protocol_headers,
};

/// Header naming the sandbox an envd request is for.
const HEADER_SANDBOX_ID: &str = "e2b-sandbox-id";
/// Header naming the in-sandbox port an envd request is for.
const HEADER_SANDBOX_PORT: &str = "e2b-sandbox-port";
/// Header carrying the envd access token of a secured sandbox.
const HEADER_ACCESS_TOKEN: &str = "x-access-token";

/// The user envd used before it had a configurable default user (< 0.4.0).
const LEGACY_DEFAULT_USER: &str = "user";

/// Asks envd to send keepalive events on long streams so idle proxies do
/// not drop them (`e2b/connection_config.py:48-49`).
const KEEPALIVE_PING_HEADER: &str = "keepalive-ping-interval";
/// The keepalive interval the SDKs use, in seconds
/// (`KEEPALIVE_PING_INTERVAL_SEC`, `e2b/connection_config.py:48`).
const KEEPALIVE_PING_INTERVAL_SECS: u16 = 50;

/// The `Keepalive-Ping-Interval: 50` header envd honours on long streams.
///
/// # Returns
///
/// A header map holding just that header.
pub(crate) fn keepalive_headers() -> HeaderMap {
    let mut headers = HeaderMap::with_capacity(1);
    headers.insert(
        KEEPALIVE_PING_HEADER,
        HeaderValue::from(KEEPALIVE_PING_INTERVAL_SECS),
    );
    headers
}

/// The version of the envd daemon running in a sandbox, as reported by the
/// API (`envdVersion`).
///
/// Some request details depend on it (for example, envd older than 0.4.0 needs
/// an explicit user). A version string that does not parse as
/// `MAJOR.MINOR.PATCH` is treated as newer than any real release, the same
/// fallback the SDKs use in debug mode (`e2b/envd/versions.py:4`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EnvdVersion {
    major: u64,
    minor: u64,
    patch: u64,
}

impl EnvdVersion {
    /// The fallback for unparseable versions: newer than any release.
    pub const UNKNOWN_LATEST: Self = Self::new(99, 99, 99);
    /// envd 0.4.0 introduced a configurable default user
    /// (`e2b/envd/versions.py:6`).
    pub(crate) const DEFAULT_USER: Self = Self::new(0, 4, 0);
    /// envd 0.5.2 added `CloseStdin`; the SDKs refuse it on older envd
    /// (`e2b/envd/versions.py:7`).
    pub(crate) const CLOSE_STDIN: Self = Self::new(0, 5, 2);
    /// envd 0.1.4 added recursive directory watching
    /// (`e2b/envd/versions.py:3`).
    pub(crate) const RECURSIVE_WATCH: Self = Self::new(0, 1, 4);
    /// envd 0.6.3 added entry info on watch events
    /// (`e2b/envd/versions.py:10`).
    pub(crate) const FS_EVENT_ENTRY_INFO: Self = Self::new(0, 6, 3);
    /// envd 0.6.4 allowed watching network mounts
    /// (`e2b/envd/versions.py:11`).
    pub(crate) const WATCH_NETWORK_MOUNTS: Self = Self::new(0, 6, 4);
    /// envd 0.5.7 accepts `application/octet-stream` (and gzip) uploads;
    /// older envd only takes multipart (`e2b/envd/versions.py:8`).
    pub(crate) const OCTET_STREAM_UPLOAD: Self = Self::new(0, 5, 7);
    /// envd 0.6.2 stores user metadata sent as `X-Metadata-*` upload
    /// headers; the SDKs refuse metadata on older envd
    /// (`e2b/envd/versions.py:9`).
    pub(crate) const FILE_METADATA: Self = Self::new(0, 6, 2);

    /// Creates a version from its parts.
    ///
    /// # Arguments
    ///
    /// * `major`, `minor`, `patch` - the semantic version components.
    ///
    /// # Returns
    ///
    /// The version.
    #[must_use]
    pub const fn new(major: u64, minor: u64, patch: u64) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Parses `MAJOR.MINOR.PATCH`, ignoring a leading `v` and any
    /// pre-release or build suffix; unparseable input yields
    /// [`EnvdVersion::UNKNOWN_LATEST`].
    ///
    /// # Arguments
    ///
    /// * `raw` - the version string from the API.
    ///
    /// # Returns
    ///
    /// The parsed version, or the fallback.
    ///
    /// # Examples
    ///
    /// ```
    /// use e2b_client::EnvdVersion;
    ///
    /// assert_eq!(EnvdVersion::parse_lenient("0.2.4"), EnvdVersion::new(0, 2, 4));
    /// assert_eq!(EnvdVersion::parse_lenient("v1.0.0-rc1"), EnvdVersion::new(1, 0, 0));
    /// assert_eq!(EnvdVersion::parse_lenient("dev"), EnvdVersion::UNKNOWN_LATEST);
    /// ```
    #[must_use]
    pub fn parse_lenient(raw: &str) -> Self {
        raw.parse().unwrap_or(Self::UNKNOWN_LATEST)
    }
}

impl FromStr for EnvdVersion {
    type Err = Error;

    fn from_str(raw: &str) -> Result<Self> {
        let core = raw.trim().trim_start_matches('v');
        let core = core.split(['-', '+']).next().unwrap_or_default();
        let mut parts = core.split('.').map(str::parse::<u64>);
        let invalid = || Error::InvalidArgument(format!("invalid envd version {raw:?}"));
        let mut next = || parts.next().ok_or_else(invalid)?.map_err(|_| invalid());
        let version = Self::new(next()?, next()?, next()?);
        Ok(version)
    }
}

impl fmt::Display for EnvdVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// What a sandbox's envd client needs to know about the sandbox.
pub(crate) struct EnvdTarget<'a> {
    pub(crate) sandbox_id: &'a str,
    pub(crate) sandbox_domain: Option<&'a str>,
    pub(crate) access_token: Option<&'a SecretString>,
    pub(crate) version: EnvdVersion,
}

/// Transport to one sandbox's envd.
///
/// Holds the per-sandbox base URLs and the headers every envd request carries:
/// `E2b-Sandbox-Id`, `E2b-Sandbox-Port`, and `X-Access-Token` when the sandbox
/// is secured (`e2b/sandbox_async/main.py:1180-1236`; JS
/// `dist/index.js:5805-5818`).
#[derive(Debug)]
pub(crate) struct EnvdClient {
    http: reqwest::Client,
    rpc_base: String,
    direct_base: String,
    headers: HeaderMap,
    version: EnvdVersion,
    request_timeout: Option<Duration>,
}

impl EnvdClient {
    /// Builds the envd transport for a sandbox.
    ///
    /// # Arguments
    ///
    /// * `http` - the shared envd HTTP client (it never carries the API key).
    /// * `config` - connection configuration, for addressing and timeouts.
    /// * `target` - the sandbox to reach.
    ///
    /// # Returns
    ///
    /// The transport.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidArgument`] if the sandbox ID or access token cannot be
    /// sent as an HTTP header.
    pub(crate) fn new(
        http: reqwest::Client,
        config: &ConnectionConfig,
        target: &EnvdTarget<'_>,
    ) -> Result<Self> {
        let mut headers = HeaderMap::with_capacity(3);
        headers.insert(
            HEADER_SANDBOX_ID,
            header_value("sandbox ID", target.sandbox_id)?,
        );
        headers.insert(HEADER_SANDBOX_PORT, HeaderValue::from(ENVD_PORT));
        if let Some(token) = target.access_token {
            let mut value = header_value("access token", token.expose_secret())?;
            // Marks the value so HTTP/2 HPACK never indexes it and debug
            // output hides it.
            value.set_sensitive(true);
            headers.insert(HEADER_ACCESS_TOKEN, value);
        }
        Ok(Self {
            http,
            rpc_base: config.envd_rpc_url(target.sandbox_id, target.sandbox_domain),
            direct_base: config.envd_direct_url(target.sandbox_id, target.sandbox_domain),
            headers,
            version: target.version,
            request_timeout: config.request_timeout(),
        })
    }

    /// The sandbox's envd version.
    pub(crate) fn version(&self) -> EnvdVersion {
        self.version
    }

    /// The client's request timeout, which bounds unary calls, stream
    /// opening and in-memory uploads.
    ///
    /// # Returns
    ///
    /// The timeout, or `None` when requests are unbounded.
    pub(crate) fn request_timeout(&self) -> Option<Duration> {
        self.request_timeout
    }

    /// The user to act as: the caller's choice, or `"user"` on envd older than
    /// 0.4.0, which has no default user (`e2b/envd/utils.py:44-57`).
    pub(crate) fn effective_user<'a>(&self, user: Option<&'a str>) -> Option<&'a str> {
        match user {
            Some(user) => Some(user),
            None if self.version < EnvdVersion::DEFAULT_USER => Some(LEGACY_DEFAULT_USER),
            None => None,
        }
    }

    /// The `Authorization: Basic base64("<user>:")` header envd RPCs use to
    /// select the acting user (`e2b/envd/utils.py:44-57`; JS
    /// `dist/index.js:1737`). Empty when no user applies.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidArgument`] if the encoded value is not a valid header.
    pub(crate) fn user_headers(&self, user: Option<&str>) -> Result<HeaderMap> {
        let mut headers = HeaderMap::new();
        if let Some(user) = self.effective_user(user) {
            let encoded = STANDARD.encode(format!("{user}:"));
            headers.insert(
                AUTHORIZATION,
                header_value("user", &format!("Basic {encoded}"))?,
            );
        }
        Ok(headers)
    }

    fn rpc_url(&self, service: &str, method: &str) -> String {
        format!("{}/{service}/{method}", self.rpc_base)
    }

    /// Makes a unary Connect call.
    ///
    /// # Arguments
    ///
    /// * `service` - e.g. `"process.Process"`.
    /// * `method` - e.g. `"List"`.
    /// * `request` - the request message.
    /// * `extra_headers` - per-call headers (e.g. the user header).
    ///
    /// # Returns
    ///
    /// The decoded response message.
    ///
    /// # Errors
    ///
    /// [`Error::Rpc`] for a Connect or plain HTTP error response,
    /// [`Error::Decode`] for an undecodable body, [`Error::Transport`] for
    /// connection failures and request timeouts.
    pub(crate) async fn unary<Req: Serialize, Res: DeserializeOwned>(
        &self,
        service: &str,
        method: &str,
        request: &Req,
        extra_headers: HeaderMap,
    ) -> Result<Res> {
        let body = serde_json::to_vec(request)?;
        let mut builder = self
            .http
            .post(self.rpc_url(service, method))
            .headers(self.headers.clone())
            .headers(extra_headers)
            .headers(protocol_headers(
                CONTENT_TYPE_UNARY_JSON,
                self.request_timeout,
            ))
            .body(body);
        if let Some(timeout) = self.request_timeout {
            builder = builder.timeout(timeout);
        }
        let response = builder.send().await?;
        let status = response.status();
        let content_type = content_type_of(&response);
        let bytes = response.bytes().await?;
        if status != reqwest::StatusCode::OK {
            return Err(error_from_response(
                status.as_u16(),
                content_type.as_deref(),
                &bytes,
            ));
        }
        decode_unary(&bytes)
    }

    /// Opens a server-streaming Connect call.
    ///
    /// The returned stream is bounded by `timeout` (sent as
    /// `Connect-Timeout-Ms` and enforced client-side), not by the request
    /// timeout; callers bound the wait for the first message themselves.
    ///
    /// # Arguments
    ///
    /// * `service`, `method` - the RPC to call.
    /// * `request` - the request message.
    /// * `extra_headers` - per-call headers.
    /// * `timeout` - the stream deadline, or `None` for no limit.
    ///
    /// # Returns
    ///
    /// The open stream.
    ///
    /// # Errors
    ///
    /// [`Error::Rpc`] when the server rejects the call before streaming,
    /// [`Error::Transport`] for connection failures.
    pub(crate) async fn server_stream<Req: Serialize, Res: DeserializeOwned>(
        &self,
        service: &str,
        method: &str,
        request: &Req,
        extra_headers: HeaderMap,
        timeout: Option<Duration>,
    ) -> Result<ConnectStream<Res>> {
        let body = encode_envelope(&serde_json::to_vec(request)?)?;
        let response = self
            .post_stream(service, method, body.into(), extra_headers, timeout)
            .await?;
        Ok(ConnectStream::new(response, timeout))
    }

    /// Makes a client-streaming Connect call: `body` yields the request
    /// envelopes, and the call finishes when envd answers after the body ends.
    ///
    /// # Arguments
    ///
    /// * `service`, `method` - the RPC to call.
    /// * `body` - the enveloped request messages, streamed.
    /// * `extra_headers` - per-call headers.
    /// * `timeout` - the call deadline (sent as `Connect-Timeout-Ms` and
    ///   enforced client-side), or `None` for no limit.
    ///
    /// # Errors
    ///
    /// [`Error::Rpc`] when envd rejects the call or ends it with an error,
    /// [`Error::Timeout`] when the deadline passes, [`Error::Protocol`] for a
    /// malformed response, [`Error::Transport`] for connection failures.
    pub(crate) async fn client_stream(
        &self,
        service: &str,
        method: &str,
        body: reqwest::Body,
        extra_headers: HeaderMap,
        timeout: Option<Duration>,
    ) -> Result<()> {
        let call = async {
            let response = self
                .post_stream(service, method, body, extra_headers, timeout)
                .await?;
            // Connect answers a client stream with one message and an
            // end-of-stream envelope; drain both so an error is surfaced.
            let mut stream: ConnectStream<Empty> = ConnectStream::new(response, None);
            while stream.next().await?.is_some() {}
            Ok(())
        };
        within(timeout, call).await
    }

    /// Runs `opening` (open a stream and read its first event) within the
    /// request timeout, as the SDKs' `first_event` does
    /// (`e2b/envd/client_async/__init__.py:113-149`). After that only the
    /// stream's own deadline applies.
    ///
    /// # Arguments
    ///
    /// * `opening` - the future that opens the stream and reads its first
    ///   event.
    ///
    /// # Returns
    ///
    /// What `opening` returns.
    ///
    /// # Errors
    ///
    /// [`Error::Timeout`] when the request timeout passes first, else the
    /// error of `opening`.
    pub(crate) async fn within_request_timeout<T>(
        &self,
        opening: impl Future<Output = Result<T>>,
    ) -> Result<T> {
        within(self.request_timeout, opening).await
    }

    /// POSTs a streaming Connect call and returns the response once it is 200.
    async fn post_stream(
        &self,
        service: &str,
        method: &str,
        body: reqwest::Body,
        extra_headers: HeaderMap,
        timeout: Option<Duration>,
    ) -> Result<reqwest::Response> {
        let response = self
            .http
            .post(self.rpc_url(service, method))
            .headers(self.headers.clone())
            .headers(extra_headers)
            .headers(protocol_headers(CONTENT_TYPE_STREAM_JSON, timeout))
            .body(body)
            .send()
            .await?;
        let status = response.status();
        if status != reqwest::StatusCode::OK {
            let content_type = content_type_of(&response);
            let bytes = response.bytes().await?;
            return Err(error_from_response(
                status.as_u16(),
                content_type.as_deref(),
                &bytes,
            ));
        }
        Ok(response)
    }

    /// Starts a request to envd's plain HTTP API at `path`, with the sandbox
    /// headers and the request timeout applied.
    pub(crate) fn http_request(&self, method: Method, path: &str) -> RequestBuilder {
        self.http_request_with_timeout(method, path, self.request_timeout)
    }

    /// Starts a request to envd's plain HTTP API at `path`, with the sandbox
    /// headers and a whole-request `timeout` in place of the request timeout.
    ///
    /// # Arguments
    ///
    /// * `method` - the HTTP method.
    /// * `path` - the route, e.g. `/files`.
    /// * `timeout` - the deadline for the whole request, or `None` for none
    ///   (streamed transfers, which a deadline would cut off).
    ///
    /// # Returns
    ///
    /// The request builder, ready for a query and body.
    pub(crate) fn http_request_with_timeout(
        &self,
        method: Method,
        path: &str,
        timeout: Option<Duration>,
    ) -> RequestBuilder {
        let builder = self
            .http
            .request(method, format!("{}{path}", self.direct_base))
            .headers(self.headers.clone());
        match timeout {
            Some(timeout) => builder.timeout(timeout),
            None => builder,
        }
    }
}

// Test-only accessors: `#[cfg(test)]` compiles this block only for
// `cargo test`, so the base URLs stay private in the published crate.
#[cfg(test)]
impl EnvdClient {
    /// The base URL Connect-RPC calls are sent to.
    pub(crate) fn rpc_base(&self) -> &str {
        &self.rpc_base
    }

    /// The base URL of the plain HTTP API (`/files`, `/health`).
    pub(crate) fn direct_base(&self) -> &str {
        &self.direct_base
    }
}

/// Runs `future` with an optional deadline.
///
/// # Errors
///
/// [`Error::Timeout`] when `limit` passes first, else the error of `future`.
async fn within<T>(limit: Option<Duration>, future: impl Future<Output = Result<T>>) -> Result<T> {
    match limit {
        // `timeout` returns `Err(Elapsed)` when the deadline wins; the inner
        // `Result` is the future's own outcome, returned as is.
        Some(limit) => tokio::time::timeout(limit, future)
            .await
            .map_err(|_elapsed| Error::Timeout(limit))?,
        None => future.await,
    }
}

fn content_type_of(response: &reqwest::Response) -> Option<String> {
    response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
}

fn header_value(what: &str, value: &str) -> Result<HeaderValue> {
    HeaderValue::from_str(value)
        .map_err(|_| Error::InvalidArgument(format!("{what} is not a valid HTTP header value")))
}

/// Converts a non-success envd HTTP response into [`Error::Envd`], reading
/// the `message` from a JSON body when there is one (`e2b/envd/api.py:143-152`).
///
/// # Errors
///
/// Always returns an error; [`Error::Transport`] if the body cannot be read.
pub(crate) async fn envd_http_error(response: reqwest::Response) -> Error {
    let status = response.status().as_u16();
    match response.bytes().await {
        Ok(body) => Error::Envd {
            status,
            message: json_message_or_text(&body),
        },
        Err(err) => Error::Transport(err),
    }
}

/// The `message` string of a JSON object body, else the body as text.
pub(crate) fn json_message_or_text(body: &[u8]) -> String {
    #[derive(serde::Deserialize)]
    struct WithMessage {
        message: String,
    }
    serde_json::from_slice::<WithMessage>(body)
        .map(|parsed| parsed.message)
        .unwrap_or_else(|_| String::from_utf8_lossy(body).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_parsing_and_ordering() {
        assert_eq!(
            "0.4.0".parse::<EnvdVersion>().expect("parse"),
            EnvdVersion::DEFAULT_USER
        );
        assert!(EnvdVersion::parse_lenient("0.3.9") < EnvdVersion::DEFAULT_USER);
        assert!(EnvdVersion::parse_lenient("0.10.0") > EnvdVersion::DEFAULT_USER);
        assert!("1.2".parse::<EnvdVersion>().is_err());
        assert!("a.b.c".parse::<EnvdVersion>().is_err());
        assert_eq!(EnvdVersion::new(1, 2, 3).to_string(), "1.2.3");
        assert!(EnvdVersion::parse_lenient("0.5.1") < EnvdVersion::CLOSE_STDIN);
        assert!(EnvdVersion::parse_lenient("0.5.2") >= EnvdVersion::CLOSE_STDIN);
        assert!(EnvdVersion::UNKNOWN_LATEST > EnvdVersion::CLOSE_STDIN);
        assert_eq!(EnvdVersion::RECURSIVE_WATCH.to_string(), "0.1.4");
        assert_eq!(EnvdVersion::FS_EVENT_ENTRY_INFO.to_string(), "0.6.3");
        assert_eq!(EnvdVersion::WATCH_NETWORK_MOUNTS.to_string(), "0.6.4");
        assert_eq!(EnvdVersion::OCTET_STREAM_UPLOAD.to_string(), "0.5.7");
    }

    #[test]
    fn unparseable_or_debug_versions_are_99_99_99() {
        for raw in ["", "dev", "debug", "1.2", "a.b.c", "0.x.1"] {
            assert_eq!(
                EnvdVersion::parse_lenient(raw),
                EnvdVersion::new(99, 99, 99),
                "{raw:?}"
            );
        }
    }

    #[test]
    fn json_message_extraction() {
        assert_eq!(
            json_message_or_text(br#"{"message":"nope","code":404}"#),
            "nope"
        );
        assert_eq!(json_message_or_text(b"plain text"), "plain text");
    }
}
