# Backlog of vectors
<sub>Last regenerated: 2026-07-14 22:35 UTC by routine</sub>

| Score | Vector | ID | Title | Status | Updated | Plan |
|-------|--------|----|----|--------|---------|------|
| 6 | test-gap | `test-gap-inquiry-idor-regression` | Add regression tests for inquiry mark_as_read cross-tenant IDOR fix (PR #497) | done | 2026-05-26 | [plan](plans/_archive/test-gap-inquiry-idor-regression.md) |
| 3 | security | `code-review-api-handlers-news-articles-idor` | routes/news_articles.rs:685 — delete_article discards TenantExtractor(_tenant) and calls NewsArticleRepository::new(stat | needs-human-judgement | 2026-07-14 |  |
| 3 | test-gap | `code-review-mobile-rn-nfc-no-tests` | frontend/apps/mobile/src/nfc/NFCCredentialManager.ts:34 — NFCCredentialManager manages building-access credentials in ex | ready | 2026-07-14 | [plan](plans/test-gap-mobile-nfc-untested.md) |
| 3 | bug | `bug-revoke-all-sessions-cookie-blindness` | revoke_all_sessions ignores refresh cookie — signs the caller out too | done | 2026-07-09 | [plan](plans/bug-revoke-all-sessions-cookie-blindness.md) |
| 3 | bug | `code-review-mobile-rn-report-fault-fake-submit` | ReportFaultScreen.tsx handleSubmit() fakes API call with setTimeout(1500) — fault reports never reach backend (App.tsx:126 wires this) | dropped | 2026-06-16 | [plan](plans/code-review-mobile-rn-report-fault-fake-submit.md) |
| 3 | bug | `code-review-reality-web-realtor-mgmt-untranslated` | Reality-web RealtorManagement.tsx hardcoded English strings — agency flow not localized to sk/cs/de | done | 2026-06-15 | [plan](plans/code-review-reality-web-realtor-mgmt-untranslated.md) |
| 3 | bug | `code-review-reality-web-share-comparison-404` | Reality-web ComparisonUrlHandler hits non-existent /api/listings/${id} — every shared comparison URL 404s | dropped | 2026-06-14 | [plan](plans/code-review-reality-web-share-comparison-404.md) |
| 3 | bug | `code-review-reality-web-listing-page-ssr-crash` | Reality-web listing detail SSR crashes on partial 200 body — JSON-LD build deref of undefined fields | dropped | 2026-06-14 | [plan](plans/code-review-reality-web-listing-page-ssr-crash.md) |
| 3 | bug | `bug-ios-searchview-uncompilable` | iOS SearchView.swift does not compile — performSearch/scheduleSearch undefined, resultsGrid corrupted | dropped | 2026-06-11 | [plan](plans/bug-ios-searchview-uncompilable.md) |
| 3 | security | `unchecked-todo-pr-1203` | PR #1203 (fix(aml_dsa): close cross-tenant IDOR in moderation + AML-review handlers (PAP-36)) merged | dropped | 2026-06-10 |  |
| 3 | security | `unchecked-todo-pr-1193` | PR #1193 (fix(aml-dsa): lock DSA reports to platform roles + fix file-path disclosure (PAP-47)) merg | dropped | 2026-06-10 |  |
| 3 | bug | `bug-schema-drift-runtime-sql-issue-1008` | Schema drift: runtime SQL errors from non-existent columns in voting/messaging/notification paths | done | 2026-06-07 |  |
| 3 | security | `security-llm-doc-idor` | IDOR: ai.rs LLM-doc handlers publish/list/get any tenant's listing descriptions & photo enhancements unscoped | dropped | 2026-06-01 | [plan](plans/security-llm-doc-idor.md) |
| 3 | security | `security-realtors-mark-inquiry-read-idor` | IDOR: reality-server realtors mark_inquiry_read flips any realtor's inquiry by ID with no owner scoping | done | 2026-05-26 | [plan](plans/_archive/security-realtors-mark-inquiry-read-idor.md) |
| 3 | security | `security-equipment-idor` | IDOR: equipment delete/update + maintenance update mutate any tenant's equipment by ID with no org scoping | done | 2026-05-25 | [plan](plans/_archive/security-equipment-idor.md) |
| 3 | security | `security-ssrf-outbound-url-validation` | SSRF: signed-document fetch + webhook-test POST issue outbound requests to unvalidated user-controlled URLs | done | 2026-05-25 | [plan](plans/_archive/security-ssrf-outbound-url-validation.md) |
| 3 | security | `security-voice-device-idor` | IDOR: unlink_voice_device deactivates any device by ID with no owner/org scoping | done | 2026-05-25 | [plan](plans/_archive/security-voice-device-idor.md) |
| 2 | bug | `code-review-mobile-rn-dashboard-no-err-ui` | frontend/apps/mobile/src/screens/dashboard/DashboardScreen.tsx:72 — DashboardScreen fires four useApiQuery calls (announ | open | 2026-07-14 |  |
| 2 | test-gap | `code-review-mobile-rn-deeplink-mgr-untested` | frontend/apps/mobile/src/qrcode/DeepLinkHandler.ts:199 — DeepLinkManager (consumed by hooks/useDeepLinkRouting.ts and ho | open | 2026-07-14 |  |
| 2 | bug | `code-review-mobile-rn-announcements-i18n` | frontend/apps/mobile/src/screens/announcements/AnnouncementsScreen.tsx:1-12 — no react-i18next import / no useTranslatio | open | 2026-07-14 |  |
| 2 | bug | `code-review-ppt-web-core-units-fetch-noauth` | frontend/apps/ppt-web/src/routes/groups/person-months.tsx:80 — useBuildingUnits calls raw fetch('/api/v1/units?...') wit | open | 2026-07-14 |  |
| 2 | bug | `code-review-ppt-web-core-dashboard-unguarded` | frontend/apps/ppt-web/src/routes/groups/core.tsx:159-160 — /dashboard/manager and /dashboard/resident are wired without  | open | 2026-07-14 |  |
| 2 | bug | `code-review-mobile-rn-wallet-passtype-id` | frontend/apps/mobile/src/nfc/NFCAccessController.ts:383 — Apple Wallet pass built with passTypeIdentifier: 'pass.bit.two | open | 2026-07-14 |  |
| 2 | bug | `code-review-mobile-rn-widget-api-swallow` | frontend/apps/mobile/src/widgets/WidgetDataProvider.ts:356-384 — apiRequest<T>() catches every error (network + non-ok r | open | 2026-07-14 |  |
| 2 | refactor | `refactor-churn-hotspots-api-server-auth-2026-07-12` | Churn hotspot cluster: api-server routes/auth.rs (runs_seen=3) + auth_tests.rs + reality-server routes/sso.rs | done | 2026-07-12 |  |
| 2 | security | `security-forgot-password-no-rate-limit` | /forgot-password and /resend-verification have no rate limit — mailbomb / token-clobber | done | 2026-07-09 | [plan](plans/security-forgot-password-no-rate-limit.md) |
| 2 | test-gap | `test-gap-hotfix-no-test-pr-959-reality-listings-pagination` | Reality-server listings pagination clamp (PR #959) shipped without a regression test for limit=-1 | done | 2026-07-05 |  |
| 2 | test-gap | `screen-map-drift-pr-1418-ppt` | PR #1418 touched routes/** (faults.route.test.tsx) without updating docs/screens/ppt/* — heuristic, test-file fix | done | 2026-07-05 |  |
| 2 | bug | `code-review-api-core-vote-partial-cmp-panic` | vote.rs:1765 calculate_question_result() uses partial_cmp().unwrap() on f64 — NaN/Inf weights panic /votes/{id}/results | done | 2026-06-16 |  |
| 2 | test-gap | `unchecked-todo-pr-1196` | PR #1196 (feat(ppt-web): add missing test coverage for faults feature) merged with 2 unchecked TODO  | dropped | 2026-06-10 |  |
| 2 | dx | `dx-push-fanout-blpop-drain` | PushFanoutWorker BLPOP queue-drain deferred — Redis path is a logging no-op | done | 2026-06-06 |  |
| 2 | refactor | `refactor-ai-rs-module-split` | ai.rs (3,134 LOC) — explicit module-split into routes/ai/{sessions,equipment,workflows,voice,llm,mod}.rs | done | 2026-06-06 |  |
| 2 | refactor | `refactor-announcements-rs-hot` | announcements.rs churn-hot — 2,722 lines this run (Epic 2B + Epic 6 work) | done | 2026-06-06 |  |
| 2 | refactor | `refactor-announcements-rs-module-split` | announcements.rs (2,722 LOC) — explicit module-split into routes/announcements/{crud,targeting,delivery,reactions,mod}.rs | done | 2026-06-06 |  |
| 2 | refactor | `refactor-app-tsx-route-coupling` | Reduce App.tsx route-aggregator coupling (top churn hotspot, merge-conflict risk) | done | 2026-06-06 |  |
| 2 | refactor | `refactor-platform-admin-rs-module-split` | platform_admin.rs (2,762 LOC) — explicit module-split into routes/platform_admin/{tenants,features,billing,audit,mod}.rs | done | 2026-06-06 |  |
| 2 | test-gap | `test-gap-screen-map-drift-pr-1033-ppt` | Screen-map drift: PR #1033 wired error/retry into AnnouncementsPage+FaultsPage via App.tsx without a docs/screens/ppt update | done | 2026-06-06 |  |
| 2 | bug | `bug-risky-churn-pr-992-mobile-app-tsx` | Risky churn: mobile App.tsx deep-link/doc-detail wiring changing across back-to-back PRs without coverage | done | 2026-06-05 |  |
| 2 | dx | `dx-integration-marketplace-stubs` | Integration marketplace install/OAuth flows are placeholders — wire backend handlers + UI navigation | done | 2026-06-05 |  |
| 2 | test-gap | `test-gap-booking-push-validation-untested` | Booking push availability/rates endpoints add batch-cap + non-negative guards with no regression test | done | 2026-06-05 |  |
| 2 | test-gap | `test-gap-hotfix-no-test-pr-874-portal-webhooks` | Portal webhook fail-closed fix (PR #874) shipped without a regression test for unverified-signature rejection | done | 2026-06-05 |  |
| 2 | test-gap | `test-gap-hotfix-no-test-pr-918-mobile-dev-review` | Mobile dev-review batch (PR #918, 5 files under frontend/apps/mobile/src) shipped without a regression test | done | 2026-06-05 |  |
| 2 | test-gap | `test-gap-hotfix-no-test-pr-921-sso-consumer` | Reality-server SSO consumer review fix (PR #921, closes #820) shipped without a regression test | done | 2026-06-05 |  |
| 2 | test-gap | `test-gap-hotfix-no-test-pr-923-branch-protection-rebase` | CI branch-protection + auto-rebase workflow change (PR #923) shipped without an integration test | done | 2026-06-05 |  |
| 2 | test-gap | `test-gap-hotfix-no-test-pr-939-deploy-server-scopes` | deploy-server OIDC scope mapping (#939) shipped without unit test for derive_oidc_scopes | done | 2026-06-05 |  |
| 2 | test-gap | `test-gap-hotfix-no-test-pr-943-mobile-dev-review-tail` | Mobile RN dev-review tail (#943) shipped without test coverage | done | 2026-06-05 |  |
| 2 | test-gap | `test-gap-hotfix-no-test-pr-990-frontend-gap-sweep` | Frontend gap-sweep (PR #990, 34 files across Epics 1/6/7B/9/10B/11/15/17/18) shipped without a regression test | done | 2026-06-05 |  |
| 2 | test-gap | `test-gap-hotfix-no-test-pr-992-mobile-doc-detail` | Mobile document-detail wiring (PR #992) shipped without a regression test for the deep-link payload path | done | 2026-06-05 |  |
| 2 | test-gap | `test-gap-screen-map-drift-pr-839-ppt` | Screen-map drift: PR #839 modified ppt-web App.tsx (FileDisputePageRoute) without a docs/screens/ppt update | done | 2026-06-05 |  |
| 2 | refactor | `refactor-ppt-web-untranslated-strings` | ppt-web status/auth components hardcode English in an otherwise i18n'd app | done | 2026-06-04 |  |
| 2 | bug | `bug-mediation-page-no-error-state` | MediationWorkspacePage shows empty/unknown state instead of error UI on dispute fetch failure | done | 2026-06-03 |  |
| 2 | bug | `bug-mobile-voting-unsafe-cast` | Mobile VotingScreen double-casts API result across boundary — render-time crash on unexpected shape | done | 2026-06-03 |  |
| 2 | bug | `bug-reality-web-realtor-invite-silent-error` | Reality-web InviteRealtorModal swallows invite-mutation failure with no error UI | done | 2026-06-03 |  |
| 2 | bug | `bug-webhook-airbnb-dup-sync-jobs` | Airbnb webhook at-least-once delivery enqueues duplicate SYNC_EXTERNAL jobs | done | 2026-06-03 |  |
| 2 | dx | `dx-documentsbrowse-folder-preselect` | DocumentsBrowse MoveFolderDialog cannot pre-select current folder (DocumentSummary lacks folder_id) | done | 2026-06-03 |  |
| 2 | test-gap | `test-gap-hotfix-no-test-pr-963-security-headers` | API + SPA security-headers middleware (PR #963) shipped without an assertion test for HSTS/nosniff/CSP | done | 2026-06-03 |  |
| 2 | test-gap | `test-gap-router-set-parity-tests` | api-server main.rs vs lib.rs::create_router diverge silently (5 routes unreachable in prod, no test asserts parity) | done | 2026-06-01 |  |
| 2 | bug | `bug-report-schedule-update-no-sql` | ReportSchedule.update_schedule stores cron in `time` workaround; documented UPDATE never runs (missing cron_expression column) | done | 2026-05-30 |  |
| 2 | test-gap | `test-gap-screen-map-drift-ppt-report-history` | Screen-map drift: report execution-history route (PR #547) added without a ppt screen doc | done | 2026-05-27 |  |
| 2 | test-gap | `test-gap-dispute-fsm-no-tests` | Dispute state machine (PR #506) shipped with no tests + no org predicate on update_status | done | 2026-05-26 |  |
| 2 | refactor | `refactor-documents-rs-hot` | documents.rs churn-hot — 10,659 lines over 14d | done | 2026-05-25 |  |
| 2 | refactor | `refactor-integrations-rs-hot` | integrations.rs churn-hot — 12,977 lines over 14d, candidate for module split | done | 2026-05-25 |  |
| 2 | refactor | `refactor-organizations-rs-hot` | organizations.rs churn-hot — 12,060 lines over 14d (multitenancy + admin) | done | 2026-05-25 |  |
| 2 | security | `security-inquiry-read-idor` | IDOR: reality-server mark_as_read flips any realtor's inquiry by ID with no owner scoping | done | 2026-05-25 | [plan](plans/_archive/security-inquiry-read-idor.md) |
| 2 | security | `security-role-gate-fail-open` | Latent fail-open: ProtectedRoute role check is skipped when user.role is falsy | done | 2026-05-25 |  |
| 2 | test-gap | `test-gap-screen-map-drift-ppt-neighbors` | Screen-map drift: PR #464 wired a neighbors route in ppt-web without a docs/screens/ppt entry | done | 2026-05-25 |  |
| 2 | test-gap | `test-gap-screen-map-drift-reality-listing` | Screen-map drift: PR #460 touched reality-web listing page without a docs/screens/reality update | closed | 2026-05-25 |  |
| 2 | refactor | `refactor-dead-dup-handler-modules` | Dead/duplicate handler modules: AuthHandler & BuildingHandler unused, routes reimplement inline | done | 2026-05-24 |  |
| 2 | security | `security-rls-migration-residual` | Complete RLS migration in 31 remaining handlers (voting, market_pricing, faults, notif_prefs, reports) | done | 2026-05-23 |  |
| 1 | bug | `code-review-mobile-rn-fault-category-i18n` | frontend/apps/mobile/src/screens/faults/ReportFaultScreen.tsx:340 — getCategoryLabel() returns hardcoded English strings | open | 2026-07-14 |  |
| 1 | bug | `code-review-mobile-rn-meter-console-error` | frontend/apps/mobile/src/screens/meters/MeterReadingScreen.tsx:149 — console.error('Failed to submit meter reading:', er | open | 2026-07-14 |  |
| 1 | bug | `code-review-mobile-rn-onboarding-untested` | frontend/apps/mobile/src/onboarding/ (FeedbackManager.ts, HelpCenter.ts, TourManager.ts) — singleton managers consumed b | open | 2026-07-14 |  |
| 1 | bug | `code-review-mobile-rn-queue-swallow-error` | frontend/apps/mobile/src/screens/faults/ReportFaultScreen.tsx:327-334 — network failure in handleSubmit is caught, the f | open | 2026-07-14 |  |
| 1 | test | `code-review-mobile-rn-voice-module-no-tests` | frontend/apps/mobile/src/voice/VoiceAssistant.ts — 311-line class (Epic 49 Story 49.2) with async native-module bridging | open | 2026-07-14 |  |
| 1 | bug | `code-review-ppt-web-core-sessions-cast` | frontend/apps/ppt-web/src/features/settings/pages/SessionsPage.tsx:84 — (sessionsQuery.data?.sessions ?? []) as unknown  | open | 2026-07-14 |  |
| 1 | bug | `code-review-mobile-rn-widget-switch-undef` | frontend/apps/mobile/src/widgets/WidgetDataProvider.ts:44-59 — fetchWidgetData() switches over config.type with no defau | open | 2026-07-14 |  |
| 1 | refactor | `churn-hotspot-backend/crates/integrations/src/booking/mod.rs` | 3185 add+del lines this run across PR #2315 (Booking.com fetch_property happy-path test) | open | 2026-07-14 |  |
| 1 | refactor | `refactor-churn-hotspot-backend-integrations-booking-mod` | backend integrations booking/mod.rs — instability watch after PR #2176 split | done | 2026-07-09 |  |
| 1 | test-gap | `test-gap-repeated-churn-oauth-integration-tests` | oauth_integration_tests.rs repeated-churn (runs_seen 2→3) — OAuth handlers still moving | dropped | 2026-07-09 |  |
| 1 | refactor | `refactor-churn-hotspot-api-server-routes-auth` | api-server routes/auth.rs — repeated hotspot + 3 static-review findings this run | done | 2026-07-09 |  |
| 1 | bug | `bug-refresh-empty-cookie-shadows-body-token` | /refresh and /logout — empty refresh_token cookie shadows valid body token | done | 2026-07-09 |  |
| 1 | bug | `code-review-mobile-native-kmp-deeplink-token-not-url-decoded` | DeepLinkRouter skips URL-decoding while Android Uri.getQueryParameter decodes — SSO tokens diverge per platform | dropped | 2026-07-05 |  |
| 1 | bug | `code-review-mobile-native-kmp-search-stale-response-race` | SearchScreen stale-response race — overlapping searches can clobber newer results | dropped | 2026-07-05 |  |
| 1 | test-gap | `test-gap-screen-map-drift-pr-1085-reality` | Screen-map drift: PR #1085 modified reality-web listing detail metadata + page without screen-doc update | dropped | 2026-07-05 |  |
| 1 | test-gap | `test-gap-screen-map-drift-pr-1100-ppt` | Screen-map drift: PR #1100 modified ppt-web App.tsx (FileDisputePageRoute extraction) without screen-doc update | dropped | 2026-07-05 |  |
| 1 | bug | `bug-risky-churn-pr-963-api-main-rs` | Risky churn: api-server main.rs security-headers wiring shipped without a middleware smoke test | dropped | 2026-07-05 |  |
| 1 | test-gap | `test-gap-screen-map-drift-pr-922-ppt` | Screen-map drift: PR #922 modified ppt-web App.tsx (dev-review rounds 1-5 fixes) without a docs/screens/ppt update | done | 2026-07-05 |  |
| 1 | refactor | `churn-hotspot-mobile-native-listing-detail-kt` | Churn hotspot: ListingDetailScreen.kt — +1279 LOC this run (gap-82-4 reality mobile favorite toggle) | done | 2026-07-05 |  |
| 1 | refactor | `refactor-churn-hotspot-mobile-documents` | Churn hotspot: DocumentsScreen.tsx — 3 PRs this run | done | 2026-07-05 |  |
| 1 | bug | `bug-ios-deeplink-info-plist-missing` | iOS deep-link layer dead at runtime — Info.plist missing CFBundleURLTypes + applinks entitlement | dropped | 2026-07-05 |  |
| 1 | test-gap | `test-gap-hotfix-no-test-pr-1288-webhook-rls` | Webhook handlers RLS migration (PR #1288, PAP-170) shipped without a new regression test for repo-layer methods | dropped | 2026-07-05 |  |
| 1 | test-gap | `test-gap-hotfix-no-test-pr-1287-rls-llm-sessions` | AI llm/sessions + integrations sync + subscriptions RLS migration (PR #1287, PAP-169) shipped without a new regression test | dropped | 2026-07-05 |  |
| 1 | test-gap | `test-gap-hotfix-no-test-pr-1289-api-ecosystem` | api_ecosystem.rs RLS migration (PR #1289, PAP-167) — 162-line handler rework shipped without a regression test for the public-connection routing | dropped | 2026-07-05 |  |
| 1 | test-gap | `test-gap-hotfix-no-test-pr-1292-mfa-rls` | mfa.rs RLS migration (PR #1292, PAP-168) shipped without a regression test; also landed broken and was hotfixed in PR #1287 | dropped | 2026-07-05 |  |
| 1 | refactor | `code-review-api-core-osrng-expect` | crypto.rs:127 SysRng.try_fill_bytes(...).expect() panics if OS CSPRNG errors during integration-credential encrypt | done | 2026-07-05 |  |
| 1 | bug | `code-review-mobile-rn-screens-mock-data` | Mobile RN production screens (Buildings/Meters/Leases/PersonMonths/Notifications/Threads/Forms) render hardcoded MOCK_* arrays — no API wiring | done | 2026-07-05 |  |
| 1 | bug | `code-review-mobile-rn-deeplink-init-unhandled` | useDeepLinkRouting.ts:27-36 — initialize() re-runs on onNavigate identity change + void promise with no .catch → duplicate nav / unhandled rejection | dropped | 2026-07-05 |  |
| 1 | refactor | `refactor-churn-hotspot-backend-crates-db-src-models-mod-rs` | Churn hotspot: backend/crates/db/src/models/mod.rs (12 commits in 19-day catch-up) | dropped | 2026-07-05 |  |
| 1 | refactor | `refactor-churn-hotspot-backend-crates-db-src-repositories-rental-rs` | Churn hotspot: backend/crates/db/src/repositories/rental.rs (11 commits in 19-day catch-up) | done | 2026-07-05 |  |
| 1 | refactor | `refactor-closed-not-merged-pr-1378` | PR #1378 closed without merge — DROP-OWNED-BY teardown theory for #1332 was wrong root cause, superseded by #1379 | done | 2026-06-15 |  |
| 1 | test-gap | `code-review-issue-1137-pkce-test-tautology` | PKCE unit test became a tautology after services/oauth.rs DRY refactor (#1132) | done | 2026-06-07 |  |
| 1 | dx | `dx-nginx-template-churn-2026-06-03` | docker/nginx admin-web + ppt-web templates churned twice this run (security headers + redirects) | done | 2026-06-06 |  |
| 1 | refactor | `refactor-ai-rs-hot` | ai.rs churn-hot — 3,142 lines this run; 3,142-line route monolith, candidate for module split | done | 2026-06-06 |  |
| 1 | refactor | `refactor-auth-refresh-spec-churn-2026-06-04` | ppt-web e2e auth-refresh.spec.ts added (+252 lines, story 79-2 token-refresh coverage) | done | 2026-06-06 |  |
| 1 | refactor | `refactor-esignature-webhook-idempotency-tests-churn-2026-06-04` | api-server esignature_webhook_idempotency_tests.rs added (+228 lines, terminal-state regression) | done | 2026-06-06 |  |
| 1 | refactor | `refactor-evidenceuploader-test-tsx-churn-2026-06-04` | ppt-web EvidenceUploader.test.tsx added (+202 lines, dispute-filing AC-2 regression) | done | 2026-06-06 |  |
| 1 | refactor | `refactor-main-rs-churn-2026-06-03` | api-server main.rs touched twice this run (gap-sweep + security headers) — minor churn marker | done | 2026-06-06 |  |
| 1 | refactor | `refactor-mediation-duplicated-spinner` | Duplicated animate-spin spinner markup across mediation page + chat thread (no shared Spinner) | done | 2026-06-06 |  |
| 1 | refactor | `refactor-mediation-ref-number-full-uuid` | Mediation reference number uppercases full UUID (DSP-<uuid>) instead of a short code | done | 2026-06-06 |  |
| 1 | refactor | `refactor-mobile-app-tsx-churn-2026-06-03` | frontend/apps/mobile/src/App.tsx churned twice this run (universal links + doc-detail wiring) | done | 2026-06-06 |  |
| 1 | refactor | `refactor-platform-admin-rs-hot` | platform_admin.rs churn-hot — 2,762 lines this run (admin/OAuth-provider feature work) | done | 2026-06-06 |  |
| 1 | refactor | `refactor-reality-web-comparison-i18n` | Reality-web ComparisonUrlHandler hardcodes English loading/error strings | done | 2026-06-06 |  |
| 1 | refactor | `refactor-routes-oauth-rs-hot` | Watch routes/oauth.rs churn after audit-log + hardening PRs | done | 2026-06-06 |  |
| 1 | refactor | `refactor-services-oauth-rs-hot` | Watch services/oauth.rs churn after introspect/revoke hardening (#933) | done | 2026-06-06 |  |
| 1 | test-gap | `test-gap-mobile-voting-transforms` | Mobile VotingScreen pure transforms toUiStatus/toUiVote have no tests | done | 2026-06-06 |  |
| 1 | bug | `bug-mobile-voting-hardcoded-locale` | Mobile VotingScreen hardcodes en-US in toLocaleDateString — vote dates never localize | done | 2026-06-05 |  |
| 1 | bug | `bug-reality-web-listing-metadata-ssr-throw` | Reality-web listing generateMetadata can throw during SSR on malformed 200 body | done | 2026-06-05 |  |
| 1 | security | `security-pkce-oauth-authcode-pr-908-closed` | PR #908 (fix(security): require PKCE on OAuth authorization-code flow, closes #823) was closed unmerged — verify whether PKCE enforcement still pending | done | 2026-06-03 |  |
| 0 | dx | `dx-routine-lag-catchup-2026-07` | Cloud routine cadence recovery — reduce 3–4d gaps between runs | dropped | 2026-07-09 |  |
| 0 | refactor | `churn-hotspot-backend-servers-api-server-src-routes-emergency-rs` | Churn hotspot: 1021 lines changed in backend/servers/api-server/src/routes/emergency.rs (window 2026 | dropped | 2026-07-05 |  |
| 0 | refactor | `churn-hotspot-backend-servers-api-server-src-routes-vendors-rs` | Churn hotspot: 929 lines changed in backend/servers/api-server/src/routes/vendors.rs (window 2026-06 | dropped | 2026-07-05 |  |
| 0 | refactor | `churn-hotspot-backend-servers-api-server-src-routes-enhanced-tenant-screening-rs` | Churn hotspot: 709 lines changed in backend/servers/api-server/src/routes/enhanced_tenant_screening. | dropped | 2026-07-05 |  |
| 0 | refactor | `churn-hotspot-backend-crates-db-src-repositories-document-rs` | Churn hotspot: 2940 lines changed in backend/crates/db/src/repositories/document.rs (window 2026-06-10 03:05Z→18:30Z) | dropped | 2026-07-05 |  |
| 0 | refactor | `churn-hotspot-backend-crates-db-src-repositories-subscription-rs` | Churn hotspot: 2856 lines changed in backend/crates/db/src/repositories/subscription.rs (window 2026-06-10 03:05Z→18:30Z) | dropped | 2026-07-05 |  |
| 0 | refactor | `churn-hotspot-backend-servers-api-server-src-routes-aml-dsa-rs` | Churn hotspot: 2691 lines changed in backend/servers/api-server/src/routes/aml_dsa.rs (window 2026-06-10 03:05Z→18:30Z) | dropped | 2026-07-05 |  |
| 0 | refactor | `churn-hotspot-mobile-native-search-screen-kt` | Churn hotspot: SearchScreen.kt — +1293 LOC this run (gap-82-3 reality mobile search/filters) | dropped | 2026-07-05 |  |
| 0 | refactor | `code-review-mobile-native-kmp-deeplink-android-bypasses-shared-router` | MainActivity reimplements deep-link dispatch instead of calling shared DeepLinkRouter — drift trap | dropped | 2026-07-05 |  |
| 0 | refactor | `refactor-churn-hotspot-mobile-announcements` | Churn hotspot: AnnouncementsScreen.tsx — 4 PRs this run, instability proxy | dropped | 2026-07-05 |  |
| 0 | refactor | `refactor-churn-hotspot-mobile-announcements-test` | Churn hotspot: AnnouncementsScreen.test.ts — 4 PRs this run, instability proxy | dropped | 2026-07-05 |  |
| 0 | dx | `closed-not-merged-pr-1274` | PR #1274 (cargo-minor-patch group, /backend, 9 updates) closed unmerged — superseded by #1313 after auto-rebase fix landed | dropped | 2026-07-05 |  |
| 0 | refactor | `churn-hotspot-backend-crates-integrations-src-booking-rs` | Churn hotspot: backend/crates/integrations/src/booking.rs (+404/−29 in PR #1294 Booking.com OTA retry) | dropped | 2026-07-05 |  |
| 0 | refactor | `churn-hotspot-backend-servers-api-server-src-routes-api-ecosystem-rs` | Churn hotspot: backend/servers/api-server/src/routes/api_ecosystem.rs (+106/−27 in PR #1293 PAP-171; second touch in 24h) | dropped | 2026-07-05 |  |
| 0 | refactor | `churn-hotspot-backend-crates-db-src-repositories-reality-portal-rs` | Churn hotspot: backend/crates/db/src/repositories/reality_portal.rs (+59/−28 in PR #1297 PAP-142 IDOR scoping) | dropped | 2026-07-05 |  |
| 0 | refactor | `churn-hotspot-backend-servers-api-server-src-routes-iot-rs` | Churn hotspot: backend/servers/api-server/src/routes/iot.rs (+278/-403 in PR #1321/#1322 PAP-151 re-land + fmt) | dropped | 2026-07-05 |  |
| 0 | refactor | `churn-hotspot-backend-servers-api-server-src-routes-reserve-funds-rs` | Churn hotspot: backend/servers/api-server/src/routes/reserve_funds.rs (+228/-255 in PR #1321 PAP-151 re-land) | dropped | 2026-07-05 |  |
| 0 | refactor | `churn-hotspot-backend-crates-db-src-repositories-sensor-rs` | Churn hotspot: backend/crates/db/src/repositories/sensor.rs (+248/-86 in PR #1321/#1322 PAP-151 re-land + fmt) | dropped | 2026-07-05 |  |
| 0 | dx | `dx-stalled-review-pr-988` | Stalled review: PR #988 (Epic: reusable Playwright E2E framework + sitemap FlowRunner) open 10d, no reviewDecision | dropped | 2026-07-05 |  |
| 0 | refactor | `churn-hotspot-backend-servers-api-server-src-routes-forms-rs` | Churn hotspot: backend/servers/api-server/src/routes/forms.rs touched 2x since 2026-06-12 (window 2026-06-12→2026-06-13) | dropped | 2026-07-05 |  |
| 0 | refactor | `churn-hotspot-backend-servers-api-server-tests-reserve-funds-cross-org-idor-tests-rs` | Churn hotspot: backend/servers/api-server/tests/reserve_funds_cross_org_idor_tests.rs touched 2x since 2026-06-12 (window 2026-06-12→2026-06-13) | dropped | 2026-07-05 |  |
| 0 | refactor | `churn-hotspot-backend-crates-db-tests-form-rls-repo-tests-rs` | Churn hotspot: backend/crates/db/tests/form_rls_repo_tests.rs touched 2x since 2026-06-12 (window 2026-06-12→2026-06-13) | dropped | 2026-07-05 |  |
| 0 | refactor | `churn-hotspot-frontend-apps-mobile-app-config-icon-test-ts` | Churn hotspot: 124 lines in frontend/apps/mobile/app.config.icon.test.ts (PR #1383 gap-85-2) | dropped | 2026-07-05 |  |
| 0 | refactor | `churn-hotspot-frontend-apps-mobile-app-config-ts` | Churn hotspot: 94 lines in frontend/apps/mobile/app.config.ts (PR #1383 gap-85-2) | dropped | 2026-07-05 |  |
| 0 | refactor | `churn-hotspot-backend-crates-db-src-repositories-form-rs` | Churn hotspot: 53 lines in backend/crates/db/src/repositories/form.rs (PR #1379 #1332 unblock) | dropped | 2026-07-05 |  |
| 0 | refactor | `churn-hotspot-backend-servers-api-server-tests-booking-oauth-csrf-tests-rs` | booking_oauth_csrf_tests.rs hotspot — 484-line NEW test file (PR #1393 #1424 OAuth CSRF coverage) | dropped | 2026-07-05 |  |
| 0 | refactor | `churn-hotspot-backend-servers-api-server-tests-booking-oauth-routes-tests-rs` | booking_oauth_routes_tests.rs hotspot — 381-line NEW test file (PR #1393 OAuth routes coverage) | dropped | 2026-07-05 |  |
| 0 | refactor | `repeated-churn-backend-servers-api-server-src-routes-forms-rs` | forms.rs repeated-churn — runs_seen=2 (#1337 explicit_auto_deref + #1397 org-scope hardening) | dropped | 2026-07-05 |  |
| 0 | dx | `closed-not-merged-pr-1425` | PR #1425 (GH #1377 document presigned-URL tests) closed unmerged — superseded by merged #1394 | dropped | 2026-07-05 |  |
| 0 | dx | `dx-stalled-review-pr-1179` | PR #1179 (docs(epics) catalog backfill for 37 mounted-but-undocumented backend modules) — stalled at 7d, no reviewDecision | dropped | 2026-07-05 |  |
| 0 | refactor | `refactor-oauth-integration-tests-hot` | Stabilize oauth_integration_tests churn — heavy edits across 3 OAuth fix PRs | dropped | 2026-06-16 |  |
| 0 | bug | `bug-announcer-stale-message` | Announcer: untracked clear-then-set timeouts can resurrect a stale screen-reader message | dropped | 2026-06-07 |  |
| 0 | dx | `dx-portfolio-dashboard-stubs` | Portfolio dashboard: alert mark-read/resolve mutations + property-card click navigation are no-op stubs | dropped | 2026-06-04 |  |
