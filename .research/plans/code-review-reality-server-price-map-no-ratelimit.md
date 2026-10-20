# code-review-reality-server-price-map-no-ratelimit

**Vector:** security
**Score:** 2
**Source:** rotating-expert-review 2026-10-20 reality-server (companion to merged PR #3092)
**Confidence:** high

## Hypothesis

The already-merged PR #3092 bounded `PRICE_MAP_CACHE` to 10 000 entries, which fixes the heap-exhaustion DoS but leaves the DB-load DoS wide open: every unique `(mode, time_window, property_type, city)` combination that isn't cached still runs the full `current_period` + `prior_period` CTE aggregation. An anonymous attacker can keep rotating `city` / `property_type` to force fresh heavy queries at the full write rate of their connection, overloading the shared public read pool. The handler needs the same per-IP rate limit the `/inquiries` family already uses (`inquiry_rate_limiters` in `AppState`) — a bounded limiter is cheap, we have the precedent and the AppState wiring in place.

## Evidence
- backend/servers/reality-server/src/routes/price_map.rs:143 — `pub async fn get_price_map` has no rate-limit guard; cache check at :133-141 misses on any new key.
- backend/servers/reality-server/src/routes/price_map.rs:149-247 — the two CTEs run `COALESCE/AVG/LAG` over the full `listings_public` set per call; cost is non-trivial even with the index coverage already in place.
- backend/servers/reality-server/src/state.rs:889-920 — `inquiry_rate_limiters: TenantRateLimiterSet` is already constructed in `AppState::new` and keyed per client IP; the knobs `INQUIRY_RATE_LIMIT_RPM`, `INQUIRY_RATE_LIMIT_MAX_IPS`, `INQUIRY_RATE_LIMIT_IDLE_TTL` define the shape of the bounded limiter we want to reuse (or clone for a public-GET sibling).
- backend/servers/reality-server/src/routes/inquiries.rs — invocation site of the limiter; mirrors the shape (`check_rate_limit(..., client_ip).await?`).
- The PR #3092 fix explicitly scoped itself to the memory bound ("TTL (correctness) and the size cap (memory bound) compose"); the DB-load bound is listed as follow-up in the review feedback.

## Files
- `backend/servers/reality-server/src/routes/price_map.rs:143`
- `backend/servers/reality-server/src/state.rs:889`
- `backend/servers/reality-server/src/routes/inquiries.rs`

## Dependencies
<!-- No upstream dependencies — handler + state wiring are both on dev HEAD. -->

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
1. In a terminal, loop `curl` against the dev `reality-server` with rotating `city`:
   ```bash
   for i in $(seq 1 500); do
     curl -s "http://127.0.0.1:8081/api/v1/price-map?mode=sale&time_window=last_30_days&property_type=flat&city=rand-$i" > /dev/null
   done
   ```
2. **Expected (after fix):** after N requests from the same IP within the rate window, requests respond with HTTP 429 and the DB connection count stays flat.
   **Actual (today):** no 429 response and no bound; `pg_stat_activity` shows the `/price-map` CTE firing per request, cache size climbs toward 10 000 before cap kicks in; DB CPU climbs linearly.

## Suggested approach
1. Extend `AppState` with a sibling bounded limiter dedicated to the public analytical GETs — call it `price_map_rate_limiter` (or a shared `public_analytics_rate_limiter`) built from the existing `TenantRateLimiterSet` with its own `PRICE_MAP_RATE_LIMIT_RPM` / `_MAX_IPS` / `_IDLE_TTL` constants in `state.rs` and env overrides, defaults e.g. `rpm=60`, `max_ips=100_000`, `idle_ttl=300s`.
2. Thread the limiter into the handler via `State<AppState>` and gate the entry at the top of `get_price_map` before the cache check — the limiter's `check_and_bump(client_ip)` returns 429 before any DB work.
3. Extract the client IP using the existing helper (`client_ip_from_headers` or equivalent used by `/inquiries`); fall back to `X-Forwarded-For` as the inquiries path does.
4. Add a counter metric so operators can observe 429 rate (same `metrics` crate macros the inquiries path uses, if any).
5. Preserve the existing cache check order: rate limit → cache → DB (cache hits still only cost the limiter check).
6. Document the new env knobs in `backend/CLAUDE.md` next to `INQUIRY_RATE_LIMIT_*`.
7. Make it opt-outable per deployment (the inquiries pattern already respects env overrides); the default must be the limited path.

## Alternatives considered
- **Reuse `inquiry_rate_limiters` directly** — rejected because the inquiry quota is tuned to form-submit frequency (12 rpm); analytical GETs want a different ceiling and conflating them would starve legitimate map-exploration or inquiry traffic.
- **Move the whole aggregation to a scheduled materialized view** — rejected because the diff grows beyond this plan's scope (schema change, migration, cache invalidation story) and the limiter is a smaller, independent bound.

## Root-cause trace
1. Symptom: anonymous caller rotating `city` keeps forcing fresh CTE aggregations, driving DB CPU to saturation while leaving app heap steady.
2. ← `get_price_map` at `backend/servers/reality-server/src/routes/price_map.rs:143` has no per-IP admission check; it hands off to the cache + DB.
3. ← The cache cap in PR #3092 bounds memory but not DB cost per cache miss — each new key is one query.
4. Origin: `price_map.rs` created as part of the public analytics surface without the rate-limit guard that the `/inquiries` family received at inquiry-form landing; the pattern was never ported to analytical GETs.

## Test plan
- [ ] Unit: new test in `backend/servers/reality-server/src/routes/price_map.rs` using `tokio::test` that drives the handler via a `Router` with the limiter wired, asserts first N requests from one IP return 200 and the N+1-th returns 429.
- [ ] Unit: a second test that mixes two IPs and confirms their buckets are independent (one at ceiling does not throttle the other).
- [ ] Regression: existing inline unit tests for the cache cap must still pass.
- [ ] Command: `cd backend && cargo test -p reality-server --tests routes::price_map`

## Out of scope
- Changing the response schema or caching strategy.
- Introducing a persistent rate-limit backend (Redis). Keep in-process bounded.
- Materializing the aggregation into a view.
- Rate-limiting other public GETs (articles, listings). Those get their own plans if needed.

## After-merge
- Move this file to `plans/_archive/code-review-reality-server-price-map-no-ratelimit.md`
- Mark the matching `backlog.json` row as `status: "done"`
