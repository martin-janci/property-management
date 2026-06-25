# Backlog of vectors

<sub>Last regenerated: 2026-06-25 20:17 UTC by routine</sub>

> **Canonical source:** `backlog.json`. This file is **regenerated** each run — do not edit by hand.

| Score | Vector | Title | Status | Source | Updated |
|-------|--------|-------|--------|--------|---------|
| 6 | test-gap | Add regression tests for inquiry mark_as_read cross-tenant IDOR fix (PR #497) | done | PR #497, PR #507 | 2026-05-26 |
| 3 | bug | Revert PR #1713 — delegation re-admission ruled out by CEO BIT-198 | open | PR #1713 | 2026-06-25 |
| 3 | bug | ReportFaultScreen.tsx handleSubmit() fakes API call with setTimeout(1500) — fault reports  | ready | Phase 1.5 review of mobile-rn segment (2026-06-16) | 2026-06-16 |
| 3 | bug | Reality-web RealtorManagement.tsx hardcoded English strings — agency flow not localized to | ready | rotating-expert-review reality-web 2026-06-14 | 2026-06-15 |
| 3 | bug | Reality-web ComparisonUrlHandler hits non-existent /api/listings/${id} — every shared comp | ready | rotating-expert-review reality-web 2026-06-14 | 2026-06-14 |
| 3 | bug | Reality-web listing detail SSR crashes on partial 200 body — JSON-LD build deref of undefi | ready | rotating-expert-review reality-web 2026-06-14 | 2026-06-14 |
| 3 | bug | iOS SearchView.swift does not compile — performSearch/scheduleSearch undefined, resultsGri | ready | issue #1266, PR #1257 (verify) | 2026-06-11 |
| 3 | security | PR #1203 (fix(aml_dsa): close cross-tenant IDOR in moderation + AML-review handlers (PAP-3 | dropped | PR #1203 | 2026-06-10 |
| 3 | security | PR #1193 (fix(aml-dsa): lock DSA reports to platform roles + fix file-path disclosure (PAP | dropped | PR #1193 | 2026-06-10 |
| 3 | bug | Schema drift: runtime SQL errors from non-existent columns in voting/messaging/notificatio | done | Issue #1008, PR #1009 | 2026-06-07 |
| 3 | security | IDOR: ai.rs LLM-doc handlers publish/list/get any tenant's listing descriptions & photo en | ready | code-review api-core 2026-05-29, ai.rs:2620 | 2026-06-01 |
| 3 | security | IDOR: reality-server realtors mark_inquiry_read flips any realtor's inquiry by ID with no  | done | issue #519, PR #508 | 2026-05-26 |
| 3 | security | IDOR: equipment delete/update + maintenance update mutate any tenant's equipment by ID wit | done | code-review api-core 2026-05-25, ai.rs:1133 | 2026-05-25 |
| 3 | security | SSRF: signed-document fetch + webhook-test POST issue outbound requests to unvalidated use | done | issue #439, signatures.rs:628 | 2026-05-25 |
| 3 | security | IDOR: unlink_voice_device deactivates any device by ID with no owner/org scoping | done | code-review api-core 2026-05-23, ai.rs:3002 | 2026-05-25 |
| 2 | refactor | Churn hotspot: 1021 lines changed in backend/servers/api-server/src/routes/emergency.rs (w | done | local git numstat since 2026-06-07, backend/server | 2026-06-25 |
| 2 | bug | Mobile RN production screens (Buildings/Meters/Leases/PersonMonths/Notifications/Threads/F | done | Phase 1.5 review of mobile-rn segment (2026-06-16) | 2026-06-25 |
| 2 | test-gap | Unchecked CI checkbox at merge: PR #1606 — feat(accounting): manager-only role gate for li | open | PR #1606 | 2026-06-25 |
| 2 | test-gap | Unchecked CI checkbox at merge: PR #1723 — fix(financial): rustfmt compliance for financia | open | PR #1723 | 2026-06-25 |
| 2 | bug | vote.rs:1765 calculate_question_result() uses partial_cmp().unwrap() on f64 — NaN/Inf weig | done | code-review api-core 2026-06-15, PR #1417 | 2026-06-16 |
| 2 | test-gap | PR #1418 touched routes/** (faults.route.test.tsx) without updating docs/screens/ppt/* — h | open | PR #1418 | 2026-06-16 |
| 2 | bug | useDeepLinkRouting.ts:27-36 — initialize() re-runs on onNavigate identity change + void pr | open | Phase 1.5 review of mobile-rn segment (2026-06-16) | 2026-06-16 |
| 2 | test-gap | PR #1196 (feat(ppt-web): add missing test coverage for faults feature) merged with 2 unche | dropped | PR #1196 | 2026-06-10 |
| 2 | dx | PushFanoutWorker BLPOP queue-drain deferred — Redis path is a logging no-op | done | PR #515, push_fanout.rs:621 | 2026-06-06 |
| 2 | refactor | ai.rs (3,134 LOC) — explicit module-split into routes/ai/{sessions,equipment,workflows,voi | done | pm-tech-lead analysis 2026-05-25, security-voice-d | 2026-06-06 |
| 2 | refactor | announcements.rs churn-hot — 2,722 lines this run (Epic 2B + Epic 6 work) | done | git log origin/dev since 2026-05-24, PR #504 | 2026-06-06 |
| 2 | refactor | announcements.rs (2,722 LOC) — explicit module-split into routes/announcements/{crud,targe | done | pm-tech-lead analysis 2026-05-25, PR #1110 | 2026-06-06 |
| 2 | refactor | Reduce App.tsx route-aggregator coupling (top churn hotspot, merge-conflict risk) | done | PR #474, PR #475 | 2026-06-06 |
| 2 | refactor | platform_admin.rs (2,762 LOC) — explicit module-split into routes/platform_admin/{tenants, | done | pm-tech-lead analysis 2026-05-25, PR #1109 | 2026-06-06 |
| 2 | test-gap | Screen-map drift: PR #1033 wired error/retry into AnnouncementsPage+FaultsPage via App.tsx | done | PR #1033, PR #1111 | 2026-06-06 |
| 2 | bug | Risky churn: mobile App.tsx deep-link/doc-detail wiring changing across back-to-back PRs w | done | PR #1103, PR #962 | 2026-06-05 |
| 2 | dx | Integration marketplace install/OAuth flows are placeholders — wire backend handlers + UI  | done | PR #1105, PR #282 | 2026-06-05 |
| 2 | test-gap | Booking push availability/rates endpoints add batch-cap + non-negative guards with no regr | done | PR #1068, PR #607 | 2026-06-05 |
| 2 | test-gap | Portal webhook fail-closed fix (PR #874) shipped without a regression test for unverified- | done | PR #1052, PR #874 | 2026-06-05 |
| 2 | test-gap | Mobile dev-review batch (PR #918, 5 files under frontend/apps/mobile/src) shipped without  | done | PR #1072, PR #918 | 2026-06-05 |
| 2 | test-gap | Reality-server SSO consumer review fix (PR #921, closes #820) shipped without a regression | done | PR #1076, PR #921 | 2026-06-05 |
| 2 | test-gap | CI branch-protection + auto-rebase workflow change (PR #923) shipped without an integratio | done | PR #1057, PR #923 | 2026-06-05 |
| 2 | test-gap | deploy-server OIDC scope mapping (#939) shipped without unit test for derive_oidc_scopes | done | PR #1106, PR #939 | 2026-06-05 |
| 2 | test-gap | Mobile RN dev-review tail (#943) shipped without test coverage | done | PR #1080, PR #943 | 2026-06-05 |
| 2 | test-gap | Frontend gap-sweep (PR #990, 34 files across Epics 1/6/7B/9/10B/11/15/17/18) shipped witho | done | PR #1081, PR #990 | 2026-06-05 |
| 2 | test-gap | Mobile document-detail wiring (PR #992) shipped without a regression test for the deep-lin | done | PR #1082, PR #992 | 2026-06-05 |
| 2 | test-gap | Screen-map drift: PR #839 modified ppt-web App.tsx (FileDisputePageRoute) without a docs/s | done | PR #1056, PR #839 | 2026-06-05 |
| 2 | refactor | ppt-web status/auth components hardcode English in an otherwise i18n'd app | done | code-review ppt-web-ui 2026-05-24, rotating-expert | 2026-06-04 |
| 2 | bug | MediationWorkspacePage shows empty/unknown state instead of error UI on dispute fetch fail | done | PR #555, code-review 2026-05-27 | 2026-06-03 |
| 2 | bug | Mobile VotingScreen double-casts API result across boundary — render-time crash on unexpec | done | code-review mobile-rn 2026-05-27, rotating-expert- | 2026-06-03 |
| 2 | bug | Reality-web InviteRealtorModal swallows invite-mutation failure with no error UI | done | code-review reality-web 2026-05-28, rotating-exper | 2026-06-03 |
| 2 | bug | Airbnb webhook at-least-once delivery enqueues duplicate SYNC_EXTERNAL jobs | done | PR #538, webhook.rs:1028 | 2026-06-03 |
| 2 | dx | DocumentsBrowse MoveFolderDialog cannot pre-select current folder (DocumentSummary lacks f | done | PR #623, PR #1031 | 2026-06-03 |
| 2 | test-gap | API + SPA security-headers middleware (PR #963) shipped without an assertion test for HSTS | done | PR #963, issue #954 | 2026-06-03 |
| 2 | test-gap | api-server main.rs vs lib.rs::create_router diverge silently (5 routes unreachable in prod | done | PR #866, issue #867 | 2026-06-01 |
| 2 | bug | ReportSchedule.update_schedule stores cron in `time` workaround; documented UPDATE never r | done | PR #611, issue #616 | 2026-05-30 |
| 2 | test-gap | Screen-map drift: report execution-history route (PR #547) added without a ppt screen doc | done | PR #547, frontend/apps/ppt-web/src/routes/lazyRout | 2026-05-27 |
| 2 | test-gap | Dispute state machine (PR #506) shipped with no tests + no org predicate on update_status | done | PR #506, issue #520 | 2026-05-26 |
| 2 | refactor | documents.rs churn-hot — 10,659 lines over 14d | done | git log origin/main since 2026-05-06, git log orig | 2026-05-25 |
| 2 | refactor | integrations.rs churn-hot — 12,977 lines over 14d, candidate for module split | done | git log origin/main since 2026-05-06, git log orig | 2026-05-25 |
| 2 | refactor | organizations.rs churn-hot — 12,060 lines over 14d (multitenancy + admin) | done | git log origin/main since 2026-05-06, git log orig | 2026-05-25 |
| 2 | security | IDOR: reality-server mark_as_read flips any realtor's inquiry by ID with no owner scoping | done | code-review reality-server 2026-05-23, inquiries.r | 2026-05-25 |
| 2 | security | Latent fail-open: ProtectedRoute role check is skipped when user.role is falsy | done | code-review ppt-web-ui 2026-05-24, ProtectedRoute. | 2026-05-25 |
| 2 | test-gap | Screen-map drift: PR #464 wired a neighbors route in ppt-web without a docs/screens/ppt en | done | PR #464 | 2026-05-25 |
| 2 | test-gap | Screen-map drift: PR #460 touched reality-web listing page without a docs/screens/reality  | closed | PR #460 | 2026-05-25 |
| 2 | refactor | Dead/duplicate handler modules: AuthHandler & BuildingHandler unused, routes reimplement i | done | code-review api-handlers 2026-05-23, PR #437 | 2026-05-24 |
| 2 | security | Complete RLS migration in 31 remaining handlers (voting, market_pricing, faults, notif_pre | done | issue #160, PR #420 | 2026-05-23 |
| 1 | bug | DeepLinkRouter skips URL-decoding while Android Uri.getQueryParameter decodes — SSO tokens | open | mobile-native-kmp segment review 2026-06-06 | 2026-06-25 |
| 1 | bug | SearchScreen stale-response race — overlapping searches can clobber newer results | open | mobile-native-kmp segment review 2026-06-06, PR #1 | 2026-06-25 |
| 1 | test-gap | Screen-map drift: PR #1085 modified reality-web listing detail metadata + page without scr | open | PR #1085 | 2026-06-25 |
| 1 | test-gap | Screen-map drift: PR #1100 modified ppt-web App.tsx (FileDisputePageRoute extraction) with | open | PR #1100 | 2026-06-25 |
| 1 | bug | Risky churn: api-server main.rs security-headers wiring shipped without a middleware smoke | open | PR #963 | 2026-06-25 |
| 1 | test-gap | Reality-server listings pagination clamp (PR #959) shipped without a regression test for l | open | PR #959, issue #953 | 2026-06-25 |
| 1 | bug | iOS deep-link layer dead at runtime — Info.plist missing CFBundleURLTypes + applinks entit | open | issue #1267, PR #1256 (verify) | 2026-06-25 |
| 1 | test-gap | Webhook handlers RLS migration (PR #1288, PAP-170) shipped without a new regression test f | open | PR #1288, PAP-170 | 2026-06-25 |
| 1 | test-gap | AI llm/sessions + integrations sync + subscriptions RLS migration (PR #1287, PAP-169) ship | open | PR #1287, PAP-169 | 2026-06-25 |
| 1 | test-gap | api_ecosystem.rs RLS migration (PR #1289, PAP-167) — 162-line handler rework shipped witho | open | PR #1289, PAP-167 | 2026-06-25 |
| 1 | test-gap | mfa.rs RLS migration (PR #1292, PAP-168) shipped without a regression test; also landed br | open | PR #1292, PR #1287 | 2026-06-25 |
| 1 | refactor | Churn hotspot: backend/crates/integrations/src/booking.rs (+404/−29 in PR #1294 Booking.co | done | PR #1294 commit 7ccce8a, PR #1693 | 2026-06-25 |
| 1 | refactor | Churn hotspot: backend/servers/api-server/src/routes/reserve_funds.rs (+228/-255 in PR #13 | done | PR #1321 commit, PR #1816 | 2026-06-25 |
| 1 | refactor | Churn hotspot: backend/crates/db/src/repositories/sensor.rs (+248/-86 in PR #1321/#1322 PA | done | PR #1321 commit, PR #1322 commit | 2026-06-25 |
| 1 | refactor | Churn hotspot: backend/crates/db/tests/form_rls_repo_tests.rs touched 2x since 2026-06-12  | done | local git numstat since 2026-06-12, PR #1719 | 2026-06-25 |
| 1 | refactor | Churn hotspot: 124 lines in frontend/apps/mobile/app.config.icon.test.ts (PR #1383 gap-85- | done | PR #1383, PR #1718 | 2026-06-25 |
| 1 | refactor | crypto.rs:127 SysRng.try_fill_bytes(...).expect() panics if OS CSPRNG errors during integr | done | code-review api-core 2026-06-15, PR #1684 | 2026-06-25 |
| 1 | refactor | forms.rs repeated-churn — runs_seen=2 (#1337 explicit_auto_deref + #1397 org-scope hardeni | done | hotspot_history.runs_seen 1→2 with new churn this  | 2026-06-25 |
| 1 | refactor | Churn hotspot: backend/crates/db/src/repositories/rental.rs | open | backend/crates/db/src/repositories/rental.rs | 2026-06-25 |
| 1 | refactor | Churn hotspot: backend/crates/integrations/src/booking/mod.rs | open | backend/crates/integrations/src/booking/mod.rs | 2026-06-25 |
| 1 | test-gap | Screen-map drift: PR #922 modified ppt-web App.tsx (dev-review rounds 1-5 fixes) without a | open | PR #922 | 2026-06-16 |
| 1 | refactor | Churn hotspot: backend/servers/api-server/src/routes/forms.rs touched 2x since 2026-06-12  | open | local git numstat since 2026-06-12, local git nums | 2026-06-16 |
| 1 | refactor | booking_oauth_csrf_tests.rs hotspot — 484-line NEW test file (PR #1393 #1424 OAuth CSRF co | open | local git numstat since 2026-06-15 (commit 67c24bd | 2026-06-16 |
| 1 | refactor | booking_oauth_routes_tests.rs hotspot — 381-line NEW test file (PR #1393 OAuth routes cove | open | local git numstat since 2026-06-15 | 2026-06-16 |
| 1 | dx | PR #1425 (GH #1377 document presigned-URL tests) closed unmerged — superseded by merged #1 | open | PR #1425 | 2026-06-16 |
| 1 | dx | PR #1179 (docs(epics) catalog backfill for 37 mounted-but-undocumented backend modules) —  | open | PR #1179 | 2026-06-16 |
| 1 | triage | Issue #1380 (no labels, OPEN): Dispatcher stale gap-scan buffer + Tier-2 escalation endpoi | open | issue #1380 | 2026-06-15 |
| 1 | refactor | PR #1378 closed without merge — DROP-OWNED-BY teardown theory for #1332 was wrong root cau | done | PR #1378, PR #1379 | 2026-06-15 |
| 1 | refactor | Churn hotspot: 94 lines in frontend/apps/mobile/app.config.ts (PR #1383 gap-85-2) | open | PR #1383 | 2026-06-15 |
| 1 | refactor | Churn hotspot: 53 lines in backend/crates/db/src/repositories/form.rs (PR #1379 #1332 unbl | open | PR #1379, issue #1332 | 2026-06-15 |
| 1 | triage | Issue #1331 (no labels, OPEN): Backend `test` job red/hanging on dev base — blocks the ent | open | #1331 | 2026-06-13 |
| 1 | dx | Stalled review: PR #988 (Epic: reusable Playwright E2E framework + sitemap FlowRunner) ope | open | PR #988 | 2026-06-13 |
| 1 | refactor | Churn hotspot: backend/servers/api-server/tests/reserve_funds_cross_org_idor_tests.rs touc | open | local git numstat since 2026-06-12 | 2026-06-13 |
| 1 | dx | PR #1274 (cargo-minor-patch group, /backend, 9 updates) closed unmerged — superseded by #1 | open | PR #1274 | 2026-06-12 |
| 1 | refactor | Churn hotspot: backend/servers/api-server/src/routes/api_ecosystem.rs (+106/−27 in PR #129 | open | PR #1293 commit 1e50156 | 2026-06-12 |
| 1 | refactor | Churn hotspot: backend/crates/db/src/repositories/reality_portal.rs (+59/−28 in PR #1297 P | open | PR #1297 commit 8c711c6 | 2026-06-12 |
| 1 | refactor | Churn hotspot: backend/servers/api-server/src/routes/iot.rs (+278/-403 in PR #1321/#1322 P | open | PR #1321 commit, PR #1322 commit | 2026-06-12 |
| 1 | test-gap | PKCE unit test became a tautology after services/oauth.rs DRY refactor (#1132) | done | #1137, PR #1132 | 2026-06-07 |
| 1 | triage | Triage: dispatcher incident — assignments-archive.json corrupted to 1/196 rows on dev bran | done | Issue #1061, #1061 closed | 2026-06-07 |
| 1 | triage | Issue #950 (no labels, OPEN): CI: trigger-deploy 403 marks all dev image builds red and bl | done | #950, PR #1143 | 2026-06-07 |
| 1 | triage | Issue #952 (no labels, OPEN): [staging] Reality SSO login dead-ends: redirect_uri callback | done | #952, PR #1144 | 2026-06-07 |
| 1 | triage | Issue #769 (no labels, OPEN): Current dev review: Deploy server | done | #769, PR #1141 | 2026-06-07 |
| 1 | triage | Issue #789 (no labels, OPEN): Dev review rounds 6-10: scheduler, notifications, admin, org | done | #789, PR #1142 | 2026-06-07 |
| 1 | dx | docker/nginx admin-web + ppt-web templates churned twice this run (security headers + redi | done | PR #963, PR #964 | 2026-06-06 |
| 1 | refactor | ai.rs churn-hot — 3,142 lines this run; 3,142-line route monolith, candidate for module sp | done | git log origin/dev since 2026-05-24, PR #1114 | 2026-06-06 |
| 1 | refactor | ppt-web e2e auth-refresh.spec.ts added (+252 lines, story 79-2 token-refresh coverage) | done | PR #1047, PR #1113 | 2026-06-06 |
| 1 | refactor | api-server esignature_webhook_idempotency_tests.rs added (+228 lines, terminal-state regre | done | PR #1034, PR #1119 | 2026-06-06 |
| 1 | refactor | ppt-web EvidenceUploader.test.tsx added (+202 lines, dispute-filing AC-2 regression) | done | PR #1048, PR #1116 | 2026-06-06 |
| 1 | refactor | api-server main.rs touched twice this run (gap-sweep + security headers) — minor churn mar | done | PR #989, PR #963 | 2026-06-06 |
| 1 | refactor | Duplicated animate-spin spinner markup across mediation page + chat thread (no shared Spin | done | PR #555, code-review 2026-05-27 | 2026-06-06 |
| 1 | refactor | Mediation reference number uppercases full UUID (DSP-<uuid>) instead of a short code | done | PR #555, code-review 2026-05-27 | 2026-06-06 |
| 1 | refactor | frontend/apps/mobile/src/App.tsx churned twice this run (universal links + doc-detail wiri | done | PR #962, PR #992 | 2026-06-06 |
| 1 | refactor | platform_admin.rs churn-hot — 2,762 lines this run (admin/OAuth-provider feature work) | done | git log origin/dev since 2026-05-24, PR #1109 | 2026-06-06 |
| 1 | refactor | Reality-web ComparisonUrlHandler hardcodes English loading/error strings | done | code-review reality-web 2026-05-28, rotating-exper | 2026-06-06 |
| 1 | refactor | Watch routes/oauth.rs churn after audit-log + hardening PRs | done | PR #930, PR #933 | 2026-06-06 |
| 1 | refactor | Watch services/oauth.rs churn after introspect/revoke hardening (#933) | done | PR #933, PR #1132 | 2026-06-06 |
| 1 | test-gap | Mobile VotingScreen pure transforms toUiStatus/toUiVote have no tests | done | code-review mobile-rn 2026-05-27, rotating-expert- | 2026-06-06 |
| 1 | triage | Issue #749 (no labels, OPEN): Code review findings: Story 6.1 announcement creation and ta | done | #749, issue #749 closed | 2026-06-06 |
| 1 | triage | Issue #755 (no labels, OPEN): Current dev review: Epic 8A Notification Preferences | done | #755, issue #755 closed | 2026-06-06 |
| 1 | triage | Issue #764 (no labels, OPEN): Current dev review: Admin MFA & Auth Hardening | done | #764, issue #764 closed | 2026-06-06 |
| 1 | triage | Issue #765 (no labels, OPEN): Current dev review: Integrations & Airbnb OAuth | done | #765, issue #765 closed | 2026-06-06 |
| 1 | bug | Mobile VotingScreen hardcodes en-US in toLocaleDateString — vote dates never localize | done | PR #1083, code-review mobile-rn 2026-05-27 | 2026-06-05 |
| 1 | bug | Reality-web listing generateMetadata can throw during SSR on malformed 200 body | done | PR #1085, code-review reality-web 2026-05-28 | 2026-06-05 |
| 1 | security | PR #908 (fix(security): require PKCE on OAuth authorization-code flow, closes #823) was cl | done | PR #908, PR #1025 | 2026-06-03 |
| 1 | triage | Issue #751 (no labels, OPEN): Current dev review: frontend/web/API-client findings | done | #751, #942 | 2026-06-02 |
| 1 | triage | Issue #752 (no labels, OPEN): Current dev review: mobile CI tooling findings | done | #752, #929 | 2026-06-02 |
| 1 | triage | Issue #756 (no labels, OPEN): Current dev review: Epic 10A OAuth Provider | done | #756, #934 | 2026-06-02 |
| 1 | triage | Issue #761 (no labels, OPEN): Current dev review: Epic 84 E-Signature & Leases | done | #761, #936 | 2026-06-02 |
| 1 | triage | Issue #763 (no labels, OPEN): Current dev review: Reality Server & Inquiries | done | #763, #935 | 2026-06-02 |
| 1 | triage | Issue #767 (no labels, OPEN): Current dev review: Mobile RN Property Management app | done | #767, #943 | 2026-06-02 |
| 1 | triage | Issue #768 (no labels, OPEN): Current dev review: Admin-web features (10B) | done | #768, #930 | 2026-06-02 |
| 1 | triage | Issue #920 (no labels, OPEN): Announcement targeting not enforced on read (intra-org discl | done | #920, #944 | 2026-06-02 |
| 1 | triage | Issue #750 (no labels, OPEN): Current dev review: backend/API/database findings | done | #750, PR #922 | 2026-06-01 |
| 1 | triage | Issue #753 (no labels, OPEN): Current dev review: Epic 6 Announcements & Communication | done | #753 | 2026-06-01 |
| 1 | triage | Issue #754 (no labels, OPEN): Current dev review: Epic 7A Basic Document Management | done | #754, PR #914 | 2026-06-01 |
| 1 | triage | Issue #757 (no labels, OPEN): Current dev review: Epic 10B Platform Administration | done | #757 | 2026-06-01 |
| 1 | triage | Issue #760 (no labels, OPEN): Current dev review: Epic 79 Disputes & Mediation | done | #760, PR #915 | 2026-06-01 |
| 1 | triage | Issue #762 (no labels, OPEN): Current dev review: Reports & Schedules | done | #762 | 2026-06-01 |
| 1 | triage | Issue #766 (no labels, OPEN): Current dev review: AI & LLM routes | done | #766, PR #879 | 2026-06-01 |
| 1 | triage | Issue #770 (no labels, OPEN): Current dev review: Faults & triage | done | #770, PR #902 | 2026-06-01 |
| 1 | triage | Issue #771 (no labels, OPEN): Current dev review: Research dispatcher & CI automation | done | #771, PR #923 | 2026-06-01 |
| 1 | triage | Issue #772 (no labels, OPEN): Current dev review: Auth core (delta confirmation) | done | #772 | 2026-06-01 |
| 1 | triage | Issue #773 (no labels, OPEN): Current dev review: Leases & rental | done | #773 | 2026-06-01 |
| 1 | triage | Issue #774 (no labels, OPEN): Current dev review: Reality server (broad) | done | #774, PR #919 | 2026-06-01 |
| 1 | triage | Issue #775 (no labels, OPEN): Current dev review: WebSocket realtime | done | #775, PR #926 | 2026-06-01 |
| 1 | triage | Issue #776 (no labels, OPEN): Current dev review: Equipment & audit log | done | #776 | 2026-06-01 |
| 1 | triage | Issue #777 (no labels, OPEN): Current dev review: Compliance & GDPR | done | #777 | 2026-06-01 |
| 1 | triage | Issue #778 (no labels, OPEN): Current dev review: Marketplace, voting, investor portal, im | done | #778, PR #882 | 2026-06-01 |
| 1 | triage | Issue #788 (no labels, OPEN): Dev review rounds 1-5: mobile-native + ppt-web surfaces | done | #788, PR #922 | 2026-06-01 |
| 1 | triage | Issue #790 (no labels, OPEN): Dev review rounds 11-15: vendor, predictive, reality-web, mi | done | #790, PR #913 | 2026-06-01 |
| 1 | triage | Issue #791 (no labels, OPEN): Dev review rounds 16-20: push, e-sign, portal, webhooks, res | done | #791, PR #924 | 2026-06-01 |
| 1 | triage | Issue #846 (no labels, OPEN): Code review: Epics 12+65 — Meters & Energy/ESG (origin/dev) | done | #846, PR #880 | 2026-06-01 |
| 1 | triage | Issue #847 (no labels, OPEN): Code review: Reality-server — Inquiries IDOR (Epics 16–19) ( | done | #847 | 2026-06-01 |
| 1 | triage | Issue #848 (no labels, OPEN): Code review: Epics 78+134 — Vendor portal stubs & Predictive | done | #848, PR #913 | 2026-06-01 |
| 1 | triage | Issue #850 (no labels, OPEN): Code review: Epics 61+146+42 — Multi-currency, Data residenc | done | #850, PR #883 | 2026-06-01 |
| 1 | triage | Issue #851 (no labels, OPEN): Code review: Epics 15+105+69 — Listings/syndication & Develo | done | #851, PR #904 | 2026-06-01 |
| 1 | triage | Issue #859 (no labels, OPEN): sqlx 0.9 breaks runtime decode of Postgres enum columns into | done | #859, PR #871 | 2026-06-01 |
| 1 | triage | Issue #867 (no labels, OPEN): Tech debt: api-server main.rs duplicates lib.rs::create_rout | done | #867, PR #870 | 2026-06-01 |
| 1 | triage | Issue #836 (no labels, OPEN): Code review: Epic 2B-C — Mobile push & device registration ( | done | #836, PR #866 | 2026-05-31 |
| 1 | triage | Issue #845 (no labels, OPEN): Code review: Epic 14 — IoT alerts, correlations, thresholds  | done | #845, PR #862 | 2026-05-31 |
| 1 | triage | Issue #849 (no labels, OPEN): Code review: Epic 10B+143 — Admin impersonation, Help, Board | done | #849, PR #869 | 2026-05-31 |
| 0 | refactor | Churn hotspot: 929 lines changed in backend/servers/api-server/src/routes/vendors.rs (wind | dropped | local git numstat since 2026-06-07 | 2026-06-25 |
| 0 | refactor | Churn hotspot: 709 lines changed in backend/servers/api-server/src/routes/enhanced_tenant_ | dropped | local git numstat since 2026-06-07 | 2026-06-25 |
| 0 | refactor | Churn hotspot: 2940 lines changed in backend/crates/db/src/repositories/document.rs (windo | dropped | local git numstat since 2026-06-10T03:05:00Z | 2026-06-25 |
| 0 | refactor | Churn hotspot: 2856 lines changed in backend/crates/db/src/repositories/subscription.rs (w | dropped | local git numstat since 2026-06-10T03:05:00Z, PR # | 2026-06-25 |
| 0 | refactor | Churn hotspot: 2691 lines changed in backend/servers/api-server/src/routes/aml_dsa.rs (win | dropped | local git numstat since 2026-06-10T03:05:00Z, PR # | 2026-06-25 |
| 0 | triage | Issue #1151 (no labels, OPEN): Research dispatcher: claimable buffer is stale — true claim | dropped | #1151 | 2026-06-25 |
| 0 | refactor | Churn hotspot: ListingDetailScreen.kt — +1279 LOC this run (gap-82-4 reality mobile favori | dropped | PR #1121 | 2026-06-25 |
| 0 | refactor | Churn hotspot: SearchScreen.kt — +1293 LOC this run (gap-82-3 reality mobile search/filter | dropped | PR #1125 | 2026-06-25 |
| 0 | refactor | MainActivity reimplements deep-link dispatch instead of calling shared DeepLinkRouter — dr | dropped | mobile-native-kmp segment review 2026-06-06 | 2026-06-25 |
| 0 | refactor | Churn hotspot: AnnouncementsScreen.tsx — 4 PRs this run, instability proxy | dropped | PR #1101, PR #1077 | 2026-06-25 |
| 0 | refactor | Churn hotspot: AnnouncementsScreen.test.ts — 4 PRs this run, instability proxy | dropped | PR #1101, PR #1077 | 2026-06-25 |
| 0 | refactor | Churn hotspot: DocumentsScreen.tsx — 3 PRs this run | dropped | PR #1101, PR #1081 | 2026-06-25 |
| 0 | triage | Dispatcher action-list.json corruption when MCP push falls back from blocked git push | dropped | #1014 | 2026-06-25 |
| 0 | triage | Issue #951 (no labels, OPEN): Deploy blocker: api-server requires ESIGN_TOKEN_SECRET + ESI | dropped | #951 | 2026-06-25 |
| 0 | refactor | Stabilize oauth_integration_tests churn — heavy edits across 3 OAuth fix PRs | dropped | PR #930, PR #933 | 2026-06-16 |
| 0 | triage | Issue #779 (no labels, OPEN): Current dev review: consolidated priority rollup (origin/dev | dropped | #779 | 2026-06-13 |
| 0 | bug | Announcer: untracked clear-then-set timeouts can resurrect a stale screen-reader message | dropped | code-review ppt-web-ui 2026-05-24, Announcer.tsx:4 | 2026-06-07 |
| 0 | dx | Portfolio dashboard: alert mark-read/resolve mutations + property-card click navigation ar | dropped | PR #328, commit 254f01d | 2026-06-04 |
