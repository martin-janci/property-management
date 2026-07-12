# security-reality-agency-invitation-no-membership

**Vector:** security
**Score:** 3
**Source:** Issue-adjacent — code-review-finding via rotating-expert-review 2026-07-12; sibling to PR #2254 fix (Closes #2249)
**Confidence:** medium

## Hypothesis
`reality-server` `create_invitation` (POST `/api/v1/agencies/{id}/invitations`) accepts any authenticated `RequestPrincipal` and mints an invitation for the path-supplied `agency_id` without verifying the caller is a member of that agency. Sibling mutating handlers (`update_agency`, `update_branding`) already gate on `check_agency_membership`; this one was missed. Because the accept endpoint promotes the invitee to the invitation's role (defaults to `realtor`), any authenticated portal user can bootstrap themselves — or an accomplice — into any agency of their choosing. Add the same membership gate the sibling handlers use and pin the behavior with a `resident-user forges invitation` regression test.

## Evidence
- `backend/servers/reality-server/src/routes/agencies.rs:332` — `create_invitation` takes `RequestPrincipal + Path(agency_id)` and calls `state.reality_portal_repo.create_invitation(agency_id, principal.user_id, data)` with no upstream membership check.
- `backend/servers/reality-server/src/routes/agencies.rs:227` and `:281` — `update_agency` and `update_branding` both call `super::agency_imports::check_agency_membership(&mut conn, id, principal.user_id).await?;` before delegating to the repo.
- `backend/servers/reality-server/src/routes/agency_imports.rs:403` — `pub(super) async fn check_agency_membership` is the canonical helper (checks agency existence, then `reality_agency_members WHERE agency_id AND user_id AND is_active`, returns 404/403).
- `backend/crates/db/src/repositories/reality_portal/agencies.rs:198` — `create_invitation` blindly `INSERT`s into `reality_agency_invitations`; there is no route-independent authorization safety net.
- Trust-surface parity: PR #2254 (Closes #2249) fixed the identical class of vulnerability in `sso.rs` (reality-server SSO exchange trusting client-supplied roles).

## Files
- `backend/servers/reality-server/src/routes/agencies.rs:332`
- `backend/servers/reality-server/src/routes/agency_imports.rs:403`
- `backend/crates/db/src/repositories/reality_portal/agencies.rs:198`

## Dependencies

## Required capabilities
- [ ] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

**Execution mode (auto-derived from the ticks):**
Mode: cloud-ok

## Repro steps
1. Bring up reality-server against the shared dev DB (`stack up pm-local` or `ppt_dev_up` via `ppt-bridge`), and prepare two portal users A and B where A is a member of agency X, and B is a member of agency Y (never a member of X).
2. As user B (authenticated), `POST /api/v1/agencies/{X}/invitations` with body `{ "email": "attacker@example.com", "role": "manager" }`.
3. Expected: HTTP 403 `Not a member of this agency`. Actual (pre-fix): HTTP 200 + a valid `RealityAgencyInvitation` row bound to agency X, minted by B.
4. `POST /api/v1/agencies/invitations/{token}/accept` as the attacker email → the attacker is a full manager of agency X, whose realtors and inventory they never had authority over.

## Suggested approach
1. In `backend/servers/reality-server/src/routes/agencies.rs::create_invitation` (starting at line 332), acquire a public connection (`state.acquire_public_conn().await`) — mirror the shape used by `update_agency` at line 220 — and call `super::agency_imports::check_agency_membership(&mut conn, agency_id, principal.user_id).await?;` before the repo call. `drop(conn)` before invoking the repo to release the pool slot, matching the pattern used by the sibling mutating handlers.
2. Confirm the role field on `CreateAgencyInvitation` is validated at the handler layer if not already — even after the fix, a member should not be able to mint an invitation for a role higher than their own without an explicit escalation gate. If validation isn't cheap to add here, capture it under *Out of scope* below and open a follow-up issue.
3. Add integration test `backend/servers/reality-server/tests/agencies_create_invitation_membership_tests.rs` with three scenarios: (a) non-member gets 403; (b) member of another agency gets 403 for target agency; (c) member of target agency gets 201. Follow the shape of `sso_exchange_tests.rs` added by PR #2254.
4. Verify (`cargo test -p reality-server --test agencies_create_invitation_membership_tests`).
5. If the utoipa docs list a 403 already, no doc update needed; otherwise, add `(status = 403, description = "Not a member of this agency")` to `create_invitation`'s `#[utoipa::path(...)]` responses block.
6. Update `docs/api/README.md` only if the OpenAPI spec regenerates; leave the OpenAPI regeneration to the standard CI check.

## Alternatives considered
- **Add the gate in the repo (`repositories/reality_portal/agencies.rs::create_invitation`)** — rejected because RLS/authz for reality-server lives at the route layer (deny-all per-handler pattern, mirroring the api-server extractor convention), and the repo lacks the `user_id` context needed to check membership without an extra query round-trip. Fixing at the route matches the codebase's existing pattern (`update_agency`, `update_branding`, all `agency_imports` handlers).
- **Introduce a `RequireAgencyMembership` extractor** — rejected because the existing helper (`agency_imports::check_agency_membership`) is already the canonical pattern used by six sibling handlers, and a one-call fix keeps the diff surgical and reviewable. A cross-cutting extractor is worth doing separately if the codebase adopts capability-style extractors more broadly, but that's out of scope for this fix.

## Root-cause trace
N/A — `security` doesn't require backward tracing; the vulnerability is a missing check at a known handler boundary, not a data-flow regression. The origin is the initial commit that added `create_invitation` without wiring in `check_agency_membership` (introduced when the agency invitation surface first landed; sibling mutating handlers were added with the check, this one was missed).

## Test plan
- [ ] `backend/servers/reality-server/tests/agencies_create_invitation_membership_tests.rs` — new file, three tests: `non_member_forbidden`, `wrong_agency_forbidden`, `member_allowed`. Each seeds two users + two agencies, obtains a JWT via the existing test helper, and asserts the status and (for the success case) that the row lands in `reality_agency_invitations` with the right `invited_by`.
- [ ] Regression scenario: a resident of agency Y attempts to `POST /api/v1/agencies/{X}/invitations` — asserts HTTP 403.
- [ ] Local verify command: `cargo test -p reality-server --test agencies_create_invitation_membership_tests` from `backend/`.

## Out of scope
- Role-escalation validation on `CreateAgencyInvitation.role` (e.g. rejecting a `realtor` who tries to mint a `manager` invitation). Capture as a follow-up issue if not trivially foldable into the same PR.
- Auditing `accept_invitation` / `revoke_invitation` — they consume tokens rather than mint them; the attack surface is bounded by an unpredictable token. Separate follow-up if the review flags them.
- Adding `PortalPrincipal` / auth to the sibling `list_members` handler (`agencies.rs:304`) — surfaced as a separate open backlog row (`security-reality-list-members-unauth`).

## After-merge
- Move this file to `plans/_archive/security-reality-agency-invitation-no-membership.md`
- Mark the matching `backlog.json` row (`security-reality-agency-invitation-no-membership`) as `status: "done"`
