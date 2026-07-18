# code-review-reality-server-invite-authz

**Vector:** security
**Score:** 3
**Source:** tier-1d dev-review 2026-07-18 segment=reality-server, commit 8c5d49b98
**Confidence:** high

## Hypothesis
`create_invitation` at `backend/servers/reality-server/src/routes/agencies.rs:373` is wired at line 38 (`POST /api/v1/agencies/{id}/invitations`) and calls the repository directly with only `AuthUser` + `Path(agency_id)`, performing **no** membership or role check. Its siblings `update_agency` (:268) and `update_branding` (:322) both call `super::agency_imports::check_agency_membership` before mutating; this handler omits that gate. Combined with `accept_invitation` (:421), any authenticated portal user can invite an attacker-controlled email as `owner`/`admin` for **any** agency id, read the invitation token from the 201 response, accept it, and take control of that agency — a broken-access-control privilege-escalation. Add the missing membership+role gate at the handler entry, tighten role validation so only `owner`/`admin` may mint invitations, and pin the fix with a `#[sqlx::test]` that fails on `dev`.

## Evidence
- `backend/servers/reality-server/src/routes/agencies.rs:373-401` — `create_invitation` signature is `State + AuthUser + Path<Uuid> + Json<CreateAgencyInvitation>`; goes straight to `reality_portal_repo.create_invitation(agency_id, principal.user_id, data)` at :381. No `check_agency_membership` call.
- `backend/servers/reality-server/src/routes/agencies.rs:268` — `update_agency` gates with `check_agency_membership(&mut conn, id, principal.user_id)`. `update_branding` at :322 mirrors the same guard.
- `backend/crates/db/src/repositories/reality_portal/agencies.rs:228-253` — repo `create_invitation` performs an unconditional `INSERT INTO reality_agency_invitations …` with no authorization check.
- `backend/servers/reality-server/src/routes/agencies.rs:421-448` — `accept_invitation` adds the caller as an agency member on any valid, unexpired token (also flagged as `code-review-reality-server-invite-email-mismatch` — see *Out of scope*).
- `agencies.rs:390` — the handler's 403 mapping for `"permission"`/`"unauthorized"` errors is dead code (nothing raises it), so the missing gate is unmasked in production.

## Files
- `backend/servers/reality-server/src/routes/agencies.rs:373`
- `backend/servers/reality-server/src/routes/agencies.rs:38`
- `backend/crates/db/src/repositories/reality_portal/agencies.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (bug/security priv-esc: trace handler→repo→policy)
- [ ] C2 — Seed data
- [x] C3 — Dev instance running (`stack up pm-local` — cross-tenant integration test needs full stack)
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion (must reproduce IDOR then confirm gate closes it)
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. Boot `stack up pm-local` and seed two orgs A and B, each with an agency (`agency_A`, `agency_B`), each with a member user (`user_A` in A, `user_B` in B).
2. Authenticate as `user_B`. `POST /api/v1/agencies/{agency_A.id}/invitations` with body `{ "email": "attacker+A@example.com", "role": "owner" }`.
3. Expected: 403 FORBIDDEN — user_B is not a member of agency_A. Actual: 201 CREATED with a token in the response body — the attacker has minted an owner invitation for someone else's agency and can now `POST /api/v1/agencies/invitations/{token}/accept` under any authenticated principal to seize it.

## Suggested approach
1. In `backend/servers/reality-server/src/routes/agencies.rs:373`, before calling the repo, invoke `super::agency_imports::check_agency_membership(&mut conn, agency_id, principal.user_id).await?;` — the same guard `update_agency`/`update_branding` use.
2. Extend the gate to require an inviter role of `admin` or `owner` — `check_agency_membership` currently only asserts membership. Either return the membership row from the check (so the handler can inspect `role`), or add a sibling `check_agency_role(&mut conn, agency_id, principal.user_id, &[Role::Owner, Role::Admin])?` helper next to it and use that here.
3. Validate `data.role` at the handler boundary against a permitted set — reject `Some("owner")` for non-owners, reject unknown values. Today `CreateAgencyInvitation.role: Option<String>` COALESCEs to `"realtor"` in the repo, so `"owner"`/`"admin"` are accepted silently.
4. Return `403` for the auth failures (not 404), so the existing `"permission"/"unauthorized"` mapping at :390 stops being dead code.
5. Add a `#[sqlx::test]` in `backend/crates/db/tests/` (or the closest existing reality_portal test module) that seeds two orgs, calls the repo through the handler flow as a non-member, and asserts `Err(_)` before the repo insert — fails on `dev`, passes after the guard.
6. In the same test, cover the role-privilege-escalation vector: as an inviter with `role="realtor"`, attempting to mint an `owner` invitation must be rejected.
7. Run `cargo fmt --all && cargo clippy -p reality-server --all-targets -- -D warnings && cargo test -p reality-server -- --test-threads=1 agencies_invitation`.

## Alternatives considered
- **Move the membership check into the repo `create_invitation`** — rejected because it duplicates the authorization contract that already lives in the handler layer (`check_agency_membership` is the sibling handlers' pattern), inverts the layer boundary (`db` crate reaching for `principal` context), and would still leave the handler unable to distinguish 403 vs 404 cleanly.
- **Rely on downstream row-level policies** — rejected because `reality_agency_invitations` has no FORCE-RLS policy today; the api-server connects as the table owner, so ENABLE-only RLS is bypassed. Adding a FORCE policy would require a separate db-migration PR and still miss the role-escalation vector.

## Root-cause trace
1. Symptom: any authenticated user creates an owner invitation for any agency and receives the token in the 201 response — full priv-esc via `POST /agencies/invitations/{token}/accept`.
2. ← Immediate cause at `backend/servers/reality-server/src/routes/agencies.rs:373-401`: `create_invitation` handler skips `check_agency_membership` and passes user-supplied `role` unfiltered to the repo.
3. ← Upstream cause at `backend/crates/db/src/repositories/reality_portal/agencies.rs:228-253`: `create_invitation` repo method has no authorization check; it treats `inviter_user_id` as a caller identity, not an authorized principal.
4. Origin: introduced when the invitation flow was added to `reality-server` without adopting the `check_agency_membership` gate already established by `update_agency`/`update_branding`. First landing to be confirmed via `git log -p` on `agencies.rs` — probable single commit that missed the mirroring.

## Test plan
- [ ] New `#[sqlx::test]` in the reality_portal agency test module — non-member calling `create_invitation` gets an `Err(_)` (403 at handler); fails on `dev` (currently returns Ok).
- [ ] Second case in the same test — a `realtor`-role inviter trying to mint an `owner` invitation gets rejected; fails on `dev`.
- [ ] Regression that the happy path (owner/admin member minting a `realtor` invitation) still returns 201 with a token.
- [ ] `cargo test -p reality-server agencies_invitation` and `cargo test -p db reality_portal::agencies`.

## Out of scope
- The sibling finding `code-review-reality-server-invite-email-mismatch` (accept_invitation not checking caller email vs invitation target) — track as a separate follow-up plan; it compounds this one but the fix is orthogonal (invitation-acceptance layer).
- The sibling finding `code-review-reality-server-members-noauth` (unauthenticated GET /members) — separate plan; different handler (`list_members`), different failure mode (unintended unauth read vs priv-esc).
- Adding FORCE-RLS + a policy to `reality_agency_invitations` — defense in depth, worth doing but a bigger db-migration PR; the handler gate is the minimal, correct fix.
- Refactoring `check_agency_membership` into a permission-decision service — kept for a future refactor plan.

## After-merge
- Move this file to `plans/_archive/code-review-reality-server-invite-authz.md`
- Mark the matching `backlog.json` row as `status: "done"`
