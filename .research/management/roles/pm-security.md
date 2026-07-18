# pm-security — 2026-07-18

## Summary
Cross-agency invitation escalation is confirmed, live, and unauthenticated-authorization-check gap in reality-server (any logged-in portal user can mint invitation tokens for arbitrary agencies and self-accept them) — release blocker, not a later item. Recent RLS/IDOR fixes (PR #2416, #2421) and mobile session-cache fixes (#2412, #2372) narrowed the api-server/mobile attack surface this sprint.

## Next actions
1. **[HIGH]** Add agency-membership check (invited_by must be an owner/admin member of agency_id) to reality-server `create_invitation` handler or `repo::create_invitation` before merge freeze. Dep: rust-backend. DoD: non-member POST /agencies/{id}/invitations returns 403; regression test added.
2. **[HIGH]** Bind `accept_invitation` to the invited email (verify principal.email == invitation.email) not token possession alone. Dep: rust-backend. DoD: token holder with mismatched email rejected; test added.
3. **[HIGH]** Audit reality_agency_invitations/reality_agency_members tables for rows created via this gap since deploy; revoke/expire any unexpected memberships. Dep: rust-backend/db-owner. DoD: audit query run, anomalous grants revoked, incident note filed.
4. **[HIGH]** Add cross-agency rejection test case to `agencies_authz_tests.rs` (currently only checks 401). Dep: rust-backend. DoD: test asserts 403 for non-member inviter.
5. **[MEDIUM]** Check `list_members` (agencies.rs:345) — also has no membership/principal check; confirm intended (public directory) or another IDOR. Dep: rust-backend. DoD: explicit decision documented, fixed if unintended.
6. **[MEDIUM]** Close test-hardening batch items #480 / #483 / #484 (WS JWT-in-logs, voice device IDOR no tests, silent notification failures) before next release cut. Dep: rust-backend. DoD: issues closed or explicitly deferred with owner sign-off.

## Risks
- **[H/H]** Cross-agency invitation escalation exploitable in current production code — confirmed no membership check anywhere. Mitigation: ship hotfix.
- **[M/H]** Unknown scope of prior exploitation since deploy — no audit trail check yet. Mitigation: run DB audit query.
- **[M/M]** `list_members` has no principal/authz extractor at all — possible unauthenticated member-list disclosure across agencies.
- **[M/M]** Test-hardening batch items #480/#483/#484 remain open; residual gap alongside newly-fixed government_portal/RAG IDORs.
- **[L/M]** No equivalent audit performed for reality-server session/membership resolution paths (parallel to the mobile/web fixes #2412/#2372/#2375).

## Open questions
- Was `create_invitation` shipped with this gap from initial implementation, or introduced by a recent refactor — need git blame to scope exposure window.
- Is `list_members` intentionally public/no-auth or an oversight?
- Has any anomalous `reality_agency_members` row already been created in production?
- Should #480 (JWT in WS query param, logged) be treated as a release blocker given 8a-3 is marked done despite the gate remaining open?

## Decisions needed
- Rotate/expire all outstanding `reality_agency_invitations` tokens after the fix ships — owner: rust-backend / db-owner.
- Notify affected agencies if audit finds unauthorized membership grants — owner: pm-security + eng-lead.
- Treat cross-agency invitation escalation as a hotfix (branch from `main` per hotfix workflow) vs. next dev merge — owner: release manager.
