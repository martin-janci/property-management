# Backlog of vectors
<sub>Last regenerated: 2026-06-19 04:30 UTC by routine</sub>

| Score | Vector | Title | Status | Source | Updated | Plan |
|-------|--------|-------|--------|--------|---------|------|
| 6 | test-gap | Add regression tests for inquiry mark_as_read cross-tenant IDOR fix (PR #497) | done | PR #497 | 2026-05-26 | plans/_archive/test-gap-inquiry-idor-regression.md |
| 3 | dx | Refill action-list — claimable=0, coverage.json stale (operator action required) | needs-human-judgement | research-routine fire-payload 2026-06-19 | 2026-06-19 | — |
| 3 | bug | ReportFaultScreen.tsx handleSubmit() fakes API call with setTimeout(1500) — fault reports never reach backend (App.tsx:126 wires this) | ready | Phase 1.5 review of mobile-rn segment (2026-06-16) | 2026-06-16 | plans/code-review-mobile-rn-report-fault-fake-submit.md |
| 3 | bug | Reality-web RealtorManagement.tsx hardcoded English strings — agency flow not localized to sk/cs/de | ready | rotating-expert-review reality-web 2026-06-14 | 2026-06-15 | plans/code-review-reality-web-realtor-mgmt-untranslated.md |
| 3 | bug | Reality-web listing detail SSR crashes on partial 200 body — JSON-LD build deref of undefined fields | ready | rotating-expert-review reality-web 2026-06-14 | 2026-06-14 | plans/code-review-reality-web-listing-page-ssr-crash.md |
| 3 | bug | Reality-web ComparisonUrlHandler hits non-existent /api/listings/${id} — every shared comparison URL 404s | ready | rotating-expert-review reality-web 2026-06-14 | 2026-06-14 | plans/code-review-reality-web-share-comparison-404.md |
| 3 | bug | iOS SearchView.swift does not compile — performSearch/scheduleSearch undefined, resultsGrid corrupted | ready | issue #1266 | 2026-06-11 | plans/bug-ios-searchview-uncompilable.md |
| 3 | security | PR #1193 (fix(aml-dsa): lock DSA reports to platform roles + fix file-path disclosure (PAP-47)) merg | dropped | PR #1193 | 2026-06-10 | — |
| 3 | security | PR #1203 (fix(aml_dsa): close cross-tenant IDOR in moderation + AML-review handlers (PAP-36)) merged | dropped | PR #1203 | 2026-06-10 | — |
| 3 | bug | Schema drift: runtime SQL errors from non-existent columns in voting/messaging/notification paths | done | Issue #1008 | 2026-06-07 | — |
| 3 | security | IDOR: ai.rs LLM-doc handlers publish/list/get any tenant's listing descriptions & photo enhancements unscoped | ready | code-review api-core 2026-05-29 | 2026-06-01 | plans/security-llm-doc-idor.md |
| 3 | security | IDOR: reality-server realtors mark_inquiry_read flips any realtor's inquiry by ID with no owner scoping | done | issue #519 | 2026-05-26 | plans/_archive/security-realtors-mark-inquiry-read-idor.md |
| 3 | security | IDOR: equipment delete/update + maintenance update mutate any tenant's equipment by ID with no org scoping | done | code-review api-core 2026-05-25 | 2026-05-25 | plans/_archive/security-equipment-idor.md |
| 3 | security | SSRF: signed-document fetch + webhook-test POST issue outbound requests to unvalidated user-controlled URLs | done | issue #439 | 2026-05-25 | plans/_archive/security-ssrf-outbound-url-validation.md |
| 3 | security | IDOR: unlink_voice_device deactivates any device by ID with no owner/org scoping | done | code-review api-core 2026-05-23 | 2026-05-25 | plans/_archive/security-voice-device-idor.md |
| 2 | test-gap | Screen-map drift: PR PR#1453 touched reality-web routes without doc updates | open | PR #1453 | 2026-06-19 | — |
| 2 | test-gap | Screen-map drift: PR PR#1454 touched ppt-web routes without doc updates | open | PR #1454 | 2026-06-19 | — |
| 2 | test-gap | Screen-map drift: PR PR#1545 touched reality-web routes without doc updates | open | PR #1545 | 2026-06-19 | — |
| 2 | test-gap | Screen-map drift: PR PR#1555 touched ppt-web routes without doc updates | open | PR #1555 | 2026-06-19 | — |
| 2 | test-gap | Screen-map drift: PR PR#1559 touched ppt-web routes without doc updates | open | PR #1559 | 2026-06-19 | — |
| 2 | bug | vote.rs:1765 calculate_question_result() uses partial_cmp().unwrap() on f64 — NaN/Inf weights panic /votes/{id}/results | done | code-review api-core 2026-06-15 | 2026-06-16 | — |
| 2 | bug | useDeepLinkRouting.ts:27-36 — initialize() re-runs on onNavigate identity change + void promise with no .catch → duplicate nav / unhandled rejection | open | Phase 1.5 review of mobile-rn segment (2026-06-16) | 2026-06-16 | — |
| 2 | bug | Mobile RN production screens (Buildings/Meters/Leases/PersonMonths/Notifications/Threads/Forms) render hardcoded MOCK_* arrays — no API wiring | open | Phase 1.5 review of mobile-rn segment (2026-06-16) | 2026-06-16 | — |
| 2 | test-gap | PR #1418 touched routes/** (faults.route.test.tsx) without updating docs/screens/ppt/* — heuristic, test-file fix | open | PR #1418 | 2026-06-16 | — |
| 2 | bug | iOS deep-link layer dead at runtime — Info.plist missing CFBundleURLTypes + applinks entitlement | open | issue #1267 | 2026-06-11 | — |
| 2 | test-gap | AI llm/sessions + integrations sync + subscriptions RLS migration (PR #1287, PAP-169) shipped without a new regression test | open | PR #1287 | 2026-06-11 | — |
| 2 | test-gap | Webhook handlers RLS migration (PR #1288, PAP-170) shipped without a new regression test for repo-layer methods | open | PR #1288 | 2026-06-11 | — |
| 2 | test-gap | api_ecosystem.rs RLS migration (PR #1289, PAP-167) — 162-line handler rework shipped without a regression test for the public-connection routing | open | PR #1289 | 2026-06-11 | — |
| 2 | test-gap | mfa.rs RLS migration (PR #1292, PAP-168) shipped without a regression test; also landed broken and was hotfixed in PR #1287 | open | PR #1292 | 2026-06-11 | — |
| 2 | test-gap | PR #1196 (feat(ppt-web): add missing test coverage for faults feature) merged with 2 unchecked TODO  | dropped | PR #1196 | 2026-06-10 | — |
| 2 | bug | DeepLinkRouter skips URL-decoding while Android Uri.getQueryParameter decodes — SSO tokens diverge per platform | open | mobile-native-kmp segment review 2026-06-06 | 2026-06-06 | — |
| 2 | bug | SearchScreen stale-response race — overlapping searches can clobber newer results | open | mobile-native-kmp segment review 2026-06-06 | 2026-06-06 | — |
| 2 | dx | PushFanoutWorker BLPOP queue-drain deferred — Redis path is a logging no-op | done | PR #515 | 2026-06-06 | — |
| 2 | refactor | ai.rs (3,134 LOC) — explicit module-split into routes/ai/{sessions,equipment,workflows,voice,llm,mod}.rs | done | pm-tech-lead analysis 2026-05-25 | 2026-06-06 | — |
| 2 | refactor | announcements.rs churn-hot — 2,722 lines this run (Epic 2B + Epic 6 work) | done | git log origin/dev since 2026-05-24 | 2026-06-06 | — |
| 2 | refactor | announcements.rs (2,722 LOC) — explicit module-split into routes/announcements/{crud,targeting,delivery,reactions,mod}.rs | done | pm-tech-lead analysis 2026-05-25 | 2026-06-06 | — |
| 2 | refactor | Reduce App.tsx route-aggregator coupling (top churn hotspot, merge-conflict risk) | done | PR #474 | 2026-06-06 | — |
| 2 | refactor | platform_admin.rs (2,762 LOC) — explicit module-split into routes/platform_admin/{tenants,features,billing,audit,mod}.rs | done | pm-tech-lead analysis 2026-05-25 | 2026-06-06 | — |
| 2 | test-gap | Screen-map drift: PR #1033 wired error/retry into AnnouncementsPage+FaultsPage via App.tsx without a docs/screens/ppt update | done | PR #1033 | 2026-06-06 | — |
| 2 | bug | Risky churn: mobile App.tsx deep-link/doc-detail wiring changing across back-to-back PRs without coverage | done | PR #1103 | 2026-06-05 | — |
| 2 | dx | Integration marketplace install/OAuth flows are placeholders — wire backend handlers + UI navigation | done | PR #1105 | 2026-06-05 | — |
| 2 | test-gap | Booking push availability/rates endpoints add batch-cap + non-negative guards with no regression test | done | PR #1068 | 2026-06-05 | — |
| 2 | test-gap | Portal webhook fail-closed fix (PR #874) shipped without a regression test for unverified-signature rejection | done | PR #1052 | 2026-06-05 | — |
| 2 | test-gap | Mobile dev-review batch (PR #918, 5 files under frontend/apps/mobile/src) shipped without a regression test | done | PR #1072 | 2026-06-05 | — |
| 2 | test-gap | Reality-server SSO consumer review fix (PR #921, closes #820) shipped without a regression test | done | PR #1076 | 2026-06-05 | — |
| 2 | test-gap | CI branch-protection + auto-rebase workflow change (PR #923) shipped without an integration test | done | PR #1057 | 2026-06-05 | — |
| 2 | test-gap | deploy-server OIDC scope mapping (#939) shipped without unit test for derive_oidc_scopes | done | PR #1106 | 2026-06-05 | — |
| 2 | test-gap | Mobile RN dev-review tail (#943) shipped without test coverage | done | PR #1080 | 2026-06-05 | — |
| 2 | test-gap | Frontend gap-sweep (PR #990, 34 files across Epics 1/6/7B/9/10B/11/15/17/18) shipped without a regression test | done | PR #1081 | 2026-06-05 | — |
| 2 | test-gap | Mobile document-detail wiring (PR #992) shipped without a regression test for the deep-link payload path | done | PR #1082 | 2026-06-05 | — |
| 2 | test-gap | Screen-map drift: PR #839 modified ppt-web App.tsx (FileDisputePageRoute) without a docs/screens/ppt update | done | PR #1056 | 2026-06-05 | — |
| 2 | refactor | ppt-web status/auth components hardcode English in an otherwise i18n'd app | done | code-review ppt-web-ui 2026-05-24 | 2026-06-04 | — |
| 2 | bug | MediationWorkspacePage shows empty/unknown state instead of error UI on dispute fetch failure | done | PR #555 | 2026-06-03 | — |
| 2 | bug | Mobile VotingScreen double-casts API result across boundary — render-time crash on unexpected shape | done | code-review mobile-rn 2026-05-27 | 2026-06-03 | — |
| 2 | bug | Reality-web InviteRealtorModal swallows invite-mutation failure with no error UI | done | code-review reality-web 2026-05-28 | 2026-06-03 | — |
| 2 | bug | Airbnb webhook at-least-once delivery enqueues duplicate SYNC_EXTERNAL jobs | done | PR #538 | 2026-06-03 | — |
| 2 | dx | DocumentsBrowse MoveFolderDialog cannot pre-select current folder (DocumentSummary lacks folder_id) | done | PR #623 | 2026-06-03 | — |
| 2 | test-gap | API + SPA security-headers middleware (PR #963) shipped without an assertion test for HSTS/nosniff/CSP | done | PR #963 | 2026-06-03 | — |
| 2 | test-gap | api-server main.rs vs lib.rs::create_router diverge silently (5 routes unreachable in prod, no test asserts parity) | done | PR #866 | 2026-06-01 | — |
| 2 | bug | ReportSchedule.update_schedule stores cron in `time` workaround; documented UPDATE never runs (missing cron_expression column) | done | PR #611 | 2026-05-30 | — |
| 2 | test-gap | Screen-map drift: report execution-history route (PR #547) added without a ppt screen doc | done | PR #547 | 2026-05-27 | — |
| 2 | test-gap | Dispute state machine (PR #506) shipped with no tests + no org predicate on update_status | done | PR #506 | 2026-05-26 | — |
| 2 | refactor | documents.rs churn-hot — 10,659 lines over 14d | done | git log origin/main since 2026-05-06 | 2026-05-25 | — |
| 2 | refactor | integrations.rs churn-hot — 12,977 lines over 14d, candidate for module split | done | git log origin/main since 2026-05-06 | 2026-05-25 | — |
| 2 | refactor | organizations.rs churn-hot — 12,060 lines over 14d (multitenancy + admin) | done | git log origin/main since 2026-05-06 | 2026-05-25 | — |
| 2 | security | IDOR: reality-server mark_as_read flips any realtor's inquiry by ID with no owner scoping | done | code-review reality-server 2026-05-23 | 2026-05-25 | plans/_archive/security-inquiry-read-idor.md |
| 2 | security | Latent fail-open: ProtectedRoute role check is skipped when user.role is falsy | done | code-review ppt-web-ui 2026-05-24 | 2026-05-25 | — |
| 2 | test-gap | Screen-map drift: PR #464 wired a neighbors route in ppt-web without a docs/screens/ppt entry | done | PR #464 | 2026-05-25 | — |
| 2 | test-gap | Screen-map drift: PR #460 touched reality-web listing page without a docs/screens/reality update | closed | PR #460 | 2026-05-25 | — |
| 2 | refactor | Dead/duplicate handler modules: AuthHandler & BuildingHandler unused, routes reimplement inline | done | code-review api-handlers 2026-05-23 | 2026-05-24 | — |
| 2 | security | Complete RLS migration in 31 remaining handlers (voting, market_pricing, faults, notif_prefs, reports) | done | issue #160 | 2026-05-23 | — |
| 1 | bug | Risky churn: api-server main.rs security-headers wiring shipped without a middleware smoke test | open | PR #963 | 2026-06-19 | — |
| 1 | refactor | Churn hotspot: 2940 lines changed in backend/crates/db/src/repositories/document.rs (window 2026-06-10 03:05Z→18:30Z) | open | local git numstat since 2026-06-10T03:05:00Z | 2026-06-19 | — |
| 1 | refactor | Churn hotspot: `backend/crates/db/src/repositories/rental.rs` | open | git log --since=2026-06-16T03:25:00Z origin/dev | 2026-06-19 | — |
| 1 | refactor | Churn hotspot: `frontend/apps/mobile/package.json` | open | git log --since=2026-06-16T03:25:00Z origin/dev | 2026-06-19 | — |
| 1 | test-gap | Reality-server listings pagination clamp (PR #959) shipped without a regression test for limit=-1 | open | PR #959 | 2026-06-19 | — |
| 1 | test-gap | Screen-map drift: PR #1085 modified reality-web listing detail metadata + page without screen-doc update | open | PR #1085 | 2026-06-19 | — |
| 1 | test-gap | Screen-map drift: PR #1100 modified ppt-web App.tsx (FileDisputePageRoute extraction) without screen-doc update | open | PR #1100 | 2026-06-19 | — |
| 1 | refactor | Churn hotspot: backend/servers/api-server/src/routes/forms.rs touched 2x since 2026-06-12 (window 2026-06-12→2026-06-13) | open | local git numstat since 2026-06-12 | 2026-06-16 | — |
| 1 | refactor | booking_oauth_csrf_tests.rs hotspot — 484-line NEW test file (PR #1393 #1424 OAuth CSRF coverage) | open | local git numstat since 2026-06-15 (commit 67c24bd..origin/dev) | 2026-06-16 | — |
| 1 | refactor | booking_oauth_routes_tests.rs hotspot — 381-line NEW test file (PR #1393 OAuth routes coverage) | open | local git numstat since 2026-06-15 | 2026-06-16 | — |
| 1 | dx | PR #1425 (GH #1377 document presigned-URL tests) closed unmerged — superseded by merged #1394 | open | PR #1425 | 2026-06-16 | — |
| 1 | dx | PR #1179 (docs(epics) catalog backfill for 37 mounted-but-undocumented backend modules) — stalled at 7d, no reviewDecision | open | PR #1179 | 2026-06-16 | — |
| 1 | refactor | forms.rs repeated-churn — runs_seen=2 (#1337 explicit_auto_deref + #1397 org-scope hardening) | open | hotspot_history.runs_seen 1→2 with new churn this run | 2026-06-16 | — |
| 1 | test-gap | Screen-map drift: PR #922 modified ppt-web App.tsx (dev-review rounds 1-5 fixes) without a docs/screens/ppt update | open | PR #922 | 2026-06-16 | — |
| 1 | refactor | Churn hotspot: 53 lines in backend/crates/db/src/repositories/form.rs (PR #1379 #1332 unblock) | open | PR #1379 | 2026-06-15 | — |
| 1 | refactor | Churn hotspot: 124 lines in frontend/apps/mobile/app.config.icon.test.ts (PR #1383 gap-85-2) | open | PR #1383 | 2026-06-15 | — |
| 1 | refactor | Churn hotspot: 94 lines in frontend/apps/mobile/app.config.ts (PR #1383 gap-85-2) | open | PR #1383 | 2026-06-15 | — |
| 1 | refactor | crypto.rs:127 SysRng.try_fill_bytes(...).expect() panics if OS CSPRNG errors during integration-credential encrypt | open | code-review api-core 2026-06-15 | 2026-06-15 | — |
| 1 | refactor | PR #1378 closed without merge — DROP-OWNED-BY teardown theory for #1332 was wrong root cause, superseded by #1379 | done | PR #1378 | 2026-06-15 | — |
| 1 | triage | Issue #1380 (no labels, OPEN): Dispatcher stale gap-scan buffer + Tier-2 escalation endpoint misconfigured | open | issue #1380 | 2026-06-15 | — |
| 1 | refactor | Churn hotspot: backend/crates/db/tests/form_rls_repo_tests.rs touched 2x since 2026-06-12 (window 2026-06-12→2026-06-13) | open | local git numstat since 2026-06-12 | 2026-06-13 | — |
| 1 | refactor | Churn hotspot: backend/servers/api-server/tests/reserve_funds_cross_org_idor_tests.rs touched 2x since 2026-06-12 (window 2026-06-12→2026-06-13) | open | local git numstat since 2026-06-12 | 2026-06-13 | — |
| 1 | dx | Stalled review: PR #988 (Epic: reusable Playwright E2E framework + sitemap FlowRunner) open 10d, no reviewDecision | open | PR #988 | 2026-06-13 | — |
| 1 | triage | Issue #1331 (no labels, OPEN): Backend `test` job red/hanging on dev base — blocks the entire backend merge pipeline | open | #1331 | 2026-06-13 | — |
| 1 | refactor | Churn hotspot: backend/crates/db/src/repositories/reality_portal.rs (+59/−28 in PR #1297 PAP-142 IDOR scoping) | open | PR #1297 commit 8c711c6 | 2026-06-12 | — |
| 1 | refactor | Churn hotspot: backend/crates/db/src/repositories/sensor.rs (+248/-86 in PR #1321/#1322 PAP-151 re-land + fmt) | open | PR #1321 commit | 2026-06-12 | — |
| 1 | refactor | Churn hotspot: backend/crates/integrations/src/booking.rs (+404/−29 in PR #1294 Booking.com OTA retry) | open | PR #1294 commit 7ccce8a | 2026-06-12 | — |
| 1 | refactor | Churn hotspot: backend/servers/api-server/src/routes/api_ecosystem.rs (+106/−27 in PR #1293 PAP-171; second touch in 24h) | open | PR #1293 commit 1e50156 | 2026-06-12 | — |
| 1 | refactor | Churn hotspot: backend/servers/api-server/src/routes/iot.rs (+278/-403 in PR #1321/#1322 PAP-151 re-land + fmt) | open | PR #1321 commit | 2026-06-12 | — |
| 1 | refactor | Churn hotspot: backend/servers/api-server/src/routes/reserve_funds.rs (+228/-255 in PR #1321 PAP-151 re-land) | open | PR #1321 commit | 2026-06-12 | — |
| 1 | dx | PR #1274 (cargo-minor-patch group, /backend, 9 updates) closed unmerged — superseded by #1313 after auto-rebase fix landed | open | PR #1274 | 2026-06-12 | — |
| 1 | refactor | Churn hotspot: 2856 lines changed in backend/crates/db/src/repositories/subscription.rs (window 2026-06-10 03:05Z→18:30Z) | open | local git numstat since 2026-06-10T03:05:00Z | 2026-06-10 | — |
| 1 | refactor | Churn hotspot: 2691 lines changed in backend/servers/api-server/src/routes/aml_dsa.rs (window 2026-06-10 03:05Z→18:30Z) | open | local git numstat since 2026-06-10T03:05:00Z | 2026-06-10 | — |
| 1 | refactor | Churn hotspot: 1021 lines changed in backend/servers/api-server/src/routes/emergency.rs (window 2026 | open | local git numstat since 2026-06-07 | 2026-06-10 | — |
| 1 | refactor | Churn hotspot: 709 lines changed in backend/servers/api-server/src/routes/enhanced_tenant_screening. | open | local git numstat since 2026-06-07 | 2026-06-10 | — |
| 1 | refactor | Churn hotspot: 929 lines changed in backend/servers/api-server/src/routes/vendors.rs (window 2026-06 | open | local git numstat since 2026-06-07 | 2026-06-10 | — |
| 1 | test-gap | PKCE unit test became a tautology after services/oauth.rs DRY refactor (#1132) | done | #1137 | 2026-06-07 | — |
| 1 | triage | Triage: dispatcher incident — assignments-archive.json corrupted to 1/196 rows on dev branch (#1061) | done | Issue #1061 | 2026-06-07 | — |
| 1 | triage | Issue #1151 (no labels, OPEN): Research dispatcher: claimable buffer is stale — true claimable work = 0 despite metric=53 | open | #1151 | 2026-06-07 | — |
| 1 | triage | Issue #769 (no labels, OPEN): Current dev review: Deploy server | done | #769 | 2026-06-07 | — |
| 1 | triage | Issue #789 (no labels, OPEN): Dev review rounds 6-10: scheduler, notifications, admin, orgs, buildings | done | #789 | 2026-06-07 | — |
| 1 | triage | Issue #950 (no labels, OPEN): CI: trigger-deploy 403 marks all dev image builds red and blocks staging auto-deploy | done | #950 | 2026-06-07 | — |
| 1 | triage | Issue #952 (no labels, OPEN): [staging] Reality SSO login dead-ends: redirect_uri callback 404s on reality apex | done | #952 | 2026-06-07 | — |
| 1 | refactor | Churn hotspot: ListingDetailScreen.kt — +1279 LOC this run (gap-82-4 reality mobile favorite toggle) | open | PR #1121 | 2026-06-06 | — |
| 1 | refactor | Churn hotspot: SearchScreen.kt — +1293 LOC this run (gap-82-3 reality mobile search/filters) | open | PR #1125 | 2026-06-06 | — |
| 1 | refactor | MainActivity reimplements deep-link dispatch instead of calling shared DeepLinkRouter — drift trap | open | mobile-native-kmp segment review 2026-06-06 | 2026-06-06 | — |
| 1 | dx | docker/nginx admin-web + ppt-web templates churned twice this run (security headers + redirects) | done | PR #963 | 2026-06-06 | — |
| 1 | refactor | ai.rs churn-hot — 3,142 lines this run; 3,142-line route monolith, candidate for module split | done | git log origin/dev since 2026-05-24 | 2026-06-06 | — |
| 1 | refactor | ppt-web e2e auth-refresh.spec.ts added (+252 lines, story 79-2 token-refresh coverage) | done | PR #1047 | 2026-06-06 | — |
| 1 | refactor | api-server esignature_webhook_idempotency_tests.rs added (+228 lines, terminal-state regression) | done | PR #1034 | 2026-06-06 | — |
| 1 | refactor | ppt-web EvidenceUploader.test.tsx added (+202 lines, dispute-filing AC-2 regression) | done | PR #1048 | 2026-06-06 | — |
| 1 | refactor | api-server main.rs touched twice this run (gap-sweep + security headers) — minor churn marker | done | PR #989 | 2026-06-06 | — |
| 1 | refactor | Duplicated animate-spin spinner markup across mediation page + chat thread (no shared Spinner) | done | PR #555 | 2026-06-06 | — |
| 1 | refactor | Mediation reference number uppercases full UUID (DSP-<uuid>) instead of a short code | done | PR #555 | 2026-06-06 | — |
| 1 | refactor | frontend/apps/mobile/src/App.tsx churned twice this run (universal links + doc-detail wiring) | done | PR #962 | 2026-06-06 | — |
| 1 | refactor | platform_admin.rs churn-hot — 2,762 lines this run (admin/OAuth-provider feature work) | done | git log origin/dev since 2026-05-24 | 2026-06-06 | — |
| 1 | refactor | Reality-web ComparisonUrlHandler hardcodes English loading/error strings | done | code-review reality-web 2026-05-28 | 2026-06-06 | — |
| 1 | refactor | Watch routes/oauth.rs churn after audit-log + hardening PRs | done | PR #930 | 2026-06-06 | — |
| 1 | refactor | Watch services/oauth.rs churn after introspect/revoke hardening (#933) | done | PR #933 | 2026-06-06 | — |
| 1 | test-gap | Mobile VotingScreen pure transforms toUiStatus/toUiVote have no tests | done | code-review mobile-rn 2026-05-27 | 2026-06-06 | — |
| 1 | triage | Issue #749 (no labels, OPEN): Code review findings: Story 6.1 announcement creation and targeting | done | #749 | 2026-06-06 | — |
| 1 | triage | Issue #755 (no labels, OPEN): Current dev review: Epic 8A Notification Preferences | done | #755 | 2026-06-06 | — |
| 1 | triage | Issue #764 (no labels, OPEN): Current dev review: Admin MFA & Auth Hardening | done | #764 | 2026-06-06 | — |
| 1 | triage | Issue #765 (no labels, OPEN): Current dev review: Integrations & Airbnb OAuth | done | #765 | 2026-06-06 | — |
| 1 | bug | Mobile VotingScreen hardcodes en-US in toLocaleDateString — vote dates never localize | done | PR #1083 | 2026-06-05 | — |
| 1 | bug | Reality-web listing generateMetadata can throw during SSR on malformed 200 body | done | PR #1085 | 2026-06-05 | — |
| 1 | security | PR #908 (fix(security): require PKCE on OAuth authorization-code flow, closes #823) was closed unmerged — verify whether PKCE enforcement still pending | done | PR #908 | 2026-06-03 | — |
| 1 | triage | Issue #751 (no labels, OPEN): Current dev review: frontend/web/API-client findings | done | #751 | 2026-06-02 | — |
| 1 | triage | Issue #752 (no labels, OPEN): Current dev review: mobile CI tooling findings | done | #752 | 2026-06-02 | — |
| 1 | triage | Issue #756 (no labels, OPEN): Current dev review: Epic 10A OAuth Provider | done | #756 | 2026-06-02 | — |
| 1 | triage | Issue #761 (no labels, OPEN): Current dev review: Epic 84 E-Signature & Leases | done | #761 | 2026-06-02 | — |
| 1 | triage | Issue #763 (no labels, OPEN): Current dev review: Reality Server & Inquiries | done | #763 | 2026-06-02 | — |
| 1 | triage | Issue #767 (no labels, OPEN): Current dev review: Mobile RN Property Management app | done | #767 | 2026-06-02 | — |
| 1 | triage | Issue #768 (no labels, OPEN): Current dev review: Admin-web features (10B) | done | #768 | 2026-06-02 | — |
| 1 | triage | Issue #920 (no labels, OPEN): Announcement targeting not enforced on read (intra-org disclosure) | done | #920 | 2026-06-02 | — |
| 1 | triage | Issue #750 (no labels, OPEN): Current dev review: backend/API/database findings | done | #750 | 2026-06-01 | — |
| 1 | triage | Issue #753 (no labels, OPEN): Current dev review: Epic 6 Announcements & Communication | done | #753 | 2026-06-01 | — |
| 1 | triage | Issue #754 (no labels, OPEN): Current dev review: Epic 7A Basic Document Management | done | #754 | 2026-06-01 | — |
| 1 | triage | Issue #757 (no labels, OPEN): Current dev review: Epic 10B Platform Administration | done | #757 | 2026-06-01 | — |
| 1 | triage | Issue #760 (no labels, OPEN): Current dev review: Epic 79 Disputes & Mediation | done | #760 | 2026-06-01 | — |
| 1 | triage | Issue #762 (no labels, OPEN): Current dev review: Reports & Schedules | done | #762 | 2026-06-01 | — |
| 1 | triage | Issue #766 (no labels, OPEN): Current dev review: AI & LLM routes | done | #766 | 2026-06-01 | — |
| 1 | triage | Issue #770 (no labels, OPEN): Current dev review: Faults & triage | done | #770 | 2026-06-01 | — |
| 1 | triage | Issue #771 (no labels, OPEN): Current dev review: Research dispatcher & CI automation | done | #771 | 2026-06-01 | — |
| 1 | triage | Issue #772 (no labels, OPEN): Current dev review: Auth core (delta confirmation) | done | #772 | 2026-06-01 | — |
| 1 | triage | Issue #773 (no labels, OPEN): Current dev review: Leases & rental | done | #773 | 2026-06-01 | — |
| 1 | triage | Issue #774 (no labels, OPEN): Current dev review: Reality server (broad) | done | #774 | 2026-06-01 | — |
| 1 | triage | Issue #775 (no labels, OPEN): Current dev review: WebSocket realtime | done | #775 | 2026-06-01 | — |
| 1 | triage | Issue #776 (no labels, OPEN): Current dev review: Equipment & audit log | done | #776 | 2026-06-01 | — |
| 1 | triage | Issue #777 (no labels, OPEN): Current dev review: Compliance & GDPR | done | #777 | 2026-06-01 | — |
| 1 | triage | Issue #778 (no labels, OPEN): Current dev review: Marketplace, voting, investor portal, impersonation | done | #778 | 2026-06-01 | — |
| 1 | triage | Issue #788 (no labels, OPEN): Dev review rounds 1-5: mobile-native + ppt-web surfaces | done | #788 | 2026-06-01 | — |
| 1 | triage | Issue #790 (no labels, OPEN): Dev review rounds 11-15: vendor, predictive, reality-web, middleware | done | #790 | 2026-06-01 | — |
| 1 | triage | Issue #791 (no labels, OPEN): Dev review rounds 16-20: push, e-sign, portal, webhooks, reserves | done | #791 | 2026-06-01 | — |
| 1 | triage | Issue #846 (no labels, OPEN): Code review: Epics 12+65 — Meters & Energy/ESG (origin/dev) | done | #846 | 2026-06-01 | — |
| 1 | triage | Issue #847 (no labels, OPEN): Code review: Reality-server — Inquiries IDOR (Epics 16–19) (origin/dev) | done | #847 | 2026-06-01 | — |
| 1 | triage | Issue #848 (no labels, OPEN): Code review: Epics 78+134 — Vendor portal stubs & Predictive maintenance gaps (origin/dev) | done | #848 | 2026-06-01 | — |
| 1 | triage | Issue #850 (no labels, OPEN): Code review: Epics 61+146+42 — Multi-currency, Data residency, Violations (origin/dev) | done | #850 | 2026-06-01 | — |
| 1 | triage | Issue #851 (no labels, OPEN): Code review: Epics 15+105+69 — Listings/syndication & Developer API stubs (origin/dev) | done | #851 | 2026-06-01 | — |
| 1 | triage | Issue #859 (no labels, OPEN): sqlx 0.9 breaks runtime decode of Postgres enum columns into Rust String (SELECT * reads 500) | done | #859 | 2026-06-01 | — |
| 1 | triage | Issue #867 (no labels, OPEN): Tech debt: api-server main.rs duplicates lib.rs::create_router — routers diverge silently | done | #867 | 2026-06-01 | — |
| 1 | triage | Issue #836 (no labels, OPEN): Code review: Epic 2B-C — Mobile push & device registration (origin/dev) | done | #836 | 2026-05-31 | — |
| 1 | triage | Issue #845 (no labels, OPEN): Code review: Epic 14 — IoT alerts, correlations, thresholds (origin/dev) | done | #845 | 2026-05-31 | — |
| 1 | triage | Issue #849 (no labels, OPEN): Code review: Epic 10B+143 — Admin impersonation, Help, Board meetings auth (origin/dev) | done | #849 | 2026-05-31 | — |
| 0 | refactor | Churn hotspot: AnnouncementsScreen.tsx — 4 PRs this run, instability proxy | dropped | PR #1101 | 2026-06-19 | — |
| 0 | refactor | Churn hotspot: AnnouncementsScreen.test.ts — 4 PRs this run, instability proxy | dropped | PR #1101 | 2026-06-19 | — |
| 0 | refactor | Churn hotspot: DocumentsScreen.tsx — 3 PRs this run | dropped | PR #1101 | 2026-06-19 | — |
| 0 | triage | Dispatcher action-list.json corruption when MCP push falls back from blocked git push | dropped | #1014 | 2026-06-19 | — |
| 0 | triage | Issue #951 (no labels, OPEN): Deploy blocker: api-server requires ESIGN_TOKEN_SECRET + ESIGN_WEBHOOK_SECRET not injected by deploy-server (staging/prod) | dropped | #951 | 2026-06-19 | — |
| 0 | refactor | Stabilize oauth_integration_tests churn — heavy edits across 3 OAuth fix PRs | dropped | PR #930 | 2026-06-16 | — |
| 0 | triage | Issue #779 (no labels, OPEN): Current dev review: consolidated priority rollup (origin/dev snapshot) | dropped | #779 | 2026-06-13 | — |
| 0 | bug | Announcer: untracked clear-then-set timeouts can resurrect a stale screen-reader message | dropped | code-review ppt-web-ui 2026-05-24 | 2026-06-07 | — |
| 0 | dx | Portfolio dashboard: alert mark-read/resolve mutations + property-card click navigation are no-op stubs | dropped | PR #328 | 2026-06-04 | — |
