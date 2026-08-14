# code-review-reality-server-sso-introspect-negcache-poison

**Vector:** bug
**Score:** 3
**Source:** rotating-expert-review (reality-server, 2026-08-14)
**Confidence:** high

## Hypothesis
reality-server `introspect_pm_token()` conflates "PM says the token is inactive" with "PM introspection HTTP call failed", then writes a negative cache entry with 60s TTL and returns Err. Within the TTL, the very next call for that same still-valid token gets a cached `TokenIntrospectionResponse { active: false }` — and downstream `sync_session()` treats that as authoritative and calls `invalidate_session()`, force-logging-out a validly-authenticated Reality Portal user for the remainder of the TTL. Fix: only cache `active=false` when PM returns a well-formed `{active:false}` body; on transport / non-success status, fail-closed on the current request via `Err` but do NOT persist any negative cache entry.

## Evidence
- `backend/servers/reality-server/src/routes/sso.rs:743-762` — the `!response.status().is_success()` branch writes `state.token_cache.set(token, false, None, None)` on transport/HTTP-status failure, then returns Err.
- `backend/servers/reality-server/src/routes/sso.rs:730-739` — subsequent cache-hit path returns `TokenIntrospectionResponse { active: false }` (no Err), losing the "introspection unavailable" signal.
- `backend/servers/reality-server/src/routes/sso.rs:1067-1101` — `sync_session()` calls `session_service.invalidate_session(portal_session)` when `!token_info.active` — direct blast radius on live sessions.

## Files
- `backend/servers/reality-server/src/routes/sso.rs:755`
- `backend/servers/reality-server/src/routes/sso.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. Configure reality-server against a PM introspect endpoint that responds with a 5xx (or times out) on the next request. Cheap fixture: point `pm_introspect_url` at a mock that returns 503.
2. Present a still-valid PM access token to any Reality-server route that reaches `introspect_pm_token()` (any protected inquiry / listing route works).
3. Observed (bug): first request → Err (introspection failed); second request within 60s → success with `active=false`; if wrapped in `sync_session()`, the portal session gets `invalidate_session()` called.
4. Expected (after fix): first request → Err; second request within 60s → Err again (no poisoned cache); portal session remains alive across a transient upstream blip.

## Suggested approach
1. In `backend/servers/reality-server/src/routes/sso.rs` (~L755) delete the `state.token_cache.set(token, false, None, None)` line inside the `!response.status().is_success()` branch. Keep the `return Err(...)`. Rationale: transport failures should not persist as "inactive" verdicts; the caller's own retry / backoff decides.
2. Also audit the `serde_json` decode branch below the status check — if it returns Err after a 200 with a malformed body, apply the same fix (no negative cache on decode failure).
3. Confirm the positive-cache path (line ~773 or wherever the `active=true` body writes the cache with real `sub`/`scope`/expiry) is unchanged.
4. Add a Rust unit test in `backend/servers/reality-server/src/routes/sso.rs` (or the nearest test module) using a `mockito`/`wiremock` fixture: (a) 503 response → Err; (b) same token immediately again → Err (proves no negative cache); (c) subsequently, 200 with `{active:true}` for the same token → success.
5. Optional: extend the test to cover `sync_session()` at line 1067-1101 so the invariant "transient introspection failure MUST NOT invalidate a live portal session" is pinned end-to-end.

## Alternatives considered
- **Cache the Err itself as a short-TTL "unavailable" sentinel** — rejected because it complicates the `TokenIntrospectionResponse` shape, requires a new variant, and the 60s window is dominated by resilient PM up-time; a plain "don't cache negatives on transport failure" is behaviour-preserving and one-line.
- **Keep the negative cache but shrink its TTL on error path only** — rejected because it does not change the correctness class (a valid session still gets invalidated); it just narrows the window, and the 60s TTL exists for legitimate `active=true` throughput not error smoothing.

## Root-cause trace
1. Symptom: authenticated Reality Portal user loses their session after a single transient PM introspect blip; recovery takes up to 60s (TTL).
2. ← `sync_session()` reads `token_info.active == false` and calls `invalidate_session(portal_session)` (`backend/servers/reality-server/src/routes/sso.rs:1082`).
3. ← Cached response returned by `introspect_pm_token()` says `active=false` (`backend/servers/reality-server/src/routes/sso.rs:730-739`, the cache-hit path).
4. ← Negative cache entry was written on the prior call's HTTP-failure branch: `token_cache.set(token, false, None, None)` (`backend/servers/reality-server/src/routes/sso.rs:758`).
5. Origin: the "Cache inactive tokens too (to prevent repeated failed validations)" comment conflates two failure modes — PM saying "no" vs PM being unreachable — a categorical mistake in the caching contract, not a race.

## Test plan
- [ ] `backend/servers/reality-server/src/routes/sso.rs` — new `#[tokio::test]` `test_introspect_transient_failure_does_not_poison_cache`: mock PM to return 503, then 503 again for the same token; assert both calls return Err and the cache does not contain the token.
- [ ] Regression scenario: mock PM 200 `{active:true}` after the two 503s; assert the third call returns Ok and populates the positive cache.
- [ ] End-to-end pin in `sync_session()`: assert `invalidate_session()` is NOT invoked when introspection fails transiently.
- [ ] Command: `cd backend && cargo test -p reality-server --tests sso`

## Out of scope
- Adding retries / circuit-breaker to the reqwest call itself (belongs to the sibling `code-review-reality-server-sso-reqwest-no-timeout` finding).
- Refactoring the cache abstraction (`token_cache`) shape or extending its API.
- Any change to positive-cache TTL or the `sub`/`scope`/expiry fields.

## After-merge
- Move this file to `plans/_archive/code-review-reality-server-sso-introspect-negcache-poison.md`
- Mark the matching `backlog.json` row as `status: "done"`
