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
    #[error("E2B API error (HTTP {status}): {message}")]
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
    #[error("envd RPC error ({code}): {message}")]
    Rpc {
        /// The Connect error code.
        code: ConnectCode,
        /// The human-readable error message.
        message: String,
    },

    /// envd's plain HTTP file API answered with a non-success status.
    #[error("envd HTTP error (HTTP {status}): {message}")]
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

    /// The HTTP transport failed (connection refused, TLS failure, reset, ...).
    #[error("HTTP transport error: {0}")]
    Transport(#[from] reqwest::Error),
}

impl Error {
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
    /// use e2b_client::{ConnectCode, Error};
    ///
    /// let err = Error::Rpc { code: ConnectCode::NotFound, message: "no such pid".into() };
    /// assert!(err.is_not_found());
    /// ```
    #[must_use]
    pub fn is_not_found(&self) -> bool {
        match self {
            Self::SandboxNotFound(_) | Self::VolumeNotFound(_) => true,
            Self::Rpc { code, .. } => *code == ConnectCode::NotFound,
            Self::Api { status, .. } | Self::Envd { status, .. } => *status == 404,
            _ => false,
        }
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
}
