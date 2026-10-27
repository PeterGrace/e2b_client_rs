# M2 Task 7: File metadata

Issue #36, Task 7 of `docs/superpowers/plans/2026-10-27-milestone-2-command-and-file-parity.md`.
Claims CLM-0129 (S3) and CLM-0139 (S4).

## What changed

- `WriteOptions::metadata(key, value)` adds one pair to a sorted
  `BTreeMap`, so the headers always go out in the same order. Every upload
  request sends one `X-Metadata-<key>: <value>` header per pair: the
  multipart request, each octet-stream request, and the streamed
  `write_stream` request. The headers are added only in `post_multipart` and
  `post_octet_stream`, which now take them as a `metadata: &HeaderMap`
  parameter.
- `WriteOptions::metadata_headers(version)` validates the pairs before any
  request is sent. A key must be a non-empty HTTP token, and a value must be
  printable US-ASCII. It then refuses any metadata on envd older than 0.6.2
  (`EnvdVersion::FILE_METADATA`), and builds the headers. Every failure is
  `Error::InvalidArgument`. `write_files` calls it after the
  empty-`entries` return, and `write_stream` calls it first.
- `WriteInfo` and `EntryInfo` gain `pub metadata: HashMap<String, String>`,
  which is empty when none is set. The values come from
  `UploadedEntry.metadata` (via `impl From<UploadedEntry> for WriteInfo`)
  and `EntryInfoWire.metadata` (proto field 11). `list`, `get_info` and
  `include_entry` watch events therefore report metadata too.
- `src/wire.rs`: `deserialize_null_as_default`, so an explicit JSON `null`
  reads as an empty map, like an absent field.
- Tests:
  - four mock tests in `tests/envd_write.rs`;
  - three unit tests on `metadata_headers`;
  - a null/absent unit test in `src/envd/filesystem_pb.rs`;
  - metadata pinned in `entry_info_parses_proto_json`,
    `watch_dir_response_members_parse` and
    `uploaded_entry_converts_to_write_info`;
  - the live test `clm_0129_write_metadata_round_trips`, which passed
    against hosted E2B (envd 0.10.0).

  A mutation check confirmed the header test: dropping the headers from
  `post_multipart` makes it fail.
- Docs:
  - metadata rows in the "Uploads" section of `docs/protocol-notes.md`;
  - a CLM-0139 row in the S4 evidence map.

## Why it differs from the plan

The plan is corrected in this same PR, with a dated note in Task 7 Step 2:

- The check is a `WriteOptions` method that takes the envd version, like
  `WatchOptions::check_supported`, not a `Filesystem` method.
- Keys and values are validated before the version is checked, in the
  SDK's order. The plan said the version came first.
- The empty batch returns before the check, as in the SDK.
- The headers are passed to `post_*` once they are validated, so they are
  not re-derived for each request.
- The `null` tolerance is new. The SDK has it too, through `map_metadata`.

Task 8 and Task 11 name none of the changed symbols.
