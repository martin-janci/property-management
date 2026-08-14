# PPT Project State

_Generated: 2026-08-14 — routine Phase 1.6 lightweight upkeep (pm-qa rotation slot). Coverage `scan_kind=upkeep`; pm_cursor idx 3 → 4 (pm-qa → pm-devops next), coverage_cursor idx 5 → 6. Sprint window 2026-08-13..08-14 merged 10 PRs — 4 i18n follow-ups (#2755/#2756/#2757/#2761), 3 code-review retries closed (#2746/#2747/#2748), SSO backslash open-redirect closed (#2758), WebSocket disconnect lifecycle hardened (#2760), rust-toolchain investigation closed unmerged path (#2745). Buffer-low fired from dispatcher for planner refill; 14 dispatcher-generated code-review-finding signals folded into backlog + 3 churn hotspots + 5 previously-null i18n/backend signals repaired._

## Executive summary

- **Delivery still at 47/49 stories done, 2 partial** (the 84-1 direct-to-S3 upload wiring and 84-2 sign page). The 2026-07-28→07-30 window shipped **17 PRs** — a mix of dispatcher follow-up work (post-merge-review issues #2560/#2561/#2562/#2563/#2564/#2557 all closed) and security/DX hardening (Android SSO CSRF #2568, scheduler RLS-leak #2567, layout review-hardening #2478, ppt-web AuthContext stale-role #2553).
- **Layout epic is now fully wired end-to-end:** PR #2549 landed publish/webhook/revalidate event emission + sink (closes #2532); PR #2478 hardened authz + publish TOCTOU + webhook replay; PR #2576 scheduled `layout_change_events` retention prune (closes #2563).
- **Documents / e-signature side:** PR #2571 added org-scoped DELETE-by-file_key for direct-upload orphan cleanup (closes #2564); PR #2504 mounted signature-request list/create as a document sub-resource (BIT-313). 84-1 (direct-to-S3 frontend wiring) and 84-2 (signer page) remain the last 2 partial stories.
- **Auto-review loop caught 3 same-window regressions:** #2573 (DELETE-by-file-key can nuke still-referenced same-org object), #2574 (Android SSO CSRF guard is half-wired — mint() has no call site so every callback is rejected), #2575 (/disputes/kpis has no window ordering validation, only test quarantined). All 3 are now queued for pm-backend / pm-mobile fixes.
- **CI + release tooling fixes:** PR #2566 (version-bump rebase+retry — closes #2561 GH006), PR #2565 (reality-web Docker build fix — closes #2560, unblocks 6-week frontend-image gap), PR #2569 (SDK drift gate runs on client + workflow changes).
- **Open PRs (3): accounting MVP-loop trio** (#2555 invoice lifecycle, #2558 invoice PDF, #2559 PAY-by-square QR) — draft-ready since 2026-07-28 with zero reviewer engagement. This is now the top delivery blocker after the auto-fix follow-ups.

## Sprint progress (`_bmad-output/implementation-artifacts/sprint-status.yaml`)

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · **epics_done = 3/5** unchanged this run. Extended-scope epics (10B, 80, 81, 82, 83, 84, 85, 79, 8A, 9) folded into `coverage.json` and largely done.

| Epic | Sprint status | Coverage status (13 epics) |
|---|---|---|
| 6 — Announcements & Communication | in-progress | 6/6 stories done in coverage |
| 7A — Basic Document Management | in-progress | 5/5 stories done in coverage (7a-1 evidence refreshed via #2571) |
| 8A — Basic Notification Preferences | done | 3/3 stories done |
| 10A — OAuth Provider Foundation | done | 3/3 stories done |
| 10B — Platform Administration | in-progress | 7/7 stories done |
| 80 — Dispute Resolution | partial | 3/3 stories done in coverage; sprint-status still says partial (pending reconciliation) |
| 84 — Documents / e-signature | (extended) | 3/5 done, 2 partial (84-1 direct-S3 wiring, 84-2 sign page); 84-1 evidence refreshed via #2571 |
| 82 / 83 / 85 / 79 / 81 / 8a / 9 | (extended) | all done in coverage; **79 re-checked this run (all 4 stories done, 79-2 evidence refreshed with PR #2553)** |

## Shipped since last run (17 PRs > #2552)

- **#2576** — gh-issue-2563: schedule layout_change_events retention prune
- **#2572** — gh-issue-2562: wire get_dispute_kpis into a reporting endpoint (spawned #2575)
- **#2571** — gh-issue-2564: org-scoped DELETE-by-file_key for direct-upload orphan cleanup (spawned #2573)
- **#2570** — gh-issue-2557: dedupe seed_org/set_ctx in db test suites
- **#2569** — dx: run SDK drift gate on client + workflow changes
- **#2568** — code-review mobile-native-kmp: Android SSO CSRF state verification (spawned #2574)
- **#2567** — code-review api-core: clear scheduler global-read RLS GUC before pool return (retry1)
- **#2566** — gh-issue-2561: version-bump rebase+retry to fix GH006 on concurrent dev merges
- **#2565** — gh-issue-2560: reality-web Docker build fix (unblocks 6-week frontend image gap)
- **#2554** — chore(research): refill starved dispatcher stack (7 new vectors, 14 promoted)
- **#2553** — code-review ppt-web-core: AuthContext cold-boot routes through refreshTokenInternal (stale-role fix)
- **#2549** — gh-issue-2532: layout publish/webhook/revalidate event emission + sink
- **#2504** — fix(api-server): signature-request list/create — mount as document sub-resource (BIT-313)
- **#2491** — chore(deps): npm-minor-patch group (5 updates)
- **#2482** — refactor: reconcile docs/repo-map.md with current tree
- **#2478** — fix(layout): review-hardening sweep (authz, publish TOCTOU, webhook replay, defensive rendering)
- **#2433** — feat(mobile-native): iOS listing detail renders through the shared resolved layout

## What's next (top 5 actions from ranked backlog)

1. **[high] Fix #2573** — DELETE /documents/by-file-key can delete a still-referenced object within the same org (regression from PR #2571) — **owner: pm-backend**. Adds a reference-check guard before delete; needed before any client wires 84-1.
2. **[high] Fix #2574** — Android SSO CSRF guard half-wired (SsoStateStore.mint() has no call site so every reality://sso callback is rejected) — **owner: pm-mobile / react-native**.
3. **[medium] Fix #2575** — /disputes/kpis has no window-ordering validation, only test is quarantined — **owner: pm-backend**.
4. **[medium] Shepherd accounting MVP-loop trio merge** (#2555, #2558, #2559) — 2-day reviewer starvation blocking the accounting stack — **owner: pm-tech-lead**.
5. **[high] Finish 84-1** direct-to-S3 wiring in ppt-web (POST /documents/upload-url consumer) — **owner: pm-frontend**. Depends on #2573.

## Blockers

- **#2574 Android SSO CSRF half-wired** — the freshly-merged CSRF fix (#2568) has no call site; every reality://sso callback is now rejected until re-wired. Owner: pm-mobile.
- **#2573 DELETE-by-file-key same-org reference gap** — new endpoint can delete a still-referenced S3 object within the same org (regression from PR #2571). Blocks safe client wiring for 84-1. Owner: pm-backend.
- **Accounting trio (#2555 / #2558 / #2559)** — no reviewer engagement in 2 days; dispatcher can't advance the MVP-loop. Owner: pm-tech-lead.

## Role focus today: **pm-backend** (rotation idx 1; last 2026-06-06, 54d stale) + pm-scrum-master always-on

- **pm-scrum-master** (always-on): produced the delivery synthesis above. Headline = the auto-review loop shipped 17 PRs in 2 days and caught 3 of its own regressions inside 24h — the loop is genuinely closing. Reviewer capacity is now the tighter constraint than implementer capacity (accounting trio starving).
- **pm-backend** (rotation): flagged the 3 fresh regressions (#2573 data-loss, #2575 quarantined-test, #2547 hotfix-no-test carryover) as backend hygiene priorities. Also recommends investigating repeated churn on services/scheduler.rs — extract retention/prune jobs to a dedicated module. Sees the accounting trio as needing pm-tech-lead reviewer attention rather than more backend work.

## Coverage (upkeep this run — 2026-07-30)

- **`coverage.json` refreshed via mechanical upkeep** — `scan_kind=upkeep`, `generated` bumped, no re-scan.
- **Epic re-check: epic-79** — cursor idx 3. All 4 stories still `done`; evidence entry added to 79-2 for PR #2553 (AuthContext cold-boot stale-role fix). `last_checked = 2026-07-30` stamped on all 4 stories.
- **Merged-PR evidence added:** 84-1 (PR #2571 orphan cleanup), 7a-1 (PR #2571 lifecycle complement). No status flips.
- **`coverage_cursor` advances 3 → 4** (epic-79 → epic-7a next run).
- **`pm_cursor` advances 1 → 2** (pm-backend → pm-frontend next run). role_last_run["pm-backend"] = 2026-07-30.
- **Composition unchanged: 47 done · 2 partial · 0 not-started** across 13 epics. Same 3 missing UC links (UC-33.x — 2 queued into action-list this run, 1 remaining). Zero orphan screens, zero validation errors.
