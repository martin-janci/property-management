# security-suspended-account-enum-oracle

**Vector:** security
**Score:** 3
**Source:** Phase 1.5 code-review 2026-07-12 (api-handlers segment) — `backend/servers/api-server/src/routes/auth.rs:699-717`
**Confidence:** high

## Hypothesis
The login handler in `routes/auth.rs` returns `401 ACCOUNT_SUSPENDED` for suspended accounts **before** verifying the password (the suspended check is at line 699; the password check begins at line 715). Any caller submitting an arbitrary password against a suspended email gets a distinct `ACCOUNT_SUSPENDED` response, while an unknown email gets the generic `INVALID_CREDENTIALS`. That distinguishability is an account-enumeration oracle for the "which of my target emails is on your suspended-users list" question — the exact class of leak that PR #956 (the email-verification gate) deliberately closed by moving the `EMAIL_NOT_VERIFIED` check to *after* the password check. Move the suspended-account check to the same place: after `verify_password` returns `Ok(true)`.

## Evidence
- `backend/servers/api-server/src/routes/auth.rs:699-711` — `if user.status == "suspended" { return Err ... ACCOUNT_SUSPENDED }` runs before any password verification.
- `backend/servers/api-server/src/routes/auth.rs:715-732` — `verify_password` starts here; the branch that returned `INVALID_CREDENTIALS` for a wrong password is at line 736.
- `backend/servers/api-server/src/routes/auth.rs:750-756` — comment explicitly documents the anti-pattern the suspended branch still exhibits: "returning `EMAIL_NOT_VERIFIED` before verifying the password turns login into an account-enumeration oracle (#956)". The email-verification branch has been correctly moved post-password-check; the suspended branch was not.
- `backend/servers/api-server/tests/auth_enumeration_tests.rs:1-16` — module doc explicitly enumerates two closed oracles (unverified-email + register EMAIL_EXISTS). No coverage for the suspended-account oracle exists — the third leak in the same class.

## Files
- `backend/servers/api-server/src/routes/auth.rs:699`
- `backend/servers/api-server/tests/auth_enumeration_tests.rs`

## Dependencies
_None — self-contained handler + test change._

## Required capabilities
- [x] C1 — Systematic debugging (bug/security in an auth path)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode:** derived from the ticks below.

Mode: cloud-ok

## Repro steps
1. Register a fresh test user, verify the email (`verify_user_email` helper), then flip `users.status = 'suspended'` via SQL against the test DB.
2. Send `POST /api/v1/auth/login` with the correct email and an **arbitrary wrong password**.
3. **Expected (post-fix):** `401 INVALID_CREDENTIALS` — indistinguishable from an unknown email.
4. **Actual (pre-fix):** `401 ACCOUNT_SUSPENDED` — the `code` field discloses that the email is registered and administratively suspended.

## Suggested approach
1. In `backend/servers/api-server/src/routes/auth.rs`, cut the `if user.status == "suspended" { … }` block currently at lines 699-711 and paste it **after** the `if !password_valid { … }` block that ends around line 748, **before** the `is_verified()` gate at line 754.
2. Keep the `record_login_attempt(false)` side-effect inside the suspended branch as it currently is — a login attempt against a suspended account is still a failed login for lockout accounting, and the caller has already proven the password so we're no longer leaking the suspended state to an unauthenticated probe.
3. Preserve the response body verbatim (`ACCOUNT_SUSPENDED` / `Account suspended. Contact support.`) — the fix is about **when** the branch runs, not what it returns; suspended users who *do* enter the right password still get the same actionable message.
4. Add three regression tests to `backend/servers/api-server/tests/auth_enumeration_tests.rs` (mirrors the naming of the existing `login_unverified_*` set):
   - `login_suspended_wrong_password_returns_generic_invalid_credentials` — seed a verified-then-suspended user, wrong password → expect `code == "INVALID_CREDENTIALS"` (this is the failing-on-main IG3 test).
   - `login_suspended_correct_password_still_blocks_with_account_suspended` — seed a verified-then-suspended user, correct password → expect `code == "ACCOUNT_SUSPENDED"` (pins that the actionable message still surfaces to legitimate callers).
   - `login_unknown_email_wrong_password_and_suspended_wrong_password_are_indistinguishable` — assert both responses have identical HTTP status, `code`, and body shape (defensive against future divergence).
5. Update the module doc at the top of `auth_enumeration_tests.rs` to enumerate the third closed oracle (suspended-account) alongside the existing two.
6. Run `cargo test -p api-server --test auth_enumeration_tests` locally; expect all 8 tests to pass (5 existing + 3 new).
7. Re-run `cargo clippy -p api-server --lib -- -D warnings` and `cargo test -p api-server --lib routes::auth`; both must remain clean.

## Alternatives considered
- **Return `INVALID_CREDENTIALS` for suspended accounts too, wholesale** — rejected because it removes the actionable "Contact support" message from legitimate suspended users (who correctly proved their password) and forces them into a support loop. The oracle fix requires moving *when* we surface the message, not eliminating it.
- **Add a per-email rate limit to the pre-password suspended branch to blunt scraping** — rejected because it doesn't close the oracle (a low-rate scraper still enumerates over time) and layers extra state on top of a fix that costs one code move. Rate-limiting the login endpoint is orthogonal work (already covered by `check_rate_limit` at line 555).

## Root-cause trace
1. Symptom: `POST /api/v1/auth/login` with `{email: <suspended>, password: <anything>}` returns `code=ACCOUNT_SUSPENDED`, distinguishable from unknown-email's `code=INVALID_CREDENTIALS`.
2. ← `backend/servers/api-server/src/routes/auth.rs:699` — the suspended check runs at handler-order position 2 (after user lookup, before password verify).
3. ← `backend/servers/api-server/src/routes/auth.rs:750` — the email-verification branch was moved post-password on #956, but the suspended branch (added earlier) was not migrated.
4. Origin: the `ACCOUNT_SUSPENDED` branch pre-dates #956; the #956 fix touched only the `EMAIL_NOT_VERIFIED` branch and did not sweep the sibling `ACCOUNT_SUSPENDED` branch that lives in the same handler with the same oracle shape.

## Test plan
- [ ] `backend/servers/api-server/tests/auth_enumeration_tests.rs::login_suspended_wrong_password_returns_generic_invalid_credentials` — new; fails on `main`, passes after the move (IG3).
- [ ] `backend/servers/api-server/tests/auth_enumeration_tests.rs::login_suspended_correct_password_still_blocks_with_account_suspended` — pins that suspended users with a correct password still get the actionable message.
- [ ] `backend/servers/api-server/tests/auth_enumeration_tests.rs::login_unknown_email_wrong_password_and_suspended_wrong_password_are_indistinguishable` — cross-check for future divergence.
- [ ] `cd backend && cargo test -p api-server --test auth_enumeration_tests` — all 8 tests green.
- [ ] `cd backend && cargo test -p api-server --lib routes::auth` — no in-crate regression.
- [ ] `cd backend && cargo clippy -p api-server --lib -- -D warnings` — clean.

## Out of scope
- The systemic `e.to_string()` DB-error information disclosure across ~437 handler arms — tracked separately as `code-review-api-handlers-raw-db-error-leak` (score 2).
- Any change to the suspended-status detection lifecycle (how accounts get marked suspended, how they get reactivated).
- Rate-limiting on `/login` — already applied via `check_rate_limit` at line 555.
- The `record_login_attempt` fire-and-forget failure-swallow (`let _ = …await`) — real but lower-priority; deferred finding from the same review slice.

## After-merge
- Move this file to `plans/_archive/security-suspended-account-enum-oracle.md`
- Mark the matching `backlog.json` row (`code-review-api-handlers-suspended-enum-oracle`) as `status: "done"`; append `"resolved: PR #<N> merged <date> — <title>"` to `evidence`.
