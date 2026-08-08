# PPT Project State

_Generated: 2026-08-08 — routine Phase 1.6 rotating run (pm-qa rotation slot). Coverage `scan_kind=upkeep`; pm_cursor idx 3 → 4 (pm-qa → pm-devops next), coverage_cursor idx 5 → 6 (epic-80 re-checked, no status change; advances to epic-81). Window shipped 6 PRs (#2712/#2711/#2709/#2707/#2706/#2708); dispatcher buffer-low trigger (claimable=5/72) fired the action-list refill below._

## Executive summary

- **Delivery unchanged at 47/49 stories done, 2 partial** (the 84-1 direct-to-S3 upload wiring and 84-2 sign page). This window's 6 merged PRs are almost entirely infra/security hardening rather than new coverage: #2707 caps workflow `api_call` response-body reads at 8 MiB (closes #2704, a memory-amplification DoS), #2706 fails closed on a partial RAG embedding batch, #2708 rejects non-finite numbers in workflow condition compare, #2711 dedupes layout tenant-override handlers, #2709 fixes reality-web `ListingForm` i18n, and #2712 adds a dispute `add_evidence` access-audit event (epic-80, closes the #2483/#2490 follow-up thread).
- **Only 2 of the 6 merged PRs map onto a tracked coverage story** — #2712 → 80-1 (already `done`; evidence appended) and #2706 → 84-5 (already `done`; evidence appended). The other 4 are workflow-automation/i18n hardening outside the tracked story set.
- **Process blockers, not code blockers, are now the top delivery risk.** The accounting MVP-loop trio (#2555/#2558/#2559) has gone from 2 days stalled (2026-07-30 check-in) to 8+ days with zero reviewer engagement — the reviewer-slot policy raised then was never actioned. Draft PR #2705 (rust-toolchain dependabot bump) has been CI-red since it opened.
- **Documentation drift found:** `sprint-status.yaml`'s epic-80 rollup still reads `stories_completed: 1` while `coverage.json` and every per-story `development_status` entry have shown all 3 stories (80-1/80-2/80-3) `done` since 2026-06-25→07-15. Raised as a decision this run rather than silently "fixed" (out of scope for static analysis to edit sprint tracking unilaterally).
- **QA gap identified:** the 3 security/DoS fixes in this window (#2707, #2706, #2708) have no confirmed regression test in the available evidence. 6 regression-test tasks queued.
- **Action-list buffer refilled from 6 open → 23 open** (target 36) — 12 role next_actions + 5 coverage/screen-gap candidates added; 2 in-progress items (data-audit add_evidence backfill, reality-web ListingForm i18n) resolved to `done` by this window's merged PRs (#2712, #2709). The coverage-gap candidate pool is now thin; the remaining shortfall depends on the dispatcher's separate `backlog.json` refill mechanism or a fresh deep `scan`.

## Sprint progress (`_bmad-output/implementation-artifacts/sprint-status.yaml`)

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · **epics_done = 2/6** (epic-8a, epic-10a `done`; epic-6, epic-7a, epic-10b `in-progress`; epic-80 `partial` per the stale rollup noted above). Extended-scope epics folded into `coverage.json` (13 epics, 49 stories) are 47/49 stories done — that broader figure is unchanged this window.

| Epic | Sprint status | Coverage status (13 epics) |
|---|---|---|
| 6 — Announcements & Communication | in-progress | 6/6 stories done in coverage |
| 7A — Basic Document Management | in-progress | 5/5 stories done in coverage |
| 8A — Basic Notification Preferences | done | 3/3 stories done |
| 10A — OAuth Provider Foundation | done | 3/3 stories done |
| 10B — Platform Administration | in-progress | 7/7 stories done |
| 80 — Dispute Resolution | partial (stale — see decisions) | 3/3 stories done in coverage; evidence refreshed this run via PR #2712 |
| 84 — Documents / e-signature | (extended) | 3/5 done, 2 partial (84-1 direct-S3 wiring, 84-2 sign page); 84-5 evidence refreshed via #2706 |
| 82 / 83 / 85 / 79 / 81 / 8a / 9 | (extended) | all done in coverage |

## Shipped since last run (6 PRs)

- **#2712** — feat(pm-data): dispute add_evidence access-audit event (closes #2483/#2490 follow-up thread; maps to coverage story 80-1)
- **#2711** — refactor: dedupe layout tenant-override handlers
- **#2709** — fix(reality-web): i18n ListingForm via next-intl catalogs
- **#2707** — fix(api-server): cap workflow api_call response-body read (8 MiB) — closes #2704 (memory-amplification DoS)
- **#2706** — fix(api-server): fail closed on partial RAG embedding batch (maps to coverage story 84-5)
- **#2708** — fix(api-server): reject non-finite numbers in workflow condition compare

## What's next (top 5 actions from ranked backlog)

1. **[high] Shepherd/unblock accounting MVP-loop trio** (#2555, #2558, #2559) — 8+ day reviewer starvation, up from 2 days last check-in — **owner: pm-tech-lead**.
2. **[high] Finish 84-1** — wire ppt-web direct-to-S3 upload via POST /documents/upload-url — **owner: pm-frontend**.
3. **[high] Finish 84-2** — build the signer-facing document-sign page in ppt-web — **owner: pm-frontend**.
4. **[medium] Resolve CI-red on draft PR #2705** (rust-toolchain 1.94.1→1.100.0) — fix-forward or close — **owner: pm-devops**.
5. **[medium] Reconcile sprint-status.yaml epic-80 rollup** (1/3) against coverage.json (3/3 done) — **owner: pm-scrum-master**.

## Blockers

- **Accounting trio (#2555/#2558/#2559)** — 8+ days, zero reviewer engagement; the 2026-07-30 reviewer-slot recommendation was never actioned. Owner: pm-tech-lead.
- **Draft PR #2705 CI-red** — dependabot rust-toolchain bump has been failing CI since it opened. Owner: pm-devops.
- **sprint-status.yaml epic-80 rollup stale** — says 1/3 stories done, actual is 3/3 per coverage.json and per-story status. Owner: pm-scrum-master.

## Role focus today: **pm-qa** (rotation idx 3; last run 2026-06-15, 54d stale) + pm-scrum-master always-on

- **pm-scrum-master** (always-on): produced the delivery synthesis above. Headline = this window's merged PRs are hardening, not new feature delivery; the real story is that both process blockers flagged 2026-07-30 (accounting trio, CI-red risk) have gotten worse rather than better, and a documentation-drift issue (epic-80 rollup) has been sitting unnoticed for weeks.
- **pm-qa** (rotation): flagged that the 3 security/DoS fixes merged this window (#2707, #2706, #2708) have no confirmed regression test in the available evidence and queued 6 regression/release-readiness tasks, including a full epic-80 regression pass to settle the sprint-status/coverage disagreement. Also flagged workflow_executor.rs as a 2-run repeated-churn hotspot needing a consolidated test suite rather than one-off fixes.

## Coverage (upkeep this run — 2026-08-08)

- **`coverage.json` refreshed via mechanical upkeep** — `scan_kind=upkeep`, `generated` bumped, no re-scan.
- **Epic re-check: epic-80** (Dispute Resolution) — cursor idx 5. All 3 stories remain `done`; evidence appended to 80-1 for PR #2712 (add_evidence access-audit event). `last_checked = 2026-08-08` stamped on all 3 stories. No status flips — the underlying issue is `sprint-status.yaml` drift, not a coverage gap.
- **Merged-PR evidence added:** 80-1 (PR #2712), 84-5 (PR #2706). No status flips.
- **`coverage_cursor` advances 5 → 6** (epic-80 → epic-81 next run).
- **`pm_cursor` advances 3 → 4** (pm-qa → pm-devops next run). `role_last_run["pm-qa"] = 2026-08-08`, `role_last_run["pm-scrum-master"] = 2026-08-08`.
- **Composition unchanged: 47 done · 2 partial · 0 not-started** across 13 epics. Same 3 missing UC links (UC-33.1/2/3 — all re-queued into action-list this run). Zero orphan epics, zero orphan screens, zero validation errors.
