# Action list

_Generated: 2026-10-17T03:24:00Z — regenerated from `action-list.json` (routine Phase 1.6 upkeep, pm-integration rotation)._

25 open items. New additions this run (pm-integration role): `pm-integration-community-group-delete-endpoint`, `pm-integration-realtor-id-wire-page-to-api`, `pm-integration-cross-service-contract-drift-audit`.

| ID | Priority | Owner | Action |
|---|---|---|---|
| `pm-devops-unblock-mobile-native-cloud-builds` | high | pm-devops | Unblock mobile-native/KMP builds in the cloud runner (issue #2652) — still 5+ open backlog items structurally unclaimable in cloud; now compounded by #2966 so claimable=9/72 (buffer-low) |
| `pm-integration-community-group-delete-endpoint` | high | pm-backend | Implement DELETE /api/v1/community/groups/{id} on api-server (route in community.rs next to get_group, utoipa path, auth + owner/manager + tenant/RLS scoping, 204/403/404 shape, idempotent) — unblocks ppt-web community group delete already wired in PR #3101 (issue #3109). |
| `pm-integration-realtor-id-wire-page-to-api` | high | pm-frontend | Wire reality-web /[locale]/realtor/[id] page to existing public GET /api/v1/realtors/{user_id}/profile via @ppt/reality-api-client — replace MOCK_AGENT (3 code-review-findings: shared-mock score 2, dead contact-form score 2, no-i18n score 1). Add 404 state, confirm [id] == user_id semantics, thread next-intl. |
| `code-review-mobile-native-kmp-inquiries-response-contract` | medium | pm-backend | mobile-native-kmp InquiriesResponse required page_size mismatches reality-server `limit` — MissingFieldException on every real /inquiries + /realtors/inquiries call |
| `code-review-api-handlers-violations-missing-role-authz` | medium | pm-backend | SECURITY: violations router gates every privileged mutation on tenant membership only (no role check) — any org member can issue fines and approve their own appeal |
| `gh-issue-3069` | medium | pm-tech-lead | Follow-up: scope ViolationRepository::list_notifications by org (same IDOR class as #2944) (PR #2977) (Closes #3069) |
| `code-review-reality-server-price-map-cache-unbounded` | medium | pm-backend | SECURITY: reality-server PRICE_MAP_CACHE unbounded on anonymous GET /price-map — attacker-controlled city rotates distinct keys past the 1h TTL → OOM DoS |
| `gh-issue-3098` | medium | pm-tech-lead | Follow-up: reconcile portfolio_benchmarks schema drift + replace dangling BIT-622 test refs (PR #3087) (Closes #3098) |
| `gh-issue-3104` | medium | pm-tech-lead | Portfolio/owner analytics: comparisons + dashboard summary + owner portfolio runtime drift (500s) (Closes #3104) |
| `pm-integration-cross-service-contract-drift-audit` | medium | pm-qa | Cross-service contract-drift audit: diff every frontend-called path (ppt-web + reality-web hooks + mobile) against axum routes and OpenAPI spec; flag DELETE/PUT/PATCH first. Add the check to api-validation CI so a future wired-but-missing endpoint (like #3101→#3109) fails on PR. |
| `code-review-mobile-native-kmp-httpclient-no-timeout` | low | pm-backend | mobile-native-kmp shared Ktor HttpClient installs no HttpTimeout — every suspend API call can hang indefinitely on Android + iOS |
| `code-review-mobile-native-kmp-cancellation-swallowed` | low | pm-backend | mobile-native-kmp: shared repositories swallow CancellationException in catch(e: Exception), breaking coroutine cancellation and showing spurious errors |
| `code-review-mobile-native-kmp-portfolio-analytics-caps-100` | low | pm-backend | mobile-native-kmp: getPortfolioAnalytics() truncates realtor portfolio at 100 listings — dashboard under-reports on large portfolios |
| `code-review-mobile-native-kmp-portfolio-analytics-unbounded-fanout-retry1` | low | pm-backend | mobile-native-kmp: getPortfolioAnalytics() fans out one analytics HTTP request per listing with no concurrency limit — up to 100 parallel GETs from a mobile device [retry 1/2] |
| `code-review-mobile-native-kmp-ssoservice-untested` | low | pm-qa | mobile-native-kmp: SsoService (deep-link token exchange, login, password reset, session restore) has zero direct tests |
| `code-review-mobile-native-kmp-create-listing-not-wired` | low | pm-backend | KMP realtor CreateListingScreen onSubmit is a NotImplementedError stub — form data discarded |
| `screen-map-drift-pr-2994-ppt` | low | pm-qa | ppt-web route refactor (PR #2994) landed without touching docs/screens/ppt |
| `code-review-reality-server-add-to-compare-toctou-max-cap` | low | pm-backend | reality-server add_to_compare TOCTOU on MAX_COMPARE_LISTINGS — two concurrent POSTs can exceed the 4-item cap |
| `code-review-api-handlers-violations-number-count-race` | low | pm-backend | violation_number / appeal_number generated via COUNT(*)+1 on the pool (no tx/sequence) collides with UNIQUE(org, number) under concurrency → opaque 500 on create |
| `code-review-api-handlers-violations-record-payment-non-atomic` | low | pm-backend | record_payment INSERTs fine_payments then UPDATEs enforcement_actions.paid_amount on separate pool connections with no transaction → partial failure records money without decrementing the fine |
| `churn-hotspot-frontend/apps/mobile/src/App.outageNewsDetail.test.tsx` | low | pm-tech-lead | Churn hotspot: frontend/apps/mobile/src/App.outageNewsDetail.test.tsx — new in PR #3094 (OutageDetail/ArticleDetail route-wiring regression test) |
| `churn-hotspot-frontend/apps/mobile/src/screens/news/ArticleDetailScreen.tsx` | low | pm-tech-lead | Churn hotspot: frontend/apps/mobile/src/screens/news/ArticleDetailScreen.tsx — new in PR #3094 (minimal RN article-detail view) |
| `churn-hotspot-frontend/apps/mobile/src/screens/outages/OutagesScreen.transforms.test.ts` | low | pm-tech-lead | Churn hotspot: frontend/apps/mobile/src/screens/outages/OutagesScreen.transforms.test.ts — new in PR #3093 (narrow/toUiOutage/formatRange transform unit tests) |
| `code-review-mobile-rn-news-personmonths-badge-untranslated` | low | pm-backend | mobile-rn: NewsScreen/ArticleDetailScreen category and PersonMonthsScreen status badges render raw English enum values on i18n screens |
| `stalled-review-pr-3024` | low | pm-devops | Stalled review: PR #3024 docs(ppt-web) reconcile dashboard screen-maps with PR #2994 auth-guard — open 10d, no reviewDecision |
