//! JSON wire types for envd's `filesystem.Filesystem` Connect service.
//!
//! Field names and types follow `filesystem/filesystem.proto` as embedded in
//! the official Python SDK (`e2b/envd/filesystem/filesystem_pb.py`, line numbers
//! in `docs/protocol-notes.md`).

use serde::{Deserialize, Serialize};

use crate::envd::connect::Empty;
use crate::wire::{EnumValue, deserialize_i64_lenient};

/// Service path prefix.
pub(crate) const SERVICE: &str = "filesystem.Filesystem";

/// A request that carries only a path (`StatRequest`, `MakeDirRequest`,
/// `RemoveRequest`).
#[derive(Debug, Clone, Serialize)]
pub(crate) struct PathRequest<'a> {
    pub(crate) path: &'a str,
}

/// `filesystem.ListDirRequest`.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct ListDirRequest<'a> {
    pub(crate) path: &'a str,
    pub(crate) depth: u32,
}

/// `filesystem.MoveRequest`.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct MoveRequest<'a> {
    pub(crate) source: &'a str,
    pub(crate) destination: &'a str,
}

/// A response that carries one optional entry (`StatResponse`,
/// `MoveResponse`, `MakeDirResponse`).
#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct EntryResponse {
    #[serde(default)]
    pub(crate) entry: Option<EntryInfoWire>,
}

/// `filesystem.ListDirResponse`.
#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct ListDirResponse {
    #[serde(default)]
    pub(crate) entries: Vec<EntryInfoWire>,
}

/// `filesystem.EntryInfo`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EntryInfoWire {
    #[serde(default)]
    pub(crate) name: String,
    #[serde(default, rename = "type")]
    pub(crate) file_type: Option<EnumValue>,
    #[serde(default)]
    pub(crate) path: String,
    #[serde(default, deserialize_with = "deserialize_i64_lenient")]
    pub(crate) size: i64,
    #[serde(default)]
    pub(crate) mode: u32,
    #[serde(default)]
    pub(crate) permissions: String,
    #[serde(default)]
    pub(crate) owner: String,
    #[serde(default)]
    pub(crate) group: String,
    #[serde(default)]
    pub(crate) modified_time: Option<String>,
    #[serde(default)]
    pub(crate) symlink_target: Option<String>,
}

/// `filesystem.WatchDirRequest`
/// (`e2b/envd/filesystem/filesystem_pb.py:404-455`).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WatchDirRequest<'a> {
    pub(crate) path: &'a str,
    pub(crate) recursive: bool,
    pub(crate) include_entry: bool,
    pub(crate) allow_network_mounts: bool,
}

/// `filesystem.WatchDirResponse`: a oneof of `start`, `filesystem` and
/// `keepalive` (`e2b/envd/filesystem/filesystem_pb.py:499-560`).
///
/// Modelled as optional fields rather than an enum, like `ProcessEvent`, so a
/// keepalive (or a member a newer envd adds) decodes to all-`None` instead of
/// failing the stream.
#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct WatchDirResponse {
    /// Present (as an empty object) on the opening event.
    #[serde(default)]
    pub(crate) start: Option<Empty>,
    #[serde(default)]
    pub(crate) filesystem: Option<FilesystemEventWire>,
}

/// `filesystem.FilesystemEvent`
/// (`e2b/envd/filesystem/filesystem_pb.py:457-497`).
#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct FilesystemEventWire {
    #[serde(default)]
    pub(crate) name: String,
    #[serde(default, rename = "type")]
    pub(crate) event_type: Option<EnumValue>,
    #[serde(default)]
    pub(crate) entry: Option<EntryInfoWire>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_info_parses_proto_json() {
        let json = r#"{"entry":{"name":"a.txt","type":"FILE_TYPE_FILE","path":"/tmp/a.txt",
            "size":"12","mode":420,"permissions":"-rw-r--r--","owner":"user","group":"user",
            "modifiedTime":"2026-01-02T03:04:05Z","unknownField":true}}"#;
        let response: EntryResponse = serde_json::from_str(json).expect("deserialize");
        let entry = response.entry.expect("entry");
        assert_eq!(entry.name, "a.txt");
        assert_eq!(
            entry.file_type,
            Some(EnumValue::Name("FILE_TYPE_FILE".into()))
        );
        assert_eq!(entry.size, 12);
        assert_eq!(entry.mode, 0o644);
        assert_eq!(entry.modified_time.as_deref(), Some("2026-01-02T03:04:05Z"));
    }

    #[test]
    fn list_dir_request_serializes_depth() {
        let json = serde_json::to_value(ListDirRequest {
            path: "/",
            depth: 2,
        })
        .expect("ser");
        assert_eq!(json, serde_json::json!({"path": "/", "depth": 2}));
    }

    #[test]
    fn watch_dir_request_serializes_camel_case() {
        let json = serde_json::to_value(WatchDirRequest {
            path: "/w",
            recursive: true,
            include_entry: false,
            allow_network_mounts: true,
        })
        .expect("serialize watch request");
        assert_eq!(
            json,
            serde_json::json!({"path": "/w", "recursive": true, "includeEntry": false,
                "allowNetworkMounts": true})
        );
    }

    #[test]
    fn watch_dir_response_members_parse() {
        let start: WatchDirResponse = serde_json::from_str(r#"{"start":{}}"#).expect("start");
        assert!(start.start.is_some() && start.filesystem.is_none());

        let keepalive: WatchDirResponse =
            serde_json::from_str(r#"{"keepalive":{}}"#).expect("keepalive");
        assert!(keepalive.start.is_none() && keepalive.filesystem.is_none());

        let event: WatchDirResponse = serde_json::from_str(
            r#"{"filesystem":{"name":"a.txt","type":"EVENT_TYPE_CREATE",
                "entry":{"name":"a.txt","path":"/w/a.txt"}}}"#,
        )
        .expect("filesystem event");
        let event = event.filesystem.expect("filesystem member");
        assert_eq!(event.name, "a.txt");
        assert_eq!(
            event.event_type,
            Some(EnumValue::Name("EVENT_TYPE_CREATE".into()))
        );
        assert_eq!(event.entry.expect("entry").path, "/w/a.txt");
    }
}
