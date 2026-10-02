# PPT Project State

_Generated: 2026-10-02 — routine Phase 1.6 upkeep (pm-data rotation slot; pm-cursor idx 6 → 7) + pm-scrum-master always-on. Coverage `scan_kind=upkeep`; coverage_cursor idx 8 → 9 (epic-83 re-checked, no material change; advances to epic-84). 21h since last routine run — on schedule._

## Executive summary

- **Delivery: 49/49 stories done across 13 epics in coverage.** No status flips this cycle (3 merged PRs touched already-done surfaces). Sprint-status drift (epics 6/7A/10B/80 flagged in-progress/partial) persists against the deeper coverage.json truth.
- **3 PRs merged** since 2026-10-01 cursor: #3009 (RegistryRulesPage save toast — closes a 2026-09-26 code-review-finding), #3010 (api-client auth-header + MFA-retry unified; closes #3006 follow-up), #3011 (ppt-web facilities fetch error-state UI — closes a 2026-10-01 code-review-finding).
- **Pipeline wedged in review.** Dispatcher's 2026-10-02 00:36 cycle: 0 claimed / 0 reviewed / 0 merge-attempts last cycle; **11/11 active tasks sit in `review`** (prior cycle merged 3). The bottleneck has shifted from implementers → reviewers.
- **Buffer-low alarm continues: `claimable=15/22`** (floor 36). Dropped from 72 total to 22 total open tasks (prior refills consumed), with 7 dep-blocked remaining. The payload that fired this routine run was explicitly `buffer-low: claimable=15/72 — refill planner`.
- **Standing infra blockers unchanged:** #2966 (utoipa-swagger-ui egress), #2652 (KMP cloud builds), #2951 (jest-expo rot). All three continue to park security + mobile PRs.
- **New issue of note: #3012 (OPEN, labelled bug/follow-up)** — "dispatcher: `git stash` is shared across worktrees — parallel implementers contaminate each other." Already ingested into action-list as `gh-issue-3012`.
- **Tier-1d demand-driven review** (dispatcher, this morning) found three high-impact reality-web gaps: `/sell` wizard discards every submission (score 3, high), `/report` page doesn't POST (score 2, medium), `/profile` renders MOCK_* data (score 2, medium). Promoted this run — see routine brief.

## Sprint progress

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · **epics_done = 3/5** at sprint-status level; coverage-level all extended epics are complete.

| Epic | Sprint status | Coverage status | Shipped PRs this window |
|---|---|---|---|
| 6 — Announcements & Communication | in-progress (3/6)† | 6/6 done | — |
| 7A — Basic Document Management | in-progress (2/5)† | 5/5 done | — |
| 8A — Basic Notification Preferences | done (3/3) | 3/3 done | — |
| 10A — OAuth Provider Foundation | done (3/3) | 3/3 done | — |
| 10B — Platform Administration | in-progress (7/7)† | 7/7 done | — |
| 80 — Dispute Resolution | partial (1/3)† | 3/3 done | — |
| 79 — API client integration | (extended) | 4/4 done | #3010 (auth-header + MFA-retry unified) |
| 83 — Portal Integrations | (extended) | 3/3 done · **last_checked 2026-10-02 (cursor re-check)** | — |
| Cross-cutting | — | — | #3009 (RegistryRulesPage toast — UC-57 polish), #3011 (facilities error UI — closes code-review-finding) |

† sprint-status.yaml drift against coverage.json — no action required this run; deep scan will reconcile.

## Shipped since last run (3 PRs merged since #3005)

- **#3009** — fix(UC-57): surface RegistryRulesPage save success + failure via toast. Closes `code-review-ppt-web-ui-registry-rules-save-silent-fail` (promoted 2026-09-26). Touches 1 page + 1 test + 6 locale json. Vector: bug · confidence high.
- **#3010** — refactor(api-client): unify triplicated auth-header + MFA-retry logic. Closes issue #3006 (post-merge follow-up on PR #3004). Collapses 3 fetch primitives onto a shared `authenticatedFetchJson` with centralised MFA-retry. Vector: refactor · confidence high.
- **#3011** — code-review-ppt-web-ui-facilities-fetch-no-error-ui: distinct error state for facilities fetch failure. Closes a code-review-finding from the 2026-10-01 slice. Adds a dedicated `errorState.test.tsx`. Vector: bug · confidence high.

## What's next (top 5 from action-list)

1. **[high]** `pm-devops-unblock-api-server-utoipa-swagger-egress` — gates every backend security PR.
2. **[high]** `gh-issue-2946-retry2` — cross-tenant IDOR (portfolio_analytics), blocked behind #2966.
3. **[high]** `gh-issue-2944-retry1` — cross-tenant IDOR (violation comments/evidence/payments + internal-notes leak), blocked behind #2966.
4. **[high]** `pm-scrum-master-drain-review-queue-2026-10-02` — new this run: 11 tasks sit in review with 0 claimed last cycle; drain before refilling.
5. **[high]** `pm-data-wire-emit-sites-declared-kpi-counters` — new this run: 4 declared KPI counters never emit → dashboards silently zero.

## Role focus today

**pm-data** (rotation slot 6 — 71d stale, longest in queue; this run's refresher) + **pm-scrum-master** (always-on).

- **pm-data:** Business KPI counters declared in `backend/servers/api-server/src/observability.rs:272-282` have **zero emit sites** — grep across the repo confirms it. No canonical event catalogue exists. Only reality-server listing-view is genuinely instrumented. Added 5 new action-list items (`pm-data-*`) and 3 new risks.
- **pm-scrum-master:** Review queue is the throughput bottleneck now, not implementer capacity. Buffer-low is a downstream symptom of 11 unreviewed-and-stacking tasks. Added 2 new coordination actions (drain-review-queue, triage-dependabot-bulk) and 3 new risks (IDORs-unpatched, stash-contamination, review-wedge).

## Carried-over blockers

- **#2966 utoipa-swagger-ui egress 403** — backend PRs unbuildable in cloud. Gates `gh-issue-2944/2945/2946` and `refactor-repeated-churn-integrations-webhook-retry1`.
- **#2652 KMP cloud builds** — all 7 open `code-review-mobile-native-kmp-*` items unclaimable in cloud; also parks dependabot kotlin/ktor/compose group.
- **#2951 jest-expo version rot** — RN mobile unit tests fail on cloud; blocks `code-review-mobile-rn-meter-reading-no-numeric-validation`.
- **#3012 git-stash sharing** — new this cycle; may corrupt parallel implementer work. Decision needed before next multi-claim batch.

## Decisions needed

- Vendor vs allowlist vs image-cache for #2966 — owner: pm-devops.
- Pause new claims until #3012 stash fix lands? — owner: pm-devops.
- Raise reviewer capacity or auto-merge low-risk frontend PRs — owner: pm-tech-lead.
- Reconcile sprint-status drift (epics 6/7A/10B/80) via deep scan — owner: pm-scrum-master.
- Analytics platform (Prometheus-only vs product analytics SDK) — owner: pm-tech-lead. (pm-data)
- Minimum-analytics DoD for stories: blocking gate or advisory — owner: pm-scrum-master. (pm-data)
