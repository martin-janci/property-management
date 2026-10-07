# PPT Project State

_Generated: 2026-10-07 — routine Phase 1.6 upkeep (pm-data rotation slot; pm-cursor idx 6 → 7) + pm-scrum-master always-on. Coverage `scan_kind=upkeep`; coverage_cursor idx 8 → 9 (epic-83 re-checked; integrations webhook refactor PR #3005 confirms per-receiver module split against stories 83-1/83-2/83-3). 4-day lag since last routine run (2026-10-03)._

## Executive summary

- **Delivery: 49/49 stories done across 13 epics in coverage** (same as last run — no story-status flips this window). Sprint-status.yaml spine remains out of sync (epic-6 counters, 7a counters, 10b/80 statuses) and a deep scan is still owed; this run did not run the scan.
- **9 PRs merged since 2026-10-03** (#3018–#3027 inclusive): the two biggest-impact items are **PR #3026** (vendor swagger-ui — unblocks api-server offline/cloud builds, retires issue #2966) and **PR #3025** (exact-pin jest-expo 56.0.5 — unblocks mobile RN test suite, retires #2951). Also: 2 reality-web silent-failure fixes (#3021 PriceAlerts mark-read, #3019 /report attachment control), 1 ppt-web i18n fix (#3020 workflow-automation toasts), 2 screen-map reconciliations (#3023 ppt/home, #3027 dashboard role-gate), 1 /sell dead-field drop (#3022), and 1 dispatcher-safety doc (#3018).
- **Buffer-low alarm persists: `claimable=16/72`** (floor 36). PR #3026 should unblock the three IDOR retries (#2944/#2945/#2946) once the cloud runner is confirmed building with the vendored asset. The retries stay owed as the single largest refill lever.
- **New follow-up blocker: issue #3029** (SyncSchedule stale error banner, regression from merged PR #2967) — now tracked at `code-review-reality-web-syncschedule-stale-error-banner` (bug, score 1).
- **Phase 1.5 code-review slice: ppt-web-core.** 3 findings — two rentals.tsx bug patterns (mutations swallow non-auth errors; queries drop isError/retry UI) and one disputes.tsx i18n refactor. 2 bugs promotable at score 2 → will reach ready at score 3 with one more signal.
- **Phase 1.6 pm-data (first run since 2026-07-23).** Headline risks: engagement/delivery event specs missing for Epics 6/7A/8A; FCM stub inflates sent-count (issue #484 open) so 8a-3's done-status for notification delivery is unreliable. Added `pm-data-announcements-engagement-events-doc` to action-list.

## Sprint progress (from pm-scrum-master)

- Sprint: Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth
- Epics done: 2 / 6
- Shipped since last run: #3026, #3025, #3020, #3027, #3023, #3022, #3019, #3021, #3018 (plus earlier-window #3005 webhook refactor, #2744 dispatcher archive-push fix)

## What's next (top 5)

1. **[high · pm-devops]** Verify PR #3026's vendored swagger-ui actually unblocks the cloud build; close #2966 and re-dispatch IDOR retries #2944/#2945/#2946.
2. **[high · pm-tech-lead]** Refill the planner with unblocked frontend-only ready items: `code-review-ppt-web-core-authed-roots-session-leak`, `code-review-reality-web-profile-mock-data`, `code-review-ppt-web-core-community-group-detail-mock-data`, `code-review-ppt-web-core-route-group-toast-i18n`, `code-review-reality-server-add-to-compare-toctou-max-cap`, `code-review-reality-server-create-handlers-return-200-not-201`.
3. **[high · pm-qa]** Triage new backlog item `code-review-reality-web-syncschedule-stale-error-banner` (issue #3029) once a frontend specialist slot opens.
4. **[high · pm-data]** Write `docs/data/announcements-engagement-events.md` (Epic 6 analytics spec; no events currently defined for announcement publish/read/acknowledge).
5. **[low · pm-qa]** Reconcile `docs/screens/reality/sell.md` (PR #3022) and `docs/screens/reality/report-listing.md` (PR #3019) — two new drift signals this run.

## Blockers (from pm-scrum-master)

- **#2966 cloud-build unblock** — PR #3026 should close, but needs a cloud build to confirm. Owner: pm-devops.
- **Mobile-native KMP builds (#2652)** — PR #3026 doesn't address Gradle egress. Owner: pm-devops.
- **PRs #2555, #2559** stalled 69 days with no reviewer. Owner: pm-tech-lead.
- **Issue #3029** regression now in backlog as `code-review-reality-web-syncschedule-stale-error-banner`. Owner: pm-backend / pm-frontend.

## Role focus today

- pm-scrum-master (always-on) + pm-data (rotation slot).

## Role summary (one-line each)

- **pm-scrum-master:** catch-up after 4d lag; two cloud-build unblocks shipped (#3026, #3025); buffer-starvation top-ranked risk persists.
- **pm-data:** no data-stack PRs this window; spec'd 5 new analytics docs needed (announcements, notification delivery, document audit, OAuth/MFA telemetry, Reality mobile parity); flagged FCM-stub inflating sent-count (issue #484).

## Coverage upkeep

- Epic re-checked: `epic-83` (external integrations) — all 3 stories still `done`; PR #3005 webhook split (airbnb/booking/subscriptions/portal/esignature/payment submodules) is fresh evidence for 83-1/83-2/83-3. `last_checked = 2026-10-07`. Advances to epic-84 next run.
