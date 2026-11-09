# code-review-api-handlers-unit-residents-add-xorg-user

**Vector:** security
**Score:** 2
**Source:** rotating-expert-review (Tier-1d api-handlers 2026-11-08) · verified in-repo 2026-11-09 · signal `code-review-api-handlers-unit-residents-add-xorg-user`
**Confidence:** high

## Hypothesis
`add_resident` (POST `/api/v1/units/{unit_id}/residents`) validates the incoming `req.user_id` only for global existence (`state.user_repo.find_by_id`, non-RLS) at `routes/unit_residents.rs:432-438`. The caller is gated by `authorize_unit(..., require_manager=true)`, which confirms the CALLER is an org member, but the resident being added is not checked against the unit's organization. A manager can therefore attach a user from any other organization as owner/tenant/family_member/subtenant of their unit, then exercise resident-only actions (in-app announcements, faults, voting ballots scoped to that unit) through that outside principal. The minimal fix is to call `state.org_member_repo.is_member(building.organization_id, req.user_id)` after the existence check and return `400 OUT_OF_ORG` on `false` — exactly the gate `authorize_unit_inner` already applies to the caller at L79-84.

## Evidence
- `backend/servers/api-server/src/routes/unit_residents.rs:418-438` — `add_resident` checks only `user_repo.find_by_id(req.user_id)` (global existence); no `is_member` call follows before the `CreateUnitResident` insert.
- `backend/servers/api-server/src/routes/unit_residents.rs:79-84` — canonical per-caller org gate: `state.org_member_repo.is_member(building.organization_id, user_id).await` returns a `403 NOT_ORG_MEMBER` on `false`.
- `backend/servers/api-server/src/routes/unit_residents.rs:44-50` — `authorize_unit` resolves the owning `building` (and thus `organization_id`) and is already called at the top of `add_resident`, so the `organization_id` is already in scope — no extra query needed.
- `backend/servers/api-server/src/state.rs:278` — `org_member_repo: OrganizationMemberRepository` on `AppState`.
- `resident_type` values at L418-420: `OWNER | TENANT | FAMILY_MEMBER | SUBTENANT`. Allowing any of these across org boundaries leaks resident-scoped data (voting, announcements, unit documents) to the outside principal.

## Files
- `backend/servers/api-server/src/routes/unit_residents.rs:432`
- `backend/servers/api-server/tests/suite_2.rs`
- `backend/servers/api-server/tests/suites`

## Required capabilities
- [x] C1 — Systematic debugging
- [x] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. Seed two organizations `org_a`, `org_b`, each with one user. Create one unit `unit_a` in a building under `org_a`.
2. Promote `manager_a` to manager of `unit_a` so `authorize_unit(..., require_manager=true)` passes.
3. `POST /api/v1/auth/login` as `manager_a` → bearer.
4. `POST /api/v1/units/{unit_a.id}/residents` with body `{"user_id":"<other_b.id>","resident_type":"owner","is_primary":true}`.
5. Expected: `400 OUT_OF_ORG` (resident is not an `org_a` member). Actual on pre-fix: `201` with a persisted `unit_residents` row attaching `other_b` as owner of `unit_a`.

## Suggested approach
1. In `routes/unit_residents.rs:add_resident`, right after the `user_exists` check at L432-438, reuse the `building` already resolved by `authorize_unit` at the top of the handler (the helper returns the building from which `organization_id` is accessible).
2. Call `state.org_member_repo.is_member(building.organization_id, req.user_id).await`. Error path: `500 DB_ERROR` identical to L193-198 in `delegations.rs`. On `Ok(false)`: `400 OUT_OF_ORG` with body `ErrorResponse::new("OUT_OF_ORG","Resident user is not a member of this organization")`. On `Ok(true)`: fall through to the `CreateUnitResident` insert.
3. Add a regression suite `backend/servers/api-server/tests/suites/unit_residents_xorg_tests.rs` and register it in `tests/suite_2.rs` using the existing `#[path = …]` pattern (see the recently-added `change_password_session_revocation_tests` in PR #3353 for the shape).
4. Keep the shape consistent with the sibling delegations fix so the two end up interoperable for reviewers (same error code, same gate call).
5. `cargo clippy -p api-server --all-targets -- -D warnings` and `cargo fmt --all` before pushing.

## Alternatives considered
- **Gate inside a resident-create middleware** — rejected because the resident's organization is only known after `authorize_unit` resolves the building; a generic middleware does not have that context and would duplicate lookup.
- **Add a DB-level check-constraint** — rejected because a constraint would reject the insert too late to return a meaningful 400 code and would conflict with cross-org admin flows (e.g. superadmin backfills).

## Root-cause trace
1. Symptom: `POST /api/v1/units/{unit}/residents` with a cross-org `user_id` succeeds; the `unit_residents` row attaches an outside principal as `owner`/`tenant`/`family_member`/`subtenant` of a unit inside a different organization.
2. ← `routes/unit_residents.rs:432-438` — only `user_repo.find_by_id(req.user_id)` runs; no `is_member` gate.
3. ← `routes/unit_residents.rs:44-50 authorize_unit` — gates the CALLER only, not `req.user_id`, so the caller's org membership check does not protect the resident being added.
4. Origin: the `add_resident` handler reused the `authorize_unit` wrapper without re-applying its internal `is_member` gate on the second principal.

## Test plan
- [ ] `backend/servers/api-server/tests/suites/unit_residents_xorg_tests.rs` — `add_resident_rejects_out_of_org_user` (fails on pre-fix: 201 OK; passes after: 400 OUT_OF_ORG).
- [ ] Same suite — `add_resident_allows_same_org_user` (positive regression so we do not over-tighten).
- [ ] Same suite — `add_resident_still_requires_manager` (unchanged: caller without manager role gets 403 before the new gate runs).
- [ ] Command: `cargo test -p api-server --test suite_2 unit_residents_xorg_tests -- --nocapture`

## Out of scope
- `update_resident` / `remove_resident` endpoints — audit separately; this plan is scoped to the add path.
- Resident type permission mapping (what each role sees post-add) — treat as a follow-up signal if the end-to-end review surfaces it.
- Admin / god-mode cross-org attachments — if required, carve out via a dedicated super-admin route rather than loosening this gate.

## After-merge
- Move this file to `plans/_archive/code-review-api-handlers-unit-residents-add-xorg-user.md`
- Mark the matching `backlog.json` row as `status: "done"`
