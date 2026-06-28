# pm-security — 2026-06-28

_Last run: 2026-05-27 (32d stale). Sprint: Epic 6, 7A, 8A & 10A. Static read; no compile/run._

## Summary

The sprint's highest-severity open issues are unresolved OAuth refresh-token revocation bypass (#481, open, gates 10a-1 and 10a-3) and WebSocket JWT token exposure in access logs (#480, open, severity high). PR #1857 (IDOR fix for LLM document context) cannot be verified via gh API due to proxy GraphQL restriction — its merge-readiness is a blocker that must be confirmed through the web UI or direct repo access.

## Next actions

| # | Action | Priority | Dependency | DoD |
|---|--------|----------|------------|-----|
| 1 | Verify and merge PR #1857 (security-llm-doc-idor IDOR fix) — confirm CI green and no outstanding review comments before prod | high | none | PR #1857 merged to dev; IDOR on LLM document context confirmed remediated |
| 2 | Resolve issue #481: integration test in oauth_integration_tests.rs replays a revoked refresh token and asserts 401; gate 10a-1 / 10a-3 promotion on it | high | rust-backend | Test in CI; #481 closed |
| 3 | Resolve issue #480: strip JWT from WebSocket query-param before access log; re-validate JWT on long-lived WS connection expiry | high | rust-backend | WS upgrade no longer logs bearer token; expiry re-check added; #480 closed |
| 4 | Fix OCR auth gap (#1772): mount OCR routes behind the global auth middleware layer in addition to AuthUser extractor; add 401 regression test | high | rust-backend | Unauthenticated request returns 401; regression test in CI |
| 5 | Enforce server-side Stripe idempotency on checkout session creation (#1785) — not just webhook settlement | medium | rust-backend | #1785 closed; duplicate creation within window returns 409 or cached response |
| 6 | Mask PII fields (guest email, phone) in Booking.com integration tracing in `backend/crates/integrations/src/booking/mod.rs` | medium | rust-backend | No guest PII at info/debug level |

## Risks

| # | Risk | P | I | Mitigation |
|---|------|---|---|------------|
| 1 | Issue #481 (OAuth revocation bypass) is open and gates 10a-1 / 10a-3 — RFC 9700 violation if shipped | medium | high | Code at auth.rs:1126 checks `revoked_at.is_some()` correctly, but issue is open because a test proving the path is missing — add replay test, close issue, block prod deploy of Epic 10A |
| 2 | PR #1857 DRAFT/pending after 12-day routine lag; LLM document endpoints exploitable cross-tenant until merged | high | high | Prioritize merge this week; do not expose LLM doc endpoints to non-internal traffic until merged |
| 3 | Message attachment IDOR (#1791) — object-store ref checks missing; cross-tenant attachment enumeration | medium | high | Add org_id ownership check before presigned URL generation in messaging attachment handler; release blocker |
| 4 | WS JWT in query param (#480) lands in access logs / Loki / CloudWatch — bearer credentials leaked to anyone with log read | high | high | Move WS auth to short-lived ticket OR strip token at Tower middleware before logging |
| 5 | oauth_integration_tests.rs is the highest-churn file this run (2718 LOC, REPEAT) — unstable auth test suite masks regressions | medium | high | Assign single owner to stabilize the file; require CI green on it before any Epic 10A `done` promotion |

## Open questions

1. PR #1857 merge-readiness could not be confirmed via gh API (GraphQL proxy disabled). What is the current review/CI status of #1857 and who is the designated approver?
2. For issue #1772 (OCR endpoints unauthenticated): the OCR router uses AuthUser extractors per-handler but it is unclear whether it is mounted inside a global auth middleware layer or relies solely on extractor-level 401. Which mount point applies?
3. Issue #1791 (message attachment IDOR) has no associated PR. Is there an active fix branch, or is it unassigned?
4. The `routes/migration.rs` tenant-boundary check compares `organization_id != Some(org_id)` for non-system templates, but skips the check for system templates. Is cross-org access to system templates intentional and documented, or a latent privilege escalation path?
5. Issue #1786 (sensor WebSocket authz integration tests missing): was #1737 (sensor WS auth PR) reviewed for **authz** correctness in addition to authn, or only authentication?

## Decisions needed

- Should PR #1857 be treated as a prod release blocker (hold all prod deploys until merged) or only block the LLM/AI document feature flag? — owner: tech-lead / product
- Message attachment IDOR (#1791) has no milestone or assignee — assign as sprint blocker for current messaging story set or defer to a hardening sprint? — owner: tech-lead
- Issue #480 (WS JWT in logs): adopt one-time ticket approach or strip at middleware layer? Architecture decision required before implementation — owner: rust-backend lead

## Raw role JSON

```json
{
  "role": "pm-security",
  "summary": "The sprint's highest-severity open issues are unresolved OAuth refresh-token revocation bypass (#481, open, gates 10a-1 and 10a-3) and WebSocket JWT token exposure in access logs (#480, open, severity high). PR #1857 (IDOR fix for LLM document context) cannot be verified via gh API due to proxy GraphQL restriction — its merge-readiness is a blocker that must be confirmed through the web UI or direct repo access.",
  "next_actions": [
    {"action":"Verify and merge PR #1857 (security-llm-doc-idor IDOR fix)","priority":"high","dependency":"none","definition_of_done":"PR #1857 merged to dev with all CI gates green; IDOR on LLM document context endpoints confirmed remediated"},
    {"action":"Resolve issue #481 (OAuth refresh-token revocation bypass) by adding replay test; block 10a-1/10a-3 promotion","priority":"high","dependency":"rust-backend","definition_of_done":"Integration test in oauth_integration_tests.rs covers revoked-token replay; issue #481 closed"},
    {"action":"Resolve issue #480 (WS JWT in access logs) — strip token + re-validate on JWT expiry","priority":"high","dependency":"rust-backend","definition_of_done":"WS upgrade no longer emits bearer token to access log; expiry re-check added; issue #480 closed"},
    {"action":"Fix OCR auth gap (#1772) — confirm middleware mount + add 401 regression test","priority":"high","dependency":"rust-backend","definition_of_done":"Unauthenticated request to both OCR endpoints returns 401; regression test in CI"},
    {"action":"Enforce server-side Stripe idempotency on checkout session creation (#1785)","priority":"medium","dependency":"rust-backend","definition_of_done":"Issue #1785 closed; duplicate checkout creation within idempotency window returns 409 or cached response"},
    {"action":"Mask PII fields in Booking.com integration tracing","priority":"medium","dependency":"rust-backend","definition_of_done":"No guest.email or guest.phone values appear in tracing output at info/debug level in backend/crates/integrations/src/booking/mod.rs"}
  ]
}
```
