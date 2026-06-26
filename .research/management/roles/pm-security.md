# pm-security — 2026-06-26

**Run context:** Rotation idx 5; last run 2026-05-27 (30d stale). Deep rotation slot.

## Summary

The most critical carried risks (#480 WS token in URL, #481 OAuth revocation bypass) show partial or full mitigation in code but remain formally open in sprint-status.yaml and must be formally closed before Epic 10A stories can be promoted to done. The new accounting-server surface (PAP-312, PRD #1817 + web #1808) has zero code on disk yet — security requirements must be defined before the first PR lands.

## Top findings

### 1. [HIGH] Issue #481 (OAuth refresh-token revocation bypass) appears resolved in code but still open

**Evidence:**
- `backend/crates/db/src/repositories/oauth.rs:413-414`: `WHERE token_hash = $1 AND revoked_at IS NULL`
- `backend/servers/api-server/src/services/oauth.rs:511`: second guard `if refresh_token.is_revoked() { ... }`

**Recommendation:** Have rust-backend formally verify the revocation path with the revocation-endpoint test added in #1393, then close #481 in sprint-status.yaml to unblock 10A story gating.

**Owner:** rust-backend

### 2. [HIGH] Issue #480 (JWT in WS query param) — application-level mitigation in place, infra-layer exposure remains

**Evidence:**
- `backend/servers/api-server/src/routes/ws_notifications.rs:122` explicitly never logs `params.token`
- But token travels in `?token=<jwt>` on the upgrade URL → nginx/CDN access logs record it server-side

**Recommendation:** Implement short-lived WS ticket endpoint (POST /api/v1/ws/ticket → opaque 30s single-use token); WS upgrade uses ticket. This is the standard WS bearer-token mitigation.

**Owner:** rust-backend

### 3. [MEDIUM] `_principal: RequestPrincipal` discard pattern in ai/workflows.rs + automation.rs

**Evidence:** 5 handlers — `ai/workflows.rs:490,558,738` + `automation.rs:425,455` — assert auth gate then discard principal without role/tenant check. Currently serve global template data so IDOR is not active, but the pattern matches the prior ai.rs cross-tenant IDOR cluster.

**Recommendation:** Add SECURITY comment to each handler documenting global-read intent + listing data classes that MUST NOT be added without tenant scoping. File follow-up to add any-valid-tenant-member check.

**Owner:** rust-backend

### 4. [MEDIUM] accounting-server has no threat model / tenant isolation design

**Evidence:**
- `backend/servers/accounting-server/`: NOT FOUND (PRD #1817 merged, no code yet)
- `frontend/apps/accounting-web/src/components/signup/SignupForm.tsx:41`: `TODO(PAP-303 #2/#3): replace with accountingApiClient.signup(...) mutation`

**Recommendation:** Produce security ADR before first backend PR: tenant isolation (shared api-server RLS vs separate DB), auth handoff (OAuth resource-server vs separate domain), PII/financial classification. Require pm-security review on auth scaffold PR.

**Owner:** pm-security

## Risks (new this rotation)

1. **pm-security-ws-jwt-in-url-infra-logs** (prob: high, impact: high) — see finding #2
2. **pm-security-accounting-server-no-threat-model** (prob: medium, impact: high) — see finding #4
3. **pm-security-ai-workflows-principal-discard-future-risk** (prob: medium, impact: high) — see finding #3

## Risks resolved this rotation

- **pm-qa-booking-oauth-no-secure-replacement** → resolved by #1393 + cross-org IDOR cluster fixes #1467/#1601/#1635/#1639/#1741

## Next actions (added to action-list.json)

- `sec-481-formally-close-oauth-revocation` [high] — close #481 in sprint-status (unblocks 10a-1/10a-3)
- `sec-480-ws-ticket-endpoint` [high] — implement POST /api/v1/ws/ticket
- `sec-accounting-server-threat-model` [high] — produce security ADR
- `sec-487-mfa-rate-limit-tests` [medium] — close #487 (clears 10a-1 gate)
- `sec-483-voice-device-idor-test` [medium] — close #483
- `sec-nginx-ws-token-log-redact` [medium] — interim ?token= redaction (pm-devops)
- `sec-ai-principal-comment` [low] — SECURITY comment on _principal discard handlers

## Decisions needed

- accounting-server tenant isolation / auth boundary — DEC-PEND-2026-06-26-A
- WS JWT-in-URL permanent design vs WS ticket — DEC-PEND-2026-06-26-B
- #481 formal close: this sprint (code fix verified) or pending additional test coverage — rust-backend

## Open questions

1. Is #481 actually fixed by `revoked_at IS NULL`, or does a separate query path still use the old `is_revoked` boolean column? support_data_session_columns_tests.rs comments suggest prior naming confusion.
2. accounting-server: shared api-server DB+RLS or separate?
3. Are nginx/CDN logs currently redacting `?token=` for /ws upgrade?
4. Issue #482 (ProtectedRoute tenants[0] fallback): work started? Blocks 10a-2.
5. #1393 Booking.com OAuth/CSRF: covers state-param CSRF on callback, or only token-exchange?
