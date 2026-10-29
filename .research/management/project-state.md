# PPT Project State

_Generated: 2026-10-29 — routine Phase 1.6 incremental upkeep. Role focus: **pm-qa** (65-day stale rotation slot; pm-cursor idx 3 → 4). Coverage `scan_kind=upkeep`; coverage_cursor idx 0 → 1 (epic-10a re-checked, 3/3 stories still done, no PRs touched OAuth provider surfaces). pm-scrum-master always-on._

## Delta since last routine run (2026-10-28 10:54 UTC)

- **0 PRs merged** on `dev` (16h quiet window).
- **4 open draft PRs touched** (dispatcher auto-impl): #3190 (ppt-web authed-roots session-leak), #3191 (invoice qty/price NaN guard), #3192 (appeal no-status guard), #3193 (moderation report-flood dedup).
- **0 new issues** since `last_issue_seen=3187`. #3187 remains the acute dev-CI blocker.
- **Dispatcher activity**: 12+ dispatcher commits in the window, all `.research/`-scoped. Dispatcher is in buffer-starved + WIP-capped state; no merges, no new claims.
- **Carried blockers unchanged**: #3168 (cargo-deny closed but recurring), #2652 (KMP cloud builds), #2951 (RN jest-expo), #3164 (violations add_evidence authz), #3131 (merge pipeline deadlock — _condition still true_), #3187 (backend CI base failure).

## Executive summary

- **Delivery: 49/49 stories done across 13 epics in coverage.** No status flips this run; `last_checked = 2026-10-29` stamped on all 3 epic-10a stories (idx 0 → 1 upkeep).
- **0 PRs merged** — a genuinely quiet shipping window. The real signal is **structural**: 11 approved dispatcher PRs are stacked behind the human-approval gate (24-day queue age at the head of the line, #3024), and the merge pipeline cannot drain itself. The buffer-starved + WIP-capped state is now chronic (3+ consecutive days).
- **pm-qa rotation came up 65 days stale** — last run 2026-08-25. The fresh read surfaces a different flavor of test-shadow debt than August's version: now the regression tests EXIST (IG3 enforced at open-time) but DRIFT against moving base during the 15-24 day review-queue wait, so the merge gate silently rots. Six pm-qa next_actions queued (two high, three medium, one low); five new risks (two high/high, three medium) and three decisions_needed.
- **Backend CI remains red on dev** via #3187 — the ONE unresolved base-branch bug now gates every backend PR. Claimed fix is PR #3188 (needs human merge).

## Sprint progress

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · **epics_done = 4/4** (all four sprint epics are status: done in sprint-status.yaml after the 2026-10-26 reconciliation). Coverage holds 13 epics all at done.

| Epic | Sprint status | Coverage status | Shipped PRs this window |
|---|---|---|---|
| 6 — Announcements & Communication | done (6/6) | 6/6 done | — |
| 7A — Basic Document Management | done (5/5) | 5/5 done | — |
| 8A — Basic Notification Preferences | done (3/3) | 3/3 done | — |
| 10A — OAuth Provider Foundation | done (3/3) | 3/3 done | — (upkeep re-check, no change) |

Non-sprint epics in coverage: 6, 7a, 8a, 9, 10a, 10b, 79, 80, 81, 82, 83, 84, 85 — all 49/49 done.

## Shipped since last run

- **(nothing merged)** — 0 PRs merged, 0 issues closed, 0 story status flips. The window is a hard quiet-shipping window; the activity is all dispatcher research + auto-impl drafts awaiting human approval.

## What's next (top 5 from today's rotation)

1. **[high]** Land #3187 fix (PR #3188 or operator equivalent) → unred backend CI (owner: pm-security + pm-tech-lead)
2. **[high]** Decide + act on #3131 unblock path (second-identity / admin-merge-lane / relaxed-protection) → drain the 11-PR review queue (owner: pm-tech-lead + pm-devops)
3. **[high]** Add review-queue aging alarm to the routine brief (owner: pm-qa)
4. **[medium]** Audit the 11 in-review auto-impl PRs for IG3-contract drift (owner: pm-qa)
5. **[medium]** Add resident_cannot_add_evidence regression test for #3164 before fix (owner: pm-qa + rust-backend)

## Blockers (active)

- **#3187** — cross_org_assign_violation_is_not_found red on dev; reds every backend PR's required `test` check. Owner: pm-security. **Single-point-of-failure for backend merges.**
- **#3131** — Merge pipeline deadlocked (24+ days). 11 approved PRs stacked. Owner: pm-tech-lead. **Chronic.**
- **#3168** — cargo-deny RUSTSEC gate; closed 2026-10-26 but condition recurs if deps shift. Owner: pm-devops.
- **#2652** — mobile-native KMP cloud builds blocked; 5 open backlog items parked. Owner: pm-devops.
- **#2951** — mobile-rn jest-expo @ RN 0.87 version rot; RN tests still ungated in cloud. Owner: pm-devops.

## Role focus today

- **pm-scrum-master** (always-on) + **pm-qa** (rotation idx 3; 65d stale)

### pm-qa summary (one-line)

Six next_actions queued around a single theme: the IG3 contract is now a time-bomb in the review queue, because a 15–24 day old "failing-on-main" test can silently pass against today's dev. The merge gate rots as a function of queue age. Two high-priority asks: unred CI via #3187, and surface review-queue-aging in the brief so the drift is visible.

### pm-scrum-master summary (one-line)

0 PRs merged; delivery is 100% parked on operator human-approval gates. The 2h dispatcher cadence is now friction (no merge throughput) rather than progress; worth considering lowering until #3131 is unblocked.
