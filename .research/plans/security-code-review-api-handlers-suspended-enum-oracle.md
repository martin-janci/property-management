# security-code-review-api-handlers-suspended-enum-oracle

**Vector:** security
**Score:** 3
**Source:** hotspot in `backend/servers/api-server/src/routes/auth.rs`
**Confidence:** high

## Hypothesis
The `POST /api/v1/auth/login` handler returns `401 ACCOUNT_SUSPENDED` **before** running argon2 password verification (suspended-check at auth.rs:707, `verify_password` at auth.rs:716). A caller submitting an arbitrary password against a suspended email therefore gets a distinct `ACCOUNT_SUSPENDED` response, whereas an unknown email gets the generic `INVALID_CREDENTIALS`. That's an account-enumeration oracle — an unauthenticated caller can iterate email addresses and identify which ones belong to suspended accounts, feeding phishing / credential-stuffing lists. The fix is to reorder: run `verify_password` first, and only branch on `is_suspended` after the password matches, so the response for a wrong-password attempt is uniform regardless of account state.

## Evidence
- `backend/servers/api-server/src/routes/auth.rs:707` — string literal `"ACCOUNT_SUSPENDED"` appears in the login flow at line 707.
- `backend/servers/api-server/src/routes/auth.rs:716` — `.verify_password(&req.password, &user.password_hash)` is invoked at line 716, 9 lines after the suspended-status check.
- Signal `code-review-api-handlers-suspended-enum-oracle` — confidence=high, +3 (source: 2026-07-12 api-handlers segment code review).
- Contrast — the same file's `INVALID_CREDENTIALS` arm (unknown-email path) intentionally uses a fixed, non-timing-oracle response; the suspended-status branch violates that discipline.

## Files
- `backend/servers/api-server/src/routes/auth.rs:699`
- `backend/servers/api-server/tests/auth_tests.rs`

## Dependencies
(none)

## Required capabilities
- [ ] C1 — Systematic debugging
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. Seed two users: user_A (email `a@ex.test`, password `Passw0rd!`, status=`active`), user_B (email `b@ex.test`, password `Passw0rd!`, status=`suspended`).
2. `POST /api/v1/auth/login` with `{"email": "b@ex.test", "password": "wrong"}` — observe `401 ACCOUNT_SUSPENDED`.
3. `POST /api/v1/auth/login` with `{"email": "unknown@ex.test", "password": "wrong"}` — observe `401 INVALID_CREDENTIALS`.
4. Expected (after fix): both responses are `401 INVALID_CREDENTIALS` because password verification runs first for both users and fails identically. `ACCOUNT_SUSPENDED` should only surface when the password IS correct.

## Suggested approach
1. In `backend/servers/api-server/src/routes/auth.rs` at the login handler (around line 699), extract the current sequence into two labeled blocks: (a) fetch-user + verify-password, (b) apply-account-state (suspended / locked / mfa-required).
2. Move the `if user.is_suspended { return 401 ACCOUNT_SUSPENDED }` check to run **after** `.verify_password(&req.password, &user.password_hash)?` succeeds. On password-verify failure keep returning the current generic `INVALID_CREDENTIALS` for both suspended and non-suspended users.
3. Preserve the argon2 timing-oracle mitigation: if the user is not found, still run a dummy `verify_password` against a stored dummy hash (unchanged from current behavior — do not skip it) so response times remain constant for unknown vs. found emails.
4. Regenerate any handler-level log lines that used to fire on the pre-verify suspended branch (`tracing::info!("login rejected: suspended")` etc.) so log fidelity survives the reorder — move them into the post-verify branch.
5. Grep the rest of `routes/auth.rs` for any other `is_suspended` / `is_locked` / status-derived branch that also runs before password verify, and apply the same reorder.

## Alternatives considered
- **Uniform error response, hidden discriminator in a header** — rejected because a client-facing enumeration oracle is only closed if the RESPONSE STATUS + BODY are identical; a hidden discriminator is trivially observed and re-opens the oracle.
- **Rate-limit `/login` at the IP + email level to blunt enumeration** — rejected as a stand-alone fix because rate-limits slow enumeration but do not eliminate the discriminator; the ordering bug should be closed at the source, and rate-limiting (partially landed in #2234 for other endpoints) is a defense-in-depth on top, not a replacement.

## Root-cause trace
1. Symptom: `POST /login` returns distinct `ACCOUNT_SUSPENDED` for wrong-password on suspended emails, `INVALID_CREDENTIALS` for wrong-password on unknown emails — observable in the failing test from *Repro* step 4.
2. ← Immediate cause at `backend/servers/api-server/src/routes/auth.rs:707` — the suspended-status guard emits its response before password verification.
3. ← Upstream cause at `backend/servers/api-server/src/routes/auth.rs` login handler shape — the "check account state, then verify secrets" ordering was likely written to short-circuit expensive argon2 for suspended accounts, an optimization at the cost of the oracle.
4. Origin: pre-existing since login handler was first authored; not introduced by any specific recent PR (the file's #2234 / #2250 / #2261 churn is all elsewhere).

## Test plan
- [ ] `backend/servers/api-server/tests/auth_tests.rs` — new test `login_suspended_wrong_password_returns_generic_invalid_credentials`: seed a suspended user, POST wrong password → assert `401` **and** `body.code == "INVALID_CREDENTIALS"` (NOT `ACCOUNT_SUSPENDED`).
- [ ] Companion test `login_suspended_right_password_returns_account_suspended`: same seed, POST the correct password → `401 ACCOUNT_SUSPENDED` (this branch stays reachable — the code is intentional, just gated on password verification succeeding).
- [ ] Non-regression: `login_active_wrong_password_returns_invalid_credentials` (already exists) still passes.
- [ ] Non-regression: `login_unknown_email_returns_invalid_credentials` (already exists) still passes and the response timing does not diverge (keep the dummy-verify path in place).
- [ ] Command: `cargo test -p api-server --test auth_tests -- login_suspended`

## Out of scope
- MFA and login-throttling changes — orthogonal.
- Redesigning the `ACCOUNT_SUSPENDED` code or its logged-in surface — unchanged.
- Auditing other handlers (registration, resend-verification, forgot-password) for similar oracles — worth a follow-up but not in this plan.

## After-merge
- Move this file to `plans/_archive/security-code-review-api-handlers-suspended-enum-oracle.md`
- Mark backlog row `security-code-review-api-handlers-suspended-enum-oracle` as `status: "done"`
