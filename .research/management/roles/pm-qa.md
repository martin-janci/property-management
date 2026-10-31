# pm-qa

_Generated: 2026-10-31T12:00:00Z_

## Summary

QA role 67 days stale (last run 2026-08-25). The 8-PR merge window is dominated by code-review promotions (bug-class fixes without accompanying regression tests in most cases) — release-readiness signal is weak until a regression-test audit is run on the merged defect classes.

## Next actions

- **[high]** Regression-test audit on the 8 merged PRs this window: #3185 (useMemo stale-token), #3186 (WS hasGivenUp), #3188 (cross-tenant IDOR assign), #3189 (ranked ballot order), #3190 (authed-roots cache leak), #3191 (invoice NaN), #3197 (listing price-zero), #3198 (OS RNG panics) — confirm each has a failing-on-main test checked in (IG3 gate)
- **[high]** Cross-tenant IDOR regression-test suite — extend the pattern established by PR #3183 to moderation.rs, aml_dsa.rs, add_evidence sub-routes, announcement/document attachment mutations
- **[medium]** Flaky-skip audit across backend/vitest/jest-expo — produce a dashboard of silently-disabled tests; coordinate on #2951 jest-expo
- **[medium]** Define release-readiness checklist for the 'all-stories-done' state
- **[medium]** Resolve 3 QA churn-hotspots (violations_role_authz_tests.rs, violations.rs, compliance_wave1b_tests.rs) into positive test-coverage expansions
- **[low]** Screen-map drift PR #2994 — reconcile frontmatter against current ppt-web routes

## Risks

- **high/high** — Coverage exhaustion masks regression debt: 0 open stories look green while 8 bug-class fixes shipped in 3 days without consistent regression tests
- **medium/high** — 8 merged PRs are mostly single-site fixes; underlying defect patterns (unchecked parse, 403-vs-404 cross-tenant, WS state, authed-cache leak) likely repeat elsewhere
- **high/medium** — pm-qa ran only once in last 67 days; #2951 jest-expo blocker silently disables mobile RN test runs

## Open questions

- Does every merged code-review PR in the 8-PR window have an IG3 failing-on-main test?
- What is the current green/skipped/flaky ratio on backend + frontend + mobile test suites?
- Is there an owner for the backend regression-test harness that locks the 403-vs-404 cross-tenant contract across all mutations?

## Decisions needed

- Mandate IG3 (failing-on-main test) as a hard gate on code-review PRs — owner: pm-tech-lead
- Promote regression-test audit to a numbered epic in coverage.json — owner: pm-scrum-master
