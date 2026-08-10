# code-review-reality-server-sso-upstream-no-timeout

**Vector:** security
**Score:** 2
**Source:** rotating-expert-review reality-server 2026-08-10 (Phase 1.5)
**Confidence:** high

## Hypothesis
The three SSO helper functions `exchange_code_for_tokens`, `get_user_info`, and `introspect_pm_token` in `routes/sso.rs` each construct `reqwest::Client::new()` per request with default settings, which means **no timeout**. Every public /sso/* endpoint funnels into these helpers, so a slow or hung upstream PM OAuth backend (or transient PM degradation) can pin each SSO handler task and its outbound socket until the kernel kills the connection. Unauthenticated traffic can therefore starve reality-server task capacity. The correct pattern already exists in the file: `PmApiClient::new` builds via `reqwest::Client::builder().timeout(Duration::from_secs(timeout_seconds)).build()` (`state.rs:757-760`). Mirror it.

## Evidence
- `backend/servers/reality-server/src/routes/sso.rs:682` — `exchange_code_for_tokens`: `let client = reqwest::Client::new();` (default, no timeout).
- `backend/servers/reality-server/src/routes/sso.rs:706` — `get_user_info`: same pattern; reachable from `/sso/callback`, `/sso/mobile/token`, `/sso/mobile/validate`, `/sso/exchange`, `/sso/sync`.
- `backend/servers/reality-server/src/routes/sso.rs:743` — `introspect_pm_token`: same pattern; reachable from `/sso/mobile/token`, `/sso/exchange`, `/sso/sync`.
- `backend/servers/reality-server/src/state.rs:757-760` (and `state.rs:941` — construction) — `PmApiClient::new` uses `Client::builder().timeout(Duration::from_secs(timeout_seconds)).build()`. The pattern is available, it just isn't used for the SSO upstreams.

## Files
- `backend/servers/reality-server/src/routes/sso.rs`
- `backend/servers/reality-server/src/state.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (security)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. Start `reality-server` locally with `PM_API_URL` pointed at a test HTTP server that accepts the connection and never responds (e.g. `nc -l 8099`).
2. `curl -v "http://localhost:8081/api/v1/sso/callback?code=abc&state=stub"` (or any endpoint that hits `exchange_code_for_tokens`).
3. Observe: request hangs indefinitely; connect a second and a third — each parks a tokio task.
4. Expected: request fails with a timeout error after N seconds (e.g. 10s), handler releases the task.

## Suggested approach
1. Introduce a single shared `reqwest::Client` for outbound SSO calls, constructed once and stored on `AppState` — mirror `PmApiClient` (`state.rs:757-760`, `:941`). Timeout knob from env: `SSO_UPSTREAM_HTTP_TIMEOUT_SECS`, default 10.
2. Rewrite `exchange_code_for_tokens` / `get_user_info` / `introspect_pm_token` to take the shared client from `AppState` rather than calling `reqwest::Client::new()`.
3. Include a connect timeout as well as a total timeout — the total is what matters, connect is defence in depth: `.connect_timeout(Duration::from_secs(5)).timeout(Duration::from_secs(SSO_UPSTREAM_HTTP_TIMEOUT_SECS))`.
4. Add a `tracing::warn!` on timeout error paths so hangs are observable.
5. Do NOT change response error mapping — a timeout should still surface as the existing upstream-error branch.
6. If the shared-client refactor is too broad, an acceptable minimal fix is to add `Client::builder().timeout(…).build().expect(…)` **inline** in each of the three helpers — same effect, more boilerplate. Prefer the shared-client route.

## Alternatives considered
- **Middleware-level request timeout (Tower `TimeoutLayer`)** — rejected because it terminates the request, not the outbound socket; the reqwest task keeps running and holds the connection until the kernel gives up. The bug is on the *outbound* client, not the inbound server.
- **Rely on system-level TCP retransmission timeouts (~15 min)** — rejected because that's the whole point of the report: default behaviour is already this.

## Root-cause trace
1. Symptom: `reality-server` handler tasks pile up during PM outage / slow upstream; new SSO requests queue behind stuck ones.
2. ← `routes/sso.rs:682` / `:706` / `:743` — outbound clients built with `reqwest::Client::new()` (no timeout).
3. ← `state.rs:757-760` — the correct constructor exists but is only used by `PmApiClient`; the SSO helpers pre-date or shortcut it.
4. Origin: latent since the SSO OAuth flow was introduced.

## Test plan
- [ ] Add a Rust integration test in `backend/servers/reality-server/tests/suites/` (e.g. `sso_upstream_timeout_tests.rs`) that spawns a `TcpListener` in the test, has the SSO helper point at it, does NOT read from the socket, and asserts the helper returns an error within `2 * SSO_UPSTREAM_HTTP_TIMEOUT_SECS`.
- [ ] Verify happy path still works: a real 200-response upstream returns quickly and the helper parses it.
- [ ] Run: `cargo test -p reality-server --test sso_upstream_timeout_tests` (adjust name as picked).

## Out of scope
- Circuit-breaker / retry policy for the SSO upstream (a follow-up plan; timeout is the required baseline).
- Rewriting `PmApiClient` — already correct, just needs to be the pattern the SSO helpers emulate.
- Adding timeouts to every other `reqwest::Client::new()` call in the workspace — do them one at a time under their own signals so blast radius stays scoped.

## After-merge
- Move this file to `plans/_archive/code-review-reality-server-sso-upstream-no-timeout.md`
- Mark `backlog.json` row `code-review-reality-server-sso-upstream-no-timeout` as `status: "done"`
