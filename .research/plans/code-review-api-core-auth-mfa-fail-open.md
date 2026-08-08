# code-review-api-core-auth-mfa-fail-open

**Vector:** security
**Score:** 3
**Source:** rotating-expert-review 2026-08-08 — code path: `backend/servers/api-server/src/routes/auth/mod.rs:388`
**Confidence:** high

## Hypothesis
The `login` handler skips MFA verification when the `two_factor_repo.get_by_user_id(user.id)` call returns `Err(...)`. The `if let Ok(Some(mfa_record))` pattern silently drops the Err arm, so a user with personal 2FA enabled who trips a transient DB error (pool exhaustion, statement timeout, RLS-context glitch on the deprecated code path) has the entire MFA verification block skipped. Because `mfa_presented` stays `false` and `AuthPolicyEnforcer::check_login` short-circuits when no org policy demands MFA for the role, the handler mints access + refresh tokens on email+password alone — a full second-factor bypass triggered by a recoverable DB error. The fix is to convert the `if let Ok(Some(...))` into a `match` that fails closed on `Err` (500 INTERNAL_ERROR), preserving the existing `Ok(None) = no MFA row` fast path.

## Evidence
- `backend/servers/api-server/src/routes/auth/mod.rs:386-388` — `#[allow(deprecated)] let mfa_check_result = state.two_factor_repo.get_by_user_id(user.id).await; if let Ok(Some(mfa_record)) = mfa_check_result { ... }` — the `Err(_)` and `Ok(None)` arms collapse to the same "skip MFA" path.
- Downstream: `AuthPolicyEnforcer::check_login` returns Ok whenever `policy.mfa_required_for(&m.role)` is false (see `services/auth_policy.rs`), so the personal-2FA + no-org-policy user has no second gate.
- Contrast: sibling `verify_two_factor` handler and the admin MFA endpoints treat repo errors as 500, not "skip". This is the outlier.

## Files
- `backend/servers/api-server/src/routes/auth/mod.rs`
- `backend/servers/api-server/tests/suites/mfa_e2e_tests.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (bug/security)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. Seed a user with `two_factor_auth.enabled = true` (existing test fixture in `mfa_e2e_tests.rs`).
2. Wrap `two_factor_repo.get_by_user_id` in a test seam that returns `Err(sqlx::Error::PoolTimedOut)` (or inject a broken pool for that one call).
3. Call `POST /api/v1/auth/login` with the user's email + password (no `two_factor_code`).
4. Expected: `500 INTERNAL_ERROR` (or a specific `MFA_LOOKUP_FAILED` code). Actual on `dev`: `200 OK` with an access-token + refresh-token pair — MFA silently skipped.

## Suggested approach
1. Rewrite the block at `routes/auth/mod.rs:386-388` from `if let Ok(Some(mfa_record)) = ...` to an explicit `match mfa_check_result { Ok(Some(rec)) => { ... existing body ... }, Ok(None) => { /* no MFA configured — proceed */ }, Err(e) => { tracing::error!(user_id = %user.id, error = %e, "MFA lookup failed — failing closed"); return Err(err_response(StatusCode::INTERNAL_SERVER_ERROR, "MFA_LOOKUP_FAILED", "Unable to verify second factor. Please retry.")); } }`.
2. Confirm no other call site in `routes/auth/mod.rs` shares the same pattern for the two-factor lookup (grep `get_by_user_id` in the file).
3. Add a regression test `mfa_lookup_error_fails_closed_returns_500` under `backend/servers/api-server/tests/suites/mfa_e2e_tests.rs` (or a new focused file) that constructs an `AppState` whose `two_factor_repo` returns `Err(sqlx::Error::PoolTimedOut)` for `get_by_user_id`, drives the login route, and asserts the response is 500 with body code `MFA_LOOKUP_FAILED` and no `Set-Cookie` for the refresh token. Use the existing `RepositoryFactory`/injection pattern — the current suite already stubs repos.
4. Verify the failing test reproduces on origin/dev before the fix (IG3).
5. Run `cargo test -p api-server --test suites -- mfa_lookup_error_fails_closed_returns_500` after the fix to confirm green.
6. Run `cargo clippy -p api-server --all-targets -- -D warnings` to catch the `#[allow(deprecated)]` interaction (the match arm order is stable across the deprecation window).

## Alternatives considered
- **Retry once on Err before failing closed** — rejected because a token-issuance path should not silently absorb latency; a single 500 with a clear error code gives the client a deterministic signal to retry, and the auth handler is already short.
- **Log-and-continue (current shape with a `warn!`)** — rejected because it preserves the bypass. The failure mode is a second-factor skip on a user who has explicitly enabled 2FA; visibility without behavior change still means the token is minted.

## Root-cause trace
1. Symptom: user with personal 2FA enabled + transient DB error on `get_by_user_id` receives valid access + refresh tokens without ever entering a TOTP code.
2. ← `routes/auth/mod.rs:388` `if let Ok(Some(mfa_record)) = mfa_check_result { ... }` — the `Err` and `Ok(None)` arms collapse to "skip MFA".
3. ← `mfa_presented` remains `false` at `routes/auth/mod.rs:382`; nothing sets it in the skip path.
4. ← `AuthPolicyEnforcer::check_login` short-circuits at `services/auth_policy.rs:299` when `policy.mfa_required_for(&m.role) == false`, so the personal-2FA opt-in is invisible to the policy layer.
5. Origin: PR that introduced the `#[allow(deprecated)] let mfa_check_result = ...; if let Ok(Some(...))` shape (2FA-at-login wiring, Epic 9 Story 9.1). Root cause is the `if let Ok(...)` idiom collapsing two semantically-different Err/None cases into the same branch — a Rust pattern-matching pitfall.

## Test plan
- [ ] New test `mfa_lookup_error_fails_closed_returns_500` in `backend/servers/api-server/tests/suites/mfa_e2e_tests.rs` — asserts 500 + `MFA_LOOKUP_FAILED` code + no refresh-token cookie when repo returns Err.
- [ ] Existing tests remain green (`admin_mfa_step_up_tests.rs`, `mfa_e2e_tests.rs`, `mfa_brute_force_rate_limit_tests.rs`).
- [ ] Exact command: `cd backend && cargo test -p api-server --test suites -- mfa` — runs all MFA suites.

## Out of scope
- Refactoring the deprecated `get_by_user_id` code path (called out with `#[allow(deprecated)]`) — this plan preserves the existing method call and only changes the error handling around it.
- Broader audit of other `if let Ok(Some(...))` patterns across the codebase — separate refactor pass.
- Changing `AuthPolicyEnforcer` — the policy layer is correct; the bypass is in the caller.

## After-merge
- Move this file to `plans/_archive/code-review-api-core-auth-mfa-fail-open.md`
- Mark the matching `backlog.json` row as `status: "done"`
