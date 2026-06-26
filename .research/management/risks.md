# PPT Risks Register

_Generated: 2026-06-26T12:18:41Z_

Items: 52 (open=47)

| Status | P×I | Owner | Risk |
|---|---|---|---|
| open | high×high | pm-backend | Cross-tenant IDOR cluster in ai.rs equipment endpoints: update_equipment (ai.rs:1113), delete_equipment (ai.rs:1131), and update_maintenance (ai.rs:1192) all bind `_principal: RequestPrincipal` and di |
| open | high×high | pm-devops | PR #1426 merged despite breaking `dev` backend compile (issue #1437) - ALL backend CI gates now broken on `dev` until #1435/#1436 lands. Indicates the PR-merge gate didn't run (or was bypassed) and th |
| open | high×high | pm-qa | Issue #480 (JWT access token in WebSocket query-param access logs) is high-severity and open; story 8a-3 has a WS dependency — shipping it leaks tokens via logs. |
| open | high×high | pm-qa | Issue #481 (OAuth refresh-token revocation bypass) is high-severity and open, yet stories 10a-1/10a-3 target this sprint; merging OAuth stories ships a broken security contract. |
| open | high×high | pm-scrum-master | Story 7a-1 not promotable: POST /api/v1/documents/upload backend handler absent; mobile upload UI (PR #447) calls a missing route in production. |
| open | high×high | pm-security | update_schedule (report_schedule.rs) missing RBAC (#614) and missing tenant/org scope (#624): an authenticated user can mutate another tenant's report schedule and/or mutate schedules without the requ |
| open | high×medium | pm-backend | 10B handler stubs (10b-4/5/6/7) are mounted routes returning success with no business logic — callers get 200/204 for operations that never happened (silent data loss / incorrect UI state) |
| open | high×medium | pm-devops | Mobile release pipeline has no merged path: EAS Android/iOS build workflows exist only in draft PRs (#566 cluster) with broken @v6 action pins (checkout/setup-node/pnpm-action-setup @v6 do not exist); |
| open | medium×high | pm-devops | security-test-gate.yml may be advisory only (not a required status check): PR #497 shipped a security IDOR fix with zero tests despite the gate existing, suggesting it does not block merges. |
| open | medium×high | pm-devops | `security-test-gate.yml` workflow file is present but enforcement-vs-advisory on `dev` branch protection remains unconfirmed since 2026-05-27. If still advisory, security-labelled PRs can merge with n |
| open | high×medium | pm-frontend | Post-merge review opened follow-up issues #480-#487 covering test gaps + minor security/UX follow-ups on the heavy frontend delivery (messaging realtime, document share-flow, OAuth UI, MFA). If these  |
| open | high×medium | pm-backend | Airbnb at-least-once webhook (webhook.rs:1028) enqueues duplicate SYNC_EXTERNAL jobs on event bursts — redundant sync load and possible reservation-state races. |
| open | high×medium | pm-backend | Redis push-fanout queue (push_fanout.rs:621) is never drained (BLPOP deferred) — jobs enqueued to push_fanout_queue are silently dropped while Epic 8A notification stories are marked done. |
| open | medium×high | pm-backend | sqlx 0.8→0.9 major bump (Dependabot PR #666) affects every workspace query; merging without a coordinated upgrade pass may break compile-time query! checks or the migrate API and block all backend CI. |
| open | medium×high | pm-security | Booking.com OAuth/credential connect flow lacks secure replacement on re-connect (#1362, #1374) — old credentials can linger and be used post-rotation. |
| open | high×medium | pm-devops | Dispatcher meta-issue #1380: stale gap-scan buffer feeds no-op claims + Tier-2 escalation endpoint misconfigured — wastes implementer cycles claiming gap stories already shipped. |
| open | high×medium | pm-scrum-master | 18 follow-up issues (#1360–#1377) from the post-merge review of 2026-06-14 merges remain untriaged; backlog grows faster than burn-down without owner assignment. |
| open | medium×high | pm-qa | PR #497 inquiry IDOR fix shipped with zero regression tests; mark_inquiry_read_for_realtor does an ownership EXISTS check then a separate UPDATE — a future refactor could silently reintroduce the IDOR |
| open | medium×high | pm-backend | record_payment handler does a check-then-insert without serializable isolation or unique-constraint guard (#1361); concurrent retries can double-write a payment. |
| open | high×medium | pm-scrum-master | 10-day cursor lag (96 PRs) means sprint-status and coverage.json may be materially stale |
| open | medium×high | pm-security | PR #725 (ai-maintenance IDOR fix) sits at verdict=changes (B1-session-IDOR, B2-sentiment-IDOR, B3-missing-test); the maintenance/chat-session/sentiment IDOR vector stays live until merged. |
| open | high×medium | pm-scrum-master | App.tsx is the top churn hotspot; concurrent edits across #503, #500, and 80-3 mediation wiring risk triple-conflict rebases. |
| open | medium×high | pm-scrum-master | Issue #481 (reusable revoked refresh tokens) is a live RFC 9700 violation if api-server is exposed to production |
| open | high×medium | pm-frontend | 10 of 22 remaining partial/not-started stories are mobile (7a-2/7a-4, 8a-3 push, epic-82 SwiftUI); unreviewed gap-82 drafts let the mobile slice fall further behind. |
| open | medium×high | pm-scrum-master | Phase 1.5 found .expect()/.unwrap() panic paths in reality-server (password-reset rng, http-client, defensive unwrap) — production crash risk |
| open | high×medium | pm-scrum-master | PR #1798 (3672-file emergency.rs refactor) introduces high merge-conflict surface area; in-flight branches touching api-server routes will need rebase |
| open | medium×high | pm-security | PR #435 merged with deferred security findings (#438/#439): P1-05 SSRF, P0-12 cookie scope, P1-04 Debug-format audit-hash domain. SSRF (#450) and ProtectedRoute fail-open (#459) have since landed; rem |
| open | high×medium | pm-scrum-master | sprint-status.yaml is materially stale (10b-3/10b-4/10b-6 still ready-for-dev despite done evidence); risks duplicated work or mis-reporting progress. |
| open | medium×high | pm-security | Issue #480 WS JWT in query param: suppression comment is human-only; future refactor could log params.token |
| open | medium×high | pm-security | Issue #481 marked open in sprint-status but fix appears present in code — stale tracker may unnecessarily block 10a-1/10a-3 or cause accidental revert |
| open | high×medium | pm-security | Issue #483 (voice device IDOR — list-commands existence leak) has no story gate and no sprint pressure |
| open | medium×high | pm-security | OAuth provider stories 10a-1/10a-2/10a-3 shipped with no introspection/refresh-rotation/PKCE security tests; a refactor could silently reintroduce revoked-token acceptance or replay. |
| open | medium×high | pm-security | ProtectedRoute role check depends on user.role possibly derived from tenants[0] — multi-tenant user could get elevated permissions on org B routes |
| open | medium×high | pm-tech-lead | Three route monoliths on hot paths this run: ai.rs (3142 lines), platform_admin.rs (2762), announcements.rs (2722). ai.rs already harbors the cross-tenant IDOR cluster above — exactly the failure mode |
| open | medium×medium | pm-backend | Epic 81 frontend calls backend report-schedule endpoints (/schedules/{id}/pause\|resume, /executions) that do not exist — 404 in production |
| open | medium×medium | pm-data | FaultStatusCount / FaultByStatusTable surfaced by the new Support Data admin page (#635) defines a fault-by-status metric for support staff. The same fault counting also exists in owner_analytics.rs a |
| open | medium×medium | pm-data | The Support Data page (#635) lets support/admin staff read cross-tenant user diagnostics (memberships, sessions with IP, activity log) gated only by the audit_read capability. There is no analytics/au |
| open | medium×medium | pm-devops | App.tsx router-file churn + 6 concurrent dispatcher drafts (#563-#568) risk repeated triple-conflict rebases, stalling the merge queue. |
| open | medium×medium | pm-devops | `eas-build-android.yml` and `eas-build-ios.yml` now exist in `.github/workflows/` (cleared since 2026-05-27 from draft PRs), but green status on `dev` is unverified. If action pins / eas-cli installat |
| open | medium×medium | pm-devops | Pre-push fmt/clippy gate (#1431) merged but is local hook only - does not enforce on CI side and does not run cargo check. Cannot prevent a contributor with hooks disabled from re-landing a #1426-clas |
| open | medium×medium | pm-frontend | Epic 6 announcement web UI (viewing/ack 6-2, comments 6-3, pin 6-4) is split across three draft PRs (#474/#475/#479) that have not merged. Backend + notification pipeline (PR #463) and WebSocket sync  |
| open | medium×medium | pm-backend | e-signature webhook status writes have no idempotency guard; a provider re-delivering a completed/voided event can overwrite a terminal state. |
| open | medium×medium | pm-qa | Cron validator drift (#1368) could silently reintroduce regression #616 (Epic 81 promotion blocker) — current tests don't pin the validator surface. |
| open | medium×medium | pm-scrum-master | Epic 7A (Document Management) has 4 of 5 stories not done; cascade dependency 7a-3 not started — sprint under-delivery |
| open | medium×medium | pm-security | P1-04 residual (PR #435): internal type internals may reach audit-trail log lines via Debug ({:?}) formatting. |
| open | low×high | pm-security | reality-server rng .expect() at handlers/users/mod.rs:551 panics on OS entropy failure during password-reset — untracked, no issue filed |
| open | low×medium | pm-tech-lead | OAuth provider (10A) backend complete; admin client-management UI + user-grants UI now shipped (PRs #468/#469/#471). Residual gap is end-to-end integration/security test coverage (PKCE flow, refresh r |
| resolved | medium×high | pm-security | Cookie Path breaking change (#617) from PR #565 session-cookie scope hardening: if mis-scoped it either silently logs users out or leaks the session cookie to unintended paths / breaks the SSO auth-ca |
| resolved | low×high | pm-backend | Two outbound-request sinks (signatures.rs:628 signed_url fetch, integrations.rs:2743 webhook-test POST) issued requests to unvalidated user/provider URLs — SSRF to 169.254.169.254 / internal services. |
| resolved | low×high | pm-scrum-master | Epic 6/8A blocked on un-built Epic 2B notification infra; deferred dispatch + WS sync (8A.2/8A.3) |
| resolved | low×high | pm-security | Voice device endpoints permitted cross-tenant device access (IDOR) — a principal could address another org's voice device by id. |
| resolved | low×medium | pm-frontend | ProtectedRoute.tsx:117 role gate was fail-open: when user.role was falsy the role check was skipped and access granted — would silently no-op the first role-gated route. |
