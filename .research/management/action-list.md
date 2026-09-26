# Action list

_Generated: 2026-09-26T02:44:52Z — regenerated from `action-list.json`._

| ID | Priority | Owner | Action |
|---|---|---|---|
| `pm-devops-unblock-mobile-native-cloud-builds` | high | pm-devops | Unblock mobile-native/KMP builds in the cloud runner (issue #2652) — currently 7/8 open backlog items are structurally unclaimable in cloud, forcing Tier-1d generator kicks every run |
| `gh-issue-2944-retry1` | high | pm-security | [CLOUD-BUILD-BLOCKED: backend api-server unbuildable in cloud runner (swagger-ui egress 403); tracked by open GitHub issue] Cross-tenant IDOR: violation comments/evidence/payments read without org sco |
| `gh-issue-2945-retry1` | high | pm-security | [CLOUD-BUILD-BLOCKED: backend api-server unbuildable in cloud runner (swagger-ui egress 403); tracked by open GitHub issue] Cross-tenant IDOR (read+write): portfolio_properties handlers missing org ve |
| `gh-issue-2978` | high | pm-frontend | ppt-web AI-chat + OCR feature hooks call the API via raw fetch() with no Authorization header (unauthenticated → 401 in prod) + silent delete (Closes #2978) |
| `pm-scrum-master-unblock-api-server-cloud-build-2652` | high | pm-devops | Unblock cloud-runner builds for api-server (swagger-ui egress 403 per issue #2652) so #2944/#2945 retries can complete |
| `pm-security-reconcile-2652-actual-vs-cited` | high | pm-devops | Reconcile issue #2652 body (mobile-native/KMP dl.google.com egress) against the claimed api-server/swagger-ui blocker so the correct root cause is tracked |
| `code-review-mobile-native-kmp-inquiries-response-contract` | medium | pm-backend | mobile-native-kmp InquiriesResponse required page_size mismatches reality-server `limit` — MissingFieldException on every real /inquiries + /realtors/inquiries call |
| `pm-scrum-master-reconcile-stale-epic-counts` | medium | pm-scrum-master | Reconcile stale epic-level completion counts (epic-6, 7a, 10b, 80) in sprint-status.yaml against development_status entries |
| `pm-scrum-master-triage-stalled-2555-2559` | medium | pm-tech-lead | Triage the two 59-day-old stalled review PRs #2555, #2559 (feat(acc)) — merge, close, or re-scope with a decision recorded |
| `pm-security-reopen-2944-2945-issue-state` | medium | pm-security | Re-open or correct the closed state of issues #2944/#2945 given their fixes are still unmerged (draft PRs #2977/#2976) |
| `pm-security-promote-llm-doc-idor-plan` | medium | pm-security | Promote plans/security-llm-doc-idor.md (Epic 64 ai.rs cross-tenant IDOR) to a tracked backlog item — add principal binding + org predicate to publish_description, list_listing_descriptions, get_photo_ |
| `code-review-mobile-native-kmp-portfolio-analytics-caps-100` | low | pm-backend | mobile-native-kmp: getPortfolioAnalytics() truncates realtor portfolio at 100 listings — dashboard under-reports on large portfolios |
| `code-review-mobile-native-kmp-portfolio-analytics-unbounded-fanout-retry1` | low | pm-backend | mobile-native-kmp: getPortfolioAnalytics() fans out one analytics HTTP request per listing with no concurrency limit — up to 100 parallel GETs from a mobile device [retry 1/2 of failed code-review-mob |
| `code-review-mobile-native-kmp-cancellation-swallowed` | low | pm-backend | mobile-native-kmp: shared repositories swallow CancellationException in catch(e: Exception), breaking coroutine cancellation and showing spurious errors |
| `code-review-mobile-native-kmp-ssoservice-untested` | low | pm-qa | mobile-native-kmp: SsoService (deep-link token exchange, login, password reset, session restore) has zero direct tests |
| `code-review-mobile-native-kmp-httpclient-no-timeout` | low | pm-backend | mobile-native-kmp shared Ktor HttpClient installs no HttpTimeout — every suspend API call can hang indefinitely on Android + iOS |
| `code-review-mobile-native-kmp-create-listing-not-wired` | low | pm-backend | KMP realtor CreateListingScreen onSubmit is a NotImplementedError stub — form data discarded |
| `screen-map-drift-pr-2894-reality` | low | pm-qa | screen-map-drift: PR #2894 touched reality-web routes without updating docs/scre |
