# PPT Action List

_Generated: 2026-08-09 · 13 items · 6 open · 7 done · 0 in-progress_

| Priority | Status | Owner | ID | Action | Dep | Source |
|---|---|---|---|---|---|---|
| high | open | pm-backend | pm-scrum-master-close-2573-blocks-84-1-2026-08-09 | Close #2573 (DELETE-by-file-key reference-check gap) — hard blocker on 84-1 direct-to-S3 upload wiring; #2573 must land ... | none | pm-scrum-master 2026-08-09 |
| medium | open | pm-tech-lead | pm-qa-inline-cfgtest-heuristic-fix-2026-08-09 | Widen routine hotfix-no-test heuristic to count inline #[cfg(test)] mod blocks (grep '#[cfg(test)]' or '#[tokio::test]'/... | none | pm-qa 2026-08-09 (rotating role) |
| medium | open | pm-security | pm-qa-layout-webhook-replay-guard-test-2026-08-09 | Add nonce+timestamp replay-guard regression test to layout webhook — PR #2718 pinned body-binding only; #2485 replay win... | none | pm-qa 2026-08-09 |
| medium | done | pm-tech-lead | gh-issue-2703 | Security: SSRF DNS-rebinding TOCTOU in workflow action api_call.rs — anti-rebinding gate is bypassable (Closes #2703) | none | dispatcher-issue-ingest 2026-08-07T08:10:30Z (#2703) |
| medium | done | pm-security | code-review-api-handlers-community-unauthenticated-reads-retry2 | SECURITY: community.rs get_group/list_posts/get_item run unauthenticated — anonymous cross-tenant read [retry 2/2 of fai... | none | dispatcher-retry-remint 2026-08-08T16:05:19Z (retry_of=code-... |
| medium | done | pm-data | data-announcement-fanout-metric-2026-07-23-retry2 | Instrument announcement fan-out with delivered/read/ack metrics per targeting scope; also feed #2484 real-SQL integratio... | none | dispatcher-retry-remint 2026-08-08T16:05:19Z (retry_of=data-... |
| low | open | pm-backend | code-review-mobile-native-kmp-portfolio-analytics-caps-100 | mobile-native-kmp: getPortfolioAnalytics() truncates realtor portfolio at 100 listings — dashboard under-reports on larg... | none | dispatcher-backlog-refill 2026-08-04T04:05:19Z (score=2 conf... |
| low | open | pm-backend | code-review-api-core-workflow-cond-parse-failopen | backend/servers/api-server/src/services/workflow_executor.rs:459-489 — evaluate_conditions() FAILS OPEN on an unparseabl... | none | dispatcher-signal-bridge 2026-08-05T22:12:57Z (tier1d signal... |
| low | open | pm-qa | pm-qa-fanout-rls-spotcheck-2026-08-09 | Spot-check #2723 announcement_fanout_metrics_tests.rs — verify the RLS visibility predicate itself (not only the metric-... | none | pm-qa 2026-08-09 |
| low | done | pm-backend | code-review-reality-web-listingform-no-i18n | reality-web ListingForm hardcodes English throughout a next-intl sk/cs/de/en app | none | dispatcher-backlog-refill 2026-08-07T04:07:19Z (score=1 conf... |
| low | done | pm-devops | dx-fixme-admin-web-mobile-config-patch-endpoint-retry1 | admin-web mobile-config Save flow blocked: PATCH /api/v1/admin/mobile-config endpoint missing [retry 1/2 of failed dx-fi... | none | dispatcher-retry-remint 2026-08-08T04:07:24Z (retry_of=dx-fi... |
| low | done | pm-devops | dx-fixme-admin-web-platform-settings-patch-endpoint-retry1 | admin-web platform-settings Save blocked: PATCH /api/v1/platform-admin/settings endpoint missing [retry 1/2 of failed dx... | none | dispatcher-retry-remint 2026-08-08T04:07:24Z (retry_of=dx-fi... |
| low | done | pm-tech-lead | refactor-churn-hotspot-api-server-auth-2026-07-31-retry1 | Churn hotspot: backend/servers/api-server/src/routes/auth.rs — 2950 lines this window (runs_seen=5, no refactor PR yet) ... | none | dispatcher-retry-remint 2026-08-08T04:07:24Z (retry_of=refac... |
