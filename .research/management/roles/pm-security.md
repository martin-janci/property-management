# pm-security — 2026-06-25

_Catch-up read: 28-day gap since 2026-05-27. Static analysis only; no compile/run._

## Executive Summary

The sprint since 2026-05-27 shipped meaningful security improvements: JWT-trusting duplicate
WebSocket channel removed (#1737), RLS service-role allowlist formalised (#1729), CSPRNG
failure propagation hardened (#1684), and the Airbnb reservation manager-gate (#1741) closes
a guest-PII access gap. However, six open draft PRs (#1823–#1825, #1795, #1797, #1799) are
each addressing high-severity findings that were surfaced AFTER the production-candidate
merges landed; this means the current `dev` branch ships with active known vulnerabilities
in OCR/ID-document PII, Stripe Checkout, attachment IDOR, and listing-status manipulation.
The pre-existing `security-llm-doc-idor` backlog item (score 3, status ready, untouched 28d)
remains the one unscheduled exploitable cross-tenant write IDOR.

## Top Concerns

- **Guest ID-document PII unprotected in transit (CRITICAL — draft #1823):** PR #1750 merged
  the ID-document upload + OCR seam without: content-sniff validation (MIME declared in
  multipart header vs. actual file magic bytes — an attacker can upload executable content
  under a PDF MIME), audit logging of PII access, or server-side content inspection. Draft
  #1823 adds these controls but is still in draft. Until merged, uploaded national ID scans
  are stored without content verification and accessed without an audit trail. File:
  `/home/user/property-management/backend/servers/api-server/tests/rental_guest_id_document_tests.rs` —
  current tests cover MIME type declared in the multipart header but do not test magic-byte
  mismatch.

- **OCR endpoints lack auth gate (HIGH — draft #1797):** The meter-reading OCR route
  (`/api/v1/ai/ocr/meter-reading`, `/api/v1/ai/ocr/correction`) in
  `/home/user/property-management/backend/servers/api-server/src/routes/ai/ocr.rs` has NO
  authentication extractor — `process_meter_reading` and `submit_correction` accept
  unauthenticated multipart uploads. The correction endpoint also lacks a manager-role gate
  for rental guest PII paths. Draft #1797 fixes this but is unmerged.

- **Stripe Checkout: no idempotency key, currency not ISO-4217 validated (HIGH — draft #1824):**
  `create_checkout_session` in
  `/home/user/property-management/backend/servers/api-server/src/services/stripe.rs` sends
  no `Idempotency-Key` header; a network retry or double-click can create duplicate payment
  sessions for the same invoice. The `currency` parameter is passed to Stripe raw — no
  ISO-4217 allowlist check. Draft #1824 and the Booking.com draft #1825 (currency allowlist)
  address this but are both still in draft.

- **Attachment file_key not bound to thread — IDOR open (HIGH — draft #1799):** Thread
  attachment IDOR: file keys can be fetched across threads/tenants by guessing the UUID.
  MIME validation gap also present. Draft #1799 is unmerged.

- **LLM-doc IDOR residual: list_listing_descriptions still tenant-blind (MEDIUM — backlog
  security-llm-doc-idor):** PR #879 fixed `publish_description` and `get_photo_enhancement`
  but `list_listing_descriptions` (routes/ai.rs ~l2666) still discards `_principal` and calls
  the tenant-blind `list_listing_descriptions(listing_id)`. This is a cross-tenant read IDOR,
  item `security-llm-doc-idor` (score 3, status ready) untouched for 28 days. File:
  `/home/user/property-management/backend/servers/api-server/src/routes/ai.rs`.

## Specific Next Actions

1. **[BLOCKER — high]** Merge or unblock draft #1823 (Guest ID-document PII hardening):
   require content-sniff (magic-byte) validation against declared MIME, add audit log on
   every read of `rental_guest_id_documents` rows, and confirm S3 object-key is under a
   manager-only presigned-URL path. DoD: CI green on #1823, content-sniff test passing,
   audit log row emitted on fetch.

2. **[BLOCKER — high]** Add auth extractor to OCR endpoints (draft #1797): mount
   `ValidatedTenantExtractor` or equivalent on both `/ai/ocr/meter-reading` and
   `/ai/ocr/correction`; add manager-role gate for any OCR path that processes rental-guest
   PII. DoD: unauthenticated request returns 401, non-manager returns 403, CI green.

3. **[BLOCKER — high]** Fix Stripe idempotency + currency allowlist (draft #1824): inject
   `Idempotency-Key: invoice-{invoice_id}` on `create_checkout_session`; validate `currency`
   against ISO-4217 before sending to Stripe API. DoD: duplicate-call test shows single
   Stripe session, invalid currency rejected with 422 before hitting Stripe.

4. **[high]** Merge draft #1799 (attachment IDOR + MIME): bind `file_key` to the thread it
   was uploaded to and validate actual MIME before serving. DoD: cross-thread fetch returns
   403/404; executable content type rejected.

5. **[medium]** Schedule and assign `security-llm-doc-idor` backlog item: fix residual
   `list_listing_descriptions` tenant-blind read in `routes/ai.rs`. Estimated scope: 2-line
   handler change + 1 repo predicate + 1 integration test. DoD: cross-tenant read returns
   empty list; test in CI.

6. **[medium]** Verify sprint-status open issues #480 (JWT in WS query-param log) and #481
   (OAuth refresh-token revocation bypass) are still open and gate 10a-* story promotion.
   Both were filed 2026-05-25 and remain open per sprint-status.yaml. DoD: both issues
   closed or explicitly deferred with PM sign-off before 10a-* can be marked done.

## Risks

- **Guest PII exposure via ID-document content-sniff bypass (high/high):** Attacker uploads
  a disguised executable (declares MIME image/jpeg, actual PHP/JS payload) as an ID document.
  File stored and later served to managers. Mitigation: block on draft #1823 merge; add
  magic-byte check in upload handler.

- **Unauthenticated OCR endpoint (high/medium):** Any actor can POST arbitrary multipart
  payloads to `/api/v1/ai/ocr/meter-reading` without a JWT. At minimum an amplified upload
  vector; if OCR backend is wired it becomes a free AI inference endpoint. Mitigation:
  auth extractor (draft #1797) must merge before OCR backend is enabled.

- **Stripe double-charge via retry (medium/high):** Network instability or UI double-submit
  creates two checkout sessions for the same invoice; no idempotency key prevents Stripe
  deduplication. Impact: tenant billed twice. Mitigation: add `Idempotency-Key` (draft
  #1824).

- **LLM-doc cross-tenant read open 28d (medium/medium):** `list_listing_descriptions`
  returns competitor tenant's AI-generated property descriptions. Low exploitability (requires
  enumeration of listing UUIDs) but has been in backlog as `ready` unassigned for 28 days.
  Mitigation: assign and land before next sprint gate.

- **OAuth refresh-token revocation bypass (high/high — carry-forward):** Issue #481 (revoked
  tokens reusable, breaks RFC 9700) is still open per sprint-status and gates 10a-1 and
  10a-3. If either story is promoted to done without closing #481, revoked refresh tokens
  remain valid. Mitigation: enforce issue gate; do not promote 10a-* without #481 closed.

## Open Questions

- Does the OCR meter-reading route (`ocr_router()`) sit behind the authenticated middleware
  layer or is it publicly mounted? The route file shows no extractor; the mount point in
  `lib.rs`/`main.rs` determines exposure — not confirmed in this read.
- Draft #1797 is listed as fixing both auth-on-OCR-endpoints AND manager-gate-rental-guest-PII
  — are these two separate handlers in different routes, or both in `routes/ai/ocr.rs`?
- Is the `rental_guest_id_documents` S3 path (key prefix `id-documents/`) restricted by
  bucket policy or only by presigned-URL TTL? If bucket is public-readable the presigned URL
  is redundant protection.
- Issues #480 and #481 have had no sprint-status update since 2026-05-25 — has either been
  triaged out-of-band without the sprint file being updated?

## Decisions Needed

- Treat drafts #1823, #1797, #1824 as pre-release blockers for any Epic 11/18 production
  promotion — owner: pm-security + pm-tech-lead.
- Assign `security-llm-doc-idor` (backlog, ready, 28d stale) to a Rust backend engineer this
  sprint or explicitly defer with documented risk acceptance — owner: pm-tech-lead.
