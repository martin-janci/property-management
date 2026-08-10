# Backlog of vectors
<sub>Last regenerated: 2026-08-10 06:15 UTC by routine</sub>

| Score | Title | Vector | Source | Updated | Status |
|-------|-------|--------|--------|---------|--------|
| 8 | the Screen.CreateListing composable wires CreateListingScreen(onSubmit = { _ ->  | bug | tier1d-dev-review segment=mobile-native-kmp | 2026-08-10 | ready |
| 6 | Add regression tests for inquiry mark_as_read cross-tenant IDOR fix (PR #497) | test-gap | PR #497 | 2026-05-26 | done |
| 5 | SECURITY: Alexa voice webhook accepts forged requests — verify_alexa_signature never checks the signature | security | standing-scan-2026-07-28 | 2026-07-28 | dropped |
| 5 | `require_rls_context` is documented (lines 181-184) as 'a stricter version ... U | bug | tier1d-dev-review segment=api-core | 2026-08-10 | ready |
| 4 | backend/servers/reality-server/src/routes/sso.rs:682 (exchange_code_for_tokens), | bug | tier1d-dev-review segment=reality-server | 2026-08-10 | open |
| 4 | introspect_pm_token() treats ANY non-2xx introspection response as an inactive t | bug | tier1d-dev-review segment=reality-server | 2026-08-10 | open |
| 3 | IDOR: equipment delete/update + maintenance update mutate any tenant's equipment by ID with no org scoping | security | code-review api-core 2026-05-25 | 2026-05-25 | done |
| 3 | SSRF: signed-document fetch + webhook-test POST issue outbound requests to unvalidated user-controlled URLs | security | issue #439 | 2026-05-25 | done |
| 3 | IDOR: unlink_voice_device deactivates any device by ID with no owner/org scoping | security | code-review api-core 2026-05-23 | 2026-05-25 | done |
| 3 | IDOR: reality-server realtors mark_inquiry_read flips any realtor's inquiry by ID with no owner scoping | security | issue #519 | 2026-05-26 | done |
| 3 | IDOR: ai.rs LLM-doc handlers publish/list/get any tenant's listing descriptions & photo enhancements unscoped | security | code-review api-core 2026-05-29 | 2026-06-01 | dropped |
| 3 | Schema drift: runtime SQL errors from non-existent columns in voting/messaging/notification paths | bug | Issue #1008 | 2026-06-07 | done |
| 3 | PR #1193 (fix(aml-dsa): lock DSA reports to platform roles + fix file-path disclosure (PAP-47)) merg | security | PR #1193 | 2026-06-10 | dropped |
| 3 | PR #1203 (fix(aml_dsa): close cross-tenant IDOR in moderation + AML-review handlers (PAP-36)) merged | security | PR #1203 | 2026-06-10 | dropped |
| 3 | iOS SearchView.swift does not compile — performSearch/scheduleSearch undefined, resultsGrid corrupted | bug | issue #1266 | 2026-06-11 | dropped |
| 3 | Reality-web listing detail SSR crashes on partial 200 body — JSON-LD build deref of undefined fields | bug | rotating-expert-review reality-web 2026-06-14 | 2026-06-14 | dropped |
| 3 | Reality-web ComparisonUrlHandler hits non-existent /api/listings/${id} — every shared comparison URL 404s | bug | rotating-expert-review reality-web 2026-06-14 | 2026-06-14 | dropped |
| 3 | Reality-web RealtorManagement.tsx hardcoded English strings — agency flow not localized to sk/cs/de | bug | rotating-expert-review reality-web 2026-06-14 | 2026-06-15 | done |
| 3 | ReportFaultScreen.tsx handleSubmit() fakes API call with setTimeout(1500) — fault reports never reach backend (App.tsx:1 | bug | Phase 1.5 review of mobile-rn segment (2026-06-16) | 2026-06-16 | dropped |
| 3 | revoke_all_sessions ignores refresh cookie — signs the caller out too | bug | Phase 1.5 code-review 2026-07-09 (api-handlers segment) | 2026-07-09 | done |
| 3 | SECURITY: community.rs 5 write handlers (create_post/add_reaction/create_comment/rsvp_event/create_inquiry) accept cross | security | Phase 1.5 rotating expert review 2026-07-23 (api-handlers segment) | 2026-07-23 | done |
| 3 | SECURITY: community.rs get_group/list_posts/get_item run unauthenticated — anonymous cross-tenant read | security | Phase 1.5 rotating expert review 2026-07-23 (api-handlers segment) | 2026-07-23 | dropped |
| 3 | SECURITY: Android SSO deep-link handler skips CSRF state check — reality://sso?token=... enables account takeover | security | Phase 1.5 rotating expert review 2026-07-27 (mobile-native-kmp segment) | 2026-07-27 | done |
| 3 | SDK drift gate is effectively unenforced — api-validation.yml only fires on docs/api/**, so committed @ppt/api-client dr | dx | standing-scan-2026-07-28 | 2026-07-28 | done |
| 3 | auth.rs repeated-churn — runs_seen=4, 2950 lines / ~107K in one module (2nd-largest route file) | refactor | standing-scan-2026-07-28 | 2026-07-28 | dropped |
| 3 | uploadDocumentDirect() silently drops building_id — building-scoped document uploads lose association vs legacy multipar | bug | Issue #2366 | 2026-08-01 | dropped |
| 3 | Scheduler notifications fire-once: transient target-resolution or dispatch error permanently drops announcement / vote n | bug | Issue #2612 | 2026-08-01 | dropped |
| 3 | api-server workflow api_call.rs has a duplicate SSRF validator that drifts from common::url_validation — plain HTTP is a | security | PR #2627 | 2026-08-02 | done |
| 3 | reality-server GET /api/v1/agencies/{id}/members has no auth or membership check — unauthenticated cross-agency member e | bug | rotating-expert-review | 2026-08-02 | dropped |
| 3 | reality-server sync_session swallows invalidate_session error — portal session survives after PM token goes inactive | security | rotating-expert-review reality-server 2026-08-01 | 2026-08-02 | dropped |
| 3 | ppt-web-core: the client heartbeat sends an APPLICATION-level ping frame {type:'ping', pa... | bug | rotating-expert-review | 2026-08-06 | done |
| 3 | dev CI broken: duplicate SQLx migration 00220 — renumber portal_get_listing_view_count to 00227 | bug | Issue #2699 | 2026-08-07 | done |
| 3 | `action_check_meter` returns `success: true` with the response 'Your latest mete | bug | tier1d-dev-review segment=api-core | 2026-08-10 | open |
| 3 | verify_hmac_signature() FAILS OPEN: line 950 `std::env::var("VOICE_WEBHOOK_SECRE | security | tier1d-dev-review segment=api-handlers | 2026-08-10 | done |
| 3 | webhook signature is verified with a non-constant-time comparison: `Ok(signature | security | tier1d-dev-review segment=api-handlers | 2026-08-10 | done |
| 3 | authenticate_voice_user() performs NO token validation: the `_access_token` para | security | tier1d-dev-review segment=api-handlers | 2026-08-10 | done |
| 3 | both handlers generate an OFFICIAL LEGAL condominium voting-minutes d | bug | tier1d-dev-review segment=api-handlers | 2026-08-10 | done |
| 2 | Complete RLS migration in 31 remaining handlers (voting, market_pricing, faults, notif_prefs, reports) | security | issue #160 | 2026-05-23 | done |
| 2 | Dead/duplicate handler modules: AuthHandler & BuildingHandler unused, routes reimplement inline | refactor | code-review api-handlers 2026-05-23 | 2026-05-24 | done |
| 2 | documents.rs churn-hot — 10,659 lines over 14d | refactor | git log origin/main since 2026-05-06 | 2026-05-25 | done |
| 2 | integrations.rs churn-hot — 12,977 lines over 14d, candidate for module split | refactor | git log origin/main since 2026-05-06 | 2026-05-25 | done |
| 2 | organizations.rs churn-hot — 12,060 lines over 14d (multitenancy + admin) | refactor | git log origin/main since 2026-05-06 | 2026-05-25 | done |
| 2 | IDOR: reality-server mark_as_read flips any realtor's inquiry by ID with no owner scoping | security | code-review reality-server 2026-05-23 | 2026-05-25 | done |
| 2 | Latent fail-open: ProtectedRoute role check is skipped when user.role is falsy | security | code-review ppt-web-ui 2026-05-24 | 2026-05-25 | done |
| 2 | Screen-map drift: PR #464 wired a neighbors route in ppt-web without a docs/screens/ppt entry | test-gap | PR #464 | 2026-05-25 | done |
| 2 | Screen-map drift: PR #460 touched reality-web listing page without a docs/screens/reality update | test-gap | PR #460 | 2026-05-25 | closed |
| 2 | Dispute state machine (PR #506) shipped with no tests + no org predicate on update_status | test-gap | PR #506 | 2026-05-26 | done |
| 2 | Screen-map drift: report execution-history route (PR #547) added without a ppt screen doc | test-gap | PR #547 | 2026-05-27 | done |
| 2 | ReportSchedule.update_schedule stores cron in `time` workaround; documented UPDATE never runs (missing cron_expression c | bug | PR #611 | 2026-05-30 | done |
| 2 | api-server main.rs vs lib.rs::create_router diverge silently (5 routes unreachable in prod, no test asserts parity) | test-gap | PR #866 | 2026-06-01 | done |
| 2 | MediationWorkspacePage shows empty/unknown state instead of error UI on dispute fetch failure | bug | PR #555 | 2026-06-03 | done |
| 2 | Mobile VotingScreen double-casts API result across boundary — render-time crash on unexpected shape | bug | code-review mobile-rn 2026-05-27 | 2026-06-03 | done |
| 2 | Reality-web InviteRealtorModal swallows invite-mutation failure with no error UI | bug | code-review reality-web 2026-05-28 | 2026-06-03 | done |
| 2 | Airbnb webhook at-least-once delivery enqueues duplicate SYNC_EXTERNAL jobs | bug | PR #538 | 2026-06-03 | done |
| 2 | DocumentsBrowse MoveFolderDialog cannot pre-select current folder (DocumentSummary lacks folder_id) | dx | PR #623 | 2026-06-03 | done |
| 2 | API + SPA security-headers middleware (PR #963) shipped without an assertion test for HSTS/nosniff/CSP | test-gap | PR #963 | 2026-06-03 | done |
| 2 | ppt-web status/auth components hardcode English in an otherwise i18n'd app | refactor | code-review ppt-web-ui 2026-05-24 | 2026-06-04 | done |
| 2 | Risky churn: mobile App.tsx deep-link/doc-detail wiring changing across back-to-back PRs without coverage | bug | PR #1103 | 2026-06-05 | done |
| 2 | Integration marketplace install/OAuth flows are placeholders — wire backend handlers + UI navigation | dx | PR #1105 | 2026-06-05 | done |
| 2 | Booking push availability/rates endpoints add batch-cap + non-negative guards with no regression test | test-gap | PR #1068 | 2026-06-05 | done |
| 2 | Portal webhook fail-closed fix (PR #874) shipped without a regression test for unverified-signature rejection | test-gap | PR #1052 | 2026-06-05 | done |
| 2 | Mobile dev-review batch (PR #918, 5 files under frontend/apps/mobile/src) shipped without a regression test | test-gap | PR #1072 | 2026-06-05 | done |
| 2 | Reality-server SSO consumer review fix (PR #921, closes #820) shipped without a regression test | test-gap | PR #1076 | 2026-06-05 | done |
| 2 | CI branch-protection + auto-rebase workflow change (PR #923) shipped without an integration test | test-gap | PR #1057 | 2026-06-05 | done |
| 2 | deploy-server OIDC scope mapping (#939) shipped without unit test for derive_oidc_scopes | test-gap | PR #1106 | 2026-06-05 | done |
| 2 | Mobile RN dev-review tail (#943) shipped without test coverage | test-gap | PR #1080 | 2026-06-05 | done |
| 2 | Frontend gap-sweep (PR #990, 34 files across Epics 1/6/7B/9/10B/11/15/17/18) shipped without a regression test | test-gap | PR #1081 | 2026-06-05 | done |
| 2 | Mobile document-detail wiring (PR #992) shipped without a regression test for the deep-link payload path | test-gap | PR #1082 | 2026-06-05 | done |
| 2 | Screen-map drift: PR #839 modified ppt-web App.tsx (FileDisputePageRoute) without a docs/screens/ppt update | test-gap | PR #1056 | 2026-06-05 | done |
| 2 | PushFanoutWorker BLPOP queue-drain deferred — Redis path is a logging no-op | dx | PR #515 | 2026-06-06 | done |
| 2 | ai.rs (3,134 LOC) — explicit module-split into routes/ai/{sessions,equipment,workflows,voice,llm,mod}.rs | refactor | pm-tech-lead analysis 2026-05-25 | 2026-06-06 | done |
| 2 | announcements.rs churn-hot — 2,722 lines this run (Epic 2B + Epic 6 work) | refactor | git log origin/dev since 2026-05-24 | 2026-06-06 | done |
| 2 | announcements.rs (2,722 LOC) — explicit module-split into routes/announcements/{crud,targeting,delivery,reactions,mod}.r | refactor | pm-tech-lead analysis 2026-05-25 | 2026-06-06 | done |
| 2 | Reduce App.tsx route-aggregator coupling (top churn hotspot, merge-conflict risk) | refactor | PR #474 | 2026-06-06 | done |
| 2 | platform_admin.rs (2,762 LOC) — explicit module-split into routes/platform_admin/{tenants,features,billing,audit,mod}.rs | refactor | pm-tech-lead analysis 2026-05-25 | 2026-06-06 | done |
| 2 | Screen-map drift: PR #1033 wired error/retry into AnnouncementsPage+FaultsPage via App.tsx without a docs/screens/ppt up | test-gap | PR #1033 | 2026-06-06 | done |
| 2 | PR #1196 (feat(ppt-web): add missing test coverage for faults feature) merged with 2 unchecked TODO  | test-gap | PR #1196 | 2026-06-10 | dropped |
| 2 | vote.rs:1765 calculate_question_result() uses partial_cmp().unwrap() on f64 — NaN/Inf weights panic /votes/{id}/results | bug | code-review api-core 2026-06-15 | 2026-06-16 | done |
| 2 | PR #1418 touched routes/** (faults.route.test.tsx) without updating docs/screens/ppt/* — heuristic, test-file fix | test-gap | PR #1418 | 2026-07-05 | done |
| 2 | Reality-server listings pagination clamp (PR #959) shipped without a regression test for limit=-1 | test-gap | PR #959 | 2026-07-05 | done |
| 2 | /forgot-password and /resend-verification have no rate limit — mailbomb / token-clobber | security | Phase 1.5 code-review 2026-07-09 (api-handlers segment) | 2026-07-09 | done |
| 2 | Churn hotspot cluster: api-server routes/auth.rs (runs_seen=3) + auth_tests.rs + reality-server routes/sso.rs | refactor | PR #2205 | 2026-07-12 | done |
| 2 | scheduler.rs units/buildings target queries lack organization_id AND-scope — fan-out can leak across tenants if create-a | security | code-review api-core 2026-07-20 | 2026-07-20 | done |
| 2 | Dashboard useActionQueue queryFn returns generateMockData — production users see fabricated action items; approve/reject | bug | code-review ppt-web-ui 2026-07-21 | 2026-07-21 | done |
| 2 | TenantSectionEditor PropInput silently JSON.parse-coerces every string prop on blur — override payload corrupted ("true" | bug | code-review ppt-web-ui 2026-07-21 | 2026-07-21 | done |
| 2 | DashboardCustomizePage 'changed since sent' check is tautological — concurrent edits during in-flight save are silently  | bug | code-review ppt-web-ui 2026-07-21 | 2026-07-21 | done |
| 2 | Screen-map drift: PR #2431 touched reality-web/src/app/api/layout-revalidate/route.ts without updating docs/screens/real | test-gap | PR #2431 | 2026-07-21 | done |
| 2 | AuthContext init bypasses refreshTokenInternal → stale role on cold-boot refresh (#574 fix gap) | bug | Phase 1.5 rotating expert review 2026-07-24 (ppt-web-core segment) | 2026-07-24 | done |
| 2 | WebSocket not re-authed on token rotation — connect() early-return leaves live socket on old token | bug | Phase 1.5 rotating expert review 2026-07-24 (ppt-web-core segment) | 2026-07-24 | done |
| 2 | screen-map drift: PR #2497 touched reality-web/app/api/layout-revalidate/route.ts w/o docs/screens/reality/ | test-gap | PR #2497 | 2026-07-24 | done |
| 2 | PR #2547 shipped scheduler retention prune fix without an api-server regression test (hotfix-no-test) | bug | PR #2547 (merged 2026-07-24) | 2026-07-27 | done |
| 2 | Churn hotspot: backend/servers/api-server/src/routes/reports.rs — 3329 lines this window, runs_seen=3 (repeated instabil | refactor | Phase 1 churn scan 2026-07-27 (commit range 2026-07-24..2026-07-27 on dev) | 2026-07-27 | done |
| 2 | AmlDashboardPage casts raw window.prompt text into the review-decision union — a typo submits an invalid AML decision | bug | standing-scan-2026-07-28 | 2026-07-28 | done |
| 2 | SECURITY: reality-web layout.tsx inlines tenant-config JSON into <script> without </script>/U+2028/U+2029 escaping — HTM | security | rotating-expert-review 2026-07-28 | 2026-07-28 | done |
| 2 | admin-web platform-settings + mobile-config Save paths are permanent no-ops — the backing endpoints do not exist | dx | standing-scan-2026-07-28 | 2026-07-28 | done |
| 2 | test-gap: voice_webhooks.rs (1148 lines, 6 mounted endpoints incl. OAuth token exchange) has no tests at all | test-gap | standing-scan-2026-07-28 | 2026-07-28 | done |
| 2 | SECURITY-LITE: layout/resolved.rs err500 handler leaks raw sqlx/serde error text on public GET /layout/resolved/{screen} | bug | rotating-expert-review api-core 2026-07-30 | 2026-07-30 | done |
| 2 | admin-web mobile-config Save flow blocked: PATCH /api/v1/admin/mobile-config endpoint missing | dx | PR #2594 (commit fb09556) | 2026-07-31 | dropped |
| 2 | admin-web platform-settings Save blocked: PATCH /api/v1/platform-admin/settings endpoint missing | dx | PR #2594 (commit fb09556) | 2026-07-31 | dropped |
| 2 | @ppt/ui-kit missing primitives (Stepper, FileUpload, RadioCards, StatusPill) — admin-web ships inline duplicates | dx | PR #2594 (commit fb09556) | 2026-07-31 | done |
| 2 | openapi-ts generator wrapper swallows errors and emits weak error types | dx | PR #2607 (commit 5c604d5) | 2026-07-31 | done |
| 2 | Churn hotspot: backend/servers/api-server/src/routes/auth.rs — 2950 lines this window (runs_seen=5, no refactor PR yet) | refactor | commit window 2026-07-30→2026-07-31 | 2026-07-31 | dropped |
| 2 | Churn hotspot: backend/servers/api-server/src/routes/reports.rs — 3329 lines this window (PR #2599 extracted helpers) | refactor | PR #2599 | 2026-07-31 | dropped |
| 2 | Churn hotspot: backend/crates/integrations/src/booking/mod.rs — 3626 lines this window (recently split by PR #2611) | refactor | PR #2611 | 2026-07-31 | done |
| 2 | Screen-map drift: reality-web layout changed without docs/screens/reality/ update (PR #2600) | test-gap | PR #2600 | 2026-07-31 | done |
| 2 | /disputes/kpis: no window_start<=window_end validation and only test is BIT-440 quarantined | test-gap | Issue #2575 | 2026-08-01 | dropped |
| 2 | Idempotency middleware trusts client-supplied X-Tenant-ID header for cache-scope key — cross-tenant collision or bypass | security | PR #2626 | 2026-08-02 | done |
| 2 | reality-server leaks raw sqlx::Error strings to internet-facing clients, bypassing util::errors::db_error | security | rotating-expert-review reality-server 2026-08-01 | 2026-08-02 | dropped |
| 2 | mobile: useOfflineSupport.processQueue reports isComplete:true after a head-of-line-blocked (halted) sync cycle | bug | rotating-expert-review (tier1d mobile-rn) | 2026-08-03 | done |
| 2 | mobile: ThreadDetailScreen.handleSend send-message mutation has onSuccess only, no onError — failed sends are silent | bug | rotating-expert-review (tier1d mobile-rn) | 2026-08-03 | done |
| 2 | ppt-web axios retry interceptor retries non-idempotent POST/PUT on 5xx / network errors — risk of duplicate server-side  | bug | rotating-expert-review (tier1d ppt-web-core) | 2026-08-03 | done |
| 2 | ppt-web getApiClient() sends requests with no Authorization header — configureApiClient is never called, so the token pr | bug | rotating-expert-review (tier1d ppt-web-core) | 2026-08-03 | done |
| 2 | ppt-web rentals mutations dereference `auth!` non-null — mid-session token loss throws uncaught TypeError instead of han | bug | rotating-expert-review (tier1d ppt-web-core) | 2026-08-03 | done |
| 2 | ppt-web has no route-outlet ErrorBoundary — a single stale-chunk lazy() rejection unmounts the entire application shell | bug | rotating-expert-review (tier1d ppt-web-core) | 2026-08-03 | done |
| 2 | AccountingInvoiceManagementPage: create/delete mutations have no onError — silent invoice failures | bug | rotating-expert-review (tier1d ppt-web-ui) | 2026-08-03 | done |
| 2 | docs/screens/reality/agency-import + agency-inquiries out of sync with PR #2636 i18n rewrite | test-gap | PR #2636 | 2026-08-03 | done |
| 2 | mobile-native-kmp: getPortfolioAnalytics() truncates realtor portfolio at 100 listings — dashboard under-reports on larg | bug | Tier1d review 2026-08-04 (mobile-native-kmp) | 2026-08-04 | open |
| 2 | mobile: useOfflineSupport — transient (5xx/network) queued actions are permanently DROPPED after 3 retries with only con | bug | rotating-expert-review (tier1d mobile-rn) | 2026-08-04 | done |
| 2 | ppt-web logout leaves 3 tenant-scoped TanStack Query roots un-purged (predictive-maintenance, sentiment, notification-an | bug | rotating-expert-review (tier1d ppt-web-core) | 2026-08-04 | done |
| 2 | ppt-web rentals + financial useMutation hooks lack onError — money-movement + platform-connect failures produce zero use | bug | rotating-expert-review (tier1d ppt-web-core) | 2026-08-04 | done |
| 2 | screen-map drift: PR #2646 touched ppt-web route wrapper (App.tsx) without updating docs/screens/ppt/ | test-gap | PR #2646 | 2026-08-04 | dropped |
| 2 | screen-map drift: PR #2647 touched 8 ppt-web route wrappers (i18n not-found fallbacks) without updating docs/screens/ppt | test-gap | PR #2647 | 2026-08-04 | dropped |
| 2 | screen-map drift: PR #2648 touched ppt-web rentals route (mutation auth guard) without updating docs/screens/ppt/ | test-gap | PR #2648 | 2026-08-04 | done |
| 2 | screen-map drift: PR #2649 touched ppt-web rentals + financial routes (onError toasts) without updating docs/screens/ppt | test-gap | PR #2649 | 2026-08-04 | done |
| 2 | api-handlers: enhanced_chat is a stubbed production handler that returns fabricated data.... | bug | tier1d-dispatcher-generator | 2026-08-06 | done |
| 2 | mobile-rn: the cold-start initialize() effect reads the stored access token (SecureSto... | bug | tier1d-dispatcher-generator | 2026-08-06 | done |
| 2 | ppt-web-core: once reconnectAttempts reaches maxReconnectAttempts (default 10, :290) sche... | bug | rotating-expert-review | 2026-08-06 | done |
| 2 | reality-server: InquiryResult::RateLimited variant is defined but never constructed or matc... | bug | tier1d-dispatcher-generator | 2026-08-06 | done |
| 2 | reality-server: the LIVE public listing-detail handler get_listing() (route wired at routes... | bug | tier1d-dispatcher-generator | 2026-08-06 | done |
| 2 | reality-web ComparisonUrlHandler uses Promise.all — one bad listing id blanks the whole shared comparison URL | bug | Tier1d review 2026-08-07 (reality-web) | 2026-08-07 | done |
| 2 | reality-web ListingForm posts NaN for area/rooms — non-numeric or negative input coerced silently | bug | Tier1d review 2026-08-07 (reality-web) | 2026-08-07 | done |
| 2 | the payment auto-matcher's inner `for invoice in &open_invoices` loop (accountin | bug | tier1d-dev-review segment=api-core | 2026-08-10 | open |
| 2 | inside a single function, `Scheduler::get_announcement_target_users`, the `"role | bug | tier1d-dev-review segment=api-core | 2026-08-10 | open |
| 2 | `require_permission` reads the caller's role from request extensions with `.get: | security | tier1d-dev-review segment=api-core | 2026-08-10 | open |
| 2 | `DelayConfig::to_duration()` multiplies the config-supplied `duration: u64` by a | bug | tier1d-dev-review segment=api-core | 2026-08-10 | open |
| 2 | `send_fcm` (FCM HTTP v1) only flags a device token for eviction when the upstrea | bug | tier1d-dev-review segment=api-core | 2026-08-10 | open |
| 2 | `handle_idempotent_request` opens a DB transaction `pool.begin()` (line 95), tak | bug | tier1d-dev-review segment=api-core | 2026-08-10 | open |
| 2 | backend/crates/db/src/repositories/llm_document.rs still exposes SIX public un-t | security | tier1d-dev-review segment=api-core | 2026-08-10 | open |
| 2 | `require_permission` derives the caller role with `request.extensions().get::<Te | bug | tier1d-dev-review segment=api-core | 2026-08-10 | open |
| 2 | the module's own header documents it as `# DEPRECATED` / 'fundamentally flawed a | security | tier1d-dev-review segment=api-core | 2026-08-10 | dropped |
| 2 | `send_vote_reminders()` selects every `status='active'` vote with `end_at <= now | bug | tier1d-dev-review segment=api-core | 2026-08-10 | open |
| 2 | evaluate_conditions() FAILS OPEN on an unparseable condition. When a stored cond | bug | tier1d-dev-review segment=api-core | 2026-08-10 | open |
| 2 | evaluate_condition_group() for LogicalOperator::Not silently evaluates only ONE  | bug | tier1d-dev-review segment=api-core | 2026-08-10 | open |
| 2 | after an expensive LLM summarization call, the handler persists the result with  | bug | tier1d-dev-review segment=api-handlers | 2026-08-10 | open |
| 2 | the listing-detail handler (returning ListingWithDetails, TenantExtractor-scoped | bug | tier1d-dev-review segment=api-handlers | 2026-08-10 | open |
| 2 | four report handlers swallow repository DB errors with `.unwrap_or_default()` an | bug | tier1d-dev-review segment=api-handlers | 2026-08-10 | open |
| 2 | in the data-residency dashboard handler, the top-level `compliance_status` field | bug | tier1d-dev-review segment=api-handlers | 2026-08-10 | open |
| 2 | the workflow-template import handler adds each imported action inside a loop wit | bug | tier1d-dev-review segment=api-handlers | 2026-08-10 | open |
| 2 | Systemic across the commonMain repository layer: every repository that wraps a s | bug | tier1d-dev-review segment=mobile-native-kmp | 2026-08-10 | open |
| 2 | CreateListing route wires onSubmit = { _ -> Result.failure(NotImplementedError(" | bug | tier1d-dev-review segment=mobile-native-kmp | 2026-08-10 | dropped |
| 2 | the composable(Screen.CreateListing.route) destination wires the screen's onSubm | bug | tier1d-dev-review segment=mobile-native-kmp | 2026-08-10 | dropped |
| 2 | the CreateListing route wires the fully-built CreateListingScreen with `onSubmit | bug | tier1d-dev-review segment=mobile-native-kmp | 2026-08-10 | dropped |
| 2 | DecimalAsLongSerializer.deserialize() does `raw.substringBefore('.').toLong()` o | bug | tier1d-dev-review segment=mobile-native-kmp | 2026-08-10 | open |
| 2 | four inline LaunchedEffect data loaders collapse ALL failures into an empty coll | bug | tier1d-dev-review segment=mobile-native-kmp | 2026-08-10 | open |
| 2 | `validateAccess` decides credential expiry (`new Date(credential.validUntil) < n | bug | tier1d-dev-review segment=mobile-rn | 2026-08-10 | open |
| 2 | frontend/apps/mobile/src/qrcode/QRCodeScanner.ts (385 lines, exports parseQRCode | test | tier1d-dev-review segment=mobile-rn | 2026-08-10 | open |
| 2 | the central `apiRequest` helper (backing every useQuery/useMutation in the app)  | bug | tier1d-dev-review segment=mobile-rn | 2026-08-10 | open |
| 2 | parseApiError() runs isNetworkError(error) as its FIRST branch, before the struc | bug | tier1d-dev-review segment=ppt-web-core | 2026-08-10 | open |
| 2 | the effect cleanup ONLY disconnects the PerformanceObservers (`for (const observ | bug | tier1d-dev-review segment=ppt-web-core | 2026-08-10 | open |
| 2 | confirmDelete/handleToggle/handleRun each `await <mutation>.mutateAsync(...)` wi | bug | tier1d-dev-review segment=ppt-web-ui | 2026-08-10 | open |
| 2 | the editable Budget Categories list is rendered with `categories.map((category,  | bug | tier1d-dev-review segment=ppt-web-ui | 2026-08-10 | open |
| 2 | the number-typed value input does `onChange={(e) => updateCondition(index, { val | bug | tier1d-dev-review segment=ppt-web-ui | 2026-08-10 | open |
| 2 | FinancialDashboardPageRoute issues three useQuery calls (getARAgingReport, getOv | bug | tier1d-dev-review segment=ppt-web-ui | 2026-08-10 | open |
| 2 | free-form window.prompt() text is cast straight to a typed enum with no validati | bug | tier1d-dev-review segment=ppt-web-ui | 2026-08-10 | open |
| 2 | handleTakeAction reads a free-text moderation action from `window.prompt('Enter  | bug | tier1d-dev-review segment=ppt-web-ui | 2026-08-10 | open |
| 2 | handleChange does `[name]: type === 'number' ? Number(value) : value \|\| undefine | bug | tier1d-dev-review segment=ppt-web-ui | 2026-08-10 | open |
| 2 | the entire shipped manager-facing dashboard renders from hardcoded module-level  | completeness | tier1d-dev-review segment=ppt-web-ui | 2026-08-10 | open |
| 2 | list_import_jobs (wired GET /api/v1/imports/jobs, PortalPrincipal-authenticated) | bug | tier1d-dev-review segment=reality-server | 2026-08-10 | open |
| 2 | UserHandler::link_pm_account() performs its guard checks (already-linked at :360 | bug | tier1d-dev-review segment=reality-server | 2026-08-10 | open |
| 2 | clamp_pagination() is documented as the single guard that keeps raw `page`/`limi | bug | tier1d-dev-review segment=reality-server | 2026-08-10 | open |
| 2 | get_price_map is PUBLIC and unauthenticated (handler takes only State + Query, n | bug | tier1d-dev-review segment=reality-server | 2026-08-10 | open |
| 2 | request_password_reset() generates and stores a reset token (handler.request_pas | bug | tier1d-dev-review segment=reality-server | 2026-08-10 | open |
| 2 | in the per-search scan loop, once new match ids are found the code enqueues the  | bug | tier1d-dev-review segment=reality-server | 2026-08-10 | open |
| 2 | TokenValidationCache::set() stores every introspection result with `expires_at:  | security | tier1d-dev-review segment=reality-server | 2026-08-10 | open |
| 2 | handleExportPDF() is a shipped no-op stub: its entire body is a comment `// In a | bug | tier1d-dev-review segment=reality-web | 2026-08-10 | open |
| 2 | frontend/apps/reality-web/src/app/[locale]/journal/page.tsx:11 imports `EDITORS_ | completeness | tier1d-dev-review segment=reality-web | 2026-08-10 | open |
| 2 | backend/servers/api-server/src/routes/regional_compliance.rs:253-259 (validate_s | bug | tier1d-dev-review segment=api-handlers | 2026-08-10 | open |
| 2 | each SSO OAuth helper constructs a fresh `reqwest::Client::new()` pe | bug | tier1d-dev-review segment=reality-server | 2026-08-10 | dropped |
| 2 | introspect_pm_token() caches active=false for the full 60s TTL on ANY non-succes | bug | tier1d-dev-review segment=reality-server | 2026-08-10 | dropped |
| 1 | Issue #836 (no labels, OPEN): Code review: Epic 2B-C — Mobile push & device registration (origin/dev) | triage | #836 | 2026-05-31 | done |
| 1 | Issue #845 (no labels, OPEN): Code review: Epic 14 — IoT alerts, correlations, thresholds (origin/dev) | triage | #845 | 2026-05-31 | done |
| 1 | Issue #849 (no labels, OPEN): Code review: Epic 10B+143 — Admin impersonation, Help, Board meetings auth (origin/dev) | triage | #849 | 2026-05-31 | done |
| 1 | Issue #750 (no labels, OPEN): Current dev review: backend/API/database findings | triage | #750 | 2026-06-01 | done |
| 1 | Issue #753 (no labels, OPEN): Current dev review: Epic 6 Announcements & Communication | triage | #753 | 2026-06-01 | done |
| 1 | Issue #754 (no labels, OPEN): Current dev review: Epic 7A Basic Document Management | triage | #754 | 2026-06-01 | done |
| 1 | Issue #757 (no labels, OPEN): Current dev review: Epic 10B Platform Administration | triage | #757 | 2026-06-01 | done |
| 1 | Issue #760 (no labels, OPEN): Current dev review: Epic 79 Disputes & Mediation | triage | #760 | 2026-06-01 | done |
| 1 | Issue #762 (no labels, OPEN): Current dev review: Reports & Schedules | triage | #762 | 2026-06-01 | done |
| 1 | Issue #766 (no labels, OPEN): Current dev review: AI & LLM routes | triage | #766 | 2026-06-01 | done |
| 1 | Issue #770 (no labels, OPEN): Current dev review: Faults & triage | triage | #770 | 2026-06-01 | done |
| 1 | Issue #771 (no labels, OPEN): Current dev review: Research dispatcher & CI automation | triage | #771 | 2026-06-01 | done |
| 1 | Issue #772 (no labels, OPEN): Current dev review: Auth core (delta confirmation) | triage | #772 | 2026-06-01 | done |
| 1 | Issue #773 (no labels, OPEN): Current dev review: Leases & rental | triage | #773 | 2026-06-01 | done |
| 1 | Issue #774 (no labels, OPEN): Current dev review: Reality server (broad) | triage | #774 | 2026-06-01 | done |
| 1 | Issue #775 (no labels, OPEN): Current dev review: WebSocket realtime | triage | #775 | 2026-06-01 | done |
| 1 | Issue #776 (no labels, OPEN): Current dev review: Equipment & audit log | triage | #776 | 2026-06-01 | done |
| 1 | Issue #777 (no labels, OPEN): Current dev review: Compliance & GDPR | triage | #777 | 2026-06-01 | done |
| 1 | Issue #778 (no labels, OPEN): Current dev review: Marketplace, voting, investor portal, impersonation | triage | #778 | 2026-06-01 | done |
| 1 | Issue #788 (no labels, OPEN): Dev review rounds 1-5: mobile-native + ppt-web surfaces | triage | #788 | 2026-06-01 | done |
| 1 | Issue #790 (no labels, OPEN): Dev review rounds 11-15: vendor, predictive, reality-web, middleware | triage | #790 | 2026-06-01 | done |
| 1 | Issue #791 (no labels, OPEN): Dev review rounds 16-20: push, e-sign, portal, webhooks, reserves | triage | #791 | 2026-06-01 | done |
| 1 | Issue #846 (no labels, OPEN): Code review: Epics 12+65 — Meters & Energy/ESG (origin/dev) | triage | #846 | 2026-06-01 | done |
| 1 | Issue #847 (no labels, OPEN): Code review: Reality-server — Inquiries IDOR (Epics 16–19) (origin/dev) | triage | #847 | 2026-06-01 | done |
| 1 | Issue #848 (no labels, OPEN): Code review: Epics 78+134 — Vendor portal stubs & Predictive maintenance gaps (origin/dev) | triage | #848 | 2026-06-01 | done |
| 1 | Issue #850 (no labels, OPEN): Code review: Epics 61+146+42 — Multi-currency, Data residency, Violations (origin/dev) | triage | #850 | 2026-06-01 | done |
| 1 | Issue #851 (no labels, OPEN): Code review: Epics 15+105+69 — Listings/syndication & Developer API stubs (origin/dev) | triage | #851 | 2026-06-01 | done |
| 1 | Issue #859 (no labels, OPEN): sqlx 0.9 breaks runtime decode of Postgres enum columns into Rust String (SELECT * reads 5 | triage | #859 | 2026-06-01 | done |
| 1 | Issue #867 (no labels, OPEN): Tech debt: api-server main.rs duplicates lib.rs::create_router — routers diverge silently | triage | #867 | 2026-06-01 | done |
| 1 | Issue #751 (no labels, OPEN): Current dev review: frontend/web/API-client findings | triage | #751 | 2026-06-02 | done |
| 1 | Issue #752 (no labels, OPEN): Current dev review: mobile CI tooling findings | triage | #752 | 2026-06-02 | done |
| 1 | Issue #756 (no labels, OPEN): Current dev review: Epic 10A OAuth Provider | triage | #756 | 2026-06-02 | done |
| 1 | Issue #761 (no labels, OPEN): Current dev review: Epic 84 E-Signature & Leases | triage | #761 | 2026-06-02 | done |
| 1 | Issue #763 (no labels, OPEN): Current dev review: Reality Server & Inquiries | triage | #763 | 2026-06-02 | done |
| 1 | Issue #767 (no labels, OPEN): Current dev review: Mobile RN Property Management app | triage | #767 | 2026-06-02 | done |
| 1 | Issue #768 (no labels, OPEN): Current dev review: Admin-web features (10B) | triage | #768 | 2026-06-02 | done |
| 1 | Issue #920 (no labels, OPEN): Announcement targeting not enforced on read (intra-org disclosure) | triage | #920 | 2026-06-02 | done |
| 1 | PR #908 (fix(security): require PKCE on OAuth authorization-code flow, closes #823) was closed unmerged — verify whether | security | PR #908 | 2026-06-03 | done |
| 1 | Mobile VotingScreen hardcodes en-US in toLocaleDateString — vote dates never localize | bug | PR #1083 | 2026-06-05 | done |
| 1 | Reality-web listing generateMetadata can throw during SSR on malformed 200 body | bug | PR #1085 | 2026-06-05 | done |
| 1 | docker/nginx admin-web + ppt-web templates churned twice this run (security headers + redirects) | dx | PR #963 | 2026-06-06 | done |
| 1 | ai.rs churn-hot — 3,142 lines this run; 3,142-line route monolith, candidate for module split | refactor | git log origin/dev since 2026-05-24 | 2026-06-06 | done |
| 1 | ppt-web e2e auth-refresh.spec.ts added (+252 lines, story 79-2 token-refresh coverage) | refactor | PR #1047 | 2026-06-06 | done |
| 1 | api-server esignature_webhook_idempotency_tests.rs added (+228 lines, terminal-state regression) | refactor | PR #1034 | 2026-06-06 | done |
| 1 | ppt-web EvidenceUploader.test.tsx added (+202 lines, dispute-filing AC-2 regression) | refactor | PR #1048 | 2026-06-06 | done |
| 1 | api-server main.rs touched twice this run (gap-sweep + security headers) — minor churn marker | refactor | PR #989 | 2026-06-06 | done |
| 1 | Duplicated animate-spin spinner markup across mediation page + chat thread (no shared Spinner) | refactor | PR #555 | 2026-06-06 | done |
| 1 | Mediation reference number uppercases full UUID (DSP-<uuid>) instead of a short code | refactor | PR #555 | 2026-06-06 | done |
| 1 | frontend/apps/mobile/src/App.tsx churned twice this run (universal links + doc-detail wiring) | refactor | PR #962 | 2026-06-06 | done |
| 1 | platform_admin.rs churn-hot — 2,762 lines this run (admin/OAuth-provider feature work) | refactor | git log origin/dev since 2026-05-24 | 2026-06-06 | done |
| 1 | Reality-web ComparisonUrlHandler hardcodes English loading/error strings | refactor | code-review reality-web 2026-05-28 | 2026-06-06 | done |
| 1 | Watch routes/oauth.rs churn after audit-log + hardening PRs | refactor | PR #930 | 2026-06-06 | done |
| 1 | Watch services/oauth.rs churn after introspect/revoke hardening (#933) | refactor | PR #933 | 2026-06-06 | done |
| 1 | Mobile VotingScreen pure transforms toUiStatus/toUiVote have no tests | test-gap | code-review mobile-rn 2026-05-27 | 2026-06-06 | done |
| 1 | Issue #749 (no labels, OPEN): Code review findings: Story 6.1 announcement creation and targeting | triage | #749 | 2026-06-06 | done |
| 1 | Issue #755 (no labels, OPEN): Current dev review: Epic 8A Notification Preferences | triage | #755 | 2026-06-06 | done |
| 1 | Issue #764 (no labels, OPEN): Current dev review: Admin MFA & Auth Hardening | triage | #764 | 2026-06-06 | done |
| 1 | Issue #765 (no labels, OPEN): Current dev review: Integrations & Airbnb OAuth | triage | #765 | 2026-06-06 | done |
| 1 | PKCE unit test became a tautology after services/oauth.rs DRY refactor (#1132) | test-gap | #1137 | 2026-06-07 | done |
| 1 | Triage: dispatcher incident — assignments-archive.json corrupted to 1/196 rows on dev branch (#1061) | triage | Issue #1061 | 2026-06-07 | done |
| 1 | Issue #769 (no labels, OPEN): Current dev review: Deploy server | triage | #769 | 2026-06-07 | done |
| 1 | Issue #789 (no labels, OPEN): Dev review rounds 6-10: scheduler, notifications, admin, orgs, buildings | triage | #789 | 2026-06-07 | done |
| 1 | Issue #950 (no labels, OPEN): CI: trigger-deploy 403 marks all dev image builds red and blocks staging auto-deploy | triage | #950 | 2026-06-07 | done |
| 1 | Issue #952 (no labels, OPEN): [staging] Reality SSO login dead-ends: redirect_uri callback 404s on reality apex | triage | #952 | 2026-06-07 | done |
| 1 | PR #1378 closed without merge — DROP-OWNED-BY teardown theory for #1332 was wrong root cause, superseded by #1379 | refactor | PR #1378 | 2026-06-15 | done |
| 1 | iOS deep-link layer dead at runtime — Info.plist missing CFBundleURLTypes + applinks entitlement | bug | issue #1267 | 2026-07-05 | dropped |
| 1 | Risky churn: api-server main.rs security-headers wiring shipped without a middleware smoke test | bug | PR #963 | 2026-07-05 | dropped |
| 1 | Churn hotspot: ListingDetailScreen.kt — +1279 LOC this run (gap-82-4 reality mobile favorite toggle) | refactor | PR #1121 | 2026-07-05 | done |
| 1 | crypto.rs:127 SysRng.try_fill_bytes(...).expect() panics if OS CSPRNG errors during integration-credential encrypt | refactor | code-review api-core 2026-06-15 | 2026-07-05 | done |
| 1 | DeepLinkRouter skips URL-decoding while Android Uri.getQueryParameter decodes — SSO tokens diverge per platform | bug | mobile-native-kmp segment review 2026-06-06 | 2026-07-05 | dropped |
| 1 | SearchScreen stale-response race — overlapping searches can clobber newer results | bug | mobile-native-kmp segment review 2026-06-06 | 2026-07-05 | dropped |
| 1 | useDeepLinkRouting.ts:27-36 — initialize() re-runs on onNavigate identity change + void promise with no .catch → duplica | bug | Phase 1.5 review of mobile-rn segment (2026-06-16) | 2026-07-05 | dropped |
| 1 | Mobile RN production screens (Buildings/Meters/Leases/PersonMonths/Notifications/Threads/Forms) render hardcoded MOCK_*  | bug | Phase 1.5 review of mobile-rn segment (2026-06-16) | 2026-07-05 | done |
| 1 | Churn hotspot: backend/crates/db/src/models/mod.rs (12 commits in 19-day catch-up) | refactor | churn since 4829015b: 12 commits | 2026-07-05 | dropped |
| 1 | Churn hotspot: backend/crates/db/src/repositories/rental.rs (11 commits in 19-day catch-up) | refactor | churn since 4829015b: 11 commits | 2026-07-05 | done |
| 1 | Churn hotspot: DocumentsScreen.tsx — 3 PRs this run | refactor | PR #1101 | 2026-07-05 | done |
| 1 | AI llm/sessions + integrations sync + subscriptions RLS migration (PR #1287, PAP-169) shipped without a new regression t | test-gap | PR #1287 | 2026-07-05 | dropped |
| 1 | Webhook handlers RLS migration (PR #1288, PAP-170) shipped without a new regression test for repo-layer methods | test-gap | PR #1288 | 2026-07-05 | dropped |
| 1 | api_ecosystem.rs RLS migration (PR #1289, PAP-167) — 162-line handler rework shipped without a regression test for the p | test-gap | PR #1289 | 2026-07-05 | dropped |
| 1 | mfa.rs RLS migration (PR #1292, PAP-168) shipped without a regression test; also landed broken and was hotfixed in PR #1 | test-gap | PR #1292 | 2026-07-05 | dropped |
| 1 | Screen-map drift: PR #1085 modified reality-web listing detail metadata + page without screen-doc update | test-gap | PR #1085 | 2026-07-05 | dropped |
| 1 | Screen-map drift: PR #1100 modified ppt-web App.tsx (FileDisputePageRoute extraction) without screen-doc update | test-gap | PR #1100 | 2026-07-05 | dropped |
| 1 | Screen-map drift: PR #922 modified ppt-web App.tsx (dev-review rounds 1-5 fixes) without a docs/screens/ppt update | test-gap | PR #922 | 2026-07-05 | done |
| 1 | /refresh and /logout — empty refresh_token cookie shadows valid body token | bug | Phase 1.5 code-review 2026-07-09 (api-handlers segment) | 2026-07-09 | done |
| 1 | api-server routes/auth.rs — repeated hotspot + 3 static-review findings this run | refactor | commits 2026-07-05..2026-07-09 | 2026-07-09 | done |
| 1 | backend integrations booking/mod.rs — instability watch after PR #2176 split | refactor | commits 2026-07-05..2026-07-09 | 2026-07-09 | done |
| 1 | oauth_integration_tests.rs repeated-churn (runs_seen 2→3) — OAuth handlers still moving | test-gap | commits 2026-07-05..2026-07-09 | 2026-07-09 | dropped |
| 1 | Churn hotspot: frontend/apps/ppt-web/messages/en.json — frontend/apps/ppt-web/messages/en.json: +3926 lines this run (ru | refactor | git-log-since-2026-07-13 | 2026-07-16 | done |
| 1 | Churn hotspot: frontend/packages/sitemap/src/json/sitemap.json — frontend/packages/sitemap/src/json/sitemap.json: +3727  | refactor | git-log-since-2026-07-13 | 2026-07-16 | done |
| 1 | scheduler.rs get_announcement_target_users() silently swallows target_ids JSON parse errors — malformed payload publishe | bug | code-review api-core 2026-07-20 | 2026-07-20 | done |
| 1 | PR #2385 (dependabot: dtolnay/rust-toolchain 1.94.1 → 1.100.0) closed unmerged — likely superseded by unrelated toolchai | dx | PR #2385 | 2026-07-20 | dropped |
| 1 | PR #2387 (dependabot: npm-minor-patch 15-update rollup) closed unmerged — superseded by the 19-update rollup #2423 | dx | PR #2387 | 2026-07-20 | dropped |
| 1 | Churn hotspot: backend/Cargo.toml — 3 touches this window (dependabot minor-patch cascade + layout-core crate) | refactor | git-log-since-2026-07-16 | 2026-07-20 | done |
| 1 | Churn hotspot: frontend/apps/mobile/package.json — 5 touches this window (Expo/expo-notifications/expo-config-plugins de | refactor | git-log-since-2026-07-16 | 2026-07-20 | done |
| 1 | Churn hotspot: frontend/apps/admin-web/src/features/layout-editor/LayoutEditorPage.tsx — 2 touches, 900 lines this run | refactor | git-log-since-2026-07-20T03:12:00Z | 2026-07-21 | done |
| 1 | Churn hotspot: docs/screens/ppt/dashboard.md — 3 touches this run (Layout & Content Manager pilot integration) | refactor | git-log-since-2026-07-20T03:12:00Z | 2026-07-21 | done |
| 1 | Churn hotspot: docs/repo-map.md — 4 touches this window (per-PR route-map refresh) | refactor | git-log-since-2026-07-16 | 2026-07-21 | done |
| 1 | Churn hotspot: infra_ops_authz_backfill_tests.rs — 364 lines this run (BIT-268 test backfill) | refactor | git-log-since-2026-07-21T03:13:00Z | 2026-07-23 | dropped |
| 1 | Churn hotspot: org_property_authz_backfill_tests.rs — 412 lines this run (BIT-268/BIT-559 authz salvage) | refactor | git-log-since-2026-07-21T03:13:00Z | 2026-07-23 | done |
| 1 | Churn hotspot: platform_admin_authz_batch2_tests.rs — 417 lines this run (BIT-557 test backfill) | refactor | git-log-since-2026-07-21T03:13:00Z | 2026-07-23 | done |
| 1 | PR #2489 closed unmerged: dependabot npm-minor-patch (5→4 update group) superseded by #2491 | triage | PR #2489 | 2026-07-23 | dropped |
| 1 | 10 ungated console.warn/error in ppt-web websocket.ts leak diagnostics in prod | refactor | Phase 1.5 rotating expert review 2026-07-24 (ppt-web-core segment) | 2026-07-24 | dropped |
| 1 | PortfolioAnalytics inquiriesTrend silently drops days with inquiries but zero views (set-difference bug) | bug | Phase 1.5 rotating expert review 2026-07-27 (mobile-native-kmp segment) | 2026-07-27 | done |
| 1 | Churn hotspot: backend/crates/integrations/src/booking/mod.rs — 3185 lines this window (post-PR-#2176 tail) | refactor | Phase 1 churn scan 2026-07-27 (commit range 2026-07-24..2026-07-27 on dev) | 2026-07-27 | done |
| 1 | reality-web listingAnalytics.ts casts untrusted ?source= query param straight to ViewSource union — pollutes listing.vie | refactor | rotating-expert-review 2026-07-28 | 2026-07-28 | done |
| 1 | Stale TODO(security) headers in faults.rs / critical_notifications.rs describe a hardcoded-false gate that no longer exi | dx | standing-scan-2026-07-28 | 2026-07-28 | done |
| 1 | layout/admin.rs (+ tenant.rs) mutation handlers end with unwrap_or_default() serialize — a failed serialize returns 200  | bug | rotating-expert-review api-core 2026-07-30 | 2026-07-30 | done |
| 1 | scheduler.rs silently swallows DB errors on notification target lookups at 3 sites — failed dispatches show as empty tar | bug | rotating-expert-review api-core 2026-07-30 | 2026-07-30 | done |
| 1 | Churn hotspot: backend/servers/api-server/src/routes/layout/admin.rs — 240 lines this window (PRs #2478, #2549) | refactor | PR #2478 | 2026-07-30 | dropped |
| 1 | Churn hotspot: backend/servers/api-server/src/routes/layout/tenant.rs — 262 lines this window (PR #2478) | refactor | PR #2478 | 2026-07-30 | dropped |
| 1 | Churn hotspot: backend/servers/api-server/src/services/scheduler.rs — 347 lines this window (PRs #2567, #2576) | refactor | PR #2567 | 2026-07-30 | done |
| 1 | reality-server password reset is non-functional in prod — token discarded, no email transport, endpoint claims a link wa | bug | rotating-expert-review reality-server 2026-08-01 | 2026-08-01 | dropped |
| 1 | mobile: empty/error/loading state strings hardcoded in English across ~18 screens despite react-i18next being wired | bug | rotating-expert-review (tier1d mobile-rn) | 2026-08-03 | done |
| 1 | mobile: usePushNotifications — debug console.log left on the device-token register/unregister production path | bug | rotating-expert-review (tier1d mobile-rn) | 2026-08-03 | done |
| 1 | ppt-web route wrappers render 8 hardcoded English 'X not found' fallbacks — sk/cs/de users see untranslated text | bug | rotating-expert-review (tier1d ppt-web-core) | 2026-08-03 | done |
| 1 | ppt-web: 103 hardcoded English aria-label attributes across feature components — screen readers hear English on sk/cs/de | bug | rotating-expert-review (tier1d ppt-web-ui) | 2026-08-03 | done |
| 1 | SessionsPage double-casts through unknown to hand-maintained interface — defeats API-boundary type-check | bug | rotating-expert-review (tier1d ppt-web-ui) | 2026-08-03 | done |
| 1 | reality-web: agency/import feature cluster hardcoded English — 65 sibling components use useTranslations | bug | rotating-expert-review (tier1d reality-web) | 2026-08-03 | done |
| 1 | reality-web ProtectedRoute renders untranslated auth-required gate for sk/cs/de | bug | rotating-expert-review (tier1d reality-web) | 2026-08-03 | done |
| 1 | Churn hotspot: 2 commits touching frontend/apps/mobile/src/hooks/useOfflineSupport.ts (window 2026-08-03T17:00Z → 2026-0 | refactor | local git log since 2026-08-03T17:00Z | 2026-08-04 | dropped |
| 1 | Churn hotspot: 2 commits touching frontend/apps/mobile/src/screens/messages/ThreadDetailScreen.tsx (window 2026-08-03T17 | refactor | local git log since 2026-08-03T17:00Z | 2026-08-04 | done |
| 1 | Churn hotspot: 2 commits touching frontend/apps/ppt-web/src/routes/groups/rentals.tsx (window 2026-08-03T17:00Z → 2026-0 | refactor | local git log since 2026-08-03T17:00Z | 2026-08-04 | dropped |
| 1 | mobile-native-kmp: getPortfolioAnalytics() fans out one analytics HTTP request per listing with no concurrency limit — u | bug | Tier1d review 2026-08-04 (mobile-native-kmp) | 2026-08-04 | dropped |
| 1 | api-handlers: the raw upstream LLM-provider error is forwarded verbatim into the client-f... | security | tier1d-dispatcher-generator | 2026-08-06 | done |
| 1 | mobile-rn: enableBiometric passes hardcoded English strings to the OS biometric dialog... | bug | tier1d-dispatcher-generator | 2026-08-06 | done |
| 1 | reality-server: a shipped-but-non-functional notification path. | bug | tier1d-dispatcher-generator | 2026-08-06 | done |
| 1 | reality-server: ListingHandler::get_listing() returns a PublicListingDetail with a whole bl... | bug | tier1d-dispatcher-generator | 2026-08-06 | done |
| 1 | reality-server: schedule_viewing() runs full input validation (future-date at :300, <=90-da... | bug | tier1d-dispatcher-generator | 2026-08-06 | done |
| 1 | Churn hotspot: backend/servers/api-server/src/routes/ai/llm.rs (fail-closed enhanced_chat #2688 + upstream error masking | refactor | PR #2688 | 2026-08-07 | done |
| 1 | Churn hotspot: backend/servers/api-server/src/services/workflow_executor.rs (+265/-192 in #2685 workflow NOT-group fix) | refactor | PR #2685 | 2026-08-07 | done |
| 1 | Churn hotspot: frontend/apps/ppt-web/src/lib/websocket.ts (+11/-94 heartbeat removal PR #2689) | refactor | PR #2689 | 2026-08-07 | dropped |
| 1 | reality-web ListingForm hardcodes English throughout a next-intl sk/cs/de/en app | bug | Tier1d review 2026-08-07 (reality-web) | 2026-08-07 | done |
| 1 | backend/servers/api-server/src/services/accounting.rs (339 lines) has NO `#[cfg( | test-gap | tier1d-dev-review segment=api-core | 2026-08-10 | open |
| 1 | a Variable-Symbol match alone contributes 0.8 and crosses the 0.5 acceptance thr | bug | tier1d-dev-review segment=api-core | 2026-08-10 | open |
| 1 | in `get_listing` (the tenant-scoped GET /api/v1/listings/{id} detail handler, li | bug | tier1d-dev-review segment=api-core | 2026-08-10 | open |
| 1 | `verify_pkce` compares the recomputed S256 challenge to the stored `challenge` w | security | tier1d-dev-review segment=api-core | 2026-08-10 | open |
| 1 | `.expect("reqwest client build should not fail")` and push_fanout.rs:773 — `.exp | bug | tier1d-dev-review segment=api-core | 2026-08-10 | open |
| 1 | the user quiet-hours lookup swallows repository errors: `self.granular_repo.get_ | bug | tier1d-dev-review segment=api-core | 2026-08-10 | open |
| 1 | `process_syndication_job` never calls any portal API yet writes `syndication_sta | bug | tier1d-dev-review segment=api-core | 2026-08-10 | open |
| 1 | `tenant_filter` is named and documented ('ensure tenant isolation') as an isolat | security | tier1d-dev-review segment=api-core | 2026-08-10 | open |
| 1 | `action_check_announcements` unconditionally returns `success: true`, 'You have  | bug | tier1d-dev-review segment=api-core | 2026-08-10 | dropped |
| 1 | the ComparisonOperator::Matches arm calls `regex::Regex::new(pattern)` (line 618 | dx | tier1d-dev-review segment=api-core | 2026-08-10 | open |
| 1 | in `list_audit_logs_impl`, the audit-chain tamper-evidence flag is read as `let  | bug | tier1d-dev-review segment=api-handlers | 2026-08-10 | open |
| 1 | class ApiClient is never instantiated anywhere in production code. grep '\bApiCl | cleanup | tier1d-dev-review segment=mobile-native-kmp | 2026-08-10 | open |
| 1 | Compose screens repeatedly re-assert nullable UI-state fields with `!!` immediat | cleanup | tier1d-dev-review segment=mobile-native-kmp | 2026-08-10 | open |
| 1 | the wire-contract guard for DecimalAsLong/DecimalAsDoubleSerializer covers only  | test | tier1d-dev-review segment=mobile-native-kmp | 2026-08-10 | open |
| 1 | shared/src/commonMain/kotlin/three/two/bit/ppt/reality/layout/LayoutRepository.k | cleanup | tier1d-dev-review segment=mobile-native-kmp | 2026-08-10 | open |
| 1 | MyListings loads via portalListingsRepository.listMyListings(limit = 100) with n | bug | tier1d-dev-review segment=mobile-native-kmp | 2026-08-10 | open |
| 1 | mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/realtor/Por | bug | tier1d-dev-review segment=mobile-native-kmp | 2026-08-10 | open |
| 1 | 15 of 41 screens under frontend/apps/mobile/src/screens render hardcoded English | bug | tier1d-dev-review segment=mobile-rn | 2026-08-10 | open |
| 1 | `find frontend/apps/mobile -name '*.test.*'` returns no nfc/ entry, a | bug | tier1d-dev-review segment=mobile-rn | 2026-08-10 | open |
| 1 | frontend/apps/mobile/src/nfc/NFCAccessController.ts (478 lines, class NFCAccessC | test | tier1d-dev-review segment=mobile-rn | 2026-08-10 | open |
| 1 | the subscribe effect early-returns when `!eventType \|\| !handlerRef.current` and  | bug | tier1d-dev-review segment=ppt-web-core | 2026-08-10 | open |
| 1 | Committed production feature code left with raw `console.error(...)` calls that  | bug | tier1d-dev-review segment=ppt-web-ui | 2026-08-10 | open |
| 1 | Numerous user-facing browser-dialog strings are hardcoded English instead of goi | i18n | tier1d-dev-review segment=ppt-web-ui | 2026-08-10 | open |
| 1 | All four app-wide terminal error screens render hardcoded English with no useTra | bug | tier1d-dev-review segment=ppt-web-ui | 2026-08-10 | open |
| 1 | ppt-web is a react-i18next app shipping sk/cs/de/en, yet several shipped feature | bug | tier1d-dev-review segment=ppt-web-ui | 2026-08-10 | open |
| 1 | The financial feature is almost entirely un-internationalized: 21 of 24 .tsx und | bug | tier1d-dev-review segment=ppt-web-ui | 2026-08-10 | open |
| 1 | the meter-reading photo thumbnail opens the stored image via `window.open(readin | bug | tier1d-dev-review segment=ppt-web-ui | 2026-08-10 | open |
| 1 | only 3 of 15 .tsx files call useTranslation (RuleCard, RuleBuilder, TemplatePrev | chore | tier1d-dev-review segment=ppt-web-ui | 2026-08-10 | open |
| 1 | the inquiry-detail handler (GET inquiry for realtor; ownership verified at :575- | bug | tier1d-dev-review segment=reality-server | 2026-08-10 | open |
| 1 | TokenValidationCache::set(), when the cache is full after expired-entry eviction | bug | tier1d-dev-review segment=reality-server | 2026-08-10 | open |
| 1 | `SectionBoundary` (a minimal class error boundary wrapping every listing-detail  | bug | tier1d-dev-review segment=reality-web | 2026-08-10 | open |
| 1 | this is a `[locale]`-routed page in a next-intl app (sk/cs/de/en) yet imports NO | bug | tier1d-dev-review segment=reality-web | 2026-08-10 | open |
| 1 | a `[locale]`-routed page that DOES call `useTranslations('pages.priceMap')` (:40 | bug | tier1d-dev-review segment=reality-web | 2026-08-10 | open |
| 0 | Portfolio dashboard: alert mark-read/resolve mutations + property-card click navigation are no-op stubs | dx | PR #328 | 2026-06-04 | dropped |
| 0 | Announcer: untracked clear-then-set timeouts can resurrect a stale screen-reader message | bug | code-review ppt-web-ui 2026-05-24 | 2026-06-07 | dropped |
| 0 | Issue #779 (no labels, OPEN): Current dev review: consolidated priority rollup (origin/dev snapshot) | triage | #779 | 2026-06-13 | dropped |
| 0 | Stabilize oauth_integration_tests churn — heavy edits across 3 OAuth fix PRs | refactor | PR #930 | 2026-06-16 | dropped |
| 0 | Churn hotspot: 2940 lines changed in backend/crates/db/src/repositories/document.rs (window 2026-06-10 03:05Z→18:30Z) | refactor | local git numstat since 2026-06-10T03:05:00Z | 2026-07-05 | dropped |
| 0 | Churn hotspot: 53 lines in backend/crates/db/src/repositories/form.rs (PR #1379 #1332 unblock) | refactor | PR #1379 | 2026-07-05 | dropped |
| 0 | Churn hotspot: backend/crates/db/src/repositories/reality_portal.rs (+59/−28 in PR #1297 PAP-142 IDOR scoping) | refactor | PR #1297 commit 8c711c6 | 2026-07-05 | dropped |
| 0 | Churn hotspot: backend/crates/db/src/repositories/sensor.rs (+248/-86 in PR #1321/#1322 PAP-151 re-land + fmt) | refactor | PR #1321 commit | 2026-07-05 | dropped |
| 0 | Churn hotspot: 2856 lines changed in backend/crates/db/src/repositories/subscription.rs (window 2026-06-10 03:05Z→18:30Z | refactor | local git numstat since 2026-06-10T03:05:00Z | 2026-07-05 | dropped |
| 0 | Churn hotspot: backend/crates/db/tests/form_rls_repo_tests.rs touched 2x since 2026-06-12 (window 2026-06-12→2026-06-13) | refactor | local git numstat since 2026-06-12 | 2026-07-05 | dropped |
| 0 | Churn hotspot: backend/crates/integrations/src/booking.rs (+404/−29 in PR #1294 Booking.com OTA retry) | refactor | PR #1294 commit 7ccce8a | 2026-07-05 | dropped |
| 0 | Churn hotspot: 2691 lines changed in backend/servers/api-server/src/routes/aml_dsa.rs (window 2026-06-10 03:05Z→18:30Z) | refactor | local git numstat since 2026-06-10T03:05:00Z | 2026-07-05 | dropped |
| 0 | Churn hotspot: backend/servers/api-server/src/routes/api_ecosystem.rs (+106/−27 in PR #1293 PAP-171; second touch in 24h | refactor | PR #1293 commit 1e50156 | 2026-07-05 | dropped |
| 0 | Churn hotspot: 1021 lines changed in backend/servers/api-server/src/routes/emergency.rs (window 2026 | refactor | local git numstat since 2026-06-07 | 2026-07-05 | dropped |
| 0 | Churn hotspot: 709 lines changed in backend/servers/api-server/src/routes/enhanced_tenant_screening. | refactor | local git numstat since 2026-06-07 | 2026-07-05 | dropped |
| 0 | Churn hotspot: backend/servers/api-server/src/routes/forms.rs touched 2x since 2026-06-12 (window 2026-06-12→2026-06-13) | refactor | local git numstat since 2026-06-12 | 2026-07-05 | dropped |
| 0 | Churn hotspot: backend/servers/api-server/src/routes/iot.rs (+278/-403 in PR #1321/#1322 PAP-151 re-land + fmt) | refactor | PR #1321 commit | 2026-07-05 | dropped |
| 0 | Churn hotspot: backend/servers/api-server/src/routes/reserve_funds.rs (+228/-255 in PR #1321 PAP-151 re-land) | refactor | PR #1321 commit | 2026-07-05 | dropped |
| 0 | Churn hotspot: 929 lines changed in backend/servers/api-server/src/routes/vendors.rs (window 2026-06 | refactor | local git numstat since 2026-06-07 | 2026-07-05 | dropped |
| 0 | booking_oauth_csrf_tests.rs hotspot — 484-line NEW test file (PR #1393 #1424 OAuth CSRF coverage) | refactor | local git numstat since 2026-06-15 (commit 67c24bd..origin/dev) | 2026-07-05 | dropped |
| 0 | booking_oauth_routes_tests.rs hotspot — 381-line NEW test file (PR #1393 OAuth routes coverage) | refactor | local git numstat since 2026-06-15 | 2026-07-05 | dropped |
| 0 | Churn hotspot: backend/servers/api-server/tests/reserve_funds_cross_org_idor_tests.rs touched 2x since 2026-06-12 (windo | refactor | local git numstat since 2026-06-12 | 2026-07-05 | dropped |
| 0 | Churn hotspot: 124 lines in frontend/apps/mobile/app.config.icon.test.ts (PR #1383 gap-85-2) | refactor | PR #1383 | 2026-07-05 | dropped |
| 0 | Churn hotspot: 94 lines in frontend/apps/mobile/app.config.ts (PR #1383 gap-85-2) | refactor | PR #1383 | 2026-07-05 | dropped |
| 0 | Churn hotspot: SearchScreen.kt — +1293 LOC this run (gap-82-3 reality mobile search/filters) | refactor | PR #1125 | 2026-07-05 | dropped |
| 0 | PR #1274 (cargo-minor-patch group, /backend, 9 updates) closed unmerged — superseded by #1313 after auto-rebase fix land | dx | PR #1274 | 2026-07-05 | dropped |
| 0 | PR #1425 (GH #1377 document presigned-URL tests) closed unmerged — superseded by merged #1394 | dx | PR #1425 | 2026-07-05 | dropped |
| 0 | MainActivity reimplements deep-link dispatch instead of calling shared DeepLinkRouter — drift trap | refactor | mobile-native-kmp segment review 2026-06-06 | 2026-07-05 | dropped |
| 0 | PR #1179 (docs(epics) catalog backfill for 37 mounted-but-undocumented backend modules) — stalled at 7d, no reviewDecisi | dx | PR #1179 | 2026-07-05 | dropped |
| 0 | Stalled review: PR #988 (Epic: reusable Playwright E2E framework + sitemap FlowRunner) open 10d, no reviewDecision | dx | PR #988 | 2026-07-05 | dropped |
| 0 | Churn hotspot: AnnouncementsScreen.tsx — 4 PRs this run, instability proxy | refactor | PR #1101 | 2026-07-05 | dropped |
| 0 | Churn hotspot: AnnouncementsScreen.test.ts — 4 PRs this run, instability proxy | refactor | PR #1101 | 2026-07-05 | dropped |
| 0 | forms.rs repeated-churn — runs_seen=2 (#1337 explicit_auto_deref + #1397 org-scope hardening) | refactor | hotspot_history.runs_seen 1→2 with new churn this run | 2026-07-05 | dropped |
| 0 | Dispatcher action-list.json corruption when MCP push falls back from blocked git push | triage | #1014 | 2026-07-05 | dropped |
| 0 | Issue #1151 (no labels, OPEN): Research dispatcher: claimable buffer is stale — true claimable work = 0 despite metric=5 | triage | #1151 | 2026-07-05 | dropped |
| 0 | Issue #1331 (no labels, OPEN): Backend `test` job red/hanging on dev base — blocks the entire backend merge pipeline | triage | #1331 | 2026-07-05 | dropped |
| 0 | Issue #1380 (no labels, OPEN): Dispatcher stale gap-scan buffer + Tier-2 escalation endpoint misconfigured | triage | issue #1380 | 2026-07-05 | dropped |
| 0 | Issue #951 (no labels, OPEN): Deploy blocker: api-server requires ESIGN_TOKEN_SECRET + ESIGN_WEBHOOK_SECRET not injected | triage | #951 | 2026-07-05 | dropped |
| 0 | Cloud routine cadence recovery — reduce 3–4d gaps between runs | dx | routine self-signal 2026-07-09 | 2026-07-09 | dropped |
