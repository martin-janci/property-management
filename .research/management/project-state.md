# PPT Project State

_Generated: 2026-07-27 — routine Phase 1.6 lightweight upkeep (Scrum Master synthesis + pm-tech-lead rotation slot; no deep role deep-dive this run — cloud routine time budget). Coverage `scan_kind=upkeep`; pm_cursor idx 0 → 1 (pm-tech-lead → pm-backend next), coverage_cursor idx 2 → 3 (epic-6 re-checked cheaply, all 6 stories still done; advances to epic-79)._

_Note: this run has a 3d lag since the last daily routine (last_run_iso=2026-07-24T03:10Z). Stale-routine alert fires — see brief `Since last run`._

## Executive summary

- **Delivery is still converging: 47/49 stories done, 2 partial** (the 84-1 direct-to-S3 upload wiring and 84-2 sign page). The 2026-07-24→07-27 window is again dominated by **test-un-quarantine waves (BIT-565/566/567/570/573/574/588/609/610/630/647/658 — restoring 200+ tests across authz, analytics, documents/OCR, voting/announcements)** plus a targeted **security hardening + follow-up-fix wave** (post-merge review issues #2528/#2530/#2531/#2532/#2533/#2534/#2536/#2537 all opened + PRs landed for #2531/#2534/#2536/#2537/PR #2544 for #2530).
- **Security fix landed this window:** PR #2548 closes `join_group` / `leave_group` cross-tenant IDOR (Follow-up #2529 finding). PR #2551 backfills cross-tenant IDOR + error-branch tests for lease-abstraction import (#2537).
- **Analytics / data instrumentation shipping:** PR #2541 (`listing.viewed` reality-web event), PR #2515 (onboarding-tour funnel), PR #2544 (extends #2515 to full signup funnel, closes #2530), PR #2526 (OAuth token-usage analytics data layer, epic-10A), PR #2550 (backs dispute KPIs with SQL + structural status transitions, closes #2533).
- **Operational fixes landed:** PR #2547 (schedule `support_tooling_events` retention prune — closes #2531 — but ships without an api-server test; flagged as `hotfix-no-test-pr-2547` this run). PR #2546 (surface orphaned S3 object on failed direct-upload — closes #2534). PR #2545 (screen-map `extractEpics` catalog fix — closes #2536).
- **No dev-red incidents in the window.** CI green. 7 open PRs (all owned by `martin-janci`, mostly dispatcher-driven): #2553 (auth-context stale-role, draft), #2549 (layout webhook emit + sink, draft, closes #2532), #2504 (signature-request mount fix), #2482 (repo-map churn tidy), #2491 (dependabot npm-minor-patch, 5 updates), #2478 (layout review-hardening sweep), #2433 (mobile-native iOS listing detail — now 6 days stale, pm-frontend follow-up needed).
- **Post-merge review pipeline is healthy:** 8 follow-up issues opened 2026-07-24, 4 already resolved via merged PRs this window (#2531→#2547, #2534→#2546, #2536→#2545, #2537→#2551). Two remain open (#2528 booking webhook hardening; #2532 layout event emission — PR #2549 draft is against it). One long-standing #2366 (building_id lost on direct-upload) is still open.

## Sprint progress (`_bmad-output/implementation-artifacts/sprint-status.yaml`)

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · **epics_done = 3/5** unchanged this run. Extended-scope epics (10B, 80, 81, 82, 83, 84, 85, 79, 8A, 9) folded into `coverage.json` and largely done.

| Epic | Sprint status | Coverage status (13 epics) |
|---|---|---|
| 6 — Announcements & Communication | in-progress | 6/6 stories done in coverage (re-checked this run — cursor advances epic-6 → epic-79) |
| 7A — Basic Document Management | in-progress | 5/5 stories done in coverage |
| 8A — Basic Notification Preferences | done | 3/3 stories done |
| 10A — OAuth Provider Foundation | done | 3/3 stories done (PR #2526 adds token-usage analytics layer on top) |
| 10B — Platform Administration | in-progress | 7/7 stories done |
| 80 — Dispute Resolution | partial | 3/3 stories done in coverage; sprint-status still says partial (pending reconciliation) |
| 84 — Documents / e-signature | (extended) | 3/5 done, 2 partial (84-1 direct-S3 wiring, 84-2 sign page) |
| 82 / 83 / 85 / 79 / 81 / 8a / 9 | (extended) | all done in coverage |

## Shipped since last run (35 non-research commits on `dev`, 30 merged PRs > #2512)

- **#2544** — feat(UC-14): instrument full signup funnel (closes #2530)
- **#2548** — fix(UC-37): close join_group/leave_group cross-tenant IDOR (closes #2529 finding)
- **#2547** — fix(api-server): schedule support_tooling_events retention prune (closes #2531) — no api-server test in diff, flagged
- **#2546** — fix(api-client): surface orphaned S3 object on failed direct upload (closes #2534)
- **#2545** — gh-issue-2536: fix screen-map extractEpics catalog source so epic refs validate
- **#2550** — feat(db): back dispute KPIs with SQL + structural status transitions (closes #2533)
- **#2551** — test(api-server): cross-tenant IDOR + error-branch tests for lease-abstraction import (closes #2537)
- **#2552** — test(api-server): restore messaging_happy_paths seed scope (BIT-658)
- **#2541** — feat(reality-web): listing.viewed analytics event with view-source, filter-state, session context
- **#2526** — feat(epic-10A): OAuth token-usage analytics data layer
- **#2515** — feat(admin-web): instrument onboarding/signup funnel analytics (10B.6)
- 15+ test-un-quarantine PRs (BIT-565/566/567/570/571/573/574/585/588/609/610/630/647): #2502/#2507/#2508/#2510/#2511/#2514/#2516/#2518/#2519/#2520/#2522/#2525/#2539/#2540/#2542/#2543
- Doc / infra tidy: #2523 (nextest partition runbook), #2524 (support-staff read audit schema), #2527 (admin-web layout-editor extract-styles), #2535 (dependabot cargo-minor-patch runbook), #2538 (verify-gate adoption check-in)

## What's next (top 5 actions — anchored to the ranked backlog & open PRs)

1. **[high] Finish #2549 (layout publish/webhook/revalidate event emission + sink)** — closes #2532; unblocks pm-data KPI layer on layout — **owner: pm-backend**
2. **[high] Finish #2553 (AuthContext cold-boot stale role)** — closes `code-review-ppt-web-core-authctx-init-stale-role` — **owner: react-web**
3. **[high] Wire ppt-web direct-to-S3 upload for building_id (long-standing #2366)** — **owner: pm-frontend**
4. **[high] Harden `booking_push_notification` webhook + Airbnb replay parity (#2528)** — **owner: pm-security / rust-backend**
5. **[medium] Add api-server test for PR #2547 scheduler retention prune (hotfix-no-test)** — **owner: pm-qa / pm-backend**

## Blockers

- **#2433 — mobile-native iOS resolved-layout PR now 6 days stale.** Owner: pm-frontend (rebase / ping / consider close).
- **#2547 landed without an api-server test.** hotfix-no-test signal +2 this run. Owner: pm-qa.
- **`docs/screens/mobile/` still empty** — mobile screen-map coverage remains at 0 files (drift detection deliberately excludes mobile until seeded). Owner: pm-frontend / pm-mobile.

## Role focus today: **pm-tech-lead** (rotation idx 0; last 2026-06-05, 52d stale) + pm-scrum-master always-on

- **pm-scrum-master** (always-on): produced the delivery synthesis above; headline this run = the 2026-07-24 post-merge-review batch has already been half-resolved (4 of 8 issues merged), which shows the auto-issue → dispatcher-claim → PR loop is genuinely closing. Test-un-quarantine tail is still active (BIT-* backfill wave). No dev-red.
- **pm-tech-lead deep-dive: DEFERRED.** This run is a lightweight upkeep — the tech-lead role has not run since 2026-06-05 (52 days), which is a real risk. Recommend a manual `pm:tech-lead` deep-run to inspect architecture drift across the 30+ PR window (dispute FSM/KPI structural changes; layout Event emission; auth cold-boot regression; scheduler-driven retention prune). Deferred here to fit the routine's cloud time budget.

## Coverage (upkeep this run — 2026-07-27)

- **`coverage.json` refreshed via mechanical upkeep** (no deep re-scan). Only `generated` timestamp was bumped to the current UTC. Cursor advances epic-6 → epic-79 (index 2 → 3).
- **`coverage_cursor` advances 2 → 3** (epic-6 → epic-79 next run).
- **`pm_cursor` advances 0 → 1** (pm-tech-lead → pm-backend next run).
- **Composition unchanged: 47 done · 2 partial · 0 not-started** across 13 epics. Same 3 missing UC links (UC-33.x dispute sub-UCs). Zero orphan screens, zero validation errors.
