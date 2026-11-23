# Action list — delivery backlog

<sub>Generated: 2026-11-23T04:10:00Z</sub>

- **Open:** 49
- **In-progress:** 7
- **Done:** 5
- **Failed:** 0

| ID | Priority | Owner | Action | Dep | Status | Deadline |
|----|----------|-------|--------|-----|--------|----------|
| `pm-devops-unblock-mobile-native-cloud-builds` | high | `pm-devops` | Unblock mobile-native/KMP builds in the cloud runner (issue #2652) — still 5+ open backlog items structurally unclaimable in cloud; now compounded by #2966 s... | none | open | — |
| `pm-scrum-master-cloud-runner-kmp-swagger-ui-egress-unblock-2026-11-16` | high | `pm-scrum-master` | Unblock mobile-native/KMP and swagger-ui egress in the cloud runner (#2652, #2966). Claimable work is a small share of the backlog, and backend security PRs ... | none | open | — |
| `pm-scrum-master-i18n-flip-countdown-8d-2026-11-23` | high | `pm-scrum-master` | i18n warn→error flip is 2026-12-01 (8 days). Promote-or-close drafts #3740/#3741 (reality-web locale), #3739 (realtor/[id] parity), #3742 (ppt-web Registry R... | pm-frontend | open | 2026-12-01 |
| `pm-scrum-master-unblock-kmp-and-swagger-ui-egress-in-the-cloud-runner-2652-2-2026-11-20` | high | `pm-scrum-master` | Unblock KMP and swagger-ui egress in the cloud runner (#2652, #2966) | none | open | — |
| `pm-scrum-master-unblock-kmp-swagger-ui-egress-2652-2966-2026-11-21` | high | `pm-scrum-master` | Unblock KMP + swagger-ui egress in the cloud runner (#2652, #2966) — claimable still ≈9/72 | none | open | — |
| `pm-scrum-master-unblock-mobile-native-kmp-swagger-ui-egress-in-the-cloud-run-2026-11-22` | high | `pm-scrum-master` | Unblock mobile-native/KMP + swagger-ui egress in the cloud runner (#2652, #2966) — 4 KMP backlog items still structurally unclaimable | none | open | — |
| `code-review-api-core-notif-action-stub-noop` | medium | `pm-backend` | api core notif action stub noop | — | open | — |
| `code-review-mobile-native-kmp-create-listing-not-wired` | medium | `pm-backend` | KMP realtor CreateListingScreen onSubmit is a NotImplementedError stub — form data discarded | — | open | — |
| `code-review-mobile-native-kmp-inquiries-response-contract` | medium | `pm-backend` | mobile-native-kmp InquiriesResponse required page_size mismatches reality-server `limit` — MissingFieldException on every real /inquiries + /realtors/inquiri... | — | open | — |
| `gh-issue-3738` | medium | `pm-tech-lead` | Follow-up: SSO pending-session capacity eviction does a full O(n log n) sort on every mint under flood (PR #3730) (Closes #3738) | — | open | — |
| `pm-backend-address-mobile-native-stubs-2026-11-21` | medium | `pm-backend` | Address mobile-native stubs: CreateListingScreen onSubmit NotImplementedError + InquiriesResponse page_size/limit contract mismatch + Ktor HttpTimeout defaul... | none | open | — |
| `pm-qa-confirm-non-kmp-medium-confidence-2026-11-09` | medium | `pm-qa` | QA audit: non-mobile-native backlog items with confidence=medium need a one-shot verification step before Phase 3 can promote them | none | open | — |
| `pm-qa-test-shadow-debt-fcm-worker-push-2026-11-23` | medium | `pm-qa` | Add regression tests covering the 2xx-FCM-delivered + worker-requeue-drop + push-db-err-leak fixes (#3725/#3726/#3731). All three are single-file source-only... | none | open | — |
| `pm-scrum-master-airbnb-env-leak-flake-2026-11-19` | medium | `pm-scrum-master` | Fix env-var leak in tests/suites/airbnb_install_routes_tests.rs (set_var without cleanup) causing airbnb_oauth_routes_tests flake in parallel suite_1 | pm-qa | open | — |
| `pm-scrum-master-draft-3676-mobile-rn-raw-status-untranslated-2026-11-23` | medium | `pm-scrum-master` | Draft #3676 (code-review-mobile-rn-raw-status-untranslated) is NOT draft but open with updatedAt 2026-11-22 — 22h stale. Nudge author or promote to ready-for... | pm-frontend | open | — |
| `churn-hotspot-frontend-apps-ppt-web-src-features-accounting-components-accountinginvoicedetail-tsx` | low | `pm-tech-lead` | Churn hotspot: AccountingInvoiceDetail.tsx — 237 LoC in PR #3703 (wire-format fix) | — | open | — |
| `code-review-api-handlers-inproc-ratelimit-not-distributed` | low | `pm-backend` | api handlers inproc ratelimit not distributed | — | open | — |
| `code-review-api-handlers-voice-webhook-rls-release-inconsistent` | low | `pm-backend` | api handlers voice webhook rls release inconsistent | — | open | — |
| `code-review-mobile-native-kmp-http-client-no-timeout` | low | `pm-backend` | Shared Ktor HttpClient has no HttpTimeout - mobile API calls can hang indefinitely | — | open | — |
| `code-review-mobile-native-kmp-my-listings-pagination-cap` | low | `pm-backend` | Reality mobile MyListings + portfolio analytics silently cap at 100 listings (no pagination) | — | open | — |
| `code-review-mobile-native-kmp-sso-error-body-unguarded` | low | `pm-backend` | KMP realtor SSO error-body parsing is unguarded — crashes on non-JSON response | — | open | — |
| `code-review-mobile-rn-hardcoded-button-labels` | low | `pm-backend` | mobile rn hardcoded button labels | — | open | — |
| `code-review-mobile-rn-raw-status-untranslated` | low | `pm-backend` | mobile rn raw status untranslated | — | open | — |
| `code-review-ppt-web-core-authctx-no-token-expiry-check` | low | `pm-backend` | ppt web core authctx no token expiry check | — | open | — |
| `code-review-ppt-web-core-orgprovider-unmemoized-retry1` | low | `pm-backend` | ppt web core orgprovider unmemoized [retry 1/2 of failed code-review-ppt-web-core-orgprovider-unmemoized] | — | open | — |
| `code-review-ppt-web-core-route-toasts-i18n-gap-retry1` | low | `pm-backend` | ppt web core route toasts i18n gap [retry 1/2 of failed code-review-ppt-web-core-route-toasts-i18n-gap] | — | open | — |
| `code-review-ppt-web-ui-accounting-invoice-form-no-i18n` | low | `pm-backend` | ppt web ui accounting invoice form no i18n | — | open | — |
| `code-review-ppt-web-ui-building-detail-no-i18n` | low | `pm-backend` | ppt web ui building detail no i18n | — | open | — |
| `code-review-ppt-web-ui-buildings-pages-no-i18n` | low | `pm-backend` | ppt web ui buildings pages no i18n | — | open | — |
| `code-review-ppt-web-ui-dsa-reports-native-alert-ux` | low | `pm-tech-lead` | ppt web ui dsa reports native alert ux | — | open | — |
| `code-review-ppt-web-ui-editfaultpage-no-i18n-retry1` | low | `pm-backend` | ppt web ui editfaultpage no i18n [retry 1/2 of failed code-review-ppt-web-ui-editfaultpage-no-i18n] | — | open | — |
| `code-review-ppt-web-ui-file-dispute-page-no-i18n` | low | `pm-backend` | ppt web ui file dispute page no i18n | — | open | — |
| `code-review-ppt-web-ui-forms-feature-no-i18n` | low | `pm-backend` | ppt web ui forms feature no i18n | — | open | — |
| `code-review-ppt-web-ui-guest-report-submission-no-i18n` | low | `pm-backend` | ppt web ui guest report submission no i18n | — | open | — |
| `code-review-ppt-web-ui-mediation-notes-swallowed-load-error` | low | `pm-backend` | ppt web ui mediation notes swallowed load error | — | open | — |
| `code-review-ppt-web-ui-moderation-native-prompt-flow` | low | `pm-backend` | ppt web ui moderation native prompt flow | — | open | — |
| `code-review-ppt-web-ui-move-folder-dialog-hardcoded-sk` | low | `pm-backend` | ppt web ui move folder dialog hardcoded sk | — | open | — |
| `code-review-ppt-web-ui-registry-fullpage-nav` | low | `pm-tech-lead` | ppt web ui registry fullpage nav | — | open | — |
| `code-review-ppt-web-ui-registry-page-no-i18n-retry1` | low | `pm-backend` | ppt web ui registry page no i18n [retry 1/2 of failed code-review-ppt-web-ui-registry-page-no-i18n] | — | open | — |
| `code-review-ppt-web-ui-registry-silent-mutation-errors` | low | `pm-backend` | ppt web ui registry silent mutation errors | — | open | — |
| `code-review-ppt-web-ui-reports-page-no-i18n` | low | `pm-backend` | ppt web ui reports page no i18n | — | open | — |
| `code-review-reality-server-url-validator-cgnat-gap` | low | `pm-backend` | reality server url validator cgnat gap | — | open | — |
| `code-review-reality-web-csv-import-filepick-no-type-guard-retry1` | low | `pm-backend` | reality web csv import filepick no type guard [retry 1/2 of failed code-review-reality-web-csv-import-filepick-no-type-guard] | — | open | — |
| `code-review-reality-web-listing-edit-no-i18n-retry1` | low | `pm-backend` | reality web listing edit no i18n [retry 1/2 of failed code-review-reality-web-listing-edit-no-i18n] | — | open | — |
| `code-review-reality-web-realtor-hub-analytics-i18n-retry1` | low | `pm-backend` | reality web realtor hub analytics i18n [retry 1/2 of failed code-review-reality-web-realtor-hub-analytics-i18n] | — | open | — |
| `code-review-reality-web-realtor-id-no-i18n-retry1` | low | `pm-backend` | reality web realtor id no i18n [retry 1/2 of failed code-review-reality-web-realtor-id-no-i18n] | — | open | — |
| `pm-integration-property-type-enum-stability` | low | `pm-integration` | Verify #3609 property_type i18n helper keeps API filter contract on stable enum codes across sk/cs/de/en; localize labels only in display fields | pm-qa | open | — |
| `pm-qa-no-test-diff-sweep-rate-2026-11-23` | low | `pm-qa` | Audit this weeks no-test-diff PRs (#3714 30 files, #3718 3 files, #3724 characterize, #3728 screen-doc) and classify each as docs-only, inline-#[cfg(test)], ... | none | open | — |
| `screen-map-drift-pr-3695-reality` | low | `pm-qa` | screen-map drift: PR #3695 changed reality-web /report route without touching docs/screens/reality/ | — | open | — |
| `pm-scrum-master-land-the-2026-12-01-i18n-warn-error-lint-flip-burn-down-merg-2026-11-22` | high | `pm-scrum-master` | Land the 2026-12-01 i18n warn→error lint-flip burn-down: merge or close the 10+ open `code-review-*-i18n-retry1` items + drafts #3676/#3677 within 9 days | none | in-progress | — |
| `code-review-ppt-web-core-logout-purge-disputes-leases-iot-retry1` | medium | `pm-security` | ppt web core logout purge disputes leases iot [retry 1/2 of failed code-review-ppt-web-core-logout-purge-disputes-leases-iot] | — | in-progress | — |
| `code-review-api-core-digest-worker-no-replica-claim` | low | `pm-backend` | api core digest worker no replica claim | — | in-progress | — |
| `code-review-api-handlers-summarize-discards-persist-err` | low | `pm-backend` | api handlers summarize discards persist err | — | in-progress | — |
| `code-review-api-handlers-tenant-config-rls-clear-swallowed` | low | `pm-backend` | api handlers tenant config rls clear swallowed | — | in-progress | — |
| `code-review-mobile-rn-list-search-placeholders-i18n` | low | `pm-backend` | mobile rn list search placeholders i18n | — | in-progress | — |
| `code-review-ppt-web-ui-bookfacility-silent-availability` | low | `pm-backend` | ppt web ui bookfacility silent availability | — | in-progress | — |
| `pm-scrum-master-resolve-dispatcher-stall-3357-3-approved-prs-still-blocked-o-2026-11-22` | high | `pm-scrum-master` | Resolve dispatcher stall #3357 — 3 approved PRs still blocked on non-mechanical conflicts across TWO merge waves (32 + 26 PRs) | none | done | — |
| `pm-frontend-characterize-the-two-high-churn-frontend-files-this-week-acc-2026-11-22` | medium | `pm-frontend` | Characterize the two high-churn frontend files this week (AccountingInvoiceDetail.tsx, reality-web report/page.tsx, leases.tsx) — if each is >800 lines and t... | none | done | — |
| `pm-frontend-close-out-the-carried-wire-the-button-but-not-the-post-patte-2026-11-22` | medium | `pm-frontend` | Close out the carried 'wire-the-button-but-not-the-POST' pattern: audit `ReportFaultScreen.tsx` (RN), reality-web `/sell` wizard Publish, and `VoteDetailPage... | none | done | — |
| `pm-frontend-confirm-the-ws-token-in-url-strip-from-3696-covers-every-web-2026-11-22` | medium | `pm-frontend` | Confirm the WS token-in-URL strip from #3696 covers every WebSocket opener on ppt-web (not just notifications) — search for `new WebSocket(` call sites and a... | none | done | — |
| `pm-scrum-master-characterize-the-post-split-push-voice-webhooks-oauth-integr-2026-11-22` | medium | `pm-scrum-master` | Characterize the post-split push/ + voice_webhooks/ + oauth_integration_tests/ child modules (watch-* in current_sprint) and promote to specific refactor tasks | none | done | — |
