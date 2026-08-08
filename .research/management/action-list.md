# Action list

_Generated: 2026-08-08T12:26:01Z_

| Status | Priority | ID | Action | Owner | Depends |
|--------|----------|----|--------|-------|---------|
| open | high | `code-review-api-core-auth-mfa-fail-open` | Fix MFA fail-open on transient DB error in login handler (routes/auth/mod.rs:388) | pm-security | — |
| open | high | `pm-qa-inquiry-notifier-fanout-test` | Add notifier/fanout assertion test for anonymous inquiries now routed through InquiriesHandler (#2719) | pm-qa | — |
| open | high | `pm-qa-vote-scheduler-notified-at-regression-test` | Add regression test for vote scheduler notified_at watermark: simulate dispatch failure, assert started_notified_at/clos | pm-qa | — |
| open | medium | `pm-qa-accounting-trio-review-decision` | Get reviewer engagement or explicit defer/close decision on #2555/#2558/#2559 (accounting MVP-loop, 10+ days stalled) | pm-qa | — |
| open | medium | `pm-qa-ssrf-toctou-regression-test` | Confirm/name the DNS-rebinding TOCTOU regression test tied to #2710/#2703 (not just general SSRF allow/deny unit tests) | pm-qa | — |
| open | low | `code-review-api-core-workflow-cond-parse-failopen` | backend/servers/api-server/src/services/workflow_executor.rs:459-489 — evaluate_conditions() FAILS OPEN on an unparseabl | pm-backend | — |
| open | low | `code-review-mobile-native-kmp-portfolio-analytics-caps-100` | mobile-native-kmp: getPortfolioAnalytics() truncates realtor portfolio at 100 listings — dashboard under-reports on larg | pm-backend | — |
| open | low | `pm-qa-ci-migration-collision-check` | Add CI check that flags duplicate/out-of-order migration numbers before merge (manual 00228->00229 renumbering required  | pm-qa | — |
| open | low | `refactor-churn-hotspot-api-server-layout-tenant-2026-07-30-retry1` | Churn hotspot: backend/servers/api-server/src/routes/layout/tenant.rs — 262 lines this window (PR #2478) [retry 1/2 of f | pm-tech-lead | — |
| in-progress | medium | `gh-issue-2703` | Security: SSRF DNS-rebinding TOCTOU in workflow action api_call.rs — anti-rebinding gate is bypassable (Closes #2703) | pm-tech-lead | — |
| in-progress | low | `code-review-reality-web-listingform-no-i18n` | reality-web ListingForm hardcodes English throughout a next-intl sk/cs/de/en app | pm-backend | — |
| in-progress | low | `dx-fixme-admin-web-mobile-config-patch-endpoint-retry1` | admin-web mobile-config Save flow blocked: PATCH /api/v1/admin/mobile-config endpoint missing [retry 1/2 of failed dx-fi | pm-devops | — |
| in-progress | low | `dx-fixme-admin-web-platform-settings-patch-endpoint-retry1` | admin-web platform-settings Save blocked: PATCH /api/v1/platform-admin/settings endpoint missing [retry 1/2 of failed dx | pm-devops | — |
| in-progress | low | `refactor-churn-hotspot-api-server-auth-2026-07-31-retry1` | Churn hotspot: backend/servers/api-server/src/routes/auth.rs — 2950 lines this window (runs_seen=5, no refactor PR yet)  | pm-tech-lead | — |
