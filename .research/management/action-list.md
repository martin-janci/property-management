# PPT Action List

Generated: 2026-06-17T00:00:00Z
Open: 50/70

| ID | Priority | Owner | Action | Status | Depends |
|---|---|---|---|---|---|
| feat-build-configuration-by-environ-build-scripts-created-mobile | high | pm-frontend | build scripts (scripts/build-mobile\|android\|ios.sh) not created (85-2-build-configuration Build Configuration by Environ | open | - |
| feat-build-configuration-by-environ-ios-xcconfig-files-mobile | high | pm-frontend | iOS xcconfig files + schemes not created (85-2-build-configuration Build Configuration by Environment) | open | - |
| feat-environment-variable-setup-env-setup-documentation-mobile | high | pm-frontend | env setup documentation missing (85-1-environment-variables Environment Variable Setup) | open | - |
| feat-environment-variable-setup-ios-info-lacks-api-base-url-mobile | high | pm-frontend | iOS Info.plist lacks API_BASE_URL/ENVIRONMENT keys (85-1-environment-variables Environment Variable Setup) | open | - |
| feat-environment-variable-setup-react-native-config-metro-bundler-mobile | high | pm-frontend | react-native-config + Metro bundler setup not done (Expo Constants used) (85-1-environment-variables Environment Variabl | open | - |
| feat-announcement-viewing-acknowled-web-viewing-ack-ui-backend | high | pm-frontend | web viewing/ack UI in draft PRs #474/#475/#479 (Epic 6 announcement web UI) — not yet merged (6-2-announcement-viewing-a | open | - |
| feat-dispute-filing-flow-task-checklist-unchecked-frontend | high | pm-frontend | task checklist unchecked (80-2-dispute-filing-flow Dispute Filing Flow) | open | - |
| bug-ios-searchview-uncompilable | high | pm-frontend | iOS SearchView.swift does not compile — performSearch/scheduleSearch undefined, resultsGrid corrupted | open | - |
| pm-qa-service-history-cross-org-idor | high | pm-qa | Add cross-org IDOR test coverage to service-history endpoints (#1372) | open | - |
| pm-qa-triage-followup-issues-2026-06-14 | high | pm-scrum-master | Triage the 18 follow-up issues #1360-#1377 from 2026-06-14 post-merge review — assign owners or close | open | - |
| pm-qa-close-issue-1332-if-ci-green | high | pm-scrum-master | Close issue #1332 if dev CI now green after PR #1379 unblock | open | - |
| pm-qa-record-payment-atomicity-test | high | pm-qa | Add regression test for record_payment non-atomic check-then-insert (#1361) — concurrent double-pay scenario | open | - |
| pm-devops-fix-dev-compile-1437 | high | pm-devops | URGENT: Fix `dev` backend compile break introduced by PR #1426 — land #1435 or #1436. Blocks ALL backend CI gates until  | open | - |
| pm-devops-dev-push-compile-gate | high | pm-devops | Add a `cargo check --workspace --tests` smoke gate on `dev` push (not just PR) — would have caught #1426 → #1437 before  | open | - |
| pm-security-make-backend-test-job-a-required-status-check-on-d | high | pm-security | Make backend `test` job a required status check on dev (issue #1538) — gates RLS/OAuth/credential-encryption regression  | open | - |
| pm-security-fix-issue-481-restore-revoked-at-is-null-filter-in | high | pm-security | Fix issue #481: restore revoked_at IS NULL filter in OAuth refresh-token production lookup query (RFC 9700) | open | - |
| pm-security-fix-issue-480-stop-logging-ws-auth-token-from-quer | high | pm-security | Fix issue #480: stop logging WS auth token from query param; move to header/cookie or redact at access log | open | - |
| pm-security-resolve-prior-run-outstanding-614-624-cross-tenant | high | pm-security | Resolve prior-run outstanding #614/#624 (cross-tenant schedule mutation + missing RBAC on update_schedule) and #617 (coo | open | - |
| pm-scrum-master-fix-ci-make-backend-test-job-a-required-check-on-d | high | pm-scrum-master | Fix CI: make backend `test` job a required check on `dev` branch (issue #1538) | open | - |
| pm-scrum-master-resolve-test-hardening-issues-480-jwt-in-ws-logs-4 | high | pm-scrum-master | Resolve test-hardening issues #480 (JWT in WS logs) + #481 (OAuth refresh-token revocation bypass) | open | - |
| pm-scrum-master-land-epic-6-announcement-web-ui-draft-prs-474-475 | high | pm-scrum-master | Land Epic 6 announcement web UI (draft PRs #474/#475/#479) to advance 6-2/6-3/6-4 | open | - |
| refactor-churn-hotspot-mobile-announcements-test | low | pm-frontend | Churn hotspot: AnnouncementsScreen.test.ts — 4 PRs this run, instability proxy | open | - |
| verify-document-upload-metadata-promote-to-done-backend | low | pm-backend | Coverage 7a-1: verify Document Upload with Metadata end-to-end and promote sprint-status ready-for-dev -> done (confirm  | open | - |
| verify-report-execution-history-presigned-download-retry-frontend | low | pm-frontend | Coverage 81-2: confirm presigned download + retry end-to-end on the report execution-history surface (ppt-web); add a co | open | - |
| feat-listing-detail-favorites-swiftui-polish-mobile | low | pm-frontend | Coverage 82-4: complete the remaining SwiftUI listing-detail polish for the Reality mobile Listing Detail & Favorites sc | open | - |
| pm-devops-app-tsx-merge-queue-confirm | low | pm-devops | Confirm `app-tsx-merge-queue.yml` is actively serializing App.tsx-touching PRs (carry-over from 2026-05-27). Verify it's | open | - |
| feat-airbnb-oauth-token-exchange-route-backend | medium | pm-backend | Coverage 83-1 (AC): implement the Airbnb OAuth token-exchange route (authorization-code → access/refresh token) for the  | open | - |
| feat-airbnb-realtime-webhook-handler-backend | medium | pm-backend | Coverage 83-1 (AC): implement the Airbnb realtime webhook handler (reservation/availability notifications) — signature v | open | - |
| fix-report-schedule-editing-cron-edits-round-trip-through-frontend | medium | pm-frontend | cron edits round-trip through the overloaded `time` field — no dedicated cron_expression column (backlog bug-report-sche | open | - |
| feat-swiftui-project-setup-no-epic-82-commits-git-mobile | medium | pm-frontend | no epic-82 commits in git log (82-1-swiftui-project-setup SwiftUI Project Setup) | open | - |
| feat-swiftui-project-setup-no-screen-map-ppt-reality-mobile-docs-mobile | medium | pm-frontend | no screen-map for ppt-reality-mobile in docs/screens/ (82-1-swiftui-project-setup SwiftUI Project Setup) | open | - |
| feat-swiftui-project-setup-vs-implementation-mapping-mobile | medium | pm-frontend | story-vs-implementation mapping unclear (epic 82 in epics-007.md targets different scope) (82-1-swiftui-project-setup Sw | open | - |
| verify-navigation-and-routing-deep-linking-url-schemes-confirmed-mobile | medium | pm-frontend | deep-linking with URL schemes not confirmed (82-2-navigation-routing Navigation and Routing) | open | - |
| feat-airbnb-oauth-and-sync-no-integrations-airbnb-backend | medium | pm-backend | no /integrations/airbnb/* API routes (83-1-airbnb-integration Airbnb OAuth and Sync) | open | - |
| pm-qa-tenant-aware-rls-helper | medium | pm-qa | Introduce canonical tenant-aware request helper for RLS tests (#1370) | open | - |
| pm-qa-canonical-seed-membership-helper | medium | pm-qa | Migrate duplicated seed_membership in IDOR tests to canonical helper (#1373) | open | - |
| pm-qa-cron-validator-drift-test | medium | pm-qa | Add silent-regression test for cron validator to prevent #616 reintroduction (#1368) | open | - |
| pm-qa-stale-draft-pr-decision | medium | pm-scrum-master | Decide on stale draft PRs #1316 (1.8d) and #1197 (5.9d) — promote, rebase or close | open | - |
| pm-qa-allowed-pet-types-enum-decode-audit | medium | pm-qa | Audit allowed_pet_types enum decode paths + add unit test for unknown variants (#1363, #1366) | open | - |
| pm-qa-ios-searchview-pagination-stale-guard | medium | pm-qa | Add iOS UI test for SearchView stale-response guard preserving pagination (#1365) | open | - |
| pm-qa-dispute-draft-autosave-coverage | medium | pm-qa | Add dispute draft auto-save tests — i18n key presence + re-render race (#1360, #1364) | open | - |
| pm-qa-record-reserve-transaction-atomicity | medium | pm-qa | Add concurrency test for record_reserve_transaction atomicity + COALESCE on budget aggregates (#1371) | open | - |
| feat-announcement-comments-discussion-web-ui-frontend | medium | pm-frontend | Coverage 6-3: implement/wire the announcement comments & discussion web UI in ppt-web (comment list + threaded reply com | open | - |
| feat-document-download-preview-mobile-slice | medium | pm-frontend | Coverage 7a-4: implement the mobile document download & preview slice (presigned download + inline preview) on the mobil | open | - |
| feat-esignature-email-webhook-idempotency-guard-backend | medium | pm-backend | Coverage 84-2: add the missing e-signature email webhook idempotency guard (terminal-state dedup) so duplicate provider  | open | - |
| pm-devops-confirm-eas-workflows-healthy | medium | pm-devops | Confirm `eas-build-android.yml` + `eas-build-ios.yml` (now in .github/workflows/) green on a no-op push — verify action  | open | - |
| pm-devops-security-test-gate-required-check | medium | pm-devops | Confirm `security-test-gate.yml` is configured as a required status check on `dev` branch protection (`gh api repos/.../ | open | - |
| pm-devops-pre-push-fmt-gate-scope | medium | pm-devops | Decide pre-push fmt/clippy gate (#1431 merged) scope — local hook only, mirror as CI status check, or both. Local-only d | open | - |
| pm-security-add-mfa-rate-limit-regression-tests-issue-487-gate | medium | pm-security | Add MFA rate-limit regression tests (issue #487) — gates story 10a-1-oauth-authorization-server | open | - |
| pm-security-audit-announce-faults-direct-gettoken-bypass-486-d | medium | pm-security | Audit announce/faults direct getToken() bypass (#486) — dual-path auth skips axios refresh interceptor | open | - |
| feat-airbnb-oauth-and-sync-no-oauth-token-exchange-backend | high | pm-backend | no OAuth token exchange route (models only) (83-1-airbnb-integration Airbnb OAuth and Sync) | done | - |
| feat-navigation-and-routing-auth-guard-evidenced-mobile | high | pm-frontend | auth guard (AC-5) not evidenced (82-2-navigation-routing Navigation and Routing) | done | - |
| pm-qa-document-download-preview-tests | high | pm-qa | Add presigned-URL minting/expiry/access-gate allow-path tests for document download/preview (#1377) | done | - |
| pm-qa-realtime-pref-sync-ci-coverage | high | pm-qa | Add CI-executable coverage for realtime preference-sync publish leg (#1376) | done | - |
| pm-qa-booking-oauth-csrf-coverage | high | pm-qa | Add Booking.com OAuth handler / CSRF / secure-credential-replacement coverage (#1362, #1374) | done | - |
| pm-qa-vote-partial-cmp-nan-fuzz | high | pm-qa | Add NaN-weight fuzz test for /votes/{id}/results to guard partial_cmp().unwrap() panic (Phase 1.5 finding) | done | - |
| verify-document-folder-organization-backend-promote | low | pm-backend | Coverage 7a-2: verify the document folder-organization backend (CRUD + RLS + capability gates) against the story ACs and | done | - |
| feat-web-announcements-faults-api-hooks-wiring | medium | pm-frontend | [CLOSED: verified already shipped on dev (gap-79-1) — AnnouncementsPage+FaultsPage wired to TanStack hooks, 22 tests pas | done | - |
| feat-home-and-search-screens-debounced-search-evidenced-mobile | medium | pm-frontend | [CLOSED-already-done-on-dev] debounced search (AC-2) not evidenced (82-3-home-search-screens Home and Search Screens) | done | - |
| feat-home-and-search-screens-filtersheet-location-features-mobile | medium | pm-frontend | FilterSheet/location features missing (82-3-home-search-screens Home and Search Screens) | done | - |
| feat-navigation-and-routing-navigation-state-preservation-mobile | medium | pm-frontend | navigation state preservation (AC-4) unverified (82-2-navigation-routing Navigation and Routing) | done | - |
| verify-home-and-search-screens-corelocation-integration-confirmed-mobile | medium | pm-frontend | CoreLocation integration not confirmed (82-3-home-search-screens Home and Search Screens) | done | - |
| pm-qa-pre-push-fmt-clippy-gate | medium | pm-qa | [CLOSED-already-done-on-dev] Add pre-push cargo fmt + clippy gate (#1375) — prevents unformatted re-lands | done | - |
| pm-qa-rls-write-download-coverage | medium | pm-qa | Extend forms RLS write/download path coverage + tighten release() discipline (#1369) | done | - |
| feat-pinned-announcements-pin-unpin-web-ui-frontend | medium | pm-frontend | Coverage 6-4: add pin/unpin announcement controls to the ppt-web announcements UI wired to the backend pin endpoint; ref | done | - |
| feat-folder-organization-mobile-implementation | medium | pm-frontend | Coverage 7a-2: implement the mobile (folder organization) slice for documents — folder tree/list + move/assign on the mo | done | - |
| feat-notification-pref-sync-mobile-push-fcm-apns-backend | medium | pm-backend | Coverage 8a-3: implement mobile OS push integration (FCM/APNs) for notification preference sync — device-token registrat | done | - |
| feat-home-and-search-screens-infinite-scroll-evidenced-mobile | medium | pm-frontend | Coverage 82-3: evidence infinite scroll (AC-4) on the Reality mobile (KMP) Search screen — add the missing pagination/sc | done | - |
| feat-booking-integration-ota-xml-parsing-backend | medium | pm-backend | Coverage 83-2: implement Booking.com OTA XML parsing/generation for rate/availability messages; include parser unit test | done | - |
| feat-booking-integration-rate-availability-push-backend | medium | pm-backend | Coverage 83-2 (AC-5): complete the Booking.com rate/availability push full flow (build + send OTA update, handle ack/err | done | - |
