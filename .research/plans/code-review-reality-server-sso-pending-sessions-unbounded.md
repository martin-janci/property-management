# code-review-reality-server-sso-pending-sessions-unbounded

**Vector:** bug
**Score:** 3
**Source:** segment=reality-server rotating review (2026-08-28)
**Confidence:** high

## Hypothesis
`sso_login` (`reality-server/src/routes/sso.rs:94`) inserts a `PendingSsoSession` into `AppState.sso_sessions: Mutex<HashMap<Uuid, PendingSsoSession>>` on every anonymous request. Entries are removed only on a successful `sso_callback`; the 10-minute expiry is checked lazily on read and never swept. There is no background evictor (contrast: `MobileTokenStore` at `state.rs:689` performs an amortized sweep). An attacker looping `GET /api/v1/sso/login` from unauthenticated clients grows the HashMap without bound — heap pressure plus, because it lives behind an async `Mutex`, contention that serializes every SSO request behind the attacker's writes. Landing a bounded map + periodic sweep fixes both the memory-DoS and the lock-contention lens.

## Evidence
- `backend/servers/reality-server/src/routes/sso.rs:94` — `state.sso_sessions.lock().await.insert(session_id, PendingSsoSession { ... });` — the only unbounded insert path
- `backend/servers/reality-server/src/state.rs:855` — `pub sso_sessions: Arc<Mutex<HashMap<Uuid, PendingSsoSession>>>` field declaration
- `backend/servers/reality-server/src/state.rs:991` — initialization: `Arc::new(Mutex::new(HashMap::new()))` — no cap, no sweeper spawned
- `backend/servers/reality-server/src/state.rs:689` — `MobileTokenStore` amortized-sweep template (call this out as the shape to mirror)
- Public unauth surface: `sso_login` route is registered without an auth extractor

## Files
- `backend/servers/reality-server/src/routes/sso.rs`
- `backend/servers/reality-server/src/state.rs`

## Dependencies
- (none — self-contained; independent of the timeout plan even though both touch `sso.rs`)

## Required capabilities
- [x] C1 — Systematic debugging (bug vector)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode (auto-derived from the ticks):**
Mode: cloud-ok

## Repro steps
1. Boot `reality-server` locally with an in-process HashMap-backed `sso_sessions` (default config).
2. Run a small loop: `for i in $(seq 1 20000); do curl -sS http://localhost:8081/api/v1/sso/login >/dev/null; done`.
3. Inspect `AppState.sso_sessions` size (via a `#[cfg(test)]` accessor, or a `/debug/sso_sessions_len` gated by `#[cfg(debug_assertions)]`). Expected after fix: size capped at ~10k with the oldest entries pruned. Observed before fix: size grows to 20000 with no eviction.

## Suggested approach
1. Add two config values on `AppState` (or a nested `SsoConfig`): `sso_pending_max_entries: usize` (default 10_000) and `sso_pending_sweep_interval: Duration` (default 60 s). Wire them through `AppState::new(...)`.
2. On `AppState::new(...)`, spawn a `tokio::spawn` background task that every `sso_pending_sweep_interval` acquires the mutex, drains entries whose `created_at + 10min < now`, and releases the lock. Keep the critical section tight — collect keys to remove first, then `retain`.
3. In `sso_login` (`sso.rs:94`) insert path, before the `insert(...)` call: `if map.len() >= max { drop(map); return Err(SsoError::TooManyPendingSessions); }` — surface a 429 to the caller instead of unbounded growth.
4. Return a `503 Service Unavailable` (or `429 Too Many Requests`) at the handler layer when the cap is hit; add the mapping in `SsoError -> IntoResponse`.
5. Optional: swap `Mutex<HashMap>` for `DashMap<Uuid, PendingSsoSession>` to remove global lock contention on the sweeper vs writer paths. If DashMap is not already a workspace dep, prefer the simpler mutex + sweeper first.
6. Add a metric counter (`sso_pending_sessions_gauge`, `sso_pending_sessions_evicted_total`) if `metrics` crate is already wired; otherwise defer.
7. Update the existing `sso_callback` cleanup path to be a no-op when the session was already swept (currently it silently `remove(...)` — leave the semantics as-is).

## Alternatives considered
- **Move `sso_sessions` to Redis with TTL** — rejected for scope (reality-server does not yet take a Redis dependency for this path; the in-process bound + sweeper closes the DoS at a fraction of the change surface).
- **Rely on the lazy-expiry-on-read check alone** — rejected because the lazy check only fires when a matching `sso_callback` arrives; the attacker path never triggers reads for their fake session ids, so entries live until process restart.

## Root-cause trace
1. Symptom: `AppState.sso_sessions` HashMap grows to millions of entries under repeated anonymous `sso_login` load.
2. ← `sso_login` inserts unconditionally at `sso.rs:94` with no cap check.
3. ← `sso_sessions` is a `Mutex<HashMap>` with no background evictor — expiry checked lazily inside `sso_callback` only.
4. Origin: SSO handlers copied a pattern from `state.rs:689` (`MobileTokenStore`) but omitted the amortized sweep half of that template.

## Test plan
- [ ] Unit test in `state.rs` sso mod tests: insert `N > max` entries, assert `insert` fails with `SsoError::TooManyPendingSessions` after `max` and that the map size is exactly `max`.
- [ ] Unit test for the sweeper: insert 10 entries with `created_at = now - 11min`, tick the sweeper interval, assert map is empty and the eviction counter equals 10.
- [ ] Integration: `curl` `/api/v1/sso/login` 15_000 times against a running server with cap=10_000, assert HTTP 429 after ~10_000th and that the server stays responsive to a concurrent `/api/v1/health`.
- [ ] Local run: `cargo test -p reality-server sso_pending_sessions_bounded`.

## Out of scope
- Rewriting session storage to Redis / Postgres — see Alternatives.
- Changing the `sso_callback` cleanup semantics.
- Adding CSRF/PKCE hardening on top of the state parameter (separate story if wanted).

## After-merge
- Move this file to `plans/_archive/code-review-reality-server-sso-pending-sessions-unbounded.md`
- Mark the matching `backlog.json` row as `status: "done"`
