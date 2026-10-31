# PPT Roadmap — upkeep 2026-10-31

_Rebuilt from `coverage.json` (49/49 stories done across 13 epics) + current `action-list.json` + sprint-status.yaml. 3-day lag since 2026-10-28 run._

## State of the project

- **Stories: 49/49 done** across 13 epics. No status flips this run; `last_checked` refreshed on all 3 epic-10a stories (coverage_cursor 0 → 1 — negative).
- Delta vs 2026-10-28 upkeep: **15 PRs merged** (#3185–#3205) — dispatcher-driven code-review sweep across ppt-web (6), reality-web (3), reality-server (2), mobile-rn (1), api-server (1 fix + 2 tests). None map to new coverage story deliveries.
- **Dispatcher buffer-low signal:** this run was fired with `buffer-low: claimable=10/72 — refill planner`. The action-list has 30 open rows; the gap between claimable and open is likely UNRESOLVED-sentinel items blocking claim. Scrum-master follow-up queued.
- **New reality-web i18n cluster:** rotating-review found 4 authed pages (account/profile/password + realtor/profile) with zero i18n coverage, plus 15 raw `next/link` imports bypassing the next-intl Link wrapper (locale-loss on nav). Same bug class as the just-shipped PR #3203.
- Screen coverage: 0 orphan screens · 0 validation errors · 3 known missing UC links (UC-33.1/33.2/33.3). 3 new screen-map-drift signals this run (PR #3186 ppt, #3197/#3203 reality — routes touched without screen-doc updates).
- Open PRs this window: 20 (14 dependabot, 1 follow-up #3171 add_evidence manager gate, 1 docs #3024, 1 i18n #3119, 2 old feat #2555/#2559).

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

_3 new drift signals this run: PR #3203 (reality-web listing-edit touched route, no screen doc), PR #3197 (same route, price-zero fix), PR #3186 (ppt-web routes/groups/iot.tsx IoT dashboard, no ppt screen doc). Plus carried: 3 missing UC links (UC-33.1/33.2/33.3), 1 ppt-web route refactor without screen-map update (PR #2994)._

### Reality-web authed i18n cluster (new, score 4)

- [medium] **Sweep reality-web authed pages** — account/profile/page.tsx + account/password/page.tsx + account/page.tsx + realtor/profile/page.tsx all ship entirely English on 5/6 locales. Same bug class as the just-shipped PR #3203 (listing-edit).
- [medium] **Migrate 15 raw `next/link` imports to `@/i18n/routing` Link** — nav from these pages re-resolves to default locale (sk), kicking cs/de/en/hu/pl users out of their locale.

Buffer: **30/36 open** · candidates ranked but unqueued: ~0 (coverage is all `done` — refill depends on dispatcher backlog scan, not this upkeep run). Buffer at 30/36 is above the half-empty floor (18) — adequate for the next 24h of dispatcher runs.
