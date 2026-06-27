# pm-security — 2026-06-27

_Run by pm-rotation index 5 → 6._

## Summary

Sprint has active auth/authz debt across four open THB issues (#480, #481, #482, #487) that gate Epic 10A OAuth stories; messaging attachment IDOR (#1791) has test coverage landed but the GitHub issue remains flagged, while SSO CSRF-skip (#1826) is documented as intentional PKCE-based protection in sso.rs:42-49 but requires independent verification that state param is always validated on callback.

## Next actions

- **[high]** Verify issue #481 (OAuth refresh-token revocation) is fully fixed: confirm OAuthRepository::find_refresh_token_by_hash includes AND revoked_at IS NULL on the production grant path (not the family-reuse any-status variant)
  - owner: rust-backend
  - DoD: Issue #481 closed with confirmed query using revoked_at IS NULL; stories 10a-1 and 10a-3 unblocked
- **[high]** Audit issue #480: WebSocket auth token in query param — confirm converged DB-checked handler (PR #1737) never writes token value to tracing/access logs; confirm session re-validation on JWT expiry
  - owner: rust-backend
  - DoD: Issue #480 closed; no JWT token value in structured logs; expiry re-check confirmed
- **[high]** Independently verify #1826 (reality-web SSO callback CSRF): confirm sso_callback at backend/servers/reality-server/src/routes/sso.rs enforces state→session-id lookup on every callback and that in-memory sso_sessions has TTL/expiry
  - owner: rust-backend
  - DoD: State param validated server-side on every callback; stale sessions pruned; #1826 closed or accepted-risk documented
- **[high]** Close or scope issue #1791 (message attachment IDOR): messaging_attachments_authz_tests.rs covers participant isolation — confirm the route handler enforces the same checks in production, not just tests
  - owner: rust-backend
  - DoD: Issue #1791 closed or residual scope documented; no unguarded attachment download path
- **[medium]** Fix issue #482 (ProtectedRoute role fallback uses tenants[0] for multi-tenant users): wrong tenant silently grants/denies based on array order; add unit tests for multi-tenant fixture
  - owner: react-web
  - DoD: Issue #482 closed; ProtectedRoute selects role from active tenant context, not array position; tests present
- **[medium]** Validate search_alert_drainer.rs PII handling: LogEmailTransport logs to_email at INFO; confirm production log filters strip it before SMTP wire-up
  - owner: rust-backend
  - DoD: Email address not in info-level logs in production config; or field redacted/hashed

## Risks

- **medium/high** — Issue #481 open: if OAuthRepository production lookup omits revoked_at IS NULL, revoked refresh tokens are reusable — direct RFC 9700 violation enabling token reuse after logout/rotation
  - mitigation: Sprint gate blocks 10a-1/10a-3 from 'done'; must close before Epic 10A ships to prod
- **high/high** — Issue #480 open: JWT token value emitted in WebSocket query param may surface in access logs (credential-in-logs exposure)
  - mitigation: PR #1737 removed duplicate handler; confirm surviving handler strips token from log context before closing #480
- **low/high** — Issue #1826 (SSO callback CSRF): sso.rs comment asserts PKCE session_id provides CSRF protection but in-memory sso_sessions has no documented TTL — stale entries could be exploited
  - mitigation: session_id IS UUID v4 (random); add TTL eviction of pending sessions older than ~10 minutes
- **medium/medium** — LogEmailTransport logs recipient email at INFO; in prod = PII in structured logs; duplicate-delivery bug could compound by sending to wrong user_id row
  - mitigation: Replace to_email field with hashed/truncated value; audit RealityPortalRepository drain query for per-row user_id scoping
- **medium/high** — Issue #487 open: MFA brute-force/rate-limit test coverage missing — no regression guard against MFA bypass via rapid guessing; gates 10a-1
  - mitigation: Add rate-limit integration tests for MFA endpoint before Epic 10A stories promote to done

## Open questions

- Does OAuthRepository::find_refresh_token_by_hash (production grant path) include AND revoked_at IS NULL, or was #481 fix only partial?
- Does sso_sessions in-memory map have TTL/cleanup, or do pending SSO sessions accumulate indefinitely?
- What is residual scope of #1791 given messaging_attachments_authz_tests.rs already covers main IDOR vectors — close or keep open?
- Is the deeplink-token-URL-decode backlog item scheduled this sprint or deferred, and does it affect OAuth redirects in Epic 10A?
- Does the org-scoped favorite alert worker (PR #1850) run service-role without RLS and fan out strictly by row-level user_id like search_alert_drainer?

## Decisions needed

- Accept or fix PII (email address) in INFO-level structured logs from LogEmailTransport before real SMTP transport enabled — owner: rust-backend + Tech Lead
- Determine whether #1826 SSO CSRF concern is fully mitigated by PKCE session_id approach or requires additional TTL eviction + documentation — owner: rust-backend + Security Lead
- Decide whether THB #480/#481/#482/#487 must all close before Epic 10A ships to prod, or if any can be formally deferred with a tracking issue — owner: Scrum Master + Tech Lead

