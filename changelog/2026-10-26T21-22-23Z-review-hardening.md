# Review follow-ups for PR #2

- Neither HTTP client follows redirects any more. reqwest forwards custom
  headers across hosts on a redirect, so `X-API-KEY` (REST) and
  `X-Access-Token` (envd) could have reached another host.
- `connect_sandbox` documents that the v1 fallback has to send a timeout, so
  `timeout: None` there sends 300 seconds and resets the sandbox lifetime.
- `CommandHandle::next_event` docs now match its behaviour: a clean end of
  stream without an exit event gives `Ok(None)`, and only `wait()` reports it
  as `Error::Protocol`.
- New tests: a 307 that is not followed, a compressed envelope rejected as
  `Error::Protocol`, and a stream that stalls after `start` ending in
  `Error::Timeout` from the command deadline (a raw TCP server, because
  wiremock's delays apply before the response headers).
- `.gitignore`: dropped cargo's commented-out boilerplate; `.worktrees/` stays.
