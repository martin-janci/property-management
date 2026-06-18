# test-gap-rental-rs-enum-cast-regression-pr-1537

**Vector:** test-gap
**Score:** 4
**Source:** PR #1537 (`fix(db): cast enum write paths on rental_guests + forms — encode + RETURNING, #1008`)
**Confidence:** medium

## Hypothesis
PR #1537 fixed two distinct WRITE-side enum encode bugs in the `#1008` family — `rental_guests.status` on `register_guest`/`register_guest_for_org`/`create_guest`/two `submit_report*` paths, and `forms.status` on `create_form`/`publish`/`archive` — but the test diff covered only the rental READ side that #1492 had already fixed. The `rental.rs` file is a churn hotspot this run, and an entire write-path family in `rental_guest_ical_enum_decode_tests` is now passing only because every new caller happens to use the `::guest_registration_status` cast. A future repo refactor that drops the cast on any of these methods will re-introduce a `42804` and re-red the same tests we just unblocked. The smallest preventive change is a tight regression test that exercises `register_guest_for_org` (the canonical org-scoped write) and asserts the RETURNING shape round-trips.

## Evidence
- `28402ee` (PR #1537) diff: `backend/crates/db/src/repositories/rental.rs:1289` `VALUES (… $18)` → `VALUES (… $18::guest_registration_status)` on the rental_guests INSERT; PR body lists five writer methods that needed the same cast and confirms the gap was reachable from `rental_guest_ical_enum_decode_tests::register_guest*_returning_decodes_status`. The merged diff carries no rental.rs-only test.
- `rental.rs` churn hotspot this run (2 changes in window; same file class as PR #1486 platform-cast follow-ups).
- The `forms.status` half of the PR is already covered by `backend/crates/db/tests/form_rls_repo_tests.rs::form_repo_force_rls_write_paths` (called out in the PR body). This plan does **not** add another forms test.

## Files
- `backend/crates/db/src/repositories/rental.rs:1289`
- `backend/crates/db/tests/rental_guest_ical_enum_decode_tests.rs`

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
1. `git stash` any local changes, then `git checkout 28402ee~1 -- backend/crates/db/src/repositories/rental.rs` to restore the un-cast `VALUES (…$18)` INSERT for `rental_guests`.
2. `cd backend && cargo test -p db --test rental_guest_ical_enum_decode_tests register_guest_for_org -- --nocapture`. Expected at `28402ee~1`: failure with PostgreSQL `42804 column "status" is of type guest_registration_status but expression is of type text`. After restoring HEAD: pass.
3. Confirm there is no test that *names* `register_guest_for_org` specifically — `grep -n register_guest_for_org backend/crates/db/tests/` returns the existing fixture name but no direct write-path assertion that survives a future revert of just the `$18::…` cast on that one method.

## Suggested approach
1. **Add `test_register_guest_for_org_persists_status_and_round_trips`** in `backend/crates/db/tests/rental_guest_ical_enum_decode_tests.rs`. Use the file's existing seed helpers; insert one guest via `RentalRepository::register_guest_for_org` with `status = GuestRegistrationStatus::Registered`. Assert: (a) the call returns `Ok(_)` (no 42804), (b) re-reading the row via `find_guest_by_id` returns `status == "registered"` as a decoded `String` (the #1008 round-trip).
2. **Add a query-level smoke** for `create_guest` INSERT shape — a tiny `#[sqlx::test]` that runs `SELECT pg_typeof($1::guest_registration_status)` and asserts it equals `'guest_registration_status'`. Acts as a freeze on the cast surface even if a future PR routes through a different writer.
3. **Verify locally:**
   ```bash
   cd backend
   cargo test -p db --test rental_guest_ical_enum_decode_tests
   ```
4. Leave the forms half untouched — `form_rls_repo_tests::form_repo_force_rls_write_paths` already exercises it (per PR body) and re-adding a parallel test would be duplicate evidence.

## Alternatives considered
- **Add a test per-method on rental.rs (`register_guest`, `register_guest_for_org`, `create_guest`, `submit_report_for_guest`, `submit_report_for_org_guest`)** — rejected because the cast surface is identical across the five callers; one canonical test on `register_guest_for_org` plus the type-of smoke covers the regression class without an LOC explosion that future repo splits would have to drag along.
- **Snapshot-test the SQL string produced by `register_guest_for_org`** — rejected on the same grounds as the document.rs plan: snapshotting query text freezes the *shape*, not the *behaviour*. A revert of the `::guest_registration_status` cast that retains the same column order would slip past a snapshot diff if someone normalised whitespace.

## Root-cause trace
1. Symptom: `rental_guest_ical_enum_decode_tests::register_guest_for_org_returning_decodes_status` red on dev before #1537.
2. ← `RentalRepository::register_guest_for_org` builds `INSERT INTO rental_guests (…, status) VALUES (…, $18)` at `backend/crates/db/src/repositories/rental.rs:1289`.
3. ← `$18` is bound as `&str` (`status.as_db_value()`), but the column type is `guest_registration_status`. With strict modes and the `#1008` sqlx-0.9 enum surface, the bind is encoded as `text` and Postgres rejects it: `42804`.
4. Origin: when PR #1492 cast the READ side (`GUEST_COLUMNS RETURNING status::text`), the WRITE side of the same family was overlooked. PR #1537 closes the gap; this plan adds the regression guard.

## Test plan
- [ ] `backend/crates/db/tests/rental_guest_ical_enum_decode_tests.rs::test_register_guest_for_org_persists_status_and_round_trips` (new)
- [ ] `backend/crates/db/tests/rental_guest_ical_enum_decode_tests.rs::test_register_guest_for_org_status_column_typeof_is_enum` (new)
- [ ] `cd backend && cargo test -p db --test rental_guest_ical_enum_decode_tests`

## Out of scope
- Forms tests — already covered by `form_rls_repo_tests::form_repo_force_rls_write_paths` per PR #1537 body.
- Re-running the platform-cast follow-ups (#1486) — that file family was already locked in by `rental_enum_decode_tests`.
- The READ-side `RETURNING {GUEST_COLUMNS}` — covered by #1492's prior tests.
- Any HTTP-layer test through `routes/portal_rentals.rs` (handler-shape would re-test serde, not enum encode).

## After-merge
- Move this file to `plans/_archive/test-gap-rental-rs-enum-cast-regression-pr-1537.md`
- Mark the matching `backlog.json` row as `status: "done"`
