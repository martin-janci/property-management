# code-review-reality-server-sso-sessions-unbounded-dos

**Vector:** security
**Score:** 3
**Source:** Phase 1.5 rotating expert review 2026-07-31 (reality-server segment); files `backend/servers/reality-server/src/routes/sso.rs`, `backend/servers/reality-server/src/state.rs`
**Confidence:** medium

## Hypothesis
`AppState.sso_sessions` is an unbounded `Arc<Mutex<HashMap<String, PendingSsoSession>>>` that receives one entry per unauthenticated `/api/v1/sso/login` call and is only pruned when the matching `/api/v1/sso/callback` arrives. The 10-minute freshness check at `sso.rs:247` runs only inside the callback path — it filters stale reads, it does not evict abandoned entries — so any client that starts an SSO flow and never returns (user closes the tab, network drop, or a script deliberately flooding logins) leaves its PKCE verifier + redirect metadata in memory forever. Adding a bounded eviction policy (either a periodic reaper task or a bounded LRU / TTL map such as `moka::sync::Cache`) closes the memory-DoS surface while preserving the callback contract.

## Evidence
- `backend/servers/reality-server/src/routes/sso.rs:94` — `sso_login` handler inserts into `state.sso_sessions.lock().insert(...)` on every anonymous request; no rate limit or size guard visible at the route or state layer.
- `backend/servers/reality-server/src/routes/sso.rs:247` — the 10-minute expiry check is inline inside `sso_callback`; abandoned entries whose callback never fires are never evicted.
- `backend/servers/reality-server/src/state.rs:841` and `:896` — `sso_sessions: Arc<Mutex<HashMap<String, PendingSsoSession>>>` is constructed with `HashMap::new()`; the struct definition and constructor confirm there is no reaper `tokio::task::spawn`, no capacity cap, and no `retain`/`prune` call anywhere in the segment.
- Grep across `backend/servers/reality-server/src/` for `spawn|reaper|retain|prune` intersected with `sso|pending|reap` returned zero hits — confirming no existing eviction path guards this map (verified during Phase 3.5 adversarial pass).

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

**Execution mode (auto-derived from the ticks):**
Mode: cloud-ok

## Repro steps
1. Start reality-server locally (`cd backend && cargo run -p reality-server`).
2. In a shell, fire N=1000 sequential `curl -s -X POST http://localhost:8081/api/v1/sso/login -H 'content-type: application/json' -d '{"redirect_uri":"reality://sso","code_challenge":"abcdef1234567890"}'` calls without ever hitting `/api/v1/sso/callback`.
3. Inspect process RSS: expect it to grow monotonically by ~O(N × size_of::<PendingSsoSession>()). Expected: bounded; actual: unbounded — every entry stays until process death.

## Suggested approach
1. Introduce a bounded eviction strategy — the simplest is a periodic reaper `tokio::task::spawn` inside `AppState::new()` that every 60 s locks `sso_sessions`, then `retain(|_, v| now.duration_since(v.created_at) < SSO_PENDING_TTL)`. Reuse the 10-minute TTL constant already declared inline at `sso.rs:247`.
2. Alternative: swap `HashMap` for `moka::sync::Cache` with `time_to_live(Duration::from_secs(600))` + `max_capacity(10_000)` and drop the manual TTL check in `sso_callback`. This also caps peak memory even under sustained abuse.
3. Move the 10-minute TTL to a `const` (`sso.rs:20`-ish) so the reaper and the callback share one source of truth.
4. Add a `metrics::counter!("reality.sso.sessions.evicted")` from the reaper so ops has visibility.
5. Regression test — see *Test plan*.
6. If a similar unbounded map exists elsewhere in reality-server (e.g. `oauth_states`, `pending_sessions`), factor the reaper into a small helper so both call sites use the same pattern; otherwise leave the sso-specific reaper inline.
7. Update `docs/api/reality-server-README.md` (if present) to note the TTL / cap.

## Alternatives considered
- **Rate-limit `/api/v1/sso/login` per source IP via `tower_governor`** — rejected because the reality-server sits behind a Cloudflare / reverse proxy that already provides IP throttling, and rate-limits alone don't help against low-QPS long-tail abandonment (the fill-then-forget attacker uses one request per minute for a month).
- **Persist pending SSO sessions to Redis with a native `EXPIRE`** — rejected as premature: the reality-server otherwise has no Redis dependency, this would introduce a new failure mode (Redis outage → SSO down) for what an in-process TTL map handles just as safely.

## Root-cause trace
N/A — this is a `security` finding surfaced by static review, not a regression trace. The origin is architectural (unbounded map on a public unauthenticated endpoint), not a specific commit that regressed prior behavior.

## Test plan
- [ ] `backend/servers/reality-server/tests/sso_pending_reaper_test.rs` — new integration test: seed 3 entries with `created_at` older than TTL, advance clock (via `tokio::time::pause`/`advance`), tick the reaper, assert the map is empty.
- [ ] Regression: existing `sso_login` + `sso_callback` happy-path tests must still pass unchanged — the reaper must not race against an in-flight callback (add a test that inserts and immediately consumes to catch the race).
- [ ] Local run: `cd backend && cargo test -p reality-server sso_pending_reaper` — the new test must be red on `origin/dev` and green after the change (IG3).

## Out of scope
- Rewriting the whole SSO flow to use short-lived JWTs instead of an in-memory map — separate design decision, tracked outside this plan.
- Rate-limiting `/api/v1/sso/login` at the ingress layer — the operational surface belongs to platform-devops, not the reality-server codebase.
- Touching the auth-server `pending_sessions` reaper (a sibling handler in `api-server`) — that one already exists and is not the subject of this finding.

## After-merge
- Move this file to `plans/_archive/code-review-reality-server-sso-sessions-unbounded-dos.md`
- Mark the matching `backlog.json` row as `status: "done"`
