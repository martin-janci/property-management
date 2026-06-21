# bug-ci-red-voting-pdf-1645

**Vector:** bug
**Score:** 3
**Source:** Issue #1645 | PR #1625 (commit `3c63a6d3`)
**Confidence:** high

## Hypothesis
PR #1625 (voting PDF, Story 5.6/5.8) merged to `dev` with its own two new `pdf_report::*` tests red, leaving `backend.yml` continuously failing since 2026-06-20 22:24 UTC. The panic is at `tests/common/mod.rs:507` (asserting `201` from `create_authenticated_user_with_org`) — so the regression is in test *setup*, almost certainly a schema/seed inconsistency the voting-PDF migration introduced (the same tests query `documents WHERE category = 'reports'::document_category`). Every backend PR is now blocked because the squash-merge gate runs the full workspace suite. Smallest fix: repair the migration/seed so test setup returns 201, then re-run `backend.yml` on `dev`.

## Evidence
- Issue #1645 — "dev backend CI red since #1625 (voting PDF) — pdf_report tests 500, blocking all backend merges"
- PR #1625 squash-merge commit `3c63a6d3` (2026-06-20 22:24 UTC) is the first red on `backend.yml` `dev`-push runs; previous tip `7648d4d2` was green
- `backend/servers/api-server/tests/voting_tests.rs:248` — `mod pdf_report` block added by #1625 with the two failing tests
- `backend/servers/api-server/tests/common/mod.rs:507` — `assert_eq!(register_resp.status, StatusCode::CREATED)` panicking with `left: 500, right: 201`
- Approved dispatcher PR #1633 (Booking.com OTA, backend, unrelated) failed CI on these exact tests on 2026-06-21 — confirms the regression is on `dev`, not in any individual PR

## Files
- `backend/servers/api-server/src/routes/voting.rs:1659`
- `backend/servers/api-server/tests/voting_tests.rs:248`
- `backend/servers/api-server/tests/common/mod.rs:507`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging
- [ ] C2 — Seed data
- [x] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. From a clean checkout of `origin/dev` (commit `716e2b6` or later), run:
   ```
   cd backend && cargo test -p api-server --test voting_tests pdf_report
   ```
2. Expected: both `pdf_report::test_get_report_pdf_returns_application_pdf_and_archives_document` and `pdf_report::test_get_report_json_with_format_pdf_returns_application_pdf` pass.
3. Actual: setup panics at `tests/common/mod.rs:507` with `assertion left == right failed: left: 500, right: 201` — `create_authenticated_user_with_org` returns HTTP 500, suggesting `/api/v1/auth/register` is 500-ing because of a schema inconsistency introduced by #1625's migration.

## Suggested approach
1. Identify the migration(s) PR #1625 added/changed under `backend/crates/db/migrations/` (likely a `documents`/`voting_reports` schema change touching `document_category` enum or a new column with a missing default).
2. Run the failing test with `RUST_LOG=debug cargo test -p api-server --test voting_tests pdf_report::test_get_report_pdf_returns_application_pdf_and_archives_document -- --nocapture` to capture the server-side panic / SQL error behind the 500.
3. Reproduce the 500 at `/api/v1/auth/register` against the freshly-migrated test DB to pinpoint the failing SQL statement (look for missing column / enum value / NOT NULL without default).
4. Patch the migration in place if it's the same migration as #1625 still on top of `dev` (squash-merge means the SHA is fresh) **or** add a new migration that finishes the schema correctly — prefer the latter if any other PR has landed since.
5. Verify locally: `cargo test -p api-server --test voting_tests pdf_report` passes; full suite (`cargo test --workspace`) passes.
6. Open a `fix(voting): …` PR; once green, force-re-run `backend.yml` on `dev` (or wait for the next push) and confirm the tip goes green.

## Alternatives considered
- **Revert #1625 entirely** — rejected because it's a shipped story (5.6/5.8) with a PDF download UI already on top of it; reverting cascades downstream. Surgical schema repair is cheaper.
- **Gate the failing tests with `#[ignore]`** — rejected because the failure is in test setup, which means the production code path is also broken for any 500-returning operation that exercises this schema. Hiding the test hides a real bug.

## Root-cause trace
1. Symptom: `backend.yml` `test` job red on every push since `3c63a6d3` — two `pdf_report::*` tests fail.
2. ← Setup panic at `backend/servers/api-server/tests/common/mod.rs:507` — `create_authenticated_user` POSTs `/api/v1/auth/register` and gets HTTP 500 instead of 201.
3. ← The 500 is most likely a SQL error from a schema invariant violated by #1625's migration (suspected: `documents.category` enum extension or related table change, since the same tests later query `documents WHERE category = 'reports'::document_category` at line 324).
4. Origin: PR #1625 commit `3c63a6d3` (2026-06-20 22:24 UTC) — `feat(voting): PDF generation + archive for vote reports (PAP-288, Story 5.6/5.8)`.

## Test plan
- [ ] `cargo test -p api-server --test voting_tests pdf_report` — both subtests pass (currently red)
- [ ] `cargo test --workspace` — full suite green (regression guard for unrelated touches)
- [ ] CI: next `backend.yml` push run on `dev` succeeds (the merge unblocks stalled PRs)

## Out of scope
- Any cosmetic changes to the rendered PDF
- Refactoring `voting.rs:1659 render_pdf_report`
- Touching unrelated voting routes / migrations

## After-merge
- Move this file to `plans/_archive/bug-ci-red-voting-pdf-1645.md`
- Mark `bug-ci-red-voting-pdf-1645` row in `.research/backlog.json` as `status: "done"`
