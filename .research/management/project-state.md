# PPT Project State

_Generated: 2026-07-26 — daily PM rotation (Scrum Master + pm-tech-lead; routine Phase 1.6 lightweight run). Coverage `scan_kind=upkeep`; pm_cursor idx 0 → 1 (pm-tech-lead ran; pm-backend next), coverage_cursor idx 2 → 3 (epic-6 re-checked, advances to epic-79)._

## Executive summary

- **Scrum Master:** Since the last run (~50h window), 35 PRs landed dominated by a large test-restoration wave plus security/scheduler fixes and new analytics instrumentation; the current sprint (Epic 6/7A/8A/10A) has 8A and 10A done, 6 and 7A still in-progress, with 3 PRs sitting in review and one (#2482) stuck on a rebase failure needing manual intervention.
- **Tech Lead:** Architecture is functionally converging (47/49 stories done) but three route/integration files (auth.rs, reports.rs, booking/mod.rs — all 3000+ lines) are absorbing repeat churn without a refactor decision, and this window's test-restoration wave plus the #2547 scheduler test-gap point to a systemic pattern of shipping without durable test coverage for background jobs and god-files.

- **Delivery snapshot:** stories done=47, partial=2, not-started=0 (of 49 total across 13 epics).
- **Action-list buffer:** 18/36 open items (target buffer keeps dispatcher fed).

## Sprint progress

Current sprint: **Epic 6, 7A, 8A & 10A - Announcements, Documents, Notifications & OAuth** · **epics_done = 2/4** in the sprint frame; extended-scope epics (10B/79/80/81/82/83/84/85/9) rolled into coverage.

## Shipped since last run

- Test-restoration wave: 16 test(api-server) un-quarantine PRs (BIT-558/566/567/570/571/573/574/585/588/609/610/630/647/658)
- fix #2548: IDOR close (security)
- fix #2547: retention-prune scheduler wiring (merged without runtime test coverage)
- fix #2534: orphaned-S3 cleanup
- 3 analytics/instrumentation feat PRs: signup-funnel, listing analytics, OAuth token-usage
- feat: layout-editor extract; feat: mobile layout-cache test
- 4 data-* PRs: fault-KPI-unification x2, audit-oauth, support-audit
- fix #2545: screen-map extractEpics + 3 docs PRs

## What's next (top actions)

- **[high]** Get reviewer sign-off and merge PR #2553 (AuthContext cold-boot init bypassing refreshTokenInternal fix — reviewer_summary still null) — _owner_: pm-scrum-master or pm-tech-lead
- **[high]** Merge already-approved PR #2549 (layout publish/webhook event wiring) to close issue #2532 — _owner_: pm-scrum-master or pm-tech-lead
- **[high]** Land the two in-progress MVP gap-closure tasks: direct-to-S3 upload wiring (84-1) and signer-facing document-sign page (84-2), both owned by pm-frontend and already the highest-ranked coverage gaps — _owner_: pm-frontend
- **[high]** Open a refactor RFC to split auth.rs (2950 lines, 4th repeat-churn cycle - now also touched by draft PR #2553 cold-boot fix) into session/OAuth/MFA submodules before the next auth-adjacent epic lands — _owner_: pm-backend
- **[medium]** Apply the same review to reports.rs (3329 lines, 3rd repeat-churn cycle, growing further via epic-6 SQL-backed dispute KPIs) — _owner_: pm-backend

## Blockers

- **refactor-churn-hotspot-repo-map-md-2026-07-20 (PR #2482, docs/repo-map.md reconciliation)** — 3 rebase attempts failed against dev — disjoint history, no merge-base, 4339-commit tree-wide conflicts; rebase cap reached, needs manual branch recreation off current dev · owner: pm-tech-lead

## Role focus today

- **pm-scrum-master:** always-on synthesis (delivery digest + shipped/next).
- **pm-tech-lead:** rotating role — flagged god-file churn (auth.rs, reports.rs, booking/mod.rs), scheduler test-gap systemic pattern, IDOR recurrence needing structural fix.

