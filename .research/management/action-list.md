# Action list

<sub>Generated: 2026-11-19T03:14:48Z — delivery actions tracked by the research routine (Phase 1.6 PM analysis).</sub>

| Priority | Owner | Action | Dependency | Status | Source |
|----------|-------|--------|------------|--------|--------|
| high | pm-devops | Unblock mobile-native/KMP builds in the cloud runner (issue #2652) — still 5+ open backlog items structurally unclaimable in cloud; now compounded by #2966 so claimable=9/72 (buffer-low) | none | open | pm-analysis 2026-08-31 (carried) |
| high | pm-integration | Confirm #3616 ListingSummary slug reaches OpenAPI/TypeSpec and @ppt/reality-api-client + KMP/iOS clients; document slug as always-populated and state detail-lookup identifier | pm-qa | open | pm-analysis 2026-11-19 |
| high | pm-scrum-master | Sweep remaining violations mutation handlers for the 403-vs-404 cross-tenant contract (#3187 revealed assign still returns 403) — pattern audit across moderation.rs/aml_dsa.rs after PR #3183 fixed most but not assign | pm-security | open | pm-analysis 2026-10-28 |
| high | pm-scrum-master | Unblock mobile-native/KMP and swagger-ui egress in the cloud runner (#2652, #2966). Claimable work is a small share of the backlog, and backend security PRs cannot compile. | none | open | pm-analysis 2026-11-16 |
| high | pm-scrum-master | Sequence listings.rs PRs: rebase #3617 (record_view existence + per-IP throttle) and #3628 (suggestions region seed); merge #3617 first (security-flavored) | pm-backend | open | pm-analysis 2026-11-19 |
| high | pm-scrum-master | Decide fate of #3612 (paginate agency import history): #3610 fixed total; confirm if #3612 still adds page/offset or close as superseded | pm-backend | open | pm-analysis 2026-11-19 |
| medium | pm-backend | KMP realtor CreateListingScreen onSubmit is a NotImplementedError stub — form data discarded | none | open | dispatcher-backlog-refill 2026-10-21T22:25:48Z (score=3 conf=high vector=bug) |
| medium | pm-backend | mobile-native-kmp InquiriesResponse required page_size mismatches reality-server `limit` — MissingFieldException on every real /inquiries + /realtors/inquiries call | none | open | dispatcher-backlog-refill 2026-10-21T22:25:48Z (score=3 conf=high vector=bug) |
| medium | pm-backend | ppt web core personmonths raw fetch noauth | none | open | dispatcher-signals-promote 2026-11-19T02:27:15Z (tier1d ppt-web-core score=3 vector=bug) |
| medium | pm-integration | Make compliance list routes echo the clamped limit (not raw param) and document max 1000; add test with limit=5000 | none | open | pm-analysis 2026-11-19 |
| medium | pm-integration | Verify sanitized voice_oauth_error_response bodies follow RFC 6749 §5.2 (error+error_description, correct status codes) on Alexa/Google account-linking path; contract test required | pm-security | open | pm-analysis 2026-11-19 |
| medium | pm-qa | QA audit: non-mobile-native backlog items with confidence=medium need a one-shot verification step before Phase 3 can promote them | none | open | pm-analysis 2026-11-09 |
| medium | pm-scrum-master | Fix env-var leak in tests/suites/airbnb_install_routes_tests.rs (set_var without cleanup) causing airbnb_oauth_routes_tests flake in parallel suite_1 | pm-qa | open | pm-analysis 2026-11-19 |
| medium | pm-scrum-master | Land #3630 (mobile WidgetSettings i18n) after CI; remaining no-hardcoded-strings warnings must drain before 2026-12-01 lint flip | pm-frontend | open | pm-analysis 2026-11-19 |
| low | pm-backend | Shared Ktor HttpClient has no HttpTimeout - mobile API calls can hang indefinitely | none | open | dispatcher-backlog-refill 2026-10-25T08:29:08Z (score=2 conf=medium vector=bug) |
| low | pm-backend | Reality mobile MyListings + portfolio analytics silently cap at 100 listings (no pagination) | none | open | dispatcher-backlog-refill 2026-10-25T08:29:08Z (score=1 conf=medium vector=bug) |
| low | pm-backend | KMP realtor SSO error-body parsing is unguarded — crashes on non-JSON response | none | open | dispatcher-backlog-refill 2026-11-04T04:24:45Z (score=2 conf=medium vector=bug) |
| low | pm-backend | reality web analytics no i18n | none | open | dispatcher-signals-promote 2026-11-10T02:25:23Z (tier1d reality-web score=1 vector=bug) |
| low | pm-backend | api core statement account iban hardcoded | none | open | dispatcher-signals-promote 2026-11-13T22:25:04Z (tier1d api-core score=1 vector=bug) |
| low | pm-backend | api core digest worker no replica claim | none | in-progress | dispatcher-signals-promote 2026-11-15T16:26:57Z (tier1d api-core score=2 vector=bug) |
| low | pm-backend | api handlers tenant config rls clear swallowed | none | in-progress | dispatcher-signals-promote 2026-11-15T16:26:57Z (tier1d api-handlers score=2 vector=bug) |
| low | pm-backend | reality server import history limit 100 trunc | none | open | dispatcher-signals-promote 2026-11-16T00:27:09Z (tier1d reality-server score=1 vector=bug) |
| low | pm-backend | reality-server record_view: comment claims existence check but handler does not verify + no per-IP throttle | none | open | dispatcher-backlog-refill 2026-11-16T18:25:46Z (score=1 conf=medium vector=bug) |
| low | pm-backend | reality server suggestions hardcoded sk region | none | open | dispatcher-signals-promote 2026-11-17T00:24:34Z (tier1d reality-server score=2 vector=bug) |
| low | pm-backend | mobile rn personmonths screen no i18n | none | open | dispatcher-signals-promote 2026-11-17T00:24:34Z (tier1d mobile-rn score=1 vector=bug) |
| low | pm-backend | mobile rn widget settings i18n leak | none | open | dispatcher-signals-promote 2026-11-17T00:24:34Z (tier1d mobile-rn score=1 vector=bug) |
| low | pm-backend | ppt web ui facilities forms hardcoded errors | none | open | dispatcher-signals-promote 2026-11-17T00:24:34Z (tier1d ppt-web-ui score=1 vector=bug) |
| low | pm-backend | reality server categories labels sk only | none | open | dispatcher-signals-promote 2026-11-17T00:24:34Z (tier1d reality-server score=1 vector=bug) |
| low | pm-backend | find_inactive_digest_candidates INNER JOIN silently excludes users without a refresh-tokens row | none | open | dispatcher-backlog-refill 2026-11-17T20:34:55Z (score=2 conf=high vector=bug) |
| low | pm-backend | [stale-resolved @ 2026-11-10T09:10:40Z: finding already fixed on dev (inquiry-type.ts helper + all locales present, landed c5d7ca5); implementer confirmed no diff needed, no PR] reality web inquiry type raw enum [retry 1/2 of failed code-review-reality-web-inquiry-type-raw-enum] | none | open | dispatcher-retry-remint 2026-11-17T20:34:55Z (retry_of=code-review-reality-web-inquiry-type-raw-enum reason=failed-no-pr cooldown_ok newest_failure=2026-11-10T09:08:44Z) |
| low | pm-backend | reality server import jobs limit unclamped [retry 1/2 of failed code-review-reality-server-import-jobs-limit-unclamped] | none | open | dispatcher-retry-remint 2026-11-17T20:34:55Z (retry_of=code-review-reality-server-import-jobs-limit-unclamped reason=failed-no-pr cooldown_ok newest_failure=2026-11-10T12:47:00Z) |
| low | pm-backend | mobile rn voting quorum divzero [retry 1/2 of failed code-review-mobile-rn-voting-quorum-divzero] | none | open | dispatcher-retry-remint 2026-11-17T20:34:55Z (retry_of=code-review-mobile-rn-voting-quorum-divzero reason=failed-no-pr cooldown_ok newest_failure=2026-11-10T16:41:17Z) |
| low | pm-backend | api core notif push channel noop | none | open | dispatcher-signals-promote 2026-11-18T00:25:54Z (tier1d api-core score=2 vector=bug) |
| low | pm-backend | api core vote reminder spam 5min | none | open | dispatcher-signals-promote 2026-11-18T00:25:54Z (tier1d api-core score=2 vector=bug) |
| low | pm-backend | reality server import fetch ssrf revalidation | none | open | dispatcher-signals-promote 2026-11-18T00:25:54Z (tier1d reality-server score=2 vector=bug) |
| low | pm-backend | api core idempotency commit swallow | none | open | dispatcher-signals-promote 2026-11-18T00:25:54Z (tier1d api-core score=2 vector=bug) |
| low | pm-backend | ppt web core init no exp check | none | open | dispatcher-signals-promote 2026-11-18T00:25:54Z (tier1d ppt-web-core score=2 vector=bug) |
| low | pm-backend | ppt web core no runtime token refresh | none | open | dispatcher-signals-promote 2026-11-18T00:25:54Z (tier1d ppt-web-core score=2 vector=bug) |
| low | pm-backend | ppt web core retry ignores retry after | none | open | dispatcher-signals-promote 2026-11-18T00:25:54Z (tier1d ppt-web-core score=1 vector=bug) |
| low | pm-backend | ppt web ui authcallback decodeuri throw | none | open | dispatcher-signals-promote 2026-11-18T00:25:54Z (tier1d ppt-web-ui score=1 vector=bug) |
| low | pm-backend | ppt web ui errorboundary raw message prod | none | open | dispatcher-signals-promote 2026-11-18T00:25:54Z (tier1d ppt-web-ui score=1 vector=bug) |
| low | pm-backend | ppt web ui categorypref hardcoded labels [retry 1/2 of failed code-review-ppt-web-ui-categorypref-hardcoded-labels] | none | open | dispatcher-retry-remint 2026-11-18T22:26:56Z (retry_of=code-review-ppt-web-ui-categorypref-hardcoded-labels reason=failed-no-pr cooldown_ok newest_failure=2026-11-11T14:35:25Z) |
| low | pm-backend | api core jwt no aud iss [retry 1/2 of failed code-review-api-core-jwt-no-aud-iss] | none | open | dispatcher-retry-remint 2026-11-18T22:26:56Z (retry_of=code-review-api-core-jwt-no-aud-iss reason=failed-no-pr cooldown_ok newest_failure=2026-11-11T20:44:06Z) |
| low | pm-backend | ppt web core financial dash no error state | none | open | dispatcher-signals-promote 2026-11-19T02:27:15Z (tier1d ppt-web-core score=2 vector=bug) |
| low | pm-backend | reality web crmconn create no error | none | open | dispatcher-signals-promote 2026-11-19T02:27:15Z (tier1d reality-web score=2 vector=bug) |
| low | pm-backend | reality web feedimport create no error | none | open | dispatcher-signals-promote 2026-11-19T02:27:15Z (tier1d reality-web score=2 vector=bug) |
| low | pm-backend | reality web listings no error state | none | open | dispatcher-signals-promote 2026-11-19T02:27:15Z (tier1d reality-web score=2 vector=bug) |
| low | pm-backend | reality web listings url nan query | none | open | dispatcher-signals-promote 2026-11-19T02:27:15Z (tier1d reality-web score=2 vector=bug) |
| low | pm-backend | ppt web core orgprovider unmemoized value | none | open | dispatcher-signals-promote 2026-11-19T02:27:15Z (tier1d ppt-web-core score=1 vector=bug) |
| low | pm-backend | reality web account routes i18n | none | open | dispatcher-signals-promote 2026-11-19T02:27:15Z (tier1d reality-web score=1 vector=bug) |
| low | pm-backend | reality web import delete toggle unhandled | none | open | dispatcher-signals-promote 2026-11-19T02:27:15Z (tier1d reality-web score=1 vector=bug) |
| low | pm-backend | reality web listing edit slovak i18n | none | open | dispatcher-signals-promote 2026-11-19T02:27:15Z (tier1d reality-web score=1 vector=bug) |
| low | pm-backend | ppt web core ws query invalidation dead | none | open | dispatcher-signals-promote 2026-11-19T02:27:15Z (tier1d ppt-web-core score=2 vector=bug) |
| low | pm-backend | reality web realtor account pages no i18n | none | open | dispatcher-signals-promote 2026-11-19T02:27:15Z (tier1d reality-web score=2 vector=bug) |
| low | pm-data | Backfill KPI definitions for shipped stories in sprint-status.yaml or the epic docs, and add an events/KPI column to the story template | none | open | pm-analysis 2026-11-17 |
| low | pm-devops | Stalled review: PR #3380 (reality-web analytics i18n) — 8d, no reviewDecision | none | open | dispatcher-backlog-refill 2026-11-18T22:26:56Z (score=1 conf=medium vector=dx) |
| low | pm-integration | Document compliance 400 behavior for negative limit/offset in TypeSpec/OpenAPI and decide: keep plain-text (StatusCode, String) or move to standard error envelope | pm-backend | open | pm-analysis 2026-11-19 |
| low | pm-integration | Verify #3609 property_type i18n helper keeps API filter contract on stable enum codes across sk/cs/de/en; localize labels only in display fields | pm-qa | open | pm-analysis 2026-11-19 |
| low | pm-qa | api core rental idor test gap | none | open | dispatcher-signals-promote 2026-11-18T00:25:54Z (tier1d api-core score=1 vector=test-gap) |
| low | pm-qa | Screen-map drift: PR #3595 preserve locale in reality-web auth navigation without docs/screens/reality update | none | done | dispatcher-backlog-refill 2026-11-18T22:26:56Z (score=2 conf=medium vector=test-gap) |
| low | pm-qa | Screen-map drift: PR #3606 wrap route groups in ProtectedRoute without docs/screens/ppt update | none | done | dispatcher-backlog-refill 2026-11-18T22:26:56Z (score=2 conf=medium vector=test-gap) |
| low | pm-scrum-master | Mark as done in action-list the items already merged this window: errorhandler-403-mislabel, currency-code-rangeerror, login-open-redirect-backslash. Check that the 17 merges are reflected in the backlog. | none | done | pm-analysis 2026-11-16 |
| low | pm-scrum-master | Review draft PRs #3582 (T33 quarantine tests), #3585 and #3586 (ppt-web locale churn plans), then mark ready or close | none | open | pm-analysis 2026-11-17 |
| low | pm-tech-lead | granular_notification.rs repo is a repeat churn hotspot (digest subsystem) | none | open | dispatcher-backlog-refill 2026-11-17T20:34:55Z (score=1 conf=high vector=refactor) |
| low | pm-tech-lead | notification_digest.rs is a repeat churn hotspot (digest subsystem) | none | open | dispatcher-backlog-refill 2026-11-17T20:34:55Z (score=1 conf=high vector=refactor) |
| low | pm-tech-lead | api core rental dead unscoped idor | none | open | dispatcher-signals-promote 2026-11-18T00:25:54Z (tier1d api-core score=2 vector=refactor) |
| low | pm-tech-lead | Churn hotspot: api-server routes/compliance.rs (compliance wave churn) | none | done | dispatcher-backlog-refill 2026-11-18T22:26:56Z (score=1 conf=high vector=refactor) |
