# code-review-api-handlers-suspended-enum-oracle

**Vector:** bug
**Score:** 3
**Source:** Rotating expert review 2026-07-12 (api-handlers segment) — signal id `code-review-api-handlers-suspended-enum-oracle`
**Confidence:** high

## Hypothesis

The login handler in `backend/servers/api-server/src/routes/auth.rs:699` returns 401 `ACCOUNT_SUSPENDED` for a suspended account BEFORE verifying the password (password check starts at line 713). A caller submitting an arbitrary password against a suspended email gets the distinct `ACCOUNT_SUSPENDED` response body, while an unknown email gets the generic `INVALID_CREDENTIALS` — an account-enumeration oracle for suspended users, reachable by any unauthenticated caller. This is the exact anti-pattern the codebase already fixed once at lines 750-756: the `EMAIL_NOT_VERIFIED` gate carries an explicit comment ("This MUST come after the password check … turns login into an account-enumeration oracle … #956") and was moved to run after `verify_password`. The suspended-account branch was not given the same treatment. The fix mirrors the verification-gate treatment: move the `status == "suspended"` check to run after `verify_password` returns `Ok(true)`.

## Evidence

- `backend/servers/api-server/src/routes/auth.rs:699` — `if user.status == "suspended"` returns 401 `ACCOUNT_SUSPENDED` before `verify_password` is ever called.
- `backend/servers/api-server/src/routes/auth.rs:713` — actual `state.auth_service.verify_password(...)` call.
- `backend/servers/api-server/src/routes/auth.rs:733-748` — `INVALID_CREDENTIALS` response for wrong password against any (unknown-or-known) email.
- `backend/servers/api-server/src/routes/auth.rs:750-756` — precedent: `!user.is_verified()` gate now runs AFTER password check with the anti-oracle comment citing PR #956.
- `record_login_attempt` calls in both branches — must remain (audit trail); the reorder must preserve them.

## Files

- `backend/servers/api-server/src/routes/auth.rs:699`
- `backend/servers/api-server/src/routes/auth.rs:713`
- `backend/servers/api-server/src/routes/auth.rs:750`

## Dependencies

<none>

## Required capabilities

- [x] C1 — Systematic debugging (security-adjacent reorder; must preserve login_attempt telemetry + all downstream MFA/policy gates)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode (auto-derived from the ticks):**
Mode: cloud-ok

## Repro steps

1. Seed a user with `status = "suspended"` and a known password hash.
2. `curl -sS -X POST http://localhost:8080/api/v1/auth/login -d '{"email":"suspended@example.com","password":"wrong"}'` — observed 401 with body `{"code":"ACCOUNT_SUSPENDED","message":"Account suspended. Contact support."}`.
3. `curl -sS -X POST http://localhost:8080/api/v1/auth/login -d '{"email":"nonexistent@example.com","password":"wrong"}'` — observed 401 with body `{"code":"INVALID_CREDENTIALS","message":"Invalid email or password"}`.
4. The two response bodies differ deterministically on the email — the enumeration signal. Expected after fix: both requests return `INVALID_CREDENTIALS`; only after `verify_password` returns `Ok(true)` does the caller receive `ACCOUNT_SUSPENDED`.

## Suggested approach

1. Move the `if user.status == "suspended" { … }` block (currently `auth.rs:699-711`) to run **after** the `if !password_valid { return … INVALID_CREDENTIALS … }` block (currently around line 733-748), i.e. co-locate it with the `!user.is_verified()` gate at line 750.
2. Preserve the pre-existing `record_login_attempt(&req.email, &ip_address, false)` audit call in the moved branch — telemetry on suspended-account login attempts must still fire (it's how the ops team notices credential stuffing against suspended accounts).
3. Add a one-line comment above the moved block that mirrors the `EMAIL_NOT_VERIFIED` comment ("This MUST come after the password check — pre-password ACCOUNT_SUSPENDED is an account-enumeration oracle") so future refactors don't undo it.
4. Preserve ordering with the verification gate — either can go first once both are post-password (recommend: suspended check first, then verification; suspended accounts should not be told to verify their email).
5. No other handler changes; the fix is entirely local to the `login()` function body.

## Alternatives considered

- **Return `INVALID_CREDENTIALS` for suspended accounts and drop `ACCOUNT_SUSPENDED` entirely** — rejected because the code intentionally surfaces the suspension to a genuine account holder (who has entered the correct password) so they know to contact support. Silently claiming "invalid credentials" for a valid-password suspended user would produce confusing tickets.
- **Add a rate limiter to the pre-password branch instead of reordering** — rejected because rate limiting can slow enumeration but does not close it; the anti-oracle pattern (established by #956 for verification) is to gate on password check first, and the codebase already has that pattern.

## Root-cause trace

1. Symptom: distinct 401 body (`ACCOUNT_SUSPENDED` vs `INVALID_CREDENTIALS`) leaks whether an email is registered-and-suspended, no password knowledge required.
2. ← `auth.rs:699-711`: suspended-status check runs before `verify_password` (line 713).
3. ← Original suspended-status feature was added independently of the #956 verification-gate fix; the anti-oracle pattern established there was not applied to the suspended branch.
4. Origin: initial account-suspension feature (predates PR #956). PR #956 moved the verification gate to post-password and named the enumeration risk in the code comment, but the sibling suspended branch was not moved.

## Test plan

- [ ] `backend/servers/api-server/tests/auth_tests.rs` — extend the existing `login::*` module (mirrors the empty-cookie tests added in PR #2270's pattern) with two new cases:
      - `login::suspended_account_wrong_password_returns_generic_invalid_credentials` — POST `/api/v1/auth/login` with a wrong password against a suspended user; asserts 401 body `code == "INVALID_CREDENTIALS"` (was `ACCOUNT_SUSPENDED`).
      - `login::suspended_account_correct_password_returns_account_suspended` — POST `/api/v1/auth/login` with the correct password against a suspended user; asserts 401 body `code == "ACCOUNT_SUSPENDED"` (regression guard on the intended UX for legitimate suspended users).
- [ ] Regression command: `cargo test -p api-server --test auth_tests login::suspended_account`.
- [ ] IG3 evidence: `suspended_account_wrong_password_returns_generic_invalid_credentials` must fail on `dev` today (`ACCOUNT_SUSPENDED` observed, `INVALID_CREDENTIALS` expected) and pass after the reorder.

## Out of scope

- Auditing the sibling handler `/auth/mfa/verify` for enumeration oracles (a separate signal covered by past issue #2159 — already merged fix).
- Broader review of the ~437 `ErrorResponse::new("DB_ERROR", e.to_string())` info-disclosure pattern (`code-review-api-handlers-raw-db-error-leak` — separate plan candidate).
- Rate limiting `/login` beyond the existing `check_rate_limit` guard at line 555.

## After-merge

- Move this file to `plans/_archive/code-review-api-handlers-suspended-enum-oracle.md`
- Mark the matching `backlog.json` row as `status: "done"`
