# Action list — delivery backlog

<sub>Generated: 2026-11-20T03:06:10Z</sub>

| ID | Priority | Owner | Action | Dep | Status | Deadline |
|----|----------|-------|--------|-----|--------|----------|
| `pm-devops-unblock-mobile-native-cloud-builds` | high | `pm-devops` | Unblock mobile-native/KMP builds in the cloud runner (issue #2652) — still 5+ open backlog items structurally unclaimable in cloud; now compounded by #2966 so claimable=9/72 (buffer-low) | none | open | — |
| `pm-integration-slug-client-propagation` | high | `pm-integration` | Confirm #3616 ListingSummary slug reaches OpenAPI/TypeSpec and @ppt/reality-api-client + KMP/iOS clients; document slug as always-populated and state detail-lookup identifier | pm-qa | open | — |
| `pm-scrum-master-3612-supersede-check-2026-11-19` | high | `pm-scrum-master` | Decide fate of #3612 (paginate agency import history): #3610 fixed total; confirm if #3612 still adds page/offset or close as superseded | pm-backend | open | — |
| `pm-scrum-master-cloud-runner-kmp-swagger-ui-egress-unblock-2026-11-16` | high | `pm-scrum-master` | Unblock mobile-native/KMP and swagger-ui egress in the cloud runner (#2652, #2966). Claimable work is a small share of the backlog, and backend security PRs cannot compile. | none | open | — |
| `pm-scrum-master-listings-rs-merge-sequence-2026-11-19` | high | `pm-scrum-master` | Sequence listings.rs PRs: rebase #3617 (record_view existence + per-IP throttle) and #3628 (suggestions region seed); merge #3617 first (security-flavored) | pm-backend | open | — |
| `pm-scrum-master-resolve-dispatcher-stall-3357-rebase-or-manually-resolve-the-2026-11-20` | high | `pm-scrum-master` | Resolve dispatcher stall #3357: rebase or manually resolve the 3 approved conflicting PRs, merging security-flavored ones first | none | open | — |
| `pm-scrum-master-sequence-listings-rs-prs-rebase-3617-record-view-existence-p-2026-11-20` | high | `pm-scrum-master` | Sequence listings.rs PRs: rebase #3617 (record_view existence + per-IP throttle) and merge before #3628 follow-up; fix #3649 so region de does not seed Wien | none | open | — |
| `pm-scrum-master-sweep-violations-mutation-handlers-moderation-rs-aml-dsa-rs-2026-11-20` | high | `pm-scrum-master` | Sweep violations mutation handlers (moderation.rs/aml_dsa.rs) for the 403-vs-404 cross-tenant contract (assign still returns 403 per #3187) | none | open | — |
| `pm-scrum-master-unblock-kmp-and-swagger-ui-egress-in-the-cloud-runner-2652-2-2026-11-20` | high | `pm-scrum-master` | Unblock KMP and swagger-ui egress in the cloud runner (#2652, #2966) | none | open | — |
| `pm-scrum-master-violations-xtenant-404-contract-sweep-2026-10-28` | high | `pm-scrum-master` | Sweep remaining violations mutation handlers for the 403-vs-404 cross-tenant contract (#3187 revealed assign still returns 403) — pattern audit across moderation.rs/aml_dsa.rs after PR #3183 fixed most but not assign | pm-security | open | — |
| `pm-tech-lead-resolve-issue-3357-stalled-dispatcher-decide-which-approved-2026-11-20` | high | `pm-tech-lead` | Resolve issue #3357 (stalled dispatcher): decide which approved PRs to rebase manually, and define an escalation path for non-mechanical conflicts that replaces the dead-end Tier-1d signal. | none | open | — |
| `pm-tech-lead-split-routes-voice-webhooks-rs-3066-loc-churned-in-5-runs-in-2026-11-20` | high | `pm-tech-lead` | Split routes/voice_webhooks.rs (3066 LoC, churned in 5 runs) into signature verification, an idempotent event handler and provider-specific parsers. Move business logic into services/. | none | open | — |
| `pm-tech-lead-split-services-push-fanout-rs-3038-loc-into-a-push-module-fc-2026-11-20` | high | `pm-tech-lead` | Split services/push_fanout.rs (3038 LoC) into a push/ module: fcm.rs, apns.rs, combined adapter, worker, config. Keep the PushTransport trait as the seam. | none | open | — |
| `code-review-api-handlers-hellosign-unbound-envelope` | medium | `pm-backend` | api handlers hellosign unbound envelope | none | open | — |
| `code-review-mobile-native-kmp-create-listing-not-wired` | medium | `pm-backend` | KMP realtor CreateListingScreen onSubmit is a NotImplementedError stub — form data discarded | none | open | — |
| `code-review-mobile-native-kmp-inquiries-response-contract` | medium | `pm-backend` | mobile-native-kmp InquiriesResponse required page_size mismatches reality-server `limit` — MissingFieldException on every real /inquiries + /realtors/inquiries call | none | open | — |
| `gh-issue-3649` | medium | `pm-tech-lead` | Follow-up: suggestions seed-city for region `de` resolves to Wien (Austria) (PR #3628) (Closes #3649) | none | open | — |
| `pm-integration-voice-oauth-rfc6749-shape` | medium | `pm-integration` | Verify sanitized voice_oauth_error_response bodies follow RFC 6749 §5.2 (error+error_description, correct status codes) on Alexa/Google account-linking path; contract test required | pm-security | open | — |
| `pm-qa-confirm-non-kmp-medium-confidence-2026-11-09` | medium | `pm-qa` | QA audit: non-mobile-native backlog items with confidence=medium need a one-shot verification step before Phase 3 can promote them | none | open | — |
| `pm-scrum-master-3630-widgetsettings-i18n-merge-2026-11-19` | medium | `pm-scrum-master` | Land #3630 (mobile WidgetSettings i18n) after CI; remaining no-hardcoded-strings warnings must drain before 2026-12-01 lint flip | pm-frontend | open | — |
| `pm-scrum-master-airbnb-env-leak-flake-2026-11-19` | medium | `pm-scrum-master` | Fix env-var leak in tests/suites/airbnb_install_routes_tests.rs (set_var without cleanup) causing airbnb_oauth_routes_tests flake in parallel suite_1 | pm-qa | open | — |
| `pm-scrum-master-reconcile-sprint-status-test-hardening-batch-items-480-487-s-2026-11-20` | medium | `pm-scrum-master` | Reconcile sprint-status test_hardening_batch (items #480-#487 still marked open, batch in-progress) against GitHub issue state and close or defer | none | open | — |
| `pm-scrum-master-triage-churn-hotspots-oauth-integration-tests-rs-voice-webho-2026-11-20` | medium | `pm-scrum-master` | Triage churn hotspots (oauth_integration_tests.rs, voice_webhooks.rs runs_seen=5, push_fanout.rs runs_seen=4) into refactor or characterization backlog items; carry on #3650 (retry-after draft) and reality-web preview i18n items | none | open | — |
| `pm-tech-lead-create-a-new-sprint-in-sprint-status-yaml-the-current-sprint-2026-11-20` | medium | `pm-tech-lead` | Create a new sprint in sprint-status.yaml. The current sprint goal is all 'done' and no longer reflects the work being merged (review follow-ups, churn characterization). | none | open | — |
| `pm-tech-lead-define-an-nfr-baseline-for-push-and-voice-webhooks-metrics-r-2026-11-20` | medium | `pm-tech-lead` | Define an NFR baseline for push and voice webhooks: metrics, retry/dead-letter policy, and FCM/APNs token-refresh failure alerting. Check the observability stack for these. | none | open | — |
| `pm-tech-lead-split-oauth-integration-tests-rs-3427-loc-by-flow-authorize-2026-11-20` | medium | `pm-tech-lead` | Split oauth_integration_tests.rs (3427 LoC) by flow: authorize, token, introspect/revoke, grants. Extract shared fixtures into a common module. | none | open | — |
| `code-review-api-core-jwt-no-aud-iss-retry1` | low | `pm-backend` | api core jwt no aud iss [retry 1/2 of failed code-review-api-core-jwt-no-aud-iss] | none | open | — |
| `code-review-api-core-workflow-notin-fail-open` | low | `pm-backend` | api core workflow notin fail open | none | open | — |
| `code-review-api-core-workflow-regex-recompile` | low | `pm-backend` | api core workflow regex recompile | none | open | — |
| `code-review-api-handlers-esign-webhook-replay` | low | `pm-backend` | api handlers esign webhook replay | none | open | — |
| `code-review-mobile-native-kmp-http-client-no-timeout` | low | `pm-backend` | Shared Ktor HttpClient has no HttpTimeout - mobile API calls can hang indefinitely | none | open | — |
| `code-review-mobile-native-kmp-my-listings-pagination-cap` | low | `pm-backend` | Reality mobile MyListings + portfolio analytics silently cap at 100 listings (no pagination) | none | open | — |
| `code-review-mobile-native-kmp-sso-error-body-unguarded` | low | `pm-backend` | KMP realtor SSO error-body parsing is unguarded — crashes on non-JSON response | none | open | — |
| `code-review-mobile-rn-faultslist-i18n-gap` | low | `pm-backend` | mobile rn faultslist i18n gap | none | open | — |
| `code-review-mobile-rn-hardcoded-button-labels` | low | `pm-backend` | mobile rn hardcoded button labels | none | open | — |
| `code-review-mobile-rn-raw-status-untranslated` | low | `pm-backend` | mobile rn raw status untranslated | none | open | — |
| `code-review-ppt-web-core-init-no-exp-check` | low | `pm-backend` | ppt web core init no exp check | none | open | — |
| `code-review-ppt-web-core-no-runtime-token-refresh` | low | `pm-backend` | ppt web core no runtime token refresh | none | open | — |
| `code-review-ppt-web-core-orgprovider-unmemoized-value` | low | `pm-backend` | ppt web core orgprovider unmemoized value | none | open | — |
| `code-review-ppt-web-core-retry-ignores-retry-after` | low | `pm-backend` | ppt web core retry ignores retry after | none | open | — |
| `code-review-ppt-web-core-ws-query-invalidation-dead` | low | `pm-backend` | ppt web core ws query invalidation dead | none | open | — |
| `code-review-ppt-web-ui-authcallback-decodeuri-throw` | low | `pm-backend` | ppt web ui authcallback decodeuri throw | none | open | — |
| `code-review-ppt-web-ui-ballotquestion-yesno-i18n-retry1` | low | `pm-tech-lead` | ppt web ui ballotquestion yesno i18n [retry 1/2 of failed code-review-ppt-web-ui-ballotquestion-yesno-i18n] | none | open | — |
| `code-review-ppt-web-ui-filedisputedialog-i18n-retry1` | low | `pm-tech-lead` | ppt web ui filedisputedialog i18n [retry 1/2 of failed code-review-ppt-web-ui-filedisputedialog-i18n] | none | open | — |
| `code-review-ppt-web-ui-marketplace-epic68-orphaned-routes-retry1` | low | `pm-backend` | ppt web ui marketplace epic68 orphaned routes [retry 1/2 of failed code-review-ppt-web-ui-marketplace-epic68-orphaned-routes] | none | open | — |
| `code-review-ppt-web-ui-rfqform-i18n-retry1` | low | `pm-tech-lead` | [already-resolved-on-dev, no change needed] ppt web ui rfqform i18n [retry 1/2 of failed code-review-ppt-web-ui-rfqform-i18n] | none | open | — |
| `code-review-reality-server-import-fetch-ssrf-revalidation` | low | `pm-backend` | reality server import fetch ssrf revalidation | none | open | — |
| `code-review-reality-web-account-routes-i18n` | low | `pm-backend` | reality web account routes i18n | none | open | — |
| `code-review-reality-web-feedimport-create-no-error` | low | `pm-backend` | reality web feedimport create no error | none | open | — |
| `code-review-reality-web-import-delete-toggle-unhandled` | low | `pm-backend` | reality web import delete toggle unhandled | none | open | — |
| `code-review-reality-web-inquiry-type-raw-enum-retry1` | low | `pm-backend` | [stale-resolved @ 2026-11-10T09:10:40Z: finding already fixed on dev (inquiry-type.ts helper + all locales present, landed c5d7ca5); implementer confirmed no diff needed, no PR] reality web inquiry type raw enum [retry 1/2 of failed code-review-reality-web-inquiry-type-raw-enum] | none | open | — |
| `code-review-reality-web-listing-edit-slovak-i18n` | low | `pm-backend` | reality web listing edit slovak i18n | none | open | — |
| `code-review-reality-web-listings-url-nan-query` | low | `pm-backend` | reality web listings url nan query | none | open | — |
| `code-review-reality-web-preview-hardcoded-alt-text` | low | `pm-backend` | reality-web agency branding preview uses hardcoded English alt text (Preview, Logo) | none | open | — |
| `code-review-reality-web-preview-hardcoded-price` | low | `pm-backend` | reality-web agency branding preview shows hardcoded `€450,000` — breaks for de/sk/cs locales | none | open | — |
| `code-review-reality-web-realtor-account-pages-no-i18n` | low | `pm-backend` | reality web realtor account pages no i18n | none | open | — |
| `code-review-reality-web-realtor-api-missing-bearer-header` | low | `pm-backend` | reality web realtor api missing bearer header | none | open | — |
| `code-review-reality-web-shared-state-primitives-no-i18n` | low | `pm-backend` | reality web shared state primitives no i18n | none | open | — |
| `code-review-search-drainer-alert-copy-no-i18n` | low | `pm-backend` | search drainer alert copy no i18n | none | open | — |
| `code-review-search-drainer-stub-marks-delivered` | low | `pm-backend` | search drainer stub marks delivered | none | open | — |
| `pm-integration-compliance-400-spec` | low | `pm-integration` | Document compliance 400 behavior for negative limit/offset in TypeSpec/OpenAPI and decide: keep plain-text (StatusCode, String) or move to standard error envelope | pm-backend | open | — |
| `pm-integration-property-type-enum-stability` | low | `pm-integration` | Verify #3609 property_type i18n helper keeps API filter contract on stable enum codes across sk/cs/de/en; localize labels only in display fields | pm-qa | open | — |
| `pm-scrum-master-review-draft-prs-3582-3585-3586-2026-11-17` | low | `pm-scrum-master` | Review draft PRs #3582 (T33 quarantine tests), #3585 and #3586 (ppt-web locale churn plans), then mark ready or close | none | open | — |
| `repeated-churn-backend-servers-reality-server-src-routes-listings-rs` | low | `pm-tech-lead` | reality-server /listings routes/listings.rs hit as repeated churn hotspot (runs_seen=2) | none | open | — |
| `stalled-review-pr-3380` | low | `pm-devops` | Stalled review: PR #3380 (reality-web analytics i18n) — 8d, no reviewDecision | none | open | — |
| `code-review-api-core-digest-worker-no-replica-claim` | low | `pm-backend` | api core digest worker no replica claim | none | in-progress | — |
| `code-review-api-handlers-tenant-config-rls-clear-swallowed` | low | `pm-backend` | api handlers tenant config rls clear swallowed | none | in-progress | — |
