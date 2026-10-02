# PPT Roadmap — upkeep 2026-10-01

_Rebuilt from `coverage.json` (49/49 stories done across 13 epics) + current `action-list.json` + sprint-status.yaml. 4-day lag since 2026-09-27 run._

## State of the project

- **Stories: 49/49 done** across 13 epics. The last 2 partial stories (84-1 direct-to-S3 wiring, 84-2 sign page) flipped to done in prior windows; MVP delivery stays at full.
- Delta vs 2026-08-31 upkeep: **7 PRs merged** this observation window since #2991 — a mix of defense-in-depth RBAC hardening (#2994, #2995, #3002), SSO i18n (#2993), one bug boundary fix (#2996), one mobile feature wiring (#3001), and one cross-cutting auth-client refactor (#3004). Zero coverage status flips (all surfaces already done); several `last_checked` stamps refreshed.
- **Buffer-low signal (new): `claimable=9/72`** — root cause is #2966 (utoipa-swagger-ui egress gating all backend PRs) + #2652 (KMP cloud builds still blocked). Every backend security retry (#2944/#2945/#2946) is parked behind the egress fix. Not a backlog-size problem.
- Screen coverage: 0 orphan screens · 0 validation errors · 3 known missing UC links (UC-33.1/33.2/33.3 already queued elsewhere).
- Open PRs: 26 total; 11 stalled including dependabot bulk. 2 non-dependabot stalled are the accounting trio (#2559, #2555) at 64d.

## Epic status (short)

- **Epic 6 — Announcements & Communication:** 6/6 done in coverage; sprint-status still shows in-progress (sprint-status.yaml drift — safe to re-reconcile next deep scan).
- **Epic 7A — Basic Document Management:** 5/5 done.
- **Epic 8A — Basic Notification Preferences:** 3/3 done.
- **Epic 10A — OAuth Provider Foundation:** 3/3 done. PR #2993 touched the SSO consumer locale pipeline (reality-web side).
- **Epic 10B — Platform Administration:** 7/7 done. PR #2994 + #3002 hardened dashboard RBAC on top of the shipped admin surface.
- **Epic 80 — Dispute Resolution:** 3/3 done in coverage (sprint-status still shows partial).
- **Epic 81 — Reports:** 2/2 done. Last checked 2026-08-31.
- **Epic 82 — iOS Reality Portal (SwiftUI):** 5/5 done. **Re-checked this run (cursor idx 7).** No merged PR in the 2026-09-28..10-01 window touched iOS SwiftUI surfaces; `last_checked = 2026-10-01` stamped on all 5 stories.
- **Epic 83 — Portal Integrations:** 3/3 done.
- **Epic 84 — Documents & e-signature:** 5/5 done.
- **Epic 85 — Build/CI configuration:** 2/2 done.
- **Epic 79 — API client integration:** 4/4 done. PR #3004 (gdprClient + paymentMatching migrated onto shared auth) is evidence on 79-1.
- **Epic 9 — 2FA/TOTP:** 1/1 done.

## Ranked plan

### infra unblock (highest leverage, score 10)

- [high] Unblock utoipa-swagger-ui egress for api-server cloud builds (**#2966**) — gates every backend PR, including all 3 IDOR retries — owner: **pm-devops**
- [high] Unblock mobile-native/KMP builds in cloud runner (**#2652**) — 5+ KMP items structurally unclaimable — owner: **pm-devops**

### security tier-1 (score 8-9)

- [high] Land #2946 fix (portfolio_analytics IDOR) — retry 2/2 draft open as `gh-issue-2946-retry2` — blocked by #2966 — owner: **pm-security**
- [high] Land #2944 fix (violation comments/evidence/payments IDOR + internal-notes leak) — retry 1/2 — blocked by #2966 — owner: **pm-security**
- [high] Land #2945 fix (portfolio_properties IDOR) — retry 1/2 — blocked by #2966 — owner: **pm-security**

### refactor-with-security-overtones (score 6)

- [medium] #3006 — unify triplicated auth-header + MFA-retry in @ppt/api-client fetch primitives (post-merge review of PR #3004) — owner: **pm-tech-lead**

### reviewer-starvation (score 5-6)

- [medium] Reviewer slot for stalled accounting trio **#2559** (PAY by Square) and **#2555** (invoice lifecycle) — 64d idle — owner: **pm-tech-lead**

### quality / RN lint (score 4-5)

- [medium] Resolve mobile RN jest-expo version rot (**#2951**) — unblocks RN code-review items — owner: **pm-devops**
- [medium] Adopt eslint-plugin-react-hooks + no-hardcoded-strings in frontend/apps/mobile — owner: **pm-frontend**

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

Buffer: **9 claimable / 72 open** (floor 36) — buffer-low. Top lever is **#2966 unblock**; everything downstream of backend builds is parked behind it.
