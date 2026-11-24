# pm-scrum-master — 2026-11-24

## Summary
30 PRs merged in-window (#3739–#3768) — the highest single-day throughput so far — dominated by the i18n warn→error flip burn-down (11 locale PRs: #3739/#3740/#3741/#3742/#3745/#3747/#3748/#3754/#3755/#3756/#3757) plus a security micro-batch (#3744 SSO O(n) sort, #3750 CGNAT in URL validator, #3761 feed-subscription SSRF, #3758 in-proc ratelimit note, #3760 voice-webhook RLS release consistency). Flip countdown is now **T-7d (2026-12-01)**; the only i18n drafts still open are the two mobile-RN ones (#3676 raw-status, #3677 hardcoded-button-labels) — both **≥22h stale and now 5 days from the promote-or-close deadline (2026-11-29).**

## Shipped since last run (2026-11-23T04:19:35Z → 2026-11-24T04:19:35Z)
- **i18n burn-down (11):** #3739 realtor/[id] parity guard, #3740 realtor analytics locale, #3741 reality-web listingEdit parity, #3742 ppt-web Registry Rules, #3745 Buildings pages, #3747 Reports page, #3748 FileDisputePage, #3754 EditFaultPage, #3755 RegistryPage SPA nav, #3756 GuestReportSubmission, #3757 forms feature.
- **Security micro-batch (5):** #3744 SSO pending-session O(n) amortized eviction (closes #3738), #3750 SSRF validator CGNAT (192.0.0.0/24 + 198.18.0.0/15), #3761 feed-subscription URL SSRF, #3758 in-process ratelimit not distributed (doc + gate), #3760 voice-webhook RLS release inconsistency.
- **Push/queue + mobile (6):** #3746 push 2xx-FCM-delivered + worker-requeue-drop + push-db-err-leak regression tests (clears 3 test-shadow debts in one PR), #3765 mobile-RN useApi 401 refresh+retry, #3767 AI chat delete error surfaced, #3766 moderation action type narrowing, #3768 AI workflows limit clamp, #3764 reality-web journal newsletter stop faking submit.
- **Platform admin (3):** #3763 capability-holders endpoint + admin-web view, #3762 payment-match date tiebreak + admin-web capability gates, #3751 no-test-diff audit landed (pm-qa).
- **Churn + screen-map (3):** #3753 invoice-remaining in integer cents, #3752 report-listing screen-map reconcile (reality), #3743 OrgProvider unmemoized retry1.
- **Issue #3738** closed via #3744.

## Sprint progress
- **Sprint:** Hardening & Refactor — Modules Under Watch, release-248 Quarantine Burn-down, Mobile-native Backlog
- **Workstreams done:** 0 of 3 (all three tracks `in-progress`/`blocked`).
- **Delivery-sprint rollup:** 6/6 epics `done` (closed 2026-11-21).
- **Coverage (upkeep this run):** 49/49 stories `done`; rotating cursor re-checked **epic-85** (2/2 mobile env-config + build-config stories still `done`; `last_checked` advanced to 2026-11-24; one `screen_gaps` note for frontmatter drift carried forward, no regression).

## Next actions (top 6, ranked)
1. **[high · pm-scrum-master → pm-frontend]** i18n warn→error flip countdown **T-7d (2026-12-01)**. Promote-or-close the two remaining mobile-RN drafts #3676 (raw status) + #3677 (hardcoded button labels) by **2026-11-29 (5 days)**. · DoD: both merged OR explicitly deferred with lint-rule carve-out.
2. **[high · pm-tech-lead]** Resolve dispatcher stall **#3357** — #3718 landed the mechanical key-union resolver last run; non-mechanical conflicts across two merge waves (82+ PRs) remain. · DoD: issue closed.
3. **[high · pm-devops]** Unblock mobile-native/KMP + swagger-ui egress in the cloud runner (**#2652, #2966**) — still 4 KMP backlog items structurally unclaimable; carried since 2026-08. · DoD: one KMP code-review task claimed and merged by cloud dispatcher.
4. **[medium · pm-backend]** Characterize the new high-churn hotspots surfaced this run: `voice_webhooks/handlers.rs` (265), `services/push/apns.rs` + `services/push/fcm.rs` (open PRs #3771/#3772 already in flight) — these are the split successors to the 6-run voice_webhooks.rs + push_fanout.rs legacy hotspots. · dep: pm-tech-lead.
5. **[medium · pm-qa]** Fold today's no-test-diff audit (#3751 landed) into Phase 1 — post the per-PR classification to the briefs table and set a gate for the push/* subtree. · dep: pm-backend.
6. **[low · pm-scrum-master]** Dependabot #3441 (axum-test 21.0.0 → 21.1.0) open since 2026-11-23 — trivial patch bump, land next cycle. · dep: pm-devops.

## Risks (delta from carried set)
- **i18n flip slip risk** (now **high/high** — was medium/high): 2 draft mobile-RN PRs with no reviewDecision are ≥22h stale, 5 days from promote-or-close. If both miss, CI will be red on 2026-12-01. Mitigation: nudge today; close-and-defer if no owner response by 2026-11-27.
- **Churn concentration in push subtree** (medium/medium): services/push/apns.rs + services/push/fcm.rs both have open hotspot PRs this cycle AND multi-file edits across the week. Three post-split modules, one post-merge review. Mitigation: pm-qa push/* test-diff mandate (carried action).
- **Dispatcher stall #3357 aging** (carried): mechanical resolver landed but non-mechanical remainder is now a week+ old. Mitigation: pm-tech-lead escalation.

## Open questions
- Is #3676 / #3677 an abandoned branch pair, or just stale? (No reviewDecision in either).
- Should the push/* subtree get its own `.github/CODEOWNERS` entry pinning pm-backend + pm-qa review?

## Decisions needed
- **Mobile-RN i18n flip carve-out policy** — if the two drafts don't land by 2026-11-29, do we flip anyway and quarantine the two files, or extend the deadline? Owner: pm-frontend (with pm-scrum-master sign-off).
- **push/* test-diff mandate** — require `#[cfg(test)]` or paired `*_tests.rs` diff for every push-stack PR. Owner: pm-qa + pm-backend.

## shipped_since_last_run / sprint_progress / blockers (digest fields)
- `shipped_since_last_run`: 30 entries (see "Shipped" section above).
- `sprint_progress`: `{"sprint": "Hardening & Refactor", "epics_done": 0, "epics_total": 3}` (3 workstreams; delivery-sprint 6/6 is historic).
- `blockers`:
  - `{"item": "dispatcher #3357", "reason": "non-mechanical conflicts across 82+ PRs after #3718 landed", "owner_role": "pm-tech-lead"}`
  - `{"item": "mobile-native KMP backlog (4 items)", "reason": "cloud-runner KMP + swagger-ui egress still blocked (#2652, #2966)", "owner_role": "pm-devops"}`
  - `{"item": "mobile-RN i18n drafts #3676/#3677", "reason": "≥22h stale, 5 days from promote-or-close, CI breaks 2026-12-01", "owner_role": "pm-frontend"}`
