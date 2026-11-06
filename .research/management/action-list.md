# PPT Action List

_Generated: 2026-11-06T00:30:00Z (routine Phase 1.6)_

**Open: 30** · Total: 38

| ID | Owner | Priority | Status | Action |
|---|---|---|---|---|
| `pm-devops-unblock-mobile-native-cloud-builds` | pm-devops | high | open | Unblock mobile-native/KMP builds in the cloud runner (issue #2652) — still 5+ open backlog items structurally unclaimable in cloud… |
| `code-review-mobile-native-kmp-create-listing-not-wired` | pm-backend | medium | open | KMP realtor CreateListingScreen onSubmit is a NotImplementedError stub — form data discarded |
| `code-review-mobile-native-kmp-inquiries-response-contract` | pm-backend | medium | open | mobile-native-kmp InquiriesResponse required page_size mismatches reality-server `limit` — MissingFieldException on every real /in… |
| `code-review-mobile-native-kmp-http-client-no-timeout` | pm-backend | low | open | Shared Ktor HttpClient has no HttpTimeout - mobile API calls can hang indefinitely |
| `code-review-mobile-native-kmp-my-listings-pagination-cap` | pm-backend | low | open | Reality mobile MyListings + portfolio analytics silently cap at 100 listings (no pagination) |
| `pm-frontend-mobile-rn-lint-gates-adopt-2026-10-28` | pm-frontend | medium | open | Adopt eslint-plugin-react-hooks + i18next/no-literal-string in frontend/apps/mobile (blocking, warn-only for one sprint) — closes … |
| `pm-scrum-master-violations-xtenant-404-contract-sweep-2026-10-28` | pm-scrum-master | high | open | Sweep remaining violations mutation handlers for the 403-vs-404 cross-tenant contract (#3187 revealed assign still returns 403) — … |
| `code-review-mobile-native-kmp-sso-error-body-unguarded` | pm-backend | low | open | KMP realtor SSO error-body parsing is unguarded — crashes on non-JSON response |
| `code-review-ppt-web-core-refresh-token-localstorage` | pm-security | low | open | SECURITY: ppt-web refresh-token persisted in localStorage — XSS-reachable |
| `code-review-reality-server-inquiry-dead-fk-classify` | pm-tech-lead | low | open | InquiriesHandler splits ListingNotFound vs RealtorNotFound by error substring, but both collapse to the same 404 at the route - de… |
| `screen-map-drift-pr-3237-ppt` | pm-qa | low | open | Screen-map drift: PR #3237 restructured ppt-web app shell + landing without docs/screens/ppt/*.md update |
| `screen-map-drift-pr-3242-ppt` | pm-qa | low | open | Screen-map drift: PR #3242 localized financial/buildings/my-unit routes without docs/screens/ppt update |
| `screen-map-drift-pr-3244-ppt` | pm-qa | low | open | Screen-map drift: PR #3244 localized faults/news/settings/error pages without docs/screens/ppt update |
| `screen-map-drift-pr-3254-ppt` | pm-qa | low | open | Screen-map drift: PR #3254 rewired ppt-web rentals routes to /bookings/* without docs/screens/ppt update |
| `screen-map-drift-pr-3293-reality` | pm-qa | low | open | Screen-map drift: PR #3293 i18n'd reality-web realtor/[id] page without updating docs/screens/reality/realtor-detail.md |
| `screen-map-drift-pr-3313-reality` | pm-qa | low | open | Screen-map drift: PR #3313 added regression tests for reality-web realtor/[id] without updating docs/screens/reality/realtor-detai… |
| `screen-map-drift-pr-3317-ppt` | pm-qa | low | open | Screen-map drift: PR #3317 biome-cleaned ppt-web ai-dashboards route test without updating docs/screens/ppt/*.md |
| `screen-map-drift-pr-3317-reality` | pm-qa | low | open | Screen-map drift: PR #3317 biome-cleaned reality-web favorites/report/layout-revalidate route files without updating docs/screens/… |
| `churn-hotspot-backend/servers/api-server/src/routes/suspend_cascade.rs` | pm-tech-lead | low | open | Churn hotspot (new): backend/servers/api-server/src/routes/suspend_cascade.rs is a top-3 churn file this run — watch for instabili… |
| `churn-hotspot-backend/servers/api-server/tests/suites/auth_tests.rs` | pm-tech-lead | low | open | Churn hotspot (new): backend/servers/api-server/tests/suites/auth_tests.rs is a top-3 churn file this run — watch for instability |
| `churn-hotspot-backend/servers/reality-server/src/util/errors.rs` | pm-tech-lead | low | open | Churn hotspot (new): backend/servers/reality-server/src/util/errors.rs is a top-3 churn file this run — watch for instability |
| `code-review-ppt-web-ui-lang-switcher-aria` | pm-tech-lead | low | open | ppt-web LanguageSwitcher has hardcoded English aria-label on the control a non-English screen-reader user would use to find their … |
| `code-review-ppt-web-ui-marketplace-no-i18n` | pm-tech-lead | low | open | ppt-web marketplace feature ships hardcoded English (placeholders, 16 service-category labels) in a 4-locale app — 0 useTranslatio… |
| `code-review-ppt-web-ui-registry-no-i18n` | pm-tech-lead | low | open | ppt-web registry feature (pet/vehicle forms + lists) ships hardcoded English placeholders in a 4-locale app — 0 useTranslation on … |
| `code-review-reality-web-share-cmp-unchecked-json` | pm-backend | low | open | reality-web shared-comparison loader casts the listings API body with an unchecked `as Promise<ListingSummary>` — no shape validat… |
| `pm-qa-round5-authz-suites-ci-execution-proof-2026-11-06` | pm-qa | high | open | Confirm in CI logs that suite_5 and suite_7 execute the round5_sec_b, r5_sec_a and registry_read_scope tests with non-zero pass co… |
| `pm-qa-screen-map-check-required-status-drift-prs-2026-11-06` | pm-qa | high | open | Hold draft screen-map auto-fix PRs #3333-#3337 until screen-map.yml (/screens validate) is green on each; block auto-approve until… |
| `pm-qa-ready-kmp-plans-assign-implementers-2026-11-06` | pm-qa | high | open | Assign implementers to the two ready mobile-native-kmp plans (inquiries-response-contract, create-listing-not-wired); each plan ne… |
| `pm-qa-churn-hotspot-tests-seed-flakiness-review-2026-11-06` | pm-qa | medium | open | Review churn-hotspot test PR #3339 (suspend_cascade) for flakiness / seed isolation — same class of bug as earlier document_folder… |
| `pm-qa-authz-fix-negative-test-gate-2026-11-06` | pm-qa | low | open | Add a QA gate requiring every authz-fix PR to include a negative test that fails without the fix; spot-check by reverting the fix … |
