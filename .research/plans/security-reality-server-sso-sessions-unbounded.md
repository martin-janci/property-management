# security-reality-server-sso-sessions-unbounded

**Vector:** security
**Score:** 2
**Source:** dispatcher Tier-1d reality-server review 2026-08-28 (signal `code-review-reality-server-sso-sessions-unbounded`)
**Confidence:** high

## Hypothesis
`reality-server`'s in-memory `sso_sessions` map (`Arc<Mutex<HashMap<String, PendingSsoSession>>>`) is initialised empty at startup with no upper bound and no eviction task, so every started-but-abandoned SSO flow leaves a `PendingSsoSession` in the map forever. An unauthenticated actor can enumerate `/sso/start` (or whatever endpoint mints entries) to grow the map without limit — a slow-motion memory exhaustion / DoS against the reality-server process, and a compliance issue for any deployment expecting bounded auth-state retention. The smallest safe change is a `HashMap` → `TTL-bounded LRU` with a periodic `retain(|_, v| v.created_at.elapsed() < TTL)` sweep (or per-insert opportunistic cleanup), plus a hard cap on entry count.

## Evidence
- `backend/servers/reality-server/src/state.rs:855` — `pub sso_sessions: Arc<Mutex<HashMap<String, PendingSsoSession>>>` — the raw `HashMap` type is the entire memory model; no bound, no `size_limit`, no `expires_at` field on `PendingSsoSession` that anything reads.
- `backend/servers/reality-server/src/state.rs:991` — `sso_sessions: Arc::new(Mutex::new(HashMap::new()))` — startup init empty with no accompanying `tokio::spawn` for a sweeper task.
- Grep across `backend/servers/reality-server/src/` for `sso_sessions` (`sso_sessions.lock`) shows only insert/get sites, never a `retain` / `remove_expired` / eviction — the map grows monotonically until the process restarts.
- Signals file: `.research/signals/2026-08-28-reality-server-tier1d.json`

## Files
- `backend/servers/reality-server/src/state.rs:855`
- `backend/servers/reality-server/src/state.rs:991`
- `backend/servers/reality-server/src/routes/sso.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (bug/security vector)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. Start `reality-server` locally (or in a unit-test harness that constructs `AppState`).
2. Insert 10_000 synthetic `PendingSsoSession` entries via the same lock the SSO start handler uses.
3. Expected: the map either rejects entries past the cap, or entries older than the configured TTL are evicted on subsequent locks / by the sweeper.
4. Actual (today): the map holds all 10_000 entries indefinitely; no eviction; a process-restart is the only way to reclaim.

## Suggested approach
1. Give `PendingSsoSession` an `inserted_at: std::time::Instant` field (small; internal type only).
2. Introduce constants near `state.rs:855`: `const SSO_SESSION_TTL: Duration = Duration::from_secs(10 * 60);` and `const SSO_SESSION_MAX: usize = 10_000;` (SSO flows shouldn't survive 10 minutes; the cap is a defence-in-depth ceiling).
3. Add a small `SsoSessionStore` newtype wrapping the `Mutex<HashMap<..>>` with three ops: `insert(k, v)` (rejects with a typed error when `len() >= MAX`, and opportunistically evicts expired entries first), `take(k)` (existing get + remove semantics), and `sweep_expired()` (retain by `inserted_at.elapsed() < TTL`). Replace the field type at `state.rs:855` and the init at `state.rs:991`.
4. In `AppState::new` / the reality-server startup hook, `tokio::spawn` a background task that calls `sweep_expired()` every 60 s until the runtime shuts down. Bound the wake with a `tokio::select!` on a shutdown channel so tests don't leak.
5. Update every call site (`routes/sso.rs` inserts + reads) to the new API; the compiler will name them via the newtype.
6. Add sqlx-free unit tests in `state.rs` (or a dedicated `sso_session_store_tests.rs`): (a) insert then take → returns entry, (b) insert past TTL then take → returns None, (c) insert past MAX → returns error, (d) sweep after TTL → map length shrinks.
7. Run `cargo test -p reality-server sso_session_store -- --nocapture` and `cargo clippy -p reality-server --all-targets -- -D warnings`.

## Alternatives considered
- **Move state to Redis** — rejected because the map is currently in-process only; introducing Redis for one struct adds a new deployment dependency and cross-worker eventual-consistency concerns for a 10-minute-TTL surface. Revisit if reality-server scales horizontally.
- **`moka` LRU crate** — rejected because the map is small (≤10k entries by the new cap), the sweep is a trivial `HashMap::retain`, and adding a dep for a 30-line policy is bloat.

## Root-cause trace
1. Symptom: `sso_sessions` map grows unbounded, memory-exhausting reality-server; abandoned SSO flows leak long-lived pending records with none of the promises around TTL.
2. ← `state.rs:855` — raw `HashMap` with no accompanying bound-or-evict policy.
3. ← `state.rs:991` — startup init has no companion `tokio::spawn` for a sweeper.
4. Origin: initial SSO-consumer scaffolding on reality-server (predates the churn window); the eviction TODO was left as a follow-up that never materialised. Surfaced by dispatcher Tier-1d review 2026-08-28.

## Test plan
- [ ] New unit tests (`backend/servers/reality-server/src/state.rs`, or a sibling file) exercising insert, take, TTL eviction, and MAX-cap rejection on the newtype.
- [ ] Regression: existing SSO happy-path tests continue to pass unchanged (the newtype's `take` is API-compatible with today's `HashMap::remove`).
- [ ] `cargo test -p reality-server sso_session -- --nocapture && cargo clippy -p reality-server --all-targets -- -D warnings`

## Out of scope
- Companion fix `code-review-reality-server-sso-reqwest-no-timeout` (sibling signal from the same Tier-1d review) — its own row remains at score 2 for the next promotion round.
- Moving pending-SSO state to Redis / cross-instance store.
- Adding metrics/telemetry counters for evictions (nice to have; not this PR).

## After-merge
- Move this file to `plans/_archive/security-reality-server-sso-sessions-unbounded.md`
- Mark the matching `backlog.json` row as `status: "done"`
