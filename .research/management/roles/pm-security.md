# pm-security — 2026-06-24

## Summary

Eight open test-hardening issues (#480–#487) gate four in-progress stories; two are security-high: **#480** (WebSocket JWT-in-query-param logging) and **#481** (OAuth refresh-token revocation bypass — RFC 9700). Seven draft PRs hardening PII / payment / IDOR / channel-validation remain unmerged from the last 8 days. A residual cross-tenant IDOR on `ai.rs list_listing_descriptions` (backlog item `security-llm-doc-idor`, status: ready) still has no merged fix.

## Next actions

1. **HIGH** — Merge or close PR #1797 (OCR endpoints unauthenticated + manager-gate on persistent rental/guest PII reads) before any rental-flow story ships to prod. *dep: rust-backend*.
2. **HIGH** — Resolve issue #481 (OAuth refresh-token revocation bypass): fix revocation query (`revoked_at IS NULL` restored in token repo) and add RFC 9700 regression test.
3. **HIGH** — Resolve issue #480 (WebSocket auth token in query-param logged + session not re-validated after JWT expiry): strip token from access-log path and add expiry re-check.
4. **HIGH** — Merge PR #1799 (msg attachment IDOR — bind file_key to thread + MIME validation) and PR #1823 (rental guest ID-doc PII audit + content-sniff + manager-gate parity).
5. **MEDIUM** — Fix residual cross-tenant read IDOR: `ai.rs list_listing_descriptions` (routes/ai.rs:2666-2685) still discards `_principal`. See backlog `security-llm-doc-idor`.
6. **MEDIUM** — Resolve issue #1758 (preflight presence-check misses length floors for JWT_SECRET / ESIGN_TOKEN_SECRET, from PR #1753) and #1782 (third `JwtService` token-verification copy left unmigrated by PR #1744; lost `token_type` field in logs).

## Risks (added today)

- **HIGH × HIGH** — OAuth refresh-token revocation bypass (#481). Block stories 10A-1 / 10A-3 from shipping until #481 closes.
- **MED × HIGH** — Seven security-flavored draft PRs in flight simultaneously — merge sequencing risk; partial fixes may ship. Prioritize #1797 + #1799 first.
- **MED × HIGH** — WS JWT in access log (#480) — bearer tokens persisted in plaintext. Confirm log pipeline doesn't persist query strings to external sinks meanwhile.
- **MED × MED** — `ProtectedRoute` role fallback uses `tenants[0]` for multi-tenant users (#482) — wrong perms on React SPA. Server-side authz is the real gate but client UI is misleading.
- **MED × MED** — Announcements/Faults wiring uses direct `getToken()` bypassing axios interceptor (#486) — silent token-refresh failures produce unauthenticated requests with no error surfaced.

## Open questions

- Has PR #1797 been reviewed by a backend security engineer, or is it still author-only?
- Issue #1782 references a third `JwtService` verification copy that wasn't migrated by PR #1744 — which file/module, and is it reachable on authenticated paths in production?
- Issues #1786 / #1763 ask for authz regression tests on the sensor WS DB-checked path after PR #1737 — have these tests been authored yet?
- `security-llm-doc-idor` (status: ready) has a plan but no linked open PR — assigned owner?
- PR #1806 switches `booking_channel` manager-gate from JWT role claim to DB-backed check — applied to all booking_channel read paths or only write?

## Decisions needed

- Declare PR #1797 a release blocker for any rental Epic story promotion.
- Confirm whether #481 blocks Sprint 10A OAuth stories from staging, or whether a feature flag can gate OAuth externally until the fix lands.
- Agree on a minimum-entropy floor for JWT_SECRET / ESIGN_TOKEN_SECRET in the preflight check (#1758).
