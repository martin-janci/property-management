# Action list — PPT delivery

_Generated: 2026-10-14T04:30:00Z_

| id | priority | owner | status | action | depends_on |
|---|---|---|---|---|---|
| pm-devops-unblock-mobile-native-cloud-builds | high | pm-devops | open | Unblock mobile-native/KMP builds in the cloud runner (issue #2652) — still 5+ open backlog items structurally unclaimable in cloud; now compounded by #2966 so claimable=9/72 (bu... | — |
| gh-issue-3076 | high | pm-security | open | cargo-deny: bump `ammonia` to clear RUSTSEC-2021-0157 (blocks all backend PRs) (Closes #3076) | — |
| gh-issue-3079 | high | pm-security | open | Follow-up: SSRF validator dedup dropped IPv4-compatible IPv6 unmapping — literal ::127.0.0.1 / ::169.254.169.254 now pass the static check (PR #2992) (Closes #3079) | — |
| pm-data-instrument-reality-sell-funnel | high | pm-data | open | Instrument reality-web /sell wizard funnel — emit listing.sell_wizard.{step_shown,step_completed,publish_submitted,publish_succeeded,publish_failed} via existing trackEvent bus;... | — |
| pm-data-instrument-reality-report-funnel | high | pm-data | open | Instrument reality-web /report submission flow — emit report.{form_shown,submitted,submit_succeeded,submit_failed} with returned id from #3046 and error class from #3067; agent-... | — |
| pm-data-portfolio-analytics-health-incident | high | pm-data | open | Treat portfolio-analytics property-metrics 500 (#3078 / open PR #3081) as a data-health incident: add alert on realtor.portfolio_metrics.5xx_rate > 1%; backfill plan for dashboa... | UNRESOLVED:PR #3081 land — portfolio-analytics 500 |
| code-review-mobile-native-kmp-inquiries-response-contract | medium | pm-backend | open | mobile-native-kmp InquiriesResponse required page_size mismatches reality-server `limit` — MissingFieldException on every real /inquiries + /realtors/inquiries call | pm-devops-unblock-mobile-native-cloud-builds |
| code-review-api-handlers-violations-missing-role-authz | medium | pm-backend | open | SECURITY: violations router gates every privileged mutation on tenant membership only (no role check) — any org member can issue fines and approve their own appeal | — |
| gh-issue-3069 | medium | pm-tech-lead | open | Follow-up: scope ViolationRepository::list_notifications by org (same IDOR class as #2944) (PR #2977) (Closes #3069) | — |
| gh-issue-3078 | medium | pm-tech-lead | open | Follow-up: portfolio-analytics property-metrics 500 at runtime (untracked schema drift) + IDOR regression tests assert too weakly (PR #3003) (Closes #3078) | — |
| pm-data-currency-metric-source-of-truth | medium | pm-data | open | Define canonical lease.currency / payment.currency metric dimensions in docs/analytics/metrics.md (follow-up on PR #3051 EUR hardcoding fix); deprecate any dashboard that comput... | — |
| pm-data-mutation-error-funnel-event | medium | pm-data | open | Add shared client.mutation.error_surfaced event helper + wire at 3 ppt-web sites (community group join/leave #3048, documents #3049, rentals #3050); downstream dashboard tile in... | — |
| pm-data-minimum-analytics-dod | medium | pm-data | open | Author minimum-analytics DoD (event name convention, required properties, PII rules, screen-map agent-log stamp); add checklist item to .github/pull_request_template.md so new s... | — |
| code-review-mobile-native-kmp-portfolio-analytics-caps-100 | low | pm-backend | open | mobile-native-kmp: getPortfolioAnalytics() truncates realtor portfolio at 100 listings — dashboard under-reports on large portfolios | pm-devops-unblock-mobile-native-cloud-builds |
| code-review-mobile-native-kmp-portfolio-analytics-unbounded-fanout-retry1 | low | pm-backend | open | mobile-native-kmp: getPortfolioAnalytics() fans out one analytics HTTP request per listing with no concurrency limit — up to 100 parallel GETs from a mobile device [retry 1/2] | pm-devops-unblock-mobile-native-cloud-builds |
| code-review-mobile-native-kmp-cancellation-swallowed | low | pm-backend | open | mobile-native-kmp: shared repositories swallow CancellationException in catch(e: Exception), breaking coroutine cancellation and showing spurious errors | pm-devops-unblock-mobile-native-cloud-builds |
| code-review-mobile-native-kmp-ssoservice-untested | low | pm-qa | open | mobile-native-kmp: SsoService (deep-link token exchange, login, password reset, session restore) has zero direct tests | pm-devops-unblock-mobile-native-cloud-builds |
| code-review-mobile-native-kmp-httpclient-no-timeout | low | pm-backend | open | mobile-native-kmp shared Ktor HttpClient installs no HttpTimeout — every suspend API call can hang indefinitely on Android + iOS | pm-devops-unblock-mobile-native-cloud-builds |
| code-review-mobile-native-kmp-create-listing-not-wired | low | pm-backend | open | KMP realtor CreateListingScreen onSubmit is a NotImplementedError stub — form data discarded | pm-devops-unblock-mobile-native-cloud-builds |
| screen-map-drift-pr-2994-ppt | low | pm-qa | open | ppt-web route refactor (PR #2994) landed without touching docs/screens/ppt | — |
| code-review-reality-server-add-to-compare-toctou-max-cap | low | pm-backend | open | reality-server add_to_compare TOCTOU on MAX_COMPARE_LISTINGS — two concurrent POSTs can exceed the 4-item cap | — |
| code-review-api-handlers-violations-number-count-race | low | pm-backend | open | violation_number / appeal_number generated via COUNT(*)+1 on the pool (no tx/sequence) collides with UNIQUE(org, number) under concurrency → opaque 500 on create | — |
| code-review-api-handlers-violations-record-payment-non-atomic | low | pm-backend | open | record_payment INSERTs fine_payments then UPDATEs enforcement_actions.paid_amount on separate pool connections with no transaction → partial failure records money without decrem... | — |
