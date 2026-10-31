# pm-scrum-master — 2026-10-31

_Routine Phase 1.6 run (72h window since 2026-10-28). pm_cursor idx 3 → 4 (pm-qa also ran this rotation slot)._

## Summary

15 PRs merged in 72h — a dispatcher-driven wave of code-review fixes
(ppt-web i18n, listing-edit price-zero, Toast cleanup, OS-RNG panics,
cross-tenant IDOR). All 13 epics remain 49/49 done; the project is
operating in maintenance mode with the dispatcher consuming the backlog.
The buffer-low trigger payload (`claimable=10/72`) reflects a stall
between dispatcher consumption and gap-scan refill, not a delivery blocker.

## Sprint progress

- Sprint: **Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth**
- Sprint epics done: 4/4
- Coverage epics done: 13/13 (49/49 stories)
- Coverage status flips this run: 0 (upkeep only — epic-10a OAuth re-checked, no change)

## Shipped since last run (15 PRs)

- **#3188** `fix(UC-violations):` assign_violation org-scoped 404 before assignee gate (closes #3187 blocker)
- **#3198** reality-server os-rng panics — don't panic on OS RNG failure in SSO + password-reset
- **#3197** reality-web listing-edit price-zero — reject empty/invalid listing price
- **#3203** reality-web listing-edit i18n-sk — localize wizard across all locales
- **#3202** ppt-web invoice vat-rate payload-drift — normalize item.vatRate
- **#3201** ppt-web ConnectionStatus div-as-button-role — make reconnect a real button
- **#3200** ppt-web Toast setstate-after-unmount — clear copy timer on unmount
- **#3199** reality-web comparison-parse-shape — guard localStorage against malformed shape
- **#3191** ppt-web invoice qty/price NaN guard
- **#3190** ppt-web authed-roots session-leak — enforce logout-purge coverage for personMonthKeys
- **#3189** mobile-rn vote-detail ranked-ordered ballot
- **#3186** ppt-web iot-ws hasgivenup — wire give-up state into dashboard UI
- **#3185** ppt-web neighbors stale-token-usememo
- **#3204** api-server compliance wave-1b regression coverage (churn hotspot)
- **#3205** reality-server reports status-filter-unvalidated

## Next actions (top 3)

1. **medium** — QA sweep: regression test for new reality-web account/profile/password i18n gap (same bug class as #3203 — four pages still English-only). _owner: pm-frontend_
2. **medium** — Investigate buffer-low: dispatcher claimable=10/72 — gap-scan to replenish or purge UNRESOLVED-sentinel items. _owner: pm-scrum-master_
3. **low** — Close out #3171 (add_evidence manager gate) — PR open 1d, no reviewer yet. _owner: pm-backend_

## Blockers

- **mobile-native/KMP cloud builds (#2652)** — blocks dispatcher claim pool. _owner: pm-devops_
- **React Native jest-expo cloud harness (#2951)** — mobile-rn cloud-verify-blocked. _owner: pm-devops_

## Open questions

- Buffer-low trigger (`claimable=10/72`) — is this a dispatcher cursor stall or genuine depletion? Investigate before next scan.
