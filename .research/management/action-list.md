# PPT Action List

_Generated: 2026-06-18 — 24 open · 31 done · 59 total._

## Open

| Priority | Owner | Action | Source |
|---|---|---|---|
| high | pm-backend | Resolve story 7a-2 folder-organization (PR #1316) — promote or close; document.rs is hottest churn file (8 PRs this wave), rebase debt growi | pm-analysis 2026-06-18 |
| high | pm-devops | Make backend `test` job required check on dev branch — close issue #1538 | pm-analysis 2026-06-18 |
| high | pm-qa | Audit 8-PR churn cluster on rental.rs and document.rs for missing integration-test coverage; file follow-up issues for any gap | pm-analysis 2026-06-18 |
| high | pm-scrum-master | Triage 6 new untriaged issues #1520, #1521, #1522, #1532, #1533, #1538 — assign labels, owner, severity, sprint/backlog decision | pm-analysis 2026-06-18 |
| high | pm-security | Add 'backend / test' (cargo test) as a required status check on dev branch protection — currently only generic 'check' context (app_id 15368 | pm-analysis 2026-06-18 |
| high | pm-security | Audit document.rs route handlers for use of deprecated legacy non-RLS methods after the 8-PR churn cluster; add clippy -D deprecated CI gate | pm-analysis 2026-06-18 |
| high | pm-security | Close issue #480: migrate WebSocket auth from JWT-in-query to short-lived one-time token (OTT) exchange so JWT is never in URL/access logs | pm-analysis 2026-06-18 |
| high | pm-security | Close issue #481: add integration test asserting revoked OAuth refresh tokens are rejected; verify revoked_at IS NULL guard still present in | pm-analysis 2026-06-18 |
| medium | pm-backend | Coverage 83-1 (AC): implement the Airbnb OAuth token-exchange route (authorization-code → access/refresh token) for the integrations surface | dispatcher-tier1-refill 2026-06-11 (cove |
| medium | pm-frontend | Coverage 6-3 (re-spec of dropped feat-announcement-comments-discussion-web-ui-frontend; prior attempt failed sandbox-no-branch, not scope):  | dispatcher-tier1-refill 2026-06-18 (re-s |
| medium | pm-frontend | Coverage 6-2 (re-spec of dropped feat-announcement-viewing-acknowled-web-viewing-ack-ui-backend; prior attempt failed sandbox-no-branch, not | dispatcher-tier1-refill 2026-06-18 (re-s |
| medium | pm-frontend | Coverage 79-1 (gap): fully wire AnnouncementsPage and FaultsPage in ppt-web to the generated @ppt/api-client TanStack Query hooks (list/deta | dispatcher-tier1-refill 2026-06-18 (cove |
| medium | pm-backend | Coverage 83-2 (gap): implement OTA (OpenTravel) XML parsing + generation for the Booking.com integration — request/response models for avail | dispatcher-tier1-refill 2026-06-18 (cove |
| medium | pm-backend | Coverage 83-2 AC-5 (gap): complete the rate/availability push flow to Booking.com — full outbound sync of price + availability changes throu | dispatcher-tier1-refill 2026-06-18 (cove |
| medium | pm-frontend | Coverage 7a-4 (re-spec of dropped feat-document-download-preview-mobile-slice; prior attempt failed sandbox-no-branch, not scope): implement | dispatcher-tier1-refill 2026-06-18 (re-s |
| medium | pm-backend | Coverage 84-2 (gap): add a webhook idempotency guard for the e-signature email integration so a terminal signing state (completed/declined/v | dispatcher-tier1-refill 2026-06-18 (cove |
| medium | pm-frontend | Coverage 7a-2 (gap): implement the mobile (React Native) slice of Folder Organization for documents — create/rename/move folders + folder tr | dispatcher-tier1-refill 2026-06-18 (cove |
| medium | pm-backend | Coverage 8a-3 (gap): wire mobile OS push delivery (FCM for Android, APNs for iOS) into the existing notification-preference sync. Backend de | dispatcher-tier1-refill 2026-06-18 (cove |
| medium | pm-frontend | no epic-82 commits in git log (82-1-swiftui-project-setup SwiftUI Project Setup) | gap-scan 2026-06-12 (buffer-low refill) |
| medium | pm-backend | Advance epic-10a (OAuth Provider Foundation): close/rebase stale draft #1197 (~7d); resolve gate issues #481/#482; assign owner for 10a-1 | pm-analysis 2026-06-18 |
| medium | pm-qa | Audit allowed_pet_types enum decode paths + add unit test for unknown variants (#1363, #1366) | pm-analysis 2026-06-15 |
| medium | pm-security | Close issue #486: replace direct getToken() calls in announcement/fault frontend modules with axios-interceptor path to prevent auth bypass  | pm-analysis 2026-06-18 |
| medium | pm-security | Re-audit five IDOR backlog signals (api-core-equipment-idor, api-core-voice-device-idor, api-core-llm-doc-idor, reality-server-inquiry-read- | pm-analysis 2026-06-18 |
| low | pm-frontend | Coverage 85-2 (gap): generate app icons for all required sizes (Android mipmap densities + iOS AppIcon set) for the React Native Property Ma | dispatcher-tier1-refill 2026-06-18 (cove |
