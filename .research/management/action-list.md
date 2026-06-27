# PPT delivery — action list

_Generated: 2026-06-27T12:30:00Z_

Status counts: done=17, dropped=11, in-progress=3, open=17

| Status | Priority | Owner | Action | Source |
|--------|----------|-------|--------|--------|
| in-progress | medium | pm-frontend | Coverage gap [mvp]: Dispute Filing Flow — verify and finish to done. Gaps: Redesigned 5-step wizard (redesignStatus: ... | dispatcher-tier1-refill 2026-06-25 (cove |
| in-progress | low | pm-backend | Churn hotspot: backend/crates/db/tests/form_rls_repo_tests.rs touched 2x since 2026-06-12 (window 2026-06-12→2026-06-13) | dispatcher-tier1-refill 2026-06-22 (back |
| in-progress | low | pm-backend | Churn hotspot: backend/crates/integrations/src/booking.rs (+404/−29 in PR #1294 Booking.com OTA retry) | dispatcher-tier1-refill 2026-06-22 (back |
| open | high | pm-scrum-master | Fix HIGH duplicate-delivery bug in search_alert_drainer.rs:317-325 — mark_search_alert_notified must run in same txn ... | pm-analysis 2026-06-27 |
| open | high | pm-scrum-master | Fix MED silent DB-error swallow in saved_search_alerts.rs:176-180/237-240 — watermark update failures must surface to... | pm-analysis 2026-06-27 |
| open | high | pm-scrum-master | Resolve 11-day research-land replay lag — 2026-06-22 and 2026-06-25 briefs on session branches were never replayed on... | pm-analysis 2026-06-27 |
| open | high | pm-scrum-master | Unblock Epic 10A by closing/deferring test-hardening gates #481 (OAuth refresh-token revocation), #482 (ProtectedRout... | pm-analysis 2026-06-27 |
| open | high | pm-security | Audit issue #480: WebSocket auth token in query param — confirm converged DB-checked handler (PR #1737) never writes ... | pm-analysis 2026-06-27 |
| open | high | pm-security | Close or scope issue #1791 (message attachment IDOR): messaging_attachments_authz_tests.rs covers participant isolati... | pm-analysis 2026-06-27 |
| open | high | pm-security | Independently verify #1826 (reality-web SSO callback CSRF): confirm sso_callback at backend/servers/reality-server/sr... | pm-analysis 2026-06-27 |
| open | high | pm-security | Verify issue #481 (OAuth refresh-token revocation) is fully fixed: confirm OAuthRepository::find_refresh_token_by_has... | pm-analysis 2026-06-27 |
| open | medium | pm-frontend | Coverage gap [mvp]: Direct Messaging — verify and finish to done. Gaps: sprint-status ready-for-dev — code is well ah... | dispatcher-tier1-refill 2026-06-25 (cove |
| open | medium | pm-frontend | Coverage gap [mvp]: Mediation and Resolution — verify and finish to done. Gaps: Party submissions endpoints unwired (... | dispatcher-tier1-refill 2026-06-25 (cove |
| open | medium | pm-scrum-master | Green-CI 7a-2-folder-organization (PR #1316) — FK/isolation fix must pass document_folder_tests before promotion from... | pm-analysis 2026-06-27 |
| open | medium | pm-scrum-master | Triage and batch-close the 37 open follow-up issues (labels: follow-up, from-merged-review) — prioritize security-cla... | pm-analysis 2026-06-27 |
| open | medium | pm-security | Fix issue #482 (ProtectedRoute role fallback uses tenants[0] for multi-tenant users): wrong tenant silently grants/de... | pm-analysis 2026-06-27 |
| open | medium | pm-security | Validate search_alert_drainer.rs PII handling: LogEmailTransport logs to_email at INFO; confirm production log filter... | pm-analysis 2026-06-27 |
| open | low | pm-backend | Coverage gap [phase3]: pgvector RAG Migration — verify and finish to done. Gaps: RAG retrieval/query service (embeddi... | dispatcher-tier1-refill 2026-06-25 (cove |
| open | low | pm-backend | Churn hotspot: 53 lines in backend/crates/db/src/repositories/form.rs (PR #1379 #1332 unblock) | dispatcher-tier1-refill 2026-06-22 (back |
| open | low | pm-backend | Churn hotspot: backend/crates/db/src/repositories/reality_portal.rs (+59/−28 in PR #1297 PAP-142 IDOR scoping) | dispatcher-tier1-refill 2026-06-22 (back |
| done | medium | pm-frontend | Coverage gap [mvp]: Contextual Help & Documentation — verify and finish to done. Gaps: sprint-status still ready-for-... | dispatcher-tier1-refill 2026-06-25 (cove |
| done | medium | pm-backend | Coverage gap [mvp]: Announcement Creation & Targeting — verify and finish to done. Gaps: sprint-status still 'review'... | dispatcher-tier1-refill 2026-06-25 (cove |
| done | medium | pm-frontend | Coverage gap [mvp]: Announcement Viewing & Acknowledgment — verify and finish to done. Gaps: sprint-status ready-for-... | dispatcher-tier1-refill 2026-06-25 (cove |
| done | medium | pm-backend | Coverage gap [mvp]: Announcement Comments & Discussion — verify and finish to done. Gaps: sprint-status ready-for-dev... | dispatcher-tier1-refill 2026-06-25 (cove |
| done | medium | pm-backend | Coverage gap [mvp]: Pinned Announcements — verify and finish to done. Gaps: sprint-status ready-for-dev — not marked ... | dispatcher-tier1-refill 2026-06-25 (cove |
| done | medium | pm-frontend | Coverage gap [mvp]: API Client Integration for Core Features — verify and finish to done. Gaps: coverage.json (last_c... | dispatcher-tier1-refill 2026-06-25 (cove |
| done | medium | pm-frontend | Mobile RN production screens (Buildings/Meters/Leases/PersonMonths/Notifications/Threads/Forms) render hardcoded MOCK... | dispatcher-tier1-refill 2026-06-22 (back |
| done | low | pm-backend | Churn hotspot: 2940 lines changed in backend/crates/db/src/repositories/document.rs (window 2026-06-10 03:05Z→18:30Z) | dispatcher-tier1-refill 2026-06-22 (back |
| done | low | pm-backend | Churn hotspot: backend/crates/db/src/repositories/sensor.rs (+248/-86 in PR #1321/#1322 PAP-151 re-land + fmt) | dispatcher-tier1-refill 2026-06-22 (back |
| done | low | pm-backend | Churn hotspot: 2856 lines changed in backend/crates/db/src/repositories/subscription.rs (window 2026-06-10 03:05Z→18:... | dispatcher-tier1-refill 2026-06-22 (back |
| done | low | pm-backend | Churn hotspot: 1021 lines changed in backend/servers/api-server/src/routes/emergency.rs (window 2026 | dispatcher-tier1-refill 2026-06-22 (back |
| done | low | pm-backend | Churn hotspot: 709 lines changed in backend/servers/api-server/src/routes/enhanced_tenant_screening. | dispatcher-tier1-refill 2026-06-22 (back |
| done | low | pm-backend | Churn hotspot: backend/servers/api-server/src/routes/forms.rs touched 2x since 2026-06-12 (window 2026-06-12→2026-06-13) | dispatcher-tier1-refill 2026-06-22 (back |
| done | low | pm-backend | Churn hotspot: backend/servers/api-server/src/routes/iot.rs (+278/-403 in PR #1321/#1322 PAP-151 re-land + fmt) | dispatcher-tier1-refill 2026-06-22 (back |
| done | low | pm-backend | Churn hotspot: backend/servers/api-server/src/routes/reserve_funds.rs (+228/-255 in PR #1321 PAP-151 re-land) | dispatcher-tier1-refill 2026-06-22 (back |
| done | low | pm-backend | Churn hotspot: 929 lines changed in backend/servers/api-server/src/routes/vendors.rs (window 2026-06 | dispatcher-tier1-refill 2026-06-22 (back |
| done | low | pm-frontend | Report execution-history: verify+harden presigned-download + retry end-to-end (coverage 81-2) | dispatcher-tier1-refill 2026-06-23 (cove |
| dropped | medium | pm-backend | Report schedule editing round-trips cron through overloaded `time` field — add dedicated cron_expression column (migr... | dispatcher-tier1-refill 2026-06-23 (cove |
| dropped | low | pm-backend | Churn hotspot: backend/servers/api-server/src/routes/api_ecosystem.rs (+106/−27 in PR #1293 PAP-171; second touch in ... | dispatcher-tier1-refill 2026-06-22 (back |
| dropped | low | pm-backend | booking_oauth_csrf_tests.rs hotspot — 484-line NEW test file (PR #1393 #1424 OAuth CSRF coverage) | dispatcher-tier1-refill 2026-06-22 (back |
| dropped | low | pm-backend | booking_oauth_routes_tests.rs hotspot — 381-line NEW test file (PR #1393 OAuth routes coverage) | dispatcher-tier1-refill 2026-06-22 (back |
| dropped | low | pm-backend | Churn hotspot: backend/servers/api-server/tests/reserve_funds_cross_org_idor_tests.rs touched 2x since 2026-06-12 (wi... | dispatcher-tier1-refill 2026-06-22 (back |
| dropped | low | pm-frontend | Churn hotspot: 94 lines in frontend/apps/mobile/app.config.ts (PR #1383 gap-85-2) | dispatcher-tier1-refill 2026-06-22 (back |
| dropped | low | pm-devops | PR #1274 (cargo-minor-patch group, /backend, 9 updates) closed unmerged — superseded by #1313 after auto-rebase fix l... | dispatcher-tier1-refill 2026-06-22 (back |
| dropped | low | pm-devops | PR #1425 (GH #1377 document presigned-URL tests) closed unmerged — superseded by merged #1394 | dispatcher-tier1-refill 2026-06-22 (back |
| dropped | low | pm-devops | PR #1179 (docs(epics) catalog backfill for 37 mounted-but-undocumented backend modules) — stalled at 7d, no reviewDec... | dispatcher-tier1-refill 2026-06-22 (back |
| dropped | low | pm-frontend | Stalled review: PR #988 (Epic: reusable Playwright E2E framework + sitemap FlowRunner) open 10d, no reviewDecision | dispatcher-tier1-refill 2026-06-22 (back |
| dropped | low | pm-tech-lead | Issue #1380 (no labels, OPEN): Dispatcher stale gap-scan buffer + Tier-2 escalation endpoint misconfigured | dispatcher-tier1-refill 2026-06-22 (back |
