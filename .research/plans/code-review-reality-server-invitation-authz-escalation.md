# code-review-reality-server-invitation-authz-escalation

**Vector:** security
**Score:** 3
**Source:** dev-review 2026-08-05 (reality-server), files backend/servers/reality-server/src/routes/agencies.rs and backend/crates/db/src/repositories/reality_portal/agencies.rs
**Confidence:** high

## Hypothesis

`POST /api/v1/agencies/{id}/invitations` (`create_invitation` at agencies.rs:373) is authenticated but performs no membership check on the caller against `{id}`, and the client-supplied `role` passes through the repo `INSERT` verbatim. Compounding that, `accept_invitation` (repo agencies.rs:256) selects the invitation by `token` alone and inserts a `reality_agency_members` row keyed on the *accepting* principal's `user_id`, never comparing `invitation.email` to the accepting user's email. Together, any authenticated portal user can invite themself into any agency as `admin` and become a real member — full `update_agency` / `update_branding` / `run_import` (SSRF surface) power. The fix mirrors the sibling routes that already call `check_agency_membership` on the caller and adds an email-match guard at accept time.

## Evidence
- `backend/servers/reality-server/src/routes/agencies.rs:373-401` — `create_invitation` never calls `super::agency_imports::check_agency_membership(&mut conn, id, principal.user_id)`, unlike `update_agency` (`agencies.rs:268`) and `update_branding` (`agencies.rs:322`) which both do.
- `backend/crates/db/src/repositories/reality_portal/agencies.rs:227-253` — `create_invitation` binds `data.role` with only `COALESCE($3, 'realtor')`; `admin` and `owner` pass through.
- `backend/crates/db/src/repositories/reality_portal/agencies.rs:256-293` — `accept_invitation` selects by token, inserts on the accepting `user_id`, never checks `invitation.email == accepting_user.email`.
- Reference fix in-repo: `agencies.rs:268` (membership check pattern already used by `update_agency` / `update_branding`).

## Files
- `backend/servers/reality-server/src/routes/agencies.rs:373`
- `backend/crates/db/src/repositories/reality_portal/agencies.rs:227`

## Dependencies

## Required capabilities
- [ ] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [x] C2 — Seed data
- [x] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [x] C7 — Code-review reception (tick if you expect controversy)

Mode: cloud-ok

## Repro steps
1. As a seeded portal user U with no membership in agency A, `POST /api/v1/agencies/{A.id}/invitations` with body `{"email":"u@example.com","role":"admin"}` and U's Bearer token. Expected: `403 Forbidden`. Actual: `201 Created` with a `token` field.
2. `POST /api/v1/agencies/invitations/{token}/accept` with U's Bearer token. Expected: `403` (email mismatch). Actual: `200` — U is now a member of agency A with `role=admin`.
3. `GET /api/v1/agencies/{A.id}/members` shows U with `role=admin`; `PATCH /api/v1/agencies/{A.id}` returns `200` — U has full admin rights on A.

## Suggested approach
1. In `backend/servers/reality-server/src/routes/agencies.rs::create_invitation` (line ~379), acquire `conn` and call `super::agency_imports::check_agency_membership(&mut conn, agency_id, principal.user_id).await?` before the repo call — mirroring `update_agency` / `update_branding`.
2. Restrict `data.role`: unless caller is `admin`/`owner` of the agency (extend `check_agency_membership` to return the caller's role, or add a `check_agency_role` helper), reject anything other than `realtor`. Return `403 Forbidden` on violation.
3. In `backend/crates/db/src/repositories/reality_portal/agencies.rs::accept_invitation` (line ~256), before the `INSERT INTO reality_agency_members`, look up the accepting user's email and compare it case-insensitively to `invitation.email` (`LOWER(u.email) = LOWER($1)`). Fail with a typed `InvitationEmailMismatch` error if they differ; the route layer maps that to `403`.
4. Return a distinct error for expired / already-accepted invitations so the test suite can assert them.
5. Extend `route_error_map` to translate `InvitationEmailMismatch` to `403` with a stable code string.
6. Add integration tests (see *Test plan*) that exercise the full attack chain against a real Postgres via the existing `reality-server` integration harness.
7. `just verify` — backend fmt + clippy + `cargo test -p reality-server`. Do NOT touch generated OpenAPI; the route contract is unchanged.

## Alternatives considered
- **Only fix `create_invitation` (leave `accept_invitation` unchanged)** — rejected because the email-mismatch gap independently enables the attack even if the caller-membership check is added (an admin inviting `attacker@example.com` for a legitimate reason still lets anyone with the token become admin).
- **Fail closed by requiring an explicit `Authorization: Invited-Bearer <token>` route** — rejected because it breaks the existing frontend accept flow (see `frontend/apps/reality-web/src/app/agencies/accept/[token]/page.tsx`), which POSTs with the user's session token; the email-match guard is enough.

## Root-cause trace
1. Symptom: unauthenticated escalation to `admin` on any agency, reproducible via the two POSTs in *Repro steps*.
2. ← `backend/servers/reality-server/src/routes/agencies.rs:379` — `create_invitation` calls the repo layer directly with `principal.user_id` as `invited_by` and never invokes `check_agency_membership`.
3. ← `backend/crates/db/src/repositories/reality_portal/agencies.rs:279` — `accept_invitation` binds the accepting principal's `user_id` into `reality_agency_members`, never comparing `invitation.email` to `accepting_user.email`.
4. Origin: the invitation surface was added without the sibling routes' membership check convention (established later by the fix that added `check_agency_membership` calls in `update_agency` / `update_branding`).

## Test plan
- [ ] `backend/servers/reality-server/tests/agencies_authz_test.rs::create_invitation_requires_membership` — seed non-member U + agency A; assert `POST /api/v1/agencies/{A}/invitations` returns `403` (fails today with `201`).
- [ ] `backend/servers/reality-server/tests/agencies_authz_test.rs::create_invitation_rejects_privileged_role_from_non_admin` — seed member U as `realtor` of A; `POST … {role:'admin'}` returns `403`.
- [ ] `backend/servers/reality-server/tests/agencies_authz_test.rs::accept_invitation_requires_email_match` — seed A with invitation for `alice@example.com`; user `bob@example.com` POSTs `.../accept`; assert `403` and no `reality_agency_members` row.
- [ ] Regression: existing tests for the happy path (`accept_invitation_happy_path` or equivalent) still pass after adding the email-match guard — matching-email accept still yields `200` and a new `reality_agency_members` row.
- [ ] Command: `cd backend && cargo test -p reality-server -- agencies_authz` and `cargo test -p db -- reality_portal::agencies`.

## Out of scope
- Refactoring `agency_imports::check_agency_membership` for reuse beyond wiring it into `create_invitation`.
- Adding rate-limit / audit-log entries for failed invitation attempts (worthwhile but a separate hardening pass).
- Broader review of `reality_portal` repo layer for other missing membership checks (queue a follow-up ticket instead — this plan is scoped to the confirmed escalation).

## After-merge
- Move this file to `plans/_archive/code-review-reality-server-invitation-authz-escalation.md`
- Mark the matching `backlog.json` row as `status: "done"`
