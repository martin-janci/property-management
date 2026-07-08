# security-mfa-bypass-on-db-err

**Vector:** security
**Score:** 3
**Source:** rotating-expert-review 2026-07-08 (api-core)
**Confidence:** high

## Hypothesis
The password-login handler in `backend/servers/api-server/src/routes/auth.rs` reads the caller's 2FA record with `state.two_factor_repo.get_by_user_id(user.id).await` and unwraps it via `if let Ok(Some(mfa_record)) = mfa_check_result { ... }`. Any DB error (`Err(_)`) silently falls through the branch, leaves `mfa_presented = false`, and reaches token issuance at `:836+` — a transient Postgres blip or a poisoned repository connection thus permits a full password-only login for an account whose owner has enrolled TOTP. The org-policy gate at `:810` only fires when *some* membership demands MFA-for-role; personally-enrolled users with no such org policy have no downstream check, so the bypass is silent and total. Fix: treat `Err(_)` from `get_by_user_id` as a hard failure (500 or explicit MFA-required 403), never as "no MFA".

## Evidence
- `backend/servers/api-server/src/routes/auth.rs:693` — `let mfa_check_result = state.two_factor_repo.get_by_user_id(user.id).await;`
- `backend/servers/api-server/src/routes/auth.rs:694` — `if let Ok(Some(mfa_record)) = mfa_check_result {` drops `Err(_)` into the fallthrough
- `backend/servers/api-server/src/routes/auth.rs:688` — `let mut mfa_presented = false;` initial state remains on the error path
- `backend/servers/api-server/src/routes/auth.rs:810` — AuthPolicyEnforcer::check_login only blocks when an org policy demands MFA; personally-enrolled 2FA has no downstream gate
- `backend/servers/api-server/src/routes/auth.rs:836+` — access-token issuance proceeds unconditionally once the MFA branch is skipped

## Files
- `backend/servers/api-server/src/routes/auth.rs:693`
- `backend/servers/api-server/tests/mfa_brute_force_rate_limit_tests.rs`
- `backend/servers/api-server/tests/auth_enumeration_tests.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging
- [x] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [x] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. Seed a user with 2FA enabled (existing `TwoFactorRepository::insert` + `enable_by_user_id` helpers cover this in `mfa_recovery_codes_tests.rs`).
2. Wrap `state.two_factor_repo` (or the pool it uses) in a test double that returns `Err(sqlx::Error::PoolTimedOut)` for `get_by_user_id`. The router hot-path is easy to seed via `#[sqlx::test]` if you pass an already-closed pool.
3. `POST /api/v1/auth/login` with the seeded user's email + password, `two_factor_code = None`.
4. Expected (post-fix): `500 INTERNAL_SERVER_ERROR` (or `403 MFA_LOOKUP_FAILED`) — MFA-enrolled account must never be issued a session on a lookup error. Observed (pre-fix): `200 OK` with an `access_token`, `refresh_token`, and no `requires_mfa` challenge — full password-only login for a 2FA-protected account.

## Suggested approach
1. In `backend/servers/api-server/src/routes/auth.rs` around line 693, replace `if let Ok(Some(mfa_record)) = mfa_check_result { … }` with an explicit `match`:
   - `Ok(Some(mfa_record))` → existing branch (unchanged).
   - `Ok(None)` → user has no MFA enrolled → continue (unchanged behavior).
   - `Err(e)` → `tracing::error!(error = %e, "MFA lookup failed at login")`; return `(StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::new("MFA_LOOKUP_ERROR", "Login temporarily unavailable")))`. Do NOT fall through to token issuance.
2. Keep the `#[allow(deprecated)]` marker (pre-RLS user-scoped repo is intentional here).
3. Confirm the existing `record_login_attempt` call at `:837` runs only on the success path; move it after the match if the fix hoists the return.
4. Add regression test `mfa_lookup_error_blocks_login` in a NEW file `backend/servers/api-server/tests/mfa_lookup_error_tests.rs` (create it; the sibling `mfa_brute_force_rate_limit_tests.rs` is the closest existing shape to mirror):
   - Use `#[sqlx::test]` to seed a 2FA-enrolled user (see `mfa_recovery_cross_user_idor_tests.rs` for the seeding pattern).
   - Inject the failure by dropping the pool passed to the router state OR by extracting a small `TwoFactorRepository` trait seam (return `Err(sqlx::Error::PoolTimedOut)` from the double). If no seam exists, add a minimal one — the trait needs only `get_by_user_id`. Document the seam choice in the PR body.
   - Assert the login response is 5xx and no session/token is issued (`session_repo::record_login_attempt` should record `success = false`).
5. Add a matching `mfa_lookup_success_ok` control test in the same new file: keep the pool healthy and assert a 2FA-enrolled user without `two_factor_code` receives a `requires_mfa` challenge (400/401 with the existing shape) — not a fresh token. This locks in that the fix didn't broaden the block to non-error paths.
6. Verify: `cargo test -p api-server --test mfa_lookup_error_tests`. `cargo clippy -p api-server --tests -- -D warnings`. `cargo fmt --all --check`.
7. Emit a `tracing::error!` (not `warn!`) — this is a security-relevant refusal to authenticate; the SRE alert threshold on `error` matters.

## Alternatives considered
- **Log the error and continue (fail-open, current behavior)** — rejected because it lets a Postgres blip downgrade a 2FA-protected account to password-only auth. The current `if let Ok(Some(_))` shape is exactly this alternative and is the bug.
- **Force MFA re-enroll on error (fail-closed with UX kicker)** — rejected because we cannot tell an unenrolled user from an errored lookup at that point; returning 500 preserves the login attempt's audit trail without side effects. Re-enroll UX belongs in a follow-up (surface via `requires_mfa` if the row later resolves).

## Root-cause trace
1. Symptom: a 2FA-enrolled user completes a password-only login and receives a valid `access_token`.
2. ← `backend/servers/api-server/src/routes/auth.rs:836` — token generation runs even though `mfa_presented == false`.
3. ← `backend/servers/api-server/src/routes/auth.rs:694` — `if let Ok(Some(_))` silently drops `Err(_)` and `Ok(None)`; `mfa_presented` stays `false` set at :688.
4. Origin: 2FA scaffold introduced by Epic 9 Story 9.1 landed the `if let` pattern instead of an explicit match; the AuthPolicyEnforcer gate at :810 covered the "org demands MFA" case, hiding the personally-enrolled bypass. Grep `git log -p -- backend/servers/api-server/src/routes/auth.rs | grep -n mfa_check_result` for the seeding commit.

## Test plan
- [ ] Regression: `mfa_lookup_error_blocks_login` in NEW `backend/servers/api-server/tests/mfa_lookup_error_tests.rs` — inject error → assert 5xx, no token, `record_login_attempt(success=false)`.
- [ ] Control: `mfa_lookup_success_ok` (same file) — happy pool → assert `requires_mfa` challenge for a 2FA user with no code (existing behavior; fix must not broaden the block).
- [ ] Run locally: `cargo test -p api-server --test mfa_lookup_error_tests` (Postgres via `#[sqlx::test]` — CI runs it under `backend.yml`).

## Out of scope
- Retry-on-transient-Postgres inside the login handler — belongs in the pool/backoff layer, not the MFA branch.
- MFA-lookup timeouts / circuit breaker — Redis-cached MFA state can come later; the fix here is: "on error, refuse to authenticate".
- The `reports.rs` fake-download-URL bug from the same review (score 2, `code-review-api-core-fake-download-url`) — separate plan next run once it scores ≥ 3.

## After-merge
- Move this file to `plans/_archive/security-mfa-bypass-on-db-err.md`
- Mark the matching `backlog.json` row `code-review-api-core-mfa-bypass-on-db-err` as `status: "done"`
