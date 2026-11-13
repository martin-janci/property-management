# pm-scrum-master — 2026-11-13

## Summary
Dispatcher pipeline went from stalled (10+ zero-merge runs per the 2026-11-09 brief) to a sustained burst: 58 PRs merged in 4 days, closing 3 follow-up issues and 2 action-list rows. Routine scheduler itself drifted 4 days — the new coordination concern.

## Next actions
1. **[high]** Triage open security issue #3427 (`Harden first-party JWT: emit + validate iss/aud across api-server & reality-server`, labels: `security`/`follow-up`/`blocked`). Confirm whether `blocked` is a real dependency or just awaiting prioritisation; either promote to a plan or defer with justification. DoD: issue #3427 has a `next-step` comment or a linked plan under `.research/plans/`.
2. **[medium]** Watch the 3 dispatcher drafts #3475/#3476/#3477 land. DoD: all 3 un-draft and merge, or get promoted to review and reassigned. _(No direct action — this is a monitoring note.)_
3. **[medium]** Close out #3357 (dispatcher-stall operator ask) as self-resolved by the 58-PR burst. DoD: issue closed with a one-line explanation + link to this run's project-state.

## Risks
- **Routine scheduler drift** (medium/medium) — flagged by pm-devops; blocks delivery visibility.
- **Dispatcher stall recurrence** (medium/high) — root cause of #3357 (dispatcher cannot self-heal from blocked PRs) is unchanged even though the symptom cleared.

## Open questions
- What's the correct treatment of the `blocked` label on #3427 — was there an upstream dep that still hasn't landed, or is the label stale?

## Decisions needed
- **Lag-floor policy** for the research routine (see pm-devops). Owner: pm-devops + pm-scrum-master.

## Shipped since last run (sample — top of 58)
- #3473 mobile-rn-features-faults-orphaned (cleanup)
- #3474 mobile-rn-person-months-no-i18n (i18n)
- #3472 gh-issue-3466 critical/outages i18n for de/pl/hu
- #3471 ppt-web-ui toast persistent-overflow leak
- #3454 mobile-rn notifications still-mock → wired to real inbox API
- #3463 ppt-web-ui 2fa QR silent fail → surfaced
- #3451 mobile-rn profile local-only → persisted to api-server
- #3424 reality-server favorites cap TOCTOU → atomic
- #3410 api-core CORS localhost-prod-default → hardened
- #3416 api-core push-fanout dropping transient failures → retried

## Sprint progress
49/49 stories done across 13 epics. No status flips. epic-10b re-checked (upkeep).

## Blockers
- **mobile-native-kmp cloud-landability starvation** (carries indefinitely; owner: pm-devops).
- **routine scheduler 4-day drift** (flagged this run; owner: pm-devops).
