# code-review-reality-server-sso-token-cache-ignores-exp

**Vector:** security
**Score:** 2
**Source:** signal `code-review-reality-server-sso-token-cache-ignores-exp` (tier1d-dispatcher-generator, 2026-08-09) — verified by routine 2026-08-11
**Confidence:** high

## Hypothesis
`TokenValidationCache` stores every PM token-introspection result with a fixed 60-second TTL, ignoring the token's own `exp` and any revocation upstream. After a PM user logs out, has their session admin-revoked, or after the token's real expiry, reality-server keeps treating the token as `active=true` for up to 60 seconds. Fix: bound each cache entry's `expires_at` by `min(configured_ttl, token_exp - now)`, and invalidate the cache entry when reality-server observes a session-end signal on its own path.

## Evidence
- `backend/servers/reality-server/src/state.rs:735-748` — `TokenValidationCache::set()` unconditionally uses `expires_at: Instant::now() + Duration::from_secs(self.ttl_seconds)` — TTL is not conditioned on the token payload.
- `backend/servers/reality-server/src/state.rs:527-538` — `CachedTokenValidation { active, sub, scope, expires_at }` carries no expiry field for the token itself, only the cache-entry Instant.
- `backend/servers/reality-server/src/routes/sso.rs:646-652` — `TokenIntrospectionResponse { active, sub, client_id, scope }` — the introspection response type doesn't parse `exp` from the upstream OAuth introspection JSON, so it isn't available at cache-set time.
- `backend/servers/reality-server/src/routes/sso.rs:726-745` — `introspect_pm_token()` returns the cached `active` verbatim on a hit and short-circuits before the network call.
- `backend/servers/reality-server/src/state.rs:947` — the cache is instantiated with `TokenValidationCache::new(60, 10000)` — 60-second stale-auth window is the default and only value.

## Files
- `backend/servers/reality-server/src/routes/sso.rs`
- `backend/servers/reality-server/src/state.rs`

## Dependencies
<!-- none -->

## Required capabilities
- [x] C1 — Systematic debugging (bug/revert/risky-churn family — this is a security bug)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [x] C7 — Code-review reception (security touch — expect scrutiny)

**Execution mode (auto-derived):** `Mode: cloud-ok`

Mode: cloud-ok

## Repro steps
1. Start reality-server with the current SSO wiring (`stack up pm-local` or equivalent).
2. Exchange a PM login for a short-lived access token (mint via api-server `/oauth/token` with `expires_in ≤ 60`).
3. Call any reality-server route protected by `introspect_pm_token()` (e.g. an authenticated agency endpoint) — response 200.
4. Revoke the PM session on api-server (`/auth/logout` or admin revoke).
5. Immediately call the same reality-server route again with the SAME bearer — expected: 401. Actual: 200 for up to 60 s because the cached `active=true` short-circuits the introspection.
6. Add an integration test that mints a token with `expires_in=1`, waits 5 s, then hits the protected route with cache warm — expected 401, actual 200.

## Suggested approach
1. Extend `TokenIntrospectionResponse` (routes/sso.rs:646) to parse `exp` from the upstream OAuth introspection JSON: `pub exp: Option<i64>`. Guard against absence; upstream servers may omit it.
2. Extend `CachedTokenValidation` (state.rs:527) with `pub token_exp: Option<Instant>` to carry the token's own expiry translated to a monotonic instant.
3. Change `TokenValidationCache::set()` (state.rs:735-748) signature to accept `token_exp: Option<i64>` and compute `expires_at = min(now + ttl, token_exp_instant)`.
4. Update the caller in `introspect_pm_token()` (routes/sso.rs:766-773) to pass `result.exp` through.
5. Add a `invalidate_on_logout()` code path — reality-server's SSO logout handler (`sync_session` or equivalent) should call `state.token_cache.invalidate(&token)` for the exiting user's PM token when known.
6. Write an integration test at `backend/servers/reality-server/tests/suites/sso_token_cache_tests.rs`:
   - `test_cache_respects_token_exp` — mint token with `expires_in=1`, warm cache, sleep 2 s, second call must miss cache and re-introspect (which returns inactive).
   - `test_cache_invalidated_on_logout` — warm cache, call `invalidate()`, next call re-introspects.
7. Update `state.rs` unit tests around cache-metrics if any assertions depend on the previous fixed-TTL behaviour.

## Alternatives considered
- **Drop the introspection cache entirely** — rejected because the 60-second cache is intentional (Story 104.2) to reduce PM introspection load; a blanket removal would restore the throughput problem it was built to fix. Bounding by `min(ttl, token_exp)` gets both properties.
- **Rely on a shorter global TTL (e.g. 5 s)** — rejected because it doesn't address post-logout revocation (still up to 5 s stale) and only trades throughput for a smaller version of the same window. Binding to `token_exp` at cache-set time closes the natural-expiry class of stale-auth; the invalidate hook closes the admin-revocation class.

## Root-cause trace
1. Symptom: post-logout / post-expiry PM token accepted as `active=true` for up to 60 s by reality-server.
2. ← `introspect_pm_token()` short-circuits on cache hit (`routes/sso.rs:731-738`) and returns cached `active` verbatim.
3. ← `TokenValidationCache::set()` (`state.rs:735-748`) computed `expires_at` from `self.ttl_seconds` alone — the token's `exp` was never observed.
4. ← `TokenIntrospectionResponse` (`routes/sso.rs:646-652`) never parses `exp` — the upstream field is silently dropped even though PM's introspection endpoint returns it.
5. Origin: Story 104.2 introduction of the token-validation cache (Epic 104). The cache was framed as a throughput optimisation and inherited the health-check cache's fixed-TTL shape (state.rs:939-947), without re-considering that a TOKEN cache's validity is bounded by the token's own semantics.

## Test plan
- [ ] `backend/servers/reality-server/tests/suites/sso_token_cache_tests.rs::test_cache_respects_token_exp` — mint with `expires_in=1`, warm cache, sleep 2 s, second call re-introspects and denies. Fails on main today because `expires_at = now + 60s` regardless of token exp.
- [ ] `backend/servers/reality-server/tests/suites/sso_token_cache_tests.rs::test_cache_invalidated_on_logout` — warm cache, invalidate, next call re-introspects. Fails on main today because there is no logout-triggered invalidation.
- [ ] `cd backend && cargo test -p reality-server sso_token_cache` — the exact command to gate this locally.

## Out of scope
- Introducing distributed cache invalidation (Redis pub/sub) — a single-process invalidate is sufficient for the current single-instance reality-server deployment; multi-instance revocation is a separate epic.
- Introducing a revocation-list endpoint on api-server — the fix here is bounded to reality-server-side cache semantics; api-server already exposes `/auth/logout` as the trigger point that reality-server's SSO adapter can hook.
- Rekeying the cache to store `token_hash + tenant_id` — the current single-token key is correct once the TTL is bounded; tenant scoping is a separate correctness question not tied to this stale-auth window.

## After-merge
- Move this file to `plans/_archive/code-review-reality-server-sso-token-cache-ignores-exp.md`
- Mark the matching `backlog.json` row (`code-review-reality-server-sso-token-cache-ignores-exp`) as `status: "done"`
