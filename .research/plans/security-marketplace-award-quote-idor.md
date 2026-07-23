# security-marketplace-award-quote-idor

**Vector:** security
**Score:** 3
**Source:** rotating-expert-review api-handlers 2026-07-23 (tier1d)
**Confidence:** high

## Hypothesis
`award_quote` (POST `/api/v1/marketplace/rfqs/{id}/award`) authorizes the caller only against the parent RFQ; the body's `quote_id` is never validated to belong to that RFQ. A tenant manager can therefore award their own RFQ to a quote from a different (potentially cross-tenant) RFQ, corrupting `rfqs.awarded_quote_id`/`awarded_to` and flipping an unrelated provider's `provider_quotes.status` to `accepted`. Fix by scoping the accept SQL to the parent RFQ (`AND rfq_id = $1`) and rejecting body `quote_id`s that don't belong.

## Evidence
- `backend/servers/api-server/src/routes/marketplace.rs:922-935` — `award_quote` loads and org-checks RFQ `{id}` then calls `award_rfq(id, data.quote_id)` with an unvalidated body `quote_id: Uuid`.
- `backend/servers/api-server/src/routes/marketplace.rs:375` — `AwardQuoteRequest.quote_id` is a raw `Uuid` with no validator.
- `backend/crates/db/src/repositories/marketplace.rs:786-808` — `award_rfq` resolves the quote via `find_quote_by_id(quote_id)` (no `rfq_id`/`org` predicate), updates rfqs.awarded_quote_id + awarded_to, then `UPDATE provider_quotes SET status='accepted' WHERE id=$1` (no rfq scope).
- `backend/crates/db/src/repositories/marketplace.rs:820-831` — sibling reject step correctly uses `WHERE rfq_id=$1 AND id!=$2`, showing the accept step's missing scope is an in-file inconsistency, not a design choice.
- Secondary: `award_rfq` has no status guard (contrast `cancel_rfq` at :837 `WHERE id=$1 AND status NOT IN ('awarded','cancelled')`), so already-awarded/cancelled RFQs re-award and re-flip quote statuses each call.

## Files
- `backend/servers/api-server/src/routes/marketplace.rs`
- `backend/crates/db/src/repositories/marketplace.rs`

## Dependencies

## Required capabilities
- [ ] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [x] C2 — Seed data
- [x] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

Mode: cloud-ok

## Repro steps
1. Seed two orgs A and B, each with an RFQ and at least one submitted `provider_quote`. Record ids: `rfq_A`, `quote_A1` (belongs to `rfq_A`), `rfq_B`, `quote_B1` (belongs to `rfq_B`).
2. As a manager in org A, POST `/api/v1/marketplace/rfqs/{rfq_A}/award` with body `{"quote_id": "<quote_B1>"}` (a quote from the other RFQ, possibly another tenant).
3. Expected: HTTP 400/403 rejecting the mismatched quote. Actual: HTTP 200 — `rfqs.awarded_quote_id = quote_B1`, `rfqs.awarded_to = provider_of(quote_B1)`, `provider_quotes[quote_B1].status = 'accepted'`.

## Suggested approach
1. In `backend/crates/db/src/repositories/marketplace.rs`, add `find_quote_by_id_and_rfq(rfq_id, quote_id) -> Result<Option<ProviderQuote>>` that scopes with `WHERE id = $1 AND rfq_id = $2` (return `Option`, callers map `None` to a 404/400).
2. In `award_rfq(id, quote_id)` at :781: replace the unscoped `find_quote_by_id` with the new scoped lookup; return early with a typed error (`RfqError::QuoteNotForRfq`) if `None`.
3. Add the status guard: change the `UPDATE rfqs` in `award_rfq` to `WHERE id = $1 AND status NOT IN ('awarded','cancelled')`; propagate `Option::None` → 409 Conflict at the handler.
4. Scope the accept SQL at :811: change `WHERE id = $1` to `WHERE id = $1 AND rfq_id = $2` (bind both).
5. In `backend/servers/api-server/src/routes/marketplace.rs::award_quote` (:922): map the new domain errors to appropriate HTTP responses (404/400 for cross-RFQ quote, 409 for already-awarded RFQ).
6. Add integration test covering the cross-RFQ (and cross-tenant) case + the double-award case.
7. Run `cargo test -p api-server marketplace::award --nocapture` and `cargo clippy -p api-server -- -D warnings`.

## Alternatives considered
- **Handler-side pre-check (fetch quote, compare `quote.rfq_id`, then call `award_rfq`)** — rejected because it leaves a TOCTOU window between the check and the two-step SQL fan-out; scoping the SQL closes the race in a single boundary.
- **Add a DB CHECK / trigger enforcing `provider_quotes.rfq_id = rfqs.awarded_quote_id → rfq.id`** — rejected because it doesn't address the accept-status write on the wrong row and would require a migration; the SQL-scope fix is smaller and covers both writes.

## Root-cause trace
1. Symptom: `rfqs.awarded_quote_id` and `provider_quotes[quote_id].status='accepted'` can be set from a quote that belongs to a different RFQ (or tenant).
2. ← `backend/crates/db/src/repositories/marketplace.rs:786` — `find_quote_by_id(quote_id)` resolves by id only.
3. ← `backend/crates/db/src/repositories/marketplace.rs:811` — accept `UPDATE provider_quotes ... WHERE id = $1` binds only the client-supplied id; no `rfq_id` constraint.
4. ← `backend/servers/api-server/src/routes/marketplace.rs:922-935` — handler passes the raw body `quote_id` to `award_rfq` after org-checking only the path `{id}` RFQ.
5. Origin: initial `award_rfq` implementation (introduced with the marketplace repo). Same IDOR class as the 2026-07-22 `ai/ocr submit_correction` finding — parent-org membership checked, foreign body id trusted.

## Test plan
- [ ] `cargo test -p api-server marketplace::award_quote_rejects_cross_rfq_quote` — POST /rfqs/A/award with a `quote_id` from RFQ B ⇒ 404/400; DB unchanged.
- [ ] `cargo test -p api-server marketplace::award_quote_rejects_already_awarded_rfq` — second award on the same RFQ ⇒ 409; no additional `provider_quotes` status flips.
- [ ] `cargo test -p api-server marketplace::award_quote_happy_path_still_works` — regression: legitimate award still succeeds and idempotently accepts the winning quote / rejects the rest.
- [ ] `cargo test -p api-server -- marketplace::award --nocapture`

## Out of scope
- Refactor of the wider marketplace RFQ lifecycle.
- Any change to quote submission or bidding endpoints.
- Multi-quote award (award-to-multiple) semantics.

## After-merge
- Move this file to `plans/_archive/security-marketplace-award-quote-idor.md`
- Mark the matching `backlog.json` row as `status: "done"`
