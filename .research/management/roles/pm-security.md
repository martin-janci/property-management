# pm-security — 2026-06-27

_Rotating role this run (pm_cursor idx 5, ~30d stale since last run 2026-05-27). Static read; no compile/run._

## Summary

The just-shipped messaging-N-party + OCR + Stripe + Booking.com surfaces inherit a pile of IDOR / PII / replay risks. Post-merge review opened 47 `from-merged-review` follow-up issues #1758-#1854, most of them security-flavoured. The dominant pattern repeats the prior `ai.rs` equipment / report-schedule cluster: a fresh mutating handler that takes `RequestPrincipal` but discards tenant_id, leaving cross-tenant access open. Multiple hardening drafts are already in flight (#1797 OCR auth + PII, #1799 message-attachment IDOR, #1806 booking_channel DB-backed authz, #1823 guest ID-doc PII, #1824 Stripe hardening, #1825 Booking.com currency, #1857 LLM-doc IDOR regression tests). Landing this cluster is the gate to GA on the new financial / messaging / rental surfaces.

## next_actions

- **[high]** Close IDOR follow-up #1791 (message attachment cross-tenant access) by landing PR #1799: thread tenant_id into message-attachment fetch + add cross-tenant regression test. DoD: foreign-tenant fetch returns 404; per-handler integration test green. dependency: none.
- **[high]** Close OCR endpoints unauth (#1772) + PII leak via PR #1797: require manager auth on /api/v1/ocr/*; filter PII fields from passport/ID-document response shapes. DoD: unauth returns 401; PII filtered. dependency: none.
- **[high]** Land Stripe + Booking.com hardening (PR #1764/#1824 Stripe webhook signature + idempotency; PR #1825 Booking.com currency validation). DoD: webhook replay rejected; currency mismatch → 400. dependency: pm-backend.
- **[medium]** Land PR #1857 LLM-doc IDOR regression tests — keeps the OCR/LLM document surface tenant-scoped under refactor. DoD: per-handler cross-tenant test in CI for every LLM/OCR mutating endpoint. dependency: pm-qa.
- **[medium]** Land PR #1806 booking_channel DB-backed authz: replace in-memory role check with DB-authoritative capability lookup. DoD: cross-tenant booking-channel mutation rejected by RLS + capability. dependency: pm-backend.

## risks

- **Message attachment IDOR cluster (high/high):** #1791 — cross-tenant access to message-thread attachments via attachment-id path param without tenant-scope check. Same class as prior ai.rs equipment / report-schedule clusters; pattern repeats across the just-shipped messaging N-party + attachments surface. Mitigation: land #1799 + audit all messaging.rs handlers.
- **OCR / guest-ID-document surface (medium/high):** #1772, #1823 — passport/ID-document PII leaks in API responses; OCR endpoints lack manager auth. High-impact under GDPR. Mitigation: land #1797 + #1823; response-shape filter; per-request signed access for raw scans.
- **Stripe webhook replay / currency confusion (medium/high — carry-over):** #1764/#1824/#1825 — without webhook signature verification + idempotency, a replay attack can double-credit or skip a charge; currency-confusion (charge-in-different-currency) is open on Booking.com integration. Mitigation: land the three PRs.
- **47 open `from-merged-review` follow-ups untriaged (medium/medium):** the post-merge review identified 47 issues #1758-#1854 across the 95-PR window — not yet bucketed by owner/severity. Mitigation: pm-scrum-master triage pass.

## open_questions

- Is the messaging-attachment IDOR class limited to attachment fetch, or does it span reaction/thread-membership endpoints too?
- Does the OCR surface (LLM-doc + guest ID-doc + rental PII) share a single missing PII-filter layer, or are these per-handler omissions?
- Are the Stripe + Booking.com payment surfaces sharing a webhook-verification middleware, or each open-coding it?

## decisions_needed

- Treat the IDOR / PII / Stripe-hardening cluster as a pre-GA blocker for the new financial / messaging / rental surfaces — owner: pm-security.
- pm-scrum-master to slot the 47 `from-merged-review` follow-ups into the sprint as a single hardening batch (analogous to thb-2026-05-25).
