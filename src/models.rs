//! Request parameters and response types for the E2B REST API.
//!
//! JSON field names follow the OpenAPI models generated into the official
//! Python SDK (`e2b/api/client/models/*.py`, cited in
//! `docs/protocol-notes.md`).

use std::collections::HashMap;
use std::time::Duration;

use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::lifecycle::{
    AutoResume, Lifecycle, LifecycleInfo, OnTimeout, SnapshotMode, deserialize_known_mode,
};
use crate::network::{NetworkConfig, NetworkInfo};

/// The template the SDKs use when none is given (`e2b/sandbox/main.py:32`).
pub const DEFAULT_TEMPLATE: &str = "base";

/// Parameters for [`crate::Client::create_sandbox`].
///
/// # Examples
///
/// ```
/// use std::time::Duration;
/// use e2b_client::CreateSandboxParams;
///
/// let params = CreateSandboxParams::new("base")
///     .timeout(Duration::from_secs(600))
///     .metadata("owner", "docs")
///     .env("GREETING", "hello");
/// assert_eq!(params.template(), "base");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateSandboxParams {
    template: String,
    timeout: Option<Duration>,
    metadata: HashMap<String, String>,
    envs: HashMap<String, String>,
    lifecycle: Lifecycle,
    allow_internet_access: Option<bool>,
    network: Option<NetworkConfig>,
    idempotency_key: Option<String>,
}

impl Default for CreateSandboxParams {
    fn default() -> Self {
        Self::new(DEFAULT_TEMPLATE)
    }
}

impl CreateSandboxParams {
    /// Starts parameters for a sandbox from `template` (an ID or alias).
    ///
    /// # Arguments
    ///
    /// * `template` - the template ID or alias, e.g. `"base"`.
    ///
    /// # Returns
    ///
    /// Parameters with no timeout, metadata or environment variables set; the
    /// server applies its defaults (300 s timeout on E2B).
    #[must_use]
    pub fn new(template: impl Into<String>) -> Self {
        Self {
            template: template.into(),
            timeout: None,
            metadata: HashMap::new(),
            envs: HashMap::new(),
            lifecycle: Lifecycle::new(),
            allow_internet_access: None,
            network: None,
            idempotency_key: None,
        }
    }

    /// The template ID or alias.
    ///
    /// # Returns
    ///
    /// The template.
    #[must_use]
    pub fn template(&self) -> &str {
        &self.template
    }

    /// Sets how long the sandbox lives before it is killed (or paused, with
    /// auto-pause). Sent in whole seconds.
    ///
    /// # Arguments
    ///
    /// * `timeout` - the sandbox lifetime.
    ///
    /// # Returns
    ///
    /// The parameters, for chaining.
    #[must_use]
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Adds one metadata key/value pair.
    ///
    /// # Arguments
    ///
    /// * `key`, `value` - the metadata entry.
    ///
    /// # Returns
    ///
    /// The parameters, for chaining.
    #[must_use]
    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Adds one environment variable for every process in the sandbox.
    ///
    /// # Arguments
    ///
    /// * `key`, `value` - the variable.
    ///
    /// # Returns
    ///
    /// The parameters, for chaining.
    #[must_use]
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.envs.insert(key.into(), value.into());
        self
    }

    /// Pauses instead of killing the sandbox when its timeout expires.
    /// Shorthand for [`CreateSandboxParams::lifecycle`] with
    /// `on_timeout(OnTimeout::pause())` (`true`) or `on_timeout(OnTimeout::Kill)`
    /// (`false`), keeping any `auto_resume` already set. Both write the same
    /// setting, so the later of the two calls wins.
    ///
    /// # Arguments
    ///
    /// * `auto_pause` - whether to auto-pause.
    ///
    /// # Returns
    ///
    /// The parameters, for chaining.
    #[must_use]
    pub fn auto_pause(mut self, auto_pause: bool) -> Self {
        let on_timeout = if auto_pause {
            OnTimeout::pause()
        } else {
            OnTimeout::Kill
        };
        self.lifecycle = self.lifecycle.on_timeout(on_timeout);
        self
    }

    /// Sets the lifecycle: what happens at the timeout, and auto-resume.
    /// Replaces any earlier [`CreateSandboxParams::auto_pause`]. The SDK's
    /// checks run in [`crate::Client::create_sandbox`], before any request.
    ///
    /// # Arguments
    ///
    /// * `lifecycle` - the settings.
    ///
    /// # Returns
    ///
    /// The parameters, for chaining.
    ///
    /// # Examples
    ///
    /// ```
    /// use e2b_client::{CreateSandboxParams, Lifecycle, OnTimeout, SnapshotMode};
    ///
    /// let params = CreateSandboxParams::new("base").lifecycle(
    ///     Lifecycle::new().on_timeout(OnTimeout::pause_with(SnapshotMode::Filesystem)),
    /// );
    /// # let _ = params;
    /// ```
    #[must_use]
    pub fn lifecycle(mut self, lifecycle: Lifecycle) -> Self {
        self.lifecycle = lifecycle;
        self
    }

    /// Makes the create request idempotent: a retry with the same key gets
    /// the same sandbox instead of a second one. Sent as the
    /// `Idempotency-Key` header.
    ///
    /// # Arguments
    ///
    /// * `key` - 1 to 255 lowercase ASCII letters, digits or hyphens, for
    ///   example a lowercase UUID. `impl Into<String>` accepts both a `&str`
    ///   and an owned `String`. Checked by [`crate::Client::create_sandbox`].
    ///
    /// # Returns
    ///
    /// The parameters, for chaining.
    #[must_use]
    pub fn idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }

    /// Allows or denies outbound internet access.
    ///
    /// # Arguments
    ///
    /// * `allow` - whether the sandbox may reach the internet.
    ///
    /// # Returns
    ///
    /// The parameters, for chaining.
    #[must_use]
    pub fn allow_internet_access(mut self, allow: bool) -> Self {
        self.allow_internet_access = Some(allow);
        self
    }

    /// Sets the sandbox's network settings: allow and deny lists, DNS
    /// servers, public traffic and per-host rules. Settings with nothing set
    /// are not sent. `allow_internet_access(false)` is the same as
    /// `deny_out([ALL_TRAFFIC])` ([`crate::ALL_TRAFFIC`]); both can be given.
    ///
    /// # Arguments
    ///
    /// * `network` - the settings.
    ///
    /// # Returns
    ///
    /// The parameters, for chaining.
    ///
    /// # Examples
    ///
    /// ```
    /// use e2b_client::{CreateSandboxParams, NetworkConfig};
    ///
    /// let params = CreateSandboxParams::new("base")
    ///     .network(NetworkConfig::new().allow_public_traffic(false));
    /// # let _ = params;
    /// ```
    #[must_use]
    pub fn network(mut self, network: NetworkConfig) -> Self {
        self.network = Some(network);
        self
    }

    /// The idempotency key, if set and valid (`resolve_idempotency_key`,
    /// `e2b/retry.py:29-44`).
    ///
    /// # Returns
    ///
    /// The key, or `None` when none was set.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidArgument`] for a key that is empty, longer than 255
    /// characters, or has a character outside `a-z`, `0-9` and `-`.
    pub(crate) fn checked_idempotency_key(&self) -> Result<Option<&str>> {
        let Some(key) = self.idempotency_key.as_deref() else {
            return Ok(None);
        };
        // Length first, in characters, as the SDK's `len(key)` counts them
        // (Rust's `str::len` counts UTF-8 bytes).
        let characters = key.chars().count();
        if !(1..=255).contains(&characters) {
            return Err(Error::InvalidArgument(format!(
                "idempotency key must be 1-255 characters (got {characters})"
            )));
        }
        let allowed = |byte: u8| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-';
        if !key.bytes().all(allowed) {
            return Err(Error::InvalidArgument(
                "idempotency key may only hold lowercase ASCII letters, digits and hyphens".into(),
            ));
        }
        Ok(Some(key))
    }

    /// The request body (`NewSandboxV2`, `e2b/api/client/models/new_sandbox_v2.py`).
    ///
    /// # Returns
    ///
    /// The body, borrowing from these parameters.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidArgument`] for a lifecycle the SDK refuses (see
    /// [`Lifecycle::auto_resume`]).
    pub(crate) fn body(&self) -> Result<NewSandboxBody<'_>> {
        let lifecycle = self.lifecycle.body()?;
        Ok(NewSandboxBody {
            template_id: &self.template,
            timeout: self.timeout.map(|timeout| timeout.as_secs()),
            metadata: &self.metadata,
            env_vars: &self.envs,
            auto_pause: lifecycle.auto_pause,
            auto_pause_memory: lifecycle.auto_pause_memory,
            auto_resume: lifecycle.auto_resume,
            allow_internet_access: self.allow_internet_access,
            // `Option::filter` keeps the settings only when something is set,
            // so an empty `NetworkConfig` leaves `network` out of the body.
            network: self.network.as_ref().filter(|network| !network.is_empty()),
        })
    }
}

/// `NewSandboxV2` request body.
#[derive(Debug, Serialize)]
pub(crate) struct NewSandboxBody<'a> {
    #[serde(rename = "templateID")]
    template_id: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    timeout: Option<u64>,
    metadata: &'a HashMap<String, String>,
    #[serde(rename = "envVars")]
    env_vars: &'a HashMap<String, String>,
    #[serde(rename = "autoPause", skip_serializing_if = "Option::is_none")]
    auto_pause: Option<bool>,
    #[serde(rename = "autoPauseMemory", skip_serializing_if = "Option::is_none")]
    auto_pause_memory: Option<bool>,
    #[serde(rename = "autoResume", skip_serializing_if = "Option::is_none")]
    auto_resume: Option<AutoResume>,
    // The spec spells this one field in snake_case
    // (`new_sandbox_v2.py:160`).
    #[serde(skip_serializing_if = "Option::is_none")]
    allow_internet_access: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    network: Option<&'a NetworkConfig>,
}

/// `ConnectSandbox` / `ConnectSandboxV2` request body: the timeout in seconds.
#[derive(Debug, Serialize)]
pub(crate) struct TimeoutBody {
    pub(crate) timeout: u64,
}

/// `SandboxRefreshRequest` body: the optional duration in seconds.
#[derive(Debug, Serialize)]
pub(crate) struct RefreshBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) duration: Option<u64>,
}

/// An empty JSON object body (`SandboxPauseRequest` with nothing set).
#[derive(Debug, Serialize)]
pub(crate) struct EmptyBody {}

/// The `Sandbox` model returned by create and connect.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SandboxCreated {
    #[serde(rename = "sandboxID")]
    pub(crate) sandbox_id: String,
    #[serde(rename = "templateID", default)]
    pub(crate) template_id: String,
    #[serde(default)]
    pub(crate) envd_version: String,
    #[serde(default)]
    pub(crate) envd_access_token: Option<String>,
    #[serde(default)]
    pub(crate) traffic_access_token: Option<String>,
    #[serde(default)]
    pub(crate) domain: Option<String>,
}

/// Whether a sandbox is running or paused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum SandboxState {
    /// The sandbox is running.
    Running,
    /// The sandbox is paused and can be resumed with
    /// [`crate::Client::connect_sandbox`].
    Paused,
    /// A state this version of the crate does not know.
    #[serde(other)]
    Unknown,
}

impl SandboxState {
    /// The wire spelling used in list filters.
    pub(crate) fn as_wire(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Paused => "paused",
            Self::Unknown => "unknown",
        }
    }
}

/// Information about a sandbox, from `GET /sandboxes/{id}` (`SandboxDetail`)
/// or one entry of a list (`ListedSandbox`).
///
/// This is a plain data record, so its fields are public. It is
/// `#[non_exhaustive]` so fields can be added without a breaking change.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct SandboxInfo {
    /// The sandbox ID.
    #[serde(rename = "sandboxID")]
    pub sandbox_id: String,
    /// The template ID.
    #[serde(rename = "templateID")]
    pub template_id: String,
    /// The template alias, if any.
    #[serde(default)]
    pub alias: Option<String>,
    /// When the sandbox started.
    pub started_at: DateTime<Utc>,
    /// When the sandbox will be killed or paused if not extended.
    pub end_at: DateTime<Utc>,
    /// Running or paused.
    pub state: SandboxState,
    /// vCPUs allocated.
    #[serde(default)]
    pub cpu_count: u32,
    /// Memory allocated, in MiB.
    #[serde(default, rename = "memoryMB")]
    pub memory_mb: u64,
    /// Disk allocated, in MiB, when reported.
    #[serde(default, rename = "diskSizeMB")]
    pub disk_size_mb: Option<u64>,
    /// The envd version.
    #[serde(default)]
    pub envd_version: String,
    /// User metadata.
    #[serde(default)]
    pub metadata: HashMap<String, String>,
    /// The domain the sandbox is reached on, when reported.
    #[serde(default)]
    pub domain: Option<String>,
    /// Whether the sandbox may reach the internet, when reported (detail
    /// only; list entries leave it `None`).
    #[serde(default)]
    pub allow_internet_access: Option<bool>,
    /// The sandbox's network settings, when reported (detail only).
    #[serde(default)]
    pub network: Option<NetworkInfo>,
    /// The sandbox's lifecycle settings, when reported (detail only).
    #[serde(default)]
    pub lifecycle: Option<LifecycleInfo>,
    /// What the sandbox's pause keeps, when reported and known (detail
    /// only); a mode this crate does not know reads as `None`.
    #[serde(default, deserialize_with = "deserialize_known_mode")]
    pub pause_mode: Option<SnapshotMode>,
}

/// Sort order for [`ListSandboxesParams::order`]: by start time, across the
/// whole result set before paging (`e2b/sandbox_async/sandbox_api.py:91`).
///
/// `#[non_exhaustive]` means code outside this crate must keep a `_` arm when
/// it `match`es on the enum, so a variant can be added later without breaking
/// callers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SortOrder {
    /// Oldest first (`asc`).
    Ascending,
    /// Newest first (`desc`).
    Descending,
}

impl SortOrder {
    /// The wire spelling of the `order` query parameter
    /// (`e2b/api/client/api/sandboxes/get_v2_sandboxes.py:14-58`).
    fn as_wire(self) -> &'static str {
        match self {
            Self::Ascending => "asc",
            Self::Descending => "desc",
        }
    }
}

/// Filters and paging for [`crate::Client::list_sandboxes`].
///
/// # Examples
///
/// ```
/// use chrono::{DateTime, Utc};
/// use e2b_client::{ListSandboxesParams, SandboxState, SortOrder};
///
/// let since: DateTime<Utc> = "2026-10-27T12:00:00Z".parse().expect("time");
/// let params = ListSandboxesParams::new()
///     .metadata("owner", "docs")
///     .state(SandboxState::Running)
///     .template("base")
///     .started_after(since)
///     .order(SortOrder::Descending)
///     .limit(10);
/// # let _ = params;
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ListSandboxesParams {
    metadata: Vec<(String, String)>,
    states: Vec<SandboxState>,
    order: Option<SortOrder>,
    started_after: Option<DateTime<Utc>>,
    template: Option<String>,
    limit: Option<u32>,
    next_token: Option<String>,
}

impl ListSandboxesParams {
    /// No filters: running and paused sandboxes, server-default page size.
    ///
    /// # Returns
    ///
    /// Empty parameters.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Only sandboxes whose metadata has `key` = `value`.
    ///
    /// # Arguments
    ///
    /// * `key`, `value` - the metadata pair to match.
    ///
    /// # Returns
    ///
    /// The parameters, for chaining.
    #[must_use]
    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.push((key.into(), value.into()));
        self
    }

    /// Only sandboxes in `state` (call again to allow several states).
    ///
    /// # Arguments
    ///
    /// * `state` - a state to include.
    ///
    /// # Returns
    ///
    /// The parameters, for chaining.
    #[must_use]
    pub fn state(mut self, state: SandboxState) -> Self {
        self.states.push(state);
        self
    }

    /// Sort by start time. Unset, the server's order applies.
    ///
    /// # Arguments
    ///
    /// * `order` - oldest or newest first.
    ///
    /// # Returns
    ///
    /// The parameters, for chaining.
    #[must_use]
    pub fn order(mut self, order: SortOrder) -> Self {
        self.order = Some(order);
        self
    }

    /// Only sandboxes started at or after `time`.
    ///
    /// # Arguments
    ///
    /// * `time` - the earliest start time.
    ///
    /// # Returns
    ///
    /// The parameters, for chaining.
    #[must_use]
    pub fn started_after(mut self, time: DateTime<Utc>) -> Self {
        self.started_after = Some(time);
        self
    }

    /// Only sandboxes from `template` (a template ID or name).
    ///
    /// # Arguments
    ///
    /// * `template` - the template ID or name. `impl Into<String>` accepts
    ///   both a `&str` and an owned `String`.
    ///
    /// # Returns
    ///
    /// The parameters, for chaining.
    #[must_use]
    pub fn template(mut self, template: impl Into<String>) -> Self {
        self.template = Some(template.into());
        self
    }

    /// Maximum number of sandboxes per page.
    ///
    /// # Arguments
    ///
    /// * `limit` - the page size.
    ///
    /// # Returns
    ///
    /// The parameters, for chaining.
    #[must_use]
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Continue from a previous page's [`SandboxPage::next_token`].
    ///
    /// # Arguments
    ///
    /// * `token` - the pagination token.
    ///
    /// # Returns
    ///
    /// The parameters, for chaining.
    #[must_use]
    pub fn next_token(mut self, token: impl Into<String>) -> Self {
        self.next_token = Some(token.into());
        self
    }

    /// The query string pairs for `GET /v2/sandboxes`.
    ///
    /// Metadata is encoded the way both SDKs do it: each key and value is
    /// percent-encoded (`encodeURIComponent`), the pairs are form-encoded into
    /// one string, and that string is the `metadata` parameter
    /// (`e2b/sandbox_async/paginator.py:50-57`; JS `dist/index.js:5656-5658`).
    /// States are comma-joined. `order` is `asc` or `desc`, `startedAfter`
    /// is an RFC 3339 time in Python's `isoformat` form and `template` is sent as given. The pairs come in
    /// the order `metadata, state, order, startedAfter, template, limit,
    /// nextToken`, each left out when unset.
    pub(crate) fn query(&self) -> Vec<(&'static str, String)> {
        let mut query = Vec::with_capacity(7);
        if !self.metadata.is_empty() {
            let mut form = url::form_urlencoded::Serializer::new(String::new());
            for (key, value) in &self.metadata {
                form.append_pair(&encode_uri_component(key), &encode_uri_component(value));
            }
            query.push(("metadata", form.finish()));
        }
        if !self.states.is_empty() {
            let states: Vec<&str> = self.states.iter().map(|state| state.as_wire()).collect();
            query.push(("state", states.join(",")));
        }
        if let Some(order) = self.order {
            query.push(("order", order.as_wire().to_owned()));
        }
        if let Some(time) = self.started_after {
            query.push(("startedAfter", isoformat(time)));
        }
        if let Some(template) = &self.template {
            query.push(("template", template.clone()));
        }
        if let Some(limit) = self.limit {
            query.push(("limit", limit.to_string()));
        }
        if let Some(token) = &self.next_token {
            query.push(("nextToken", token.clone()));
        }
        query
    }
}

/// Python's `datetime.isoformat()` of an aware UTC time, as the SDK sends
/// `startedAfter` (`get_v2_sandboxes.py:46-49`): `+00:00` for UTC, and six
/// fractional digits only when the microseconds are not zero. Python has no
/// nanoseconds, so a finer part is truncated to microseconds.
///
/// chrono's plain `to_rfc3339` picks 3, 6 or 9 digits from the value, which
/// differs from Python for millisecond and nanosecond times.
///
/// # Arguments
///
/// * `time` - the time to format.
///
/// # Returns
///
/// The formatted time, e.g. `2026-10-27T12:00:00.123000+00:00`.
fn isoformat(time: DateTime<Utc>) -> String {
    let format = if time.timestamp_subsec_micros() == 0 {
        SecondsFormat::Secs
    } else {
        SecondsFormat::Micros
    };
    // `false`: write the offset as `+00:00`, not `Z`.
    time.to_rfc3339_opts(format, false)
}

/// JavaScript's `encodeURIComponent`: everything except `A-Z a-z 0-9 - _ . ! ~ * ' ( )`.
fn encode_uri_component(raw: &str) -> String {
    use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
    const COMPONENT: &percent_encoding::AsciiSet = &NON_ALPHANUMERIC
        .remove(b'-')
        .remove(b'_')
        .remove(b'.')
        .remove(b'!')
        .remove(b'~')
        .remove(b'*')
        .remove(b'\'')
        .remove(b'(')
        .remove(b')');
    utf8_percent_encode(raw, COMPONENT).to_string()
}

/// One page of [`crate::Client::list_sandboxes`] results.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SandboxPage {
    /// The sandboxes on this page.
    pub sandboxes: Vec<SandboxInfo>,
    /// The token for the next page (from the `x-next-token` header), or
    /// `None` on the last page.
    pub next_token: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_body_uses_spec_field_names() {
        let params = CreateSandboxParams::new("base")
            .timeout(Duration::from_secs(120))
            .metadata("k", "v")
            .env("A", "1")
            .auto_pause(true)
            .allow_internet_access(false);
        let json = serde_json::to_value(params.body().expect("body")).expect("serialize");
        assert_eq!(
            json,
            serde_json::json!({
                "templateID": "base",
                "timeout": 120,
                "metadata": {"k": "v"},
                "envVars": {"A": "1"},
                "autoPause": true,
                "allow_internet_access": false
            })
        );
    }

    #[test]
    fn create_body_omits_unset_optionals() {
        let json = serde_json::to_value(CreateSandboxParams::default().body().expect("body"))
            .expect("ser");
        assert_eq!(
            json,
            serde_json::json!({"templateID": "base", "metadata": {}, "envVars": {}})
        );
    }

    #[test]
    fn auto_pause_is_shorthand_for_the_lifecycle_on_timeout() {
        // Arrange
        let paused = CreateSandboxParams::default()
            .lifecycle(Lifecycle::new().auto_resume(true))
            .auto_pause(true);
        let replaced = CreateSandboxParams::default()
            .auto_pause(true)
            .lifecycle(Lifecycle::new().on_timeout(OnTimeout::Kill));
        let overridden = CreateSandboxParams::default()
            .lifecycle(Lifecycle::new().on_timeout(OnTimeout::pause_with(SnapshotMode::Full)))
            .auto_pause(false);

        // Act
        let bodies = [paused, replaced, overridden]
            .map(|params| serde_json::to_value(params.body().expect("body")).expect("ser"));

        // Assert: `auto_pause` keeps `auto_resume`, `lifecycle` replaces
        // `auto_pause`, and `auto_pause(false)` drops the pause mode.
        let base = |extra: serde_json::Value| {
            let mut body = serde_json::json!({"templateID": "base", "metadata": {}, "envVars": {}});
            if let (Some(body), serde_json::Value::Object(extra)) = (body.as_object_mut(), extra) {
                body.extend(extra);
            }
            body
        };
        assert_eq!(
            bodies,
            [
                base(serde_json::json!({"autoPause": true, "autoResume": {"enabled": true}})),
                base(serde_json::json!({"autoPause": false})),
                base(serde_json::json!({"autoPause": false})),
            ]
        );
    }

    #[test]
    fn idempotency_key_accepts_1_to_255_of_lowercase_digits_and_hyphens() {
        for key in [
            "a",
            "0-9",
            &"z".repeat(255),
            "9f1c2d3e-0000-4abc-8def-0123456789ab",
        ] {
            let params = CreateSandboxParams::default().idempotency_key(key);
            assert_eq!(params.checked_idempotency_key().expect("valid"), Some(key));
        }
        assert_eq!(
            CreateSandboxParams::default()
                .checked_idempotency_key()
                .expect("unset"),
            None
        );
    }

    #[test]
    fn idempotency_key_refuses_bad_lengths_and_characters() {
        // Arrange: "\u{e9}" (e-acute) is one character but two UTF-8 bytes, so
        // 200 of them pass the length check and fail on the character set,
        // as in the SDK.
        let cases = [
            (String::new(), "1-255 characters (got 0)"),
            ("a".repeat(256), "1-255 characters (got 256)"),
            ("\u{e9}".repeat(200), "lowercase"),
            ("Upper".to_owned(), "lowercase"),
            ("under_score".to_owned(), "lowercase"),
            ("with space".to_owned(), "lowercase"),
        ];

        for (key, expected) in cases {
            // Act
            let error = CreateSandboxParams::default()
                .idempotency_key(key.clone())
                .checked_idempotency_key()
                .expect_err("invalid key");

            // Assert
            assert!(
                matches!(&error, Error::InvalidArgument(message) if message.contains(expected)),
                "{key:?}: {error:?}"
            );
        }
    }

    #[test]
    fn sandbox_info_parses_detail() {
        let json = r#"{"templateID":"t1","sandboxID":"s1","clientID":"c","alias":"base",
            "startedAt":"2026-01-01T00:00:00Z","endAt":"2026-01-01T00:05:00Z",
            "envdVersion":"0.2.0","cpuCount":2,"memoryMB":512,"diskSizeMB":1024,
            "state":"paused","metadata":{"a":"b"},"newField":1}"#;
        let info: SandboxInfo = serde_json::from_str(json).expect("deserialize");
        assert_eq!(info.sandbox_id, "s1");
        assert_eq!(info.state, SandboxState::Paused);
        assert_eq!(info.memory_mb, 512);
        assert_eq!(info.metadata.get("a").map(String::as_str), Some("b"));
    }

    #[test]
    fn unknown_state_does_not_fail() {
        let state: SandboxState = serde_json::from_str(r#""snapshotting""#).expect("state");
        assert_eq!(state, SandboxState::Unknown);
    }

    #[test]
    fn list_query_matches_sdk_encoding() {
        let params = ListSandboxesParams::new()
            .metadata("user id", "a&b")
            .state(SandboxState::Running)
            .state(SandboxState::Paused)
            .limit(5)
            .next_token("tok");
        let query = params.query();
        assert_eq!(
            query,
            vec![
                // encodeURIComponent then URLSearchParams: "%" is form-encoded again.
                ("metadata", "user%2520id=a%2526b".to_owned()),
                ("state", "running,paused".to_owned()),
                ("limit", "5".to_owned()),
                ("nextToken", "tok".to_owned()),
            ]
        );
    }

    #[test]
    fn list_query_orders_new_filters_like_the_sdk() {
        // Arrange
        let started_after: DateTime<Utc> = "2026-10-27T12:00:00Z".parse().expect("time");
        let params = ListSandboxesParams::new()
            .next_token("tok")
            .limit(5)
            .template("base")
            .started_after(started_after)
            .order(SortOrder::Ascending)
            .state(SandboxState::Running)
            .metadata("k", "v");

        // Act
        let query = params.query();

        // Assert: the setters were called in reverse, the pairs are not.
        assert_eq!(
            query,
            vec![
                ("metadata", "k=v".to_owned()),
                ("state", "running".to_owned()),
                ("order", "asc".to_owned()),
                ("startedAfter", "2026-10-27T12:00:00+00:00".to_owned()),
                ("template", "base".to_owned()),
                ("limit", "5".to_owned()),
                ("nextToken", "tok".to_owned()),
            ]
        );
    }

    #[test]
    fn isoformat_matches_python_for_whole_milli_micro_and_nanosecond_times() {
        // Arrange: the outputs are what Python's `isoformat()` prints for the
        // same times (Python truncates to microseconds when it parses them).
        let cases = [
            ("2026-10-27T12:00:00Z", "2026-10-27T12:00:00+00:00"),
            ("2026-10-27T12:00:00.5Z", "2026-10-27T12:00:00.500000+00:00"),
            (
                "2026-10-27T12:00:00.123456Z",
                "2026-10-27T12:00:00.123456+00:00",
            ),
            (
                "2026-10-27T12:00:00.123456789Z",
                "2026-10-27T12:00:00.123456+00:00",
            ),
            (
                "2026-10-27T12:00:00.000000500Z",
                "2026-10-27T12:00:00+00:00",
            ),
        ];

        for (input, expected) in cases {
            // Act
            let time: DateTime<Utc> = input.parse().expect("time");

            // Assert
            assert_eq!(isoformat(time), expected, "{input}");
        }
    }

    #[test]
    fn list_query_form_encodes_several_metadata_pairs_into_one_string() {
        let params = ListSandboxesParams::new()
            .metadata("a b", "1")
            .metadata("c", "x/y");

        let query = params.query();

        // One `metadata` parameter whose value is a form-encoded string of
        // the encodeURIComponent-ed pairs. HashMap order is unspecified, so
        // the pairs are compared as a sorted list.
        let [("metadata", metadata)] = query.as_slice() else {
            panic!("expected only a metadata parameter: {query:?}");
        };
        let mut pairs: Vec<(String, String)> = url::form_urlencoded::parse(metadata.as_bytes())
            .map(|(key, value)| (key.into_owned(), value.into_owned()))
            .collect();
        pairs.sort();
        assert_eq!(
            pairs,
            [
                ("a%20b".to_owned(), "1".to_owned()),
                ("c".to_owned(), "x%2Fy".to_owned())
            ]
        );
        assert!(metadata.contains('&'), "{metadata}");
    }

    #[test]
    fn sandbox_created_parses_fields_and_optional_tokens() {
        let full: SandboxCreated = serde_json::from_str(
            r#"{"templateID":"t1","sandboxID":"s1","clientID":"c1","envdVersion":"0.5.7",
            "envdAccessToken":"envd-tok","trafficAccessToken":"traffic-tok",
            "domain":"example.net"}"#,
        )
        .expect("full");
        let minimal: SandboxCreated = serde_json::from_str(
            r#"{"templateID":"t1","sandboxID":"s1","clientID":"c1","envdVersion":"0.5.7"}"#,
        )
        .expect("minimal");

        assert_eq!(full.template_id, "t1");
        assert_eq!(full.sandbox_id, "s1");
        assert_eq!(full.envd_version, "0.5.7");
        assert_eq!(full.envd_access_token.as_deref(), Some("envd-tok"));
        assert_eq!(full.traffic_access_token.as_deref(), Some("traffic-tok"));
        assert_eq!(full.domain.as_deref(), Some("example.net"));
        assert!(minimal.envd_access_token.is_none());
        assert!(minimal.traffic_access_token.is_none());
        assert!(minimal.domain.is_none());
    }

    #[test]
    fn encode_uri_component_keeps_unreserved_marks() {
        assert_eq!(
            encode_uri_component("a-b_c.d!e~f*g'h(i)j k/"),
            "a-b_c.d!e~f*g'h(i)j%20k%2F"
        );
    }
}
