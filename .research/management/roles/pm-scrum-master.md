# pm-scrum-master — 2026-11-22

_Phase 1.6 — static, read-only. Cloud routine run._

## Summary

26 PRs merged in-window (#3687-#3712) — a frontend-heavy wave (WS token-leak, invoice empty-items, admin console contract, logout cache purge, 2 i18n i18n drafts landed) plus three pm-integration/pm-scrum-master follow-ups from the 2026-11-21 action list. Delivery sprint remains `done` (hardening track active); 5 drafts open, 1 new follow-up issue (#3711) filed.

## Next actions

| Priority | Action | Dependency | Definition of done |
|---|---|---|---|
| high | Land the 2026-12-01 i18n warn→error lint-flip burn-down: merge or close the 10+ open `code-review-*-i18n-retry1` items + drafts #3676/#3677 within 9 days | pm-frontend | frontend/apps/mobile no-hardcoded-strings = 0 warnings on dev; lint flipped warn→error without CI break |
| high | Resolve dispatcher stall #3357 — 3 approved PRs still blocked on non-mechanical conflicts across TWO merge waves (32 + 26 PRs) | pm-tech-lead | #3357 closed; dispatcher backlog claimable > 20/72 |
| high | Unblock mobile-native/KMP + swagger-ui egress in the cloud runner (#2652, #2966) — 4 KMP backlog items still structurally unclaimable | pm-devops | cloud runner can `./gradlew build` KMP targets; backend compiles with swagger-ui dist |
| medium | Characterize the post-split push/ + voice_webhooks/ + oauth_integration_tests/ child modules (watch-* in current_sprint) and promote to specific refactor tasks | pm-backend | each watch-* item either has a concrete split task or is closed as sized-OK |
| medium | Triage new issue #3711 (dispatcher signals-promote dev-truth guard follow-up) into the dispatcher-reliability track | pm-tech-lead | #3711 has an owner, a plan, and lands or closes within one sprint-week |
| medium | Promote-or-close the 5 draft PRs (#3713 dispatcher dev-truth guard, #3714 ppt-web date/number locale, #3715 ppt-web native confirm dialogs, #3676/#3677 mobile-rn i18n) | pm-scrum-master | each draft is merged, converted to ready-for-review, or closed with a note |

## Risks

| Prob | Impact | Risk | Mitigation |
|---|---|---|---|
| high | high | The 2026-12-01 mobile i18n lint flip (warn→error) is 9 days out and there are still 10+ open `code-review-*-i18n-retry1` items plus 2 open draft i18n PRs — CI will break the moment the flip lands if any slip | Daily burn-down count on the i18n action-list slice; promote drafts aggressively; pre-flip gate: `pnpm --filter mobile lint --no-warn` must exit 0 on dev for 48h |
| high | medium | Dispatcher stall #3357 persisting across two merge waves (58 + 26 = 84 PRs) suggests the blocker is not merge-wave dilution — likely a conflict pattern the rebaser cannot auto-resolve | Human-driven review of the 3 blocked PRs; publish a rebase-pattern note; treat remaining instances as `rebase-poisoned` and close/reopen |
| medium | medium | 5 open drafts + new issue #3711 means the dispatcher pipeline still has latent mass; another day without draft-close policy creates a PR-review starvation signal | 72h draft-close policy decision from 2026-11-21 (DEC) must be landed as a dispatcher-reconcile rule |

## Open questions

- Should the dispatcher 'promote-or-close drafts >72h' rule be a hard gate (auto-close) or a soft signal (Telegram alert)?
- Can the 2026-12-01 i18n lint flip slip by one sprint if any of the 10+ retry1 items fail CI on the flip day?

## Decisions needed

- Harden or soften the 72h draft close policy — owner: pm-scrum-master
- Decide whether to defer or hold the 2026-12-01 mobile i18n lint flip — owner: pm-frontend + pm-qa
