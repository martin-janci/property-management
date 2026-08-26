# pm-scrum-master — 2026-08-26

## Summary

Sprint "Epic 6/7A/8A/10A" is functionally complete at story level (all tracked stories across epics 6, 7a, 8a, 10a, 10b, 80, plus 79/82/84/85/9 read as done in `sprint-status.yaml`), but the epic-level status rollup is stale (epic-6/7a/10b/80 still show in-progress/partial with old `stories_completed` counts that contradict their own story lines). This run's 5 merged PRs (#2848–#2852) are quality/hardening work layered on top of that finished sprint, not new sprint stories, and the real open work is the unclaimed mobile-native/KMP backlog plus three stalled human-authored accounting PRs.

## Sprint progress

- Sprint: **Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth**
- Epics done: **2 / 6**  (rollup stale; story-level detail reads all closed epics done — reconciliation needed)

## Shipped since last run

- **#2848** — churn-hotspot AML dashboard extract (frontend/apps/ppt-web compliance)
- **#2849** — moderation overdue affordance (ContentModerationPage)
- **#2850** — verification-badge i18n snapshot
- **#2851** — voice OAuth token encryption round-trip tests (voice_webhooks.rs)
- **#2852** — AML dashboard test dedup

## Next actions

- **[high]** Reconcile epic-level status/`stories_completed` fields in `sprint-status.yaml` (epic-6, epic-7a, epic-10b, epic-80) against the story-level `development_status` lines, which already show all stories done — DoD: `epics:` block status + `stories_completed` match the story-level detail for all 6 epics.
- **[high]** Triage the 3 stalled accounting review PRs (#2555, #2558, #2559 — UC-ACC-05.17/05.9/05.8), open since 2026-07-28 with no activity since 2026-07-30 and outside dispatcher-owned scope — dependency: human maintainer (martin-janci) / pm-tech-lead; DoD: each PR merged, closed, or explicitly marked WIP with an owner and next step.
- **[medium]** Escalate the 6 open mobile-native/KMP action-list items (unclaimed for 3+ runs, cloud-unlandable) to a human/local-toolchain landing path instead of leaving them in the cloud dispatcher queue — DoD: each item either landed locally, explicitly deferred with rationale, or reassigned to a scheduled local sprint.
- **[medium]** Pick up `code-review-mobile-native-kmp-inquiries-response-contract` (backlog `status=ready`, has a plan file, medium priority, real MissingFieldException on every `/inquiries` call) — DoD: `InquiriesResponse` field contract matches reality-server `limit`, regression test added, PR merged.
- **[medium]** Request a full `coverage.json` scan (current one is `scan_kind=upkeep`, not a full scan) now that the tracked sprint reads 100% done, to confirm no new gaps before next-sprint planning — DoD: `coverage.json` regenerated with `scan_kind=full` and reviewed for new partial/not-started stories.
- **[low]** Have pm-tech-lead/pm-security review `AmlDashboardPage.tsx`, now flagged as a churn hotspot for a 2nd consecutive run (326 churn), to decide if it needs a decomposition pass rather than continued patch PRs — DoD: decision recorded: refactor scheduled or explicitly deferred.

## Risks

- **[high/medium]** Stale epic-level status rollups in `sprint-status.yaml` misrepresent sprint completion, which could distort next-sprint planning or executive reporting. **Mitigation:** reconcile `epics:` block against `development_status` as a routine data-hygiene action.
- **[high/medium]** Mobile-native/KMP backlog (6 items, 0 claimed across 3+ runs) keeps growing because it's structurally cloud-unlandable, creating an invisible debt pile. **Mitigation:** stand up a periodic local-toolchain landing slot or explicitly move items to a deferred/human-only queue.
- **[medium/medium]** 3 stalled accounting PRs (#2555/#2558/#2559) risk rot/merge-conflict as they age untouched (already ~4 weeks) outside dispatcher scope. **Mitigation:** human maintainer triage this run; close or actively review.
- **[medium/medium]** Repeated churn (2nd run) on `AmlDashboardPage.tsx` and (4th run) `voice_webhooks.rs` signals unstable hotspots that could be masking a design/test-flakiness problem rather than genuine incremental improvement. **Mitigation:** tech-lead review to decide refactor vs. continued patchwork.
- **[medium/low]** No open PRs or new issues this run plus a fully-done sprint spine means the routine may be running low on genuinely new sprint work, risking idle dispatcher cycles unless the backlog/coverage gap-driven track is fed. **Mitigation:** trigger the full coverage scan (next_action above) and prioritize the backlog's remaining open/ready items.

## Blockers

- **PR #2555 / #2558 / #2559 (UC-ACC-05.17/05.9/05.8)** — open since 2026-07-28, idle since 2026-07-30, outside dispatcher-owned scope — needs explicit human review decision. Owner: human maintainer / pm-tech-lead.
- **6 open mobile-native/KMP action-list items** (portfolio-analytics caps-100, unbounded-fanout retry1, cancellation-swallowed, ssoservice-untested, inquiries-response-contract, httpclient-no-timeout) — cloud-unlandable (no Kotlin/Gradle toolchain in the cloud agent) — 0 claimed across 3+ dispatcher runs. Owner: pm-backend (needs local toolchain execution).
- **epic-6, epic-7a, epic-10b, epic-80 status/`stories_completed` fields** — stale rollups contradict the story-level `development_status` (which shows all stories in these epics as done). Owner: pm-scrum-master.

## Open questions

- Is UC-33.1/33.2/33.3 (flagged missing in `coverage.json` `screen_gaps`) intentionally out of scope for this sprint, or does it need a new epic/story assignment?
- Is martin-janci still actively working the 3 stalled accounting PRs, or should they be reassigned/closed?
- Should the mobile-native/KMP action-list items be moved into a dedicated local-toolchain workflow given the cloud agent structurally cannot land Kotlin/Gradle changes?
- Now that `sprint-status.yaml` shows every tracked story done, is a new sprint already defined, or is this routine currently running purely off the backlog/coverage gap track?

## Decisions needed

- Reconcile epic-level status/`stories_completed` rollups in `sprint-status.yaml` vs. the fully-done story-level detail — owner: pm-scrum-master / repo maintainer
- Decide disposition of the 3 stalled accounting PRs (#2555/#2558/#2559) — owner: human maintainer (martin-janci) / pm-tech-lead
- Decide whether mobile-native/KMP backlog needs a dedicated local-landing track since it is structurally cloud-unlandable — owner: pm-tech-lead
- Confirm whether a new sprint should be defined now that all epics in the current sprint spine read done at story level — owner: pm-scrum-master
