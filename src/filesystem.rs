//! Files in a sandbox: upload and download over envd's HTTP `/files` API, and
//! metadata operations over the `filesystem.Filesystem` Connect service.

use std::sync::Arc;

use bytes::Bytes;
use chrono::{DateTime, Utc};
use reqwest::Method;
use reqwest::multipart::{Form, Part};
use serde::Deserialize;

use crate::envd::connect::Empty;
use crate::envd::filesystem_pb::{
    self, EntryInfoWire, EntryResponse, ListDirRequest, ListDirResponse, MoveRequest, PathRequest,
};
use crate::envd::{EnvdClient, envd_http_error};
use crate::error::{ConnectCode, Error, Result};
use crate::wire::EnumValue;

/// envd's file transfer route (`e2b/envd/api.py:21`).
const FILES_ROUTE: &str = "/files";

/// What kind of filesystem entry something is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FileType {
    /// A regular file.
    File,
    /// A directory.
    Directory,
    /// A symbolic link.
    Symlink,
    /// A type this crate does not know, or none reported.
    Unknown,
}

impl FileType {
    /// Maps the proto enum (`filesystem.FileType`, by name or number) and the
    /// HTTP API's lowercase spelling (`"file"`, `"dir"`) to a [`FileType`].
    fn from_wire(value: Option<&EnumValue>) -> Self {
        match value {
            Some(EnumValue::Name(name)) => match name.as_str() {
                "FILE_TYPE_FILE" | "file" => Self::File,
                "FILE_TYPE_DIRECTORY" | "dir" | "directory" => Self::Directory,
                "FILE_TYPE_SYMLINK" | "symlink" => Self::Symlink,
                _ => Self::Unknown,
            },
            Some(EnumValue::Number(1)) => Self::File,
            Some(EnumValue::Number(2)) => Self::Directory,
            Some(EnumValue::Number(3)) => Self::Symlink,
            Some(EnumValue::Number(_)) | None => Self::Unknown,
        }
    }
}

/// Metadata about a file or directory (`filesystem.EntryInfo`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct EntryInfo {
    /// The base name.
    pub name: String,
    /// File, directory or symlink.
    pub file_type: FileType,
    /// The absolute path.
    pub path: String,
    /// Size in bytes.
    pub size: i64,
    /// Unix mode bits.
    pub mode: u32,
    /// Permissions in `ls -l` form, e.g. `-rw-r--r--`.
    pub permissions: String,
    /// Owning user.
    pub owner: String,
    /// Owning group.
    pub group: String,
    /// Last modification time, when reported and parseable.
    pub modified_time: Option<DateTime<Utc>>,
    /// The link target, for symlinks.
    pub symlink_target: Option<String>,
}

impl From<EntryInfoWire> for EntryInfo {
    fn from(wire: EntryInfoWire) -> Self {
        Self {
            file_type: FileType::from_wire(wire.file_type.as_ref()),
            modified_time: wire
                .modified_time
                .as_deref()
                .and_then(|raw| DateTime::parse_from_rfc3339(raw).ok())
                .map(|time| time.with_timezone(&Utc)),
            name: wire.name,
            path: wire.path,
            size: wire.size,
            mode: wire.mode,
            permissions: wire.permissions,
            owner: wire.owner,
            group: wire.group,
            symlink_target: wire.symlink_target,
        }
    }
}

/// What envd reports about an uploaded file.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct WriteInfo {
    /// The base name.
    pub name: String,
    /// The absolute path written.
    pub path: String,
    /// The entry type.
    pub file_type: FileType,
}

/// One element of the `/files` upload response (envd OpenAPI `EntryInfo`).
#[derive(Debug, Deserialize)]
struct UploadedEntry {
    #[serde(default)]
    name: String,
    #[serde(default)]
    path: String,
    #[serde(default, rename = "type")]
    file_type: Option<EnumValue>,
}

/// The filesystem service of one sandbox. Get it from
/// [`crate::Sandbox::files`].
///
/// Every method takes an optional `user`: the sandbox user to act as, which
/// also decides how relative paths resolve. `None` uses the sandbox default.
#[derive(Debug, Clone)]
pub struct Filesystem {
    envd: Arc<EnvdClient>,
}

impl Filesystem {
    pub(crate) fn new(envd: Arc<EnvdClient>) -> Self {
        Self { envd }
    }

    fn file_query<'a>(&self, path: &'a str, user: Option<&'a str>) -> Vec<(&'static str, &'a str)> {
        let mut query = Vec::with_capacity(2);
        query.push(("path", path));
        if let Some(user) = self.envd.effective_user(user) {
            query.push(("username", user));
        }
        query
    }

    /// Downloads a file (`GET /files?path=...`).
    ///
    /// # Arguments
    ///
    /// * `path` - the file to read.
    /// * `user` - the user to read as.
    ///
    /// # Returns
    ///
    /// The file's bytes.
    ///
    /// # Errors
    ///
    /// [`Error::Envd`] (404 when the file does not exist), or
    /// [`Error::Transport`].
    pub async fn read(&self, path: &str, user: Option<&str>) -> Result<Bytes> {
        let response = self
            .envd
            .http_request(Method::GET, FILES_ROUTE)
            .query(&self.file_query(path, user))
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(envd_http_error(response).await);
        }
        Ok(response.bytes().await?)
    }

    /// Downloads a file and decodes it as UTF-8, replacing invalid sequences.
    ///
    /// # Arguments
    ///
    /// * `path` - the file to read.
    /// * `user` - the user to read as.
    ///
    /// # Returns
    ///
    /// The file's text.
    ///
    /// # Errors
    ///
    /// As [`Filesystem::read`].
    pub async fn read_to_string(&self, path: &str, user: Option<&str>) -> Result<String> {
        let bytes = self.read(path, user).await?;
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }

    /// Uploads a file, creating parent directories and overwriting any
    /// existing file (`POST /files?path=...`, multipart field `file`, the SDKs'
    /// default upload encoding: JS `dist/index.js:2226-2249`).
    ///
    /// # Arguments
    ///
    /// * `path` - where to write.
    /// * `data` - the content (`&'static str`, `String`, `Vec<u8>`, `Bytes`...).
    /// * `user` - the user to write as (and who will own the file).
    ///
    /// # Returns
    ///
    /// What envd reports about the written file.
    ///
    /// # Errors
    ///
    /// [`Error::Envd`] for a non-2xx answer, [`Error::Protocol`] if envd
    /// reports no written file, [`Error::Decode`] or [`Error::Transport`].
    pub async fn write(
        &self,
        path: &str,
        data: impl Into<Bytes>,
        user: Option<&str>,
    ) -> Result<WriteInfo> {
        let data: Bytes = data.into();
        // `u64::try_from` cannot fail for an in-memory buffer on 64-bit targets.
        let length = u64::try_from(data.len()).unwrap_or(u64::MAX);
        let part = Part::stream_with_length(data, length).file_name(path.to_owned());
        let form = Form::new().part("file", part);
        let response = self
            .envd
            .http_request(Method::POST, FILES_ROUTE)
            .query(&self.file_query(path, user))
            .multipart(form)
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(envd_http_error(response).await);
        }
        let body = response.bytes().await?;
        let entries: Vec<UploadedEntry> = serde_json::from_slice(&body)?;
        let entry = entries
            .into_iter()
            .next()
            .ok_or_else(|| Error::Protocol("envd reported no written file".into()))?;
        Ok(WriteInfo {
            file_type: FileType::from_wire(entry.file_type.as_ref()),
            name: entry.name,
            path: entry.path,
        })
    }

    /// Lists a directory (`ListDir`).
    ///
    /// # Arguments
    ///
    /// * `path` - the directory.
    /// * `depth` - how many levels to descend; 1 lists only direct children.
    /// * `user` - the user to act as.
    ///
    /// # Returns
    ///
    /// The entries.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidArgument`] if `depth` is 0
    /// (`e2b/sandbox_async/filesystem/filesystem.py:508-509`), [`Error::Rpc`]
    /// (`not_found` for a missing directory), or [`Error::Transport`].
    pub async fn list(&self, path: &str, depth: u32, user: Option<&str>) -> Result<Vec<EntryInfo>> {
        if depth == 0 {
            return Err(Error::InvalidArgument("depth should be at least 1".into()));
        }
        let response: ListDirResponse = self
            .envd
            .unary(
                filesystem_pb::SERVICE,
                "ListDir",
                &ListDirRequest { path, depth },
                self.envd.user_headers(user)?,
            )
            .await?;
        Ok(response.entries.into_iter().map(EntryInfo::from).collect())
    }

    /// Gets metadata about a path (`Stat`).
    ///
    /// # Arguments
    ///
    /// * `path` - the path.
    /// * `user` - the user to act as.
    ///
    /// # Returns
    ///
    /// The entry.
    ///
    /// # Errors
    ///
    /// [`Error::Rpc`] (`not_found` for a missing path), [`Error::Protocol`]
    /// if envd returns no entry, or [`Error::Transport`].
    pub async fn get_info(&self, path: &str, user: Option<&str>) -> Result<EntryInfo> {
        let response: EntryResponse = self
            .envd
            .unary(
                filesystem_pb::SERVICE,
                "Stat",
                &PathRequest { path },
                self.envd.user_headers(user)?,
            )
            .await?;
        entry_of(response)
    }

    /// Whether a path exists.
    ///
    /// # Arguments
    ///
    /// * `path` - the path.
    /// * `user` - the user to act as.
    ///
    /// # Returns
    ///
    /// `true` if it exists.
    ///
    /// # Errors
    ///
    /// Errors of [`Filesystem::get_info`] other than `not_found`.
    pub async fn exists(&self, path: &str, user: Option<&str>) -> Result<bool> {
        match self.get_info(path, user).await {
            Ok(_) => Ok(true),
            Err(Error::Rpc {
                code: ConnectCode::NotFound,
                ..
            }) => Ok(false),
            Err(err) => Err(err),
        }
    }

    /// Creates a directory and any missing parents (`MakeDir`).
    ///
    /// # Arguments
    ///
    /// * `path` - the directory to create.
    /// * `user` - the user to act as.
    ///
    /// # Returns
    ///
    /// `true` if it was created, `false` if it already existed
    /// (`e2b/sandbox_async/filesystem/filesystem.py:663-676`).
    ///
    /// # Errors
    ///
    /// [`Error::Rpc`] other than `already_exists`, or [`Error::Transport`].
    pub async fn make_dir(&self, path: &str, user: Option<&str>) -> Result<bool> {
        let result: Result<EntryResponse> = self
            .envd
            .unary(
                filesystem_pb::SERVICE,
                "MakeDir",
                &PathRequest { path },
                self.envd.user_headers(user)?,
            )
            .await;
        match result {
            Ok(_) => Ok(true),
            Err(Error::Rpc {
                code: ConnectCode::AlreadyExists,
                ..
            }) => Ok(false),
            Err(err) => Err(err),
        }
    }

    /// Moves or renames a file or directory (`Move`).
    ///
    /// # Arguments
    ///
    /// * `source` - the current path.
    /// * `destination` - the new path.
    /// * `user` - the user to act as.
    ///
    /// # Returns
    ///
    /// The entry at its new path.
    ///
    /// # Errors
    ///
    /// [`Error::Rpc`] (`not_found` for a missing source), [`Error::Protocol`]
    /// if envd returns no entry, or [`Error::Transport`].
    pub async fn rename(
        &self,
        source: &str,
        destination: &str,
        user: Option<&str>,
    ) -> Result<EntryInfo> {
        let response: EntryResponse = self
            .envd
            .unary(
                filesystem_pb::SERVICE,
                "Move",
                &MoveRequest {
                    source,
                    destination,
                },
                self.envd.user_headers(user)?,
            )
            .await?;
        entry_of(response)
    }

    /// Removes a file or directory (recursively) (`Remove`).
    ///
    /// # Arguments
    ///
    /// * `path` - the path to remove.
    /// * `user` - the user to act as.
    ///
    /// # Errors
    ///
    /// [`Error::Rpc`] (`not_found` for a missing path) or [`Error::Transport`].
    pub async fn remove(&self, path: &str, user: Option<&str>) -> Result<()> {
        let _: Empty = self
            .envd
            .unary(
                filesystem_pb::SERVICE,
                "Remove",
                &PathRequest { path },
                self.envd.user_headers(user)?,
            )
            .await?;
        Ok(())
    }
}

fn entry_of(response: EntryResponse) -> Result<EntryInfo> {
    response
        .entry
        .map(EntryInfo::from)
        .ok_or_else(|| Error::Protocol("envd returned no entry".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_type_mapping() {
        let name = |raw: &str| FileType::from_wire(Some(&EnumValue::Name(raw.into())));
        assert_eq!(name("FILE_TYPE_FILE"), FileType::File);
        assert_eq!(name("dir"), FileType::Directory);
        assert_eq!(name("FILE_TYPE_SYMLINK"), FileType::Symlink);
        assert_eq!(name("FILE_TYPE_UNSPECIFIED"), FileType::Unknown);
        assert_eq!(
            FileType::from_wire(Some(&EnumValue::Number(2))),
            FileType::Directory
        );
        assert_eq!(FileType::from_wire(None), FileType::Unknown);
    }

    #[test]
    fn entry_info_conversion_parses_time() {
        let wire = EntryInfoWire {
            name: "a".into(),
            modified_time: Some("2026-01-02T03:04:05.5+01:00".into()),
            ..EntryInfoWire::default()
        };
        let entry = EntryInfo::from(wire);
        let time = entry.modified_time.expect("time");
        assert_eq!(time.to_rfc3339(), "2026-01-02T02:04:05.500+00:00");
    }
}
