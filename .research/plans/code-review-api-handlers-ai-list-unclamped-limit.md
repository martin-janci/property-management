# code-review-api-handlers-ai-list-unclamped-limit

**Vector:** security
**Score:** 2
**Source:** hotspot in `backend/servers/api-server/src/routes/ai/sessions.rs` (rotating-expert-review 2026-08-13, tier-1d api-handlers slice)
**Confidence:** high

## Hypothesis
The AI list endpoints forward a caller-controlled `?limit=` straight into a SQL `LIMIT $n` bind with **no upper-bound clamp**, unlike the rest of the codebase (`listing.rs:136` uses `.min(100)`, admin handlers use `.clamp(1,100)` / `.min(500)`). An authenticated caller can request `?limit=100000000` and force an unbounded result-set materialization on tenant-scoped tables (`ai_chat_sessions`, `ai_chat_messages`, `equipment_maintenance`, LLM request log) — DB scan plus Rust `Vec` allocation resource-exhaustion / self-scoped DoS. Smallest fix: clamp `limit` at each route site (or, better, inside the repo methods to mirror `listing.rs:136`) using the same `.clamp(1, 100)` shape as the existing house style.

## Evidence
- `backend/servers/api-server/src/routes/ai/sessions.rs:101` — `query.limit.unwrap_or(50)` forwarded straight to `list_user_sessions(...)`; same shape at `sessions.rs:205`, `:924`, `:997`.
- `backend/servers/api-server/src/routes/ai/equipment.rs:203`, `:297`, `:379`; `ai/llm.rs:1448`; `ai/voice.rs:308` — same missing-clamp pattern.
- `backend/crates/db/src/repositories/ai_chat.rs:154` — `LIMIT $2 OFFSET $3` binds `limit` directly (no `.min()`); `equipment.rs list_maintenance` (LIMIT $3 OFFSET $4, ~line 326) same shape.
- Contrast (verified 2026-08-13): `backend/crates/db/src/repositories/listing.rs:136` — `let limit = query.limit.unwrap_or(20).min(100);`. Admin routes clamp too — `routes/admin/users.rs:58 .min(100)`, `routes/admin/audit.rs:235 .min(500)`, `routes/admin/users_lifecycle.rs:273 .clamp(1,100)`. `reality-server` even has a dedicated `listings_pagination_clamp_tests.rs` suite for this.
- `.research/signals/2026-08-13-api-handlers-tier1d.json` — dispatcher-generated tier-1d signal `code-review-api-handlers-ai-list-unclamped-limit`, medium confidence; upgraded to high via direct verification 2026-08-13 in routine Phase 2.

## Files
- `backend/servers/api-server/src/routes/ai/sessions.rs:101`
- `backend/servers/api-server/src/routes/ai/equipment.rs:203`
- `backend/servers/api-server/src/routes/ai/llm.rs:1448`
- `backend/servers/api-server/src/routes/ai/voice.rs:308`
- `backend/crates/db/src/repositories/ai_chat.rs`
- `backend/crates/db/src/repositories/equipment.rs`
- `backend/crates/db/src/repositories/listing.rs`

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

Reason: clamp is a pure route/repo edit; no DB migration, no dev-stack needed, no browser or device.

## Repro steps
1. `cargo test -p api-server --test suite_5 -- --ignored ai_list_unclamped_limit_test 2>&1 | head -40` (before fix): the test binds a call with `?limit=1000000` on `GET /api/v1/ai/sessions` and asserts the returned `sessions.len() <= 100`; today it comes back with whatever the repo returns unbounded.
2. Expected after fix: the same list request with `?limit=1000000` returns at most 100 rows AND a request with `?limit=-1` returns HTTP 400 (validated) rather than reaching Postgres as a bind error → 500.

## Suggested approach
1. Add a shared helper (or inline `.clamp(1, 100)`) at each of the 9 route sites — `ai/sessions.rs:101, :205, :924, :997`, `ai/equipment.rs:203, :297, :379`, `ai/llm.rs:1448`, `ai/voice.rs:308` — mirroring `listing.rs:136`.
2. Also clamp inside the two repo methods (`ai_chat.rs:list_user_sessions`, `equipment.rs:list_maintenance`) as defense-in-depth so future callers can't re-open the hole.
3. Reject negative `?limit=` at the route with a clean 400 (the current path reaches Postgres and returns 500).
4. Add regression tests: `backend/servers/api-server/tests/suites/ai_list_pagination_clamp_tests.rs` — modeled on `reality-server/tests/suites/listings_pagination_clamp_tests.rs`. Cover: (T1) `?limit=1000000` → returned rows ≤ 100; (T2) `?limit=-1` → 400; (T3) `?limit=50` (in-range) → up to 50 rows; run against `/api/v1/ai/sessions` as the primary case, add a smoke over one equipment + one llm endpoint.
5. Wire the new suite into `tests/suite_5.rs` next to `market_pricing_happy_path_tests`.
6. Run `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p api-server --test suite_5 ai_list_pagination_clamp` inside `backend/`.
7. Optional (not blocking): consider a `PageLimit` newtype in `common` that wraps clamp+default and expose it via query extractors — but ship the direct clamp first; the newtype refactor is a separate followup vector.

## Alternatives considered
- **Do nothing / rely on caller trust** — rejected because the endpoints are user-facing and JWT auth alone does not defend against a legitimate user exhausting DB/CPU with a single `?limit=` value; the exact contrast (`listing.rs:136`) proves the codebase already treats this as a bug shape, not a feature.
- **Add a global middleware that rewrites `?limit` in the query string** — rejected because it would silently mutate an inbound HTTP parameter (surprising for API consumers reading response contracts) and would not fix the repo layer, leaving the defense-in-depth hole open for any future direct-repo caller.

## Root-cause trace
1. Symptom: `GET /api/v1/ai/sessions?limit=100000000` returns hundreds of MB and drives DB latency; no HTTP 400 or clamp.
2. ← Route handler at `backend/servers/api-server/src/routes/ai/sessions.rs:101` binds `query.limit.unwrap_or(50)` directly.
3. ← Repo method at `backend/crates/db/src/repositories/ai_chat.rs:154` binds `LIMIT $2 OFFSET $3` with no `.min()`.
4. Origin: the AI-endpoints family predates the pagination-clamp house style that `listing.rs:136` established (see `reality-server/tests/suites/listings_pagination_clamp_tests.rs`). No single commit — a whole-family miss.

## Test plan
- [ ] `backend/servers/api-server/tests/suites/ai_list_pagination_clamp_tests.rs` — new suite: `?limit=1000000` clamped to ≤100; `?limit=-1` returns 400; `?limit=50` returns ≤50.
- [ ] Regression on the negative-limit path — before the fix `?limit=-1` reaches Postgres and returns 500; after, 400.
- [ ] `cd backend && cargo test -p api-server --test suite_5 ai_list_pagination_clamp` — new tests fail on `main` (IG3) and pass after the clamp edits.

## Out of scope
- Introducing a shared `PageLimit` newtype (deferred to a followup — this plan ships direct `.clamp(1,100)` at each site plus repo-layer defense-in-depth).
- Auditing non-AI endpoints for the same pattern — a separate sweep can cover those; this plan is scoped to the AI + equipment surface named by the tier-1d signal.
- Bumping the default `unwrap_or(50)` — behavior-neutral change out of scope.

## After-merge
- Move this file to `plans/_archive/code-review-api-handlers-ai-list-unclamped-limit.md`
- Mark the matching `backlog.json` row (`code-review-api-handlers-ai-list-unclamped-limit`) as `status: "done"`
