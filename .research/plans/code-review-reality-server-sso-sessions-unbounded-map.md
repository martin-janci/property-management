# code-review-reality-server-sso-sessions-unbounded-map

**Vector:** security
**Score:** 3
**Source:** review-2026-07-31 (reality-server segment, pm-backend expert)
**Confidence:** high

## Hypothesis
`AppState.sso_sessions: Arc<Mutex<HashMap<String, PendingSsoSession>>>` (reality-server) is the PKCE-flow pending-session store. It is inserted into on every unauthenticated `GET /api/v1/sso/login`, and only removed on a successful matching `/sso/callback`. Nothing ever sweeps entries for their 10-minute expiry, so any login started and abandoned (user closed the tab, no callback, callback with a wrong `state`) leaks a `PendingSsoSession` forever. An unauthenticated attacker can flood `/sso/login` and grow the mutex-guarded `HashMap` without bound — memory-exhaustion DoS plus global-mutex contention on every subsequent SSO login. This is the exact leak pattern that was already fixed for the sibling `SsoTokenService` mobile-token map and for `TokenValidationCache`; `sso_sessions` was missed.

## Evidence
- `backend/servers/reality-server/src/state.rs:841` — `pub sso_sessions: Arc<Mutex<HashMap<String, PendingSsoSession>>>` field, no sweep task, no cap.
- `backend/servers/reality-server/src/routes/sso.rs:94` — `state.sso_sessions.lock().await.insert(session_id.clone(), PendingSsoSession { code_verifier, redirect_uri, created_at })` on every unauth `GET /api/v1/sso/login`.
- `backend/servers/reality-server/src/routes/sso.rs:235` — the only `.remove(&session_id)` path (successful `/sso/callback` with matching state).
- `backend/servers/reality-server/src/state.rs:453` — `SsoTokenService::mint` already runs the amortized eviction (`tokens.retain(|_, t| t.expires_at >= now)`); this is the pattern to mirror.
- Attack shape: `for i in $(seq 1 1e6); do curl -s https://.../api/v1/sso/login >/dev/null; done` → unbounded RSS growth on reality-server, no auth needed.

## Files
- `backend/servers/reality-server/src/state.rs:841`
- `backend/servers/reality-server/src/routes/sso.rs:94`

## Dependencies
<!-- none -->

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
1. `stack up pm-local` and hit the reality-server on port 8081.
2. Loop `curl -s -X GET 'http://localhost:8081/api/v1/sso/login?redirect_uri=http://x' >/dev/null` 100_000 times (no auth header needed — the route is unauthenticated).
3. Observe `AppState.sso_sessions` `HashMap` length via a debug endpoint or `perf`; expected: bounded ≤ some sweep cap; actual: grows monotonically to 100_000+ entries with the mutex hot on every subsequent SSO login.

## Suggested approach
1. In `routes/sso.rs:94`, immediately before the `.insert(...)`, run an amortized sweep: `sessions.retain(|_, s| now - s.created_at < Duration::minutes(10));` (mirror `SsoTokenService::mint` at `state.rs:453-460`).
2. Add a hard cap: if `sessions.len() > SSO_SESSION_MAX` (e.g. 10_000) after the sweep, drop the oldest N by `created_at` — this bounds RSS even under active abuse where every session is still within the 10-minute TTL.
3. Extract the sweep into a small helper on `AppState` (`prune_sso_sessions(&self)`) so both the on-insert amortized call and a future periodic tokio task can share it — no duplication.
4. Emit a `sso.session_pruned` metric (or `tracing::warn!` on the cap path) so we can see abuse in logs.
5. Add a unit test in `state.rs` that inserts a session with a stale `created_at` and asserts the next `prune` drops it; add an integration test in `routes/sso.rs` that fills past `SSO_SESSION_MAX` and asserts the map size stays bounded.

## Alternatives considered
- **Periodic tokio background task** — rejected because it requires plumbing an owned handle into `AppState::new`, complicates shutdown, and the amortized-on-insert path (proven by `SsoTokenService`) is O(existing map size) per login which is trivially bounded once the cap is in place. Amortized wins on simplicity for the same asymptotic behaviour.
- **Redis-backed session store** — rejected because reality-server already has an in-process `Arc<Mutex<HashMap>>` for the sibling `SsoTokenService` for exactly this shape of short-lived state; adding a redis dependency for a 10-minute PKCE nonce is over-engineering when a bounded in-process map is enough. Revisit only if reality-server ever needs multi-instance session stickiness.

## Root-cause trace
1. Symptom: unauthenticated attacker floods `/api/v1/sso/login` → reality-server RSS grows unbounded.
2. ← `backend/servers/reality-server/src/routes/sso.rs:94` unconditionally inserts into `state.sso_sessions` without checking or sweeping.
3. ← `backend/servers/reality-server/src/state.rs:841` defines the map with no accompanying eviction (unlike `SsoTokenService` at `state.rs:453`).
4. Origin: the SSO login flow was added without applying the same eviction discipline the sibling `SsoTokenService` and `TokenValidationCache` already use; the pattern gap is the latent issue.

## Test plan
- [ ] Unit test in `backend/servers/reality-server/src/state.rs` (or a new `state_tests.rs` sibling) that inserts a session with `created_at = now - 11min`, calls `prune_sso_sessions`, asserts the entry is gone.
- [ ] Integration test in `backend/servers/reality-server/tests/sso_tests.rs` (create if absent) that hits `/api/v1/sso/login` `SSO_SESSION_MAX + 100` times in a single tokio runtime and asserts `state.sso_sessions.lock().await.len() <= SSO_SESSION_MAX`.
- [ ] `cargo test -p reality-server` locally (once C3 dev-stack is up) or `cargo test --manifest-path backend/Cargo.toml -p reality-server` from a workspace root.

## Out of scope
- Do NOT introduce a redis-backed store.
- Do NOT change the PKCE state validation on `/sso/callback` — the leak is in the pending-map lifecycle, not in the callback logic.
- Do NOT touch `SsoTokenService` or `TokenValidationCache` — they already have the fix; the plan is to *port* their pattern, not refactor them.

## After-merge
- Move this file to `plans/_archive/code-review-reality-server-sso-sessions-unbounded-map.md`
- Mark the matching `backlog.json` row as `status: "done"`
