# code-review-reality-server-sso-reqwest-no-timeout

**Vector:** security
**Score:** 2
**Source:** Phase 1.5 rotating expert review 2026-08-18 (reality-server segment)
**Confidence:** high

## Hypothesis
Three reality-server SSO handlers build ad-hoc `reqwest::Client::new()` instances and call `.send().await` with no per-request or per-client timeout. `reqwest` ships no default wall-clock bound, so a slow or hung PM upstream response holds an axum worker task (and any acquired DB connection) indefinitely. `create_mobile_sso_token` is anonymous-callable, so an attacker submitting bogus `pm_access_token` values can force one PM round-trip per request; concurrent floods saturate axum worker slots and the DB pool that `create_session` acquires. Installing a bounded client (same pattern reality-server already uses for the PM health probe at `state.rs:757`) closes the amplification.

## Evidence
- `backend/servers/reality-server/src/routes/sso.rs:687` — `exchange_code_for_tokens` uses `reqwest::Client::new().post(...).send().await`; no `.timeout()` on client or request.
- `backend/servers/reality-server/src/routes/sso.rs:711` — `get_user_info` follows the same unbounded pattern.
- `backend/servers/reality-server/src/routes/sso.rs:748` — `introspect_pm_token` follows the same unbounded pattern.
- `backend/servers/reality-server/src/state.rs:757` — the PM health probe already builds a shared `reqwest::Client` with `.timeout(Duration::from_secs(...))`, proving the pattern is understood.
- `backend/servers/reality-server/src/main.rs:485` — `routes::sso::router()` mounts `sso_callback`, `create_mobile_sso_token`, `exchange_pm_token`, `sync_session`; the mobile-token path is anonymous.

## Files
- `backend/servers/reality-server/src/routes/sso.rs`
- `backend/servers/reality-server/src/state.rs`

## Dependencies

## Required capabilities
- [ ] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

Mode: cloud-ok

## Repro steps
1. Start reality-server locally with a PM upstream that stalls on `/token`, `/userinfo`, `/introspect` (a slow-loris mock: accept the TCP connection, hold headers, never respond).
2. Issue `POST /api/v1/sso/mobile/token` with a syntactically valid but unresolvable `pm_access_token`. Observe the axum task hanging on `send().await` — the request never returns, and its DB connection stays checked out.
3. Repeat step 2 concurrently up to the axum worker pool size. New unrelated public endpoints (e.g. `GET /api/v1/listings`) begin queueing behind the exhausted worker/DB pool.
4. Expected: each SSO handler bounds the PM call at a few seconds and returns 504 / 502 / a mapped error so worker slots free promptly.

## Suggested approach
1. Add a single shared, bounded `reqwest::Client` on `AppState` (or a lazily-initialised `OnceLock<Client>` in `sso.rs`). Base it on the pattern at `state.rs:757` — `.timeout(Duration::from_secs(10))` for the whole request, plus `.connect_timeout(Duration::from_secs(5))`.
2. Replace all three `reqwest::Client::new().send().await` sites (`sso.rs:687`, `:711`, `:748`) with `state.pm_client.<verb>(...)`, threading `state: State<AppState>` if not already available (each handler already receives it).
3. Map the `reqwest::Error::is_timeout()` case to a distinct 504 response (`ApiError::UpstreamTimeout` or the existing bad-gateway variant) so callers see a clean failure, not a hang.
4. Add a `tracing::warn!(...)` log with the failing method + duration for observability.
5. Reuse `.pool_max_idle_per_host(...)` from the health-probe client if it's tuned; do not introduce a second per-request Connection tower.

## Alternatives considered
- **Wrap each call site in `tokio::time::timeout(...)`** — rejected because it leaves the underlying HTTP connection dangling in the reqwest pool and doesn't cancel the socket read; a shared bounded client cancels the request end-to-end.
- **Move all SSO calls to a dedicated background worker with a queue** — rejected because it changes the request/response contract for callers and adds operational surface for what is fundamentally a single-line-per-site timeout fix.

## Root-cause trace
N/A — security vector; the failure is a missing bound on a public-facing external call, not a data-flow regression. The `state.rs:757` PM health probe adopted `.timeout()` when it was introduced; the SSO handlers pre-date that pattern and were never retrofitted.

## Test plan
- [ ] `backend/servers/reality-server/tests/sso_timeout.rs` — integration test using a `tokio::net::TcpListener` that accepts connections and never writes; assert the SSO handler returns within N seconds with 504.
- [ ] Regression: `cargo test -p reality-server sso` for the existing SSO flow tests (upsert_sso_user, create_session) still pass.
- [ ] Local: `cargo test -p reality-server --test sso_timeout` — the new file above.

## Out of scope
- Broader network-egress policy for reality-server (env-driven allowlist etc.).
- Timeout hardening of other external HTTP calls in reality-server (agencies, imports, etc.) — file separate plans if audits find them.
- Refactoring the SSO error taxonomy beyond adding one upstream-timeout variant.

## After-merge
- Move this file to `plans/_archive/code-review-reality-server-sso-reqwest-no-timeout.md`
- Mark the matching `backlog.json` row as `status: "done"`
