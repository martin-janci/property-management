# PPT Project State

_Generated: 2026-08-01 — routine Phase 1.6 lightweight upkeep (Scrum Master synthesis + pm-frontend rotation slot). Coverage `scan_kind=upkeep`; pm_cursor idx 2 → 3 (pm-frontend → pm-qa next), coverage_cursor idx 4 → 5 (epic-7a re-checked — all 5 stories still done, `last_checked` refreshed; advances to epic-80)._

## Executive summary

- **Delivery unchanged at 47/49 stories done, 2 partial** (84-1 direct-to-S3 upload wiring and 84-2 sign page). The 2026-07-31→08-01 window shipped **5 PRs** — all post-merge / hardening work: PR #2613 (scheduler vote-lifecycle extraction), PR #2614 (layout webhook replay integration test — closes gh-issue-2485), PR #2615 (openapi-ts drift-gate hardening), PR #2616 (admin-web inline-primitives → @ppt/ui-kit consolidation), PR #2618 (reality-web screen-map reconciliation for PR #2600).
- **Auto-review loop clearing its own backlog:** three long-standing in-progress items resolved this window — gh-issue-2485 (layout webhook replay) via PR #2614, test-gap-screen-map-drift-pr-2600-reality via PR #2618, and pm-backend-scheduler-rs-refactor-extract-jobs (partial — vote-lifecycle extracted, retention/prune still open under repeated-churn slot) via PR #2613.
- **Two frontend `partial` stories remain the delivery gate:** 84-1 (ppt-web direct-to-S3 upload wiring) is still blocked on gh-issue-2573 (same-org reference-check guard) with no backend churn on it for 3 days; 84-2 (signer sign page) has failed 3 retries as a single squash and needs to be split.
- **New follow-up in flight:** issue #2612 (fire-once notifications) opened this window from post-merge review — needs owner-role triage (pm-backend vs pm-mobile depending on where the double-fire lives). Duplicate #2617 already closed.
- **Open PR watch:** #2619 (draft, further refactor of integrations/booking/mod.rs — pattern continuation of PR #2611); the accounting MVP-loop trio (#2555 / #2558 / #2559) is now **4 days without reviewer engagement**, up from 2 days last run. The reviewer-slot policy raised as DEC-107 on 2026-07-30 needs an actual decision.

## Sprint progress (`_bmad-output/implementation-artifacts/sprint-status.yaml`)

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · **epics_done = 3/5** unchanged this run. Extended-scope epics (10B, 80, 81, 82, 83, 84, 85, 79, 8A, 9) folded into `coverage.json` and largely done.

| Epic | Sprint status | Coverage status (13 epics) |
|---|---|---|
| 6 — Announcements & Communication | in-progress | 6/6 stories done in coverage |
| 7A — Basic Document Management | in-progress | 5/5 stories done in coverage (**re-checked this run — no change, evidence refreshed**) |
| 8A — Basic Notification Preferences | done | 3/3 stories done (fire-once follow-up #2612 in triage) |
| 10A — OAuth Provider Foundation | done | 3/3 stories done |
| 10B — Platform Administration | in-progress | 7/7 stories done |
| 80 — Dispute Resolution | partial | 3/3 stories done in coverage; sprint-status still says partial (pending reconciliation) |
| 84 — Documents / e-signature | (extended) | 3/5 done, 2 partial (84-1 direct-S3 wiring blocked on #2573, 84-2 signer page needs slice-split) |
| 82 / 83 / 85 / 79 / 81 / 8a / 9 | (extended) | all done in coverage |

## Shipped since last run (5 PRs > #2611)

- **#2613** — refactor(api-server): extract scheduler vote-lifecycle jobs to submodule (partially addresses pm-backend-scheduler-rs-refactor-extract-jobs — vote lifecycle done; retention/prune extraction still open under repeated-churn slot)
- **#2614** — test(api-server): integration test for outbound layout webhook replay/timestamp (**closes gh-issue-2485** — layout webhook replay/timestamp coverage now enforced)
- **#2615** — chore(api-validation): harden openapi-ts drift-gate generator steps (closes dx-fixme-api-client-generator-typescript-error-types)
- **#2616** — dx-fixme(admin-web): consolidate admin-web inline primitives onto @ppt/ui-kit (unlocks the same pattern for ppt-web — see next-actions)
- **#2618** — test-gap(reality-web): reconcile reality-web screen-maps with PR #2600 inline-script hardening (**closes test-gap-screen-map-drift-pr-2600-reality**)

## What's next (top 5 actions from ranked backlog)

1. **[high] Split 84-2 signer sign page into 3 mergeable slices** (route/manifest → capture → verify+delivery) — retry3 single-squash pattern is failing; go incremental with per-slice screen-map flip — **owner: pm-frontend**. See pm-frontend-84-2-signer-page-scoped-2026-08-01.
2. **[high] Unblock 84-1 by landing gh-issue-2573** (DELETE-by-file-key same-org reference guard) — 3 days stalled with zero backend churn on it — **owner: pm-backend / pm-scrum-master to escalate**. 84-1 wiring is queued to re-open the moment #2573 lands.
3. **[high] Escalate accounting MVP-loop trio** (#2555 / #2558 / #2559) — now 4 days reviewer-starved, up from 2 days last run — assign a named reviewer or split the trio — **owner: pm-tech-lead**.
4. **[medium] Triage new follow-up issue #2612** (fire-once notifications) — decide owner_role, scope, whether it blocks 8A extended-scope closure — **owner: pm-tech-lead**.
5. **[medium] Extend #2616's ui-kit consolidation pattern to ppt-web** — audit top-10 duplicated Spinner/EmptyState/Button variants (per code-review-ppt-web-ui-duplicated-spinner-markup) and land as one deprecation PR — **owner: pm-frontend**.

## Blockers

- **gh-issue-2573 DELETE-by-file-key same-org reference gap** — still open 3 days after being raised; blocks 84-1 unblock. Owner: pm-backend.
- **Accounting trio (#2555 / #2558 / #2559)** — 4 days without reviewer engagement (up from 2). Blocks accounting MVP-loop closure. Owner: pm-tech-lead.
- **84-2 signer page retry loop** — three failed single-squash attempts; the retry pattern itself is the blocker, not the underlying complexity. Owner: pm-frontend.

## Role focus today: **pm-frontend** (rotation idx 2; last 2026-06-10, 52d stale) + pm-scrum-master always-on

- **pm-scrum-master** (always-on): produced the delivery synthesis above. Headline = 5 PRs merged, all resolving in-progress hygiene items; the frontend `partial` slice is now the only thing between the project and 49/49. Reviewer capacity (accounting trio) is now a 4-day blocker — reviewer-slot decision from 2026-07-30 (DEC-107) needs to actually resolve.
- **pm-frontend** (rotation, 52d stale): identifies the 84-2 retry pattern as the highest-leverage fix — three single-squash attempts have failed, splitting into route/manifest/capture/verify slices lets each attempt land ≤400 LOC. Sees the #2616 admin-web ui-kit consolidation as a directly transferable pattern for ppt-web (10 duplicated primitives per code-review). Flags 84-1 blocking chain (frontend blocked on backend blocked on nothing) as a coordination gap — recommends pm-scrum-master hand #2573 explicitly to a backend owner this window. Recommends closing UC-33.3 to complete the UC-33.x wave (2/3 already queued 2026-07-30, 1 remaining).

## Coverage (upkeep this run — 2026-08-01)

- **`coverage.json` refreshed via mechanical upkeep** — `scan_kind=upkeep`, `generated` bumped, no re-scan.
- **Epic re-check: epic-7a** — cursor idx 4. All 5 stories still `done`; `last_checked = 2026-08-01` stamped. Evidence note added to 7a-1 (PR #2614 confirms broader webhook-hardening pattern that also applies to future document webhooks). Evidence note added to 84-1 (still partial; still blocked on #2573).
- **`coverage_cursor` advances 4 → 5** (epic-7a → epic-80 next run).
- **`pm_cursor` advances 2 → 3** (pm-frontend → pm-qa next run). role_last_run["pm-frontend"] = 2026-08-01.
- **Composition unchanged: 47 done · 2 partial · 0 not-started** across 13 epics. Same 3 missing UC links (UC-33.x — 2 queued 2026-07-30, UC-33.3 queued this run). Zero orphan screens, zero validation errors.
