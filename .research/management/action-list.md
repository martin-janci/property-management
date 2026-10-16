# Action list

_Generated: 2026-10-16T10:54:11Z — regenerated from `action-list.json` by routine Phase 1.6 (pm-scrum-master + pm-integration rotation 2026-10-16)._

30 open items. This run: +8 new actions from pm-scrum-master (infra unblocks, stalled-PR reviewer slots) and pm-integration (analytics drift PR #3107, OpenAPI inquiries source-of-truth, repo schema-drift CI guard, outbound-SSRF audit). Marked done: 2 mobile-rn i18n items resolved by PRs #3105 / #3106.

| ID | Priority | Owner | Action |
|---|---|---|---|
| `pm-backend-land-pr-3107-analytics-drift` | high | pm-backend | Land draft PR #3107 (portfolio_benchmarks reconcile) and the other #3104 items: remove name column from property_comparisons SELECT, fix portfolio lookup, fix get_dashboard_summary via add-column migrations (00236/00237/00238); un-ignore pa_list_comparisons_succeeds, pp_list_comparisons_succeeds, analytics_owner_success_tests |
| `pm-backend-openapi-source-of-truth-inquiries` | high | pm-backend | Make OpenAPI the source of truth for reality-server inquiries responses; align TypeSpec, utoipa, @ppt/reality-api-client and KMP openapi-generator on limit vs page_size and realtors response shape; add a CI contract check in api-validation.yml |
| `pm-devops-retry-idor-fixes-2944-2945-2946` | high | pm-devops | Retry cross-tenant IDOR fixes #2946 (portfolio_analytics), #2944 (dispute comments/evidence/payments/internal-notes leak) and #2945 (portfolio_properties) once #2966 is cleared |
| `pm-devops-unblock-mobile-native-cloud-builds` | high | pm-devops | Unblock mobile-native/KMP builds in the cloud runner (issue #2652) — still 5+ open backlog items structurally unclaimable in cloud; now compounded by #2966 so claimable=9/72 (buffer-low) |
| `pm-devops-unblock-utoipa-swagger-ui-egress` | high | pm-devops | Unblock utoipa-swagger-ui crates.io egress for api-server in the cloud runner (#2966) so IDOR retries (#2946 #2944 #2945) can build |
| `code-review-api-handlers-violations-missing-role-authz` | medium | pm-backend | SECURITY: violations router gates every privileged mutation on tenant membership only (no role check) — any org member can issue fines and approve their own appeal |
| `code-review-mobile-native-kmp-inquiries-response-contract` | medium | pm-backend | mobile-native-kmp InquiriesResponse required page_size mismatches reality-server `limit` — MissingFieldException on every real /inquiries + /realtors/inquiries call |
| `code-review-reality-server-price-map-cache-unbounded` | medium | pm-backend | SECURITY: reality-server PRICE_MAP_CACHE unbounded on anonymous GET /price-map — attacker-controlled city rotates distinct keys past the 1h TTL → OOM DoS |
| `pm-qa-schema-drift-guard-repos` | medium | pm-qa | Add a schema-drift guard for repositories: CI test that executes each repo list/summary query against the migrated schema (smoke test per repo), so a SELECT on a missing column fails in CI not at runtime 500 |
| `pm-security-audit-outbound-http-after-nat64-ssrf` | medium | pm-security | Audit outbound HTTP callers after #3103 NAT64 SSRF fix: confirm e-signature, webhook, S3 and embedding integrations route through the filtered client, and that timeouts/retries/error mapping behave consistently when a destination is blocked |
| `gh-issue-3069` | medium | pm-tech-lead | Follow-up: scope ViolationRepository::list_notifications by org (same IDOR class as #2944) (PR #2977) (Closes #3069) |
| `gh-issue-3098` | medium | pm-tech-lead | Follow-up: reconcile portfolio_benchmarks schema drift + replace dangling BIT-622 test refs (PR #3087) (Closes #3098) |
| `gh-issue-3104` | medium | pm-tech-lead | Portfolio/owner analytics: comparisons + dashboard summary + owner portfolio runtime drift (500s) (Closes #3104) |
| `pm-tech-lead-reviewer-slots-2559-2555` | medium | pm-tech-lead | Assign reviewer slots to stalled PRs #2559 (PAY by Square QR) and #2555 (invoice lifecycle), or close them |
| `code-review-api-handlers-violations-number-count-race` | low | pm-backend | violation_number / appeal_number generated via COUNT(*)+1 on the pool (no tx/sequence) collides with UNIQUE(org, number) under concurrency → opaque 500 on create |
| `code-review-api-handlers-violations-record-payment-non-atomic` | low | pm-backend | record_payment INSERTs fine_payments then UPDATEs enforcement_actions.paid_amount on separate pool connections with no transaction → partial failure records money without decrementing the fine |
| `code-review-mobile-native-kmp-cancellation-swallowed` | low | pm-backend | mobile-native-kmp: shared repositories swallow CancellationException in catch(e: Exception), breaking coroutine cancellation and showing spurious errors |
| `code-review-mobile-native-kmp-create-listing-not-wired` | low | pm-backend | KMP realtor CreateListingScreen onSubmit is a NotImplementedError stub — form data discarded |
| `code-review-mobile-native-kmp-httpclient-no-timeout` | low | pm-backend | mobile-native-kmp shared Ktor HttpClient installs no HttpTimeout — every suspend API call can hang indefinitely on Android + iOS |
| `code-review-mobile-native-kmp-portfolio-analytics-caps-100` | low | pm-backend | mobile-native-kmp: getPortfolioAnalytics() truncates realtor portfolio at 100 listings — dashboard under-reports on large portfolios |
| `code-review-mobile-native-kmp-portfolio-analytics-unbounded-fanout-retry1` | low | pm-backend | mobile-native-kmp: getPortfolioAnalytics() fans out one analytics HTTP request per listing with no concurrency limit — up to 100 parallel GETs from a mobile device [retry 1/2] |
| `code-review-mobile-rn-news-personmonths-badge-untranslated` | low | pm-backend | mobile-rn: NewsScreen/ArticleDetailScreen category and PersonMonthsScreen status badges render raw English enum values on i18n screens |
| `code-review-reality-server-add-to-compare-toctou-max-cap` | low | pm-backend | reality-server add_to_compare TOCTOU on MAX_COMPARE_LISTINGS — two concurrent POSTs can exceed the 4-item cap |
| `stalled-review-pr-3024` | low | pm-devops | Stalled review: PR #3024 docs(ppt-web) reconcile dashboard screen-maps with PR #2994 auth-guard — open 10d, no reviewDecision |
| `pm-frontend-review-pr-2902-and-3006` | low | pm-frontend | Review draft PR #2902 (saved-searches screen-map drift vs PR #2894) with /screens validate; triage #3006 (api-client auth-header + MFA-retry unification) |
| `code-review-mobile-native-kmp-ssoservice-untested` | low | pm-qa | mobile-native-kmp: SsoService (deep-link token exchange, login, password reset, session restore) has zero direct tests |
| `screen-map-drift-pr-2994-ppt` | low | pm-qa | ppt-web route refactor (PR #2994) landed without touching docs/screens/ppt |
| `churn-hotspot-frontend/apps/mobile/src/App.outageNewsDetail.test.tsx` | low | pm-tech-lead | Churn hotspot: frontend/apps/mobile/src/App.outageNewsDetail.test.tsx — new in PR #3094 (OutageDetail/ArticleDetail route-wiring regression test) |
| `churn-hotspot-frontend/apps/mobile/src/screens/news/ArticleDetailScreen.tsx` | low | pm-tech-lead | Churn hotspot: frontend/apps/mobile/src/screens/news/ArticleDetailScreen.tsx — new in PR #3094 (minimal RN article-detail view) |
| `churn-hotspot-frontend/apps/mobile/src/screens/outages/OutagesScreen.transforms.test.ts` | low | pm-tech-lead | Churn hotspot: frontend/apps/mobile/src/screens/outages/OutagesScreen.transforms.test.ts — new in PR #3093 (narrow/toUiOutage/formatRange transform unit tests) |

---

### Recently done

- `code-review-mobile-rn-detail-back-button-hardcoded-en` — mobile-rn: NEW OutageDetail/ArticleDetail back-button labels render raw '‹ News' / '‹ Outages' English in files whose other strings resolve through t('…')
- `code-review-mobile-rn-outage-status-untranslated` — mobile-rn: OutagesScreen status badge renders raw outage.status.replace('_',' ') in English on a screen whose filters are already t('outages.filters.*')
