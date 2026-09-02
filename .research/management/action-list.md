# Action list

_Generated: 2026-09-02T00:35:00Z — regenerated from `action-list.json`._

| ID | Priority | Owner | Action |
|---|---|---|---|
| `pm-devops-unblock-mobile-native-cloud-builds` | high | pm-devops | Unblock mobile-native/KMP builds in the cloud runner (issue #2652) — currently 7/8 open backlog items are structurally unclaimable in cloud, forcing Tier-1d generator kicks every r |
| `pm-scrum-master-promote-84-x-directly` | high | pm-frontend | Promote 84-1 (ppt-web direct-to-S3 upload) and 84-2 (signer-facing document-sign page) directly from ranked backlog to an implementer-pair window — 5 upkeep windows without dispatc |
| `pm-security-compliance-db-error-regression-test` | high | pm-qa | Add integration test for the compliance audit-log DB-error path (#2925 regression guard): inject a sqlx failure, assert 500 body contains NO connection-string or SQL fragments |
| `pm-security-h2-rustsec-2026-0258` | high | pm-security | Land the h2 crate bump for RUSTSEC-2026-0258 (empty-DATA-frame DoS; gh-issue-2797) — confirm patched h2 present in backend/Cargo.lock, cargo-deny advisories green on dev |
| `pm-security-raw-db-leak-audit-sibling-sites` | high | pm-backend | Grep-sweep for the raw-sqlx-error leak class fixed in PR #2925 across backend/servers/api-server/src/routes/** — every secondary DB call (count/aggregate helpers) that stringifies  |
| `code-review-mobile-native-kmp-inquiries-response-contract` | medium | pm-backend | mobile-native-kmp InquiriesResponse required page_size mismatches reality-server `limit` — MissingFieldException on every real /inquiries + /realtors/inquiries call |
| `pm-security-close-carried-webhook-cache-risks` | medium | pm-security | Close carried risks #2485 (layout-webhook replay guard) and #2486 (mobile LAYOUT_CACHE_KEY cross-tenant) — both open 6+ weeks with no PR movement; land PRs or explicitly de-priorit |
| `pm-security-push-fanout-userid-log-scrubbing-policy` | medium | pm-devops | Audit push_fanout.rs receipt-processing paths for user_id in structured logs (`user_id = %user_id` at info+ level, ~10 sites) — decide policy: scrub at shipper OR downgrade info→de |
| `pm-security-typed-error-enum-extend-inquiries-reports` | medium | pm-backend | Extend the reality-server typed error enum pattern from PR #2922 (saved_searches) to inquiries.rs and reports.rs on the same server — 1 more route migrated + regression test this s |
| `code-review-mobile-native-kmp-cancellation-swallowed` | low | pm-backend | mobile-native-kmp: shared repositories swallow CancellationException in catch(e: Exception), breaking coroutine cancellation and showing spurious errors |
| `code-review-mobile-native-kmp-create-listing-not-wired` | low | pm-backend | KMP realtor CreateListingScreen onSubmit is a NotImplementedError stub — form data discarded |
| `code-review-mobile-native-kmp-httpclient-no-timeout` | low | pm-backend | mobile-native-kmp shared Ktor HttpClient installs no HttpTimeout — every suspend API call can hang indefinitely on Android + iOS |
| `code-review-mobile-native-kmp-portfolio-analytics-caps-100` | low | pm-backend | mobile-native-kmp: getPortfolioAnalytics() truncates realtor portfolio at 100 listings — dashboard under-reports on large portfolios |
| `code-review-mobile-native-kmp-portfolio-analytics-unbounded-fanout-retry1` | low | pm-backend | mobile-native-kmp: getPortfolioAnalytics() fans out one analytics HTTP request per listing with no concurrency limit — up to 100 parallel GETs from a mobile device [retry 1/2 of fa |
| `code-review-mobile-native-kmp-ssoservice-untested` | low | pm-qa | mobile-native-kmp: SsoService (deep-link token exchange, login, password reset, session restore) has zero direct tests |
| `code-review-reality-server-agent-review-self-review` | low | pm-backend | reality-server agent review endpoint allows self-review — an agent can post a review of their own reviews via missing subject != reviewer gate |
| `screen-map-drift-pr-2894-reality` | low | pm-qa | screen-map-drift: PR #2894 touched reality-web routes without updating docs/scre |
