# pm-qa — rotating slice 2026-11-09

**Summary**: Test discipline is holding on this week's seven merged PRs — five carry new test files (#3350 Comparison-URL validation, #3353 password-change session revocation, #3354 registry auth, #3355 registry i18n, #3359 registry mutation-toast). The two zero-test merges are intentional (#3351 i18n-only / string catalogues, #3358 screen-map-doc-only). Primary risk this slice is churn-concentration in `RegistryPage.tsx` (4 PRs in 3 days) without a consolidated regression matrix — each PR adds one specific test, nobody owns the integration view.

## next_actions
- [medium] Add a `RegistryPage` integration test that exercises all mutation paths (pet+vehicle create/update/delete) in one file — current coverage is PR-shaped, not surface-shaped — owner: pm-frontend, DoD: one `RegistryPage.integration.test.tsx` under `frontend/apps/ppt-web/src/features/registry/pages/` that imports all four hooks landed across #3354/#3355/#3359.
- [high] Resolve the `UNRESOLVED:pm-qa` sentinel on `pm-frontend-mobile-rn-lint-gates-adopt-2026-10-28` — pm-qa accepts ownership of the lint-gate rollout plan but needs the mobile-native cloud-build unblock (#2652) to ship first (lint-gate CI needs green mobile-native to test). Rewrite `dependency` to `pm-devops-unblock-mobile-native-cloud-builds` and `depends_on: ["pm-devops-unblock-mobile-native-cloud-builds"]`. — owner: pm-qa, DoD: action-list row carries resolved depends_on and T25 passes.
- [medium] Backfill a regression test for issue #3357 (orphaned PRs): add a dispatcher-self-test rule that flags any assignment whose base commit is not an ancestor of `origin/dev`. Prevents the 3-PR silent-orphan class from recurring. — owner: pm-qa (co: pm-devops), DoD: `.research/dispatcher-self-test.sh` T31 (new) hard-fails on `git merge-base --is-ancestor $base origin/dev` = false.

## risks
- [medium/medium] Churn hotspot `RegistryPage.tsx` is up to 4 PRs in 3 days — mutation-handler semantics are evolving in rapid small PRs, each with narrow unit tests. A regression spanning the registry page is likely before an integration test lands. Mitigation: add the integration test in next_action #1 within the week.
- [low/high] `UNRESOLVED:pm-qa` and `UNRESOLVED:pm-security` sentinels in action-list have been open since 2026-10-28 (12 days). The dispatcher's claim-gate correctly skips them, but they're hiding real planning work. Mitigation: next_action #2 above clears one; pm-security owes the second (violations cross-tenant 404 sweep). Escalate to decisions.md if unchanged in 7 days.
- [medium/medium] The orphaned-PR incident (issue #3357) did not have a self-test hard-fail — the dispatcher silently kept three approved PRs on unreachable base commits. If pm-qa next_action #3 doesn't land, the next history-rewrite may repeat.

## open_questions
- Is the `RegistryPage` churn a sign of under-specified acceptance criteria (each PR fixes a separate small gap the AC missed), or is this feature genuinely still evolving? If the former — pm-qa should request an AC consolidation pass with pm-frontend.
- Should the `code-review-ppt-web-core-refresh-token-localstorage` row's confidence be upgraded from `medium` to `high`? The underlying finding is a structural fact (localStorage write is observable in the source) not a hypothesis — pm-security should own this decision.

## decisions_needed
- Tag refactor policy for high-churn frontend files (≥4 PRs in a rolling week) — do we require an integration test before the next mutation PR lands? — owner: pm-tech-lead
