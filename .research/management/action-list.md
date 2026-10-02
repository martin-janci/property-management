# Action list

_Generated: 2026-10-02T02:35:22Z — regenerated from `action-list.json` (routine Phase 1.6 upkeep, pm-data rotation)._

35 open items (1 done, 1 completed this run: `gh-issue-3006` → PR #3010 merged). New this run: 7 pm-analysis actions (2 scrum-master + 5 pm-data).

| ID | Priority | Owner | Action |
|---|---|---|---|
| `pm-devops-unblock-api-server-utoipa-swagger-egress` | high | pm-devops | Unblock utoipa-swagger-ui crates.io / egress for api-server in cloud runner (issue #2966) — every backend-touching dispatcher task is CLOUD-BUILD-BLOCKED; this is the single largest buffer-starvation lever and gates all 3 IDOR retries (#2944/#2945/#2946) |
| `gh-issue-2946-retry2` | high | pm-security | [CLOUD-BUILD-BLOCKED: backend api-server unbuildable in cloud runner (swagger-ui egress 403, #2966); tracked by open GitHub issue] Cross-tenant IDOR (read+write): portfolio_analytics property metrics computed org_id then discarded (Closes #2946) [retry 2/2 of failed gh-issue-2946] |
| `gh-issue-2944-retry1` | high | pm-security | [CLOUD-BUILD-BLOCKED: backend api-server unbuildable in cloud runner (swagger-ui egress 403, #2966); tracked by open GitHub issue] Cross-tenant IDOR: violation comments/evidence/payments read without org scoping (+ internal-notes privilege leak) (Closes #2944) [retry 1/2 of failed gh-issue-2944] |
| `gh-issue-2945-retry1` | high | pm-security | [CLOUD-BUILD-BLOCKED: backend api-server unbuildable in cloud runner (swagger-ui egress 403, #2966); tracked by open GitHub issue] Cross-tenant IDOR (read+write): portfolio_properties handlers missing org verification (Closes #2945) [retry 1/2 of failed gh-issue-2945] |
| `pm-scrum-master-stalled-accounting-pay-by-square-2559` | medium | pm-tech-lead | Stalled 64d: PR #2559 (PAY by Square accounting slice) needs reviewer slot — one of two non-dependabot >60d PRs; part of the accounting MVP trio that has been reviewer-starved for 2 months |
| `pm-scrum-master-stalled-invoice-lifecycle-2555` | medium | pm-tech-lead | Stalled 64d: PR #2555 (invoice lifecycle accounting slice) needs reviewer slot — companion to #2559; both are the aging accounting MVP trio tracked since 2026-07-30 |
| `pm-devops-unblock-mobile-native-cloud-builds` | high | pm-devops | Unblock mobile-native/KMP builds in the cloud runner (issue #2652) — still 5+ open backlog items structurally unclaimable in cloud; now compounded by #2966 so claimable=9/72 (buffer-low) |
| `pm-devops-unblock-mobile-rn-jest-expo-version-rot` | medium | pm-devops | Resolve mobile/React Native jest-expo version rot (#2951) — RN mobile unit tests failing on cloud runner; blocks mobile RN code-review items from landing |
| `code-review-ppt-web-core-authed-roots-session-leak` | medium | pm-backend | logout() AUTHED_QUERY_KEY_ROOTS allow-list omits mounted route groups; caches leak across users on shared workstation |
| `code-review-mobile-native-kmp-inquiries-response-contract` | medium | pm-backend | mobile-native-kmp InquiriesResponse required page_size mismatches reality-server `limit` — MissingFieldException on every real /inquiries + /realtors/inquiries call |
| `code-review-mobile-native-kmp-httpclient-no-timeout` | low | pm-backend | mobile-native-kmp shared Ktor HttpClient installs no HttpTimeout — every suspend API call can hang indefinitely on Android + iOS |
| `code-review-mobile-native-kmp-cancellation-swallowed` | low | pm-backend | mobile-native-kmp: shared repositories swallow CancellationException in catch(e: Exception), breaking coroutine cancellation and showing spurious errors |
| `code-review-mobile-native-kmp-portfolio-analytics-caps-100` | low | pm-backend | mobile-native-kmp: getPortfolioAnalytics() truncates realtor portfolio at 100 listings — dashboard under-reports on large portfolios |
| `code-review-mobile-native-kmp-portfolio-analytics-unbounded-fanout-retry1` | low | pm-backend | mobile-native-kmp: getPortfolioAnalytics() fans out one analytics HTTP request per listing with no concurrency limit — up to 100 parallel GETs from a mobile device [retry 1/2] |
| `code-review-mobile-native-kmp-ssoservice-untested` | low | pm-qa | mobile-native-kmp: SsoService (deep-link token exchange, login, password reset, session restore) has zero direct tests |
| `code-review-mobile-native-kmp-create-listing-not-wired` | low | pm-backend | KMP realtor CreateListingScreen onSubmit is a NotImplementedError stub — form data discarded |
| `code-review-mobile-rn-meter-reading-no-numeric-validation` | low | pm-backend | Mobile RN meter-reading input accepts non-numeric — silent write of garbage values |
| `code-review-reality-server-dup-url-validator` | low | pm-tech-lead | reality-server carries a duplicated URL-validation helper — merge into shared crate |
| `refactor-repeated-churn-integrations-webhook-retry1` | low | pm-tech-lead | [cloud-build-blocked: api-server unbuildable in cloud runner (utoipa-swagger-ui egress 403, #2966) AND no ppt-bridge MCP configured — structurally unlandable in this runner; not implemented] backend api-server routes/integrations/webhook.rs — repeated-churn (runs_seen=3, 2799 lines this window) [retry 1/2] |
| `code-review-ppt-web-api-client-auth-fetch-triplication` | low | pm-tech-lead | Triplicated (and already divergent) auth-header + MFA-retry logic in @ppt/api-client fetch primitives |
| `code-review-ppt-web-ui-workflow-automation-toasts-hardcoded-english` | low | pm-tech-lead | workflow-automation pages emit English-literal toast titles while rest of file uses i18n |
| `screen-map-drift-pr-2994-ppt` | low | pm-qa | ppt-web route refactor (PR #2994) landed without touching docs/screens/ppt |
| `screen-map-drift-pr-2995-ppt` | low | pm-qa | ppt-web Home CTA refactor (PR #2995) landed without touching docs/screens/ppt |
| `screen-map-drift-pr-3002-ppt` | low | pm-qa | ppt-web manager-dashboard role-gate (PR #3002) landed without touching docs/screens/ppt |
| `churn-hotspot-frontend/apps/ppt-web/src/features/workflow-automation/pages/CreateRulePage.tsx` | low | pm-tech-lead | ppt-web workflow-automation CreateRulePage.tsx — 4 commits in 14d (new hotspot) |
| `churn-hotspot-frontend/apps/ppt-web/src/features/workflow-automation/pages/TemplateLibraryPage.tsx` | low | pm-tech-lead | ppt-web workflow-automation TemplateLibraryPage.tsx — 3 commits in 14d (new hotspot) |
| `churn-hotspot-frontend/apps/ppt-web/src/routes/groups/core.tsx` | low | pm-tech-lead | ppt-web src/routes/groups/core.tsx — 4 commits in last 14d (new hotspot) |
| `gh-issue-3012` | high | pm-tech-lead | dispatcher: `git stash` is shared across worktrees — parallel implementers contaminate each other (Closes #3012) |
| `pm-scrum-master-drain-review-queue-2026-10-02` | high | pm-tech-lead | Drain review queue: 11 tasks sit in review (0 claimed this cycle); prioritize cross-tenant IDOR retries (gh-issue-2944/2945/2946) |
| `pm-scrum-master-triage-dependabot-bulk-2026-10-02` | low | pm-devops | Triage dependabot bulk (#3008/#3007/#2981/#2972/#2962/#2961/#2957) — merge npm/cargo minors; defer KMP/compose until #2652 unblocks |
| `pm-data-wire-emit-sites-declared-kpi-counters` | high | pm-data | Wire emit sites for the 4 declared KPI counters (faults_created_total, votes_cast_total, organizations_created_total, auth_login_total) in api-server/src/observability.rs:272-282 — currently described-but-never-emitted |
| `pm-data-domain-event-catalogue` | high | pm-data | Author canonical domain-event catalogue (docs/data/event-catalog.md) covering fault, vote, payment, booking, dispute, announcement events |
| `pm-data-fault-sla-histogram-metric` | medium | pm-data | Define a fault-SLA metric (histogram of time to triage + resolve, labelled by priority); single shared FaultStatusCount definition reused by support-data + owner/portfolio KPIs |
| `pm-data-metrics-assertion-test-helper` | medium | pm-qa | Add a metrics-assertion test helper to shared test utils; require ≥3 KPI handlers to assert emission |
| `pm-data-stripe-webhook-payment-counters` | medium | pm-data | Add Stripe webhook payment_completed/failed counters with idempotency labels (no PII in labels) |
