# Backlog of vectors

<sub>Last regenerated: 2026-07-23 03:20 UTC by routine</sub>

| Score | Vector | Title | Status | Sources | Files | Updated |
|-------|--------|-------|--------|---------|-------|---------|
| 6 | test-gap | Add regression tests for inquiry mark_as_read cross-tenant IDOR fix (PR #497) | done | PR #497, PR #507 | backend/servers/reality-server/src/routes/inquiries.rs, b... | 2026-05-26 |
| 3 | security | [api-handlers] backend/servers/api-server/src/routes/marketplace.rs:922-951 — award_quote (wired ... | ready | rotating-expert-review | backend/servers/api-server/src/routes/marketplace.rs, bac... | 2026-07-23 |
| 3 | bug | revoke_all_sessions ignores refresh cookie — signs the caller out too | done | Phase 1.5 code-review 2026-07-09 (api-handlers segment) | backend/servers/api-server/src/routes/auth.rs | 2026-07-09 |
| 3 | bug | ReportFaultScreen.tsx handleSubmit() fakes API call with setTimeout(1500) — fault reports never r... | dropped | Phase 1.5 review of mobile-rn segment (2026-06-16) | frontend/apps/mobile/src/screens/faults/ReportFaultScreen... | 2026-06-16 |
| 3 | bug | Reality-web RealtorManagement.tsx hardcoded English strings — agency flow not localized to sk/cs/de | done | rotating-expert-review reality-web 2026-06-14 | frontend/apps/reality-web/src/components/agency/RealtorMa... | 2026-06-15 |
| 3 | bug | Reality-web ComparisonUrlHandler hits non-existent /api/listings/${id} — every shared comparison ... | dropped | rotating-expert-review reality-web 2026-06-14 | frontend/apps/reality-web/src/components/comparison/Compa... | 2026-06-14 |
| 3 | bug | Reality-web listing detail SSR crashes on partial 200 body — JSON-LD build deref of undefined fields | dropped | rotating-expert-review reality-web 2026-06-14 | frontend/apps/reality-web/src/app/[locale]/listings/[slug... | 2026-06-14 |
| 3 | bug | iOS SearchView.swift does not compile — performSearch/scheduleSearch undefined, resultsGrid corru... | dropped | issue #1266, PR #1257 (verify) | mobile-native/iosApp/iosApp/Features/Search/SearchView.swift | 2026-06-11 |
| 3 | security | PR #1203 (fix(aml_dsa): close cross-tenant IDOR in moderation + AML-review handlers (PAP-36)) merged | dropped | PR #1203 |  | 2026-06-10 |
| 3 | security | PR #1193 (fix(aml-dsa): lock DSA reports to platform roles + fix file-path disclosure (PAP-47)) merg | dropped | PR #1193 |  | 2026-06-10 |
| 3 | bug | Schema drift: runtime SQL errors from non-existent columns in voting/messaging/notification paths | done | Issue #1008, PR #1009 | backend/crates/db/src/repositories/vote.rs, backend/serve... | 2026-06-07 |
| 3 | security | IDOR: ai.rs LLM-doc handlers publish/list/get any tenant's listing descriptions & photo enhanceme... | dropped | code-review api-core 2026-05-29, ai.rs:2620 | backend/servers/api-server/src/routes/ai.rs, backend/crat... | 2026-06-01 |
| 3 | security | IDOR: reality-server realtors mark_inquiry_read flips any realtor's inquiry by ID with no owner s... | done | issue #519, PR #508 | backend/servers/reality-server/src/routes/realtors.rs, ba... | 2026-05-26 |
| 3 | security | IDOR: equipment delete/update + maintenance update mutate any tenant's equipment by ID with no or... | done | code-review api-core 2026-05-25, ai.rs:1133 | backend/servers/api-server/src/routes/ai.rs, backend/crat... | 2026-05-25 |
| 3 | security | SSRF: signed-document fetch + webhook-test POST issue outbound requests to unvalidated user-contr... | done | issue #439, signatures.rs:628 | backend/servers/api-server/src/routes/signatures.rs, back... | 2026-05-25 |
| 3 | security | IDOR: unlink_voice_device deactivates any device by ID with no owner/org scoping | done | code-review api-core 2026-05-23, ai.rs:3002 | backend/servers/api-server/src/routes/ai.rs, backend/crat... | 2026-05-25 |
| 2 | bug | [api-handlers] backend/servers/api-server/src/routes/layout/admin.rs:15-20 (bad_request helper) r... | open | rotating-expert-review | backend/servers/api-server/src/routes/layout/admin.rs | 2026-07-23 |
| 2 | test-gap | Hotfix without test: PR #2436 — PR #2436 'fix(api-server): log malformed announcement target_ids ... | open | PR #2436 | backend/servers/api-server/src/services/scheduler.rs | 2026-07-23 |
| 2 | test-gap | Hotfix without test: PR #2454 — PR #2454 'fix(dev-stack): repair bitrotted docker-compose dev flo... | open | PR #2454 | backend/servers/api-server/Cargo.toml, docker-compose.dev... | 2026-07-23 |
| 2 | test-gap | Screen-map drift: PR #2431 touched reality-web/src/app/api/layout-revalidate/route.ts without upd... | open | PR #2431 | frontend/apps/reality-web/src/app/api/layout-revalidate/r... | 2026-07-21 |
| 2 | bug | TenantSectionEditor PropInput silently JSON.parse-coerces every string prop on blur — override pa... | done | code-review ppt-web-ui 2026-07-21 | frontend/apps/ppt-web/src/features/layout/TenantSectionEd... | 2026-07-21 |
| 2 | bug | DashboardCustomizePage 'changed since sent' check is tautological — concurrent edits during in-fl... | done | code-review ppt-web-ui 2026-07-21 | frontend/apps/ppt-web/src/features/layout/DashboardCustom... | 2026-07-21 |
| 2 | bug | Dashboard useActionQueue queryFn returns generateMockData — production users see fabricated actio... | done | code-review ppt-web-ui 2026-07-21 | frontend/apps/ppt-web/src/features/dashboard/hooks/useAct... | 2026-07-21 |
| 2 | security | scheduler.rs units/buildings target queries lack organization_id AND-scope — fan-out can leak acr... | done | code-review api-core 2026-07-20 | backend/servers/api-server/src/services/scheduler.rs | 2026-07-20 |
| 2 | refactor | Churn hotspot cluster: api-server routes/auth.rs (runs_seen=3) + auth_tests.rs + reality-server r... | done | PR #2205, PR #2250 | backend/servers/api-server/src/routes/auth.rs, backend/se... | 2026-07-12 |
| 2 | security | /forgot-password and /resend-verification have no rate limit — mailbomb / token-clobber | done | Phase 1.5 code-review 2026-07-09 (api-handlers segment) | backend/servers/api-server/src/routes/auth.rs | 2026-07-09 |
| 2 | test-gap | Reality-server listings pagination clamp (PR #959) shipped without a regression test for limit=-1 | done | PR #959, issue #953 | backend/servers/reality-server/src/routes/listings.rs | 2026-07-05 |
| 2 | test-gap | PR #1418 touched routes/** (faults.route.test.tsx) without updating docs/screens/ppt/* — heuristi... | done | PR #1418, PR #2070 | frontend/apps/ppt-web/src/routes/groups/faults.route.test... | 2026-07-05 |
| 2 | bug | vote.rs:1765 calculate_question_result() uses partial_cmp().unwrap() on f64 — NaN/Inf weights pan... | done | code-review api-core 2026-06-15, PR #1417 | backend/crates/db/src/repositories/vote.rs | 2026-06-16 |
| 2 | test-gap | PR #1196 (feat(ppt-web): add missing test coverage for faults feature) merged with 2 unchecked TODO  | dropped | PR #1196 |  | 2026-06-10 |
| 2 | dx | PushFanoutWorker BLPOP queue-drain deferred — Redis path is a logging no-op | done | PR #515, push_fanout.rs:621 | backend/servers/api-server/src/services/push_fanout.rs | 2026-06-06 |
| 2 | refactor | ai.rs (3,134 LOC) — explicit module-split into routes/ai/{sessions,equipment,workflows,voice,llm,... | done | pm-tech-lead analysis 2026-05-25, security-voice-device-idor (PR #461) | backend/servers/api-server/src/routes/ai.rs | 2026-06-06 |
| 2 | refactor | announcements.rs churn-hot — 2,722 lines this run (Epic 2B + Epic 6 work) | done | git log origin/dev since 2026-05-24, PR #504 | backend/servers/api-server/src/routes/announcements.rs | 2026-06-06 |
| 2 | refactor | announcements.rs (2,722 LOC) — explicit module-split into routes/announcements/{crud,targeting,de... | done | pm-tech-lead analysis 2026-05-25, PR #1110 | backend/servers/api-server/src/routes/announcements.rs | 2026-06-06 |
| 2 | refactor | Reduce App.tsx route-aggregator coupling (top churn hotspot, merge-conflict risk) | done | PR #474, PR #475 | frontend/apps/ppt-web/src/App.tsx | 2026-06-06 |
| 2 | refactor | platform_admin.rs (2,762 LOC) — explicit module-split into routes/platform_admin/{tenants,feature... | done | pm-tech-lead analysis 2026-05-25, PR #1109 | backend/servers/api-server/src/routes/platform_admin.rs | 2026-06-06 |
| 2 | test-gap | Screen-map drift: PR #1033 wired error/retry into AnnouncementsPage+FaultsPage via App.tsx withou... | done | PR #1033, PR #1111 | frontend/apps/ppt-web/src/App.tsx, docs/screens/ppt/annou... | 2026-06-06 |
| 2 | bug | Risky churn: mobile App.tsx deep-link/doc-detail wiring changing across back-to-back PRs without ... | done | PR #1103, PR #962 | frontend/apps/mobile/src/App.tsx | 2026-06-05 |
| 2 | dx | Integration marketplace install/OAuth flows are placeholders — wire backend handlers + UI navigation | done | PR #1105, PR #282 | frontend/apps/ppt-web/src/features/api-ecosystem/pages/In... | 2026-06-05 |
| 2 | test-gap | Booking push availability/rates endpoints add batch-cap + non-negative guards with no regression ... | done | PR #1068, PR #607 | backend/servers/api-server/src/routes/integrations/instal... | 2026-06-05 |
| 2 | test-gap | Portal webhook fail-closed fix (PR #874) shipped without a regression test for unverified-signatu... | done | PR #1052, PR #874 | backend/servers/api-server/src/routes/portal_webhooks.rs | 2026-06-05 |
| 2 | test-gap | Mobile dev-review batch (PR #918, 5 files under frontend/apps/mobile/src) shipped without a regre... | done | PR #1072, PR #918 | frontend/apps/mobile/src/App.tsx, frontend/apps/mobile/sr... | 2026-06-05 |
| 2 | test-gap | Reality-server SSO consumer review fix (PR #921, closes #820) shipped without a regression test | done | PR #1076, PR #921 | backend/servers/reality-server/src/state.rs | 2026-06-05 |
| 2 | test-gap | CI branch-protection + auto-rebase workflow change (PR #923) shipped without an integration test | done | PR #1057, PR #923 | .github/workflows/auto-rebase-stale-drafts.yml, .github/w... | 2026-06-05 |
| 2 | test-gap | deploy-server OIDC scope mapping (#939) shipped without unit test for derive_oidc_scopes | done | PR #1106, PR #939 | backend/servers/deploy-server/src/infra/audit.rs | 2026-06-05 |
| 2 | test-gap | Mobile RN dev-review tail (#943) shipped without test coverage | done | PR #1080, PR #943 | frontend/apps/mobile/src/App.tsx, frontend/apps/mobile/sr... | 2026-06-05 |
| 2 | test-gap | Frontend gap-sweep (PR #990, 34 files across Epics 1/6/7B/9/10B/11/15/17/18) shipped without a re... | done | PR #1081, PR #990 | frontend/apps/ppt-web/src/features/auth/pages/ProfileEdit... | 2026-06-05 |
| 2 | test-gap | Mobile document-detail wiring (PR #992) shipped without a regression test for the deep-link paylo... | done | PR #1082, PR #992 | frontend/apps/mobile/src/screens/documents/DocumentDetail... | 2026-06-05 |
| 2 | test-gap | Screen-map drift: PR #839 modified ppt-web App.tsx (FileDisputePageRoute) without a docs/screens/... | done | PR #1056, PR #839 | frontend/apps/ppt-web/src/App.tsx, docs/screens/ppt/ | 2026-06-05 |
| 2 | refactor | ppt-web status/auth components hardcode English in an otherwise i18n'd app | done | code-review ppt-web-ui 2026-05-24, rotating-expert-review | frontend/apps/ppt-web/src/components/ConnectionStatus.tsx... | 2026-06-04 |
| 2 | bug | MediationWorkspacePage shows empty/unknown state instead of error UI on dispute fetch failure | done | PR #555, code-review 2026-05-27 | frontend/apps/ppt-web/src/features/disputes/pages/Mediati... | 2026-06-03 |
| 2 | bug | Mobile VotingScreen double-casts API result across boundary — render-time crash on unexpected shape | done | code-review mobile-rn 2026-05-27, rotating-expert-review | frontend/apps/mobile/src/screens/voting/VotingScreen.tsx | 2026-06-03 |
| 2 | bug | Reality-web InviteRealtorModal swallows invite-mutation failure with no error UI | done | code-review reality-web 2026-05-28, rotating-expert-review | frontend/apps/reality-web/src/components/agency/RealtorMa... | 2026-06-03 |
| 2 | bug | Airbnb webhook at-least-once delivery enqueues duplicate SYNC_EXTERNAL jobs | done | PR #538, webhook.rs:1028 | backend/servers/api-server/src/routes/integrations/webhoo... | 2026-06-03 |
| 2 | dx | DocumentsBrowse MoveFolderDialog cannot pre-select current folder (DocumentSummary lacks folder_id) | done | PR #623, PR #1031 | frontend/apps/ppt-web/src/features/documents/components/D... | 2026-06-03 |
| 2 | test-gap | API + SPA security-headers middleware (PR #963) shipped without an assertion test for HSTS/nosnif... | done | PR #963, issue #954 | backend/crates/api-core/src/middleware/security_headers.rs | 2026-06-03 |
| 2 | test-gap | api-server main.rs vs lib.rs::create_router diverge silently (5 routes unreachable in prod, no te... | done | PR #866, issue #867 | backend/servers/api-server/src/main.rs, backend/servers/a... | 2026-06-01 |
| 2 | bug | ReportSchedule.update_schedule stores cron in `time` workaround; documented UPDATE never runs (mi... | done | PR #611, issue #616 | backend/crates/db/src/repositories/report_schedule.rs | 2026-05-30 |
| 2 | test-gap | Screen-map drift: report execution-history route (PR #547) added without a ppt screen doc | done | PR #547, frontend/apps/ppt-web/src/routes/lazyRoutes.tsx | frontend/apps/ppt-web/src/features/reports/pages/Schedule... | 2026-05-27 |
| 2 | test-gap | Dispute state machine (PR #506) shipped with no tests + no org predicate on update_status | done | PR #506, issue #520 | backend/crates/db/src/models/disputes.rs, backend/crates/... | 2026-05-26 |
| 2 | refactor | documents.rs churn-hot — 10,659 lines over 14d | done | git log origin/main since 2026-05-06, git log origin/dev since 2026-05-20 | backend/servers/api-server/src/routes/documents.rs | 2026-05-25 |
| 2 | refactor | integrations.rs churn-hot — 12,977 lines over 14d, candidate for module split | done | git log origin/main since 2026-05-06, git log origin/dev since 2026-05-20 | backend/servers/api-server/src/routes/integrations.rs | 2026-05-25 |
| 2 | refactor | organizations.rs churn-hot — 12,060 lines over 14d (multitenancy + admin) | done | git log origin/main since 2026-05-06, git log origin/dev since 2026-05-20 | backend/servers/api-server/src/routes/organizations.rs | 2026-05-25 |
| 2 | security | IDOR: reality-server mark_as_read flips any realtor's inquiry by ID with no owner scoping | done | code-review reality-server 2026-05-23, inquiries.rs:554 | backend/servers/reality-server/src/routes/inquiries.rs, b... | 2026-05-25 |
| 2 | security | Latent fail-open: ProtectedRoute role check is skipped when user.role is falsy | done | code-review ppt-web-ui 2026-05-24, ProtectedRoute.tsx:117 | frontend/apps/ppt-web/src/components/ProtectedRoute.tsx, ... | 2026-05-25 |
| 2 | test-gap | Screen-map drift: PR #464 wired a neighbors route in ppt-web without a docs/screens/ppt entry | done | PR #464 | frontend/apps/ppt-web/src/routes/lazyRoutes.tsx | 2026-05-25 |
| 2 | test-gap | Screen-map drift: PR #460 touched reality-web listing page without a docs/screens/reality update | closed | PR #460 | frontend/apps/reality-web/src/app/[locale]/listings/[slug... | 2026-05-25 |
| 2 | refactor | Dead/duplicate handler modules: AuthHandler & BuildingHandler unused, routes reimplement inline | done | code-review api-handlers 2026-05-23, PR #437 | backend/servers/api-server/src/handlers/auth/mod.rs, back... | 2026-05-24 |
| 2 | security | Complete RLS migration in 31 remaining handlers (voting, market_pricing, faults, notif_prefs, rep... | done | issue #160, PR #420 | backend/servers/api-server/src/handlers/voting/mod.rs, ba... | 2026-05-23 |
| 1 | bug | [api-handlers] backend/servers/api-server/src/routes/layout/admin.rs:46,:123,:154,:181,:221,:296 ... | open | rotating-expert-review | backend/servers/api-server/src/routes/layout/admin.rs, ba... | 2026-07-23 |
| 1 | refactor | Churn hotspot: backend/servers/api-server/tests/platform_admin_authz_batch2_tests.rs — new/growin... | open | PR #2447 | backend/servers/api-server/tests/platform_admin_authz_bat... | 2026-07-23 |
| 1 | refactor | Churn hotspot: backend/servers/api-server/tests/org_property_authz_backfill_tests.rs — new/growin... | open | PR #2447, PR #2453 | backend/servers/api-server/tests/org_property_authz_backf... | 2026-07-23 |
| 1 | refactor | Churn hotspot: backend/servers/api-server/tests/admin_platform_happy_path_batch3_tests.rs — new/g... | open | PR #2439 | backend/servers/api-server/tests/admin_platform_happy_pat... | 2026-07-23 |
| 1 | refactor | Churn hotspot: docs/repo-map.md — 4 touches this window (per-PR route-map refresh) | open | git-log-since-2026-07-16, git-log-since-2026-07-20T03:12:00Z | docs/repo-map.md | 2026-07-21 |
| 1 | refactor | Churn hotspot: docs/screens/ppt/dashboard.md — 3 touches this run (Layout & Content Manager pilot... | open | git-log-since-2026-07-20T03:12:00Z | docs/screens/ppt/dashboard.md | 2026-07-21 |
| 1 | refactor | Churn hotspot: frontend/apps/admin-web/src/features/layout-editor/LayoutEditorPage.tsx — 2 touche... | open | git-log-since-2026-07-20T03:12:00Z | frontend/apps/admin-web/src/features/layout-editor/Layout... | 2026-07-21 |
| 1 | bug | scheduler.rs get_announcement_target_users() silently swallows target_ids JSON parse errors — mal... | done | code-review api-core 2026-07-20 | backend/servers/api-server/src/services/scheduler.rs | 2026-07-20 |
| 1 | refactor | Churn hotspot: frontend/apps/mobile/package.json — 5 touches this window (Expo/expo-notifications... | open | git-log-since-2026-07-16 | frontend/apps/mobile/package.json | 2026-07-20 |
| 1 | refactor | Churn hotspot: backend/Cargo.toml — 3 touches this window (dependabot minor-patch cascade + layou... | open | git-log-since-2026-07-16 | backend/Cargo.toml | 2026-07-20 |
| 1 | dx | PR #2385 (dependabot: dtolnay/rust-toolchain 1.94.1 → 1.100.0) closed unmerged — likely supersede... | dropped | PR #2385 | .github/workflows/backend.yml | 2026-07-20 |
| 1 | dx | PR #2387 (dependabot: npm-minor-patch 15-update rollup) closed unmerged — superseded by the 19-up... | dropped | PR #2387, PR #2423 | frontend/pnpm-lock.yaml | 2026-07-20 |
| 1 | refactor | Churn hotspot: frontend/apps/ppt-web/messages/en.json — frontend/apps/ppt-web/messages/en.json: +... | done | git-log-since-2026-07-13 | frontend/apps/ppt-web/messages/en.json | 2026-07-16 |
| 1 | refactor | Churn hotspot: frontend/packages/sitemap/src/json/sitemap.json — frontend/packages/sitemap/src/js... | done | git-log-since-2026-07-13 | frontend/packages/sitemap/src/json/sitemap.json | 2026-07-16 |
| 1 | refactor | backend integrations booking/mod.rs — instability watch after PR #2176 split | done | commits 2026-07-05..2026-07-09 | backend/crates/integrations/src/booking/mod.rs | 2026-07-09 |
| 1 | test-gap | oauth_integration_tests.rs repeated-churn (runs_seen 2→3) — OAuth handlers still moving | dropped | commits 2026-07-05..2026-07-09 | backend/servers/api-server/tests/oauth_integration_tests.rs | 2026-07-09 |
| 1 | refactor | api-server routes/auth.rs — repeated hotspot + 3 static-review findings this run | done | commits 2026-07-05..2026-07-09 | backend/servers/api-server/src/routes/auth.rs | 2026-07-09 |
| 1 | bug | /refresh and /logout — empty refresh_token cookie shadows valid body token | done | Phase 1.5 code-review 2026-07-09 (api-handlers segment) | backend/servers/api-server/src/routes/auth.rs | 2026-07-09 |
| 1 | bug | DeepLinkRouter skips URL-decoding while Android Uri.getQueryParameter decodes — SSO tokens diverg... | dropped | mobile-native-kmp segment review 2026-06-06 | mobile-native/shared/src/commonMain/kotlin/three/two/bit/... | 2026-07-05 |
| 1 | bug | SearchScreen stale-response race — overlapping searches can clobber newer results | dropped | mobile-native-kmp segment review 2026-06-06, PR #1125 | mobile-native/androidApp/src/main/java/three/two/bit/ppt/... | 2026-07-05 |
| 1 | test-gap | Screen-map drift: PR #1085 modified reality-web listing detail metadata + page without screen-doc... | dropped | PR #1085 | frontend/apps/reality-web/src/app/[locale]/listings/[slug... | 2026-07-05 |
| 1 | test-gap | Screen-map drift: PR #1100 modified ppt-web App.tsx (FileDisputePageRoute extraction) without scr... | dropped | PR #1100 | frontend/apps/ppt-web/src/App.tsx, frontend/apps/ppt-web/... | 2026-07-05 |
| 1 | bug | Risky churn: api-server main.rs security-headers wiring shipped without a middleware smoke test | dropped | PR #963 | backend/servers/api-server/src/main.rs, backend/crates/ap... | 2026-07-05 |
| 1 | test-gap | Screen-map drift: PR #922 modified ppt-web App.tsx (dev-review rounds 1-5 fixes) without a docs/s... | done | PR #922, PR #2075 | frontend/apps/ppt-web/src/App.tsx | 2026-07-05 |
| 1 | refactor | Churn hotspot: ListingDetailScreen.kt — +1279 LOC this run (gap-82-4 reality mobile favorite toggle) | done | PR #1121, PR #2059 | mobile-native/androidApp/src/main/java/three/two/bit/ppt/... | 2026-07-05 |
| 1 | refactor | Churn hotspot: DocumentsScreen.tsx — 3 PRs this run | done | PR #1101, PR #1081 | frontend/apps/mobile/src/screens/documents/DocumentsScree... | 2026-07-05 |
| 1 | bug | iOS deep-link layer dead at runtime — Info.plist missing CFBundleURLTypes + applinks entitlement | dropped | issue #1267, PR #1256 (verify) | mobile-native/iosApp/Info.plist | 2026-07-05 |
| 1 | test-gap | Webhook handlers RLS migration (PR #1288, PAP-170) shipped without a new regression test for repo... | dropped | PR #1288, PAP-170 | backend/servers/api-server/src/routes/integrations/webhoo... | 2026-07-05 |
| 1 | test-gap | AI llm/sessions + integrations sync + subscriptions RLS migration (PR #1287, PAP-169) shipped wit... | dropped | PR #1287, PAP-169 | backend/servers/api-server/src/routes/ai/llm.rs, backend/... | 2026-07-05 |
| 1 | test-gap | api_ecosystem.rs RLS migration (PR #1289, PAP-167) — 162-line handler rework shipped without a re... | dropped | PR #1289, PAP-167 | backend/servers/api-server/src/routes/api_ecosystem.rs | 2026-07-05 |
| 1 | test-gap | mfa.rs RLS migration (PR #1292, PAP-168) shipped without a regression test; also landed broken an... | dropped | PR #1292, PR #1287 | backend/servers/api-server/src/routes/mfa.rs | 2026-07-05 |
| 1 | refactor | crypto.rs:127 SysRng.try_fill_bytes(...).expect() panics if OS CSPRNG errors during integration-c... | done | code-review api-core 2026-06-15, PR #2074 | backend/crates/integrations/src/crypto.rs | 2026-07-05 |
| 1 | bug | Mobile RN production screens (Buildings/Meters/Leases/PersonMonths/Notifications/Threads/Forms) r... | done | Phase 1.5 review of mobile-rn segment (2026-06-16) | frontend/apps/mobile/src/screens/buildings/BuildingsScree... | 2026-07-05 |
| 1 | bug | useDeepLinkRouting.ts:27-36 — initialize() re-runs on onNavigate identity change + void promise w... | dropped | Phase 1.5 review of mobile-rn segment (2026-06-16) | frontend/apps/mobile/src/hooks/useDeepLinkRouting.ts | 2026-07-05 |
| 1 | refactor | Churn hotspot: backend/crates/db/src/models/mod.rs (12 commits in 19-day catch-up) | dropped | churn since 4829015b: 12 commits | backend/crates/db/src/models/mod.rs | 2026-07-05 |
| 1 | refactor | Churn hotspot: backend/crates/db/src/repositories/rental.rs (11 commits in 19-day catch-up) | done | churn since 4829015b: 11 commits | backend/crates/db/src/repositories/rental.rs | 2026-07-05 |
| 1 | refactor | PR #1378 closed without merge — DROP-OWNED-BY teardown theory for #1332 was wrong root cause, sup... | done | PR #1378, PR #1379 | backend/crates/db/src/repositories/form.rs, backend/crate... | 2026-06-15 |
| 1 | test-gap | PKCE unit test became a tautology after services/oauth.rs DRY refactor (#1132) | done | #1137, PR #1132 | backend/servers/api-server/src/services/oauth.rs | 2026-06-07 |
| 1 | triage | Triage: dispatcher incident — assignments-archive.json corrupted to 1/196 rows on dev branch (#1061) | done | Issue #1061, #1061 closed | .research/management/assignments-archive.json | 2026-06-07 |
| 1 | triage | Issue #950 (no labels, OPEN): CI: trigger-deploy 403 marks all dev image builds red and blocks st... | done | #950, PR #1143 |  | 2026-06-07 |
| 1 | triage | Issue #952 (no labels, OPEN): [staging] Reality SSO login dead-ends: redirect_uri callback 404s o... | done | #952, PR #1144 |  | 2026-06-07 |
| 1 | triage | Issue #769 (no labels, OPEN): Current dev review: Deploy server | done | #769, PR #1141 |  | 2026-06-07 |
| 1 | triage | Issue #789 (no labels, OPEN): Dev review rounds 6-10: scheduler, notifications, admin, orgs, buil... | done | #789, PR #1142 |  | 2026-06-07 |
| 1 | dx | docker/nginx admin-web + ppt-web templates churned twice this run (security headers + redirects) | done | PR #963, PR #964 | docker/nginx/admin-web.nginx.conf.template, docker/nginx/... | 2026-06-06 |
| 1 | refactor | ai.rs churn-hot — 3,142 lines this run; 3,142-line route monolith, candidate for module split | done | git log origin/dev since 2026-05-24, PR #1114 | backend/servers/api-server/src/routes/ai.rs | 2026-06-06 |
| 1 | refactor | ppt-web e2e auth-refresh.spec.ts added (+252 lines, story 79-2 token-refresh coverage) | done | PR #1047, PR #1113 | frontend/apps/ppt-web/e2e/auth-refresh.spec.ts | 2026-06-06 |
| 1 | refactor | api-server esignature_webhook_idempotency_tests.rs added (+228 lines, terminal-state regression) | done | PR #1034, PR #1119 | backend/servers/api-server/tests/esignature_webhook_idemp... | 2026-06-06 |
| 1 | refactor | ppt-web EvidenceUploader.test.tsx added (+202 lines, dispute-filing AC-2 regression) | done | PR #1048, PR #1116 | frontend/apps/ppt-web/src/features/disputes/components/Ev... | 2026-06-06 |
| 1 | refactor | api-server main.rs touched twice this run (gap-sweep + security headers) — minor churn marker | done | PR #989, PR #963 | backend/servers/api-server/src/main.rs | 2026-06-06 |
| 1 | refactor | Duplicated animate-spin spinner markup across mediation page + chat thread (no shared Spinner) | done | PR #555, code-review 2026-05-27 | frontend/apps/ppt-web/src/features/disputes/pages/Mediati... | 2026-06-06 |
| 1 | refactor | Mediation reference number uppercases full UUID (DSP-<uuid>) instead of a short code | done | PR #555, code-review 2026-05-27 | frontend/apps/ppt-web/src/features/disputes/pages/Mediati... | 2026-06-06 |
| 1 | refactor | frontend/apps/mobile/src/App.tsx churned twice this run (universal links + doc-detail wiring) | done | PR #962, PR #992 | frontend/apps/mobile/src/App.tsx | 2026-06-06 |
| 1 | refactor | platform_admin.rs churn-hot — 2,762 lines this run (admin/OAuth-provider feature work) | done | git log origin/dev since 2026-05-24, PR #1109 | backend/servers/api-server/src/routes/platform_admin.rs | 2026-06-06 |
| 1 | refactor | Reality-web ComparisonUrlHandler hardcodes English loading/error strings | done | code-review reality-web 2026-05-28, rotating-expert-review | frontend/apps/reality-web/src/components/comparison/Compa... | 2026-06-06 |
| 1 | refactor | Watch routes/oauth.rs churn after audit-log + hardening PRs | done | PR #930, PR #933 | backend/servers/api-server/src/routes/oauth.rs | 2026-06-06 |
| 1 | refactor | Watch services/oauth.rs churn after introspect/revoke hardening (#933) | done | PR #933, PR #1132 | backend/servers/api-server/src/services/oauth.rs | 2026-06-06 |
| 1 | test-gap | Mobile VotingScreen pure transforms toUiStatus/toUiVote have no tests | done | code-review mobile-rn 2026-05-27, rotating-expert-review | frontend/apps/mobile/src/screens/voting/VotingScreen.tsx | 2026-06-06 |
| 1 | triage | Issue #749 (no labels, OPEN): Code review findings: Story 6.1 announcement creation and targeting | done | #749, issue #749 closed |  | 2026-06-06 |
| 1 | triage | Issue #755 (no labels, OPEN): Current dev review: Epic 8A Notification Preferences | done | #755, issue #755 closed |  | 2026-06-06 |
| 1 | triage | Issue #764 (no labels, OPEN): Current dev review: Admin MFA & Auth Hardening | done | #764, issue #764 closed |  | 2026-06-06 |
| 1 | triage | Issue #765 (no labels, OPEN): Current dev review: Integrations & Airbnb OAuth | done | #765, issue #765 closed |  | 2026-06-06 |
| 1 | bug | Mobile VotingScreen hardcodes en-US in toLocaleDateString — vote dates never localize | done | PR #1083, code-review mobile-rn 2026-05-27 | frontend/apps/mobile/src/screens/voting/VotingScreen.tsx | 2026-06-05 |
| 1 | bug | Reality-web listing generateMetadata can throw during SSR on malformed 200 body | done | PR #1085, code-review reality-web 2026-05-28 | frontend/apps/reality-web/src/app/[locale]/listings/[slug... | 2026-06-05 |
| 1 | security | PR #908 (fix(security): require PKCE on OAuth authorization-code flow, closes #823) was closed un... | done | PR #908, PR #1025 | backend/servers/api-server/src/routes/oauth_provider.rs | 2026-06-03 |
| 1 | triage | Issue #751 (no labels, OPEN): Current dev review: frontend/web/API-client findings | done | #751, #942 |  | 2026-06-02 |
| 1 | triage | Issue #752 (no labels, OPEN): Current dev review: mobile CI tooling findings | done | #752, #929 |  | 2026-06-02 |
| 1 | triage | Issue #756 (no labels, OPEN): Current dev review: Epic 10A OAuth Provider | done | #756, #934 |  | 2026-06-02 |
| 1 | triage | Issue #761 (no labels, OPEN): Current dev review: Epic 84 E-Signature & Leases | done | #761, #936 |  | 2026-06-02 |
| 1 | triage | Issue #763 (no labels, OPEN): Current dev review: Reality Server & Inquiries | done | #763, #935 |  | 2026-06-02 |
| 1 | triage | Issue #767 (no labels, OPEN): Current dev review: Mobile RN Property Management app | done | #767, #943 |  | 2026-06-02 |
| 1 | triage | Issue #768 (no labels, OPEN): Current dev review: Admin-web features (10B) | done | #768, #930 |  | 2026-06-02 |
| 1 | triage | Issue #920 (no labels, OPEN): Announcement targeting not enforced on read (intra-org disclosure) | done | #920, #944 |  | 2026-06-02 |
| 1 | triage | Issue #750 (no labels, OPEN): Current dev review: backend/API/database findings | done | #750, PR #922 |  | 2026-06-01 |
| 1 | triage | Issue #753 (no labels, OPEN): Current dev review: Epic 6 Announcements & Communication | done | #753 |  | 2026-06-01 |
| 1 | triage | Issue #754 (no labels, OPEN): Current dev review: Epic 7A Basic Document Management | done | #754, PR #914 |  | 2026-06-01 |
| 1 | triage | Issue #757 (no labels, OPEN): Current dev review: Epic 10B Platform Administration | done | #757 |  | 2026-06-01 |
| 1 | triage | Issue #760 (no labels, OPEN): Current dev review: Epic 79 Disputes & Mediation | done | #760, PR #915 |  | 2026-06-01 |
| 1 | triage | Issue #762 (no labels, OPEN): Current dev review: Reports & Schedules | done | #762 |  | 2026-06-01 |
| 1 | triage | Issue #766 (no labels, OPEN): Current dev review: AI & LLM routes | done | #766, PR #879 |  | 2026-06-01 |
| 1 | triage | Issue #770 (no labels, OPEN): Current dev review: Faults & triage | done | #770, PR #902 |  | 2026-06-01 |
| 1 | triage | Issue #771 (no labels, OPEN): Current dev review: Research dispatcher & CI automation | done | #771, PR #923 |  | 2026-06-01 |
| 1 | triage | Issue #772 (no labels, OPEN): Current dev review: Auth core (delta confirmation) | done | #772 |  | 2026-06-01 |
| 1 | triage | Issue #773 (no labels, OPEN): Current dev review: Leases & rental | done | #773 |  | 2026-06-01 |
| 1 | triage | Issue #774 (no labels, OPEN): Current dev review: Reality server (broad) | done | #774, PR #919 |  | 2026-06-01 |
| 1 | triage | Issue #775 (no labels, OPEN): Current dev review: WebSocket realtime | done | #775, PR #926 |  | 2026-06-01 |
| 1 | triage | Issue #776 (no labels, OPEN): Current dev review: Equipment & audit log | done | #776 |  | 2026-06-01 |
| 1 | triage | Issue #777 (no labels, OPEN): Current dev review: Compliance & GDPR | done | #777 |  | 2026-06-01 |
| 1 | triage | Issue #778 (no labels, OPEN): Current dev review: Marketplace, voting, investor portal, impersona... | done | #778, PR #882 |  | 2026-06-01 |
| 1 | triage | Issue #788 (no labels, OPEN): Dev review rounds 1-5: mobile-native + ppt-web surfaces | done | #788, PR #922 |  | 2026-06-01 |
| 1 | triage | Issue #790 (no labels, OPEN): Dev review rounds 11-15: vendor, predictive, reality-web, middleware | done | #790, PR #913 |  | 2026-06-01 |
| 1 | triage | Issue #791 (no labels, OPEN): Dev review rounds 16-20: push, e-sign, portal, webhooks, reserves | done | #791, PR #924 |  | 2026-06-01 |
| 1 | triage | Issue #846 (no labels, OPEN): Code review: Epics 12+65 — Meters & Energy/ESG (origin/dev) | done | #846, PR #880 |  | 2026-06-01 |
| 1 | triage | Issue #847 (no labels, OPEN): Code review: Reality-server — Inquiries IDOR (Epics 16–19) (origin/... | done | #847 |  | 2026-06-01 |
| 1 | triage | Issue #848 (no labels, OPEN): Code review: Epics 78+134 — Vendor portal stubs & Predictive mainte... | done | #848, PR #913 |  | 2026-06-01 |
| 1 | triage | Issue #850 (no labels, OPEN): Code review: Epics 61+146+42 — Multi-currency, Data residency, Viol... | done | #850, PR #883 |  | 2026-06-01 |
| 1 | triage | Issue #851 (no labels, OPEN): Code review: Epics 15+105+69 — Listings/syndication & Developer API... | done | #851, PR #904 |  | 2026-06-01 |
| 1 | triage | Issue #859 (no labels, OPEN): sqlx 0.9 breaks runtime decode of Postgres enum columns into Rust S... | done | #859, PR #871 |  | 2026-06-01 |
| 1 | triage | Issue #867 (no labels, OPEN): Tech debt: api-server main.rs duplicates lib.rs::create_router — ro... | done | #867, PR #870 |  | 2026-06-01 |
| 1 | triage | Issue #836 (no labels, OPEN): Code review: Epic 2B-C — Mobile push & device registration (origin/... | done | #836, PR #866 |  | 2026-05-31 |
| 1 | triage | Issue #845 (no labels, OPEN): Code review: Epic 14 — IoT alerts, correlations, thresholds (origin... | done | #845, PR #862 |  | 2026-05-31 |
| 1 | triage | Issue #849 (no labels, OPEN): Code review: Epic 10B+143 — Admin impersonation, Help, Board meetin... | done | #849, PR #869 |  | 2026-05-31 |
| 0 | dx | Cloud routine cadence recovery — reduce 3–4d gaps between runs | dropped | routine self-signal 2026-07-09 | .research/state.json | 2026-07-09 |
| 0 | refactor | Churn hotspot: 1021 lines changed in backend/servers/api-server/src/routes/emergency.rs (window 2026 | dropped | local git numstat since 2026-06-07 | backend/servers/api-server/src/routes/emergency.rs | 2026-07-05 |
| 0 | refactor | Churn hotspot: 929 lines changed in backend/servers/api-server/src/routes/vendors.rs (window 2026-06 | dropped | local git numstat since 2026-06-07 | backend/servers/api-server/src/routes/vendors.rs | 2026-07-05 |
| 0 | refactor | Churn hotspot: 709 lines changed in backend/servers/api-server/src/routes/enhanced_tenant_screening. | dropped | local git numstat since 2026-06-07 | backend/servers/api-server/src/routes/enhanced_tenant_scr... | 2026-07-05 |
| 0 | refactor | Churn hotspot: 2940 lines changed in backend/crates/db/src/repositories/document.rs (window 2026-... | dropped | local git numstat since 2026-06-10T03:05:00Z | backend/crates/db/src/repositories/document.rs | 2026-07-05 |
| 0 | refactor | Churn hotspot: 2856 lines changed in backend/crates/db/src/repositories/subscription.rs (window 2... | dropped | local git numstat since 2026-06-10T03:05:00Z, PR #1246 | backend/crates/db/src/repositories/subscription.rs | 2026-07-05 |
| 0 | refactor | Churn hotspot: 2691 lines changed in backend/servers/api-server/src/routes/aml_dsa.rs (window 202... | dropped | local git numstat since 2026-06-10T03:05:00Z, PR #1193 | backend/servers/api-server/src/routes/aml_dsa.rs | 2026-07-05 |
| 0 | triage | Issue #1151 (no labels, OPEN): Research dispatcher: claimable buffer is stale — true claimable wo... | dropped | #1151 | .research/gc1-reconcile.sh, .research/management/action-l... | 2026-07-05 |
| 0 | refactor | Churn hotspot: SearchScreen.kt — +1293 LOC this run (gap-82-3 reality mobile search/filters) | dropped | PR #1125 | mobile-native/androidApp/src/main/java/three/two/bit/ppt/... | 2026-07-05 |
| 0 | refactor | MainActivity reimplements deep-link dispatch instead of calling shared DeepLinkRouter — drift trap | dropped | mobile-native-kmp segment review 2026-06-06 | mobile-native/androidApp/src/main/java/three/two/bit/ppt/... | 2026-07-05 |
| 0 | refactor | Churn hotspot: AnnouncementsScreen.tsx — 4 PRs this run, instability proxy | dropped | PR #1101, PR #1077 | frontend/apps/mobile/src/screens/announcements/Announceme... | 2026-07-05 |
| 0 | refactor | Churn hotspot: AnnouncementsScreen.test.ts — 4 PRs this run, instability proxy | dropped | PR #1101, PR #1077 | frontend/apps/mobile/src/screens/announcements/Announceme... | 2026-07-05 |
| 0 | triage | Dispatcher action-list.json corruption when MCP push falls back from blocked git push | dropped | #1014 | .research/management/action-list.json, .research/dispatch... | 2026-07-05 |
| 0 | triage | Issue #951 (no labels, OPEN): Deploy blocker: api-server requires ESIGN_TOKEN_SECRET + ESIGN_WEBH... | dropped | #951 |  | 2026-07-05 |
| 0 | dx | PR #1274 (cargo-minor-patch group, /backend, 9 updates) closed unmerged — superseded by #1313 aft... | dropped | PR #1274 |  | 2026-07-05 |
| 0 | refactor | Churn hotspot: backend/crates/integrations/src/booking.rs (+404/−29 in PR #1294 Booking.com OTA r... | dropped | PR #1294 commit 7ccce8a | backend/crates/integrations/src/booking.rs | 2026-07-05 |
| 0 | refactor | Churn hotspot: backend/servers/api-server/src/routes/api_ecosystem.rs (+106/−27 in PR #1293 PAP-1... | dropped | PR #1293 commit 1e50156 | backend/servers/api-server/src/routes/api_ecosystem.rs | 2026-07-05 |
| 0 | refactor | Churn hotspot: backend/crates/db/src/repositories/reality_portal.rs (+59/−28 in PR #1297 PAP-142 ... | dropped | PR #1297 commit 8c711c6 | backend/crates/db/src/repositories/reality_portal.rs | 2026-07-05 |
| 0 | refactor | Churn hotspot: backend/servers/api-server/src/routes/iot.rs (+278/-403 in PR #1321/#1322 PAP-151 ... | dropped | PR #1321 commit, PR #1322 commit | backend/servers/api-server/src/routes/iot.rs | 2026-07-05 |
| 0 | refactor | Churn hotspot: backend/servers/api-server/src/routes/reserve_funds.rs (+228/-255 in PR #1321 PAP-... | dropped | PR #1321 commit | backend/servers/api-server/src/routes/reserve_funds.rs | 2026-07-05 |
| 0 | refactor | Churn hotspot: backend/crates/db/src/repositories/sensor.rs (+248/-86 in PR #1321/#1322 PAP-151 r... | dropped | PR #1321 commit, PR #1322 commit | backend/crates/db/src/repositories/sensor.rs | 2026-07-05 |
| 0 | triage | Issue #1331 (no labels, OPEN): Backend `test` job red/hanging on dev base — blocks the entire bac... | dropped | #1331 |  | 2026-07-05 |
| 0 | dx | Stalled review: PR #988 (Epic: reusable Playwright E2E framework + sitemap FlowRunner) open 10d, ... | dropped | PR #988 | frontend/packages/e2e/ | 2026-07-05 |
| 0 | refactor | Churn hotspot: backend/servers/api-server/src/routes/forms.rs touched 2x since 2026-06-12 (window... | dropped | local git numstat since 2026-06-12, local git numstat since 2026-06-15 | backend/servers/api-server/src/routes/forms.rs | 2026-07-05 |
| 0 | refactor | Churn hotspot: backend/servers/api-server/tests/reserve_funds_cross_org_idor_tests.rs touched 2x ... | dropped | local git numstat since 2026-06-12 | backend/servers/api-server/tests/reserve_funds_cross_org_... | 2026-07-05 |
| 0 | refactor | Churn hotspot: backend/crates/db/tests/form_rls_repo_tests.rs touched 2x since 2026-06-12 (window... | dropped | local git numstat since 2026-06-12 | backend/crates/db/tests/form_rls_repo_tests.rs | 2026-07-05 |
| 0 | triage | Issue #1380 (no labels, OPEN): Dispatcher stale gap-scan buffer + Tier-2 escalation endpoint misc... | dropped | issue #1380 | .research/management/action-list.json, .research/manageme... | 2026-07-05 |
| 0 | refactor | Churn hotspot: 124 lines in frontend/apps/mobile/app.config.icon.test.ts (PR #1383 gap-85-2) | dropped | PR #1383 | frontend/apps/mobile/app.config.icon.test.ts | 2026-07-05 |
| 0 | refactor | Churn hotspot: 94 lines in frontend/apps/mobile/app.config.ts (PR #1383 gap-85-2) | dropped | PR #1383 | frontend/apps/mobile/app.config.ts | 2026-07-05 |
| 0 | refactor | Churn hotspot: 53 lines in backend/crates/db/src/repositories/form.rs (PR #1379 #1332 unblock) | dropped | PR #1379, issue #1332 | backend/crates/db/src/repositories/form.rs | 2026-07-05 |
| 0 | refactor | booking_oauth_csrf_tests.rs hotspot — 484-line NEW test file (PR #1393 #1424 OAuth CSRF coverage) | dropped | local git numstat since 2026-06-15 (commit 67c24bd..origin/dev) | backend/servers/api-server/tests/booking_oauth_csrf_tests.rs | 2026-07-05 |
| 0 | refactor | booking_oauth_routes_tests.rs hotspot — 381-line NEW test file (PR #1393 OAuth routes coverage) | dropped | local git numstat since 2026-06-15 | backend/servers/api-server/tests/booking_oauth_routes_tes... | 2026-07-05 |
| 0 | refactor | forms.rs repeated-churn — runs_seen=2 (#1337 explicit_auto_deref + #1397 org-scope hardening) | dropped | hotspot_history.runs_seen 1→2 with new churn this run | backend/servers/api-server/src/routes/forms.rs | 2026-07-05 |
| 0 | dx | PR #1425 (GH #1377 document presigned-URL tests) closed unmerged — superseded by merged #1394 | dropped | PR #1425 |  | 2026-07-05 |
| 0 | dx | PR #1179 (docs(epics) catalog backfill for 37 mounted-but-undocumented backend modules) — stalled... | dropped | PR #1179 |  | 2026-07-05 |
| 0 | refactor | Stabilize oauth_integration_tests churn — heavy edits across 3 OAuth fix PRs | dropped | PR #930, PR #933 | backend/servers/api-server/tests/oauth_integration_tests.rs | 2026-06-16 |
| 0 | triage | Issue #779 (no labels, OPEN): Current dev review: consolidated priority rollup (origin/dev snapshot) | dropped | #779 |  | 2026-06-13 |
| 0 | bug | Announcer: untracked clear-then-set timeouts can resurrect a stale screen-reader message | dropped | code-review ppt-web-ui 2026-05-24, Announcer.tsx:49 | frontend/apps/ppt-web/src/components/Announcer.tsx | 2026-06-07 |
| 0 | dx | Portfolio dashboard: alert mark-read/resolve mutations + property-card click navigation are no-op... | dropped | PR #328, commit 254f01d | frontend/apps/ppt-web/src/features/portfolio-performance/... | 2026-06-04 |
