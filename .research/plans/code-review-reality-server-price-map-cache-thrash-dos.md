# code-review-reality-server-price-map-cache-thrash-dos

**Vector:** security
**Score:** 3
**Source:** Issue #3132
**Confidence:** high

## Hypothesis
PR #3092 shipped `MAX_PRICE_MAP_CACHE_ENTRIES = 10_000` + `insert_bounded()` for `PRICE_MAP_CACHE`, which solved the OOM but did not neutralize the abuse surface on anonymous `GET /api/v1/price-map`. Two residual issues remain: (a) `insert_bounded` evicts via a full-keyset clone + `O(n log n)` `sort_by_key` under the global `RwLock` write guard, converting the OOM DoS into a CPU + lock-contention amplifier; (b) `city`/`property_type` are still fully attacker-controlled, so a distinct-key flood thrashes legitimate entries out of the cache and every unauthenticated request falls through to the full price-map aggregation query. Smallest change: replace the eviction sort with a single-pass `min_by_key` and reject unknown `city`/`property_type` at the top of `get_price_map`.

## Evidence
- `backend/servers/reality-server/src/routes/price_map.rs:67-85` — `insert_bounded` clones every key into `Vec<(String, Instant)>` and runs `sort_by_key` while holding the write lock (confirmed on disk).
- `backend/servers/reality-server/src/routes/price_map.rs:33-54` — `PRICE_MAP_CACHE` + `MAX_PRICE_MAP_CACHE_ENTRIES` definition.
- Issue #3132 (post-merge review of PR #3092) — explicit perf + security findings with concrete proposed patches.
- `backend/servers/reality-server/tests/suites/price_map_tests.rs` — existing suite only asserts the cap holds, not that eviction cost stays bounded or that unknown keys are rejected.

## Files
- `backend/servers/reality-server/src/routes/price_map.rs`
- `backend/servers/reality-server/tests/suites/price_map_tests.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode (auto-derived from the ticks):**
Mode: cloud-ok

## Repro steps
1. Build reality-server at current `dev` and seed Postgres with a small price-map dataset.
2. From an unauthenticated client, loop `GET /api/v1/price-map?city=<rand>&property_type=<rand>` with distinct `rand` values at ≥50 req/s for 60s until the cache fills past `MAX_PRICE_MAP_CACHE_ENTRIES=10_000`.
3. Measure p95 latency and CPU on reality-server once the cache is saturated.
4. Expected after fix: p95 should NOT scale with cache size (no O(n log n) eviction cost); unknown `city` or `property_type` should return `400 Bad Request` without hitting the aggregation query or taking the cache write lock.
5. Actual on current `dev`: each at-capacity insert clones ~10k Strings + sorts them under the write lock; legitimate entries are evicted; unknown keys still execute the full aggregation query.

## Suggested approach
1. In `insert_bounded` (`price_map.rs:67-85`), replace the full-keyset clone + sort with a single-pass `min_by_key(|(_, v)| v.expires_at)` eviction of the oldest live entry. Since every entry shares `CACHE_TTL`, insertion order == `expires_at` order, so a single eviction is sufficient per over-cap insert.
2. In `get_price_map`, validate `city` against the known enumeration (reuse any existing allowlist from `city`/`region` seed data; if none exists, add a `const KNOWN_CITIES: &[&str]`) and `property_type` against the typed enum used elsewhere in reality-server. Return `400 Bad Request` on unknown values BEFORE acquiring the cache lock.
3. Keep `MAX_PRICE_MAP_CACHE_ENTRIES` as a backstop — don't remove it; the allowlist bounds the key space in normal operation but the backstop still protects against future enum drift.
4. Benchmark the eviction path in `price_map_tests.rs`: at `MAX_PRICE_MAP_CACHE_ENTRIES` fill, assert that a single insert does ≤ one `remove` + one `min_by_key` scan (no clone-the-keyset path).
5. Add a regression test that `GET /api/v1/price-map?city=NOT-A-CITY` returns 400 without touching the cache (assert cache size unchanged).
6. Do NOT widen scope — the inquiry-rate-limit mirror suggested in the issue is a separate, larger change and belongs in its own plan.

## Alternatives considered
- **Back the cache with the `lru` crate** — rejected because it adds a dependency for a one-off, and the single-oldest `min_by_key` eviction gives the same O(n) step without a new crate; if hot-path profiling later shows the O(n) scan is itself the bottleneck, moving to `lru` is a localized follow-up.
- **Per-IP distinct-key rate limiter mirroring `INQUIRY_RATE_LIMIT_MAX_IPS`** — rejected as out of scope for this plan; it is a defensible second layer but the allowlist + cheaper eviction are the minimum change that closes the thrash vector described in #3132.

## Root-cause trace
1. Symptom: under a distinct-key flood on `GET /api/v1/price-map`, p95 latency climbs with cache size and the cache hit rate collapses on legitimate traffic.
2. ← Immediate cause at `backend/servers/reality-server/src/routes/price_map.rs:77-85`: `by_age` clones every key and `sort_by_key` runs O(n log n) under the write lock on every at-capacity insert.
3. ← Upstream cause at `backend/servers/reality-server/src/routes/price_map.rs` (entry point to `get_price_map`): `city`/`property_type` are inserted into the cache without validation, so the key space is attacker-controlled.
4. Origin: PR #3092 (merged 2026-10-19) shipped the size cap but deferred the allowlist and used the naive eviction. The eviction and the lack of validation are both introduced by that PR.

## Test plan
- [ ] `backend/servers/reality-server/tests/suites/price_map_tests.rs` — add a case that fills cache to capacity, then does 100 at-capacity inserts and asserts eviction does NOT clone the whole keyset (via a counting wrapper around the HashMap iter, or by asserting the test completes within a tight CPU budget).
- [ ] Add a regression test asserting `GET /api/v1/price-map?city=<unknown>` returns 400 and leaves `PRICE_MAP_CACHE.len()` unchanged.
- [ ] Add a regression test asserting `GET /api/v1/price-map?property_type=<unknown>` returns 400 identically.
- [ ] Command: `cd backend && cargo test -p reality-server --test price_map_tests`

## Out of scope
- Per-IP distinct-key rate limiting (separate plan, cross-cuts other anonymous endpoints).
- Moving to the `lru` crate (dependency change, revisit only if the O(n) `min_by_key` is itself hot).
- Changing the `CACHE_TTL` or the aggregation query shape.

## After-merge
- Move this file to `plans/_archive/code-review-reality-server-price-map-cache-thrash-dos.md`
- Mark the matching `backlog.json` row as `status: "done"`
