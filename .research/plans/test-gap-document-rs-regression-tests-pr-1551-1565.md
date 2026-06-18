# test-gap-document-rs-regression-tests-pr-1551-1565

**Vector:** test-gap
**Score:** 8
**Source:** PR #1551 (`fix(db): cast document_category in count_accessible_rls` — closes #1528), PR #1565 (`fix(db): correct document get-by-id read path — u.name + enum ::text decode`)
**Confidence:** medium

## Hypothesis
Two consecutive fix-only PRs landed on `backend/crates/db/src/repositories/document.rs` in 48 h without a regression test for either bug — both are in the `#1008` enum-encode/decode family that has bitten this repo repeatedly. The file is also a top-3 churn hotspot (runs_seen=2 triggers `repeated-churn` this run). Without a failing-on-`HEAD~PR` test for each fix, a future repo-layer refactor can silently re-introduce the same 500/42703/42883 errors. Adding two small regression tests on the existing test infrastructure closes both gaps for the cost of one PR.

## Evidence
- `bb61766` (PR #1565) diff: replaced `u.full_name` → `u.name` and added `d.category::text AS category_text` / `d.access_scope::text AS access_scope_text` aliases inside `find_by_id_with_details_rls` (`backend/crates/db/src/repositories/document.rs:596`). Body notes `test_upload_metadata_roundtrips_through_get_handler` was the failing test on dev that motivated the fix — that test currently passes only because the PR shipped without a regression-guard for the underlying SELECT shape.
- `fdee14c` (PR #1551) diff: `count_accessible_rls` at `document.rs:805` got `category = $7` → `category = $7::document_category` (closes #1528). PR body: "the lone document-category filter left uncast — so a non-manager listing with `?category=` filter 42883'd on the COUNT query." No new test file in the diff.
- Both PRs touch the same repo file; the file is in `state.hotspot_history` (`runs_seen` becomes 2 this run, `recent_churn=2940` from the 2026-06-10 window).
- Cumulative signals this run: `hotfix-no-test-pr-1551-document-rs` (+2), `hotfix-no-test-pr-1565-document-rs` (+2), `risky-churn-pr-1551-document-rs` (+2), `risky-churn-pr-1565-document-rs` (+2). Capped at 8.

## Files
- `backend/crates/db/src/repositories/document.rs:596`
- `backend/crates/db/src/repositories/document.rs:805`
- `backend/servers/api-server/tests/document_upload_tests.rs:1146`
- `backend/crates/db/tests/documents_rls_cross_tenant_tests.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [x] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

**Execution mode (auto-derived from the ticks):**

Mode: cloud-ok

## Repro steps
1. Check out the parent commit of PR #1565 (`git checkout bb61766^ -- backend/crates/db/src/repositories/document.rs`) so `find_by_id_with_details_rls` still SELECTs `u.full_name` and reads `category`/`access_scope` directly from `d.*`.
2. Run `cd backend && cargo test -p api-server --test document_upload_tests test_upload_metadata_roundtrips_through_get_handler -- --nocapture`. Expected (before fix): test fails — `Failed to get document` / 42703 `users.full_name` does not exist, or `ColumnDecode` on `category`. After fix: 18/18 pass.
3. Now check out PR #1551's parent for `count_accessible_rls` only; with a `?category=…` filter the COUNT query returns `42883 operator does not exist: document_category = text` on a non-manager listing — that is the gap a new test must cover (the existing tests at HEAD don't traverse the category-filter path).

## Suggested approach
1. **Test 1 — SELECT shape lock** for `find_by_id_with_details_rls` (`document.rs:596`). Add `test_find_by_id_with_details_rls_decodes_category_and_user_name` to `backend/crates/db/tests/documents_rls_cross_tenant_tests.rs`. Seed: one document with `category = 'lease_agreement'`, `access_scope = 'organization'`, `created_by` pointing to a user row (`users.name = 'Alice'`). Assert `doc.category == "lease_agreement"`, `doc.access_scope == "organization"`, `doc.created_by_name.as_deref() == Some("Alice")`. The test fails on the pre-#1565 SELECT (`u.full_name` not present + raw enum decode).
2. **Test 2 — category filter via count_accessible_rls** (`document.rs:805`). Extend the same test file (or `document_access_rls_tests.rs`) with `test_count_accessible_rls_with_category_filter_does_not_42883`. Seed: 2 documents (`policy`, `lease_agreement`) accessible to a non-manager user; call `count_accessible_rls` with `category = Some("policy")`. Assert returns `Ok(1)` and never errors with `42883`. The test fails on the pre-#1551 query (operator-not-exists at COUNT).
3. **Verify locally:**
   ```bash
   cd backend
   cargo test -p api-server --test document_upload_tests test_upload_metadata_roundtrips_through_get_handler
   cargo test -p db --test documents_rls_cross_tenant_tests
   cargo test -p db --test document_access_rls_tests
   ```
4. Update `evidence` row to point at the two test names; do **not** alter the fixes themselves — this plan is regression-only.

## Alternatives considered
- **Single integration test through the HTTP handler (`/api/v1/documents/{id}` + `/api/v1/documents?category=…`)** — rejected because the bug is repo-layer (SELECT shape / SQL cast). HTTP-layer tests add fixture cost (auth, JWT, MinIO seed) for the same assertion power.
- **Snapshot test of the raw `find_by_id_with_details_rls` query string** — rejected because it freezes the SQL text rather than the *behaviour*; a benign rename (e.g. `category_text` → `category_t`) would needlessly re-tick the snapshot without protecting against the actual class of bug (42703 / ColumnDecode / 42883).

## Root-cause trace
1. Symptom (pre-#1565): GET `/api/v1/documents/{id}` returns 500 "Failed to get document"; `test_upload_metadata_roundtrips_through_get_handler` red on dev.
2. ← Handler bubbles `Err` from `DocumentRepository::find_by_id_with_details_rls` at `backend/crates/db/src/repositories/document.rs:596`.
3. ← Two latent shape bugs in the same SELECT: `u.full_name` (users column is `name` since migration 00001) and `d.*` returning raw enums into `String` struct fields (the #1008 decode class).
4. Origin: the file's natural growth — `find_by_id` etc. were converted to RLS variants in the PAP-110/PAP-112 sweeps (the 2026-06-10 churn window) and one method ended up out of sync with its siblings. PR #1551 (count_accessible_rls cast) is the parallel bug on the WHERE side of the same enum-encode family.

## Test plan
- [ ] `backend/crates/db/tests/documents_rls_cross_tenant_tests.rs::test_find_by_id_with_details_rls_decodes_category_and_user_name` (new)
- [ ] `backend/crates/db/tests/document_access_rls_tests.rs::test_count_accessible_rls_with_category_filter_does_not_42883` (new)
- [ ] `cd backend && cargo test -p db --test documents_rls_cross_tenant_tests && cargo test -p db --test document_access_rls_tests && cargo test -p api-server --test document_upload_tests`

## Out of scope
- Re-shaping `find_by_id_with_details_rls`'s SELECT further (the current `category::text AS category_text` aliasing is fine; we are testing it, not refactoring).
- Touching the deprecated `find_by_id` wrapper at `document.rs:1291`.
- Sibling repos with the same `find_by_id_with_details_rls` pattern (announcement.rs, outage.rs) — they were not in the hotspot window. File separate items if they regress.
- Any test for the `document_shares` `u.name` rename in PR #1565 (`document.rs:1561`) — covered implicitly by existing share-listing tests.

## After-merge
- Move this file to `plans/_archive/test-gap-document-rs-regression-tests-pr-1551-1565.md`
- Mark the matching `backlog.json` row as `status: "done"`
