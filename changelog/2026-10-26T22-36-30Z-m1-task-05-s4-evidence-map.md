# M1 Task 5: S4 evidence map and missing wire tests (issue #9)

- `docs/verification/s4-evidence-map.md` maps each of the 51 claims (S4's
  CLM-0067 to CLM-0116, plus CLM-0122) to the tests that prove it. Existing
  tests cover 29 claims, and new or tightened tests cover 19. CLM-0085 is
  only partly covered, and CLM-0071 and CLM-0109 are refuted.
- Refuting tests, kept and ignored. Per the plan, the crate is not fixed
  here:
  - `config::tests::explicit_empty_values_count_as_unset` (CLM-0071). An
    explicit empty builder value is kept instead of counting as unset, and
    `.api_url("")` makes `build()` fail.
  - `envd_api::close_stdin_is_refused_before_envd_0_5_2` (CLM-0109).
    `close_stdin` has no envd version check.
- CLM-0085 is partial: `clientID` in a create or connect response is never
  parsed, so no test can assert it.
- New tests:
  - REST: every request carries `x-api-key`.
  - envd: every request carries the sandbox id and port headers, and the
    access token only when one was issued, never the API key. RPCs are
    uncompressed Connect JSON at `/<pkg.Service>/<Method>`. The file API
    selects the user with `username`, defaulting to `user` on legacy envd.
    The first stream event must arrive within the request timeout.
  - Config: explicit, env and default resolution for every setting, and the
    full hosted-domain list.
  - Sandbox: envd addressing follows the domain the API returned, and the
    `Debug` output of `Client`, `ConnectionConfig` and `Sandbox` omits the
    API key and the access token.
  - Models and protocol: several metadata pairs encoded into one string,
    `SandboxCreated` fields, an end event's `error`, and the 99.99.99 version
    fallback.
- Tightened assertions: literal defaults (`e2b.app`, 60 s), `SandboxNotFound`
  on a 404 from info, the keepalive header on Connect, no request on a
  depth-0 list, and literal snake_case spellings for every `ConnectCode`.
- Test helpers: `common::sandbox_from_json` builds a sandbox from any create
  response, and `stalling_envd` now takes the frames it sends before
  stalling.
- Plan: two dated corrections in Task 5 (the map format, the CLM-0078 test
  name, the ignore reason format). Task 8 Step 1 now records one piece of
  evidence per test, not per row.
