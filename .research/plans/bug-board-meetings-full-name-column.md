# bug-board-meetings-full-name-column

**Vector:** bug
**Score:** 3
**Source:** Issue #1586
**Confidence:** high

## Hypothesis
Two queries in `backend/crates/db/src/repositories/board_meetings.rs` (`list_board_members` at line 138 and `list_motions` at line 895) select `u.full_name`, but the `users` table has no `full_name` column — only `name`. Both code paths will fail with Postgres `42703 undefined column "full_name"` at fetch time and return 500 on the board-member-list and motion-list endpoints. PR #1565 fixed the identical bug class in `document.rs`; this plan sweeps the remaining two occurrences in the same shape, and adds a failing-on-main integration test so it cannot silently re-ship.

## Evidence
- Issue #1586 (post-merge review of PR #1565) names the exact two call sites and the failure mode.
- `grep -n 'u\.full_name' backend/crates/db/src/repositories/board_meetings.rs` returns lines 138, 895.
- `users` schema has `name`, not `full_name` (per migrations sweep cited in issue body; same as fix shape in `document.rs` from PR #1565).
- PR #1565 merged 2026-06-18T09:39:26Z, task #1008 — direct precedent and fix shape.

## Files
- `backend/crates/db/src/repositories/board_meetings.rs`
- `backend/servers/api-server/src/routes/board_meetings.rs`
- `backend/servers/api-server/tests/board_meetings_auth_tests.rs`

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
1. Boot api-server against a seeded DB with at least one board meeting and one motion (the standard board-meeting fixture).
2. `GET /api/v1/board-meetings/{id}/members` — expected 200 + populated `user_name`/`user_email`; actual: 500 with Postgres `42703 column "u.full_name" does not exist`.
3. `GET /api/v1/board-meetings/{id}/motions` — same failure shape on the JOIN that aliases `u.full_name AS proposed_by_name`.

## Suggested approach
1. In `backend/crates/db/src/repositories/board_meetings.rs:138`, replace `u.full_name as user_name` with `u.name as user_name` (keep `u.email as user_email`).
2. In `backend/crates/db/src/repositories/board_meetings.rs:895`, replace `u.full_name as proposed_by_name` with `u.name as proposed_by_name`.
3. Run `cargo sqlx prepare --workspace` to refresh `.sqlx/` query data if these queries are macro-checked, then `cargo check -p db -p api-server`.
4. Extend `backend/servers/api-server/tests/board_meetings_auth_tests.rs` (or add a sibling `board_meetings_list_paths_tests.rs` next to the existing `board_meetings_cross_org_idor_tests.rs`) with an integration test that creates a meeting + 2 members + 1 motion, calls both list endpoints, and asserts 200 + populated `user_name` / `proposed_by_name`. The test must fail on `main` (run it once with the unfixed code to confirm IG3).
5. Sweep with `grep -rn 'u\.full_name' backend/crates/db/src/repositories/ backend/servers/` to confirm zero remaining DB references (the `airbnb.rs:824` `full_name` is an external API struct field and unrelated — leave it).
6. Update story coverage entry for #1565 if linked in `coverage.json` (mark sweep complete).

## Alternatives considered
- **Add a sqlx-checked typed view of users that aliases `name AS full_name` at the DB layer** — rejected because the column was simply renamed/wrong from the start; aliasing institutionalises a name the rest of the schema doesn't have and hides the bug from future readers.
- **Skip the regression test, rely on the existing document.rs fix as precedent** — rejected because the document-read path is already covered by `test_upload_metadata_roundtrips_through_get_handler`; the board-meetings list paths had no equivalent, which is exactly why this bug class slipped through to live routes.

## Root-cause trace
1. Symptom: 500 with `42703 undefined column "full_name"` on `GET /api/v1/board-meetings/{id}/members` and `.../motions`.
2. ← `backend/crates/db/src/repositories/board_meetings.rs:138` selects `u.full_name as user_name` against a `users` table that exposes only `name`.
3. ← `backend/crates/db/src/repositories/board_meetings.rs:895` same column reference in the motion-list query.
4. Origin: original board-meetings queries pre-dated PR #1565's awareness; the column was never created in any migration (mirror of the bug class PR #1565 fixed in `document.rs` for task #1008).

## Test plan
- [ ] New integration test: `backend/servers/api-server/tests/board_meetings_auth_tests.rs::test_list_members_and_motions_resolve_user_name` (or a new `board_meetings_list_paths_tests.rs` sibling) — fails on `main` (Postgres 42703), passes after the column rename.
- [ ] Regression: assert `user_name` and `proposed_by_name` are non-empty strings on the happy path; assert no 500 on the JOIN.
- [ ] Command: `cargo test -p api-server --test board_meetings_auth_tests` (or `--test board_meetings_list_paths_tests` if a new file is created).

## Out of scope
- The maintainability finding #3 in issue #1586 (shared SELECT fragment / sqlx `Type` derive) — that's a refactor for a separate plan once the immediate correctness bug is closed.
- Sweeping `full_name` references outside `backend/crates/db/src/repositories/` (e.g. the unrelated `airbnb.rs:824` external struct field).
- Adding board-meetings handler/route tests beyond the two list paths covered by the regression test.

## After-merge
- Move this file to `plans/_archive/bug-board-meetings-full-name-column.md`
- Mark the matching `backlog.json` row as `status: "done"`
