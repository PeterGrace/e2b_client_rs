# M2 Task 6: Streamed upload and download

Issue #35, Task 6 of `docs/superpowers/plans/2026-10-27-milestone-2-command-and-file-parity.md`.
Claims CLM-0066 and CLM-0128 (S3), CLM-0138 (now whole) and CLM-0142 (S4).

## What changed

- `Filesystem::write_stream(path, impl AsyncRead + Send + 'static,
  &WriteOptions) -> WriteInfo`. The reader is wrapped in a tokio
  `BufReader` and read as the body is sent: an octet-stream
  `POST /files?path=<p>` on envd 0.5.7 or newer, gzip-compressed on the fly
  (`gzip_body`) with `Content-Encoding: gzip` when `gzip(true)`, and a
  multipart upload with one streamed `file` part, uncompressed, on older
  envd. It reuses Task 5's `post_octet_stream`, `post_multipart` and
  `no_written_file`, and passes `None` as the timeout, so the client's
  request timeout does not bound it (the SDK's behaviour).
- `Filesystem::read_stream(path, user) -> FileStream`, and the new public
  `FileStream` with `next_chunk() -> Option<Bytes>`. The `GET /files`
  request has no timeout; envd's status is checked before returning, so a
  missing file fails `read_stream` itself with `Error::Envd { status: 404 }`.
- `Cargo.toml`: tokio's `io-util` feature, for `BufReader`. `Cargo.lock` is
  unchanged (the feature pulls in no new crate), so async-compression stays
  at 0.4.42.
- `tests/common`: `sandbox_from_json_with_client(server, &client, created)`;
  `sandbox_from_json` now calls it with `client_for`.
- Tests: eight mock tests in the new `tests/envd_stream_files.rs`, and the
  live tests `clm_0066_write_stream_uploads_more_than_it_buffers` (64 MiB
  from `tokio::io::repeat`, plain and gzip) and
  `clm_0128_read_stream_downloads_in_chunks` (8 MiB checked chunk by chunk).
  Both passed against hosted E2B, one at a time.
- Docs: streamed rows in the "Uploads" section of `docs/protocol-notes.md`, a
  streamed-download row in the file API table, streamed uploads removed from
  "Not implemented"; in the S4 evidence map, CLM-0138 is now `yes` and
  CLM-0142 has a row.

## Why it differs from the plan

The plan has been corrected in this same PR (dated notes in Task 6):

- The multipart fallback passes `options` to `post_multipart` unchanged,
  not a clone with `gzip: false`: `post_multipart` never reads `gzip`. Both
  branches therefore pass the caller's options through, so Task 7's
  metadata headers reach `write_stream` without further change there.
- The timeout tests use a 1 s request timeout and a 3 s answer delay rather
  than 100 ms and 300 ms, so sandbox creation stays well inside the timeout
  on a slow CI runner. A mutation check (giving both calls the request
  timeout) makes exactly the two timeout tests fail.
- One extra test, `write_stream_surfaces_a_reader_error_as_transport`, pins
  the rustdoc's promise that a reader error is `Error::Transport`.
