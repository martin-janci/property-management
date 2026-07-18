# security-reality-server-agency-invitation-escalation

**Vector:** security
**Score:** 3
**Source:** code-review 2026-07-18 (rotating expert review — reality-server segment)
**Confidence:** high

## Hypothesis
Reality-server's agency-invitation flow lacks authorization on both create and accept, giving any authenticated portal user a self-service path to join any agency as any role. `create_invitation` accepts the target `agency_id` from the URL path and issues a token without verifying that `principal.user_id` is already a member (owner/admin) of that agency. `accept_invitation` then resolves the invitation row by token alone and adds the caller as a member, without comparing `invitation.email` to the caller's account email. Fixing either half alone still leaves the chain exploitable — both handlers must gate together.

## Evidence
- `backend/servers/reality-server/src/routes/agencies.rs:373` — `create_invitation` passes `principal.user_id + Path(agency_id)` straight to the repo; no membership check in the handler.
- `backend/crates/db/src/repositories/reality_portal/agencies.rs:228` — `repo::create_invitation` INSERTs into `reality_agency_invitations` without verifying `invited_by ∈ reality_agency_members(agency_id)`. The route-level error mapping references `permission`/`unauthorized` strings the repo never emits — so there is no 403 path anywhere.
- `backend/crates/db/src/repositories/reality_portal/agencies.rs:256` — `accept_invitation` resolves the row via `WHERE token = $1 AND accepted_at IS NULL AND expires_at > NOW()` and inserts a `reality_agency_members` row using `invitation.agency_id` + `invitation.role` with no cross-check against the caller's email.
- `backend/servers/reality-server/src/routes/agencies.rs:421` — `accept_invitation` handler passes `principal.user_id` and `token` only; no additional binding.
- `backend/servers/reality-server/tests/agencies_authz_tests.rs:197` — the existing authz test only asserts 401 vs non-401; no cross-agency rejection coverage.

## Files
- `backend/servers/reality-server/src/routes/agencies.rs:373`
- `backend/servers/reality-server/src/routes/agencies.rs:421`
- `backend/crates/db/src/repositories/reality_portal/agencies.rs:228`
- `backend/crates/db/src/repositories/reality_portal/agencies.rs:256`
- `backend/servers/reality-server/tests/agencies_authz_tests.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [x] C7 — Code-review reception (tick if you expect controversy)

**Execution mode (auto-derived from the ticks):**
Mode: cloud-ok

## Repro steps
1. As authenticated portal user `alice@example.com` who is NOT a member of any agency, POST `/api/v1/agencies/<victim_agency_id>/invitations` with body `{"email":"attacker@evil.com","role":"admin"}`.
2. Server currently returns 201 with the invitation token in the body.
3. As authenticated portal user `bob@evil.com` (any email, not `attacker@evil.com`), POST `/api/v1/agencies/invitations/<token>/accept`.
4. Server currently returns 200 with a `RealityAgencyMember` row — bob is now `admin` of `victim_agency_id` without ever being invited by a real member.
5. Expected: step 2 returns 403 ("only members can invite"); step 4 returns 400/403 ("token does not belong to caller's email").

## Suggested approach
1. In `repo::create_invitation` (agencies.rs:228), gate the INSERT on `SELECT 1 FROM reality_agency_members WHERE agency_id = $1 AND user_id = $2 AND role IN ('owner','admin')` for `(agency_id, invited_by)`. Return a distinct `SqlxError` variant (e.g. `RowNotFound`) that the handler maps to `403 Forbidden` (rename the current `"permission"/"unauthorized"` string-match to match the real error, or add an explicit typed error).
2. In `repo::accept_invitation` (agencies.rs:256), fetch `invitation.email` from the row, then look up the caller's email via `principal.user_id` (join `users` table or accept `principal.email` if it's already on `RequestPrincipal`) and reject on mismatch with a specific 400 code — `TOKEN_EMAIL_MISMATCH`.
3. Handler layer (`routes/agencies.rs`): map the new error variants to 403/400 explicitly instead of the current string-contains fallback.
4. Add DB-level defense: an insertion trigger on `reality_agency_invitations` requiring the row's `invited_by` to satisfy the membership predicate above (belt-and-suspenders). Optional but recommended given the severity.
5. Extend `agencies_authz_tests.rs`: add `create_invitation_rejects_non_member_403`, `accept_invitation_rejects_email_mismatch_400`, and an end-to-end "attacker chain" test that asserts both steps fail.
6. After merge: run an audit query against production `reality_agency_invitations` / `reality_agency_members` for `invited_by` that is NOT in the target `agency`'s member set at the time the invitation was created; expire/revoke any anomalies and file an incident note.
7. Post an issue/comment referencing the security fix that the incident audit query has run and the result was zero anomalies (or list them).

## Alternatives considered
- **Middleware-level agency-membership extractor** — rejected because most reality-server routes are membership-scoped in different ways (owner-only for some, any-member for others), and a single middleware would either be too coarse (denies legitimate any-member endpoints) or duplicate the logic anyway. Fixing at the handler+repo layer keeps the check next to the SQL it protects.
- **Delete-and-recreate invitation flow** (require the owner to physically log in via a separate portal to issue invites) — rejected because the current API surface is already contract with the frontend/mobile; the fix must be non-breaking. A pure server-side authorization check is drop-in.

## Root-cause trace
1. Symptom: any authenticated portal user can grant themselves membership in any agency at any role.
2. ← Handler `accept_invitation` (routes/agencies.rs:421) does not verify caller identity against the invitation.
3. ← Repo `accept_invitation` (repositories/reality_portal/agencies.rs:256) selects by token alone.
4. ← Handler `create_invitation` (routes/agencies.rs:373) does not verify caller is a member.
5. ← Repo `create_invitation` (repositories/reality_portal/agencies.rs:228) never checks `invited_by ∈ members(agency_id)` — the string `"permission"/"unauthorized"` in the handler's error map is documentation of intent, not a check that ever runs.
6. Origin: initial implementation of the agency invitation surface — no defensive check was ever added; test coverage stopped at 401/anonymous, which passes.

## Test plan
- [ ] `backend/servers/reality-server/tests/agencies_authz_tests.rs::create_invitation_rejects_non_member_403` — new test, authenticated non-member gets 403.
- [ ] `backend/servers/reality-server/tests/agencies_authz_tests.rs::accept_invitation_rejects_email_mismatch_400` — new test, wrong-email holder of a valid token gets 400.
- [ ] `backend/servers/reality-server/tests/agencies_authz_tests.rs::attacker_chain_blocked` — new integration test, full non-member → invite-self → accept flow fails at step one.
- [ ] Command: `cd backend && cargo test -p reality-server --test agencies_authz_tests` — expect the three new tests to fail on `main` and pass after the fix.

## Out of scope
- Rotating existing valid invitation tokens: separate operational step in the incident-response note (Suggested approach step 6/7).
- Auditing `list_members` (`agencies.rs:345`) for a possible member-PII disclosure IDOR — flagged by pm-security this run; file as a follow-up issue after this plan lands, don't bundle.
- Middleware/extractor refactor to unify agency-membership checks across the whole surface — larger refactor, not part of this hotfix-shaped change.

## After-merge
- Move this file to `plans/_archive/security-reality-server-agency-invitation-escalation.md`
- Mark the matching `backlog.json` row as `status: "done"`
