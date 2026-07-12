# code-review-api-handlers-suspended-enum-oracle

**Vector:** security
**Score:** 3
**Source:** rotating-expert-review
**Confidence:** high

## Hypothesis
The login handler in `backend/servers/api-server/src/routes/auth.rs` returns `401 ACCOUNT_SUSPENDED` for a suspended account **before** the password check. Any caller can distinguish a suspended email (specific error) from an unknown email (generic `INVALID_CREDENTIALS`) — an account-enumeration oracle on the highest-value auth endpoint. The email-verification gate was moved *after* the password check for exactly this reason (PR #956 comment). The fix is to move the suspended-account gate to after `verify_password` too, returning `ACCOUNT_SUSPENDED` only when credentials are valid.

## Evidence
- backend/servers/api-server/src/routes/auth.rs:699 — suspended check runs before password verification (line 713).
- backend/servers/api-server/src/routes/auth.rs:750-756 — email-verification gate deliberately runs post-password, citing #956 for exactly the enumeration-oracle risk.
- Contrast: `unknown email` path returns generic `INVALID_CREDENTIALS`; `suspended email` path returns distinct `ACCOUNT_SUSPENDED` — differential response reveals account existence.

## Files
- `backend/servers/api-server/src/routes/auth.rs:699`
- `backend/servers/api-server/tests/auth_tests.rs`

## Dependencies

## Required capabilities
- [ ] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

Mode: cloud-ok

## Repro steps
1. Register a user, then use the admin flow / DB update to set `status = 'suspended'`.
2. Send `POST /api/v1/auth/login` with the suspended user's email and an arbitrary/wrong password.
3. Send the same request with an email that does not exist in the DB.
4. Observe: suspended → `401 ACCOUNT_SUSPENDED`; unknown → `401 INVALID_CREDENTIALS`. The differential response is the oracle.

## Suggested approach
1. In `login()` at `backend/servers/api-server/src/routes/auth.rs:699`, move the `if user.status == UserStatus::Suspended` branch to after `verify_password(...)` (lines 713-...) but before the email-verification gate.
2. Both the unknown-email path and the wrong-password path already return the generic `INVALID_CREDENTIALS`; the suspended branch will now only fire when the caller has proven credentials.
3. Preserve the current server-side log line (so ops still sees suspended-login attempts).
4. Add an integration test `login_suspended_account_leaks_no_enumeration_pre_password` in `tests/auth_tests.rs`: suspended-user + wrong password must return `INVALID_CREDENTIALS`, not `ACCOUNT_SUSPENDED`.
5. Add a second test asserting suspended + correct password still returns `ACCOUNT_SUSPENDED` (post-fix behaviour preserved).

## Alternatives considered
- **Return `INVALID_CREDENTIALS` uniformly and only log suspended server-side** — rejected because the UX contract on the client (a suspended-user banner) relies on the distinct code path when credentials are valid.
- **Add a random-delay jitter without changing the branch order** — rejected because the differential response body is the leak, not timing; delay alone doesn't close the oracle.

## Root-cause trace
1. Symptom: `POST /login {email: suspended, password: garbage}` → 401 `ACCOUNT_SUSPENDED`; `POST /login {email: nonexistent, password: garbage}` → 401 `INVALID_CREDENTIALS`.
2. ← Immediate cause at auth.rs:699 — suspended branch runs before `verify_password` at auth.rs:713.
3. ← Upstream cause: the suspended-account gate was authored independently of the email-verification gate, which was already moved post-password in response to #956 for the same enumeration-oracle class.
4. Origin: PR that introduced the suspended-account short-circuit (predates #956; the fix for #956 covered only email-verification, not suspended-account status).

## Test plan
- [ ] `login_suspended_account_wrong_password_returns_invalid_credentials` in `backend/servers/api-server/tests/auth_tests.rs` — asserts no `ACCOUNT_SUSPENDED` when credentials are wrong.
- [ ] `login_suspended_account_correct_password_still_returns_suspended` in the same file — preserves the post-auth contract.
- [ ] Run `cd backend && cargo test -p api-server --test auth_tests -- login_suspended` locally.

## Out of scope
Renaming error codes, rate-limiting suspended login attempts, timing-side-channel hardening (`verify_password` already uses constant-time comparison via argon2).

## After-merge
- Move this file to `plans/_archive/code-review-api-handlers-suspended-enum-oracle.md`
- Mark the matching `backlog.json` row as `status: "done"`
