# PPT Roadmap — upkeep 2026-10-30

_Rebuilt from `coverage.json` (49/49 stories done across 13 epics) + current `action-list.json` + sprint-status.yaml. Quiet 2-day window: 0 PRs merged since 2026-10-28 (cursor still at #3183)._

## State of the project

- **Stories: 49/49 done** across 13 epics. No status flips this run; `last_checked` refreshed on epic-10a (3 OAuth Provider stories — cursor idx 0 → 1).
- Delta vs 2026-10-28 upkeep: **0 PRs merged** · **0 new issues** · **+5 new drafts** opened by the dispatcher (#3191–#3196, including 3 Dependabot group PRs #3194/#3195/#3196). Open-PR count rose 25 → 31.
- **Carried blockers, all unchanged**: #3187 (cross-tenant assign-404 CI gate), #3168 (cargo-deny RUSTSEC), #3164 (violations add_evidence authz — in-flight PR #3171), #3131 (merge pipeline deadlock), #2652 (KMP cloud builds), #2951 (mobile RN jest-expo).
- **Stale-PR aging warning**: 7 of 31 open PRs are 60+ days old (accounting #2555/#2559 at ~70d; Dependabot #2584/#2588/#2589/#2590/#2866 at 60–103d). Reviewer-starvation pattern (first called out 2026-07-30) has not cleared.
- **Test-shadow debt is now actively compounding** (new pm-qa risk `pm-qa-test-shadow-compounding-dispatcher-fix-loop-2026-10-30`): 7 open `code-review-*` fix PRs touch user-visible defects but only #3184 ships with regression tests.
- Screen coverage: 0 orphan screens · 0 validation errors · 3 known missing UC links (UC-33.1/33.2/33.3 already queued elsewhere).

## Epic status (short)

- **Epic 6, 7A, 8A, 10A, 10B, 80, 81, 82, 83, 84, 85, 79, 9** — all done in coverage.
- **Epic 10A — OAuth Provider Foundation:** 3/3 done. **Re-checked this run (cursor idx 0).** No merged PR in the 2026-10-28..2026-10-30 window touched OAuth surfaces; `last_checked = 2026-10-30` stamped on all 3 stories.

## Ranked plan

### merge-queue unblock (highest leverage, score 10)

- [high] **Resolve #3187 cross-tenant assign-404 CI gate** — draft PR #3188 exists but needs the regression test file in the same commit before merge — owner: **pm-security** (+ pm-qa on the test attach)
- [high] **Resolve #3168 cargo-deny advisories** (3 new RUSTSEC advisories gate every backend merge today) — owner: **pm-security** (carried)
- [high] Unblock mobile-native/KMP builds in the cloud runner (**#2652**) — 5+ KMP items structurally unclaimable in cloud; blocks regression testing for all KMP PRs — owner: **pm-devops** (carried)

### tier-1 security (score 8-9)

- [high] **Gate violations add_evidence sub-resource (#3164)** — draft PR #3171 exists; add cross-tenant integration test using `community_cross_tenant_idor_tests.rs` pattern before merge — owner: **pm-backend**
- [medium] Sweep remaining `add_*` sub-routes (announcements/attachments, documents/signatures, mediation/evidence) for the same ungated pattern — chronic add_evidence pattern-defect, one-shot audit — owner: **pm-backend**

### test-coverage & QA (score 7-8, pm-qa role focus today)

- [high] **Attach failing-on-main cross-tenant assign-404 regression test to draft PR #3188** before merge (reuse `violations_mutation_cross_org_idor_tests` scaffolding) — owner: **pm-qa**
- [high] **Replace pure-Rust announcement cross-tenant fanout re-model test (#2484) with sqlx integration test** exercising the real RLS predicate — unblocks carried risk aged 99 days — owner: **pm-qa**
- [high] **Land one worked e2e-form-submit smoke example (ReportFaultScreen with MSW)** co-owned with pm-frontend — DoD-sets the gate for the remaining ~12 forms — owner: **pm-qa + pm-frontend**
- [medium] Expand add_evidence-pattern cross-tenant regression coverage before #3171 merges — owner: **pm-qa**
- [medium] Attach failing-on-main tests to draft PRs #3192 (file_appeal status guard) + #3193 (moderation report flood dedup) before merge-queue — owner: **pm-qa**
- [medium] Draft shared proptest corpus crate under `backend/crates/test-corpus/` for sanitizers — carries over the 66-day-old 2026-08-25 fuzz-corpus decision — owner: **pm-qa**

### backend reliability (score 6-7)

- [medium] Diagnose + resolve #3131 merge-pipeline deadlock — long-stale; likely sqlx transaction scope or advisory-lock contention — owner: **pm-backend** (carried)
- [medium] Follow-up #3171 (violations add_evidence authz) — owner: **pm-backend**
- [medium] #3006 — unify triplicated auth-header + MFA-retry in @ppt/api-client fetch primitives — owner: **pm-tech-lead** (carried)

### frontend quality (score 5-6, from 2026-10-28 pm-frontend run — in flight)

- [high] Wire `ReportFaultScreen.tsx` handleSubmit to real `/api/v1/faults` POST — fake-submit data-loss bug — owner: **pm-frontend**
- [high] Wire reality-web `/sell` wizard Publish to reality-server POST — same pattern — owner: **pm-frontend**
- [high] Audit ppt-web logout AUTHED_QUERY_KEY_ROOTS allow-list — tenant isolation defect (draft PR #3190 exists) — owner: **pm-frontend**
- [medium] Adopt eslint-plugin-react-hooks + i18next/no-literal-string in `frontend/apps/mobile/` — owner: **pm-frontend**

### scrum-master coordination (score 5-6)

- [medium] **Explicit reviewer slot for 7 PRs 60+ days old** — accounting trio #2555/#2559 at ~70d, Dependabot cluster #2584–#2866 at 60–103d; reviewer-starvation worsening — owner: **pm-scrum-master**
- [high, carried] Sweep remaining violations mutation handlers for the 403-vs-404 cross-tenant contract — pattern audit after #3187 — owner: **pm-scrum-master**

### quality / RN lint (score 4-5, carried)

- [medium] Resolve mobile RN jest-expo version rot (**#2951**) — owner: **pm-devops**

### KMP code-review backlog (score 2-3, all blocked on #2652)

- CreateListingScreen onSubmit stub
- InquiriesResponse page_size vs reality-server `limit` mismatch
- Ktor HttpClient has no HttpTimeout
- getPortfolioAnalytics() 100-cap + unbounded fan-out

### carried follow-ups (score 2-3)

- Portfolio/owner analytics drift (gh #3104)
- NewsScreen category badge i18n defaultValue fallback (gh #3116 — draft PR #3119)
- Violations/appeal_number COUNT(*)+1 race (draft PR #3075)
- screen-map drift for PR #2994 (ppt-web route refactor — draft PR #3024)
- Un-quarantine dispute KPIs test + window_start<=window_end validation
- Reconcile sprint-status.yaml drift vs coverage.json

### Screen-map drift

_Nothing new this run — carried: 3 missing UC links (UC-33.1/33.2/33.3), 1 ppt-web route refactor without screen-map update (PR #2994, draft fix #3024)._

Buffer: **42/36 open** · 0 candidates ranked but unqueued (coverage all `done`; refill is dispatcher-external). Buffer now 6 above target — healthy; the 2026-10-28 half-empty warning has cleared after pm-frontend (+6) and pm-qa (+6) rotations landed their next_actions in-queue.
