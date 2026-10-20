# Action list

_Generated: 2026-10-20T03:21:31Z — regenerated from `action-list.json` (routine Phase 1.6 upkeep, pm-tech-lead rotation; 2026-10-20)._

32 open items. New this run: #3131 approver-identity deadlock (high, pm-devops) + `code-review-reality-server-price-map-no-ratelimit` (high, pm-security — plan promoted this run). #3131 fans out across 16 approved PRs stuck in review; the price-map rate-limit is a direct follow-on to merged PR #3092 (bounded cache cap).

| ID | Priority | Owner | Action |
|---|---|---|---|
| `pm-devops-unblock-mobile-native-cloud-builds` | high | pm-devops | Unblock mobile-native/KMP builds in the cloud runner (issue #2652) — still 5+ open backlog items structurally unclaimable in cloud; now compounded by #2966 so claimable=9/72 (buffer-low) |
| `gh-issue-3124` | high | pm-security | cargo-deny advisories FAILED on dev: RUSTSEC-2026-0368/0369 (zip 8.6.0 via rust_xlsxwriter) blocks every backend PR merge (Closes #3124) |
| `gh-issue-3126` | high | pm-security | cargo-deny: ammonia 4.1.7 RUSTSEC-2026-0373 still fails advisory check on dev (second blocker after zip fix #3125) (Closes #3126) |
| `gh-issue-3131` | high | pm-devops | SECURITY/DELIVERY: resolve the dispatcher-bot approval-identity deadlock — 16 approved PRs stuck in review because the bot cannot self-approve its own PRs; pick a GitHub-App identity or a human-rota and drain the queue oldest-first |
| `code-review-reality-server-price-map-no-ratelimit` | high | pm-security | SECURITY: anonymous /price-map has no per-IP rate limit — attacker rotates city/property_type to force fresh full-table CTE scans past the cache cap; add a bounded limiter mirroring INQUIRY_RATE_LIMIT_* (plan promoted this run) |
| `code-review-mobile-native-kmp-inquiries-response-contract` | medium | pm-backend | mobile-native-kmp InquiriesResponse required page_size mismatches reality-server `limit` — MissingFieldException on every real /inquiries + /realtors/inquiries call |
| `code-review-api-handlers-violations-missing-role-authz` | medium | pm-backend | SECURITY: violations router gates every privileged mutation on tenant membership only (no role check) — any org member can issue fines and approve their own appeal |
| `gh-issue-3069` | medium | pm-tech-lead | Follow-up: scope ViolationRepository::list_notifications by org (same IDOR class as #2944) (PR #2977) (Closes #3069) |
| `gh-issue-3098` | medium | pm-tech-lead | Follow-up: reconcile portfolio_benchmarks schema drift + replace dangling BIT-622 test refs (PR #3087) (Closes #3098) |
| `gh-issue-3104` | medium | pm-tech-lead | Portfolio/owner analytics: comparisons + dashboard summary + owner portfolio runtime drift (500s) (Closes #3104) |
| `gh-issue-3109` | medium | pm-tech-lead | Follow-up: implement api-server `DELETE /community/groups/{id}` so wired frontend delete works end-to-end (PR #3101) (Closes #3109) |
| `gh-issue-3116` | medium | pm-tech-lead | Follow-up: NewsScreen category badge lacks the i18n defaultValue fallback that ArticleDetailScreen uses (PR #3110) (Closes #3116) |
| `code-review-api-handlers-migration-upload-drops-file` | medium | pm-backend | api-server /migration/import/upload: silent dead-store drops uploaded bytes (data loss on Epic 66 bulk-import) |
| `pm-integration-audit-integration-prs-openapi-drift` | medium | pm-integration | Audit integration-touching PRs since 2026-10-17 for OpenAPI/TypeSpec drift (draft #3123 is api-core TOTP RNG — no contract impact; recent merged #3121/#3122 are client-side only) |
| `pm-integration-rustsec-exception-policy-ci-surface` | medium | pm-integration | Surface the RUSTSEC-exception policy in the cargo-deny CI gate description so #3124-class blockers are triaged faster next time (bounded deadline, approver captured in the exception row) |
| `gh-issue-3128` | medium | pm-tech-lead | Follow-up: type the AML assessments list endpoint instead of untyped serde_json::Value (PR #3122) (Closes #3128) |
| `code-review-mobile-native-kmp-portfolio-analytics-caps-100` | low | pm-backend | mobile-native-kmp: getPortfolioAnalytics() truncates realtor portfolio at 100 listings — dashboard under-reports on large portfolios |
| `code-review-mobile-native-kmp-portfolio-analytics-unbounded-fanout-retry1` | low | pm-backend | mobile-native-kmp: getPortfolioAnalytics() fans out one analytics HTTP request per listing with no concurrency limit — up to 100 parallel GETs from a mobile device [retry 1/2] |
| `code-review-mobile-native-kmp-cancellation-swallowed` | low | pm-backend | mobile-native-kmp: shared repositories swallow CancellationException in catch(e: Exception), breaking coroutine cancellation and showing spurious errors |
| `code-review-mobile-native-kmp-ssoservice-untested` | low | pm-qa | mobile-native-kmp: SsoService (deep-link token exchange, login, password reset, session restore) has zero direct tests |
| `code-review-mobile-native-kmp-httpclient-no-timeout` | low | pm-backend | mobile-native-kmp shared Ktor HttpClient installs no HttpTimeout — every suspend API call can hang indefinitely on Android + iOS |
| `code-review-mobile-native-kmp-create-listing-not-wired` | low | pm-backend | KMP realtor CreateListingScreen onSubmit is a NotImplementedError stub — form data discarded |
| `screen-map-drift-pr-2994-ppt` | low | pm-qa | ppt-web route refactor (PR #2994) landed without touching docs/screens/ppt |
| `code-review-reality-server-add-to-compare-toctou-max-cap` | low | pm-backend | reality-server add_to_compare TOCTOU on MAX_COMPARE_LISTINGS — two concurrent POSTs can exceed the 4-item cap |
| `code-review-api-handlers-violations-number-count-race` | low | pm-backend | violation_number / appeal_number generated via COUNT(*)+1 on the pool (no tx/sequence) collides with UNIQUE(org, number) under concurrency → opaque 500 on create |
| `code-review-api-handlers-violations-record-payment-non-atomic` | low | pm-backend | record_payment INSERTs fine_payments then UPDATEs enforcement_actions.paid_amount on separate pool connections with no transaction → partial failure records money without decrementing the fine |
| `code-review-reality-server-register-account-enumeration` | low | pm-backend | reality-server: POST /api/v1/users/register leaks existence via 409+differentiated body (account-enumeration oracle) |
| `code-review-api-core-totp-rng-expect-panic` | low | pm-backend | api-core: TotpService::encrypt_secret() panics on CSPRNG failure via .expect() instead of threading TotpError |
| `code-review-reality-web-formatprice-hardcoded-locale` | low | pm-backend | reality-web: ListingHeader & AgencyListings hardcode locale=en-US in formatPrice on a 4-language portal |
| `code-review-api-handlers-impersonation-active-raw-xff-audit` | low | pm-backend | admin /impersonation/active audit skips `audit_ip_from_headers` → raw XFF overflows VARCHAR(45), audit INSERT fails silently |
| `code-review-api-handlers-migration-export-fake-url-stub` | low | pm-backend | api-server /migration/export: auto-ready with fake example.com URL + fabricated record counts (silent broken feature) |
| `pm-integration-document-sso-transient-401-contract` | low | pm-integration | Document the reality-server SSO consumer transient-401 contract vs reality-web auth-context cushion (PR #3121) in docs/api/README.md — ensure client/server treat transient vs permanent 401 symmetrically |
