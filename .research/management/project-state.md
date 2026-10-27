# PPT Project State

_Generated: 2026-10-27 — routine Phase 1.6 incremental upkeep. Role focus: **pm-frontend** (82-day stale rotation slot; pm-cursor idx 2 → 3). Coverage `scan_kind=upkeep`; coverage_cursor idx 12 → 0 (epic-9 2FA/TOTP re-checked, 1/1 story still done, no material change). pm-scrum-master always-on._

## Delta since last routine run (2026-10-27 01:07Z)

- **1 PR merged** (#3183) — code-review-api-handlers-violations-no-xtenant-idor-tests: cross-tenant IDOR regression suite + 404 fix for violation mutations. Does NOT map to a coverage story (addresses an archived code-review task_id; coverage composition unchanged).
- **Dispatcher payload today:** `buffer-low: claimable=6/72 — refill planner (dispatcher 2026-10-27; WIP full, 9 PRs jammed in review needing human merge)`. The `6/72` is almost entirely explained by the review-jam: 9 open rows in `assignments.json` all carry `status=review verdict=approve` — the reviewer bot has signed off but no human merge has drained them in >24h. Refill is not actually the bottleneck today; **reviewer-merge throughput is**.
- **3 fresh authz findings** on `backend/servers/api-server/src/routes/violations.rs` (Phase 1.5 code review on churn-aligned segment `api-handlers`): add_evidence role-gate asymmetry at :375 (medium), list_violations + list_appeals bulk leak at :295/:620 (medium), create_appeal missing violator-identity check at :584-600 (low). All candidate_vector `security`, confidence `medium`. These flow to `backlog.json` via the routine's Phase 2 and are tracked here as a sweep candidate rather than per-site rows.
- **No new issues since 3175** that aren't already queued.
- **1 churn hotspot NEW**: `backend/servers/api-server/src/routes/violations.rs` (58 lines across 2 commits; the hot commits are the #3187 fast-path + add_evidence authz fix). Confirms the "violations route is in flux" pattern.

<sub>Prior upkeep notes (archived): 2026-10-26 — pm-backend rotation; coverage_cursor 11 → 12 (epic-8a re-checked). 2026-10-22 — pm-tech-lead rotation; coverage_cursor 10 → 11 (epic-85 re-checked).</sub>

## Executive summary

- **Delivery: 49/49 stories done across 13 epics in coverage.** No status flips this run; `last_checked` refreshed on the single epic-9 story (`9-1-totp-2fa-setup`).
- **1 PR merged** this window — narrow code-review follow-up; no coverage movement.
- **Live blocker: 9-PR review-jam** — the dispatcher cannot drain WIP until a human merge window clears the approved-review backlog. Everything else (refill depth, buffer target, backlog promotion) is secondary while this holds.
- **New risk: violations.rs route is leaking authz oversights at a steady rate** — 3 findings in a single churn-aligned review slice, on top of #3164 last week. One-shot pattern sweep now beats whack-a-mole per site.
- **pm-frontend rotation came up 82-day stale** — last run 2026-08-06. Today's lens finds the frontend side is itself jammed on the same reviewer-merge gate (#3185, #3186 both approved + stuck).

## Sprint progress

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · **epics_done = 3/5** at sprint level (sprint-status.yaml); coverage-level all extended epics are complete.

| Epic | Sprint status | Coverage status | Shipped PRs this window |
|---|---|---|---|
| 6 — Announcements & Communication | in-progress (3/6)† | 6/6 done | — |
| 7A — Basic Document Management | in-progress (2/5)† | 5/5 done | — |
| 8A — Basic Notification Preferences | done (3/3) | 3/3 done | — |
| 10A — OAuth Provider Foundation | done (3/3) | 3/3 done | — |
| 10B — Platform Administration | in-progress (7/7)† | 7/7 done | — |
| 80 — Dispute Resolution | partial (1/3)† | 3/3 done | — |
| 79 — API client integration | (extended) | 4/4 done | — |
| 82 — iOS Reality Portal (SwiftUI) | (extended) | 5/5 done | — |
| 83 — Portal Integrations | (extended) | 3/3 done | — |
| 84 — Documents & e-signature | (extended) | 5/5 done | — |
| 85 — Build/CI configuration | (extended) | 2/2 done | — |
| 9 — 2FA/TOTP | (extended) | 1/1 done · **last_checked 2026-10-27 (cursor re-check)** | — |
| Cross-cutting (code-review) | — | — | #3183 (xtenant-IDOR regression suite + 404 fix — doesn't map to a coverage story) |

† sprint-status.yaml drift against coverage.json — queued as low-priority `pm-scrum-master-reconcile-sprint-status-vs-coverage` (dropped from the live action-list; a deep scan would resolve authoritatively).

## Shipped since last run (1 PR merged)

- **#3183** — code-review-api-handlers-violations-no-xtenant-idor-tests: cross-tenant IDOR regression suite + 404 fix for violation mutations (archived code-review task_id resolved).

## Blockers (1 live delivery, carried hardware)

- **9-PR review-jam (LIVE)** — all 9 open assignments carry `verdict=approve` + `status=review` with no merge progress in >24h. Specifically: #3024 (screen-map drift), #3075 (violation number race), #3108 (gh-issue-3104 analytics), #3119 (gh-issue-3116 i18n), #3171 (violations add_evidence authz), #3184 (violations_role_authz tests churn), #3185 (ppt-web-core neighbors stale token), #3186 (ppt-web-core useIotWebSocket), #3188 (gh-issue-3187 failing-on-dev test fix). Owner: **pm-tech-lead** (merge-gate owner).
- **#2652** — mobile-native/KMP cloud builds blocked (carried, high). Owner: pm-devops.
- **#2951** — mobile RN jest-expo version rot (carried, medium). Owner: pm-devops.
- **#3131** — merge-pipeline deadlock (carried, medium). Owner: pm-backend.

## What's next (top 5 from the roadmap)

1. **Clear the 9-PR approved-review jam** — merge the 9 approved assignments after a last-minute sanity re-review — owner: **pm-tech-lead** — this is the entire delivery picture today.
2. **Sweep violations.rs for role-gate / identity-check gaps** (Phase 1.5 surfaced 3 fresh findings on top of #3164 last week) — one-shot audit before per-site patching — owner: **pm-backend**.
3. **Decide the auto-merge policy for Claude-Approvals PRs after cool-down** — the 9-PR jam suggests the human-merge step is the primary delivery bottleneck, not reviewer throughput — owner: **pm-tech-lead**.
4. **Diagnose #3131 merge-pipeline deadlock** — carried, long-stale; likely sqlx advisory-lock contention — owner: **pm-backend**.
5. **Link missing use-cases UC-33.1 / UC-33.2 / UC-33.3 to their screen-maps** (coverage.screen_gaps.missing_use_cases carries them) — owner: **pm-frontend**.

## Role focus today: pm-frontend (+ pm-scrum-master always-on)

- **pm-frontend** (82-day stale rotation): The frontend side is itself jammed on the same reviewer-merge gate — #3185 (useNeighbors stale-token useMemo) and #3186 (useIotWebSocket.hasGivenUp drop) are both approved and stuck in `assignments.json status=review`. Lens also flags the screen-map drift carried from PR #2994 and the three missing UC links (33.1/33.2/33.3). KMP CreateListingScreen stub remains dep-blocked by #2652 cloud-build env — no change this run.
- **pm-scrum-master**: Review-throttled day. 1 narrow PR merged (#3183). The real work today is unjamming the 9 approved PRs; everything else is secondary. Queued decision on auto-merge policy; queued one-shot sweep for violations.rs authz pattern.

## Coverage (upkeep this run — 2026-10-27)

- **`coverage.json` refreshed via mechanical upkeep** — `scan_kind=upkeep`, `generated=2026-10-27T16:40:00Z`, no re-scan.
- **Epic re-check: epic-9 2FA/TOTP (cursor idx 12)** — single story `9-1-totp-2fa-setup` still `done`; no merged PR in the 2026-10-27 window touched 2FA / TOTP surfaces. `last_checked = 2026-10-27` stamped.
- **Merged-PR evidence check:** #3183 is a code-review fix; does not map to a coverage story (no evidence appended).
- **`coverage_cursor` advances 12 → 0** (epic-9 → first-sorted-epic next run; 13 distinct epics so (12+1) mod 13 = 0).
- **`pm_cursor` advances 2 → 3** (pm-frontend → pm-qa next). role_last_run["pm-frontend"] = 2026-10-27.
- **Composition unchanged: 49 done · 0 partial · 0 not-started** across 13 epics. Missing UC links: 3 (UC-33.1/33.2/33.3, now explicitly queued via `pm-frontend-link-missing-ucs-33` + three `gap-screen-uc-*` rows).
