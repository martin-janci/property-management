# code-review-reality-server-sso-pending-sessions-unbounded

**Vector:** security
**Score:** 3
**Source:** rotating-expert-review reality-server 2026-08-10 (Phase 1.5)
**Confidence:** high

## Hypothesis
`GET /api/v1/sso/login` inserts a `PendingSsoSession` into a plain `HashMap` on every hit but the only removal path is `sso_callback` for entries the caller chooses to redeem. An unauthenticated attacker can flood `/sso/login` and never follow the callback, growing the map without bound — a memory-DoS on `reality-server`. The sibling in-memory store `SsoTokenService::create_mobile_token` already fixes the same class of bug via `tokens.retain(|_, t| t.expires_at >= now)` on insert (issue #820). Mirror that pattern: sweep expired entries on insert *and* enforce a hard size cap.

## Evidence
- `backend/servers/reality-server/src/routes/sso.rs:94-101` — `sso_login` calls `state.sso_sessions.lock().await.insert(session_id.clone(), PendingSsoSession { …, created_at: Utc::now() })` unconditionally.
- `backend/servers/reality-server/src/routes/sso.rs:234-252` — `sso_callback` is the only writer that removes; the 10-minute expiry gate at line 247 only runs when the caller returns.
- `backend/servers/reality-server/src/state.rs:855` — `pub sso_sessions: Arc<Mutex<HashMap<String, PendingSsoSession>>>`; `state.rs:991` — constructed empty at boot, no sweep task spawned, no bounded map.
- `backend/servers/reality-server/src/state.rs:461` — `SsoTokenService::create_mobile_token` demonstrates the fix: `tokens.retain(|_, t| t.expires_at >= now);` before insert. Same class of bug is unfixed here.

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
1. Boot `reality-server` locally against a real DB.
2. In a tight loop: `for i in $(seq 1 100000); do curl -s "http://localhost:8081/api/v1/sso/login?redirect_uri=https%3A%2F%2Fexample.com%2Fdone" -o /dev/null; done`
3. Inspect process RSS or hit a debug endpoint that reports `sso_sessions.lock().await.len()`.
4. Expected: map length capped (e.g. ≤ MAX_PENDING_SSO_SESSIONS) with older entries evicted. Actual: length grows to 100k+ with proportional heap use.

## Suggested approach
1. In `routes/sso.rs::sso_login`, before the `insert`, acquire the lock once and (a) call `.retain(|_, s| Utc::now().signed_duration_since(s.created_at) < Duration::minutes(10))` to drop stale entries (mirror `state.rs:461`), then (b) enforce a `MAX_PENDING_SSO_SESSIONS` cap by evicting the oldest by `created_at` when the map is at capacity.
2. Extract the sweep+cap into a small helper (`insert_bounded`) on the `sso_sessions` guard for symmetry with the existing token-retain pattern.
3. Choose the cap from env (`SSO_PENDING_SESSIONS_MAX`, default 10_000). Wire it through `AppState` construction in `state.rs` alongside the existing SSO knobs.
4. Add a debug `tracing::warn!` when the cap-driven eviction path fires so operators see the pressure.
5. Do NOT introduce a background sweeper — inline sweep-on-insert matches the `create_mobile_token` pattern and keeps the change to one file + one struct field.
6. Preserve behaviour on the happy path: an entry inserted and redeemed within 10 minutes must still be found by `sso_callback`.

## Alternatives considered
- **Background sweep task (`tokio::spawn` every 60s)** — rejected because it adds a task lifetime to manage, is easy to lose on restart / process fork, and is not the shape used by the sibling `create_mobile_token` fix; inline sweep-on-insert is the established pattern here.
- **Rely on the 10-min expiry check in `sso_callback` alone** — rejected because callback is caller-controlled; the abandoned-callback path is *the* attack vector.

## Root-cause trace
1. Symptom: `reality-server` RSS grows without bound after sustained anonymous `/sso/login` traffic; `sso_sessions` map never shrinks.
2. ← `routes/sso.rs:94-101` — every login inserts, no sweep, no cap.
3. ← `state.rs:991` — `sso_sessions` initialised as an unbounded `HashMap` with no accompanying sweeper or bounded-map wrapper.
4. Origin: latent since `sso_sessions` was introduced; the `SsoTokenService` fix (issue #820) landed the retain pattern for a sibling but did not audit `sso_sessions`.

## Test plan
- [ ] Add a Rust unit test in `backend/servers/reality-server/src/routes/sso.rs` (or a new `sso_tests` module) that inserts 20_001 fake `PendingSsoSession` entries via the same code path and asserts the guarded map length ≤ cap (10_000 default) and that at least one warn was emitted.
- [ ] Add a companion test that inserts an expired entry (`created_at = now - 11min`), inserts a fresh entry, and asserts the expired one is gone (mirror the existing `create_mobile_token` retain test if any; else pattern-match on `tokens.retain` sibling).
- [ ] Verify `sso_callback` still finds a fresh entry inserted then redeemed within 10 minutes (happy path regression).
- [ ] Run: `cargo test -p reality-server --lib sso`

## Out of scope
- Rewriting `PendingSsoSession` storage as Redis / DB-backed — a bounded in-memory map matches the existing pattern and blast radius.
- Rate-limiting `/sso/login` — orthogonal (a separate signal); the map bound is defense-in-depth even with a limiter.
- Auditing every other `Arc<Mutex<HashMap<…>>>` in `state.rs` — do them one at a time under their own signals.

## After-merge
- Move this file to `plans/_archive/code-review-reality-server-sso-pending-sessions-unbounded.md`
- Mark `backlog.json` row `code-review-reality-server-sso-pending-sessions-unbounded` as `status: "done"`
