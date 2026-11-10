# code-review-reality-server-price-map-cache-unbounded

**Vector:** bug
**Score:** 3
**Source:** rotating-expert-review 2026-10-15 (dispatcher Tier-1d reality-server) · hotspot in `backend/servers/reality-server/src/routes/price_map.rs`
**Confidence:** high

## Hypothesis
The `PRICE_MAP_CACHE` process-global `HashMap<String, CacheEntry>` on the public, unauthenticated `GET /api/v1/price-map` handler is reachable with attacker-controlled `city` and `property_type` query parameters that land verbatim in the cache key, and the map has no size cap — only a 1 h TTL that is reclaimed on the write path. An attacker looping `?city=<unique-random>` inserts a brand-new entry on every distinct value, so distinct keys accumulate inside the TTL window and heap grows without bound → OOM DoS. The smallest safe fix is a bounded insert (LRU or size-capped eviction) mirroring the `INQUIRY_RATE_LIMIT_MAX_IPS = 100_000` bound already used in `state.rs` for the sibling anonymous-abuse surface, plus a unit test that asserts the cache size stays ≤ `MAX_ENTRIES` after N distinct-city inserts within the TTL.

## Evidence
- `backend/servers/reality-server/src/routes/price_map.rs:32-38` — `PRICE_MAP_CACHE: OnceLock<Arc<RwLock<HashMap<String, CacheEntry>>>>` with no cap and only TTL for eviction.
- `backend/servers/reality-server/src/routes/price_map.rs:97-100` — `get_price_map(State, Query<PriceMapQuery>)` is public: no `RequestPrincipal` / no auth extractor.
- `backend/servers/reality-server/src/routes/price_map.rs:125-131` — `cache_key = format!("{}:{}:{}:{}", mode, time_window_str, query.property_type.unwrap_or(""), query.city.unwrap_or(""))`; attacker-controlled `city` and `property_type` land verbatim.
- `backend/servers/reality-server/src/routes/price_map.rs:227-234` — write path unconditionally `cache.insert(cache_key, CacheEntry { … expires_at: now + CACHE_TTL })`; `cache.retain(|_, v| Instant::now() < v.expires_at)` at `:229` only drops entries that have *already* passed `CACHE_TTL = 3600s` (`:41`).
- `backend/servers/reality-server/src/state.rs:910-915` — precedent: `INQUIRY_RATE_LIMIT_MAX_IPS = 100_000` built with `with_default_bounded`, documented as *"Bound memory against an attacker rotating source IPs … the map never exceeds this many entries."*

## Files
- `backend/servers/reality-server/src/routes/price_map.rs`
- `backend/servers/reality-server/src/state.rs:910`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [x] C2 — Seed data
- [x] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

**Execution mode (auto-derived from the ticks):** no C4/C5 ticks.

Mode: cloud-ok

## Repro steps
1. From a dev stack (`stack up pm-local`), confirm the public endpoint works: `curl -s "http://localhost:8081/api/v1/price-map?city=bratislava&property_type=apartment&mode=sell&time_window=12_months"` returns 200.
2. Loop `curl -s "http://localhost:8081/api/v1/price-map?city=$(uuidgen)&property_type=apartment&mode=sell&time_window=12_months" >/dev/null` from one terminal.
3. Observe `reality-server` RSS (`ps -o rss= -p $(pgrep reality-server)` or the stack's Prometheus `process_resident_memory_bytes`) climb unbounded over the 1 h TTL; `cache.read().await.len()` grows linearly with distinct `city` values.
4. Expected after fix: cache size plateaus at `MAX_ENTRIES` (bounded LRU evicts the oldest when the cap is hit); RSS stabilises.

## Suggested approach
1. Add `MAX_PRICE_MAP_CACHE_ENTRIES` constant beside `CACHE_TTL` at `price_map.rs:41`; pick a conservative value (suggested: `10_000`, well below `INQUIRY_RATE_LIMIT_MAX_IPS = 100_000` since each `CacheEntry` carries a `Vec<DistrictAggregate>` not a bare counter).
2. Replace the raw `HashMap` with a bounded structure: either (a) swap to a `linked-hash-map`/`lru` crate already in the workspace, or (b) implement an in-place `insert_bounded(key, value)` helper that, on cap, evicts the oldest-`inserted_at` entry (`CacheEntry` already carries `expires_at` — add `inserted_at: Instant` next to it or sort by `expires_at - CACHE_TTL`). Prefer (a) if a bounded-map crate is already a workspace dep; otherwise (b) is one function.
3. Call the bounded insert at `price_map.rs:227-234` in place of `cache.insert`.
4. Keep the existing TTL `retain` on the write path — the two eviction policies compose (TTL is correctness, LRU is memory bound).
5. Add a unit test under `backend/servers/reality-server/src/routes/price_map.rs` `#[cfg(test)] mod tests`: insert `MAX_PRICE_MAP_CACHE_ENTRIES + 10` entries, assert `cache.read().await.len() == MAX_PRICE_MAP_CACHE_ENTRIES` and that the oldest keys are the ones evicted.
6. Optional defence-in-depth (do NOT bundle unless trivial): tighten the key by rejecting `city` / `property_type` values that are not in the known allowlist from the `listings` table before they reach the key. Out of scope for this plan if it ripples beyond `price_map.rs`.

## Alternatives considered
- **Require auth on `GET /api/v1/price-map`** — rejected because the handler is intentionally public-market data; adding auth changes the product contract (anonymous visitors on reality-web consume this), and the goal is to bound memory, not to gate access.
- **Shorten `CACHE_TTL` to minutes** — rejected because the TTL is already a correctness constant (the backing query is expensive); shrinking it costs performance under legitimate traffic without actually bounding the per-TTL distinct-key accumulation. A size cap is the targeted fix.

## Root-cause trace
1. Symptom: unbounded heap growth / OOM on `reality-server` under a looped `GET /api/v1/price-map?city=<unique>` from an unauthenticated client.
2. ← `price_map.rs:227-234` unconditionally `cache.insert(cache_key, …)` with no size check.
3. ← `price_map.rs:125-131` builds the key from attacker-controlled `city` / `property_type` strings off the public handler.
4. ← `price_map.rs:32-38` the cache is declared as a plain `HashMap` with no size bound and only TTL reclamation.
5. Origin: initial introduction of the `price_map` route — the sibling `state.rs:910-915` inquiry-rate-limiter was created *with* a bound (precedent exists in the same codebase), so the latent gap is specific to this file.

## Test plan
- [ ] New unit test in `backend/servers/reality-server/src/routes/price_map.rs` `tests` module: `price_map_cache_bounded_under_distinct_key_flood` — insert `MAX+10` entries, assert `len == MAX`; assert the oldest-inserted entries are the ones gone.
- [ ] Regression scenario: concurrent-write test asserting the LRU eviction is thread-safe (two tasks each inserting `MAX/2 + 1` entries; final size `== MAX`).
- [ ] Local command: `cd backend && cargo test -p reality-server --lib routes::price_map::tests` (and the full `cargo test -p reality-server` before pushing).

## Out of scope
- Hardening the `city` / `property_type` allowlist (noted as optional in step 6 above).
- Rate-limiting `GET /api/v1/price-map` globally (separate concern — bounded cache is the DoS fix the finding calls out).
- Touching `listings.rs:700-706` / `price_map.rs:158-206` `sqlx::AssertSqlSafe` sites — that's the sibling finding `code-review-reality-server-assertsqlsafe-format-sql`, tracked separately.

## After-merge
- Move this file to `plans/_archive/code-review-reality-server-price-map-cache-unbounded.md`
- Mark the matching `backlog.json` row as `status: "done"`
