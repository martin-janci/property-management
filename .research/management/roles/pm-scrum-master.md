# pm-scrum-master (2026-10-16)

**Summary.** Three small PRs merged this window — SSRF NAT64 hardening (#3103), mobile i18n fixes (#3105, #3106). Sprint-status shows no story moved. `coverage.json` lists 49/49 stories done, so there are no partial/not-started stories to rank; planning follows `.research/management/action-list.md` (now +8 fresh rows). The buffer is thin because the infra unblocks (#2966, #2652) are still outstanding.

## Shipped since last run

- PR #3103 — gh-issue-3099: block NAT64 RFC 8215 local-use prefix in SSRF filter (closes #3099, security hardening with regression tests).
- PR #3105 — fix(mobile): externalize detail-screen back-button labels to i18n.
- PR #3106 — fix(mobile): translate OutagesScreen status badge via i18n.

## Sprint progress

- Sprint: **Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth**
- `epics_done = 2/6` at sprint level (stale against coverage.json which records all extended epics as done).

## Blockers

- **IDOR retries (#2946 / #2944 / #2945)** — api-server cannot build in the cloud runner because utoipa-swagger-ui cannot reach crates.io or egress (#2966). All 3 retries depend on this. Owner: `pm-devops`.
- **mobile-native / KMP backlog** — Cloud runner cannot do mobile-native builds (#2652). Seven items are dep_blocked, which is why claimable is 7/72. Owner: `pm-devops`.
- **PR #2559 (PAY by Square) and PR #2555 (invoice lifecycle)** — stalled ~64 d as of 2026-10-01 and waiting for a reviewer slot. Owner: `pm-tech-lead`.

## Next actions

1. **[high]** Unblock utoipa-swagger-ui crates.io egress for api-server in the cloud runner (#2966). DoD: `cargo check -p api-server` succeeds in the cloud runner; the 3 IDOR retries become claimable. — `pm-devops`.
2. **[high]** Retry the cross-tenant IDOR fixes #2946 / #2944 / #2945 once #2966 is cleared. DoD: each is merged to `dev` with a cross-tenant regression test; the issues close. — `pm-devops`.
3. **[high]** Triage umbrella #3104 (portfolio and owner analytics 500s) and finish draft PR #3107 (portfolio_benchmarks drift). Split the 4 drift bugs into separate claimable issues; link to the #2946 portfolio_analytics IDOR work to avoid conflicts. DoD: #3107 merged and each of the 4 bugs has its own issue; no BIT-622 refs remain. — `pm-backend`.
4. **[medium]** Unblock mobile-native/KMP cloud builds (#2652) so the 7 dep_blocked items become claimable. — `pm-devops`.
5. **[medium]** Assign reviewer slots to stalled PRs #2559 / #2555, or close them. — `pm-tech-lead`.
6. **[low]** Review draft PR #2902 (saved-searches screen-map drift vs PR #2894) and run `/screens validate`. Also unify the auth-header and MFA-retry logic in the api-client (#3006). — `pm-frontend`.

## Risks

- **High / medium.** Actionable buffer only 7/72, dominated by mobile-native-gated items → dispatcher idles or churns on score=1 noise. Mitigate by prioritising #2966 and #2652 and score-gating noise.
- **High / high.** Three IDOR issues (#2944, #2945, #2946) stay unfixed because of the swagger-ui egress block. Mitigate by escalating #2966 — vendoring the crate or feature-gating swagger-ui in cloud builds is on the table.
- **Medium / medium.** Analytics drift 500s (#3104) in production owner-analytics and portfolio; potential merge conflict between #3107 and the #2946 IDOR fix. Mitigate by sequencing and by splitting #3104 into separate issues.
- **High / low.** Tracking is stale (epic-10b shows in-progress with 7/7 done; epic-80 shows partial with 3/3 done; sprint dated 2025-12-21). Reconcile and start a new sprint.
- **Medium / medium.** `coverage.json` reports 100% done, which hides UC-33.1–UC-33.3 and the mobile comment-thread / document-permission UIs noted as follow-ups.

## Open questions

- Is the 2025-12 sprint goal still active? Epics 6/7A show stories_completed 3/2 vs totals of 6/5, yet every development_status entry is done.
- Should epic-10b / epic-80 be flipped to done?
- Are UC-33.1–UC-33.3 meant to be built, and under which epic?
- Are #2559 / #2555 rebased or superseded (last status 2026-10-01, not re-checked)?
- Do mobile i18n PRs #3105 / #3106 close any tracked issue or story?

## Decisions needed

- Invest in the #2966 egress fix vs vendoring/feature-gating swagger-ui for cloud builds — owner: `pm-devops`.
- Define the next sprint and goal (current one is stale) — owner: `pm-scrum-master` with the human product owner.
- Decide whether the #3104 analytics drift bugs outrank new feature work, given the 500s — owner: `pm-tech-lead`.
- Close or reassign stalled PRs #2559 / #2555 — owner: `pm-tech-lead`.
