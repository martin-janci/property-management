# pm-qa — 2026-10-31

_Rotation slot 3 (pm-cursor idx 3 → 4). Last pm-qa run was 2026-08-25 (67d stale)._

## Summary

Test coverage expanded across 15 merged PRs — 12 of the 15 added new test files
(test-gap deltas of +60–171 lines each). Zero release-blocking regressions this
window; the churn pattern shows `compliance_wave1b_tests.rs` hitting
repeated-churn threshold (3rd touch in 14 days, 171 lines added by #3204) —
compliance surface is still stabilising rather than stable.

## Next actions

1. **medium** — Add E2E test for reality-web account/profile/password i18n coverage once sibling backend ships messages — regression-pin the just-closed gap. _DoD: test file asserts non-English string renders in cs/de/en/hu/pl._
2. **low** — Reconcile 2 open screen-map-drift signals (#3203, #3186) — screen docs did not follow route changes. _DoD: screen-map updates land for touched routes._
3. **low** — Audit `compliance_wave1b_tests.rs` (3rd churn touch) — are the new cases shadowing a flaky invariant or genuinely new scenarios? _DoD: file stabilises (no churn in next 2 runs)._

## Risks

- **reality-web authed i18n cluster (high prob × medium impact):** account/profile/password + realtor/profile pages ship entirely in English on 5/6 non-default locales (same bug class as PR #3203). Plus 15 raw `next/link` imports bypass `@/i18n/routing` Link, re-resolving href to default locale on every nav. _Mitigation: promote `code-review-reality-web-account-pages-no-i18n` to a plan; one PR sweeps all four pages._

## Release readiness signal

**amber** — delivery is healthy (49/49 stories done; 15 PRs merged in 72h) but
15 open code-review-finding items + 2 CI-blocking infra issues (mobile-native/KMP
#2652, RN jest-expo #2951) mean no clean release gate can run without a human
sign-off on the amber-but-shipping state.
