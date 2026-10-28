# M4 Task 2: error taxonomy and an opaque transport error (issue #92)

`Error` gains `status()` and the SDK 2.61 categories as classifiers:
`is_auth`, `is_rate_limited`, `is_service_busy`, `is_not_enough_space`,
`is_sandbox_unreachable`, `is_timeout` and `is_retryable`, next to
`is_not_found` (CLM-0169). `Error::Transport` holds the new opaque
`TransportError` (`is_timeout`, `is_connect`, `Display`, `source()`), so no
reqwest type is public, and `secrecy` is re-exported (CLM-0170).

## Breaking changes

- `Error::Transport(reqwest::Error)` is now `Error::Transport(TransportError)`.
  Code that called reqwest methods on it (`err.is_timeout()` on the inner
  `reqwest::Error`) calls `TransportError::is_timeout` / `is_connect`, or
  `Error::is_timeout` / `is_retryable`.
- `impl From<reqwest::Error> for Error` is gone, so `?` no longer turns a
  `reqwest::Error` into an `Error`. Inside the crate every reqwest result is
  converted with `.map_err(Error::transport)` (30 sites in `src/client.rs`,
  `src/envd/connect.rs`, `src/envd/mod.rs`, `src/filesystem.rs`,
  `src/sandbox.rs`).
- `Error::Api`, `Error::Rpc` and `Error::Envd` are `#[non_exhaustive]`:
  patterns need `..` (`Error::Api { status, message }` becomes
  `Error::Api { status, message, .. }`), and the variants can no longer be
  built outside the crate.
- New public items (not breaking): `TransportError`, the `secrecy` re-export
  (`e2b_client::secrecy::ExposeSecret`).

## Tests

- New: `tests/error_taxonomy.rs` (5 tests), 5 unit tests in `src/error.rs`,
  doctests on `status`, `is_not_found`, `is_timeout`, `is_retryable` and
  `TransportError`.
- Changed: 19 patterns in 9 test files gained `..`.
- Inspection (CLM-0170): `cargo doc --no-deps && ! grep -rl reqwest
  target/doc/e2b_client/ --include=*.html` exits 0. The `write_stream`
  rustdoc says "the HTTP client" instead of naming reqwest.
- Live on hosted, as a smoke test of the converted transport path:
  `clm_0015_connect_unknown_sandbox_is_not_found` passed (run
  `dev-20261028T130658Z-2723204`).

## Docs

- `README.md`: the Errors row names `TransportError` and the classifiers.
- `docs/protocol-notes.md`: new section "Error classification"; a first
  bullet in "Deliberate differences" (classifier methods, not exception
  classes).
- `docs/verification/s4-evidence-map.md`: new row CLM-0169; the intro's M4
  list is "CLM-0168 and CLM-0169".
- The M4 plan's Task 2 carries a Corrected note: 30 conversion sites, not 32,
  and the line numbers that moved after Task 1.
