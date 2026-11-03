# PPT Roadmap — upkeep 2026-11-03

_Rebuilt from `coverage.json` (49/49 stories done across 13 epics) + current `action-list.json` + sprint-status.yaml. 1-day lag since 2026-11-02 run._

## State of the project

- **Stories: 49/49 done** across 13 epics. No status flips this run; `last_checked` refreshed on epic-10a (cursor re-check — no OAuth PRs in window).
- Delta vs 2026-11-02 upkeep: **5 PRs merged** — all quality-track with explicit IG3 TDD evidence. Nothing maps to a coverage story.
- **Blockers unchanged (6 carried):** #3187 dev-red CI (cross-tenant assign-404 contract), #3168 cargo-deny RUSTSEC, #3164 violations add_evidence authz, #3131 merge pipeline deadlock, #2652 KMP cloud, #2951 jest-expo.
- Screen coverage: 0 orphan screens · 0 validation errors · 3 known missing UC links (UC-33.1/33.2/33.3, carried).
- Open PRs this window: 3 drafts (#3221, #3222, #3193) — backend-path gated on #3187.

## Epic status (short)

- **Epic 6 — Announcements & Communication:** 6/6 done.
- **Epic 7A — Basic Document Management:** 5/5 done.
- **Epic 8A — Basic Notification Preferences:** 3/3 done.
- **Epic 10A — OAuth Provider Foundation:** 3/3 done. **Re-checked this run (cursor idx 0).** No merged PR in the 2026-11-02..2026-11-03 window touched OAuth surfaces; `last_checked = 2026-11-03` stamped on all 3 stories.
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

### dev-red CI unblock (highest leverage, score 10)

- [high] **Decide + fix the violations.assign cross-tenant 404-vs-403 contract (#3187)** — either `assign_violation` returns 404 cross-tenant OR the regression test's role shape is adjusted — unblocks `backend.yml` on dev — owner: **pm-security** (pm-qa validates)
- [high] **Resolve #3168 cargo-deny advisories** (3 RUSTSEC on dev) — bump transitive crate(s) or bounded exception with deadline — owner: **pm-security**
- [high] Unblock mobile-native/KMP builds in cloud runner (**#2652**) — 5+ KMP items structurally unclaimable — owner: **pm-devops** (carried)

### tier-1 security (score 8-9)

- [high] **Gate violations add_evidence sub-resource (#3164)** with a tenant-scoped authz check; add cross-tenant integration test on the `community_cross_tenant_idor_tests.rs` pattern — owner: **pm-backend**
- [medium] Sweep remaining `add_*` sub-routes (announcements/attachments, documents/signatures, mediation/evidence) for the same ungated pattern — owner: **pm-backend**

### test strategy (pm-qa focus, score 6-8)

- [high] Add VoteDetailScreen.test.tsx cast-POST contract assertion — closes the ranked-ballot regression class driving the 2nd churn run — owner: **pm-qa + pm-frontend**
- [high] Adopt per-screen MSW/Mirage Submit-POST gate (mobile + reality-web) — closes the chronic fake-submit-pattern cluster class — owner: **pm-qa + pm-frontend**
- [medium] Replace pure-Rust announcement fan-out model test with `#[sqlx::test]` integration pinning the actual RLS policy (carried #2484) — owner: **pm-qa**
- [medium] Un-quarantine the dispute KPIs test + add window_start<=window_end validation + reporting-consumer-contract case — owner: **pm-qa + pm-backend**

### backend reliability (score 6-7)

- [medium] Diagnose + resolve #3131 merge-pipeline deadlock — long-stale — owner: **pm-backend** (carried)
- [medium] Follow-up on draft PR #3167 (unassign moderation case 404, fixes #3163) — owner: **pm-backend** (if still open)

### refactor-with-security-overtones (score 6)

- [medium] #3006 — unify triplicated auth-header + MFA-retry in @ppt/api-client fetch primitives — owner: **pm-tech-lead** (carried)

### routine delivery (score 4-5)

- [medium] Add staleness-bias to pm_cursor rotation (prefer any role >60d stale over current index) — closes rotation-skew meta-risk — owner: **pm-scrum-master**
- [medium] Resolve mobile RN jest-expo version rot (**#2951**) — owner: **pm-devops**

### KMP code-review backlog (score 2-3, all blocked on #2652)

- CreateListingScreen onSubmit stub
- InquiriesResponse page_size vs reality-server `limit` mismatch
- Ktor HttpClient has no HttpTimeout
- getPortfolioAnalytics() 100-cap + unbounded fan-out

### carried follow-ups (score 2-3)

- Portfolio/owner analytics drift (gh #3104)
- NewsScreen category badge i18n defaultValue fallback (gh #3116)
- Violations/appeal_number COUNT(*)+1 race
- screen-map drift for PR #2994 (ppt-web route refactor)
- mobile VotingScreen "Invalid Date" / NaN quorum / eligibility.reason raw string
- /moderation/report flood (no dedup/rate-limit)
- Reconcile sprint-status.yaml drift vs coverage.json (now pm-qa-owned)

### Screen-map drift

_Nothing new this run — PR #3216 reconciled account-listing-edit (PR #3212 drift). Carried: 3 missing UC links (UC-33.1/33.2/33.3), 1 ppt-web route refactor without screen-map update (PR #2994)._

Buffer: **27/36 open** · candidates ranked but unqueued: ~0 (coverage is all `done`; refill depends on the dispatcher's external backlog scan, not this upkeep run). Buffer below floor by 9 but above half (18), so no scan warning emitted this run.
