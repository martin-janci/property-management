# Backlog of vectors

<sub>Last regenerated: 2026-07-06 18:18 UTC by routine</sub>

| Score | Title | Vector | Status | Confidence | Sources | Updated | Plan |
|-------|-------|--------|--------|------------|---------|---------|------|
| 6 | Add regression tests for inquiry mark_as_read cross-tenant IDOR fix (PR #497) | test-gap | done | high | PR #497, PR #507 | 2026-05-26 | [test-gap-inquiry-idor-regression.md](plans/_archive/test-gap-inquiry-idor-regression.md) |
| 3 | ReportFaultScreen.tsx handleSubmit() fakes API call with setTimeout(1500) — fault reports never reac | bug | ready | high | Phase 1.5 review of mobile-rn segment (2026-06-16) | 2026-06-16 | [code-review-mobile-rn-report-fault-fake-submit.md](plans/code-review-mobile-rn-report-fault-fake-submit.md) |
| 3 | Reality-web RealtorManagement.tsx hardcoded English strings — agency flow not localized to sk/cs/de | bug | ready | high | rotating-expert-review reality-web 2026-06-14 | 2026-06-15 | [code-review-reality-web-realtor-mgmt-untranslated.md](plans/code-review-reality-web-realtor-mgmt-untranslated.md) |
| 3 | Reality-web ComparisonUrlHandler hits non-existent /api/listings/${id} — every shared comparison URL | bug | ready | high | rotating-expert-review reality-web 2026-06-14 | 2026-06-14 | [code-review-reality-web-share-comparison-404.md](plans/code-review-reality-web-share-comparison-404.md) |
| 3 | Reality-web listing detail SSR crashes on partial 200 body — JSON-LD build deref of undefined fields | bug | ready | high | rotating-expert-review reality-web 2026-06-14 | 2026-06-14 | [code-review-reality-web-listing-page-ssr-crash.md](plans/code-review-reality-web-listing-page-ssr-crash.md) |
| 3 | iOS SearchView.swift does not compile — performSearch/scheduleSearch undefined, resultsGrid corrupte | bug | ready | high | issue #1266, PR #1257 (verify) | 2026-06-11 | [bug-ios-searchview-uncompilable.md](plans/bug-ios-searchview-uncompilable.md) |
| 3 | PR #1203 (fix(aml_dsa): close cross-tenant IDOR in moderation + AML-review handlers (PAP-36)) merged | security | dropped | medium | PR #1203 | 2026-06-10 |  |
| 3 | PR #1193 (fix(aml-dsa): lock DSA reports to platform roles + fix file-path disclosure (PAP-47)) merg | security | dropped | medium | PR #1193 | 2026-06-10 |  |
| 3 | Schema drift: runtime SQL errors from non-existent columns in voting/messaging/notification paths | bug | done | high | Issue #1008, PR #1009 | 2026-06-07 |  |
| 3 | IDOR: ai.rs LLM-doc handlers publish/list/get any tenant's listing descriptions & photo enhancements | security | ready | high | code-review api-core 2026-05-29, ai.rs:2620 | 2026-06-01 | [security-llm-doc-idor.md](plans/security-llm-doc-idor.md) |
| 3 | IDOR: reality-server realtors mark_inquiry_read flips any realtor's inquiry by ID with no owner scop | security | done | high | issue #519, PR #508 | 2026-05-26 | [security-realtors-mark-inquiry-read-idor.md](plans/_archive/security-realtors-mark-inquiry-read-idor.md) |
| 3 | IDOR: equipment delete/update + maintenance update mutate any tenant's equipment by ID with no org s | security | done | high | code-review api-core 2026-05-25, ai.rs:1133 | 2026-05-25 | [security-equipment-idor.md](plans/_archive/security-equipment-idor.md) |
| 3 | SSRF: signed-document fetch + webhook-test POST issue outbound requests to unvalidated user-controll | security | done | high | issue #439, signatures.rs:628 | 2026-05-25 | [security-ssrf-outbound-url-validation.md](plans/_archive/security-ssrf-outbound-url-validation.md) |
| 3 | IDOR: unlink_voice_device deactivates any device by ID with no owner/org scoping | security | done | high | code-review api-core 2026-05-23, ai.rs:3002 | 2026-05-25 | [security-voice-device-idor.md](plans/_archive/security-voice-device-idor.md) |
| 2 | frontend/apps/ppt-web/src/features/facilities/pages/BookFacilityPage.tsx:47-54 — | bug | open | medium | rotating-expert-review | 2026-07-06 |  |
| 2 | Reality-server listings pagination clamp (PR #959) shipped without a regression test for limit=-1 | test-gap | done | high | PR #959, issue #953 | 2026-07-05 |  |
| 2 | PR #1418 touched routes/** (faults.route.test.tsx) without updating docs/screens/ppt/* — heuristic,  | test-gap | done | medium | PR #1418, PR #2070 | 2026-07-05 |  |
| 2 | vote.rs:1765 calculate_question_result() uses partial_cmp().unwrap() on f64 — NaN/Inf weights panic  | bug | done | high | code-review api-core 2026-06-15, PR #1417 | 2026-06-16 |  |
| 2 | PR #1196 (feat(ppt-web): add missing test coverage for faults feature) merged with 2 unchecked TODO  | test-gap | dropped | medium | PR #1196 | 2026-06-10 |  |
| 2 | PushFanoutWorker BLPOP queue-drain deferred — Redis path is a logging no-op | dx | done | high | PR #515, push_fanout.rs:621 | 2026-06-06 |  |
| 2 | ai.rs (3,134 LOC) — explicit module-split into routes/ai/{sessions,equipment,workflows,voice,llm,mod | refactor | done | high | pm-tech-lead analysis 2026-05-25, security-voice-device-idor | 2026-06-06 |  |
| 2 | announcements.rs churn-hot — 2,722 lines this run (Epic 2B + Epic 6 work) | refactor | done | medium | git log origin/dev since 2026-05-24, PR #504 | 2026-06-06 |  |
| 2 | announcements.rs (2,722 LOC) — explicit module-split into routes/announcements/{crud,targeting,deliv | refactor | done | high | pm-tech-lead analysis 2026-05-25, PR #1110 | 2026-06-06 |  |
| 2 | Reduce App.tsx route-aggregator coupling (top churn hotspot, merge-conflict risk) | refactor | done | medium | PR #474, PR #475 | 2026-06-06 |  |
| 2 | platform_admin.rs (2,762 LOC) — explicit module-split into routes/platform_admin/{tenants,features,b | refactor | done | high | pm-tech-lead analysis 2026-05-25, PR #1109 | 2026-06-06 |  |
| 2 | Screen-map drift: PR #1033 wired error/retry into AnnouncementsPage+FaultsPage via App.tsx without a | test-gap | done | medium | PR #1033, PR #1111 | 2026-06-06 |  |
| 2 | Risky churn: mobile App.tsx deep-link/doc-detail wiring changing across back-to-back PRs without cov | bug | done | medium | PR #1103, PR #962 | 2026-06-05 |  |
| 2 | Integration marketplace install/OAuth flows are placeholders — wire backend handlers + UI navigation | dx | done |  | PR #1105, PR #282 | 2026-06-05 |  |
| 2 | Booking push availability/rates endpoints add batch-cap + non-negative guards with no regression tes | test-gap | done | high | PR #1068, PR #607 | 2026-06-05 |  |
| 2 | Portal webhook fail-closed fix (PR #874) shipped without a regression test for unverified-signature  | test-gap | done | high | PR #1052, PR #874 | 2026-06-05 |  |
| 2 | Mobile dev-review batch (PR #918, 5 files under frontend/apps/mobile/src) shipped without a regressi | test-gap | done | high | PR #1072, PR #918 | 2026-06-05 |  |
| 2 | Reality-server SSO consumer review fix (PR #921, closes #820) shipped without a regression test | test-gap | done | high | PR #1076, PR #921 | 2026-06-05 |  |
| 2 | CI branch-protection + auto-rebase workflow change (PR #923) shipped without an integration test | test-gap | done | high | PR #1057, PR #923 | 2026-06-05 |  |
| 2 | deploy-server OIDC scope mapping (#939) shipped without unit test for derive_oidc_scopes | test-gap | done | high | PR #1106, PR #939 | 2026-06-05 |  |
| 2 | Mobile RN dev-review tail (#943) shipped without test coverage | test-gap | done | high | PR #1080, PR #943 | 2026-06-05 |  |
| 2 | Frontend gap-sweep (PR #990, 34 files across Epics 1/6/7B/9/10B/11/15/17/18) shipped without a regre | test-gap | done | high | PR #1081, PR #990 | 2026-06-05 |  |
| 2 | Mobile document-detail wiring (PR #992) shipped without a regression test for the deep-link payload  | test-gap | done | high | PR #1082, PR #992 | 2026-06-05 |  |
| 2 | Screen-map drift: PR #839 modified ppt-web App.tsx (FileDisputePageRoute) without a docs/screens/ppt | test-gap | done | medium | PR #1056, PR #839 | 2026-06-05 |  |
| 2 | ppt-web status/auth components hardcode English in an otherwise i18n'd app | refactor | done | medium | code-review ppt-web-ui 2026-05-24, rotating-expert-review | 2026-06-04 |  |
| 2 | MediationWorkspacePage shows empty/unknown state instead of error UI on dispute fetch failure | bug | done | high | PR #555, code-review 2026-05-27 | 2026-06-03 |  |
| 2 | Mobile VotingScreen double-casts API result across boundary — render-time crash on unexpected shape | bug | done | medium | code-review mobile-rn 2026-05-27, rotating-expert-review | 2026-06-03 |  |
| 2 | Reality-web InviteRealtorModal swallows invite-mutation failure with no error UI | bug | done | high | code-review reality-web 2026-05-28, rotating-expert-review | 2026-06-03 |  |
| 2 | Airbnb webhook at-least-once delivery enqueues duplicate SYNC_EXTERNAL jobs | bug | done | high | PR #538, webhook.rs:1028 | 2026-06-03 |  |
| 2 | DocumentsBrowse MoveFolderDialog cannot pre-select current folder (DocumentSummary lacks folder_id) | dx | done | high | PR #623, PR #1031 | 2026-06-03 |  |
| 2 | API + SPA security-headers middleware (PR #963) shipped without an assertion test for HSTS/nosniff/C | test-gap | done | high | PR #963, issue #954 | 2026-06-03 |  |
| 2 | api-server main.rs vs lib.rs::create_router diverge silently (5 routes unreachable in prod, no test  | test-gap | done | high | PR #866, issue #867 | 2026-06-01 |  |
| 2 | ReportSchedule.update_schedule stores cron in `time` workaround; documented UPDATE never runs (missi | bug | done | high | PR #611, issue #616 | 2026-05-30 |  |
| 2 | Screen-map drift: report execution-history route (PR #547) added without a ppt screen doc | test-gap | done | medium | PR #547, frontend/apps/ppt-web/src/routes/lazyRoutes.tsx | 2026-05-27 |  |
| 2 | Dispute state machine (PR #506) shipped with no tests + no org predicate on update_status | test-gap | done | high | PR #506, issue #520 | 2026-05-26 |  |
| 2 | documents.rs churn-hot — 10,659 lines over 14d | refactor | done | medium | git log origin/main since 2026-05-06, git log origin/dev sin | 2026-05-25 |  |
| 2 | integrations.rs churn-hot — 12,977 lines over 14d, candidate for module split | refactor | done | medium | git log origin/main since 2026-05-06, git log origin/dev sin | 2026-05-25 |  |
| 2 | organizations.rs churn-hot — 12,060 lines over 14d (multitenancy + admin) | refactor | done | medium | git log origin/main since 2026-05-06, git log origin/dev sin | 2026-05-25 |  |
| 2 | IDOR: reality-server mark_as_read flips any realtor's inquiry by ID with no owner scoping | security | done | high | code-review reality-server 2026-05-23, inquiries.rs:554 | 2026-05-25 | [security-inquiry-read-idor.md](plans/_archive/security-inquiry-read-idor.md) |
| 2 | Latent fail-open: ProtectedRoute role check is skipped when user.role is falsy | security | done | medium | code-review ppt-web-ui 2026-05-24, ProtectedRoute.tsx:117 | 2026-05-25 |  |
| 2 | Screen-map drift: PR #464 wired a neighbors route in ppt-web without a docs/screens/ppt entry | test-gap | done | medium | PR #464 | 2026-05-25 |  |
| 2 | Screen-map drift: PR #460 touched reality-web listing page without a docs/screens/reality update | test-gap | closed | medium | PR #460 | 2026-05-25 |  |
| 2 | Dead/duplicate handler modules: AuthHandler & BuildingHandler unused, routes reimplement inline | refactor | done | medium | code-review api-handlers 2026-05-23, PR #437 | 2026-05-24 |  |
| 2 | Complete RLS migration in 31 remaining handlers (voting, market_pricing, faults, notif_prefs, report | security | done |  | issue #160, PR #420 | 2026-05-23 |  |
| 1 | Mobile RN production screens (Buildings/Meters/Leases/PersonMonths/Notifications/Threads/Forms) rend | bug | done | high | Phase 1.5 review of mobile-rn segment (2026-06-16), PR #2118 | 2026-07-06 |  |
| 1 | Churn hotspot: backend/crates/db/src/models/multi_currency.rs — refactor candidate | refactor | open | high | 14-day churn top-3 (2 PR touches in window ending 2026-07-06 | 2026-07-06 |  |
| 1 | Churn hotspot: backend/crates/db/src/models/regional_compliance.rs — refactor candidate | refactor | open | high | 14-day churn top-3 (2 PR touches in window ending 2026-07-06 | 2026-07-06 |  |
| 1 | Churn hotspot: backend/servers/api-server/src/routes/regional_compliance.rs — refactor candidate | refactor | open | high | 14-day churn top-3 (2 PR touches in window ending 2026-07-06 | 2026-07-06 |  |
| 1 | frontend/apps/ppt-web/src/features/facilities/pages/BookFacilityPage.tsx:33,69,9 | refactor | open | medium | rotating-expert-review | 2026-07-06 |  |
| 1 | frontend/apps/ppt-web/src/features/settings/pages/SessionsPage.tsx:84 — `(sessio | refactor | open | medium | rotating-expert-review | 2026-07-06 |  |
| 1 | DeepLinkRouter skips URL-decoding while Android Uri.getQueryParameter decodes — SSO tokens diverge p | bug | open | high | mobile-native-kmp segment review 2026-06-06 | 2026-07-05 |  |
| 1 | SearchScreen stale-response race — overlapping searches can clobber newer results | bug | open | high | mobile-native-kmp segment review 2026-06-06, PR #1125 | 2026-07-05 |  |
| 1 | Screen-map drift: PR #1085 modified reality-web listing detail metadata + page without screen-doc up | test-gap | open | medium | PR #1085 | 2026-07-05 |  |
| 1 | Screen-map drift: PR #1100 modified ppt-web App.tsx (FileDisputePageRoute extraction) without screen | test-gap | open | medium | PR #1100 | 2026-07-05 |  |
| 1 | Risky churn: api-server main.rs security-headers wiring shipped without a middleware smoke test | bug | open | medium | PR #963 | 2026-07-05 |  |
| 1 | Screen-map drift: PR #922 modified ppt-web App.tsx (dev-review rounds 1-5 fixes) without a docs/scre | test-gap | done | medium | PR #922, PR #2075 | 2026-07-05 |  |
| 1 | Churn hotspot: ListingDetailScreen.kt — +1279 LOC this run (gap-82-4 reality mobile favorite toggle) | refactor | done | medium | PR #1121, PR #2059 | 2026-07-05 |  |
| 1 | Churn hotspot: DocumentsScreen.tsx — 3 PRs this run | refactor | done | high | PR #1101, PR #1081 | 2026-07-05 |  |
| 1 | iOS deep-link layer dead at runtime — Info.plist missing CFBundleURLTypes + applinks entitlement | bug | open | high | issue #1267, PR #1256 (verify) | 2026-07-05 |  |
| 1 | Webhook handlers RLS migration (PR #1288, PAP-170) shipped without a new regression test for repo-la | test-gap | open | medium | PR #1288, PAP-170 | 2026-07-05 |  |
| 1 | AI llm/sessions + integrations sync + subscriptions RLS migration (PR #1287, PAP-169) shipped withou | test-gap | open | medium | PR #1287, PAP-169 | 2026-07-05 |  |
| 1 | api_ecosystem.rs RLS migration (PR #1289, PAP-167) — 162-line handler rework shipped without a regre | test-gap | open | medium | PR #1289, PAP-167 | 2026-07-05 |  |
| 1 | mfa.rs RLS migration (PR #1292, PAP-168) shipped without a regression test; also landed broken and w | test-gap | open | medium | PR #1292, PR #1287 | 2026-07-05 |  |
| 1 | crypto.rs:127 SysRng.try_fill_bytes(...).expect() panics if OS CSPRNG errors during integration-cred | refactor | done | medium | code-review api-core 2026-06-15, PR #2074 | 2026-07-05 |  |
| 1 | useDeepLinkRouting.ts:27-36 — initialize() re-runs on onNavigate identity change + void promise with | bug | open | medium | Phase 1.5 review of mobile-rn segment (2026-06-16) | 2026-07-05 |  |
| 1 | Churn hotspot: backend/crates/db/src/models/mod.rs (12 commits in 19-day catch-up) | refactor | open | high | churn since 4829015b: 12 commits | 2026-07-05 |  |
| 1 | Churn hotspot: backend/crates/db/src/repositories/rental.rs (11 commits in 19-day catch-up) | refactor | open | high | churn since 4829015b: 11 commits | 2026-07-05 |  |
| 1 | PR #1378 closed without merge — DROP-OWNED-BY teardown theory for #1332 was wrong root cause, supers | refactor | done | high | PR #1378, PR #1379 | 2026-06-15 |  |
| 1 | PKCE unit test became a tautology after services/oauth.rs DRY refactor (#1132) | test-gap | done | high | #1137, PR #1132 | 2026-06-07 |  |
| 1 | Triage: dispatcher incident — assignments-archive.json corrupted to 1/196 rows on dev branch (#1061) | triage | done | high | Issue #1061, #1061 closed | 2026-06-07 |  |
| 1 | Issue #950 (no labels, OPEN): CI: trigger-deploy 403 marks all dev image builds red and blocks stagi | triage | done | high | #950, PR #1143 | 2026-06-07 |  |
| 1 | Issue #952 (no labels, OPEN): [staging] Reality SSO login dead-ends: redirect_uri callback 404s on r | triage | done | high | #952, PR #1144 | 2026-06-07 |  |
| 1 | Issue #769 (no labels, OPEN): Current dev review: Deploy server | triage | done | high | #769, PR #1141 | 2026-06-07 |  |
| 1 | Issue #789 (no labels, OPEN): Dev review rounds 6-10: scheduler, notifications, admin, orgs, buildin | triage | done | high | #789, PR #1142 | 2026-06-07 |  |
| 1 | docker/nginx admin-web + ppt-web templates churned twice this run (security headers + redirects) | dx | done | high | PR #963, PR #964 | 2026-06-06 |  |
| 1 | ai.rs churn-hot — 3,142 lines this run; 3,142-line route monolith, candidate for module split | refactor | done | medium | git log origin/dev since 2026-05-24, PR #1114 | 2026-06-06 |  |
| 1 | ppt-web e2e auth-refresh.spec.ts added (+252 lines, story 79-2 token-refresh coverage) | refactor | done | high | PR #1047, PR #1113 | 2026-06-06 |  |
| 1 | api-server esignature_webhook_idempotency_tests.rs added (+228 lines, terminal-state regression) | refactor | done | high | PR #1034, PR #1119 | 2026-06-06 |  |
| 1 | ppt-web EvidenceUploader.test.tsx added (+202 lines, dispute-filing AC-2 regression) | refactor | done | high | PR #1048, PR #1116 | 2026-06-06 |  |
| 1 | api-server main.rs touched twice this run (gap-sweep + security headers) — minor churn marker | refactor | done | high | PR #989, PR #963 | 2026-06-06 |  |
| 1 | Duplicated animate-spin spinner markup across mediation page + chat thread (no shared Spinner) | refactor | done | medium | PR #555, code-review 2026-05-27 | 2026-06-06 |  |
| 1 | Mediation reference number uppercases full UUID (DSP-<uuid>) instead of a short code | refactor | done | medium | PR #555, code-review 2026-05-27 | 2026-06-06 |  |
| 1 | frontend/apps/mobile/src/App.tsx churned twice this run (universal links + doc-detail wiring) | refactor | done | high | PR #962, PR #992 | 2026-06-06 |  |
| 1 | platform_admin.rs churn-hot — 2,762 lines this run (admin/OAuth-provider feature work) | refactor | done | medium | git log origin/dev since 2026-05-24, PR #1109 | 2026-06-06 |  |
| 1 | Reality-web ComparisonUrlHandler hardcodes English loading/error strings | refactor | done | medium | code-review reality-web 2026-05-28, rotating-expert-review | 2026-06-06 |  |
| 1 | Watch routes/oauth.rs churn after audit-log + hardening PRs | refactor | done | high | PR #930, PR #933 | 2026-06-06 |  |
| 1 | Watch services/oauth.rs churn after introspect/revoke hardening (#933) | refactor | done | high | PR #933, PR #1132 | 2026-06-06 |  |
| 1 | Mobile VotingScreen pure transforms toUiStatus/toUiVote have no tests | test-gap | done | medium | code-review mobile-rn 2026-05-27, rotating-expert-review | 2026-06-06 |  |
| 1 | Issue #749 (no labels, OPEN): Code review findings: Story 6.1 announcement creation and targeting | triage | done | high | #749, issue #749 closed | 2026-06-06 |  |
| 1 | Issue #755 (no labels, OPEN): Current dev review: Epic 8A Notification Preferences | triage | done | high | #755, issue #755 closed | 2026-06-06 |  |
| 1 | Issue #764 (no labels, OPEN): Current dev review: Admin MFA & Auth Hardening | triage | done | high | #764, issue #764 closed | 2026-06-06 |  |
| 1 | Issue #765 (no labels, OPEN): Current dev review: Integrations & Airbnb OAuth | triage | done | high | #765, issue #765 closed | 2026-06-06 |  |
| 1 | Mobile VotingScreen hardcodes en-US in toLocaleDateString — vote dates never localize | bug | done | medium | PR #1083, code-review mobile-rn 2026-05-27 | 2026-06-05 |  |
| 1 | Reality-web listing generateMetadata can throw during SSR on malformed 200 body | bug | done | medium | PR #1085, code-review reality-web 2026-05-28 | 2026-06-05 |  |
| 1 | PR #908 (fix(security): require PKCE on OAuth authorization-code flow, closes #823) was closed unmer | security | done | medium | PR #908, PR #1025 | 2026-06-03 |  |
| 1 | Issue #751 (no labels, OPEN): Current dev review: frontend/web/API-client findings | triage | done | high | #751, #942 | 2026-06-02 |  |
| 1 | Issue #752 (no labels, OPEN): Current dev review: mobile CI tooling findings | triage | done | high | #752, #929 | 2026-06-02 |  |
| 1 | Issue #756 (no labels, OPEN): Current dev review: Epic 10A OAuth Provider | triage | done | high | #756, #934 | 2026-06-02 |  |
| 1 | Issue #761 (no labels, OPEN): Current dev review: Epic 84 E-Signature & Leases | triage | done | high | #761, #936 | 2026-06-02 |  |
| 1 | Issue #763 (no labels, OPEN): Current dev review: Reality Server & Inquiries | triage | done | high | #763, #935 | 2026-06-02 |  |
| 1 | Issue #767 (no labels, OPEN): Current dev review: Mobile RN Property Management app | triage | done | high | #767, #943 | 2026-06-02 |  |
| 1 | Issue #768 (no labels, OPEN): Current dev review: Admin-web features (10B) | triage | done | high | #768, #930 | 2026-06-02 |  |
| 1 | Issue #920 (no labels, OPEN): Announcement targeting not enforced on read (intra-org disclosure) | triage | done | high | #920, #944 | 2026-06-02 |  |
| 1 | Issue #750 (no labels, OPEN): Current dev review: backend/API/database findings | triage | done | high | #750, PR #922 | 2026-06-01 |  |
| 1 | Issue #753 (no labels, OPEN): Current dev review: Epic 6 Announcements & Communication | triage | done | high | #753 | 2026-06-01 |  |
| 1 | Issue #754 (no labels, OPEN): Current dev review: Epic 7A Basic Document Management | triage | done | high | #754, PR #914 | 2026-06-01 |  |
| 1 | Issue #757 (no labels, OPEN): Current dev review: Epic 10B Platform Administration | triage | done | high | #757 | 2026-06-01 |  |
| 1 | Issue #760 (no labels, OPEN): Current dev review: Epic 79 Disputes & Mediation | triage | done | high | #760, PR #915 | 2026-06-01 |  |
| 1 | Issue #762 (no labels, OPEN): Current dev review: Reports & Schedules | triage | done | high | #762 | 2026-06-01 |  |
| 1 | Issue #766 (no labels, OPEN): Current dev review: AI & LLM routes | triage | done | high | #766, PR #879 | 2026-06-01 |  |
| 1 | Issue #770 (no labels, OPEN): Current dev review: Faults & triage | triage | done | high | #770, PR #902 | 2026-06-01 |  |
| 1 | Issue #771 (no labels, OPEN): Current dev review: Research dispatcher & CI automation | triage | done | high | #771, PR #923 | 2026-06-01 |  |
| 1 | Issue #772 (no labels, OPEN): Current dev review: Auth core (delta confirmation) | triage | done | high | #772 | 2026-06-01 |  |
| 1 | Issue #773 (no labels, OPEN): Current dev review: Leases & rental | triage | done | high | #773 | 2026-06-01 |  |
| 1 | Issue #774 (no labels, OPEN): Current dev review: Reality server (broad) | triage | done | high | #774, PR #919 | 2026-06-01 |  |
| 1 | Issue #775 (no labels, OPEN): Current dev review: WebSocket realtime | triage | done | high | #775, PR #926 | 2026-06-01 |  |
| 1 | Issue #776 (no labels, OPEN): Current dev review: Equipment & audit log | triage | done | high | #776 | 2026-06-01 |  |
| 1 | Issue #777 (no labels, OPEN): Current dev review: Compliance & GDPR | triage | done | high | #777 | 2026-06-01 |  |
| 1 | Issue #778 (no labels, OPEN): Current dev review: Marketplace, voting, investor portal, impersonatio | triage | done | high | #778, PR #882 | 2026-06-01 |  |
| 1 | Issue #788 (no labels, OPEN): Dev review rounds 1-5: mobile-native + ppt-web surfaces | triage | done | high | #788, PR #922 | 2026-06-01 |  |
| 1 | Issue #790 (no labels, OPEN): Dev review rounds 11-15: vendor, predictive, reality-web, middleware | triage | done | high | #790, PR #913 | 2026-06-01 |  |
| 1 | Issue #791 (no labels, OPEN): Dev review rounds 16-20: push, e-sign, portal, webhooks, reserves | triage | done | high | #791, PR #924 | 2026-06-01 |  |
| 1 | Issue #846 (no labels, OPEN): Code review: Epics 12+65 — Meters & Energy/ESG (origin/dev) | triage | done | high | #846, PR #880 | 2026-06-01 |  |
| 1 | Issue #847 (no labels, OPEN): Code review: Reality-server — Inquiries IDOR (Epics 16–19) (origin/dev | triage | done | high | #847 | 2026-06-01 |  |
| 1 | Issue #848 (no labels, OPEN): Code review: Epics 78+134 — Vendor portal stubs & Predictive maintenan | triage | done | high | #848, PR #913 | 2026-06-01 |  |
| 1 | Issue #850 (no labels, OPEN): Code review: Epics 61+146+42 — Multi-currency, Data residency, Violati | triage | done | high | #850, PR #883 | 2026-06-01 |  |
| 1 | Issue #851 (no labels, OPEN): Code review: Epics 15+105+69 — Listings/syndication & Developer API st | triage | done | high | #851, PR #904 | 2026-06-01 |  |
| 1 | Issue #859 (no labels, OPEN): sqlx 0.9 breaks runtime decode of Postgres enum columns into Rust Stri | triage | done | high | #859, PR #871 | 2026-06-01 |  |
| 1 | Issue #867 (no labels, OPEN): Tech debt: api-server main.rs duplicates lib.rs::create_router — route | triage | done | high | #867, PR #870 | 2026-06-01 |  |
| 1 | Issue #836 (no labels, OPEN): Code review: Epic 2B-C — Mobile push & device registration (origin/dev | triage | done | high | #836, PR #866 | 2026-05-31 |  |
| 1 | Issue #845 (no labels, OPEN): Code review: Epic 14 — IoT alerts, correlations, thresholds (origin/de | triage | done | high | #845, PR #862 | 2026-05-31 |  |
| 1 | Issue #849 (no labels, OPEN): Code review: Epic 10B+143 — Admin impersonation, Help, Board meetings  | triage | done | high | #849, PR #869 | 2026-05-31 |  |
| 0 | Churn hotspot: 1021 lines changed in backend/servers/api-server/src/routes/emergency.rs (window 2026 | refactor | dropped | high | local git numstat since 2026-06-07 | 2026-07-05 |  |
| 0 | Churn hotspot: 929 lines changed in backend/servers/api-server/src/routes/vendors.rs (window 2026-06 | refactor | dropped | high | local git numstat since 2026-06-07 | 2026-07-05 |  |
| 0 | Churn hotspot: 709 lines changed in backend/servers/api-server/src/routes/enhanced_tenant_screening. | refactor | dropped | high | local git numstat since 2026-06-07 | 2026-07-05 |  |
| 0 | Churn hotspot: 2940 lines changed in backend/crates/db/src/repositories/document.rs (window 2026-06- | refactor | dropped | high | local git numstat since 2026-06-10T03:05:00Z | 2026-07-05 |  |
| 0 | Churn hotspot: 2856 lines changed in backend/crates/db/src/repositories/subscription.rs (window 2026 | refactor | dropped | high | local git numstat since 2026-06-10T03:05:00Z, PR #1246 | 2026-07-05 |  |
| 0 | Churn hotspot: 2691 lines changed in backend/servers/api-server/src/routes/aml_dsa.rs (window 2026-0 | refactor | dropped | high | local git numstat since 2026-06-10T03:05:00Z, PR #1193 | 2026-07-05 |  |
| 0 | Issue #1151 (no labels, OPEN): Research dispatcher: claimable buffer is stale — true claimable work  | triage | dropped | high | #1151 | 2026-07-05 |  |
| 0 | Churn hotspot: SearchScreen.kt — +1293 LOC this run (gap-82-3 reality mobile search/filters) | refactor | dropped | medium | PR #1125 | 2026-07-05 |  |
| 0 | MainActivity reimplements deep-link dispatch instead of calling shared DeepLinkRouter — drift trap | refactor | dropped | high | mobile-native-kmp segment review 2026-06-06 | 2026-07-05 |  |
| 0 | Churn hotspot: AnnouncementsScreen.tsx — 4 PRs this run, instability proxy | refactor | dropped | high | PR #1101, PR #1077 | 2026-07-05 |  |
| 0 | Churn hotspot: AnnouncementsScreen.test.ts — 4 PRs this run, instability proxy | refactor | dropped | high | PR #1101, PR #1077 | 2026-07-05 |  |
| 0 | Dispatcher action-list.json corruption when MCP push falls back from blocked git push | triage | dropped | high | #1014 | 2026-07-05 |  |
| 0 | Issue #951 (no labels, OPEN): Deploy blocker: api-server requires ESIGN_TOKEN_SECRET + ESIGN_WEBHOOK | triage | dropped | high | #951 | 2026-07-05 |  |
| 0 | PR #1274 (cargo-minor-patch group, /backend, 9 updates) closed unmerged — superseded by #1313 after  | dx | dropped | high | PR #1274 | 2026-07-05 |  |
| 0 | Churn hotspot: backend/crates/integrations/src/booking.rs (+404/−29 in PR #1294 Booking.com OTA retr | refactor | dropped | high | PR #1294 commit 7ccce8a | 2026-07-05 |  |
| 0 | Churn hotspot: backend/servers/api-server/src/routes/api_ecosystem.rs (+106/−27 in PR #1293 PAP-171; | refactor | dropped | high | PR #1293 commit 1e50156 | 2026-07-05 |  |
| 0 | Churn hotspot: backend/crates/db/src/repositories/reality_portal.rs (+59/−28 in PR #1297 PAP-142 IDO | refactor | dropped | high | PR #1297 commit 8c711c6 | 2026-07-05 |  |
| 0 | Churn hotspot: backend/servers/api-server/src/routes/iot.rs (+278/-403 in PR #1321/#1322 PAP-151 re- | refactor | dropped | high | PR #1321 commit, PR #1322 commit | 2026-07-05 |  |
| 0 | Churn hotspot: backend/servers/api-server/src/routes/reserve_funds.rs (+228/-255 in PR #1321 PAP-151 | refactor | dropped | high | PR #1321 commit | 2026-07-05 |  |
| 0 | Churn hotspot: backend/crates/db/src/repositories/sensor.rs (+248/-86 in PR #1321/#1322 PAP-151 re-l | refactor | dropped | high | PR #1321 commit, PR #1322 commit | 2026-07-05 |  |
| 0 | Issue #1331 (no labels, OPEN): Backend `test` job red/hanging on dev base — blocks the entire backen | triage | dropped | high | #1331 | 2026-07-05 |  |
| 0 | Stalled review: PR #988 (Epic: reusable Playwright E2E framework + sitemap FlowRunner) open 10d, no  | dx | dropped | high | PR #988 | 2026-07-05 |  |
| 0 | Churn hotspot: backend/servers/api-server/src/routes/forms.rs touched 2x since 2026-06-12 (window 20 | refactor | dropped | high | local git numstat since 2026-06-12, local git numstat since  | 2026-07-05 |  |
| 0 | Churn hotspot: backend/servers/api-server/tests/reserve_funds_cross_org_idor_tests.rs touched 2x sin | refactor | dropped | high | local git numstat since 2026-06-12 | 2026-07-05 |  |
| 0 | Churn hotspot: backend/crates/db/tests/form_rls_repo_tests.rs touched 2x since 2026-06-12 (window 20 | refactor | dropped | high | local git numstat since 2026-06-12 | 2026-07-05 |  |
| 0 | Issue #1380 (no labels, OPEN): Dispatcher stale gap-scan buffer + Tier-2 escalation endpoint misconf | triage | dropped | high | issue #1380 | 2026-07-05 |  |
| 0 | Churn hotspot: 124 lines in frontend/apps/mobile/app.config.icon.test.ts (PR #1383 gap-85-2) | refactor | dropped | high | PR #1383 | 2026-07-05 |  |
| 0 | Churn hotspot: 94 lines in frontend/apps/mobile/app.config.ts (PR #1383 gap-85-2) | refactor | dropped | high | PR #1383 | 2026-07-05 |  |
| 0 | Churn hotspot: 53 lines in backend/crates/db/src/repositories/form.rs (PR #1379 #1332 unblock) | refactor | dropped | high | PR #1379, issue #1332 | 2026-07-05 |  |
| 0 | booking_oauth_csrf_tests.rs hotspot — 484-line NEW test file (PR #1393 #1424 OAuth CSRF coverage) | refactor | dropped | high | local git numstat since 2026-06-15 (commit 67c24bd..origin/d | 2026-07-05 |  |
| 0 | booking_oauth_routes_tests.rs hotspot — 381-line NEW test file (PR #1393 OAuth routes coverage) | refactor | dropped | high | local git numstat since 2026-06-15 | 2026-07-05 |  |
| 0 | forms.rs repeated-churn — runs_seen=2 (#1337 explicit_auto_deref + #1397 org-scope hardening) | refactor | dropped | high | hotspot_history.runs_seen 1→2 with new churn this run | 2026-07-05 |  |
| 0 | PR #1425 (GH #1377 document presigned-URL tests) closed unmerged — superseded by merged #1394 | dx | dropped | high | PR #1425 | 2026-07-05 |  |
| 0 | PR #1179 (docs(epics) catalog backfill for 37 mounted-but-undocumented backend modules) — stalled at | dx | dropped | high | PR #1179 | 2026-07-05 |  |
| 0 | Stabilize oauth_integration_tests churn — heavy edits across 3 OAuth fix PRs | refactor | dropped | high | PR #930, PR #933 | 2026-06-16 |  |
| 0 | Issue #779 (no labels, OPEN): Current dev review: consolidated priority rollup (origin/dev snapshot) | triage | dropped | high | #779 | 2026-06-13 |  |
| 0 | Announcer: untracked clear-then-set timeouts can resurrect a stale screen-reader message | bug | dropped | medium | code-review ppt-web-ui 2026-05-24, Announcer.tsx:49 | 2026-06-07 |  |
| 0 | Portfolio dashboard: alert mark-read/resolve mutations + property-card click navigation are no-op st | dx | dropped |  | PR #328, commit 254f01d | 2026-06-04 |  |
