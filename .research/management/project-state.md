# PPT Project State

_Generated: 2026-10-25 — routine Phase 1.6 upkeep (pm-backend rotation slot; pm-cursor idx 1 -> 2) + pm-scrum-master always-on. Coverage `scan_kind=upkeep`; coverage_cursor idx 11 -> 12 (epic-8a Basic Notification Preferences re-checked, all 3 stories still done, no PR in the 2026-10-23..2026-10-25 window touched notification surfaces). 2-day lag since 2026-10-23 run._

## Delta since last routine run (2026-10-23)

- **8 PRs merged** (fat window): #3144 (gradient-color sanitize), #3145 (push-fanout surface errors — hotfix WITHOUT test), #3146 (reality-server PRICE_MAP_CACHE eviction), #3147 (migration-import storage error handling), #3148 (ammonia RUSTSEC guard), #3150 (AML/DSA routing fix), #3151 (href scheme XSS allowlist), #3156 (shared bearerAuthHeader refactor).
- **5 issues closed**: #3132 (by #3146), #3136 (by #3147), #3143 (by #3148), #3149 (by #3150). #3131 remains OPEN as the critical merge-pipeline-deadlock blocker; #3157 OPEN with #3158 drafted.
- **1 new critical blocker: #3131** — merge-pipeline deadlock. 15 open PRs touched this window, ALL in `review` status. 16 WIP slots consumed by human-gated PRs → **claimable=8/72 (buffer-low)**. Zero queue progress available on an otherwise fat merge window. Root cause: self-approval block on research-dispatcher PRs; no reviewer rotation.
- **PR #3145 red flag**: push-fanout error-surface hotfix shipped WITHOUT a regression test. Repeats the 2026-07-30 pm-backend standard (hotfix needs follow-up at merge time) — enforcement gap.
- **Churn is flat.** The only file touched twice is `frontend/apps/reality-web/src/app/[locale]/agency/[slug]/page.tsx` (both XSS/sanitize fixes). Shared-sanitize-helper debt surfacing.
- **pm-backend rotation note:** pm-backend last ran 2026-07-30 — ~3 months stale. Timely given 3 of 8 merged PRs touch backend (#3145/#3146/#3147).

<sub>Prior upkeep note (archived): 2026-10-23 — pm-tech-lead rotation; coverage_cursor 10 -> 11 (epic-85 re-checked).</sub>

## Executive summary

- **Delivery: 49/49 stories done across 13 epics in coverage.** No status flips this run; `last_checked` refreshed on all 3 epic-8a stories (cursor re-check — negative, no PR touched notification-preference / critical-notifications / WS realtime surfaces).
- **8 PRs merged** in the observation window: a mix of security (gradient/href sanitize, ammonia RUSTSEC guard, AML routing), reliability (push-fanout error surfacing, PRICE_MAP_CACHE eviction, migration-import error path), and shared-code hygiene (bearerAuthHeader refactor).
- **But the merge-queue is deadlocked.** #3131 (buffer-low 8/72 claimable, 16 WIP slots on human-gated PRs) is the ONE blocker that matters today — nothing else can land until it clears.
- **Prior merge-queue gates:** cargo-deny (#3124) effectively cleared by #3148 (ammonia); #2966 swagger-ui egress still carried; #2652 KMP cloud builds still carried.
- **Hotfix-without-test regression**: #3145 is a repeat of the 2026-07-30 pattern — a new PM decision is overdue to enforce "any `fix(` / `hotfix` commit must modify or add a test file" at CI level.

## Sprint progress

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · **epics_done = 3/5** at sprint level (sprint-status.yaml); coverage-level all extended epics are complete.

| Epic | Sprint status | Coverage status | Shipped PRs this window |
|---|---|---|---|
| 6 — Announcements & Communication | in-progress (3/6)† | 6/6 done | — |
| 7A — Basic Document Management | in-progress (2/5)† | 5/5 done | — |
| 8A — Basic Notification Preferences | done (3/3) | 3/3 done · **last_checked 2026-10-25 (cursor re-check)** | — |
| 10A — OAuth Provider Foundation | done (3/3) | 3/3 done | — |
| 10B — Platform Administration | in-progress (7/7)† | 7/7 done | — |
| 80 — Dispute Resolution | partial (1/3)† | 3/3 done | — |
| 79 — API client integration | (extended) | 4/4 done | #3156 (shared bearerAuthHeader refactor — likely @ppt/api-client) |
| 82 — iOS Reality Portal (SwiftUI) | (extended) | 5/5 done | — |
| 83 — Portal Integrations | (extended) | 3/3 done | — |
| 84 — Documents & e-signature | (extended) | 5/5 done | #3147 (migration-import storage error handling — adjacent to 84-x doc-import flow) |
| 85 — Build/CI configuration | (extended) | 2/2 done | #3148 (ammonia RUSTSEC guard — cargo-deny hygiene) |
| 9 — 2FA/TOTP | (extended) | 1/1 done | — |
| Cross-cutting | — | — | #3144, #3145, #3146, #3150, #3151 (reality-web/reality-server + AML/DSA) |

† sprint-status.yaml drift against coverage.json — no action required this run; deep scan will reconcile.

## Shipped since last run (8 PRs merged)

- **#3144** — reality-web agency page: gradient-color sanitize (XSS surface narrowing)
- **#3145** — api-server push-fanout: surface Redis publish errors instead of silent swallow (**hotfix, no regression test — follow-up queued**)
- **#3146** — reality-server PRICE_MAP_CACHE eviction (closes unbounded-growth DoS vector; closes #3132)
- **#3147** — migration-import storage error handling (closes #3136)
- **#3148** — ammonia RUSTSEC guard (closes #3143; effectively clears the carried cargo-deny gate from the 2026-10-19 run)
- **#3150** — AML/DSA routing fix (closes #3149)
- **#3151** — reality-web agency page: href-scheme XSS allowlist
- **#3156** — @ppt/api-client shared `bearerAuthHeader` refactor (unifies the triplicated auth-header path — closes the long-carried #3006-class refactor-with-security-overtones)

## Blockers (1 new critical, 2 carried)

- **#3131 (NEW, high)** — merge-pipeline deadlock: 16/16 WIP slots on human-gated PRs in review → claimable=8/72 (buffer-low). Blocks every new dispatcher claim until reviewer rotation lands. Owner: pm-tech-lead.
- **#2966** — api-server utoipa-swagger-ui egress 403 (carried, high). Blocks backend cloud builds. Owner: pm-devops.
- **#2652** — mobile-native/KMP cloud builds blocked. Owner: pm-devops.
- **(Cleared this run)** — cargo-deny RUSTSEC-2026-0368/0369 effectively superseded by #3148 ammonia guard. Confirm cleanup of the exception row.

## What's next (top 5 from the roadmap)

1. **Unblock #3131 merge-pipeline deadlock** — define self-approval exception for research-dispatcher PRs OR rotate human reviewers — owner: pm-tech-lead — **gates every new dispatcher claim**.
2. **Clear review backlog for 15 stuck PRs** (#3024, #3028, #3070, #3072, #3074, #3075, #3107, #3108, #3111, #3120, #3152, #3153, #3154, #3155, #3158) — bulk-assign reviewers — owner: pm-tech-lead — depends on #3131.
3. **Backfill regression test for PR #3145** push-fanout error-surface hotfix — owner: pm-backend — business-critical path.
4. **Fast-lane PR #3158** (moderation sub-paths follow-up for #3157) — owner: pm-backend — small scope, unblocks CRS adjacency.
5. **Unblock #2966 swagger-ui egress** — owner: pm-devops — carried; still gates backend cloud builds.

## Role focus today: pm-backend (+ pm-scrum-master always-on)

- **pm-backend** (3-month stale rotation): Backend had 3 touching merges and one of them (#3145) shipped without a test — exactly the 2026-07-30 standard this role codified. Also surfaced: shared-sanitize-helper debt (agency/[slug]/page.tsx twice), PRICE_MAP_CACHE-class audit opportunity, migration-import retry audit question. Six concrete actions queued.
- **pm-scrum-master**: Fat merge window (8 PRs, 5 issues closed) masking a merge-pipeline deadlock (#3131). Review-queue aging is now the top delivery risk — 15 PRs sitting in `review` means a rebase-cost spiral starts next run if we don't clear it.

## Coverage (upkeep this run — 2026-10-25)

- **`coverage.json` refreshed via mechanical upkeep** — `scan_kind=upkeep`, `generated=2026-10-25T04:00:00Z`, no re-scan.
- **Epic re-check: epic-8a Basic Notification Preferences (cursor idx 11)** — all 3 stories (8a-1 channel-toggles / 8a-2 critical-override / 8a-3 preference-sync) still `done`; no PR in the 2026-10-23..2026-10-25 window touched notification-preference / critical-notifications / WS realtime surfaces. `last_checked = 2026-10-25` stamped on all 3. Negative-check evidence appended on each.
- **Merged-PR evidence**: no coverage story cleanly maps to #3144-#3158 (closest-adjacency is #3147 to epic-84 doc-import, #3156 to epic-79 api-client — both already done; no status flip). PR #3146 (PRICE_MAP_CACHE) closes a backlog-tracked risk, not a coverage story.
- **`coverage_cursor` advances 11 -> 12** (epic-8a -> epic-9 next run).
