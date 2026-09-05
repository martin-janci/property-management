# PPT Project State

_Generated: 2026-09-05 — routine Phase 1.6 lightweight upkeep (pm-security rotation slot; 46-day stale slot refreshed) + pm-scrum-master always-on. Coverage `scan_kind=upkeep` (not re-scanned this run); pm_cursor idx 5 → 6 (pm-security → pm-data next), coverage_cursor idx 7 → 8 (epic-82 next). Sprint window 2026-09-01..09-05 shipped **14 PRs** — 9 code-review/refactor/test fixes + 1 accounting feature + 5 dependabot bumps + 1 closed-unmerged (superseded)._

## Executive summary

- **Delivery still at 47/49 stories done, 2 partial** (84-1 direct-to-S3 upload wiring and 84-2 sign page). No status flips this window. UC-ACC-05.9 (invoice PDF) shipped via #2558 — the long-stalled accounting-MVP trio finally cleared its reviewer starvation.
- **14 merged PRs since 2026-09-01 brief:**
  - Backend security: **#2919** (reality-server: reject self-review — agent-review self-review guard, IDOR-adjacent), **#2925** (fix compliance raw DB error leak regression introduced by #2915 → then swept by #2920 → residual re-fixed), **#2926** (refactor: remove FCM legacy dead endpoint — attack-surface reduction).
  - Backend correctness: **#2922** (refactor: saved-search error enum, replaces string-status), **#2928** (fix: report_summary snapshot-consistency).
  - Test hardening: **#2931** (regression test for #2928), **#2932** (e2e test for agent-review self-review guard around #2919).
  - Frontend correctness/i18n: **#2927** (test: EmergencyContactDirectoryPage i18n regression).
  - Accounting feature: **#2558** (feat: invoice PDF render UC-ACC-05.9) — long-stalled MVP trio member, finally landed after 30+ days.
  - Dependabot bumps: **#2935** (npm-minor), **#2673** (ktor), **#2585** (base64), **#2586** (validator), **#2583** (rust_xlsxwriter).
  - Closed unmerged: **#2934** (cargo-minor-patch — superseded).
- **Auto-review loop still working:** the code-review generator continues to bring resolution PRs to green in-window. The #2915 → #2920 → #2925 chain (fix → sweep → regression follow-up) is a recurring pattern worth automating (see risks below).
- **Buffer starved (cloud-only) — unchanged:** 7/8 open backlog items remain mobile-native/KMP, structurally unclaimable in cloud (issue #2652, AGP/Gradle egress). No dispatcher movement on this since 2026-08-31.
- **Reviewer-starvation risk cleared for accounting trio** (#2555/#2558/#2559 was 3-item cluster; #2558 landed this window). #2555 and #2559 still open — will follow up next brief.

## Since 2026-09-01

**Shipped 14 PRs / 1 closed:** #2919 self-review guard (security) · #2922 saved-search error enum · #2925 compliance raw-DB leak regression · #2926 FCM legacy endpoint removed · #2927 EmergencyContactDirectoryPage i18n test · #2928 report_summary snapshot-consistency · #2931 regression test for #2928 · #2932 e2e for agent-review self-review guard · **#2558 invoice PDF render UC-ACC-05.9** · dependabot #2935/#2673/#2585/#2586/#2583 · #2934 closed unmerged (superseded).

## Sprint progress (`_bmad-output/implementation-artifacts/sprint-status.yaml`)

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · **epics_done = 3/5** unchanged this run. Extended-scope epics (10B, 80, 81, 82, 83, 84, 85, 79, 8A, 9) folded into `coverage.json` and largely done.

| Epic | Sprint status | Coverage status (13 epics) |
|---|---|---|
| 6 — Announcements & Communication | in-progress | 6/6 stories done in coverage |
| 7A — Basic Document Management | in-progress | 5/5 stories done in coverage |
| 8A — Basic Notification Preferences | done | 3/3 stories done |
| 10A — OAuth Provider Foundation | done | 3/3 stories done |
| 10B — Platform Administration | in-progress | 7/7 stories done |
| 80 — Dispute Resolution | partial | 3/3 stories done in coverage |
| 81 — Reports | (extended) | 2/2 stories done in coverage |
| 84 — Documents / e-signature | (extended) | 3/5 done, 2 partial (84-1, 84-2) — unchanged |
| 82 / 83 / 85 / 79 / 7a / 8a / 9 | (extended) | all done in coverage |
| Accounting UC-ACC-05.x (extended) | in-progress | UC-ACC-05.9 shipped this window via #2558 |

## What's next (top 5 actions from ranked backlog)

1. **[high] Unblock mobile-native/KMP builds in the cloud runner (issue #2652)** — 7/8 open backlog items structurally unclaimable — **owner: pm-devops**. Chronic since 2026-08-31; buffer still starved.
2. **[high] Wire ppt-web direct-to-S3 upload (84-1 partial)** — POST /api/v1/documents/upload-url consumer + regression test — **owner: pm-frontend**. Drops partial count 2 → 1.
3. **[high] Build signer-facing document-sign page (84-2 partial)** — flip screen-map ppt/document-sign buildStatus planned → shipped — **owner: pm-frontend**. Closes 49/49 MVP when paired with #2.
4. **[high] Resolve gh-issue-2797** — cargo-deny RUSTSEC-2026-0258 (h2 empty-DATA-frame DoS) blocks every backend PR — **owner: pm-security**. Standing since 2026-08-18.
5. **[medium] Follow up on remaining accounting-MVP-trio PRs (#2555, #2559)** — reviewer-starvation was cleared for #2558 this window; extend the reviewer-slot pattern to the other two.

## Blockers

- **None new this run.** 14 merged PRs, no regressions detected in-window (aside from the #2925 catch-up on #2920's earlier residual).
- **Standing:** gh-issue-2797 (cargo-deny RUSTSEC-2026-0258 h2 DoS) blocks every backend PR until landed. Owner: pm-security.
- **Standing infra:** issue #2652 — mobile-native/KMP builds unlandable in cloud runner. Owner: pm-devops.
- **Aging:** 84-1 + 84-2 partial stories unchanged for 5 upkeep windows. Owner: pm-frontend.

## Role focus today: **pm-security** (rotation idx 5; last 2026-07-21, 46d stale) + pm-scrum-master always-on

- **pm-scrum-master** (always-on): 14 PRs in 4 days is a strong delivery cadence — the code-review + auto-merge loop is compounding. The **#2915 → #2920 → #2925** collision (fix PR introducing regression that a follow-up sweep partially missed, then a second follow-up caught) confirms the open-question from the 09-01 brief: automated post-merge pattern-grep is warranted. Also: **#2558 landing (accounting UC-ACC-05.9)** validates that reviewer-slot rotation unblocks stalled trios — apply the same pattern to #2555/#2559 next.
- **pm-security** (rotation, first run in 46 days):
  - **Security wins this window:** #2919 (agent-review self-review guard, IDOR-adjacent), backed by e2e test #2932; #2925 (compliance raw DB error leak fully swept); #2926 (dead FCM legacy endpoint removed — attack-surface reduction).
  - **New risk surfaced:** **regression-window between resolution PRs** — the #2915 → #2920 → #2925 chain shows security-sensitive cleanup PRs can silently re-introduce the exact anti-pattern the previous PR removed, within a merge window shorter than the routine's daily cadence. Sanitized-output cleanups, IDOR sweeps, and CSRF wiring are the highest-impact categories. Recommend post-merge sanity grep (see risks.json new entry).
  - Standing security items unchanged: **gh-issue-2797** (cargo-deny RUSTSEC-2026-0258 h2 DoS); layout webhook replay guard (#2485); mobile LAYOUT_CACHE_KEY tenant-scoping (#2486).

## Coverage (upkeep this run — 2026-09-05)

- **`coverage.json` not re-scanned this run** — upkeep, no epic re-check triggered by merged PRs. Merged-PR evidence: 14 PRs; none match a coverage story by keyword for a status flip (all correctness/security/refactor/test of already-shipped surfaces, plus one accounting sub-UC #2558 which is not tracked as a coverage story).
- **`coverage_cursor` advances 7 → 8** (epic-82 → epic-83 next run).
- **`pm_cursor` advances 5 → 6** (pm-security → pm-data next run). role_last_run["pm-security"] = 2026-09-05.
- **Composition unchanged: 47 done · 2 partial · 0 not-started** across 13 epics. **Missing UC links: 3** (UC-33.1/33.2/33.3 already queued). Zero orphan screens, zero validation errors.
