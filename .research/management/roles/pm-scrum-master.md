# Role: pm-scrum-master — 2026-10-07

> Delivery lead / coordinator. Always-on this run. Catch-up after 4-day lag (last routine run 2026-10-03).

**Summary:** Catch-up run. PR #3026 (vendor swagger-ui) should clear the #2966 cloud-build blocker that has starved the dispatcher buffer (16/72), and PR #3025 (exact-pin jest-expo) should clear the #2951 RN jest-expo rot. Both unblock several already-queued action-list items. `coverage.json` and `sprint-status.yaml` spines remain stale: coverage shows 49/49 done, generated 2026-10-01, and epic counters in sprint-status lag the per-story statuses.

## Shipped since last run

- **#3026** fix(backend): vendor swagger-ui so backend builds offline (#2966) — infra unblock for api-server cloud builds (issue closure not confirmed in observation data).
- **#3025** fix(mobile): exact-pin jest-expo 56.0.5 — RN test suite green (should resolve `pm-devops-unblock-mobile-rn-jest-expo-version-rot` / #2951).
- **#3020** fix(ppt-web): i18n workflow-automation toast titles (should close `code-review-ppt-web-ui-workflow-automation-toasts-hardcoded-english`).
- **#3027 + #3023** screen-map reconciliation (closes `screen-map-drift-pr-3002-ppt`; #3023 reconciles ppt/home with PR #2995).
- **#3022 / #3019 / #3021** reality-web fixes (/sell dead seller fields #3016, /report attachment control #3017, PriceAlerts mark-read silent fail); issues #3016 and #3017 closed.
- **#3018** dispatcher: forbid git stash in shared worktrees (#3012 closed).
- Earlier in window: #3005 refactor(api-server) webhook.rs → per-receiver submodules (2026-10-03); #2744 dispatcher oversize-archive push fix (#2743 closed).

## Sprint progress

- Sprint: Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth
- Epics done: 2 / 6

## Blockers

- **gh-issue-2944/2945/2946 IDOR retries (security).** Depended on #2966 swagger-ui egress. #3026 should unblock, but needs confirming with a cloud build before re-queuing as claimable. Owner: pm-devops.
- **Mobile-native/KMP backlog (5+ items).** Cloud Gradle builds still blocked (#2652); #3026 does not address this. Owner: pm-devops.
- **PRs #2555 and #2559** (accounting: invoice lifecycle, PAY by Square). Stalled 69 days with no reviewer. Owner: pm-tech-lead.
- **Issue #3029** (SyncSchedule stale error banner, PR #2967 regression). Added to backlog as `code-review-reality-web-syncschedule-stale-error-banner` this run. Owner: pm-backend/pm-frontend.

## Next actions

1. **[high · pm-devops]** Verify #3026 vendored build in cloud runner; close #2966; mark `pm-devops-unblock-api-server-utoipa-swagger-egress` done. **DoD:** `cargo check -p api-server` succeeds offline in cloud; #2966 closed; action-list item closed. _(This routine already marked the action-list item done — step 1 is now cloud-side verification.)_
2. **[high · pm-devops]** Re-dispatch three IDOR retries (#2946, #2944, #2945) with CLOUD-BUILD-BLOCKED tag removed. Largest single buffer refill. **DoD:** 3 tasks claimable; each opens a PR with a cross-tenant regression test.
3. **[high · pm-qa]** Triage new issue #3029 (SyncSchedule stale error banner, regression from #2967) now in backlog. Finishing a regression ranks above new work. **DoD:** item claimable; acceptance test asserts the banner is gone after Cancel → Edit following a failed save.
4. **[medium · pm-devops]** Re-enable mobile RN items gated on #2951 (meter-reading numeric validation); add jest-expo pin to the dependabot ignore list. **DoD:** dependency cleared on `code-review-mobile-rn-meter-reading-no-numeric-validation`; item claimable.
5. **[high · pm-tech-lead]** Refill the planner with unblocked, non-backend ready items: `code-review-ppt-web-core-authed-roots-session-leak` (security boost), `code-review-reality-web-profile-mock-data`, `code-review-ppt-web-core-community-group-detail-mock-data`, `code-review-ppt-web-core-route-group-toast-i18n`, `code-review-reality-server-add-to-compare-toctou-max-cap`, `code-review-reality-server-create-handlers-return-200-not-201`. **DoD:** ≥6 of these claimable; owners spread across pm-backend and pm-qa.
6. **[low · pm-qa]** Reconcile reality drift items `screen-map-drift-pr-3013-reality`, `-pr-3014-reality`, and this run's new `-pr-3019-reality` / `-pr-3022-reality`. **DoD:** screen-map validate passes.

## Risks (new / notable)

- **Dispatcher buffer starvation persists (claimable 16/72)** — probability high, impact high. Mitigation: confirm #3026; unblock KMP (#2652); prioritise frontend-only + docs items.
- **Planning spines stale** — coverage.json says 49/49 done (generated 2026-10-01), sprint-status epic counters out of line (epic-6 3/6 vs 6 stories done; epic-7a 2/5 vs 5 done; epic-10b and epic-80 show in-progress/partial though stories are done). Mitigation: run `/ppt-project-management scan` locally to reconcile.
- **Reality-web /sell + /report churn + repeated drift** — probability medium, impact medium. More regressions like #3029 are likely. Mitigation: require screen-map updates in the same PR; add a regression test per churned page.
- **PRs #2555/#2559 stalled 69 days** — probability high, impact low. Will rot against dev as the webhook.rs split and other refactors land. Mitigation: decide merge or close; rebase if kept.

## Open questions

- Was #2966 auto-closed by PR #3026? Does the cloud runner actually build api-server offline now?
- Is dependabot PR #3030 (and #2981/#3024/#3028/#2902) mergeable, or does one touch jest-expo or RN 0.87?
- What is the scope of #3029 and which files does PR #2967 touch? Is it frontend-only?
- Why is the sprint (Epics 6/7A/8A/10A, started 2025-12-21) still active in October 2026? Should it close and a new one open?
- Is `post-merge-review.json` queueing follow-ups for #3025/#3026 that overlap the actions above?

## Decisions needed

- Close current sprint and define a new sprint goal. All listed stories are done; live work is infra unblock + IDOR hardening + reality-web cleanup. Owner: pm-product.
- Merge or close stalled PRs #2555/#2559. Owner: pm-tech-lead.
- Prioritise KMP cloud-build unblock (#2652) vs. keeping KMP items parked. Owner: pm-devops.
- Run a coverage scan now and refresh coverage.json (stale since 2026-10-01). Owner: pm-scrum-master via orchestrator.
