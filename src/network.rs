//! Sandbox network settings: outbound allow and deny lists, DNS servers,
//! public traffic, and per-host header rules, set at create through
//! [`crate::CreateSandboxParams::network`] and, through [`NetworkUpdate`], by
//! [`crate::Client::update_sandbox_network`], and read back in
//! [`crate::SandboxInfo::network`].
//!
//! The JSON shapes are `SandboxNetworkConfig`, `SandboxNetworkUpdateConfig`,
//! `SandboxNetworkRule` and `SandboxNetworkTransform`
//! (`e2b/api/client/models/`); the SDK builds them in
//! `e2b/sandbox/sandbox_api.py:826-869, 928-996, 1128-1152`.

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::wire::deserialize_null_as_default;

/// The CIDR block that matches all IPv4 traffic, the SDKs' `ALL_TRAFFIC`
/// (`e2b/sandbox/network.py:8`). `deny_out([ALL_TRAFFIC])` blocks all
/// egress, which is what `allow_internet_access(false)` does.
pub const ALL_TRAFFIC: &str = "0.0.0.0/0";

/// The egress fields that create and update share. Each is sent only when
/// set; an empty list is sent as `[]` (for `dnsServers` it means "no DNS
/// server", `e2b/sandbox/sandbox_api.py:955-958`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct Egress {
    #[serde(skip_serializing_if = "Option::is_none")]
    allow_out: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    deny_out: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dns_servers: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rules: Option<BTreeMap<String, Vec<NetworkRule>>>,
}

/// Collects any iterable of string-like items into owned strings.
///
/// Rust note: `I: IntoIterator<Item = S>, S: Into<String>` accepts an array
/// of `&str`, a `Vec<String>` or any iterator, much like a Python function
/// that takes any iterable of `str`.
fn strings<I, S>(items: I) -> Vec<String>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    items.into_iter().map(Into::into).collect()
}

/// The egress setters, for a builder with an `egress: Egress` field.
///
/// Rust note: `macro_rules!` pastes these methods into each `impl` block
/// that invokes it, so the create and update builders share one copy of the
/// code without a trait.
macro_rules! egress_setters {
    () => {
        /// Allows outbound traffic to these CIDR blocks, IP addresses or
        /// host names (`*.example.com` matches subdomains). Allowed entries
        /// take precedence over `deny_out`.
        ///
        /// # Arguments
        ///
        /// * `targets` - the allowed destinations.
        ///
        /// # Returns
        ///
        /// The settings, for chaining.
        #[must_use]
        pub fn allow_out<I, S>(mut self, targets: I) -> Self
        where
            I: IntoIterator<Item = S>,
            S: Into<String>,
        {
            self.egress.allow_out = Some(strings(targets));
            self
        }

        /// Denies outbound traffic to these CIDR blocks or IP addresses
        /// (the API does not accept host names here). `[ALL_TRAFFIC]` denies
        /// everything not allowed.
        ///
        /// # Arguments
        ///
        /// * `targets` - the denied destinations.
        ///
        /// # Returns
        ///
        /// The settings, for chaining.
        #[must_use]
        pub fn deny_out<I, S>(mut self, targets: I) -> Self
        where
            I: IntoIterator<Item = S>,
            S: Into<String>,
        {
            self.egress.deny_out = Some(strings(targets));
            self
        }

        /// The IPv4 DNS servers the sandbox may reach when `allow_out`
        /// names a domain. Unset, `8.8.8.8` is allowed; an empty list allows
        /// none. This only opens the firewall; it does not change the
        /// sandbox's `/etc/resolv.conf`.
        ///
        /// # Arguments
        ///
        /// * `servers` - the DNS server addresses.
        ///
        /// # Returns
        ///
        /// The settings, for chaining.
        #[must_use]
        pub fn dns_servers<I, S>(mut self, servers: I) -> Self
        where
            I: IntoIterator<Item = S>,
            S: Into<String>,
        {
            self.egress.dns_servers = Some(strings(servers));
            self
        }

        /// Adds a rule for outbound HTTP and HTTPS requests to `host`.
        /// Rules for one host are sent in the order added, but hosted E2B
        /// accepts only one per host (it answers 400 to more). A rule does
        /// not allow egress by itself: the host must also be in `allow_out`.
        ///
        /// # Arguments
        ///
        /// * `host` - the domain (or IP or CIDR block) the rule applies to.
        /// * `rule` - the rule.
        ///
        /// # Returns
        ///
        /// The settings, for chaining.
        #[must_use]
        pub fn rule(mut self, host: impl Into<String>, rule: NetworkRule) -> Self {
            // `get_or_insert_with` creates the map on the first rule, and
            // `entry(..).or_default()` the host's list on its first rule, like
            // Python's `setdefault`.
            self.egress
                .rules
                .get_or_insert_with(BTreeMap::new)
                .entry(host.into())
                .or_default()
                .push(rule);
            self
        }
    };
}

/// Network settings for a new sandbox ([`crate::CreateSandboxParams::network`]).
///
/// Each setting is sent only when set. Settings with nothing set are not sent
/// at all, as the SDK leaves out an empty `network`.
///
/// # Examples
///
/// ```
/// use e2b_client::{ALL_TRAFFIC, CreateSandboxParams, NetworkConfig};
///
/// // Deny all egress except one address, and keep the sandbox's URLs private.
/// let network = NetworkConfig::new()
///     .deny_out([ALL_TRAFFIC])
///     .allow_out(["1.1.1.1"])
///     .allow_public_traffic(false);
/// let params = CreateSandboxParams::new("base").network(network);
/// # let _ = params;
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkConfig {
    // Rust note: `flatten` writes the egress fields at this level of the
    // JSON object, as if they were declared here.
    #[serde(flatten)]
    egress: Egress,
    #[serde(skip_serializing_if = "Option::is_none")]
    allow_public_traffic: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mask_request_host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    https_ports: Option<Vec<u16>>,
}

impl NetworkConfig {
    /// No settings: the server's defaults apply.
    ///
    /// # Returns
    ///
    /// Empty settings.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    egress_setters!();

    /// Whether the sandbox's URLs ([`crate::Sandbox::get_host`]) are public.
    /// When `false`, requests to them need the sandbox's traffic access
    /// token. Unset, the project's default applies (public, unless the
    /// project changed it).
    ///
    /// # Arguments
    ///
    /// * `allow` - whether public traffic is allowed.
    ///
    /// # Returns
    ///
    /// The settings, for chaining.
    #[must_use]
    pub fn allow_public_traffic(mut self, allow: bool) -> Self {
        self.allow_public_traffic = Some(allow);
        self
    }

    /// A custom host mask for requests to the sandbox, with `${PORT}` for the
    /// port, e.g. `"${PORT}-myapp.example.com"`.
    ///
    /// # Arguments
    ///
    /// * `mask` - the host mask.
    ///
    /// # Returns
    ///
    /// The settings, for chaining.
    #[must_use]
    pub fn mask_request_host(mut self, mask: impl Into<String>) -> Self {
        self.mask_request_host = Some(mask.into());
        self
    }

    /// Ports in the sandbox that serve HTTPS rather than plain HTTP, for
    /// services that terminate TLS themselves. The proxy does not verify
    /// their certificates, so self-signed ones work. The envd port (49983)
    /// cannot be listed.
    ///
    /// # Arguments
    ///
    /// * `ports` - the ports.
    ///
    /// # Returns
    ///
    /// The settings, for chaining.
    #[must_use]
    pub fn https_ports(mut self, ports: impl IntoIterator<Item = u16>) -> Self {
        self.https_ports = Some(ports.into_iter().collect());
        self
    }

    /// Whether nothing is set, in which case the create body leaves `network`
    /// out (`network_body if network_body else UNSET`,
    /// `e2b/sandbox_async/sandbox_api.py:250`).
    pub(crate) fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

/// A replacement for a running sandbox's egress settings
/// ([`crate::Client::update_sandbox_network`], `SandboxNetworkUpdate`,
/// `e2b/sandbox/sandbox_api.py:515-561`).
///
/// The update replaces the whole egress configuration: every field left
/// unset is cleared on the server (DNS servers go back to `8.8.8.8`). Repeat
/// every setting that should stay. An update with nothing set is sent as
/// `{}`, which clears everything.
///
/// # Examples
///
/// ```
/// use e2b_client::{ALL_TRAFFIC, NetworkUpdate};
///
/// // Deny all egress except one address; any earlier rules are cleared.
/// let lock_down = NetworkUpdate::new().deny_out([ALL_TRAFFIC]).allow_out(["1.1.1.1"]);
/// # let _ = lock_down;
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct NetworkUpdate {
    // Rust note: as in `NetworkConfig`, `flatten` writes the camelCase egress
    // fields at the top level of the JSON object.
    #[serde(flatten)]
    egress: Egress,
    // The spec spells this one field in snake_case, so it has no
    // `rename_all` (`sandbox_network_update_config.py:102-103`).
    #[serde(skip_serializing_if = "Option::is_none")]
    allow_internet_access: Option<bool>,
}

impl NetworkUpdate {
    /// An update that sets nothing: sent as is, it clears every egress
    /// setting.
    ///
    /// # Returns
    ///
    /// An empty update.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    egress_setters!();

    /// Allows or denies all internet access; `false` is the same as
    /// `deny_out([ALL_TRAFFIC])`.
    ///
    /// # Arguments
    ///
    /// * `allow` - whether the sandbox may reach the internet.
    ///
    /// # Returns
    ///
    /// The update, for chaining.
    #[must_use]
    pub fn allow_internet_access(mut self, allow: bool) -> Self {
        self.allow_internet_access = Some(allow);
        self
    }
}

/// A rule for outbound HTTP and HTTPS requests to one host: headers to set
/// on each request (`SandboxNetworkRule`, `e2b/sandbox/sandbox_api.py:149-224`).
///
/// Header values are often credentials, so `Debug` prints header names only.
///
/// # Examples
///
/// ```
/// use e2b_client::NetworkRule;
///
/// let rule = NetworkRule::new().header("X-Api-Key", "secret");
/// assert_eq!(rule.headers()["X-Api-Key"], "secret");
/// assert!(!format!("{rule:?}").contains("secret"));
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkRule {
    // A rule without headers is sent as `{}`, as the SDK sends a rule
    // without a transform (`e2b/sandbox/sandbox_api.py:834-837`).
    #[serde(
        default,
        deserialize_with = "deserialize_null_as_default",
        skip_serializing_if = "NetworkTransform::is_empty"
    )]
    transform: NetworkTransform,
}

impl NetworkRule {
    /// A rule that changes nothing yet.
    ///
    /// # Returns
    ///
    /// An empty rule.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets `name: value` on every matching request, replacing a header of
    /// that name the request already has. Setting the same name twice keeps
    /// the last value.
    ///
    /// # Arguments
    ///
    /// * `name` - the header name.
    /// * `value` - the header value.
    ///
    /// # Returns
    ///
    /// The rule, for chaining.
    #[must_use]
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.transform.headers.insert(name.into(), value.into());
        self
    }

    /// The headers this rule sets.
    ///
    /// # Returns
    ///
    /// Header names and values, sorted by name.
    #[must_use]
    pub fn headers(&self) -> &BTreeMap<String, String> {
        &self.transform.headers
    }
}

/// `SandboxNetworkTransform`: what a rule changes.
#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
struct NetworkTransform {
    #[serde(
        default,
        deserialize_with = "deserialize_null_as_default",
        skip_serializing_if = "BTreeMap::is_empty"
    )]
    headers: BTreeMap<String, String>,
}

impl NetworkTransform {
    fn is_empty(&self) -> bool {
        self.headers.is_empty()
    }
}

// Header values are often credentials (`Authorization`), so `Debug` shows
// only the header names (CLM-0122's rule). Rust note: writing `impl Debug`
// by hand, instead of `#[derive(Debug)]`, is how a type controls what
// `{:?}` prints, like a custom `__repr__`.
impl fmt::Debug for NetworkTransform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NetworkTransform")
            .field("headers", &self.headers.keys().collect::<Vec<_>>())
            .finish()
    }
}

/// A sandbox's network settings as [`crate::Client::get_sandbox_info`]
/// reports them (`SandboxNetworkConfig` in `SandboxDetail`). A missing or
/// `null` list or map reads as empty.
///
/// This is a plain data record, so its fields are public. It is
/// `#[non_exhaustive]` so fields can be added without a breaking change.
///
/// # Examples
///
/// ```no_run
/// use e2b_client::RequestOptions;
///
/// # async fn demo(sandbox: e2b_client::Sandbox) -> e2b_client::Result<()> {
/// if let Some(network) = sandbox.get_info(&RequestOptions::new()).await?.network {
///     println!("allow {:?}, deny {:?}", network.allow_out, network.deny_out);
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct NetworkInfo {
    /// Allowed outbound destinations.
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub allow_out: Vec<String>,
    /// Denied outbound destinations.
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub deny_out: Vec<String>,
    /// The DNS servers the sandbox may reach; `None` means the default,
    /// `8.8.8.8`, and an empty list means none.
    #[serde(default)]
    pub dns_servers: Option<Vec<String>>,
    /// Whether the sandbox's URLs are public, when reported.
    #[serde(default)]
    pub allow_public_traffic: Option<bool>,
    /// The custom host mask, if any.
    #[serde(default)]
    pub mask_request_host: Option<String>,
    /// Ports served over HTTPS.
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub https_ports: Vec<u16>,
    /// Per-host rules.
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub rules: BTreeMap<String, Vec<NetworkRule>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn deny_all_serializes_to_only_deny_out() {
        // Arrange
        let network = NetworkConfig::new().deny_out([ALL_TRAFFIC]);

        // Act
        let value = serde_json::to_value(&network).expect("serialize");

        // Assert
        assert_eq!(value, json!({"denyOut": ["0.0.0.0/0"]}));
    }

    #[test]
    fn rule_debug_hides_header_values() {
        // Arrange
        let rule = NetworkRule::new().header("Authorization", "Bearer s3cret");

        // Act
        let debug = format!("{rule:?}");

        // Assert
        assert!(debug.contains("Authorization"), "{debug}");
        assert!(!debug.contains("s3cret"), "{debug}");
    }

    #[test]
    fn config_debug_hides_rule_header_values() {
        let network = NetworkConfig::new().rule("a.test", NetworkRule::new().header("K", "s3cret"));

        let debug = format!("{network:?}");

        assert!(!debug.contains("s3cret"), "{debug}");
    }

    #[test]
    fn rules_for_one_host_keep_their_order_and_an_empty_rule_is_an_empty_object() {
        let network = NetworkConfig::new()
            .rule("a.test", NetworkRule::new().header("X-First", "1"))
            .rule("a.test", NetworkRule::new());

        let value = serde_json::to_value(&network).expect("serialize");

        assert_eq!(
            value,
            json!({"rules": {"a.test": [{"transform": {"headers": {"X-First": "1"}}}, {}]}})
        );
    }

    #[test]
    fn only_a_config_with_nothing_set_is_empty() {
        assert!(NetworkConfig::new().is_empty());
        assert!(
            !NetworkConfig::new()
                .dns_servers(Vec::<String>::new())
                .is_empty()
        );
        assert!(!NetworkConfig::new().allow_public_traffic(true).is_empty());
    }

    #[test]
    fn update_sends_allow_internet_access_in_snake_case_next_to_camel_case_egress() {
        // Arrange
        let update = NetworkUpdate::new()
            .dns_servers(Vec::<String>::new())
            .allow_internet_access(false);

        // Act
        let value = serde_json::to_value(&update).expect("serialize");

        // Assert
        assert_eq!(
            value,
            json!({"dnsServers": [], "allow_internet_access": false})
        );
    }

    #[test]
    fn update_debug_hides_rule_header_values() {
        let update = NetworkUpdate::new().rule("a.test", NetworkRule::new().header("K", "s3cret"));

        let debug = format!("{update:?}");

        assert!(debug.contains("a.test"), "{debug}");
        assert!(!debug.contains("s3cret"), "{debug}");
    }

    #[test]
    fn rule_reads_null_transform_and_null_headers_as_empty() {
        let null_transform: NetworkRule =
            serde_json::from_value(json!({"transform": null})).expect("null transform");
        let null_headers: NetworkRule =
            serde_json::from_value(json!({"transform": {"headers": null}})).expect("null headers");

        assert!(null_transform.headers().is_empty());
        assert!(null_headers.headers().is_empty());
    }

    #[test]
    fn info_reads_null_fields_as_empty_or_none() {
        let info: NetworkInfo = serde_json::from_value(json!({
            "allowOut": null, "denyOut": null, "dnsServers": null, "allowPublicTraffic": null,
            "maskRequestHost": null, "httpsPorts": null, "rules": null, "egressProxy": null
        }))
        .expect("info");

        assert_eq!(info, NetworkInfo::default());
    }
}
