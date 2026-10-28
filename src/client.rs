//! The REST control-plane client: create, connect (resume), inspect, list,
//! pause, kill, extend, snapshot and fork sandboxes, and manage volumes.

use std::sync::Arc;
use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::{Method, RequestBuilder, StatusCode};
use serde::{Deserialize, Serialize};

use crate::config::ConnectionConfig;
use crate::envd::json_message_or_text;
use crate::error::{Error, Result};
use crate::lifecycle::{ConnectOptions, PauseOptions};
use crate::metrics::{MetricsParams, SandboxMetrics};
use crate::models::{
    ConnectBody, CreateSandboxParams, ForkOptions, ForkResultWire, ListSandboxesParams, PauseBody,
    RefreshBody, SandboxCreated, SandboxInfo, SandboxPage, TimeoutBody,
};
use crate::network::NetworkUpdate;
use crate::sandbox::Sandbox;
use crate::snapshots::{ListSnapshotsParams, SnapshotInfo, SnapshotOptions, SnapshotPage};
use crate::volumes::{NewVolumeBody, Volume, VolumeInfo, VolumeWire};

/// The header carrying the team API key (`e2b/api/__init__.py:251`; JS
/// `dist/index.js:1528`).
const HEADER_API_KEY: &str = "x-api-key";
/// The response header carrying the next list page token
/// (`e2b/paginator.py:48-50`).
const HEADER_NEXT_TOKEN: &str = "x-next-token";
/// The request header carrying a create's idempotency key
/// (`e2b/api/client/api/sandboxes/post_v2_sandboxes.py:21`; JS
/// `dist/index.js:5568`).
const HEADER_IDEMPOTENCY_KEY: &str = "Idempotency-Key";
/// What hosted E2B's 409 for a pause says when the sandbox is already paused
/// (`Error pausing sandbox - sandbox '<id>' is already paused`, live
/// 2026-10-28). Used only for a pause that asked for a mode (issue #81).
const ALREADY_PAUSED: &str = "already paused";

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
            .build()
            .map_err(Error::transport)?;
        // envd gets its own client so the API key is never sent to sandboxes.
        let envd_http = reqwest::Client::builder()
            .user_agent(user_agent())
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(Error::transport)?;
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
    /// * `params` - template, timeout, metadata, environment variables,
    ///   network, lifecycle, volume mounts, an MCP configuration and an
    ///   optional idempotency key.
    ///
    /// # Returns
    ///
    /// A [`Sandbox`] handle with its envd access token and address resolved.
    /// With an MCP configuration ([`CreateSandboxParams::mcp`]), the gateway
    /// has been started, as the SDKs do, and [`Sandbox::mcp_token`] holds
    /// the token it was started with.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidArgument`] for an invalid idempotency key or
    /// lifecycle, before any request; [`Error::Api`] for a non-2xx answer,
    /// [`Error::Decode`] for an unexpected body, [`Error::Transport`] for
    /// connection failures. With an MCP configuration, if the gateway does
    /// not start the sandbox is killed and the error is
    /// [`Error::McpGateway`] with the gateway's stderr for a non-zero exit,
    /// or the error of the command call itself (for example
    /// [`Error::Rpc`]). If this future is dropped while the gateway is
    /// starting (for example by a `tokio::time::timeout` around the call),
    /// the kill does not run and the sandbox lives until its timeout, as
    /// in the SDKs.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::time::Duration;
    ///
    /// use e2b_client::{Client, CreateSandboxParams};
    ///
    /// # async fn demo(client: Client) -> e2b_client::Result<()> {
    /// let params = CreateSandboxParams::new("base")
    ///     .timeout(Duration::from_secs(120))
    ///     .metadata("purpose", "docs");
    /// let sandbox = client.create_sandbox(&params).await?;
    /// println!("{}", sandbox.sandbox_id());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_sandbox(&self, params: &CreateSandboxParams) -> Result<Sandbox> {
        // Every check runs before any request, in the SDK's order: the key
        // (`sandbox_async/sandbox_api.py:228`), then the lifecycle (`:232`).
        let idempotency_key = params.checked_idempotency_key()?;
        let body = params.body()?;
        let mut request = self
            .request(Method::POST, &["v2", "sandboxes"])?
            .json(&body);
        // `if let` runs the block only when the key is `Some`, like
        // `if key is not None:` in Python.
        if let Some(key) = idempotency_key {
            request = request.header(HEADER_IDEMPOTENCY_KEY, key);
        }
        let created: SandboxCreated =
            parse_success(request.send().await.map_err(Error::transport)?).await?;
        let sandbox = Sandbox::from_created(self.clone(), created)?;
        // `if let ... && let ...` (a let chain) runs the block only when an
        // MCP configuration is set and starting the gateway failed.
        if let Some(config) = params.mcp_config()
            && let Err(error) = sandbox.start_mcp_gateway(config).await
        {
            // As in the SDKs, a sandbox whose gateway failed is killed. The
            // kill's own result is ignored so the gateway error is reported.
            let _ = sandbox.kill().await;
            return Err(error);
        }
        Ok(sandbox)
    }

    /// Connects to a sandbox, resuming it first if it is paused
    /// (`POST /v2/sandboxes/{id}/connect`, the SDK's
    /// `connect(timeout, on_resume)`, `e2b/sandbox_async/sandbox_api.py:498-520`).
    ///
    /// [`ConnectOptions::timeout`](crate::ConnectOptions::timeout) sets a new
    /// lifetime from now; without one the sandbox keeps its current lifetime.
    /// [`OnResume::Reboot`](crate::OnResume::Reboot) sends `memory: false`;
    /// a restore, the API's default, sends no `memory` field.
    ///
    /// # Arguments
    ///
    /// * `sandbox_id` - the sandbox to connect to.
    /// * `options` - the timeout and the resume mode;
    ///   [`ConnectOptions::new`](crate::ConnectOptions::new) for neither.
    ///
    /// # Returns
    ///
    /// A [`Sandbox`] handle.
    ///
    /// # Errors
    ///
    /// [`Error::SandboxNotFound`] for HTTP 404, [`Error::Api`] for other
    /// non-2xx answers (the API documents refusing a reboot this way where
    /// that capability is not enabled), [`Error::Transport`] for connection
    /// failures.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::time::Duration;
    /// use e2b_client::{Client, ConnectOptions, OnResume};
    ///
    /// # async fn demo(client: &Client) -> e2b_client::Result<()> {
    /// // Resume from the memory snapshot, with ten minutes to live.
    /// let options = ConnectOptions::new().timeout(Duration::from_secs(600));
    /// let sandbox = client.connect_sandbox("sbx-1", &options).await?;
    /// println!("connected to {}", sandbox.sandbox_id());
    /// // Or boot it cold from its filesystem.
    /// let reboot = ConnectOptions::new().on_resume(OnResume::Reboot);
    /// let rebooted = client.connect_sandbox("sbx-2", &reboot).await?;
    /// println!("rebooted {}", rebooted.sandbox_id());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn connect_sandbox(
        &self,
        sandbox_id: &str,
        options: &ConnectOptions,
    ) -> Result<Sandbox> {
        let body = ConnectBody {
            timeout: options.timeout_value().map(|timeout| timeout.as_secs()),
            memory: options.memory(),
        };
        let response = self
            .send_json(
                Method::POST,
                &["v2", "sandboxes", sandbox_id, "connect"],
                &body,
            )
            .await?;
        // A 404 is the sandbox missing: there is no other route to try.
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
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn demo(client: e2b_client::Client, sandbox_id: &str) -> e2b_client::Result<()> {
    /// let info = client.get_sandbox_info(sandbox_id).await?;
    /// println!("{:?} until {}", info.state, info.end_at);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_sandbox_info(&self, sandbox_id: &str) -> Result<SandboxInfo> {
        let response = self
            .request(Method::GET, &["sandboxes", sandbox_id])?
            .send()
            .await
            .map_err(Error::transport)?;
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
            .await
            .map_err(Error::transport)?;
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
            .await
            .map_err(Error::transport)?;
        let next_token = next_token_of(&response);
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
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn demo(client: e2b_client::Client, sandbox_id: &str) -> e2b_client::Result<()> {
    /// if !client.kill_sandbox(sandbox_id).await? {
    ///     println!("{sandbox_id} was already gone");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn kill_sandbox(&self, sandbox_id: &str) -> Result<bool> {
        let response = self
            .request(Method::DELETE, &["sandboxes", sandbox_id])?
            .send()
            .await
            .map_err(Error::transport)?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(false);
        }
        ensure_success(response).await.map(|_| true)
    }

    /// Pauses a sandbox (`POST /sandboxes/{id}/pause`, the SDK's
    /// `pause(mode)`, `e2b/sandbox_async/sandbox_api.py:399-432`). Resume it
    /// with [`Client::connect_sandbox`].
    ///
    /// The body is `{"memory": true}` for [`SnapshotMode::Full`],
    /// `{"memory": false}` for [`SnapshotMode::Filesystem`], and `{}` with
    /// [`PauseOptions::new`](crate::PauseOptions::new), the API's default.
    ///
    /// # Arguments
    ///
    /// * `sandbox_id` - the sandbox to pause.
    /// * `options` - what the pause keeps.
    ///
    /// # Returns
    ///
    /// `true` if it was paused, `false` if it was already paused. Without a
    /// mode any HTTP 409 means already paused, as in the SDK. With a mode,
    /// only a 409 whose message says the sandbox "is already paused" does.
    ///
    /// # Errors
    ///
    /// [`Error::SandboxNotFound`] for HTTP 404, [`Error::Transport`] for
    /// connection failures, and [`Error::Api`] for other non-2xx answers,
    /// including, with a mode, any 409 that is not "already paused": the
    /// API refusing that mode (for example
    /// `snapshot_filesystem_only_unsupported_node`, the node predates
    /// filesystem-only snapshots). SDK 2.61 returns `false` for every 409
    /// (`e2b/sandbox_async/sandbox_api.py:421-423`); the crate departs from
    /// it here so a refusal is not mistaken for a pause. A filesystem-only
    /// pause refused for the team is an [`Error::Api`] with status 400.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use e2b_client::{Client, PauseOptions, SnapshotMode};
    ///
    /// # async fn demo(client: &Client) -> e2b_client::Result<()> {
    /// let options = PauseOptions::new().mode(SnapshotMode::Filesystem);
    /// if !client.pause_sandbox("sbx-1", &options).await? {
    ///     println!("it was already paused");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`SnapshotMode::Full`]: crate::SnapshotMode::Full
    /// [`SnapshotMode::Filesystem`]: crate::SnapshotMode::Filesystem
    pub async fn pause_sandbox(&self, sandbox_id: &str, options: &PauseOptions) -> Result<bool> {
        let body = PauseBody {
            memory: options.memory(),
        };
        let response = self
            .send_json(Method::POST, &["sandboxes", sandbox_id, "pause"], &body)
            .await?;
        match response.status() {
            StatusCode::NOT_FOUND => Err(Error::SandboxNotFound(sandbox_id.to_owned())),
            // No mode: every 409 is "already paused", as in the SDK.
            StatusCode::CONFLICT if body.memory.is_none() => Ok(false),
            // A mode was asked: only the "already paused" message is; any
            // other 409 is the API refusing that mode (issue #81).
            StatusCode::CONFLICT => match api_error(response).await {
                Error::Api { ref message, .. } if message.contains(ALREADY_PAUSED) => Ok(false),
                error => Err(error),
            },
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
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::time::Duration;
    ///
    /// # async fn demo(client: e2b_client::Client, sandbox_id: &str) -> e2b_client::Result<()> {
    /// client.set_sandbox_timeout(sandbox_id, Duration::from_secs(900)).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn set_sandbox_timeout(&self, sandbox_id: &str, timeout: Duration) -> Result<()> {
        let body = TimeoutBody {
            timeout: timeout.as_secs(),
        };
        let response = self
            .send_json(Method::POST, &["sandboxes", sandbox_id, "timeout"], &body)
            .await?;
        ensure_found_and_success(response, sandbox_id).await
    }

    /// Replaces a running sandbox's egress settings, without a restart
    /// (`PUT /sandboxes/{id}/network`). Fields the update leaves unset are
    /// cleared; see [`NetworkUpdate`].
    ///
    /// # Arguments
    ///
    /// * `sandbox_id` - the sandbox.
    /// * `update` - the new settings.
    ///
    /// # Errors
    ///
    /// [`Error::SandboxNotFound`] for HTTP 404, [`Error::Api`] for other
    /// non-2xx answers, [`Error::Transport`] for connection failures.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use e2b_client::{Client, NetworkUpdate};
    ///
    /// # async fn demo(client: Client, sandbox_id: &str) -> e2b_client::Result<()> {
    /// // Cut the sandbox off from the internet without restarting it.
    /// let update = NetworkUpdate::new().allow_internet_access(false);
    /// client.update_sandbox_network(sandbox_id, &update).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn update_sandbox_network(
        &self,
        sandbox_id: &str,
        update: &NetworkUpdate,
    ) -> Result<()> {
        let response = self
            .send_json(Method::PUT, &["sandboxes", sandbox_id, "network"], update)
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
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::time::Duration;
    ///
    /// # async fn demo(client: e2b_client::Client, sandbox_id: &str) -> e2b_client::Result<()> {
    /// client.refresh_sandbox(sandbox_id, Some(Duration::from_secs(60))).await?;
    /// # Ok(())
    /// # }
    /// ```
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

    /// Snapshots a sandbox (`POST /sandboxes/{id}/snapshots`). The sandbox
    /// pauses while the snapshot is taken, then keeps running. Snapshots are
    /// persistent: they outlive the sandbox until
    /// [`Client::delete_snapshot`] removes them.
    ///
    /// # Arguments
    ///
    /// * `sandbox_id` - the sandbox.
    /// * `options` - the name and mode.
    ///
    /// # Returns
    ///
    /// The snapshot; start sandboxes from it with
    /// `CreateSandboxParams::new(snapshot.snapshot_id)`.
    ///
    /// # Errors
    ///
    /// [`Error::SandboxNotFound`] for HTTP 404, [`Error::Api`] for other
    /// non-2xx answers (among them 400 or 409 for a refused filesystem-only
    /// snapshot, see [`SnapshotOptions::mode`]), [`Error::Decode`] for an
    /// unexpected body, [`Error::Transport`] for connection failures.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use e2b_client::{Client, CreateSandboxParams, SnapshotOptions};
    ///
    /// # async fn demo(client: Client) -> e2b_client::Result<()> {
    /// let options = SnapshotOptions::new().name("before-upgrade");
    /// let snapshot = client.create_snapshot("sandbox-id", &options).await?;
    /// let copy = client
    ///     .create_sandbox(&CreateSandboxParams::new(&snapshot.snapshot_id))
    ///     .await?;
    /// # let _ = copy;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_snapshot(
        &self,
        sandbox_id: &str,
        options: &SnapshotOptions,
    ) -> Result<SnapshotInfo> {
        let response = self
            .send_json(
                Method::POST,
                &["sandboxes", sandbox_id, "snapshots"],
                &options.body(),
            )
            .await?;
        parse_sandbox_response(response, sandbox_id).await
    }

    /// Lists snapshots, one page at a time (`GET /snapshots`).
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
    /// use e2b_client::{Client, ListSnapshotsParams};
    ///
    /// # async fn demo(client: Client) -> e2b_client::Result<()> {
    /// let mut params = ListSnapshotsParams::new();
    /// loop {
    ///     let page = client.list_snapshots(&params).await?;
    ///     for snapshot in &page.snapshots {
    ///         println!("{} {:?}", snapshot.snapshot_id, snapshot.names);
    ///     }
    ///     match page.next_token {
    ///         Some(token) => params = params.next_token(token),
    ///         None => break,
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list_snapshots(&self, params: &ListSnapshotsParams) -> Result<SnapshotPage> {
        let response = self
            .request(Method::GET, &["snapshots"])?
            .query(&params.query())
            .send()
            .await
            .map_err(Error::transport)?;
        let next_token = next_token_of(&response);
        // A `null` body reads as an empty page, as in the SDK
        // (`if res.parsed is None: return []`).
        let snapshots: Option<Vec<SnapshotInfo>> = parse_success(response).await?;
        Ok(SnapshotPage {
            snapshots: snapshots.unwrap_or_default(),
            next_token,
        })
    }

    /// Deletes a snapshot (`DELETE /templates/{id}`: a snapshot is a
    /// template). The ID is sent as one path segment, so a namespaced name
    /// such as `team/my-snapshot:v1` does not split the route.
    ///
    /// # Arguments
    ///
    /// * `snapshot_id` - the snapshot ID or name.
    ///
    /// # Returns
    ///
    /// `true` if it was deleted, `false` if it did not exist (HTTP 404).
    ///
    /// # Errors
    ///
    /// [`Error::Api`] for a non-2xx answer other than 404,
    /// [`Error::Transport`] for connection failures. Hosted E2B refuses to
    /// delete a snapshot while a sandbox created from it is running, with
    /// HTTP 400 `cannot delete template '<id>' because there are running
    /// sandboxes using it`: kill those sandboxes first.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn demo(client: e2b_client::Client) -> e2b_client::Result<()> {
    /// let deleted = client.delete_snapshot("team/my-snapshot:v1").await?;
    /// println!("deleted: {deleted}");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn delete_snapshot(&self, snapshot_id: &str) -> Result<bool> {
        let response = self
            .request(Method::DELETE, &["templates", snapshot_id])?
            .send()
            .await
            .map_err(Error::transport)?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(false);
        }
        ensure_success(response).await.map(|_| true)
    }

    /// Forks a running sandbox (`POST /sandboxes/{id}/fork`). The source is
    /// briefly paused, snapshotted with its memory and resumed, keeping its
    /// ID and expiry; every fork boots from that one snapshot, so each starts
    /// from the source's memory and filesystem.
    ///
    /// # Arguments
    ///
    /// * `sandbox_id` - the sandbox to fork.
    /// * `options` - the forks' timeout and count.
    ///
    /// # Returns
    ///
    /// One result per fork the API reports, in its order: a handle to a
    /// running fork, or why that fork failed. Forks succeed or fail
    /// independently. A per-fork error is [`Error::Api`] with the code and
    /// message the API gave for that fork. A per-fork 404 stays an
    /// [`Error::Api`], not [`Error::SandboxNotFound`]: it is about something
    /// that fork needed, not the source. [`Error::is_not_found`] is still
    /// `true` for it, as for any API 404, so check the source's existence
    /// by the call's own `Err`, not by a fork's. An entry with neither a
    /// sandbox nor an error is [`Error::Protocol`]. An entry that cannot be
    /// decoded is [`Error::Decode`], or [`Error::Protocol`] naming the
    /// sandbox when it has a `sandbox.sandboxID`, so a fork that may have
    /// started can still be killed; either way the other entries are
    /// unaffected. Every `Ok` fork is a billed sandbox: kill it when done.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidArgument`] for a count outside 1 to
    /// [`MAX_FORK_COUNT`](crate::MAX_FORK_COUNT), before any request;
    /// [`Error::SandboxNotFound`] for HTTP 404; [`Error::Api`] for other
    /// non-2xx answers; [`Error::Decode`] for a body that is not a JSON
    /// array; [`Error::Transport`] for connection failures.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use e2b_client::{Client, ForkOptions};
    ///
    /// # async fn demo(client: Client) -> e2b_client::Result<()> {
    /// let forks = client
    ///     .fork_sandbox("sandbox-id", &ForkOptions::new().count(2))
    ///     .await?;
    /// for fork in forks {
    ///     match fork {
    ///         Ok(sandbox) => println!("fork {}", sandbox.sandbox_id()),
    ///         Err(error) => eprintln!("a fork failed to start: {error}"),
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fork_sandbox(
        &self,
        sandbox_id: &str,
        options: &ForkOptions,
    ) -> Result<Vec<Result<Sandbox>>> {
        let body = options.body()?;
        let response = self
            .send_json(Method::POST, &["sandboxes", sandbox_id, "fork"], &body)
            .await?;
        // Each entry is decoded on its own, so one malformed entry does not
        // hide the forks that did start (each a running, billed sandbox).
        let entries: Vec<serde_json::Value> = parse_sandbox_response(response, sandbox_id).await?;
        Ok(entries
            .into_iter()
            .map(|entry| self.fork_result(entry))
            .collect())
    }

    /// One fork's outcome. An error wins over a sandbox, and an entry with
    /// neither is an error, as in the SDK
    /// (`e2b/sandbox_async/sandbox_api.py:474-493`).
    ///
    /// # Arguments
    ///
    /// * `entry` - one element of the fork response's array.
    ///
    /// # Returns
    ///
    /// The fork's handle.
    ///
    /// # Errors
    ///
    /// [`Error::Protocol`] naming the sandbox for an entry that cannot be
    /// decoded but has a `sandbox.sandboxID`, [`Error::Decode`] for any other
    /// entry that is not a `SandboxForkResult`, [`Error::Api`] for a per-fork
    /// error, [`Error::Protocol`] for an entry with neither field.
    fn fork_result(&self, entry: serde_json::Value) -> Result<Sandbox> {
        // `deserialize(&entry)` decodes from a borrow, so `entry` is still
        // there to read the sandbox ID from when decoding fails.
        let result = match ForkResultWire::deserialize(&entry) {
            Ok(result) => result,
            Err(error) => {
                let started = entry
                    .get("sandbox")
                    .and_then(|sandbox| sandbox.get("sandboxID"))
                    .and_then(serde_json::Value::as_str);
                return Err(match started {
                    // The fork may be running and billed: keep its ID so
                    // the caller can kill it.
                    Some(sandbox_id) => Error::Protocol(format!(
                        "the fork result for sandbox {sandbox_id} could not be decoded \
                         ({error}); if it started, kill it with `kill_sandbox`"
                    )),
                    None => Error::Decode(error),
                });
            }
        };
        match (result.sandbox, result.error) {
            (_, Some(error)) => Err(Error::Api {
                status: error.code,
                message: error.message,
            }),
            (Some(created), None) => Sandbox::from_created(self.clone(), created),
            (None, None) => Err(Error::Protocol(
                "a fork result has neither a sandbox nor an error".into(),
            )),
        }
    }

    /// Creates a volume (`POST /volumes`), the SDK's `Volume.create`.
    /// Volumes are persistent and billed: they outlive every sandbox that
    /// mounts them until [`Client::destroy_volume`] removes them.
    ///
    /// # Arguments
    ///
    /// * `name` - the volume name, which
    ///   [`CreateSandboxParams::volume_mount`] takes. The crate sends it as
    ///   given; the API decides which names it accepts.
    ///
    /// # Returns
    ///
    /// The volume, with its content token.
    ///
    /// # Errors
    ///
    /// [`Error::Api`] for a non-2xx answer: for example a name the API
    /// refuses, or HTTP 403 `use of volumes is not enabled` from hosted E2B
    /// for a team without volumes. [`Error::Decode`] for an unexpected body,
    /// [`Error::Transport`] for connection failures.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use e2b_client::{Client, CreateSandboxParams, WriteOptions};
    ///
    /// # async fn demo(client: Client) -> e2b_client::Result<()> {
    /// let volume = client.create_volume("data").await?;
    /// let params = CreateSandboxParams::new("base").volume_mount(volume.name(), "/mnt/data");
    /// let sandbox = client.create_sandbox(&params).await?;
    /// let files = sandbox.files();
    /// files.write("/mnt/data/kept.txt", "outlives the sandbox", &WriteOptions::default()).await?;
    /// sandbox.kill().await?;
    /// client.destroy_volume(volume.volume_id()).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_volume(&self, name: &str) -> Result<Volume> {
        let response = self
            .send_json(Method::POST, &["volumes"], &NewVolumeBody { name })
            .await?;
        let wire: VolumeWire = parse_success(response).await?;
        Ok(wire.into())
    }

    /// Looks up a volume (`GET /volumes/{id}`), the SDK's `Volume.get_info`
    /// and `Volume.connect`.
    ///
    /// # Arguments
    ///
    /// * `volume_id` - the volume ID ([`Volume::volume_id`]).
    ///
    /// # Returns
    ///
    /// The volume, with its content token.
    ///
    /// # Errors
    ///
    /// [`Error::VolumeNotFound`] for HTTP 404, [`Error::Api`] for other
    /// non-2xx answers (hosted E2B's volume IDs are UUIDs, and it answers
    /// 400 `Invalid volume ID` for an ID of another shape),
    /// [`Error::Decode`] for an unexpected body, [`Error::Transport`] for
    /// connection failures.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn demo(client: e2b_client::Client, volume_id: &str) -> e2b_client::Result<()> {
    /// let volume = client.get_volume(volume_id).await?;
    /// println!("{} is {}", volume.volume_id(), volume.name());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_volume(&self, volume_id: &str) -> Result<Volume> {
        let response = self
            .request(Method::GET, &["volumes", volume_id])?
            .send()
            .await
            .map_err(Error::transport)?;
        if response.status() == StatusCode::NOT_FOUND {
            return Err(Error::VolumeNotFound(volume_id.to_owned()));
        }
        let wire: VolumeWire = parse_success(response).await?;
        Ok(wire.into())
    }

    /// Lists the team's volumes (`GET /volumes`), the SDK's `Volume.list`.
    /// The API returns them in one response, without paging.
    ///
    /// # Returns
    ///
    /// Every volume's ID and name (no tokens); a `null` body reads as none,
    /// as in the SDK.
    ///
    /// # Errors
    ///
    /// [`Error::Api`] for a non-2xx answer, [`Error::Decode`] for an
    /// unexpected body, [`Error::Transport`] for connection failures.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn demo(client: e2b_client::Client) -> e2b_client::Result<()> {
    /// for volume in client.list_volumes().await? {
    ///     println!("{} {}", volume.volume_id, volume.name);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list_volumes(&self) -> Result<Vec<VolumeInfo>> {
        let response = self
            .request(Method::GET, &["volumes"])?
            .send()
            .await
            .map_err(Error::transport)?;
        // `Option` so a `null` body decodes to `None` (`if res.parsed is
        // None: return []`) instead of failing.
        let volumes: Option<Vec<VolumeInfo>> = parse_success(response).await?;
        Ok(volumes.unwrap_or_default())
    }

    /// Destroys a volume and its data (`DELETE /volumes/{id}`), the SDK's
    /// `Volume.destroy`.
    ///
    /// # Arguments
    ///
    /// * `volume_id` - the volume ID ([`Volume::volume_id`]).
    ///
    /// # Returns
    ///
    /// `true` if it was destroyed, `false` if it did not exist (HTTP 404).
    ///
    /// # Errors
    ///
    /// [`Error::Api`] for a non-2xx answer other than 404 (hosted E2B answers
    /// 400 `Invalid volume ID` for an ID that is not a UUID),
    /// [`Error::Transport`] for connection failures.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn demo(client: e2b_client::Client, volume_id: &str) -> e2b_client::Result<()> {
    /// if !client.destroy_volume(volume_id).await? {
    ///     println!("no volume {volume_id}");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn destroy_volume(&self, volume_id: &str) -> Result<bool> {
        let response = self
            .request(Method::DELETE, &["volumes", volume_id])?
            .send()
            .await
            .map_err(Error::transport)?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(false);
        }
        ensure_success(response).await.map(|_| true)
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
        self.request(method, segments)?
            .json(body)
            .send()
            .await
            .map_err(Error::transport)
    }
}

/// The next page token from `x-next-token` (`e2b/paginator.py:48-50`), or
/// `None` on the last page.
///
/// # Arguments
///
/// * `response` - a list response, borrowed so its body can still be read.
///
/// # Returns
///
/// The token, or `None` when the header is missing, empty or not ASCII.
fn next_token_of(response: &reqwest::Response) -> Option<String> {
    response
        .headers()
        .get(HEADER_NEXT_TOKEN)
        // `to_str` fails on a non-ASCII header; `.ok()` turns that into `None`.
        .and_then(|value| value.to_str().ok())
        .filter(|token| !token.is_empty())
        .map(str::to_owned)
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
        Err(err) => Error::transport(err),
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
    let body = ensure_success(response)
        .await?
        .bytes()
        .await
        .map_err(Error::transport)?;
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
