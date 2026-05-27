# test-gap-inquiry-idor-regression

**Vector:** test-gap
**Score:** 5
**Source:** PR #497 | backend/crates/db/src/repositories/reality_portal.rs:779
**Confidence:** high

## Hypothesis
PR #497 fixed a cross-tenant IDOR in the inquiry `mark_as_read` flow by adding an ownership `EXISTS` check in `mark_inquiry_read_for_realtor`, but it shipped with **zero regression tests** — the three acceptance-criteria TODOs in the PR body are all unchecked and no test file references inquiries. The fix is a two-step "ownership-EXISTS-then-UPDATE" pattern; a future refactor of either query (or a revert) could silently reintroduce the IDOR with no CI signal. The smallest change that closes the gap is a new integration test that fails if the ownership scoping is removed.

## Evidence
- `backend/crates/db/src/repositories/reality_portal.rs:779-804` — `mark_inquiry_read_for_realtor` does `SELECT EXISTS(... WHERE id=$1 AND realtor_id=$2)`, returns `Ok(false)` when not owned, else `UPDATE ... WHERE id=$1 AND read_at IS NULL` and returns `Ok(true)`.
- `backend/servers/reality-server/src/routes/inquiries.rs:557-570` — `mark_as_read` handler calls `mark_inquiry_read_for_realtor(id, principal.user_id)`; `Ok(false)` maps to 404, `Ok(true)` to 204.
- PR #497 body: 3 unchecked test TODOs — (a) realtor B → `PUT /api/v1/inquiries/{A_id}/read` → 404 and `read_at` stays NULL; (b) realtor A marks own → 204, `read_at` set; (c) idempotent re-mark → 204.
- `find backend -path '*test*' -name '*.rs' | xargs grep -l inquir` → only `backend/crates/db/tests/rls_smoke_tests.rs`; no test covers the mark-read ownership path.

## Files
- `backend/crates/db/src/repositories/reality_portal.rs:779`
- `backend/servers/reality-server/src/routes/inquiries.rs:557`
- `backend/servers/reality-server/tests/raw_pool_audit_tests.rs`

## Required capabilities
- [x] C1 — Systematic debugging
- [x] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode (auto-derived from the ticks):**
Mode: local-only (reason: integration tests need a live Postgres `DATABASE_URL` + `cargo test`, which the cloud sandbox lacks; runnable via the ppt-bridge MCP against a remote DB if available)

## Repro steps
1. On `main`, confirm there is no test exercising `mark_inquiry_read_for_realtor` ownership scoping: `grep -rn 'mark_inquiry_read_for_realtor' backend/servers/reality-server/tests backend/crates/db/tests` → no hits.
2. Mentally remove the `AND realtor_id = $2` clause (or the `if !owned { return Ok(false) }` guard) from `reality_portal.rs:785-794`. Today the test suite stays green — the regression is invisible. Expected after this plan: the new test fails (a foreign realtor would get 204 + `read_at` set instead of 404).

## Suggested approach
1. Create `backend/servers/reality-server/tests/inquiry_idor_tests.rs`, mirroring the harness style of `backend/servers/reality-server/tests/raw_pool_audit_tests.rs` (pool setup from `DATABASE_URL`, transactional fixtures).
2. Seed two realtors (A, B), a listing owned by A, and one inquiry on A's listing (so its `realtor_id` = A). Capture the inquiry id.
3. Test (a) — ownership denial: call `mark_inquiry_read_for_realtor(inquiry_id, B_id)`; assert it returns `Ok(false)` and that `SELECT read_at FROM listing_inquiries WHERE id=$1` is still NULL. (If exercising the HTTP layer, assert 404 from `PUT /api/v1/inquiries/{id}/read` as realtor B.)
4. Test (b) — positive path: call with `A_id`; assert `Ok(true)` and `read_at IS NOT NULL`, `status = 'read'`.
5. Test (c) — idempotent re-mark: call again with `A_id`; assert `Ok(true)` (owned) and that `read_at` is unchanged (the `AND read_at IS NULL` clause skips the second UPDATE).
6. Run `cargo test -p reality-server --test inquiry_idor_tests` and confirm all three pass; then temporarily delete the `realtor_id` scoping to confirm test (a) goes red (proves it's a real regression guard), and restore.
7. Reference PR #497 in the test-PR body and tick its three TODO boxes.

## Alternatives considered
- **Unit-test the SQL string only** — rejected because asserting on the query text doesn't prove the ownership check executes against real rows; only an integration test with seeded cross-tenant data catches a logic regression.
- **Add the assertion to `rls_smoke_tests.rs`** — rejected because that suite targets RLS-policy enforcement at the db crate; the inquiry mark-read scoping is an application-layer ownership check in reality-server and belongs in a server-level integration test next to `raw_pool_audit_tests.rs`.

## Root-cause trace
N/A — test-gap doesn't need backward tracing. (The underlying IDOR was already fixed by PR #497; this plan only adds the missing regression guard.)

## Test plan
- [ ] `backend/servers/reality-server/tests/inquiry_idor_tests.rs` — realtor B marks A's inquiry → `Ok(false)` / 404, `read_at` stays NULL (fails today if scoping removed)
- [ ] Same file — realtor A marks own → `Ok(true)` / 204, `read_at` set; re-mark → `Ok(true)`, `read_at` unchanged (idempotent)
- [ ] `cargo test -p reality-server --test inquiry_idor_tests` passes with the fix in place and test (a) goes red when the `realtor_id` scoping is removed

## Out of scope
- Changing the `mark_inquiry_read_for_realtor` implementation (the fix from #497 stays as-is).
- Adding ownership tests for `respond_to_inquiry` (POST `/{id}/respond`) — flagged separately by pm-qa; track as its own item.
- Any change to the deprecated `mark_inquiry_read(id)` method (reality_portal.rs:767).

## After-merge
- Move this file to `plans/_archive/test-gap-inquiry-idor-regression.md`
- Mark the matching `backlog.json` row as `status: "done"`
