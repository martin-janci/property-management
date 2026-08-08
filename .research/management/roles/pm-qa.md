# pm-qa — 2026-08-08

## Summary
Epic 6/7A/8A/10A sit at 47/49 stories done with generally solid regression coverage, but two of yesterday's shipped fixes — the vote-scheduler `notified_at` watermark retry and the anonymous-inquiry notifier fanout — have no dedicated regression test verified in the repo. The SSRF TOCTOU fix and the 00228/00229/00230 migration renumbering both check out clean on origin/dev. The 3 stalled accounting-trio PRs (10+ days, no reviewer) remain an unresolved process/release-readiness signal (gh API access blocked for further verification this session).

## Next actions

- **[high]** Add a regression test for the vote scheduler's `notified_at` watermark (`servers/api-server/src/services/scheduler/votes.rs`) that simulates a dispatch failure and asserts `started_notified_at`/`closed_notified_at` stays NULL so the next tick retries. (owner: rust-backend)
  - DoD: new test (`crates/db/tests` or `servers/api-server` tests) covers failure-leaves-NULL and success-stamps-once paths for both watermarks.
- **[high]** Add a notifier/fanout assertion test for anonymous inquiries now routed through `InquiriesHandler` (#2719). (owner: rust-backend)
  - DoD: test in `servers/reality-server/tests/suites/inquiries_*` (or new suite) asserts notifier is invoked for the anonymous-submitter path.
- **[medium]** Confirm/name the explicit DNS-rebinding TOCTOU regression test tied to #2710/#2703, not just general SSRF allow/deny unit tests. (owner: rust-backend)
  - DoD: a test case exists that re-resolves DNS between validation and connect and asserts the connection is blocked.
- **[medium]** Get reviewer engagement (or an explicit defer/close decision) on #2555/#2558/#2559. (owner: PM)
  - DoD: each PR has either a completed review pass or a logged decision to defer out of this release.
- **[low]** Add a CI check that flags duplicate/out-of-order migration numbers before merge, given the manual 00228→00229 renumbering required by #2716. (owner: rust-backend)
  - DoD: CI fails a PR that reuses an already-claimed migration number on the target branch.

## Risks

- **[med prob / high impact]** Scheduler's stamp-only-on-success watermark (votes.rs) has no direct failure-path test; a future refactor could silently break retry/no-duplicate-notification guarantees. Mitigation: add dedicated failure/retry regression test before next scheduler-touching change.
- **[med prob / med impact]** Anonymous-inquiry notifier fanout (#2719) is untested — anonymous inquiries could silently fail to notify or double-notify. Mitigation: add integration test asserting notifier invocation for the anonymous path.
- **[med prob / high impact]** Manual migration renumbering during concurrent same-day merges (#2714/#2716/#2717) worked this time but is a repeatable collision risk with no automated guard. Mitigation: add CI migration-number-uniqueness check.
- **[med prob / med impact]** 3 accounting-trio PRs stalled >10 days with no reviewer engagement — unclear if abandoned, blocked, or simply unstaffed ahead of a release cut. Mitigation: PM to explicitly schedule review or defer/close.

## Open questions

- Is there a test that specifically re-resolves DNS mid-request (TOCTOU) for the SSRF fix, beyond the general allow/deny unit tests in `crates/common/src/url_validation.rs`?
- What is the current review/CI status of #2555/#2558/#2559? (`gh` API access returned 403 in this session; status could not be confirmed directly.)
- Does the vote scheduler have any end-to-end/integration-level test validating watermark durability across multiple scheduler ticks, or only unit-level logic?
- Is migration 00229 (platform_settings) at risk of a second collision from any other still-open PR touching the 00228-00231 range?

## Decisions needed

- Decide whether to block the next release cut until scheduler-watermark and anonymous-inquiry-notifier regression tests land — owner: rust-backend.
- Decide disposition (review, defer, or close) of accounting-trio PRs #2555/#2558/#2559 — owner: PM.
