# PPT Roadmap

_Generated 2026-06-27T12:00:00Z · scan_kind=upkeep · supersedes the 2026-06-23 deep regen with this run's reconciliations._

## State of the project

- **Stories:** 49 scanned across 13 epics — **46 done · 3 partial · 0 not-started**.
- **Partial work by platform:** ppt-web 2 · backend 2
- **Screen coverage:** 2 orphan epics · 29 orphan screens · 4 missing UC links.

**Top 3 gaps:**
1. **Security follow-up cluster** — 47 `from-merged-review` issues #1758-#1854 (incl. IDOR #1791 / OCR PII #1772 / Stripe #1764 / Booking JWT #1787) need landing before Stripe + messaging-N-party + booking surfaces are GA-safe.
2. **Genuinely unfinished slices** — 84-5 pgvector RAG (migration only); 80-3 mediation party-submission endpoints unwired; 80-2 dispute redesign 5-step wizard + i18n; 6-3/6-4 mobile comments/pinned UI (web shipped).
3. **Infra blockers** — issue #1014 archive-push MCP size limit; issue #1680 dispatcher cron environment can't run core pipeline; repeated migration-version collisions (#1724/#1755/#1757) lacking pre-commit guard.

## Ranked plan

### mvp
- [high] Known UX nit (per sprint-status): share panel uses window.confirm and lacks UUID validation on user-ID input — non-blocking polish item (7a-5-document-sharing Document Sharing) — owner: pm-security — score: 9
- [high] Redesigned 5-step wizard (redesignStatus: in-progress) not shipped — only single-page form is live (80-2-dispute-filing-flow Dispute Filing Flow) — owner: pm-frontend — score: 8
- [high] Localized `disputes.draft*` i18n keys for 6 message bundles are a documented fast-follow (currently English-default t(key, defaultValue)) (80-2-dispute-filing-flow Dispute Filing Flow) — owner: pm-frontend — score: 8
- [high] Party submissions endpoints unwired (apiStatus stays partial per dispute-detail screen-map) (80-3-mediation-resolution Mediation and Resolution) — owner: pm-frontend — score: 8
- [high] Mediation-resolution flow not promoted to done in sprint-status — story remains partial pending submissions integration (80-3-mediation-resolution Mediation and Resolution) — owner: pm-frontend — score: 8
- [high] no screen-map (orphan epic) (10b-6-user-onboarding-tour User Onboarding Tour) — owner: pm-frontend — score: 7
- [high] no screen-map (orphan epic) (85-1-environment-variables Environment Variable Setup) — owner: pm-frontend — score: 7
- [high] AC-5 automated build scripts not found under mobile-native (no build*.sh located) — verify CLI build automation exists (85-2-build-configuration Build Configuration by Environment) — owner: pm-frontend — score: 7
- [high] Story markdown still Status: pending (stale vs. merged code) (85-2-build-configuration Build Configuration by Environment) — owner: pm-frontend — score: 7
- [medium] no screen-map (orphan epic) (10b-1-organization-management-dashboard Organization Management Dashboard) — owner: pm-frontend — score: 6
- [medium] no screen-map (orphan epic) (10b-2-feature-flag-management Feature Flag Management) — owner: pm-frontend — score: 6
- [medium] no screen-map (orphan epic) (79-3-error-handling-toasts Error Handling and Toast Notifications) — owner: pm-frontend — score: 6
- [medium] No sprint-status entry to corroborate; trigger-specific unit/integration test not confirmed (84-4-notification-triggers Notification Trigger System) — owner: pm-backend — score: 6
- [medium] no screen-map (orphan epic) (84-4-notification-triggers Notification Trigger System) — owner: pm-backend — score: 6
- [medium] no screen-map (orphan epic) (8a-1-channel-level-notification-toggles Channel-Level Notification Toggles) — owner: pm-frontend — score: 6
- [medium] no screen-map (orphan epic) (8a-2-critical-notification-override Critical Notification Override) — owner: pm-frontend — score: 6

### phase2
- [medium] sprint-status.yaml has no 81-1 key (only epic-80 keys present); classification driven by code+screen+PR consensus (81-1-report-schedule-editing Report Schedule Editing) — owner: pm-frontend — score: 5
- [medium] Screen apiStatus: partial and several gap-81-1 follow-up PRs indicate minor edge-case/RBAC fixes were needed post-delivery (81-1-report-schedule-editing Report Schedule Editing) — owner: pm-frontend — score: 5
- [medium] sprint-status.yaml has no 81-2 key (only epic-80 keys present); classification driven by code+screen+PR consensus (81-2-report-execution-history Report Execution History) — owner: pm-frontend — score: 5
- [medium] Screen apiStatus: partial — download/retry flow had a test-gap follow-up (d1785e5fb) (81-2-report-execution-history Report Execution History) — owner: pm-frontend — score: 5

### phase3
- [medium] RAG retrieval/query service (embedding generation + similarity search) not implemented — migration only (84-5-pgvector-rag pgvector RAG Migration) — owner: pm-backend — score: 6
- [medium] Vector path is conditional/optional (JSONB fallback) rather than a hard pgvector dependency (84-5-pgvector-rag pgvector RAG Migration) — owner: pm-backend — score: 6

### phase4
- [low] no screen-map (orphan epic) (82-1-swiftui-project-setup SwiftUI Project Setup) — owner: pm-frontend — score: 4
- [low] Inline inquiry thread/conversation view noted as deferred in KMP agent log (inquiry-reply API + scheduling backend); iOS SendInquiryView present so iOS slice co (82-5-inquiries-account Inquiries and Account) — owner: pm-frontend — score: 4
- [low] Booking.com uses credential-based connect + OTA-XML rather than OAuth proper (matches Booking.com's real API model, not a true OAuth handshake like Airbnb) (83-2-booking-integration Booking.com OAuth and Sync) — owner: pm-backend — score: 3
- [low] Implemented and labeled under Epic 105 'Portal Syndication / Story 105.4' in code; functionally covers this story's portal-webhook scope but story-id alignment  (83-3-portal-webhooks Real Estate Portal Webhooks) — owner: pm-backend — score: 3

#### Screen-map drift
- [medium] Backfill frontmatter `epics:` across ~120 screen-maps — restores epic→screen linkage
- [medium] Decide screen-map (or no-UI marker) for orphan epic epic-85
- [medium] Decide screen-map (or no-UI marker) for orphan epic epic-8a
- [medium] Link UC UC-10 to a screen-map
- [medium] Link UC UC-29 to a screen-map
- [medium] Link UC UC-33 to a screen-map
- [medium] Link UC UC-40 to a screen-map

---
Buffer: 20/36 open · 0 candidates ranked but unqueued
