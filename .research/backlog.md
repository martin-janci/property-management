# Backlog of vectors
<sub>Last regenerated: 2026-08-14 18:29 UTC by routine</sub>

| Score | Vector | ID | Title | Status | Updated | Plan |
|-------|--------|----|----|--------|---------|------|
| 6 | test-gap | `test-gap-inquiry-idor-regression` | Add regression tests for inquiry mark_as_read cross-tenant IDOR fix (PR #497) | done | 2026-05-26 | [plan](plans/_archive/test-gap-inquiry-idor-regression.md) |
| 5 | security | `security-voice-webhook-alexa-signature-not-verified` | SECURITY: Alexa voice webhook accepts forged requests — verify_alexa_signature never checks the sign | dropped | 2026-07-28 |  |
| 3 | bug | `code-review-mobile-rn-offline-401-dropped` | mobile-rn: offline sync queue treats ALL 4xx (incl. recoverable 401 expired-token / 429 rate-limit)  | done | 2026-08-12 |  |
| 3 | bug | `code-review-ppt-web-core-logout-purge-notif-triggers` | ppt-web logout cache purge still misses the 'notification-triggers' query root (PR #2650 fix incompl | done | 2026-08-12 |  |
| 3 | bug | `gh-issue-2699-migration-collision` | dev CI broken: duplicate SQLx migration 00220 — renumber portal_get_listing_view_count to 00227 | done | 2026-08-07 | [plan](plans/_archive/gh-issue-2699-migration-collision.md) |
| 3 | bug | `code-review-ppt-web-core-ws-pong-timeout-drop` | ppt-web-core: the client heartbeat sends an APPLICATION-level ping frame {type:'ping', pa... | done | 2026-08-06 | [plan](plans/code-review-ppt-web-core-ws-pong-timeout-drop.md) |
| 3 | security | `code-review-reality-server-sso-session-invalidate-swallowed` | reality-server sync_session swallows invalidate_session error — portal session survives after PM tok | dropped | 2026-08-02 | [plan](plans/code-review-reality-server-sso-session-invalidate-swallowed.md) |
| 3 | bug | `code-review-reality-server-agency-members-unauth-idor` | reality-server GET /api/v1/agencies/{id}/members has no auth or membership check — unauthenticated c | dropped | 2026-08-02 |  |
| 3 | security | `code-review-api-core-ssrf-validator-drift` | api-server workflow api_call.rs has a duplicate SSRF validator that drifts from common::url_validati | done | 2026-08-02 |  |
| 3 | bug | `bug-scheduler-notifications-fire-once` | Scheduler notifications fire-once: transient target-resolution or dispatch error permanently drops a | dropped | 2026-08-01 | [plan](plans/bug-scheduler-notifications-fire-once.md) |
| 3 | bug | `bug-direct-upload-drops-building-id` | uploadDocumentDirect() silently drops building_id — building-scoped document uploads lose associatio | dropped | 2026-08-01 |  |
| 3 | refactor | `repeated-churn-api-server-routes-auth-rs-2026-07-28` | auth.rs repeated-churn — runs_seen=4, 2950 lines / ~107K in one module (2nd-largest route file) | dropped | 2026-07-28 |  |
| 3 | dx | `dx-api-validation-drift-gate-never-runs-on-dev` | SDK drift gate is effectively unenforced — api-validation.yml only fires on docs/api/**, so committe | done | 2026-07-28 |  |
| 3 | security | `code-review-mobile-native-kmp-android-sso-deeplink-missing-csrf-state` | SECURITY: Android SSO deep-link handler skips CSRF state check — reality://sso?token=... enables acc | done | 2026-07-27 | [plan](plans/code-review-mobile-native-kmp-android-sso-deeplink-missing-csrf-state.md) |
| 3 | security | `code-review-api-handlers-community-unauthenticated-reads` | SECURITY: community.rs get_group/list_posts/get_item run unauthenticated — anonymous cross-tenant re | dropped | 2026-07-23 | [plan](plans/code-review-api-handlers-community-unauthenticated-reads.md) |
| 3 | security | `code-review-api-handlers-community-cross-tenant-idor` | SECURITY: community.rs 5 write handlers (create_post/add_reaction/create_comment/rsvp_event/create_i | done | 2026-07-23 | [plan](plans/code-review-api-handlers-community-cross-tenant-idor.md) |
| 3 | bug | `bug-revoke-all-sessions-cookie-blindness` | revoke_all_sessions ignores refresh cookie — signs the caller out too | done | 2026-07-09 | [plan](plans/bug-revoke-all-sessions-cookie-blindness.md) |
| 3 | bug | `code-review-mobile-rn-report-fault-fake-submit` | ReportFaultScreen.tsx handleSubmit() fakes API call with setTimeout(1500) — fault reports never reac | dropped | 2026-06-16 | [plan](plans/code-review-mobile-rn-report-fault-fake-submit.md) |
| 3 | bug | `code-review-reality-web-realtor-mgmt-untranslated` | Reality-web RealtorManagement.tsx hardcoded English strings — agency flow not localized to sk/cs/de | done | 2026-06-15 | [plan](plans/code-review-reality-web-realtor-mgmt-untranslated.md) |
| 3 | bug | `code-review-reality-web-share-comparison-404` | Reality-web ComparisonUrlHandler hits non-existent /api/listings/${id} — every shared comparison URL | dropped | 2026-06-14 | [plan](plans/code-review-reality-web-share-comparison-404.md) |
| 3 | bug | `code-review-reality-web-listing-page-ssr-crash` | Reality-web listing detail SSR crashes on partial 200 body — JSON-LD build deref of undefined fields | dropped | 2026-06-14 | [plan](plans/code-review-reality-web-listing-page-ssr-crash.md) |
| 3 | bug | `bug-ios-searchview-uncompilable` | iOS SearchView.swift does not compile — performSearch/scheduleSearch undefined, resultsGrid corrupte | dropped | 2026-06-11 | [plan](plans/bug-ios-searchview-uncompilable.md) |
| 3 | security | `unchecked-todo-pr-1203` | PR #1203 (fix(aml_dsa): close cross-tenant IDOR in moderation + AML-review handlers (PAP-36)) merged | dropped | 2026-06-10 |  |
| 3 | security | `unchecked-todo-pr-1193` | PR #1193 (fix(aml-dsa): lock DSA reports to platform roles + fix file-path disclosure (PAP-47)) merg | dropped | 2026-06-10 |  |
| 3 | bug | `bug-schema-drift-runtime-sql-issue-1008` | Schema drift: runtime SQL errors from non-existent columns in voting/messaging/notification paths | done | 2026-06-07 |  |
| 3 | security | `security-llm-doc-idor` | IDOR: ai.rs LLM-doc handlers publish/list/get any tenant's listing descriptions & photo enhancements | dropped | 2026-06-01 | [plan](plans/security-llm-doc-idor.md) |
| 3 | security | `security-realtors-mark-inquiry-read-idor` | IDOR: reality-server realtors mark_inquiry_read flips any realtor's inquiry by ID with no owner scop | done | 2026-05-26 | [plan](plans/_archive/security-realtors-mark-inquiry-read-idor.md) |
| 3 | security | `security-equipment-idor` | IDOR: equipment delete/update + maintenance update mutate any tenant's equipment by ID with no org s | done | 2026-05-25 | [plan](plans/_archive/security-equipment-idor.md) |
| 3 | security | `security-ssrf-outbound-url-validation` | SSRF: signed-document fetch + webhook-test POST issue outbound requests to unvalidated user-controll | done | 2026-05-25 | [plan](plans/_archive/security-ssrf-outbound-url-validation.md) |
| 3 | security | `security-voice-device-idor` | IDOR: unlink_voice_device deactivates any device by ID with no owner/org scoping | done | 2026-05-25 | [plan](plans/_archive/security-voice-device-idor.md) |
| 2 | bug | `code-review-reality-web-comparison-view-i18n` | reality-web ComparisonView renders hardcoded English on 4-locale portal | open | 2026-08-14 |  |
| 2 | bug | `code-review-reality-web-realtor-mgmt-i18n` | reality-web RealtorManagement — destructive remove-realtor modal untranslated (partial i18n) | open | 2026-08-14 |  |
| 2 | bug | `code-review-reality-web-agency-errorstate-i18n` | reality-web AgencyErrorStates 'No Agency Found' state hardcoded English | open | 2026-08-14 |  |
| 2 | security | `code-review-api-handlers-share-pw-no-throttle` | api-handlers public share password endpoint is unthrottled — brute-forceable | open | 2026-08-14 |  |
| 2 | bug | `code-review-ppt-web-core-perfmetrics-listener-leak` | usePerformanceMetrics never removes its visibilitychange/load listeners on cleanup — event-listener  | done | 2026-08-12 |  |
| 2 | bug | `code-review-mobile-native-kmp-cancellation-swallowed` | mobile-native-kmp: shared repositories swallow CancellationException in catch(e: Exception), breakin | open | 2026-08-11 |  |
| 2 | bug | `code-review-api-core-voice-actions-fabricated-empty` | Voice check-announcements & check-meter fabricate success with empty data — residents told 'no new a | dropped | 2026-08-10 |  |
| 2 | bug | `code-review-api-core-quiet-drain-drops-failed-delivery` | Quiet-hours drain marks held push released even when delivery failed (sent=0) — held notification pe | dropped | 2026-08-10 |  |
| 2 | bug | `code-review-api-handlers-reports-csv-injection` | Reports CSV export writes user-authored vote titles unescaped — spreadsheet formula injection (bypas | done | 2026-08-10 |  |
| 2 | bug | `code-review-reality-web-comparison-share-all-or-nothing` | reality-web ComparisonUrlHandler uses Promise.all — one bad listing id blanks the whole shared compa | done | 2026-08-07 |  |
| 2 | bug | `code-review-reality-web-listingform-nan-validation` | reality-web ListingForm posts NaN for area/rooms — non-numeric or negative input coerced silently | done | 2026-08-07 |  |
| 2 | bug | `code-review-api-handlers-enhanced-chat-stub` | api-handlers: enhanced_chat is a stubbed production handler that returns fabricated data.... | done | 2026-08-06 |  |
| 2 | bug | `code-review-mobile-rn-auth-restore-stale-token` | mobile-rn: the cold-start initialize() effect reads the stored access token (SecureSto... | done | 2026-08-06 |  |
| 2 | bug | `code-review-ppt-web-core-ws-giveup-no-resume` | ppt-web-core: once reconnectAttempts reaches maxReconnectAttempts (default 10, :290) sche... | done | 2026-08-06 |  |
| 2 | bug | `code-review-reality-server-inquiries-no-ratelimit` | reality-server: InquiryResult::RateLimited variant is defined but never constructed or matc... | done | 2026-08-06 |  |
| 2 | bug | `code-review-reality-server-listing-viewcount-hardcoded-zero` | reality-server: the LIVE public listing-detail handler get_listing() (route wired at routes... | done | 2026-08-06 |  |
| 2 | bug | `code-review-mobile-rn-offline-terminal-drop-dataloss` | mobile: useOfflineSupport — transient (5xx/network) queued actions are permanently DROPPED after 3 r | done | 2026-08-04 |  |
| 2 | bug | `code-review-ppt-web-core-logout-cache-purge-gap` | ppt-web logout leaves 3 tenant-scoped TanStack Query roots un-purged (predictive-maintenance, sentim | done | 2026-08-04 |  |
| 2 | bug | `code-review-ppt-web-core-mutation-no-onerror` | ppt-web rentals + financial useMutation hooks lack onError — money-movement + platform-connect failu | done | 2026-08-04 |  |
| 2 | bug | `code-review-mobile-native-kmp-portfolio-analytics-caps-100` | mobile-native-kmp: getPortfolioAnalytics() truncates realtor portfolio at 100 listings — dashboard u | open | 2026-08-04 |  |
| 2 | test-gap | `screen-map-drift-pr-2646-ppt` | screen-map drift: PR #2646 touched ppt-web route wrapper (App.tsx) without updating docs/screens/ppt | dropped | 2026-08-04 |  |
| 2 | test-gap | `screen-map-drift-pr-2647-ppt` | screen-map drift: PR #2647 touched 8 ppt-web route wrappers (i18n not-found fallbacks) without updat | dropped | 2026-08-04 |  |
| 2 | test-gap | `screen-map-drift-pr-2648-ppt` | screen-map drift: PR #2648 touched ppt-web rentals route (mutation auth guard) without updating docs | done | 2026-08-04 |  |
| 2 | test-gap | `screen-map-drift-pr-2649-ppt` | screen-map drift: PR #2649 touched ppt-web rentals + financial routes (onError toasts) without updat | done | 2026-08-04 |  |
| 2 | bug | `code-review-ppt-web-ui-accounting-invoice-silent-fail` | AccountingInvoiceManagementPage: create/delete mutations have no onError — silent invoice failures | done | 2026-08-03 |  |
| 2 | bug | `code-review-ppt-web-core-api-token-provider-unset` | ppt-web getApiClient() sends requests with no Authorization header — configureApiClient is never cal | done | 2026-08-03 |  |
| 2 | bug | `code-review-ppt-web-core-api-retry-nonidempotent` | ppt-web axios retry interceptor retries non-idempotent POST/PUT on 5xx / network errors — risk of du | done | 2026-08-03 |  |
| 2 | bug | `code-review-mobile-rn-offline-sync-false-complete` | mobile: useOfflineSupport.processQueue reports isComplete:true after a head-of-line-blocked (halted) | done | 2026-08-03 |  |
| 2 | bug | `code-review-mobile-rn-thread-send-silent-fail` | mobile: ThreadDetailScreen.handleSend send-message mutation has onSuccess only, no onError — failed  | done | 2026-08-03 |  |
| 2 | bug | `code-review-ppt-web-core-route-error-boundary-gap` | ppt-web has no route-outlet ErrorBoundary — a single stale-chunk lazy() rejection unmounts the entir | done | 2026-08-03 |  |
| 2 | bug | `code-review-ppt-web-core-rentals-auth-nonnull-mutation` | ppt-web rentals mutations dereference `auth!` non-null — mid-session token loss throws uncaught Type | done | 2026-08-03 |  |
| 2 | test-gap | `screen-map-drift-pr-2636-reality` | docs/screens/reality/agency-import + agency-inquiries out of sync with PR #2636 i18n rewrite | done | 2026-08-03 |  |
| 2 | security | `code-review-reality-server-db-error-leak-to-client` | reality-server leaks raw sqlx::Error strings to internet-facing clients, bypassing util::errors::db_ | dropped | 2026-08-02 | [plan](plans/code-review-reality-server-db-error-leak-to-client.md) |
| 2 | security | `code-review-api-core-idempotency-client-tenant` | Idempotency middleware trusts client-supplied X-Tenant-ID header for cache-scope key — cross-tenant  | done | 2026-08-02 |  |
| 2 | test-gap | `test-gap-disputes-kpis-window-validation` | /disputes/kpis: no window_start<=window_end validation and only test is BIT-440 quarantined | dropped | 2026-08-01 |  |
| 2 | refactor | `refactor-churn-hotspot-integrations-booking-mod-2026-07-31` | Churn hotspot: backend/crates/integrations/src/booking/mod.rs — 3626 lines this window (recently spl | done | 2026-07-31 |  |
| 2 | refactor | `refactor-churn-hotspot-api-server-reports-2026-07-31` | Churn hotspot: backend/servers/api-server/src/routes/reports.rs — 3329 lines this window (PR #2599 e | dropped | 2026-07-31 |  |
| 2 | refactor | `refactor-churn-hotspot-api-server-auth-2026-07-31` | Churn hotspot: backend/servers/api-server/src/routes/auth.rs — 2950 lines this window (runs_seen=5,  | dropped | 2026-07-31 |  |
| 2 | test-gap | `test-gap-screen-map-drift-pr-2600-reality` | Screen-map drift: reality-web layout changed without docs/screens/reality/ update (PR #2600) | done | 2026-07-31 |  |
| 2 | dx | `dx-fixme-admin-web-mobile-config-patch-endpoint` | admin-web mobile-config Save flow blocked: PATCH /api/v1/admin/mobile-config endpoint missing | dropped | 2026-07-31 |  |
| 2 | dx | `dx-fixme-admin-web-platform-settings-patch-endpoint` | admin-web platform-settings Save blocked: PATCH /api/v1/platform-admin/settings endpoint missing | dropped | 2026-07-31 |  |
| 2 | dx | `dx-fixme-admin-web-ui-kit-primitives-missing` | @ppt/ui-kit missing primitives (Stepper, FileUpload, RadioCards, StatusPill) — admin-web ships inlin | done | 2026-07-31 |  |
| 2 | dx | `dx-fixme-api-client-generator-typescript-error-types` | openapi-ts generator wrapper swallows errors and emits weak error types | done | 2026-07-31 |  |
| 2 | bug | `code-review-api-core-resolved-rs-leaks-db-error` | SECURITY-LITE: layout/resolved.rs err500 handler leaks raw sqlx/serde error text on public GET /layo | done | 2026-07-30 |  |
| 2 | security | `code-review-reality-web-inline-tenant-json-xss` | SECURITY: reality-web layout.tsx inlines tenant-config JSON into <script> without </script>/U+2028/U | done | 2026-07-28 |  |
| 2 | test-gap | `test-gap-voice-webhooks-zero-coverage` | test-gap: voice_webhooks.rs (1148 lines, 6 mounted endpoints incl. OAuth token exchange) has no test | done | 2026-07-28 |  |
| 2 | bug | `code-review-ppt-web-aml-review-decision-untrusted-cast` | AmlDashboardPage casts raw window.prompt text into the review-decision union — a typo submits an inv | done | 2026-07-28 |  |
| 2 | dx | `dx-admin-web-platform-settings-mobile-config-permanent-noop` | admin-web platform-settings + mobile-config Save paths are permanent no-ops — the backing endpoints  | done | 2026-07-28 |  |
| 2 | refactor | `refactor-churn-hotspot-api-server-reports-2026-07-27` | Churn hotspot: backend/servers/api-server/src/routes/reports.rs — 3329 lines this window, runs_seen= | done | 2026-07-27 |  |
| 2 | bug | `bug-hotfix-no-test-pr-2547` | PR #2547 shipped scheduler retention prune fix without an api-server regression test (hotfix-no-test | done | 2026-07-27 |  |
| 2 | bug | `code-review-ppt-web-core-authctx-init-stale-role` | AuthContext init bypasses refreshTokenInternal → stale role on cold-boot refresh (#574 fix gap) | done | 2026-07-24 |  |
| 2 | bug | `code-review-ppt-web-core-ws-token-rotation-stale` | WebSocket not re-authed on token rotation — connect() early-return leaves live socket on old token | done | 2026-07-24 |  |
| 2 | test-gap | `screen-map-drift-pr-2497-reality` | screen-map drift: PR #2497 touched reality-web/app/api/layout-revalidate/route.ts w/o docs/screens/r | done | 2026-07-24 |  |
| 2 | test-gap | `screen-map-drift-pr-2431-reality` | Screen-map drift: PR #2431 touched reality-web/src/app/api/layout-revalidate/route.ts without updati | done | 2026-07-21 |  |
| 2 | bug | `code-review-ppt-web-ui-propinput-json-coerce` | TenantSectionEditor PropInput silently JSON.parse-coerces every string prop on blur — override paylo | done | 2026-07-21 |  |
| 2 | bug | `code-review-ppt-web-ui-savedirty-stale-closure` | DashboardCustomizePage 'changed since sent' check is tautological — concurrent edits during in-fligh | done | 2026-07-21 |  |
| 2 | bug | `code-review-ppt-web-ui-actionqueue-mock-shipped` | Dashboard useActionQueue queryFn returns generateMockData — production users see fabricated action i | done | 2026-07-21 |  |
| 2 | security | `code-review-api-core-scheduler-units-target-cross-tenant` | scheduler.rs units/buildings target queries lack organization_id AND-scope — fan-out can leak across | done | 2026-07-20 |  |
| 2 | refactor | `refactor-churn-hotspots-api-server-auth-2026-07-12` | Churn hotspot cluster: api-server routes/auth.rs (runs_seen=3) + auth_tests.rs + reality-server rout | done | 2026-07-12 |  |
| 2 | security | `security-forgot-password-no-rate-limit` | /forgot-password and /resend-verification have no rate limit — mailbomb / token-clobber | done | 2026-07-09 | [plan](plans/security-forgot-password-no-rate-limit.md) |
| 2 | test-gap | `test-gap-hotfix-no-test-pr-959-reality-listings-pagination` | Reality-server listings pagination clamp (PR #959) shipped without a regression test for limit=-1 | done | 2026-07-05 |  |
| 2 | test-gap | `screen-map-drift-pr-1418-ppt` | PR #1418 touched routes/** (faults.route.test.tsx) without updating docs/screens/ppt/* — heuristic,  | done | 2026-07-05 |  |
| 2 | bug | `code-review-api-core-vote-partial-cmp-panic` | vote.rs:1765 calculate_question_result() uses partial_cmp().unwrap() on f64 — NaN/Inf weights panic  | done | 2026-06-16 |  |
| 2 | test-gap | `unchecked-todo-pr-1196` | PR #1196 (feat(ppt-web): add missing test coverage for faults feature) merged with 2 unchecked TODO  | dropped | 2026-06-10 |  |
| 2 | dx | `dx-push-fanout-blpop-drain` | PushFanoutWorker BLPOP queue-drain deferred — Redis path is a logging no-op | done | 2026-06-06 |  |
| 2 | refactor | `refactor-ai-rs-module-split` | ai.rs (3,134 LOC) — explicit module-split into routes/ai/{sessions,equipment,workflows,voice,llm,mod | done | 2026-06-06 |  |
| 2 | refactor | `refactor-announcements-rs-hot` | announcements.rs churn-hot — 2,722 lines this run (Epic 2B + Epic 6 work) | done | 2026-06-06 |  |
| 2 | refactor | `refactor-announcements-rs-module-split` | announcements.rs (2,722 LOC) — explicit module-split into routes/announcements/{crud,targeting,deliv | done | 2026-06-06 |  |
| 2 | refactor | `refactor-app-tsx-route-coupling` | Reduce App.tsx route-aggregator coupling (top churn hotspot, merge-conflict risk) | done | 2026-06-06 |  |
| 2 | refactor | `refactor-platform-admin-rs-module-split` | platform_admin.rs (2,762 LOC) — explicit module-split into routes/platform_admin/{tenants,features,b | done | 2026-06-06 |  |
| 2 | test-gap | `test-gap-screen-map-drift-pr-1033-ppt` | Screen-map drift: PR #1033 wired error/retry into AnnouncementsPage+FaultsPage via App.tsx without a | done | 2026-06-06 |  |
| 2 | bug | `bug-risky-churn-pr-992-mobile-app-tsx` | Risky churn: mobile App.tsx deep-link/doc-detail wiring changing across back-to-back PRs without cov | done | 2026-06-05 |  |
| 2 | dx | `dx-integration-marketplace-stubs` | Integration marketplace install/OAuth flows are placeholders — wire backend handlers + UI navigation | done | 2026-06-05 |  |
| 2 | test-gap | `test-gap-booking-push-validation-untested` | Booking push availability/rates endpoints add batch-cap + non-negative guards with no regression tes | done | 2026-06-05 |  |
| 2 | test-gap | `test-gap-hotfix-no-test-pr-874-portal-webhooks` | Portal webhook fail-closed fix (PR #874) shipped without a regression test for unverified-signature  | done | 2026-06-05 |  |
| 2 | test-gap | `test-gap-hotfix-no-test-pr-918-mobile-dev-review` | Mobile dev-review batch (PR #918, 5 files under frontend/apps/mobile/src) shipped without a regressi | done | 2026-06-05 |  |
| 2 | test-gap | `test-gap-hotfix-no-test-pr-921-sso-consumer` | Reality-server SSO consumer review fix (PR #921, closes #820) shipped without a regression test | done | 2026-06-05 |  |
| 2 | test-gap | `test-gap-hotfix-no-test-pr-923-branch-protection-rebase` | CI branch-protection + auto-rebase workflow change (PR #923) shipped without an integration test | done | 2026-06-05 |  |
| 2 | test-gap | `test-gap-hotfix-no-test-pr-939-deploy-server-scopes` | deploy-server OIDC scope mapping (#939) shipped without unit test for derive_oidc_scopes | done | 2026-06-05 |  |
| 2 | test-gap | `test-gap-hotfix-no-test-pr-943-mobile-dev-review-tail` | Mobile RN dev-review tail (#943) shipped without test coverage | done | 2026-06-05 |  |
| 2 | test-gap | `test-gap-hotfix-no-test-pr-990-frontend-gap-sweep` | Frontend gap-sweep (PR #990, 34 files across Epics 1/6/7B/9/10B/11/15/17/18) shipped without a regre | done | 2026-06-05 |  |
| 2 | test-gap | `test-gap-hotfix-no-test-pr-992-mobile-doc-detail` | Mobile document-detail wiring (PR #992) shipped without a regression test for the deep-link payload  | done | 2026-06-05 |  |
| 2 | test-gap | `test-gap-screen-map-drift-pr-839-ppt` | Screen-map drift: PR #839 modified ppt-web App.tsx (FileDisputePageRoute) without a docs/screens/ppt | done | 2026-06-05 |  |
| 2 | refactor | `refactor-ppt-web-untranslated-strings` | ppt-web status/auth components hardcode English in an otherwise i18n'd app | done | 2026-06-04 |  |
| 2 | bug | `bug-mediation-page-no-error-state` | MediationWorkspacePage shows empty/unknown state instead of error UI on dispute fetch failure | done | 2026-06-03 |  |
| 2 | bug | `bug-mobile-voting-unsafe-cast` | Mobile VotingScreen double-casts API result across boundary — render-time crash on unexpected shape | done | 2026-06-03 |  |
| 2 | bug | `bug-reality-web-realtor-invite-silent-error` | Reality-web InviteRealtorModal swallows invite-mutation failure with no error UI | done | 2026-06-03 |  |
| 2 | bug | `bug-webhook-airbnb-dup-sync-jobs` | Airbnb webhook at-least-once delivery enqueues duplicate SYNC_EXTERNAL jobs | done | 2026-06-03 |  |
| 2 | dx | `dx-documentsbrowse-folder-preselect` | DocumentsBrowse MoveFolderDialog cannot pre-select current folder (DocumentSummary lacks folder_id) | done | 2026-06-03 |  |
| 2 | test-gap | `test-gap-hotfix-no-test-pr-963-security-headers` | API + SPA security-headers middleware (PR #963) shipped without an assertion test for HSTS/nosniff/C | done | 2026-06-03 |  |
| 2 | test-gap | `test-gap-router-set-parity-tests` | api-server main.rs vs lib.rs::create_router diverge silently (5 routes unreachable in prod, no test  | done | 2026-06-01 |  |
| 2 | bug | `bug-report-schedule-update-no-sql` | ReportSchedule.update_schedule stores cron in `time` workaround; documented UPDATE never runs (missi | done | 2026-05-30 |  |
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
| 2 | security | `security-rls-migration-residual` | Complete RLS migration in 31 remaining handlers (voting, market_pricing, faults, notif_prefs, report | done | 2026-05-23 |  |
| 1 | refactor | `code-review-ppt-web-ui-protectedroute-i18n` | ppt-web-ui: ProtectedRoute hardcodes English 'Access Denied' / 'Loading...' strings instead of using | done | 2026-08-12 |  |
| 1 | refactor | `code-review-ppt-web-ui-offline-ind-i18n` | ppt-web-ui: OfflineIndicator banner text is hardcoded English, not i18n | done | 2026-08-12 |  |
| 1 | bug | `code-review-mobile-rn-date-locale-en-us` | mobile-rn: date formatters hardcode 'en-US' locale in many screens (incomplete fix of #2282), so dat | done | 2026-08-12 |  |
| 1 | bug | `code-review-mobile-rn-meterdetail-i18n` | mobile-rn: MeterDetailScreen renders hardcoded English UI strings not wrapped in t(), while sibling  | done | 2026-08-12 |  |
| 1 | test-gap | `churn-hotspot-frontend-apps-ppt-web-src-routes-groups-rentals-mappers-test-tsx` | ppt-web rentals API↔UI mappers now covered by regression tests — monitor for further churn | done | 2026-08-12 |  |
| 1 | test-gap | `churn-hotspot-frontend-apps-mobile-src-hooks-useOfflineSupport-test-ts` | mobile useOfflineSupport now covered by retryable-4xx regression tests — verify future changes don't | done | 2026-08-12 |  |
| 1 | refactor | `churn-hotspot-backend-servers-api-server-src-services-oauth-rs` | api-server oauth.rs churning around token-usage recording — monitor for further refactor pressure | done | 2026-08-12 |  |
| 1 | dx | `stalled-review-pr-2555` | stalled review: PR #2555 feat(acc) UC-ACC-05.17 wire sent/cancelled invoice lifecycle (15d open, 13d | needs-human-judgement | 2026-08-12 |  |
| 1 | dx | `stalled-review-pr-2558` | stalled review: PR #2558 feat(acc) UC-ACC-05.9 invoice PDF render endpoint (15d open, 13d idle) | needs-human-judgement | 2026-08-12 |  |
| 1 | dx | `stalled-review-pr-2559` | stalled review: PR #2559 feat(acc) UC-ACC-05.8 PAY by square QR endpoint (15d open, 13d idle) | needs-human-judgement | 2026-08-12 |  |
| 1 | dx | `closed-not-merged-pr-2705` | closed-not-merged: PR #2705 dx-cnm-pr-2385-retry2 (rust-toolchain 1.94.1→1.100.0) — second retry clo | done | 2026-08-12 |  |
| 1 | test-gap | `code-review-mobile-native-kmp-ssoservice-untested` | mobile-native-kmp: SsoService (deep-link token exchange, login, password reset, session restore) has | open | 2026-08-11 |  |
| 1 | bug | `code-review-api-core-quiet-schedule-err-failopen` | Notification pipeline swallows quiet-hours schedule DB error and fails open — push delivered during  | done | 2026-08-10 |  |
| 1 | bug | `code-review-reality-web-listingform-no-i18n` | reality-web ListingForm hardcodes English throughout a next-intl sk/cs/de/en app | done | 2026-08-07 |  |
| 1 | refactor | `churn-hotspot-backend-servers-api-server-src-services-workflow_executor-rs` | Churn hotspot: backend/servers/api-server/src/services/workflow_executor.rs (+265/-192 in #2685 work | done | 2026-08-07 |  |
| 1 | refactor | `churn-hotspot-frontend-apps-ppt-web-src-lib-websocket-ts` | Churn hotspot: frontend/apps/ppt-web/src/lib/websocket.ts (+11/-94 heartbeat removal PR #2689) | dropped | 2026-08-07 |  |
| 1 | refactor | `churn-hotspot-backend-servers-api-server-src-routes-ai-llm-rs` | Churn hotspot: backend/servers/api-server/src/routes/ai/llm.rs (fail-closed enhanced_chat #2688 + up | done | 2026-08-07 |  |
| 1 | security | `code-review-api-handlers-ai-upstream-error-leak` | api-handlers: the raw upstream LLM-provider error is forwarded verbatim into the client-f... | done | 2026-08-06 |  |
| 1 | bug | `code-review-mobile-rn-biometric-prompt-i18n` | mobile-rn: enableBiometric passes hardcoded English strings to the OS biometric dialog... | done | 2026-08-06 |  |
| 1 | bug | `code-review-reality-server-inquiry-email-stub` | reality-server: a shipped-but-non-functional notification path. | done | 2026-08-06 |  |
| 1 | bug | `code-review-reality-server-orphan-listinghandler-stub-detail` | reality-server: ListingHandler::get_listing() returns a PublicListingDetail with a whole bl... | done | 2026-08-06 |  |
| 1 | bug | `code-review-reality-server-orphan-schedule-viewing-stub` | reality-server: schedule_viewing() runs full input validation (future-date at :300, <=90-da... | done | 2026-08-06 |  |
| 1 | bug | `code-review-mobile-native-kmp-portfolio-analytics-unbounded-fanout` | mobile-native-kmp: getPortfolioAnalytics() fans out one analytics HTTP request per listing with no c | dropped | 2026-08-04 |  |
| 1 | refactor | `churn-hotspot-frontend-apps-ppt-web-src-routes-groups-rentals-tsx` | Churn hotspot: 2 commits touching frontend/apps/ppt-web/src/routes/groups/rentals.tsx (window 2026-0 | dropped | 2026-08-04 |  |
| 1 | refactor | `churn-hotspot-frontend-apps-mobile-src-screens-messages-ThreadDetailScreen-tsx` | Churn hotspot: 2 commits touching frontend/apps/mobile/src/screens/messages/ThreadDetailScreen.tsx ( | done | 2026-08-04 |  |
| 1 | refactor | `churn-hotspot-frontend-apps-mobile-src-hooks-useOfflineSupport-ts` | Churn hotspot: 2 commits touching frontend/apps/mobile/src/hooks/useOfflineSupport.ts (window 2026-0 | dropped | 2026-08-04 |  |
| 1 | bug | `code-review-ppt-web-ui-aria-label-i18n` | ppt-web: 103 hardcoded English aria-label attributes across feature components — screen readers hear | done | 2026-08-03 |  |
| 1 | bug | `code-review-ppt-web-ui-sessions-double-cast` | SessionsPage double-casts through unknown to hand-maintained interface — defeats API-boundary type-c | done | 2026-08-03 |  |
| 1 | bug | `code-review-reality-web-agency-import-i18n` | reality-web: agency/import feature cluster hardcoded English — 65 sibling components use useTranslat | done | 2026-08-03 |  |
| 1 | bug | `code-review-reality-web-protectedroute-i18n` | reality-web ProtectedRoute renders untranslated auth-required gate for sk/cs/de | done | 2026-08-03 |  |
| 1 | bug | `code-review-mobile-rn-hardcoded-empty-error-i18n` | mobile: empty/error/loading state strings hardcoded in English across ~18 screens despite react-i18n | done | 2026-08-03 |  |
| 1 | bug | `code-review-mobile-rn-push-debug-console-log` | mobile: usePushNotifications — debug console.log left on the device-token register/unregister produc | done | 2026-08-03 |  |
| 1 | bug | `code-review-ppt-web-core-notfound-i18n-gap` | ppt-web route wrappers render 8 hardcoded English 'X not found' fallbacks — sk/cs/de users see untra | done | 2026-08-03 |  |
| 1 | bug | `code-review-reality-server-password-reset-no-transport` | reality-server password reset is non-functional in prod — token discarded, no email transport, endpo | dropped | 2026-08-01 |  |
| 1 | refactor | `refactor-churn-hotspot-api-server-scheduler-2026-07-30` | Churn hotspot: backend/servers/api-server/src/services/scheduler.rs — 347 lines this window (PRs #25 | done | 2026-07-30 |  |
| 1 | refactor | `refactor-churn-hotspot-api-server-layout-tenant-2026-07-30` | Churn hotspot: backend/servers/api-server/src/routes/layout/tenant.rs — 262 lines this window (PR #2 | dropped | 2026-07-30 |  |
| 1 | refactor | `refactor-churn-hotspot-api-server-layout-admin-2026-07-30` | Churn hotspot: backend/servers/api-server/src/routes/layout/admin.rs — 240 lines this window (PRs #2 | dropped | 2026-07-30 |  |
| 1 | bug | `code-review-api-core-admin-rs-swallowed-serialize` | layout/admin.rs (+ tenant.rs) mutation handlers end with unwrap_or_default() serialize — a failed se | done | 2026-07-30 |  |
| 1 | bug | `code-review-api-core-scheduler-rs-silent-target-err` | scheduler.rs silently swallows DB errors on notification target lookups at 3 sites — failed dispatch | done | 2026-07-30 |  |
| 1 | refactor | `code-review-reality-web-viewsource-untrusted-cast` | reality-web listingAnalytics.ts casts untrusted ?source= query param straight to ViewSource union —  | done | 2026-07-28 |  |
| 1 | dx | `dx-stale-todo-security-comments-faults-critical-notifications` | Stale TODO(security) headers in faults.rs / critical_notifications.rs describe a hardcoded-false gat | done | 2026-07-28 |  |
| 1 | refactor | `refactor-churn-hotspot-integrations-booking-mod-rs-2026-07-27` | Churn hotspot: backend/crates/integrations/src/booking/mod.rs — 3185 lines this window (post-PR-#217 | done | 2026-07-27 |  |
| 1 | bug | `code-review-mobile-native-kmp-portfolio-analytics-drops-view-zero-days` | PortfolioAnalytics inquiriesTrend silently drops days with inquiries but zero views (set-difference  | done | 2026-07-27 |  |
| 1 | refactor | `code-review-ppt-web-core-ws-ungated-console` | 10 ungated console.warn/error in ppt-web websocket.ts leak diagnostics in prod | dropped | 2026-07-24 |  |
| 1 | refactor | `refactor-churn-hotspot-platform-admin-authz-batch2-2026-07-23` | Churn hotspot: platform_admin_authz_batch2_tests.rs — 417 lines this run (BIT-557 test backfill) | done | 2026-07-23 |  |
| 1 | refactor | `refactor-churn-hotspot-org-property-authz-backfill-2026-07-23` | Churn hotspot: org_property_authz_backfill_tests.rs — 412 lines this run (BIT-268/BIT-559 authz salv | done | 2026-07-23 |  |
| 1 | refactor | `refactor-churn-hotspot-infra-ops-authz-backfill-2026-07-23` | Churn hotspot: infra_ops_authz_backfill_tests.rs — 364 lines this run (BIT-268 test backfill) | dropped | 2026-07-23 |  |
| 1 | triage | `triage-closed-not-merged-pr-2489` | PR #2489 closed unmerged: dependabot npm-minor-patch (5→4 update group) superseded by #2491 | dropped | 2026-07-23 |  |
| 1 | refactor | `refactor-churn-hotspot-repo-map-md-2026-07-20` | Churn hotspot: docs/repo-map.md — 4 touches this window (per-PR route-map refresh) | done | 2026-07-21 |  |
| 1 | refactor | `refactor-churn-hotspot-ppt-dashboard-md-2026-07-21` | Churn hotspot: docs/screens/ppt/dashboard.md — 3 touches this run (Layout & Content Manager pilot in | done | 2026-07-21 |  |
| 1 | refactor | `refactor-churn-hotspot-layouteditorpage-tsx-2026-07-21` | Churn hotspot: frontend/apps/admin-web/src/features/layout-editor/LayoutEditorPage.tsx — 2 touches,  | done | 2026-07-21 |  |
| 1 | bug | `code-review-api-core-scheduler-target-ids-silent-parse` | scheduler.rs get_announcement_target_users() silently swallows target_ids JSON parse errors — malfor | done | 2026-07-20 |  |
| 1 | refactor | `refactor-churn-hotspot-mobile-package-json-2026-07-20` | Churn hotspot: frontend/apps/mobile/package.json — 5 touches this window (Expo/expo-notifications/ex | done | 2026-07-20 |  |
| 1 | refactor | `refactor-churn-hotspot-backend-cargo-toml-2026-07-20` | Churn hotspot: backend/Cargo.toml — 3 touches this window (dependabot minor-patch cascade + layout-c | done | 2026-07-20 |  |
| 1 | dx | `dx-closed-not-merged-pr-2385` | PR #2385 (dependabot: dtolnay/rust-toolchain 1.94.1 → 1.100.0) closed unmerged — likely superseded b | dropped | 2026-07-20 |  |
| 1 | dx | `dx-closed-not-merged-pr-2387` | PR #2387 (dependabot: npm-minor-patch 15-update rollup) closed unmerged — superseded by the 19-updat | dropped | 2026-07-20 |  |
| 1 | refactor | `refactor-churn-hotspots-en-json-2026-07-16` | Churn hotspot: frontend/apps/ppt-web/messages/en.json — frontend/apps/ppt-web/messages/en.json: +392 | done | 2026-07-16 |  |
| 1 | refactor | `refactor-churn-hotspots-sitemap-json-2026-07-16` | Churn hotspot: frontend/packages/sitemap/src/json/sitemap.json — frontend/packages/sitemap/src/json/ | done | 2026-07-16 |  |
| 1 | refactor | `refactor-churn-hotspot-backend-integrations-booking-mod` | backend integrations booking/mod.rs — instability watch after PR #2176 split | done | 2026-07-09 |  |
| 1 | test-gap | `test-gap-repeated-churn-oauth-integration-tests` | oauth_integration_tests.rs repeated-churn (runs_seen 2→3) — OAuth handlers still moving | dropped | 2026-07-09 |  |
| 1 | refactor | `refactor-churn-hotspot-api-server-routes-auth` | api-server routes/auth.rs — repeated hotspot + 3 static-review findings this run | done | 2026-07-09 |  |
| 1 | bug | `bug-refresh-empty-cookie-shadows-body-token` | /refresh and /logout — empty refresh_token cookie shadows valid body token | done | 2026-07-09 |  |
| 1 | bug | `code-review-mobile-native-kmp-deeplink-token-not-url-decoded` | DeepLinkRouter skips URL-decoding while Android Uri.getQueryParameter decodes — SSO tokens diverge p | dropped | 2026-07-05 |  |
| 1 | bug | `code-review-mobile-native-kmp-search-stale-response-race` | SearchScreen stale-response race — overlapping searches can clobber newer results | dropped | 2026-07-05 |  |
| 1 | test-gap | `test-gap-screen-map-drift-pr-1085-reality` | Screen-map drift: PR #1085 modified reality-web listing detail metadata + page without screen-doc up | dropped | 2026-07-05 |  |
| 1 | test-gap | `test-gap-screen-map-drift-pr-1100-ppt` | Screen-map drift: PR #1100 modified ppt-web App.tsx (FileDisputePageRoute extraction) without screen | dropped | 2026-07-05 |  |
| 1 | bug | `bug-risky-churn-pr-963-api-main-rs` | Risky churn: api-server main.rs security-headers wiring shipped without a middleware smoke test | dropped | 2026-07-05 |  |
| 1 | test-gap | `test-gap-screen-map-drift-pr-922-ppt` | Screen-map drift: PR #922 modified ppt-web App.tsx (dev-review rounds 1-5 fixes) without a docs/scre | done | 2026-07-05 |  |
| 1 | refactor | `churn-hotspot-mobile-native-listing-detail-kt` | Churn hotspot: ListingDetailScreen.kt — +1279 LOC this run (gap-82-4 reality mobile favorite toggle) | done | 2026-07-05 |  |
| 1 | refactor | `refactor-churn-hotspot-mobile-documents` | Churn hotspot: DocumentsScreen.tsx — 3 PRs this run | done | 2026-07-05 |  |
| 1 | bug | `bug-ios-deeplink-info-plist-missing` | iOS deep-link layer dead at runtime — Info.plist missing CFBundleURLTypes + applinks entitlement | dropped | 2026-07-05 |  |
| 1 | test-gap | `test-gap-hotfix-no-test-pr-1288-webhook-rls` | Webhook handlers RLS migration (PR #1288, PAP-170) shipped without a new regression test for repo-la | dropped | 2026-07-05 |  |
| 1 | test-gap | `test-gap-hotfix-no-test-pr-1287-rls-llm-sessions` | AI llm/sessions + integrations sync + subscriptions RLS migration (PR #1287, PAP-169) shipped withou | dropped | 2026-07-05 |  |
| 1 | test-gap | `test-gap-hotfix-no-test-pr-1289-api-ecosystem` | api_ecosystem.rs RLS migration (PR #1289, PAP-167) — 162-line handler rework shipped without a regre | dropped | 2026-07-05 |  |
| 1 | test-gap | `test-gap-hotfix-no-test-pr-1292-mfa-rls` | mfa.rs RLS migration (PR #1292, PAP-168) shipped without a regression test; also landed broken and w | dropped | 2026-07-05 |  |
| 1 | refactor | `code-review-api-core-osrng-expect` | crypto.rs:127 SysRng.try_fill_bytes(...).expect() panics if OS CSPRNG errors during integration-cred | done | 2026-07-05 |  |
| 1 | bug | `code-review-mobile-rn-screens-mock-data` | Mobile RN production screens (Buildings/Meters/Leases/PersonMonths/Notifications/Threads/Forms) rend | done | 2026-07-05 |  |
| 1 | bug | `code-review-mobile-rn-deeplink-init-unhandled` | useDeepLinkRouting.ts:27-36 — initialize() re-runs on onNavigate identity change + void promise with | dropped | 2026-07-05 |  |
| 1 | refactor | `refactor-churn-hotspot-backend-crates-db-src-models-mod-rs` | Churn hotspot: backend/crates/db/src/models/mod.rs (12 commits in 19-day catch-up) | dropped | 2026-07-05 |  |
| 1 | refactor | `refactor-churn-hotspot-backend-crates-db-src-repositories-rental-rs` | Churn hotspot: backend/crates/db/src/repositories/rental.rs (11 commits in 19-day catch-up) | done | 2026-07-05 |  |
| 1 | refactor | `refactor-closed-not-merged-pr-1378` | PR #1378 closed without merge — DROP-OWNED-BY teardown theory for #1332 was wrong root cause, supers | done | 2026-06-15 |  |
| 1 | test-gap | `code-review-issue-1137-pkce-test-tautology` | PKCE unit test became a tautology after services/oauth.rs DRY refactor (#1132) | done | 2026-06-07 |  |
| 1 | triage | `triage-issue-1061-dispatcher-archive-corruption` | Triage: dispatcher incident — assignments-archive.json corrupted to 1/196 rows on dev branch (#1061) | done | 2026-06-07 |  |
| 1 | triage | `triage-issue-950` | Issue #950 (no labels, OPEN): CI: trigger-deploy 403 marks all dev image builds red and blocks stagi | done | 2026-06-07 |  |
| 1 | triage | `triage-issue-952` | Issue #952 (no labels, OPEN): [staging] Reality SSO login dead-ends: redirect_uri callback 404s on r | done | 2026-06-07 |  |
| 1 | triage | `triage-issue-769` | Issue #769 (no labels, OPEN): Current dev review: Deploy server | done | 2026-06-07 |  |
| 1 | triage | `triage-issue-789` | Issue #789 (no labels, OPEN): Dev review rounds 6-10: scheduler, notifications, admin, orgs, buildin | done | 2026-06-07 |  |
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
| 1 | triage | `triage-issue-749` | Issue #749 (no labels, OPEN): Code review findings: Story 6.1 announcement creation and targeting | done | 2026-06-06 |  |
| 1 | triage | `triage-issue-755` | Issue #755 (no labels, OPEN): Current dev review: Epic 8A Notification Preferences | done | 2026-06-06 |  |
| 1 | triage | `triage-issue-764` | Issue #764 (no labels, OPEN): Current dev review: Admin MFA & Auth Hardening | done | 2026-06-06 |  |
| 1 | triage | `triage-issue-765` | Issue #765 (no labels, OPEN): Current dev review: Integrations & Airbnb OAuth | done | 2026-06-06 |  |
| 1 | bug | `bug-mobile-voting-hardcoded-locale` | Mobile VotingScreen hardcodes en-US in toLocaleDateString — vote dates never localize | done | 2026-06-05 |  |
| 1 | bug | `bug-reality-web-listing-metadata-ssr-throw` | Reality-web listing generateMetadata can throw during SSR on malformed 200 body | done | 2026-06-05 |  |
| 1 | security | `security-pkce-oauth-authcode-pr-908-closed` | PR #908 (fix(security): require PKCE on OAuth authorization-code flow, closes #823) was closed unmer | done | 2026-06-03 |  |
| 1 | triage | `triage-issue-751` | Issue #751 (no labels, OPEN): Current dev review: frontend/web/API-client findings | done | 2026-06-02 |  |
| 1 | triage | `triage-issue-752` | Issue #752 (no labels, OPEN): Current dev review: mobile CI tooling findings | done | 2026-06-02 |  |
| 1 | triage | `triage-issue-756` | Issue #756 (no labels, OPEN): Current dev review: Epic 10A OAuth Provider | done | 2026-06-02 |  |
| 1 | triage | `triage-issue-761` | Issue #761 (no labels, OPEN): Current dev review: Epic 84 E-Signature & Leases | done | 2026-06-02 |  |
| 1 | triage | `triage-issue-763` | Issue #763 (no labels, OPEN): Current dev review: Reality Server & Inquiries | done | 2026-06-02 |  |
| 1 | triage | `triage-issue-767` | Issue #767 (no labels, OPEN): Current dev review: Mobile RN Property Management app | done | 2026-06-02 |  |
| 1 | triage | `triage-issue-768` | Issue #768 (no labels, OPEN): Current dev review: Admin-web features (10B) | done | 2026-06-02 |  |
| 1 | triage | `triage-issue-920` | Issue #920 (no labels, OPEN): Announcement targeting not enforced on read (intra-org disclosure) | done | 2026-06-02 |  |
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
| 1 | triage | `triage-issue-778` | Issue #778 (no labels, OPEN): Current dev review: Marketplace, voting, investor portal, impersonatio | done | 2026-06-01 |  |
| 1 | triage | `triage-issue-788` | Issue #788 (no labels, OPEN): Dev review rounds 1-5: mobile-native + ppt-web surfaces | done | 2026-06-01 |  |
| 1 | triage | `triage-issue-790` | Issue #790 (no labels, OPEN): Dev review rounds 11-15: vendor, predictive, reality-web, middleware | done | 2026-06-01 |  |
| 1 | triage | `triage-issue-791` | Issue #791 (no labels, OPEN): Dev review rounds 16-20: push, e-sign, portal, webhooks, reserves | done | 2026-06-01 |  |
| 1 | triage | `triage-issue-846` | Issue #846 (no labels, OPEN): Code review: Epics 12+65 — Meters & Energy/ESG (origin/dev) | done | 2026-06-01 |  |
| 1 | triage | `triage-issue-847` | Issue #847 (no labels, OPEN): Code review: Reality-server — Inquiries IDOR (Epics 16–19) (origin/dev | done | 2026-06-01 |  |
| 1 | triage | `triage-issue-848` | Issue #848 (no labels, OPEN): Code review: Epics 78+134 — Vendor portal stubs & Predictive maintenan | done | 2026-06-01 |  |
| 1 | triage | `triage-issue-850` | Issue #850 (no labels, OPEN): Code review: Epics 61+146+42 — Multi-currency, Data residency, Violati | done | 2026-06-01 |  |
| 1 | triage | `triage-issue-851` | Issue #851 (no labels, OPEN): Code review: Epics 15+105+69 — Listings/syndication & Developer API st | done | 2026-06-01 |  |
| 1 | triage | `triage-issue-859` | Issue #859 (no labels, OPEN): sqlx 0.9 breaks runtime decode of Postgres enum columns into Rust Stri | done | 2026-06-01 |  |
| 1 | triage | `triage-issue-867` | Issue #867 (no labels, OPEN): Tech debt: api-server main.rs duplicates lib.rs::create_router — route | done | 2026-06-01 |  |
| 1 | triage | `triage-issue-836` | Issue #836 (no labels, OPEN): Code review: Epic 2B-C — Mobile push & device registration (origin/dev | done | 2026-05-31 |  |
| 1 | triage | `triage-issue-845` | Issue #845 (no labels, OPEN): Code review: Epic 14 — IoT alerts, correlations, thresholds (origin/de | done | 2026-05-31 |  |
| 1 | triage | `triage-issue-849` | Issue #849 (no labels, OPEN): Code review: Epic 10B+143 — Admin impersonation, Help, Board meetings  | done | 2026-05-31 |  |
| 0 | dx | `dx-routine-lag-catchup-2026-07` | Cloud routine cadence recovery — reduce 3–4d gaps between runs | dropped | 2026-07-09 |  |
| 0 | refactor | `churn-hotspot-backend-servers-api-server-src-routes-emergency-rs` | Churn hotspot: 1021 lines changed in backend/servers/api-server/src/routes/emergency.rs (window 2026 | dropped | 2026-07-05 |  |
| 0 | refactor | `churn-hotspot-backend-servers-api-server-src-routes-vendors-rs` | Churn hotspot: 929 lines changed in backend/servers/api-server/src/routes/vendors.rs (window 2026-06 | dropped | 2026-07-05 |  |
| 0 | refactor | `churn-hotspot-backend-servers-api-server-src-routes-enhanced-tenant-screening-rs` | Churn hotspot: 709 lines changed in backend/servers/api-server/src/routes/enhanced_tenant_screening. | dropped | 2026-07-05 |  |
| 0 | refactor | `churn-hotspot-backend-crates-db-src-repositories-document-rs` | Churn hotspot: 2940 lines changed in backend/crates/db/src/repositories/document.rs (window 2026-06- | dropped | 2026-07-05 |  |
| 0 | refactor | `churn-hotspot-backend-crates-db-src-repositories-subscription-rs` | Churn hotspot: 2856 lines changed in backend/crates/db/src/repositories/subscription.rs (window 2026 | dropped | 2026-07-05 |  |
| 0 | refactor | `churn-hotspot-backend-servers-api-server-src-routes-aml-dsa-rs` | Churn hotspot: 2691 lines changed in backend/servers/api-server/src/routes/aml_dsa.rs (window 2026-0 | dropped | 2026-07-05 |  |
| 0 | triage | `triage-issue-1151` | Issue #1151 (no labels, OPEN): Research dispatcher: claimable buffer is stale — true claimable work  | dropped | 2026-07-05 |  |
| 0 | refactor | `churn-hotspot-mobile-native-search-screen-kt` | Churn hotspot: SearchScreen.kt — +1293 LOC this run (gap-82-3 reality mobile search/filters) | dropped | 2026-07-05 |  |
| 0 | refactor | `code-review-mobile-native-kmp-deeplink-android-bypasses-shared-router` | MainActivity reimplements deep-link dispatch instead of calling shared DeepLinkRouter — drift trap | dropped | 2026-07-05 |  |
| 0 | refactor | `refactor-churn-hotspot-mobile-announcements` | Churn hotspot: AnnouncementsScreen.tsx — 4 PRs this run, instability proxy | dropped | 2026-07-05 |  |
| 0 | refactor | `refactor-churn-hotspot-mobile-announcements-test` | Churn hotspot: AnnouncementsScreen.test.ts — 4 PRs this run, instability proxy | dropped | 2026-07-05 |  |
| 0 | triage | `triage-dispatcher-mcp-push-large-file-issue-1014` | Dispatcher action-list.json corruption when MCP push falls back from blocked git push | dropped | 2026-07-05 |  |
| 0 | triage | `triage-issue-951` | Issue #951 (no labels, OPEN): Deploy blocker: api-server requires ESIGN_TOKEN_SECRET + ESIGN_WEBHOOK | dropped | 2026-07-05 |  |
| 0 | dx | `closed-not-merged-pr-1274` | PR #1274 (cargo-minor-patch group, /backend, 9 updates) closed unmerged — superseded by #1313 after  | dropped | 2026-07-05 |  |
| 0 | refactor | `churn-hotspot-backend-crates-integrations-src-booking-rs` | Churn hotspot: backend/crates/integrations/src/booking.rs (+404/−29 in PR #1294 Booking.com OTA retr | dropped | 2026-07-05 |  |
| 0 | refactor | `churn-hotspot-backend-servers-api-server-src-routes-api-ecosystem-rs` | Churn hotspot: backend/servers/api-server/src/routes/api_ecosystem.rs (+106/−27 in PR #1293 PAP-171; | dropped | 2026-07-05 |  |
| 0 | refactor | `churn-hotspot-backend-crates-db-src-repositories-reality-portal-rs` | Churn hotspot: backend/crates/db/src/repositories/reality_portal.rs (+59/−28 in PR #1297 PAP-142 IDO | dropped | 2026-07-05 |  |
| 0 | refactor | `churn-hotspot-backend-servers-api-server-src-routes-iot-rs` | Churn hotspot: backend/servers/api-server/src/routes/iot.rs (+278/-403 in PR #1321/#1322 PAP-151 re- | dropped | 2026-07-05 |  |
| 0 | refactor | `churn-hotspot-backend-servers-api-server-src-routes-reserve-funds-rs` | Churn hotspot: backend/servers/api-server/src/routes/reserve_funds.rs (+228/-255 in PR #1321 PAP-151 | dropped | 2026-07-05 |  |
| 0 | refactor | `churn-hotspot-backend-crates-db-src-repositories-sensor-rs` | Churn hotspot: backend/crates/db/src/repositories/sensor.rs (+248/-86 in PR #1321/#1322 PAP-151 re-l | dropped | 2026-07-05 |  |
| 0 | triage | `triage-issue-1331` | Issue #1331 (no labels, OPEN): Backend `test` job red/hanging on dev base — blocks the entire backen | dropped | 2026-07-05 |  |
| 0 | dx | `dx-stalled-review-pr-988` | Stalled review: PR #988 (Epic: reusable Playwright E2E framework + sitemap FlowRunner) open 10d, no  | dropped | 2026-07-05 |  |
| 0 | refactor | `churn-hotspot-backend-servers-api-server-src-routes-forms-rs` | Churn hotspot: backend/servers/api-server/src/routes/forms.rs touched 2x since 2026-06-12 (window 20 | dropped | 2026-07-05 |  |
| 0 | refactor | `churn-hotspot-backend-servers-api-server-tests-reserve-funds-cross-org-idor-tests-rs` | Churn hotspot: backend/servers/api-server/tests/reserve_funds_cross_org_idor_tests.rs touched 2x sin | dropped | 2026-07-05 |  |
| 0 | refactor | `churn-hotspot-backend-crates-db-tests-form-rls-repo-tests-rs` | Churn hotspot: backend/crates/db/tests/form_rls_repo_tests.rs touched 2x since 2026-06-12 (window 20 | dropped | 2026-07-05 |  |
| 0 | triage | `triage-issue-1380` | Issue #1380 (no labels, OPEN): Dispatcher stale gap-scan buffer + Tier-2 escalation endpoint misconf | dropped | 2026-07-05 |  |
| 0 | refactor | `churn-hotspot-frontend-apps-mobile-app-config-icon-test-ts` | Churn hotspot: 124 lines in frontend/apps/mobile/app.config.icon.test.ts (PR #1383 gap-85-2) | dropped | 2026-07-05 |  |
| 0 | refactor | `churn-hotspot-frontend-apps-mobile-app-config-ts` | Churn hotspot: 94 lines in frontend/apps/mobile/app.config.ts (PR #1383 gap-85-2) | dropped | 2026-07-05 |  |
| 0 | refactor | `churn-hotspot-backend-crates-db-src-repositories-form-rs` | Churn hotspot: 53 lines in backend/crates/db/src/repositories/form.rs (PR #1379 #1332 unblock) | dropped | 2026-07-05 |  |
| 0 | refactor | `churn-hotspot-backend-servers-api-server-tests-booking-oauth-csrf-tests-rs` | booking_oauth_csrf_tests.rs hotspot — 484-line NEW test file (PR #1393 #1424 OAuth CSRF coverage) | dropped | 2026-07-05 |  |
| 0 | refactor | `churn-hotspot-backend-servers-api-server-tests-booking-oauth-routes-tests-rs` | booking_oauth_routes_tests.rs hotspot — 381-line NEW test file (PR #1393 OAuth routes coverage) | dropped | 2026-07-05 |  |
| 0 | refactor | `repeated-churn-backend-servers-api-server-src-routes-forms-rs` | forms.rs repeated-churn — runs_seen=2 (#1337 explicit_auto_deref + #1397 org-scope hardening) | dropped | 2026-07-05 |  |
| 0 | dx | `closed-not-merged-pr-1425` | PR #1425 (GH #1377 document presigned-URL tests) closed unmerged — superseded by merged #1394 | dropped | 2026-07-05 |  |
| 0 | dx | `dx-stalled-review-pr-1179` | PR #1179 (docs(epics) catalog backfill for 37 mounted-but-undocumented backend modules) — stalled at | dropped | 2026-07-05 |  |
| 0 | refactor | `refactor-oauth-integration-tests-hot` | Stabilize oauth_integration_tests churn — heavy edits across 3 OAuth fix PRs | dropped | 2026-06-16 |  |
| 0 | triage | `triage-issue-779` | Issue #779 (no labels, OPEN): Current dev review: consolidated priority rollup (origin/dev snapshot) | dropped | 2026-06-13 |  |
| 0 | bug | `bug-announcer-stale-message` | Announcer: untracked clear-then-set timeouts can resurrect a stale screen-reader message | dropped | 2026-06-07 |  |
| 0 | dx | `dx-portfolio-dashboard-stubs` | Portfolio dashboard: alert mark-read/resolve mutations + property-card click navigation are no-op st | dropped | 2026-06-04 |  |
| — | — | `code-review-api-handlers-share-log-proxy-ip` | code-review-api-handlers-share-log-proxy-ip | — | — |  |
