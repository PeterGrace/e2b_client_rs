//! Request parameters and response types for the E2B REST API.
//!
//! JSON field names follow the OpenAPI models generated into the official
//! Python SDK (`e2b/api/client/models/*.py`, cited in
//! `docs/protocol-notes.md`).

use std::collections::HashMap;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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
    auto_pause: Option<bool>,
    allow_internet_access: Option<bool>,
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
            auto_pause: None,
            allow_internet_access: None,
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
        self.auto_pause = Some(auto_pause);
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

    /// The request body (`NewSandboxV2`, `e2b/api/client/models/new_sandbox_v2.py`).
    pub(crate) fn body(&self) -> NewSandboxBody<'_> {
        NewSandboxBody {
            template_id: &self.template,
            timeout: self.timeout.map(|timeout| timeout.as_secs()),
            metadata: &self.metadata,
            env_vars: &self.envs,
            auto_pause: self.auto_pause,
            allow_internet_access: self.allow_internet_access,
        }
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
    // The spec spells this one field in snake_case
    // (`new_sandbox_v2.py:160`).
    #[serde(skip_serializing_if = "Option::is_none")]
    allow_internet_access: Option<bool>,
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
}

/// Filters and paging for [`crate::Client::list_sandboxes`].
///
/// # Examples
///
/// ```
/// use e2b_client::{ListSandboxesParams, SandboxState};
///
/// let params = ListSandboxesParams::new()
///     .metadata("owner", "docs")
///     .state(SandboxState::Running)
///     .limit(10);
/// # let _ = params;
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ListSandboxesParams {
    metadata: Vec<(String, String)>,
    states: Vec<SandboxState>,
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
    /// States are comma-joined.
    pub(crate) fn query(&self) -> Vec<(&'static str, String)> {
        let mut query = Vec::with_capacity(4);
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
        if let Some(limit) = self.limit {
            query.push(("limit", limit.to_string()));
        }
        if let Some(token) = &self.next_token {
            query.push(("nextToken", token.clone()));
        }
        query
    }
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
        let json = serde_json::to_value(params.body()).expect("serialize");
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
        let json = serde_json::to_value(CreateSandboxParams::default().body()).expect("ser");
        assert_eq!(
            json,
            serde_json::json!({"templateID": "base", "metadata": {}, "envVars": {}})
        );
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
    fn encode_uri_component_keeps_unreserved_marks() {
        assert_eq!(
            encode_uri_component("a-b_c.d!e~f*g'h(i)j k/"),
            "a-b_c.d!e~f*g'h(i)j%20k%2F"
        );
    }
}
