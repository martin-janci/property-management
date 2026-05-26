# security-realtors-mark-inquiry-read-idor

**Vector:** security
**Score:** 3
**Source:** Issue #519 | PR #508 | realtors.rs:250
**Confidence:** high

## Hypothesis
The reality-server `mark_inquiry_read` handler at `routes/realtors.rs:250` takes only `State` and `Path<Uuid>` — it never binds the authenticated principal — and calls the unscoped repo method `mark_inquiry_read(id)`, whose query `UPDATE listing_inquiries SET status = 'read', read_at = NOW() WHERE id = $1 AND read_at IS NULL` has no realtor predicate. Any authenticated portal user can flip another realtor's inquiry to `read` by enumerating UUIDs — a cross-account write IDOR. This is the sibling of the `inquiries.rs` `mark_as_read` handler that PR #497 already scoped; PR #508 fixed `respond_to_inquiry` but left this second read-marking route open. The smallest fix is to bind `principal: RequestPrincipal` and call the existing scoped method `mark_inquiry_read_for_realtor(id, principal.user_id)`, returning 404 when no owned row matches.

## Evidence
- `backend/servers/reality-server/src/routes/realtors.rs:250` — `mark_inquiry_read(State, Path<Uuid>)` binds no principal and calls `state.reality_portal_repo.mark_inquiry_read(id)`; wired at realtors.rs:28 as `POST /api/v1/realtors/inquiries/{id}/read`
- `backend/crates/db/src/repositories/reality_portal.rs:767` — `mark_inquiry_read(&self, id)` runs `UPDATE listing_inquiries SET status = 'read', read_at = NOW() WHERE id = $1 AND read_at IS NULL` with no realtor/owner predicate
- `backend/crates/db/src/repositories/reality_portal.rs:779` — scoped sibling `mark_inquiry_read_for_realtor(&self, id, realtor_id)` already exists and is the method `inquiries.rs:564 mark_as_read` uses after PR #497
- Issue #519 (follow-up, from-merged-review of PR #508) flags this exact gap: "mark_inquiry_read IDOR still open in realtors.rs"
- `respond_to_inquiry` in the same file (realtors.rs:281) already binds `principal` and scopes via `respond_to_inquiry(id, principal.user_id, ...)` — the read-marking route is the lone unscoped write on the realtors surface

## Files
- `backend/servers/reality-server/src/routes/realtors.rs:250`
- `backend/crates/db/src/repositories/reality_portal.rs:767`
- `backend/servers/reality-server/tests/inquiry_idor_tests.rs`

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [x] C2 — Seed data
- [x] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

**Execution mode (auto-derived from the ticks):**
Mode: cloud-ok

The fix and its regression test run entirely against a seeded Postgres via the
ppt-bridge MCP — no browser or device needed.

## Repro steps
1. Seed two realtor accounts (realtor A, realtor B) and one listing inquiry owned by realtor A.
2. Authenticate as realtor B and call `POST /api/v1/realtors/inquiries/{A_inquiry_id}/read`.
3. Expected: 404 (B does not own the inquiry, the row is untouched). Actual on `main`: 204 and the inquiry's `status` flips to `read` / `read_at` is set — B mutated A's inquiry.

## Suggested approach
1. In `backend/servers/reality-server/src/routes/realtors.rs:250`, change the handler signature to `mark_inquiry_read(State(state): State<AppState>, principal: RequestPrincipal, Path(id): Path<Uuid>)`.
2. Replace the call `state.reality_portal_repo.mark_inquiry_read(id)` with `state.reality_portal_repo.mark_inquiry_read_for_realtor(id, principal.user_id)` (the scoped method at reality_portal.rs:779).
3. Map a zero-rows-affected / `Ok(false)` result to `StatusCode::NOT_FOUND` so a non-owning or non-existent inquiry returns 404 instead of 204 — mirror how `inquiries.rs:557 mark_as_read` reports ownership misses.
4. Confirm `mark_inquiry_read_for_realtor` reports affected-row count (or returns `Result<bool, _>`); if it returns `()` unconditionally, have it surface "no owned row" so the handler can return 404.
5. Leave the now-unused `reality_portal_repo.mark_inquiry_read(id)` method only if another caller exists; grep confirms `realtors.rs:256` is its sole caller, so delete the unscoped method to prevent re-introduction.
6. Add the regression test (see Test plan) extending the existing `inquiry_idor_tests.rs`.
7. Run `cargo test -p reality-server --test inquiry_idor_tests` against a seeded DB and confirm the new case fails on `main` and passes after the fix.

## Alternatives considered
- **Add a `realtor_id` predicate inline in the existing `mark_inquiry_read` query** — rejected because it duplicates the already-correct `mark_inquiry_read_for_realtor` method and leaves two near-identical queries to drift apart, which is how this second IDOR slipped past the PR #497 fix in the first place.
- **Rely on Postgres RLS to block the cross-tenant write** — rejected because reality_portal_repo uses the raw pool (no RLS session context on this path) and listing_inquiries ownership is by `realtor_id`, not an org RLS policy; the application-layer ownership check is the contract the sibling handlers already enforce.

## Root-cause trace
1. Symptom: realtor B can mark realtor A's inquiry `read` via `POST /api/v1/realtors/inquiries/{id}/read`, returning 204.
2. ← `backend/servers/reality-server/src/routes/realtors.rs:256` calls `mark_inquiry_read(id)` with only the path id, no principal scoping.
3. ← `backend/crates/db/src/repositories/reality_portal.rs:768` runs an UPDATE keyed solely on `id` (and `read_at IS NULL`), with no `realtor_id` predicate.
4. Origin: the unscoped `mark_inquiry_read` route predates the PR #497 ownership fix, which scoped only the parallel `inquiries.rs mark_as_read` route and added `mark_inquiry_read_for_realtor` without redirecting this second caller; PR #508 hardened `respond_to_inquiry` but did not touch this handler.

## Test plan
- [ ] `backend/servers/reality-server/tests/inquiry_idor_tests.rs` — add `mark_inquiry_read_rejects_non_owning_realtor`: realtor B marking realtor A's inquiry returns 404 and leaves `status`/`read_at` unchanged.
- [ ] Regression scenario: realtor A marking their own inquiry returns 204 and sets `read_at`; a second mark by A is idempotent (still 204, no error).
- [ ] `cargo test -p reality-server --test inquiry_idor_tests` (against a seeded Postgres via ppt-bridge `ppt_dev_up` + seed).

## Out of scope
- The `inquiries.rs` `mark_as_read` route (already scoped by PR #497).
- The dispute state-machine authz gap from issue #520 and any other realtors-surface handlers that already bind and use `principal`.
- Broader RLS adoption on the reality_portal repository — this plan only closes the single unscoped read-marking write.

## After-merge
- Move this file to `plans/_archive/security-realtors-mark-inquiry-read-idor.md`
- Mark the matching `backlog.json` row as `status: "done"`
