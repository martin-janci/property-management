# code-review-reality-server-listings-viewcount

**Vector:** bug
**Score:** 3
**Source:** commit 980519f8 (dispatcher Tier-1d dev-review, 2026-08-06)
**Confidence:** high

## Hypothesis
`GET /api/v1/listings/{id}` in reality-server hardcodes `view_count: 0` on the response with a `// Would need analytics query` comment, but the app already persists views via `RealityPortalRepository::track_view` (wired through `handlers/listings/mod.rs::track_view`) into `listing_analytics`. The public read path never joins the analytics store, so every public listing detail page reports 0 views regardless of traffic. Fix: read the running total from `listing_analytics` for the listing id and return it in the same response envelope; keep the write path (`track_view`) unchanged.

## Evidence
- `backend/servers/reality-server/src/routes/listings.rs:432` — `view_count: 0, // Would need analytics query` (hardcoded).
- `backend/servers/reality-server/src/handlers/listings/mod.rs:130-138` — `track_view` delegates to `RealityPortalRepository::track_view`, which persists to `listing_analytics`.
- `backend/servers/reality-server/src/routes/portal_listings.rs:548-580` — the *owner-scoped* analytics read (`get_my_listing_analytics`) already resolves per-listing analytics from `listing_analytics`, proving the data is there; the *public* detail route just doesn't ask for it.

## Files
- `backend/servers/reality-server/src/routes/listings.rs`
- `backend/servers/reality-server/src/handlers/listings/mod.rs`

## Dependencies
<!-- No task_id dependencies; this plan can be claimed immediately. -->

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
1. Seed a listing, then call `POST` (or trigger any read of the public listing detail via SSR) enough times that `track_view` fires at least N=5 times against `listing_analytics`.
2. `curl http://localhost:8081/api/v1/listings/{id}` — response contains `"view_count": 0`.
3. Expected after fix: `"view_count": 5` (the aggregate the analytics store already holds).

## Suggested approach
1. In `routes/listings.rs`, after the existing per-listing fetch (~line 400), read the aggregate view count via the shared `RealityPortalRepository` — mirror the pattern used by `routes/portal_listings.rs::get_my_listing_analytics` but strip the ownership gate (the public read is unauthenticated but only exposes an integer, not per-user rows).
2. Add a `count_views(listing_id) -> Result<i64, _>` (or reuse the existing analytics-count query if one already lives on the repository) so the route reads a single integer with a bounded SQL cost.
3. Wire the result into the response builder at `listings.rs:432`, replacing the `view_count: 0` literal. Drop the `// Would need analytics query` comment.
4. Verify the read is bounded by the `listing_id` filter — no cross-listing scan — and gated by the same public-read path (no RLS org context needed since `listing_analytics` rows already carry listing_id and the aggregate is not per-user).
5. Add an integration test under `backend/servers/reality-server/tests/` that: seeds a listing, calls the internal `track_view` handler N times, then hits `GET /api/v1/listings/{id}` and asserts `view_count == N`.
6. Run `cargo test -p reality-server` and `cargo clippy -p reality-server --all-targets -- -D warnings`.

## Alternatives considered
- **Materialised counter column on `listings`** — rejected because it duplicates state already stored in `listing_analytics`, forces a second write on every view, and drifts on retention/prune; the aggregate query is cheap enough for a public detail endpoint.
- **Return the field as `null` instead of a real count** — rejected because the response schema already commits to an integer, existing clients (reality-web listing detail, KMP mobile detail) render the number directly, and hiding the count when the data exists is a worse product outcome than the current lie.

## Root-cause trace
1. Symptom: public `GET /api/v1/listings/{id}` responses always contain `view_count: 0`, even for listings with recorded analytics traffic.
2. ← `routes/listings.rs:432` builds the response with a hardcoded `view_count: 0` and a `// Would need analytics query` note — no fetch is issued.
3. ← Reads never wired: the public listing-detail response schema shipped with a `view_count` field before the analytics read path existed; the writer (`RealityPortalRepository::track_view`) landed later but no one updated the response builder to close the loop.
4. Origin: shipped with the public listings response envelope introduction (predates the analytics store landing — the write path was added subsequently in the `handlers/listings/track_view` wire-up).

## Test plan
- [ ] New integration test in `backend/servers/reality-server/tests/` — seed listing + N `track_view` calls, then assert `GET /listings/{id}.view_count == N`.
- [ ] Regression: assert that a listing with no `listing_analytics` rows returns `view_count: 0` (fast-path for cold listings; no error).
- [ ] `cargo test -p reality-server` — full crate green.
- [ ] `cargo clippy -p reality-server --all-targets -- -D warnings`.

## Out of scope
- The 2 orphaned handlers (`get_listing`, `schedule_viewing`) flagged in the same dispatcher Tier-1d generator commit — those are `refactor` vectors, tracked separately as `code-review-reality-server-orphaned-get-listing` and `code-review-reality-server-orphaned-schedule-viewing`.
- Any change to `track_view` semantics or `listing_analytics` schema — the write path is correct; only the read is broken.
- Per-day / per-source breakdowns — the public field is a single aggregate integer.

## After-merge
- Move this file to `plans/_archive/code-review-reality-server-listings-viewcount.md`
- Mark the matching `backlog.json` row as `status: "done"`
