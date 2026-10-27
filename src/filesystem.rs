//! Files in a sandbox: upload and download over envd's HTTP `/files` API
//! (in memory or streamed), metadata operations and directory watching over the
//! `filesystem.Filesystem` Connect service.

use std::sync::Arc;
use std::time::Duration;

use async_compression::tokio::bufread::GzipEncoder;
use bytes::Bytes;
use chrono::{DateTime, Utc};
use reqwest::Method;
use reqwest::header::{CONTENT_ENCODING, CONTENT_TYPE, HeaderValue};
use reqwest::multipart::{Form, Part};
use serde::Deserialize;
use tokio::io::{AsyncBufRead, AsyncRead, BufReader};
use tokio_util::io::ReaderStream;

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

impl From<UploadedEntry> for WriteInfo {
    fn from(entry: UploadedEntry) -> Self {
        Self {
            file_type: FileType::from_wire(entry.file_type.as_ref()),
            name: entry.name,
            path: entry.path,
        }
    }
}

/// One file for [`Filesystem::write_files`]: where to write, and what.
///
/// # Examples
///
/// ```
/// use e2b_client::WriteEntry;
///
/// let entries = [
///     WriteEntry::new("/home/user/a.txt", "alpha"),
///     WriteEntry::new("/home/user/b.bin", vec![0_u8, 1, 2]),
/// ];
/// # let _ = entries;
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteEntry {
    path: String,
    data: Bytes,
}

impl WriteEntry {
    /// Creates an entry.
    ///
    /// # Arguments
    ///
    /// * `path` - where to write.
    /// * `data` - the content (`&'static str`, `String`, `Vec<u8>`, `Bytes`...).
    ///
    /// # Returns
    ///
    /// The entry.
    #[must_use]
    pub fn new(path: impl Into<String>, data: impl Into<Bytes>) -> Self {
        Self {
            path: path.into(),
            data: data.into(),
        }
    }
}

/// How [`Filesystem::write_files`] and [`Filesystem::write_stream`] upload:
/// as which user, and with which encoding.
///
/// By default [`Filesystem::write_files`] uploads are `multipart/form-data`,
/// the SDKs' default for in-memory data, with all files in one request.
/// Octet-stream and gzip send one request per file, and need envd 0.5.7 or
/// newer: on older envd they fall back to uncompressed multipart, as the SDKs
/// do. [`Filesystem::write_stream`] always uses octet-stream where envd
/// supports it, so only `user` and `gzip` matter there.
///
/// # Examples
///
/// ```
/// use e2b_client::WriteOptions;
///
/// // Gzip-compressed octet-stream uploads, written as root.
/// let options = WriteOptions::default().gzip(true).user("root");
/// # let _ = options;
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WriteOptions {
    user: Option<String>,
    octet_stream: bool,
    gzip: bool,
}

impl WriteOptions {
    /// Writes as `user` (who will own the files) instead of the sandbox's
    /// default user.
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

    /// Uploads each file as its own `application/octet-stream` request
    /// instead of one multipart request. Needs envd 0.5.7 or newer; older
    /// envd gets multipart.
    ///
    /// # Arguments
    ///
    /// * `octet_stream` - whether to use octet-stream uploads.
    ///
    /// # Returns
    ///
    /// The options, for chaining.
    #[must_use]
    pub fn octet_stream(mut self, octet_stream: bool) -> Self {
        self.octet_stream = octet_stream;
        self
    }

    /// Gzip-compresses each file's body and sends `Content-Encoding: gzip`.
    /// Implies octet-stream, because the encoding applies to the whole
    /// request body. Needs envd 0.5.7 or newer; older envd gets
    /// *uncompressed* multipart, as in the SDKs.
    ///
    /// # Arguments
    ///
    /// * `gzip` - whether to compress uploads.
    ///
    /// # Returns
    ///
    /// The options, for chaining.
    #[must_use]
    pub fn gzip(mut self, gzip: bool) -> Self {
        self.gzip = gzip;
        self
    }
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
    /// Python note: the SDK's `timeout=0` means no limit, but here only
    /// `None` does. `Some(Duration::ZERO)` is a deadline that has already
    /// passed (sent as `Connect-Timeout-Ms: 1`), so the watch ends at once.
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

/// A file being downloaded, from [`Filesystem::read_stream`].
///
/// Python note: this is the SDK's `read(format="stream")` iterator. Pull
/// chunks with `while let Some(chunk) = stream.next_chunk().await? { ... }`;
/// dropping the stream (letting it go out of scope) closes the connection.
#[derive(Debug)]
pub struct FileStream {
    response: reqwest::Response,
}

impl FileStream {
    /// Waits for the next chunk of the file. Chunk sizes are whatever the
    /// connection delivers; only their concatenation is meaningful.
    ///
    /// # Returns
    ///
    /// `Some(chunk)` until the file ends, then `None`.
    ///
    /// # Errors
    ///
    /// [`Error::Transport`] if the connection fails mid-file.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn demo(sandbox: e2b_client::Sandbox) -> e2b_client::Result<()> {
    /// let mut stream = sandbox.files().read_stream("/home/user/big.log", None).await?;
    /// let mut total = 0;
    /// while let Some(chunk) = stream.next_chunk().await? {
    ///     total += chunk.len();
    /// }
    /// # let _ = total;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn next_chunk(&mut self) -> Result<Option<Bytes>> {
        Ok(self.response.chunk().await?)
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

    /// The `/files` query for a read: `path`, plus `username` when a user
    /// applies.
    fn file_query<'a>(&self, path: &'a str, user: Option<&'a str>) -> Vec<(&'static str, &'a str)> {
        self.upload_query(Some(path), user)
    }

    /// The `/files` query for an upload: `path` when given (a multipart
    /// upload of several files has none), plus `username` when a user
    /// applies.
    ///
    /// # Arguments
    ///
    /// * `path` - the target path, if the request names one.
    /// * `user` - the caller's user, resolved with `effective_user`.
    ///
    /// # Returns
    ///
    /// The query pairs, borrowed from the arguments.
    fn upload_query<'a>(
        &self,
        path: Option<&'a str>,
        user: Option<&'a str>,
    ) -> Vec<(&'static str, &'a str)> {
        let mut query = Vec::with_capacity(2);
        // `extend` with an `Option` adds its value when it is `Some`, like
        // appending only when the value is not `None` in Python.
        query.extend(path.map(|path| ("path", path)));
        query.extend(
            self.envd
                .effective_user(user)
                .map(|user| ("username", user)),
        );
        query
    }

    /// Whether an upload with `options` goes as octet-stream: asked for
    /// (gzip implies it) and supported by this sandbox's envd
    /// (`e2b/sandbox_async/filesystem/filesystem.py:369-384`).
    fn uses_octet_stream(&self, options: &WriteOptions) -> bool {
        (options.octet_stream || options.gzip)
            && self.envd.version() >= EnvdVersion::OCTET_STREAM_UPLOAD
    }

    /// Uploads one file as an `application/octet-stream` body
    /// (`POST /files?path=<p>`), with `Content-Encoding: gzip` when
    /// `options.gzip` is set (the caller compresses `body`).
    ///
    /// # Arguments
    ///
    /// * `path` - where to write.
    /// * `body` - the request body.
    /// * `options` - user and gzip.
    /// * `timeout` - the whole-request deadline, or `None` for none.
    ///
    /// # Returns
    ///
    /// What envd reports about the written file.
    ///
    /// # Errors
    ///
    /// As [`Filesystem::write`].
    async fn post_octet_stream(
        &self,
        path: &str,
        body: reqwest::Body,
        options: &WriteOptions,
        timeout: Option<Duration>,
    ) -> Result<Vec<WriteInfo>> {
        let mut request = self
            .envd
            .http_request_with_timeout(Method::POST, FILES_ROUTE, timeout)
            .query(&self.upload_query(Some(path), options.user.as_deref()))
            .header(
                CONTENT_TYPE,
                HeaderValue::from_static("application/octet-stream"),
            );
        if options.gzip {
            request = request.header(CONTENT_ENCODING, HeaderValue::from_static("gzip"));
        }
        written_entries(request.body(body).send().await?).await
    }

    /// Uploads a multipart form of `file` parts (`POST /files`).
    ///
    /// # Arguments
    ///
    /// * `form` - the parts, each named `file` with the target path as its
    ///   filename.
    /// * `path` - the `path` query, sent only for a single-file upload.
    /// * `options` - the user.
    /// * `timeout` - the whole-request deadline, or `None` for none.
    ///
    /// # Returns
    ///
    /// What envd reports about each written file.
    ///
    /// # Errors
    ///
    /// As [`Filesystem::write`].
    async fn post_multipart(
        &self,
        form: Form,
        path: Option<&str>,
        options: &WriteOptions,
        timeout: Option<Duration>,
    ) -> Result<Vec<WriteInfo>> {
        let response = self
            .envd
            .http_request_with_timeout(Method::POST, FILES_ROUTE, timeout)
            .query(&self.upload_query(path, options.user.as_deref()))
            .multipart(form)
            .send()
            .await?;
        written_entries(response).await
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

    /// Downloads a file as a stream of chunks (`GET /files?path=...`),
    /// without holding the whole file in memory.
    ///
    /// No timeout applies at all, including the wait for envd's response
    /// headers, because a deadline would cut off any download that outlasts
    /// it; this is the SDK's default. A server that accepts the connection
    /// and never answers therefore hangs this call. Bound it yourself with
    /// `tokio::time::timeout` around this call and [`FileStream::next_chunk`]
    /// if needed.
    ///
    /// # Arguments
    ///
    /// * `path` - the file to read.
    /// * `user` - the user to read as.
    ///
    /// # Returns
    ///
    /// A [`FileStream`] positioned at the start of the file. envd's status
    /// is checked here, so a missing file fails this call, not the first
    /// chunk.
    ///
    /// # Errors
    ///
    /// [`Error::Envd`] (404 when the file does not exist), or
    /// [`Error::Transport`].
    pub async fn read_stream(&self, path: &str, user: Option<&str>) -> Result<FileStream> {
        let response = self
            .envd
            .http_request_with_timeout(Method::GET, FILES_ROUTE, None)
            .query(&self.file_query(path, user))
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(envd_http_error(response).await);
        }
        Ok(FileStream { response })
    }

    /// Uploads a file, creating parent directories and overwriting any
    /// existing file (`POST /files?path=...`, multipart field `file`, the SDKs'
    /// default upload encoding: JS `dist/index.js:2226-2249`). It is a
    /// one-entry [`Filesystem::write_files`] with default options.
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
        let options = WriteOptions {
            user: user.map(str::to_owned),
            ..WriteOptions::default()
        };
        self.write_files(&[WriteEntry::new(path, data)], &options)
            .await?
            .into_iter()
            .next()
            .ok_or_else(no_written_file)
    }

    /// Writes several files (`POST /files`), creating parent directories and
    /// overwriting existing files.
    ///
    /// Multipart (the default) sends all files in one request, with the
    /// `path` query only when there is exactly one file. Octet-stream and
    /// gzip send one request per file, concurrently; on envd older than
    /// 0.5.7 they fall back to uncompressed multipart. Every request is
    /// bounded by the client's request timeout.
    ///
    /// The octet-stream requests all start at once, with no limit, as the
    /// SDK's `asyncio.gather` does: a batch of N files opens N requests to
    /// envd. For a very large batch, call `write_files` on chunks of
    /// `entries` (`entries.chunks(64)`) to bound it.
    ///
    /// # Arguments
    ///
    /// * `entries` - the files.
    /// * `options` - user and encoding.
    ///
    /// # Returns
    ///
    /// What envd reports about each written file, in envd's order (for
    /// octet-stream, the order of `entries`). An empty `entries` returns an
    /// empty list without a request.
    ///
    /// # Errors
    ///
    /// As [`Filesystem::write`]. With octet-stream, the first failing
    /// upload's error is returned, and the other uploads are dropped
    /// (cancelled) where still running.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use e2b_client::{Sandbox, WriteEntry, WriteOptions};
    ///
    /// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
    /// let entries = [
    ///     WriteEntry::new("/home/user/a.txt", "alpha"),
    ///     WriteEntry::new("/home/user/sub/b.txt", "beta"),
    /// ];
    /// let written = sandbox.files().write_files(&entries, &WriteOptions::default()).await?;
    /// assert_eq!(written.len(), 2);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn write_files(
        &self,
        entries: &[WriteEntry],
        options: &WriteOptions,
    ) -> Result<Vec<WriteInfo>> {
        if entries.is_empty() {
            return Ok(Vec::new());
        }
        let timeout = self.envd.request_timeout();
        if self.uses_octet_stream(options) {
            // `Bytes::clone` is a reference-count bump, not a copy of the data.
            // `map` builds the upload futures lazily; none runs until
            // `try_join_all` polls them all together, like `asyncio.gather`.
            let uploads = entries.iter().map(|entry| {
                let body = if options.gzip {
                    gzip_body(std::io::Cursor::new(entry.data.clone()))
                } else {
                    reqwest::Body::from(entry.data.clone())
                };
                self.post_octet_stream(&entry.path, body, options, timeout)
            });
            let written = futures_util::future::try_join_all(uploads).await?;
            return Ok(written.into_iter().flatten().collect());
        }
        let form = entries.iter().fold(Form::new(), |form, entry| {
            // `u64::try_from` cannot fail for an in-memory buffer on 64-bit targets.
            let length = u64::try_from(entry.data.len()).unwrap_or(u64::MAX);
            let part =
                Part::stream_with_length(entry.data.clone(), length).file_name(entry.path.clone());
            form.part("file", part)
        });
        // A slice pattern: `[only]` matches a slice of exactly one element.
        let single_path = match entries {
            [only] => Some(only.path.as_str()),
            _ => None,
        };
        self.post_multipart(form, single_path, options, timeout)
            .await
    }

    /// Uploads a file from a reader without loading it into memory
    /// (`POST /files?path=...`, `application/octet-stream`), creating parent
    /// directories and overwriting any existing file.
    ///
    /// No request timeout applies, because a deadline would cut off any
    /// transfer that outlasts it; this matches the SDKs. On envd older than
    /// 0.5.7 the upload falls back to a streamed multipart body, uncompressed,
    /// as in the SDKs.
    ///
    /// # Arguments
    ///
    /// * `path` - where to write.
    /// * `reader` - the content, read to its end (a socket, a byte slice, a
    ///   `tokio::fs::File`...). It must be `Send + 'static` because reqwest
    ///   may poll the body on another task, so it cannot borrow local data.
    /// * `options` - user and gzip (`octet_stream` is implied).
    ///
    /// # Returns
    ///
    /// What envd reports about the written file.
    ///
    /// # Errors
    ///
    /// As [`Filesystem::write`]; a read error from `reader` surfaces as
    /// [`Error::Transport`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use e2b_client::{Sandbox, WriteOptions};
    ///
    /// # async fn demo(sandbox: Sandbox) -> e2b_client::Result<()> {
    /// // Any `AsyncRead` works: a `tokio::fs::File`, a socket, a byte slice.
    /// let data: &'static [u8] = b"a,b\n1,2\n";
    /// let options = WriteOptions::default().gzip(true);
    /// sandbox.files().write_stream("/home/user/data.csv", data, &options).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn write_stream(
        &self,
        path: &str,
        reader: impl AsyncRead + Send + 'static,
        options: &WriteOptions,
    ) -> Result<WriteInfo> {
        // `BufReader` adds the buffered-read interface (`AsyncBufRead`) the
        // gzip encoder needs; reads still happen only as the body is sent.
        let reader = BufReader::new(reader);
        let written = if self.envd.version() >= EnvdVersion::OCTET_STREAM_UPLOAD {
            let body = if options.gzip {
                gzip_body(reader)
            } else {
                reqwest::Body::wrap_stream(ReaderStream::new(reader))
            };
            self.post_octet_stream(path, body, options, None).await?
        } else {
            // `Part::stream` has no length, so the part is sent chunked.
            // `post_multipart` reads only the user from `options`, so gzip is
            // dropped on this path, as in the SDKs.
            let body = reqwest::Body::wrap_stream(ReaderStream::new(reader));
            let part = Part::stream(body).file_name(path.to_owned());
            self.post_multipart(Form::new().part("file", part), Some(path), options, None)
                .await?
        };
        written.into_iter().next().ok_or_else(no_written_file)
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

/// The error for an upload answer that lists no written file
/// (`e2b/sandbox_async/filesystem/filesystem.py:424-428`).
fn no_written_file() -> Error {
    Error::Protocol("envd reported no written file".into())
}

/// Reads an upload response: the JSON array of written entries.
///
/// # Arguments
///
/// * `response` - envd's answer to a `POST /files`.
///
/// # Returns
///
/// One [`WriteInfo`] per entry, in envd's order.
///
/// # Errors
///
/// [`Error::Envd`] for a non-2xx status, [`Error::Protocol`] for an empty
/// array, [`Error::Decode`] or [`Error::Transport`].
async fn written_entries(response: reqwest::Response) -> Result<Vec<WriteInfo>> {
    if !response.status().is_success() {
        return Err(envd_http_error(response).await);
    }
    let body = response.bytes().await?;
    let entries: Vec<UploadedEntry> = serde_json::from_slice(&body)?;
    if entries.is_empty() {
        return Err(no_written_file());
    }
    Ok(entries.into_iter().map(WriteInfo::from).collect())
}

/// A request body that gzip-compresses `source` as it is sent, so the
/// compressed data is never held whole in memory.
///
/// # Arguments
///
/// * `source` - the plain bytes, as a buffered async reader. It must be
///   `Send + 'static` because reqwest may poll the body on another task.
///
/// # Returns
///
/// A streamed body (sent with chunked transfer encoding).
fn gzip_body(source: impl AsyncBufRead + Send + 'static) -> reqwest::Body {
    reqwest::Body::wrap_stream(ReaderStream::new(GzipEncoder::new(source)))
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
    fn write_options_default_to_multipart_and_chain() {
        assert_eq!(
            WriteOptions::default(),
            WriteOptions {
                user: None,
                octet_stream: false,
                gzip: false
            }
        );
        let options = WriteOptions::default()
            .user("root")
            .octet_stream(true)
            .gzip(true);
        assert_eq!(options.user.as_deref(), Some("root"));
        assert!(options.octet_stream && options.gzip);
    }

    #[test]
    fn uploaded_entry_converts_to_write_info() {
        let entry: UploadedEntry =
            serde_json::from_str(r#"{"name":"a","type":"dir","path":"/w/a"}"#).expect("json");
        let info = WriteInfo::from(entry);
        assert_eq!(info.name, "a");
        assert_eq!(info.path, "/w/a");
        assert_eq!(info.file_type, FileType::Directory);
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
