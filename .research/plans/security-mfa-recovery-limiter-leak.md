# security-mfa-recovery-limiter-leak

**Vector:** security
**Score:** 3
**Source:** Issue #1583
**Confidence:** high

## Hypothesis
The in-process MFA recovery-verify rate limiter (`MFA_RECOVERY_RATE_LIMITER` at `backend/servers/api-server/src/routes/mfa.rs:39`) keyed on `user_id` grows without bound — only the success path (`recovery_attempts_reset`) ever removes a key. On a public auth surface that's a slow memory-exhaustion DoS: an attacker enumerating user UUIDs leaves a permanent entry per attempt. The second smell on the same path is observability: denial events are written to the audit log as `AuditAction::MfaBackupCodeUsed` (success semantics), polluting any alert that counts that action as a real MFA bypass. The smallest correct change is to prune expired entries opportunistically inside `recovery_attempt_allowed` and emit a dedicated `MfaRecoveryRateLimited` audit variant — mirroring the `RefreshTokenReplayDetected` precedent in `audit_log.rs`.

## Evidence
- Issue #1583 (post-merge review of PR #1580) names the limiter's exact static + method symbols and the audit-action mislabel.
- `backend/servers/api-server/src/routes/mfa.rs:39` declares `static MFA_RECOVERY_RATE_LIMITER: LazyLock<Mutex<HashMap<Uuid, RecoveryRateLimitEntry>>>` with no eviction outside the success branch.
- `recovery_attempt_allowed` (line 45) inserts/updates per user but never `retain`-prunes; only `recovery_attempts_reset` (line 65) removes a key.
- PR #1580 merged 2026-06-18T11:44:10Z, closing #1523 (rate-limit MFA recovery verify) — the security gap is closed, but the implementation has the shape issues above.
- `backend/servers/api-server/tests/mfa_recovery_cross_user_idor_tests.rs::test_recovery_verify_is_rate_limited_per_user` exists (added by #1580) and is the natural home for the test-ordering hardening below.

## Files
- `backend/servers/api-server/src/routes/mfa.rs`
- `backend/crates/db/src/models/audit_log.rs`
- `backend/servers/api-server/tests/mfa_recovery_cross_user_idor_tests.rs`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [x] C7 — Code-review reception (tick if you expect controversy)

**Execution mode (auto-derived from the ticks):**
Mode: cloud-ok

## Repro steps
1. Hit `POST /api/v1/auth/mfa/recovery-verify` with N distinct user_id values (each request denied — wrong code is fine, the limiter records on the pre-hash branch).
2. Inspect `MFA_RECOVERY_RATE_LIMITER.lock().len()` — grows by N, never shrinks even after `MFA_RECOVERY_WINDOW` (900 s) has passed for every entry.
3. Trigger one denial after the burst — `audit_log` row is written with `action = MfaBackupCodeUsed` (success enum variant) and `resource_type = "mfa_recovery_rate_limited"`. Querying success-rate dashboards by `action` over-counts the throttled denials as successful uses.

## Suggested approach
1. In `backend/servers/api-server/src/routes/mfa.rs:45`, inside `recovery_attempt_allowed`, opportunistically sweep stale entries before the `entry.or_insert(...)` step:
   ```rust
   // cheap eviction: only run when the map is large enough to matter
   if map.len() >= 64 {
       let window = MFA_RECOVERY_WINDOW;
       map.retain(|_, e| now.duration_since(e.window_start) < window);
   }
   ```
   The size threshold keeps the cost bounded on the hot path; the swept window matches the rolling window already defined at line 32.
2. In `backend/crates/db/src/models/audit_log.rs`, add a new variant `MfaRecoveryRateLimited` with the matching Postgres enum value (follow the existing `RefreshTokenReplayDetected` migration shape — search for that variant for the exact pattern + migration filename convention used in the repo).
3. In `backend/servers/api-server/src/routes/mfa.rs`, on the denial branch (around the `recovery_attempt_allowed(user_id) == false` check at line 1072), emit `AuditAction::MfaRecoveryRateLimited` instead of `AuditAction::MfaBackupCodeUsed`; keep `details.outcome` as-is for fine-grained context but drop the success-variant overload.
4. In `backend/servers/api-server/tests/mfa_recovery_cross_user_idor_tests.rs::test_recovery_verify_is_rate_limited_per_user`, call `recovery_attempts_reset(user_id)` (already in-crate-public) at the top of the test so the limiter starts clean and the test is order-/repeat-independent.
5. Add a new failing-on-main test in the same file: `test_recovery_limiter_evicts_expired_entries` — insert N>64 entries with synthetic past `Instant::now()` (or fast-forward via a feature-gated test hook), trigger one new attempt, assert the map size has dropped to ≤ N/2. (If `Instant` mocking is too invasive, gate the sweep on a `pub(crate)` test helper that exposes the map's len.)
6. Update the audit-log migration in `backend/migrations/` adding the new enum value; run `cargo sqlx prepare --workspace`.
7. Run `cargo test -p api-server --test mfa_recovery_cross_user_idor_tests` and `cargo clippy -p api-server --tests -- -D warnings`.

## Alternatives considered
- **Replace the in-process limiter with a Redis `INCR` + `EXPIRE` key per user (sessions Redis is already in the stack).** Rejected for this plan — that's the correct long-term fix the original PR author flagged, but it's a separate larger change (Redis client wiring, key namespacing, fallback when Redis is down, instance-coordinated tests). Tracking-issue-worthy follow-up; in-process eviction here unblocks the unbounded-growth DoS today.
- **Add a periodic background sweep task (tokio::spawn'd timer) instead of opportunistic eviction inside the request.** Rejected because it introduces a new long-lived task to lifecycle around shutdown, doubles the surface for the hot path to lock-contend with, and gains nothing over a size-gated `retain` that runs only when the map has grown.

## Root-cause trace
1. Symptom: `MFA_RECOVERY_RATE_LIMITER`'s `HashMap` size strictly grows for the process lifetime, even after every recorded entry's `MFA_RECOVERY_WINDOW` has elapsed.
2. ← `recovery_attempt_allowed` at `backend/servers/api-server/src/routes/mfa.rs:45-63` only re-arms the per-entry window in place; it never `remove`s a key after the window expires.
3. ← `recovery_attempts_reset` at line 65 is the *only* place a key is removed, and it's called only on the success path (line 1302). Denial branch (line 1072) records the denial and returns without touching the map.
4. Origin: PR #1580 (commit landed via `feat-...-mfa-recovery-rate-limit` track for issue #1523), modeling on `caddy_ask.rs`'s loopback-only limiter where unbounded growth was bounded by the small set of trusted IPs. The same shape is not safe on a user-keyed public auth surface.

## Test plan
- [ ] Reset-on-setup: `test_recovery_verify_is_rate_limited_per_user` calls `recovery_attempts_reset(user_id)` at the top so repeat runs in the same process are deterministic.
- [ ] New: `test_recovery_limiter_evicts_expired_entries` — insert > size-threshold expired entries, trigger one attempt, assert the map shrinks. Must fail on `main`.
- [ ] New: `test_recovery_denial_uses_rate_limited_audit_action` — trigger a denial, query the audit_log row, assert `action = MfaRecoveryRateLimited` (not `MfaBackupCodeUsed`). Must fail on `main`.
- [ ] Command: `cargo test -p api-server --test mfa_recovery_cross_user_idor_tests` and `cargo sqlx prepare --workspace` for the new enum variant.

## Out of scope
- Migrating the limiter to Redis-backed counters (separate plan; flagged in *Alternatives*).
- Reworking the `caddy_ask.rs` limiter or other in-process limiters on the same shape — sweep them in a follow-up audit, this plan stays surgical to the MFA-recovery path.
- Reorganising audit-action variants beyond the one new `MfaRecoveryRateLimited` value (no broader enum cleanup).

## After-merge
- Move this file to `plans/_archive/security-mfa-recovery-limiter-leak.md`
- Mark the matching `backlog.json` row as `status: "done"`
