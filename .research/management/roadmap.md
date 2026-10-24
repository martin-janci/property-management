# PPT Roadmap — upkeep 2026-10-19

_Rebuilt from `coverage.json` (49/49 stories done across 13 epics) + current `action-list.json` + sprint-status.yaml. 3-day lag since 2026-10-16 run._

## State of the project

- **Stories: 49/49 done** across 13 epics. No status flips this run; `last_checked` refreshed on 10a-1 (PR #3121 evidence) and all 5 epic-84 stories (cursor re-check — negative).
- Delta vs 2026-10-16 upkeep: **2 PRs merged** — both frontend code-review follow-ups (reality-web auth-ctx transient-logout cushion + ppt-web AML dashboard flags), both tests-included, both resolving prior backlog items.
- **Buffer now doubly constrained:** #2966 (swagger-ui egress) gates backend builds; **#3124 cargo-deny** (new, RUSTSEC-2026-0368/0369 on zip 8.6.0 via rust_xlsxwriter) gates backend merges even on green builds; #2652 (KMP) continues to park mobile-native work.
- Screen coverage: 0 orphan screens · 0 validation errors · 3 known missing UC links (UC-33.1/33.2/33.3 already queued elsewhere).
- Open PRs: 1 draft (#3123 api-core TOTP RNG expect panic — no contract impact). The accounting stall trio (#2559, #2555) remains carried.

## Epic status (short)

- **Epic 6 — Announcements & Communication:** 6/6 done in coverage; sprint-status.yaml drift (safe to re-reconcile next deep scan).
- **Epic 7A — Basic Document Management:** 5/5 done.
- **Epic 8A — Basic Notification Preferences:** 3/3 done.
- **Epic 10A — OAuth Provider Foundation:** 3/3 done. PR #3121 adds SSO-consumer-side evidence on 10a-1 (reality-web auth-context transient-logout cushion).
- **Epic 10B — Platform Administration:** 7/7 done.
- **Epic 80 — Dispute Resolution:** 3/3 done in coverage (sprint-status still shows partial).
- **Epic 81 — Reports:** 2/2 done.
- **Epic 82 — iOS Reality Portal (SwiftUI):** 5/5 done.
- **Epic 83 — Portal Integrations:** 3/3 done.
- **Epic 84 — Documents & e-signature:** 5/5 done. **Re-checked this run (cursor idx 9).** No merged PR in the 2026-10-18..2026-10-19 window touched epic-84 surfaces; `last_checked = 2026-10-19` stamped on all 5 stories.
- **Epic 85 — Build/CI configuration:** 2/2 done.
- **Epic 79 — API client integration:** 4/4 done.
- **Epic 9 — 2FA/TOTP:** 1/1 done.

## Ranked plan

### merge-queue unblock (highest leverage, score 10)

- [high] **Resolve #3124 cargo-deny advisories** (RUSTSEC-2026-0368 / RUSTSEC-2026-0369 on zip 8.6.0 via rust_xlsxwriter) — bump the version or add a bounded cargo-deny exception with deadline — gates every backend merge — owner: **pm-security**
- [high] **Unblock #2966 utoipa-swagger-ui egress** for api-server cloud builds — gates every backend build — owner: **pm-devops**
- [high] Unblock mobile-native/KMP builds in cloud runner (**#2652**) — 5+ KMP items structurally unclaimable — owner: **pm-devops**

### security tier-1 (score 8-9, all blocked on the merge-queue above)

- [high] Land #2946 fix (portfolio_analytics IDOR) — retry 2/2 draft open as `gh-issue-2946-retry2` — blocked by #2966 **AND** #3124 — owner: **pm-security**
- [high] Land #2944 fix (violation comments/evidence/payments IDOR + internal-notes leak) — retry 1/2 — blocked by #2966 + #3124 — owner: **pm-security**
- [high] Land #2945 fix (portfolio_properties IDOR) — retry 1/2 — blocked by #2966 + #3124 — owner: **pm-security**

### refactor-with-security-overtones (score 6)

- [medium] #3006 — unify triplicated auth-header + MFA-retry in @ppt/api-client fetch primitives — owner: **pm-tech-lead** (carried)

### reviewer-starvation (score 5-6)

- [medium] Reviewer slot for stalled accounting trio **#2559** (PAY by Square) and **#2555** (invoice lifecycle) — 67d idle — owner: **pm-tech-lead**

### pm-integration follow-ups (new this run, score 4-5)

- [medium] Audit integration-touching PRs since 2026-10-17 for OpenAPI/TypeSpec drift — owner: **pm-integration**
- [medium] Surface the RUSTSEC-exception policy in the cargo-deny CI gate description so #3124-class blockers triage faster next time — owner: **pm-integration** (depends on #3124)
- [low] Document the reality-server SSO consumer transient-401 contract vs reality-web auth-context cushion (PR #3121) in docs/api/README.md — owner: **pm-integration**

### quality / RN lint (score 4-5, carried)

- [medium] Resolve mobile RN jest-expo version rot (**#2951**) — owner: **pm-devops**

### KMP code-review backlog (score 2-3, all blocked on #2652)

- InquiriesResponse page_size vs reality-server `limit` mismatch
- Ktor HttpClient has no HttpTimeout
- CancellationException swallowed in repos
- getPortfolioAnalytics() 100-cap + unbounded fan-out
- CreateListingScreen onSubmit stub
- SsoService untested

### carried refactors (score 2-3)

- reality-server duplicated URL-validation helper — merge into shared crate
- api-server routes/integrations/webhook.rs repeated-churn (3 hotspot windows)
- logout() AUTHED_QUERY_KEY_ROOTS allow-list gap (session leak)

Buffer: **26/36 open** (floor 36) · candidates remaining ranked but unqueued: ~10. Buffer below floor by 10 but above half; the twin merge-gate (#2966 + #3124) is still the top lever — unblocking either frees the IDOR tier back onto the runner.
