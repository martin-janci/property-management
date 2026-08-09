# code-review-reality-server-sso-oauth-helpers-harden

**Vector:** bug
**Score:** 4
**Source:** Rotating expert review 2026-08-08 (reality-server): 2 sibling signals — sso-rs-negcache-transient-fail, sso-rs-client-no-timeout-pool
**Confidence:** medium

## Hypothesis
`backend/servers/reality-server/src/routes/sso.rs` has two reliability defects in the OAuth helpers that speak to the PM introspection/OAuth endpoints. First, each helper (`exchange_code_for_tokens`, `get_user_info`, `introspect_pm_token`) constructs a fresh `reqwest::Client::new()` per invocation with no request timeout — a hung or slow PM server blocks the awaiting request indefinitely and risks worker/connection exhaustion. Second, `introspect_pm_token` caches `active=false` for the full 60 s TTL on ANY non-success PM response, so a transient 5xx during a PM deploy causes 60 s of false-negative introspections for valid tokens. The fix is to wire a shared, timeout-bounded reqwest client (client-level or per-request `.timeout(...)`) and narrow the negative-cache condition to responses that unambiguously mean 'inactive' (e.g. HTTP 200 with `active=false`, or 401), never on 5xx / connect / read-timeouts.

## Evidence
- `backend/servers/reality-server/src/routes/sso.rs:753-763` — `introspect_pm_token()` writes `state.token_cache.set(token, false, None, None)` for the full TTL on `!response.status().is_success()`. Transient PM outages permanently invalidate active tokens for 60 s.
- `backend/servers/reality-server/src/routes/sso.rs:682` (`exchange_code_for_tokens`), `:706` (`get_user_info`), `:743` (`introspect_pm_token`) — each function calls `reqwest::Client::new()` inline. `reqwest::Client::new()` has no default request timeout; a hung upstream blocks the awaiter indefinitely.
- Sibling-issue linkage: both signals target the same three helpers and the same file, so a single PR shared-clientifies + timeouts + narrows the neg-cache in one pass.

## Files
- `backend/servers/reality-server/src/routes/sso.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (bug fix, timeout + cache reasoning)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode (auto-derived from the ticks):** neither C4 nor C5 is ticked.

Mode: cloud-ok

## Repro steps
1. Stand up a fake PM introspection server that returns HTTP 503 once, then HTTP 200 with `active: true` on every subsequent request.
2. Call `introspect_pm_token(&state, token)`. Today: the first call sees 503 → `active=false` cached for 60 s; the second (and every subsequent call within 60 s) returns `active=false` from cache even though PM is healthy again.
3. Expected after the change: the 5xx response does NOT poison the cache; the second call re-hits PM, sees 200 + `active=true`, and returns `active=true`.
4. Separately: stand up a fake server that accepts a TCP connection but never writes a response. Call `exchange_code_for_tokens`. Today: the request hangs until the tokio runtime or the client abort. Expected after: the helper returns a timeout error within a bounded configured window (e.g. 5–10 s).

## Suggested approach
1. Introduce a shared `HttpClient` (either on `AppState` or a module-static `OnceCell<reqwest::Client>`) built with `reqwest::ClientBuilder::new().timeout(Duration::from_secs(<config>)).connect_timeout(Duration::from_secs(<config>)).build()`. Reuse it from all three helpers.
2. Add config keys for the timeout(s) (default e.g. 8 s request timeout, 3 s connect timeout) with an env override; document in `docs/api/README.md` if that's where SSO timeouts belong.
3. In `introspect_pm_token`, narrow the negative-cache write: only cache `active=false` when the PM response is HTTP 200 with a body parsed as `{ active: false }`, or an explicit 401. Log-and-return-error on 5xx / connect/read timeouts without touching the cache.
4. Ensure `exchange_code_for_tokens` and `get_user_info` propagate a timeout error as a clean HTTP 502/504 (whichever matches existing patterns) instead of a generic 500 — mirror how sibling reality-server upstream calls surface upstream failures.
5. Add regression tests using `wiremock` (already in reality-server test-deps? — if not, use `httpmock` or hand-rolled hyper stub) to cover: (a) 5xx does not poison the cache, (b) hung upstream returns Err within the timeout budget, (c) 200 `{active:false}` still caches negative as before.

## Alternatives considered
- **Only add a timeout, leave the negative-cache alone** — rejected because a 60-s cache of `active=false` on 5xx is the more user-visible failure mode (users are logged out for a minute even after PM recovers). Both defects share the same file/callers and adjacent lines; splitting them halves the review payoff.
- **Circuit-breaker instead of narrowed cache condition** — rejected as over-engineering for a single upstream. A narrow "only cache on unambiguous inactive" rule is O(3 lines) and doesn't add a new dependency; a circuit-breaker library needs config, metrics wiring, and its own tests. Revisit if PM introspection latency becomes a recurring incident.

## Root-cause trace
1. Symptom: users signed into reality-web get logged out for ~60 s any time PM introspection blips (503 during a deploy).
2. ← Immediate cause at `sso.rs:753-763` — `!response.status().is_success()` branch calls `state.token_cache.set(token, false, None, None)`, conflating transient failure with inactive-token.
3. ← Upstream cause at `sso.rs:682, :706, :743` — helpers construct a fresh `reqwest::Client::new()` with no timeout, so long PM latency also blocks the caller instead of surfacing a fast timeout error.
4. Origin: initial SSO integration (commit history via `git log --oneline -- backend/servers/reality-server/src/routes/sso.rs` before the introspection cache landed); the negative-cache added the poisoning behavior when the introspection TTL was widened to 60 s.

## Test plan
- [ ] `backend/servers/reality-server/tests/sso_introspection_cache.rs` (new) — PM 5xx does NOT cache `active=false`; subsequent call re-hits PM and gets the fresh answer.
- [ ] `backend/servers/reality-server/tests/sso_client_timeout.rs` (new) — hung upstream returns Err within the configured request-timeout budget (allow ±500 ms slack).
- [ ] `backend/servers/reality-server/tests/sso_introspection_cache.rs` — 200 `{active:false}` still caches for the TTL (regression on existing behavior).
- [ ] `cargo test -p reality-server` (full crate) to check nothing else consumed the per-call `Client::new()`.

## Out of scope
- Adding a metrics counter for PM upstream failures (nice-to-have, separate signal).
- Rewriting the token cache TTL policy (constant vs. adaptive).
- Any changes to PM's introspection endpoint or the token-cache implementation itself.

## After-merge
- Move this file to `plans/_archive/code-review-reality-server-sso-oauth-helpers-harden.md`
- Mark the matching `backlog.json` row as `status: "done"`
