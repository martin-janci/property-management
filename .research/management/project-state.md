# Project state

<sub>Generated: 2026-06-27T14:25:00Z by ppt-project-management skill (rotating: pm-security)</sub>

## Executive summary

Sprint 'Epic 6, 7A, 8A & 10A' is materially ahead of its yaml labels: coverage.json (2026-06-23) confirms Epic 6 all 6 stories done, Epic 7A all 5 stories done, Epic 8A done, Epic 10B done — sprint-status.yaml is stale and needs reconciliation. The 11-day catch-up window (PRs #1567-#1856, 96 merges) delivered major work across Epics 3, 4, 6, 11, 16, 18, and messaging, but three high-severity bugs discovered in the saved-search-alerts drainer (Epic 16) and six open test-hardening gates (#480-#485, #487) are blocking story promotion for 10A and Epic 80 partials.

## Sprint progress

- Sprint: Epic 6, 7A, 8A & 10A - Announcements, Documents, Notifications & OAuth
- Epics done: 3/6

## Shipped since last run

- PR #1726 — Epic 11 Stripe Checkout integration
- PR #1709 — Epic 11 payment reminders + auto-overdue
- PR #1717 — Epic 11 income/balance/cash-flow reports + PDF/xlsx export
- PR #1725 — Epic 11 financial reports screen
- PR #1705 — Epic 4 fault lifecycle notifications
- PR #1704 — Epic 4 fault reports/analytics page
- PR #1715 — Epic 4 offline queue mobile
- PR #1847 — Epic 16 alert_frequency cadence
- PR #1849 — Epic 16 email/push transport drainer
- PR #1850 — Epic 16 org-scoped favorite alert worker
- PR #1689 — UC-05.8 N-party group threads
- PR #1702 — UC-05.9 attachment S3 presigned upload
- PR #1750 — Epic 18 story 18.2 guest ID OCR
- 6-1-announcement-creation-targeting: verified done (2026-06-25)
- 6-2-announcement-viewing-acknowledgment: verified done (2026-06-25)
- 6-3-announcement-comments-discussion: verified done (2026-06-25)
- 6-4-pinned-announcements: verified done (2026-06-25)
- 6-5-direct-messaging: verified done (2026-06-25)
- 10b-7-contextual-help-documentation: reconciled done (2026-06-25)

## What's next (top 5)

- **[high]** Reconcile sprint-status.yaml: promote Epic 6 (all 6 stories), Epic 7A (all 5 stories), Epic 10B (all 7 stories) to done and flip epic statuses to done to match coverage.json 2026-06-23 ground truth — owner: none
- **[high]** Fix Epic 16 drainer HIGH bugs (row reservation + transactional enqueue) before the next alert worker deployment; assign rust-backend to author the patch and open a PR against dev — owner: pm-backend
- **[high]** Close or defer test-hardening issues #481, #487 (OAuth backend) so 10a-1 and 10a-3 can be promoted; assign rust-backend to write refresh-token revocation regression test and MFA rate-limit test — owner: pm-backend
- **[medium]** Close test-hardening issue #482 (ProtectedRoute multi-tenant role fallback) so 10a-2 can be promoted; assign react-web to add ProtectedRoute unit tests covering multi-tenant users — owner: pm-frontend
- **[medium]** Wire party submissions endpoints in ppt-web dispute-detail to unblock 80-3-mediation-resolution; update dispute-detail screen-map apiStatus from partial to complete after merge — owner: pm-frontend

## Blockers

- **10a-1-oauth-authorization-server, 10a-3-oauth-token-management** — Test-hardening gate: issue #481 (OAuth refresh-token revocation bypass — revoked tokens reusable, breaks RFC 9700) and #487 (MFA rate-limit coverage missing) are open; stories must not move to done until resolved (owner: pm-backend)
- **10a-2-oauth-client-registration** — Test-hardening gate: issue #482 (ProtectedRoute role fallback uses tenants[0] — wrong for multi-tenant users, no unit tests) is open (owner: pm-frontend)
- **80-2-dispute-filing-flow** — 5-step redesigned wizard not shipped; i18n keys for draft messages missing from 6 locale bundles (English-default fallback only) (owner: pm-frontend)
- **80-3-mediation-resolution** — Party submissions endpoints unwired in frontend — dispute-detail screen apiStatus stays partial (owner: pm-frontend)
- **Epic 16 saved-search-alerts drainer** — Phase 1.5 found two HIGH bugs: no row reservation causing duplicate emails/pushes under concurrency; non-transactional enqueue+watermark advance causing duplicates on crash. Plus MEDIUM: no backoff on retries. PRs #1847-#1850 already merged; fixes not yet confirmed landed. (owner: pm-backend)

## Role focus today

- pm-scrum-master, pm-security

## Per-role summaries

### pm-scrum-master

Sprint 'Epic 6, 7A, 8A & 10A' is materially ahead of its yaml labels: coverage.json (2026-06-23) confirms Epic 6 all 6 stories done, Epic 7A all 5 stories done, Epic 8A done, Epic 10B done — sprint-status.yaml is stale and needs reconciliation. The 11-day catch-up window (PRs #1567-#1856, 96 merges) delivered major work across Epics 3, 4, 6, 11, 16, 18, and messaging, but three high-severity bugs discovered in the saved-search-alerts drainer (Epic 16) and six open test-hardening gates (#480-#485, #487) are blocking story promotion for 10A and Epic 80 partials.

### pm-security

Three open security-classified issues from the test-hardening batch (#480, #481, #487) remain unresolved and gate multiple OAuth and notification stories that are in-progress this sprint. The WebSocket JWT-in-query-param surface is confirmed in both frontend (websocket.ts:94) and backend (ws_notifications.rs:83), and refresh-token revocation logic at auth.rs:1126 looks sound, but the DB query used by find_by_token_hash_any_status cannot be verified without the session repository source — leaving issue #481 status ambiguous.
