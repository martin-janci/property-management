# Action list

_Generated: 2026-10-01T04:30:00Z — regenerated from `action-list.json` (routine Phase 1.6 upkeep, pm-security rotation)._

20 open items. Top of list is the two infra unblocks that gate most of the rest. New additions this run: `#2966 swagger-ui egress`, `#3006 auth-header unify`, two stalled-PR reviewer-slot items (`#2559` / `#2555`), `#2951 jest-expo`.

| ID | Priority | Owner | Action |
|---|---|---|---|
| `pm-devops-unblock-api-server-utoipa-swagger-egress` | high | pm-devops | Unblock utoipa-swagger-ui crates.io / egress for api-server in cloud runner (#2966) — single largest buffer-starvation lever; gates all 3 IDOR retries |
| `gh-issue-2946-retry2` | high | pm-security | Cross-tenant IDOR portfolio_analytics (Closes #2946) [retry 2/2] — draft PR gh-issue-2946-retry2 open; blocked by #2966 |
| `gh-issue-2944-retry1` | high | pm-security | Cross-tenant IDOR violation comments/evidence/payments + internal-notes privilege leak (Closes #2944) [retry 1/2] |
| `gh-issue-2945-retry1` | high | pm-security | Cross-tenant IDOR portfolio_properties handlers missing org verification (Closes #2945) [retry 1/2] |
| `gh-issue-3006-unify-auth-header-mfa-retry` | medium | pm-tech-lead | Unify triplicated auth-header + MFA-retry logic in @ppt/api-client fetch primitives (post-merge review of PR #3004) |
| `pm-scrum-master-stalled-accounting-pay-by-square-2559` | medium | pm-tech-lead | Stalled 64d: PR #2559 (PAY by Square accounting) needs reviewer slot |
| `pm-scrum-master-stalled-invoice-lifecycle-2555` | medium | pm-tech-lead | Stalled 64d: PR #2555 (invoice lifecycle accounting) needs reviewer slot |
| `pm-devops-unblock-mobile-native-cloud-builds` | high | pm-devops | Unblock mobile-native/KMP builds in cloud runner (#2652) |
| `pm-devops-unblock-mobile-rn-jest-expo-version-rot` | medium | pm-devops | Resolve mobile RN jest-expo version rot (#2951) |
| `code-review-ppt-web-core-authed-roots-session-leak` | medium | pm-backend | logout() AUTHED_QUERY_KEY_ROOTS allow-list omits mounted route groups |
| `code-review-mobile-native-kmp-inquiries-response-contract` | medium | pm-backend | KMP InquiriesResponse page_size vs reality-server `limit` mismatch |
| `code-review-mobile-native-kmp-httpclient-no-timeout` | low | pm-backend | KMP shared Ktor HttpClient has no HttpTimeout |
| `code-review-mobile-native-kmp-cancellation-swallowed` | low | pm-backend | KMP repositories swallow CancellationException |
| `code-review-mobile-native-kmp-portfolio-analytics-caps-100` | low | pm-backend | getPortfolioAnalytics() truncates realtor portfolio at 100 listings |
| `code-review-mobile-native-kmp-portfolio-analytics-unbounded-fanout-retry1` | low | pm-backend | getPortfolioAnalytics() unbounded fan-out [retry 1/2] |
| `code-review-mobile-native-kmp-ssoservice-untested` | low | pm-qa | KMP SsoService has zero direct tests |
| `code-review-mobile-native-kmp-create-listing-not-wired` | low | pm-backend | KMP CreateListingScreen onSubmit is NotImplementedError stub |
| `code-review-mobile-rn-meter-reading-no-numeric-validation` | low | pm-backend | RN meter-reading input accepts non-numeric |
| `code-review-reality-server-dup-url-validator` | low | pm-tech-lead | reality-server duplicated URL-validation helper — merge into shared crate |
| `refactor-repeated-churn-integrations-webhook-retry1` | low | pm-tech-lead | api-server routes/integrations/webhook.rs repeated-churn [retry 1/2] |
