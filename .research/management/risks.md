# Risks

<sub>Generated: 2026-07-26T05:20:00Z</sub>

| Status | Prob | Impact | Owner | Risk | Mitigation |
|--------|------|--------|-------|------|------------|
| open | high | high | pm-tech-lead | Recurring cross-tenant IDOR patches across independent domains (documents #2438/#2447, disputes #245 | Add a lint/macro or checklist gate requiring explicit org-scope proof for any new tenant-scoped hand |
| open | high | medium | pm-tech-lead | auth.rs and reports.rs are god-files absorbing repeat cross-epic churn (OAuth, MFA, cold-boot fixes  | Bound file growth with a module-split threshold; assign clear sub-ownership |
|  | high | medium | pm-data | Multiple shipped MVP features (Epic 6, 10A, 10B, 80, 84) lack KPI instrumentation — product/business | Sequence pm-data KPI-definition tasks (7 items just added to action-list); establish minimum-analyti |
|  | high | medium | pm-data | FaultStatusCount metric (support-data) diverges from owner/portfolio fault KPIs (open decision from  | Land single-source-of-truth metric definitions in shared module; deprecate duplicates |
| open | medium | high | pm-tech-lead | Test-restoration wave un-quarantines then re-quarantines tests in the same PR (#2511: 48 restored, 1 | Require root-cause note per re-quarantined test, not silent re-ignore |
|  | medium | high | pm-qa | Announcement cross-tenant fan-out guard is tested only via a pure-Rust re-model, not the real SQL (# | Replace pure-Rust model test with sqlx integration test that exercises the actual RLS policy |
|  | medium | high | pm-security | Layout publish webhook lacks timestamp/replay protection (#2485) — a captured legitimate publish can | Fix #2485 (add nonce+timestamp+HMAC parity with esignature webhook); track integration test in actio |
|  | medium | high | pm-integration | Webhook handlers across integrations (booking, airbnb, esignature, layout) lack consistent hardening | Cross-cutting audit action-list item integrations-webhook-hardening-audit-2026-07-23 |
| open | medium | medium | pm-scrum-master | auth.rs (runs_seen 4) and reports.rs (runs_seen 3) are repeat churn hotspots with no consolidation t | Escalate the churn-hotspot refactor items for auth.rs/reports.rs from low to medium priority next sc |
| open | medium | medium | pm-scrum-master | fix #2547 (retention-prune scheduler wiring) merged without automated test coverage — implementer no | Queue a follow-up integration/e2e test exercising the scheduler firing path before next retention wi |
| open | medium | medium | pm-tech-lead | #2547 scheduler-firing test gap is not isolated - auto-unpin, favorite-alerts, notification-triggers | Build one reusable scheduler test harness rather than one-off fixes per job |
| open | medium | medium | pm-tech-lead | booking/mod.rs (3185 lines) newly hot despite 83-2 marked done - could signal instability reintroduc | Fold into the already-planned cross-cutting webhook/integration audit |
|  | medium | medium | pm-data | Append-only support_tooling_events + support-data audit trail have ON DELETE RESTRICT but no TTL — l | Publish retention policy; if GDPR-in-scope, add lifecycle policy for personal-data-containing events |
|  | medium | medium | pm-security | Mobile LAYOUT_CACHE_KEY is not tenant-scoped and survives logout (#2486) — user A's layout can leak  | Fix #2486 (namespace cache key by tenant_id + purge on logout); add regression test in QA action-lis |
|  | low | high | pm-security | add_evidence dispute sub-resource remains cross-tenant-writable until PR #2490 lands (#2483) — PR #2 | Land PR #2490 promptly; add subroute-authz regression as part of ongoing IDOR sweep pattern |
