# PPT Action List

_Generated: 2026-08-13 · 13 items · 13 open (target buffer: 36)_

| Priority | Status | Owner | ID | Action | Dep | Source |
|---|---|---|---|---|---|---|
| high | open | pm-qa | pm-qa-triage-pr-2684-quarantine | Triage PR #2684 (auto-impl/code-review-api-core-workflow-cond-parse-failopen): identify the test-shard 1-4 failure as... | none | pm-analysis 2026-08-13 |
| high | open | pm-scrum-master | pm-scrum-master-fix-dispatcher-archive-oversize-2743 | Fix dispatcher archive-oversize regression (issue #2743): shard or prune .research/management/*-archive.json (~638KiB... | none | pm-analysis 2026-08-13 |
| high | open | pm-scrum-master | pm-scrum-master-fix-retry-remint-ghost-dedup | Fix retry-remint.sh dedup so it doesn't re-mint tasks (e.g. code-review-api-handlers-voice-webhook-default-secret) al... | none | pm-analysis 2026-08-13 |
| high | open | pm-scrum-master | pm-scrum-master-story-84-1-s3-presigned-urls-frontend | Wire ppt-web direct-to-S3 upload flow to POST /documents/upload-url (story 84-1-s3-presigned-urls, mvp, partial) | none | pm-analysis 2026-08-13 |
| high | open | pm-scrum-master | pm-scrum-master-story-84-2-signer-page | Build the signer-facing document-sign page in ppt-web (story 84-2-esignature-email, mvp, partial) — scope tightly, pr... | none | pm-analysis 2026-08-13 |
| medium | open | pm-scrum-master | pm-scrum-master-kmp-alt-landing-path | Find an alternate landing path for the 4 structurally-unlandable KMP items sitting in the claim buffer (cloud runner ... | none | pm-analysis 2026-08-13 |
| medium | open | pm-scrum-master | pm-scrum-master-reconcile-epic-status-sprint-yaml | Reconcile epic-level status/story-count fields in sprint-status.yaml (epic-6, epic-7a, epic-10b, epic-80) against the... | none | pm-analysis 2026-08-13 |
| low | open | pm-devops | closed-not-merged-pr-2705 | closed-not-merged: PR #2705 dx-cnm-pr-2385-retry2 (rust-toolchain 1.94.1→1.100.0) — second retry closed unmerged, inv... | none | dispatcher-backlog-refill 2026-08-13T00:24:37Z (score=1 conf=medium vector=dx) |
| low | open | pm-backend | code-review-api-core-workflow-cond-parse-failopen | backend/servers/api-server/src/services/workflow_executor.rs:459-489 — evaluate_conditions() FAILS OPEN on an unparse... | none | dispatcher-signal-bridge 2026-08-05T22:12:57Z (tier1d signal; score=2 conf=medium vector=bug seg=api-core) |
| low | open | pm-backend | code-review-mobile-native-kmp-cancellation-swallowed | mobile-native-kmp: shared repositories swallow CancellationException in catch(e: Exception), breaking coroutine cance... | none | dispatcher-backlog-refill 2026-08-11T22:04:10Z (score=2 conf=medium vector=bug) |
| low | open | pm-backend | code-review-mobile-native-kmp-portfolio-analytics-caps-100 | mobile-native-kmp: getPortfolioAnalytics() truncates realtor portfolio at 100 listings — dashboard under-reports on l... | none | dispatcher-backlog-refill 2026-08-04T04:05:19Z (score=2 conf=medium vector=bug) |
| low | open | pm-backend | code-review-mobile-native-kmp-portfolio-analytics-unbounded-fanout-retry1 | mobile-native-kmp: getPortfolioAnalytics() fans out one analytics HTTP request per listing with no concurrency limit ... | none | dispatcher-retry-remint 2026-08-11T06:06:45Z (retry_of=code-review-mobile-native-kmp-portfolio-analytics-unbounded-fanout reason=failed-no-pr cooldown_ok newest_failure=2026-08-04T04:14:00Z) |
| low | open | pm-qa | code-review-mobile-native-kmp-ssoservice-untested | mobile-native-kmp: SsoService (deep-link token exchange, login, password reset, session restore) has zero direct tests | none | dispatcher-backlog-refill 2026-08-11T22:04:10Z (score=1 conf=medium vector=test-gap) |
