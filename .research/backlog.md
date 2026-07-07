# Backlog of vectors
<sub>Last regenerated: 2026-07-07 04:00 UTC by routine</sub>

| Score | Vector | ID | Title | Status | Updated | Plan |
|-------|--------|----|----|--------|---------|------|
| 6 | test-gap | `test-gap-inquiry-idor-regression` | Add regression tests for inquiry mark_as_read cross-tenant IDOR fix (PR #497) | done | 2026-05-26 | [plan](plans/_archive/test-gap-inquiry-idor-regression.md) |
| 3 | security | `code-review-ppt-web-core-accounting-leak-on-logout` | frontend/apps/ppt-web/src/lib/queryKeys.ts:347-368 — AUTHED_QUERY_KEY_ROOTS om | ready | 2026-07-07 | [plan](plans/code-review-ppt-web-core-accounting-leak-on-logout.md) |
| 3 | bug | `code-review-mobile-rn-report-fault-fake-submit` | ReportFaultScreen.tsx handleSubmit() fakes API call with setTimeout(1500) — fault reports nev… | ready | 2026-06-16 | [plan](plans/code-review-mobile-rn-report-fault-fake-submit.md) |
| 3 | bug | `code-review-reality-web-realtor-mgmt-untranslated` | Reality-web RealtorManagement.tsx hardcoded English strings — agency flow not localized to sk… | ready | 2026-06-15 | [plan](plans/code-review-reality-web-realtor-mgmt-untranslated.md) |
| 3 | bug | `code-review-reality-web-share-comparison-404` | Reality-web ComparisonUrlHandler hits non-existent /api/listings/${id} — every shared compari… | ready | 2026-06-14 | [plan](plans/code-review-reality-web-share-comparison-404.md) |
| 3 | bug | `code-review-reality-web-listing-page-ssr-crash` | Reality-web listing detail SSR crashes on partial 200 body — JSON-LD build deref of undefined… | ready | 2026-06-14 | [plan](plans/code-review-reality-web-listing-page-ssr-crash.md) |
| 3 | bug | `bug-ios-searchview-uncompilable` | iOS SearchView.swift does not compile — performSearch/scheduleSearch undefined, resultsGrid c… | ready | 2026-06-11 | [plan](plans/bug-ios-searchview-uncompilable.md) |
| 3 | security | `unchecked-todo-pr-1203` | PR #1203 (fix(aml_dsa): close cross-tenant IDOR in moderation + AML-review handlers (PAP-36))… | dropped | 2026-06-10 |  |
| 3 | security | `unchecked-todo-pr-1193` | PR #1193 (fix(aml-dsa): lock DSA reports to platform roles + fix file-path disclosure (PAP-47… | dropped | 2026-06-10 |  |
| 3 | bug | `bug-schema-drift-runtime-sql-issue-1008` | Schema drift: runtime SQL errors from non-existent columns in voting/messaging/notification p… | done | 2026-06-07 |  |
| 3 | security | `security-llm-doc-idor` | IDOR: ai.rs LLM-doc handlers publish/list/get any tenant's listing descriptions & photo enhan… | ready | 2026-06-01 | [plan](plans/security-llm-doc-idor.md) |
| 3 | security | `security-realtors-mark-inquiry-read-idor` | IDOR: reality-server realtors mark_inquiry_read flips any realtor's inquiry by ID with no own… | done | 2026-05-26 | [plan](plans/_archive/security-realtors-mark-inquiry-read-idor.md) |
| 3 | security | `security-equipment-idor` | IDOR: equipment delete/update + maintenance update mutate any tenant's equipment by ID with n… | done | 2026-05-25 | [plan](plans/_archive/security-equipment-idor.md) |
| 3 | security | `security-ssrf-outbound-url-validation` | SSRF: signed-document fetch + webhook-test POST issue outbound requests to unvalidated user-c… | done | 2026-05-25 | [plan](plans/_archive/security-ssrf-outbound-url-validation.md) |
| 3 | security | `security-voice-device-idor` | IDOR: unlink_voice_device deactivates any device by ID with no owner/org scoping | done | 2026-05-25 | [plan](plans/_archive/security-voice-device-idor.md) |
| 2 | bug | `code-review-ppt-web-core-perf-metrics-listener-leak` | frontend/apps/ppt-web/src/hooks/usePerformanceMetrics.ts:142,152-158 — window. | open | 2026-07-07 |  |
| 2 | test-gap | `test-gap-hotfix-no-test-pr-959-reality-listings-pagination` | Reality-server listings pagination clamp (PR #959) shipped without a regression test for limi… | done | 2026-07-05 |  |
| 2 | test-gap | `screen-map-drift-pr-1418-ppt` | PR #1418 touched routes/** (faults.route.test.tsx) without updating docs/screens/ppt/* — heur… | done | 2026-07-05 |  |
| 2 | bug | `code-review-api-core-vote-partial-cmp-panic` | vote.rs:1765 calculate_question_result() uses partial_cmp().unwrap() on f64 — NaN/Inf weights… | done | 2026-06-16 |  |
| 2 | test-gap | `unchecked-todo-pr-1196` | PR #1196 (feat(ppt-web): add missing test coverage for faults feature) merged with 2 unchecke… | dropped | 2026-06-10 |  |
| 2 | dx | `dx-push-fanout-blpop-drain` | PushFanoutWorker BLPOP queue-drain deferred — Redis path is a logging no-op | done | 2026-06-06 |  |
| 2 | refactor | `refactor-ai-rs-module-split` | ai.rs (3,134 LOC) — explicit module-split into routes/ai/{sessions,equipment,workflows,voice,… | done | 2026-06-06 |  |
| 2 | refactor | `refactor-announcements-rs-hot` | announcements.rs churn-hot — 2,722 lines this run (Epic 2B + Epic 6 work) | done | 2026-06-06 |  |
| 2 | refactor | `refactor-announcements-rs-module-split` | announcements.rs (2,722 LOC) — explicit module-split into routes/announcements/{crud,targetin… | done | 2026-06-06 |  |
| 2 | refactor | `refactor-app-tsx-route-coupling` | Reduce App.tsx route-aggregator coupling (top churn hotspot, merge-conflict risk) | done | 2026-06-06 |  |
| 2 | refactor | `refactor-platform-admin-rs-module-split` | platform_admin.rs (2,762 LOC) — explicit module-split into routes/platform_admin/{tenants,fea… | done | 2026-06-06 |  |
| 2 | test-gap | `test-gap-screen-map-drift-pr-1033-ppt` | Screen-map drift: PR #1033 wired error/retry into AnnouncementsPage+FaultsPage via App.tsx wi… | done | 2026-06-06 |  |
| 2 | bug | `bug-risky-churn-pr-992-mobile-app-tsx` | Risky churn: mobile App.tsx deep-link/doc-detail wiring changing across back-to-back PRs with… | done | 2026-06-05 |  |
| 2 | dx | `dx-integration-marketplace-stubs` | Integration marketplace install/OAuth flows are placeholders — wire backend handlers + UI nav… | done | 2026-06-05 |  |
| 2 | test-gap | `test-gap-booking-push-validation-untested` | Booking push availability/rates endpoints add batch-cap + non-negative guards with no regress… | done | 2026-06-05 |  |
| 2 | test-gap | `test-gap-hotfix-no-test-pr-874-portal-webhooks` | Portal webhook fail-closed fix (PR #874) shipped without a regression test for unverified-sig… | done | 2026-06-05 |  |
| 2 | test-gap | `test-gap-hotfix-no-test-pr-918-mobile-dev-review` | Mobile dev-review batch (PR #918, 5 files under frontend/apps/mobile/src) shipped without a r… | done | 2026-06-05 |  |
| 2 | test-gap | `test-gap-hotfix-no-test-pr-921-sso-consumer` | Reality-server SSO consumer review fix (PR #921, closes #820) shipped without a regression test | done | 2026-06-05 |  |
| 2 | test-gap | `test-gap-hotfix-no-test-pr-923-branch-protection-rebase` | CI branch-protection + auto-rebase workflow change (PR #923) shipped without an integration t… | done | 2026-06-05 |  |
| 2 | test-gap | `test-gap-hotfix-no-test-pr-939-deploy-server-scopes` | deploy-server OIDC scope mapping (#939) shipped without unit test for derive_oidc_scopes | done | 2026-06-05 |  |
| 2 | test-gap | `test-gap-hotfix-no-test-pr-943-mobile-dev-review-tail` | Mobile RN dev-review tail (#943) shipped without test coverage | done | 2026-06-05 |  |
| 2 | test-gap | `test-gap-hotfix-no-test-pr-990-frontend-gap-sweep` | Frontend gap-sweep (PR #990, 34 files across Epics 1/6/7B/9/10B/11/15/17/18) shipped without … | done | 2026-06-05 |  |
| 2 | test-gap | `test-gap-hotfix-no-test-pr-992-mobile-doc-detail` | Mobile document-detail wiring (PR #992) shipped without a regression test for the deep-link p… | done | 2026-06-05 |  |
| 2 | test-gap | `test-gap-screen-map-drift-pr-839-ppt` | Screen-map drift: PR #839 modified ppt-web App.tsx (FileDisputePageRoute) without a docs/scre… | done | 2026-06-05 |  |
| 2 | refactor | `refactor-ppt-web-untranslated-strings` | ppt-web status/auth components hardcode English in an otherwise i18n'd app | done | 2026-06-04 |  |
| 2 | bug | `bug-mediation-page-no-error-state` | MediationWorkspacePage shows empty/unknown state instead of error UI on dispute fetch failure | done | 2026-06-03 |  |
| 2 | bug | `bug-mobile-voting-unsafe-cast` | Mobile VotingScreen double-casts API result across boundary — render-time crash on unexpected… | done | 2026-06-03 |  |
| 2 | bug | `bug-reality-web-realtor-invite-silent-error` | Reality-web InviteRealtorModal swallows invite-mutation failure with no error UI | done | 2026-06-03 |  |
| 2 | bug | `bug-webhook-airbnb-dup-sync-jobs` | Airbnb webhook at-least-once delivery enqueues duplicate SYNC_EXTERNAL jobs | done | 2026-06-03 |  |
| 2 | dx | `dx-documentsbrowse-folder-preselect` | DocumentsBrowse MoveFolderDialog cannot pre-select current folder (DocumentSummary lacks fold… | done | 2026-06-03 |  |
| 2 | test-gap | `test-gap-hotfix-no-test-pr-963-security-headers` | API + SPA security-headers middleware (PR #963) shipped without an assertion test for HSTS/no… | done | 2026-06-03 |  |
| 2 | test-gap | `test-gap-router-set-parity-tests` | api-server main.rs vs lib.rs::create_router diverge silently (5 routes unreachable in prod, n… | done | 2026-06-01 |  |
| 2 | bug | `bug-report-schedule-update-no-sql` | ReportSchedule.update_schedule stores cron in `time` workaround; documented UPDATE never runs… | done | 2026-05-30 |  |
| 2 | test-gap | `test-gap-screen-map-drift-ppt-report-history` | Screen-map drift: report execution-history route (PR #547) added without a ppt screen doc | done | 2026-05-27 |  |
| 2 | test-gap | `test-gap-dispute-fsm-no-tests` | Dispute state machine (PR #506) shipped with no tests + no org predicate on update_status | done | 2026-05-26 |  |
| 2 | refactor | `refactor-documents-rs-hot` | documents.rs churn-hot — 10,659 lines over 14d | done | 2026-05-25 |  |
| 2 | refactor | `refactor-integrations-rs-hot` | integrations.rs churn-hot — 12,977 lines over 14d, candidate for module split | done | 2026-05-25 |  |
| 2 | refactor | `refactor-organizations-rs-hot` | organizations.rs churn-hot — 12,060 lines over 14d (multitenancy + admin) | done | 2026-05-25 |  |
| 2 | security | `security-inquiry-read-idor` | IDOR: reality-server mark_as_read flips any realtor's inquiry by ID with no owner scoping | done | 2026-05-25 | [plan](plans/_archive/security-inquiry-read-idor.md) |
| 2 | security | `security-role-gate-fail-open` | Latent fail-open: ProtectedRoute role check is skipped when user.role is falsy | done | 2026-05-25 |  |
| 2 | test-gap | `test-gap-screen-map-drift-ppt-neighbors` | Screen-map drift: PR #464 wired a neighbors route in ppt-web without a docs/screens/ppt entry | done | 2026-05-25 |  |
| 2 | test-gap | `test-gap-screen-map-drift-reality-listing` | Screen-map drift: PR #460 touched reality-web listing page without a docs/screens/reality upd… | closed | 2026-05-25 |  |
| 2 | refactor | `refactor-dead-dup-handler-modules` | Dead/duplicate handler modules: AuthHandler & BuildingHandler unused, routes reimplement inline | done | 2026-05-24 |  |
| 2 | security | `security-rls-migration-residual` | Complete RLS migration in 31 remaining handlers (voting, market_pricing, faults, notif_prefs,… | done | 2026-05-23 |  |
| 1 | bug | `code-review-mobile-rn-screens-mock-data` | Mobile RN production screens (Buildings/Meters/Leases/PersonMonths/Notifications/Threads/Form… | done | 2026-07-07 |  |
| 1 | refactor | `refactor-churn-hotspot-backend-crates-db-src-repositories-rental-rs` | Churn hotspot: backend/crates/db/src/repositories/rental.rs (11 commits in 19-day catch-up) | open | 2026-07-07 |  |
| 1 | refactor | `refactor-churn-hotspot-backend-crates-db-src-repositories-rental-oauth-rs` | rental/oauth.rs +594 lines this run — new sub-module created by PR #2100 renta | open | 2026-07-07 |  |
| 1 | refactor | `refactor-churn-hotspot-backend-crates-db-src-repositories-rental-bookings-rs` | rental/bookings.rs +527 lines this run — new sub-module from PR #2100 rental s | open | 2026-07-07 |  |
| 1 | bug | `code-review-ppt-web-core-auth-init-skips-role-rederive` | frontend/apps/ppt-web/src/contexts/AuthContext.tsx:400-410 — mount-time init r | open | 2026-07-07 |  |
| 1 | bug | `code-review-mobile-native-kmp-deeplink-token-not-url-decoded` | DeepLinkRouter skips URL-decoding while Android Uri.getQueryParameter decodes — SSO tokens di… | open | 2026-07-05 |  |
| 1 | bug | `code-review-mobile-native-kmp-search-stale-response-race` | SearchScreen stale-response race — overlapping searches can clobber newer results | open | 2026-07-05 |  |
| 1 | test-gap | `test-gap-screen-map-drift-pr-1085-reality` | Screen-map drift: PR #1085 modified reality-web listing detail metadata + page without screen… | open | 2026-07-05 |  |
| 1 | test-gap | `test-gap-screen-map-drift-pr-1100-ppt` | Screen-map drift: PR #1100 modified ppt-web App.tsx (FileDisputePageRoute extraction) without… | open | 2026-07-05 |  |
| 1 | bug | `bug-risky-churn-pr-963-api-main-rs` | Risky churn: api-server main.rs security-headers wiring shipped without a middleware smoke test | open | 2026-07-05 |  |
| 1 | test-gap | `test-gap-screen-map-drift-pr-922-ppt` | Screen-map drift: PR #922 modified ppt-web App.tsx (dev-review rounds 1-5 fixes) without a do… | done | 2026-07-05 |  |
| 1 | refactor | `churn-hotspot-mobile-native-listing-detail-kt` | Churn hotspot: ListingDetailScreen.kt — +1279 LOC this run (gap-82-4 reality mobile favorite … | done | 2026-07-05 |  |
| 1 | refactor | `refactor-churn-hotspot-mobile-documents` | Churn hotspot: DocumentsScreen.tsx — 3 PRs this run | done | 2026-07-05 |  |
| 1 | bug | `bug-ios-deeplink-info-plist-missing` | iOS deep-link layer dead at runtime — Info.plist missing CFBundleURLTypes + applinks entitlem… | open | 2026-07-05 |  |
| 1 | test-gap | `test-gap-hotfix-no-test-pr-1288-webhook-rls` | Webhook handlers RLS migration (PR #1288, PAP-170) shipped without a new regression test for … | open | 2026-07-05 |  |
| 1 | test-gap | `test-gap-hotfix-no-test-pr-1287-rls-llm-sessions` | AI llm/sessions + integrations sync + subscriptions RLS migration (PR #1287, PAP-169) shipped… | open | 2026-07-05 |  |
| 1 | test-gap | `test-gap-hotfix-no-test-pr-1289-api-ecosystem` | api_ecosystem.rs RLS migration (PR #1289, PAP-167) — 162-line handler rework shipped without … | open | 2026-07-05 |  |
| 1 | test-gap | `test-gap-hotfix-no-test-pr-1292-mfa-rls` | mfa.rs RLS migration (PR #1292, PAP-168) shipped without a regression test; also landed broke… | open | 2026-07-05 |  |
| 1 | refactor | `code-review-api-core-osrng-expect` | crypto.rs:127 SysRng.try_fill_bytes(...).expect() panics if OS CSPRNG errors during integrati… | done | 2026-07-05 |  |
| 1 | bug | `code-review-mobile-rn-deeplink-init-unhandled` | useDeepLinkRouting.ts:27-36 — initialize() re-runs on onNavigate identity change + void promi… | open | 2026-07-05 |  |
| 1 | refactor | `refactor-churn-hotspot-backend-crates-db-src-models-mod-rs` | Churn hotspot: backend/crates/db/src/models/mod.rs (12 commits in 19-day catch-up) | open | 2026-07-05 |  |
| 1 | refactor | `refactor-closed-not-merged-pr-1378` | PR #1378 closed without merge — DROP-OWNED-BY teardown theory for #1332 was wrong root cause,… | done | 2026-06-15 |  |
| 1 | test-gap | `code-review-issue-1137-pkce-test-tautology` | PKCE unit test became a tautology after services/oauth.rs DRY refactor (#1132) | done | 2026-06-07 |  |
| 1 | triage | `triage-issue-1061-dispatcher-archive-corruption` | Triage: dispatcher incident — assignments-archive.json corrupted to 1/196 rows on dev branch … | done | 2026-06-07 |  |
| 1 | triage | `triage-issue-950` | Issue #950 (no labels, OPEN): CI: trigger-deploy 403 marks all dev image builds red and block… | done | 2026-06-07 |  |
| 1 | triage | `triage-issue-952` | Issue #952 (no labels, OPEN): [staging] Reality SSO login dead-ends: redirect_uri callback 40… | done | 2026-06-07 |  |
| 1 | triage | `triage-issue-769` | Issue #769 (no labels, OPEN): Current dev review: Deploy server | done | 2026-06-07 |  |
| 1 | triage | `triage-issue-789` | Issue #789 (no labels, OPEN): Dev review rounds 6-10: scheduler, notifications, admin, orgs, … | done | 2026-06-07 |  |
| 1 | dx | `dx-nginx-template-churn-2026-06-03` | docker/nginx admin-web + ppt-web templates churned twice this run (security headers + redirec… | done | 2026-06-06 |  |
| 1 | refactor | `refactor-ai-rs-hot` | ai.rs churn-hot — 3,142 lines this run; 3,142-line route monolith, candidate for module split | done | 2026-06-06 |  |
| 1 | refactor | `refactor-auth-refresh-spec-churn-2026-06-04` | ppt-web e2e auth-refresh.spec.ts added (+252 lines, story 79-2 token-refresh coverage) | done | 2026-06-06 |  |
| 1 | refactor | `refactor-esignature-webhook-idempotency-tests-churn-2026-06-04` | api-server esignature_webhook_idempotency_tests.rs added (+228 lines, terminal-state regressi… | done | 2026-06-06 |  |
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
| 1 | triage | `triage-issue-749` | Issue #749 (no labels, OPEN): Code review findings: Story 6.1 announcement creation and targe… | done | 2026-06-06 |  |
| 1 | triage | `triage-issue-755` | Issue #755 (no labels, OPEN): Current dev review: Epic 8A Notification Preferences | done | 2026-06-06 |  |
| 1 | triage | `triage-issue-764` | Issue #764 (no labels, OPEN): Current dev review: Admin MFA & Auth Hardening | done | 2026-06-06 |  |
| 1 | triage | `triage-issue-765` | Issue #765 (no labels, OPEN): Current dev review: Integrations & Airbnb OAuth | done | 2026-06-06 |  |
| 1 | bug | `bug-mobile-voting-hardcoded-locale` | Mobile VotingScreen hardcodes en-US in toLocaleDateString — vote dates never localize | done | 2026-06-05 |  |
| 1 | bug | `bug-reality-web-listing-metadata-ssr-throw` | Reality-web listing generateMetadata can throw during SSR on malformed 200 body | done | 2026-06-05 |  |
| 1 | security | `security-pkce-oauth-authcode-pr-908-closed` | PR #908 (fix(security): require PKCE on OAuth authorization-code flow, closes #823) was close… | done | 2026-06-03 |  |
| 1 | triage | `triage-issue-751` | Issue #751 (no labels, OPEN): Current dev review: frontend/web/API-client findings | done | 2026-06-02 |  |
| 1 | triage | `triage-issue-752` | Issue #752 (no labels, OPEN): Current dev review: mobile CI tooling findings | done | 2026-06-02 |  |
| 1 | triage | `triage-issue-756` | Issue #756 (no labels, OPEN): Current dev review: Epic 10A OAuth Provider | done | 2026-06-02 |  |
| 1 | triage | `triage-issue-761` | Issue #761 (no labels, OPEN): Current dev review: Epic 84 E-Signature & Leases | done | 2026-06-02 |  |
| 1 | triage | `triage-issue-763` | Issue #763 (no labels, OPEN): Current dev review: Reality Server & Inquiries | done | 2026-06-02 |  |
| 1 | triage | `triage-issue-767` | Issue #767 (no labels, OPEN): Current dev review: Mobile RN Property Management app | done | 2026-06-02 |  |
| 1 | triage | `triage-issue-768` | Issue #768 (no labels, OPEN): Current dev review: Admin-web features (10B) | done | 2026-06-02 |  |
| 1 | triage | `triage-issue-920` | Issue #920 (no labels, OPEN): Announcement targeting not enforced on read (intra-org disclosu… | done | 2026-06-02 |  |
| 1 | triage | `triage-issue-750` | Issue #750 (no labels, OPEN): Current dev review: backend/API/database findings | done | 2026-06-01 |  |
| 1 | triage | `triage-issue-753` | Issue #753 (no labels, OPEN): Current dev review: Epic 6 Announcements & Communication | done | 2026-06-01 |  |
| 1 | triage | `triage-issue-754` | Issue #754 (no labels, OPEN): Current dev review: Epic 7A Basic Document Management | done | 2026-06-01 |  |
| 1 | triage | `triage-issue-757` | Issue #757 (no labels, OPEN): Current dev review: Epic 10B Platform Administration | done | 2026-06-01 |  |
| 1 | triage | `triage-issue-760` | Issue #760 (no labels, OPEN): Current dev review: Epic 79 Disputes & Mediation | done | 2026-06-01 |  |
| 1 | triage | `triage-issue-762` | Issue #762 (no labels, OPEN): Current dev review: Reports & Schedules | done | 2026-06-01 |  |
| 1 | triage | `triage-issue-766` | Issue #766 (no labels, OPEN): Current dev review: AI & LLM routes | done | 2026-06-01 |  |
| 1 | triage | `triage-issue-770` | Issue #770 (no labels, OPEN): Current dev review: Faults & triage | done | 2026-06-01 |  |
| 1 | triage | `triage-issue-771` | Issue #771 (no labels, OPEN): Current dev review: Research dispatcher & CI automation | done | 2026-06-01 |  |
| 1 | triage | `triage-issue-772` | Issue #772 (no labels, OPEN): Current dev review: Auth core (delta confirmation) | done | 2026-06-01 |  |
| 1 | triage | `triage-issue-773` | Issue #773 (no labels, OPEN): Current dev review: Leases & rental | done | 2026-06-01 |  |
| 1 | triage | `triage-issue-774` | Issue #774 (no labels, OPEN): Current dev review: Reality server (broad) | done | 2026-06-01 |  |
| 1 | triage | `triage-issue-775` | Issue #775 (no labels, OPEN): Current dev review: WebSocket realtime | done | 2026-06-01 |  |
| 1 | triage | `triage-issue-776` | Issue #776 (no labels, OPEN): Current dev review: Equipment & audit log | done | 2026-06-01 |  |
| 1 | triage | `triage-issue-777` | Issue #777 (no labels, OPEN): Current dev review: Compliance & GDPR | done | 2026-06-01 |  |
| 1 | triage | `triage-issue-778` | Issue #778 (no labels, OPEN): Current dev review: Marketplace, voting, investor portal, imper… | done | 2026-06-01 |  |
| 1 | triage | `triage-issue-788` | Issue #788 (no labels, OPEN): Dev review rounds 1-5: mobile-native + ppt-web surfaces | done | 2026-06-01 |  |
| 1 | triage | `triage-issue-790` | Issue #790 (no labels, OPEN): Dev review rounds 11-15: vendor, predictive, reality-web, middl… | done | 2026-06-01 |  |
| 1 | triage | `triage-issue-791` | Issue #791 (no labels, OPEN): Dev review rounds 16-20: push, e-sign, portal, webhooks, reserves | done | 2026-06-01 |  |
| 1 | triage | `triage-issue-846` | Issue #846 (no labels, OPEN): Code review: Epics 12+65 — Meters & Energy/ESG (origin/dev) | done | 2026-06-01 |  |
| 1 | triage | `triage-issue-847` | Issue #847 (no labels, OPEN): Code review: Reality-server — Inquiries IDOR (Epics 16–19) (ori… | done | 2026-06-01 |  |
| 1 | triage | `triage-issue-848` | Issue #848 (no labels, OPEN): Code review: Epics 78+134 — Vendor portal stubs & Predictive ma… | done | 2026-06-01 |  |
| 1 | triage | `triage-issue-850` | Issue #850 (no labels, OPEN): Code review: Epics 61+146+42 — Multi-currency, Data residency, … | done | 2026-06-01 |  |
| 1 | triage | `triage-issue-851` | Issue #851 (no labels, OPEN): Code review: Epics 15+105+69 — Listings/syndication & Developer… | done | 2026-06-01 |  |
| 1 | triage | `triage-issue-859` | Issue #859 (no labels, OPEN): sqlx 0.9 breaks runtime decode of Postgres enum columns into Ru… | done | 2026-06-01 |  |
| 1 | triage | `triage-issue-867` | Issue #867 (no labels, OPEN): Tech debt: api-server main.rs duplicates lib.rs::create_router … | done | 2026-06-01 |  |
| 1 | triage | `triage-issue-836` | Issue #836 (no labels, OPEN): Code review: Epic 2B-C — Mobile push & device registration (ori… | done | 2026-05-31 |  |
| 1 | triage | `triage-issue-845` | Issue #845 (no labels, OPEN): Code review: Epic 14 — IoT alerts, correlations, thresholds (or… | done | 2026-05-31 |  |
| 1 | triage | `triage-issue-849` | Issue #849 (no labels, OPEN): Code review: Epic 10B+143 — Admin impersonation, Help, Board me… | done | 2026-05-31 |  |
| 0 | refactor | `churn-hotspot-backend-servers-api-server-src-routes-emergency-rs` | Churn hotspot: 1021 lines changed in backend/servers/api-server/src/routes/emergency.rs (wind… | dropped | 2026-07-05 |  |
| 0 | refactor | `churn-hotspot-backend-servers-api-server-src-routes-vendors-rs` | Churn hotspot: 929 lines changed in backend/servers/api-server/src/routes/vendors.rs (window … | dropped | 2026-07-05 |  |
| 0 | refactor | `churn-hotspot-backend-servers-api-server-src-routes-enhanced-tenant-screening-rs` | Churn hotspot: 709 lines changed in backend/servers/api-server/src/routes/enhanced_tenant_scr… | dropped | 2026-07-05 |  |
| 0 | refactor | `churn-hotspot-backend-crates-db-src-repositories-document-rs` | Churn hotspot: 2940 lines changed in backend/crates/db/src/repositories/document.rs (window 2… | dropped | 2026-07-05 |  |
| 0 | refactor | `churn-hotspot-backend-crates-db-src-repositories-subscription-rs` | Churn hotspot: 2856 lines changed in backend/crates/db/src/repositories/subscription.rs (wind… | dropped | 2026-07-05 |  |
| 0 | refactor | `churn-hotspot-backend-servers-api-server-src-routes-aml-dsa-rs` | Churn hotspot: 2691 lines changed in backend/servers/api-server/src/routes/aml_dsa.rs (window… | dropped | 2026-07-05 |  |
| 0 | triage | `triage-issue-1151` | Issue #1151 (no labels, OPEN): Research dispatcher: claimable buffer is stale — true claimabl… | dropped | 2026-07-05 |  |
| 0 | refactor | `churn-hotspot-mobile-native-search-screen-kt` | Churn hotspot: SearchScreen.kt — +1293 LOC this run (gap-82-3 reality mobile search/filters) | dropped | 2026-07-05 |  |
| 0 | refactor | `code-review-mobile-native-kmp-deeplink-android-bypasses-shared-router` | MainActivity reimplements deep-link dispatch instead of calling shared DeepLinkRouter — drift… | dropped | 2026-07-05 |  |
| 0 | refactor | `refactor-churn-hotspot-mobile-announcements` | Churn hotspot: AnnouncementsScreen.tsx — 4 PRs this run, instability proxy | dropped | 2026-07-05 |  |
| 0 | refactor | `refactor-churn-hotspot-mobile-announcements-test` | Churn hotspot: AnnouncementsScreen.test.ts — 4 PRs this run, instability proxy | dropped | 2026-07-05 |  |
| 0 | triage | `triage-dispatcher-mcp-push-large-file-issue-1014` | Dispatcher action-list.json corruption when MCP push falls back from blocked git push | dropped | 2026-07-05 |  |
| 0 | triage | `triage-issue-951` | Issue #951 (no labels, OPEN): Deploy blocker: api-server requires ESIGN_TOKEN_SECRET + ESIGN_… | dropped | 2026-07-05 |  |
| 0 | dx | `closed-not-merged-pr-1274` | PR #1274 (cargo-minor-patch group, /backend, 9 updates) closed unmerged — superseded by #1313… | dropped | 2026-07-05 |  |
| 0 | refactor | `churn-hotspot-backend-crates-integrations-src-booking-rs` | Churn hotspot: backend/crates/integrations/src/booking.rs (+404/−29 in PR #1294 Booking.com O… | dropped | 2026-07-05 |  |
| 0 | refactor | `churn-hotspot-backend-servers-api-server-src-routes-api-ecosystem-rs` | Churn hotspot: backend/servers/api-server/src/routes/api_ecosystem.rs (+106/−27 in PR #1293 P… | dropped | 2026-07-05 |  |
| 0 | refactor | `churn-hotspot-backend-crates-db-src-repositories-reality-portal-rs` | Churn hotspot: backend/crates/db/src/repositories/reality_portal.rs (+59/−28 in PR #1297 PAP-… | dropped | 2026-07-05 |  |
| 0 | refactor | `churn-hotspot-backend-servers-api-server-src-routes-iot-rs` | Churn hotspot: backend/servers/api-server/src/routes/iot.rs (+278/-403 in PR #1321/#1322 PAP-… | dropped | 2026-07-05 |  |
| 0 | refactor | `churn-hotspot-backend-servers-api-server-src-routes-reserve-funds-rs` | Churn hotspot: backend/servers/api-server/src/routes/reserve_funds.rs (+228/-255 in PR #1321 … | dropped | 2026-07-05 |  |
| 0 | refactor | `churn-hotspot-backend-crates-db-src-repositories-sensor-rs` | Churn hotspot: backend/crates/db/src/repositories/sensor.rs (+248/-86 in PR #1321/#1322 PAP-1… | dropped | 2026-07-05 |  |
| 0 | triage | `triage-issue-1331` | Issue #1331 (no labels, OPEN): Backend `test` job red/hanging on dev base — blocks the entire… | dropped | 2026-07-05 |  |
| 0 | dx | `dx-stalled-review-pr-988` | Stalled review: PR #988 (Epic: reusable Playwright E2E framework + sitemap FlowRunner) open 1… | dropped | 2026-07-05 |  |
| 0 | refactor | `churn-hotspot-backend-servers-api-server-src-routes-forms-rs` | Churn hotspot: backend/servers/api-server/src/routes/forms.rs touched 2x since 2026-06-12 (wi… | dropped | 2026-07-05 |  |
| 0 | refactor | `churn-hotspot-backend-servers-api-server-tests-reserve-funds-cross-org-idor-tests-rs` | Churn hotspot: backend/servers/api-server/tests/reserve_funds_cross_org_idor_tests.rs touched… | dropped | 2026-07-05 |  |
| 0 | refactor | `churn-hotspot-backend-crates-db-tests-form-rls-repo-tests-rs` | Churn hotspot: backend/crates/db/tests/form_rls_repo_tests.rs touched 2x since 2026-06-12 (wi… | dropped | 2026-07-05 |  |
| 0 | triage | `triage-issue-1380` | Issue #1380 (no labels, OPEN): Dispatcher stale gap-scan buffer + Tier-2 escalation endpoint … | dropped | 2026-07-05 |  |
| 0 | refactor | `churn-hotspot-frontend-apps-mobile-app-config-icon-test-ts` | Churn hotspot: 124 lines in frontend/apps/mobile/app.config.icon.test.ts (PR #1383 gap-85-2) | dropped | 2026-07-05 |  |
| 0 | refactor | `churn-hotspot-frontend-apps-mobile-app-config-ts` | Churn hotspot: 94 lines in frontend/apps/mobile/app.config.ts (PR #1383 gap-85-2) | dropped | 2026-07-05 |  |
| 0 | refactor | `churn-hotspot-backend-crates-db-src-repositories-form-rs` | Churn hotspot: 53 lines in backend/crates/db/src/repositories/form.rs (PR #1379 #1332 unblock) | dropped | 2026-07-05 |  |
| 0 | refactor | `churn-hotspot-backend-servers-api-server-tests-booking-oauth-csrf-tests-rs` | booking_oauth_csrf_tests.rs hotspot — 484-line NEW test file (PR #1393 #1424 OAuth CSRF cover… | dropped | 2026-07-05 |  |
| 0 | refactor | `churn-hotspot-backend-servers-api-server-tests-booking-oauth-routes-tests-rs` | booking_oauth_routes_tests.rs hotspot — 381-line NEW test file (PR #1393 OAuth routes coverage) | dropped | 2026-07-05 |  |
| 0 | refactor | `repeated-churn-backend-servers-api-server-src-routes-forms-rs` | forms.rs repeated-churn — runs_seen=2 (#1337 explicit_auto_deref + #1397 org-scope hardening) | dropped | 2026-07-05 |  |
| 0 | dx | `closed-not-merged-pr-1425` | PR #1425 (GH #1377 document presigned-URL tests) closed unmerged — superseded by merged #1394 | dropped | 2026-07-05 |  |
| 0 | dx | `dx-stalled-review-pr-1179` | PR #1179 (docs(epics) catalog backfill for 37 mounted-but-undocumented backend modules) — sta… | dropped | 2026-07-05 |  |
| 0 | refactor | `refactor-oauth-integration-tests-hot` | Stabilize oauth_integration_tests churn — heavy edits across 3 OAuth fix PRs | dropped | 2026-06-16 |  |
| 0 | triage | `triage-issue-779` | Issue #779 (no labels, OPEN): Current dev review: consolidated priority rollup (origin/dev sn… | dropped | 2026-06-13 |  |
| 0 | bug | `bug-announcer-stale-message` | Announcer: untracked clear-then-set timeouts can resurrect a stale screen-reader message | dropped | 2026-06-07 |  |
| 0 | dx | `dx-portfolio-dashboard-stubs` | Portfolio dashboard: alert mark-read/resolve mutations + property-card click navigation are n… | dropped | 2026-06-04 |  |
