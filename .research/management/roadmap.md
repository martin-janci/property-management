# PPT Roadmap — upkeep 2026-11-08

_Rebuilt from `coverage.json` (49/49 stories done across 13 epics) + current `action-list.json` + sprint-status.yaml. 2-day lag since 2026-11-06 run. pm-qa rotation slot (idx 3 of 8) is today's role focus._

## State of the project

- **Stories: 49/49 done** across 13 epics. No status flips this run; `last_checked` refreshed on all 3 epic-10a stories (cursor re-check — negative; PR #3353 touches password_reset.rs, not the OAuth authorization server).
- Delta vs 2026-11-06: **5 PRs merged** — all small follow-ups against already-shipped surfaces. Breakdown: 1 security (#3353 session revocation on change_password, closes #3352), 1 boundary-validation (#3350 shared-comparison body parse), 2 i18n (#3351 + #3355 registry), 1 auth-context (#3354 registry). None map to coverage stories.
- **New dominant blocker: #3357 dispatcher stall** — pre-run 0 claimed / 0 merged; Tier-1d signals write-and-forget pipeline broken (7 orphaned signals from 2026-11-07); 3 approved PRs blocked on non-mechanical conflicts. Everything downstream is gated on a non-zero claimable pool.
- **Closed-not-merged orphans this run**: #3301, #3333, #3356 (dispatcher cleanup due to orphaned dev history). #3356 orphan leaves `code-review-ppt-web-ui-registry-mutation-silent-fail` unfixed in dev — flipped back to `open`.
- Screen coverage: 0 orphan screens · 0 validation errors · 3 known missing UC links (UC-33.1/33.2/33.3 already queued elsewhere).

## Epic status (short)

- **Epic 6 — Announcements & Communication:** 6/6 done.
- **Epic 7A — Basic Document Management:** 5/5 done.
- **Epic 8A — Basic Notification Preferences:** 3/3 done.
- **Epic 10A — OAuth Provider Foundation:** 3/3 done. **Re-checked this run (cursor idx 0).** No merged PR in the 2026-11-07..2026-11-08 window touched OAuth authorize/token/introspect/revoke surfaces; `last_checked = 2026-11-08` stamped on all 3 stories.
- **Epic 10B — Platform Administration:** 7/7 done.
- **Epic 80 — Dispute Resolution:** 3/3 done.
- **Epic 81 — Reports:** 2/2 done.
- **Epic 82 — iOS Reality Portal (SwiftUI):** 5/5 done.
- **Epic 83 — Portal Integrations:** 3/3 done.
- **Epic 84 — Documents & e-signature:** 5/5 done.
- **Epic 85 — Build/CI configuration:** 2/2 done.
- **Epic 79 — API client integration:** 4/4 done.
- **Epic 9 — 2FA/TOTP:** 1/1 done.

## Ranked plan

### pipeline unblock (highest leverage, score 10 — #3357)

- [high] **Operator-unblock dispatcher stall (#3357)** — planner refresh with non-mobile-native vectors OR clear the 2 UNRESOLVED depends_on sentinels (pm-qa/pm-security role-token poison); pipeline is pinned at 0 claimed / 0 merged until this moves — owner: **pm-scrum-master**
- [high] **Wire tier-1d signal ingestion OR stop emitting** — 7 orphaned signals from 2026-11-07 are a write-only loop; add CI/cron guard failing on >48h stale signals and either ingest or disable — owner: **pm-qa** / **pm-devops**

### merge-queue unblock (highest leverage, score 10 — carried)

- [high] **Resolve #3168 cargo-deny advisories** (3 new RUSTSEC advisories on dev) — bump the offending transitive crate(s) or add bounded cargo-deny exceptions with a deadline — gates every backend merge today — owner: **pm-security** (carried)
- [high] **Fix #3187 cross-tenant assign-404 contract** (gates backend CI on dev) — owner: **pm-security** (carried)
- [high] Unblock mobile-native/KMP builds in cloud runner (**#2652**) — 5+ KMP items structurally unclaimable — owner: **pm-devops** (carried)

### tier-1 security (score 8-9)

- [high] **Add multi-session revocation regression test for PR #3353 surface** — assert OTHER refresh tokens revoked while CURRENT session preserved on /auth/change-password; close the "did not log me out of my own browser tab" failure mode — owner: **pm-qa** (new)
- [high] Gate violations `add_evidence` sub-resource (**#3164**) with tenant-scoped authz + cross-tenant integration test — owner: **pm-backend** (carried)
- [medium] Sweep remaining `add_*` sub-routes for the same ungated pattern — owner: **pm-backend** (carried)

### user-facing silent-fail cluster (score 7-8, mix new + carried)

- [medium] **Re-open and re-land RegistryPage mutation-error-surface fix** (orphaned #3356) — destructive actions silently succeed on 500; sibling i18n/auth fixes landed around it — owner: **pm-frontend** (re-opened this run)
- [high] Wire `ReportFaultScreen.tsx` handleSubmit to real `/api/v1/faults` POST (currently `setTimeout(1500)` fake-submit) — owner: **pm-frontend** (carried)
- [high] Wire reality-web `/sell` wizard Publish to reality-server POST (seller submissions silently discarded) — owner: **pm-frontend** (carried)

### backend reliability (score 6-7, carried)

- [medium] Diagnose + resolve #3131 merge-pipeline deadlock — owner: **pm-backend** (carried)
- [medium] Audit ppt-web logout AUTHED_QUERY_KEY_ROOTS allow-list (TanStack caches leak across users) — owner: **pm-frontend** (carried)

### test-shadow + gate hygiene (score 5-6, new)

- [medium] **Add reality-web shared-comparison body-validator unit test** (PR #3350 surface) — close the boundary-guard test gap — owner: **pm-qa** (new)
- [medium] **Add RegistryPage auth-context regression test** (PR #3354 surface) — same gate shape as the historical gate #486 for useMessaging — owner: **pm-qa** (new)
- [medium] **Enforce dispatcher-fix-PR-requires-failing-test gate** (carried from 2026-08-25 decision proposal, 75 days unresolved) — owner: **pm-qa** / **pm-tech-lead** (new/carried)

### refactor-with-security-overtones (score 6, carried)

- [medium] #3006 — unify triplicated auth-header + MFA-retry in @ppt/api-client — owner: **pm-tech-lead** (carried)

### KMP code-review backlog (score 2-3, all blocked on #2652)

- CreateListingScreen onSubmit stub
- InquiriesResponse page_size vs reality-server `limit` mismatch
- Ktor HttpClient has no HttpTimeout
- SSO error-body unguarded (crashes on non-JSON response)
- getPortfolioAnalytics() 100-cap + unbounded fan-out

### carried follow-ups (score 2-3)

- gh #3283/#3304/#3305/#3306/#3307/#3308/#3309/#3310 (in-progress, dispatcher-authored follow-ups to merged PRs)
- reality-server inquiry dead FK classify (refactor, score 1)
- pm-frontend mobile-rn lint gates adoption (eslint-plugin-react-hooks + i18next/no-literal-string)
- pm-scrum-master violations-xtenant-404-contract-sweep (post-#3187 pattern audit)

### Screen-map drift

- [low] screen-map drift for PR #3293 reality-web realtor-detail (carried, pm-qa)
- [low] i18n missing-key guard test for registry pet/vehicle forms (new, pm-qa — PR #3351/#3355 verification)

Buffer: **18/36 open** · candidates ranked but unqueued: ~0 (coverage is all `done` — refill depends on the dispatcher's external backlog scan + resolution of #3357). Buffer at exactly half (18/36). The dispatcher's tier-1d signal pipeline is the natural refill source and is currently dead — operator action on #3357 is the unblock, not a `scan` run (which is cloud-forbidden).
