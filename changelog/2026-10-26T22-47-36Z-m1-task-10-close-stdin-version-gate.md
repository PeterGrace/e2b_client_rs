# M1 Task 10: close_stdin version gate (issue #19, fixes CLM-0109)

- `Commands::close_stdin` now refuses envd older than 0.5.2 with
  `Error::InvalidArgument`, before any request is sent. envd below 0.5.2 has
  no `CloseStdin`; the SDK refuses the call in the same case
  (py `sandbox_async/commands/command.py:189-193`, `envd/versions.py:7`).
  Before, the crate sent the request, and the result depended on what envd
  answered.
- The threshold is `EnvdVersion::CLOSE_STDIN` in `src/envd/mod.rs`, next to
  `DEFAULT_USER` (0.4.0). An unparseable version is treated as 99.99.99, so
  it passes the gate, as in the SDK's debug mode.
- `InvalidArgument` was kept because it is the crate's variant for a call
  refused before any request (as with a depth-0 `list`), and it is what the
  refuting test expected.
- `tests/envd_api.rs::close_stdin_is_refused_before_envd_0_5_2` is no longer
  ignored. `version_parsing_and_ordering` pins the 0.5.1/0.5.2 boundary.
- The S4 evidence map's CLM-0109 row is now covered, and the plan has a
  Task 10 section.
