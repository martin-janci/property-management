# security-marketplace-award-quote-cross-tenant-idor

**Vector:** security
**Score:** 3
**Source:** signals/2026-07-23-api-handlers-tier1d.json (rotating-expert-review, api-handlers)
**Confidence:** high

## Hypothesis

`marketplace::award_quote` authorizes only the parent RFQ (`load_rfq_for_org` verifies the path `{id}` belongs to the caller's org) but then persists a client-supplied `quote_id` without validating it belongs to that RFQ. `MarketplaceRepository::award_rfq` finds the quote by id alone, updates `rfqs.awarded_to`/`awarded_quote_id` with the derived provider, and separately runs `UPDATE provider_quotes SET status='accepted' WHERE id=$1` — a cross-RFQ (and cross-tenant) write of an unrelated provider's quote. The fix is a one-line scoping addition on both the accept UPDATE and the lookup, mirroring the sibling reject step at `marketplace.rs:820-831` which already scopes with `WHERE rfq_id=$1 AND id!=$2`.

## Evidence

- `backend/servers/api-server/src/routes/marketplace.rs:922-951` — `award_quote` handler binds `user: AuthUser`, calls `load_rfq_for_org(&state, user.user_id, id)` for the path RFQ, then `state.marketplace_repo.award_rfq(id, data.quote_id)` where `data.quote_id: Uuid` comes verbatim from `AwardQuoteRequest` (schema at `marketplace.rs:93`) with zero validation that the quote is scoped to this RFQ or org.
- `backend/crates/db/src/repositories/marketplace.rs:781-835` — `award_rfq(id, quote_id)`: (a) `find_quote_by_id(quote_id)` resolves the quote by id alone, no `rfq_id` predicate; (b) `UPDATE rfqs ... WHERE id=$1` binds only the path RFQ; (c) `UPDATE provider_quotes SET status='accepted' WHERE id=$1` binds only the client `quote_id`; (d) the sibling reject step in the same method uses `WHERE rfq_id=$1 AND id != $2` correctly, proving the accept step's missing predicate is an in-file inconsistency, not intent.
- Failure scenario (from Tier-1d): a manager who owns RFQ-A can award it to a `quote_id` that belongs to a different RFQ (potentially another tenant's), corrupting `RFQ-A.awarded_to`/`awarded_quote_id` with an unrelated provider AND flipping that unrelated provider's quote to `accepted`.
- Secondary correctness defect: no status guard on `award_rfq` — compare `cancel_rfq` at `marketplace.rs:837` which uses `WHERE id=$1 AND status NOT IN ('awarded','cancelled')`. An already-awarded or cancelled RFQ can be re-awarded and re-flip quote statuses on each call.
- Existing happy-path coverage at `backend/servers/api-server/tests/marketplace_ecosystem_backfill_batch3_tests.rs:238-266` (`award_quote_succeeds`) — seeds `rfq → provider → quote` all in the same org and asserts 200. The IDOR-defence regression tests must be net-new and belong alongside this happy-path suite.

## Files

- `backend/servers/api-server/src/routes/marketplace.rs:922`
- `backend/crates/db/src/repositories/marketplace.rs:781`
- `backend/servers/api-server/tests/marketplace_ecosystem_backfill_batch3_tests.rs`

## Dependencies

<none>

## Required capabilities

- [x] C1 — Systematic debugging (bug/security vector)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [x] C7 — Code-review reception (security fix — expect scrutiny)

**Execution mode:** `Mode: cloud-ok` (no C4/C5 tick; backend Rust + sqlx tests run headless).

## Repro steps

1. Seed two orgs `A` and `B` with `org_admin` users; seed an RFQ `RFQ-A` under org `A` (via `seed_rfq(&pool, org_a, user_a)`); seed a provider + quote `Q-B` under org `B` for a different RFQ `RFQ-B` (via `seed_rfq`, `seed_provider`, `seed_quote`).
2. As `user_a` (bearer token minted with `mint_token(user_a, ..., Some(org_a))`), POST `/api/v1/marketplace/rfqs/{RFQ-A}/award` with body `{ "quote_id": "<Q-B>" }`.
3. **Expected:** 404/403/422 rejecting the foreign-RFQ quote. **Actual on `dev`:** 200 OK; `rfqs.awarded_to = Q-B.provider_id`, `rfqs.awarded_quote_id = Q-B`; `provider_quotes.status` for `Q-B` flipped to `accepted`.

## Suggested approach

1. `backend/crates/db/src/repositories/marketplace.rs:781` — change `award_rfq(id, quote_id)` signature to add an org-scoped or rfq-scoped guard: the cheapest correct fix is to resolve the quote with a bounded query `SELECT ... FROM provider_quotes WHERE id=$1 AND rfq_id=$2` (adding `find_quote_by_id_for_rfq`, or inlining), and return `Ok(None)` when the row is missing so the handler surfaces 404.
2. Same file, `UPDATE provider_quotes SET status='accepted' WHERE id=$1` → add `AND rfq_id=$2`, binding both. This alone closes the cross-tenant write.
3. Add a status guard on the `UPDATE rfqs` — `WHERE id=$1 AND status NOT IN ('awarded','cancelled')` mirroring `cancel_rfq` at `marketplace.rs:837` — so re-awarding a closed RFQ is a no-op.
4. `backend/servers/api-server/src/routes/marketplace.rs:922-951` — no handler-level change required if the repo now returns `Ok(None)` on a cross-RFQ quote; the existing `.ok_or_else(|| NOT_FOUND)` already covers it. Update the `tracing::error!` message to reflect the new failure mode.
5. `backend/servers/api-server/tests/marketplace_ecosystem_backfill_batch3_tests.rs` — add `#[sqlx::test]` `award_quote_rejects_foreign_rfq_quote` (same-tenant, different RFQ) and `award_quote_rejects_cross_tenant_quote` (org B's quote against org A's RFQ) — both expect 404, both assert the target RFQ's `awarded_to` is still NULL and the foreign quote's `status` is still `submitted` post-call.
6. Optional in the same PR (cheap): add `award_quote_rejects_already_awarded` — award once, then a second call with the same `quote_id` returns 404/409 and the DB is unchanged.
7. Run `cargo test -p api-server --test marketplace_ecosystem_backfill_batch3_tests`; regen sqlx offline data if any query text changed (`cd backend && cargo sqlx prepare --workspace`).

## Alternatives considered

- **Add a defence-in-depth check in the handler before calling `award_rfq`** — rejected because it duplicates repo logic and leaves the repo method still willing to write across RFQs if any future caller passes bad ids. The fix belongs at the SQL boundary where the assumption lives.
- **Introduce a new `award_rfq_with_quote_check` method and leave the old one** — rejected because the old method has no legitimate use case (every caller supplies both ids and expects them related), so keeping it is a footgun for the next author.

## Root-cause trace

1. Symptom: `provider_quotes.status='accepted'` and `rfqs.awarded_to`/`awarded_quote_id` mutated on an RFQ/quote pair that never belonged together.
2. ← `UPDATE provider_quotes ... WHERE id=$1` at `backend/crates/db/src/repositories/marketplace.rs:808-817` — the accept step binds only the client-supplied `quote_id`, no `rfq_id`.
3. ← `award_rfq(id, quote_id)` at `backend/crates/db/src/repositories/marketplace.rs:781-835` — accepts an unbounded `(id, quote_id)` pair; the repo layer's contract with the handler is that the pair is already validated, but the handler at `marketplace.rs:922-951` only validated `id` (via `load_rfq_for_org`) and passed `data.quote_id` through untouched.
4. Origin: initial `award_rfq` implementation shipped with the accept UPDATE bound only on `id` while the reject UPDATE (three lines below at `:820-831`) correctly used `WHERE rfq_id=$1 AND id != $2` — the accept-vs-reject asymmetry was a copy-paste omission at the time the method was authored.

## Test plan

- [ ] `#[sqlx::test]` `award_quote_rejects_foreign_rfq_quote` in `backend/servers/api-server/tests/marketplace_ecosystem_backfill_batch3_tests.rs` — same-tenant, different RFQ; expect 404, assert DB unchanged.
- [ ] `#[sqlx::test]` `award_quote_rejects_cross_tenant_quote` in the same file — org B's quote against org A's RFQ; expect 404, assert DB unchanged.
- [ ] `#[sqlx::test]` `award_quote_rejects_already_awarded` — award once then reprocess with the same `quote_id`; expect 404/409, DB unchanged since first call.
- [ ] Command: `cd backend && cargo test -p api-server --test marketplace_ecosystem_backfill_batch3_tests`.

## Out of scope

- Broader marketplace IDOR sweep — this plan fixes `award_quote` only. Other marketplace endpoints (compare_quotes, reject_quote, invitations, badges) are separately handled by prior code-review findings; do not fold them in.
- Rewriting `MarketplaceRepository`'s helpers into a generic org-scoping wrapper — a targeted 2-line predicate fix is safer.

## After-merge

- Move this file to `plans/_archive/security-marketplace-award-quote-cross-tenant-idor.md`
- Mark the matching `backlog.json` row (`code-review-api-handlers-marketplace-award-quote-idor`) as `status: "done"`
