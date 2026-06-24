# Backlog of vectors
<sub>Last regenerated: 2026-06-24 16:12 UTC by routine</sub>

_222 items · 73 open · 7 ready · 121 done · 20 dropped._

| Score | Vector | Status | ID | Title | Updated |
|---|---|---|---|---|---|
| 6 | test-gap | done | `test-gap-inquiry-idor-regression` | Add regression tests for inquiry mark_as_read cross-tenant IDOR fix (PR #497) | 2026-05-26 |
| 3 | refactor | done | `revert-pr-1713` | Revert PR #1713 — delegation frontend removed (BIT-213 retirement, intentional) | 2026-06-24 |
| 3 | bug | open | `unchecked-todo-pr-1768` | Unchecked TODOs in merged PR #1768 — check post-merge follow-up | 2026-06-24 |
| 3 | bug | open | `unchecked-todo-pr-1709` | Unchecked TODOs in merged PR #1709 — check post-merge follow-up | 2026-06-24 |
| 3 | bug | open | `unchecked-todo-pr-1703` | Unchecked TODOs in merged PR #1703 — check post-merge follow-up | 2026-06-24 |
| 3 | bug | open | `unchecked-todo-pr-1717` | Unchecked TODOs in merged PR #1717 — check post-merge follow-up | 2026-06-24 |
| 3 | bug | open | `unchecked-todo-pr-1705` | Unchecked TODOs in merged PR #1705 — check post-merge follow-up | 2026-06-24 |
| 3 | bug | open | `unchecked-todo-pr-1685` | Unchecked TODOs in merged PR #1685 — check post-merge follow-up | 2026-06-24 |
| 3 | bug | open | `unchecked-todo-pr-1641` | Unchecked TODOs in merged PR #1641 — check post-merge follow-up | 2026-06-24 |
| 3 | security | ready | `code-review-api-handlers-platform-admin-jwt-only-gate` | backend/servers/api-server/src/routes/subscriptions.rs:69-132 — require_super_admin gates all 8 platform-admin write endpoints (create/update/delete plans, c... | 2026-06-24 |
| 3 | bug | ready | `code-review-mobile-rn-report-fault-fake-submit` | ReportFaultScreen.tsx handleSubmit() fakes API call with setTimeout(1500) — fault reports never reach backend (App.tsx:126 wires this) | 2026-06-16 |
| 3 | bug | ready | `code-review-reality-web-realtor-mgmt-untranslated` | Reality-web RealtorManagement.tsx hardcoded English strings — agency flow not localized to sk/cs/de | 2026-06-15 |
| 3 | bug | ready | `code-review-reality-web-share-comparison-404` | Reality-web ComparisonUrlHandler hits non-existent /api/listings/${id} — every shared comparison URL 404s | 2026-06-14 |
| 3 | bug | ready | `code-review-reality-web-listing-page-ssr-crash` | Reality-web listing detail SSR crashes on partial 200 body — JSON-LD build deref of undefined fields | 2026-06-14 |
| 3 | bug | ready | `bug-ios-searchview-uncompilable` | iOS SearchView.swift does not compile — performSearch/scheduleSearch undefined, resultsGrid corrupted | 2026-06-11 |
| 3 | security | dropped | `unchecked-todo-pr-1203` | PR #1203 (fix(aml_dsa): close cross-tenant IDOR in moderation + AML-review handlers (PAP-36)) merged | 2026-06-10 |
| 3 | security | dropped | `unchecked-todo-pr-1193` | PR #1193 (fix(aml-dsa): lock DSA reports to platform roles + fix file-path disclosure (PAP-47)) merg | 2026-06-10 |
| 3 | bug | done | `bug-schema-drift-runtime-sql-issue-1008` | Schema drift: runtime SQL errors from non-existent columns in voting/messaging/notification paths | 2026-06-07 |
| 3 | security | ready | `security-llm-doc-idor` | IDOR: ai.rs LLM-doc handlers publish/list/get any tenant's listing descriptions & photo enhancements unscoped | 2026-06-01 |
| 3 | security | done | `security-realtors-mark-inquiry-read-idor` | IDOR: reality-server realtors mark_inquiry_read flips any realtor's inquiry by ID with no owner scoping | 2026-05-26 |
| 3 | security | done | `security-equipment-idor` | IDOR: equipment delete/update + maintenance update mutate any tenant's equipment by ID with no org scoping | 2026-05-25 |
| 3 | security | done | `security-ssrf-outbound-url-validation` | SSRF: signed-document fetch + webhook-test POST issue outbound requests to unvalidated user-controlled URLs | 2026-05-25 |
| 3 | security | done | `security-voice-device-idor` | IDOR: unlink_voice_device deactivates any device by ID with no owner/org scoping | 2026-05-25 |
| 2 | bug | done | `code-review-mobile-rn-screens-mock-data` | Mobile RN production screens (Buildings/Meters/Leases/PersonMonths/Notifications/Threads/Forms) render hardcoded MOCK_* arrays — no API wiring | 2026-06-24 |
| 2 | dx | open | `unchecked-todo-pr-1723` | Unchecked TODOs in merged PR #1723 — check post-merge follow-up | 2026-06-24 |
| 2 | test-gap | open | `hotfix-no-test-pr-1757` | PR #1757 merged (fix(db): resolve duplicate migration version 00192 collision (#1757)) without any *_test.rs / tests/ / *.test.* file diff in the squash commit | 2026-06-24 |
| 2 | test-gap | open | `hotfix-no-test-pr-1751` | PR #1751 merged (fix(dispatcher): archive-terminal reconciler pass + unique branch names/collision guard (#1747, #1739) (#1751)) without any *_test.rs / test... | 2026-06-24 |
| 2 | bug | open | `code-review-api-handlers-agencies-unwrap-as-ref` | backend/servers/api-server/src/routes/agencies.rs:747 — if member.is_none() \|\| !member.as_ref().unwrap().is_active — guard relies on short-circuit \|\| eva... | 2026-06-24 |
| 2 | security | open | `code-review-api-handlers-role-const-drift` | backend/servers/api-server/src/routes/feature_packages.rs:29-34 SUPER_ADMIN_ROLES = [SuperAdministrator, super_admin, superadmin, platform_admin] | 2026-06-24 |
| 2 | bug | done | `code-review-api-core-vote-partial-cmp-panic` | vote.rs:1765 calculate_question_result() uses partial_cmp().unwrap() on f64 — NaN/Inf weights panic /votes/{id}/results | 2026-06-16 |
| 2 | test-gap | open | `screen-map-drift-pr-1418-ppt` | PR #1418 touched routes/** (faults.route.test.tsx) without updating docs/screens/ppt/* — heuristic, test-file fix | 2026-06-16 |
| 2 | bug | open | `code-review-mobile-rn-deeplink-init-unhandled` | useDeepLinkRouting.ts:27-36 — initialize() re-runs on onNavigate identity change + void promise with no .catch → duplicate nav / unhandled rejection | 2026-06-16 |
| 2 | bug | open | `bug-ios-deeplink-info-plist-missing` | iOS deep-link layer dead at runtime — Info.plist missing CFBundleURLTypes + applinks entitlement | 2026-06-11 |
| 2 | test-gap | open | `test-gap-hotfix-no-test-pr-1288-webhook-rls` | Webhook handlers RLS migration (PR #1288, PAP-170) shipped without a new regression test for repo-layer methods | 2026-06-11 |
| 2 | test-gap | open | `test-gap-hotfix-no-test-pr-1287-rls-llm-sessions` | AI llm/sessions + integrations sync + subscriptions RLS migration (PR #1287, PAP-169) shipped without a new regression test | 2026-06-11 |
| 2 | test-gap | open | `test-gap-hotfix-no-test-pr-1289-api-ecosystem` | api_ecosystem.rs RLS migration (PR #1289, PAP-167) — 162-line handler rework shipped without a regression test for the public-connection routing | 2026-06-11 |
| 2 | test-gap | open | `test-gap-hotfix-no-test-pr-1292-mfa-rls` | mfa.rs RLS migration (PR #1292, PAP-168) shipped without a regression test; also landed broken and was hotfixed in PR #1287 | 2026-06-11 |
| 2 | test-gap | dropped | `unchecked-todo-pr-1196` | PR #1196 (feat(ppt-web): add missing test coverage for faults feature) merged with 2 unchecked TODO  | 2026-06-10 |
| 2 | dx | done | `dx-push-fanout-blpop-drain` | PushFanoutWorker BLPOP queue-drain deferred — Redis path is a logging no-op | 2026-06-06 |
| 2 | refactor | done | `refactor-ai-rs-module-split` | ai.rs (3,134 LOC) — explicit module-split into routes/ai/{sessions,equipment,workflows,voice,llm,mod}.rs | 2026-06-06 |
| 2 | refactor | done | `refactor-announcements-rs-hot` | announcements.rs churn-hot — 2,722 lines this run (Epic 2B + Epic 6 work) | 2026-06-06 |
| 2 | refactor | done | `refactor-announcements-rs-module-split` | announcements.rs (2,722 LOC) — explicit module-split into routes/announcements/{crud,targeting,delivery,reactions,mod}.rs | 2026-06-06 |
| 2 | refactor | done | `refactor-app-tsx-route-coupling` | Reduce App.tsx route-aggregator coupling (top churn hotspot, merge-conflict risk) | 2026-06-06 |
| 2 | refactor | done | `refactor-platform-admin-rs-module-split` | platform_admin.rs (2,762 LOC) — explicit module-split into routes/platform_admin/{tenants,features,billing,audit,mod}.rs | 2026-06-06 |
| 2 | test-gap | done | `test-gap-screen-map-drift-pr-1033-ppt` | Screen-map drift: PR #1033 wired error/retry into AnnouncementsPage+FaultsPage via App.tsx without a docs/screens/ppt update | 2026-06-06 |
| 2 | bug | done | `bug-risky-churn-pr-992-mobile-app-tsx` | Risky churn: mobile App.tsx deep-link/doc-detail wiring changing across back-to-back PRs without coverage | 2026-06-05 |
| 2 | dx | done | `dx-integration-marketplace-stubs` | Integration marketplace install/OAuth flows are placeholders — wire backend handlers + UI navigation | 2026-06-05 |
| 2 | test-gap | done | `test-gap-booking-push-validation-untested` | Booking push availability/rates endpoints add batch-cap + non-negative guards with no regression test | 2026-06-05 |
| 2 | test-gap | done | `test-gap-hotfix-no-test-pr-874-portal-webhooks` | Portal webhook fail-closed fix (PR #874) shipped without a regression test for unverified-signature rejection | 2026-06-05 |
| 2 | test-gap | done | `test-gap-hotfix-no-test-pr-918-mobile-dev-review` | Mobile dev-review batch (PR #918, 5 files under frontend/apps/mobile/src) shipped without a regression test | 2026-06-05 |
| 2 | test-gap | done | `test-gap-hotfix-no-test-pr-921-sso-consumer` | Reality-server SSO consumer review fix (PR #921, closes #820) shipped without a regression test | 2026-06-05 |
| 2 | test-gap | done | `test-gap-hotfix-no-test-pr-923-branch-protection-rebase` | CI branch-protection + auto-rebase workflow change (PR #923) shipped without an integration test | 2026-06-05 |
| 2 | test-gap | done | `test-gap-hotfix-no-test-pr-939-deploy-server-scopes` | deploy-server OIDC scope mapping (#939) shipped without unit test for derive_oidc_scopes | 2026-06-05 |
| 2 | test-gap | done | `test-gap-hotfix-no-test-pr-943-mobile-dev-review-tail` | Mobile RN dev-review tail (#943) shipped without test coverage | 2026-06-05 |
| 2 | test-gap | done | `test-gap-hotfix-no-test-pr-990-frontend-gap-sweep` | Frontend gap-sweep (PR #990, 34 files across Epics 1/6/7B/9/10B/11/15/17/18) shipped without a regression test | 2026-06-05 |
| 2 | test-gap | done | `test-gap-hotfix-no-test-pr-992-mobile-doc-detail` | Mobile document-detail wiring (PR #992) shipped without a regression test for the deep-link payload path | 2026-06-05 |
| 2 | test-gap | done | `test-gap-screen-map-drift-pr-839-ppt` | Screen-map drift: PR #839 modified ppt-web App.tsx (FileDisputePageRoute) without a docs/screens/ppt update | 2026-06-05 |
| 2 | refactor | done | `refactor-ppt-web-untranslated-strings` | ppt-web status/auth components hardcode English in an otherwise i18n'd app | 2026-06-04 |
| 2 | bug | done | `bug-mediation-page-no-error-state` | MediationWorkspacePage shows empty/unknown state instead of error UI on dispute fetch failure | 2026-06-03 |
| 2 | bug | done | `bug-mobile-voting-unsafe-cast` | Mobile VotingScreen double-casts API result across boundary — render-time crash on unexpected shape | 2026-06-03 |
| 2 | bug | done | `bug-reality-web-realtor-invite-silent-error` | Reality-web InviteRealtorModal swallows invite-mutation failure with no error UI | 2026-06-03 |
| 2 | bug | done | `bug-webhook-airbnb-dup-sync-jobs` | Airbnb webhook at-least-once delivery enqueues duplicate SYNC_EXTERNAL jobs | 2026-06-03 |
| 2 | dx | done | `dx-documentsbrowse-folder-preselect` | DocumentsBrowse MoveFolderDialog cannot pre-select current folder (DocumentSummary lacks folder_id) | 2026-06-03 |
| 2 | test-gap | done | `test-gap-hotfix-no-test-pr-963-security-headers` | API + SPA security-headers middleware (PR #963) shipped without an assertion test for HSTS/nosniff/CSP | 2026-06-03 |
| 2 | test-gap | done | `test-gap-router-set-parity-tests` | api-server main.rs vs lib.rs::create_router diverge silently (5 routes unreachable in prod, no test asserts parity) | 2026-06-01 |
| 2 | bug | done | `bug-report-schedule-update-no-sql` | ReportSchedule.update_schedule stores cron in `time` workaround; documented UPDATE never runs (missing cron_expression column) | 2026-05-30 |
| 2 | test-gap | done | `test-gap-screen-map-drift-ppt-report-history` | Screen-map drift: report execution-history route (PR #547) added without a ppt screen doc | 2026-05-27 |
| 2 | test-gap | done | `test-gap-dispute-fsm-no-tests` | Dispute state machine (PR #506) shipped with no tests + no org predicate on update_status | 2026-05-26 |
| 2 | refactor | done | `refactor-documents-rs-hot` | documents.rs churn-hot — 10,659 lines over 14d | 2026-05-25 |
| 2 | refactor | done | `refactor-integrations-rs-hot` | integrations.rs churn-hot — 12,977 lines over 14d, candidate for module split | 2026-05-25 |
| 2 | refactor | done | `refactor-organizations-rs-hot` | organizations.rs churn-hot — 12,060 lines over 14d (multitenancy + admin) | 2026-05-25 |
| 2 | security | done | `security-inquiry-read-idor` | IDOR: reality-server mark_as_read flips any realtor's inquiry by ID with no owner scoping | 2026-05-25 |
| 2 | security | done | `security-role-gate-fail-open` | Latent fail-open: ProtectedRoute role check is skipped when user.role is falsy | 2026-05-25 |
| 2 | test-gap | done | `test-gap-screen-map-drift-ppt-neighbors` | Screen-map drift: PR #464 wired a neighbors route in ppt-web without a docs/screens/ppt entry | 2026-05-25 |
| 2 | test-gap | closed | `test-gap-screen-map-drift-reality-listing` | Screen-map drift: PR #460 touched reality-web listing page without a docs/screens/reality update | 2026-05-25 |
| 2 | refactor | done | `refactor-dead-dup-handler-modules` | Dead/duplicate handler modules: AuthHandler & BuildingHandler unused, routes reimplement inline | 2026-05-24 |
| 2 | security | done | `security-rls-migration-residual` | Complete RLS migration in 31 remaining handlers (voting, market_pricing, faults, notif_prefs, reports) | 2026-05-23 |
| 1 | bug | open | `code-review-mobile-native-kmp-deeplink-token-not-url-decoded` | DeepLinkRouter skips URL-decoding while Android Uri.getQueryParameter decodes — SSO tokens diverge per platform | 2026-06-24 |
| 1 | bug | open | `code-review-mobile-native-kmp-search-stale-response-race` | SearchScreen stale-response race — overlapping searches can clobber newer results | 2026-06-24 |
| 1 | test-gap | open | `test-gap-screen-map-drift-pr-1085-reality` | Screen-map drift: PR #1085 modified reality-web listing detail metadata + page without screen-doc update | 2026-06-24 |
| 1 | test-gap | open | `test-gap-screen-map-drift-pr-1100-ppt` | Screen-map drift: PR #1100 modified ppt-web App.tsx (FileDisputePageRoute extraction) without screen-doc update | 2026-06-24 |
| 1 | bug | open | `bug-risky-churn-pr-963-api-main-rs` | Risky churn: api-server main.rs security-headers wiring shipped without a middleware smoke test | 2026-06-24 |
| 1 | test-gap | open | `test-gap-hotfix-no-test-pr-959-reality-listings-pagination` | Reality-server listings pagination clamp (PR #959) shipped without a regression test for limit=-1 | 2026-06-24 |
| 1 | refactor | done | `churn-hotspot-backend-servers-api-server-src-routes-emergency-rs` | Churn hotspot: 1021 lines changed in backend/servers/api-server/src/routes/emergency.rs (window 2026 | 2026-06-24 |
| 1 | refactor | done | `churn-hotspot-backend-crates-db-src-repositories-document-rs` | Churn hotspot: 2940 lines changed in backend/crates/db/src/repositories/document.rs (window 2026-06-10 03:05Z→18:30Z) | 2026-06-24 |
| 1 | refactor | done | `churn-hotspot-backend-crates-integrations-src-booking-rs` | Churn hotspot: backend/crates/integrations/src/booking.rs (+404/−29 in PR #1294 Booking.com OTA retry) | 2026-06-24 |
| 1 | refactor | done | `churn-hotspot-backend-servers-api-server-src-routes-reserve-funds-rs` | Churn hotspot: backend/servers/api-server/src/routes/reserve_funds.rs (+228/-255 in PR #1321 PAP-151 re-land) | 2026-06-24 |
| 1 | refactor | done | `churn-hotspot-backend-crates-db-src-repositories-sensor-rs` | Churn hotspot: backend/crates/db/src/repositories/sensor.rs (+248/-86 in PR #1321/#1322 PAP-151 re-land + fmt) | 2026-06-24 |
| 1 | refactor | done | `churn-hotspot-backend-crates-db-tests-form-rls-repo-tests-rs` | Churn hotspot: backend/crates/db/tests/form_rls_repo_tests.rs touched 2x since 2026-06-12 (window 2026-06-12→2026-06-13) | 2026-06-24 |
| 1 | refactor | done | `churn-hotspot-frontend-apps-mobile-app-config-icon-test-ts` | Churn hotspot: 124 lines in frontend/apps/mobile/app.config.icon.test.ts (PR #1383 gap-85-2) | 2026-06-24 |
| 1 | refactor | done | `code-review-api-core-osrng-expect` | crypto.rs:127 SysRng.try_fill_bytes(...).expect() panics if OS CSPRNG errors during integration-credential encrypt | 2026-06-24 |
| 1 | refactor | done | `repeated-churn-backend-servers-api-server-src-routes-forms-rs` | forms.rs repeated-churn — runs_seen=2 (#1337 explicit_auto_deref + #1397 org-scope hardening) | 2026-06-24 |
| 1 | refactor | open | `churn-hotspot-backend-crates-db-src-repositories-rental-rs` | Churn hotspot: backend/crates/db/src/repositories/rental.rs (backend/crates/db/src/repositories/rental.rs: 2939 lines changed since 2026-06-16 (PRs include r... | 2026-06-24 |
| 1 | triage | open | `triage-issue-1793` | Issue #1793: Follow-up: fault notifications — two transitions (triage, confirm) emit nothing + scattered dispatch logic (PR #1705) | 2026-06-24 |
| 1 | triage | open | `triage-issue-1792` | Issue #1792: Follow-up: event bus retry — blocking per-subscription retry + silent Lagged/buffer-overflow drop defeat at-least-once ( | 2026-06-24 |
| 1 | triage | open | `triage-issue-1791` | Issue #1791: Follow-up: message attachments — link trusts client-supplied file_key (IDOR) + unvalidated content-type (PR #1702) | 2026-06-24 |
| 1 | triage | open | `triage-issue-1790` | Issue #1790: Follow-up: payment reminders re-fire every ~5 min for the whole due-date window (no persistent dedup) (PR #1709) | 2026-06-24 |
| 1 | triage | open | `triage-issue-1789` | Issue #1789: Follow-up: N-party conversations — tests assert snake_case against camelCase wire; start_thread response still 2-party-s | 2026-06-24 |
| 1 | triage | open | `triage-issue-1788` | Issue #1788: Follow-up: Booking.com listing-push — currency validation is structural-only, duplicates existing SupportedCurrency allo | 2026-06-24 |
| 1 | triage | open | `triage-issue-1787` | Issue #1787: Follow-up: align get_booking_conflicts manager gate with canonical DB-backed predicate (PR #1741) | 2026-06-24 |
| 1 | triage | open | `triage-issue-1786` | Issue #1786: Follow-up: sensor WS handler — add authz regression tests for the surviving DB-checked path (PR #1737) | 2026-06-24 |
| 1 | triage | open | `triage-issue-1785` | Issue #1785: Follow-up: Stripe Checkout — multi-currency amount conversion + Stripe idempotency-key + webhook amount cross-check (PR  | 2026-06-24 |
| 1 | triage | open | `triage-issue-1784` | Issue #1784: Follow-up: portal-listings — DB-level status/enum defense-in-depth + IG3 test note (PR #1746) | 2026-06-24 |
| 1 | triage | open | `triage-issue-1783` | Issue #1783: Follow-up: guest ID-document upload/OCR seam — PII audit-logging, content-type sniffing, structured-PII authz parity (PR | 2026-06-24 |
| 1 | triage | open | `triage-issue-1782` | Issue #1782: Follow-up: a third access-token verification copy (JwtService) left unmigrated + log lost token_type field (PR #1744) | 2026-06-24 |
| 1 | triage | open | `triage-issue-1780` | Issue #1780: Follow-up: camelCase wire flip (PR #1768) breaks React Native mobile messaging | 2026-06-24 |
| 1 | triage | open | `triage-issue-1777` | Issue #1777: Follow-up: meter reminders can be sent twice to a resident with multiple unit meters (PR #1703) | 2026-06-24 |
| 1 | triage | open | `triage-issue-1776` | Issue #1776: Follow-up: N+1 block-check loop in start_thread for N-party conversations (PR #1689) | 2026-06-24 |
| 1 | triage | open | `triage-issue-1773` | Issue #1773: Follow-up: send_message + unread/read model not generalized for N-party group threads (PR #1689) | 2026-06-24 |
| 1 | triage | open | `triage-issue-1772` | Issue #1772: Follow-up: OCR endpoints unauthenticated + meter-reminder dedup/doc drift (PR #1703) | 2026-06-24 |
| 1 | triage | open | `triage-issue-1771` | Issue #1771: Follow-up: unread-count ignores per-participant soft-delete/archive (PR #1696) | 2026-06-24 |
| 1 | triage | open | `triage-issue-1770` | Issue #1770: Follow-up: bind attachment file_key to the issued upload key + validate MIME on link (PR #1702) | 2026-06-24 |
| 1 | triage | open | `triage-issue-1769` | Issue #1769: Follow-up: payment-reminder scheduler can double-send + UTC date boundary + tautological tests (PR #1709) | 2026-06-24 |
| 1 | triage | open | `triage-issue-1767` | Issue #1767: Follow-up: offline fault queue ordering/concurrency + offline-edit dedup gaps (PR #1715) | 2026-06-24 |
| 1 | triage | open | `triage-issue-1766` | Issue #1766: Follow-up: persisted rental booking/guest PII reads (list_bookings/get_booking/get_booking_with_guests/get_guest) not ma | 2026-06-24 |
| 1 | triage | open | `triage-issue-1765` | Issue #1765: Follow-up: event bus backoff jitter + head-of-line blocking on retry (PR #1716) | 2026-06-24 |
| 1 | triage | open | `triage-issue-1764` | Issue #1764: Follow-up: Stripe Checkout hardening — session-creation idempotency key + webhook amount cross-check (PR #1726) | 2026-06-24 |
| 1 | triage | open | `triage-issue-1763` | Issue #1763: Follow-up: add integration test for sensor WS non-member 403 rejection (PR #1737) | 2026-06-24 |
| 1 | triage | open | `triage-issue-1762` | Issue #1762: Follow-up: portal listing edit silently downgrades sold/rented/archived to draft (PR #1746) | 2026-06-24 |
| 1 | triage | open | `triage-issue-1761` | Issue #1761: Follow-up: restore raw principal_kind in tenant-resolution warn logs; carry over #1675 finding-3 (PR #1744) | 2026-06-24 |
| 1 | triage | open | `triage-issue-1760` | Issue #1760: Follow-up: duplicate SQLx migration 00192 broke dev + ID-document PII hardening (PR #1750) | 2026-06-24 |
| 1 | triage | open | `triage-issue-1759` | Issue #1759: Follow-up: tighten Booking.com listing-push rate/currency validation (ISO-4217 whitelist, mixed-currency, amount bounds) | 2026-06-24 |
| 1 | triage | open | `triage-issue-1758` | Issue #1758: Follow-up: preflight presence-check misses length floors for JWT_SECRET / ESIGN_TOKEN_SECRET (PR #1753) | 2026-06-24 |
| 1 | triage | open | `triage-issue-1680` | Issue #1680: Research dispatcher: cron environment can&#39;t run core pipeline (fire-and-forget implementers die, no DB, git-push + a | 2026-06-24 |
| 1 | test-gap | open | `test-gap-screen-map-drift-pr-922-ppt` | Screen-map drift: PR #922 modified ppt-web App.tsx (dev-review rounds 1-5 fixes) without a docs/screens/ppt update | 2026-06-16 |
| 1 | refactor | open | `churn-hotspot-backend-servers-api-server-src-routes-forms-rs` | Churn hotspot: backend/servers/api-server/src/routes/forms.rs touched 2x since 2026-06-12 (window 2026-06-12→2026-06-13) | 2026-06-16 |
| 1 | refactor | open | `churn-hotspot-backend-servers-api-server-tests-booking-oauth-csrf-tests-rs` | booking_oauth_csrf_tests.rs hotspot — 484-line NEW test file (PR #1393 #1424 OAuth CSRF coverage) | 2026-06-16 |
| 1 | refactor | open | `churn-hotspot-backend-servers-api-server-tests-booking-oauth-routes-tests-rs` | booking_oauth_routes_tests.rs hotspot — 381-line NEW test file (PR #1393 OAuth routes coverage) | 2026-06-16 |
| 1 | dx | open | `closed-not-merged-pr-1425` | PR #1425 (GH #1377 document presigned-URL tests) closed unmerged — superseded by merged #1394 | 2026-06-16 |
| 1 | dx | open | `dx-stalled-review-pr-1179` | PR #1179 (docs(epics) catalog backfill for 37 mounted-but-undocumented backend modules) — stalled at 7d, no reviewDecision | 2026-06-16 |
| 1 | triage | open | `triage-issue-1380` | Issue #1380 (no labels, OPEN): Dispatcher stale gap-scan buffer + Tier-2 escalation endpoint misconfigured | 2026-06-15 |
| 1 | refactor | done | `refactor-closed-not-merged-pr-1378` | PR #1378 closed without merge — DROP-OWNED-BY teardown theory for #1332 was wrong root cause, superseded by #1379 | 2026-06-15 |
| 1 | refactor | open | `churn-hotspot-frontend-apps-mobile-app-config-ts` | Churn hotspot: 94 lines in frontend/apps/mobile/app.config.ts (PR #1383 gap-85-2) | 2026-06-15 |
| 1 | refactor | open | `churn-hotspot-backend-crates-db-src-repositories-form-rs` | Churn hotspot: 53 lines in backend/crates/db/src/repositories/form.rs (PR #1379 #1332 unblock) | 2026-06-15 |
| 1 | triage | open | `triage-issue-1331` | Issue #1331 (no labels, OPEN): Backend `test` job red/hanging on dev base — blocks the entire backend merge pipeline | 2026-06-13 |
| 1 | dx | open | `dx-stalled-review-pr-988` | Stalled review: PR #988 (Epic: reusable Playwright E2E framework + sitemap FlowRunner) open 10d, no reviewDecision | 2026-06-13 |
| 1 | refactor | open | `churn-hotspot-backend-servers-api-server-tests-reserve-funds-cross-org-idor-tests-rs` | Churn hotspot: backend/servers/api-server/tests/reserve_funds_cross_org_idor_tests.rs touched 2x since 2026-06-12 (window 2026-06-12→2026-06-13) | 2026-06-13 |
| 1 | dx | open | `closed-not-merged-pr-1274` | PR #1274 (cargo-minor-patch group, /backend, 9 updates) closed unmerged — superseded by #1313 after auto-rebase fix landed | 2026-06-12 |
| 1 | refactor | open | `churn-hotspot-backend-servers-api-server-src-routes-api-ecosystem-rs` | Churn hotspot: backend/servers/api-server/src/routes/api_ecosystem.rs (+106/−27 in PR #1293 PAP-171; second touch in 24h) | 2026-06-12 |
| 1 | refactor | open | `churn-hotspot-backend-crates-db-src-repositories-reality-portal-rs` | Churn hotspot: backend/crates/db/src/repositories/reality_portal.rs (+59/−28 in PR #1297 PAP-142 IDOR scoping) | 2026-06-12 |
| 1 | refactor | open | `churn-hotspot-backend-servers-api-server-src-routes-iot-rs` | Churn hotspot: backend/servers/api-server/src/routes/iot.rs (+278/-403 in PR #1321/#1322 PAP-151 re-land + fmt) | 2026-06-12 |
| 1 | test-gap | done | `code-review-issue-1137-pkce-test-tautology` | PKCE unit test became a tautology after services/oauth.rs DRY refactor (#1132) | 2026-06-07 |
| 1 | triage | done | `triage-issue-1061-dispatcher-archive-corruption` | Triage: dispatcher incident — assignments-archive.json corrupted to 1/196 rows on dev branch (#1061) | 2026-06-07 |
| 1 | triage | done | `triage-issue-950` | Issue #950 (no labels, OPEN): CI: trigger-deploy 403 marks all dev image builds red and blocks staging auto-deploy | 2026-06-07 |
| 1 | triage | done | `triage-issue-952` | Issue #952 (no labels, OPEN): [staging] Reality SSO login dead-ends: redirect_uri callback 404s on reality apex | 2026-06-07 |
| 1 | triage | done | `triage-issue-769` | Issue #769 (no labels, OPEN): Current dev review: Deploy server | 2026-06-07 |
| 1 | triage | done | `triage-issue-789` | Issue #789 (no labels, OPEN): Dev review rounds 6-10: scheduler, notifications, admin, orgs, buildings | 2026-06-07 |
| 1 | dx | done | `dx-nginx-template-churn-2026-06-03` | docker/nginx admin-web + ppt-web templates churned twice this run (security headers + redirects) | 2026-06-06 |
| 1 | refactor | done | `refactor-ai-rs-hot` | ai.rs churn-hot — 3,142 lines this run; 3,142-line route monolith, candidate for module split | 2026-06-06 |
| 1 | refactor | done | `refactor-auth-refresh-spec-churn-2026-06-04` | ppt-web e2e auth-refresh.spec.ts added (+252 lines, story 79-2 token-refresh coverage) | 2026-06-06 |
| 1 | refactor | done | `refactor-esignature-webhook-idempotency-tests-churn-2026-06-04` | api-server esignature_webhook_idempotency_tests.rs added (+228 lines, terminal-state regression) | 2026-06-06 |
| 1 | refactor | done | `refactor-evidenceuploader-test-tsx-churn-2026-06-04` | ppt-web EvidenceUploader.test.tsx added (+202 lines, dispute-filing AC-2 regression) | 2026-06-06 |
| 1 | refactor | done | `refactor-main-rs-churn-2026-06-03` | api-server main.rs touched twice this run (gap-sweep + security headers) — minor churn marker | 2026-06-06 |
| 1 | refactor | done | `refactor-mediation-duplicated-spinner` | Duplicated animate-spin spinner markup across mediation page + chat thread (no shared Spinner) | 2026-06-06 |
| 1 | refactor | done | `refactor-mediation-ref-number-full-uuid` | Mediation reference number uppercases full UUID (DSP-<uuid>) instead of a short code | 2026-06-06 |
| 1 | refactor | done | `refactor-mobile-app-tsx-churn-2026-06-03` | frontend/apps/mobile/src/App.tsx churned twice this run (universal links + doc-detail wiring) | 2026-06-06 |
| 1 | refactor | done | `refactor-platform-admin-rs-hot` | platform_admin.rs churn-hot — 2,762 lines this run (admin/OAuth-provider feature work) | 2026-06-06 |
| 1 | refactor | done | `refactor-reality-web-comparison-i18n` | Reality-web ComparisonUrlHandler hardcodes English loading/error strings | 2026-06-06 |
| 1 | refactor | done | `refactor-routes-oauth-rs-hot` | Watch routes/oauth.rs churn after audit-log + hardening PRs | 2026-06-06 |
| 1 | refactor | done | `refactor-services-oauth-rs-hot` | Watch services/oauth.rs churn after introspect/revoke hardening (#933) | 2026-06-06 |
| 1 | test-gap | done | `test-gap-mobile-voting-transforms` | Mobile VotingScreen pure transforms toUiStatus/toUiVote have no tests | 2026-06-06 |
| 1 | triage | done | `triage-issue-749` | Issue #749 (no labels, OPEN): Code review findings: Story 6.1 announcement creation and targeting | 2026-06-06 |
| 1 | triage | done | `triage-issue-755` | Issue #755 (no labels, OPEN): Current dev review: Epic 8A Notification Preferences | 2026-06-06 |
| 1 | triage | done | `triage-issue-764` | Issue #764 (no labels, OPEN): Current dev review: Admin MFA & Auth Hardening | 2026-06-06 |
| 1 | triage | done | `triage-issue-765` | Issue #765 (no labels, OPEN): Current dev review: Integrations & Airbnb OAuth | 2026-06-06 |
| 1 | bug | done | `bug-mobile-voting-hardcoded-locale` | Mobile VotingScreen hardcodes en-US in toLocaleDateString — vote dates never localize | 2026-06-05 |
| 1 | bug | done | `bug-reality-web-listing-metadata-ssr-throw` | Reality-web listing generateMetadata can throw during SSR on malformed 200 body | 2026-06-05 |
| 1 | security | done | `security-pkce-oauth-authcode-pr-908-closed` | PR #908 (fix(security): require PKCE on OAuth authorization-code flow, closes #823) was closed unmerged — verify whether PKCE enforcement still pending | 2026-06-03 |
| 1 | triage | done | `triage-issue-751` | Issue #751 (no labels, OPEN): Current dev review: frontend/web/API-client findings | 2026-06-02 |
| 1 | triage | done | `triage-issue-752` | Issue #752 (no labels, OPEN): Current dev review: mobile CI tooling findings | 2026-06-02 |
| 1 | triage | done | `triage-issue-756` | Issue #756 (no labels, OPEN): Current dev review: Epic 10A OAuth Provider | 2026-06-02 |
| 1 | triage | done | `triage-issue-761` | Issue #761 (no labels, OPEN): Current dev review: Epic 84 E-Signature & Leases | 2026-06-02 |
| 1 | triage | done | `triage-issue-763` | Issue #763 (no labels, OPEN): Current dev review: Reality Server & Inquiries | 2026-06-02 |
| 1 | triage | done | `triage-issue-767` | Issue #767 (no labels, OPEN): Current dev review: Mobile RN Property Management app | 2026-06-02 |
| 1 | triage | done | `triage-issue-768` | Issue #768 (no labels, OPEN): Current dev review: Admin-web features (10B) | 2026-06-02 |
| 1 | triage | done | `triage-issue-920` | Issue #920 (no labels, OPEN): Announcement targeting not enforced on read (intra-org disclosure) | 2026-06-02 |
| 1 | triage | done | `triage-issue-750` | Issue #750 (no labels, OPEN): Current dev review: backend/API/database findings | 2026-06-01 |
| 1 | triage | done | `triage-issue-753` | Issue #753 (no labels, OPEN): Current dev review: Epic 6 Announcements & Communication | 2026-06-01 |
| 1 | triage | done | `triage-issue-754` | Issue #754 (no labels, OPEN): Current dev review: Epic 7A Basic Document Management | 2026-06-01 |
| 1 | triage | done | `triage-issue-757` | Issue #757 (no labels, OPEN): Current dev review: Epic 10B Platform Administration | 2026-06-01 |
| 1 | triage | done | `triage-issue-760` | Issue #760 (no labels, OPEN): Current dev review: Epic 79 Disputes & Mediation | 2026-06-01 |
| 1 | triage | done | `triage-issue-762` | Issue #762 (no labels, OPEN): Current dev review: Reports & Schedules | 2026-06-01 |
| 1 | triage | done | `triage-issue-766` | Issue #766 (no labels, OPEN): Current dev review: AI & LLM routes | 2026-06-01 |
| 1 | triage | done | `triage-issue-770` | Issue #770 (no labels, OPEN): Current dev review: Faults & triage | 2026-06-01 |
| 1 | triage | done | `triage-issue-771` | Issue #771 (no labels, OPEN): Current dev review: Research dispatcher & CI automation | 2026-06-01 |
| 1 | triage | done | `triage-issue-772` | Issue #772 (no labels, OPEN): Current dev review: Auth core (delta confirmation) | 2026-06-01 |
| 1 | triage | done | `triage-issue-773` | Issue #773 (no labels, OPEN): Current dev review: Leases & rental | 2026-06-01 |
| 1 | triage | done | `triage-issue-774` | Issue #774 (no labels, OPEN): Current dev review: Reality server (broad) | 2026-06-01 |
| 1 | triage | done | `triage-issue-775` | Issue #775 (no labels, OPEN): Current dev review: WebSocket realtime | 2026-06-01 |
| 1 | triage | done | `triage-issue-776` | Issue #776 (no labels, OPEN): Current dev review: Equipment & audit log | 2026-06-01 |
| 1 | triage | done | `triage-issue-777` | Issue #777 (no labels, OPEN): Current dev review: Compliance & GDPR | 2026-06-01 |
| 1 | triage | done | `triage-issue-778` | Issue #778 (no labels, OPEN): Current dev review: Marketplace, voting, investor portal, impersonation | 2026-06-01 |
| 1 | triage | done | `triage-issue-788` | Issue #788 (no labels, OPEN): Dev review rounds 1-5: mobile-native + ppt-web surfaces | 2026-06-01 |
| 1 | triage | done | `triage-issue-790` | Issue #790 (no labels, OPEN): Dev review rounds 11-15: vendor, predictive, reality-web, middleware | 2026-06-01 |
| 1 | triage | done | `triage-issue-791` | Issue #791 (no labels, OPEN): Dev review rounds 16-20: push, e-sign, portal, webhooks, reserves | 2026-06-01 |
| 1 | triage | done | `triage-issue-846` | Issue #846 (no labels, OPEN): Code review: Epics 12+65 — Meters & Energy/ESG (origin/dev) | 2026-06-01 |
| 1 | triage | done | `triage-issue-847` | Issue #847 (no labels, OPEN): Code review: Reality-server — Inquiries IDOR (Epics 16–19) (origin/dev) | 2026-06-01 |
| 1 | triage | done | `triage-issue-848` | Issue #848 (no labels, OPEN): Code review: Epics 78+134 — Vendor portal stubs & Predictive maintenance gaps (origin/dev) | 2026-06-01 |
| 1 | triage | done | `triage-issue-850` | Issue #850 (no labels, OPEN): Code review: Epics 61+146+42 — Multi-currency, Data residency, Violations (origin/dev) | 2026-06-01 |
| 1 | triage | done | `triage-issue-851` | Issue #851 (no labels, OPEN): Code review: Epics 15+105+69 — Listings/syndication & Developer API stubs (origin/dev) | 2026-06-01 |
| 1 | triage | done | `triage-issue-859` | Issue #859 (no labels, OPEN): sqlx 0.9 breaks runtime decode of Postgres enum columns into Rust String (SELECT * reads 500) | 2026-06-01 |
| 1 | triage | done | `triage-issue-867` | Issue #867 (no labels, OPEN): Tech debt: api-server main.rs duplicates lib.rs::create_router — routers diverge silently | 2026-06-01 |
| 1 | triage | done | `triage-issue-836` | Issue #836 (no labels, OPEN): Code review: Epic 2B-C — Mobile push & device registration (origin/dev) | 2026-05-31 |
| 1 | triage | done | `triage-issue-845` | Issue #845 (no labels, OPEN): Code review: Epic 14 — IoT alerts, correlations, thresholds (origin/dev) | 2026-05-31 |
| 1 | triage | done | `triage-issue-849` | Issue #849 (no labels, OPEN): Code review: Epic 10B+143 — Admin impersonation, Help, Board meetings auth (origin/dev) | 2026-05-31 |
| 0 | refactor | dropped | `churn-hotspot-backend-servers-api-server-src-routes-vendors-rs` | Churn hotspot: 929 lines changed in backend/servers/api-server/src/routes/vendors.rs (window 2026-06 | 2026-06-24 |
| 0 | refactor | dropped | `churn-hotspot-backend-servers-api-server-src-routes-enhanced-tenant-screening-rs` | Churn hotspot: 709 lines changed in backend/servers/api-server/src/routes/enhanced_tenant_screening. | 2026-06-24 |
| 0 | refactor | dropped | `churn-hotspot-backend-crates-db-src-repositories-subscription-rs` | Churn hotspot: 2856 lines changed in backend/crates/db/src/repositories/subscription.rs (window 2026-06-10 03:05Z→18:30Z) | 2026-06-24 |
| 0 | refactor | dropped | `churn-hotspot-backend-servers-api-server-src-routes-aml-dsa-rs` | Churn hotspot: 2691 lines changed in backend/servers/api-server/src/routes/aml_dsa.rs (window 2026-06-10 03:05Z→18:30Z) | 2026-06-24 |
| 0 | triage | dropped | `triage-issue-1151` | Issue #1151 (no labels, OPEN): Research dispatcher: claimable buffer is stale — true claimable work = 0 despite metric=53 | 2026-06-24 |
| 0 | refactor | dropped | `churn-hotspot-mobile-native-listing-detail-kt` | Churn hotspot: ListingDetailScreen.kt — +1279 LOC this run (gap-82-4 reality mobile favorite toggle) | 2026-06-24 |
| 0 | refactor | dropped | `churn-hotspot-mobile-native-search-screen-kt` | Churn hotspot: SearchScreen.kt — +1293 LOC this run (gap-82-3 reality mobile search/filters) | 2026-06-24 |
| 0 | refactor | dropped | `code-review-mobile-native-kmp-deeplink-android-bypasses-shared-router` | MainActivity reimplements deep-link dispatch instead of calling shared DeepLinkRouter — drift trap | 2026-06-24 |
| 0 | refactor | dropped | `refactor-churn-hotspot-mobile-announcements` | Churn hotspot: AnnouncementsScreen.tsx — 4 PRs this run, instability proxy | 2026-06-24 |
| 0 | refactor | dropped | `refactor-churn-hotspot-mobile-announcements-test` | Churn hotspot: AnnouncementsScreen.test.ts — 4 PRs this run, instability proxy | 2026-06-24 |
| 0 | refactor | dropped | `refactor-churn-hotspot-mobile-documents` | Churn hotspot: DocumentsScreen.tsx — 3 PRs this run | 2026-06-24 |
| 0 | triage | dropped | `triage-dispatcher-mcp-push-large-file-issue-1014` | Dispatcher action-list.json corruption when MCP push falls back from blocked git push | 2026-06-24 |
| 0 | triage | dropped | `triage-issue-951` | Issue #951 (no labels, OPEN): Deploy blocker: api-server requires ESIGN_TOKEN_SECRET + ESIGN_WEBHOOK_SECRET not injected by deploy-server (staging/prod) | 2026-06-24 |
| 0 | refactor | dropped | `refactor-oauth-integration-tests-hot` | Stabilize oauth_integration_tests churn — heavy edits across 3 OAuth fix PRs | 2026-06-16 |
| 0 | triage | dropped | `triage-issue-779` | Issue #779 (no labels, OPEN): Current dev review: consolidated priority rollup (origin/dev snapshot) | 2026-06-13 |
| 0 | bug | dropped | `bug-announcer-stale-message` | Announcer: untracked clear-then-set timeouts can resurrect a stale screen-reader message | 2026-06-07 |
| 0 | dx | dropped | `dx-portfolio-dashboard-stubs` | Portfolio dashboard: alert mark-read/resolve mutations + property-card click navigation are no-op stubs | 2026-06-04 |
