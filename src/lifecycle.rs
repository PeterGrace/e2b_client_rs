//! What a sandbox does when its timeout expires, and what a pause or a
//! snapshot keeps (`e2b/sandbox/sandbox_api.py:627-662, 1046-1125`).

use serde::{Deserialize, Deserializer, Serialize};

use crate::error::{Error, Result};

/// What a pause or snapshot persists (`SnapshotMode`,
/// `e2b/sandbox/sandbox_api.py:627-636`).
///
/// `#[non_exhaustive]` means code outside this crate must keep a `_` arm when
/// it `match`es on the enum, so a mode can be added later without breaking
/// callers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum SnapshotMode {
    /// The filesystem and the memory: running processes, in-memory state
    /// and open connections come back on resume.
    Full,
    /// Only the filesystem: smaller and faster, and resuming cold-boots the
    /// sandbox from disk.
    Filesystem,
}

impl SnapshotMode {
    /// The API's `memory` flag for this mode (`resolve_snapshot_memory`,
    /// `e2b/sandbox/sandbox_api.py:639-662`): `true` only for a full snapshot.
    pub(crate) fn keeps_memory(self) -> bool {
        matches!(self, Self::Full)
    }
}

/// What happens when a sandbox's timeout expires.
///
/// The SDK refuses a `mode` on a kill at run time; here [`OnTimeout::Kill`]
/// has no mode field, so the compiler rules that combination out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum OnTimeout {
    /// Kill the sandbox (the API's default).
    Kill,
    /// Pause it, so [`crate::Client::connect_sandbox`] can resume it.
    Pause {
        /// What the pause keeps; `None` leaves it to the API (currently
        /// [`SnapshotMode::Full`]).
        mode: Option<SnapshotMode>,
    },
}

impl OnTimeout {
    /// Pause on timeout, with the API's default snapshot.
    ///
    /// # Returns
    ///
    /// [`OnTimeout::Pause`] with no mode.
    #[must_use]
    pub fn pause() -> Self {
        Self::Pause { mode: None }
    }

    /// Pause on timeout, keeping what `mode` says.
    ///
    /// # Arguments
    ///
    /// * `mode` - full or filesystem-only.
    ///
    /// # Returns
    ///
    /// [`OnTimeout::Pause`] with that mode.
    #[must_use]
    pub fn pause_with(mode: SnapshotMode) -> Self {
        Self::Pause { mode: Some(mode) }
    }
}

/// Lifecycle settings for a new sandbox ([`crate::CreateSandboxParams::lifecycle`],
/// the SDK's `lifecycle`). Unset fields are left to the API.
///
/// # Examples
///
/// ```
/// use e2b_client::{CreateSandboxParams, Lifecycle, OnTimeout, SnapshotMode};
///
/// let lifecycle = Lifecycle::new()
///     .on_timeout(OnTimeout::pause_with(SnapshotMode::Full))
///     .auto_resume(true);
/// let params = CreateSandboxParams::new("base").lifecycle(lifecycle);
/// # let _ = params;
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Lifecycle {
    on_timeout: Option<OnTimeout>,
    auto_resume: Option<bool>,
}

impl Lifecycle {
    /// Nothing set: the API's defaults (kill on timeout).
    ///
    /// # Returns
    ///
    /// Empty settings.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// What happens at the timeout.
    ///
    /// # Arguments
    ///
    /// * `on_timeout` - kill, or pause with an optional mode.
    ///
    /// # Returns
    ///
    /// The settings, for chaining.
    #[must_use]
    pub fn on_timeout(mut self, on_timeout: OnTimeout) -> Self {
        self.on_timeout = Some(on_timeout);
        self
    }

    /// Whether traffic to a paused sandbox resumes it. `true` needs
    /// [`OnTimeout::Pause`] without [`SnapshotMode::Filesystem`]; `false`
    /// opts out even if the API's default changes.
    ///
    /// # Arguments
    ///
    /// * `auto_resume` - whether activity resumes the sandbox.
    ///
    /// # Returns
    ///
    /// The settings, for chaining.
    #[must_use]
    pub fn auto_resume(mut self, auto_resume: bool) -> Self {
        self.auto_resume = Some(auto_resume);
        self
    }

    /// The create-body fields, after the SDK's checks
    /// (`build_lifecycle_config`, `e2b/sandbox/sandbox_api.py:1054-1125`).
    ///
    /// # Returns
    ///
    /// `autoPause` only when `on_timeout` is set, `autoPauseMemory` only for
    /// a pause with a mode, and `autoResume` only when `auto_resume` is set.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidArgument`] for `auto_resume(true)` without a pause on
    /// timeout, or with a filesystem-only pause.
    pub(crate) fn body(&self) -> Result<LifecycleBody> {
        // `Some(mode)` for a pause (whose own mode may be `None`), `None`
        // otherwise. An unset `on_timeout` counts as a kill for the checks,
        // as in the SDK, but is still left out of the body.
        let pause_mode = match self.on_timeout {
            Some(OnTimeout::Pause { mode }) => Some(mode),
            Some(OnTimeout::Kill) | None => None,
        };
        if self.auto_resume == Some(true) {
            match pause_mode {
                None => {
                    return Err(Error::InvalidArgument(
                        "auto_resume can only be true when on_timeout is a pause".into(),
                    ));
                }
                Some(Some(SnapshotMode::Filesystem)) => {
                    return Err(Error::InvalidArgument(
                        "auto_resume cannot be true with a filesystem-only pause: such a \
                         sandbox must be resumed with connect_sandbox"
                            .into(),
                    ));
                }
                Some(Some(SnapshotMode::Full) | None) => {}
            }
        }
        Ok(LifecycleBody {
            auto_pause: self
                .on_timeout
                .map(|on_timeout| matches!(on_timeout, OnTimeout::Pause { .. })),
            // `flatten` turns `Option<Option<_>>` into `Option<_>`, like
            // collapsing a nested "maybe a maybe".
            auto_pause_memory: pause_mode.flatten().map(SnapshotMode::keeps_memory),
            auto_resume: self.auto_resume.map(|enabled| AutoResume { enabled }),
        })
    }
}

/// The lifecycle fields of `NewSandboxV2` (`SandboxLifecycleBody` in the SDK).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LifecycleBody {
    pub(crate) auto_pause: Option<bool>,
    pub(crate) auto_pause_memory: Option<bool>,
    pub(crate) auto_resume: Option<AutoResume>,
}

/// `SandboxAutoResumeConfig`: `{enabled}`
/// (`e2b/api/client/models/sandbox_auto_resume_config.py:18, 37`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub(crate) struct AutoResume {
    enabled: bool,
}

/// The timeout action a sandbox reports (`SandboxLifecycle.onTimeout`,
/// `e2b/api/client/models/sandbox_lifecycle.py:44-46`).
///
/// The default is [`TimeoutAction::Unknown`], which is what a missing or
/// `null` `onTimeout` reads as.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum TimeoutAction {
    /// Killed at the timeout.
    Kill,
    /// Paused at the timeout.
    Pause,
    /// An action this version of the crate does not know, or none reported.
    // `serde(other)` maps any other string here instead of failing, and
    // `#[default]` makes this the value `Default::default()` returns.
    #[serde(other)]
    #[default]
    Unknown,
}

/// A sandbox's lifecycle as [`crate::Client::get_sandbox_info`] reports it.
///
/// This is a plain data record, so its fields are public. It is
/// `#[non_exhaustive]` so fields can be added without a breaking change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct LifecycleInfo {
    /// What happens at the timeout; [`TimeoutAction::Unknown`] when the API
    /// leaves it out or sends `null`.
    #[serde(default, deserialize_with = "deserialize_action_or_unknown")]
    pub on_timeout: TimeoutAction,
    /// Whether activity resumes the paused sandbox.
    #[serde(default)]
    pub auto_resume: bool,
}

/// Reads `onTimeout`, treating `null` as [`TimeoutAction::Unknown`], so a
/// lifecycle without an action does not fail the whole detail. (A missing
/// field never reaches this function: `#[serde(default)]` covers it.)
///
/// # Arguments
///
/// * `deserializer` - serde's source for the field.
///
/// # Returns
///
/// The action, or [`TimeoutAction::Unknown`] for `null`.
///
/// # Errors
///
/// The deserializer's error when the value is neither a string nor `null`.
fn deserialize_action_or_unknown<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<TimeoutAction, D::Error> {
    // `unwrap_or_default` turns `None` (JSON `null`) into `TimeoutAction::Unknown`.
    Ok(Option::<TimeoutAction>::deserialize(deserializer)?.unwrap_or_default())
}

/// Reads `pauseMode`, treating `null` and values this crate does not know as
/// `None`, so a mode the API adds later does not fail the whole detail.
///
/// # Arguments
///
/// * `deserializer` - serde's source for the field.
///
/// # Returns
///
/// The mode, or `None`.
///
/// # Errors
///
/// The deserializer's error when the value is neither a string nor `null`.
pub(crate) fn deserialize_known_mode<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Option<SnapshotMode>, D::Error> {
    // `std::result::Result` is spelled out because `crate::error::Result`
    // (a one-parameter alias) is in scope.
    let raw = Option::<String>::deserialize(deserializer)?;
    Ok(match raw.as_deref() {
        Some("full") => Some(SnapshotMode::Full),
        Some("filesystem") => Some(SnapshotMode::Filesystem),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The body of `lifecycle` as JSON, to compare with what the SDK sends.
    fn body_json(lifecycle: Lifecycle) -> serde_json::Value {
        let body = lifecycle.body().expect("valid lifecycle");
        serde_json::json!({
            "autoPause": body.auto_pause,
            "autoPauseMemory": body.auto_pause_memory,
            "autoResume": body.auto_resume,
        })
    }

    #[test]
    fn unset_lifecycle_sends_nothing() {
        assert_eq!(
            Lifecycle::new().body().expect("body"),
            LifecycleBody {
                auto_pause: None,
                auto_pause_memory: None,
                auto_resume: None
            }
        );
    }

    #[test]
    fn valid_shapes_match_the_sdk_body() {
        // Arrange: (lifecycle, autoPause, autoPauseMemory, autoResume).
        let cases = [
            (
                Lifecycle::new()
                    .on_timeout(OnTimeout::pause_with(SnapshotMode::Filesystem))
                    .auto_resume(false),
                serde_json::json!({"autoPause": true, "autoPauseMemory": false,
                    "autoResume": {"enabled": false}}),
            ),
            (
                Lifecycle::new()
                    .on_timeout(OnTimeout::pause_with(SnapshotMode::Full))
                    .auto_resume(true),
                serde_json::json!({"autoPause": true, "autoPauseMemory": true,
                    "autoResume": {"enabled": true}}),
            ),
            (
                Lifecycle::new()
                    .on_timeout(OnTimeout::pause())
                    .auto_resume(true),
                serde_json::json!({"autoPause": true, "autoPauseMemory": null,
                    "autoResume": {"enabled": true}}),
            ),
            (
                Lifecycle::new().on_timeout(OnTimeout::Kill),
                serde_json::json!({"autoPause": false, "autoPauseMemory": null,
                    "autoResume": null}),
            ),
            // `auto_resume(false)` needs no pause: the SDK only refuses `true`.
            (
                Lifecycle::new().auto_resume(false),
                serde_json::json!({"autoPause": null, "autoPauseMemory": null,
                    "autoResume": {"enabled": false}}),
            ),
        ];

        for (lifecycle, expected) in cases {
            // Act and Assert
            assert_eq!(body_json(lifecycle), expected, "{lifecycle:?}");
        }
    }

    #[test]
    fn auto_resume_without_a_pause_is_refused() {
        for lifecycle in [
            Lifecycle::new().auto_resume(true),
            Lifecycle::new()
                .on_timeout(OnTimeout::Kill)
                .auto_resume(true),
        ] {
            let error = lifecycle.body().expect_err("refused");
            assert!(
                matches!(&error, Error::InvalidArgument(message) if message.contains("pause")),
                "{lifecycle:?}: {error:?}"
            );
        }
    }

    #[test]
    fn auto_resume_with_a_filesystem_pause_is_refused() {
        let lifecycle = Lifecycle::new()
            .on_timeout(OnTimeout::pause_with(SnapshotMode::Filesystem))
            .auto_resume(true);

        let error = lifecycle.body().expect_err("refused");

        assert!(
            matches!(&error, Error::InvalidArgument(message) if message.contains("filesystem")),
            "{error:?}"
        );
    }

    #[test]
    fn the_last_on_timeout_wins() {
        let lifecycle = Lifecycle::new()
            .on_timeout(OnTimeout::pause_with(SnapshotMode::Full))
            .on_timeout(OnTimeout::Kill);

        assert_eq!(lifecycle.body().expect("body").auto_pause, Some(false));
        assert_eq!(lifecycle.body().expect("body").auto_pause_memory, None);
    }

    #[test]
    fn lifecycle_info_reads_a_missing_or_null_action_as_unknown() {
        for json in [
            r#"{"autoResume": true}"#,
            r#"{"onTimeout": null, "autoResume": true}"#,
            r#"{"onTimeout": "hibernate", "autoResume": true}"#,
        ] {
            let info: LifecycleInfo = serde_json::from_str(json).expect(json);
            assert_eq!(info.on_timeout, TimeoutAction::Unknown, "{json}");
            assert!(info.auto_resume, "{json}");
        }
        let info: LifecycleInfo = serde_json::from_str(r#"{"onTimeout": "pause"}"#).expect("pause");
        assert_eq!(info.on_timeout, TimeoutAction::Pause);
        assert!(!info.auto_resume);
    }

    #[test]
    fn snapshot_mode_uses_the_sdk_spelling() {
        assert_eq!(
            serde_json::to_value(SnapshotMode::Full).expect("full"),
            serde_json::json!("full")
        );
        assert_eq!(
            serde_json::to_value(SnapshotMode::Filesystem).expect("filesystem"),
            serde_json::json!("filesystem")
        );
        assert!(SnapshotMode::Full.keeps_memory());
        assert!(!SnapshotMode::Filesystem.keeps_memory());
    }
}
