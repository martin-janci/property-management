# Action list — delivery backlog

<sub>Generated: 2026-11-21T03:06:10Z</sub>

| ID | Priority | Owner | Action | Dep | Status | Deadline |
|----|----------|-------|--------|-----|--------|----------|
| `pm-backend-sweep-push-voice-rls-before-verify-ordering-2026-11-21` | high | `pm-backend` | Sweep the new push/ (post-#3659) and voice_webhooks/ (post-#3685) child modules for the RLS-before-verify ordering bug: webhook signature verification must precede the RlsConnection extractor so every real Alexa/Google body does not return 401 | none | open | — |
| `pm-backend-unquarantine-release-248-tests-2026-11-21` | high | `pm-backend` | Un-quarantine the release-248 tests introduced by #3675 (tenant application/forms 500 fixes + feed delete); establish a quarantine-aging policy (every quarantined test has owner+deadline) | none | open | — |
| `pm-devops-unblock-mobile-native-cloud-builds` | high | `pm-devops` | Unblock mobile-native/KMP builds in the cloud runner (issue #2652) — still 5+ open backlog items structurally unclaimable in cloud; now compounded by #2966 so claimable=9/72 (buffer-low) | none | open | — |
| `pm-integration-slug-client-propagation` | high | `pm-integration` | Confirm #3616 ListingSummary slug reaches OpenAPI/TypeSpec and @ppt/reality-api-client + KMP/iOS clients; document slug as always-populated and state detail-lookup identifier | pm-qa | open | — |
| `pm-scrum-master-3612-supersede-check-2026-11-19` | high | `pm-scrum-master` | Decide fate of #3612 (paginate agency import history): #3610 fixed total; confirm if #3612 still adds page/offset or close as superseded | pm-backend | open | — |
| `pm-scrum-master-cloud-runner-kmp-swagger-ui-egress-unblock-2026-11-16` | high | `pm-scrum-master` | Unblock mobile-native/KMP and swagger-ui egress in the cloud runner (#2652, #2966). Claimable work is a small share of the backlog, and backend security PRs cannot compile. | none | open | — |
| `pm-scrum-master-listings-rs-merge-sequence-2026-11-19` | high | `pm-scrum-master` | Sequence listings.rs PRs: rebase #3617 (record_view existence + per-IP throttle) and #3628 (suggestions region seed); merge #3617 first (security-flavored) | pm-backend | open | — |
| `pm-scrum-master-promote-or-close-6-open-prs-2026-11-21` | high | `pm-scrum-master` | Promote-or-close the 6 open PRs (#3668 dependabot cargo group rebase, #3687/#3688 pm-integration drafts, #3676/#3677 mobile-rn i18n drafts, #3682 search-drainer) within 48h | none | open | — |
| `pm-scrum-master-resolve-dispatcher-stall-3357-2026-11-21` | high | `pm-scrum-master` | Resolve dispatcher stall #3357 — same blockers as last run; the 32-PR merge wave did not drain it (3 approved PRs blocked on non-mechanical conflicts) | none | open | — |
| `pm-scrum-master-sweep-postsplit-modules-rls-before-verify-2026-11-21` | high | `pm-scrum-master` | Sweep the new push/ and voice_webhooks/ child modules for the RLS-before-verify ordering bug that survived the split | none | open | — |
| `pm-scrum-master-unblock-kmp-and-swagger-ui-egress-in-the-cloud-runner-2652-2-2026-11-20` | high | `pm-scrum-master` | Unblock KMP and swagger-ui egress in the cloud runner (#2652, #2966) | none | open | — |
| `pm-scrum-master-unblock-kmp-swagger-ui-egress-2652-2966-2026-11-21` | high | `pm-scrum-master` | Unblock KMP + swagger-ui egress in the cloud runner (#2652, #2966) — claimable still ≈9/72 | none | open | — |
| `pm-scrum-master-violations-xtenant-404-contract-sweep-2026-10-28` | high | `pm-scrum-master` | Sweep remaining violations mutation handlers for the 403-vs-404 cross-tenant contract (#3187 revealed assign still returns 403) — pattern audit across moderation.rs/aml_dsa.rs after PR #3183 fixed most but not assign | pm-security | open | — |
| `code-review-mobile-native-kmp-create-listing-not-wired` | medium | `pm-backend` | KMP realtor CreateListingScreen onSubmit is a NotImplementedError stub — form data discarded | none | open | — |
| `code-review-mobile-native-kmp-inquiries-response-contract` | medium | `pm-backend` | mobile-native-kmp InquiriesResponse required page_size mismatches reality-server `limit` — MissingFieldException on every real /inquiries + /realtors/inquiries call | none | open | — |
| `pm-backend-address-mobile-native-stubs-2026-11-21` | medium | `pm-backend` | Address mobile-native stubs: CreateListingScreen onSubmit NotImplementedError + InquiriesResponse page_size/limit contract mismatch + Ktor HttpTimeout default. Package as one KMP PR per issue once #2652 egress unblocks | none | open | — |
| `pm-backend-finish-accounting-mvp-trio-2558-2026-11-21` | medium | `pm-backend` | Finish the accounting MVP trio: verify #2558 state after #3661 landed sent/cancel lifecycle + PAY-by-square QR; close as superseded or sequence the remaining slice | none | open | — |
| `pm-backend-regression-coverage-3669-reality-surface-2026-11-21` | medium | `pm-backend` | Add regression coverage for #3669 reality-server portal auth fix + buyer inquiry threads + notification settings + agency dashboard so the broad surface does not regress on the next push_fanout touch | none | open | — |
| `pm-integration-voice-oauth-rfc6749-shape` | medium | `pm-integration` | Verify sanitized voice_oauth_error_response bodies follow RFC 6749 §5.2 (error+error_description, correct status codes) on Alexa/Google account-linking path; contract test required | pm-security | open | — |
| `pm-qa-confirm-non-kmp-medium-confidence-2026-11-09` | medium | `pm-qa` | QA audit: non-mobile-native backlog items with confidence=medium need a one-shot verification step before Phase 3 can promote them | none | open | — |
| `pm-scrum-master-3630-widgetsettings-i18n-merge-2026-11-19` | medium | `pm-scrum-master` | Land #3630 (mobile WidgetSettings i18n) after CI; remaining no-hardcoded-strings warnings must drain before 2026-12-01 lint flip | pm-frontend | open | — |
| `pm-scrum-master-airbnb-env-leak-flake-2026-11-19` | medium | `pm-scrum-master` | Fix env-var leak in tests/suites/airbnb_install_routes_tests.rs (set_var without cleanup) causing airbnb_oauth_routes_tests flake in parallel suite_1 | pm-qa | open | — |
| `pm-scrum-master-reset-sprint-status-yaml-hardening-track-2026-11-21` | medium | `pm-scrum-master` | Reset sprint-status.yaml — current sprint goal is fully 'done'; next sprint goal should name the hardening/refactor track explicitly (new modules under watch, release-248 quarantine burn-down, mobile-native backlog) | none | open | — |
| `code-review-api-core-digest-worker-no-replica-claim` | low | `pm-backend` | api core digest worker no replica claim | none | in-progress | — |
| `code-review-api-core-docgen-empty-llm-blank-doc` | low | `pm-backend` | api core docgen empty llm blank doc | none | open | — |
| `code-review-api-core-workflow-notin-fail-open` | low | `pm-backend` | api core workflow notin fail open | none | open | — |
| `code-review-api-handlers-summarize-discards-persist-err` | low | `pm-backend` | api handlers summarize discards persist err | none | open | — |
| `code-review-api-handlers-tenant-config-rls-clear-swallowed` | low | `pm-backend` | api handlers tenant config rls clear swallowed | none | in-progress | — |
| `code-review-api-handlers-workflow-enable-silent-fail` | low | `pm-backend` | api handlers workflow enable silent fail | none | open | — |
| `code-review-mobile-native-kmp-http-client-no-timeout` | low | `pm-backend` | Shared Ktor HttpClient has no HttpTimeout - mobile API calls can hang indefinitely | none | open | — |
| `code-review-mobile-native-kmp-my-listings-pagination-cap` | low | `pm-backend` | Reality mobile MyListings + portfolio analytics silently cap at 100 listings (no pagination) | none | open | — |
| `code-review-mobile-native-kmp-sso-error-body-unguarded` | low | `pm-backend` | KMP realtor SSO error-body parsing is unguarded — crashes on non-JSON response | none | open | — |
| `code-review-mobile-rn-hardcoded-button-labels` | low | `pm-backend` | mobile rn hardcoded button labels | none | open | — |
| `code-review-mobile-rn-list-search-placeholders-i18n` | low | `pm-backend` | mobile rn list search placeholders i18n | none | open | — |
| `code-review-mobile-rn-raw-status-untranslated` | low | `pm-backend` | mobile rn raw status untranslated | none | open | — |
| `code-review-ppt-web-core-ws-notifications-token-in-url-retry1` | low | `pm-backend` | ppt web core ws notifications token in url [retry 1/2 of failed code-review-ppt-web-core-ws-notifications-token-in-url] | none | open | — |
| `code-review-ppt-web-ui-bookfacility-silent-availability` | low | `pm-backend` | ppt web ui bookfacility silent availability | none | open | — |
| `code-review-ppt-web-ui-date-number-formatters-no-locale` | low | `pm-backend` | ppt web ui date number formatters no locale | none | open | — |
| `code-review-ppt-web-ui-document-hooks-hardcoded-sk` | low | `pm-backend` | ppt web ui document hooks hardcoded sk | none | open | — |
| `code-review-ppt-web-ui-facilities-silent-load-errors` | low | `pm-backend` | ppt web ui facilities silent load errors | none | open | — |
| `code-review-ppt-web-ui-invoice-empty-items` | low | `pm-backend` | ppt web ui invoice empty items | none | open | — |
| `code-review-ppt-web-ui-invoice-vat-table-drift` | low | `pm-backend` | ppt web ui invoice vat table drift | none | open | — |
| `code-review-ppt-web-ui-mediation-sessions-swallowed-error` | low | `pm-backend` | ppt web ui mediation sessions swallowed error | none | open | — |
| `code-review-reality-server-drainer-mark-fail-redeliver` | low | `pm-backend` | reality server drainer mark fail redeliver | none | open | — |
| `code-review-reality-server-sso-http-no-timeout` | low | `pm-backend` | reality server sso http no timeout | none | open | — |
| `code-review-reality-web-agency-slug-i18n` | low | `pm-backend` | reality web agency slug i18n | none | open | — |
| `code-review-reality-web-checksession-5xx-logout` | low | `pm-backend` | reality web checksession 5xx logout | none | open | — |
| `code-review-reality-web-compare-button-i18n` | low | `pm-backend` | reality web compare button i18n | none | open | — |
| `code-review-reality-web-comparison-tray-i18n` | low | `pm-backend` | reality web comparison tray i18n | none | open | — |
| `code-review-reality-web-listing-grid-empty-i18n` | low | `pm-backend` | reality web listing grid empty i18n | none | open | — |
| `code-review-reality-web-report-page-hardcoded-sk-strings-retry1` | low | `pm-backend` | reality web report page hardcoded sk strings [retry 1/2 of failed code-review-reality-web-report-page-hardcoded-sk-strings] | none | open | — |
| `code-review-search-drainer-stub-marks-delivered` | low | `pm-backend` | search drainer stub marks delivered | none | open | — |
| `pm-backend-characterize-postsplit-modules-under-watch-2026-11-21` | low | `pm-backend` | Characterize the post-split push/ and voice_webhooks/ child modules in sprint-status and coverage; the new file footprint should move to a 'done, under watch' bucket for the next 2 runs so regressions surface | none | open | — |
| `pm-integration-compliance-400-spec` | low | `pm-integration` | Document compliance 400 behavior for negative limit/offset in TypeSpec/OpenAPI and decide: keep plain-text (StatusCode, String) or move to standard error envelope | pm-backend | open | — |
| `pm-integration-property-type-enum-stability` | low | `pm-integration` | Verify #3609 property_type i18n helper keeps API filter contract on stable enum codes across sk/cs/de/en; localize labels only in display fields | pm-qa | open | — |
| `repeated-churn-backend-servers-api-server-tests-suites-oauth-integration-tests-rs` | low | `pm-qa` | oauth_integration_tests.rs is a repeated-churn hotspot (3427 LoC, runs_seen→2) — split by flow | none | open | — |
| `code-review-mobile-rn-faults-screen-no-i18n` | low | `pm-tech-lead` | mobile rn faults screen no i18n | none | open | — |
| `code-review-mobile-rn-person-months-hardcoded-i18n` | low | `pm-tech-lead` | mobile rn person months hardcoded i18n | none | open | — |
| `code-review-mobile-rn-widget-settings-alerts-i18n` | low | `pm-tech-lead` | mobile rn widget settings alerts i18n | none | open | — |
| `code-review-ppt-web-core-create-lease-empty-pickers-retry1` | low | `pm-tech-lead` | ppt web core create lease empty pickers [retry 1/2 of failed code-review-ppt-web-core-create-lease-empty-pickers] | none | open | — |
| `code-review-ppt-web-ui-facilities-hardcoded-i18n` | low | `pm-tech-lead` | ppt web ui facilities hardcoded i18n | none | open | — |
| `code-review-ppt-web-ui-native-confirm-dialogs` | low | `pm-tech-lead` | ppt web ui native confirm dialogs | none | open | — |
| `code-review-reality-web-report-attachments-disabled-3017-retry1` | low | `pm-tech-lead` | reality web report attachments disabled 3017 [retry 1/2 of failed code-review-reality-web-report-attachments-disabled-3017] | none | open | — |
| `repeated-churn-backend-servers-api-server-src-routes-voice-webhooks-rs` | low | `pm-tech-lead` | voice_webhooks.rs is a 5-run repeated-churn hotspot (3066 LoC, 5 runs_seen) — module boundaries needed | none | open | — |
