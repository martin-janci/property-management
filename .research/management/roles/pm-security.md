# pm-security

<sub>Last run: 2026-06-27</sub>

## Summary

Three open security-classified issues from the test-hardening batch (#480, #481, #487) remain unresolved and gate multiple OAuth and notification stories that are in-progress this sprint. The WebSocket JWT-in-query-param surface is confirmed in both frontend (websocket.ts:94) and backend (ws_notifications.rs:83), and refresh-token revocation logic at auth.rs:1126 looks sound, but the DB query used by find_by_token_hash_any_status cannot be verified without the session repository source — leaving issue #481 status ambiguous.

## Next actions

- **[high]** Resolve issue #480: move WebSocket auth off the URL query parameter — either (a) exchange a short-lived WS-specific one-time token via a REST endpoint before upgrade, or (b) ensure the reverse proxy strips the token query param from access logs and confirm the server TraceLayer does not emit the full URI at info/debug level (dep: rust-backend; DoD: JWT no longer appears in server access logs for WS upgrade requests; issue #480 closed)
- **[high]** Verify and close issue #481: read backend/crates/db/src/repositories/session.rs find_by_token_hash_any_status to confirm it does NOT filter revoked_at IS NULL (i.e., it does return revoked rows so replay detection at auth.rs:1126 fires correctly) (dep: none; DoD: Session repo query confirmed correct or bug patched; issue #481 closed; 10a-1 and 10a-3 story gates cleared)
- **[high]** Review and merge IDOR draft PR #1857 (security-llm-doc-idor): LLM document endpoints must enforce tenant-scoped RLS just like all other document paths; block story 7a-3 (permission-based-access) on this merge (dep: rust-backend; DoD: PR #1857 merged to dev; LLM doc routes confirmed RLS-gated; cross-tenant test added)
- **[high]** Fix issue #487: add MFA rate-limit integration test covering brute-force lockout (≥N wrong codes within window → 429) before 10a-1 OAuth authorization server can ship to prod (dep: rust-backend; DoD: Rate-limit test passes in CI; issue #487 closed; 10a-1 story gate cleared)
- **[medium]** Audit guest ID-document OCR pipeline (Epic 18, story 18.2, route ai/ocr.rs) for PII leakage: confirm OCR result fields (national_id, passport numbers) are not logged at any tracing level, are not stored beyond the session, and that the multipart upload is size/MIME validated before reaching the AI backend (dep: none; DoD: PII hardening draft PR #1823 reviewed and merged, or explicit deferral decision recorded)
- **[medium]** Confirm message-attachment presigned-upload IDOR posture: validate that link_message_attachment (messaging.rs) re-checks thread participation and that the file_key received from the client is opaque/cannot be used to overwrite another tenant's S3 objects (dep: none; DoD: Code review confirms file_key is server-generated UUID path, not caller-supplied; or fix shipped)

## Risks

- **high/high**: WebSocket JWT exposed in HTTP access logs: buildWebSocketUrl appends ?token=<jwt> (websocket.ts:94); if the reverse proxy or the backend TraceLayer logs the full request URI, every WS connection leaks a 15-min bearer token into logs — a stored-credential exposure — mitigation: Short-lived one-time WS ticket endpoint (REST pre-auth) or proxy log scrubbing; issue #480 open and unresolved
- **medium/high**: Refresh-token revocation gap (issue #481): if find_by_token_hash_any_status filters revoked_at IS NULL, the replay detection branch at auth.rs:1126 never fires for a revoked token, allowing an attacker who stole a token before revocation to continue using it — mitigation: Read session.rs to confirm the query fetches any-status rows; fix if not; issue #481 is a sprint gate for 10a-1 and 10a-3
- **medium/high**: LLM document IDOR (draft PR #1857 unmerged): if LLM-document endpoints are not tenant-RLS-gated, a manager in tenant A can read or overwrite AI-processed documents belonging to tenant B — mitigation: Merge PR #1857 before any LLM-document feature ships to prod; treat as release blocker
- **medium/high**: Guest ID-document OCR (story 18.2, PR #1750 merged) processes government-issued identity documents; PII hardening draft #1823 is still a draft — if OCR results or raw images are logged or persisted without retention controls, a breach exposes passports/IDs at scale — mitigation: Merge PII hardening PR #1823 before Epic 18 prod rollout; ensure OCR route does not log multipart body or extracted fields
- **low/high**: Stripe webhook replay: verify_signature in stripe.rs enforces a 300-second timestamp tolerance window; if the STRIPE_WEBHOOK_SECRET env var is missing or rotated without coordinated deploy, the fallback behaviour could accept unsigned events and trigger fraudulent payment settlements — mitigation: Add startup assertion that STRIPE_WEBHOOK_SECRET is set (similar to JWT_SECRET pattern); alert on signature-reject rate spike

## Open questions

- Does find_by_token_hash_any_status in backend/crates/db/src/repositories/session.rs omit a revoked_at IS NULL filter? The handler logic at auth.rs:1126 depends on receiving revoked rows to trigger replay detection — if the query filters them out, issue #481 is a live auth bypass.
- Does the reverse proxy (or Tower TraceLayer) log the full request URI including the ?token= query parameter for WebSocket upgrades? If yes, issue #480 is a prod data-at-rest exposure today.
- What is the content-type allowlist enforced by generate_upload_url for message attachments? The storage.rs comment says 'content-type validated by the storage layer' but the allowed MIME set is not visible in the reviewed slice — is there a MIME allowlist blocking upload of HTML/SVG for content-sniffing attacks?
- Is the guest OCR multipart upload (ai/ocr.rs) guarded by the same manager-auth middleware as other sensitive routes, or is it accidentally public while the OCR backend is stubbed (501)?
- PR #1857 (IDOR for LLM docs) is listed as a draft — who is the assigned reviewer and what is the target merge date?

## Decisions needed

- WebSocket auth mechanism: adopt short-lived one-time ticket endpoint vs. accept token-in-query-param with log-scrubbing mitigation — owner: rust-backend + pm-tech-lead
- PII retention policy for guest ID-document OCR results: how long are raw images and extracted fields stored, and under which GDPR lawful basis — owner: pm-compliance + pm-tech-lead
- Release gate policy: confirm that issues #480, #481, #487 must be closed before any Epic 10A story ships to prod (current sprint-status marks them as open gates but no prod-freeze date is set) — owner: pm-delivery
