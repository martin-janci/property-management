# PPT Roadmap — upkeep 2026-11-08

_Rebuilt from `coverage.json` (49/49 stories done across 13 epics) + current `action-list.json` + sprint-status.yaml. 2-day lag since 2026-11-06 run._

## State of the project

- **Stories: 49/49 done** across 13 epics. No status flips this run; `last_checked` refreshed on all 3 epic-10a stories (cursor idx 0, re-check — negative); PR #3353 session-revocation evidence appended to 10a-3-oauth-token-management.
- Delta vs 2026-11-06: **5 PRs merged** — #3350 (reality-web shared-comparison JSON shape), #3351/3354/3355 (ppt-web registry i18n + auth-context + chrome i18n), #3353 (api-server session revocation on password change — closes #3352).
- **New blocker: #3357** — dispatcher pipeline fully stalled on 3 approved PRs (non-mechanical conflicts) + Tier-1d signals dead-end. Queued as `pm-scrum-master-unblock-dispatcher-stall-3357-2026-11-08` (high).
- Open PRs this window: 1 draft (#3356 — RegistryPage mutation error surfacing; verify green, ready for review).

## Epic status (short)

- **Epic 6 — Announcements & Communication:** 6/6 done in coverage; sprint-status.yaml drift.
- **Epic 7A — Basic Document Management:** 5/5 done.
- **Epic 8A — Basic Notification Preferences:** 3/3 done.
- **Epic 10A — OAuth Provider Foundation:** 3/3 done. **Re-checked this run (cursor idx 0 → 1).** PR #3353 session-revocation evidence appended to 10a-3; `last_checked = 2026-11-08` stamped on all 3 stories.
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

⚠ Buffer below half — consider running scan to refresh coverage. Buffer: **14/36 open** · candidates ranked but unqueued: ~0 (coverage all `done` — refill depends on the dispatcher's external backlog scan, not this upkeep run). Buffer below floor by 22 and below half (18) by 4.
