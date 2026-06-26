# pm-security — 2026-06-26

_Last run: 2026-06-26T06:30:00Z (previous run 2026-05-27)_

## Summary

7/8 test-hardening issues from thb-2026-05-25 still open and gating Epic 10A OAuth stories; 4 security-critical PRs (#1797 OCR auth, #1799 messaging file_key confused-deputy, #1806 booking_channel manager gate, #1823 rental ID PII) still unmerged as of 2026-06-26.

## Next actions

- **[high]** Merge PR #1797 (OCR auth) — lib.rs:256 mounts /api/v1/ai/ocr/* without auth extractor — owner: `pm-backend`
- **[high]** Merge PR #1799 + add file_key prefix validation in link_message_attachment (messaging.rs ~1530) so file_key must begin with messages/{thread_id}/ — owner: `pm-backend`
- **[high]** Close issue #481 (OAuth refresh-token revocation): verify session.rs:55 fix is comprehensive across all token lookup paths and close to unblock 10a-1/10a-3 — owner: `pm-backend`
- **[medium]** Investigate #482 (ProtectedRoute tenants[0] role fallback): add multi-tenant unit test and close — owner: `pm-frontend`
- **[medium]** Merge PR #1823 (rental guest ID PII hardening): drop %file_key from tracing::error structured field (rentals.rs:1184) — owner: `pm-backend`
- **[medium]** Close issue #480 end-to-end (WS expiry disconnect integration test) to unblock 8a-3 — owner: `pm-backend`

## Risks

- **high/high** — Unauthenticated OCR endpoints (POST /api/v1/ai/ocr/meter-reading + /correction): both handlers lack auth extractor; multipart parser runs without JWT; any future payload processing behind the door is immediately exploitable
  - Mitigation: Block PR #1797 close until both handlers have auth + CI test asserts 401 on unauth path
- **medium/high** — Messaging confused-deputy: link_message_attachment does not validate file_key starts with messages/{thread_id}/; participant in two threads can link thread-A storage into thread-B
  - Mitigation: Server-side prefix check in link_message_attachment; cross-thread confusion test
- **high/high** — Epic 10A OAuth fully blocked by 3 open hardening issues (#481 revocation, #482 ProtectedRoute, #487 MFA rate-limit) — any new OAuth provider work lands without verified gates
  - Mitigation: Triage each blocking issue (close-with-evidence or formal defer) before epic-10A implementation kicks off
- **medium/medium** — Guest ID-document PII leak via structured logs: rentals.rs:1184 emits %file_key (id-documents/<org>/<file>) in tracing::error fields shipped to SIEM/3rd-party log sinks
  - Mitigation: Replace %file_key with truncated/hashed token; grep all id-document tracing macros
- **low/medium** — Delegation backend remains active after #1713 frontend revert: routes/delegations.rs fully mounted and accepts create/accept/revoke/check — backend may be in inconsistent state still reachable by direct API
  - Mitigation: Confirm BIT-213 retirement scope: if backend kept active intentionally, document allowed caller set + verify RLS isolation tests

## Open questions

- Has PR #1806 (DB-backed manager gate replacing JWT role claim) merged? Which endpoints still trust the JWT role claim?
- Is the original #481 bug in a different query path beyond session.rs (token-info / OAuth introspection)?
- Have PRs #1824 (Stripe) + #1825 (Booking.com) merged? Do idempotency keys prevent duplicate charges on network retry?
- Issue #483 voice device IDOR: is the list-commands endpoint accessible to non-managers; is the IDOR test added?
- PR #1713 BIT-213 reconciliation rationale: data-model conflict, premature exposure, or frontend security gap — and does it require any backend retirement?

## Decisions needed

- Gate epic-10A OAuth start on closure of #481 + #487, or formally defer with risk-acceptance? (owner: pm-delivery + rust-backend)
- Retire OCR endpoints entirely until real OCR backend wired, or just auth-gate them? (owner: pm-delivery)
- Delegation backend (delegations.rs) — feature-flag-gate or retire in line with BIT-213 frontend retirement, or intentionally keep as headless API? (owner: pm-delivery + rust-backend)
