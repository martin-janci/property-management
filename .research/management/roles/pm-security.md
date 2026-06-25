# pm-security — 2026-06-25

_Rotation idx 5 (last run 2026-05-27 — 29 days stale). Rendered from agent JSON._

## Summary

The sprint carries three release-blocking security gaps open in parallel: unauthenticated meter-OCR endpoints (`/api/v1/ai/ocr/*`) confirmed in source, a third `JwtService` token-verifier copy (issue #1782) not yet migrated to the unified path landed in #1744, and refresh-token revocation bypassed in the OAuth path (issue #481, `revoked_at IS NULL` omitted). The support-data `refresh_tokens` column bug (10b-5, #1829) is resolved; `refresh_tokens` schema correctly uses `revoked_at` throughout and `SessionRepository` queries are consistent.

## Next actions

1. **[high]** Add `AuthUser`/`RequestPrincipal` extractor to both handlers in `backend/servers/api-server/src/routes/ai/ocr.rs` — `process_meter_reading` and `submit_correction` are currently callable without any token. _DoD:_ both handlers require a valid access token; integration test confirms 401 for unauthenticated caller.
2. **[high]** Locate and migrate the third standalone JWT-verify path flagged in issue #1782 — `grep` the backend for `DecodingKey::from_secret` outside `api-core/src/extractors/auth.rs` and the canonical `JwtService`, then port to the unified verifier from #1744. _DoD:_ only two token-verification sites remain (`JwtService` for issuance, api-core extractor for request auth); third copy removed and CI green.
3. **[high]** Fix OAuth refresh-token revocation query (issue #481) — restore `revoked_at IS NULL` predicate in `backend/servers/api-server/src/services/oauth.rs` refresh path so revoked tokens cannot be replayed. _DoD:_ `oauth_refresh_token_tests.rs` has a regression test proving a revoked token returns 401; story gates 10a-1 and 10a-3 unblocked.
4. **[high]** Close test-hardening batch item #480 — WebSocket auth token in query param is written to access logs; mask or move to `Authorization` header in `backend/servers/api-server/src/routes/ws_notifications.rs`. _DoD:_ token no longer appears in tracing/access log output; WS session re-validated after JWT expiry.
5. **[high]** Audit PII logging for guest ID-document OCR path (issue #1783) in `backend/servers/api-server/src/routes/rentals.rs` `extract_guest_id_document` and `backend/servers/api-server/src/services/id_ocr.rs` — confirm no raw OCR text (names, DOB, document numbers) reaches `tracing::debug/info` spans. _DoD:_ code review confirms PII fields are not logged; audit-log entry written with event type only.
6. **[medium]** Resolve attachment IDOR (issue #1791) and document share `file_key` binding (PR #1799) before promoting 7a-2 folder-organization or 7a-5 document-sharing to done. _DoD:_ attachment lookup is org-scoped; `file_key` bound to requesting tenant at pre-signed URL generation; cross-tenant test proves 404 not 200.

## Risks

| # | Risk | P | I | Mitigation |
|---|---|---|---|---|
| 1 | Unauthenticated meter-OCR endpoints (`/api/v1/ai/ocr/meter-reading`, `/api/v1/ai/ocr/correction`) — confirmed in `routes/ai/ocr.rs`; handlers accept multipart/JSON with no auth extractor | high | medium | Add `RequestPrincipal` extractor before next staging deploy; currently 501 stub limits damage |
| 2 | OAuth refresh-token revocation bypass (issue #481) — `revoked_at IS NULL` guard removed; replayed revoked refresh tokens accepted, RFC 9700 violation | high | high | Restore predicate in `oauth.rs`; gate 10a-1/10a-3 on fix; add `sqlx::test` regression |
| 3 | Third `JwtService`/token-verifier copy (issue #1782) not migrated after #1744 unified the canonical path — divergent validation parameters allow forged or mistyped tokens | medium | high | Grep `DecodingKey::from_secret` outside `auth.rs` and `JwtService`; consolidate before any Epic 10A story ships |
| 4 | JWT access token exposed in WebSocket query param and written to access logs (issue #480, sev=high) — token harvestable from log aggregation | medium | high | Move to `Sec-WebSocket-Protocol` or first WS message; scrub query-param from `TraceLayer`; gate 8a-3 on closure |
| 5 | Guest ID-document PII (name, DOB, passport/ID) from OCR may surface in tracing spans (issue #1783) — no PII audit-logging confirmed for `extract_guest_id_document` | medium | high | Audit `tracing::debug` calls in `id_ocr.rs` and `rentals.rs`; redact PII; add append-only audit log per GDPR |

## Open questions

- Is the third `JwtService` copy referenced in issue #1782 inside api-server, reality-server, or a shared crate — and does it omit the `token_type == 'access'` check that RUST-002 required?
- Do open draft PRs #1797 (OCR-auth) and #1799 (attachment `file_key` binding) have approvals and estimated merge dates, or are they blocked on review capacity?
- Issue #1766 (rental booking PII reads) — is guest PII exposure via booking-channel APIs scoped to manager-only paths, or are resident-facing/unauthenticated endpoints involved?
- The `ProtectedRoute` role fallback (issue #482) uses `tenants[0]` for multi-tenant users — is there a known test user population with multiple tenant memberships in staging that would expose this before prod?
- `sprint-status.yaml` was last updated 2026-05-25; has Epic 10A moved beyond `ready-for-dev` since then, and if so what is the current auth-server implementation surface that should be in scope for this review?

## Decisions needed

- Decide whether unauthenticated `/api/v1/ai/ocr/*` endpoints should be disabled (return 404/403) at the router level until auth is wired, or if adding the extractor is sufficient — owner: pm-security + rust-backend.
- Decide if Epic 10A stories (10a-1, 10a-3) are hard-blocked from staging deploy until issue #481 (refresh-token revocation bypass) is fixed, given RFC 9700 compliance is a stated requirement — owner: pm-tech-lead.
