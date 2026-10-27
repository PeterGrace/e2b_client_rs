//! Files in a sandbox: upload and download over envd's HTTP `/files` API,
//! metadata operations and directory watching over the
//! `filesystem.Filesystem` Connect service.

use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use chrono::{DateTime, Utc};
use reqwest::Method;
use reqwest::multipart::{Form, Part};
use serde::Deserialize;

use crate::envd::connect::{ConnectStream, Empty};
use crate::envd::filesystem_pb::{
    self, EntryInfoWire, EntryResponse, ListDirRequest, ListDirResponse, MoveRequest, PathRequest,
    WatchDirRequest, WatchDirResponse,
};
use crate::envd::{EnvdClient, EnvdVersion, envd_http_error, keepalive_headers};
use crate::error::{ConnectCode, Error, Result};
use crate::wire::EnumValue;

/// envd's file transfer route (`e2b/envd/api.py:21`).
const FILES_ROUTE: &str = "/files";

/// The SDKs' default bound on a directory watch (`timeout=60`,
/// `e2b/sandbox_async/filesystem/filesystem.py:685`).
pub const DEFAULT_WATCH_TIMEOUT: Duration = Duration::from_secs(60);

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

/// What happened to a watched entry (`filesystem.EventType`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FilesystemEventType {
    /// The entry was created.
    Create,
    /// The entry was written to.
    Write,
    /// The entry was removed.
    Remove,
    /// The entry was renamed.
    Rename,
    /// The entry's permissions changed.
    Chmod,
}

impl FilesystemEventType {
    /// Maps the proto enum, by name or number, to an event type.
    ///
    /// # Arguments
    ///
    /// * `value` - the `type` field of a `filesystem.FilesystemEvent`.
    ///
    /// # Returns
    ///
    /// The event type, or `None` for `EVENT_TYPE_UNSPECIFIED`, a missing
    /// type or one this crate does not know: the SDKs drop such events
    /// (`e2b/sandbox/filesystem/watch_handle.py:36-46`).
    fn from_wire(value: Option<&EnumValue>) -> Option<Self> {
        // `value?` returns `None` from the function when there is no type,
        // like an early `if value is None: return None` in Python.
        match value? {
            EnumValue::Name(name) => match name.as_str() {
                "EVENT_TYPE_CREATE" => Some(Self::Create),
                "EVENT_TYPE_WRITE" => Some(Self::Write),
                "EVENT_TYPE_REMOVE" => Some(Self::Remove),
                "EVENT_TYPE_RENAME" => Some(Self::Rename),
                "EVENT_TYPE_CHMOD" => Some(Self::Chmod),
                _ => None,
            },
            EnumValue::Number(1) => Some(Self::Create),
            EnumValue::Number(2) => Some(Self::Write),
            EnumValue::Number(3) => Some(Self::Remove),
            EnumValue::Number(4) => Some(Self::Rename),
            EnumValue::Number(5) => Some(Self::Chmod),
            EnumValue::Number(_) => None,
        }
    }
}

/// One change in a watched directory (`filesystem.FilesystemEvent`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct FilesystemEvent {
    /// The entry's path relative to the watched directory.
    pub name: String,
    /// What happened.
    pub event_type: FilesystemEventType,
    /// The entry's info, when the watch asked for it with
    /// [`WatchOptions::include_entry`] and envd had it. Often `None` for
    /// removals and renames away, where the entry no longer exists.
    pub entry: Option<EntryInfo>,
}

/// Options for [`Filesystem::watch_dir`].
///
/// # Examples
///
/// ```
/// use std::time::Duration;
/// use e2b_client::WatchOptions;
///
/// let options = WatchOptions::default()
///     .recursive(true)
///     .include_entry(true)
///     .user("root")
///     .timeout(Some(Duration::from_secs(300)));
/// # let _ = options;
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatchOptions {
    recursive: bool,
    include_entry: bool,
    allow_network_mounts: bool,
    user: Option<String>,
    timeout: Option<Duration>,
}

impl Default for WatchOptions {
    fn default() -> Self {
        Self {
            recursive: false,
            include_entry: false,
            allow_network_mounts: false,
            user: None,
            timeout: Some(DEFAULT_WATCH_TIMEOUT),
        }
    }
}

impl WatchOptions {
    /// Also watches subdirectories. Needs envd 0.1.4 or newer.
    ///
    /// # Arguments
    ///
    /// * `recursive` - whether to watch the whole tree.
    ///
    /// # Returns
    ///
    /// The options, for chaining.
    #[must_use]
    pub fn recursive(mut self, recursive: bool) -> Self {
        self.recursive = recursive;
        self
    }

    /// Asks envd to attach the entry's info to each event, when it has it.
    /// Needs envd 0.6.3 or newer.
    ///
    /// # Arguments
    ///
    /// * `include_entry` - whether events carry [`FilesystemEvent::entry`].
    ///
    /// # Returns
    ///
    /// The options, for chaining.
    #[must_use]
    pub fn include_entry(mut self, include_entry: bool) -> Self {
        self.include_entry = include_entry;
        self
    }

    /// Allows watching a directory on a network mount (NFS, CIFS, SMB,
    /// FUSE), which envd refuses by default. Events there may be unreliable
    /// or missing. Needs envd 0.6.4 or newer.
    ///
    /// # Arguments
    ///
    /// * `allow_network_mounts` - whether network mounts may be watched.
    ///
    /// # Returns
    ///
    /// The options, for chaining.
    #[must_use]
    pub fn allow_network_mounts(mut self, allow_network_mounts: bool) -> Self {
        self.allow_network_mounts = allow_network_mounts;
        self
    }

    /// Watches as `user` instead of the sandbox's default user.
    ///
    /// # Arguments
    ///
    /// * `user` - the user name, e.g. `"root"`.
    ///
    /// # Returns
    ///
    /// The options, for chaining.
    #[must_use]
    pub fn user(mut self, user: impl Into<String>) -> Self {
        self.user = Some(user.into());
        self
    }

    /// Bounds the whole watch; `None` for no limit. Defaults to
    /// [`DEFAULT_WATCH_TIMEOUT`] (60 s), as in the SDKs. The deadline counts
    /// from when the watch opens, not from the last event.
    ///
    /// # Arguments
    ///
    /// * `timeout` - the stream deadline.
    ///
    /// # Returns
    ///
    /// The options, for chaining.
    #[must_use]
    pub fn timeout(mut self, timeout: Option<Duration>) -> Self {
        self.timeout = timeout;
        self
    }

    /// Refuses an option the sandbox's envd is too old for, as the SDKs do
    /// before sending anything
    /// (`e2b/sandbox_async/filesystem/filesystem.py:705-721`).
    ///
    /// # Arguments
    ///
    /// * `version` - the sandbox's envd version.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidArgument`] naming the first option that needs a newer
    /// envd.
    fn check_supported(&self, version: EnvdVersion) -> Result<()> {
        let gates = [
            (
                self.recursive,
                EnvdVersion::RECURSIVE_WATCH,
                "recursive watching",
            ),
            (
                self.include_entry,
                EnvdVersion::FS_EVENT_ENTRY_INFO,
                "entry info on watch events",
            ),
            (
                self.allow_network_mounts,
                EnvdVersion::WATCH_NETWORK_MOUNTS,
                "watching network mounts",
            ),
        ];
        // `find` stops at the first gate that fails, like Python's
        // `next((g for g in gates if ...), None)`.
        match gates
            .into_iter()
            .find(|&(wanted, needs, _)| wanted && version < needs)
        {
            Some((_, needs, what)) => Err(Error::InvalidArgument(format!(
                "{what} needs envd {needs} or newer; this sandbox runs envd {version}"
            ))),
            None => Ok(()),
        }
    }
}

/// An open directory watch. Pull events with
/// [`next_event`](Self::next_event); drop the handle to stop watching.
///
/// Python note: there are no `on_event`/`on_exit` callbacks. Loop with
/// `while let Some(event) = watch.next_event().await? { ... }`, and dropping
/// the handle (letting it go out of scope) is the SDK's `stop()`: it closes
/// the HTTP stream, which ends the watch in envd.
#[derive(Debug)]
pub struct WatchHandle {
    stream: ConnectStream<WatchDirResponse>,
}

impl WatchHandle {
    /// Waits for the next filesystem event. Keepalives, unspecified event
    /// types and types this crate does not know are skipped.
    ///
    /// # Returns
    ///
    /// `Some(event)` per change, `None` once envd ends the watch cleanly.
    ///
    /// # Errors
    ///
    /// [`Error::Timeout`] when the watch timeout passes, [`Error::Rpc`] if
    /// envd ends the watch with an error, [`Error::Protocol`],
    /// [`Error::Decode`] or [`Error::Transport`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn demo(sandbox: e2b_client::Sandbox) -> e2b_client::Result<()> {
    /// use e2b_client::{FilesystemEventType, WatchOptions};
    ///
    /// let mut watch = sandbox.files().watch_dir("/tmp", &WatchOptions::default()).await?;
    /// let mut created = Vec::new();
    /// while let Some(event) = watch.next_event().await? {
    ///     if event.event_type == FilesystemEventType::Create {
    ///         created.push(event.name);
    ///     }
    /// }
    /// # let _ = created;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn next_event(&mut self) -> Result<Option<FilesystemEvent>> {
        while let Some(response) = self.stream.next().await? {
            // `let ... else { continue }` skips responses that are not a
            // filesystem event (keepalives) or carry an unknown type.
            let Some(event) = response.filesystem else {
                continue;
            };
            let Some(event_type) = FilesystemEventType::from_wire(event.event_type.as_ref()) else {
                continue;
            };
            return Ok(Some(FilesystemEvent {
                name: event.name,
                event_type,
                entry: event.entry.map(EntryInfo::from),
            }));
        }
        Ok(None)
    }
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

    /// Watches a directory for changes (`WatchDir`).
    ///
    /// The watch is open once envd confirms it with a start event, awaited
    /// within the request timeout. It then lasts until the handle is
    /// dropped or the watch timeout of `options` passes. Version gates are
    /// checked first, so an option the sandbox's envd is too old for sends
    /// nothing.
    ///
    /// # Arguments
    ///
    /// * `path` - the directory to watch.
    /// * `options` - recursion, entry info, network mounts, user and timeout.
    ///
    /// # Returns
    ///
    /// A [`WatchHandle`] to pull events from.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidArgument`] when an option needs a newer envd (nothing
    /// is sent), [`Error::Timeout`] if the watch does not open within the
    /// request timeout, [`Error::Protocol`] if it does not open with a start
    /// event, [`Error::Rpc`] (`not_found` for a missing directory) or
    /// [`Error::Transport`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn demo(sandbox: e2b_client::Sandbox) -> e2b_client::Result<()> {
    /// use e2b_client::WatchOptions;
    ///
    /// let files = sandbox.files();
    /// let mut watch = files.watch_dir("/tmp", &WatchOptions::default().recursive(true)).await?;
    /// files.write("/tmp/new.txt", "x", None).await?;
    /// let first = watch.next_event().await?;
    /// # let _ = first;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn watch_dir(&self, path: &str, options: &WatchOptions) -> Result<WatchHandle> {
        options.check_supported(self.envd.version())?;
        let request = WatchDirRequest {
            path,
            recursive: options.recursive,
            include_entry: options.include_entry,
            allow_network_mounts: options.allow_network_mounts,
        };
        let mut headers = self.envd.user_headers(options.user.as_deref())?;
        headers.extend(keepalive_headers());
        let stream = self
            .envd
            .within_request_timeout(async {
                let mut stream: ConnectStream<WatchDirResponse> = self
                    .envd
                    .server_stream(
                        filesystem_pb::SERVICE,
                        "WatchDir",
                        &request,
                        headers,
                        options.timeout,
                    )
                    .await?;
                match stream.next().await? {
                    Some(WatchDirResponse { start: Some(_), .. }) => Ok(stream),
                    _ => Err(Error::Protocol(
                        "WatchDir stream did not open with a start event".into(),
                    )),
                }
            })
            .await?;
        Ok(WatchHandle { stream })
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
    fn event_type_mapping_drops_unspecified_and_unknown() {
        let name = |raw: &str| FilesystemEventType::from_wire(Some(&EnumValue::Name(raw.into())));
        let number = |raw: i64| FilesystemEventType::from_wire(Some(&EnumValue::Number(raw)));
        let all = [
            FilesystemEventType::Create,
            FilesystemEventType::Write,
            FilesystemEventType::Remove,
            FilesystemEventType::Rename,
            FilesystemEventType::Chmod,
        ];
        let names = [
            "EVENT_TYPE_CREATE",
            "EVENT_TYPE_WRITE",
            "EVENT_TYPE_REMOVE",
            "EVENT_TYPE_RENAME",
            "EVENT_TYPE_CHMOD",
        ];
        // `zip` pairs the two arrays element by element, like Python's `zip`;
        // the proto numbers are 1 to 5 in the same order.
        for ((expected, raw), wire_number) in all.into_iter().zip(names).zip(1..) {
            assert_eq!(name(raw), Some(expected), "{raw}");
            assert_eq!(number(wire_number), Some(expected), "{wire_number}");
        }
        assert_eq!(name("EVENT_TYPE_UNSPECIFIED"), None);
        assert_eq!(name("EVENT_TYPE_SOMETHING_NEW"), None);
        assert_eq!(number(0), None);
        assert_eq!(number(6), None);
        assert_eq!(FilesystemEventType::from_wire(None), None);
    }

    #[test]
    fn watch_options_default_and_version_gates() {
        let defaults = WatchOptions::default();
        assert_eq!(defaults.timeout, Some(DEFAULT_WATCH_TIMEOUT));
        assert!(!defaults.recursive && !defaults.include_entry && !defaults.allow_network_mounts);
        let oldest = EnvdVersion::new(0, 0, 1);
        assert!(defaults.check_supported(oldest).is_ok());

        let all = WatchOptions::default()
            .recursive(true)
            .include_entry(true)
            .allow_network_mounts(true);
        assert!(
            all.check_supported(EnvdVersion::WATCH_NETWORK_MOUNTS)
                .is_ok()
        );
        let err = all
            .check_supported(EnvdVersion::new(0, 1, 3))
            .expect_err("recursive on 0.1.3");
        assert!(
            matches!(&err, Error::InvalidArgument(message) if message.contains("recursive")),
            "{err:?}"
        );
        let err = all
            .check_supported(EnvdVersion::new(0, 6, 3))
            .expect_err("network mounts on 0.6.3");
        assert!(
            matches!(&err, Error::InvalidArgument(message) if message.contains("0.6.4")),
            "{err:?}"
        );
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
