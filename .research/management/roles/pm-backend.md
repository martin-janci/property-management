# pm-backend — 2026-07-30

**Summary:** Backend churn this window is dominated by layout (scheduler.rs, layout/tenant.rs, layout/admin.rs) plus dispute-KPI wiring — most correctness fixes landed cleanly, but two of the freshly-merged PRs shipped with same-org logic gaps that reviewers immediately flagged (#2573 DELETE-by-file-key, #2575 /disputes/kpis window ordering). Backend security posture is otherwise healthy: PR #2567 closed the scheduler global-read RLS GUC leak, and PR #2478 hardened layout authz + publish TOCTOU + webhook replay.

## Next actions
1. **[high]** Fix #2573 — add same-org reference-count / active-attachment guard in DELETE /documents/by-file-key before the endpoint is wired from any client. DoD: reference-check integration test covering (a) same-org still-referenced → 409/reject, (b) same-org orphan → 204, (c) cross-tenant → 404.
2. **[medium]** Fix #2575 — add window_start <= window_end validation on GET /disputes/kpis; un-quarantine the KPI test and re-enable it in the default suite. DoD: 400 on inverted window; test in default suite and green.
3. **[medium]** Add regression test for PR #2547 scheduler retention prune (still flagged hotfix-no-test). DoD: api-server sqlx test asserts prune deletes older-than-N rows on schedule.
4. **[medium]** Review + merge the accounting MVP-loop trio (#2555, #2558, #2559) or route reviewer feedback back to the implementer. Dependency: pm-tech-lead. DoD: each PR either merged or explicit review comments block further work.
5. **[low]** Investigate repeated churn on backend/servers/api-server/src/services/scheduler.rs (top churn 2 windows running) — consider extraction of retention/prune jobs to a dedicated module. DoD: design note in .research or issue opened with proposed module split.

## Risks
- **[med prob / high impact]** DELETE-by-file-key (#2571 → #2573) is a data-loss surface: a same-org active reference could be deleted, orphaning a live document row against its S3 object. Mitigation: reference-count guard before delete; do not wire from any client until the guard is in and tested.
- **[med prob / med impact]** Dispute KPI endpoint (#2572 → #2575) is quarantined-test-only in main; a shape regression could ship undetected until reporting starts consuming it. Mitigation: un-quarantine the KPIs test; add window validation; consider adding a second test asserting the reporting-consumer contract.

## Open questions
- Is the accounting MVP-loop backend surface (invoice PDF render, PAY-by-square QR, sent/cancelled lifecycle) hitting the same test-quarantine pattern as the dispute KPI endpoint?

## Decisions needed
- Standard: a hotfix that ships without a regression test needs an explicit follow-up issue at merge time (not discovered a run later) — owner: pm-tech-lead.
