# pm-scrum-master — 2026-11-23

_Phase 1.6 — static, read-only. Cloud routine run._

## Summary

24 PRs merged in-window (#3713–#3737) — overwhelmingly dispatcher output: 11 `code-review-*` fixes across api-handlers/api-core/ppt-web-ui/reality-server, 5 `pm-frontend/pm-backend/pm-scrum-master` carried actions closing out, 2 legitimate bugfixes (#3723 ppt-web mediation-sessions swallowed error surfaced, #3725 FCM 2xx body-parse → delivered), 1 feature (#3727 operator roster endpoint), 1 screen-doc (#3728 lease-create reconcile). The 2026-11-21 dispatcher stall #3357 unblocked via #3718 (key-union i18n merge-conflict resolver). Delivery sprint remains `done`; hardening track active. 8 drafts open (7 i18n-flip-adjacent, all updated 2h-4h ago); 1 open follow-up issue #3738 (SSO sessions sort hot-path, PR #3744 already drafted). i18n warn→error flip T-8d.

## Next actions

| Priority | Action | Dependency | Definition of done |
|---|---|---|---|
| high | i18n warn→error flip countdown 8d (2026-12-01). Promote-or-close drafts #3740/#3741 (reality-web locale), #3739 (realtor/[id] parity), #3742 (ppt-web Registry Rules), #3676 (mobile-rn raw status). | pm-frontend | All i18n-flip drafts either merged, converted, or formally deferred by 2026-11-29 |
| high | Resolve dispatcher stall #3357 — reviewer-approved PRs still blocked across two merge waves (82+ PRs). #3718 landed the mechanical resolver; non-mechanical conflicts remain. | pm-tech-lead | #3357 closed; non-mechanical conflicts manually rebased OR closed with a note |
| high | Unblock mobile-native/KMP + swagger-ui egress in the cloud runner (#2652, #2966) — 4 KMP backlog items still structurally unclaimable | pm-devops | cloud runner can `./gradlew build` KMP targets; backend compiles with swagger-ui dist |
| medium | Nudge draft #3676 (code-review-mobile-rn-raw-status-untranslated) — not a draft but open for 22h with no reviewDecision. | pm-frontend | #3676 merged or ready-for-review |
| medium | Follow-up #3738 (SSO pending-session O(n) sort under flood) — PR #3744 is already drafted; verify it ships before the 48h stall threshold. | pm-backend | #3744 merged or formally deferred |

## Risks

| Prob | Impact | Risk | Mitigation |
|---|---|---|---|
| high | high | 2026-12-01 mobile i18n lint flip T-8d with 7 open drafts all moved in the last 4h — dispatcher is pushing drafts faster than they land. CI will break on any slip past 2026-11-29. | Daily burn-down count; hard promote-or-close by 2026-11-29 (one weekend of drainage before flip); pre-flip gate: `pnpm --filter mobile lint --no-warn` must exit 0 on dev for 48h. |
| medium | medium | Dispatcher queue is draining faster than refills can land (45 open / 49 after refill vs ≥36 target; sustainable). But 7/8 open PRs are drafts — if one lands clean and 6 CI-fail, pipeline starves overnight. | Monitor draft→merged conversion ratio; auto-wake if drafts ≥48h old count exceeds 3. |
| medium | medium | FCM/worker/push stack has three untested single-file source-only fixes merged this window (#3725/#3726/#3731). Any regression in production catches no automated signal. | pm-qa's test-shadow burn-down (now in action-list); push/* lint-or-review rule under decisions. |

## Open questions

- Is the 24-PR window representative throughput or a backlog-drain burst? (Previous window: 26; this one: 24 — appears steady.)
- Should the dispatcher push `#[cfg(test)]`-less PRs touching `push/*` into a review-mandatory queue?

## Decisions needed

- Promote-or-close deadline for the 7 draft i18n PRs: set 2026-11-29 as the hard date to leave a weekend of CI drainage before the 2026-12-01 lint flip — owner: pm-scrum-master + pm-frontend.
- Mandate a `#[cfg(test)]` block (or paired `*_tests.rs` diff) for every PR touching `push/apns.rs`, `push/fcm.rs`, or `push/worker.rs` — owner: pm-qa + pm-backend.
- Investigate the 50% no-test-diff rate on this window's merged PRs — owner: pm-qa.
