//! The REST control-plane client: create, connect (resume), inspect, list,
//! pause, kill, and extend sandboxes.

use std::sync::Arc;
use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::{Method, RequestBuilder, StatusCode};
use serde::Serialize;

use crate::config::ConnectionConfig;
use crate::envd::json_message_or_text;
use crate::error::{Error, Result};
use crate::metrics::{MetricsParams, SandboxMetrics};
use crate::models::{
    CreateSandboxParams, EmptyBody, ListSandboxesParams, RefreshBody, SandboxCreated, SandboxInfo,
    SandboxPage, TimeoutBody,
};
use crate::sandbox::Sandbox;

/// The header carrying the team API key (`e2b/api/__init__.py:251`; JS
/// `dist/index.js:1528`).
const HEADER_API_KEY: &str = "x-api-key";
/// The response header carrying the next list page token
/// (`e2b/paginator.py:48-50`).
const HEADER_NEXT_TOKEN: &str = "x-next-token";
/// The sandbox timeout the SDKs fall back to (`e2b/sandbox/main.py:30`); the
/// v1 connect body requires a timeout.
const DEFAULT_SANDBOX_TIMEOUT: Duration = Duration::from_secs(300);

/// The `User-Agent` sent on every request.
fn user_agent() -> String {
    format!("e2b-client-rs/{}", env!("CARGO_PKG_VERSION"))
}

/// An E2B API client.
///
/// Cloning is cheap: clones share one connection pool (the state sits behind
/// an [`Arc`], Rust's reference-counted shared pointer).
///
/// # Examples
///
/// ```no_run
/// use e2b_client::{Client, CreateSandboxParams};
///
/// # async fn demo() -> e2b_client::Result<()> {
/// let client = Client::from_env()?;
/// let sandbox = client.create_sandbox(&CreateSandboxParams::default()).await?;
/// println!("created {}", sandbox.sandbox_id());
/// sandbox.kill().await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct Client {
    inner: Arc<ClientInner>,
}

#[derive(Debug)]
struct ClientInner {
    config: ConnectionConfig,
    api_http: reqwest::Client,
    envd_http: reqwest::Client,
}

impl Client {
    /// Creates a client from a resolved configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - the connection configuration.
    ///
    /// # Returns
    ///
    /// The client.
    ///
    /// # Errors
    ///
    /// - [`Error::MissingApiKey`] if the configuration has no API key.
    /// - [`Error::InvalidConfig`] if the API key cannot be sent as a header.
    /// - [`Error::Transport`] if the HTTP client cannot be initialised.
    pub fn new(config: ConnectionConfig) -> Result<Self> {
        let api_key = config.api_key().ok_or(Error::MissingApiKey)?;
        let mut key_header = HeaderValue::from_str(api_key)
            .map_err(|_| Error::InvalidConfig("API key is not a valid HTTP header value".into()))?;
        key_header.set_sensitive(true);
        let mut api_headers = HeaderMap::with_capacity(1);
        api_headers.insert(HEADER_API_KEY, key_header);

        // Neither API redirects, and reqwest forwards custom headers such as
        // `X-API-KEY` and `X-Access-Token` across hosts on a redirect, so
        // redirects are never followed.
        let api_http = reqwest::Client::builder()
            .user_agent(user_agent())
            .default_headers(api_headers)
            .redirect(reqwest::redirect::Policy::none())
            .build()?;
        // envd gets its own client so the API key is never sent to sandboxes.
        let envd_http = reqwest::Client::builder()
            .user_agent(user_agent())
            .redirect(reqwest::redirect::Policy::none())
            .build()?;
        Ok(Self {
            inner: Arc::new(ClientInner {
                config,
                api_http,
                envd_http,
            }),
        })
    }

    /// Creates a client configured from `E2B_API_KEY`, `E2B_API_URL`,
    /// `E2B_DOMAIN` and `E2B_SANDBOX_URL`.
    ///
    /// # Returns
    ///
    /// The client.
    ///
    /// # Errors
    ///
    /// As [`ConnectionConfig::from_env`] and [`Client::new`].
    pub fn from_env() -> Result<Self> {
        Self::new(ConnectionConfig::from_env()?)
    }

    /// The configuration this client uses.
    ///
    /// # Returns
    ///
    /// The configuration.
    #[must_use]
    pub fn config(&self) -> &ConnectionConfig {
        &self.inner.config
    }

    pub(crate) fn envd_http(&self) -> &reqwest::Client {
        &self.inner.envd_http
    }

    /// Creates a sandbox (`POST /v2/sandboxes`).
    ///
    /// # Arguments
    ///
    /// * `params` - template, timeout, metadata and environment variables.
    ///
    /// # Returns
    ///
    /// A [`Sandbox`] handle with its envd access token and address resolved.
    ///
    /// # Errors
    ///
    /// [`Error::Api`] for a non-2xx answer, [`Error::Decode`] for an
    /// unexpected body, [`Error::Transport`] for connection failures.
    pub async fn create_sandbox(&self, params: &CreateSandboxParams) -> Result<Sandbox> {
        let response = self
            .send_json(Method::POST, &["v2", "sandboxes"], &params.body())
            .await?;
        let created: SandboxCreated = parse_success(response).await?;
        Sandbox::from_created(self.clone(), created)
    }

    /// Connects to a sandbox, resuming it first if it is paused
    /// (`POST /v2/sandboxes/{id}/connect`).
    ///
    /// If the server has no v2 route (HTTP 404) the v1 route
    /// `POST /sandboxes/{id}/connect`, also part of the E2B API, is tried
    /// before reporting the sandbox as missing.
    ///
    /// # Arguments
    ///
    /// * `sandbox_id` - the sandbox to connect to.
    /// * `timeout` - a new sandbox lifetime from now; `None` sends none on
    ///   the v2 route, so the server keeps its default. The v1 route requires
    ///   a timeout, so on the fallback `None` sends 300 seconds, which resets
    ///   the sandbox's lifetime to 5 minutes from now.
    ///
    /// # Returns
    ///
    /// A [`Sandbox`] handle.
    ///
    /// # Errors
    ///
    /// [`Error::SandboxNotFound`] if neither route knows the sandbox,
    /// [`Error::Api`] for other non-2xx answers, [`Error::Transport`] for
    /// connection failures.
    pub async fn connect_sandbox(
        &self,
        sandbox_id: &str,
        timeout: Option<Duration>,
    ) -> Result<Sandbox> {
        let v2_path = ["v2", "sandboxes", sandbox_id, "connect"];
        let response = match timeout {
            Some(timeout) => {
                self.send_json(
                    Method::POST,
                    &v2_path,
                    &TimeoutBody {
                        timeout: timeout.as_secs(),
                    },
                )
                .await?
            }
            None => {
                self.send_json(Method::POST, &v2_path, &EmptyBody {})
                    .await?
            }
        };
        let response = if response.status() == StatusCode::NOT_FOUND {
            let body = TimeoutBody {
                timeout: timeout.unwrap_or(DEFAULT_SANDBOX_TIMEOUT).as_secs(),
            };
            self.send_json(Method::POST, &["sandboxes", sandbox_id, "connect"], &body)
                .await?
        } else {
            response
        };
        let created: SandboxCreated = parse_sandbox_response(response, sandbox_id).await?;
        Sandbox::from_created(self.clone(), created)
    }

    /// Gets information about a sandbox (`GET /sandboxes/{id}`).
    ///
    /// # Arguments
    ///
    /// * `sandbox_id` - the sandbox to look up.
    ///
    /// # Returns
    ///
    /// The sandbox information.
    ///
    /// # Errors
    ///
    /// [`Error::SandboxNotFound`] for HTTP 404, [`Error::Api`] for other
    /// non-2xx answers, [`Error::Transport`] for connection failures.
    pub async fn get_sandbox_info(&self, sandbox_id: &str) -> Result<SandboxInfo> {
        let response = self
            .request(Method::GET, &["sandboxes", sandbox_id])?
            .send()
            .await?;
        parse_sandbox_response(response, sandbox_id).await
    }

    /// Reads a sandbox's CPU, memory and disk samples
    /// (`GET /sandboxes/{id}/metrics`).
    ///
    /// # Arguments
    ///
    /// * `sandbox_id` - the sandbox.
    /// * `params` - the optional time range.
    ///
    /// # Returns
    ///
    /// The samples, in the order the API returns them; empty if there are
    /// none yet.
    ///
    /// # Errors
    ///
    /// [`Error::SandboxNotFound`] for HTTP 404, [`Error::Api`] for other
    /// non-2xx answers, [`Error::Decode`] for an unexpected body,
    /// [`Error::Transport`] for connection failures.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use e2b_client::{Client, MetricsParams};
    ///
    /// # async fn demo(client: Client) -> e2b_client::Result<()> {
    /// let samples = client
    ///     .get_sandbox_metrics("sandbox-id", &MetricsParams::new())
    ///     .await?;
    /// if let Some(latest) = samples.last() {
    ///     println!("{:.1}% CPU, {} bytes used", latest.cpu_used_pct, latest.mem_used);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_sandbox_metrics(
        &self,
        sandbox_id: &str,
        params: &MetricsParams,
    ) -> Result<Vec<SandboxMetrics>> {
        let response = self
            .request(Method::GET, &["sandboxes", sandbox_id, "metrics"])?
            .query(&params.query())
            .send()
            .await?;
        // The API may answer `null` for "no samples"; `Option` accepts it,
        // like `list_sandboxes` does.
        let metrics: Option<Vec<SandboxMetrics>> =
            parse_sandbox_response(response, sandbox_id).await?;
        Ok(metrics.unwrap_or_default())
    }

    /// Lists sandboxes, one page at a time (`GET /v2/sandboxes`).
    ///
    /// # Arguments
    ///
    /// * `params` - filters and the page token.
    ///
    /// # Returns
    ///
    /// One page, with the token for the next page if there is one.
    ///
    /// # Errors
    ///
    /// [`Error::Api`] for a non-2xx answer, [`Error::Decode`] for an
    /// unexpected body, [`Error::Transport`] for connection failures.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use e2b_client::{Client, ListSandboxesParams};
    ///
    /// # async fn demo(client: Client) -> e2b_client::Result<()> {
    /// let mut params = ListSandboxesParams::new();
    /// loop {
    ///     let page = client.list_sandboxes(&params).await?;
    ///     for sandbox in &page.sandboxes {
    ///         println!("{} {:?}", sandbox.sandbox_id, sandbox.state);
    ///     }
    ///     match page.next_token {
    ///         Some(token) => params = params.next_token(token),
    ///         None => break,
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list_sandboxes(&self, params: &ListSandboxesParams) -> Result<SandboxPage> {
        let response = self
            .request(Method::GET, &["v2", "sandboxes"])?
            .query(&params.query())
            .send()
            .await?;
        let next_token = response
            .headers()
            .get(HEADER_NEXT_TOKEN)
            .and_then(|value| value.to_str().ok())
            .filter(|token| !token.is_empty())
            .map(str::to_owned);
        let sandboxes: Option<Vec<SandboxInfo>> = parse_success(response).await?;
        Ok(SandboxPage {
            sandboxes: sandboxes.unwrap_or_default(),
            next_token,
        })
    }

    /// Kills a sandbox (`DELETE /sandboxes/{id}`).
    ///
    /// # Arguments
    ///
    /// * `sandbox_id` - the sandbox to kill.
    ///
    /// # Returns
    ///
    /// `true` if it was killed, `false` if it did not exist.
    ///
    /// # Errors
    ///
    /// [`Error::Api`] for a non-2xx answer other than 404,
    /// [`Error::Transport`] for connection failures.
    pub async fn kill_sandbox(&self, sandbox_id: &str) -> Result<bool> {
        let response = self
            .request(Method::DELETE, &["sandboxes", sandbox_id])?
            .send()
            .await?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(false);
        }
        ensure_success(response).await.map(|_| true)
    }

    /// Pauses a sandbox (`POST /sandboxes/{id}/pause`). Resume it with
    /// [`Client::connect_sandbox`].
    ///
    /// # Arguments
    ///
    /// * `sandbox_id` - the sandbox to pause.
    ///
    /// # Returns
    ///
    /// `true` if it was paused, `false` if it was already paused (HTTP 409).
    ///
    /// # Errors
    ///
    /// [`Error::SandboxNotFound`] for HTTP 404, [`Error::Api`] for other
    /// non-2xx answers, [`Error::Transport`] for connection failures.
    pub async fn pause_sandbox(&self, sandbox_id: &str) -> Result<bool> {
        let response = self
            .send_json(
                Method::POST,
                &["sandboxes", sandbox_id, "pause"],
                &EmptyBody {},
            )
            .await?;
        match response.status() {
            StatusCode::NOT_FOUND => Err(Error::SandboxNotFound(sandbox_id.to_owned())),
            StatusCode::CONFLICT => Ok(false),
            _ => ensure_success(response).await.map(|_| true),
        }
    }

    /// Sets a sandbox's remaining lifetime, counted from now
    /// (`POST /sandboxes/{id}/timeout`).
    ///
    /// # Arguments
    ///
    /// * `sandbox_id` - the sandbox.
    /// * `timeout` - the new lifetime, sent in whole seconds.
    ///
    /// # Errors
    ///
    /// [`Error::SandboxNotFound`] for HTTP 404, [`Error::Api`] for other
    /// non-2xx answers, [`Error::Transport`] for connection failures.
    pub async fn set_sandbox_timeout(&self, sandbox_id: &str, timeout: Duration) -> Result<()> {
        let body = TimeoutBody {
            timeout: timeout.as_secs(),
        };
        let response = self
            .send_json(Method::POST, &["sandboxes", sandbox_id, "timeout"], &body)
            .await?;
        ensure_found_and_success(response, sandbox_id).await
    }

    /// Extends a sandbox's life (`POST /sandboxes/{id}/refreshes`).
    ///
    /// # Arguments
    ///
    /// * `sandbox_id` - the sandbox.
    /// * `duration` - how long to extend by, in whole seconds; `None` uses the
    ///   server default.
    ///
    /// # Errors
    ///
    /// [`Error::SandboxNotFound`] for HTTP 404, [`Error::Api`] for other
    /// non-2xx answers, [`Error::Transport`] for connection failures.
    pub async fn refresh_sandbox(
        &self,
        sandbox_id: &str,
        duration: Option<Duration>,
    ) -> Result<()> {
        let body = RefreshBody {
            duration: duration.map(|duration| duration.as_secs()),
        };
        let response = self
            .send_json(Method::POST, &["sandboxes", sandbox_id, "refreshes"], &body)
            .await?;
        ensure_found_and_success(response, sandbox_id).await
    }

    /// Builds a request to `api_url` plus the given path segments, each
    /// percent-encoded as a single segment.
    fn request(&self, method: Method, segments: &[&str]) -> Result<RequestBuilder> {
        let mut url = self.inner.config.api_url().clone();
        url.path_segments_mut()
            .map_err(|()| Error::InvalidConfig("API URL cannot be a base URL".into()))?
            .pop_if_empty()
            .extend(segments);
        let builder = self.inner.api_http.request(method, url);
        Ok(match self.inner.config.request_timeout() {
            Some(timeout) => builder.timeout(timeout),
            None => builder,
        })
    }

    async fn send_json<B: Serialize>(
        &self,
        method: Method,
        segments: &[&str],
        body: &B,
    ) -> Result<reqwest::Response> {
        Ok(self.request(method, segments)?.json(body).send().await?)
    }
}

/// Converts a non-2xx response into [`Error::Api`], keeping the E2B error
/// body's `message` when present.
async fn api_error(response: reqwest::Response) -> Error {
    let status = response.status().as_u16();
    match response.bytes().await {
        Ok(body) => {
            let message = if body.is_empty() {
                format!("HTTP {status}")
            } else {
                json_message_or_text(&body)
            };
            Error::Api { status, message }
        }
        Err(err) => Error::Transport(err),
    }
}

async fn ensure_success(response: reqwest::Response) -> Result<reqwest::Response> {
    if response.status().is_success() {
        Ok(response)
    } else {
        Err(api_error(response).await)
    }
}

async fn ensure_found_and_success(response: reqwest::Response, sandbox_id: &str) -> Result<()> {
    if response.status() == StatusCode::NOT_FOUND {
        return Err(Error::SandboxNotFound(sandbox_id.to_owned()));
    }
    ensure_success(response).await.map(|_| ())
}

async fn parse_success<T: serde::de::DeserializeOwned>(response: reqwest::Response) -> Result<T> {
    let body = ensure_success(response).await?.bytes().await?;
    Ok(serde_json::from_slice(&body)?)
}

async fn parse_sandbox_response<T: serde::de::DeserializeOwned>(
    response: reqwest::Response,
    sandbox_id: &str,
) -> Result<T> {
    if response.status() == StatusCode::NOT_FOUND {
        return Err(Error::SandboxNotFound(sandbox_id.to_owned()));
    }
    parse_success(response).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_without_api_key_is_rejected() {
        let config = ConnectionConfig::builder()
            .build_with_env(|_| None)
            .expect("config");
        assert!(matches!(Client::new(config), Err(Error::MissingApiKey)));
    }

    #[test]
    fn api_key_with_newline_is_rejected() {
        let config = ConnectionConfig::builder()
            .api_key("bad\nkey")
            .build_with_env(|_| None)
            .expect("cfg");
        assert!(matches!(Client::new(config), Err(Error::InvalidConfig(_))));
    }

    #[test]
    fn request_urls_encode_each_segment() {
        let config = ConnectionConfig::builder()
            .api_key("k")
            .api_url("https://api.example.test/prefix/")
            .build_with_env(|_| None)
            .expect("config");
        let client = Client::new(config).expect("client");
        let request = client
            .request(Method::GET, &["sandboxes", "a/b c"])
            .expect("url")
            .build()
            .expect("req");
        assert_eq!(
            request.url().as_str(),
            "https://api.example.test/prefix/sandboxes/a%2Fb%20c"
        );
    }
}
