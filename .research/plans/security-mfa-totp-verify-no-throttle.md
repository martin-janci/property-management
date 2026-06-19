# security-mfa-totp-verify-no-throttle

**Vector:** security
**Score:** 2
**Source:** Phase 1.5 rotating expert review (api-handlers segment), 2026-06-19
**Confidence:** high

## Hypothesis
`verify_mfa_setup` and `regenerate_backup_codes` both ask the user to prove a current TOTP, but neither caps the number of attempts per user. The TOTP keyspace is only 10^6 with a ~30-second window — an attacker with a valid access token can brute-force the 6-digit code online (~1.7 % chance per 30s slot at ~30 req/s for a single window). The fix is to plug the same per-user attempt-limiter pattern PR #1580 added on the recovery-code path into these two TOTP verifications.

## Evidence
- `backend/servers/api-server/src/routes/mfa.rs:288-380` — `verify_mfa_setup` performs `totp.check_current(code)` and proceeds, no attempt counter / lockout
- `backend/servers/api-server/src/routes/mfa.rs:942-965` — `regenerate_backup_codes` also verifies the current TOTP unguarded before issuing fresh backup codes (a high-value action — fresh recovery codes can be abused for later disable)
- `backend/servers/api-server/src/routes/mfa.rs:1087` — analogue: the limiter helper introduced by PR #1580 against `verify_recovery_code`
- PR #1602 (merged 2026-06-19) hardened the recovery-code limiter (stale-entry eviction) — TOTP paths still naked

## Files
- `backend/servers/api-server/src/routes/mfa.rs:288`
- `backend/servers/api-server/src/routes/mfa.rs:942`
- `backend/servers/api-server/tests/mfa_recovery_cross_user_idor_tests.rs`

## Dependencies

(none — limiter primitives already in tree from #1580/#1602)

## Required capabilities
- [x] C1 — Systematic debugging
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Mode: cloud-ok** (Rust unit/integration tests only)

## Repro steps
1. User `victim` has MFA enrolled.
2. Attacker obtains a `victim` access token (e.g. through XSS / session hijack).
3. Attacker POSTs `POST /api/v1/users/me/mfa/verify` with random 6-digit codes at 30 req/s.
4. Observe: every wrong attempt returns 401 `invalid_code`, no throttling, no audit trail of failed attempts; ~1.7 % cumulative chance of guessing during a single 30s window.
5. Expected: after `MAX_TOTP_ATTEMPTS` (≤ 5) failures within a sliding window, the endpoint returns 429 `totp_locked` and stays locked for the configured cooldown.

## Suggested approach
1. Introduce a per-user TOTP attempt counter mirroring `recovery_attempt_allowed` — either reuse the same in-memory store with a `kind: Totp | Recovery` discriminator, or add a sibling `totp_attempt_allowed()`. Prefer the former so a single attacker can't grind both surfaces in parallel.
2. Wire the gate into `verify_mfa_setup` (mfa.rs:288) — call before `totp.check_current` and on failure increment the counter + audit-log the failed attempt.
3. Wire the same gate into `regenerate_backup_codes` (mfa.rs:942) — same pattern.
4. Audit-action: add `Mfa_TotpVerifyLocked` (or reuse the dedicated action style from #1602's `MfaRecoveryLocked`). One action per locked path.
5. Update `backend/servers/api-server/src/routes/mfa.rs` module-level doc to spell out the rule: "any handler that calls `totp.check_current()` consults `totp_attempt_allowed()` first."
6. Centralise the limiter helpers in a `mfa::limiter` submodule if not already — see sibling plan `security-disable-mfa-recovery-no-ratelimit`.

## Alternatives considered
- **Skip the limiter for `verify_mfa_setup` because it's during enrollment** — rejected: enrollment IS the point of attack (the attacker uses a stolen token to *complete* enrollment to lock the legitimate user out, or to bypass setup-MFA-required policies). Enrollment is not a trusted phase.
- **Only rate-limit at the IP layer (reverse proxy)** — rejected: low-bandwidth grinding can stay below IP-rate caps; per-user is the only correct granularity for an account-takeover threat.

## Root-cause trace
1. Symptom: TOTP verification endpoints accept unbounded attempts per user; recovery-code endpoint does not (post-#1580).
2. ← `verify_mfa_setup` (`mfa.rs:288`) calls `totp.check_current()` with no preceding guard. Same shape at `regenerate_backup_codes` (`mfa.rs:942`).
3. ← PR #1580 scoped its fix to `verify_recovery_code` only, in response to issue #1523 which was scoped to the recovery-code endpoint. The other two TOTP paths were not in scope.
4. Origin: the TOTP-verify handlers (added with the original MFA epic) never had a per-user limiter — pre-dates the limiter helper. They were not flagged by routine `code-review` until churn-aligned segment review focused on `mfa.rs` (this run).

## Test plan
- [ ] Integration test in `mfa_recovery_cross_user_idor_tests.rs` (or new `mfa_totp_throttle_tests.rs`) that POSTs `MAX_TOTP_ATTEMPTS + 1` wrong codes to `verify_mfa_setup` and asserts the (N+1)th returns 429 / `totp_locked`
- [ ] Same against `regenerate_backup_codes`
- [ ] Test that the shared counter is shared: exhausting attempts on `verify_recovery_code` ALSO blocks subsequent TOTP attempts within the cooldown
- [ ] Run: `cargo test -p api-server --test mfa_recovery_cross_user_idor_tests`

## Out of scope
- Hardware-token / WebAuthn path — separate handlers, not in this scope.
- The `disable_mfa` recovery-code throttle — covered by sibling plan `security-disable-mfa-recovery-no-ratelimit`.
- Persisting attempt counters to Postgres (current in-memory limiter is sufficient for the threat model; persistence is a separate refactor).

## After-merge
- Move this file to `plans/_archive/security-mfa-totp-verify-no-throttle.md`
- Mark the matching `backlog.json` row `code-review-api-handlers-totp-verify-no-throttle` as `status: "done"`
