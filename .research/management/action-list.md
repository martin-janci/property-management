# Action list

<sub>Generated: 2026-06-27T14:50:00Z</sub>

| Status | Priority | Owner | Action | Dependency |
|--------|----------|-------|--------|------------|
| open | high | pm-tech-lead | iOS SearchView.swift does not compile — performSearch/scheduleSearch undefined, resultsGrid corrupted | none |
| open | high | pm-tech-lead | ReportFaultScreen.tsx handleSubmit() fakes API call with setTimeout(1500) — fault reports never reach backend (App.tsx:1 | none |
| open | high | pm-backend | search_alert_drainer.rs:244-326 | none |
| open | high | pm-backend | saved_search_alerts.rs:216-240 | none |
| open | high | pm-tech-lead | Reality-web listing detail SSR crashes on partial 200 body — JSON-LD build deref of undefined fields | none |
| open | high | pm-tech-lead | Reality-web RealtorManagement.tsx hardcoded English strings — agency flow not localized to sk/cs/de | none |
| open | high | pm-tech-lead | Reality-web ComparisonUrlHandler hits non-existent /api/listings/${id} — every shared comparison URL 404s | none |
| open | high | pm-scrum-master | Close or defer test-hardening issues #481, #487 (OAuth backend) so 10a-1 and 10a-3 can be promoted; assign rust-backend  | none |
| open | high | pm-scrum-master | Fix Epic 16 drainer HIGH bugs (row reservation + transactional enqueue) before the next alert worker deployment; assign  | none |
| open | high | pm-scrum-master | Reconcile sprint-status.yaml: promote Epic 6 (all 6 stories), Epic 7A (all 5 stories), Epic 10B (all 7 stories) to done  | none |
| open | high | pm-security | Fix issue #487: add MFA rate-limit integration test covering brute-force lockout (≥N wrong codes within window → 429) be | none |
| open | high | pm-security | Resolve issue #480: move WebSocket auth off the URL query parameter — either (a) exchange a short-lived WS-specific one- | none |
| open | high | pm-security | Review and merge IDOR draft PR #1857 (security-llm-doc-idor): LLM document endpoints must enforce tenant-scoped RLS just | none |
| open | high | pm-security | Verify and close issue #481: read backend/crates/db/src/repositories/session.rs find_by_token_hash_any_status to confirm | none |
| open | high | pm-tech-lead | IDOR: ai.rs LLM-doc handlers publish/list/get any tenant's listing descriptions & photo enhancements unscoped | none |
| open | medium | pm-frontend | Coverage gap [mvp]: Direct Messaging — verify and finish to done. Gaps: sprint-status ready-for-dev — code is well ahead | none |
| open | medium | pm-frontend | Coverage gap [mvp]: Mediation and Resolution — verify and finish to done. Gaps: Party submissions endpoints unwired (api | none |
| open | medium | pm-frontend | useDeepLinkRouting.ts:27-36 — initialize() re-runs on onNavigate identity change + void promise with no .catch → duplica | none |
| open | medium | pm-backend | search_alert_drainer.rs:122-174 | none |
| open | medium | pm-scrum-master | Close test-hardening issue #482 (ProtectedRoute multi-tenant role fallback) so 10a-2 can be promoted; assign react-web t | none |
| open | medium | pm-scrum-master | Wire party submissions endpoints in ppt-web dispute-detail to unblock 80-3-mediation-resolution; update dispute-detail s | none |
| open | medium | pm-security | Audit guest ID-document OCR pipeline (Epic 18, story 18.2, route ai/ocr.rs) for PII leakage: confirm OCR result fields ( | none |
| open | medium | pm-security | Confirm message-attachment presigned-upload IDOR posture: validate that link_message_attachment (messaging.rs) re-checks | none |
| open | low | pm-backend | Coverage gap [phase3]: pgvector RAG Migration — verify and finish to done. Gaps: RAG retrieval/query service (embedding  | none |
| open | low | pm-backend | Churn hotspot: 53 lines in backend/crates/db/src/repositories/form.rs (PR #1379 #1332 unblock) | none |
| open | low | pm-backend | Churn hotspot: backend/crates/db/src/repositories/reality_portal.rs (+59/−28 in PR #1297 PAP-142 IDOR scoping) | none |
| open | low | pm-scrum-master | Update sprint-status.yaml sprint_name and sprint_goal to reflect the active delivery scope (Epics 3, 4, 11, 16, 18, mess | none |
| in-progress | medium | pm-frontend | Coverage gap [mvp]: Dispute Filing Flow — verify and finish to done. Gaps: Redesigned 5-step wizard (redesignStatus: in- | none |
| in-progress | low | pm-backend | Churn hotspot: backend/crates/db/tests/form_rls_repo_tests.rs touched 2x since 2026-06-12 (window 2026-06-12→2026-06-13) | none |
| in-progress | low | pm-backend | Churn hotspot: backend/crates/integrations/src/booking.rs (+404/−29 in PR #1294 Booking.com OTA retry) | none |
| done | medium | pm-frontend | Coverage gap [mvp]: Contextual Help & Documentation — verify and finish to done. Gaps: sprint-status still ready-for-dev | none |
| done | medium | pm-backend | Coverage gap [mvp]: Announcement Creation & Targeting — verify and finish to done. Gaps: sprint-status still 'review' (n | none |
| done | medium | pm-frontend | Coverage gap [mvp]: Announcement Viewing & Acknowledgment — verify and finish to done. Gaps: sprint-status ready-for-dev | none |
| done | medium | pm-backend | Coverage gap [mvp]: Announcement Comments & Discussion — verify and finish to done. Gaps: sprint-status ready-for-dev —  | none |
| done | medium | pm-backend | Coverage gap [mvp]: Pinned Announcements — verify and finish to done. Gaps: sprint-status ready-for-dev — not marked don | none |
| done | medium | pm-frontend | Coverage gap [mvp]: API Client Integration for Core Features — verify and finish to done. Gaps: coverage.json (last_chec | none |
| done | medium | pm-frontend | Mobile RN production screens (Buildings/Meters/Leases/PersonMonths/Notifications/Threads/Forms) render hardcoded MOCK_*  | none |
| done | low | pm-backend | Churn hotspot: 2940 lines changed in backend/crates/db/src/repositories/document.rs (window 2026-06-10 03:05Z→18:30Z) | none |
| done | low | pm-backend | Churn hotspot: backend/crates/db/src/repositories/sensor.rs (+248/-86 in PR #1321/#1322 PAP-151 re-land + fmt) | none |
| done | low | pm-backend | Churn hotspot: 2856 lines changed in backend/crates/db/src/repositories/subscription.rs (window 2026-06-10 03:05Z→18:30Z | none |
| done | low | pm-backend | Churn hotspot: 1021 lines changed in backend/servers/api-server/src/routes/emergency.rs (window 2026 | none |
| done | low | pm-backend | Churn hotspot: 709 lines changed in backend/servers/api-server/src/routes/enhanced_tenant_screening. | none |
| done | low | pm-backend | Churn hotspot: backend/servers/api-server/src/routes/forms.rs touched 2x since 2026-06-12 (window 2026-06-12→2026-06-13) | none |
| done | low | pm-backend | Churn hotspot: backend/servers/api-server/src/routes/iot.rs (+278/-403 in PR #1321/#1322 PAP-151 re-land + fmt) | none |
| done | low | pm-backend | Churn hotspot: backend/servers/api-server/src/routes/reserve_funds.rs (+228/-255 in PR #1321 PAP-151 re-land) | none |
| done | low | pm-backend | Churn hotspot: 929 lines changed in backend/servers/api-server/src/routes/vendors.rs (window 2026-06 | none |
| done | low | pm-frontend | Report execution-history: verify+harden presigned-download + retry end-to-end (coverage 81-2) | none |
| dropped | medium | pm-backend | Report schedule editing round-trips cron through overloaded `time` field — add dedicated cron_expression column (migrati | none |
| dropped | low | pm-backend | Churn hotspot: backend/servers/api-server/src/routes/api_ecosystem.rs (+106/−27 in PR #1293 PAP-171; second touch in 24h | none |
| dropped | low | pm-backend | booking_oauth_csrf_tests.rs hotspot — 484-line NEW test file (PR #1393 #1424 OAuth CSRF coverage) | none |
| dropped | low | pm-backend | booking_oauth_routes_tests.rs hotspot — 381-line NEW test file (PR #1393 OAuth routes coverage) | none |
| dropped | low | pm-backend | Churn hotspot: backend/servers/api-server/tests/reserve_funds_cross_org_idor_tests.rs touched 2x since 2026-06-12 (windo | none |
| dropped | low | pm-frontend | Churn hotspot: 94 lines in frontend/apps/mobile/app.config.ts (PR #1383 gap-85-2) | none |
| dropped | low | pm-devops | PR #1274 (cargo-minor-patch group, /backend, 9 updates) closed unmerged — superseded by #1313 after auto-rebase fix land | none |
| dropped | low | pm-devops | PR #1425 (GH #1377 document presigned-URL tests) closed unmerged — superseded by merged #1394 | none |
| dropped | low | pm-devops | PR #1179 (docs(epics) catalog backfill for 37 mounted-but-undocumented backend modules) — stalled at 7d, no reviewDecisi | none |
| dropped | low | pm-frontend | Stalled review: PR #988 (Epic: reusable Playwright E2E framework + sitemap FlowRunner) open 10d, no reviewDecision | none |
| dropped | low | pm-tech-lead | Issue #1380 (no labels, OPEN): Dispatcher stale gap-scan buffer + Tier-2 escalation endpoint misconfigured | none |
