# code-review-reality-server-price-map-eviction-dos

**Vector:** security
**Score:** 2
**Source:** Issue #3132 (post-merge review of PR #3092)
**Confidence:** high

## Hypothesis
PR #3092 added a `MAX_PRICE_MAP_CACHE_ENTRIES = 10_000` + `insert_bounded()` size cap to defeat an OOM DoS on the anonymous `GET /api/v1/price-map` handler, but shifted the attack rather than removing it. `insert_bounded` at capacity performs a full `.iter().map(k.clone())` plus `O(n log n)` `sort_by_key` on every insert while holding the global `tokio::sync::RwLock` write guard, so the exact same anonymous distinct-key flood now serializes CPU + lock-contention through the write lock. Separately, `city` and `property_type` are not validated against a known enumeration, so the attacker-controlled cache-key space remains unbounded and legitimate entries are constantly evicted — cache hit rate collapses to ~0 and every anonymous request falls through to the full DB aggregation. Smallest change: replace the full sort with a single-pass oldest-eviction (overflow is always 1 since the map grows one key at a time), and validate `city` / `property_type` against the known enum before building the cache key.

## Evidence
- Issue #3132 (post-merge-reviewed): two medium-severity findings on PR #3092 — perf (`insert_bounded` sort cost) + security (cache-thrash / DB-load DoS)
- `backend/servers/reality-server/src/routes/price_map.rs:67-87` — `insert_bounded`: `.iter().map(k.clone())` + `sort_by_key` on every at-cap insert under `cache` write lock
- `backend/servers/reality-server/src/routes/price_map.rs:170-176` — `cache_key = format!("{}:{}:{}:{}", mode, time_window_str, property_type_or_empty, city_or_empty)` — no validation of `property_type` / `city` against a known set
- PR #3092 description explicitly deferred `city`/`property_type` allowlisting as "optional / out-of-scope"; sibling `state.rs` validates `INQUIRY_RATE_LIMIT_MAX_IPS` for the equivalent anonymous-abuse surface

## Files
- `backend/servers/reality-server/src/routes/price_map.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [x] C7 — Code-review reception

**Execution mode (auto-derived from the ticks):**
Mode: cloud-ok

## Repro steps
1. Build with the current `dev` HEAD and run `reality-server` locally against a seeded DB.
2. From an unauthenticated client, issue `N` requests to `GET /api/v1/price-map?city=<rand-i>&property_type=apartment` with `N > MAX_PRICE_MAP_CACHE_ENTRIES = 10_000` and distinct `rand-i` per request.
3. Observe: once the cache reaches 10_000 entries every subsequent request allocates and clones ~10k `String`s and sorts them under the write lock; `curl` latency balloons and CPU on the server pegs. The size cap holds (no OOM), but the cache hit rate for the subsequent legitimate (`city=Bratislava`) request is near zero because its entry was just evicted; each legitimate request falls through to the full DB aggregation.
4. Expected after fix: eviction is O(n) single-pass (no sort, no full keyset clone); unknown `city` / `property_type` return `400` before touching the cache so distinct-key floods never grow it in the first place.

## Suggested approach
1. In `backend/servers/reality-server/src/routes/price_map.rs:67-87`, replace the `sort_by_key` block with a single-pass oldest-eviction:
   ```rust
   if !cache.contains_key(&key) && cache.len() >= MAX_PRICE_MAP_CACHE_ENTRIES {
       if let Some(oldest_key) = cache
           .iter()
           .min_by_key(|(_, v)| v.expires_at)
           .map(|(k, _)| k.clone())
       {
           cache.remove(&oldest_key);
       }
   }
   ```
   Since the map grows at most one key per insert, overflow is always 1 — a single `min_by_key` + one `String` clone is sufficient and strictly cheaper than the current full sort.
2. In `get_price_map` (around line 148), validate `query.property_type` and `query.city` against a known enumeration before building `cache_key`. Property type: match the `apartment | house | land | commercial` set the aggregation query already uses (same style as `time_window` validation at line 157-170 — return `400 BAD_REQUEST` with a one-line message on unknown values). `city` is user data, so instead of a static allowlist, either (a) accept only cities present in a materialised `listings.city DISTINCT` set loaded at startup / refreshed hourly (the aggregation already touches that column), or (b) short-circuit on `city` length > N chars / disallowed characters so the key space is at least length-bounded.
3. Add a per-IP distinct-key rate limit (optional, defense-in-depth) mirroring `INQUIRY_RATE_LIMIT_MAX_IPS` in `backend/servers/reality-server/src/state.rs` — reject a client that has rotated more than K distinct `(city, property_type)` combinations in a rolling window; emit `429 TOO_MANY_REQUESTS`.
4. Keep `MAX_PRICE_MAP_CACHE_ENTRIES = 10_000` as the memory backstop — the new validation is the primary control; the cap becomes defense-in-depth.
5. Add an inline comment on `insert_bounded` noting that overflow is always 1 by construction so the single-pass eviction is exact; and that this is intentionally not an `lru::LruCache` because every entry shares `CACHE_TTL`, so insertion order == `expires_at` order.

## Alternatives considered
- **Replace `HashMap` with `lru::LruCache` or `BTreeMap<Instant, key>` side-index** — rejected because the single-pass `min_by_key` fix is one 5-line change that already drops `O(n log n)` to `O(n)` without a new dependency or a second data-structure to keep in sync; the issue flags LRU as the "true O(log n)" option but notes a FIFO is "exact here" — matches the proposed fix's complexity trade-off.
- **Allowlist `city` from a static constant** — rejected because the deployment is multi-region (SK / CZ / EU) and the city set is data, not code; a startup-materialised `listings.city DISTINCT` cache (option (a) in step 2) tracks the real data without a hardcoded enum per region.

## Root-cause trace
1. Symptom: anonymous CPU + lock-contention DoS and cache-thrash DB-load DoS on `GET /api/v1/price-map` under distinct-key flood (observed post-merge of PR #3092).
2. ← `insert_bounded` at `price_map.rs:76-87` runs full-keyset clone + `sort_by_key` under write lock on every at-cap insert; executed per anonymous request once cap is reached.
3. ← `cache_key` at `price_map.rs:170-176` concatenates attacker-controlled `city` / `property_type` with no validation; distinct-key space is unbounded, so the "at cap" branch is reachable on every request.
4. Origin: PR #3092 (merged 2026-10-19) — the size cap was added as the sole DoS control; `city`/`property_type` validation and eviction cost were explicitly deferred as out-of-scope.

## Test plan
- [ ] `backend/servers/reality-server/tests/price_map_eviction_bounds_test.rs` (new) — stage `MAX_PRICE_MAP_CACHE_ENTRIES + 100` distinct-key requests, assert each `insert_bounded` call completes within a small bounded budget (e.g. 5 ms on CI) and that total clones across the burst is `O(N)`, not `O(N^2)` or `O(N log N) × N`.
- [ ] `backend/servers/reality-server/tests/price_map_input_validation_test.rs` (new) — `property_type=foo` → `400`; `city=<very-long-string>` → `400`; cache size unchanged after both.
- [ ] Regression: existing `price_map_cache_bounded_test.rs` from PR #3092 keeps passing — the cap invariant must not regress.
- [ ] Command: `cd backend && cargo test -p reality-server --test price_map_eviction_bounds_test --test price_map_input_validation_test --test price_map_cache_bounded_test`

## Out of scope
- Rewriting `PRICE_MAP_CACHE` on top of a different data structure (`lru::LruCache`, `moka`, `BTreeMap` side-index). The single-pass eviction fix lands the perf win without a new dep; migrating the data structure can be a separate refactor.
- Materialising a `listings.city` background refresher. Step 2's option (a) can land as a static-length-bound (option (b)) first; the hourly refresh is a follow-up PR if the length bound proves too permissive against targeted enumeration.
- Any change to the aggregation SQL itself — the DoS is in the cache-eviction and key-validation layer, not the query.

## After-merge
- Move this file to `plans/_archive/code-review-reality-server-price-map-eviction-dos.md`
- Mark the matching `backlog.json` row as `status: "done"`
