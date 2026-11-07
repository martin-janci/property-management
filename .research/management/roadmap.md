# PPT Roadmap — upkeep 2026-11-07

_Rebuilt from `coverage.json` (49/49 stories done across 13 epics) + current `action-list.json` + sprint-status.yaml. 1-day lag since 2026-11-06 routine run._

## State of the project

- **Stories: 49/49 done** across 13 epics. No status flips this run; `last_checked` refreshed on all 3 epic-10a stories (cursor re-check — negative; OAuth surface unchanged in window).
- Delta vs 2026-11-06: **2 PRs merged** (#3350 reality-web share-cmp schema validation [has test] · #3351 ppt-web registry i18n [no test]). Both are refactor/technical-debt items, not coverage-story-delivery; both flipped to `done` in action-list.
- **Planner buffer at 6/72 claimable** — critical. All remaining open non-dep-blocked work is mobile-native-gated on #2652; dispatcher's 3-implementer pool is starving.
- **Test-shadow pattern persists** — PR #3351 landed WITHOUT a regression test (same class pm-qa flagged on 2026-08-25). The 2026-08-25 decision proposal (CI diff-guard on dispatcher-spawned fix PRs) is still unformalized 74 days later.
- Screen coverage: 0 orphan screens · 0 validation errors · 3 known missing UC links (UC-33.1/33.2/33.3 already queued elsewhere).
- Open PRs this window: 0 touched since last run. Carried drafts unchanged.

## Epic status (short)

- **Epic 6 — Announcements & Communication:** 6/6 done.
- **Epic 7A — Basic Document Management:** 5/5 done.
- **Epic 8A — Basic Notification Preferences:** 3/3 done.
- **Epic 10A — OAuth Provider Foundation:** 3/3 done. **Re-checked this run (cursor idx 0).** No merged PR in the 2026-11-06..2026-11-07 window touched OAuth surfaces; `last_checked = 2026-11-07` stamped on all 3 stories.
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

### planner unblock (highest leverage, score 10)

- [high] **Dispatcher Tier-1b off-rotation backlog scan** (backend + frontend + reality vectors) to break mobile-native gating — planner at 6/72 claimable; need a non-mobile parallel queue so the 3-implementer pool doesn't starve — owner: **pm-scrum-master / pm-devops**
- [high] Unblock mobile-native/KMP builds in cloud runner (**#2652**) — carried; now ALSO the single release-gate for mobile-native QA — owner: **pm-devops**

### merge-queue unblock (score 9)

- [high] **Resolve #3168 cargo-deny advisories** — gates every backend PR merge today (carried) — owner: **pm-security**
- [high] **Confirm #3187 cross-tenant assign-404 contract status** on dev CI (10 days carried, still UNRESOLVED:pm-security sentinel) — owner: **pm-qa / pm-security**

### tier-1 security (score 8)

- [high] **Gate violations add_evidence sub-resource (#3164)** with a tenant-scoped authz check; carried — owner: **pm-backend**
- [medium] Sweep remaining `add_*` sub-routes (announcements/attachments, documents/signatures, mediation/evidence) for ungated pattern — owner: **pm-backend**

### QA hardening — this run's pm-qa focus (score 6-7)

- [high] **Escalate planner buffer-low as release-readiness risk** (publish release-blocker view distinct from planner-buffer view; mobile-native items surface as release blockers) — owner: **pm-qa**
- [medium] **Backfill Vitest i18n regression test for PR #3351 registry components** (closes test-shadow debt on #3351) — owner: **pm-qa** (dep: pm-frontend)
- [medium] **Formalize the 2026-08-25 CI diff-guard decision** (dispatcher-spawned fix PRs require failing-on-main test in same PR; draft the workflow check; quantify 30-day false-positive rate) — owner: **pm-tech-lead + pm-qa**
- [medium] **Replace pure-Rust announcement fan-out predicate test with sqlx integration test (#2484)** — carried 107 days — owner: **pm-qa** (dep: pm-backend)
- [medium] **Audit last 30 days of PRs for the fake-submit UI pattern** (handleSubmit → setTimeout → no POST) — pm-frontend flagged three screens; broader class likely — owner: **pm-qa** (dep: pm-frontend)

### backend reliability (score 6)

- [medium] Diagnose + resolve #3131 merge-pipeline deadlock (carried) — owner: **pm-backend**

### refactor-with-security-overtones (score 5)

- [medium] #3006 — unify triplicated auth-header + MFA-retry in @ppt/api-client fetch primitives (carried) — owner: **pm-tech-lead**

### quality / RN lint (score 4, carried)

- [medium] Adopt eslint-plugin-react-hooks + i18next/no-literal-string in `frontend/apps/mobile` (unblock by pm-qa accepting warn-only-for-one-sprint — clears the UNRESOLVED:pm-qa sentinel) — owner: **pm-frontend**
- [medium] Resolve mobile RN jest-expo version rot (**#2951**) — owner: **pm-devops**

### KMP code-review backlog (score 2-3, all blocked on #2652)

- CreateListingScreen onSubmit stub
- InquiriesResponse page_size vs reality-server `limit` mismatch
- Ktor HttpClient has no HttpTimeout
- MyListings pagination cap
- SSO error-body unguarded

### Screen-map drift

- [low] `screen-map-drift-pr-3293-reality` — PR #3293 i18n'd reality-web realtor/[id] without updating `docs/screens/reality/realtor-detail.md` (carried; owner pm-qa).

## Follow-up (from merged reviews, all in-progress)

- 7 follow-up issues (#3304–#3310) from PRs #3287/#3286/#3269/#3260/#3281/#3289/#3291 — all `in-progress`, owner pm-tech-lead. No action needed this run.
- #3283 realtor i18n follow-up (carried).

Buffer: **17/36 open** · candidates ranked but unqueued: ~0 (coverage is all `done` — refill depends on the dispatcher's external backlog scan, not this upkeep run). Buffer below floor by 19 and critically below half (18) by 1 — ⚠ **planner-health critical**: off-rotation Tier-1b backlog scan (backend+frontend+reality) is queued at the top of the plan this run.
