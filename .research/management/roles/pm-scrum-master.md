# pm-scrum-master — 2026-11-14

## Summary

Sprint (Epic 6/7A/8A/10A) is fully done (4/4 sprint epics; 6/6 incl. 10B and 80 in sprint-status). 21 PRs merged since last run (#3478–#3501), including security fix #3501 (ammonia RUSTSEC-2026-0533) and the #3427 JWT iss/aud triage (#3480). The only open delivery items are three new review/in-progress PRs (#3506, #3507, #3508) and the stacked Dependabot majors. Note: coverage.json was not re-scanned this run; the 49/49 figure comes from upkeep of the prior deep scan.

## Sprint progress

- Sprint: Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth
- epics_done = 6/6 (at coverage level including 10B + 80)

## Shipped since last run

- **#3501** — gh-issue-3498: bump ammonia to 4.2.5 (RUSTSEC-2026-0533). Closes #3498.
- **#3497** — fix(api-server): voice book-facility must not report false success.
- **#3499** — guard date formatters against invalid input (formatDate RangeError).
- **#3485/#3486/#3490/#3491** — ppt-web-core error-handling + route-guard batch.
- **#3482/#3483** — ppt-web-core privacy route + reports schedule.
- **#3481/#3484/#3487/#3489/#3492/#3493/#3494/#3495/#3496** — reality-web i18n + fix batch.
- **#3478** — pm-devops dependabot major-bumps review.
- **#3479** — pm-devops stale-routine lag-floor guard.
- **#3480** — pm-scrum-master triage of security issue #3427 (JWT iss/aud) → promoted to `plans/security-jwt-iss-aud-cross-service.md`.

## Next actions

1. **[high]** Land #3507 (gh-issue-3505 disputes E2E auth) and resolve the ppt-web E2E regression tracked by #3505. Confirm it does not mask a real auth/route-guard regression from the #3485–#3491 batch. DoD: #3507 merged, E2E green on dev, #3505 closed.
2. **[high]** Implement the #3427 JWT iss/aud hardening per the plan at `plans/security-jwt-iss-aud-cross-service.md`, and ship #3508 (scheduler no-panic supervision, in-progress) as a crash-risk item. DoD: iss/aud emitted and validated in api-server and reality-server (plan or PR landed); #3508 merged with a panic-recovery test.
3. **[medium]** Human-review the 3 major Dependabot bumps (#3447 vitest 5, #3444 jsonwebtoken 11, #3443 utoipa-swagger-ui 10). Sequence jsonwebtoken after #3427 work (touches same auth code), then finish #3506 (gh-issue-3502 price-map i18n). DoD: each major merged or deferred with written justification; #3506 merged and #3502 closed.

## Risks

- **medium/medium** — Churn on admin-web TenantLifecyclePage, reality-web journal page and ppt-web mgmt-routes-auth-guard tests points to rework or flaky tests from the rapid merge burst. Mitigation: pm-qa checks regression tests on #3488 and #3500 before merge.
- **medium/high** — ppt-web E2E regression (#3505) may mean auth-guard changes broke an E2E flow. Mitigation: prioritise #3507 and verify root cause (test fixture vs product regression).
- **medium/medium** — Stacked Dependabot majors (vitest, jsonwebtoken, utoipa-swagger-ui) are stalling. Mitigation: set a review SLA (pm-devops).

## Open questions

- Is #3505 a test-only auth-fixture issue or a real product regression?
- Is coverage.json current? It was last deep-scanned several days ago.

## Decisions needed

- Dependabot major-review SLA/WIP cap — owner: pm-devops.
- Close #3357 (dispatcher stall) as self-resolved — owner: pm-scrum-master.
