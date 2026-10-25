# PPT Roadmap — upkeep 2026-10-25

_Rebuilt from `coverage.json` (49/49 stories done across 13 epics) + current `action-list.json` + sprint-status.yaml. 2-day lag since 2026-10-23 run._

## State of the project

- **Stories: 49/49 done** across 13 epics. No status flips this run; `last_checked` refreshed on all 3 epic-8a stories (cursor idx 11; negative re-check — no PR touched notification surfaces).
- Delta vs 2026-10-23 upkeep: **8 PRs merged** (fat window) + **4 issues closed** (#3132/#3136/#3143/#3149). But merge-pipeline deadlock #3131 (16 WIP slots on human-gated PRs in review → claimable=8/72, buffer-low) means zero queue progress is actually available until reviewer rotation lands.
- Cargo-deny gate effectively cleared by #3148 (ammonia RUSTSEC guard). Carried: #2966 (swagger-ui egress), #2652 (KMP cloud builds).
- Hotfix #3145 (push-fanout surface errors) shipped WITHOUT a regression test — repeat of the 2026-07-30 pattern.
- Screen coverage: 0 orphan screens · 0 validation errors · 3 known missing UC links (UC-33.1/33.2/33.3 already queued elsewhere).
- 15 open PRs in `review` status with ZERO reviewer touches: #3024, #3028, #3070, #3072, #3074, #3075, #3107, #3108, #3111, #3120, #3152, #3153, #3154, #3155, #3158.

## Epic status (short)

- **Epic 6 — Announcements & Communication:** 6/6 done in coverage.
- **Epic 7A — Basic Document Management:** 5/5 done.
- **Epic 8A — Basic Notification Preferences:** 3/3 done. **Re-checked this run (cursor idx 11).** `last_checked = 2026-10-25` on all 3.
- **Epic 10A — OAuth Provider Foundation:** 3/3 done.
- **Epic 10B — Platform Administration:** 7/7 done.
- **Epic 80 — Dispute Resolution:** 3/3 done in coverage (sprint-status still shows partial).
- **Epic 81 — Reports:** 2/2 done.
- **Epic 82 — iOS Reality Portal (SwiftUI):** 5/5 done.
- **Epic 83 — Portal Integrations:** 3/3 done.
- **Epic 84 — Documents & e-signature:** 5/5 done. (#3147 migration-import error-handling evidence adjacent.)
- **Epic 85 — Build/CI configuration:** 2/2 done. (#3148 ammonia RUSTSEC guard evidence adjacent.)
- **Epic 79 — API client integration:** 4/4 done. (#3156 shared bearerAuthHeader evidence adjacent — closes the long-carried #3006-class refactor.)
- **Epic 9 — 2FA/TOTP:** 1/1 done.

## Ranked plan

### merge-queue unblock (highest leverage, score 10)

- [high] **Resolve #3131 merge-pipeline deadlock** — self-approval exception for research-dispatcher PRs OR human reviewer rotation — gates every new dispatcher claim — owner: **pm-tech-lead**
- [high] **Clear 15 review-stuck PRs** (#3024, #3028, #3070, #3072, #3074, #3075, #3107, #3108, #3111, #3120, #3152, #3153, #3154, #3155, #3158) — bulk-assign reviewers by owner_role — depends on #3131 — owner: **pm-tech-lead**
- [high] **Unblock #2966 utoipa-swagger-ui egress** for api-server cloud builds — gates every backend build — owner: **pm-devops** (carried)
- [high] Unblock mobile-native/KMP builds in cloud runner (**#2652**) — owner: **pm-devops** (carried)

### regression-test backfill (score 8, pm-backend)

- [high] **Backfill test for PR #3145 push-fanout error-surface hotfix** — exercise success + Redis-publish-failure branches — owner: **pm-backend**

### cross-cutting hygiene (score 6-7, pm-backend)

- [medium] Fast-lane PR #3158 (moderation sub-paths follow-up for #3157) — owner: **pm-backend**
- [medium] Audit PR #3147 migration-import retry/rollback correctness — owner: **pm-backend**
- [medium] Extract shared `sanitizeAgencyHtml()` helper — agency/[slug]/page.tsx twice this window — owner: **pm-backend** (dep: pm-tech-lead shared-crate design)

### security tier-1 (score 7, all blocked on #3131 and the carried gates)

- [high] Land #2946 fix (portfolio_analytics IDOR) — retry 2/2 draft — blocked by #3131 + #2966 — owner: **pm-security**
- [high] Land #2944 fix (violation comments/evidence/payments IDOR + internal-notes leak) — retry 1/2 — blocked by #3131 + #2966 — owner: **pm-security**
- [high] Land #2945 fix (portfolio_properties IDOR) — retry 1/2 — blocked by #3131 + #2966 — owner: **pm-security**

### reviewer-starvation (score 5-6)

- [medium] Reviewer slot for stalled accounting trio **#2559** (PAY by Square) and **#2555** (invoice lifecycle) — 85+ days idle — owner: **pm-tech-lead**

### pm-integration follow-ups (carried, score 4-5)

- [medium] Audit integration-touching PRs since 2026-10-17 for OpenAPI/TypeSpec drift — owner: **pm-integration** (carried)
- [low] Document the reality-server SSO consumer transient-401 contract (PR #3121) — owner: **pm-integration** (carried)

### Follow-up issues (score 4-5)

- [medium] gh-issue-3069 (ViolationRepository org-scoping IDOR follow-up) — owner: **pm-tech-lead**
- [medium] gh-issue-3098 (portfolio_benchmarks schema drift + BIT-622 test refs) — owner: **pm-tech-lead**
- [medium] gh-issue-3104 (portfolio/owner analytics 500s runtime drift) — owner: **pm-tech-lead**
- [medium] gh-issue-3109 (api-server DELETE /community/groups/{id} end-to-end) — owner: **pm-tech-lead**
- [medium] gh-issue-3116 (NewsScreen i18n defaultValue fallback) — owner: **pm-tech-lead**
- [medium] gh-issue-3157 (moderation sub-paths — PR #3158 drafted) — owner: **pm-tech-lead**

### quality / RN lint (score 4-5, carried)

- [low] Resolve mobile RN jest-expo version rot (**#2951**) — owner: **pm-devops**
- [low] Confirm ammonia RUSTSEC exception cleanup after #3148 — owner: **pm-devops**
- [low] Audit other reality-server anonymous-GET caches for the unbounded-key pattern #3146 fixed — owner: **pm-backend**

### KMP code-review backlog (score 2-3, all blocked on #2652)

- code-review-mobile-native-kmp-create-listing-not-wired
- code-review-mobile-native-kmp-inquiries-response-contract

### carried code-review backlog (score 2-3)

- code-review-reality-server-add-to-compare-toctou-max-cap
- code-review-reality-server-register-account-enumeration
- code-review-api-handlers-violations-{missing-role-authz, number-count-race, record-payment-non-atomic}
- code-review-api-core-workflow-condition-err-aborts-batch
- code-review-ppt-web-core-{iot-ws-reconnect-no-backoff, usewebsocket-subscribe-gate}
- code-review-reality-web-{agency-page-i18n-gap, crm-console-error-leak, realtor-api-unvalidated-json-cast}
- screen-map-drift-pr-2994-ppt
- stalled-review-pr-3024-retry1

Buffer: **32/36 open** (floor 36) · candidates remaining ranked but unqueued: ~0 — queue is fed; the real blocker is review-side (#3131), not planner-side. Buffer-low signal in Phase 1 payload refers to the dispatcher's `claimable()` predicate (8/72), which collapses to the merge-pipeline deadlock. Unblocking #3131 restores claim-flow without a scan refresh being needed.
