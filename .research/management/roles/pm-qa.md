# pm-qa role output — 2026-08-12

## Summary

Backend notification hand-off (QuietHoursDrainWorker.drain_due) is only unit-tested at the pure-decision-function level with no DB-integration coverage, and the just-shipped CSV-formula-injection fix (PR #2731) only covers vote titles — other CSV export surfaces (GDPR export, reports) likely share the same gap. Frontend has two known, still-open test gaps (perfmetrics listener leak, notification-triggers logout purge already fixed but check the pattern) that should be closed out this sprint.

## Next actions

1. **[high]** Add DB-integration test for QuietHoursDrainWorker.drain_due() exercising real held_notifications: partial-channel-failure retry, full-success release, batch_limit boundary across ticks. DoD: New `#[sqlx::test]` in api-server tests/suites asserting `release_at`/`released_at` transitions for held rows under mixed `PipelineResult` outcomes.
2. **[high]** Audit GDPR export (routes/gdpr.rs), reports/mod.rs and market_pricing.rs CSV-generation paths for the same formula-injection vector fixed for vote titles in PR #2731. DoD: every CSV export writing user-authored free text calls `sanitize_csv_cell`; each fixed field has a leading-`=`/`+`/`-`/`@` regression test.
3. **[medium]** Write mount/unmount test coverage for usePerformanceMetrics.ts asserting visibilitychange/load listeners are removed on cleanup and effect doesn't re-run on every inline onReport identity change. Depends on `code-review-ppt-web-core-perfmetrics-listener-leak`.
4. **[medium]** Add a structural regression test that fails when a query-key root used by any @ppt/api-client hook factory is missing from ppt-web `AUTHED_QUERY_KEY_ROOTS` logout allowlist. This is the third missed root in a row (analytics, PR #2650 notifications, PR #2741 notification-triggers).
5. **[medium]** Request test-coverage review on in-review PR #2714 (gh-issue-2612-retry1) before it merges to dev — it touches the same quiet-hours/notification-scheduling surface as the untested drain_due() path.
6. **[low]** Once the offline-sync 4xx-handling fix (PR #2738) lands, add tests asserting 401 (expired token) and 429 (rate-limit) are retried/requeued while genuine permanent 4xx (400/422) are dropped.

## Risks

- **medium-high** — QuietHoursDrainWorker regression could silently drop or infinitely re-hold user notifications in production; no DB-integration fence.
- **high-high** — CSV formula-injection fix is opt-in per call site; sibling exports likely still ship the vulnerability.
- **high-medium** — AUTHED_QUERY_KEY_ROOTS allowlist missed a query root 3× in a row; a 4th regression is likely without a structural guard.
- **medium-low** — perfmetrics fix could land without a regression test.
- **medium-medium** — PR #2714 test coverage unknown (branch not visible in this session).

## Open questions

- What is the actual scope/diff of PR #2714 (gh-issue-2612-retry1)? Not present on any local/origin branch reachable from this checkout.
- Does routes/gdpr.rs's CSV export format include free-text fields (names, messages, addresses) that would need `sanitize_csv_cell`, or purely structured/numeric data?
- Is action-list.json stale relative to dev — three items already merged as PRs #2739/#2740/#2741 and should be closed in the tracker? (This run closes them.)
- Is there an existing owner/cadence for a repo-wide "CSV export sanitization" sweep, or should this be a one-off audit task this sprint?

## Decisions needed

- Block PR #2714 merge pending an explicit test-coverage review of its notification hand-off changes — owner: pm-qa / rust-backend.
- Scope of CSV-injection audit (GDPR export, reports, market pricing): single sprint task or split per-route — owner: pm-backend.
