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
    /// Set for a PTY session, absent for a plain command.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) pty: Option<PtyConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) tag: Option<&'a str>,
    /// `Some` for a command; `None` for a PTY, whose start request the SDKs
    /// send without a `stdin` field (`e2b/sandbox_async/commands/pty.py:166-185`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) stdin: Option<bool>,
}

/// `process.PTY`.
#[derive(Debug, Clone, Copy, Serialize)]
pub(crate) struct PtyConfig {
    pub(crate) size: PtySizeWire,
}

/// `process.PTY.Size` (both fields are `uint32` in the proto).
#[derive(Debug, Clone, Copy, Serialize)]
pub(crate) struct PtySizeWire {
    pub(crate) cols: u32,
    pub(crate) rows: u32,
}

/// `process.UpdateRequest`: resizes a PTY.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct UpdateRequest {
    pub(crate) process: ProcessSelector,
    pub(crate) pty: PtyConfig,
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
    #[serde(default, deserialize_with = "deserialize_base64_opt")]
    pub(crate) pty: Option<Bytes>,
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

/// `process.ProcessInput`: a oneof of `stdin` or `pty` bytes.
///
/// Serde's externally tagged enum gives the proto3 JSON shape of a oneof:
/// `{"stdin": "<base64>"}` or `{"pty": "<base64>"}`. The `'a` lifetime lets
/// the message borrow the caller's bytes instead of copying them.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ProcessInput<'a> {
    /// Bytes for the process's stdin.
    Stdin(#[serde(serialize_with = "serialize_base64")] &'a [u8]),
    /// Bytes for the process's PTY (used by PTY sessions).
    Pty(#[serde(serialize_with = "serialize_base64")] &'a [u8]),
}

/// `process.StreamInputRequest`: a oneof of `start`, `data` and `keepalive`
/// (`e2b/envd/process/process_pb.py:597-716`).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum StreamInputRequest<'a> {
    /// Opens the stream for one process; always the first message.
    Start { process: ProcessSelector },
    /// One chunk of input.
    Data { input: ProcessInput<'a> },
    /// Keeps an idle stream alive; serializes as `{"keepalive": {}}`.
    Keepalive {},
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
    use serde_json::json;

    use super::*;

    #[test]
    fn stream_input_messages_match_proto_json() {
        let start = StreamInputRequest::Start {
            process: ProcessSelector::Pid(3),
        };
        let stdin = StreamInputRequest::Data {
            input: ProcessInput::Stdin(b"hi"),
        };
        let pty = StreamInputRequest::Data {
            input: ProcessInput::Pty(b"ls\r"),
        };

        assert_eq!(
            serde_json::to_value(start).expect("serialize start"),
            json!({"start": {"process": {"pid": 3}}})
        );
        assert_eq!(
            serde_json::to_value(stdin).expect("serialize stdin data"),
            json!({"data": {"input": {"stdin": "aGk="}}})
        );
        assert_eq!(
            serde_json::to_value(pty).expect("serialize pty data"),
            json!({"data": {"input": {"pty": "bHMN"}}})
        );
        assert_eq!(
            serde_json::to_value(StreamInputRequest::Keepalive {}).expect("serialize keepalive"),
            json!({"keepalive": {}})
        );
    }

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
            pty: None,
            tag: None,
            stdin: Some(false),
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
    fn pty_start_request_has_pty_size_and_no_stdin() {
        let envs = HashMap::new();
        let request = StartRequest {
            process: ProcessConfig {
                cmd: "/bin/bash",
                args: vec!["-i", "-l"],
                envs: &envs,
                cwd: None,
            },
            pty: Some(PtyConfig {
                size: PtySizeWire { cols: 80, rows: 24 },
            }),
            tag: None,
            stdin: None,
        };

        let json = serde_json::to_value(&request).expect("serialize");

        assert_eq!(
            json,
            json!({
                "process": {"cmd": "/bin/bash", "args": ["-i", "-l"]},
                "pty": {"size": {"cols": 80, "rows": 24}}
            })
        );
    }

    #[test]
    fn update_request_matches_proto_json() {
        let request = UpdateRequest {
            process: ProcessSelector::Pid(5),
            pty: PtyConfig {
                size: PtySizeWire {
                    cols: 120,
                    rows: 40,
                },
            },
        };

        let json = serde_json::to_value(&request).expect("serialize");

        assert_eq!(
            json,
            json!({"process": {"pid": 5}, "pty": {"size": {"cols": 120, "rows": 40}}})
        );
    }

    #[test]
    fn data_event_parses_pty_bytes() {
        let response: EventResponse =
            serde_json::from_str(r#"{"event":{"data":{"pty":"JCA="}}}"#).expect("deserialize");

        let data = response
            .event
            .and_then(|event| event.data)
            .expect("data event");

        assert_eq!(data.pty.as_deref(), Some(&b"$ "[..]));
        assert!(data.stdout.is_none() && data.stderr.is_none());
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
            input: ProcessInput::Stdin(b"hi\n"),
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
    fn end_event_parses_exit_code_and_error() {
        let response: EventResponse = serde_json::from_str(
            r#"{"event":{"end":{"exitCode":137,"exited":false,"error":"signal: killed"}}}"#,
        )
        .expect("deserialize");
        let end = response
            .event
            .and_then(|event| event.end)
            .expect("end event");
        assert_eq!(end.exit_code, 137);
        assert_eq!(end.error.as_deref(), Some("signal: killed"));
    }

    #[test]
    fn unknown_event_kinds_and_fields_are_ignored() {
        let response: EventResponse =
            serde_json::from_str(r#"{"event":{"keepalive":{}},"extra":1}"#).expect("deserialize");
        let event = response.event.expect("event");
        assert!(event.start.is_none() && event.data.is_none() && event.end.is_none());
    }
}
