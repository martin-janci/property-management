# Backlog of vectors

<sub>Last regenerated: 2026-06-05 03:15 UTC by routine</sub>

> **Canonical source:** `backlog.json`. This file is **regenerated** from it
> each run — do not edit by hand. To drop, defer, or re-score a vector, edit
> `backlog.json` and let the next run rebuild this view (or commit both
> together).

Ranked list of improvement / bugfix ideas the daily research routine has
surfaced. Higher-score items go to the top. The manual implementation agent
picks from here.

| Score | Title | Vector | Source | Updated | Status |
|-------|-------|--------|--------|---------|--------|
| 6 | Add regression tests for inquiry mark_as_read cross-tenant IDOR fix (PR #497) | test-gap | PR #497, PR #507, PR #548 | 2026-05-26 | done |
| 3 | Schema drift: runtime SQL errors from non-existent columns in voting/messaging/notification paths | bug | Issue #1008 | 2026-06-03 | open |
| 3 | IDOR: ai.rs LLM-doc handlers publish/list/get any tenant's listing descriptions & photo enhancements unscoped | security | code-review api-core 2026-05-29, ai.rs:2620, ai.rs:2599, ai.rs:2847, PR #879 | 2026-06-01 | ready |
| 3 | IDOR: reality-server realtors mark_inquiry_read flips any realtor's inquiry by ID with no owner scoping | security | issue #519, PR #508, realtors.rs:250, PR #548 | 2026-05-26 | done |
| 3 | IDOR: unlink_voice_device deactivates any device by ID with no owner/org scoping | security | code-review api-core 2026-05-23, ai.rs:3002, PR #461 | 2026-05-25 | done |
| 3 | SSRF: signed-document fetch + webhook-test POST issue outbound requests to unvalidated user-controlled URLs | security | issue #439, signatures.rs:628, integrations.rs:2743, PR #450 | 2026-05-25 | done |
| 3 | IDOR: equipment delete/update + maintenance update mutate any tenant's equipment by ID with no org scoping | security | code-review api-core 2026-05-25, ai.rs:1133, equipment.rs:144 | 2026-05-25 | done |
| 2 | Mobile document-detail wiring (PR #992) shipped without a regression test for the deep-link payload path | test-gap | PR #1082, PR #992 | 2026-06-05 | done |
| 2 | Frontend gap-sweep (PR #990, 34 files across Epics 1/6/7B/9/10B/11/15/17/18) shipped without a regression test | test-gap | PR #1081, PR #990 | 2026-06-05 | done |
| 2 | Risky churn: mobile App.tsx deep-link/doc-detail wiring changing across back-to-back PRs without coverage | bug | PR #1103, PR #962, PR #992 | 2026-06-05 | done |
| 2 | Mobile RN dev-review tail (#943) shipped without test coverage | test-gap | PR #1080, PR #943, issue #767 | 2026-06-05 | done |
| 2 | deploy-server OIDC scope mapping (#939) shipped without unit test for derive_oidc_scopes | test-gap | PR #1106, PR #939 | 2026-06-05 | done |
| 2 | Portal webhook fail-closed fix (PR #874) shipped without a regression test for unverified-signature rejection | test-gap | PR #1052, PR #874 | 2026-06-05 | done |
| 2 | Mobile dev-review batch (PR #918, 5 files under frontend/apps/mobile/src) shipped without a regression test | test-gap | PR #1072, PR #918 | 2026-06-05 | done |
| 2 | Reality-server SSO consumer review fix (PR #921, closes #820) shipped without a regression test | test-gap | PR #1076, PR #921 | 2026-06-05 | done |
| 2 | CI branch-protection + auto-rebase workflow change (PR #923) shipped without an integration test | test-gap | PR #1057, PR #923 | 2026-06-05 | done |
| 2 | Screen-map drift: PR #839 modified ppt-web App.tsx (FileDisputePageRoute) without a docs/screens/ppt update | test-gap | PR #1056, PR #839 | 2026-06-05 | done |
| 2 | Booking push availability/rates endpoints add batch-cap + non-negative guards with no regression test | test-gap | PR #1068, PR #607, issue #572 | 2026-06-05 | done |
| 2 | Integration marketplace install/OAuth flows are placeholders — wire backend handlers + UI navigation | dx | PR #1105, PR #282, PR #328, commit 254f01d, commit c97781a | 2026-06-05 | done |
| 2 | Screen-map drift: PR #1100 modified ppt-web App.tsx (FileDisputePageRoute extraction) without screen-doc update | test-gap | PR #1100 | 2026-06-05 | open |
| 2 | Screen-map drift: PR #1085 modified reality-web listing detail metadata + page without screen-doc update | test-gap | PR #1085 | 2026-06-05 | open |
| 2 | ppt-web status/auth components hardcode English in an otherwise i18n'd app | refactor | code-review ppt-web-ui 2026-05-24, rotating-expert-review, PR #549, PR #1046 | 2026-06-04 | done |
| 2 | Screen-map drift: PR #1033 wired error/retry into AnnouncementsPage+FaultsPage via App.tsx without a docs/screens/ppt update | test-gap | PR #1033 | 2026-06-04 | open |
| 2 | Airbnb webhook at-least-once delivery enqueues duplicate SYNC_EXTERNAL jobs | bug | PR #538, webhook.rs:1028, PR #841, PR #1030 | 2026-06-03 | done |
| 2 | Reality-web InviteRealtorModal swallows invite-mutation failure with no error UI | bug | code-review reality-web 2026-05-28, rotating-expert-review, PR #1023 | 2026-06-03 | done |
| 2 | MediationWorkspacePage shows empty/unknown state instead of error UI on dispute fetch failure | bug | PR #555, code-review 2026-05-27, PR #1029 | 2026-06-03 | done |
| 2 | DocumentsBrowse MoveFolderDialog cannot pre-select current folder (DocumentSummary lacks folder_id) | dx | PR #623, PR #1031 | 2026-06-03 | done |
| 2 | Mobile VotingScreen double-casts API result across boundary — render-time crash on unexpected shape | bug | code-review mobile-rn 2026-05-27, rotating-expert-review, PR #1028 | 2026-06-03 | done |
| 2 | Reality-server listings pagination clamp (PR #959) shipped without a regression test for limit=-1 | test-gap | PR #959, issue #953 | 2026-06-03 | open |
| 2 | API + SPA security-headers middleware (PR #963) shipped without an assertion test for HSTS/nosniff/CSP | test-gap | PR #963, issue #954, PR #1021 | 2026-06-03 | done |
| 2 | Risky churn: api-server main.rs security-headers wiring shipped without a middleware smoke test | bug | PR #963 | 2026-06-03 | open |
| 2 | api-server main.rs vs lib.rs::create_router diverge silently (5 routes unreachable in prod, no test asserts parity) | test-gap | PR #866, issue #867, issue #836, PR #870 | 2026-06-01 | done |
| 2 | Screen-map drift: PR #922 modified ppt-web App.tsx (dev-review rounds 1-5 fixes) without a docs/screens/ppt update | test-gap | PR #922 | 2026-06-01 | open |
| 2 | ReportSchedule.update_schedule stores cron in `time` workaround; documented UPDATE never runs (missing cron_expression column) | bug | PR #611, issue #616, PR #643, PR #815 | 2026-05-30 | done |
| 2 | Reduce App.tsx route-aggregator coupling (top churn hotspot, merge-conflict risk) | refactor | PR #474, PR #475, PR #489, PR #511, PR #547, PR #549, PR #555 | 2026-05-27 | open |
| 2 | Screen-map drift: report execution-history route (PR #547) added without a ppt screen doc | test-gap | PR #547, frontend/apps/ppt-web/src/routes/lazyRoutes.tsx, PR #623 | 2026-05-27 | done |
| 2 | announcements.rs churn-hot — 2,722 lines this run (Epic 2B + Epic 6 work) | refactor | git log origin/dev since 2026-05-24, PR #504, PR #505, PR #548 | 2026-05-26 | open |
| 2 | Dispute state machine (PR #506) shipped with no tests + no org predicate on update_status | test-gap | PR #506, issue #520, PR #514, PR #548 | 2026-05-26 | done |
| 2 | PushFanoutWorker BLPOP queue-drain deferred — Redis path is a logging no-op | dx | PR #515, push_fanout.rs:621 | 2026-05-26 | open |
| 2 | integrations.rs churn-hot — 12,977 lines over 14d, candidate for module split | refactor | git log origin/main since 2026-05-06, git log origin/dev since 2026-05-20, PR #456 | 2026-05-25 | done |
| 2 | organizations.rs churn-hot — 12,060 lines over 14d (multitenancy + admin) | refactor | git log origin/main since 2026-05-06, git log origin/dev since 2026-05-20, PR #456 | 2026-05-25 | done |
| 2 | documents.rs churn-hot — 10,659 lines over 14d | refactor | git log origin/main since 2026-05-06, git log origin/dev since 2026-05-20, PR #456 | 2026-05-25 | done |
| 2 | IDOR: reality-server mark_as_read flips any realtor's inquiry by ID with no owner scoping | security | code-review reality-server 2026-05-23, inquiries.rs:554 | 2026-05-25 | done |
| 2 | Latent fail-open: ProtectedRoute role check is skipped when user.role is falsy | security | code-review ppt-web-ui 2026-05-24, ProtectedRoute.tsx:117, PR #459 | 2026-05-25 | done |
| 2 | Screen-map drift: PR #464 wired a neighbors route in ppt-web without a docs/screens/ppt entry | test-gap | PR #464 | 2026-05-25 | done |
| 2 | Screen-map drift: PR #460 touched reality-web listing page without a docs/screens/reality update | test-gap | PR #460 | 2026-05-25 | closed |
| 2 | ai.rs (3,134 LOC) — explicit module-split into routes/ai/{sessions,equipment,workflows,voice,llm,mod}.rs | refactor | pm-tech-lead analysis 2026-05-25, security-voice-device-idor (PR #461), security-equipment-idor | 2026-05-25 | open |
| 2 | platform_admin.rs (2,762 LOC) — explicit module-split into routes/platform_admin/{tenants,features,billing,audit,mod}.rs | refactor | pm-tech-lead analysis 2026-05-25 | 2026-05-25 | open |
| 2 | announcements.rs (2,722 LOC) — explicit module-split into routes/announcements/{crud,targeting,delivery,reactions,mod}.rs | refactor | pm-tech-lead analysis 2026-05-25 | 2026-05-25 | open |
| 2 | Dead/duplicate handler modules: AuthHandler & BuildingHandler unused, routes reimplement inline | refactor | code-review api-handlers 2026-05-23, PR #437 | 2026-05-24 | done |
| 2 | Complete RLS migration in 31 remaining handlers (voting, market_pricing, faults, notif_prefs, reports) | security | issue #160, PR #420, PR #421 | 2026-05-23 | done |
| 1 | Reality-web listing generateMetadata can throw during SSR on malformed 200 body | bug | PR #1085, code-review reality-web 2026-05-28, rotating-expert-review | 2026-06-05 | done |
| 1 | Mobile VotingScreen hardcodes en-US in toLocaleDateString — vote dates never localize | bug | PR #1083, code-review mobile-rn 2026-05-27, rotating-expert-review | 2026-06-05 | done |
| 1 | Triage: dispatcher incident — assignments-archive.json corrupted to 1/196 rows on dev branch (#1061) | triage | Issue #1061 | 2026-06-05 | open |
| 1 | Churn hotspot: AnnouncementsScreen.test.ts — 4 PRs this run, instability proxy | refactor | PR #1101, PR #1077, PR #1083, PR #1098 | 2026-06-05 | open |
| 1 | Churn hotspot: AnnouncementsScreen.tsx — 4 PRs this run, instability proxy | refactor | PR #1101, PR #1077, PR #1083, PR #1098 | 2026-06-05 | open |
| 1 | Churn hotspot: DocumentsScreen.tsx — 3 PRs this run | refactor | PR #1101, PR #1081, PR #1082 | 2026-06-05 | open |
| 1 | ppt-web e2e auth-refresh.spec.ts added (+252 lines, story 79-2 token-refresh coverage) | refactor | PR #1047 | 2026-06-04 | open |
| 1 | api-server esignature_webhook_idempotency_tests.rs added (+228 lines, terminal-state regression) | refactor | PR #1034 | 2026-06-04 | open |
| 1 | ppt-web EvidenceUploader.test.tsx added (+202 lines, dispute-filing AC-2 regression) | refactor | PR #1048 | 2026-06-04 | open |
| 1 | PR #908 (fix(security): require PKCE on OAuth authorization-code flow, closes #823) was closed unmerged — verify whether PKCE enforcement still pending | security | PR #908, PR #1025 | 2026-06-03 | done |
| 1 | api-server main.rs touched twice this run (gap-sweep + security headers) — minor churn marker | refactor | PR #989, PR #963 | 2026-06-03 | open |
| 1 | docker/nginx admin-web + ppt-web templates churned twice this run (security headers + redirects) | dx | PR #963, PR #964 | 2026-06-03 | open |
| 1 | frontend/apps/mobile/src/App.tsx churned twice this run (universal links + doc-detail wiring) | refactor | PR #962, PR #992 | 2026-06-03 | open |
| 1 | Issue #952 (no labels, OPEN): [staging] Reality SSO login dead-ends: redirect_uri callback 404s on reality apex | triage | #952 | 2026-06-03 | open |
| 1 | Issue #951 (no labels, OPEN): Deploy blocker: api-server requires ESIGN_TOKEN_SECRET + ESIGN_WEBHOOK_SECRET not injected by deploy-server (staging/prod) | triage | #951 | 2026-06-03 | open |
| 1 | Issue #950 (no labels, OPEN): CI: trigger-deploy 403 marks all dev image builds red and blocks staging auto-deploy | triage | #950 | 2026-06-03 | open |
| 1 | Dispatcher action-list.json corruption when MCP push falls back from blocked git push | triage | #1014 | 2026-06-03 | open |
| 1 | Issue #920 (no labels, OPEN): Announcement targeting not enforced on read (intra-org disclosure) | triage | #920, #944 | 2026-06-02 | done |
| 1 | Issue #751 (no labels, OPEN): Current dev review: frontend/web/API-client findings | triage | #751, #942 | 2026-06-02 | done |
| 1 | Issue #752 (no labels, OPEN): Current dev review: mobile CI tooling findings | triage | #752, #929 | 2026-06-02 | done |
| 1 | Issue #756 (no labels, OPEN): Current dev review: Epic 10A OAuth Provider | triage | #756, #934 | 2026-06-02 | done |
| 1 | Issue #761 (no labels, OPEN): Current dev review: Epic 84 E-Signature & Leases | triage | #761, #936 | 2026-06-02 | done |
| 1 | Issue #763 (no labels, OPEN): Current dev review: Reality Server & Inquiries | triage | #763, #935 | 2026-06-02 | done |
| 1 | Issue #767 (no labels, OPEN): Current dev review: Mobile RN Property Management app | triage | #767, #943 | 2026-06-02 | done |
| 1 | Issue #768 (no labels, OPEN): Current dev review: Admin-web features (10B) | triage | #768, #930 | 2026-06-02 | done |
| 1 | Stabilize oauth_integration_tests churn — heavy edits across 3 OAuth fix PRs | refactor | PR #930, PR #933 | 2026-06-02 | open |
| 1 | Watch services/oauth.rs churn after introspect/revoke hardening (#933) | refactor | PR #933 | 2026-06-02 | open |
| 1 | Watch routes/oauth.rs churn after audit-log + hardening PRs | refactor | PR #930, PR #933 | 2026-06-02 | open |
| 1 | Issue #750 (no labels, OPEN): Current dev review: backend/API/database findings | triage | #750, PR #922 | 2026-06-01 | done |
| 1 | Issue #753 (no labels, OPEN): Current dev review: Epic 6 Announcements & Communication | triage | #753 | 2026-06-01 | done |
| 1 | Issue #754 (no labels, OPEN): Current dev review: Epic 7A Basic Document Management | triage | #754, PR #914 | 2026-06-01 | done |
| 1 | Issue #757 (no labels, OPEN): Current dev review: Epic 10B Platform Administration | triage | #757 | 2026-06-01 | done |
| 1 | Issue #760 (no labels, OPEN): Current dev review: Epic 79 Disputes & Mediation | triage | #760, PR #915 | 2026-06-01 | done |
| 1 | Issue #762 (no labels, OPEN): Current dev review: Reports & Schedules | triage | #762 | 2026-06-01 | done |
| 1 | Issue #766 (no labels, OPEN): Current dev review: AI & LLM routes | triage | #766, PR #879 | 2026-06-01 | done |
| 1 | Issue #770 (no labels, OPEN): Current dev review: Faults & triage | triage | #770, PR #902 | 2026-06-01 | done |
| 1 | Issue #771 (no labels, OPEN): Current dev review: Research dispatcher & CI automation | triage | #771, PR #923 | 2026-06-01 | done |
| 1 | Issue #772 (no labels, OPEN): Current dev review: Auth core (delta confirmation) | triage | #772 | 2026-06-01 | done |
| 1 | Issue #773 (no labels, OPEN): Current dev review: Leases & rental | triage | #773 | 2026-06-01 | done |
| 1 | Issue #774 (no labels, OPEN): Current dev review: Reality server (broad) | triage | #774, PR #919 | 2026-06-01 | done |
| 1 | Issue #775 (no labels, OPEN): Current dev review: WebSocket realtime | triage | #775, PR #926 | 2026-06-01 | done |
| 1 | Issue #776 (no labels, OPEN): Current dev review: Equipment & audit log | triage | #776 | 2026-06-01 | done |
| 1 | Issue #777 (no labels, OPEN): Current dev review: Compliance & GDPR | triage | #777 | 2026-06-01 | done |
| 1 | Issue #778 (no labels, OPEN): Current dev review: Marketplace, voting, investor portal, impersonation | triage | #778, PR #882 | 2026-06-01 | done |
| 1 | Issue #788 (no labels, OPEN): Dev review rounds 1-5: mobile-native + ppt-web surfaces | triage | #788, PR #922 | 2026-06-01 | done |
| 1 | Issue #790 (no labels, OPEN): Dev review rounds 11-15: vendor, predictive, reality-web, middleware | triage | #790, PR #913 | 2026-06-01 | done |
| 1 | Issue #791 (no labels, OPEN): Dev review rounds 16-20: push, e-sign, portal, webhooks, reserves | triage | #791, PR #924 | 2026-06-01 | done |
| 1 | Issue #846 (no labels, OPEN): Code review: Epics 12+65 — Meters & Energy/ESG (origin/dev) | triage | #846, PR #880 | 2026-06-01 | done |
| 1 | Issue #847 (no labels, OPEN): Code review: Reality-server — Inquiries IDOR (Epics 16–19) (origin/dev) | triage | #847 | 2026-06-01 | done |
| 1 | Issue #848 (no labels, OPEN): Code review: Epics 78+134 — Vendor portal stubs & Predictive maintenance gaps (origin/dev) | triage | #848, PR #913 | 2026-06-01 | done |
| 1 | Issue #850 (no labels, OPEN): Code review: Epics 61+146+42 — Multi-currency, Data residency, Violations (origin/dev) | triage | #850, PR #883 | 2026-06-01 | done |
| 1 | Issue #851 (no labels, OPEN): Code review: Epics 15+105+69 — Listings/syndication & Developer API stubs (origin/dev) | triage | #851, PR #904 | 2026-06-01 | done |
| 1 | Issue #859 (no labels, OPEN): sqlx 0.9 breaks runtime decode of Postgres enum columns into Rust String (SELECT * reads 500) | triage | #859, PR #871 | 2026-06-01 | done |
| 1 | Issue #867 (no labels, OPEN): Tech debt: api-server main.rs duplicates lib.rs::create_router — routers diverge silently | triage | #867, PR #870 | 2026-06-01 | done |
| 1 | Issue #836 (no labels, OPEN): Code review: Epic 2B-C — Mobile push & device registration (origin/dev) | triage | #836, PR #866 | 2026-05-31 | done |
| 1 | Issue #845 (no labels, OPEN): Code review: Epic 14 — IoT alerts, correlations, thresholds (origin/dev) | triage | #845, PR #862 | 2026-05-31 | done |
| 1 | Issue #849 (no labels, OPEN): Code review: Epic 10B+143 — Admin impersonation, Help, Board meetings auth (origin/dev) | triage | #849, PR #869 | 2026-05-31 | done |
| 1 | Issue #749 (no labels, OPEN): Code review findings: Story 6.1 announcement creation and targeting | triage | #749 | 2026-05-30 | open |
| 1 | Issue #755 (no labels, OPEN): Current dev review: Epic 8A Notification Preferences | triage | #755 | 2026-05-30 | open |
| 1 | Issue #764 (no labels, OPEN): Current dev review: Admin MFA & Auth Hardening | triage | #764 | 2026-05-30 | open |
| 1 | Issue #765 (no labels, OPEN): Current dev review: Integrations & Airbnb OAuth | triage | #765 | 2026-05-30 | open |
| 1 | Issue #769 (no labels, OPEN): Current dev review: Deploy server | triage | #769 | 2026-05-30 | open |
| 1 | Issue #779 (no labels, OPEN): Current dev review: consolidated priority rollup (origin/dev snapshot) | triage | #779 | 2026-05-30 | open |
| 1 | Issue #789 (no labels, OPEN): Dev review rounds 6-10: scheduler, notifications, admin, orgs, buildings | triage | #789 | 2026-05-30 | open |
| 1 | Reality-web ComparisonUrlHandler hardcodes English loading/error strings | refactor | code-review reality-web 2026-05-28, rotating-expert-review | 2026-05-28 | open |
| 1 | Mediation reference number uppercases full UUID (DSP-<uuid>) instead of a short code | refactor | PR #555, code-review 2026-05-27 | 2026-05-27 | open |
| 1 | Duplicated animate-spin spinner markup across mediation page + chat thread (no shared Spinner) | refactor | PR #555, code-review 2026-05-27 | 2026-05-27 | open |
| 1 | Mobile VotingScreen pure transforms toUiStatus/toUiVote have no tests | test-gap | code-review mobile-rn 2026-05-27, rotating-expert-review | 2026-05-27 | open |
| 1 | ai.rs churn-hot — 3,142 lines this run; 3,142-line route monolith, candidate for module split | refactor | git log origin/dev since 2026-05-24 | 2026-05-25 | open |
| 1 | platform_admin.rs churn-hot — 2,762 lines this run (admin/OAuth-provider feature work) | refactor | git log origin/dev since 2026-05-24 | 2026-05-25 | open |
| 1 | Announcer: untracked clear-then-set timeouts can resurrect a stale screen-reader message | bug | code-review ppt-web-ui 2026-05-24, Announcer.tsx:49 | 2026-05-24 | open |
| 0 | Portfolio dashboard: alert mark-read/resolve mutations + property-card click navigation are no-op stubs | dx | PR #328, commit 254f01d | 2026-06-04 | dropped |
