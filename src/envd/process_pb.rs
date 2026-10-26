//! JSON wire types for envd's `process.Process` Connect service.
//!
//! Field names and types follow `process/process.proto` as embedded in the
//! official Python SDK (`e2b/envd/process/process_pb.py`, line numbers in
//! `docs/protocol-notes.md`). Only the messages this crate sends or receives
//! are modelled; proto3 JSON uses lowerCamelCase field names and omits fields
//! that hold their default value, so every field here tolerates absence.

use std::collections::HashMap;

use bytes::Bytes;
use serde::{Deserialize, Serialize};

use crate::wire::{deserialize_base64_opt, serialize_base64};

/// Service path prefix.
pub(crate) const SERVICE: &str = "process.Process";

/// `process.ProcessConfig`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProcessConfig<'a> {
    pub(crate) cmd: &'a str,
    pub(crate) args: Vec<&'a str>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub(crate) envs: &'a HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) cwd: Option<&'a str>,
}

/// `process.StartRequest`.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct StartRequest<'a> {
    pub(crate) process: ProcessConfig<'a>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) tag: Option<&'a str>,
    pub(crate) stdin: bool,
}

/// `process.ProcessSelector`: a oneof of `pid` or `tag`.
///
/// Serde's default "externally tagged" enum encoding produces exactly the
/// proto3 JSON shape of a oneof: `{"pid": 7}` or `{"tag": "x"}`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ProcessSelector {
    Pid(u32),
}

/// `process.ConnectRequest`.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct ConnectRequest {
    pub(crate) process: ProcessSelector,
}

/// `process.StartResponse` and `process.ConnectResponse` (same shape).
#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct EventResponse {
    #[serde(default)]
    pub(crate) event: Option<ProcessEvent>,
}

/// `process.ProcessEvent`: a oneof of `start`, `data`, `end`, `keepalive`.
///
/// Modelled as optional fields rather than an enum so that an event type a
/// newer envd adds is ignored instead of failing the whole stream.
#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct ProcessEvent {
    #[serde(default)]
    pub(crate) start: Option<StartEvent>,
    #[serde(default)]
    pub(crate) data: Option<DataEvent>,
    #[serde(default)]
    pub(crate) end: Option<EndEvent>,
}

/// `process.ProcessEvent.StartEvent`.
#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct StartEvent {
    #[serde(default)]
    pub(crate) pid: u32,
}

/// `process.ProcessEvent.DataEvent`: a oneof of `stdout`, `stderr`, `pty`.
#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct DataEvent {
    #[serde(default, deserialize_with = "deserialize_base64_opt")]
    pub(crate) stdout: Option<Bytes>,
    #[serde(default, deserialize_with = "deserialize_base64_opt")]
    pub(crate) stderr: Option<Bytes>,
}

/// `process.ProcessEvent.EndEvent` (the `exited` and `status` fields are not
/// needed and are skipped).
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EndEvent {
    #[serde(default)]
    pub(crate) exit_code: i32,
    #[serde(default)]
    pub(crate) error: Option<String>,
}

/// `process.ListRequest` (empty).
#[derive(Debug, Clone, Serialize)]
pub(crate) struct ListRequest {}

/// `process.ListResponse`.
#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct ListResponse {
    #[serde(default)]
    pub(crate) processes: Vec<ProcessInfoWire>,
}

/// `process.ProcessInfo`.
#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct ProcessInfoWire {
    #[serde(default)]
    pub(crate) config: Option<ProcessConfigWire>,
    #[serde(default)]
    pub(crate) pid: u32,
    #[serde(default)]
    pub(crate) tag: Option<String>,
}

/// `process.ProcessConfig`, as received.
#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct ProcessConfigWire {
    #[serde(default)]
    pub(crate) cmd: String,
    #[serde(default)]
    pub(crate) args: Vec<String>,
    #[serde(default)]
    pub(crate) envs: HashMap<String, String>,
    #[serde(default)]
    pub(crate) cwd: Option<String>,
}

/// `process.SendInputRequest`.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct SendInputRequest<'a> {
    pub(crate) process: ProcessSelector,
    pub(crate) input: ProcessInput<'a>,
}

/// `process.ProcessInput`: the `stdin` member of the oneof.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct ProcessInput<'a> {
    #[serde(serialize_with = "serialize_base64")]
    pub(crate) stdin: &'a [u8],
}

/// `process.SendSignalRequest`.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct SendSignalRequest {
    pub(crate) process: ProcessSelector,
    pub(crate) signal: &'static str,
}

/// `process.CloseStdinRequest`.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct CloseStdinRequest {
    pub(crate) process: ProcessSelector,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_request_matches_proto_json() {
        let envs = HashMap::from([("A".to_owned(), "1".to_owned())]);
        let request = StartRequest {
            process: ProcessConfig {
                cmd: "/bin/bash",
                args: vec!["-l", "-c", "echo hi"],
                envs: &envs,
                cwd: Some("/tmp"),
            },
            tag: None,
            stdin: false,
        };
        let json = serde_json::to_value(&request).expect("serialize");
        assert_eq!(
            json,
            serde_json::json!({
                "process": {
                    "cmd": "/bin/bash",
                    "args": ["-l", "-c", "echo hi"],
                    "envs": {"A": "1"},
                    "cwd": "/tmp"
                },
                "stdin": false
            })
        );
    }

    #[test]
    fn selector_is_a_oneof_object() {
        let json = serde_json::to_value(ConnectRequest {
            process: ProcessSelector::Pid(7),
        })
        .expect("serialize");
        assert_eq!(json, serde_json::json!({"process": {"pid": 7}}));
    }

    #[test]
    fn send_input_base64_encodes_stdin() {
        let json = serde_json::to_value(SendInputRequest {
            process: ProcessSelector::Pid(1),
            input: ProcessInput { stdin: b"hi\n" },
        })
        .expect("serialize");
        assert_eq!(
            json,
            serde_json::json!({"process": {"pid": 1}, "input": {"stdin": "aGkK"}})
        );
    }

    #[test]
    fn end_event_defaults_apply_when_fields_are_omitted() {
        let response: EventResponse =
            serde_json::from_str(r#"{"event":{"end":{"exited":true,"status":"exit status 0"}}}"#)
                .expect("deserialize");
        let end = response
            .event
            .and_then(|event| event.end)
            .expect("end event");
        assert_eq!(end.exit_code, 0);
        assert!(end.error.is_none());
    }

    #[test]
    fn unknown_event_kinds_and_fields_are_ignored() {
        let response: EventResponse =
            serde_json::from_str(r#"{"event":{"keepalive":{}},"extra":1}"#).expect("deserialize");
        let event = response.event.expect("event");
        assert!(event.start.is_none() && event.data.is_none() && event.end.is_none());
    }
}
