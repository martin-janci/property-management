# PPT Roadmap — upkeep 2026-11-09

_Rebuilt from `coverage.json` (49/49 stories done across 13 epics) + current `action-list.json` + sprint-status.yaml. 57h lag since 2026-11-06 run. Trigger: dispatcher buffer-low fire — `claimable=6/72 — all open items gated(mobile-native)/dep-blocked`._

## State of the project

- **Stories: 49/49 done** across 13 epics. No status flips this run; `last_checked` refreshed on all 3 epic-10a stories (cursor re-check, cursor idx 0 → 1).
- **Delta vs 2026-11-06**: 7 PRs merged (5 ppt-web registry theme, 1 reality-web schema-validation, 1 api-server password-session revocation). Also 3 intentional orphan closures (issue #3357) — not regressions.
- **Open issues this window**: #3357 dispatcher root-cause writeup for the 2026-11-08 orphan incident.
- **New risk — RegistryPage.tsx churn (medium/medium)**: 4 PRs in 3 days, narrow PR-shaped unit tests, no surface-level integration test. Mitigation queued in action-list.
- **New risk — orphan-base recurrence (medium/high)**: no self-test fired for the 2026-11-08 silent-orphan class. Action queued.
- **Screen coverage**: 0 new orphan screens this run · 0 validation errors · drift status unchanged from prior run · PR #3358 closed the realtor-detail drift carried from #3293.

## Epic status (short)

- **Epic 6 — Announcements & Communication:** 6/6 done.
- **Epic 7A — Basic Document Management:** 5/5 done.
- **Epic 8A — Basic Notification Preferences:** 3/3 done.
- **Epic 9 — 2FA/TOTP:** 1/1 done.
- **Epic 10A — OAuth Provider Foundation:** 3/3 done. **Re-checked this run (cursor idx 0).** No merged PR this window touched OAuth surfaces; `last_checked = 2026-11-09` stamped on all 3 stories.
- **Epic 10B — Platform Administration:** 7/7 done.
- **Epic 79 — API client integration:** 4/4 done.
- **Epic 80 — Dispute Resolution:** 3/3 done.
- **Epic 81 — Reports:** 2/2 done.
- **Epic 82 — iOS Reality Portal (SwiftUI):** 5/5 done.
- **Epic 83 — Portal Integrations:** 3/3 done.
- **Epic 84 — Documents & e-signature:** 5/5 done.
- **Epic 85 — Build/CI configuration:** 2/2 done.

## Ranked plan

### planner-refill (highest leverage — direct answer to today's buffer-low fire)

- [medium] **Promote `code-review-reality-server-sso-oauth-json-unbounded`** — Phase 1.5 surfaced this bug (reality-server/src/routes/sso.rs:763 + :780 unchecked `response.json().await?` at the PM OAuth boundary). Parallel to the pattern just fixed in PR #3350. Backend, cloud-landable, bypasses mobile-native gate. — owner: pm-backend (via routine Phase 3)
- [low] **Promote `code-review-reality-server-public-listing-resolve-dup`** — 4 hand-rolled copies of the public-active-listing gate; extract `util/listings::resolve_active_public_listing` helper. Backend, cloud-landable, +1 refactor. — owner: pm-backend (via routine Phase 3)

### delivery hygiene (score 7-8)

- [high] **Clear the two 12-day-stale UNRESOLVED: sentinels** — pm-qa accepted the mobile-rn lint-gates adoption today; pm-security still owes the violations xtenant-404 contract sweep. Hidden planner-drain. — owner: pm-scrum-master
- [high] **Decide pm-devops-unblock-mobile-native-cloud-builds (#2652)** — 4 KMP backlog rows + 1 action-list row gated. Ship-by-date or explicit deferral. — owner: pm-devops
- [medium] **Dispatcher-self-test T31 for orphan-base** — would have caught the 2026-11-08 silent-orphan class. — owner: pm-qa (co: pm-devops)
- [medium] **RegistryPage integration test** — 4 PRs in 3 days, PR-shaped unit coverage only. Cross-mutation regression likely. — owner: pm-frontend

### tier-1 security (carried, score 7)

- [medium] **Sweep remaining `add_*` sub-routes** for the ungated-pattern (announcements/attachments, documents/signatures, mediation/evidence) — carried from 2026-10-26 roadmap; not this run's focus. — owner: pm-backend

### KMP code-review backlog (score 2-3, all blocked on #2652)

- CreateListingScreen onSubmit stub (plan ready; unclaimable until #2652)
- InquiriesResponse page_size vs reality-server `limit` mismatch (plan ready; unclaimable until #2652)
- Ktor HttpClient has no HttpTimeout
- getPortfolioAnalytics() 100-cap + unbounded fan-out
- KMP realtor SSO error-body parsing unguarded

### carried follow-ups (score 2-3)

- SECURITY: ppt-web refresh-token in localStorage (score 2, security but confidence=medium — security fast-track needs high; promotion blocked until confidence upgrade or score bump)
- Portfolio/owner analytics drift (gh #3104, carried)
- NewsScreen category badge i18n defaultValue fallback (gh #3116, carried)

### Screen-map drift

_Nothing new this run — PR #3358 closed the realtor-detail drift carried from #3293._

Buffer: **13/36 open** · candidates ranked but unqueued: 2 (both reality-server Phase 1.5 findings, pending Phase 3 promotion this run). ⚠ Buffer still below half (18) — expected to climb by ~2 once Phase 3 promotes the reality-server findings; mobile-native keystone remains the structural tax on buffer depth.
