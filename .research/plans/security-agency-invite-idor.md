# security-agency-invite-idor

**Vector:** security
**Score:** 3
**Source:** code-review reality-server 2026-07-16 — POST `/api/v1/agencies/{id}/invitations` at `backend/servers/reality-server/src/routes/agencies.rs:373`; independently confirmed by pm-security role rotation 2026-07-16
**Confidence:** high

## Hypothesis
`POST /api/v1/agencies/{id}/invitations` in reality-server extracts `RequestPrincipal` but never calls `check_agency_membership()` before invoking `repositories::reality_portal::agencies::create_invitation`. The repo layer is a bare INSERT into `reality_agency_invitations` with no auth predicate. Any authenticated portal user can mint a live 7-day invitation token (any email, any role) for any agency — cross-tenant phishing + membership pollution. The sister mutating handlers `update_agency` (agencies.rs:268) and `update_branding` (agencies.rs:322) DO call `check_agency_membership`, so this is a missing-guard outlier, not a design decision. The smallest correct fix mirrors the sister handlers: call `check_agency_membership(&mut conn, agency_id, principal.user_id)` before the repo call, and add server-side allow-list validation of `data.role` to prevent role-elevation on acceptance.

## Evidence
- `backend/servers/reality-server/src/routes/agencies.rs:373` — `create_invitation` handler extracts `RequestPrincipal` then jumps straight to `repositories::reality_portal::agencies::create_invitation(...)` with no membership check
- `backend/servers/reality-server/src/routes/agencies.rs:268,322` — sister handlers `update_agency` and `update_branding` DO call `check_agency_membership()` before the repo call; this handler is the outlier
- `backend/crates/db/src/repositories/reality_portal/agencies.rs:223-248` — repo `create_invitation` is a bare INSERT into `reality_agency_invitations`; no auth predicate; `role` is `COALESCE(data.role, 'realtor')` with zero enum validation
- `backend/servers/reality-server/src/routes/agencies.rs:390` — the route's fallback error-string match on `permission`/`unauthorized` is dead code because the DB layer never raises those strings — masking the missing gate on inspection
- Rotating expert (Rust) review of segment `reality-server` traced the full call path from Axum route to unscoped INSERT; pm-security independently confirmed 2026-07-16 and extended: `data.role` is also unvalidated — caller can request `owner`/`admin` and get privilege escalation on invitation acceptance

## Files
- `backend/servers/reality-server/src/routes/agencies.rs`
- `backend/crates/db/src/repositories/reality_portal/agencies.rs`
- `backend/servers/reality-server/tests/agencies_authz_tests.rs`

## Dependencies
<!-- No blocking plan; this is a standalone security fix. -->

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [x] C7 — Code-review reception (tick if you expect controversy)

Mode: cloud-ok

_(Rust-only change; no browser or device required.)_

## Repro steps
1. Provision two portal users `A` and `B` in the reality-server dev stack. `A` is a member of agency `AGENCY_A`; `B` is a member of `AGENCY_B` and NOT a member of `AGENCY_A`.
2. As `B`, obtain a bearer token, then `POST /api/v1/agencies/{AGENCY_A}/invitations` with body `{"email":"attacker@example.com","role":"realtor"}`.
3. **Expected (post-fix):** 403 `FORBIDDEN`. **Actual (today, pre-fix):** 201 `Created` with a valid 7-day invitation token for `AGENCY_A`.
4. Repeat step 2 with body `{"email":"attacker@example.com","role":"owner"}`. **Expected (post-fix):** 400 `Bad Request` (role not in allow-list). **Actual (today):** 201 `Created` — invitation minted with `role=owner`.

## Suggested approach
1. Add `check_agency_membership(&mut conn, agency_id, principal.user_id).await?` at `agencies.rs:373`, immediately after the `RequestPrincipal` extraction, mirroring `update_agency` (agencies.rs:268) and `update_branding` (agencies.rs:322). Confirm the helper's error surface returns 403 not 500.
2. Delete the now-dead fallback error-string match on `permission`/`unauthorized` at `agencies.rs:390` — the DB layer never raises those strings, and step 1 makes it unreachable.
3. In `CreateAgencyInvitation` DTO deserialization (search for the struct near `agencies.rs:373`), constrain `role: AgencyRole` to a serde-validated enum (`realtor` | `manager` — verify allowed set against product intent; NEVER include `owner`/`admin`). If the enum doesn't exist yet, add it under `backend/crates/common/src/reality_portal/mod.rs` or the closest analog.
4. Update `create_invitation` in `repositories/reality_portal/agencies.rs:223-248` to take a typed `AgencyRole` instead of `Option<String>`. Remove the `COALESCE(data.role, 'realtor')` — the enum default lives at the API layer now.
5. Add `agencies_authz_tests.rs` cases (style mirrors `report_schedule_rbac_tests.rs`):
   - `create_invitation_as_non_member_is_rejected` — B → 403 on `POST /api/v1/agencies/{AGENCY_A}/invitations`
   - `create_invitation_as_member_succeeds` — A → 201 on same route
   - `create_invitation_with_invalid_role_is_rejected` — A with `role: "owner"` → 400
   - `create_invitation_without_auth_is_rejected` — no JWT → 401/403 (outer gate)
6. `cargo fmt && cargo clippy -p reality-server -p db -- -D warnings && cargo test -p reality-server agencies_authz_tests`

## Alternatives considered
- **Type-level `AgencyMember` extractor (rejected as first step, kept as follow-up)** — cleaner long-term because it makes it impossible to compile a route without the check, but out of scope for the release-blocker hotfix; queued as `pm-security-agency-member-extractor` action-list item.
- **Repo-layer WHERE-EXISTS guard (rejected)** — pushing the check into the SQL (`INSERT ... WHERE EXISTS (SELECT 1 FROM reality_agency_members WHERE ...)`) would work but breaks the handler-level auth pattern the rest of `agencies.rs` follows and yields a less informative 500 instead of 403.

## Root-cause trace
1. Symptom: `POST /api/v1/agencies/{AGENCY_A}/invitations` with a token belonging to a member of `AGENCY_B` returns 201 with a valid invitation token.
2. ← handler at `backend/servers/reality-server/src/routes/agencies.rs:373` — extracts `RequestPrincipal` but never calls `check_agency_membership`
3. ← repo at `backend/crates/db/src/repositories/reality_portal/agencies.rs:223` — bare INSERT into `reality_agency_invitations`; no `EXISTS` guard on `reality_agency_members`
4. Origin: handler shipped without the guard from the outset; sister handlers `update_agency` / `update_branding` in the same file call the helper, so the omission is an oversight not a design choice. Repo signature accepts `Option<String>` for `role`, which invited the deserialization-side omission.

## Test plan
- [ ] `backend/servers/reality-server/tests/agencies_authz_tests.rs::create_invitation_as_non_member_is_rejected` — asserts 403 on cross-agency invite (fails on `main`, passes post-fix)
- [ ] `backend/servers/reality-server/tests/agencies_authz_tests.rs::create_invitation_with_invalid_role_is_rejected` — asserts 400 on `role: "owner"` (fails on `main`, passes post-fix)
- [ ] `backend/servers/reality-server/tests/agencies_authz_tests.rs::create_invitation_as_member_succeeds` — asserts existing happy path stays green
- [ ] Local: `cargo test -p reality-server agencies_authz_tests -- --nocapture`

## Out of scope
- The type-level `AgencyMember` extractor refactor (separate action `pm-security-agency-member-extractor`).
- Auditing `list_members` for missing auth (separate action `pm-security-reality-list-members-authz-audit`).
- Backfill / incident response for any invitations that may already have been minted through this route — that is an ops decision, not a code plan.

## After-merge
- Move this file to `plans/_archive/security-agency-invite-idor.md`
- Mark the matching `backlog.json` row (`code-review-reality-server-agency-invite-idor`) as `status: "done"`
