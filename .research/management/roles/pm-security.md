# pm-security

_Last run: 2026-06-26_

## Summary

Sprint window (2026-06-16 to 2026-06-26) delivered substantial auth hardening: 18 security PRs closed covering portal IDOR tests, unified auth verification path, Airbnb manager-role gate, duplicate WS JWT consumer removed, unit-content revocation, centralized principal extractor, financial/payment auth, and cross-tenant OAuth upsert fix. However, 10 open follow-up issues remain, with the message-attachment file_key IDOR (#1791), unvalidated JWT_SECRET length in preflight (#1758), persisted booking guest PII role gap (#1766), and unmigrated third JwtService verification copy (#1782) representing the highest pre-prod exposure.

## Next actions

- **[high] Validate client-supplied file_key in link_message_attachment against the server-issued prefix (messages/{thread_id}/) before persisting to DB -- any arbitrary S3 key is currently accepted from the client**
  - dependency: rust-backend
  - dod: link_message_attachment rejects file_key values that do not match the messages/{thread_id}/ prefix; integration test covers cross-thread key injection attempt returning 400
- **[high] Add minimum-length floor (>=32 bytes) for JWT_SECRET and ESIGN_TOKEN_SECRET in preflight.rs check_required_env -- current guard only checks non-empty, allowing a trivially-guessable 1-char secret in prod (#1758)**
  - dependency: rust-backend
  - dod: REQUIRED_PROD_ENV_VARS extended with length thresholds; preflight test asserts short JWT_SECRET fails in non-dev mode
- **[high] Audit and migrate the third JWT access-token verification copy (JwtService in services/jwt.rs vs two other decode paths) to the unified extractor -- restore lost log line (#1782)**
  - dependency: rust-backend
  - dod: Single JWT verification call site confirmed; tracing log on token failure present; no inline jsonwebtoken::decode calls outside the canonical extractor
- **[high] Add manager-only role gate to list_bookings and get_booking in rentals.rs -- endpoints currently scope only by tenant (TenantExtractor) and expose guest PII (name, email, phone) to any org member (#1766)**
  - dependency: rust-backend
  - dod: Both handlers require property_manager or technical_manager role; 403 returned for non-manager authenticated members; regression test covering resident attempting access
- **[high] Fix OAuth refresh-token revocation bypass (#481): restore the revoked_at IS NULL predicate in the token lookup query so revoked tokens cannot be reused across the RFC 9700 rotation boundary**
  - dependency: rust-backend
  - dod: Query includes revoked_at IS NULL; #sqlx::test reproduces revoked-token rejection; story gate 10a-1 and 10a-3 unblocked
- **[medium] Add PII audit-logging and content-type sniffing validation to the guest ID-document upload/OCR seam -- currently file content-type is client-declared with no server-side sniff (#1783)**
  - dependency: rust-backend
  - dod: Server validates content-type via magic-byte sniff; PII upload events emitted to audit trail; OCR result access gated to manager role

## Risks

- **[medium/high] Message attachment file_key IDOR (#1791): link_message_attachment persists any client-supplied S3 key without prefix validation -- participant of thread A can link an object from thread B's namespace, poisoning the attachment record and gaining a presigned download URL for an unauthorized S3 object**
  - mitigation: Server-side prefix validation (messages/{thread_id}/) before DB insert; add cross-thread key injection integration test
- **[high/high] OAuth refresh-token revocation bypass (#481): removed revoked_at IS NULL predicate means revoked tokens may be accepted post-rotation, breaking RFC 9700 and enabling session hijacking via stolen refresh token**
  - mitigation: Restore revocation predicate; block 10a-1 and 10a-3 story promotion until closed
- **[low/high] Weak JWT_SECRET / ESIGN_TOKEN_SECRET accepted in production (#1758): preflight only checks non-empty, a 1-character secret passes, making token forgery trivially feasible if a short secret is deployed by accident**
  - mitigation: Add >=32-byte length floor to check_required_env; add explicit test cases for short-secret rejection
- **[medium/high] Guest PII exposure via list_bookings / get_booking (#1766): both rental booking endpoints scope by tenant but not by manager role, allowing any authenticated org member (e.g., a resident with org access) to read guest name, email, and phone for all bookings**
  - mitigation: Add manager-role guard matching the pattern used in Airbnb reservations (PR #1741); scope with require_manager_role helper already present in codebase
- **[medium/high] WebSocket JWT token in query parameter logged (#480): JWT access token transmitted as URL query param appears in access logs, enabling token exfiltration from log aggregation systems; session not re-validated after expiry**
  - mitigation: Switch WS handshake to Authorization header or short-lived ticket exchange; add expiry re-validation on WS connection upgrade

## Open questions

- Is the third JWT access-token verification copy (#1782) in a crate outside servers/api-server/src/services/jwt.rs (e.g., api-core or integrations)? The grep of services/ found only one JwtService struct, suggesting the unmigrated copy may be in a different crate layer.
- Do OCR endpoints referenced in #1772 run unauthenticated in the documents/intelligence router or only the external-facing reality-server? The intelligence.rs reprocess_ocr handler carries AuthUser + TenantExtractor -- is there a separate unauthenticated OCR ingestion path?
- What is the role assertion model for list_bookings / get_booking -- is TenantExtractor intended to be the only gate (manager context implied by org membership) or is an explicit manager-role check planned for #1766?
- Has the reality-web SSO callback e2e issue (#1826 -- integration-only pattern cementing) been assigned to a sprint story, or is it tracked as a standalone hardening item outside the current sprint?
- Is migration 00192 duplicate (#1760) resolved on dev? Sprint status does not reference it; a broken migration in the sequence can silently skip RLS policy creation for later tables.

## Decisions needed

- Decide whether list_bookings / get_booking guest PII access requires manager role (matching Airbnb gate) or whether any org member is legitimately entitled to read booking guest PII -- owner: product-owner + rust-backend
- Confirm whether the OAuth epic (10a) stories can enter review with #481 (revocation bypass) still open or whether the story gate must hold until #481 is closed -- owner: pm-delivery
- Approve or defer minimum-length secret enforcement (#1758) as a prod-deploy gate vs. a later hardening item given the low-probability/high-impact profile -- owner: pm-security + DevOps
- Determine whether the WebSocket JWT-in-query-param issue (#480) requires an architecture change (ticket exchange) or a quick mitigation (masked logging + expiry re-check) for the current sprint -- owner: rust-backend + pm-security
