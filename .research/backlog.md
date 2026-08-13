# Backlog of vectors

<sub>Last regenerated: 2026-08-13 17:35 UTC by routine</sub>

| Score | Vector | Title | Status | Confidence | Plan | Sources | Updated |
|-------|--------|-------|--------|------------|------|---------|---------|
| 6 | test-gap | Add regression tests for inquiry mark_as_read cross-tenant IDOR fix (PR #497) | done | high | plans/_archive/test-gap-inquiry-idor-regression.md | PR #497, PR #507, PR #548 | 2026-05-26 |
| 5 | security | SECURITY: Alexa voice webhook accepts forged requests — verify_alexa_signature never checks the signature | dropped | high |  | standing-scan-2026-07-28 | 2026-07-28 |
| 3 | bug | mobile-rn: offline sync queue treats ALL 4xx (incl. recoverable 401 expired-token / 429 rate-limit) as permanent and drops the queued action, silently losing offline-created content | done | medium |  | PR #2738, Tier1d review 2026-08-12 (mobile-rn) | 2026-08-12 |
| 3 | bug | ppt-web logout cache purge still misses the 'notification-triggers' query root (PR #2650 fix incomplete) — user notification-preference cache leaks into the next session | done | high |  | Tier1d review 2026-08-12 (ppt-web-core) | 2026-08-12 |
| 3 | bug | dev CI broken: duplicate SQLx migration 00220 — renumber portal_get_listing_view_count to 00227 | done | high | plans/_archive/gh-issue-2699-migration-collision.md | Issue #2699, PR #2692 (2026-08-07 00:08), PR #2700 | 2026-08-07 |
| 3 | bug | ppt-web-core: the client heartbeat sends an APPLICATION-level ping frame {type:'ping', pa... | done | high | plans/code-review-ppt-web-core-ws-pong-timeout-drop.md | rotating-expert-review | 2026-08-06 |
| 3 | security | reality-server sync_session swallows invalidate_session error — portal session survives after PM token goes inactive | dropped | high | plans/code-review-reality-server-sso-session-invalidate-swallowed.md | rotating-expert-review reality-server 2026-08-01 | 2026-08-02 |
| 3 | bug | reality-server GET /api/v1/agencies/{id}/members has no auth or membership check — unauthenticated cross-agency member enumeration (IDOR) | dropped | high |  | rotating-expert-review, segment:reality-server, expert:rust | 2026-08-02 |
| 3 | security | api-server workflow api_call.rs has a duplicate SSRF validator that drifts from common::url_validation — plain HTTP is allowed in prod | done | high |  | PR #2627, rotating-expert-review api-core 2026-08-02 | 2026-08-02 |
| 3 | bug | Scheduler notifications fire-once: transient target-resolution or dispatch error permanently drops announcement / vote notifications | dropped | high | plans/bug-scheduler-notifications-fire-once.md | Issue #2612, PR #2608 | 2026-08-01 |
| 3 | bug | uploadDocumentDirect() silently drops building_id — building-scoped document uploads lose association vs legacy multipart path | dropped | high |  | Issue #2366, PR #2345 | 2026-08-01 |
| 3 | refactor | auth.rs repeated-churn — runs_seen=4, 2950 lines / ~107K in one module (2nd-largest route file) | dropped | high |  | standing-scan-2026-07-28, state.json hotspot_history | 2026-07-28 |
| 3 | dx | SDK drift gate is effectively unenforced — api-validation.yml only fires on docs/api/**, so committed @ppt/api-client drift sits on dev unseen | done | high |  | standing-scan-2026-07-28, gh-run-30399201262 | 2026-07-28 |
| 3 | security | SECURITY: Android SSO deep-link handler skips CSRF state check — reality://sso?token=... enables account takeover | done | high | plans/code-review-mobile-native-kmp-android-sso-deeplink-missing-csrf-state.md | Phase 1.5 rotating expert review 2026-07-27 (mobile-native-kmp segment) | 2026-07-27 |
| 3 | security | SECURITY: community.rs get_group/list_posts/get_item run unauthenticated — anonymous cross-tenant read | dropped | medium | plans/code-review-api-handlers-community-unauthenticated-reads.md | Phase 1.5 rotating expert review 2026-07-23 (api-handlers segment) | 2026-07-23 |
| 3 | security | SECURITY: community.rs 5 write handlers (create_post/add_reaction/create_comment/rsvp_event/create_inquiry) accept cross-tenant IDs | done | medium | plans/code-review-api-handlers-community-cross-tenant-idor.md | Phase 1.5 rotating expert review 2026-07-23 (api-handlers segment) | 2026-07-23 |
| 3 | bug | revoke_all_sessions ignores refresh cookie — signs the caller out too | done | high | plans/bug-revoke-all-sessions-cookie-blindness.md | Phase 1.5 code-review 2026-07-09 (api-handlers segment) | 2026-07-09 |
| 3 | bug | ReportFaultScreen.tsx handleSubmit() fakes API call with setTimeout(1500) — fault reports never reach backend (App.tsx:126 wires this) | dropped | high | plans/code-review-mobile-rn-report-fault-fake-submit.md | Phase 1.5 review of mobile-rn segment (2026-06-16) | 2026-06-16 |
| 3 | bug | Reality-web RealtorManagement.tsx hardcoded English strings — agency flow not localized to sk/cs/de | done | high | plans/code-review-reality-web-realtor-mgmt-untranslated.md | rotating-expert-review reality-web 2026-06-14 | 2026-06-15 |
| 3 | bug | Reality-web ComparisonUrlHandler hits non-existent /api/listings/${id} — every shared comparison URL 404s | dropped | high | plans/code-review-reality-web-share-comparison-404.md | rotating-expert-review reality-web 2026-06-14 | 2026-06-14 |
| 3 | bug | Reality-web listing detail SSR crashes on partial 200 body — JSON-LD build deref of undefined fields | dropped | high | plans/code-review-reality-web-listing-page-ssr-crash.md | rotating-expert-review reality-web 2026-06-14 | 2026-06-14 |
| 3 | bug | iOS SearchView.swift does not compile — performSearch/scheduleSearch undefined, resultsGrid corrupted | dropped | high | plans/bug-ios-searchview-uncompilable.md | issue #1266, PR #1257 (verify) | 2026-06-11 |
| 3 | security | PR #1203 (fix(aml_dsa): close cross-tenant IDOR in moderation + AML-review handlers (PAP-36)) merged | dropped | medium |  | PR #1203 | 2026-06-10 |
| 3 | security | PR #1193 (fix(aml-dsa): lock DSA reports to platform roles + fix file-path disclosure (PAP-47)) merg | dropped | medium |  | PR #1193 | 2026-06-10 |
| 3 | bug | Schema drift: runtime SQL errors from non-existent columns in voting/messaging/notification paths | done | high |  | Issue #1008, PR #1009, PR #1040 | 2026-06-07 |
| 3 | security | IDOR: ai.rs LLM-doc handlers publish/list/get any tenant's listing descriptions & photo enhancements unscoped | dropped | high | plans/security-llm-doc-idor.md | code-review api-core 2026-05-29, ai.rs:2620, ai.rs:2599, ai.rs:2847, PR #879 | 2026-06-01 |
| 3 | security | IDOR: reality-server realtors mark_inquiry_read flips any realtor's inquiry by ID with no owner scoping | done | high | plans/_archive/security-realtors-mark-inquiry-read-idor.md | issue #519, PR #508, realtors.rs:250, PR #548 | 2026-05-26 |
| 3 | security | IDOR: equipment delete/update + maintenance update mutate any tenant's equipment by ID with no org scoping | done | high | plans/_archive/security-equipment-idor.md | code-review api-core 2026-05-25, ai.rs:1133, equipment.rs:144 | 2026-05-25 |
| 3 | security | SSRF: signed-document fetch + webhook-test POST issue outbound requests to unvalidated user-controlled URLs | done | high | plans/_archive/security-ssrf-outbound-url-validation.md | issue #439, signatures.rs:628, integrations.rs:2743, PR #450 | 2026-05-25 |
| 3 | security | IDOR: unlink_voice_device deactivates any device by ID with no owner/org scoping | done | high | plans/_archive/security-voice-device-idor.md | code-review api-core 2026-05-23, ai.rs:3002, PR #461 | 2026-05-25 |
| 2 | security | api-handlers: share password endpoint has no rate-limit throttle | open | medium |  | rotating-expert-review | 2026-08-13 |
| 2 | test-gap | add regression test for voice webhooks Alexa branch fail-closed (PR #2748) | open | high |  | PR #2748 | 2026-08-13 |
| 2 | bug | usePerformanceMetrics never removes its visibilitychange/load listeners on cleanup — event-listener leak, compounded by effect re-running on inline onReport | done | medium |  | Tier1d review 2026-08-12 (ppt-web-core) | 2026-08-12 |
| 2 | bug | mobile-native-kmp: shared repositories swallow CancellationException in catch(e: Exception), breaking coroutine cancellation and showing spurious errors | open | medium |  | Tier1d review 2026-08-11 (mobile-native-kmp) | 2026-08-11 |
| 2 | bug | Voice check-announcements & check-meter fabricate success with empty data — residents told 'no new announcements'/'no pending readings' without any query | dropped | medium |  | tier1d-dispatcher-generator, api-core segment review 2026-08-10 | 2026-08-10 |
| 2 | bug | Quiet-hours drain marks held push released even when delivery failed (sent=0) — held notification permanently lost on transient failure | dropped | medium |  | tier1d-dispatcher-generator, api-core segment review 2026-08-10 | 2026-08-10 |
| 2 | bug | Reports CSV export writes user-authored vote titles unescaped — spreadsheet formula injection (bypasses the repo's own sanitize_csv_cell) | done | medium |  | tier1d-dispatcher-generator, rotating-expert-review, api-handlers segment review 2026-08-10 | 2026-08-10 |
| 2 | bug | reality-web ComparisonUrlHandler uses Promise.all — one bad listing id blanks the whole shared comparison URL | done | high |  | Tier1d review 2026-08-07 (reality-web), PR #2701 | 2026-08-07 |
| 2 | bug | reality-web ListingForm posts NaN for area/rooms — non-numeric or negative input coerced silently | done | high |  | Tier1d review 2026-08-07 (reality-web), PR #2702 | 2026-08-07 |
| 2 | bug | api-handlers: enhanced_chat is a stubbed production handler that returns fabricated data.... | done | high |  | tier1d-dispatcher-generator | 2026-08-06 |
| 2 | bug | mobile-rn: the cold-start initialize() effect reads the stored access token (SecureSto... | done | medium |  | tier1d-dispatcher-generator | 2026-08-06 |
| 2 | bug | ppt-web-core: once reconnectAttempts reaches maxReconnectAttempts (default 10, :290) sche... | done | medium |  | rotating-expert-review | 2026-08-06 |
| 2 | bug | reality-server: InquiryResult::RateLimited variant is defined but never constructed or matc... | done | medium |  | tier1d-dispatcher-generator | 2026-08-06 |
| 2 | bug | reality-server: the LIVE public listing-detail handler get_listing() (route wired at routes... | done | high |  | tier1d-dispatcher-generator | 2026-08-06 |
| 2 | bug | mobile: useOfflineSupport — transient (5xx/network) queued actions are permanently DROPPED after 3 retries with only console.error — silent loss of offline-created content | done | medium |  | rotating-expert-review (tier1d mobile-rn), PR #2645 | 2026-08-04 |
| 2 | bug | ppt-web logout leaves 3 tenant-scoped TanStack Query roots un-purged (predictive-maintenance, sentiment, notification-analytics) — cross-user data leak on shared workstations | done | medium |  | rotating-expert-review (tier1d ppt-web-core), PR #2650 | 2026-08-04 |
| 2 | bug | ppt-web rentals + financial useMutation hooks lack onError — money-movement + platform-connect failures produce zero user feedback | done | medium |  | rotating-expert-review (tier1d ppt-web-core), PR #2649 | 2026-08-04 |
| 2 | bug | mobile-native-kmp: getPortfolioAnalytics() truncates realtor portfolio at 100 listings — dashboard under-reports on large portfolios | open | medium |  | Tier1d review 2026-08-04 (mobile-native-kmp) | 2026-08-04 |
| 2 | test-gap | screen-map drift: PR #2646 touched ppt-web route wrapper (App.tsx) without updating docs/screens/ppt/ | dropped | medium |  | PR #2646 | 2026-08-04 |
| 2 | test-gap | screen-map drift: PR #2647 touched 8 ppt-web route wrappers (i18n not-found fallbacks) without updating docs/screens/ppt/ | dropped | medium |  | PR #2647 | 2026-08-04 |
| 2 | test-gap | screen-map drift: PR #2648 touched ppt-web rentals route (mutation auth guard) without updating docs/screens/ppt/ | done | medium |  | PR #2648 | 2026-08-04 |
| 2 | test-gap | screen-map drift: PR #2649 touched ppt-web rentals + financial routes (onError toasts) without updating docs/screens/ppt/ | done | medium |  | PR #2649 | 2026-08-04 |
| 2 | bug | AccountingInvoiceManagementPage: create/delete mutations have no onError — silent invoice failures | done | medium |  | rotating-expert-review (tier1d ppt-web-ui) | 2026-08-03 |
| 2 | bug | ppt-web getApiClient() sends requests with no Authorization header — configureApiClient is never called, so the token provider stays undefined | done | high |  | rotating-expert-review (tier1d ppt-web-core) | 2026-08-03 |
| 2 | bug | ppt-web axios retry interceptor retries non-idempotent POST/PUT on 5xx / network errors — risk of duplicate server-side writes | done | medium |  | rotating-expert-review (tier1d ppt-web-core) | 2026-08-03 |
| 2 | bug | mobile: useOfflineSupport.processQueue reports isComplete:true after a head-of-line-blocked (halted) sync cycle | done | medium |  | rotating-expert-review (tier1d mobile-rn) | 2026-08-03 |
| 2 | bug | mobile: ThreadDetailScreen.handleSend send-message mutation has onSuccess only, no onError — failed sends are silent | done | medium |  | rotating-expert-review (tier1d mobile-rn) | 2026-08-03 |
| 2 | bug | ppt-web has no route-outlet ErrorBoundary — a single stale-chunk lazy() rejection unmounts the entire application shell | done | medium |  | rotating-expert-review (tier1d ppt-web-core) | 2026-08-03 |
| 2 | bug | ppt-web rentals mutations dereference `auth!` non-null — mid-session token loss throws uncaught TypeError instead of handled auth error | done | medium |  | rotating-expert-review (tier1d ppt-web-core) | 2026-08-03 |
| 2 | test-gap | docs/screens/reality/agency-import + agency-inquiries out of sync with PR #2636 i18n rewrite | done | medium |  | PR #2636 | 2026-08-03 |
| 2 | security | reality-server leaks raw sqlx::Error strings to internet-facing clients, bypassing util::errors::db_error | dropped | high | plans/code-review-reality-server-db-error-leak-to-client.md | rotating-expert-review reality-server 2026-08-01 | 2026-08-02 |
| 2 | security | Idempotency middleware trusts client-supplied X-Tenant-ID header for cache-scope key — cross-tenant collision or bypass | done | medium |  | PR #2626, rotating-expert-review api-core 2026-08-02 | 2026-08-02 |
| 2 | test-gap | /disputes/kpis: no window_start<=window_end validation and only test is BIT-440 quarantined | dropped | high |  | Issue #2575, PR #2572 | 2026-08-01 |
| 2 | refactor | Churn hotspot: backend/crates/integrations/src/booking/mod.rs — 3626 lines this window (recently split by PR #2611) | done | high |  | PR #2611, commit window 2026-07-30→2026-07-31 | 2026-07-31 |
| 2 | refactor | Churn hotspot: backend/servers/api-server/src/routes/reports.rs — 3329 lines this window (PR #2599 extracted helpers) | dropped | high |  | PR #2599, commit window 2026-07-30→2026-07-31 | 2026-07-31 |
| 2 | refactor | Churn hotspot: backend/servers/api-server/src/routes/auth.rs — 2950 lines this window (runs_seen=5, no refactor PR yet) | dropped | high |  | commit window 2026-07-30→2026-07-31 | 2026-07-31 |
| 2 | test-gap | Screen-map drift: reality-web layout changed without docs/screens/reality/ update (PR #2600) | done | medium |  | PR #2600 | 2026-07-31 |
| 2 | dx | admin-web mobile-config Save flow blocked: PATCH /api/v1/admin/mobile-config endpoint missing | dropped | medium |  | PR #2594 (commit fb09556), PR #2601 | 2026-07-31 |
| 2 | dx | admin-web platform-settings Save blocked: PATCH /api/v1/platform-admin/settings endpoint missing | dropped | medium |  | PR #2594 (commit fb09556), PR #2601 | 2026-07-31 |
| 2 | dx | @ppt/ui-kit missing primitives (Stepper, FileUpload, RadioCards, StatusPill) — admin-web ships inline duplicates | done | medium |  | PR #2594 (commit fb09556) | 2026-07-31 |
| 2 | dx | openapi-ts generator wrapper swallows errors and emits weak error types | done | medium |  | PR #2607 (commit 5c604d5) | 2026-07-31 |
| 2 | bug | SECURITY-LITE: layout/resolved.rs err500 handler leaks raw sqlx/serde error text on public GET /layout/resolved/{screen} — 8 sites, no server-side log | done | medium |  | rotating-expert-review api-core 2026-07-30 | 2026-07-30 |
| 2 | security | SECURITY: reality-web layout.tsx inlines tenant-config JSON into <script> without </script>/U+2028/U+2029 escaping — HTML injection window per request | done | medium |  | rotating-expert-review 2026-07-28, segment:reality-web | 2026-07-28 |
| 2 | test-gap | test-gap: voice_webhooks.rs (1148 lines, 6 mounted endpoints incl. OAuth token exchange) has no tests at all | done | high |  | standing-scan-2026-07-28 | 2026-07-28 |
| 2 | bug | AmlDashboardPage casts raw window.prompt text into the review-decision union — a typo submits an invalid AML decision | done | high |  | standing-scan-2026-07-28 | 2026-07-28 |
| 2 | dx | admin-web platform-settings + mobile-config Save paths are permanent no-ops — the backing endpoints do not exist | done | high |  | standing-scan-2026-07-28 | 2026-07-28 |
| 2 | refactor | Churn hotspot: backend/servers/api-server/src/routes/reports.rs — 3329 lines this window, runs_seen=3 (repeated instability) | done | high |  | Phase 1 churn scan 2026-07-27 (commit range 2026-07-24..2026-07-27 on dev) | 2026-07-27 |
| 2 | bug | PR #2547 shipped scheduler retention prune fix without an api-server regression test (hotfix-no-test) | done | high |  | PR #2547 (merged 2026-07-24), closes GitHub issue #2531 | 2026-07-27 |
| 2 | bug | AuthContext init bypasses refreshTokenInternal → stale role on cold-boot refresh (#574 fix gap) | done | medium |  | Phase 1.5 rotating expert review 2026-07-24 (ppt-web-core segment) | 2026-07-24 |
| 2 | bug | WebSocket not re-authed on token rotation — connect() early-return leaves live socket on old token | done | medium |  | Phase 1.5 rotating expert review 2026-07-24 (ppt-web-core segment) | 2026-07-24 |
| 2 | test-gap | screen-map drift: PR #2497 touched reality-web/app/api/layout-revalidate/route.ts w/o docs/screens/reality/ | done | medium |  | PR #2497 | 2026-07-24 |
| 2 | test-gap | Screen-map drift: PR #2431 touched reality-web/src/app/api/layout-revalidate/route.ts without updating docs/screens/reality/*.md (heuristic — internal API, not user-facing screen) | done | medium |  | PR #2431 | 2026-07-21 |
| 2 | bug | TenantSectionEditor PropInput silently JSON.parse-coerces every string prop on blur — override payload corrupted ("true" -> boolean, "[]" -> array) | done | medium |  | code-review ppt-web-ui 2026-07-21 | 2026-07-21 |
| 2 | bug | DashboardCustomizePage 'changed since sent' check is tautological — concurrent edits during in-flight save are silently discarded | done | medium |  | code-review ppt-web-ui 2026-07-21 | 2026-07-21 |
| 2 | bug | Dashboard useActionQueue queryFn returns generateMockData — production users see fabricated action items; approve/reject/dismiss are silent no-ops | done | medium |  | code-review ppt-web-ui 2026-07-21 | 2026-07-21 |
| 2 | security | scheduler.rs units/buildings target queries lack organization_id AND-scope — fan-out can leak across tenants if create-announcement validation is bypassed | done | medium |  | code-review api-core 2026-07-20 | 2026-07-20 |
| 2 | refactor | Churn hotspot cluster: api-server routes/auth.rs (runs_seen=3) + auth_tests.rs + reality-server routes/sso.rs | done | medium |  | PR #2205, PR #2250, PR #2261, PR #2270, PR #2271, PR #2254, commit 5877ade..ac3afb3 | 2026-07-12 |
| 2 | security | /forgot-password and /resend-verification have no rate limit — mailbomb / token-clobber | done | high | plans/security-forgot-password-no-rate-limit.md | Phase 1.5 code-review 2026-07-09 (api-handlers segment) | 2026-07-09 |
| 2 | test-gap | Reality-server listings pagination clamp (PR #959) shipped without a regression test for limit=-1 | done | high |  | PR #959, issue #953, PR #2073 | 2026-07-05 |
| 2 | test-gap | PR #1418 touched routes/** (faults.route.test.tsx) without updating docs/screens/ppt/* — heuristic, test-file fix | done | medium |  | PR #1418, PR #2070 | 2026-07-05 |
| 2 | bug | vote.rs:1765 calculate_question_result() uses partial_cmp().unwrap() on f64 — NaN/Inf weights panic /votes/{id}/results | done | high |  | code-review api-core 2026-06-15, PR #1417 | 2026-06-16 |
| 2 | test-gap | PR #1196 (feat(ppt-web): add missing test coverage for faults feature) merged with 2 unchecked TODO | dropped | medium |  | PR #1196 | 2026-06-10 |
| 2 | dx | PushFanoutWorker BLPOP queue-drain deferred — Redis path is a logging no-op | done | high |  | PR #515, push_fanout.rs:621, PR #1115 | 2026-06-06 |
| 2 | refactor | ai.rs (3,134 LOC) — explicit module-split into routes/ai/{sessions,equipment,workflows,voice,llm,mod}.rs | done | high |  | pm-tech-lead analysis 2026-05-25, security-voice-device-idor (PR #461), security-equipment-idor, PR | 2026-06-06 |
| 2 | refactor | announcements.rs churn-hot — 2,722 lines this run (Epic 2B + Epic 6 work) | done | medium |  | git log origin/dev since 2026-05-24, PR #504, PR #505, PR #548, PR #1110 | 2026-06-06 |
| 2 | refactor | announcements.rs (2,722 LOC) — explicit module-split into routes/announcements/{crud,targeting,delivery,reactions,mod}.rs | done | high |  | pm-tech-lead analysis 2026-05-25, PR #1110 | 2026-06-06 |
| 2 | refactor | Reduce App.tsx route-aggregator coupling (top churn hotspot, merge-conflict risk) | done | medium |  | PR #474, PR #475, PR #489, PR #511, PR #547, PR #549, PR #555, PR #1108 | 2026-06-06 |
| 2 | refactor | platform_admin.rs (2,762 LOC) — explicit module-split into routes/platform_admin/{tenants,features,billing,audit,mod}.rs | done | high |  | pm-tech-lead analysis 2026-05-25, PR #1109 | 2026-06-06 |
| 2 | test-gap | Screen-map drift: PR #1033 wired error/retry into AnnouncementsPage+FaultsPage via App.tsx without a docs/screens/ppt update | done | medium |  | PR #1033, PR #1111 | 2026-06-06 |
| 2 | bug | Risky churn: mobile App.tsx deep-link/doc-detail wiring changing across back-to-back PRs without coverage | done | medium |  | PR #1103, PR #962, PR #992 | 2026-06-05 |
| 2 | dx | Integration marketplace install/OAuth flows are placeholders — wire backend handlers + UI navigation | done |  |  | PR #1105, PR #282, PR #328, commit 254f01d, commit c97781a | 2026-06-05 |
| 2 | test-gap | Booking push availability/rates endpoints add batch-cap + non-negative guards with no regression test | done | high |  | PR #1068, PR #607, issue #572 | 2026-06-05 |
| 2 | test-gap | Portal webhook fail-closed fix (PR #874) shipped without a regression test for unverified-signature rejection | done | high |  | PR #1052, PR #874 | 2026-06-05 |
| 2 | test-gap | Mobile dev-review batch (PR #918, 5 files under frontend/apps/mobile/src) shipped without a regression test | done | high |  | PR #1072, PR #918 | 2026-06-05 |
| 2 | test-gap | Reality-server SSO consumer review fix (PR #921, closes #820) shipped without a regression test | done | high |  | PR #1076, PR #921 | 2026-06-05 |
| 2 | test-gap | CI branch-protection + auto-rebase workflow change (PR #923) shipped without an integration test | done | high |  | PR #1057, PR #923 | 2026-06-05 |
| 2 | test-gap | deploy-server OIDC scope mapping (#939) shipped without unit test for derive_oidc_scopes | done | high |  | PR #1106, PR #939 | 2026-06-05 |
| 2 | test-gap | Mobile RN dev-review tail (#943) shipped without test coverage | done | high |  | PR #1080, PR #943, issue #767 | 2026-06-05 |
| 2 | test-gap | Frontend gap-sweep (PR #990, 34 files across Epics 1/6/7B/9/10B/11/15/17/18) shipped without a regression test | done | high |  | PR #1081, PR #990 | 2026-06-05 |
| 2 | test-gap | Mobile document-detail wiring (PR #992) shipped without a regression test for the deep-link payload path | done | high |  | PR #1082, PR #992 | 2026-06-05 |
| 2 | test-gap | Screen-map drift: PR #839 modified ppt-web App.tsx (FileDisputePageRoute) without a docs/screens/ppt update | done | medium |  | PR #1056, PR #839 | 2026-06-05 |
| 2 | refactor | ppt-web status/auth components hardcode English in an otherwise i18n'd app | done | medium |  | code-review ppt-web-ui 2026-05-24, rotating-expert-review, PR #549, PR #1046 | 2026-06-04 |
| 2 | bug | MediationWorkspacePage shows empty/unknown state instead of error UI on dispute fetch failure | done | high |  | PR #555, code-review 2026-05-27, PR #1029 | 2026-06-03 |
| 2 | bug | Mobile VotingScreen double-casts API result across boundary — render-time crash on unexpected shape | done | medium |  | code-review mobile-rn 2026-05-27, rotating-expert-review, PR #1028 | 2026-06-03 |
| 2 | bug | Reality-web InviteRealtorModal swallows invite-mutation failure with no error UI | done | high |  | code-review reality-web 2026-05-28, rotating-expert-review, PR #1023 | 2026-06-03 |
| 2 | bug | Airbnb webhook at-least-once delivery enqueues duplicate SYNC_EXTERNAL jobs | done | high |  | PR #538, webhook.rs:1028, PR #841, PR #1030 | 2026-06-03 |
| 2 | dx | DocumentsBrowse MoveFolderDialog cannot pre-select current folder (DocumentSummary lacks folder_id) | done | high |  | PR #623, PR #1031 | 2026-06-03 |
| 2 | test-gap | API + SPA security-headers middleware (PR #963) shipped without an assertion test for HSTS/nosniff/CSP | done | high |  | PR #963, issue #954, PR #1021 | 2026-06-03 |
| 2 | test-gap | api-server main.rs vs lib.rs::create_router diverge silently (5 routes unreachable in prod, no test asserts parity) | done | high |  | PR #866, issue #867, issue #836, PR #870 | 2026-06-01 |
| 2 | bug | ReportSchedule.update_schedule stores cron in `time` workaround; documented UPDATE never runs (missing cron_expression column) | done | high |  | PR #611, issue #616, PR #643, PR #815 | 2026-05-30 |
| 2 | test-gap | Screen-map drift: report execution-history route (PR #547) added without a ppt screen doc | done | medium |  | PR #547, frontend/apps/ppt-web/src/routes/lazyRoutes.tsx, PR #623 | 2026-05-27 |
| 2 | test-gap | Dispute state machine (PR #506) shipped with no tests + no org predicate on update_status | done | high |  | PR #506, issue #520, PR #514, PR #548 | 2026-05-26 |
| 2 | refactor | documents.rs churn-hot — 10,659 lines over 14d | done | medium |  | git log origin/main since 2026-05-06, git log origin/dev since 2026-05-20, PR #456 | 2026-05-25 |
| 2 | refactor | integrations.rs churn-hot — 12,977 lines over 14d, candidate for module split | done | medium |  | git log origin/main since 2026-05-06, git log origin/dev since 2026-05-20, PR #456 | 2026-05-25 |
| 2 | refactor | organizations.rs churn-hot — 12,060 lines over 14d (multitenancy + admin) | done | medium |  | git log origin/main since 2026-05-06, git log origin/dev since 2026-05-20, PR #456 | 2026-05-25 |
| 2 | security | IDOR: reality-server mark_as_read flips any realtor's inquiry by ID with no owner scoping | done | high | plans/_archive/security-inquiry-read-idor.md | code-review reality-server 2026-05-23, inquiries.rs:554 | 2026-05-25 |
| 2 | security | Latent fail-open: ProtectedRoute role check is skipped when user.role is falsy | done | medium |  | code-review ppt-web-ui 2026-05-24, ProtectedRoute.tsx:117, PR #459 | 2026-05-25 |
| 2 | test-gap | Screen-map drift: PR #464 wired a neighbors route in ppt-web without a docs/screens/ppt entry | done | medium |  | PR #464 | 2026-05-25 |
| 2 | test-gap | Screen-map drift: PR #460 touched reality-web listing page without a docs/screens/reality update | closed | medium |  | PR #460 | 2026-05-25 |
| 2 | refactor | Dead/duplicate handler modules: AuthHandler & BuildingHandler unused, routes reimplement inline | done | medium |  | code-review api-handlers 2026-05-23, PR #437 | 2026-05-24 |
| 2 | security | Complete RLS migration in 31 remaining handlers (voting, market_pricing, faults, notif_prefs, reports) | done |  |  | issue #160, PR #420, PR #421 | 2026-05-23 |
| 1 | bug | reality-web: ComparisonView hardcoded strings — no useTranslations coverage | open | medium |  | rotating-expert-review | 2026-08-13 |
| 1 | bug | reality-web: realtor-management page hardcoded strings — i18n gap | open | medium |  | rotating-expert-review | 2026-08-13 |
| 1 | bug | reality-web: agency error-state UI hardcoded strings — i18n gap | open | medium |  | rotating-expert-review | 2026-08-13 |
| 1 | refactor | ppt-web-ui: ProtectedRoute hardcodes English 'Access Denied' / 'Loading...' strings instead of using i18n | done | medium |  | Tier1d review 2026-08-12 (ppt-web-ui) | 2026-08-12 |
| 1 | refactor | ppt-web-ui: OfflineIndicator banner text is hardcoded English, not i18n | done | medium |  | Tier1d review 2026-08-12 (ppt-web-ui) | 2026-08-12 |
| 1 | bug | mobile-rn: date formatters hardcode 'en-US' locale in many screens (incomplete fix of #2282), so dates never localize for sk/cs/de users | done | medium |  | Tier1d review 2026-08-12 (mobile-rn) | 2026-08-12 |
| 1 | bug | mobile-rn: MeterDetailScreen renders hardcoded English UI strings not wrapped in t(), while sibling strings use i18n | done | medium |  | Tier1d review 2026-08-12 (mobile-rn) | 2026-08-12 |
| 1 | test-gap | ppt-web rentals API↔UI mappers now covered by regression tests — monitor for further churn | done | high |  | PR #2735 | 2026-08-12 |
| 1 | test-gap | mobile useOfflineSupport now covered by retryable-4xx regression tests — verify future changes don't re-introduce data loss | done | high |  | PR #2738 | 2026-08-12 |
| 1 | refactor | api-server oauth.rs churning around token-usage recording — monitor for further refactor pressure | done | high |  | PR #2732 | 2026-08-12 |
| 1 | dx | stalled review: PR #2555 feat(acc) UC-ACC-05.17 wire sent/cancelled invoice lifecycle (15d open, 13d idle) | needs-human-judgement | medium |  | PR #2555 | 2026-08-12 |
| 1 | dx | stalled review: PR #2558 feat(acc) UC-ACC-05.9 invoice PDF render endpoint (15d open, 13d idle) | needs-human-judgement | medium |  | PR #2558 | 2026-08-12 |
| 1 | dx | stalled review: PR #2559 feat(acc) UC-ACC-05.8 PAY by square QR endpoint (15d open, 13d idle) | needs-human-judgement | medium |  | PR #2559 | 2026-08-12 |
| 1 | dx | closed-not-merged: PR #2705 dx-cnm-pr-2385-retry2 (rust-toolchain 1.94.1→1.100.0) — second retry closed unmerged, investigate root cause | done | medium |  | PR #2705 | 2026-08-12 |
| 1 | test-gap | mobile-native-kmp: SsoService (deep-link token exchange, login, password reset, session restore) has zero direct tests | open | medium |  | Tier1d review 2026-08-11 (mobile-native-kmp) | 2026-08-11 |
| 1 | bug | Notification pipeline swallows quiet-hours schedule DB error and fails open — push delivered during user's configured quiet hours | done | medium |  | tier1d-dispatcher-generator, api-core segment review 2026-08-10 | 2026-08-10 |
| 1 | bug | reality-web ListingForm hardcodes English throughout a next-intl sk/cs/de/en app | done | medium |  | Tier1d review 2026-08-07 (reality-web) | 2026-08-07 |
| 1 | refactor | Churn hotspot: backend/servers/api-server/src/services/workflow_executor.rs (+265/-192 in #2685 workflow NOT-group fix) | done | high |  | PR #2685 | 2026-08-07 |
| 1 | refactor | Churn hotspot: frontend/apps/ppt-web/src/lib/websocket.ts (+11/-94 heartbeat removal PR #2689) | dropped | high |  | PR #2689 | 2026-08-07 |
| 1 | refactor | Churn hotspot: backend/servers/api-server/src/routes/ai/llm.rs (fail-closed enhanced_chat #2688 + upstream error masking #2694) | done | high |  | PR #2688, PR #2694 | 2026-08-07 |
| 1 | security | api-handlers: the raw upstream LLM-provider error is forwarded verbatim into the client-f... | done | medium |  | tier1d-dispatcher-generator | 2026-08-06 |
| 1 | bug | mobile-rn: enableBiometric passes hardcoded English strings to the OS biometric dialog... | done | medium |  | tier1d-dispatcher-generator | 2026-08-06 |
| 1 | bug | reality-server: a shipped-but-non-functional notification path. | done | medium |  | tier1d-dispatcher-generator | 2026-08-06 |
| 1 | bug | reality-server: ListingHandler::get_listing() returns a PublicListingDetail with a whole bl... | done | medium |  | tier1d-dispatcher-generator | 2026-08-06 |
| 1 | bug | reality-server: schedule_viewing() runs full input validation (future-date at :300, <=90-da... | done | medium |  | tier1d-dispatcher-generator | 2026-08-06 |
| 1 | bug | mobile-native-kmp: getPortfolioAnalytics() fans out one analytics HTTP request per listing with no concurrency limit — up to 100 parallel GETs from a mobile device | dropped | medium |  | Tier1d review 2026-08-04 (mobile-native-kmp) | 2026-08-04 |
| 1 | refactor | Churn hotspot: 2 commits touching frontend/apps/ppt-web/src/routes/groups/rentals.tsx (window 2026-08-03T17:00Z → 2026-08-04T03:00Z) | dropped | high |  | local git log since 2026-08-03T17:00Z | 2026-08-04 |
| 1 | refactor | Churn hotspot: 2 commits touching frontend/apps/mobile/src/screens/messages/ThreadDetailScreen.tsx (window 2026-08-03T17:00Z → 2026-08-04T03:00Z) | done | high |  | local git log since 2026-08-03T17:00Z | 2026-08-04 |
| 1 | refactor | Churn hotspot: 2 commits touching frontend/apps/mobile/src/hooks/useOfflineSupport.ts (window 2026-08-03T17:00Z → 2026-08-04T03:00Z) | dropped | high |  | local git log since 2026-08-03T17:00Z | 2026-08-04 |
| 1 | bug | ppt-web: 103 hardcoded English aria-label attributes across feature components — screen readers hear English on sk/cs/de | done | medium |  | rotating-expert-review (tier1d ppt-web-ui) | 2026-08-03 |
| 1 | bug | SessionsPage double-casts through unknown to hand-maintained interface — defeats API-boundary type-check | done | medium |  | rotating-expert-review (tier1d ppt-web-ui) | 2026-08-03 |
| 1 | bug | reality-web: agency/import feature cluster hardcoded English — 65 sibling components use useTranslations | done | medium |  | rotating-expert-review (tier1d reality-web) | 2026-08-03 |
| 1 | bug | reality-web ProtectedRoute renders untranslated auth-required gate for sk/cs/de | done | medium |  | rotating-expert-review (tier1d reality-web) | 2026-08-03 |
| 1 | bug | mobile: empty/error/loading state strings hardcoded in English across ~18 screens despite react-i18next being wired | done | medium |  | rotating-expert-review (tier1d mobile-rn) | 2026-08-03 |
| 1 | bug | mobile: usePushNotifications — debug console.log left on the device-token register/unregister production path | done | medium |  | rotating-expert-review (tier1d mobile-rn) | 2026-08-03 |
| 1 | bug | ppt-web route wrappers render 8 hardcoded English 'X not found' fallbacks — sk/cs/de users see untranslated text | done | medium |  | rotating-expert-review (tier1d ppt-web-core) | 2026-08-03 |
| 1 | bug | reality-server password reset is non-functional in prod — token discarded, no email transport, endpoint claims a link was sent | dropped | high |  | rotating-expert-review reality-server 2026-08-01 | 2026-08-01 |
| 1 | refactor | Churn hotspot: backend/servers/api-server/src/services/scheduler.rs — 347 lines this window (PRs #2567, #2576) | done | high |  | PR #2567, PR #2576 | 2026-07-30 |
| 1 | refactor | Churn hotspot: backend/servers/api-server/src/routes/layout/tenant.rs — 262 lines this window (PR #2478) | dropped | high |  | PR #2478 | 2026-07-30 |
| 1 | refactor | Churn hotspot: backend/servers/api-server/src/routes/layout/admin.rs — 240 lines this window (PRs #2478, #2549) | dropped | high |  | PR #2478, PR #2549 | 2026-07-30 |
| 1 | bug | layout/admin.rs (+ tenant.rs) mutation handlers end with unwrap_or_default() serialize — a failed serialize returns 200 OK / body null instead of 500 | done | medium |  | rotating-expert-review api-core 2026-07-30 | 2026-07-30 |
| 1 | bug | scheduler.rs silently swallows DB errors on notification target lookups at 3 sites — failed dispatches show as empty target sets, no log/metric | done | medium |  | rotating-expert-review api-core 2026-07-30 | 2026-07-30 |
| 1 | refactor | reality-web listingAnalytics.ts casts untrusted ?source= query param straight to ViewSource union — pollutes listing.viewed analytics with unbounded cardinality | done | medium |  | rotating-expert-review 2026-07-28, segment:reality-web | 2026-07-28 |
| 1 | dx | Stale TODO(security) headers in faults.rs / critical_notifications.rs describe a hardcoded-false gate that no longer exists | done | high |  | standing-scan-2026-07-28 | 2026-07-28 |
| 1 | refactor | Churn hotspot: backend/crates/integrations/src/booking/mod.rs — 3185 lines this window (post-PR-#2176 tail) | done | high |  | Phase 1 churn scan 2026-07-27 (commit range 2026-07-24..2026-07-27 on dev) | 2026-07-27 |
| 1 | bug | PortfolioAnalytics inquiriesTrend silently drops days with inquiries but zero views (set-difference bug) | done | high |  | Phase 1.5 rotating expert review 2026-07-27 (mobile-native-kmp segment) | 2026-07-27 |
| 1 | refactor | 10 ungated console.warn/error in ppt-web websocket.ts leak diagnostics in prod | dropped | medium |  | Phase 1.5 rotating expert review 2026-07-24 (ppt-web-core segment) | 2026-07-24 |
| 1 | refactor | Churn hotspot: platform_admin_authz_batch2_tests.rs — 417 lines this run (BIT-557 test backfill) | done | high |  | git-log-since-2026-07-21T03:13:00Z | 2026-07-23 |
| 1 | refactor | Churn hotspot: org_property_authz_backfill_tests.rs — 412 lines this run (BIT-268/BIT-559 authz salvage) | done | high |  | git-log-since-2026-07-21T03:13:00Z | 2026-07-23 |
| 1 | refactor | Churn hotspot: infra_ops_authz_backfill_tests.rs — 364 lines this run (BIT-268 test backfill) | dropped | high |  | git-log-since-2026-07-21T03:13:00Z | 2026-07-23 |
| 1 | triage | PR #2489 closed unmerged: dependabot npm-minor-patch (5→4 update group) superseded by #2491 | dropped | high |  | PR #2489 | 2026-07-23 |
| 1 | refactor | Churn hotspot: docs/repo-map.md — 4 touches this window (per-PR route-map refresh) | done | high |  | git-log-since-2026-07-16, git-log-since-2026-07-20T03:12:00Z | 2026-07-21 |
| 1 | refactor | Churn hotspot: docs/screens/ppt/dashboard.md — 3 touches this run (Layout & Content Manager pilot integration) | done | high |  | git-log-since-2026-07-20T03:12:00Z | 2026-07-21 |
| 1 | refactor | Churn hotspot: frontend/apps/admin-web/src/features/layout-editor/LayoutEditorPage.tsx — 2 touches, 900 lines this run | done | high |  | git-log-since-2026-07-20T03:12:00Z | 2026-07-21 |
| 1 | bug | scheduler.rs get_announcement_target_users() silently swallows target_ids JSON parse errors — malformed payload publishes zero notifications with no log | done | high |  | code-review api-core 2026-07-20 | 2026-07-20 |
| 1 | refactor | Churn hotspot: frontend/apps/mobile/package.json — 5 touches this window (Expo/expo-notifications/expo-config-plugins dependabot cascade) | done | high |  | git-log-since-2026-07-16 | 2026-07-20 |
| 1 | refactor | Churn hotspot: backend/Cargo.toml — 3 touches this window (dependabot minor-patch cascade + layout-core crate) | done | high |  | git-log-since-2026-07-16 | 2026-07-20 |
| 1 | dx | PR #2385 (dependabot: dtolnay/rust-toolchain 1.94.1 → 1.100.0) closed unmerged — likely superseded by unrelated toolchain pin or GH-action security rollup | dropped | high |  | PR #2385 | 2026-07-20 |
| 1 | dx | PR #2387 (dependabot: npm-minor-patch 15-update rollup) closed unmerged — superseded by the 19-update rollup #2423 | dropped | high |  | PR #2387, PR #2423 | 2026-07-20 |
| 1 | refactor | Churn hotspot: frontend/apps/ppt-web/messages/en.json — frontend/apps/ppt-web/messages/en.json: +3926 lines this run (runs_seen was 0; last_seen never) | done | high |  | git-log-since-2026-07-13 | 2026-07-16 |
| 1 | refactor | Churn hotspot: frontend/packages/sitemap/src/json/sitemap.json — frontend/packages/sitemap/src/json/sitemap.json: +3727 lines this run (runs_seen was 0; last_seen never) | done | high |  | git-log-since-2026-07-13 | 2026-07-16 |
| 1 | refactor | backend integrations booking/mod.rs — instability watch after PR #2176 split | done | high |  | commits 2026-07-05..2026-07-09 | 2026-07-09 |
| 1 | test-gap | oauth_integration_tests.rs repeated-churn (runs_seen 2→3) — OAuth handlers still moving | dropped | high |  | commits 2026-07-05..2026-07-09 | 2026-07-09 |
| 1 | refactor | api-server routes/auth.rs — repeated hotspot + 3 static-review findings this run | done | high |  | commits 2026-07-05..2026-07-09 | 2026-07-09 |
| 1 | bug | /refresh and /logout — empty refresh_token cookie shadows valid body token | done | medium |  | Phase 1.5 code-review 2026-07-09 (api-handlers segment) | 2026-07-09 |
| 1 | bug | DeepLinkRouter skips URL-decoding while Android Uri.getQueryParameter decodes — SSO tokens diverge per platform | dropped | high |  | mobile-native-kmp segment review 2026-06-06 | 2026-07-05 |
| 1 | bug | SearchScreen stale-response race — overlapping searches can clobber newer results | dropped | high |  | mobile-native-kmp segment review 2026-06-06, PR #1125 | 2026-07-05 |
| 1 | test-gap | Screen-map drift: PR #1085 modified reality-web listing detail metadata + page without screen-doc update | dropped | medium |  | PR #1085 | 2026-07-05 |
| 1 | test-gap | Screen-map drift: PR #1100 modified ppt-web App.tsx (FileDisputePageRoute extraction) without screen-doc update | dropped | medium |  | PR #1100 | 2026-07-05 |
| 1 | bug | Risky churn: api-server main.rs security-headers wiring shipped without a middleware smoke test | dropped | medium |  | PR #963 | 2026-07-05 |
| 1 | test-gap | Screen-map drift: PR #922 modified ppt-web App.tsx (dev-review rounds 1-5 fixes) without a docs/screens/ppt update | done | medium |  | PR #922, PR #2075 | 2026-07-05 |
| 1 | refactor | Churn hotspot: ListingDetailScreen.kt — +1279 LOC this run (gap-82-4 reality mobile favorite toggle) | done | medium |  | PR #1121, PR #2059 | 2026-07-05 |
| 1 | refactor | Churn hotspot: DocumentsScreen.tsx — 3 PRs this run | done | high |  | PR #1101, PR #1081, PR #1082, PR #2077 | 2026-07-05 |
| 1 | bug | iOS deep-link layer dead at runtime — Info.plist missing CFBundleURLTypes + applinks entitlement | dropped | high |  | issue #1267, PR #1256 (verify) | 2026-07-05 |
| 1 | test-gap | Webhook handlers RLS migration (PR #1288, PAP-170) shipped without a new regression test for repo-layer methods | dropped | medium |  | PR #1288, PAP-170 | 2026-07-05 |
| 1 | test-gap | AI llm/sessions + integrations sync + subscriptions RLS migration (PR #1287, PAP-169) shipped without a new regression test | dropped | medium |  | PR #1287, PAP-169, PAP-150 | 2026-07-05 |
| 1 | test-gap | api_ecosystem.rs RLS migration (PR #1289, PAP-167) — 162-line handler rework shipped without a regression test for the public-connection routing | dropped | medium |  | PR #1289, PAP-167, PAP-150 | 2026-07-05 |
| 1 | test-gap | mfa.rs RLS migration (PR #1292, PAP-168) shipped without a regression test; also landed broken and was hotfixed in PR #1287 | dropped | medium |  | PR #1292, PR #1287, PAP-168, PAP-150 | 2026-07-05 |
| 1 | refactor | crypto.rs:127 SysRng.try_fill_bytes(...).expect() panics if OS CSPRNG errors during integration-credential encrypt | done | medium |  | code-review api-core 2026-06-15, PR #2074 | 2026-07-05 |
| 1 | bug | Mobile RN production screens (Buildings/Meters/Leases/PersonMonths/Notifications/Threads/Forms) render hardcoded MOCK_* arrays — no API wiring | done | high |  | Phase 1.5 review of mobile-rn segment (2026-06-16) | 2026-07-05 |
| 1 | bug | useDeepLinkRouting.ts:27-36 — initialize() re-runs on onNavigate identity change + void promise with no .catch → duplicate nav / unhandled rejection | dropped | medium |  | Phase 1.5 review of mobile-rn segment (2026-06-16) | 2026-07-05 |
| 1 | refactor | Churn hotspot: backend/crates/db/src/models/mod.rs (12 commits in 19-day catch-up) | dropped | high |  | churn since 4829015b: 12 commits | 2026-07-05 |
| 1 | refactor | Churn hotspot: backend/crates/db/src/repositories/rental.rs (11 commits in 19-day catch-up) | done | high |  | churn since 4829015b: 11 commits | 2026-07-05 |
| 1 | refactor | PR #1378 closed without merge — DROP-OWNED-BY teardown theory for #1332 was wrong root cause, superseded by #1379 | done | high |  | PR #1378, PR #1379 | 2026-06-15 |
| 1 | test-gap | PKCE unit test became a tautology after services/oauth.rs DRY refactor (#1132) | done | high |  | #1137, PR #1132, PR #1146 | 2026-06-07 |
| 1 | triage | Triage: dispatcher incident — assignments-archive.json corrupted to 1/196 rows on dev branch (#1061) | done | high |  | Issue #1061, #1061 closed | 2026-06-07 |
| 1 | triage | Issue #950 (no labels, OPEN): CI: trigger-deploy 403 marks all dev image builds red and blocks staging auto-deploy | done | high |  | #950, PR #1143, issue #950 closed | 2026-06-07 |
| 1 | triage | Issue #952 (no labels, OPEN): [staging] Reality SSO login dead-ends: redirect_uri callback 404s on reality apex | done | high |  | #952, PR #1144, issue #952 closed | 2026-06-07 |
| 1 | triage | Issue #769 (no labels, OPEN): Current dev review: Deploy server | done | high |  | #769, PR #1141, issue #769 closed | 2026-06-07 |
| 1 | triage | Issue #789 (no labels, OPEN): Dev review rounds 6-10: scheduler, notifications, admin, orgs, buildings | done | high |  | #789, PR #1142, issue #789 closed | 2026-06-07 |
| 1 | dx | docker/nginx admin-web + ppt-web templates churned twice this run (security headers + redirects) | done | high |  | PR #963, PR #964, PR #1107 | 2026-06-06 |
| 1 | refactor | ai.rs churn-hot — 3,142 lines this run; 3,142-line route monolith, candidate for module split | done | medium |  | git log origin/dev since 2026-05-24, PR #1114 | 2026-06-06 |
| 1 | refactor | ppt-web e2e auth-refresh.spec.ts added (+252 lines, story 79-2 token-refresh coverage) | done | high |  | PR #1047, PR #1113 | 2026-06-06 |
| 1 | refactor | api-server esignature_webhook_idempotency_tests.rs added (+228 lines, terminal-state regression) | done | high |  | PR #1034, PR #1119 | 2026-06-06 |
| 1 | refactor | ppt-web EvidenceUploader.test.tsx added (+202 lines, dispute-filing AC-2 regression) | done | high |  | PR #1048, PR #1116 | 2026-06-06 |
| 1 | refactor | api-server main.rs touched twice this run (gap-sweep + security headers) — minor churn marker | done | high |  | PR #989, PR #963, PR #1120 | 2026-06-06 |
| 1 | refactor | Duplicated animate-spin spinner markup across mediation page + chat thread (no shared Spinner) | done | medium |  | PR #555, code-review 2026-05-27, PR #1128 | 2026-06-06 |
| 1 | refactor | Mediation reference number uppercases full UUID (DSP-<uuid>) instead of a short code | done | medium |  | PR #555, code-review 2026-05-27, PR #1130 | 2026-06-06 |
| 1 | refactor | frontend/apps/mobile/src/App.tsx churned twice this run (universal links + doc-detail wiring) | done | high |  | PR #962, PR #992, PR #1131 | 2026-06-06 |
| 1 | refactor | platform_admin.rs churn-hot — 2,762 lines this run (admin/OAuth-provider feature work) | done | medium |  | git log origin/dev since 2026-05-24, PR #1109 | 2026-06-06 |
| 1 | refactor | Reality-web ComparisonUrlHandler hardcodes English loading/error strings | done | medium |  | code-review reality-web 2026-05-28, rotating-expert-review, PR #1127 | 2026-06-06 |
| 1 | refactor | Watch routes/oauth.rs churn after audit-log + hardening PRs | done | high |  | PR #930, PR #933, PR #1133 | 2026-06-06 |
| 1 | refactor | Watch services/oauth.rs churn after introspect/revoke hardening (#933) | done | high |  | PR #933, PR #1132 | 2026-06-06 |
| 1 | test-gap | Mobile VotingScreen pure transforms toUiStatus/toUiVote have no tests | done | medium |  | code-review mobile-rn 2026-05-27, rotating-expert-review, PR #1117 | 2026-06-06 |
| 1 | triage | Issue #749 (no labels, OPEN): Code review findings: Story 6.1 announcement creation and targeting | done | high |  | #749, issue #749 closed | 2026-06-06 |
| 1 | triage | Issue #755 (no labels, OPEN): Current dev review: Epic 8A Notification Preferences | done | high |  | #755, issue #755 closed | 2026-06-06 |
| 1 | triage | Issue #764 (no labels, OPEN): Current dev review: Admin MFA & Auth Hardening | done | high |  | #764, issue #764 closed | 2026-06-06 |
| 1 | triage | Issue #765 (no labels, OPEN): Current dev review: Integrations & Airbnb OAuth | done | high |  | #765, issue #765 closed | 2026-06-06 |
| 1 | bug | Mobile VotingScreen hardcodes en-US in toLocaleDateString — vote dates never localize | done | medium |  | PR #1083, code-review mobile-rn 2026-05-27, rotating-expert-review | 2026-06-05 |
| 1 | bug | Reality-web listing generateMetadata can throw during SSR on malformed 200 body | done | medium |  | PR #1085, code-review reality-web 2026-05-28, rotating-expert-review | 2026-06-05 |
| 1 | security | PR #908 (fix(security): require PKCE on OAuth authorization-code flow, closes #823) was closed unmerged — verify whether PKCE enforcement still pending | done | medium |  | PR #908, PR #1025 | 2026-06-03 |
| 1 | triage | Issue #751 (no labels, OPEN): Current dev review: frontend/web/API-client findings | done | high |  | #751, #942 | 2026-06-02 |
| 1 | triage | Issue #752 (no labels, OPEN): Current dev review: mobile CI tooling findings | done | high |  | #752, #929 | 2026-06-02 |
| 1 | triage | Issue #756 (no labels, OPEN): Current dev review: Epic 10A OAuth Provider | done | high |  | #756, #934 | 2026-06-02 |
| 1 | triage | Issue #761 (no labels, OPEN): Current dev review: Epic 84 E-Signature & Leases | done | high |  | #761, #936 | 2026-06-02 |
| 1 | triage | Issue #763 (no labels, OPEN): Current dev review: Reality Server & Inquiries | done | high |  | #763, #935 | 2026-06-02 |
| 1 | triage | Issue #767 (no labels, OPEN): Current dev review: Mobile RN Property Management app | done | high |  | #767, #943 | 2026-06-02 |
| 1 | triage | Issue #768 (no labels, OPEN): Current dev review: Admin-web features (10B) | done | high |  | #768, #930 | 2026-06-02 |
| 1 | triage | Issue #920 (no labels, OPEN): Announcement targeting not enforced on read (intra-org disclosure) | done | high |  | #920, #944 | 2026-06-02 |
| 1 | triage | Issue #750 (no labels, OPEN): Current dev review: backend/API/database findings | done | high |  | #750, PR #922 | 2026-06-01 |
| 1 | triage | Issue #753 (no labels, OPEN): Current dev review: Epic 6 Announcements & Communication | done | high |  | #753 | 2026-06-01 |
| 1 | triage | Issue #754 (no labels, OPEN): Current dev review: Epic 7A Basic Document Management | done | high |  | #754, PR #914 | 2026-06-01 |
| 1 | triage | Issue #757 (no labels, OPEN): Current dev review: Epic 10B Platform Administration | done | high |  | #757 | 2026-06-01 |
| 1 | triage | Issue #760 (no labels, OPEN): Current dev review: Epic 79 Disputes & Mediation | done | high |  | #760, PR #915 | 2026-06-01 |
| 1 | triage | Issue #762 (no labels, OPEN): Current dev review: Reports & Schedules | done | high |  | #762 | 2026-06-01 |
| 1 | triage | Issue #766 (no labels, OPEN): Current dev review: AI & LLM routes | done | high |  | #766, PR #879 | 2026-06-01 |
| 1 | triage | Issue #770 (no labels, OPEN): Current dev review: Faults & triage | done | high |  | #770, PR #902 | 2026-06-01 |
| 1 | triage | Issue #771 (no labels, OPEN): Current dev review: Research dispatcher & CI automation | done | high |  | #771, PR #923 | 2026-06-01 |
| 1 | triage | Issue #772 (no labels, OPEN): Current dev review: Auth core (delta confirmation) | done | high |  | #772 | 2026-06-01 |
| 1 | triage | Issue #773 (no labels, OPEN): Current dev review: Leases & rental | done | high |  | #773 | 2026-06-01 |
| 1 | triage | Issue #774 (no labels, OPEN): Current dev review: Reality server (broad) | done | high |  | #774, PR #919 | 2026-06-01 |
| 1 | triage | Issue #775 (no labels, OPEN): Current dev review: WebSocket realtime | done | high |  | #775, PR #926 | 2026-06-01 |
| 1 | triage | Issue #776 (no labels, OPEN): Current dev review: Equipment & audit log | done | high |  | #776 | 2026-06-01 |
| 1 | triage | Issue #777 (no labels, OPEN): Current dev review: Compliance & GDPR | done | high |  | #777 | 2026-06-01 |
| 1 | triage | Issue #778 (no labels, OPEN): Current dev review: Marketplace, voting, investor portal, impersonation | done | high |  | #778, PR #882 | 2026-06-01 |
| 1 | triage | Issue #788 (no labels, OPEN): Dev review rounds 1-5: mobile-native + ppt-web surfaces | done | high |  | #788, PR #922 | 2026-06-01 |
| 1 | triage | Issue #790 (no labels, OPEN): Dev review rounds 11-15: vendor, predictive, reality-web, middleware | done | high |  | #790, PR #913 | 2026-06-01 |
| 1 | triage | Issue #791 (no labels, OPEN): Dev review rounds 16-20: push, e-sign, portal, webhooks, reserves | done | high |  | #791, PR #924 | 2026-06-01 |
| 1 | triage | Issue #846 (no labels, OPEN): Code review: Epics 12+65 — Meters & Energy/ESG (origin/dev) | done | high |  | #846, PR #880 | 2026-06-01 |
| 1 | triage | Issue #847 (no labels, OPEN): Code review: Reality-server — Inquiries IDOR (Epics 16–19) (origin/dev) | done | high |  | #847 | 2026-06-01 |
| 1 | triage | Issue #848 (no labels, OPEN): Code review: Epics 78+134 — Vendor portal stubs & Predictive maintenance gaps (origin/dev) | done | high |  | #848, PR #913 | 2026-06-01 |
| 1 | triage | Issue #850 (no labels, OPEN): Code review: Epics 61+146+42 — Multi-currency, Data residency, Violations (origin/dev) | done | high |  | #850, PR #883 | 2026-06-01 |
| 1 | triage | Issue #851 (no labels, OPEN): Code review: Epics 15+105+69 — Listings/syndication & Developer API stubs (origin/dev) | done | high |  | #851, PR #904 | 2026-06-01 |
| 1 | triage | Issue #859 (no labels, OPEN): sqlx 0.9 breaks runtime decode of Postgres enum columns into Rust String (SELECT * reads 500) | done | high |  | #859, PR #871 | 2026-06-01 |
| 1 | triage | Issue #867 (no labels, OPEN): Tech debt: api-server main.rs duplicates lib.rs::create_router — routers diverge silently | done | high |  | #867, PR #870 | 2026-06-01 |
| 1 | triage | Issue #836 (no labels, OPEN): Code review: Epic 2B-C — Mobile push & device registration (origin/dev) | done | high |  | #836, PR #866 | 2026-05-31 |
| 1 | triage | Issue #845 (no labels, OPEN): Code review: Epic 14 — IoT alerts, correlations, thresholds (origin/dev) | done | high |  | #845, PR #862 | 2026-05-31 |
| 1 | triage | Issue #849 (no labels, OPEN): Code review: Epic 10B+143 — Admin impersonation, Help, Board meetings auth (origin/dev) | done | high |  | #849, PR #869 | 2026-05-31 |
| 0 | dx | Cloud routine cadence recovery — reduce 3–4d gaps between runs | dropped | high |  | routine self-signal 2026-07-09 | 2026-07-09 |
| 0 | refactor | Churn hotspot: 1021 lines changed in backend/servers/api-server/src/routes/emergency.rs (window 2026 | dropped | high |  | local git numstat since 2026-06-07 | 2026-07-05 |
| 0 | refactor | Churn hotspot: 929 lines changed in backend/servers/api-server/src/routes/vendors.rs (window 2026-06 | dropped | high |  | local git numstat since 2026-06-07 | 2026-07-05 |
| 0 | refactor | Churn hotspot: 709 lines changed in backend/servers/api-server/src/routes/enhanced_tenant_screening. | dropped | high |  | local git numstat since 2026-06-07 | 2026-07-05 |
| 0 | refactor | Churn hotspot: 2940 lines changed in backend/crates/db/src/repositories/document.rs (window 2026-06-10 03:05Z→18:30Z) | dropped | high |  | local git numstat since 2026-06-10T03:05:00Z | 2026-07-05 |
| 0 | refactor | Churn hotspot: 2856 lines changed in backend/crates/db/src/repositories/subscription.rs (window 2026-06-10 03:05Z→18:30Z) | dropped | high |  | local git numstat since 2026-06-10T03:05:00Z, PR #1246 | 2026-07-05 |
| 0 | refactor | Churn hotspot: 2691 lines changed in backend/servers/api-server/src/routes/aml_dsa.rs (window 2026-06-10 03:05Z→18:30Z) | dropped | high |  | local git numstat since 2026-06-10T03:05:00Z, PR #1193, PR #1203 | 2026-07-05 |
| 0 | triage | Issue #1151 (no labels, OPEN): Research dispatcher: claimable buffer is stale — true claimable work = 0 despite metric=53 | dropped | high |  | #1151 | 2026-07-05 |
| 0 | refactor | Churn hotspot: SearchScreen.kt — +1293 LOC this run (gap-82-3 reality mobile search/filters) | dropped | medium |  | PR #1125 | 2026-07-05 |
| 0 | refactor | MainActivity reimplements deep-link dispatch instead of calling shared DeepLinkRouter — drift trap | dropped | high |  | mobile-native-kmp segment review 2026-06-06 | 2026-07-05 |
| 0 | refactor | Churn hotspot: AnnouncementsScreen.tsx — 4 PRs this run, instability proxy | dropped | high |  | PR #1101, PR #1077, PR #1083, PR #1098 | 2026-07-05 |
| 0 | refactor | Churn hotspot: AnnouncementsScreen.test.ts — 4 PRs this run, instability proxy | dropped | high |  | PR #1101, PR #1077, PR #1083, PR #1098 | 2026-07-05 |
| 0 | triage | Dispatcher action-list.json corruption when MCP push falls back from blocked git push | dropped | high |  | #1014 | 2026-07-05 |
| 0 | triage | Issue #951 (no labels, OPEN): Deploy blocker: api-server requires ESIGN_TOKEN_SECRET + ESIGN_WEBHOOK_SECRET not injected by deploy-server (staging/prod) | dropped | high |  | #951 | 2026-07-05 |
| 0 | dx | PR #1274 (cargo-minor-patch group, /backend, 9 updates) closed unmerged — superseded by #1313 after auto-rebase fix landed | dropped | high |  | PR #1274 | 2026-07-05 |
| 0 | refactor | Churn hotspot: backend/crates/integrations/src/booking.rs (+404/−29 in PR #1294 Booking.com OTA retry) | dropped | high |  | PR #1294 commit 7ccce8a | 2026-07-05 |
| 0 | refactor | Churn hotspot: backend/servers/api-server/src/routes/api_ecosystem.rs (+106/−27 in PR #1293 PAP-171; second touch in 24h) | dropped | high |  | PR #1293 commit 1e50156 | 2026-07-05 |
| 0 | refactor | Churn hotspot: backend/crates/db/src/repositories/reality_portal.rs (+59/−28 in PR #1297 PAP-142 IDOR scoping) | dropped | high |  | PR #1297 commit 8c711c6 | 2026-07-05 |
| 0 | refactor | Churn hotspot: backend/servers/api-server/src/routes/iot.rs (+278/-403 in PR #1321/#1322 PAP-151 re-land + fmt) | dropped | high |  | PR #1321 commit, PR #1322 commit | 2026-07-05 |
| 0 | refactor | Churn hotspot: backend/servers/api-server/src/routes/reserve_funds.rs (+228/-255 in PR #1321 PAP-151 re-land) | dropped | high |  | PR #1321 commit | 2026-07-05 |
| 0 | refactor | Churn hotspot: backend/crates/db/src/repositories/sensor.rs (+248/-86 in PR #1321/#1322 PAP-151 re-land + fmt) | dropped | high |  | PR #1321 commit, PR #1322 commit | 2026-07-05 |
| 0 | triage | Issue #1331 (no labels, OPEN): Backend `test` job red/hanging on dev base — blocks the entire backend merge pipeline | dropped | high |  | #1331 | 2026-07-05 |
| 0 | dx | Stalled review: PR #988 (Epic: reusable Playwright E2E framework + sitemap FlowRunner) open 10d, no reviewDecision | dropped | high |  | PR #988 | 2026-07-05 |
| 0 | refactor | Churn hotspot: backend/servers/api-server/src/routes/forms.rs touched 2x since 2026-06-12 (window 2026-06-12→2026-06-13) | dropped | high |  | local git numstat since 2026-06-12, local git numstat since 2026-06-15 | 2026-07-05 |
| 0 | refactor | Churn hotspot: backend/servers/api-server/tests/reserve_funds_cross_org_idor_tests.rs touched 2x since 2026-06-12 (window 2026-06-12→2026-06-13) | dropped | high |  | local git numstat since 2026-06-12 | 2026-07-05 |
| 0 | refactor | Churn hotspot: backend/crates/db/tests/form_rls_repo_tests.rs touched 2x since 2026-06-12 (window 2026-06-12→2026-06-13) | dropped | high |  | local git numstat since 2026-06-12 | 2026-07-05 |
| 0 | triage | Issue #1380 (no labels, OPEN): Dispatcher stale gap-scan buffer + Tier-2 escalation endpoint misconfigured | dropped | high |  | issue #1380 | 2026-07-05 |
| 0 | refactor | Churn hotspot: 124 lines in frontend/apps/mobile/app.config.icon.test.ts (PR #1383 gap-85-2) | dropped | high |  | PR #1383 | 2026-07-05 |
| 0 | refactor | Churn hotspot: 94 lines in frontend/apps/mobile/app.config.ts (PR #1383 gap-85-2) | dropped | high |  | PR #1383 | 2026-07-05 |
| 0 | refactor | Churn hotspot: 53 lines in backend/crates/db/src/repositories/form.rs (PR #1379 #1332 unblock) | dropped | high |  | PR #1379, issue #1332 | 2026-07-05 |
| 0 | refactor | booking_oauth_csrf_tests.rs hotspot — 484-line NEW test file (PR #1393 #1424 OAuth CSRF coverage) | dropped | high |  | local git numstat since 2026-06-15 (commit 67c24bd..origin/dev) | 2026-07-05 |
| 0 | refactor | booking_oauth_routes_tests.rs hotspot — 381-line NEW test file (PR #1393 OAuth routes coverage) | dropped | high |  | local git numstat since 2026-06-15 | 2026-07-05 |
| 0 | refactor | forms.rs repeated-churn — runs_seen=2 (#1337 explicit_auto_deref + #1397 org-scope hardening) | dropped | high |  | hotspot_history.runs_seen 1→2 with new churn this run | 2026-07-05 |
| 0 | dx | PR #1425 (GH #1377 document presigned-URL tests) closed unmerged — superseded by merged #1394 | dropped | high |  | PR #1425 | 2026-07-05 |
| 0 | dx | PR #1179 (docs(epics) catalog backfill for 37 mounted-but-undocumented backend modules) — stalled at 7d, no reviewDecision | dropped | high |  | PR #1179 | 2026-07-05 |
| 0 | refactor | Stabilize oauth_integration_tests churn — heavy edits across 3 OAuth fix PRs | dropped | high |  | PR #930, PR #933 | 2026-06-16 |
| 0 | triage | Issue #779 (no labels, OPEN): Current dev review: consolidated priority rollup (origin/dev snapshot) | dropped | high |  | #779 | 2026-06-13 |
| 0 | bug | Announcer: untracked clear-then-set timeouts can resurrect a stale screen-reader message | dropped | medium |  | code-review ppt-web-ui 2026-05-24, Announcer.tsx:49 | 2026-06-07 |
| 0 | dx | Portfolio dashboard: alert mark-read/resolve mutations + property-card click navigation are no-op stubs | dropped |  |  | PR #328, commit 254f01d | 2026-06-04 |
|  |  | code-review-api-handlers-share-log-proxy-ip |  | medium |  |  |  |
