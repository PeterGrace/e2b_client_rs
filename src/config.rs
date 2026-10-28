//! Connection configuration: API key, API URL, domain, and how envd is
//! addressed.
//!
//! Resolution follows the official SDKs
//! (`e2b/connection_config.py:221-319`): an explicit builder value wins, then
//! the environment variable, then the default. An empty string counts as
//! unset at both of the first two steps, as in the SDKs' `value or env or
//! default`.

use std::fmt;
use std::time::Duration;

use secrecy::{ExposeSecret, SecretString};
use url::Url;

use crate::error::{Error, Result};

/// The default E2B domain (`E2B_DOMAIN` unset).
pub const DEFAULT_DOMAIN: &str = "e2b.app";

/// The port envd listens on inside every sandbox.
pub const ENVD_PORT: u16 = 49983;

/// The default per-request timeout used by the official SDKs (60 seconds).
pub const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

/// A timeout as the setters store it: `Some(Duration::ZERO)` means no limit,
/// as SDK 2.61's `timeout=0` and `request_timeout=0` do
/// (`e2b/envd/utils.py:15-24`, `e2b/api/__init__.py:280`).
///
/// Python note: in the SDK `0` is falsy, so `timeout or None` turns it into
/// "no timeout". Rust has no truthiness, so the rule is spelled out here once
/// and every setter calls it.
///
/// # Arguments
///
/// * `timeout` - the caller's value.
///
/// # Returns
///
/// `None` for `None` or zero, else the value unchanged.
pub(crate) fn no_limit_if_zero(timeout: Option<Duration>) -> Option<Duration> {
    // `filter` keeps `Some(d)` only when the closure returns true, like
    // `d if d else None` in Python.
    timeout.filter(|timeout| !timeout.is_zero())
}

/// A per-call request timeout, as an options struct stores it: unset uses
/// the configured [`ConnectionConfig::request_timeout`]; set to `None` (or
/// zero) means no limit; set to a duration means that duration. Only the
/// options structs' `request_timeout` setters make one.
///
/// Python note: this is the SDK's `request_timeout: Optional[float] = None`
/// argument, where `None` means "use the connection's". Rust needs the
/// extra `Option` level to tell "not given" from "given as no limit".
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct RequestTimeout(Option<Option<Duration>>);

impl RequestTimeout {
    /// A timeout the caller set explicitly; zero counts as no limit.
    ///
    /// # Arguments
    ///
    /// * `timeout` - the per-call timeout, or `None` for no limit.
    ///
    /// # Returns
    ///
    /// The set value.
    pub(crate) fn set(timeout: Option<Duration>) -> Self {
        Self(Some(no_limit_if_zero(timeout)))
    }

    /// The timeout to apply.
    ///
    /// # Arguments
    ///
    /// * `configured` - what applies when the caller set nothing.
    ///
    /// # Returns
    ///
    /// The caller's value when set, else `configured`.
    pub(crate) fn resolve(self, configured: Option<Duration>) -> Option<Duration> {
        // `unwrap_or` on the outer `Option`: "not set" falls back.
        self.0.unwrap_or(configured)
    }
}

/// Hosted E2B domains whose envd RPC traffic goes through the shared
/// `sandbox.<domain>` endpoint instead of the per-sandbox host
/// (`e2b/sandbox_domains.py:1-9`).
const SUPPORTED_SANDBOX_DOMAINS: [&str; 7] = [
    "e2b.app",
    "e2b.dev",
    "e2b.pro",
    "e2b-staging.dev",
    "e2b-tango.dev",
    "e2b-juliett.dev",
    "foxtrot-2.e2b.dev",
];

const ENV_API_KEY: &str = "E2B_API_KEY";
const ENV_API_URL: &str = "E2B_API_URL";
const ENV_DOMAIN: &str = "E2B_DOMAIN";
const ENV_SANDBOX_URL: &str = "E2B_SANDBOX_URL";

/// Resolved connection settings shared by every request a [`crate::Client`]
/// makes.
///
/// Build one with [`ConnectionConfig::builder`] or [`ConnectionConfig::from_env`].
/// The API key is held as a [`SecretString`], so it never appears in `Debug`
/// output or logs.
#[derive(Clone)]
pub struct ConnectionConfig {
    api_key: Option<SecretString>,
    api_url: Url,
    domain: String,
    sandbox_url: Option<Url>,
    request_timeout: Option<Duration>,
}

impl fmt::Debug for ConnectionConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ConnectionConfig")
            .field("api_key", &self.api_key.as_ref().map(|_| "[redacted]"))
            .field("api_url", &self.api_url.as_str())
            .field("domain", &self.domain)
            .field("sandbox_url", &self.sandbox_url.as_ref().map(Url::as_str))
            .field("request_timeout", &self.request_timeout)
            .finish()
    }
}

impl ConnectionConfig {
    /// Starts a builder. Values left unset are resolved from the environment
    /// when [`ConnectionConfigBuilder::build`] runs.
    ///
    /// # Returns
    ///
    /// An empty [`ConnectionConfigBuilder`].
    ///
    /// # Examples
    ///
    /// ```
    /// use e2b_client::ConnectionConfig;
    ///
    /// let config = ConnectionConfig::builder()
    ///     .api_key("e2b_test")
    ///     .domain("example.com")
    ///     .build()?;
    /// assert_eq!(config.api_url().as_str(), "https://api.example.com/");
    /// # Ok::<(), e2b_client::Error>(())
    /// ```
    #[must_use]
    pub fn builder() -> ConnectionConfigBuilder {
        ConnectionConfigBuilder::default()
    }

    /// Builds a configuration purely from `E2B_API_KEY`, `E2B_API_URL`,
    /// `E2B_DOMAIN` and `E2B_SANDBOX_URL`.
    ///
    /// # Returns
    ///
    /// The resolved configuration.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidConfig`] if `E2B_API_URL` or `E2B_SANDBOX_URL` is not a
    /// valid URL. A missing API key is not an error here; it is reported by
    /// [`crate::Client::new`].
    pub fn from_env() -> Result<Self> {
        Self::builder().build()
    }

    /// The API key, if one was configured.
    pub(crate) fn api_key(&self) -> Option<&str> {
        self.api_key.as_ref().map(ExposeSecret::expose_secret)
    }

    /// The REST API base URL (`E2B_API_URL`, else `https://api.<domain>`).
    ///
    /// # Returns
    ///
    /// The base URL every REST path is joined onto.
    #[must_use]
    pub fn api_url(&self) -> &Url {
        &self.api_url
    }

    /// The E2B domain (`E2B_DOMAIN`, else `e2b.app`).
    ///
    /// # Returns
    ///
    /// The domain used for per-sandbox hosts when the API does not return one.
    #[must_use]
    pub fn domain(&self) -> &str {
        &self.domain
    }

    /// The per-request timeout, or `None` when timeouts are disabled.
    ///
    /// # Returns
    ///
    /// The timeout applied to REST calls, unary envd calls, and opening envd
    /// streams.
    #[must_use]
    pub fn request_timeout(&self) -> Option<Duration> {
        self.request_timeout
    }

    /// The URL envd's Connect-RPC services are reached at for a sandbox.
    ///
    /// Mirrors `ConnectionConfig.get_sandbox_url` in the Python SDK
    /// (`e2b/connection_config.py:336-351`): the `E2B_SANDBOX_URL` override
    /// wins; hosted E2B domains use `https://sandbox.<domain>`; any other
    /// domain uses the per-sandbox host `https://<port>-<sandbox_id>.<domain>`.
    ///
    /// # Arguments
    ///
    /// * `sandbox_id` - the sandbox ID.
    /// * `sandbox_domain` - the domain the API returned for the sandbox, if any.
    ///
    /// # Returns
    ///
    /// The base URL, without a trailing slash.
    ///
    /// # Examples
    ///
    /// ```
    /// use e2b_client::ConnectionConfig;
    ///
    /// let config = ConnectionConfig::builder().api_key("k").domain("example.com").build()?;
    /// assert_eq!(config.envd_rpc_url("sbx1", None), "https://49983-sbx1.example.com");
    /// assert_eq!(config.envd_rpc_url("sbx1", Some("e2b.app")), "https://sandbox.e2b.app");
    /// # Ok::<(), e2b_client::Error>(())
    /// ```
    #[must_use]
    pub fn envd_rpc_url(&self, sandbox_id: &str, sandbox_domain: Option<&str>) -> String {
        if let Some(url) = &self.sandbox_url {
            return trim_url(url);
        }
        let domain = sandbox_domain.unwrap_or(&self.domain);
        if SUPPORTED_SANDBOX_DOMAINS.contains(&domain) {
            return format!("https://sandbox.{domain}");
        }
        format!("https://{}", host_name(ENVD_PORT, sandbox_id, domain))
    }

    /// The URL envd's plain HTTP API (`/files`, `/health`) is reached at.
    ///
    /// Mirrors `get_sandbox_direct_url` (`e2b/connection_config.py:353-364`):
    /// the `E2B_SANDBOX_URL` override wins, otherwise always the per-sandbox
    /// host.
    ///
    /// # Arguments
    ///
    /// * `sandbox_id` - the sandbox ID.
    /// * `sandbox_domain` - the domain the API returned for the sandbox, if any.
    ///
    /// # Returns
    ///
    /// The base URL, without a trailing slash.
    #[must_use]
    pub fn envd_direct_url(&self, sandbox_id: &str, sandbox_domain: Option<&str>) -> String {
        if let Some(url) = &self.sandbox_url {
            return trim_url(url);
        }
        let domain = sandbox_domain.unwrap_or(&self.domain);
        format!("https://{}", host_name(ENVD_PORT, sandbox_id, domain))
    }

    /// The host name that reaches `port` inside a sandbox from outside it,
    /// for an HTTP or WebSocket client.
    ///
    /// Mirrors `ConnectionConfig.get_host` (`e2b/connection_config.py:366-379`;
    /// JS `dist/index.js:1138-1141`): `<port>-<sandbox_id>.<domain>`, with the
    /// domain the API reported for the sandbox, else this configuration's
    /// domain. `E2B_SANDBOX_URL` does not change it, as in the SDKs.
    ///
    /// # Arguments
    ///
    /// * `sandbox_id` - the sandbox ID.
    /// * `sandbox_domain` - the domain the API returned for the sandbox, if any.
    /// * `port` - the port a service listens on inside the sandbox.
    ///
    /// # Returns
    ///
    /// The host name, without a scheme.
    ///
    /// # Examples
    ///
    /// ```
    /// use e2b_client::ConnectionConfig;
    ///
    /// let config = ConnectionConfig::builder().api_key("k").domain("example.com").build()?;
    /// assert_eq!(config.sandbox_host("sbx1", None, 3000), "3000-sbx1.example.com");
    /// assert_eq!(config.sandbox_host("sbx1", Some("e2b.app"), 8080), "8080-sbx1.e2b.app");
    /// # Ok::<(), e2b_client::Error>(())
    /// ```
    #[must_use]
    pub fn sandbox_host(
        &self,
        sandbox_id: &str,
        sandbox_domain: Option<&str>,
        port: u16,
    ) -> String {
        // `unwrap_or` on an `Option<&str>` picks the reported domain, else a
        // borrow of ours: no allocation until `format!` builds the result.
        host_name(port, sandbox_id, sandbox_domain.unwrap_or(&self.domain))
    }
}

/// `<port>-<sandbox_id>.<domain>` (`e2b/connection_config.py:366-379`).
fn host_name(port: u16, sandbox_id: &str, domain: &str) -> String {
    format!("{port}-{sandbox_id}.{domain}")
}

fn trim_url(url: &Url) -> String {
    url.as_str().trim_end_matches('/').to_owned()
}

/// Builder for [`ConnectionConfig`].
///
/// Every setter is optional. Unset values fall back to the environment, then to
/// the defaults, when [`build`](Self::build) is called.
///
/// An explicit empty string counts as unset, exactly like an empty environment
/// variable: `.api_url("")` resolves `E2B_API_URL`, then the default, rather
/// than failing to parse. This matches the Python SDK's `value or env or
/// default` (`e2b/connection_config.py:289-291, 310-318`) for `api_key`,
/// `api_url`, `domain` and `sandbox_url`.
///
/// # Examples
///
/// [`build`](Self::build) reads the real process environment, so this example
/// assumes `E2B_API_URL` is unset (or empty). If it is set, `.api_url("")`
/// resolves to that value instead, which is the rule being shown.
///
/// ```
/// use e2b_client::ConnectionConfig;
///
/// let config = ConnectionConfig::builder()
///     .api_key("e2b_test")
///     .domain("example.com")
///     .api_url("")
///     .build()?;
/// assert_eq!(config.api_url().as_str(), "https://api.example.com/");
/// # Ok::<(), e2b_client::Error>(())
/// ```
#[derive(Default)]
pub struct ConnectionConfigBuilder {
    api_key: Option<SecretString>,
    api_url: Option<String>,
    domain: Option<String>,
    sandbox_url: Option<String>,
    // `Option<Option<..>>` distinguishes "not set" (use the default) from
    // "explicitly disabled" (no timeout).
    request_timeout: Option<Option<Duration>>,
}

impl fmt::Debug for ConnectionConfigBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ConnectionConfigBuilder")
            .field("api_key", &self.api_key.as_ref().map(|_| "[redacted]"))
            .field("api_url", &self.api_url)
            .field("domain", &self.domain)
            .field("sandbox_url", &self.sandbox_url)
            .field("request_timeout", &self.request_timeout)
            .finish()
    }
}

impl ConnectionConfigBuilder {
    /// Sets the API key, overriding `E2B_API_KEY`. An empty key counts as
    /// unset.
    ///
    /// # Arguments
    ///
    /// * `api_key` - the team API key, sent as the `X-API-KEY` header.
    ///
    /// # Returns
    ///
    /// The builder, for chaining.
    #[must_use]
    pub fn api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(SecretString::from(api_key.into()));
        self
    }

    /// Sets the REST API base URL, overriding `E2B_API_URL` and the
    /// `https://api.<domain>` default. An empty URL counts as unset.
    ///
    /// # Arguments
    ///
    /// * `api_url` - for example `https://api.e2b.app`.
    ///
    /// # Returns
    ///
    /// The builder, for chaining.
    #[must_use]
    pub fn api_url(mut self, api_url: impl Into<String>) -> Self {
        self.api_url = Some(api_url.into());
        self
    }

    /// Sets the E2B domain, overriding `E2B_DOMAIN`. An empty domain counts as
    /// unset.
    ///
    /// # Arguments
    ///
    /// * `domain` - for example `e2b.app`.
    ///
    /// # Returns
    ///
    /// The builder, for chaining.
    #[must_use]
    pub fn domain(mut self, domain: impl Into<String>) -> Self {
        self.domain = Some(domain.into());
        self
    }

    /// Sends all envd traffic to one fixed URL, overriding `E2B_SANDBOX_URL`.
    /// An empty URL counts as unset.
    ///
    /// This is the official SDKs' `sandbox_url` option. The `E2b-Sandbox-Id`
    /// and `E2b-Sandbox-Port` headers on every envd request tell the server
    /// which sandbox is meant.
    ///
    /// # Arguments
    ///
    /// * `sandbox_url` - the base URL for envd requests.
    ///
    /// # Returns
    ///
    /// The builder, for chaining.
    #[must_use]
    pub fn sandbox_url(mut self, sandbox_url: impl Into<String>) -> Self {
        self.sandbox_url = Some(sandbox_url.into());
        self
    }

    /// Sets the per-request timeout. `None`, or a zero duration (the SDK's
    /// `request_timeout=0`), disables it.
    ///
    /// # Arguments
    ///
    /// * `timeout` - the timeout, or `None` (or zero) for no timeout.
    ///   Defaults to [`DEFAULT_REQUEST_TIMEOUT`].
    ///
    /// # Returns
    ///
    /// The builder, for chaining.
    #[must_use]
    pub fn request_timeout(mut self, timeout: Option<Duration>) -> Self {
        self.request_timeout = Some(no_limit_if_zero(timeout));
        self
    }

    /// Resolves the configuration, reading the environment for unset (or
    /// empty) values.
    ///
    /// # Returns
    ///
    /// The resolved [`ConnectionConfig`].
    ///
    /// # Errors
    ///
    /// [`Error::InvalidConfig`] if the API URL or sandbox URL does not parse.
    pub fn build(self) -> Result<ConnectionConfig> {
        self.build_with_env(|key| std::env::var(key).ok())
    }

    /// [`build`](Self::build) with an injectable environment lookup, so tests
    /// never touch the real process environment.
    pub(crate) fn build_with_env(
        self,
        env: impl Fn(&str) -> Option<String>,
    ) -> Result<ConnectionConfig> {
        // The SDKs use `value or env or default`, so an empty string counts as
        // unset, whether it came from the builder or the environment.
        // `Option::filter` turns `Some("")` into `None` before the fallback.
        let lookup = |key: &str| env(key).filter(|value| !value.is_empty());

        let domain = non_empty(self.domain)
            .or_else(|| lookup(ENV_DOMAIN))
            .unwrap_or_else(|| DEFAULT_DOMAIN.to_owned());
        let api_url_raw = non_empty(self.api_url)
            .or_else(|| lookup(ENV_API_URL))
            .unwrap_or_else(|| format!("https://api.{domain}"));
        let api_url = parse_url("API URL", &api_url_raw)?;
        let sandbox_url = non_empty(self.sandbox_url)
            .or_else(|| lookup(ENV_SANDBOX_URL))
            .map(|raw| parse_url("sandbox URL", &raw))
            .transpose()?;
        let api_key = self
            .api_key
            .filter(|key| !key.expose_secret().is_empty())
            .or_else(|| lookup(ENV_API_KEY).map(SecretString::from));

        Ok(ConnectionConfig {
            api_key,
            api_url,
            domain,
            sandbox_url,
            request_timeout: self
                .request_timeout
                .unwrap_or(Some(DEFAULT_REQUEST_TIMEOUT)),
        })
    }
}

/// `None` for an unset or empty builder value, so both fall through to the
/// environment.
fn non_empty(value: Option<String>) -> Option<String> {
    value.filter(|value| !value.is_empty())
}

fn parse_url(what: &str, raw: &str) -> Result<Url> {
    Url::parse(raw).map_err(|err| Error::InvalidConfig(format!("{what} {raw:?}: {err}")))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn env_of(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect();
        move |key| map.get(key).cloned()
    }

    #[test]
    fn defaults_without_env() {
        let config = ConnectionConfig::builder()
            .build_with_env(env_of(&[]))
            .expect("config");
        // Literals, not the constants, so a changed default fails here.
        assert_eq!(config.domain(), "e2b.app");
        assert_eq!(config.api_url().as_str(), "https://api.e2b.app/");
        assert!(config.api_key().is_none());
        assert_eq!(config.request_timeout(), Some(Duration::from_secs(60)));
    }

    #[test]
    fn env_values_are_used_when_builder_is_empty() {
        let env = env_of(&[
            ("E2B_API_KEY", "env-key"),
            ("E2B_DOMAIN", "example.org"),
            ("E2B_SANDBOX_URL", "http://localhost:9000/"),
        ]);
        let config = ConnectionConfig::builder()
            .build_with_env(env)
            .expect("config");
        assert_eq!(config.api_key(), Some("env-key"));
        assert_eq!(config.domain(), "example.org");
        assert_eq!(config.api_url().as_str(), "https://api.example.org/");
        assert_eq!(config.envd_rpc_url("sbx", None), "http://localhost:9000");
        assert_eq!(config.envd_direct_url("sbx", None), "http://localhost:9000");
    }

    #[test]
    fn builder_values_override_env() {
        let env = env_of(&[
            ("E2B_API_KEY", "env-key"),
            ("E2B_API_URL", "https://env.example"),
        ]);
        let config = ConnectionConfig::builder()
            .api_key("builder-key")
            .api_url("https://builder.example")
            .request_timeout(None)
            .build_with_env(env)
            .expect("config");
        assert_eq!(config.api_key(), Some("builder-key"));
        assert_eq!(config.api_url().as_str(), "https://builder.example/");
        assert_eq!(config.request_timeout(), None);
    }

    #[test]
    fn zero_means_no_limit() {
        assert_eq!(no_limit_if_zero(Some(Duration::ZERO)), None);
        assert_eq!(no_limit_if_zero(None), None);
        assert_eq!(
            no_limit_if_zero(Some(Duration::from_millis(1))),
            Some(Duration::from_millis(1))
        );
        let config = ConnectionConfig::builder()
            .request_timeout(Some(Duration::ZERO))
            .build_with_env(env_of(&[]))
            .expect("config");
        assert_eq!(config.request_timeout(), None);
    }

    #[test]
    fn request_timeout_resolves_unset_none_zero_and_a_value() {
        let configured = Some(Duration::from_secs(60));
        let short = Duration::from_millis(200);
        assert_eq!(RequestTimeout::default().resolve(configured), configured);
        assert_eq!(RequestTimeout::set(None).resolve(configured), None);
        assert_eq!(
            RequestTimeout::set(Some(Duration::ZERO)).resolve(configured),
            None
        );
        assert_eq!(
            RequestTimeout::set(Some(short)).resolve(configured),
            Some(short)
        );
        assert_eq!(RequestTimeout::default().resolve(None), None);
    }

    #[test]
    fn empty_env_values_count_as_unset() {
        let env = env_of(&[("E2B_DOMAIN", ""), ("E2B_API_KEY", "")]);
        let config = ConnectionConfig::builder()
            .build_with_env(env)
            .expect("config");
        assert_eq!(config.domain(), DEFAULT_DOMAIN);
        assert!(config.api_key().is_none());
    }

    /// Env values for every setting that has an environment variable.
    const FULL_ENV: [(&str, &str); 4] = [
        ("E2B_API_KEY", "env-key"),
        ("E2B_API_URL", "https://env-api.example"),
        ("E2B_DOMAIN", "env.example"),
        ("E2B_SANDBOX_URL", "http://env-sandbox.example"),
    ];

    #[test]
    fn each_setting_resolves_explicit_then_env_then_default() {
        // Arrange: `&env` works because a reference to a closure is itself
        // callable, so one lookup serves several builds.
        let env = env_of(&FULL_ENV);
        let empty_env = env_of(&[
            ("E2B_API_KEY", ""),
            ("E2B_API_URL", ""),
            ("E2B_DOMAIN", ""),
            ("E2B_SANDBOX_URL", ""),
        ]);

        // Act
        let explicit = ConnectionConfig::builder()
            .api_key("explicit-key")
            .api_url("https://explicit-api.example")
            .domain("explicit.example")
            .sandbox_url("http://explicit-sandbox.example")
            .build_with_env(&env)
            .expect("explicit config");
        let from_env = ConnectionConfig::builder()
            .build_with_env(&env)
            .expect("env config");
        let defaults = ConnectionConfig::builder()
            .build_with_env(empty_env)
            .expect("default config");

        // Assert
        assert_eq!(explicit.api_key(), Some("explicit-key"));
        assert_eq!(explicit.api_url().as_str(), "https://explicit-api.example/");
        assert_eq!(explicit.domain(), "explicit.example");
        assert_eq!(
            explicit.envd_rpc_url("sbx", None),
            "http://explicit-sandbox.example"
        );
        assert_eq!(from_env.api_key(), Some("env-key"));
        assert_eq!(from_env.api_url().as_str(), "https://env-api.example/");
        assert_eq!(from_env.domain(), "env.example");
        assert_eq!(
            from_env.envd_rpc_url("sbx", None),
            "http://env-sandbox.example"
        );
        assert!(defaults.api_key().is_none());
        assert_eq!(defaults.api_url().as_str(), "https://api.e2b.app/");
        assert_eq!(defaults.domain(), "e2b.app");
        assert_eq!(
            defaults.envd_rpc_url("sbx", None),
            "https://sandbox.e2b.app"
        );
    }

    #[test]
    fn explicit_empty_values_count_as_unset() {
        // Arrange: the SDKs resolve `value or env or default`, so an explicit
        // empty string falls through to the env var as well.
        let env = env_of(&FULL_ENV);

        // Act
        let config = ConnectionConfig::builder()
            .api_key("")
            .api_url("")
            .domain("")
            .sandbox_url("")
            .build_with_env(env)
            .expect("config");

        // Assert
        assert_eq!(config.api_key(), Some("env-key"));
        assert_eq!(config.api_url().as_str(), "https://env-api.example/");
        assert_eq!(config.domain(), "env.example");
        assert_eq!(
            config.envd_rpc_url("sbx", None),
            "http://env-sandbox.example"
        );
    }

    #[test]
    fn every_hosted_domain_routes_rpc_through_the_shared_sandbox_host() {
        // Arrange: the hosted list from `e2b/sandbox_domains.py:1-9`, spelled
        // out so a changed list fails here.
        let hosted = [
            "e2b.app",
            "e2b.dev",
            "e2b.pro",
            "e2b-staging.dev",
            "e2b-tango.dev",
            "e2b-juliett.dev",
            "foxtrot-2.e2b.dev",
        ];

        for domain in hosted {
            // Act
            let config = ConnectionConfig::builder()
                .domain(domain)
                .build_with_env(env_of(&[]))
                .expect("config");

            // Assert: RPCs share `sandbox.<domain>`; the file API never does.
            assert_eq!(
                config.envd_rpc_url("abc", None),
                format!("https://sandbox.{domain}")
            );
            assert_eq!(
                config.envd_direct_url("abc", None),
                format!("https://49983-abc.{domain}")
            );
        }
    }

    #[test]
    fn invalid_url_is_rejected() {
        let result = ConnectionConfig::builder()
            .api_url("not a url")
            .build_with_env(env_of(&[]));
        assert!(matches!(result, Err(Error::InvalidConfig(_))));
    }

    #[test]
    fn envd_urls_for_custom_domain_use_per_sandbox_host() {
        let config = ConnectionConfig::builder()
            .domain("example.com")
            .build_with_env(env_of(&[]))
            .expect("c");
        assert_eq!(
            config.envd_rpc_url("abc", None),
            "https://49983-abc.example.com"
        );
        assert_eq!(
            config.envd_direct_url("abc", None),
            "https://49983-abc.example.com"
        );
        // A domain returned by the API wins over the configured one.
        assert_eq!(
            config.envd_rpc_url("abc", Some("other.net")),
            "https://49983-abc.other.net"
        );
    }

    #[test]
    fn envd_urls_for_hosted_domain_split_rpc_and_direct() {
        let config = ConnectionConfig::builder()
            .build_with_env(env_of(&[]))
            .expect("config");
        assert_eq!(config.envd_rpc_url("abc", None), "https://sandbox.e2b.app");
        assert_eq!(
            config.envd_direct_url("abc", None),
            "https://49983-abc.e2b.app"
        );
    }

    #[test]
    fn debug_output_redacts_api_key() {
        let config = ConnectionConfig::builder()
            .api_key("super-secret")
            .build_with_env(env_of(&[]))
            .expect("config");
        let rendered = format!("{config:?}");
        assert!(!rendered.contains("super-secret"));
        assert!(rendered.contains("[redacted]"));
    }

    #[test]
    fn sandbox_host_uses_the_sandbox_domain_then_the_configured_one() {
        // Arrange: an envd URL override that must not leak into the host.
        let config = ConnectionConfig::builder()
            .api_key("k")
            .domain("example.com")
            .sandbox_url("https://ignored.test")
            .build_with_env(|_| None)
            .expect("config");

        // Act and Assert: E2B_SANDBOX_URL does not change get_host, as in
        // the SDKs.
        assert_eq!(
            config.sandbox_host("sbx1", None, 3000),
            "3000-sbx1.example.com"
        );
        assert_eq!(
            config.sandbox_host("sbx1", Some("e2b.app"), 8080),
            "8080-sbx1.e2b.app"
        );
    }
}
