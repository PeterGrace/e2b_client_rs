//! JSON wire types for envd's `filesystem.Filesystem` Connect service.
//!
//! Field names and types follow `filesystem/filesystem.proto` as embedded in
//! the official Python SDK (`e2b/envd/filesystem/filesystem_pb.py`, line numbers
//! in `docs/protocol-notes.md`).

use serde::{Deserialize, Serialize};

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
}
