# security-reality-server-agency-invitation-idor

**Vector:** security
**Score:** 3
**Source:** code-review 2026-08-01 (reality-server segment); hotspot in `backend/servers/reality-server/src/routes/agencies.rs`
**Confidence:** high

## Hypothesis
`create_invitation` and `accept_invitation` in reality-server's agency routes lack the membership + email-identity checks that every other agency-mutation route enforces (`update_agency` and the members route both call `check_agency_membership`). Any authenticated Reality Portal user can POST to `/api/v1/agencies/{arbitrary_id}/invitations` with an attacker-controlled `email` and `role: 'owner'`, receive the invitation `token` in the response body, and then POST `/api/v1/agencies/invitations/{token}/accept` to be inserted into `reality_agency_members` at the requested role. This is a straight IDOR + privilege-escalation chain into any agency's data; the smallest fix adds one `check_agency_membership` call to `create_invitation` (matching the update_agency pattern) and an email-equality check in `accept_invitation` between the caller's account email and `invitation.email`.

## Evidence
- `backend/servers/reality-server/src/routes/agencies.rs:373` — `create_invitation` handler goes straight to `state.reality_portal_repo.create_invitation(agency_id, principal.user_id, data)` with no membership guard, while `update_agency` at `agencies.rs:268` explicitly calls `super::agency_imports::check_agency_membership(&mut conn, id, principal.user_id).await?;`.
- `backend/crates/db/src/repositories/reality_portal/agencies.rs:228` — repo `create_invitation` is a bare INSERT into `reality_agency_invitations`; no ownership check at the DB layer either.
- `backend/servers/reality-server/src/routes/agencies.rs:421` — `accept_invitation` calls `state.reality_portal_repo.accept_invitation(&token, principal.user_id)` — it never compares `principal`'s email against `invitation.email`.
- `backend/crates/db/src/repositories/reality_portal/agencies.rs:256` — repo `accept_invitation` fetches the invitation by `token`, marks it accepted, and INSERTs `(agency_id, user_id, invitation.role)` into `reality_agency_members`. `user_id` is the caller — not derived from `invitation.email` — so the caller becomes a member at the invitation's role regardless of who was invited.
- Existing tests (`backend/servers/reality-server/tests/suites/agencies_authz_tests.rs:175+`) cover only the 401-unauthenticated paths for both handlers; there is no test asserting "non-member cannot create invitation for someone else's agency" or "user whose email ≠ invitation.email cannot redeem the token" — so the two gaps above are unnoticed.

## Files
- `backend/servers/reality-server/src/routes/agencies.rs`
- `backend/crates/db/src/repositories/reality_portal/agencies.rs`
- `backend/servers/reality-server/tests/suites/agencies_authz_tests.rs`

## Dependencies
<!-- No blockers. -->

## Required capabilities
- [x] C1 — Systematic debugging (security IDOR, tracing to repo layer)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. As user `A` (member of agency `AGENCY_A`, no relation to `AGENCY_B`), authenticate to reality-server and POST `/api/v1/agencies/{AGENCY_B_UUID}/invitations` with body `{"email":"a@example.com","role":"owner","message":null}` using `A`'s session cookie / bearer.
2. Server returns 201 with the invitation JSON including `token`.
3. As the same user `A`, POST `/api/v1/agencies/invitations/{token}/accept` (no body needed — the `RequestPrincipal` extractor supplies `user_id`).
4. Server returns 200 with `RealityAgencyMember { agency_id: AGENCY_B, user_id: A, role: "owner" }`.
5. Expected: step 1 returns 403 (`A` is not a member of `AGENCY_B`) OR step 3 returns 403 (`A`'s account email does not match the invitation's `email`). Actual: both succeed and `A` is now an `owner` of `AGENCY_B`.

## Suggested approach
1. In `backend/servers/reality-server/src/routes/agencies.rs::create_invitation` (line 373), acquire a connection from the pool and call `super::agency_imports::check_agency_membership(&mut conn, agency_id, principal.user_id).await?` before delegating to the repo — mirror the exact pattern from `update_agency` at line 268. Optionally require role ≥ `manager` / `owner` if the agency has a role hierarchy (check what `check_agency_membership` returns; if it's boolean-membership only, layer a role check on top).
2. Validate the `role` field on `CreateAgencyInvitation` (currently `Option<String>`) against an explicit allowlist (`realtor|manager|owner`) at the handler or repo layer to prevent silent role coercion via unknown strings.
3. In `accept_invitation` (line 421), before calling `state.reality_portal_repo.accept_invitation(...)`, fetch the invitation by token, then look up the principal's account email (via `state.user_repo.get(...).email` or the session claims) and reject with 403 when `principal_email.to_lowercase() != invitation.email.to_lowercase()`. Prefer moving this check inside the repo's transaction (in `repositories/reality_portal/agencies.rs::accept_invitation`) so it is atomic with the `INSERT INTO reality_agency_members` — the handler-layer check races against a concurrent membership change otherwise.
4. Add two `#[sqlx::test]` cases to `tests/suites/agencies_authz_tests.rs`: `create_invitation_non_member_returns_403` (seed A as member of AGENCY_A only, POST to AGENCY_B, expect 403) and `accept_invitation_email_mismatch_returns_403` (seed invitation with email `b@example.com`, POST accept as user with email `a@example.com`, expect 403 and assert no row was added to `reality_agency_members`).
5. If any legitimate flow depends on the pre-fix behavior (e.g. a "self-serve join" UX), migrate it to a distinct route with an explicit approval step — do NOT relax the new checks. Verify with `rg -F 'create_invitation\|accept_invitation' frontend mobile-native` to catch client-side callers that assume the old semantics.
6. Run `cd backend && cargo test -p reality-server --test suite_5 -- agencies_authz` to execute the new authz cases + confirm existing 401 paths still pass; then `cargo clippy -p reality-server --all-targets -- -D warnings`.
7. Follow the same fix pattern in `agency_imports.rs` if a matching pair exists there — a quick `rg -n 'check_agency_membership' backend/servers/reality-server/src/routes/` should confirm no other route mutates agency state without it.

## Alternatives considered
- **Enforce membership solely at the DB layer (add a policy/WHERE clause in repo)** — rejected because reality-server's repo layer today does not have an authenticated-session context and the pool is not RLS-scoped; adding a repo-layer check duplicates the missing plumbing without gaining atomicity, whereas the handler check + email check inside the accept-transaction is the pattern the rest of the crate already uses.
- **Rate-limit `create_invitation` per user as a mitigation** — rejected because the vulnerability is authorization, not abuse-volume; rate limiting would slow an attacker but still permits the takeover, and no other agency route has a rate-limit shim to reuse.

## Root-cause trace
1. Symptom: authenticated non-member of AGENCY_B can join AGENCY_B as `owner` via the two-step invite/accept flow (see Repro steps).
2. ← Immediate cause at `backend/servers/reality-server/src/routes/agencies.rs:381` — `create_invitation` handler passes `agency_id` straight to the repo with no membership guard.
3. ← Immediate cause at `backend/servers/reality-server/src/routes/agencies.rs:428` — `accept_invitation` handler passes `principal.user_id` to the repo without comparing the caller's email to `invitation.email`.
4. ← Repo layer (`backend/crates/db/src/repositories/reality_portal/agencies.rs:228,256`) has no defensive check either — bare INSERTs.
5. Origin: the invitation flow appears to have been added without following the `update_agency`/`list_members` guard pattern (`check_agency_membership` at `agencies.rs:268/322`). Both handlers ship a `403 Not authorized` string mapping in the error match arm, but the repo never emits it, so the branch is dead code — the intent was there, the check was skipped.

## Test plan
- [ ] `backend/servers/reality-server/tests/suites/agencies_authz_tests.rs::create_invitation_non_member_returns_403` — seed user A as member of agency `alpha`; POST `/api/v1/agencies/{beta}/invitations` as A with body `{email:"a@example.com", role:"owner"}` → expect 403.
- [ ] `backend/servers/reality-server/tests/suites/agencies_authz_tests.rs::accept_invitation_email_mismatch_returns_403` — seed invitation for `victim@example.com` in `beta`; authenticate as `attacker@example.com` and POST `.../invitations/{token}/accept` → expect 403 and `SELECT COUNT(*) FROM reality_agency_members WHERE agency_id = $beta AND user_id = $attacker` returns 0.
- [ ] Regression: existing `create_invitation_unauthenticated_returns_401` and `accept_invitation_unauthenticated_returns_401` continue to pass.
- [ ] `cd backend && cargo test -p reality-server --test suite_5 -- agencies_authz` (compiles + runs against the ephemeral migrated Postgres in CI; local devs should use `stack up pm-local` first if running outside CI).

## Out of scope
- Rewriting the invitation model (e.g. adding a nonce, changing token format, expiring on first fetch) — the fix is the missing authorization check, not the token design.
- Rate-limiting the invitation endpoints — not what this issue is about; may be filed separately if abuse patterns emerge.
- Auditing the paginated-total bug (`reports.rs:342`, `imports.rs:141`, `agency_imports.rs:183`) or the `create_invitation` test-gap (`agencies_authz_tests.rs:175`) — filed as sibling backlog rows (`code-review-reality-server-paginated-total-truncated`, `code-review-reality-server-invitation-idor-test-gap`); those are separate diffs.
- Frontend/mobile client updates to surface the new 403 responses — client-side UX is orthogonal; the server-side authz gap is the security fix.

## After-merge
- Move this file to `plans/_archive/security-reality-server-agency-invitation-idor.md`
- Mark `backlog.json` row `code-review-reality-server-agency-invitation-idor` as `status: "done"`
