//! Volumes: persistent storage that outlives sandboxes and can be mounted
//! into them (`e2b/volume/volume_async.py`). This module covers managing
//! volumes ([`crate::Client::create_volume`] and its siblings) and mounting
//! them ([`crate::CreateSandboxParams::volume_mount`]); reading and writing
//! a volume's files without a sandbox (the volume content API) is not
//! implemented.

use std::fmt;

use secrecy::SecretString;
use serde::{Deserialize, Serialize};

/// A volume, with the token for its content API (`VolumeAndToken`,
/// `e2b/volume/types.py:21-30`), from [`crate::Client::create_volume`] or
/// [`crate::Client::get_volume`].
///
/// Volumes are persistent and billed: they outlive every sandbox that mounts
/// them until [`crate::Client::destroy_volume`] removes them.
///
/// `Debug` prints the ID, name and domain but never the token. `Clone`
/// copies the token too, still wrapped (Python note: Rust values are moved,
/// not shared, unless cloned, so a handle you also pass on needs a clone).
///
/// # Examples
///
/// ```no_run
/// use e2b_client::{CreateSandboxParams, RequestOptions};
///
/// # async fn demo(client: e2b_client::Client) -> e2b_client::Result<()> {
/// let volume = client.create_volume("datasets", &RequestOptions::new()).await?;
/// let params = CreateSandboxParams::new("base").volume_mount(volume.name(), "/mnt/data");
/// let sandbox = client.create_sandbox(&params).await?;
/// # let _ = sandbox;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct Volume {
    volume_id: String,
    name: String,
    token: SecretString,
    domain: Option<String>,
}

impl Volume {
    /// The volume ID, which [`crate::Client::get_volume`] and
    /// [`crate::Client::destroy_volume`] take.
    ///
    /// # Returns
    ///
    /// The ID.
    #[must_use]
    pub fn volume_id(&self) -> &str {
        &self.volume_id
    }

    /// The volume name, which [`crate::CreateSandboxParams::volume_mount`]
    /// takes.
    ///
    /// # Returns
    ///
    /// The name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The token for the volume content API. The crate does not use it (the
    /// content API is not implemented); it is returned so a caller can.
    ///
    /// # Returns
    ///
    /// The token, wrapped in a [`SecretString`] so it is not printed by
    /// accident. Python note: call `secrecy::ExposeSecret::expose_secret` on
    /// it to read the string, the way you would unwrap pydantic's
    /// `SecretStr` with `get_secret_value()`.
    #[must_use]
    pub fn token(&self) -> &SecretString {
        &self.token
    }

    /// The host for volume content requests, as the API reports it. Only
    /// teams on a custom (BYOC) cluster get one; an empty domain reads as
    /// `None`, as in the SDK (`e2b/volume/volume_async.py:159-163, 227-231`). The SDK
    /// falls back to the configured domain where this is `None`.
    ///
    /// # Returns
    ///
    /// The domain, if the API sent a non-empty one.
    #[must_use]
    pub fn domain(&self) -> Option<&str> {
        self.domain.as_deref()
    }
}

// Written by hand rather than derived, so the token can never reach a log.
// `finish_non_exhaustive` prints `..` where the token would be.
impl fmt::Debug for Volume {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Volume")
            .field("volume_id", &self.volume_id)
            .field("name", &self.name)
            .field("domain", &self.domain)
            .finish_non_exhaustive()
    }
}

/// `VolumeAndToken` as the API sends it
/// (`e2b/api/client/models/volume_and_token.py:50-63`).
#[derive(Deserialize)]
pub(crate) struct VolumeWire {
    #[serde(rename = "volumeID")]
    volume_id: String,
    name: String,
    token: String,
    #[serde(default)]
    domain: Option<String>,
}

impl From<VolumeWire> for Volume {
    fn from(wire: VolumeWire) -> Self {
        Self {
            volume_id: wire.volume_id,
            name: wire.name,
            token: SecretString::from(wire.token),
            // `Option::filter` turns `Some("")` into `None`, as the SDK's
            // `domain if isinstance(domain, str) and domain else None`.
            domain: wire.domain.filter(|domain| !domain.is_empty()),
        }
    }
}

/// `NewVolume` request body: `{name}`.
#[derive(Serialize)]
pub(crate) struct NewVolumeBody<'a> {
    pub(crate) name: &'a str,
}

/// One entry of [`crate::Client::list_volumes`] (`VolumeInfo`,
/// `e2b/volume/types.py:11-18`): no token.
///
/// This is a plain data record, so its fields are public. It is
/// `#[non_exhaustive]` so fields can be added without a breaking change.
///
/// # Examples
///
/// ```no_run
/// use e2b_client::RequestOptions;
///
/// # async fn demo(client: e2b_client::Client) -> e2b_client::Result<()> {
/// let volumes = client.list_volumes(&RequestOptions::new()).await?;
/// let names: Vec<String> = volumes.into_iter().map(|v| v.name).collect();
/// println!("{names:?}");
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
#[non_exhaustive]
pub struct VolumeInfo {
    /// The volume ID.
    #[serde(rename = "volumeID")]
    pub volume_id: String,
    /// The volume name.
    pub name: String,
}

/// A volume mounted into a sandbox (`SandboxVolumeMount`,
/// `e2b/api/client/models/sandbox_volume_mount.py:10-19`), as sent at create
/// and reported in [`crate::SandboxInfo::volume_mounts`].
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
/// for mount in sandbox.get_info(&RequestOptions::new()).await?.volume_mounts {
///     println!("{} at {}", mount.name, mount.path);
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct VolumeMount {
    /// The volume name.
    pub name: String,
    /// Where it is mounted inside the sandbox.
    pub path: String,
}

impl VolumeMount {
    /// A mount of volume `name` at `path`.
    ///
    /// # Arguments
    ///
    /// * `name` - the volume name.
    /// * `path` - the mount path inside the sandbox.
    ///
    /// # Returns
    ///
    /// The mount.
    #[must_use]
    pub fn new(name: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            path: path.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use secrecy::ExposeSecret;

    use super::*;

    #[test]
    fn volume_reads_the_wire_shape_and_hides_the_token_from_debug() {
        // Arrange
        let wire: VolumeWire = serde_json::from_value(serde_json::json!({
            "volumeID": "vol-1", "name": "data", "token": "tok-secret", "domain": "byoc.test"
        }))
        .expect("wire");

        // Act
        let volume = Volume::from(wire);
        let debug = format!("{volume:?}");

        // Assert
        assert_eq!(volume.volume_id(), "vol-1");
        assert_eq!(volume.name(), "data");
        assert_eq!(volume.token().expose_secret(), "tok-secret");
        assert_eq!(volume.domain(), Some("byoc.test"));
        assert!(!debug.contains("tok-secret"), "{debug}");
        assert!(
            debug.contains("vol-1") && debug.contains("byoc.test"),
            "{debug}"
        );
        let copy = volume.clone();
        assert_eq!(copy.token().expose_secret(), "tok-secret");
        assert!(!format!("{copy:?}").contains("tok-secret"));
    }

    #[test]
    fn a_missing_null_or_empty_domain_reads_as_none() {
        for domain in [
            serde_json::json!({}),
            serde_json::json!({"domain": null}),
            serde_json::json!({"domain": ""}),
        ] {
            let mut body = serde_json::json!({"volumeID": "v", "name": "n", "token": "t"});
            if let (Some(body), serde_json::Value::Object(extra)) =
                (body.as_object_mut(), domain.clone())
            {
                body.extend(extra);
            }
            let wire: VolumeWire = serde_json::from_value(body).expect("wire");
            assert_eq!(Volume::from(wire).domain(), None, "{domain}");
        }
    }

    #[test]
    fn new_volume_body_is_just_the_name() {
        let body = serde_json::to_value(NewVolumeBody { name: "data" }).expect("ser");
        assert_eq!(body, serde_json::json!({"name": "data"}));
    }
}
