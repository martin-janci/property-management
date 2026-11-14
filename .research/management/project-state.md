# PPT Project State

_Generated: 2026-11-14 — routine Phase 1.6 incremental upkeep. Role focus: **pm-security** (idx 5 — 44-day stale rotation slot). pm-scrum-master always-on. Coverage `scan_kind=upkeep`; coverage_cursor idx 2 → 3 (epic-6 re-checked, 6/6 done; no announcement/communication story changes this window)._

## Delta since last routine run (2026-11-13 11:30 UTC)

- **21 PRs merged** in ~15h (range #3478–#3501). Composition: ~16 `code-review-*` findings landed across ppt-web-core, admin-web, reality-web + batch of fix/i18n PRs, plus #3480 (JWT iss/aud triage) and #3479 (lag-floor guard) + #3497 (voice book-facility correctness) + #3499 (formatDate RangeError) + #3501 (ammonia 4.2.5 — RUSTSEC-2026-0533, closes #3498).
- **1 security issue closed** (#3498 — bump ammonia for RUSTSEC-2026-0533, via #3501).
- **2 follow-up issues opened**: #3502 (price-map i18n follow-up, PR #3506 in review) and #3505 (ppt-web E2E regression, PR #3507 in review). #3505 surfaced right after the auth-guard batch merged — flagged by pm-security as a potential regression vector.
- **Open delivery items**: 8 open PRs touched including #3447 vitest bump, #3488 admin-web tenant-restore wire, #3500 fault-detail mutations, #3503 aichat race, #3504 route-query silent error, #3506 price-map i18n (review), #3507 disputes E2E (review), #3508 scheduler no-panic supervision (in-progress).
- **Routine lag**: 0d 15h — on schedule.

## Executive summary

- **Delivery**: 49/49 stories done across 13 epics in coverage. No status flips this run; `last_checked` refreshed on epic-6 (idx 2 → 3, Announcements & Communication — 6/6 done, nothing touched this window).
- **Security focus** (pm-security rotation): #3427 JWT iss/aud is still open in code — neither access-token validator checks iss/aud, and minted tokens carry no iss/aud claims. `plans/security-jwt-iss-aud-cross-service.md` is ready; only design sign-off blocks it. Separately, the #3486/#3482 auth-guard batch produced an E2E regression (#3505) — must be verified as a test-fixture issue, not a product regression.
- **Dependabot majors**: still stacked (vitest 5, jsonwebtoken 11, utoipa-swagger-ui 10). jsonwebtoken 11 should be sequenced after #3427.
- **Churn hotspots**: admin-web TenantLifecyclePage.tsx (267 lines this window), reality-web journal/page.tsx (239), ppt-web mgmt-routes-auth-guard.route.test.tsx (171). The first two are post-merge fallout from the i18n batch; the third is new test coverage for the auth-guard gating — expected high churn given the batch.

## Sprint progress

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · epics_done = 4/4 at sprint level · 49/49 stories done in coverage.

| Epic | Sprint status | Coverage status |
|---|---|---|
| 6 — Announcements & Communication | done (6/6) | 6/6 done · **last_checked 2026-11-14 (cursor re-check)** |
| 7A — Basic Document Management | done (5/5) | 5/5 done |
| 8A — Basic Notification Preferences | done (3/3) | 3/3 done |
| 10A — OAuth Provider Foundation | done (3/3) | 3/3 done |
| 10B — Platform Administration | done (7/7) | 7/7 done |
| 80 — Dispute Resolution | done (3/3) | 3/3 done |

## What's next (top 5 from the ranked plan)

1. **[high] pm-scrum-master** — Land #3507 (gh-issue-3505 disputes E2E auth) and resolve the ppt-web E2E regression tracked by #3505. Confirm it is not masking a real auth/route-guard regression from the #3485–#3491 batch.
2. **[high] pm-security** — Implement #3427 JWT iss/aud per `plans/security-jwt-iss-aud-cross-service.md`. Patch both verifier copies (`auth.rs` ~L71–76 and `services/jwt.rs` ~L198–224) and both services (api-server + reality-server). Dual-accept rollout. DoD includes a cross-service token-replay test.
3. **[high] pm-backend** — Ship #3508 (scheduler no-panic supervision, in-progress) as a crash-risk item.
4. **[medium] pm-devops** — Human-review the 3 major Dependabot bumps (#3447 vitest 5, #3444 jsonwebtoken 11, #3443 utoipa-swagger-ui 10). Sequence jsonwebtoken after #3427 work; finish #3506 (gh-issue-3502 price-map i18n).
5. **[medium] pm-security** — Backend audit of community/disputes/messaging/neighbors/privacy endpoints: ProtectedRoute is UX gating only; verify role/tenant/RLS checks exist independent of the SPA.

## Blockers

- **mobile-native-kmp cloud build** (issue #2652 closed but structural gap persists). 5+ backlog items permanently unclaimable in cloud. Owner: pm-devops.
- **#3427 JWT iss/aud** (labeled `blocked`). Triaged by #3480; plan ready at `plans/security-jwt-iss-aud-cross-service.md`. Awaits human design sign-off (iss/aud values, rollout window). Owner: pm-security / pm-scrum-master.

## Role focus today

- pm-scrum-master (always-on): delivery picture, security triage follow-through, E2E regression tracking.
- pm-security (rotating, idx 5): JWT iss/aud, auth-guard batch regressions, jsonwebtoken 11 pre-merge review.
