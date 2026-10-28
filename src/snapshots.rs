//! Snapshots: persistent copies of a sandbox that new sandboxes can start
//! from (`e2b/sandbox_async/main.py:886-1060`).

use std::borrow::Cow;

use serde::{Deserialize, Serialize};

use crate::lifecycle::SnapshotMode;
use crate::wire::deserialize_null_as_default;

/// Options for [`crate::Client::create_snapshot`].
///
/// # Examples
///
/// ```
/// use e2b_client::{SnapshotMode, SnapshotOptions};
///
/// let options = SnapshotOptions::new().name("before-upgrade").mode(SnapshotMode::Full);
/// # let _ = options;
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SnapshotOptions {
    name: Option<String>,
    mode: Option<SnapshotMode>,
}

impl SnapshotOptions {
    /// An unnamed snapshot with the API's default mode (full).
    ///
    /// # Returns
    ///
    /// Empty options.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Names the snapshot. If a snapshot with this name exists, the new one
    /// becomes a new build of it instead of a new snapshot. An empty name is
    /// not sent, as in the SDK (`name if name else UNSET`).
    ///
    /// # Arguments
    ///
    /// * `name` - the snapshot name.
    ///
    /// # Returns
    ///
    /// The options, for chaining.
    #[must_use]
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// What the snapshot keeps. The source sandbox keeps running either way.
    /// Sandboxes made from a [`SnapshotMode::Filesystem`] snapshot cold-boot
    /// from disk, without the source's processes and memory; on hosted E2B
    /// the `base` template empties `/tmp` at every boot, so files meant to
    /// survive such a snapshot belong elsewhere, such as `/home/user`.
    ///
    /// The SDK documents two refusals of a filesystem-only snapshot
    /// (`py: sandbox_async/main.py:905`): HTTP 400 when the feature is not
    /// enabled for the team (`snapshot_filesystem_only_disabled`), and 409
    /// when the sandbox's node predates the option
    /// (`snapshot_filesystem_only_unsupported_node`; pausing and resuming the
    /// sandbox, then retrying, may help). A full snapshot works in both
    /// cases. Both reach the caller as [`crate::Error::Api`].
    ///
    /// # Arguments
    ///
    /// * `mode` - full or filesystem-only.
    ///
    /// # Returns
    ///
    /// The options, for chaining.
    #[must_use]
    pub fn mode(mut self, mode: SnapshotMode) -> Self {
        self.mode = Some(mode);
        self
    }

    /// The request body, `SandboxSnapshotRequest` (`{name?, memory?}`,
    /// `e2b/sandbox_async/sandbox_api.py:350-356`).
    ///
    /// # Returns
    ///
    /// A body borrowing the name; an unset or empty name and an unset mode
    /// are left out.
    pub(crate) fn body(&self) -> SnapshotRequest<'_> {
        SnapshotRequest {
            // `filter` keeps the `Some` only when the closure is true, the
            // SDK's `name if name else UNSET`.
            name: self.name.as_deref().filter(|name| !name.is_empty()),
            memory: self.mode.map(SnapshotMode::keeps_memory),
        }
    }
}

/// The wire form of [`SnapshotOptions`]. `skip_serializing_if` leaves a
/// `None` field out of the JSON, where serde would otherwise write `null`.
#[derive(Debug, Serialize)]
pub(crate) struct SnapshotRequest<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    memory: Option<bool>,
}

/// A snapshot (`SnapshotInfo`, `e2b/sandbox/sandbox_api.py:1364-1370`).
/// Create a sandbox from it with `CreateSandboxParams::new(snapshot_id)`.
///
/// This is a plain data record, so its fields are public. It is
/// `#[non_exhaustive]` so fields can be added without a breaking change.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[non_exhaustive]
pub struct SnapshotInfo {
    /// The snapshot's ID: a template ID with a tag, or a namespaced name with
    /// a tag, e.g. `my-snapshot:latest`.
    #[serde(rename = "snapshotID")]
    pub snapshot_id: String,
    /// The snapshot's full names, e.g. `project-slug/my-snapshot:v2`; empty
    /// when the API sends none or `null`, as in the SDK
    /// (`list(names) if names else []`).
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub names: Vec<String>,
}

/// Filters and paging for [`crate::Client::list_snapshots`]. As in the SDK,
/// an empty string or a zero limit counts as unset and is not sent
/// (`e2b/sandbox_async/paginator.py:136-140`).
///
/// # Examples
///
/// ```
/// use e2b_client::ListSnapshotsParams;
///
/// let params = ListSnapshotsParams::new().name("before-upgrade").limit(10);
/// # let _ = params;
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ListSnapshotsParams {
    sandbox_id: Option<String>,
    name: Option<String>,
    limit: Option<u32>,
    next_token: Option<String>,
}

impl ListSnapshotsParams {
    /// No filters: every snapshot of the team, in pages of the API's default
    /// size.
    ///
    /// # Returns
    ///
    /// Empty parameters.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Only snapshots taken from this sandbox.
    ///
    /// # Arguments
    ///
    /// * `sandbox_id` - the source sandbox.
    ///
    /// # Returns
    ///
    /// The parameters, for chaining.
    #[must_use]
    pub fn sandbox_id(mut self, sandbox_id: impl Into<String>) -> Self {
        self.sandbox_id = Some(sandbox_id.into());
        self
    }

    /// Only snapshots with this name or ID, optionally tag-qualified
    /// (`my-snapshot`, `my-project/my-snapshot` or `my-snapshot:v1`).
    ///
    /// # Arguments
    ///
    /// * `name` - the name or ID.
    ///
    /// # Returns
    ///
    /// The parameters, for chaining.
    #[must_use]
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Maximum number of snapshots per page.
    ///
    /// # Arguments
    ///
    /// * `limit` - the page size; 0 leaves it to the API.
    ///
    /// # Returns
    ///
    /// The parameters, for chaining.
    #[must_use]
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Continue from a previous page's [`SnapshotPage::next_token`].
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

    /// The query pairs (`get_snapshots.py:22-30`), in the SDK's order:
    /// `sandboxID`, `name`, `limit`, `nextToken`.
    ///
    /// # Returns
    ///
    /// The set pairs; an unset or empty value, or a zero limit, is left out.
    pub(crate) fn query(&self) -> Vec<(&'static str, Cow<'_, str>)> {
        // `Cow` lets the string filters be borrowed while the limit, which
        // has to be formatted, is owned: no copy of the strings is made. A
        // nested `fn` rather than a closure, because only a `fn` signature
        // can say that the result borrows from the argument.
        fn borrowed(value: &Option<String>) -> Option<Cow<'_, str>> {
            value
                .as_deref()
                .filter(|value| !value.is_empty())
                .map(Cow::Borrowed)
        }
        [
            ("sandboxID", borrowed(&self.sandbox_id)),
            ("name", borrowed(&self.name)),
            (
                "limit",
                self.limit
                    .filter(|limit| *limit != 0)
                    .map(|limit| Cow::Owned(limit.to_string())),
            ),
            ("nextToken", borrowed(&self.next_token)),
        ]
        .into_iter()
        .filter_map(|(key, value)| value.map(|value| (key, value)))
        .collect()
    }
}

/// One page of [`crate::Client::list_snapshots`] results.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SnapshotPage {
    /// The snapshots on this page.
    pub snapshots: Vec<SnapshotInfo>,
    /// The token for the next page (from `x-next-token`), or `None` on the
    /// last page.
    pub next_token: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body_json(options: &SnapshotOptions) -> serde_json::Value {
        serde_json::to_value(options.body()).expect("serialize")
    }

    #[test]
    fn body_sends_the_name_and_the_memory_flag_only_when_set() {
        assert_eq!(body_json(&SnapshotOptions::new()), serde_json::json!({}));
        assert_eq!(
            body_json(&SnapshotOptions::new().name("snap").mode(SnapshotMode::Full)),
            serde_json::json!({"name": "snap", "memory": true})
        );
        assert_eq!(
            body_json(&SnapshotOptions::new().mode(SnapshotMode::Filesystem)),
            serde_json::json!({"memory": false})
        );
    }

    #[test]
    fn body_leaves_out_an_empty_name() {
        assert_eq!(
            body_json(&SnapshotOptions::new().name("")),
            serde_json::json!({})
        );
    }

    #[test]
    fn query_keeps_the_sdk_order_whatever_the_setter_order() {
        // Arrange
        let params = ListSnapshotsParams::new()
            .next_token("t")
            .limit(5)
            .name("n")
            .sandbox_id("s");

        // Act
        let query = params.query();

        // Assert
        let pairs: Vec<(&str, &str)> = query
            .iter()
            .map(|(key, value)| (*key, value.as_ref()))
            .collect();
        assert_eq!(
            pairs,
            [
                ("sandboxID", "s"),
                ("name", "n"),
                ("limit", "5"),
                ("nextToken", "t")
            ]
        );
    }

    #[test]
    fn query_leaves_out_empty_strings_and_a_zero_limit() {
        let params = ListSnapshotsParams::new()
            .sandbox_id("")
            .name("")
            .limit(0)
            .next_token("");
        assert!(params.query().is_empty());
    }

    #[test]
    fn snapshot_info_reads_missing_or_null_names_as_empty() {
        for body in [
            serde_json::json!({"snapshotID": "t:1"}),
            serde_json::json!({"snapshotID": "t:1", "names": null}),
        ] {
            let info: SnapshotInfo = serde_json::from_value(body).expect("decode");
            assert_eq!(info.snapshot_id, "t:1");
            assert!(info.names.is_empty());
        }
    }
}
