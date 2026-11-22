# Action list — delivery backlog

<sub>Generated: 2026-11-22T03:06:30Z</sub>

- **Open:** 59
- **In-progress:** 6
- **Done:** 2

| ID | Priority | Owner | Action | Dep | Status | Deadline |
|----|----------|-------|--------|-----|--------|----------|
| `pm-devops-unblock-mobile-native-cloud-builds` | high | `pm-devops` | Unblock mobile-native/KMP builds in the cloud runner (issue #2652) — still 5+ open backlog items structurally unclaimable in cloud; now compounded by #2966 so claimable=9/72 (bu... | none | open | — |
| `pm-frontend-burn-down-the-frontend-i18n-slice-10-code-review-i18n-retry1-2026-11-22` | high | `pm-frontend` | Burn down the frontend-i18n slice (10+ `code-review-*-i18n-retry1` items + drafts #3676/#3677) before 2026-12-01 CI gate flip | none | open | — |
| `pm-frontend-promote-drafts-3714-ppt-web-date-number-locale-formatters-an-2026-11-22` | high | `pm-frontend` | Promote drafts #3714 (ppt-web date/number locale formatters) and #3715 (ppt-web native confirm dialogs) to ready-for-review within 48h — both are from the 2026-11-19/20 action-l... | none | open | — |
| `pm-frontend-verify-the-authed-query-key-roots-logout-purge-pattern-from--2026-11-22` | high | `pm-frontend` | Verify the `AUTHED_QUERY_KEY_ROOTS` logout-purge pattern from #3706 is comprehensive — audit every feature-group root query key, add a unit test that asserts the allow-list matc... | none | open | — |
| `pm-scrum-master-cloud-runner-kmp-swagger-ui-egress-unblock-2026-11-16` | high | `pm-scrum-master` | Unblock mobile-native/KMP and swagger-ui egress in the cloud runner (#2652, #2966). Claimable work is a small share of the backlog, and backend security PRs cannot compile. | none | open | — |
| `pm-scrum-master-land-the-2026-12-01-i18n-warn-error-lint-flip-burn-down-merg-2026-11-22` | high | `pm-scrum-master` | Land the 2026-12-01 i18n warn→error lint-flip burn-down: merge or close the 10+ open `code-review-*-i18n-retry1` items + drafts #3676/#3677 within 9 days | none | open | — |
| `pm-scrum-master-resolve-dispatcher-stall-3357-3-approved-prs-still-blocked-o-2026-11-22` | high | `pm-scrum-master` | Resolve dispatcher stall #3357 — 3 approved PRs still blocked on non-mechanical conflicts across TWO merge waves (32 + 26 PRs) | none | open | — |
| `pm-scrum-master-unblock-kmp-and-swagger-ui-egress-in-the-cloud-runner-2652-2-2026-11-20` | high | `pm-scrum-master` | Unblock KMP and swagger-ui egress in the cloud runner (#2652, #2966) | none | open | — |
| `pm-scrum-master-unblock-kmp-swagger-ui-egress-2652-2966-2026-11-21` | high | `pm-scrum-master` | Unblock KMP + swagger-ui egress in the cloud runner (#2652, #2966) — claimable still ≈9/72 | none | open | — |
| `pm-scrum-master-unblock-mobile-native-kmp-swagger-ui-egress-in-the-cloud-run-2026-11-22` | high | `pm-scrum-master` | Unblock mobile-native/KMP + swagger-ui egress in the cloud runner (#2652, #2966) — 4 KMP backlog items still structurally unclaimable | none | open | — |
| `code-review-mobile-native-kmp-create-listing-not-wired` | medium | `pm-backend` | KMP realtor CreateListingScreen onSubmit is a NotImplementedError stub — form data discarded | None | open | — |
| `code-review-mobile-native-kmp-inquiries-response-contract` | medium | `pm-backend` | mobile-native-kmp InquiriesResponse required page_size mismatches reality-server `limit` — MissingFieldException on every real /inquiries + /realtors/inquiries call | None | open | — |
| `gh-issue-3711` | medium | `pm-tech-lead` | dispatcher: signals-promote promotes stale already-resolved findings (needs dev-truth guard) (Closes #3711) | None | open | — |
| `pm-backend-address-mobile-native-stubs-2026-11-21` | medium | `pm-backend` | Address mobile-native stubs: CreateListingScreen onSubmit NotImplementedError + InquiriesResponse page_size/limit contract mismatch + Ktor HttpTimeout default. Package as one KM... | none | open | — |
| `pm-frontend-characterize-the-two-high-churn-frontend-files-this-week-acc-2026-11-22` | medium | `pm-frontend` | Characterize the two high-churn frontend files this week (AccountingInvoiceDetail.tsx, reality-web report/page.tsx, leases.tsx) — if each is >800 lines and touched by >3 differe... | none | open | — |
| `pm-frontend-close-out-the-carried-wire-the-button-but-not-the-post-patte-2026-11-22` | medium | `pm-frontend` | Close out the carried 'wire-the-button-but-not-the-POST' pattern: audit `ReportFaultScreen.tsx` (RN), reality-web `/sell` wizard Publish, and `VoteDetailPage` ranked-ballot subm... | none | open | — |
| `pm-frontend-confirm-the-ws-token-in-url-strip-from-3696-covers-every-web-2026-11-22` | medium | `pm-frontend` | Confirm the WS token-in-URL strip from #3696 covers every WebSocket opener on ppt-web (not just notifications) — search for `new WebSocket(` call sites and assert no `?token=` | none | open | — |
| `pm-qa-confirm-non-kmp-medium-confidence-2026-11-09` | medium | `pm-qa` | QA audit: non-mobile-native backlog items with confidence=medium need a one-shot verification step before Phase 3 can promote them | none | open | — |
| `pm-scrum-master-airbnb-env-leak-flake-2026-11-19` | medium | `pm-scrum-master` | Fix env-var leak in tests/suites/airbnb_install_routes_tests.rs (set_var without cleanup) causing airbnb_oauth_routes_tests flake in parallel suite_1 | pm-qa | open | — |
| `pm-scrum-master-characterize-the-post-split-push-voice-webhooks-oauth-integr-2026-11-22` | medium | `pm-scrum-master` | Characterize the post-split push/ + voice_webhooks/ + oauth_integration_tests/ child modules (watch-* in current_sprint) and promote to specific refactor tasks | none | open | — |
| `pm-scrum-master-promote-or-close-the-5-draft-prs-3713-dispatcher-dev-truth-g-2026-11-22` | medium | `pm-scrum-master` | Promote-or-close the 5 draft PRs (#3713 dispatcher dev-truth guard, #3714 ppt-web date/number locale, #3715 ppt-web native confirm dialogs, #3676/#3677 mobile-rn i18n) | none | open | — |
| `pm-scrum-master-triage-new-issue-3711-dispatcher-signals-promote-dev-truth-g-2026-11-22` | medium | `pm-scrum-master` | Triage new issue #3711 (dispatcher signals-promote dev-truth guard follow-up) into the dispatcher-reliability track | none | open | — |
| `code-review-api-core-fcm-2xx-parse-retry` | low | `pm-backend` | push/fcm.rs: 2xx FCM response with unexpected body shape triggers retry — duplicate-delivery risk | None | open | — |
| `code-review-api-core-push-db-err-leak` | low | `pm-security` | push/combined.rs + apns.rs leak raw sqlx::Error display into NotificationError::PushFailed | None | open | — |
| `code-review-api-core-worker-silent-requeue-drop` | low | `pm-backend` | push/worker.rs: Retry disposition silently drops job when Redis requeue fails (NOAUTH/LOADING/reset) | None | open | — |
| `code-review-api-handlers-upload-folderid-parse-swallow` | low | `pm-backend` | api handlers upload folderid parse swallow | None | open | — |
| `code-review-mobile-native-kmp-http-client-no-timeout` | low | `pm-backend` | Shared Ktor HttpClient has no HttpTimeout - mobile API calls can hang indefinitely | None | open | — |
| `code-review-mobile-native-kmp-my-listings-pagination-cap` | low | `pm-backend` | Reality mobile MyListings + portfolio analytics silently cap at 100 listings (no pagination) | None | open | — |
| `code-review-mobile-native-kmp-sso-error-body-unguarded` | low | `pm-backend` | KMP realtor SSO error-body parsing is unguarded — crashes on non-JSON response | None | open | — |
| `code-review-mobile-rn-hardcoded-button-labels` | low | `pm-backend` | mobile rn hardcoded button labels | None | open | — |
| `code-review-mobile-rn-raw-status-untranslated` | low | `pm-backend` | mobile rn raw status untranslated | None | open | — |
| `code-review-ppt-web-core-orgprovider-unmemoized-retry1` | low | `pm-backend` | ppt web core orgprovider unmemoized [retry 1/2 of failed code-review-ppt-web-core-orgprovider-unmemoized] | None | open | — |
| `code-review-ppt-web-ui-buildings-pages-no-i18n` | low | `pm-backend` | ppt web ui buildings pages no i18n | None | open | — |
| `code-review-ppt-web-ui-date-number-formatters-no-locale` | low | `pm-backend` | ppt web ui date number formatters no locale | None | open | — |
| `code-review-ppt-web-ui-document-hooks-hardcoded-sk` | low | `pm-backend` | ppt web ui document hooks hardcoded sk | None | open | — |
| `code-review-ppt-web-ui-dsa-reports-native-alert-ux` | low | `pm-tech-lead` | ppt web ui dsa reports native alert ux | None | open | — |
| `code-review-ppt-web-ui-file-dispute-page-no-i18n` | low | `pm-backend` | ppt web ui file dispute page no i18n | None | open | — |
| `code-review-ppt-web-ui-mediation-notes-swallowed-load-error` | low | `pm-backend` | ppt web ui mediation notes swallowed load error | None | open | — |
| `code-review-ppt-web-ui-mediation-sessions-swallowed-error` | low | `pm-backend` | ppt web ui mediation sessions swallowed error | None | open | — |
| `code-review-ppt-web-ui-move-folder-dialog-hardcoded-sk` | low | `pm-backend` | ppt web ui move folder dialog hardcoded sk | None | open | — |
| `code-review-ppt-web-ui-native-confirm-dialogs` | low | `pm-tech-lead` | ppt web ui native confirm dialogs | None | open | — |
| `code-review-ppt-web-ui-registry-fullpage-nav` | low | `pm-tech-lead` | ppt web ui registry fullpage nav | None | open | — |
| `code-review-ppt-web-ui-registry-page-no-i18n-retry1` | low | `pm-backend` | ppt web ui registry page no i18n [retry 1/2 of failed code-review-ppt-web-ui-registry-page-no-i18n] | None | open | — |
| `code-review-ppt-web-ui-registry-silent-mutation-errors` | low | `pm-backend` | ppt web ui registry silent mutation errors | None | open | — |
| `code-review-ppt-web-ui-registry-silent-mutation-fail-retry1` | low | `pm-backend` | ppt web ui registry silent mutation fail [retry 1/2 of failed code-review-ppt-web-ui-registry-silent-mutation-fail] | None | open | — |
| `code-review-ppt-web-ui-reports-page-no-i18n` | low | `pm-backend` | ppt web ui reports page no i18n | None | open | — |
| `code-review-reality-server-sso-sessions-unbounded-map` | low | `pm-backend` | reality server sso sessions unbounded map | None | open | — |
| `code-review-reality-server-url-validator-cgnat-gap` | low | `pm-backend` | reality server url validator cgnat gap | None | open | — |
| `code-review-reality-web-agency-slug-i18n` | low | `pm-backend` | reality web agency slug i18n | None | open | — |
| `code-review-reality-web-agency-slug-no-i18n-retry1` | low | `pm-backend` | reality web agency slug no i18n [retry 1/2 of failed code-review-reality-web-agency-slug-no-i18n] | None | open | — |
| `code-review-reality-web-compare-button-i18n` | low | `pm-backend` | reality web compare button i18n | None | open | — |
| `code-review-reality-web-comparison-tray-i18n` | low | `pm-backend` | reality web comparison tray i18n | None | open | — |
| `code-review-reality-web-csv-import-filepick-no-type-guard-retry1` | low | `pm-backend` | reality web csv import filepick no type guard [retry 1/2 of failed code-review-reality-web-csv-import-filepick-no-type-guard] | None | open | — |
| `code-review-reality-web-listing-edit-no-i18n-retry1` | low | `pm-backend` | reality web listing edit no i18n [retry 1/2 of failed code-review-reality-web-listing-edit-no-i18n] | None | open | — |
| `code-review-reality-web-listing-grid-empty-i18n` | low | `pm-backend` | reality web listing grid empty i18n | None | open | — |
| `code-review-reality-web-realtor-hub-analytics-i18n-retry1` | low | `pm-backend` | reality web realtor hub analytics i18n [retry 1/2 of failed code-review-reality-web-realtor-hub-analytics-i18n] | None | open | — |
| `code-review-reality-web-realtor-id-no-i18n-retry1` | low | `pm-backend` | reality web realtor id no i18n [retry 1/2 of failed code-review-reality-web-realtor-id-no-i18n] | None | open | — |
| `pm-backend-characterize-postsplit-modules-under-watch-2026-11-21` | low | `pm-backend` | Characterize the post-split push/ and voice_webhooks/ child modules in sprint-status and coverage; the new file footprint should move to a 'done, under watch' bucket for the nex... | none | open | — |
| `pm-integration-property-type-enum-stability` | low | `pm-integration` | Verify #3609 property_type i18n helper keeps API filter contract on stable enum codes across sk/cs/de/en; localize labels only in display fields | pm-qa | open | — |
| `code-review-ppt-web-core-logout-purge-disputes-leases-iot-retry1` | medium | `pm-security` | ppt web core logout purge disputes leases iot [retry 1/2 of failed code-review-ppt-web-core-logout-purge-disputes-leases-iot] | None | in-progress | — |
| `code-review-api-core-digest-worker-no-replica-claim` | low | `pm-backend` | api core digest worker no replica claim | None | in-progress | — |
| `code-review-api-handlers-summarize-discards-persist-err` | low | `pm-backend` | api handlers summarize discards persist err | None | in-progress | — |
| `code-review-api-handlers-tenant-config-rls-clear-swallowed` | low | `pm-backend` | api handlers tenant config rls clear swallowed | None | in-progress | — |
| `code-review-mobile-rn-list-search-placeholders-i18n` | low | `pm-backend` | mobile rn list search placeholders i18n | None | in-progress | — |
| `code-review-ppt-web-ui-bookfacility-silent-availability` | low | `pm-backend` | ppt web ui bookfacility silent availability | None | in-progress | — |
| `pm-scrum-master-3630-widgetsettings-i18n-merge-2026-11-19` | medium | `pm-scrum-master` | Land #3630 (mobile WidgetSettings i18n) after CI; remaining no-hardcoded-strings warnings must drain before 2026-12-01 lint flip | pm-frontend | done | — |
| `code-review-api-core-docgen-empty-llm-blank-doc` | low | `pm-backend` | api core docgen empty llm blank doc | None | done | — |
