# Action list

_Generated: 2026-10-07T12:40:00Z — regenerated from `action-list.json` (routine Phase 1.6, pm-data rotation)._

33 open items. This run marked `pm-devops-unblock-api-server-utoipa-swagger-egress` (PR #3026) and `pm-devops-unblock-mobile-rn-jest-expo-version-rot` (PR #3025) **done**. New: `pm-data-announcements-engagement-events-doc`, `pm-scrum-master-triage-3029-syncschedule-stale-banner`.

| ID | Priority | Owner | Action |
|---|---|---|---|
| `gh-issue-2946-retry2` | high | pm-security | [CLOUD-BUILD-BLOCKED: backend api-server unbuildable in cloud runner (swagger-ui egress 403, #2966); tracked by open GitHub issue] Cross-tenant IDOR (read+write |
| `gh-issue-2944-retry1` | high | pm-security | [CLOUD-BUILD-BLOCKED: backend api-server unbuildable in cloud runner (swagger-ui egress 403, #2966); tracked by open GitHub issue] Cross-tenant IDOR: violation  |
| `gh-issue-2945-retry1` | high | pm-security | [CLOUD-BUILD-BLOCKED: backend api-server unbuildable in cloud runner (swagger-ui egress 403, #2966); tracked by open GitHub issue] Cross-tenant IDOR (read+write |
| `pm-devops-unblock-mobile-native-cloud-builds` | high | pm-devops | Unblock mobile-native/KMP builds in the cloud runner (issue #2652) — still 5+ open backlog items structurally unclaimable in cloud; now compounded by #2966 so c |
| `pm-data-announcements-engagement-events-doc` | high | pm-data | Write docs/data/announcements-engagement-events.md — define events for announcement published/read/acknowledged/comment-created/pin/unpin with trigger, properti |
| `pm-scrum-master-triage-3029-syncschedule-stale-banner` | high | pm-qa | Backlog triage of issue #3029 (SyncSchedule stale error banner, PR #2967 regression) — item now exists at code-review-reality-web-syncschedule-stale-error-banne |
| `code-review-mobile-native-kmp-httpclient-no-timeout` | low | pm-backend | mobile-native-kmp shared Ktor HttpClient installs no HttpTimeout — every suspend API call can hang indefinitely on Android + iOS |
| `code-review-mobile-native-kmp-cancellation-swallowed` | low | pm-backend | mobile-native-kmp: shared repositories swallow CancellationException in catch(e: Exception), breaking coroutine cancellation and showing spurious errors |
| `code-review-mobile-native-kmp-portfolio-analytics-caps-100` | low | pm-backend | mobile-native-kmp: getPortfolioAnalytics() truncates realtor portfolio at 100 listings — dashboard under-reports on large portfolios |
| `code-review-mobile-native-kmp-portfolio-analytics-unbounded-fanout-retry1` | low | pm-backend | mobile-native-kmp: getPortfolioAnalytics() fans out one analytics HTTP request per listing with no concurrency limit — up to 100 parallel GETs from a mobile dev |
| `code-review-mobile-native-kmp-ssoservice-untested` | low | pm-qa | mobile-native-kmp: SsoService (deep-link token exchange, login, password reset, session restore) has zero direct tests |
| `code-review-mobile-native-kmp-create-listing-not-wired` | low | pm-backend | KMP realtor CreateListingScreen onSubmit is a NotImplementedError stub — form data discarded |
| `code-review-mobile-rn-meter-reading-no-numeric-validation` | low | pm-backend | Mobile RN meter-reading input accepts non-numeric — silent write of garbage values |
| `code-review-reality-server-dup-url-validator` | low | pm-tech-lead | reality-server carries a duplicated URL-validation helper — merge into shared crate |
| `code-review-ppt-web-ui-workflow-automation-toasts-hardcoded-english` | low | pm-tech-lead | workflow-automation pages emit English-literal toast titles while rest of file uses i18n |
| `screen-map-drift-pr-2994-ppt` | low | pm-qa | ppt-web route refactor (PR #2994) landed without touching docs/screens/ppt |
| `churn-hotspot-frontend/apps/ppt-web/src/features/workflow-automation/pages/CreateRulePage.tsx` | low | pm-tech-lead | ppt-web workflow-automation CreateRulePage.tsx — 4 commits in 14d (new hotspot) |
| `churn-hotspot-frontend/apps/ppt-web/src/features/workflow-automation/pages/TemplateLibraryPage.tsx` | low | pm-tech-lead | ppt-web workflow-automation TemplateLibraryPage.tsx — 3 commits in 14d (new hotspot) |
| `churn-hotspot-frontend/apps/ppt-web/src/routes/groups/core.tsx` | low | pm-tech-lead | ppt-web src/routes/groups/core.tsx — 4 commits in last 14d (new hotspot) |
| `code-review-reality-web-profile-mock-data` | low | pm-backend | reality-web /profile route renders MOCK_* fixtures — every signed-in user sees the same fabricated identity, listings, activity, reviews |
| `code-review-reality-server-create-handlers-return-200-not-201` | low | pm-backend | reality-server create handlers return 200 OK but utoipa spec claims 201 Created — OpenAPI contract vs runtime divergence across 9 endpoints |
| `screen-map-drift-pr-3013-reality` | low | pm-qa | PR #3013 touched reality-web /sell route without updating docs/screens/reality/sell.md |
| `screen-map-drift-pr-3014-reality` | low | pm-qa | PR #3014 touched reality-web /report route without updating docs/screens/reality/report-listing.md |
| `churn-hotspot-frontend/apps/reality-web/src/app/[locale]/report/page.tsx` | low | pm-tech-lead | reality-web /report page recent churn — PR #3014 submit-wiring change |
| `churn-hotspot-frontend/apps/reality-web/src/app/[locale]/sell/page.tsx` | low | pm-tech-lead | reality-web /sell page recent churn — PR #3013 publish-wiring change |
| `churn-hotspot-frontend/apps/reality-web/src/components/home/FeaturedListings.tsx` | low | pm-tech-lead | reality-web FeaturedListings recent churn — PR #3015 error-surface fix |
| `code-review-reality-server-add-to-compare-toctou-max-cap` | low | pm-backend | reality-server add_to_compare TOCTOU on MAX_COMPARE_LISTINGS — two concurrent POSTs can exceed the 4-item cap |
| `code-review-ppt-web-core-community-group-detail-mock-data` | low | pm-backend | ppt-web /community/groups/:groupId renders hardcoded mock group — every group shows fake "Sample Group" with no-op actions |
| `code-review-ppt-web-core-route-group-toast-i18n` | low | pm-backend | ppt-web route-group action toasts use hardcoded English — bypass react-i18next, render English on sk/cs/de locales |
| `pm-scrum-master-stalled-accounting-pay-by-square-2559` | medium | pm-tech-lead | Stalled 64d: PR #2559 (PAY by Square accounting slice) needs reviewer slot — one of two non-dependabot >60d PRs; part of the accounting MVP trio that has been r |
| `pm-scrum-master-stalled-invoice-lifecycle-2555` | medium | pm-tech-lead | Stalled 64d: PR #2555 (invoice lifecycle accounting slice) needs reviewer slot — companion to #2559; both are the aging accounting MVP trio tracked since 2026-0 |
| `code-review-ppt-web-core-authed-roots-session-leak` | medium | pm-backend | logout() AUTHED_QUERY_KEY_ROOTS allow-list omits mounted route groups; caches leak across users on shared workstation |
| `code-review-mobile-native-kmp-inquiries-response-contract` | medium | pm-backend | mobile-native-kmp InquiriesResponse required page_size mismatches reality-server `limit` — MissingFieldException on every real /inquiries + /realtors/inquiries  |
