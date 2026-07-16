# Backlog of vectors
<sub>Last regenerated: 2026-07-16 02:20 UTC by routine</sub>

| Score | Title | Vector | Confidence | Status | Sources | Updated | Plan |
|-------|-------|--------|------------|--------|---------|---------|------|
| 6 | Add regression tests for inquiry mark_as_read cross-tenant IDOR fix (PR #497) | test-gap | high | done | PR #497, PR #507, PR #548 | 2026-05-26 | plans/_archive/test-gap-inquiry-idor-regression.md |
| 3 | Fix cross-tenant IDOR at POST /api/v1/agencies/{id}/invitations (reality-server) | security | high | ready | rotating-expert-review 2026-07-16 | 2026-07-16 | plans/security-agency-invite-idor.md |
| 3 | revoke_all_sessions ignores refresh cookie — signs the caller out too | bug | high | done | Phase 1.5 code-review 2026-07-09 (api-handlers segment) | 2026-07-09 | plans/bug-revoke-all-sessions-cookie-blindness.md |
| 3 | ReportFaultScreen.tsx handleSubmit() fakes API call with setTimeout(1500) — fault reports never r... | bug | high | dropped | Phase 1.5 review of mobile-rn segment (2026-06-16) | 2026-06-16 | plans/code-review-mobile-rn-report-fault-fake-submit.md |
| 3 | Reality-web RealtorManagement.tsx hardcoded English strings — agency flow not localized to sk/cs/de | bug | high | done | rotating-expert-review reality-web 2026-06-14 | 2026-06-15 | plans/code-review-reality-web-realtor-mgmt-untranslated.md |
| 3 | Reality-web ComparisonUrlHandler hits non-existent /api/listings/${id} — every shared comparison ... | bug | high | dropped | rotating-expert-review reality-web 2026-06-14 | 2026-06-14 | plans/code-review-reality-web-share-comparison-404.md |
| 3 | Reality-web listing detail SSR crashes on partial 200 body — JSON-LD build deref of undefined fields | bug | high | dropped | rotating-expert-review reality-web 2026-06-14 | 2026-06-14 | plans/code-review-reality-web-listing-page-ssr-crash.md |
| 3 | iOS SearchView.swift does not compile — performSearch/scheduleSearch undefined, resultsGrid corru... | bug | high | dropped | issue #1266, PR #1257 (verify) | 2026-06-11 | plans/bug-ios-searchview-uncompilable.md |
| 3 | PR #1203 (fix(aml_dsa): close cross-tenant IDOR in moderation + AML-review handlers (PAP-36)) merged | security | medium | dropped | PR #1203 | 2026-06-10 |  |
| 3 | PR #1193 (fix(aml-dsa): lock DSA reports to platform roles + fix file-path disclosure (PAP-47)) merg | security | medium | dropped | PR #1193 | 2026-06-10 |  |
| 3 | Schema drift: runtime SQL errors from non-existent columns in voting/messaging/notification paths | bug | high | done | Issue #1008, PR #1009, PR #1040 | 2026-06-07 |  |
| 3 | IDOR: ai.rs LLM-doc handlers publish/list/get any tenant's listing descriptions & photo enhanceme... | security | high | dropped | code-review api-core 2026-05-29, ai.rs:2620, ai.rs:2599, ... | 2026-06-01 | plans/security-llm-doc-idor.md |
| 3 | IDOR: reality-server realtors mark_inquiry_read flips any realtor's inquiry by ID with no owner s... | security | high | done | issue #519, PR #508, realtors.rs:250, PR #548 | 2026-05-26 | plans/_archive/security-realtors-mark-inquiry-read-idor.md |
| 3 | IDOR: equipment delete/update + maintenance update mutate any tenant's equipment by ID with no or... | security | high | done | code-review api-core 2026-05-25, ai.rs:1133, equipment.rs... | 2026-05-25 | plans/_archive/security-equipment-idor.md |
| 3 | SSRF: signed-document fetch + webhook-test POST issue outbound requests to unvalidated user-contr... | security | high | done | issue #439, signatures.rs:628, integrations.rs:2743, PR #450 | 2026-05-25 | plans/_archive/security-ssrf-outbound-url-validation.md |
| 3 | IDOR: unlink_voice_device deactivates any device by ID with no owner/org scoping | security | high | done | code-review api-core 2026-05-23, ai.rs:3002, PR #461 | 2026-05-25 | plans/_archive/security-voice-device-idor.md |
| 2 | Add screen-map entry for reality-web change in PR #2351 | test-gap | medium | open | PR #2351 | 2026-07-16 |  |
| 2 | Add screen-map entry for reality-web change in PR #2355 | test-gap | medium | open | PR #2355 | 2026-07-16 |  |
| 2 | Churn hotspot cluster: api-server routes/auth.rs (runs_seen=3) + auth_tests.rs + reality-server r... | refactor | medium | done | PR #2205, PR #2250, PR #2261, PR #2270, PR #2271, PR #225... | 2026-07-12 |  |
| 2 | /forgot-password and /resend-verification have no rate limit — mailbomb / token-clobber | security | high | done | Phase 1.5 code-review 2026-07-09 (api-handlers segment) | 2026-07-09 | plans/security-forgot-password-no-rate-limit.md |
| 2 | Reality-server listings pagination clamp (PR #959) shipped without a regression test for limit=-1 | test-gap | high | done | PR #959, issue #953, PR #2073 | 2026-07-05 |  |
| 2 | PR #1418 touched routes/** (faults.route.test.tsx) without updating docs/screens/ppt/* — heuristi... | test-gap | medium | done | PR #1418, PR #2070 | 2026-07-05 |  |
| 2 | vote.rs:1765 calculate_question_result() uses partial_cmp().unwrap() on f64 — NaN/Inf weights pan... | bug | high | done | code-review api-core 2026-06-15, PR #1417 | 2026-06-16 |  |
| 2 | PR #1196 (feat(ppt-web): add missing test coverage for faults feature) merged with 2 unchecked TODO  | test-gap | medium | dropped | PR #1196 | 2026-06-10 |  |
| 2 | PushFanoutWorker BLPOP queue-drain deferred — Redis path is a logging no-op | dx | high | done | PR #515, push_fanout.rs:621, PR #1115 | 2026-06-06 |  |
| 2 | ai.rs (3,134 LOC) — explicit module-split into routes/ai/{sessions,equipment,workflows,voice,llm,... | refactor | high | done | pm-tech-lead analysis 2026-05-25, security-voice-device-i... | 2026-06-06 |  |
| 2 | announcements.rs churn-hot — 2,722 lines this run (Epic 2B + Epic 6 work) | refactor | medium | done | git log origin/dev since 2026-05-24, PR #504, PR #505, PR... | 2026-06-06 |  |
| 2 | announcements.rs (2,722 LOC) — explicit module-split into routes/announcements/{crud,targeting,de... | refactor | high | done | pm-tech-lead analysis 2026-05-25, PR #1110 | 2026-06-06 |  |
| 2 | Reduce App.tsx route-aggregator coupling (top churn hotspot, merge-conflict risk) | refactor | medium | done | PR #474, PR #475, PR #489, PR #511, PR #547, PR #549, PR ... | 2026-06-06 |  |
| 2 | platform_admin.rs (2,762 LOC) — explicit module-split into routes/platform_admin/{tenants,feature... | refactor | high | done | pm-tech-lead analysis 2026-05-25, PR #1109 | 2026-06-06 |  |
| 2 | Screen-map drift: PR #1033 wired error/retry into AnnouncementsPage+FaultsPage via App.tsx withou... | test-gap | medium | done | PR #1033, PR #1111 | 2026-06-06 |  |
| 2 | Risky churn: mobile App.tsx deep-link/doc-detail wiring changing across back-to-back PRs without ... | bug | medium | done | PR #1103, PR #962, PR #992 | 2026-06-05 |  |
| 2 | Integration marketplace install/OAuth flows are placeholders — wire backend handlers + UI navigation | dx |  | done | PR #1105, PR #282, PR #328, commit 254f01d, commit c97781a | 2026-06-05 |  |
| 2 | Booking push availability/rates endpoints add batch-cap + non-negative guards with no regression ... | test-gap | high | done | PR #1068, PR #607, issue #572 | 2026-06-05 |  |
| 2 | Portal webhook fail-closed fix (PR #874) shipped without a regression test for unverified-signatu... | test-gap | high | done | PR #1052, PR #874 | 2026-06-05 |  |
| 2 | Mobile dev-review batch (PR #918, 5 files under frontend/apps/mobile/src) shipped without a regre... | test-gap | high | done | PR #1072, PR #918 | 2026-06-05 |  |
| 2 | Reality-server SSO consumer review fix (PR #921, closes #820) shipped without a regression test | test-gap | high | done | PR #1076, PR #921 | 2026-06-05 |  |
| 2 | CI branch-protection + auto-rebase workflow change (PR #923) shipped without an integration test | test-gap | high | done | PR #1057, PR #923 | 2026-06-05 |  |
| 2 | deploy-server OIDC scope mapping (#939) shipped without unit test for derive_oidc_scopes | test-gap | high | done | PR #1106, PR #939 | 2026-06-05 |  |
| 2 | Mobile RN dev-review tail (#943) shipped without test coverage | test-gap | high | done | PR #1080, PR #943, issue #767 | 2026-06-05 |  |
| 2 | Frontend gap-sweep (PR #990, 34 files across Epics 1/6/7B/9/10B/11/15/17/18) shipped without a re... | test-gap | high | done | PR #1081, PR #990 | 2026-06-05 |  |
| 2 | Mobile document-detail wiring (PR #992) shipped without a regression test for the deep-link paylo... | test-gap | high | done | PR #1082, PR #992 | 2026-06-05 |  |
| 2 | Screen-map drift: PR #839 modified ppt-web App.tsx (FileDisputePageRoute) without a docs/screens/... | test-gap | medium | done | PR #1056, PR #839 | 2026-06-05 |  |
| 2 | ppt-web status/auth components hardcode English in an otherwise i18n'd app | refactor | medium | done | code-review ppt-web-ui 2026-05-24, rotating-expert-review... | 2026-06-04 |  |
| 2 | MediationWorkspacePage shows empty/unknown state instead of error UI on dispute fetch failure | bug | high | done | PR #555, code-review 2026-05-27, PR #1029 | 2026-06-03 |  |
| 2 | Mobile VotingScreen double-casts API result across boundary — render-time crash on unexpected shape | bug | medium | done | code-review mobile-rn 2026-05-27, rotating-expert-review,... | 2026-06-03 |  |
| 2 | Reality-web InviteRealtorModal swallows invite-mutation failure with no error UI | bug | high | done | code-review reality-web 2026-05-28, rotating-expert-revie... | 2026-06-03 |  |
| 2 | Airbnb webhook at-least-once delivery enqueues duplicate SYNC_EXTERNAL jobs | bug | high | done | PR #538, webhook.rs:1028, PR #841, PR #1030 | 2026-06-03 |  |
| 2 | DocumentsBrowse MoveFolderDialog cannot pre-select current folder (DocumentSummary lacks folder_id) | dx | high | done | PR #623, PR #1031 | 2026-06-03 |  |
| 2 | API + SPA security-headers middleware (PR #963) shipped without an assertion test for HSTS/nosnif... | test-gap | high | done | PR #963, issue #954, PR #1021 | 2026-06-03 |  |
| 2 | api-server main.rs vs lib.rs::create_router diverge silently (5 routes unreachable in prod, no te... | test-gap | high | done | PR #866, issue #867, issue #836, PR #870 | 2026-06-01 |  |
| 2 | ReportSchedule.update_schedule stores cron in `time` workaround; documented UPDATE never runs (mi... | bug | high | done | PR #611, issue #616, PR #643, PR #815 | 2026-05-30 |  |
| 2 | Screen-map drift: report execution-history route (PR #547) added without a ppt screen doc | test-gap | medium | done | PR #547, frontend/apps/ppt-web/src/routes/lazyRoutes.tsx,... | 2026-05-27 |  |
| 2 | Dispute state machine (PR #506) shipped with no tests + no org predicate on update_status | test-gap | high | done | PR #506, issue #520, PR #514, PR #548 | 2026-05-26 |  |
| 2 | documents.rs churn-hot — 10,659 lines over 14d | refactor | medium | done | git log origin/main since 2026-05-06, git log origin/dev ... | 2026-05-25 |  |
| 2 | integrations.rs churn-hot — 12,977 lines over 14d, candidate for module split | refactor | medium | done | git log origin/main since 2026-05-06, git log origin/dev ... | 2026-05-25 |  |
| 2 | organizations.rs churn-hot — 12,060 lines over 14d (multitenancy + admin) | refactor | medium | done | git log origin/main since 2026-05-06, git log origin/dev ... | 2026-05-25 |  |
| 2 | IDOR: reality-server mark_as_read flips any realtor's inquiry by ID with no owner scoping | security | high | done | code-review reality-server 2026-05-23, inquiries.rs:554 | 2026-05-25 | plans/_archive/security-inquiry-read-idor.md |
| 2 | Latent fail-open: ProtectedRoute role check is skipped when user.role is falsy | security | medium | done | code-review ppt-web-ui 2026-05-24, ProtectedRoute.tsx:117... | 2026-05-25 |  |
| 2 | Screen-map drift: PR #464 wired a neighbors route in ppt-web without a docs/screens/ppt entry | test-gap | medium | done | PR #464 | 2026-05-25 |  |
| 2 | Screen-map drift: PR #460 touched reality-web listing page without a docs/screens/reality update | test-gap | medium | closed | PR #460 | 2026-05-25 |  |
| 2 | Dead/duplicate handler modules: AuthHandler & BuildingHandler unused, routes reimplement inline | refactor | medium | done | code-review api-handlers 2026-05-23, PR #437 | 2026-05-24 |  |
| 2 | Complete RLS migration in 31 remaining handlers (voting, market_pricing, faults, notif_prefs, rep... | security |  | done | issue #160, PR #420, PR #421 | 2026-05-23 |  |
| 1 | Investigate churn hotspot: backend/servers/api-server/src/routes/portal_webhooks.rs | refactor | high | open | git log 2026-07-13..2026-07-16 | 2026-07-16 |  |
| 1 | Investigate churn hotspot: backend/servers/reality-server/src/main.rs | refactor | high | open | git log 2026-07-13..2026-07-16 | 2026-07-16 |  |
| 1 | backend integrations booking/mod.rs — instability watch after PR #2176 split | refactor | high | done | commits 2026-07-05..2026-07-09 | 2026-07-09 |  |
| 1 | oauth_integration_tests.rs repeated-churn (runs_seen 2→3) — OAuth handlers still moving | test-gap | high | dropped | commits 2026-07-05..2026-07-09 | 2026-07-09 |  |
| 1 | api-server routes/auth.rs — repeated hotspot + 3 static-review findings this run | refactor | high | done | commits 2026-07-05..2026-07-09 | 2026-07-09 |  |
| 1 | /refresh and /logout — empty refresh_token cookie shadows valid body token | bug | medium | done | Phase 1.5 code-review 2026-07-09 (api-handlers segment) | 2026-07-09 |  |
| 1 | DeepLinkRouter skips URL-decoding while Android Uri.getQueryParameter decodes — SSO tokens diverg... | bug | high | dropped | mobile-native-kmp segment review 2026-06-06 | 2026-07-05 |  |
| 1 | SearchScreen stale-response race — overlapping searches can clobber newer results | bug | high | dropped | mobile-native-kmp segment review 2026-06-06, PR #1125 | 2026-07-05 |  |
| 1 | Screen-map drift: PR #1085 modified reality-web listing detail metadata + page without screen-doc... | test-gap | medium | dropped | PR #1085 | 2026-07-05 |  |
| 1 | Screen-map drift: PR #1100 modified ppt-web App.tsx (FileDisputePageRoute extraction) without scr... | test-gap | medium | dropped | PR #1100 | 2026-07-05 |  |
| 1 | Risky churn: api-server main.rs security-headers wiring shipped without a middleware smoke test | bug | medium | dropped | PR #963 | 2026-07-05 |  |
| 1 | Screen-map drift: PR #922 modified ppt-web App.tsx (dev-review rounds 1-5 fixes) without a docs/s... | test-gap | medium | done | PR #922, PR #2075 | 2026-07-05 |  |
| 1 | Churn hotspot: ListingDetailScreen.kt — +1279 LOC this run (gap-82-4 reality mobile favorite toggle) | refactor | medium | done | PR #1121, PR #2059 | 2026-07-05 |  |
| 1 | Churn hotspot: DocumentsScreen.tsx — 3 PRs this run | refactor | high | done | PR #1101, PR #1081, PR #1082, PR #2077 | 2026-07-05 |  |
| 1 | iOS deep-link layer dead at runtime — Info.plist missing CFBundleURLTypes + applinks entitlement | bug | high | dropped | issue #1267, PR #1256 (verify) | 2026-07-05 |  |
| 1 | Webhook handlers RLS migration (PR #1288, PAP-170) shipped without a new regression test for repo... | test-gap | medium | dropped | PR #1288, PAP-170 | 2026-07-05 |  |
| 1 | AI llm/sessions + integrations sync + subscriptions RLS migration (PR #1287, PAP-169) shipped wit... | test-gap | medium | dropped | PR #1287, PAP-169, PAP-150 | 2026-07-05 |  |
| 1 | api_ecosystem.rs RLS migration (PR #1289, PAP-167) — 162-line handler rework shipped without a re... | test-gap | medium | dropped | PR #1289, PAP-167, PAP-150 | 2026-07-05 |  |
| 1 | mfa.rs RLS migration (PR #1292, PAP-168) shipped without a regression test; also landed broken an... | test-gap | medium | dropped | PR #1292, PR #1287, PAP-168, PAP-150 | 2026-07-05 |  |
| 1 | crypto.rs:127 SysRng.try_fill_bytes(...).expect() panics if OS CSPRNG errors during integration-c... | refactor | medium | done | code-review api-core 2026-06-15, PR #2074 | 2026-07-05 |  |
| 1 | Mobile RN production screens (Buildings/Meters/Leases/PersonMonths/Notifications/Threads/Forms) r... | bug | high | done | Phase 1.5 review of mobile-rn segment (2026-06-16) | 2026-07-05 |  |
| 1 | useDeepLinkRouting.ts:27-36 — initialize() re-runs on onNavigate identity change + void promise w... | bug | medium | dropped | Phase 1.5 review of mobile-rn segment (2026-06-16) | 2026-07-05 |  |
| 1 | Churn hotspot: backend/crates/db/src/models/mod.rs (12 commits in 19-day catch-up) | refactor | high | dropped | churn since 4829015b: 12 commits | 2026-07-05 |  |
| 1 | Churn hotspot: backend/crates/db/src/repositories/rental.rs (11 commits in 19-day catch-up) | refactor | high | done | churn since 4829015b: 11 commits | 2026-07-05 |  |
| 1 | PR #1378 closed without merge — DROP-OWNED-BY teardown theory for #1332 was wrong root cause, sup... | refactor | high | done | PR #1378, PR #1379 | 2026-06-15 |  |
| 1 | PKCE unit test became a tautology after services/oauth.rs DRY refactor (#1132) | test-gap | high | done | #1137, PR #1132, PR #1146 | 2026-06-07 |  |
| 1 | Triage: dispatcher incident — assignments-archive.json corrupted to 1/196 rows on dev branch (#1061) | triage | high | done | Issue #1061, #1061 closed | 2026-06-07 |  |
| 1 | Issue #950 (no labels, OPEN): CI: trigger-deploy 403 marks all dev image builds red and blocks st... | triage | high | done | #950, PR #1143, issue #950 closed | 2026-06-07 |  |
| 1 | Issue #952 (no labels, OPEN): [staging] Reality SSO login dead-ends: redirect_uri callback 404s o... | triage | high | done | #952, PR #1144, issue #952 closed | 2026-06-07 |  |
| 1 | Issue #769 (no labels, OPEN): Current dev review: Deploy server | triage | high | done | #769, PR #1141, issue #769 closed | 2026-06-07 |  |
| 1 | Issue #789 (no labels, OPEN): Dev review rounds 6-10: scheduler, notifications, admin, orgs, buil... | triage | high | done | #789, PR #1142, issue #789 closed | 2026-06-07 |  |
| 1 | docker/nginx admin-web + ppt-web templates churned twice this run (security headers + redirects) | dx | high | done | PR #963, PR #964, PR #1107 | 2026-06-06 |  |
| 1 | ai.rs churn-hot — 3,142 lines this run; 3,142-line route monolith, candidate for module split | refactor | medium | done | git log origin/dev since 2026-05-24, PR #1114 | 2026-06-06 |  |
| 1 | ppt-web e2e auth-refresh.spec.ts added (+252 lines, story 79-2 token-refresh coverage) | refactor | high | done | PR #1047, PR #1113 | 2026-06-06 |  |
| 1 | api-server esignature_webhook_idempotency_tests.rs added (+228 lines, terminal-state regression) | refactor | high | done | PR #1034, PR #1119 | 2026-06-06 |  |
| 1 | ppt-web EvidenceUploader.test.tsx added (+202 lines, dispute-filing AC-2 regression) | refactor | high | done | PR #1048, PR #1116 | 2026-06-06 |  |
| 1 | api-server main.rs touched twice this run (gap-sweep + security headers) — minor churn marker | refactor | high | done | PR #989, PR #963, PR #1120 | 2026-06-06 |  |
| 1 | Duplicated animate-spin spinner markup across mediation page + chat thread (no shared Spinner) | refactor | medium | done | PR #555, code-review 2026-05-27, PR #1128 | 2026-06-06 |  |
| 1 | Mediation reference number uppercases full UUID (DSP-<uuid>) instead of a short code | refactor | medium | done | PR #555, code-review 2026-05-27, PR #1130 | 2026-06-06 |  |
| 1 | frontend/apps/mobile/src/App.tsx churned twice this run (universal links + doc-detail wiring) | refactor | high | done | PR #962, PR #992, PR #1131 | 2026-06-06 |  |
| 1 | platform_admin.rs churn-hot — 2,762 lines this run (admin/OAuth-provider feature work) | refactor | medium | done | git log origin/dev since 2026-05-24, PR #1109 | 2026-06-06 |  |
| 1 | Reality-web ComparisonUrlHandler hardcodes English loading/error strings | refactor | medium | done | code-review reality-web 2026-05-28, rotating-expert-revie... | 2026-06-06 |  |
| 1 | Watch routes/oauth.rs churn after audit-log + hardening PRs | refactor | high | done | PR #930, PR #933, PR #1133 | 2026-06-06 |  |
| 1 | Watch services/oauth.rs churn after introspect/revoke hardening (#933) | refactor | high | done | PR #933, PR #1132 | 2026-06-06 |  |
| 1 | Mobile VotingScreen pure transforms toUiStatus/toUiVote have no tests | test-gap | medium | done | code-review mobile-rn 2026-05-27, rotating-expert-review,... | 2026-06-06 |  |
| 1 | Issue #749 (no labels, OPEN): Code review findings: Story 6.1 announcement creation and targeting | triage | high | done | #749, issue #749 closed | 2026-06-06 |  |
| 1 | Issue #755 (no labels, OPEN): Current dev review: Epic 8A Notification Preferences | triage | high | done | #755, issue #755 closed | 2026-06-06 |  |
| 1 | Issue #764 (no labels, OPEN): Current dev review: Admin MFA & Auth Hardening | triage | high | done | #764, issue #764 closed | 2026-06-06 |  |
| 1 | Issue #765 (no labels, OPEN): Current dev review: Integrations & Airbnb OAuth | triage | high | done | #765, issue #765 closed | 2026-06-06 |  |
| 1 | Mobile VotingScreen hardcodes en-US in toLocaleDateString — vote dates never localize | bug | medium | done | PR #1083, code-review mobile-rn 2026-05-27, rotating-expe... | 2026-06-05 |  |
| 1 | Reality-web listing generateMetadata can throw during SSR on malformed 200 body | bug | medium | done | PR #1085, code-review reality-web 2026-05-28, rotating-ex... | 2026-06-05 |  |
| 1 | PR #908 (fix(security): require PKCE on OAuth authorization-code flow, closes #823) was closed un... | security | medium | done | PR #908, PR #1025 | 2026-06-03 |  |
| 1 | Issue #751 (no labels, OPEN): Current dev review: frontend/web/API-client findings | triage | high | done | #751, #942 | 2026-06-02 |  |
| 1 | Issue #752 (no labels, OPEN): Current dev review: mobile CI tooling findings | triage | high | done | #752, #929 | 2026-06-02 |  |
| 1 | Issue #756 (no labels, OPEN): Current dev review: Epic 10A OAuth Provider | triage | high | done | #756, #934 | 2026-06-02 |  |
| 1 | Issue #761 (no labels, OPEN): Current dev review: Epic 84 E-Signature & Leases | triage | high | done | #761, #936 | 2026-06-02 |  |
| 1 | Issue #763 (no labels, OPEN): Current dev review: Reality Server & Inquiries | triage | high | done | #763, #935 | 2026-06-02 |  |
| 1 | Issue #767 (no labels, OPEN): Current dev review: Mobile RN Property Management app | triage | high | done | #767, #943 | 2026-06-02 |  |
| 1 | Issue #768 (no labels, OPEN): Current dev review: Admin-web features (10B) | triage | high | done | #768, #930 | 2026-06-02 |  |
| 1 | Issue #920 (no labels, OPEN): Announcement targeting not enforced on read (intra-org disclosure) | triage | high | done | #920, #944 | 2026-06-02 |  |
| 1 | Issue #750 (no labels, OPEN): Current dev review: backend/API/database findings | triage | high | done | #750, PR #922 | 2026-06-01 |  |
| 1 | Issue #753 (no labels, OPEN): Current dev review: Epic 6 Announcements & Communication | triage | high | done | #753 | 2026-06-01 |  |
| 1 | Issue #754 (no labels, OPEN): Current dev review: Epic 7A Basic Document Management | triage | high | done | #754, PR #914 | 2026-06-01 |  |
| 1 | Issue #757 (no labels, OPEN): Current dev review: Epic 10B Platform Administration | triage | high | done | #757 | 2026-06-01 |  |
| 1 | Issue #760 (no labels, OPEN): Current dev review: Epic 79 Disputes & Mediation | triage | high | done | #760, PR #915 | 2026-06-01 |  |
| 1 | Issue #762 (no labels, OPEN): Current dev review: Reports & Schedules | triage | high | done | #762 | 2026-06-01 |  |
| 1 | Issue #766 (no labels, OPEN): Current dev review: AI & LLM routes | triage | high | done | #766, PR #879 | 2026-06-01 |  |
| 1 | Issue #770 (no labels, OPEN): Current dev review: Faults & triage | triage | high | done | #770, PR #902 | 2026-06-01 |  |
| 1 | Issue #771 (no labels, OPEN): Current dev review: Research dispatcher & CI automation | triage | high | done | #771, PR #923 | 2026-06-01 |  |
| 1 | Issue #772 (no labels, OPEN): Current dev review: Auth core (delta confirmation) | triage | high | done | #772 | 2026-06-01 |  |
| 1 | Issue #773 (no labels, OPEN): Current dev review: Leases & rental | triage | high | done | #773 | 2026-06-01 |  |
| 1 | Issue #774 (no labels, OPEN): Current dev review: Reality server (broad) | triage | high | done | #774, PR #919 | 2026-06-01 |  |
| 1 | Issue #775 (no labels, OPEN): Current dev review: WebSocket realtime | triage | high | done | #775, PR #926 | 2026-06-01 |  |
| 1 | Issue #776 (no labels, OPEN): Current dev review: Equipment & audit log | triage | high | done | #776 | 2026-06-01 |  |
| 1 | Issue #777 (no labels, OPEN): Current dev review: Compliance & GDPR | triage | high | done | #777 | 2026-06-01 |  |
| 1 | Issue #778 (no labels, OPEN): Current dev review: Marketplace, voting, investor portal, impersona... | triage | high | done | #778, PR #882 | 2026-06-01 |  |
| 1 | Issue #788 (no labels, OPEN): Dev review rounds 1-5: mobile-native + ppt-web surfaces | triage | high | done | #788, PR #922 | 2026-06-01 |  |
| 1 | Issue #790 (no labels, OPEN): Dev review rounds 11-15: vendor, predictive, reality-web, middleware | triage | high | done | #790, PR #913 | 2026-06-01 |  |
| 1 | Issue #791 (no labels, OPEN): Dev review rounds 16-20: push, e-sign, portal, webhooks, reserves | triage | high | done | #791, PR #924 | 2026-06-01 |  |
| 1 | Issue #846 (no labels, OPEN): Code review: Epics 12+65 — Meters & Energy/ESG (origin/dev) | triage | high | done | #846, PR #880 | 2026-06-01 |  |
| 1 | Issue #847 (no labels, OPEN): Code review: Reality-server — Inquiries IDOR (Epics 16–19) (origin/... | triage | high | done | #847 | 2026-06-01 |  |
| 1 | Issue #848 (no labels, OPEN): Code review: Epics 78+134 — Vendor portal stubs & Predictive mainte... | triage | high | done | #848, PR #913 | 2026-06-01 |  |
| 1 | Issue #850 (no labels, OPEN): Code review: Epics 61+146+42 — Multi-currency, Data residency, Viol... | triage | high | done | #850, PR #883 | 2026-06-01 |  |
| 1 | Issue #851 (no labels, OPEN): Code review: Epics 15+105+69 — Listings/syndication & Developer API... | triage | high | done | #851, PR #904 | 2026-06-01 |  |
| 1 | Issue #859 (no labels, OPEN): sqlx 0.9 breaks runtime decode of Postgres enum columns into Rust S... | triage | high | done | #859, PR #871 | 2026-06-01 |  |
| 1 | Issue #867 (no labels, OPEN): Tech debt: api-server main.rs duplicates lib.rs::create_router — ro... | triage | high | done | #867, PR #870 | 2026-06-01 |  |
| 1 | Issue #836 (no labels, OPEN): Code review: Epic 2B-C — Mobile push & device registration (origin/... | triage | high | done | #836, PR #866 | 2026-05-31 |  |
| 1 | Issue #845 (no labels, OPEN): Code review: Epic 14 — IoT alerts, correlations, thresholds (origin... | triage | high | done | #845, PR #862 | 2026-05-31 |  |
| 1 | Issue #849 (no labels, OPEN): Code review: Epic 10B+143 — Admin impersonation, Help, Board meetin... | triage | high | done | #849, PR #869 | 2026-05-31 |  |
| 0 | Cloud routine cadence recovery — reduce 3–4d gaps between runs | dx | high | dropped | routine self-signal 2026-07-09 | 2026-07-09 |  |
| 0 | Churn hotspot: 1021 lines changed in backend/servers/api-server/src/routes/emergency.rs (window 2026 | refactor | high | dropped | local git numstat since 2026-06-07 | 2026-07-05 |  |
| 0 | Churn hotspot: 929 lines changed in backend/servers/api-server/src/routes/vendors.rs (window 2026-06 | refactor | high | dropped | local git numstat since 2026-06-07 | 2026-07-05 |  |
| 0 | Churn hotspot: 709 lines changed in backend/servers/api-server/src/routes/enhanced_tenant_screening. | refactor | high | dropped | local git numstat since 2026-06-07 | 2026-07-05 |  |
| 0 | Churn hotspot: 2940 lines changed in backend/crates/db/src/repositories/document.rs (window 2026-... | refactor | high | dropped | local git numstat since 2026-06-10T03:05:00Z | 2026-07-05 |  |
| 0 | Churn hotspot: 2856 lines changed in backend/crates/db/src/repositories/subscription.rs (window 2... | refactor | high | dropped | local git numstat since 2026-06-10T03:05:00Z, PR #1246 | 2026-07-05 |  |
| 0 | Churn hotspot: 2691 lines changed in backend/servers/api-server/src/routes/aml_dsa.rs (window 202... | refactor | high | dropped | local git numstat since 2026-06-10T03:05:00Z, PR #1193, P... | 2026-07-05 |  |
| 0 | Issue #1151 (no labels, OPEN): Research dispatcher: claimable buffer is stale — true claimable wo... | triage | high | dropped | #1151 | 2026-07-05 |  |
| 0 | Churn hotspot: SearchScreen.kt — +1293 LOC this run (gap-82-3 reality mobile search/filters) | refactor | medium | dropped | PR #1125 | 2026-07-05 |  |
| 0 | MainActivity reimplements deep-link dispatch instead of calling shared DeepLinkRouter — drift trap | refactor | high | dropped | mobile-native-kmp segment review 2026-06-06 | 2026-07-05 |  |
| 0 | Churn hotspot: AnnouncementsScreen.tsx — 4 PRs this run, instability proxy | refactor | high | dropped | PR #1101, PR #1077, PR #1083, PR #1098 | 2026-07-05 |  |
| 0 | Churn hotspot: AnnouncementsScreen.test.ts — 4 PRs this run, instability proxy | refactor | high | dropped | PR #1101, PR #1077, PR #1083, PR #1098 | 2026-07-05 |  |
| 0 | Dispatcher action-list.json corruption when MCP push falls back from blocked git push | triage | high | dropped | #1014 | 2026-07-05 |  |
| 0 | Issue #951 (no labels, OPEN): Deploy blocker: api-server requires ESIGN_TOKEN_SECRET + ESIGN_WEBH... | triage | high | dropped | #951 | 2026-07-05 |  |
| 0 | PR #1274 (cargo-minor-patch group, /backend, 9 updates) closed unmerged — superseded by #1313 aft... | dx | high | dropped | PR #1274 | 2026-07-05 |  |
| 0 | Churn hotspot: backend/crates/integrations/src/booking.rs (+404/−29 in PR #1294 Booking.com OTA r... | refactor | high | dropped | PR #1294 commit 7ccce8a | 2026-07-05 |  |
| 0 | Churn hotspot: backend/servers/api-server/src/routes/api_ecosystem.rs (+106/−27 in PR #1293 PAP-1... | refactor | high | dropped | PR #1293 commit 1e50156 | 2026-07-05 |  |
| 0 | Churn hotspot: backend/crates/db/src/repositories/reality_portal.rs (+59/−28 in PR #1297 PAP-142 ... | refactor | high | dropped | PR #1297 commit 8c711c6 | 2026-07-05 |  |
| 0 | Churn hotspot: backend/servers/api-server/src/routes/iot.rs (+278/-403 in PR #1321/#1322 PAP-151 ... | refactor | high | dropped | PR #1321 commit, PR #1322 commit | 2026-07-05 |  |
| 0 | Churn hotspot: backend/servers/api-server/src/routes/reserve_funds.rs (+228/-255 in PR #1321 PAP-... | refactor | high | dropped | PR #1321 commit | 2026-07-05 |  |
| 0 | Churn hotspot: backend/crates/db/src/repositories/sensor.rs (+248/-86 in PR #1321/#1322 PAP-151 r... | refactor | high | dropped | PR #1321 commit, PR #1322 commit | 2026-07-05 |  |
| 0 | Issue #1331 (no labels, OPEN): Backend `test` job red/hanging on dev base — blocks the entire bac... | triage | high | dropped | #1331 | 2026-07-05 |  |
| 0 | Stalled review: PR #988 (Epic: reusable Playwright E2E framework + sitemap FlowRunner) open 10d, ... | dx | high | dropped | PR #988 | 2026-07-05 |  |
| 0 | Churn hotspot: backend/servers/api-server/src/routes/forms.rs touched 2x since 2026-06-12 (window... | refactor | high | dropped | local git numstat since 2026-06-12, local git numstat sin... | 2026-07-05 |  |
| 0 | Churn hotspot: backend/servers/api-server/tests/reserve_funds_cross_org_idor_tests.rs touched 2x ... | refactor | high | dropped | local git numstat since 2026-06-12 | 2026-07-05 |  |
| 0 | Churn hotspot: backend/crates/db/tests/form_rls_repo_tests.rs touched 2x since 2026-06-12 (window... | refactor | high | dropped | local git numstat since 2026-06-12 | 2026-07-05 |  |
| 0 | Issue #1380 (no labels, OPEN): Dispatcher stale gap-scan buffer + Tier-2 escalation endpoint misc... | triage | high | dropped | issue #1380 | 2026-07-05 |  |
| 0 | Churn hotspot: 124 lines in frontend/apps/mobile/app.config.icon.test.ts (PR #1383 gap-85-2) | refactor | high | dropped | PR #1383 | 2026-07-05 |  |
| 0 | Churn hotspot: 94 lines in frontend/apps/mobile/app.config.ts (PR #1383 gap-85-2) | refactor | high | dropped | PR #1383 | 2026-07-05 |  |
| 0 | Churn hotspot: 53 lines in backend/crates/db/src/repositories/form.rs (PR #1379 #1332 unblock) | refactor | high | dropped | PR #1379, issue #1332 | 2026-07-05 |  |
| 0 | booking_oauth_csrf_tests.rs hotspot — 484-line NEW test file (PR #1393 #1424 OAuth CSRF coverage) | refactor | high | dropped | local git numstat since 2026-06-15 (commit 67c24bd..origi... | 2026-07-05 |  |
| 0 | booking_oauth_routes_tests.rs hotspot — 381-line NEW test file (PR #1393 OAuth routes coverage) | refactor | high | dropped | local git numstat since 2026-06-15 | 2026-07-05 |  |
| 0 | forms.rs repeated-churn — runs_seen=2 (#1337 explicit_auto_deref + #1397 org-scope hardening) | refactor | high | dropped | hotspot_history.runs_seen 1→2 with new churn this run | 2026-07-05 |  |
| 0 | PR #1425 (GH #1377 document presigned-URL tests) closed unmerged — superseded by merged #1394 | dx | high | dropped | PR #1425 | 2026-07-05 |  |
| 0 | PR #1179 (docs(epics) catalog backfill for 37 mounted-but-undocumented backend modules) — stalled... | dx | high | dropped | PR #1179 | 2026-07-05 |  |
| 0 | Stabilize oauth_integration_tests churn — heavy edits across 3 OAuth fix PRs | refactor | high | dropped | PR #930, PR #933 | 2026-06-16 |  |
| 0 | Issue #779 (no labels, OPEN): Current dev review: consolidated priority rollup (origin/dev snapshot) | triage | high | dropped | #779 | 2026-06-13 |  |
| 0 | Announcer: untracked clear-then-set timeouts can resurrect a stale screen-reader message | bug | medium | dropped | code-review ppt-web-ui 2026-05-24, Announcer.tsx:49 | 2026-06-07 |  |
| 0 | Portfolio dashboard: alert mark-read/resolve mutations + property-card click navigation are no-op... | dx |  | dropped | PR #328, commit 254f01d | 2026-06-04 |  |
