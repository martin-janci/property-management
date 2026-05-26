# Backlog of vectors

<sub>Last regenerated: 2026-05-26 22:37 UTC by routine</sub>

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
| 5 | BOLA: Airbnb/Booking integration handlers in install.rs missing tenant check (connect/disconnect/sync/status) | security | code-review api-handlers 2026-05-26, install.rs:600, PR #538, PR #544 | 2026-05-26 | ready |
| 3 | IDOR: reality-server realtors mark_inquiry_read flips any realtor's inquiry by ID with no owner scoping | security | issue #519, PR #508, realtors.rs:250, PR #548 | 2026-05-26 | done |
| 3 | IDOR: equipment delete/update + maintenance update mutate any tenant's equipment by ID with no org scoping | security | code-review api-core 2026-05-25, ai.rs:1133, equipment.rs:144 | 2026-05-25 | done |
| 3 | SSRF: signed-document fetch + webhook-test POST issue outbound requests to unvalidated user-controlled URLs | security | issue #439, signatures.rs:628, integrations.rs:2743, PR #450 | 2026-05-25 | done |
| 3 | IDOR: unlink_voice_device deactivates any device by ID with no owner/org scoping | security | code-review api-core 2026-05-23, ai.rs:3002, PR #461 | 2026-05-25 | done |
| 2 | e-signature fail-closed (PR #532): ESIGN secrets must be set in staging/prod or api-server panics on restart | security | PR #532 | 2026-05-26 | open |
| 2 | Screen-map drift: PR #547 added a report execution-history route in ppt-web without a docs/screens/ppt entry | test-gap | PR #547 | 2026-05-26 | open |
| 2 | Dispute state machine (PR #506) shipped with no tests + no org predicate on update_status | test-gap | PR #506, issue #520, PR #514, PR #548 | 2026-05-26 | open |
| 2 | Reduce App.tsx route-aggregator coupling (top churn hotspot, merge-conflict risk) | refactor | PR #474, PR #475, PR #489, PR #511 | 2026-05-26 | open |
| 2 | announcements.rs churn-hot — 2,722 lines this run (Epic 2B + Epic 6 work) | refactor | git log origin/dev since 2026-05-24, PR #504, PR #505 | 2026-05-26 | open |
| 2 | announcements.rs (2,722 LOC) — explicit module-split into routes/announcements/{crud,targeting,delivery,reactions,mod}.rs | refactor | pm-tech-lead analysis 2026-05-25 | 2026-05-25 | open |
| 2 | platform_admin.rs (2,762 LOC) — explicit module-split into routes/platform_admin/{tenants,features,billing,audit,mod}.rs | refactor | pm-tech-lead analysis 2026-05-25 | 2026-05-25 | open |
| 2 | ai.rs (3,134 LOC) — explicit module-split into routes/ai/{sessions,equipment,workflows,voice,llm,mod}.rs | refactor | pm-tech-lead analysis 2026-05-25, security-voice-device-idor (PR #461), security-equipment-idor | 2026-05-25 | open |
| 2 | Screen-map drift: PR #460 touched reality-web listing page without a docs/screens/reality update | test-gap | PR #460 | 2026-05-25 | closed |
| 2 | Screen-map drift: PR #464 wired a neighbors route in ppt-web without a docs/screens/ppt entry | test-gap | PR #464 | 2026-05-25 | done |
| 2 | ppt-web status/auth components hardcode English in an otherwise i18n'd app | refactor | code-review ppt-web-ui 2026-05-24, rotating-expert-review | 2026-05-25 | open |
| 2 | Latent fail-open: ProtectedRoute role check is skipped when user.role is falsy | security | code-review ppt-web-ui 2026-05-24, ProtectedRoute.tsx:117, PR #459 | 2026-05-25 | done |
| 2 | IDOR: reality-server mark_as_read flips any realtor's inquiry by ID with no owner scoping | security | code-review reality-server 2026-05-23, inquiries.rs:554 | 2026-05-25 | done |
| 2 | documents.rs churn-hot — 10,659 lines over 14d | refactor | git log origin/main since 2026-05-06, git log origin/dev since 2026-05-20, PR #456 | 2026-05-25 | done |
| 2 | organizations.rs churn-hot — 12,060 lines over 14d (multitenancy + admin) | refactor | git log origin/main since 2026-05-06, git log origin/dev since 2026-05-20, PR #456 | 2026-05-25 | done |
| 2 | integrations.rs churn-hot — 12,977 lines over 14d, candidate for module split | refactor | git log origin/main since 2026-05-06, git log origin/dev since 2026-05-20, PR #456 | 2026-05-25 | done |
| 2 | Dead/duplicate handler modules: AuthHandler & BuildingHandler unused, routes reimplement inline | refactor | code-review api-handlers 2026-05-23, PR #437 | 2026-05-24 | done |
| 2 | Integration marketplace install/OAuth flows are placeholders — wire backend handlers + UI navigation | dx | PR #282, PR #328, commit c97781a, commit 254f01d | 2026-05-23 | open |
| 2 | Complete RLS migration in 31 remaining handlers (voting, market_pricing, faults, notif_prefs, reports) | security | issue #160, PR #420, PR #421 | 2026-05-23 | done |
| 2 | Portfolio dashboard: alert mark-read/resolve mutations + property-card click navigation are no-op stubs | dx | PR #328, commit 254f01d | 2026-05-20 | open |
| 1 | Integrations install.rs logs the OAuth state (CSRF) token at debug level | security | code-review api-handlers 2026-05-26, install.rs:456 | 2026-05-26 | open |
| 1 | platform_admin.rs churn-hot — 2,762 lines this run (admin/OAuth-provider feature work) | refactor | git log origin/dev since 2026-05-24 | 2026-05-25 | open |
| 1 | ai.rs churn-hot — 3,142 lines this run; 3,142-line route monolith, candidate for module split | refactor | git log origin/dev since 2026-05-24 | 2026-05-25 | open |
| 1 | Announcer: untracked clear-then-set timeouts can resurrect a stale screen-reader message | bug | code-review ppt-web-ui 2026-05-24, Announcer.tsx:49 | 2026-05-24 | open |
