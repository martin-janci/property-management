# pm-security — 2026-06-26 (rotation, 30 days overdue since 2026-05-27)

## Summary

This sprint has a dense cluster of unmerged security-hardening draft PRs covering authentication gaps (unauthenticated OCR endpoints; JWT role vs DB manager gate on booking_channel; third unmigrated JWT verification copy), PII exposure (guest booking reads without manager gate; guest ID-doc upload without content-sniff or audit), and broken integrity guarantees (IDOR on message attachments; OAuth refresh-token revocation bypass still open as issue #481; event-bus at-least-once defeated by Lagged drops). None of the 8 high-priority hardening PRs (#1797, #1799, #1801, #1802, #1804, #1806, #1823, #1824) are merged; all remain drafts aged 2 days.

## Next actions

1. **[high]** Merge PR #1797 — add auth extractor to OCR endpoints (`process_meter_reading`, `submit_correction` in `backend/servers/api-server/src/routes/ai/ocr.rs`); currently unauthenticated, accepts anonymous multipart uploads. **DoD:** both handlers require valid AuthUser; CI green; issue #1772 closed. — dependency: rust-backend
2. **[high]** Resolve issue #481 — OAuth refresh-token revocation bypass: confirm `revoked_at IS NULL` predicate present after PR #1829 and add a `#[sqlx::test]` regression verifying revoked tokens return 401. **DoD:** test passes; issue closed; 10a-1 and 10a-3 gates cleared. — dependency: rust-backend
3. **[high]** Merge PR #1799 — bind message attachment `file_key` to the issuing thread prefix + MIME validate `link_message_attachment`, closing the IDOR where any user can attach another thread's or documents/ object to their own message. **DoD:** rejects unscoped file_key; MIME allowlist enforced; issues #1791 / #1770 closed. — dependency: rust-backend
4. **[high]** Merge PR #1823 — manager-gate `list_bookings` / `get_booking` / `get_booking_with_guests` / `get_guest`, content-sniff on guest ID-doc upload, emit PII audit log on access. **DoD:** non-manager gets 403; sniff rejects non-image; audit row written; issues #1766 / #1783 closed. — dependency: rust-backend
5. **[high]** Merge PR #1806 — replace JWT role claim check in `get_booking_conflicts` and `push_listing_to_booking` with DB-backed `MembershipRepository::is_manager_in_org` (same pattern as `faults.rs`). **DoD:** both handlers DB-checked; issue #1787 closed. — dependency: rust-backend
6. **[medium]** Fix issue #480 — WS bearer token in `?token=` URL leaks JWT into nginx/proxy access logs. Route via `Sec-WebSocket-Protocol` subprotocol bearer header or one-time ticket endpoint. **DoD:** WS upgrade contains no credential in URL; access log line shows no token; issue #480 closed. — dependency: rust-backend

## Risks

- **Unauthenticated OCR endpoints** — probability high, impact high. Abuse vector for storage spam + model poisoning once OCR backend wired. Mitigate by merging PR #1797 before any OCR backend connects.
- **OAuth refresh-token revocation bypass (#481)** — probability medium, impact high. Revoked tokens may remain usable until TTL expiry; breaks RFC 9700 and invalidates logout/session-revoke guarantees. Blocks 10a-1 + 10a-3. Mitigate via sqlx::test regression + #481 closure.
- **IDOR on message attachments** — probability high, impact high. Cross-tenant document exfiltration via messaging path. Mitigate by merging PR #1799 immediately.
- **Guest booking PII without manager gate** — probability high, impact high. GDPR art. 32 compliance at risk; no audit trail on guest ID docs. Mitigate by merging PR #1823.
- **JWT verification path divergence + booking_channel JWT-role authz** — probability medium, impact high. Demoted manager in org A operates as manager in org B until token expires. Mitigate via PR #1806 + JwtService canonical migration + cross-tenant tests.

## Open questions

- Did PR #1829 restore the `revoked_at IS NULL` predicate on refresh-token lookup, or only fix the column-name bug? #481 remains open — needs explicit confirmation before 10a-1/10a-3 gate clears.
- What code path does `JwtService` (issue #1782) serve and is it reachable from any production route handler? No associated draft PR visible.
- PR #1826 follow-up: state-parameter validation in reality-web auth-callback — is the state cookie verified before token exchange, or is the CSRF-skip path cemented by PR #1822 tests?
- Issue #1758 (preflight secret length floor): minimum length for JWT_SECRET / ESIGN_TOKEN_SECRET — risk of short secrets in non-prod envs being promoted?
- Event-bus at-least-once fix (PR #1801, issue #1792): does the keep-alive-on-Lagged replay missed messages or resume from current head? Silent message loss on lag could mean missed payment/fault/notification events with no observability.

## Decisions needed

- Promote all 8 hardening draft PRs (#1797, #1799, #1801, #1802, #1804, #1806, #1823, #1824) to review and fast-track before any prod deployment, vs defer individual items post-launch with compensating controls — owner: pm-security + engineering-lead
- WebSocket auth strategy: keep `?token=` (leaks JWT to proxy logs) vs short-lived WS ticket endpoint vs `Sec-WebSocket-Protocol` bearer subprotocol — owner: rust-backend + pm-security
- `PlatformAdminRepository` sqlx macro migration (#1851): schedule as part of 10b-5 hardening or defer — owner: rust-backend
