# PPT Roadmap — upkeep 2026-10-26

_Rebuilt from `coverage.json` (49/49 stories done across 13 epics) + current `action-list.json` + sprint-status.yaml. 4-day lag since 2026-10-22 run._

## State of the project

- **Stories: 49/49 done** across 13 epics. No status flips this run; `last_checked` refreshed on all 3 epic-8a stories (cursor re-check — negative).
- Delta vs 2026-10-22 upkeep: **3 PRs merged** — all reality-web (realtor-api shape validation, safe-error log redaction, agency i18n price fix). None map to coverage stories.
- **New blocker: #3168 cargo-deny** — 3 new RUSTSEC advisories on dev gate every backend merge. Second cargo-deny gate in a month after #3124 (resolved 2026-10-19) — dependency hygiene is becoming a recurring merge-queue tax.
- **New tier-1 security gap: #3164** — violations `add_evidence` sub-route ungated; same shape as historical dispute `add_evidence` gap (#2483). Suggests `add_evidence`-pattern sub-routes are systemically under-tested.
- Screen coverage: 0 orphan screens · 0 validation errors · 3 known missing UC links (UC-33.1/33.2/33.3 already queued elsewhere).
- Open PRs this window: 3 drafts (#3161 push_fanout FCM eviction, #3165 migration_import test coverage, #3167 unassign moderation case).

## Epic status (short)

- **Epic 6 — Announcements & Communication:** 6/6 done in coverage; sprint-status.yaml drift.
- **Epic 7A — Basic Document Management:** 5/5 done.
- **Epic 8A — Basic Notification Preferences:** 3/3 done. **Re-checked this run (cursor idx 11).** No merged PR in the 2026-10-23..2026-10-26 window touched notification-preference surfaces; `last_checked = 2026-10-26` stamped on all 3 stories.
- **Epic 10A — OAuth Provider Foundation:** 3/3 done.
- **Epic 10B — Platform Administration:** 7/7 done.
- **Epic 80 — Dispute Resolution:** 3/3 done in coverage (sprint-status still shows partial).
- **Epic 81 — Reports:** 2/2 done.
- **Epic 82 — iOS Reality Portal (SwiftUI):** 5/5 done.
- **Epic 83 — Portal Integrations:** 3/3 done.
- **Epic 84 — Documents & e-signature:** 5/5 done.
- **Epic 85 — Build/CI configuration:** 2/2 done.
- **Epic 79 — API client integration:** 4/4 done.
- **Epic 9 — 2FA/TOTP:** 1/1 done.

## Ranked plan

### merge-queue unblock (highest leverage, score 10)

- [high] **Resolve #3168 cargo-deny advisories** (3 new RUSTSEC advisories on dev) — bump the offending transitive crate(s) or add bounded cargo-deny exceptions with a deadline — gates every backend merge today — owner: **pm-security**
- [high] Unblock mobile-native/KMP builds in cloud runner (**#2652**) — 5+ KMP items structurally unclaimable — owner: **pm-devops** (carried)

### tier-1 security (score 8-9)

- [high] **Gate violations add_evidence sub-resource (#3164)** with a tenant-scoped authz check; add cross-tenant integration test on the `community_cross_tenant_idor_tests.rs` pattern — owner: **pm-backend**
- [medium] Sweep remaining `add_*` sub-routes (announcements/attachments, documents/signatures, mediation/evidence) for the same ungated pattern — one-shot audit before shipping the #3164 fix — owner: **pm-backend**

### backend reliability (score 6-7)

- [medium] Diagnose + resolve #3131 merge-pipeline deadlock — long-stale; likely sqlx transaction scope or advisory-lock contention — owner: **pm-backend** (carried)
- [medium] Follow-up on draft PR #3167 (unassign moderation case 404, fixes #3163) — owner: **pm-backend**
- [low] Push_fanout FCM eviction (draft PR #3161) — review-ready; carried `repeated-churn-push_fanout` item is addressed by this PR landing — owner: **pm-backend**

### refactor-with-security-overtones (score 6)

- [medium] #3006 — unify triplicated auth-header + MFA-retry in @ppt/api-client fetch primitives — owner: **pm-tech-lead** (carried)

### quality / RN lint (score 4-5, carried)

- [medium] Resolve mobile RN jest-expo version rot (**#2951**) — owner: **pm-devops**

### KMP code-review backlog (score 2-3, all blocked on #2652)

- CreateListingScreen onSubmit stub
- InquiriesResponse page_size vs reality-server `limit` mismatch
- Ktor HttpClient has no HttpTimeout
- getPortfolioAnalytics() 100-cap + unbounded fan-out

### carried follow-ups (score 2-3)

- Portfolio/owner analytics drift (gh #3104)
- NewsScreen category badge i18n defaultValue fallback (gh #3116)
- Violations/appeal_number COUNT(*)+1 race (local repro welcome)
- screen-map drift for PR #2994 (ppt-web route refactor)
- Un-quarantine dispute KPIs test + window_start<=window_end validation
- Reconcile sprint-status.yaml drift vs coverage.json

### Screen-map drift

_Nothing new this run — carried: 3 missing UC links (UC-33.1/33.2/33.3), 1 ppt-web route refactor without screen-map update (PR #2994)._

Buffer: **17/36 open** · candidates ranked but unqueued: ~0 (coverage is all `done` — refill depends on the dispatcher's external backlog scan, not this upkeep run). Buffer below floor by 19 and below half (18) by 1 — a dispatcher backlog scan is overdue to replenish, but a `scan` on this skill is explicitly out of scope for an incremental run.
