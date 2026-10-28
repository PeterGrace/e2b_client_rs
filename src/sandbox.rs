//! A handle to one running sandbox.

use std::fmt;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::Engine;
use base64::engine::general_purpose::STANDARD_NO_PAD;
use reqwest::{Method, StatusCode};
use secrecy::{ExposeSecret, SecretString};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use tokio::sync::OnceCell;
use url::Url;

use crate::client::Client;
use crate::commands::{CommandOptions, Commands};
use crate::config::RequestTimeout;
use crate::envd::{EnvdClient, EnvdTarget, EnvdVersion, envd_http_error};
use crate::error::{Error, Result};
use crate::filesystem::{FILES_ROUTE, FileOptions, Filesystem};
use crate::lifecycle::PauseOptions;
use crate::mcp::{MCP_PORT, MCP_TOKEN_ENV, MCP_TOKEN_PATH, MCP_USER, gateway_command};
use crate::metrics::{MetricsParams, SandboxMetrics};
use crate::models::{ForkOptions, SandboxCreated, SandboxInfo};
use crate::network::NetworkUpdate;
use crate::pty::Pty;
use crate::snapshots::{ListSnapshotsParams, SnapshotInfo, SnapshotOptions, SnapshotPage};

/// envd's health route (`e2b/envd/api.py:22`).
const ENVD_HEALTH_ROUTE: &str = "/health";

/// Options for [`Sandbox::upload_url`] and [`Sandbox::download_url`].
///
/// The default reads or writes as envd's default user (`"user"` on envd
/// older than 0.4.0) with a URL that does not expire.
///
/// # Examples
///
/// ```
/// use std::time::Duration;
/// use e2b_client::FileUrlOptions;
///
/// let options = FileUrlOptions::default()
///     .user("root")
///     .expires_in(Duration::from_secs(300));
/// # let _ = options;
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileUrlOptions {
    user: Option<String>,
    expires_in: Option<Duration>,
}

impl FileUrlOptions {
    /// Sets the user the URL reads or writes as.
    ///
    /// # Arguments
    ///
    /// * `user` - the sandbox user, e.g. `"root"`. `impl Into<String>`
    ///   accepts both `&str` and `String`.
    ///
    /// # Returns
    ///
    /// The options, for chaining.
    #[must_use]
    pub fn user(mut self, user: impl Into<String>) -> Self {
        self.user = Some(user.into());
        self
    }

    /// Makes the URL stop working this long after it is made. Only a
    /// secured sandbox (one with an envd access token, which every hosted
    /// sandbox is) can sign an expiry.
    ///
    /// The expiry is counted in whole seconds, as in the SDKs: the URL
    /// expires at the current Unix second plus `expires_in`'s whole
    /// seconds, and any fraction of a second is dropped. Making the URL
    /// fails with [`crate::Error::InvalidArgument`] if `expires_in` is
    /// under one second, since that URL would already be expired.
    ///
    /// # Arguments
    ///
    /// * `expires_in` - how long the URL stays valid, at least one second.
    ///
    /// # Returns
    ///
    /// The options, for chaining.
    #[must_use]
    pub fn expires_in(mut self, expires_in: Duration) -> Self {
        self.expires_in = Some(expires_in);
        self
    }
}

/// What a signed URL lets its holder do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileOperation {
    Read,
    Write,
}

impl FileOperation {
    /// The operation's name inside the signed string.
    fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
        }
    }
}

/// The SDKs' v1 file signature: `v1_` + unpadded standard base64 of SHA-256
/// over `<path>:<operation>:<user>:<token>[:<expiration>]`, with an empty
/// user for envd's default user (`e2b/sandbox/signature.py:15-47`).
///
/// # Arguments
///
/// * `path` - the file path, exactly as sent in the `path` query pair.
/// * `operation` - read (download) or write (upload).
/// * `user` - the user named in the URL, if any.
/// * `token` - the sandbox's envd access token.
/// * `expiration` - the expiry as Unix seconds, if any.
///
/// # Returns
///
/// The `signature` query value.
fn file_signature(
    path: &str,
    operation: FileOperation,
    user: Option<&str>,
    token: &str,
    expiration: Option<u64>,
) -> String {
    let user = user.unwrap_or_default();
    let operation = operation.as_str();
    let raw = match expiration {
        Some(expiration) => format!("{path}:{operation}:{user}:{token}:{expiration}"),
        None => format!("{path}:{operation}:{user}:{token}"),
    };
    format!(
        "v1_{}",
        STANDARD_NO_PAD.encode(Sha256::digest(raw.as_bytes()))
    )
}

/// The Unix time, in whole seconds, `expires_in` from now: the current
/// second plus `expires_in`'s whole seconds, which is the SDKs'
/// `int(time.time()) + expiration_in_seconds`
/// (`e2b/sandbox/signature.py:28-32`).
///
/// # Errors
///
/// [`Error::InvalidArgument`] if `expires_in` is under one second (the URL
/// would expire on arrival), the system clock is before 1970, or the sum
/// overflows.
fn expiration_from_now(expires_in: Duration) -> Result<u64> {
    let seconds = expires_in.as_secs();
    if seconds == 0 {
        return Err(Error::InvalidArgument(
            "a URL expiration must be at least one second".into(),
        ));
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| Error::InvalidArgument("the system clock is before 1970".into()))?;
    now.as_secs()
        .checked_add(seconds)
        .ok_or_else(|| Error::InvalidArgument("the URL expiration overflows".into()))
}

/// A sandbox you created or connected to.
///
/// It bundles the REST lifecycle calls for this sandbox with access to the
/// sandbox's envd daemon through [`Sandbox::commands`], [`Sandbox::pty`] and
/// [`Sandbox::files`].
/// Cloning is cheap and clones share connections.
///
/// # Examples
///
/// ```no_run
/// use e2b_client::{Client, CommandOptions, CreateSandboxParams, WriteOptions};
///
/// # async fn demo() -> e2b_client::Result<()> {
/// let client = Client::from_env()?;
/// let sandbox = client.create_sandbox(&CreateSandboxParams::new("base")).await?;
/// let result = sandbox.commands().run("echo hello", &CommandOptions::default()).await?;
/// assert_eq!(result.stdout, "hello\n");
/// sandbox.files().write("/tmp/note.txt", "hi", &WriteOptions::default()).await?;
/// sandbox.kill().await?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct Sandbox {
    client: Client,
    sandbox_id: String,
    template_id: String,
    domain: Option<String>,
    traffic_access_token: Option<SecretString>,
    envd: Arc<EnvdClient>,
    // The MCP gateway's token, filled once (`tokio::sync::OnceCell`, an
    // async `functools.cached_property`). The `Arc` makes clones share it,
    // as they share the envd connection.
    mcp_token: Arc<OnceCell<SecretString>>,
}

impl fmt::Debug for Sandbox {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Sandbox")
            .field("sandbox_id", &self.sandbox_id)
            .field("template_id", &self.template_id)
            .field("domain", &self.domain)
            .field("envd_version", &self.envd.version())
            .finish_non_exhaustive()
    }
}

impl Sandbox {
    /// Builds the handle from a create/connect response.
    pub(crate) fn from_created(client: Client, created: SandboxCreated) -> Result<Self> {
        let access_token = created.envd_access_token.map(SecretString::from);
        let target = EnvdTarget {
            sandbox_id: &created.sandbox_id,
            sandbox_domain: created.domain.as_deref(),
            access_token: access_token.as_ref(),
            version: EnvdVersion::parse_lenient(&created.envd_version),
        };
        let envd = EnvdClient::new(client.envd_http().clone(), client.config(), &target)?;
        Ok(Self {
            client,
            sandbox_id: created.sandbox_id,
            template_id: created.template_id,
            domain: created.domain,
            traffic_access_token: created.traffic_access_token.map(SecretString::from),
            envd: Arc::new(envd),
            mcp_token: Arc::new(OnceCell::new()),
        })
    }

    /// The sandbox ID.
    ///
    /// # Returns
    ///
    /// The ID.
    #[must_use]
    pub fn sandbox_id(&self) -> &str {
        &self.sandbox_id
    }

    /// The template the sandbox was created from.
    ///
    /// # Returns
    ///
    /// The template ID (empty if the server did not report it).
    #[must_use]
    pub fn template_id(&self) -> &str {
        &self.template_id
    }

    /// The domain the API reported for this sandbox, if any.
    ///
    /// # Returns
    ///
    /// The sandbox domain.
    #[must_use]
    pub fn domain(&self) -> Option<&str> {
        self.domain.as_deref()
    }

    /// The envd version running in the sandbox.
    ///
    /// # Returns
    ///
    /// The version.
    #[must_use]
    pub fn envd_version(&self) -> EnvdVersion {
        self.envd.version()
    }

    /// The token for reaching non-envd ports of a sandbox with restricted
    /// public traffic, if the API issued one.
    ///
    /// # Returns
    ///
    /// The token, wrapped so it is not printed by accident.
    #[must_use]
    pub fn traffic_access_token(&self) -> Option<&SecretString> {
        self.traffic_access_token.as_ref()
    }

    /// The host name that reaches `port` inside this sandbox, for example to
    /// open `https://<host>/` on a web server the sandbox runs. The SDKs'
    /// `get_host(port)`.
    ///
    /// When the sandbox was created with public traffic disallowed, requests
    /// to the host also need [`Sandbox::traffic_access_token`].
    ///
    /// # Arguments
    ///
    /// * `port` - the port the service listens on inside the sandbox.
    ///
    /// # Returns
    ///
    /// The host name, without a scheme, e.g. `3000-<sandbox id>.e2b.app`.
    #[must_use]
    pub fn get_host(&self, port: u16) -> String {
        // `as_deref` turns the stored `Option<String>` into the
        // `Option<&str>` the config takes, borrowing instead of cloning.
        self.client
            .config()
            .sandbox_host(&self.sandbox_id, self.domain.as_deref(), port)
    }

    /// The URL of the sandbox's MCP gateway, `https://<get_host(50005)>/mcp`
    /// (`e2b/sandbox/main.py:226-232`). It is only served on a sandbox
    /// created with [`crate::CreateSandboxParams::mcp`]; send
    /// [`Sandbox::mcp_token`] with each request.
    ///
    /// # Returns
    ///
    /// The URL, e.g. `https://50005-<sandbox id>.e2b.app/mcp`.
    #[must_use]
    pub fn mcp_url(&self) -> String {
        format!("https://{}/mcp", self.get_host(MCP_PORT))
    }

    /// The MCP gateway's access token: the one [`Client::create_sandbox`]
    /// generated, or, on a handle from [`Client::connect_sandbox`], read
    /// once from `/etc/mcp-gateway/.token` as root and then kept, as the
    /// SDKs' `get_mcp_token` does. Unlike the SDKs, trailing whitespace is
    /// trimmed from the file, so the token can go in a header as it is.
    ///
    /// A failed read is not kept: the next call reads the file again.
    ///
    /// # Returns
    ///
    /// The token, wrapped so it is not printed by accident. Clones of this
    /// handle share it.
    ///
    /// # Errors
    ///
    /// The errors of [`Filesystem::read_to_string`], for example
    /// [`Error::Envd`] with status 404 on a sandbox without a gateway.
    pub async fn mcp_token(&self) -> Result<&SecretString> {
        // `get_or_try_init` runs the closure only while the cell is empty,
        // and stores its value only when it returns `Ok`.
        self.mcp_token
            .get_or_try_init(|| async {
                let token = self
                    .files()
                    .read_to_string(MCP_TOKEN_PATH, &FileOptions::new().user(MCP_USER))
                    .await?;
                // Trimmed, unlike the SDK: the token is a UUID, and a
                // trailing newline would make it unusable in a header.
                Ok(SecretString::from(token.trim_end().to_owned()))
            })
            .await
    }

    /// Starts the MCP gateway as the SDKs do
    /// (`e2b/sandbox_async/main.py:245-268`): `mcp-gateway --config '<json>'`
    /// as root, with a new random UUID v4 in `GATEWAY_ACCESS_TOKEN`, and
    /// keeps that token for [`Sandbox::mcp_token`]. The token goes only in
    /// the environment, never on the command line.
    ///
    /// # Arguments
    ///
    /// * `config` - the MCP server configuration.
    ///
    /// # Errors
    ///
    /// [`Error::McpGateway`] with the gateway's stderr if it exits non-zero;
    /// the errors of [`Commands::run`].
    pub(crate) async fn start_mcp_gateway(&self, config: &Map<String, Value>) -> Result<()> {
        let command = gateway_command(config)?;
        let token = SecretString::from(uuid::Uuid::new_v4().to_string());
        // `CommandOptions::env` keeps its own plain `String` copy of the
        // token, and so does the request body. Building the options inline
        // makes them a temporary, dropped at the end of this statement, so
        // that copy is gone before the token is stored below.
        let result = self
            .commands()
            .run(
                &command,
                &CommandOptions::default()
                    .user(MCP_USER)
                    .env(MCP_TOKEN_ENV, token.expose_secret()),
            )
            .await?;
        if !result.success() {
            return Err(Error::McpGateway(result.stderr));
        }
        // `set` fails only when the cell is already filled, which it never
        // is on a handle fresh from `create_sandbox`, so the result is
        // ignored.
        let _ = self.mcp_token.set(token);
        Ok(())
    }

    /// The process service: run commands and manage processes.
    ///
    /// # Returns
    ///
    /// A [`Commands`] handle for this sandbox.
    #[must_use]
    pub fn commands(&self) -> Commands {
        Commands::new(Arc::clone(&self.envd))
    }

    /// The PTY service: interactive terminal sessions.
    ///
    /// # Returns
    ///
    /// A [`Pty`] handle for this sandbox.
    #[must_use]
    pub fn pty(&self) -> Pty {
        Pty::new(Arc::clone(&self.envd))
    }

    /// The filesystem service: read, write and manage files.
    ///
    /// # Returns
    ///
    /// A [`Filesystem`] handle for this sandbox.
    #[must_use]
    pub fn files(&self) -> Filesystem {
        Filesystem::new(Arc::clone(&self.envd))
    }

    /// A URL that downloads `path` with a plain `GET`, with no API key or
    /// headers needed. On a secured sandbox (every hosted one) it is signed
    /// with the sandbox's envd access token; the URL carries the signature,
    /// never the token. Treat it as a credential until it expires.
    ///
    /// # Arguments
    ///
    /// * `path` - the file to download.
    /// * `options` - the user to read as, and an optional expiry.
    ///
    /// # Returns
    ///
    /// The URL.
    ///
    /// # Errors
    ///
    /// [`crate::Error::InvalidArgument`] if an expiry is asked for on a
    /// sandbox without an access token, the expiry is under one second, or
    /// the system clock is before 1970;
    /// [`crate::Error::InvalidConfig`] if the envd URL does not parse.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::time::Duration;
    /// use e2b_client::{FileUrlOptions, Sandbox};
    ///
    /// # fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
    /// let url = sandbox.download_url(
    ///     "/home/user/report.pdf",
    ///     &FileUrlOptions::default().expires_in(Duration::from_secs(600)),
    /// )?;
    /// # let _ = url;
    /// # Ok(())
    /// # }
    /// ```
    pub fn download_url(&self, path: &str, options: &FileUrlOptions) -> Result<String> {
        self.file_url(path, FileOperation::Read, options)
    }

    /// A URL that uploads to `path`: POST the file to it as
    /// `multipart/form-data` in a part named `file`, with no API key or
    /// headers needed. On a secured sandbox (every hosted one) it is signed
    /// with the sandbox's envd access token; the URL carries the signature,
    /// never the token. Treat it as a credential until it expires.
    ///
    /// # Arguments
    ///
    /// * `path` - the file to write.
    /// * `options` - the user to write as, and an optional expiry.
    ///
    /// # Returns
    ///
    /// The URL.
    ///
    /// # Errors
    ///
    /// As [`Sandbox::download_url`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use e2b_client::{FileUrlOptions, Sandbox};
    ///
    /// # fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
    /// let url = sandbox.upload_url("/home/user/input.csv", &FileUrlOptions::default())?;
    /// # let _ = url;
    /// # Ok(())
    /// # }
    /// ```
    pub fn upload_url(&self, path: &str, options: &FileUrlOptions) -> Result<String> {
        self.file_url(path, FileOperation::Write, options)
    }

    /// Builds a `/files` URL for `operation`, signed when the sandbox has an
    /// access token (`e2b/sandbox/main.py:102-213`; JS `dist/index.js:6225-6290`).
    fn file_url(
        &self,
        path: &str,
        operation: FileOperation,
        options: &FileUrlOptions,
    ) -> Result<String> {
        let token = self.envd.access_token();
        if token.is_none() && options.expires_in.is_some() {
            return Err(Error::InvalidArgument(
                "a URL expiration needs a secured sandbox (one with an envd access token)".into(),
            ));
        }
        // An empty user means envd's default user, as no user does: the
        // signature is the same and the SDKs leave `username` out.
        let user = self
            .envd
            .effective_user(options.user.as_deref())
            .filter(|user| !user.is_empty());
        let mut url = Url::parse(&format!("{}{FILES_ROUTE}", self.envd.direct_base()))
            .map_err(|err| Error::InvalidConfig(format!("envd URL: {err}")))?;
        {
            // `query_pairs_mut` borrows `url` mutably until this block ends,
            // and form-encodes each pair as it is appended.
            let mut query = url.query_pairs_mut();
            if !path.is_empty() {
                query.append_pair("path", path);
            }
            if let Some(user) = user {
                query.append_pair("username", user);
            }
            if let Some(token) = token {
                // `transpose` turns `Option<Result<_>>` into `Result<Option<_>>`
                // so `?` can return the error.
                let expiration = options.expires_in.map(expiration_from_now).transpose()?;
                query.append_pair(
                    "signature",
                    &file_signature(path, operation, user, token.expose_secret(), expiration),
                );
                if let Some(expiration) = expiration {
                    query.append_pair("signature_expiration", &expiration.to_string());
                }
            }
        }
        Ok(url.into())
    }

    /// Checks envd's health endpoint.
    ///
    /// # Returns
    ///
    /// `true` if envd answered, `false` if the proxy reported the sandbox
    /// unavailable (HTTP 502), as the SDKs do (`e2b/sandbox_async/main.py:134-168`).
    ///
    /// # Errors
    ///
    /// [`crate::Error::Envd`] for another non-2xx status,
    /// [`crate::Error::Transport`] for connection failures.
    pub async fn is_running(&self) -> Result<bool> {
        let response = self
            .envd
            .http_request(Method::GET, ENVD_HEALTH_ROUTE, RequestTimeout::default())
            .send()
            .await
            .map_err(Error::transport)?;
        if response.status() == StatusCode::BAD_GATEWAY {
            return Ok(false);
        }
        if !response.status().is_success() {
            return Err(envd_http_error(response).await);
        }
        Ok(true)
    }

    /// Kills the sandbox. See [`Client::kill_sandbox`].
    ///
    /// # Returns
    ///
    /// `true` if it was killed, `false` if it no longer existed.
    ///
    /// # Errors
    ///
    /// As [`Client::kill_sandbox`].
    pub async fn kill(&self) -> Result<bool> {
        self.client.kill_sandbox(&self.sandbox_id).await
    }

    /// Pauses the sandbox. See [`Client::pause_sandbox`].
    ///
    /// # Returns
    ///
    /// `true` if it was paused, `false` if it was already paused.
    ///
    /// # Errors
    ///
    /// As [`Client::pause_sandbox`].
    pub async fn pause(&self) -> Result<bool> {
        self.client.pause_sandbox(&self.sandbox_id).await
    }

    /// Pauses the sandbox, choosing what the pause keeps. See
    /// [`Client::pause_sandbox_with`].
    ///
    /// # Arguments
    ///
    /// * `options` - what the pause keeps.
    ///
    /// # Returns
    ///
    /// `true` if it was paused, `false` if it was already paused (see
    /// [`Client::pause_sandbox_with`] for how a 409 is read when a mode is
    /// set).
    ///
    /// # Errors
    ///
    /// As [`Client::pause_sandbox_with`].
    pub async fn pause_with(&self, options: &PauseOptions) -> Result<bool> {
        self.client
            .pause_sandbox_with(&self.sandbox_id, options)
            .await
    }

    /// Sets the sandbox's remaining lifetime. See [`Client::set_sandbox_timeout`].
    ///
    /// # Arguments
    ///
    /// * `timeout` - the new lifetime from now.
    ///
    /// # Errors
    ///
    /// As [`Client::set_sandbox_timeout`].
    pub async fn set_timeout(&self, timeout: Duration) -> Result<()> {
        self.client
            .set_sandbox_timeout(&self.sandbox_id, timeout)
            .await
    }

    /// Replaces this sandbox's egress settings, without a restart. See
    /// [`Client::update_sandbox_network`].
    ///
    /// # Arguments
    ///
    /// * `update` - the new settings; fields it leaves unset are cleared.
    ///
    /// # Errors
    ///
    /// As [`Client::update_sandbox_network`].
    pub async fn update_network(&self, update: &NetworkUpdate) -> Result<()> {
        self.client
            .update_sandbox_network(&self.sandbox_id, update)
            .await
    }

    /// Extends the sandbox's life. See [`Client::refresh_sandbox`].
    ///
    /// # Arguments
    ///
    /// * `duration` - how long to extend by, or `None` for the server default.
    ///
    /// # Errors
    ///
    /// As [`Client::refresh_sandbox`].
    pub async fn refresh(&self, duration: Option<Duration>) -> Result<()> {
        self.client
            .refresh_sandbox(&self.sandbox_id, duration)
            .await
    }

    /// Fetches current information about the sandbox. See
    /// [`Client::get_sandbox_info`].
    ///
    /// # Returns
    ///
    /// The sandbox information.
    ///
    /// # Errors
    ///
    /// As [`Client::get_sandbox_info`].
    pub async fn get_info(&self) -> Result<SandboxInfo> {
        self.client.get_sandbox_info(&self.sandbox_id).await
    }

    /// Reads this sandbox's CPU, memory and disk samples. See
    /// [`Client::get_sandbox_metrics`].
    ///
    /// # Arguments
    ///
    /// * `params` - the optional time range.
    ///
    /// # Returns
    ///
    /// The samples; empty if there are none yet.
    ///
    /// # Errors
    ///
    /// As [`Client::get_sandbox_metrics`].
    pub async fn get_metrics(&self, params: &MetricsParams) -> Result<Vec<SandboxMetrics>> {
        self.client
            .get_sandbox_metrics(&self.sandbox_id, params)
            .await
    }

    /// Snapshots this sandbox. See [`Client::create_snapshot`].
    ///
    /// # Arguments
    ///
    /// * `options` - the name and mode.
    ///
    /// # Returns
    ///
    /// The snapshot.
    ///
    /// # Errors
    ///
    /// As [`Client::create_snapshot`].
    pub async fn create_snapshot(&self, options: &SnapshotOptions) -> Result<SnapshotInfo> {
        self.client.create_snapshot(&self.sandbox_id, options).await
    }

    /// Lists the snapshots taken from this sandbox. See
    /// [`Client::list_snapshots`]; any `sandbox_id` in `params` is replaced
    /// by this sandbox's, as the SDK's instance `list_snapshots` does.
    ///
    /// # Arguments
    ///
    /// * `params` - the other filters and the page token.
    ///
    /// # Returns
    ///
    /// One page.
    ///
    /// # Errors
    ///
    /// As [`Client::list_snapshots`].
    pub async fn list_snapshots(&self, params: &ListSnapshotsParams) -> Result<SnapshotPage> {
        // `.clone()` copies the caller's parameters, since the builder
        // methods take `self` by value and `params` is only borrowed.
        let params = params.clone().sandbox_id(self.sandbox_id.as_str());
        self.client.list_snapshots(&params).await
    }

    /// Forks this sandbox. See [`Client::fork_sandbox`].
    ///
    /// # Arguments
    ///
    /// * `options` - the forks' timeout and count.
    ///
    /// # Returns
    ///
    /// One result per fork: a handle, or why that fork failed.
    ///
    /// # Errors
    ///
    /// As [`Client::fork_sandbox`].
    pub async fn fork(&self, options: &ForkOptions) -> Result<Vec<Result<Sandbox>>> {
        self.client.fork_sandbox(&self.sandbox_id, options).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ConnectionConfig;

    const API_KEY: &str = "api-key-secret";
    const ENVD_TOKEN: &str = "envd-token-secret";
    const TRAFFIC_TOKEN: &str = "traffic-token-secret";
    const MCP_TOKEN: &str = "mcp-token-secret";

    /// A client on the custom domain `configured.test`, with no envd URL
    /// override, so addressing is derived from domains.
    fn client() -> Client {
        let config = ConnectionConfig::builder()
            .api_key(API_KEY)
            .domain("configured.test")
            .build_with_env(|_| None)
            .expect("config");
        Client::new(config).expect("client")
    }

    /// A handle built from a create response reporting `domain`.
    fn sandbox_on(domain: Option<&str>) -> Sandbox {
        let created: SandboxCreated = serde_json::from_value(serde_json::json!({
            "templateID": "base",
            "sandboxID": "sbx",
            "envdVersion": "0.5.7",
            "envdAccessToken": ENVD_TOKEN,
            "trafficAccessToken": TRAFFIC_TOKEN,
            "domain": domain,
        }))
        .expect("created");
        Sandbox::from_created(client(), created).expect("sandbox")
    }

    #[test]
    fn envd_addressing_uses_the_api_domain_else_the_configured_one() {
        // Arrange: a hosted domain from the API must switch RPCs to the
        // shared host, which the configured custom domain would not.
        let from_api = sandbox_on(Some("e2b.app"));
        let from_config = sandbox_on(None);

        // Act and Assert
        assert_eq!(from_api.envd.rpc_base(), "https://sandbox.e2b.app");
        assert_eq!(from_api.envd.direct_base(), "https://49983-sbx.e2b.app");
        assert_eq!(
            from_config.envd.rpc_base(),
            "https://49983-sbx.configured.test"
        );
        assert_eq!(
            from_config.envd.direct_base(),
            "https://49983-sbx.configured.test"
        );
    }

    #[test]
    fn get_host_follows_the_reported_domain() {
        // Arrange, Act and Assert: the reported domain wins; without one the
        // client's configured domain applies.
        assert_eq!(
            sandbox_on(Some("e2b.app")).get_host(3000),
            "3000-sbx.e2b.app"
        );
        assert_eq!(sandbox_on(None).get_host(8080), "8080-sbx.configured.test");
    }

    #[test]
    fn file_signature_matches_the_sdk_algorithm() {
        // Arrange, Act and Assert: vectors computed with the Python SDK's
        // `hashlib.sha256` + `base64.b64encode(..).rstrip(b"=")`.
        assert_eq!(
            file_signature("/home/user/a.txt", FileOperation::Read, None, "tok", None),
            "v1_Zkbb3914oLJLvMBzCAHQhm2ydBLxbSx2eQUR/kLdvIY"
        );
        assert_eq!(
            file_signature(
                "/tmp/up.bin",
                FileOperation::Write,
                Some("root"),
                "tok",
                Some(1_700_000_000)
            ),
            "v1_Y0uzWP5ST4jxa8RLcFIf65h1K6NEbMc0oox+Mgq89DY"
        );
    }

    #[test]
    fn expiration_is_whole_seconds_and_refuses_under_one_second() {
        // Arrange
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after 1970")
            .as_secs();

        // Act
        let expiration =
            expiration_from_now(Duration::from_millis(60_900)).expect("60.9 s expiration");
        let rejected = [
            expiration_from_now(Duration::ZERO),
            expiration_from_now(Duration::from_millis(999)),
            expiration_from_now(Duration::MAX),
        ];

        // Assert: the fraction is dropped, so 60.9 s is now + 60.
        assert!(
            (now + 60..=now + 61).contains(&expiration),
            "{expiration} is not now + 60 s"
        );
        for result in rejected {
            assert!(
                matches!(result, Err(Error::InvalidArgument(_))),
                "{result:?}"
            );
        }
    }

    #[test]
    fn signed_urls_carry_the_signature_not_the_token() {
        // Arrange
        let sandbox = sandbox_on(None);
        let options = FileUrlOptions::default().expires_in(Duration::from_secs(60));

        // Act
        let urls = [
            sandbox.download_url("/a", &options).expect("download url"),
            sandbox.upload_url("/a", &options).expect("upload url"),
        ];

        // Assert
        for url in &urls {
            assert!(url.contains("signature=v1_"), "unsigned URL");
            assert!(!url.contains(ENVD_TOKEN), "envd token leaked into a URL");
        }
    }

    #[test]
    fn debug_output_omits_the_api_key_and_access_tokens() {
        // Arrange: an MCP token is held too.
        let sandbox = sandbox_on(None);
        sandbox
            .mcp_token
            .set(SecretString::from(MCP_TOKEN))
            .expect("empty cell");

        // Act: `{:?}` renders the `Debug` impl, what `dbg!` and logs show.
        // The `EnvdClient` now holds the access token itself, so its own
        // `Debug` is checked too.
        let rendered = [
            format!("{:?}", sandbox.client),
            format!("{:?}", sandbox.client.config()),
            format!("{sandbox:?}"),
            format!("{:?}", sandbox.envd),
        ];

        // Assert
        for text in &rendered {
            assert!(!text.contains(API_KEY), "API key leaked: {text}");
            assert!(!text.contains(ENVD_TOKEN), "envd token leaked: {text}");
            assert!(
                !text.contains(TRAFFIC_TOKEN),
                "traffic token leaked: {text}"
            );
            assert!(!text.contains(MCP_TOKEN), "MCP token leaked: {text}");
        }
    }
}
