//! A handle to one running sandbox.

use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use reqwest::{Method, StatusCode};
use secrecy::SecretString;

use crate::client::Client;
use crate::commands::Commands;
use crate::envd::{EnvdClient, EnvdTarget, EnvdVersion, envd_http_error};
use crate::error::Result;
use crate::filesystem::Filesystem;
use crate::models::{SandboxCreated, SandboxInfo};
use crate::pty::Pty;

/// envd's health route (`e2b/envd/api.py:22`).
const ENVD_HEALTH_ROUTE: &str = "/health";

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
/// use e2b_client::{Client, CommandOptions, CreateSandboxParams};
///
/// # async fn demo() -> e2b_client::Result<()> {
/// let client = Client::from_env()?;
/// let sandbox = client.create_sandbox(&CreateSandboxParams::new("base")).await?;
/// let result = sandbox.commands().run("echo hello", &CommandOptions::default()).await?;
/// assert_eq!(result.stdout, "hello\n");
/// sandbox.files().write("/tmp/note.txt", "hi", None).await?;
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
            .http_request(Method::GET, ENVD_HEALTH_ROUTE)
            .send()
            .await?;
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ConnectionConfig;

    const API_KEY: &str = "api-key-secret";
    const ENVD_TOKEN: &str = "envd-token-secret";
    const TRAFFIC_TOKEN: &str = "traffic-token-secret";

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
    fn debug_output_omits_the_api_key_and_access_tokens() {
        // Arrange
        let sandbox = sandbox_on(None);

        // Act: `{:?}` renders the `Debug` impl, what `dbg!` and logs show.
        let rendered = [
            format!("{:?}", sandbox.client),
            format!("{:?}", sandbox.client.config()),
            format!("{sandbox:?}"),
        ];

        // Assert
        for text in &rendered {
            assert!(!text.contains(API_KEY), "API key leaked: {text}");
            assert!(!text.contains(ENVD_TOKEN), "envd token leaked: {text}");
            assert!(
                !text.contains(TRAFFIC_TOKEN),
                "traffic token leaked: {text}"
            );
        }
    }
}
