# code-review-api-handlers-mfa-disable-no-rate-limit

**Vector:** security
**Score:** 3
**Source:** Phase 1.5 rotating expert review 2026-11-08 (api-handlers segment)
**Confidence:** high

## Hypothesis
`POST /api/v1/auth/mfa/disable` accepts a 6-digit TOTP code (or recovery code) and returns `400 INVALID_CODE` on mismatch, but applies **no per-user rate limiter**. An attacker who has stolen a valid access token (phished, XSS'd, or leaked via logs) can brute-force the ~10^6 TOTP code space in minutes to disable MFA — a full MFA bypass. The sibling setup-verification handler (`verify_mfa_setup`, line 329) and the recovery-code verifier (`verify_recovery_code`, line 1126) already carry per-user sliding-window throttles (`setup_verify_attempt_allowed` / `recovery_attempt_allowed`). Mirror that pattern inside `disable_mfa`: check the limiter before any DB access, return `429 RATE_LIMITED` when exceeded, and reset the counter on successful disable.

## Evidence
- `backend/servers/api-server/src/routes/mfa.rs:591-803` — `disable_mfa` handler: receives `DisableMfaRequest { code }`, calls `totp_service.verify_code(&decrypted_secret, &req.code)` at line 660-663 and the recovery-code lookup at line 672-696, then returns `400 INVALID_CODE` at line 700-709. No limiter invoked anywhere in the handler body.
- `backend/servers/api-server/src/routes/mfa.rs:329` — `verify_mfa_setup` calls `setup_verify_attempt_allowed(user_id)` before any DB read; the (N+1)th attempt returns `429 RATE_LIMITED`.
- `backend/servers/api-server/src/routes/mfa.rs:1126` — `verify_recovery_code` calls `recovery_attempt_allowed(user_id)` the same way.
- `backend/servers/api-server/src/routes/mfa.rs:87-103` — `setup_verify_attempt_allowed` + `setup_verify_attempts_reset` — process-global `LazyLock<UserRateLimiter>` keyed on `user_id`, 10 attempts / 900s window, sweep threshold 1024. Same shape at line 50-67 for recovery.
- `backend/servers/api-server/src/routes/mfa.rs:109-116` — router mounts `/disable` at `post(disable_mfa)`, confirming the handler is live on `/api/v1/auth/mfa/disable` (reachability gate satisfied).

## Files
- `backend/servers/api-server/src/routes/mfa.rs:591`
- `backend/servers/api-server/tests/suites/mfa_brute_force_rate_limit_tests.rs`

## Dependencies


## Required capabilities
- [x] C1 — Systematic debugging (security bug; trace the auth boundary end-to-end before touching the handler)
- [ ] C2 — Seed data
- [x] C3 — Dev instance running (integration tests hit Postgres via SQLx)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

**Execution mode (auto-derived from the ticks):** `cloud-ok`

Mode: cloud-ok

## Repro steps
1. Boot a seeded api-server; register a user, enable MFA, capture a valid access token.
2. Keeping that access token, POST to `/api/v1/auth/mfa/disable` with 20 different wrong 6-digit codes in sequence (e.g. `000000`, `000001`, …).
   - Expected (after fix): the first ~10 attempts return `400 INVALID_CODE`, then every subsequent attempt returns `429 RATE_LIMITED` until the 900s window elapses.
   - Actual today: every attempt returns `400 INVALID_CODE`; no attempt counter; a loop can exhaust the 10^6 space unthrottled.
3. Submit the correct TOTP code once the window elapses — expected `200 OK` (disabled) and the counter is cleared so the user is not penalised post-fix.

## Suggested approach
1. Add a new sliding-window limiter dedicated to disable: `MFA_DISABLE_RATE_LIMITER: LazyLock<UserRateLimiter>` + `MFA_DISABLE_MAX_ATTEMPTS: u32 = 10` + `MFA_DISABLE_WINDOW: Duration = Duration::from_secs(900)` + `MFA_DISABLE_SWEEP_THRESHOLD: usize = 1024`, near the existing `MFA_SETUP_VERIFY_*` constants at `mfa.rs:78-83`.
2. Add `fn disable_attempt_allowed(user_id)` + `pub fn disable_attempts_reset(user_id)` mirroring the setup pair at `mfa.rs:87-103`.
3. In `disable_mfa` (`mfa.rs:591`), insert the limiter check immediately after extracting `user_id` from the JWT claims (before the `two_factor_repo.get_by_user_id_rls` call at line 609), returning the same `429 RATE_LIMITED` shape used at `mfa.rs:331-338`.
4. After the successful disable (after `tx.commit()` at line 765, before the audit log), call `disable_attempts_reset(user_id)` so a legitimate user is never penalised by earlier mistyped codes — mirror the reset pattern at `verify_mfa_setup` (success path calls `setup_verify_attempts_reset`).
5. Keep the TOTP vs recovery-code branch unchanged — both paths share the single limiter (consistent with how `verify_mfa_setup` throttles TOTP attempts regardless of success shape).
6. Extend `backend/servers/api-server/tests/suites/mfa_brute_force_rate_limit_tests.rs` with one new case — see *Test plan*. The file's top-level doc already documents the brute-force rationale; the new case extends the same discipline.
7. Run `cargo test -p api-server --test suite_2 mfa_brute_force_rate_limit` locally; confirm the new case fails on `main` and passes after the fix.

## Alternatives considered
- **Reuse `MFA_SETUP_VERIFY_RATE_LIMITER` for `disable_mfa`** — rejected because a legitimate user's setup-flow attempts would prematurely lock the disable path (two unrelated flows sharing one counter). Separate limiters keep the semantics clean, and the memory cost of a second `HashMap<Uuid, SlidingWindow>` with a sweep threshold of 1024 is negligible.
- **Rely on the login-path rate limiter (`SessionRepository::check_rate_limit`)** — rejected because that limiter keys on email/IP during login, not on `user_id` of an authenticated caller. An attacker with a stolen access token never hits the login path.

## Root-cause trace
1. Symptom: `POST /api/v1/auth/mfa/disable` with a wrong TOTP code returns `400 INVALID_CODE`; a loop of 10^6 attempts eventually guesses a valid code and disables MFA.
2. ← `backend/servers/api-server/src/routes/mfa.rs:700-709` returns `BAD_REQUEST` on mismatch, no counter touched.
3. ← `backend/servers/api-server/src/routes/mfa.rs:591-608` handler entry does NOT call `disable_attempt_allowed` (function does not exist) — unlike `verify_mfa_setup:329` and `verify_recovery_code:1126`.
4. Origin: the disable handler was added before the per-user brute-force discipline landed (issue #2159 added `setup_verify_attempt_allowed` in PR that followed PR #473). The asymmetry was never audited after the setup/recovery handlers got their limiters.

## Test plan
- [ ] `backend/servers/api-server/tests/suites/mfa_brute_force_rate_limit_tests.rs` — new test `test_mfa_disable_rate_limited`: enable MFA for a user, POST `/api/v1/auth/mfa/disable` with 11 wrong codes; assert first 10 return `400 INVALID_CODE` and the 11th returns `429 RATE_LIMITED`. Then assert MFA is still enabled (`get_by_user_id` row `enabled = true`).
- [ ] Regression (IG3): the new test MUST fail on `main` (returns 400 for all attempts) and pass after the fix.
- [ ] Command: `cargo test -p api-server --test suite_2 test_mfa_disable_rate_limited -- --nocapture`

## Out of scope
- Any change to `verify_mfa_setup`, `verify_recovery_code`, or `regenerate_backup_codes` (that last one is covered by the sibling plan `code-review-api-handlers-mfa-regen-codes-no-rate-limit.md`).
- Switching the limiter backend from in-process `LazyLock<HashMap>` to Redis (same discipline as the existing limiters).
- Audit-log changes — `AuditAction::MfaDisabled` already fires on the success path; no new audit variant needed.

## After-merge
- Move this file to `plans/_archive/code-review-api-handlers-mfa-disable-no-rate-limit.md`
- Mark the matching `backlog.json` row (`code-review-api-handlers-mfa-disable-no-rate-limit`) as `status: "done"`
