# code-review-api-handlers-unit-residents-add-xorg-user

**Vector:** security
**Score:** 2
**Source:** rotating-expert-review (dispatcher Tier-1d 2026-11-08 api-handlers)
**Confidence:** high

## Hypothesis
`add_resident` in `backend/servers/api-server/src/routes/unit_residents.rs` validates
the incoming `req.user_id` only for *global* existence (`state.user_repo.find_by_id`) and
never checks that the user belongs to the unit's organization before calling
`unit_resident_repo.create`. The manager-gated check established upstream by
`authorize_unit(..., require_manager=true)` guards the caller, not the target user. A
manager can therefore attach a user from a *different* organization as a resident of a
unit in their own building; the attached resident carries `receives_notifications` /
`receives_mail` flags, so the foreign user begins receiving cross-tenant unit
communications and is linked into the unit's resident roster. Smallest safe fix:
after the existence check at L418-438, call `org_member_repo.is_member(<building.organization_id>, req.user_id)`
— reusing the org id already resolved inside `authorize_unit` — and reject with
`USER_NOT_MEMBER` otherwise.

## Evidence
- `backend/servers/api-server/src/routes/unit_residents.rs:418-438` — `find_by_id` is the
  only validation applied to `req.user_id`; confirmed in this run by reading L410-465 directly.
- `backend/servers/api-server/src/routes/unit_residents.rs:452-463` — `unit_resident_repo.create`
  is called immediately after the existence check with `receives_notifications` /
  `receives_mail` propagated verbatim, so a cross-org attach immediately subscribes the
  foreign user to the unit's notification stream.
- `backend/servers/api-server/src/routes/unit_residents.rs:79-92` — `authorize_unit_inner`
  already demonstrates the correct pattern: it gates callers via
  `org_member_repo.is_member(building.organization_id, auth.user_id)`. The same org id
  is reachable inside `add_resident` (`authorize_unit` returns the unit->building->org chain).
- Contrast: `backend/servers/api-server/src/routes/delegations.rs` and
  `backend/servers/api-server/src/routes/energy.rs` both run explicit membership gates on
  their target users; `unit_residents::add_resident` is the odd one out.

## Files
- `backend/servers/api-server/src/routes/unit_residents.rs:418`
- `backend/servers/api-server/src/routes/unit_residents.rs:79`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. Seed two organizations A and B. In org A seed a unit with manager `mgr_a`; in org B seed a non-member user `user_b`.
2. Authenticate as `mgr_a` and POST `/api/v1/units/<unit_a>/residents` with body `{ "user_id": "<user_b>", "resident_type": "tenant", "receives_notifications": true, "receives_mail": false }`.
3. Expected: `400 Bad Request` with code `USER_NOT_MEMBER`. Actual: `201 Created` — `user_b` now shows up in the unit's resident roster and starts receiving org-A unit notifications.

## Suggested approach
1. Extend the tuple returned by `authorize_unit` (or the already-propagated `building` context inside `add_resident`) to carry `organization_id` into the handler scope. If the helper already returns it, bind it explicitly as `let org_id = building.organization_id;`.
2. After the existence check at L418-438, call `state.org_member_repo.is_member(org_id, req.user_id).await` — preserve the surrounding `rls.release().await` idiom on error.
3. On `Ok(false)`, return `StatusCode::BAD_REQUEST` with `ErrorResponse::new("USER_NOT_MEMBER", "Resident user is not a member of this organization")`.
4. On the repository `is_member` failure, log via `tracing::error!` and return `DB_ERROR` matching the existing style at L423-429.
5. Add a `tracing::warn!(org_id = %org_id, target = %req.user_id, "cross-org resident attach rejected")` on the rejection so the signal is observable for SOC.

## Alternatives considered
- **Enforce the gate purely at the DB layer with a trigger on `unit_residents.user_id`** — rejected: Postgres can't cheaply check "is member of building's org" without an extra lookup, and this hides the policy from the handler layer where the existing authz helpers live.
- **Narrow the fix to `receives_mail=true` cases only** — rejected: residency is itself cross-tenant data linkage; the flag-conditional carve-out would still leak the unit roster.

## Root-cause trace
1. Symptom: a manager in org A attaches `user_b` (org B) as a resident of `unit_a`; `user_b` appears on `GET /units/<unit_a>/residents` and receives notifications about `unit_a`.
2. ← `routes/unit_residents.rs:418-438` — only a global existence check; no org gate before create.
3. ← `routes/unit_residents.rs:79-92` — the correct pattern exists on the authz side but was not applied to `req.user_id`; the target user bypass is the regression.
4. Origin: initial `add_resident` implementation. The handler was authored before the `org_member_repo` helper landed; subsequent sweeps picked up `delegations.rs` and `energy.rs` but missed `unit_residents.rs` on the member-gate pass.

## Test plan
- [ ] `backend/servers/api-server/tests/suites/unit_residents_tests.rs` — new case
      `add_resident_rejects_cross_org_user`: seeds two orgs, a unit under org-A with
      manager `mgr_a`, and a non-member `user_b` in org-B; asserts `400 Bad Request` +
      `USER_NOT_MEMBER` and that no `unit_residents` row is created.
- [ ] Same file — regression case `add_resident_allows_same_org_user` to protect the
      happy path (201 + row created).
- [ ] Exact command: `cargo test -p api-server --test suites unit_residents_tests::add_resident_rejects_cross_org_user`.

## Out of scope
- Backfilling the org-membership check on `update_resident` / `remove_resident` (handler
  code already looks up the resident row via RLS, so pre-existing stale cross-org rows
  are the follow-up; file a separate backlog row if one surfaces during repro).
- Changing the `receives_notifications` / `receives_mail` semantics.
- Refactoring the shared `org_member_repo.is_member` helper.

## After-merge
- Move this file to `plans/_archive/code-review-api-handlers-unit-residents-add-xorg-user.md`
- Mark the matching `backlog.json` row as `status: "done"`
