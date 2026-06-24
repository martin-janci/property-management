# Backlog of vectors

<sub>Last regenerated: 2026-06-24 22:49 UTC by routine</sub>

| Score | Vector | Title | Source | Updated | Status |
|-------|--------|-------|--------|---------|--------|
| 6 | test-gap | Add regression tests for inquiry mark_as_read cross-tenant IDOR fix (PR #497) | PR #497, PR #507 | 2026-05-26 | done |
| 3 | security | IDOR: ai.rs LLM-doc handlers publish/list/get any tenant's listing descriptions & photo enhancements unscoped | code-review api-core 2026-05-29, ai.rs:2620 | 2026-06-24 | done |
| 3 | bug | iOS SearchView.swift does not compile — performSearch/scheduleSearch undefined, resultsGrid corrupted | issue #1266, PR #1257 (verify) | 2026-06-24 | done |
| 3 | bug | Reality-web ComparisonUrlHandler hits non-existent /api/listings/${id} — every shared comparison URL 404s | rotating-expert-review reality-web 2026-06-14, ... | 2026-06-24 | done |
| 3 | bug | Reality-web listing detail SSR crashes on partial 200 body — JSON-LD build deref of undefined fields | rotating-expert-review reality-web 2026-06-14, ... | 2026-06-24 | done |
| 3 | bug | Reality-web RealtorManagement.tsx hardcoded English strings — agency flow not localized to sk/cs/de | rotating-expert-review reality-web 2026-06-14, ... | 2026-06-24 | done |
| 3 | bug | ReportFaultScreen.tsx handleSubmit() fakes API call with setTimeout(1500) — fault reports never reach backend (App.ts... | Phase 1.5 review of mobile-rn segment (2026-06-... | 2026-06-24 | done |
| 3 | bug | Revert: PR #1713 backs out #1690 (delegation frontend re-add) — feature stays retired | PR #1713 | 2026-06-24 | open |
| 3 | bug | AuthContext.tsx — initializeAuth() trusts expired access tokens (no JWT exp check) | rotating-expert-review | 2026-06-24 | ready |
| 3 | security | PR #1203 (fix(aml_dsa): close cross-tenant IDOR in moderation + AML-review handlers (PAP-36)) merged | PR #1203 | 2026-06-10 | dropped |
| 3 | security | PR #1193 (fix(aml-dsa): lock DSA reports to platform roles + fix file-path disclosure (PAP-47)) merg | PR #1193 | 2026-06-10 | dropped |
| 3 | bug | Schema drift: runtime SQL errors from non-existent columns in voting/messaging/notification paths | Issue #1008, PR #1009 | 2026-06-07 | done |
| 3 | security | IDOR: reality-server realtors mark_inquiry_read flips any realtor's inquiry by ID with no owner scoping | issue #519, PR #508 | 2026-05-26 | done |
| 3 | security | IDOR: equipment delete/update + maintenance update mutate any tenant's equipment by ID with no org scoping | code-review api-core 2026-05-25, ai.rs:1133 | 2026-05-25 | done |
| 3 | security | SSRF: signed-document fetch + webhook-test POST issue outbound requests to unvalidated user-controlled URLs | issue #439, signatures.rs:628 | 2026-05-25 | done |
| 3 | security | IDOR: unlink_voice_device deactivates any device by ID with no owner/org scoping | code-review api-core 2026-05-23, ai.rs:3002 | 2026-05-25 | done |
| 2 | bug | Mobile RN production screens (Buildings/Meters/Leases/PersonMonths/Notifications/Threads/Forms) render hardcoded MOCK... | Phase 1.5 review of mobile-rn segment (2026-06-16) | 2026-06-24 | open |
| 2 | bug | person-months.tsx — useBuildingUnits() bypasses @ppt/api-client (raw fetch, no Auth/X-Tenant headers) | rotating-expert-review | 2026-06-24 | open |
| 2 | bug | vote.rs:1765 calculate_question_result() uses partial_cmp().unwrap() on f64 — NaN/Inf weights panic /votes/{id}/results | code-review api-core 2026-06-15, PR #1417 | 2026-06-16 | done |
| 2 | test-gap | PR #1418 touched routes/** (faults.route.test.tsx) without updating docs/screens/ppt/* — heuristic, test-file fix | PR #1418 | 2026-06-16 | open |
| 2 | bug | useDeepLinkRouting.ts:27-36 — initialize() re-runs on onNavigate identity change + void promise with no .catch → dupl... | Phase 1.5 review of mobile-rn segment (2026-06-16) | 2026-06-16 | open |
| 2 | bug | iOS deep-link layer dead at runtime — Info.plist missing CFBundleURLTypes + applinks entitlement | issue #1267, PR #1256 (verify) | 2026-06-11 | open |
| 2 | test-gap | Webhook handlers RLS migration (PR #1288, PAP-170) shipped without a new regression test for repo-layer methods | PR #1288, PAP-170 | 2026-06-11 | open |
| 2 | test-gap | AI llm/sessions + integrations sync + subscriptions RLS migration (PR #1287, PAP-169) shipped without a new regressio... | PR #1287, PAP-169 | 2026-06-11 | open |
| 2 | test-gap | api_ecosystem.rs RLS migration (PR #1289, PAP-167) — 162-line handler rework shipped without a regression test for th... | PR #1289, PAP-167 | 2026-06-11 | open |
| 2 | test-gap | mfa.rs RLS migration (PR #1292, PAP-168) shipped without a regression test; also landed broken and was hotfixed in PR... | PR #1292, PR #1287 | 2026-06-11 | open |
| 2 | test-gap | PR #1196 (feat(ppt-web): add missing test coverage for faults feature) merged with 2 unchecked TODO  | PR #1196 | 2026-06-10 | dropped |
| 2 | dx | PushFanoutWorker BLPOP queue-drain deferred — Redis path is a logging no-op | PR #515, push_fanout.rs:621 | 2026-06-06 | done |
| 2 | refactor | ai.rs (3,134 LOC) — explicit module-split into routes/ai/{sessions,equipment,workflows,voice,llm,mod}.rs | pm-tech-lead analysis 2026-05-25, security-voic... | 2026-06-06 | done |
| 2 | refactor | announcements.rs churn-hot — 2,722 lines this run (Epic 2B + Epic 6 work) | git log origin/dev since 2026-05-24, PR #504 | 2026-06-06 | done |
| 2 | refactor | announcements.rs (2,722 LOC) — explicit module-split into routes/announcements/{crud,targeting,delivery,reactions,mod... | pm-tech-lead analysis 2026-05-25, PR #1110 | 2026-06-06 | done |
| 2 | refactor | Reduce App.tsx route-aggregator coupling (top churn hotspot, merge-conflict risk) | PR #474, PR #475 | 2026-06-06 | done |
| 2 | refactor | platform_admin.rs (2,762 LOC) — explicit module-split into routes/platform_admin/{tenants,features,billing,audit,mod}.rs | pm-tech-lead analysis 2026-05-25, PR #1109 | 2026-06-06 | done |
| 2 | test-gap | Screen-map drift: PR #1033 wired error/retry into AnnouncementsPage+FaultsPage via App.tsx without a docs/screens/ppt... | PR #1033, PR #1111 | 2026-06-06 | done |
| 2 | bug | Risky churn: mobile App.tsx deep-link/doc-detail wiring changing across back-to-back PRs without coverage | PR #1103, PR #962 | 2026-06-05 | done |
| 2 | dx | Integration marketplace install/OAuth flows are placeholders — wire backend handlers + UI navigation | PR #1105, PR #282 | 2026-06-05 | done |
| 2 | test-gap | Booking push availability/rates endpoints add batch-cap + non-negative guards with no regression test | PR #1068, PR #607 | 2026-06-05 | done |
| 2 | test-gap | Portal webhook fail-closed fix (PR #874) shipped without a regression test for unverified-signature rejection | PR #1052, PR #874 | 2026-06-05 | done |
| 2 | test-gap | Mobile dev-review batch (PR #918, 5 files under frontend/apps/mobile/src) shipped without a regression test | PR #1072, PR #918 | 2026-06-05 | done |
| 2 | test-gap | Reality-server SSO consumer review fix (PR #921, closes #820) shipped without a regression test | PR #1076, PR #921 | 2026-06-05 | done |
| 2 | test-gap | CI branch-protection + auto-rebase workflow change (PR #923) shipped without an integration test | PR #1057, PR #923 | 2026-06-05 | done |
| 2 | test-gap | deploy-server OIDC scope mapping (#939) shipped without unit test for derive_oidc_scopes | PR #1106, PR #939 | 2026-06-05 | done |
| 2 | test-gap | Mobile RN dev-review tail (#943) shipped without test coverage | PR #1080, PR #943 | 2026-06-05 | done |
| 2 | test-gap | Frontend gap-sweep (PR #990, 34 files across Epics 1/6/7B/9/10B/11/15/17/18) shipped without a regression test | PR #1081, PR #990 | 2026-06-05 | done |
| 2 | test-gap | Mobile document-detail wiring (PR #992) shipped without a regression test for the deep-link payload path | PR #1082, PR #992 | 2026-06-05 | done |
| 2 | test-gap | Screen-map drift: PR #839 modified ppt-web App.tsx (FileDisputePageRoute) without a docs/screens/ppt update | PR #1056, PR #839 | 2026-06-05 | done |
| 2 | refactor | ppt-web status/auth components hardcode English in an otherwise i18n'd app | code-review ppt-web-ui 2026-05-24, rotating-exp... | 2026-06-04 | done |
| 2 | bug | MediationWorkspacePage shows empty/unknown state instead of error UI on dispute fetch failure | PR #555, code-review 2026-05-27 | 2026-06-03 | done |
| 2 | bug | Mobile VotingScreen double-casts API result across boundary — render-time crash on unexpected shape | code-review mobile-rn 2026-05-27, rotating-expe... | 2026-06-03 | done |
| 2 | bug | Reality-web InviteRealtorModal swallows invite-mutation failure with no error UI | code-review reality-web 2026-05-28, rotating-ex... | 2026-06-03 | done |
| 2 | bug | Airbnb webhook at-least-once delivery enqueues duplicate SYNC_EXTERNAL jobs | PR #538, webhook.rs:1028 | 2026-06-03 | done |
| 2 | dx | DocumentsBrowse MoveFolderDialog cannot pre-select current folder (DocumentSummary lacks folder_id) | PR #623, PR #1031 | 2026-06-03 | done |
| 2 | test-gap | API + SPA security-headers middleware (PR #963) shipped without an assertion test for HSTS/nosniff/CSP | PR #963, issue #954 | 2026-06-03 | done |
| 2 | test-gap | api-server main.rs vs lib.rs::create_router diverge silently (5 routes unreachable in prod, no test asserts parity) | PR #866, issue #867 | 2026-06-01 | done |
| 2 | bug | ReportSchedule.update_schedule stores cron in `time` workaround; documented UPDATE never runs (missing cron_expressio... | PR #611, issue #616 | 2026-05-30 | done |
| 2 | test-gap | Screen-map drift: report execution-history route (PR #547) added without a ppt screen doc | PR #547, frontend/apps/ppt-web/src/routes/lazyR... | 2026-05-27 | done |
| 2 | test-gap | Dispute state machine (PR #506) shipped with no tests + no org predicate on update_status | PR #506, issue #520 | 2026-05-26 | done |
| 2 | refactor | documents.rs churn-hot — 10,659 lines over 14d | git log origin/main since 2026-05-06, git log o... | 2026-05-25 | done |
| 2 | refactor | integrations.rs churn-hot — 12,977 lines over 14d, candidate for module split | git log origin/main since 2026-05-06, git log o... | 2026-05-25 | done |
| 2 | refactor | organizations.rs churn-hot — 12,060 lines over 14d (multitenancy + admin) | git log origin/main since 2026-05-06, git log o... | 2026-05-25 | done |
| 2 | security | IDOR: reality-server mark_as_read flips any realtor's inquiry by ID with no owner scoping | code-review reality-server 2026-05-23, inquirie... | 2026-05-25 | done |
| 2 | security | Latent fail-open: ProtectedRoute role check is skipped when user.role is falsy | code-review ppt-web-ui 2026-05-24, ProtectedRou... | 2026-05-25 | done |
| 2 | test-gap | Screen-map drift: PR #464 wired a neighbors route in ppt-web without a docs/screens/ppt entry | PR #464 | 2026-05-25 | done |
| 2 | test-gap | Screen-map drift: PR #460 touched reality-web listing page without a docs/screens/reality update | PR #460 | 2026-05-25 | closed |
| 2 | refactor | Dead/duplicate handler modules: AuthHandler & BuildingHandler unused, routes reimplement inline | code-review api-handlers 2026-05-23, PR #437 | 2026-05-24 | done |
| 2 | security | Complete RLS migration in 31 remaining handlers (voting, market_pricing, faults, notif_prefs, reports) | issue #160, PR #420 | 2026-05-23 | done |
| 1 | bug | DeepLinkRouter skips URL-decoding while Android Uri.getQueryParameter decodes — SSO tokens diverge per platform | mobile-native-kmp segment review 2026-06-06 | 2026-06-24 | open |
| 1 | bug | SearchScreen stale-response race — overlapping searches can clobber newer results | mobile-native-kmp segment review 2026-06-06, PR... | 2026-06-24 | open |
| 1 | test-gap | Screen-map drift: PR #1085 modified reality-web listing detail metadata + page without screen-doc update | PR #1085 | 2026-06-24 | open |
| 1 | test-gap | Screen-map drift: PR #1100 modified ppt-web App.tsx (FileDisputePageRoute extraction) without screen-doc update | PR #1100 | 2026-06-24 | open |
| 1 | bug | Risky churn: api-server main.rs security-headers wiring shipped without a middleware smoke test | PR #963 | 2026-06-24 | open |
| 1 | test-gap | Reality-server listings pagination clamp (PR #959) shipped without a regression test for limit=-1 | PR #959, issue #953 | 2026-06-24 | open |
| 1 | triage | Issue #1151 (no labels, OPEN): Research dispatcher: claimable buffer is stale — true claimable work = 0 despite metri... | #1151 | 2026-06-24 | dropped |
| 1 | refactor | Churn hotspot: ListingDetailScreen.kt — +1279 LOC this run (gap-82-4 reality mobile favorite toggle) | PR #1121 | 2026-06-24 | dropped |
| 1 | refactor | Churn hotspot: SearchScreen.kt — +1293 LOC this run (gap-82-3 reality mobile search/filters) | PR #1125 | 2026-06-24 | dropped |
| 1 | refactor | MainActivity reimplements deep-link dispatch instead of calling shared DeepLinkRouter — drift trap | mobile-native-kmp segment review 2026-06-06 | 2026-06-24 | dropped |
| 1 | refactor | Churn hotspot: AnnouncementsScreen.tsx — 4 PRs this run, instability proxy | PR #1101, PR #1077 | 2026-06-24 | dropped |
| 1 | refactor | Churn hotspot: AnnouncementsScreen.test.ts — 4 PRs this run, instability proxy | PR #1101, PR #1077 | 2026-06-24 | dropped |
| 1 | refactor | Churn hotspot: DocumentsScreen.tsx — 3 PRs this run | PR #1101, PR #1081 | 2026-06-24 | dropped |
| 1 | triage | Dispatcher action-list.json corruption when MCP push falls back from blocked git push | #1014 | 2026-06-24 | dropped |
| 1 | triage | Issue #951 (no labels, OPEN): Deploy blocker: api-server requires ESIGN_TOKEN_SECRET + ESIGN_WEBHOOK_SECRET not injec... | #951 | 2026-06-24 | dropped |
| 1 | refactor | Churn hotspot: backend/servers/api-server/src/routes/forms.rs touched 2x since 2026-06-12 (window 2026-06-12→2026-06-13) | local git numstat since 2026-06-12, local git n... | 2026-06-24 | done |
| 1 | refactor | Churn hotspot: backend/crates/db/tests/form_rls_repo_tests.rs touched 2x since 2026-06-12 (window 2026-06-12→2026-06-13) | local git numstat since 2026-06-12, PR #1719 | 2026-06-24 | done |
| 1 | refactor | Churn hotspot: 124 lines in frontend/apps/mobile/app.config.icon.test.ts (PR #1383 gap-85-2) | PR #1383, PR #1718 | 2026-06-24 | done |
| 1 | refactor | Churn hotspot: 53 lines in backend/crates/db/src/repositories/form.rs (PR #1379 #1332 unblock) | PR #1379, issue #1332 | 2026-06-24 | done |
| 1 | refactor | crypto.rs:127 SysRng.try_fill_bytes(...).expect() panics if OS CSPRNG errors during integration-credential encrypt | code-review api-core 2026-06-15, PR #1684 | 2026-06-24 | done |
| 1 | refactor | forms.rs repeated-churn — runs_seen=2 (#1337 explicit_auto_deref + #1397 org-scope hardening) | hotspot_history.runs_seen 1→2 with new churn th... | 2026-06-24 | done |
| 1 | dx | PR #1749 closed unmerged — see evidence for supersedence | PR #1749 | 2026-06-24 | open |
| 1 | dx | PR #1727 closed unmerged — see evidence for supersedence | PR #1727 | 2026-06-24 | open |
| 1 | dx | PR #1720 closed unmerged — see evidence for supersedence | PR #1720 | 2026-06-24 | open |
| 1 | dx | PR #1698 closed unmerged — see evidence for supersedence | PR #1698 | 2026-06-24 | open |
| 1 | dx | PR #1634 closed unmerged — see evidence for supersedence | PR #1634 | 2026-06-24 | open |
| 1 | test-gap | Screen-map drift: PR #922 modified ppt-web App.tsx (dev-review rounds 1-5 fixes) without a docs/screens/ppt update | PR #922 | 2026-06-16 | open |
| 1 | refactor | booking_oauth_csrf_tests.rs hotspot — 484-line NEW test file (PR #1393 #1424 OAuth CSRF coverage) | local git numstat since 2026-06-15 (commit 67c2... | 2026-06-16 | open |
| 1 | refactor | booking_oauth_routes_tests.rs hotspot — 381-line NEW test file (PR #1393 OAuth routes coverage) | local git numstat since 2026-06-15 | 2026-06-16 | open |
| 1 | dx | PR #1425 (GH #1377 document presigned-URL tests) closed unmerged — superseded by merged #1394 | PR #1425 | 2026-06-16 | open |
| 1 | dx | PR #1179 (docs(epics) catalog backfill for 37 mounted-but-undocumented backend modules) — stalled at 7d, no reviewDec... | PR #1179 | 2026-06-16 | open |
| 1 | triage | Issue #1380 (no labels, OPEN): Dispatcher stale gap-scan buffer + Tier-2 escalation endpoint misconfigured | issue #1380 | 2026-06-15 | open |
| 1 | refactor | PR #1378 closed without merge — DROP-OWNED-BY teardown theory for #1332 was wrong root cause, superseded by #1379 | PR #1378, PR #1379 | 2026-06-15 | done |
| 1 | refactor | Churn hotspot: 94 lines in frontend/apps/mobile/app.config.ts (PR #1383 gap-85-2) | PR #1383 | 2026-06-15 | open |
| 1 | triage | Issue #1331 (no labels, OPEN): Backend `test` job red/hanging on dev base — blocks the entire backend merge pipeline | #1331 | 2026-06-13 | open |
| 1 | dx | Stalled review: PR #988 (Epic: reusable Playwright E2E framework + sitemap FlowRunner) open 10d, no reviewDecision | PR #988 | 2026-06-13 | open |
| 1 | refactor | Churn hotspot: backend/servers/api-server/tests/reserve_funds_cross_org_idor_tests.rs touched 2x since 2026-06-12 (wi... | local git numstat since 2026-06-12 | 2026-06-13 | open |
| 1 | dx | PR #1274 (cargo-minor-patch group, /backend, 9 updates) closed unmerged — superseded by #1313 after auto-rebase fix l... | PR #1274 | 2026-06-12 | open |
| 1 | refactor | Churn hotspot: backend/crates/integrations/src/booking.rs (+404/−29 in PR #1294 Booking.com OTA retry) | PR #1294 commit 7ccce8a | 2026-06-12 | open |
| 1 | refactor | Churn hotspot: backend/servers/api-server/src/routes/api_ecosystem.rs (+106/−27 in PR #1293 PAP-171; second touch in ... | PR #1293 commit 1e50156 | 2026-06-12 | open |
| 1 | refactor | Churn hotspot: backend/crates/db/src/repositories/reality_portal.rs (+59/−28 in PR #1297 PAP-142 IDOR scoping) | PR #1297 commit 8c711c6 | 2026-06-12 | open |
| 1 | refactor | Churn hotspot: backend/servers/api-server/src/routes/iot.rs (+278/-403 in PR #1321/#1322 PAP-151 re-land + fmt) | PR #1321 commit, PR #1322 commit | 2026-06-12 | open |
| 1 | refactor | Churn hotspot: backend/servers/api-server/src/routes/reserve_funds.rs (+228/-255 in PR #1321 PAP-151 re-land) | PR #1321 commit | 2026-06-12 | open |
| 1 | refactor | Churn hotspot: backend/crates/db/src/repositories/sensor.rs (+248/-86 in PR #1321/#1322 PAP-151 re-land + fmt) | PR #1321 commit, PR #1322 commit | 2026-06-12 | open |
| 1 | refactor | Churn hotspot: 1021 lines changed in backend/servers/api-server/src/routes/emergency.rs (window 2026 | local git numstat since 2026-06-07 | 2026-06-10 | open |
| 1 | refactor | Churn hotspot: 929 lines changed in backend/servers/api-server/src/routes/vendors.rs (window 2026-06 | local git numstat since 2026-06-07 | 2026-06-10 | open |
| 1 | refactor | Churn hotspot: 709 lines changed in backend/servers/api-server/src/routes/enhanced_tenant_screening. | local git numstat since 2026-06-07 | 2026-06-10 | open |
| 1 | refactor | Churn hotspot: 2940 lines changed in backend/crates/db/src/repositories/document.rs (window 2026-06-10 03:05Z→18:30Z) | local git numstat since 2026-06-10T03:05:00Z | 2026-06-10 | open |
| 1 | refactor | Churn hotspot: 2856 lines changed in backend/crates/db/src/repositories/subscription.rs (window 2026-06-10 03:05Z→18:... | local git numstat since 2026-06-10T03:05:00Z, P... | 2026-06-10 | open |
| 1 | refactor | Churn hotspot: 2691 lines changed in backend/servers/api-server/src/routes/aml_dsa.rs (window 2026-06-10 03:05Z→18:30Z) | local git numstat since 2026-06-10T03:05:00Z, P... | 2026-06-10 | open |
| 1 | test-gap | PKCE unit test became a tautology after services/oauth.rs DRY refactor (#1132) | #1137, PR #1132 | 2026-06-07 | done |
| 1 | triage | Triage: dispatcher incident — assignments-archive.json corrupted to 1/196 rows on dev branch (#1061) | Issue #1061, #1061 closed | 2026-06-07 | done |
| 1 | triage | Issue #950 (no labels, OPEN): CI: trigger-deploy 403 marks all dev image builds red and blocks staging auto-deploy | #950, PR #1143 | 2026-06-07 | done |
| 1 | triage | Issue #952 (no labels, OPEN): [staging] Reality SSO login dead-ends: redirect_uri callback 404s on reality apex | #952, PR #1144 | 2026-06-07 | done |
| 1 | triage | Issue #769 (no labels, OPEN): Current dev review: Deploy server | #769, PR #1141 | 2026-06-07 | done |
| 1 | triage | Issue #789 (no labels, OPEN): Dev review rounds 6-10: scheduler, notifications, admin, orgs, buildings | #789, PR #1142 | 2026-06-07 | done |
| 1 | dx | docker/nginx admin-web + ppt-web templates churned twice this run (security headers + redirects) | PR #963, PR #964 | 2026-06-06 | done |
| 1 | refactor | ai.rs churn-hot — 3,142 lines this run; 3,142-line route monolith, candidate for module split | git log origin/dev since 2026-05-24, PR #1114 | 2026-06-06 | done |
| 1 | refactor | ppt-web e2e auth-refresh.spec.ts added (+252 lines, story 79-2 token-refresh coverage) | PR #1047, PR #1113 | 2026-06-06 | done |
| 1 | refactor | api-server esignature_webhook_idempotency_tests.rs added (+228 lines, terminal-state regression) | PR #1034, PR #1119 | 2026-06-06 | done |
| 1 | refactor | ppt-web EvidenceUploader.test.tsx added (+202 lines, dispute-filing AC-2 regression) | PR #1048, PR #1116 | 2026-06-06 | done |
| 1 | refactor | api-server main.rs touched twice this run (gap-sweep + security headers) — minor churn marker | PR #989, PR #963 | 2026-06-06 | done |
| 1 | refactor | Duplicated animate-spin spinner markup across mediation page + chat thread (no shared Spinner) | PR #555, code-review 2026-05-27 | 2026-06-06 | done |
| 1 | refactor | Mediation reference number uppercases full UUID (DSP-<uuid>) instead of a short code | PR #555, code-review 2026-05-27 | 2026-06-06 | done |
| 1 | refactor | frontend/apps/mobile/src/App.tsx churned twice this run (universal links + doc-detail wiring) | PR #962, PR #992 | 2026-06-06 | done |
| 1 | refactor | platform_admin.rs churn-hot — 2,762 lines this run (admin/OAuth-provider feature work) | git log origin/dev since 2026-05-24, PR #1109 | 2026-06-06 | done |
| 1 | refactor | Reality-web ComparisonUrlHandler hardcodes English loading/error strings | code-review reality-web 2026-05-28, rotating-ex... | 2026-06-06 | done |
| 1 | refactor | Watch routes/oauth.rs churn after audit-log + hardening PRs | PR #930, PR #933 | 2026-06-06 | done |
| 1 | refactor | Watch services/oauth.rs churn after introspect/revoke hardening (#933) | PR #933, PR #1132 | 2026-06-06 | done |
| 1 | test-gap | Mobile VotingScreen pure transforms toUiStatus/toUiVote have no tests | code-review mobile-rn 2026-05-27, rotating-expe... | 2026-06-06 | done |
| 1 | triage | Issue #749 (no labels, OPEN): Code review findings: Story 6.1 announcement creation and targeting | #749, issue #749 closed | 2026-06-06 | done |
| 1 | triage | Issue #755 (no labels, OPEN): Current dev review: Epic 8A Notification Preferences | #755, issue #755 closed | 2026-06-06 | done |
| 1 | triage | Issue #764 (no labels, OPEN): Current dev review: Admin MFA & Auth Hardening | #764, issue #764 closed | 2026-06-06 | done |
| 1 | triage | Issue #765 (no labels, OPEN): Current dev review: Integrations & Airbnb OAuth | #765, issue #765 closed | 2026-06-06 | done |
| 1 | bug | Mobile VotingScreen hardcodes en-US in toLocaleDateString — vote dates never localize | PR #1083, code-review mobile-rn 2026-05-27 | 2026-06-05 | done |
| 1 | bug | Reality-web listing generateMetadata can throw during SSR on malformed 200 body | PR #1085, code-review reality-web 2026-05-28 | 2026-06-05 | done |
| 1 | security | PR #908 (fix(security): require PKCE on OAuth authorization-code flow, closes #823) was closed unmerged — verify whet... | PR #908, PR #1025 | 2026-06-03 | done |
| 1 | triage | Issue #751 (no labels, OPEN): Current dev review: frontend/web/API-client findings | #751, #942 | 2026-06-02 | done |
| 1 | triage | Issue #752 (no labels, OPEN): Current dev review: mobile CI tooling findings | #752, #929 | 2026-06-02 | done |
| 1 | triage | Issue #756 (no labels, OPEN): Current dev review: Epic 10A OAuth Provider | #756, #934 | 2026-06-02 | done |
| 1 | triage | Issue #761 (no labels, OPEN): Current dev review: Epic 84 E-Signature & Leases | #761, #936 | 2026-06-02 | done |
| 1 | triage | Issue #763 (no labels, OPEN): Current dev review: Reality Server & Inquiries | #763, #935 | 2026-06-02 | done |
| 1 | triage | Issue #767 (no labels, OPEN): Current dev review: Mobile RN Property Management app | #767, #943 | 2026-06-02 | done |
| 1 | triage | Issue #768 (no labels, OPEN): Current dev review: Admin-web features (10B) | #768, #930 | 2026-06-02 | done |
| 1 | triage | Issue #920 (no labels, OPEN): Announcement targeting not enforced on read (intra-org disclosure) | #920, #944 | 2026-06-02 | done |
| 1 | triage | Issue #750 (no labels, OPEN): Current dev review: backend/API/database findings | #750, PR #922 | 2026-06-01 | done |
| 1 | triage | Issue #753 (no labels, OPEN): Current dev review: Epic 6 Announcements & Communication | #753 | 2026-06-01 | done |
| 1 | triage | Issue #754 (no labels, OPEN): Current dev review: Epic 7A Basic Document Management | #754, PR #914 | 2026-06-01 | done |
| 1 | triage | Issue #757 (no labels, OPEN): Current dev review: Epic 10B Platform Administration | #757 | 2026-06-01 | done |
| 1 | triage | Issue #760 (no labels, OPEN): Current dev review: Epic 79 Disputes & Mediation | #760, PR #915 | 2026-06-01 | done |
| 1 | triage | Issue #762 (no labels, OPEN): Current dev review: Reports & Schedules | #762 | 2026-06-01 | done |
| 1 | triage | Issue #766 (no labels, OPEN): Current dev review: AI & LLM routes | #766, PR #879 | 2026-06-01 | done |
| 1 | triage | Issue #770 (no labels, OPEN): Current dev review: Faults & triage | #770, PR #902 | 2026-06-01 | done |
| 1 | triage | Issue #771 (no labels, OPEN): Current dev review: Research dispatcher & CI automation | #771, PR #923 | 2026-06-01 | done |
| 1 | triage | Issue #772 (no labels, OPEN): Current dev review: Auth core (delta confirmation) | #772 | 2026-06-01 | done |
| 1 | triage | Issue #773 (no labels, OPEN): Current dev review: Leases & rental | #773 | 2026-06-01 | done |
| 1 | triage | Issue #774 (no labels, OPEN): Current dev review: Reality server (broad) | #774, PR #919 | 2026-06-01 | done |
| 1 | triage | Issue #775 (no labels, OPEN): Current dev review: WebSocket realtime | #775, PR #926 | 2026-06-01 | done |
| 1 | triage | Issue #776 (no labels, OPEN): Current dev review: Equipment & audit log | #776 | 2026-06-01 | done |
| 1 | triage | Issue #777 (no labels, OPEN): Current dev review: Compliance & GDPR | #777 | 2026-06-01 | done |
| 1 | triage | Issue #778 (no labels, OPEN): Current dev review: Marketplace, voting, investor portal, impersonation | #778, PR #882 | 2026-06-01 | done |
| 1 | triage | Issue #788 (no labels, OPEN): Dev review rounds 1-5: mobile-native + ppt-web surfaces | #788, PR #922 | 2026-06-01 | done |
| 1 | triage | Issue #790 (no labels, OPEN): Dev review rounds 11-15: vendor, predictive, reality-web, middleware | #790, PR #913 | 2026-06-01 | done |
| 1 | triage | Issue #791 (no labels, OPEN): Dev review rounds 16-20: push, e-sign, portal, webhooks, reserves | #791, PR #924 | 2026-06-01 | done |
| 1 | triage | Issue #846 (no labels, OPEN): Code review: Epics 12+65 — Meters & Energy/ESG (origin/dev) | #846, PR #880 | 2026-06-01 | done |
| 1 | triage | Issue #847 (no labels, OPEN): Code review: Reality-server — Inquiries IDOR (Epics 16–19) (origin/dev) | #847 | 2026-06-01 | done |
| 1 | triage | Issue #848 (no labels, OPEN): Code review: Epics 78+134 — Vendor portal stubs & Predictive maintenance gaps (origin/dev) | #848, PR #913 | 2026-06-01 | done |
| 1 | triage | Issue #850 (no labels, OPEN): Code review: Epics 61+146+42 — Multi-currency, Data residency, Violations (origin/dev) | #850, PR #883 | 2026-06-01 | done |
| 1 | triage | Issue #851 (no labels, OPEN): Code review: Epics 15+105+69 — Listings/syndication & Developer API stubs (origin/dev) | #851, PR #904 | 2026-06-01 | done |
| 1 | triage | Issue #859 (no labels, OPEN): sqlx 0.9 breaks runtime decode of Postgres enum columns into Rust String (SELECT * read... | #859, PR #871 | 2026-06-01 | done |
| 1 | triage | Issue #867 (no labels, OPEN): Tech debt: api-server main.rs duplicates lib.rs::create_router — routers diverge silently | #867, PR #870 | 2026-06-01 | done |
| 1 | triage | Issue #836 (no labels, OPEN): Code review: Epic 2B-C — Mobile push & device registration (origin/dev) | #836, PR #866 | 2026-05-31 | done |
| 1 | triage | Issue #845 (no labels, OPEN): Code review: Epic 14 — IoT alerts, correlations, thresholds (origin/dev) | #845, PR #862 | 2026-05-31 | done |
| 1 | triage | Issue #849 (no labels, OPEN): Code review: Epic 10B+143 — Admin impersonation, Help, Board meetings auth (origin/dev) | #849, PR #869 | 2026-05-31 | done |
| 0 | refactor | Stabilize oauth_integration_tests churn — heavy edits across 3 OAuth fix PRs | PR #930, PR #933 | 2026-06-16 | dropped |
| 0 | triage | Issue #779 (no labels, OPEN): Current dev review: consolidated priority rollup (origin/dev snapshot) | #779 | 2026-06-13 | dropped |
| 0 | bug | Announcer: untracked clear-then-set timeouts can resurrect a stale screen-reader message | code-review ppt-web-ui 2026-05-24, Announcer.ts... | 2026-06-07 | dropped |
| 0 | dx | Portfolio dashboard: alert mark-read/resolve mutations + property-card click navigation are no-op stubs | PR #328, commit 254f01d | 2026-06-04 | dropped |
