# PPT Action List

_Generated: 2026-06-26T12:18:41Z_

Items: 48 (open=18, in-progress=3, done=16, dropped=11, failed=0)

| Status | Priority | Owner | Action | Source |
|---|---|---|---|---|
| in-progress | medium | pm-frontend | Coverage gap [mvp]: Dispute Filing Flow — verify and finish to done. Gaps: Redesigned 5-step wizard (redesignStatus: in-progress) not shippe | dispatcher-tier1-refill 2026-06-25 (coverage partial-story g |
| in-progress | low | pm-backend | Churn hotspot: backend/crates/db/tests/form_rls_repo_tests.rs touched 2x since 2026-06-12 (window 2026-06-12→2026-06-13) | dispatcher-tier1-refill 2026-06-22 (backlog.json promote) |
| in-progress | low | pm-backend | Churn hotspot: backend/crates/integrations/src/booking.rs (+404/−29 in PR #1294 Booking.com OTA retry) | dispatcher-tier1-refill 2026-06-22 (backlog.json promote) |
| open | high | pm-scrum-master | Close or formally defer security gate issues #481 (revoked token) and #487 (MFA rate-limit) to unblock Epic 10A | pm-analysis 2026-06-26 |
| open | high | pm-scrum-master | Fix 7a-2 CI failure (document_folder_tests FK/isolation) and re-green to move folder-organization from review to done | pm-analysis 2026-06-26 |
| open | high | pm-scrum-master | Resolve issue #480 (JWT in WebSocket query param) — high-severity Phase 1.5 security finding | pm-analysis 2026-06-26 |
| open | high | pm-security | Close or downgrade issue #481 (audit OAuth+session paths confirm revoked_at IS NULL is enforced) | pm-analysis 2026-06-26 |
| open | high | pm-security | Fix reality-server panic paths: replace .expect('OS rng failed') at handlers/users/mod.rs:551 and .expect('Failed to create HTTP client') at | pm-analysis 2026-06-26 |
| open | high | pm-security | Resolve issue #480 (JWT in WS query param): add structured-log test asserting token absent + verify session expiry evicts WS connection | pm-analysis 2026-06-26 |
| open | medium | pm-frontend | Coverage gap [mvp]: Announcement Viewing & Acknowledgment — verify and finish to done. Gaps: sprint-status ready-for-dev — code is ahead of  | dispatcher-tier1-refill 2026-06-25 (coverage partial-story g |
| open | medium | pm-frontend | Coverage gap [mvp]: Direct Messaging — verify and finish to done. Gaps: sprint-status ready-for-dev — code is well ahead of the label but st | dispatcher-tier1-refill 2026-06-25 (coverage partial-story g |
| open | medium | pm-frontend | Coverage gap [mvp]: Mediation and Resolution — verify and finish to done. Gaps: Party submissions endpoints unwired (apiStatus stays partial | dispatcher-tier1-refill 2026-06-25 (coverage partial-story g |
| open | medium | pm-scrum-master | Pick up 7a-3-permission-based-access (ready-for-dev) once 7a-2 lands — gate for 7a-4/7a-5 | pm-analysis 2026-06-26 |
| open | medium | pm-scrum-master | Reconcile sprint-status.yaml epic-6 (5/6 done) and epic-10b (7/7 done) to 2026-06-25 verifications | pm-analysis 2026-06-26 |
| open | medium | pm-security | Add MFA brute-force / rate-limit integration test coverage for #487 (blocks 10a-1) | pm-analysis 2026-06-26 |
| open | medium | pm-security | Fix #482: trace user.role in ProtectedRoute back to deriveActiveRole; add multi-tenant unit tests | pm-analysis 2026-06-26 |
| open | medium | pm-security | Review PR #1809 accounting: grep for residual provider_secret/api_key fields without #[serde(skip_serializing)]; confirm 404 body tenant-neu | pm-analysis 2026-06-26 |
| open | low | pm-backend | Coverage gap [phase3]: pgvector RAG Migration — verify and finish to done. Gaps: RAG retrieval/query service (embedding generation + similar | dispatcher-tier1-refill 2026-06-25 (coverage partial-story g |
| open | low | pm-backend | Churn hotspot: 53 lines in backend/crates/db/src/repositories/form.rs (PR #1379 #1332 unblock) | dispatcher-tier1-refill 2026-06-22 (backlog.json promote) |
| open | low | pm-backend | Churn hotspot: backend/crates/db/src/repositories/reality_portal.rs (+59/−28 in PR #1297 PAP-142 IDOR scoping) | dispatcher-tier1-refill 2026-06-22 (backlog.json promote) |
| open | low | pm-scrum-master | Triage the 75 auto-filed follow-up/from-merged-review issues (#1437–#1852) for promotable gates | pm-analysis 2026-06-26 |
| done | medium | pm-frontend | Coverage gap [mvp]: Contextual Help & Documentation — verify and finish to done. Gaps: sprint-status still ready-for-dev though backend+web+ | dispatcher-tier1-refill 2026-06-25 (coverage partial-story g |
| done | medium | pm-backend | Coverage gap [mvp]: Announcement Creation & Targeting — verify and finish to done. Gaps: sprint-status still 'review' (not flipped to done)  | dispatcher-tier1-refill 2026-06-25 (coverage partial-story g |
| done | medium | pm-backend | Coverage gap [mvp]: Announcement Comments & Discussion — verify and finish to done. Gaps: sprint-status ready-for-dev — not marked done; ann | dispatcher-tier1-refill 2026-06-25 (coverage partial-story g |
| done | medium | pm-backend | Coverage gap [mvp]: Pinned Announcements — verify and finish to done. Gaps: sprint-status ready-for-dev — not marked done; No mobile pinned- | dispatcher-tier1-refill 2026-06-25 (coverage partial-story g |
| done | medium | pm-frontend | Coverage gap [mvp]: API Client Integration for Core Features — verify and finish to done. Gaps: coverage.json (last_checked 2026-05-23) stil | dispatcher-tier1-refill 2026-06-25 (coverage partial-story g |
| done | medium | pm-frontend | Mobile RN production screens (Buildings/Meters/Leases/PersonMonths/Notifications/Threads/Forms) render hardcoded MOCK_* arrays — no API wiri | dispatcher-tier1-refill 2026-06-22 (backlog.json promote) |  |
| done | low | pm-backend | Churn hotspot: 2940 lines changed in backend/crates/db/src/repositories/document.rs (window 2026-06-10 03:05Z→18:30Z) | dispatcher-tier1-refill 2026-06-22 (backlog.json promote) |  |
| done | low | pm-backend | Churn hotspot: backend/crates/db/src/repositories/sensor.rs (+248/-86 in PR #1321/#1322 PAP-151 re-land + fmt) | dispatcher-tier1-refill 2026-06-22 (backlog.json promote) |  |
| done | low | pm-backend | Churn hotspot: 2856 lines changed in backend/crates/db/src/repositories/subscription.rs (window 2026-06-10 03:05Z→18:30Z) | dispatcher-tier1-refill 2026-06-22 (backlog.json promote) |  |
| done | low | pm-backend | Churn hotspot: 1021 lines changed in backend/servers/api-server/src/routes/emergency.rs (window 2026 | dispatcher-tier1-refill 2026-06-22 (backlog.json promote) |  |
| done | low | pm-backend | Churn hotspot: 709 lines changed in backend/servers/api-server/src/routes/enhanced_tenant_screening. | dispatcher-tier1-refill 2026-06-22 (backlog.json promote) |  |
| done | low | pm-backend | Churn hotspot: backend/servers/api-server/src/routes/forms.rs touched 2x since 2026-06-12 (window 2026-06-12→2026-06-13) | dispatcher-tier1-refill 2026-06-22 (backlog.json promote) |  |
| done | low | pm-backend | Churn hotspot: backend/servers/api-server/src/routes/iot.rs (+278/-403 in PR #1321/#1322 PAP-151 re-land + fmt) | dispatcher-tier1-refill 2026-06-22 (backlog.json promote) |  |
| done | low | pm-backend | Churn hotspot: backend/servers/api-server/src/routes/reserve_funds.rs (+228/-255 in PR #1321 PAP-151 re-land) | dispatcher-tier1-refill 2026-06-22 (backlog.json promote) |  |
| done | low | pm-backend | Churn hotspot: 929 lines changed in backend/servers/api-server/src/routes/vendors.rs (window 2026-06 | dispatcher-tier1-refill 2026-06-22 (backlog.json promote) |  |
| done | low | pm-frontend | Report execution-history: verify+harden presigned-download + retry end-to-end (coverage 81-2) | dispatcher-tier1-refill 2026-06-23 (coverage partial-story g |
| dropped | medium | pm-backend | Report schedule editing round-trips cron through overloaded `time` field — add dedicated cron_expression column (migration+repo+route) (cove | dispatcher-tier1-refill 2026-06-23 (coverage partial-story g |
| dropped | low | pm-backend | Churn hotspot: backend/servers/api-server/src/routes/api_ecosystem.rs (+106/−27 in PR #1293 PAP-171; second touch in 24h) | dispatcher-tier1-refill 2026-06-22 (backlog.json promote) |  |
| dropped | low | pm-backend | booking_oauth_csrf_tests.rs hotspot — 484-line NEW test file (PR #1393 #1424 OAuth CSRF coverage) | dispatcher-tier1-refill 2026-06-22 (backlog.json promote) |  |
| dropped | low | pm-backend | booking_oauth_routes_tests.rs hotspot — 381-line NEW test file (PR #1393 OAuth routes coverage) | dispatcher-tier1-refill 2026-06-22 (backlog.json promote) |  |
| dropped | low | pm-backend | Churn hotspot: backend/servers/api-server/tests/reserve_funds_cross_org_idor_tests.rs touched 2x since 2026-06-12 (window 2026-06-12→2026-06 | dispatcher-tier1-refill 2026-06-22 (backlog.json promote) |  |
| dropped | low | pm-frontend | Churn hotspot: 94 lines in frontend/apps/mobile/app.config.ts (PR #1383 gap-85-2) | dispatcher-tier1-refill 2026-06-22 (backlog.json promote) |  |
| dropped | low | pm-devops | PR #1274 (cargo-minor-patch group, /backend, 9 updates) closed unmerged — superseded by #1313 after auto-rebase fix landed | dispatcher-tier1-refill 2026-06-22 (backlog.json promote) |  |
| dropped | low | pm-devops | PR #1425 (GH #1377 document presigned-URL tests) closed unmerged — superseded by merged #1394 | dispatcher-tier1-refill 2026-06-22 (backlog.json promote) |  |
| dropped | low | pm-devops | PR #1179 (docs(epics) catalog backfill for 37 mounted-but-undocumented backend modules) — stalled at 7d, no reviewDecision | dispatcher-tier1-refill 2026-06-22 (backlog.json promote) |  |
| dropped | low | pm-frontend | Stalled review: PR #988 (Epic: reusable Playwright E2E framework + sitemap FlowRunner) open 10d, no reviewDecision | dispatcher-tier1-refill 2026-06-22 (backlog.json promote) |  |
| dropped | low | pm-tech-lead | Issue #1380 (no labels, OPEN): Dispatcher stale gap-scan buffer + Tier-2 escalation endpoint misconfigured | dispatcher-tier1-refill 2026-06-22 (backlog.json promote) |  |
