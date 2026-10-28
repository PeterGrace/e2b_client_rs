//! Error types returned by every fallible call in this crate.
//!
//! There are three families of remote failure, mirroring the three protocols an
//! E2B client speaks:
//!
//! - [`Error::Api`]: the REST control plane (`api.<domain>`) answered with a
//!   non-success HTTP status. The E2B error body (`{"code", "message"}`) is
//!   parsed when present.
//! - [`Error::Rpc`]: an envd Connect-RPC call (process or filesystem service)
//!   failed with a Connect error code, either in a unary error response, in a
//!   streaming end-of-stream frame, or synthesised from a plain HTTP error that
//!   a proxy sent instead of envd.
//! - [`Error::Envd`]: envd's plain HTTP file API (`/files`, `/health`) answered
//!   with a non-success status.
//!
//! On top of the variants, [`Error::status`] and the `is_*` classifiers
//! answer the questions the official SDKs answer with exception classes
//! (`AuthenticationException`, `RateLimitException`, `ServiceBusyException`,
//! `NotEnoughSpaceException`, `SandboxNotRunningException`,
//! `TimeoutException`), whichever protocol the error came from.

use std::fmt;
use std::time::Duration;

/// A `Result` alias whose error type is this crate's [`Error`].
///
/// Python note: Rust has no exceptions. Every fallible function returns a
/// `Result`, and the `?` operator plays the role of "let the exception
/// propagate".
pub type Result<T> = std::result::Result<T, Error>;

/// Every error this crate can return.
///
/// The enum is `#[non_exhaustive]` so new variants can be added without a
/// breaking change; match on the variants you care about and keep a `_` arm.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// No API key was configured: neither passed to the builder nor found in
    /// `E2B_API_KEY`.
    #[error(
        "no E2B API key: pass one to ConnectionConfig::builder().api_key(...) \
         or set the E2B_API_KEY environment variable"
    )]
    MissingApiKey,

    /// A configuration value could not be used, for example an unparseable URL.
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),

    /// An argument to a call was rejected before any request was sent.
    #[error("invalid argument: {0}")]
    InvalidArgument(String),

    /// The E2B REST API answered with a non-success HTTP status.
    ///
    /// `#[non_exhaustive]`: match it with `Error::Api { status, .. }`, so a
    /// field added later is not a breaking change.
    #[error("E2B API error (HTTP {status}): {message}")]
    #[non_exhaustive]
    Api {
        /// The HTTP status code.
        status: u16,
        /// The `message` from the E2B error body, or the raw body when it was
        /// not an E2B error object.
        message: String,
    },

    /// The REST API reported that the sandbox does not exist (HTTP 404).
    #[error("sandbox {0} not found")]
    SandboxNotFound(String),

    /// The REST API reported that the volume does not exist (HTTP 404 from
    /// [`crate::Client::get_volume`]).
    #[error("volume {0} not found")]
    VolumeNotFound(String),

    /// The MCP gateway did not start in a sandbox created with
    /// [`crate::CreateSandboxParams::mcp`]: `mcp-gateway` exited with a
    /// non-zero code. The sandbox was killed. Holds the gateway's stderr (the
    /// SDKs' `"Failed to start MCP gateway: <stderr>"`).
    #[error("failed to start the MCP gateway: {0}")]
    McpGateway(String),

    /// An envd Connect-RPC call failed.
    ///
    /// `#[non_exhaustive]`: match it with `Error::Rpc { code, .. }`.
    #[error("envd RPC error ({code}): {message}")]
    #[non_exhaustive]
    Rpc {
        /// The Connect error code.
        code: ConnectCode,
        /// The human-readable error message.
        message: String,
    },

    /// envd's plain HTTP file API answered with a non-success status.
    ///
    /// `#[non_exhaustive]`: match it with `Error::Envd { status, .. }`.
    #[error("envd HTTP error (HTTP {status}): {message}")]
    #[non_exhaustive]
    Envd {
        /// The HTTP status code.
        status: u16,
        /// The `message` from the JSON error body, or the raw body.
        message: String,
    },

    /// The server sent something that violates the protocol, for example a
    /// truncated Connect envelope or a stream that ended without an
    /// end-of-stream frame.
    #[error("protocol error: {0}")]
    Protocol(String),

    /// A response body could not be decoded as the expected JSON shape.
    #[error("could not decode response: {0}")]
    Decode(#[from] serde_json::Error),

    /// A client-side deadline expired.
    #[error("timed out after {0:?}")]
    Timeout(Duration),

    /// The HTTP transport failed (connection refused, TLS failure, reset,
    /// request timeout, ...). [`TransportError`] says which.
    #[error("HTTP transport error: {0}")]
    Transport(#[source] TransportError),
}

impl Error {
    /// Wraps a reqwest error. reqwest stays private: callers see only
    /// [`TransportError`].
    ///
    /// Python note: Rust's `?` converts errors with a `From` impl. A
    /// `From<reqwest::Error>` impl would be public API (it would tie this
    /// crate's semver to reqwest's), so call sites convert explicitly with
    /// `.map_err(Error::transport)?`.
    pub(crate) fn transport(error: reqwest::Error) -> Self {
        Self::Transport(TransportError(error))
    }

    /// The HTTP status of a REST or envd HTTP error.
    ///
    /// # Returns
    ///
    /// The status for [`Error::Api`] and [`Error::Envd`], else `None`.
    /// [`Error::SandboxNotFound`] and [`Error::VolumeNotFound`] are 404s by
    /// definition and return `None`; an [`Error::Rpc`] has a code instead.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn demo(client: &e2b_client::Client) {
    /// if let Err(err) = client.get_sandbox_info("sbx-1").await {
    ///     if err.status() == Some(403) {
    ///         eprintln!("this key may not read that sandbox");
    ///     }
    /// }
    /// # }
    /// ```
    #[must_use]
    pub fn status(&self) -> Option<u16> {
        match self {
            Self::Api { status, .. } | Self::Envd { status, .. } => Some(*status),
            _ => None,
        }
    }

    /// The Connect code of an [`Error::Rpc`], else `None`.
    fn rpc_code(&self) -> Option<ConnectCode> {
        match self {
            Self::Rpc { code, .. } => Some(*code),
            _ => None,
        }
    }

    /// Whether this error means "the thing you asked about does not exist":
    /// a missing sandbox or volume, an RPC `not_found`, or an HTTP 404 from
    /// the REST or envd file API.
    ///
    /// # Returns
    ///
    /// `true` for [`Error::SandboxNotFound`], [`Error::VolumeNotFound`],
    /// [`Error::Rpc`] with [`ConnectCode::NotFound`], and
    /// [`Error::Api`]/[`Error::Envd`] with status 404.
    ///
    /// # Examples
    ///
    /// ```
    /// use e2b_client::Error;
    ///
    /// let err = Error::SandboxNotFound("sbx-1".into());
    /// assert!(err.is_not_found());
    /// ```
    #[must_use]
    pub fn is_not_found(&self) -> bool {
        match self {
            Self::SandboxNotFound(_) | Self::VolumeNotFound(_) => true,
            _ => self.status() == Some(404) || self.rpc_code() == Some(ConnectCode::NotFound),
        }
    }

    /// Whether the server refused the credentials: HTTP 401 from the REST
    /// or envd HTTP API, or an RPC `unauthenticated` (the SDKs'
    /// `AuthenticationException`, `py: api/__init__.py:164-168`,
    /// `py: envd/api.py:26`, `py: envd/rpc.py:28`).
    ///
    /// # Returns
    ///
    /// `true` for those errors. [`Error::MissingApiKey`] is a local
    /// configuration error, not a refusal, and returns `false`.
    #[must_use]
    pub fn is_auth(&self) -> bool {
        self.status() == Some(401) || self.rpc_code() == Some(ConnectCode::Unauthenticated)
    }

    /// Whether the server is rate limiting: HTTP 429, or an RPC
    /// `resource_exhausted` (the SDKs' `RateLimitException`,
    /// `py: api/__init__.py:170-174`, `py: envd/api.py:28-30`,
    /// `py: envd/rpc.py:31-33`).
    ///
    /// # Returns
    ///
    /// `true` for those errors.
    #[must_use]
    pub fn is_rate_limited(&self) -> bool {
        self.status() == Some(429) || self.rpc_code() == Some(ConnectCode::ResourceExhausted)
    }

    /// Whether the REST API refused the call as temporarily busy (HTTP 503):
    /// nothing was changed, so the same call can be retried after a short
    /// wait (the SDKs' `ServiceBusyException`, `py: api/__init__.py:176-180`).
    /// An envd 503 is not this: the SDKs raise it only for REST.
    ///
    /// # Returns
    ///
    /// `true` for [`Error::Api`] with status 503.
    #[must_use]
    pub fn is_service_busy(&self) -> bool {
        matches!(self, Self::Api { status: 503, .. })
    }

    /// Whether the sandbox's disk is full: envd answered HTTP 507 (the SDKs'
    /// `NotEnoughSpaceException`, `py: envd/api.py:32`).
    ///
    /// # Returns
    ///
    /// `true` for [`Error::Envd`] with status 507.
    #[must_use]
    pub fn is_not_enough_space(&self) -> bool {
        matches!(self, Self::Envd { status: 507, .. })
    }

    /// Whether envd could not be reached through the proxy: envd HTTP 502 or
    /// an RPC `unavailable`. The message says whether the sandbox is gone
    /// ("was not found") or running with envd's port closed ("port is not
    /// open") (the SDKs' `SandboxNotRunningException` and
    /// `SandboxUnreachableException`, `py: exceptions.py:4-36`,
    /// `py: envd/api.py:31`, `py: envd/rpc.py:30`).
    ///
    /// # Returns
    ///
    /// `true` for those errors.
    #[must_use]
    pub fn is_sandbox_unreachable(&self) -> bool {
        matches!(self, Self::Envd { status: 502, .. })
            || self.rpc_code() == Some(ConnectCode::Unavailable)
    }

    /// Whether a deadline ended the call: a client-side deadline
    /// ([`Error::Timeout`]), an RPC `deadline_exceeded` or `canceled` (the
    /// SDKs' `TimeoutException`, `py: envd/rpc.py:34-39`), or a request
    /// timeout in the transport.
    ///
    /// # Returns
    ///
    /// `true` for those errors.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Duration;
    /// use e2b_client::Error;
    ///
    /// assert!(Error::Timeout(Duration::from_secs(60)).is_timeout());
    /// ```
    #[must_use]
    pub fn is_timeout(&self) -> bool {
        match self {
            Self::Timeout(_) => true,
            Self::Rpc { code, .. } => {
                matches!(code, ConnectCode::DeadlineExceeded | ConnectCode::Canceled)
            }
            Self::Transport(transport) => transport.is_timeout(),
            _ => false,
        }
    }

    /// Whether retrying the same call later can succeed without changing
    /// it: the server is rate limiting ([`Error::is_rate_limited`]), the REST
    /// API is busy ([`Error::is_service_busy`]), or the connection was never
    /// made ([`TransportError::is_connect`]), so the request never reached
    /// the server. Timeouts are not included: a timed-out request may have
    /// taken effect.
    ///
    /// # Returns
    ///
    /// `true` for those errors.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::time::Duration;
    ///
    /// # async fn demo(client: &e2b_client::Client) -> e2b_client::Result<()> {
    /// let info = match client.get_sandbox_info("sbx-1").await {
    ///     Err(err) if err.is_retryable() => {
    ///         tokio::time::sleep(Duration::from_secs(1)).await;
    ///         client.get_sandbox_info("sbx-1").await?
    ///     }
    ///     other => other?,
    /// };
    /// println!("{:?}", info.state);
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn is_retryable(&self) -> bool {
        self.is_rate_limited()
            || self.is_service_busy()
            || matches!(self, Self::Transport(transport) if transport.is_connect())
    }
}

/// A failure of the HTTP transport under a call: the connection could not be
/// made, TLS failed, the connection dropped, or the request timed out.
///
/// The underlying HTTP client is an implementation detail, so this type is
/// opaque. Its `Display` is the transport's message, and
/// [`std::error::Error::source`] gives the cause below it (for example the
/// I/O error).
///
/// # Examples
///
/// ```no_run
/// use e2b_client::Error;
///
/// # async fn demo(client: &e2b_client::Client) {
/// if let Err(Error::Transport(transport)) = client.get_sandbox_info("sbx-1").await {
///     if transport.is_connect() {
///         eprintln!("could not reach the API: {transport}");
///     }
/// }
/// # }
/// ```
#[derive(Debug)]
pub struct TransportError(reqwest::Error);

impl TransportError {
    /// Whether the request timed out (the configured request timeout or a
    /// per-call one).
    ///
    /// # Returns
    ///
    /// `true` for a timeout.
    #[must_use]
    pub fn is_timeout(&self) -> bool {
        self.0.is_timeout()
    }

    /// Whether the connection could not be made (refused, DNS failure, TLS
    /// handshake), so the request never reached the server.
    ///
    /// # Returns
    ///
    /// `true` for a connection failure.
    #[must_use]
    pub fn is_connect(&self) -> bool {
        self.0.is_connect()
    }
}

impl fmt::Display for TransportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl std::error::Error for TransportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        // reqwest's own cause (an I/O or hyper error), not reqwest's error
        // itself, whose message `Display` already shows.
        self.0.source()
    }
}

/// The Connect protocol error codes (the gRPC status codes, spelled in
/// `snake_case` on the wire).
///
/// Source: `@connectrpc/connect` 2.2.0 `dist/cjs/code.js:31-91` (numeric values)
/// and `dist/cjs/protocol-connect/code-string.js:24-31` (wire spelling).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConnectCode {
    /// The operation was cancelled.
    Canceled,
    /// Unknown error.
    Unknown,
    /// The client sent an invalid argument.
    InvalidArgument,
    /// The deadline expired before the operation completed.
    DeadlineExceeded,
    /// The requested entity was not found.
    NotFound,
    /// The entity the client tried to create already exists.
    AlreadyExists,
    /// The caller is not allowed to perform the operation.
    PermissionDenied,
    /// A resource (quota, rate limit, disk) is exhausted.
    ResourceExhausted,
    /// The system is not in a state required for the operation.
    FailedPrecondition,
    /// The operation was aborted.
    Aborted,
    /// The operation was attempted past the valid range.
    OutOfRange,
    /// The operation is not implemented.
    Unimplemented,
    /// An internal server error.
    Internal,
    /// The service is unavailable.
    Unavailable,
    /// Unrecoverable data loss.
    DataLoss,
    /// The request lacks valid authentication.
    Unauthenticated,
}

impl ConnectCode {
    /// Parses the `snake_case` wire spelling of a code (e.g. `"not_found"`).
    ///
    /// # Arguments
    ///
    /// * `wire` - the `code` string from a Connect error JSON object.
    ///
    /// # Returns
    ///
    /// The matching code, or `None` for an unrecognised string.
    ///
    /// # Examples
    ///
    /// ```
    /// use e2b_client::ConnectCode;
    ///
    /// assert_eq!(ConnectCode::from_wire("already_exists"), Some(ConnectCode::AlreadyExists));
    /// assert_eq!(ConnectCode::from_wire("nope"), None);
    /// ```
    #[must_use]
    pub fn from_wire(wire: &str) -> Option<Self> {
        Some(match wire {
            "canceled" => Self::Canceled,
            "unknown" => Self::Unknown,
            "invalid_argument" => Self::InvalidArgument,
            "deadline_exceeded" => Self::DeadlineExceeded,
            "not_found" => Self::NotFound,
            "already_exists" => Self::AlreadyExists,
            "permission_denied" => Self::PermissionDenied,
            "resource_exhausted" => Self::ResourceExhausted,
            "failed_precondition" => Self::FailedPrecondition,
            "aborted" => Self::Aborted,
            "out_of_range" => Self::OutOfRange,
            "unimplemented" => Self::Unimplemented,
            "internal" => Self::Internal,
            "unavailable" => Self::Unavailable,
            "data_loss" => Self::DataLoss,
            "unauthenticated" => Self::Unauthenticated,
            _ => return None,
        })
    }

    /// The `snake_case` wire spelling of this code.
    ///
    /// # Returns
    ///
    /// A static string such as `"not_found"`.
    #[must_use]
    pub fn as_wire(self) -> &'static str {
        match self {
            Self::Canceled => "canceled",
            Self::Unknown => "unknown",
            Self::InvalidArgument => "invalid_argument",
            Self::DeadlineExceeded => "deadline_exceeded",
            Self::NotFound => "not_found",
            Self::AlreadyExists => "already_exists",
            Self::PermissionDenied => "permission_denied",
            Self::ResourceExhausted => "resource_exhausted",
            Self::FailedPrecondition => "failed_precondition",
            Self::Aborted => "aborted",
            Self::OutOfRange => "out_of_range",
            Self::Unimplemented => "unimplemented",
            Self::Internal => "internal",
            Self::Unavailable => "unavailable",
            Self::DataLoss => "data_loss",
            Self::Unauthenticated => "unauthenticated",
        }
    }

    /// The code to use for a plain (non-Connect) HTTP error response, such as
    /// a proxy answering in front of envd.
    ///
    /// This is the table the official Python SDK uses
    /// (`e2b/envd/client_shared.py:65-80`, `PLAIN_HTTP_ERROR_CODES`), which keeps
    /// 404 as `not_found` and 409 as `already_exists` so callers can branch on
    /// them; statuses without an entry map to `unknown`.
    ///
    /// # Arguments
    ///
    /// * `status` - the HTTP status code.
    ///
    /// # Returns
    ///
    /// The Connect code for that status.
    #[must_use]
    pub fn from_plain_http_status(status: u16) -> Self {
        match status {
            400 => Self::InvalidArgument,
            401 => Self::Unauthenticated,
            403 => Self::PermissionDenied,
            404 => Self::NotFound,
            409 => Self::AlreadyExists,
            413 | 429 => Self::ResourceExhausted,
            499 => Self::Canceled,
            500 => Self::Internal,
            501 | 505 => Self::Unimplemented,
            502 | 503 => Self::Unavailable,
            504 => Self::DeadlineExceeded,
            _ => Self::Unknown,
        }
    }
}

impl fmt::Display for ConnectCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_wire())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every code with its Connect wire spelling, written out so a wrong
    /// spelling cannot round-trip unnoticed.
    const ALL_CODES: [(ConnectCode, &str); 16] = [
        (ConnectCode::Canceled, "canceled"),
        (ConnectCode::Unknown, "unknown"),
        (ConnectCode::InvalidArgument, "invalid_argument"),
        (ConnectCode::DeadlineExceeded, "deadline_exceeded"),
        (ConnectCode::NotFound, "not_found"),
        (ConnectCode::AlreadyExists, "already_exists"),
        (ConnectCode::PermissionDenied, "permission_denied"),
        (ConnectCode::ResourceExhausted, "resource_exhausted"),
        (ConnectCode::FailedPrecondition, "failed_precondition"),
        (ConnectCode::Aborted, "aborted"),
        (ConnectCode::OutOfRange, "out_of_range"),
        (ConnectCode::Unimplemented, "unimplemented"),
        (ConnectCode::Internal, "internal"),
        (ConnectCode::Unavailable, "unavailable"),
        (ConnectCode::DataLoss, "data_loss"),
        (ConnectCode::Unauthenticated, "unauthenticated"),
    ];

    #[test]
    fn wire_spelling_round_trips_for_every_code() {
        for (code, wire) in ALL_CODES {
            assert_eq!(ConnectCode::from_wire(wire), Some(code));
            assert_eq!(code.as_wire(), wire);
            assert_eq!(code.to_string(), wire);
        }
    }

    #[test]
    fn unknown_wire_string_is_none() {
        assert_eq!(ConnectCode::from_wire("NotFound"), None);
    }

    #[test]
    fn plain_http_status_mapping_matches_python_sdk_table() {
        assert_eq!(
            ConnectCode::from_plain_http_status(404),
            ConnectCode::NotFound
        );
        assert_eq!(
            ConnectCode::from_plain_http_status(409),
            ConnectCode::AlreadyExists
        );
        assert_eq!(
            ConnectCode::from_plain_http_status(429),
            ConnectCode::ResourceExhausted
        );
        assert_eq!(
            ConnectCode::from_plain_http_status(502),
            ConnectCode::Unavailable
        );
        assert_eq!(
            ConnectCode::from_plain_http_status(504),
            ConnectCode::DeadlineExceeded
        );
        assert_eq!(
            ConnectCode::from_plain_http_status(418),
            ConnectCode::Unknown
        );
    }

    #[test]
    fn is_not_found_covers_each_family() {
        assert!(Error::SandboxNotFound("sbx".into()).is_not_found());
        assert!(Error::VolumeNotFound("vol".into()).is_not_found());
        assert!(
            Error::Api {
                status: 404,
                message: String::new()
            }
            .is_not_found()
        );
        assert!(
            Error::Envd {
                status: 404,
                message: String::new()
            }
            .is_not_found()
        );
        assert!(
            !Error::Api {
                status: 500,
                message: String::new()
            }
            .is_not_found()
        );
        assert!(!Error::McpGateway("bad config".into()).is_not_found());
        assert!(
            !Error::Rpc {
                code: ConnectCode::Internal,
                message: String::new()
            }
            .is_not_found()
        );
    }

    #[test]
    fn status_is_the_http_status_of_api_and_envd_only() {
        let api = Error::Api {
            status: 403,
            message: String::new(),
        };
        let envd = Error::Envd {
            status: 507,
            message: String::new(),
        };
        let rpc = Error::Rpc {
            code: ConnectCode::NotFound,
            message: String::new(),
        };
        assert_eq!(api.status(), Some(403));
        assert_eq!(envd.status(), Some(507));
        assert_eq!(rpc.status(), None);
        assert_eq!(Error::SandboxNotFound("sbx".into()).status(), None);
        assert_eq!(Error::Timeout(Duration::from_secs(1)).status(), None);
    }

    #[test]
    fn service_busy_is_a_rest_503_only() {
        assert!(
            Error::Api {
                status: 503,
                message: String::new()
            }
            .is_service_busy()
        );
        assert!(
            !Error::Envd {
                status: 503,
                message: String::new()
            }
            .is_service_busy()
        );
        assert!(
            !Error::Rpc {
                code: ConnectCode::Unavailable,
                message: String::new()
            }
            .is_service_busy()
        );
    }

    #[test]
    fn an_envd_502_and_an_rpc_unavailable_are_sandbox_unreachable() {
        let proxy = Error::Envd {
            status: 502,
            message: "The sandbox was not found".into(),
        };
        let rpc = Error::Rpc {
            code: ConnectCode::Unavailable,
            message: String::new(),
        };
        assert!(proxy.is_sandbox_unreachable());
        assert!(rpc.is_sandbox_unreachable());
        assert!(
            !Error::Api {
                status: 502,
                message: String::new()
            }
            .is_sandbox_unreachable()
        );
        assert!(!proxy.is_retryable());
    }

    #[test]
    fn timeouts_cover_the_client_deadline_and_the_rpc_codes() {
        assert!(Error::Timeout(Duration::from_millis(1)).is_timeout());
        for code in [ConnectCode::DeadlineExceeded, ConnectCode::Canceled] {
            assert!(
                Error::Rpc {
                    code,
                    message: String::new()
                }
                .is_timeout(),
                "{code}"
            );
        }
        assert!(
            !Error::Rpc {
                code: ConnectCode::Internal,
                message: String::new()
            }
            .is_timeout()
        );
        assert!(
            !Error::Api {
                status: 504,
                message: String::new()
            }
            .is_timeout()
        );
    }

    #[test]
    fn auth_and_rate_limits_cover_both_http_apis_and_rpc() {
        for error in [
            Error::Api {
                status: 401,
                message: String::new(),
            },
            Error::Envd {
                status: 401,
                message: String::new(),
            },
            Error::Rpc {
                code: ConnectCode::Unauthenticated,
                message: String::new(),
            },
        ] {
            assert!(error.is_auth(), "{error:?}");
            assert!(!error.is_retryable(), "{error:?}");
        }
        for error in [
            Error::Api {
                status: 429,
                message: String::new(),
            },
            Error::Envd {
                status: 429,
                message: String::new(),
            },
            Error::Rpc {
                code: ConnectCode::ResourceExhausted,
                message: String::new(),
            },
        ] {
            assert!(error.is_rate_limited(), "{error:?}");
            assert!(error.is_retryable(), "{error:?}");
        }
        assert!(!Error::MissingApiKey.is_auth());
        assert!(
            !Error::Api {
                status: 403,
                message: String::new()
            }
            .is_auth()
        );
    }
}
