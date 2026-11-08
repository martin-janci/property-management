# code-review-api-handlers-mfa-regen-codes-no-rate-limit

**Vector:** security
**Score:** 3
**Source:** Phase 1.5 rotating expert review 2026-11-08 (api-handlers segment)
**Confidence:** high

## Hypothesis
`POST /api/v1/auth/mfa/backup-codes/regenerate` verifies a 6-digit TOTP code and, on success, rotates all of the user's recovery codes (invalidating the printed set). It applies **no per-user rate limiter** — asymmetric with `verify_mfa_setup` (line 329) and `verify_recovery_code` (line 1126), both of which already throttle the same 10^6-code brute force. An attacker with a stolen access token can brute-force the TOTP against `/regenerate`; a single hit both (a) bypasses MFA with attacker-controlled recovery codes and (b) locks the legitimate user out of their current printed recovery codes (worse than just disabling MFA, because audit shows only a normal regeneration). Apply the same per-user sliding-window throttle before `totp_service.verify_code` is called.

## Evidence
- `backend/servers/api-server/src/routes/mfa.rs:912-1062` — `regenerate_backup_codes` handler: calls `totp_service.verify_code(&decrypted_secret, &req.code)` at line 981-993, returns `400 INVALID_CODE` at line 995-1003 on mismatch, then on success calls `regenerate_backup_codes_rls` (line 1019-1032) which writes the new hashed codes and invalidates the old set. No limiter invoked anywhere in the handler body.
- `backend/servers/api-server/src/routes/mfa.rs:329` — sibling `verify_mfa_setup` invokes `setup_verify_attempt_allowed` before any DB read and returns `429 RATE_LIMITED` on overflow.
- `backend/servers/api-server/src/routes/mfa.rs:1126` — sibling `verify_recovery_code` invokes `recovery_attempt_allowed` the same way.
- `backend/servers/api-server/src/routes/mfa.rs:87-103` — `setup_verify_attempt_allowed` + `setup_verify_attempts_reset` define the shared pattern this plan replicates: process-global `LazyLock<UserRateLimiter>` keyed on `user_id`, 10 attempts / 900s window.
- `backend/servers/api-server/src/routes/mfa.rs:109-116` — router mounts `/backup-codes/regenerate` at `post(regenerate_backup_codes)`, confirming the handler is live on `/api/v1/auth/mfa/backup-codes/regenerate` (reachability gate satisfied).

## Files
- `backend/servers/api-server/src/routes/mfa.rs:912`
- `backend/servers/api-server/tests/suites/mfa_brute_force_rate_limit_tests.rs`

## Dependencies
- code-review-api-handlers-mfa-disable-no-rate-limit

## Required capabilities
- [x] C1 — Systematic debugging (security bug; trace the auth boundary end-to-end)
- [ ] C2 — Seed data
- [x] C3 — Dev instance running (integration tests hit Postgres via SQLx)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

**Execution mode (auto-derived from the ticks):** `cloud-ok`

Mode: cloud-ok

## Repro steps
1. Boot a seeded api-server; register a user, enable MFA (capture the printed recovery codes), capture a valid access token.
2. Keeping that access token, POST to `/api/v1/auth/mfa/backup-codes/regenerate` with 20 different wrong 6-digit codes in sequence.
   - Expected (after fix): first ~10 attempts return `400 INVALID_CODE`, then every subsequent attempt returns `429 RATE_LIMITED` until the 900s window elapses.
   - Actual today: every attempt returns `400 INVALID_CODE`; no attempt counter; the 10^6 code space is enumerable unthrottled.
3. Submit the correct TOTP code once the window elapses — expected `200 OK` with new backup codes and the limiter counter cleared so the legitimate user is not penalised.
4. Separate-identity side effect to call out (worse than disable): if an attacker guesses a valid TOTP against `/regenerate`, the printed recovery codes the user holds on paper become invalid — the user is locked out and the audit log shows a normal `MfaBackupCodesRegenerated` event, not a suspicious disable.

## Suggested approach
1. Add a new sliding-window limiter for regenerate: `MFA_REGEN_RATE_LIMITER: LazyLock<UserRateLimiter>` + `MFA_REGEN_MAX_ATTEMPTS: u32 = 10` + `MFA_REGEN_WINDOW: Duration = Duration::from_secs(900)` + `MFA_REGEN_SWEEP_THRESHOLD: usize = 1024`, grouped with the existing `MFA_SETUP_VERIFY_*` and `MFA_RECOVERY_*` constants at `mfa.rs:38-83`.
2. Add `fn regen_attempt_allowed(user_id)` + `pub fn regen_attempts_reset(user_id)` mirroring the setup pair at `mfa.rs:87-103`.
3. In `regenerate_backup_codes` (`mfa.rs:912`), insert the limiter check immediately after extracting `user_id` from the JWT claims (before `two_factor_repo.get_by_user_id_rls` at line 930), returning `429 RATE_LIMITED` with the same error shape used elsewhere (`mfa.rs:331-338`).
4. After the successful regenerate (after the `regenerate_backup_codes_rls` call at line 1019-1032 succeeds, before the audit log), call `regen_attempts_reset(user_id)` so a legitimate user is never penalised by prior mistyped attempts.
5. Keep the `AuditAction::MfaBackupCodesRegenerated` and the `rls.release()` ordering unchanged — the limiter check must be first so no DB work happens for throttled attempts (same discipline as `verify_mfa_setup`).
6. Extend `backend/servers/api-server/tests/suites/mfa_brute_force_rate_limit_tests.rs` with one new case — see *Test plan*.
7. Run `cargo test -p api-server --test suite_2 mfa_brute_force_rate_limit`.

## Alternatives considered
- **Reuse `MFA_SETUP_VERIFY_RATE_LIMITER` for `regenerate_backup_codes`** — rejected because setup is a one-shot flow (TOTP verify → enable) while regenerate is a repeatable post-enable operation; sharing counters would let an attacker exhaust the legitimate user's setup-verification budget by targeting regenerate (or vice versa). Dedicated limiters keep per-flow semantics clean.
- **Require re-entering the account password instead of adding a limiter** — rejected because it does not solve the brute-force exposure on its own (an attacker with password + session can still brute-force the TOTP post-password-step), and it adds a UX regression for a legitimate user rotating codes. A rate limiter is the minimal correct fix; password confirmation is a separate hardening worth a follow-up.

## Root-cause trace
1. Symptom: `POST /api/v1/auth/mfa/backup-codes/regenerate` with a wrong TOTP returns `400 INVALID_CODE`; a loop of 10^6 attempts eventually guesses correctly and rotates recovery codes under attacker control.
2. ← `backend/servers/api-server/src/routes/mfa.rs:995-1003` returns `BAD_REQUEST` on mismatch, no counter touched.
3. ← `backend/servers/api-server/src/routes/mfa.rs:912-928` handler entry does NOT call `regen_attempt_allowed` (function does not exist) — unlike `verify_mfa_setup:329` and `verify_recovery_code:1126`.
4. Origin: same provenance as the sibling disable-handler gap — the regenerate handler was added alongside the setup/verify handlers, but the brute-force discipline retrofit (issue #2159) only patched `verify_mfa_setup`. The regenerate and disable handlers were missed in that sweep.

## Test plan
- [ ] `backend/servers/api-server/tests/suites/mfa_brute_force_rate_limit_tests.rs` — new test `test_mfa_regenerate_backup_codes_rate_limited`: enable MFA for a user, POST `/api/v1/auth/mfa/backup-codes/regenerate` with 11 wrong codes; assert first 10 return `400 INVALID_CODE` and the 11th returns `429 RATE_LIMITED`. Assert no `MfaBackupCodesRegenerated` audit row was written and the existing recovery codes are still valid.
- [ ] Regression (IG3): the new test MUST fail on `main` (returns 400 for all attempts) and pass after the fix.
- [ ] Command: `cargo test -p api-server --test suite_2 test_mfa_regenerate_backup_codes_rate_limited -- --nocapture`

## Out of scope
- Any change to `verify_mfa_setup`, `verify_recovery_code`, or `disable_mfa` (disable is covered by the sibling plan `code-review-api-handlers-mfa-disable-no-rate-limit.md`).
- Switching the limiter backend to Redis — same discipline as the existing in-process limiters; follow convention.
- Requiring password re-confirmation on `/regenerate` — worthwhile defense-in-depth but tracked separately; not needed for this fix.

## After-merge
- Move this file to `plans/_archive/code-review-api-handlers-mfa-regen-codes-no-rate-limit.md`
- Mark the matching `backlog.json` row (`code-review-api-handlers-mfa-regen-codes-no-rate-limit`) as `status: "done"`
