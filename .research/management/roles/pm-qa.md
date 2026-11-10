# pm-qa — 2026-11-09

## Summary

7/7 merged PRs this window shipped with new regression tests (RegistryPage.toast.test.tsx, registryPageI18n.test.ts, RegistryPage.test.tsx, RegistryRulesPage.test.tsx, ComparisonUrlHandler.test.tsx, listingSummarySchema.ts, change_password_session_revocation_tests.rs). Test-shadow debt did NOT grow; release readiness is stable. The visible regression this window is the dispatcher queue (not code quality) — zero merged work for ~10 dispatcher runs until this burst landed the pre-existing relands.

## Next actions

1. **[medium]** Write a one-shot QA regression test asserting the dispatcher's happy path on a quarantined PR (claim → rebase → merge-attempt) so #3357-class stalls fail CI instead of silently queueing. Dep: pm-devops.
2. **[medium]** Adopt eslint-plugin-react-hooks + i18next/no-literal-string in frontend/apps/mobile — CARRIED from 2026-08-25 decision, 76 days unresolved. Dep: pm-frontend.
3. **[medium]** QA audit: non-mobile-native backlog items with confidence=medium (currently only code-review-ppt-web-core-refresh-token-localstorage) need a one-shot verification step before Phase 3 can promote them. Dep: pm-security.

## Risks

- **Dispatcher stall multiplier (high/medium)** — #3357 means every day without a refill pushes the implementer loop further from steady-state; 6/72 buffer at 3 concurrent × 12 runs/day = ~17 runs of head-room before full starvation. Mitigation: Phase 3 priority on non-mobile-native vectors this run.

## Open questions

- Is the 7-PR burst representative of baseline throughput once #3357 is resolved, or an artifact of pre-existing relands finally landing?
- Do the 4 open mobile-native-kmp bug backlog items need a local-only Mode: declaration in their eventual plans?

## Decisions needed

- Should the routine's buffer-low trigger text be upgraded to a hard-stop that forces Phase 1.5 to pick the oldest non-mobile-native segment? — owner: pm-scrum-master.
