# security-portal-principal-agency-bypass

**Vector:** security
**Score:** 3
**Source:** Issue #1584 · PR #1561 (post-merge reviewer finding) · `routes/imports.rs` vs `routes/agency_imports.rs`
**Confidence:** high

## Hypothesis
`PortalPrincipal` (added in PR #1561) drops the codebase's real agency-authorization model. The sibling route family (`agency_imports.rs`) gates every action on an explicit `reality_agency_members` membership check; the new `imports.rs` path authorizes purely by `user_id` equality against the resource's owner column. The fix only works because `feed_subscriptions.agency_id` happens to store a *user* id, so a legitimate second member of the same agency cannot see that agency's feeds/jobs at all — only the single user whose UUID equals the agency id can. Reconciling the two import surfaces onto one membership-based authorization model (or renaming the per-user surface so the intent is explicit) eliminates the divergent authz and unblocks multi-user agencies.

## Evidence
- `backend/servers/reality-server/src/routes/imports.rs` — 11 handlers thread `principal.user_id` through every repo call (jobs + feeds) and 200/404 turns on `WHERE id = $1 AND user_id = $2` predicates, *not* on agency membership.
- `backend/servers/reality-server/src/routes/agency_imports.rs` — sibling surface gates every action on `SELECT 1 FROM reality_agency_members WHERE agency_id = $1 AND user_id = $2 AND is_active = TRUE` (403 on miss).
- `backend/servers/reality-server/tests/imports_idor_tests.rs` — comment explicitly acknowledges the mismatch: "GH #1300 finding 2 pre-existing mismatch", test registers agency UUIDs as users so scoping coincides.
- Migration 00063 — `feed_subscriptions.agency_id` is `REFERENCES reality_agencies(id)` but the live flow populates it with `principal.user_id`.
- `backend/crates/api-core/src/extractors/principal.rs` — `PortalPrincipal` duplicates `RequestPrincipal`'s JWT decode + users lookup + `principal_kind` re-derivation.

## Files
- `backend/servers/reality-server/src/routes/imports.rs`
- `backend/servers/reality-server/src/routes/agency_imports.rs`
- `backend/crates/db/src/repositories/reality_portal.rs`
- `backend/crates/api-core/src/extractors/principal.rs`
- `backend/servers/reality-server/tests/imports_idor_tests.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [x] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [x] C7 — Code-review reception (tick if you expect controversy)

**Execution mode (auto-derived from the ticks):** no C4/C5 → cloud-ok.

Mode: cloud-ok

## Repro steps
1. Seed: tenant agency A with two members M1 (user_id == agency_id by today's accident) and M2 (distinct user_id). Create a feed subscription S owned by A.
2. As M2 (legitimate second member of A), call `GET /api/v1/reality/imports/feeds/{S.id}` against reality-server.
3. Expected (correct authz): `200 OK` — M2 is an active member of the owning agency.
4. Actual (on `dev`): `404 NOT_FOUND` — query is `WHERE id = $1 AND user_id = $2` against M2's id, which doesn't match the agency_id-as-user-id stored on S.

## Suggested approach
1. Extract a shared `assert_agency_member(conn, agency_id, user_id) -> Result<(), 403>` helper from `agency_imports.rs` into a sibling module (e.g. `routes/agency_membership.rs` or `api-core::auth`).
2. In `routes/imports.rs`, for each by-id handler: resolve the resource's owning agency (one SQL fetch on the row, no agency context needed), then call `assert_agency_member(conn, owning_agency, principal.user_id)`. Replace the current `user_id` scoping in repository methods with `agency_id` scoping.
3. Update `backend/crates/db/src/repositories/reality_portal.rs` by-id methods to take `agency_id` (semantic) instead of `user_id`; rename parameters so the column intent is explicit.
4. Update `imports_idor_tests.rs`: drop the "register agency UUIDs as users" workaround; add a positive test asserting a second active member of the agency sees the feed (the regression case this plan fixes).
5. Pin platform-admin handling: add a one-line test in `imports_idor_tests.rs` asserting `platform`-kind callers are still 404 on another user's resource (Issue #1584 finding 3).
6. Factor the shared JWT-decode + kind-re-derivation from `PortalPrincipal` and `RequestPrincipal` into a private helper in `api-core/src/extractors/principal.rs`, or make tenant/membership enforcement a mode of a single extractor (Issue #1584 finding 4 — security-critical decode in one place).
7. Track the schema reconciliation (column `agency_id` is FK to `reality_agencies(id)` but stores user ids today) as a follow-up migration row — *not* in this plan; this plan stops at consistent authz semantics.

## Alternatives considered
- **Rename to "per-user imports" surface and document** — rejected because the column has an FK to `reality_agencies(id)` and the sibling route family already implements per-agency semantics; renaming would entrench the data-model mismatch instead of resolving it.
- **Add `feed_subscriptions.user_id` and authorize on it directly** — rejected because it duplicates ownership state with no migration plan and still leaves multi-member agencies broken; the authoritative ownership lives in `reality_agency_members`.

## Root-cause trace
1. Symptom: second member of an agency gets 404 on their own agency's feeds.
2. ← `routes/imports.rs` by-id handlers use `WHERE id = $1 AND user_id = $2` (member's user_id) against rows where the stored "user_id" is actually the agency_id.
3. ← `feed_subscriptions.agency_id` column populated with `principal.user_id` despite FK to `reality_agencies(id)` (migration 00063 vs runtime writes).
4. Origin: PR #1561 (merged 2026-06-18) introduced `PortalPrincipal` and dropped the membership check that `agency_imports.rs` still uses, picking the simpler `user_id`-equality predicate to fix the cross-user IDOR without addressing the underlying agency-vs-user column semantics.

## Test plan
- [ ] New test in `backend/servers/reality-server/tests/imports_idor_tests.rs`: `feed_visible_to_second_active_agency_member` — seeds A with M1, M2, S; M2 → 200 (regression test, fails on `dev`).
- [ ] Update existing tests: drop the "register agency UUIDs as users" workaround in `imports_idor_tests.rs::seed_helpers` to keep the test surface honest after the authz change.
- [ ] Pin platform-admin: `imports_idor_tests.rs::platform_admin_still_404_on_other_users_feed`.
- [ ] Cross-agency negative: `imports_idor_tests.rs::cross_agency_member_gets_403` (was 404; assert the membership-check 403 path).
- [ ] Command: `cargo test -p reality-server --test imports_idor_tests` (and the broader `cargo test -p reality-server`).

## Out of scope
- The `feed_subscriptions.agency_id` schema migration (column-vs-value mismatch) — tracked as a separate plan once authz is consistent; this plan does not move data.
- Caching the per-request `users` lookup (Issue #1584 finding 5, performance) — revisit if the import endpoints show up in latency profiles.

## After-merge
- Move this file to `plans/_archive/security-portal-principal-agency-bypass.md`
- Mark the matching `backlog.json` row as `status: "done"`
