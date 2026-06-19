# security-disable-mfa-recovery-no-ratelimit

**Vector:** security
**Score:** 3
**Source:** Phase 1.5 rotating expert review (api-handlers segment), 2026-06-19
**Confidence:** high

## Hypothesis
`disable_mfa` accepts a recovery code as proof-of-identity to turn MFA off, but the path skips the `recovery_attempt_allowed()` throttle that PR #1580 added to `verify_recovery_code`. An attacker who has stolen a session token can brute-force the 10-character recovery code with unlimited online attempts and disable MFA on the victim's account. The fix is to gate the same throttle helper on the disable path so the lockout / per-user attempt counter applies symmetrically.

## Evidence
- `backend/servers/api-server/src/routes/mfa.rs:629-659` — `disable_mfa` recovery-code branch verifies the code (constant-time-eq) and proceeds straight to teardown, no `recovery_attempt_allowed(user_id)` call
- `backend/servers/api-server/src/routes/mfa.rs:1087` — `verify_recovery_code` calls `recovery_attempt_allowed(user_id)` first; throttling guard introduced by PR #1580 closes #1523
- `backend/servers/api-server/src/routes/mfa.rs:552` (handler entry) — same recovery_code → MFA-off effect, no throttle = bypasses #1523 hardening for the disable surface
- PR #1602 (merged 2026-06-19) extended the limiter to evict stale entries + dedicated audit action — `disable_mfa` was not touched

## Files
- `backend/servers/api-server/src/routes/mfa.rs:552`
- `backend/servers/api-server/src/routes/mfa.rs:629`
- `backend/servers/api-server/tests/mfa_recovery_cross_user_idor_tests.rs`

## Dependencies

(none — `disable_mfa` is independent of the verify-code limiter rollout that just landed)

## Required capabilities
- [x] C1 — Systematic debugging
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Mode: cloud-ok** (Rust unit/integration tests, no browser or device needed)

## Repro steps
1. Authenticated user `victim` has MFA enabled with a recovery-code set.
2. Attacker holds `victim`'s access token (e.g. via session hijack / XSS) and starts POSTing `DELETE /api/v1/users/me/mfa` with random 10-char recovery codes.
3. Observe: with the bug, every request returns 401 `invalid_code` indefinitely — no 429 / lockout — so the attacker can grind through the keyspace.
4. Expected: after `MAX_RECOVERY_ATTEMPTS` failures within the window (same as `verify_recovery_code`), the path returns 429 / `recovery_locked` and stays locked until reset.

## Suggested approach
1. In `disable_mfa` (entry around `mfa.rs:552`), before reaching the recovery-code verify branch, call `recovery_attempt_allowed(&state, user_id).await?` — return the same 429 / `recovery_locked` error variant the verify endpoint returns.
2. On wrong-code branch in `disable_mfa` (~`mfa.rs:629-659`), call the failure-counter increment helper used by `verify_recovery_code` (so attempts on the disable path also feed the lockout).
3. On successful disable, call the success-reset helper (mirror `verify_recovery_code`).
4. Add an audit-action variant for `disable_mfa_recovery_locked` distinct from the verify-path action (per the #1602 pattern — separate audit action keeps tracing clean).
5. If the limiter helpers are private to the `routes/mfa.rs` module, hoist them to a `mfa::limiter` submodule or pub(crate) — do NOT duplicate logic.
6. Document in the file-level comment that any recovery-code-gated surface (currently `verify_recovery_code` + `disable_mfa`) must consult the limiter — defensive against the next path that grows over recovery codes.

## Alternatives considered
- **Make `disable_mfa` not accept recovery codes (require password + TOTP)** — rejected: breaks the documented user-flow ("locked out without TOTP → recovery code disables MFA"), defeats the whole purpose of recovery codes.
- **Rely on the access-token TTL to bound the attack window** — rejected: recovery codes are 10 chars (~`62^10` keyspace but practical entropy ~`36^10`); against an attacker with a fresh token, even 1h is enough for >100k attempts at modest QPS unless gated.

## Root-cause trace
1. Symptom: unbounded recovery-code attempts on `DELETE /api/v1/users/me/mfa` (disable path) succeed in completing brute-force where `POST .../recovery-codes/verify` blocks at the limiter.
2. ← `disable_mfa` (`mfa.rs:552`) reaches the recovery-code verify block (`mfa.rs:629-659`) without consulting the limiter.
3. ← The limiter helper `recovery_attempt_allowed()` was introduced by PR #1580 against `verify_recovery_code` only — `disable_mfa` was not in the diff because the issue (#1523) was scoped to the verify endpoint.
4. Origin: PR #1580 (merged 2026-06-18) fixed half the surface; the disable_mfa path predates #1580 and was overlooked.

## Test plan
- [ ] Integration test in `backend/servers/api-server/tests/mfa_recovery_cross_user_idor_tests.rs` (or a sibling) that calls `disable_mfa` with wrong recovery codes `MAX_RECOVERY_ATTEMPTS + 1` times and asserts the (N+1)th returns 429 / `recovery_locked`
- [ ] Test that a correct recovery code on `disable_mfa` resets the counter (mirrors `verify_recovery_code` success path)
- [ ] Test that exhausting attempts on `verify_recovery_code` ALSO blocks `disable_mfa` (shared counter, not per-endpoint)
- [ ] Run: `cargo test -p api-server --test mfa_recovery_cross_user_idor_tests`

## Out of scope
- Refactoring the limiter to a generic per-action rate-limit framework — keep the change minimal: hoist the existing helper and call it.
- Extending the limiter to TOTP-verify paths (`verify_mfa_setup`, `regenerate_backup_codes`) — covered by the sibling plan `security-mfa-totp-verify-no-throttle`.
- Audit-log retention or DB schema changes for the new audit-action variant.

## After-merge
- Move this file to `plans/_archive/security-disable-mfa-recovery-no-ratelimit.md`
- Mark the matching `backlog.json` row `code-review-api-handlers-disable-mfa-no-ratelimit` as `status: "done"`
