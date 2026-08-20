# code-review-reality-server-submit-report-no-rate-limit

**Vector:** security
**Score:** 2
**Source:** reality-server segment review 2026-08-19 (Phase 1.5 code-review slice)
**Confidence:** high

## Hypothesis
`POST /api/v1/reports` is anonymous-capable (`OptionalRequestPrincipal`) and accepts up to a 5000-char description plus ten 2048-char attachment URLs, but no per-IP throttle is wired even though the endpoint's own doc-comment calls out the requirement. An unauthenticated attacker can flood the endpoint with valid active-listing UUIDs and bloat `listing_reports` + the moderation queue unboundedly. The fix is to reuse the existing `client_ip_bucket` + `TenantRateLimiterSet` pattern that already gates the sibling anonymous inquiry POSTs.

## Evidence
- `backend/servers/reality-server/src/routes/reports.rs:149` — `submit_report(OptionalRequestPrincipal(_), … ) -> Result<Json<SubmitReportResponse>, ApiError>`; no rate-limit call in the body.
- `backend/servers/reality-server/src/routes/reports.rs:120` — the endpoint's own doc-comment: "anonymous reports still go through but should be rate-limited by IP at the moderation layer."
- `backend/servers/reality-server/src/routes/inquiries.rs:87` (`client_ip_bucket`) and `:119` (`enforce_inquiry_rate_limit`) — reusable, tested throttling helpers already carried by the same crate.
- `backend/servers/reality-server/src/routes/inquiries.rs:309` and `:422` — `send_contact_message` and `request_viewing` invoke `enforce_inquiry_rate_limit(&state, &headers, listing_id).await?` before any DB work, exactly the shape needed here.
- `backend/servers/reality-server/src/state.rs:888` (`inquiry_rate_limiters: Arc<TenantRateLimiterSet>`) and `:894` (`inquiry_trusted_proxy_hops`) — the shared state slot the new limiter (or a reuse of the existing one) plugs into.

## Files
- `backend/servers/reality-server/src/routes/reports.rs`
- `backend/servers/reality-server/src/routes/inquiries.rs`
- `backend/servers/reality-server/src/state.rs`
- `backend/servers/reality-server/tests/suites/reports_tests.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (security regression class)
- [x] C2 — Seed data (active listing + attacker IP)
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode (auto-derived from the ticks):** no C4, no C5 → `Mode: cloud-ok`

Mode: cloud-ok

## Repro steps
1. Start reality-server against a seeded DB with at least one `active` listing (call it `L`).
2. From a single client IP, `curl -sS -X POST /api/v1/reports -H 'X-Forwarded-For: 203.0.113.5' -H 'Content-Type: application/json' -d '{"listing_id":"<L>","reason":"other","description":"spam"}'` in a tight loop (e.g. 500 iterations).
3. Expected: after N ≈ the inquiry per-IP quota, subsequent requests return `429 Too Many Requests` and `SELECT COUNT(*) FROM listing_reports WHERE listing_id=<L>` stays at N. Actual today: every request returns `202 Accepted` and `listing_reports` grows without bound.

## Suggested approach
1. Add `headers: HeaderMap` to `submit_report`'s handler signature — mirror `send_contact_message` at `routes/inquiries.rs:301`.
2. Introduce a small helper `enforce_public_report_rate_limit(&state, &headers, listing_id).await?` in `routes/reports.rs` that reuses `state.inquiry_rate_limiters` + `client_ip_bucket(headers, state.inquiry_trusted_proxy_hops)`. Reusing the existing limiter keeps blast radius contained (one field, no new env vars); if operator wants an independent bucket, add `state.report_rate_limiters` in a follow-up.
3. Call the helper as the very first line of `submit_report`, before description length / attachment URL / `listing_exists` checks — matches the "reject before DB work" ordering used in the inquiry path.
4. Map `TenantRateLimiterSet::Decision::Denied { retry_after_secs }` to `ApiError::TooManyRequests { retry_after_secs }` the same way `enforce_inquiry_rate_limit` does.
5. Extend `backend/servers/reality-server/tests/suites/reports_tests.rs` with the regression test in the Test plan.
6. Update the endpoint doc-comment: replace "should be rate-limited by IP at the moderation layer" with a one-line note that the throttle now runs at the route layer.

## Alternatives considered
- **Move the throttle into a moderation-layer background job** — rejected because it lets DoS traffic still reach the DB and only trims after-the-fact; the DoS surface is the request path, not the queue.
- **Add a fresh env-configurable `report_rate_limiters` slot on `AppState`** — rejected as first cut because it doubles the config surface for a fix whose blast radius today is a single endpoint; reuse of `inquiry_rate_limiters` matches the burst profile (listing-scoped anonymous POST). Can be split off later if operators need distinct budgets.

## Root-cause trace
1. Symptom: `POST /api/v1/reports` accepts unlimited anonymous requests from a single IP; `listing_reports` grows without bound.
2. ← `submit_report` at `backend/servers/reality-server/src/routes/reports.rs:149` does not invoke any rate-limit helper before the `INSERT`.
3. ← When the reports route was added, the throttling helpers already existed in `routes/inquiries.rs:87` / `:119` but were not invoked here; the doc-comment at `routes/reports.rs:120` acknowledged the gap but no wiring followed.
4. Origin: the initial reports-route commit that introduced `submit_report` — the sibling inquiry throttle predates this handler, so this is a copy-forward omission, not a regression.

## Test plan
- [ ] `backend/servers/reality-server/tests/suites/reports_tests.rs` — seed one `active` listing, POST `/api/v1/reports` N+1 times with a fixed `X-Forwarded-For`; assert the (N+1)-th response is `429` and `SELECT COUNT(*) FROM listing_reports WHERE listing_id = $1` equals the pre-trip quota.
- [ ] Regression: POST from two distinct `X-Forwarded-For` values interleaved — each bucket should trip independently, proving the IP bucketing.
- [ ] Command: `cargo test -p reality-server --test suites -- reports_tests::submit_report_is_rate_limited_per_ip`

## Out of scope
- Introducing a distinct `report_rate_limiters` state field (leave as a follow-up if the operator wants a separate budget).
- Rate-limiting the authenticated `list_my_reports` / `update_report_status` paths (no reported abuse surface, and they already require auth + role checks).
- Adding CAPTCHA / turnstile challenges (product decision, out of scope for a routing-layer throttle).
- The parallel pagination-total bug in `list_my_reports` (`code-review-reality-server-list-my-reports-total-page-length`) — same file but a different failure mode; will land as its own change.

## After-merge
- Move this file to `plans/_archive/code-review-reality-server-submit-report-no-rate-limit.md`
- Mark the matching `backlog.json` row as `status: "done"`
