# code-review-reality-server-sso-introspect-negcache-poison

**Vector:** bug
**Score:** 3
**Source:** Tier1d review 2026-08-14 (reality-server)
**Confidence:** high

## Hypothesis
`introspect_pm_token()` in `reality-server`'s SSO path conflates a **genuinely inactive PM token** with a **transient introspection failure** (5xx / 429 / network blip). On the cache-miss path (`backend/servers/reality-server/src/routes/sso.rs:743-762`), when the upstream PM `introspect` endpoint returns `!response.status().is_success()`, the code calls `state.token_cache.set(token, false, None, None)` *before* returning `Err`. This poisons the 60-second TTL cache: the very next call for that same still-valid token is served from cache and returns a **successful** `TokenIntrospectionResponse { active: false, .. }`, so downstream callers see "token inactive" instead of "introspection unavailable". Smallest fix: only cache `active: false` when we have an authoritative body from the upstream (`response.status().is_success() && parsed.active == false`), never on a transport/HTTP-error path.

## Evidence
- `backend/servers/reality-server/src/routes/sso.rs:743-762` — `introspect_pm_token()` cache-miss branch: on `!response.status().is_success()` it still executes `state.token_cache.set(token, false, None, None)` (line 758, comment "Cache inactive tokens too (to prevent repeated failed validations)"), then returns `Err`.
- `backend/servers/reality-server/src/routes/sso.rs:730-739` — cache-hit branch on the follow-up call: returns `Ok(TokenIntrospectionResponse { active: false, .. })` from the poisoned cache entry, no distinguishing `Err` this time.
- Cache TTL is 60s (`state.token_cache` config); a single upstream 5xx therefore blackholes a valid PM session for up to 60s from this server's perspective.

## Files
- `backend/servers/reality-server/src/routes/sso.rs:743`
- `backend/servers/reality-server/src/routes/sso.rs:755`
- `backend/servers/reality-server/src/routes/sso.rs:758`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

**Execution mode (auto-derived from the ticks):**

Mode: cloud-ok

## Repro steps
1. Stand up reality-server pointed at a stub PM introspect endpoint that returns `503` for the first call and `200 {"active": true, …}` for the second call.
2. Issue two SSO validation requests with the same PM token, spaced under 60 s.
3. Expected: the second call re-introspects (or returns `Err(TransportError)`); actual: the second call returns `Ok({"active": false, …})` from the poisoned cache — user is signed out despite a valid session.

## Suggested approach
1. In `backend/servers/reality-server/src/routes/sso.rs`, restructure the cache-miss branch of `introspect_pm_token()` so `token_cache.set(..., false, ...)` runs **only** after we have a successful HTTP response AND the parsed body has `active == false`.
2. On the `!response.status().is_success()` path: log at `warn!` with the status + token-prefix, return `Err` immediately, **do not touch the cache**.
3. On a body-parse failure (`response.json::<TokenIntrospectionResponse>().await` returns `Err`): treat as transient — return `Err`, do not cache.
4. Only after `let parsed = …?; parsed.active == false` → `token_cache.set(token, false, None, None)` (preserve the "prevent repeated failed validations" intent, but only for authoritative "no").
5. Positive path (`parsed.active == true`) already caches correctly — leave it.
6. Sanity-check the `refresh_token` and any other cache-populating callers in the same module for the same anti-pattern; fix in place if the shape matches.
7. Add a one-line comment above the write explaining "authoritative-inactive only — never cache transport failures" to prevent regression.

## Alternatives considered
- **Shorten the negative-cache TTL to 5 s** — rejected because it only reduces the blast-radius window; a valid token still gets marked inactive during the outage and users still get spurious sign-outs. Correctness beats latency here.
- **Add a second retry inside `introspect_pm_token()` before caching** — rejected because the cache write is the bug, not the retry policy; retrying and *still* caching the failure on double-503 hides the same defect for one more round-trip.

## Root-cause trace
1. Symptom: valid PM session is treated as inactive by reality-server for up to 60s after any transient upstream introspect failure.
2. ← immediate cause at `backend/servers/reality-server/src/routes/sso.rs:758` — cache write happens on the `!is_success()` branch.
3. ← upstream cause: the "cache inactive tokens too" optimisation was added without distinguishing "authoritatively inactive body" from "no body / non-2xx".
4. Origin: the comment at line 757 was written intending "authoritative inactive body", but the surrounding branch guard evolved and the write was left in place — code and comment drifted.

## Test plan
- [ ] Unit test in `backend/servers/reality-server/src/routes/sso.rs` (or its `#[cfg(test)] mod tests`) using a mocked HTTP layer that returns 503 then 200/active, asserting the second call re-introspects.
- [ ] Regression test: mock returns `200 {"active": false, …}` twice — verify the second call is served from cache (existing positive path must not regress).
- [ ] `cd backend && cargo test -p reality-server routes::sso::tests`

## Out of scope
- Rewriting `TokenCache` semantics or the shared cache abstraction.
- Adding a circuit breaker around upstream PM introspect.
- Widening the introspect API contract with PM.

## After-merge
- Move this file to `plans/_archive/code-review-reality-server-sso-introspect-negcache-poison.md`
- Mark the matching `backlog.json` row as `status: "done"`
