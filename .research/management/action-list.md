# Action list

_Generated: 2026-10-06T06:30:00Z — regenerated from `action-list.json` (routine Phase 1.6 upkeep, pm-data rotation)._

32 open items. Three items closed this run (#3017, #3016 fixes landed as #3019 / #3022; dashboard screen-map drift superseded by #3027). Two pm-data items added. Top of list unchanged: the three draft-but-approved unblocks (#3025/#3026/#3028) sitting on human-merge, plus the infra blocker they are gated on.

## Top (unblockers + stalled)

| ID | Priority | Owner | Action |
|---|---|---|---|
| `pm-devops-unblock-api-server-utoipa-swagger-egress` | high | pm-devops | Unblock utoipa-swagger-ui crates.io / egress for api-server (#2966) — **fix in draft PR #3026, reviewer-approved; needs human approve + merge**. Single largest buffer-starvation lever |
| `gh-issue-2946-retry2` | high | pm-security | Cross-tenant IDOR portfolio_analytics (Closes #2946) [retry 2/2] — draft PR open; blocked by #2966 |
| `gh-issue-2944-retry1` | high | pm-security | Cross-tenant IDOR violation comments/evidence/payments + internal-notes leak (Closes #2944) [retry 1/2] — blocked by #2966 |
| `gh-issue-2945-retry1` | high | pm-security | Cross-tenant IDOR portfolio_properties handlers missing org verification (Closes #2945) [retry 1/2] — blocked by #2966 |
| `pm-devops-unblock-mobile-native-cloud-builds` | high | pm-devops | Unblock mobile-native/KMP cloud builds (#2652) |
| `pm-devops-unblock-mobile-rn-jest-expo-version-rot` | medium | pm-devops | Resolve mobile RN jest-expo version rot (#2951) — **fix in draft PR #3025, approved; needs human merge** |
| `pm-scrum-master-stalled-accounting-pay-by-square-2559` | medium | pm-tech-lead | Stalled 68d: PR #2559 (PAY by Square accounting) needs reviewer slot |
| `pm-scrum-master-stalled-invoice-lifecycle-2555` | medium | pm-tech-lead | Stalled 68d: PR #2555 (invoice lifecycle accounting) needs reviewer slot |

## pm-data (new this run — rotation idx 6)

| ID | Priority | Owner | Action |
|---|---|---|---|
| `pm-data-silent-dropped-input-contract-tests-2026-10-06` | medium | pm-qa | Add a vitest data-contract-fuzz helper (frontend/packages/testing-helpers); mount form, submit against msw, diff-assert form-schema vs captured request body; pilot on reality-web /sell + /report (both just fixed this run) |
| `pm-data-reviewer-feedback-classifier-2026-10-06` | low | pm-data | Daily jq-based bucket counter over assignments.json + archive; emit top-N reviewer-note classifier into project-state.md |

## Backlog code-review items (carried)

| ID | Priority | Owner | Action |
|---|---|---|---|
| `code-review-ppt-web-core-authed-roots-session-leak` | medium | pm-backend | logout() AUTHED_QUERY_KEY_ROOTS allow-list omits mounted route groups — PR #2987 approved; needs human merge |
| `code-review-reality-server-add-to-compare-toctou-max-cap` | medium | pm-tech-lead | reality-server add-to-compare TOCTOU + max-cap — PR #3028 approved; needs human merge |
| `code-review-reality-server-dup-url-validator` | low | pm-tech-lead | reality-server duplicated URL-validation helper — PR #2992 approved; needs human merge (sqlx suite after #2966 clears) |
| `code-review-mobile-native-kmp-inquiries-response-contract` | medium | pm-backend | KMP InquiriesResponse page_size vs reality-server `limit` mismatch |
| `code-review-mobile-native-kmp-httpclient-no-timeout` | low | pm-backend | KMP shared Ktor HttpClient has no HttpTimeout |
| `code-review-mobile-native-kmp-cancellation-swallowed` | low | pm-backend | KMP repositories swallow CancellationException |
| `code-review-mobile-native-kmp-portfolio-analytics-caps-100` | low | pm-backend | getPortfolioAnalytics() truncates realtor portfolio at 100 listings |
| `code-review-mobile-native-kmp-portfolio-analytics-unbounded-fanout-retry1` | low | pm-backend | getPortfolioAnalytics() unbounded fan-out [retry 1/2] |
| `code-review-mobile-native-kmp-ssoservice-untested` | low | pm-qa | KMP SsoService has zero direct tests |
| `code-review-mobile-native-kmp-create-listing-not-wired` | low | pm-backend | KMP CreateListingScreen onSubmit is NotImplementedError stub |
| `code-review-mobile-rn-meter-reading-no-numeric-validation` | low | pm-backend | RN meter-reading input accepts non-numeric — PR #2997 approved |
| `code-review-ppt-web-ui-workflow-automation-toasts-hardcoded-english` | medium | pm-frontend | workflow-automation toasts hardcoded English — PR #3020 approved; needs human merge |
| `code-review-ppt-web-core-community-group-detail-mock-data` | medium | pm-frontend | ppt-web community-group detail still uses mock data |
| `code-review-ppt-web-core-route-group-toast-i18n` | low | pm-frontend | ppt-web route-group action toasts hardcoded English — same class as workflow-automation |
| `code-review-reality-web-profile-mock-data` | medium | pm-frontend | reality-web profile page still uses mock data |
| `code-review-reality-server-create-handlers-return-200-not-201` | low | pm-backend | reality-server create handlers return 200 not 201 |
| `screen-map-drift-pr-3013-reality` | low | pm-frontend | screen-map drift from PR #3013 (reality) |
| `screen-map-drift-pr-3014-reality` | low | pm-frontend | screen-map drift from PR #3014 (reality) |
| `churn-hotspot-frontend/apps/ppt-web/src/features/workflow-automation/pages/CreateRulePage.tsx` | low | pm-frontend | churn hotspot: CreateRulePage |
| `churn-hotspot-frontend/apps/ppt-web/src/features/workflow-automation/pages/TemplateLibraryPage.tsx` | low | pm-frontend | churn hotspot: TemplateLibraryPage |
| `churn-hotspot-frontend/apps/ppt-web/src/routes/groups/core.tsx` | low | pm-frontend | churn hotspot: groups/core.tsx |
| `churn-hotspot-frontend/apps/reality-web/src/components/home/FeaturedListings.tsx` | low | pm-frontend | churn hotspot: FeaturedListings |

## Closed this run

| ID | Resolved by |
|---|---|
| `churn-hotspot-frontend/apps/reality-web/src/app/[locale]/report/page.tsx` | PR #3019 (Closes #3017) |
| `churn-hotspot-frontend/apps/reality-web/src/app/[locale]/sell/page.tsx` | PR #3022 (Closes #3016) |
| `screen-map-drift-pr-2994-ppt` | superseded by PR #3027 (draft PR #3024 verdict=changes abandoned) |
