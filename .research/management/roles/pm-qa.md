# pm-qa

_Generated: 2026-11-03T20:36:04Z_

## Summary

First pm-qa run in 70 days. Active sprint is fully delivered (4/4 epics done); backend CI is RED on dev from #3187 and must clear before any backend merge. This run's 7 merged PRs mostly added appropriate regression tests, but PR #3220 (community-feature i18n, 23 files / 3563 LOC) added only 1 i18n-key test for the coverage breadth, and the fresh #3223 follow-up surfaces a cross-boundary test gap the mobile code already exposes (English-sentence coupling).

## Next actions
- [high] **Add a cross-boundary test pinning the three Rust `repositories/vote.rs` eligibility-reason literals against the mobile `ELIGIBILITY_REASON_KEYS` map so a backend copy-edit fails CI (issue #3223 interim mitigation)** — dep: `none` — DoD: Mobile test imports or inlines the three backend literals and asserts they are keys of ELIGIBILITY_REASON_KEYS; fails before any copy edit to vote.rs and passes against the current strings
- [high] **Resolve backend CI #3187 — either backend fix or test shape adjustment — so backend.yml goes green on dev and unblocks every backend PR** — dep: `pm-security` — DoD: backend.yml green on dev; cross_org_assign_violation_is_not_found passing
- [medium] **Extend PR #3222's VoteDetailScreen.test.tsx eligibility-reason branch to cover all three mapped reasons (today only `voteNotAcceptingBallots` is covered)** — dep: `none` — DoD: Three test cases, one per mapped reason, asserting both the t() key and the rendered fallback
- [medium] **Add at least one locale-switching smoke test for the community pages (ppt-web EventCard or MarketplacePage) — PR #3220 ships 3563 LOC with only one i18n-key test; a locale switch can regress silently via hard-coded strings reintroduced** — dep: `none` — DoD: A locale-switch test renders at least one community page in `sk` and asserts a non-English key label
- [medium] **Resolve #2951 jest-expo version rot to unlock mobile RN test harness upgrades (carried)** — dep: `pm-devops` — DoD: jest-expo pinned; mobile test suite runnable in cloud sandbox
- [medium] **Audit the four open draft PRs (#3224-#3227) for regression tests on the fix path before merge — three of them target silent-UX-failure bugs where tests would otherwise never catch regressions** — dep: `none` — DoD: Each draft either carries a regression test or is annotated with a test-gap waiver + test follow-up issue

## Risks
- [medium/high] **Backend CI red on dev means test gate has been de facto bypassed on recent merges (merges happen on green frontend but backend red) — regression coverage asymmetric between stacks** — mitigation: Confirm no backend merges have landed since 2026-10-28 while CI is red; if any have, open a post-merge review issue per PR
- [medium/medium] **Mobile RN jest-expo rot means mobile regression is detected only by snapshot/branch tests — subtle behavior regressions slip through** — mitigation: Pin jest-expo to the compatible Expo 55 version and re-run full RN suite as a smoke pass
- [high/medium] **pm-qa rotation at 70 days stale let the test-gap cluster grow unseen — if the same pattern recurs (rotation idle while shipping continues), we accumulate uncovered defects faster than we find them** — mitigation: Mandatory per-30-day rotation floor in routine Phase 1.6
- [medium/medium] **Reality-web open drafts #3225 (mock agent profile) and #3226 (feed silent-fail) address user-visible bugs with no regression tests in backlog — merging them without tests leaves the bug class open** — mitigation: Block merge pending at least one regression test per draft; or file a parallel test-gap follow-up

## Open questions
- Is any backend PR expected to merge while #3187 is unresolved? If so, how are we gating?

## Decisions needed
- Should pm-qa rotation have a hard staleness floor (e.g. 30 days) that overrides the quiet-day Phase 1.6 skip — owner: pm-scrum-master
