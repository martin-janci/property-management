# PPT delivery — project state

<sub>Generated: 2026-11-25 by routine Phase 1.6 (pm-scrum-master + pm-security)</sub>

## Executive summary

Sprint "Hardening & Refactor" has 0 of 3 workstreams done: watch-module refactors and quarantine burn-down are in-progress, and the mobile-native KMP track is blocked. Coverage was 49/49 stories done at the last scan (2026-11-24), so planning is hardening-driven, not gap-driven. 15 PRs (#3769–#3785) merged on 2026-11-24, headlined by the UC-10 resident person-month submissions slice (#3775) and the seed-RLS-context fix (#3770). The i18n warn→error flip is T-6d (2026-12-01).

## Sprint progress

- **Sprint:** Hardening & Refactor — Modules Under Watch, release-248 Quarantine Burn-down, Mobile-native Backlog
- **Workstreams done:** 0 / 3
- **Churn hotspots this run:** PersonMonthsScreen.tsx (444), person_months.rs (436), apns.rs (415)
- **Code review slice:** api-core (churn-aligned) — 0 findings, segment is clean post recent fixes

## Shipped since last run

Fifteen PRs merged on 2026-11-24. Highlights:

- **feat(UC-10):** resident person-month submissions with manager approval (#3775) — new UC-10 slice
- **fix(api-server):** evict APNs token on 410 even when error body is unparseable (#3769)
- **code-review-api-core-rls-context-pool-not-conn (#3770):** bind seed RLS context to the query connection
- **Push hotspot characterization:** FCM regression tests (#3771), APNs+FCM hotspots (#3780), voice OAuth error-channel test (#3772)
- **Dependabot/axum-test 21.0.0→21.1.0** (#3773), **quarantine burn-down cadence** (#3774)
- **pm-integration** property_type i18n enum stability (#3776)
- **pm-scrum-master** airbnb env-leak flake fix (#3778)
- **ppt-web i18n + UI** code-review PRs: #3779, #3782, #3783, #3785
- **Dispatcher unshallow fix** (#3784 closes #3781)

## What's next (top 5)

1. **[high]** Promote or close mobile-RN i18n drafts #3676 and #3677 before the 2026-12-01 flip — pm-frontend (deadline 2026-11-29).
2. **[high]** Verify dispatcher stall #3357 status after #3784 and resolve remaining non-mechanical conflicts — pm-tech-lead.
3. **[high]** `FORCE ROW LEVEL SECURITY` on `person_month_submissions` (and `person_months` if not forced) + route submission repo calls through the RLS connection + cross-tenant integration test — pm-security.
4. **[high]** Unblock KMP + swagger-ui egress for the cloud runner (#2652, #2966), or formally defer mobile-native track 3 — pm-devops.
5. **[medium]** Restrict non-manager reads on person-month endpoints — any org resident can read other units' occupancy counts — pm-security.

## Blockers

- **Mobile-native KMP backlog** (create-listing-screen, inquiries-response-contract, ktor-http-timeout, sso-error-body) — cloud-runner Gradle/KMP + swagger-ui egress blocked (#2652, #2966). Carried since 2026-08.
- **Dispatcher stall #3357** — non-mechanical merge conflicts; #3784 closed the unshallow symptom.
- **i18n flip gate** — mobile-RN drafts #3676 / #3677 stale with no reviewDecision.

## Role focus today

- `pm-scrum-master` (always-on)
- `pm-security` (rotating; last run 2026-11-16)

### pm-scrum-master one-liner

Hardening sprint — 0/3 workstreams done. 15 PRs merged yesterday. i18n flip at T-6d gated on two stale mobile-RN drafts.

### pm-security one-liner

UC-10 authorization is correct at handler level but lacks DB-level defense in depth (`person_month_submissions` not `FORCE`'d; repo uses the pool). Residents can also read any unit's person-month counts via GET endpoints. Add `FORCE RLS` + RLS-connection refactor + gate reads on residency / manager; audit the pattern repo-wide.
