# Action list

_Generated: 2026-09-25T10:50:43Z — regenerated from `action-list.json`._

| ID | Priority | Owner | Action |
|---|---|---|---|
| `gh-issue-2944-retry1` | high | pm-security | [CLOUD-BUILD-BLOCKED: backend api-server unbuildable in cloud runner (swagger-ui egress 403); tracked by open GitHub issue] Cross-tenant IDOR: violation comments/evidence/payments  |
| `gh-issue-2945-retry1` | high | pm-security | [CLOUD-BUILD-BLOCKED: backend api-server unbuildable in cloud runner (swagger-ui egress 403); tracked by open GitHub issue] Cross-tenant IDOR (read+write): portfolio_properties han |
| `pm-devops-unblock-mobile-native-cloud-builds` | high | pm-devops | Unblock mobile-native/KMP builds in the cloud runner (issue #2652) — currently 7/8 open backlog items are structurally unclaimable in cloud, forcing Tier-1d generator kicks every r |
| `pm-scrum-master-clear-needs-human-review-queue-2026-09-25` | high | pm-scrum-master | Clear the 7 PRs on needs-human-review (fast-track security IDOR retries #2977/#2976 ahead of correctness PRs #2969/#2968/#2967 and the older #2902/#2744) — reviewer throughput is t |
| `pm-security-idor-regression-tests-violations-portfolio-2026-09-25` | high | pm-security | Add cross-org IDOR regression tests for the 7 vulnerable handlers on PRs #2977/#2976 before merge — mirror payment_management_tests::list_payments_is_org_scoped_and_counted and dis |
| `code-review-mobile-native-kmp-inquiries-response-contract` | medium | pm-backend | mobile-native-kmp InquiriesResponse required page_size mismatches reality-server `limit` — MissingFieldException on every real /inquiries + /realtors/inquiries call |
| `pm-devops-unblock-utoipa-swagger-ui-cloud-2966-2026-09-25` | medium | pm-devops | Unblock issue #2966 (utoipa-swagger-ui build-script egress in the cloud runner) — vendor or mirror the crate's build-script dependency so api-server verify can run in the sandbox;  |
| `pm-scrum-master-reconcile-sprint-status-epic-rollup-2026-09-25` | medium | pm-scrum-master | Reconcile _bmad-output/implementation-artifacts/sprint-status.yaml — epics 6/7a/10b/80 show status in-progress/partial with stale stories_completed counts even though every listed  |
| `pm-security-audit-discarded-authuser-idor-pattern-2026-09-25` | medium | pm-security | Audit dev tree for the discarded-AuthUser IDOR anti-pattern (handler signatures with '_auth: AuthUser' or '_user: AuthUser' paired with a path-supplied Uuid lookup) across api-serv |
| `code-review-mobile-native-kmp-cancellation-swallowed` | low | pm-backend | mobile-native-kmp: shared repositories swallow CancellationException in catch(e: Exception), breaking coroutine cancellation and showing spurious errors |
| `code-review-mobile-native-kmp-create-listing-not-wired` | low | pm-backend | KMP realtor CreateListingScreen onSubmit is a NotImplementedError stub — form data discarded |
| `code-review-mobile-native-kmp-httpclient-no-timeout` | low | pm-backend | mobile-native-kmp shared Ktor HttpClient installs no HttpTimeout — every suspend API call can hang indefinitely on Android + iOS |
| `code-review-mobile-native-kmp-portfolio-analytics-caps-100` | low | pm-backend | mobile-native-kmp: getPortfolioAnalytics() truncates realtor portfolio at 100 listings — dashboard under-reports on large portfolios |
| `code-review-mobile-native-kmp-portfolio-analytics-unbounded-fanout-retry1` | low | pm-backend | mobile-native-kmp: getPortfolioAnalytics() fans out one analytics HTTP request per listing with no concurrency limit — up to 100 parallel GETs from a mobile device [retry 1/2 of fa |
| `code-review-mobile-native-kmp-ssoservice-untested` | low | pm-qa | mobile-native-kmp: SsoService (deep-link token exchange, login, password reset, session restore) has zero direct tests |
| `screen-map-drift-pr-2894-reality` | low | pm-qa | screen-map-drift: PR #2894 touched reality-web routes without updating docs/scre |
