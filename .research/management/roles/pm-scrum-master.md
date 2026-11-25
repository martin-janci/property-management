# pm-scrum-master — 2026-11-25

## Summary

Sprint "Hardening & Refactor" has 0 of 3 workstreams done: the watch-module refactors and quarantine burn-down are in-progress, and the mobile-native KMP track is blocked. Coverage was 49/49 stories done at the last scan (2026-11-24), so planning is hardening-driven, not gap-driven. 15 PRs (#3769–#3785) merged on 2026-11-24, and the i18n warn→error flip is now T-6d (2026-12-01).

## Sprint progress

- Sprint: Hardening & Refactor — Modules Under Watch, release-248 Quarantine Burn-down, Mobile-native Backlog
- Workstreams done: 0 / 3

## Shipped since last run (15 PRs, all merged 2026-11-24)

- #3775 feat(UC-10): resident person-month submissions with manager approval
- #3769 fix(api-server): evict APNs token on 410 even when error body is unparseable
- #3770 code-review-api-core-rls-context-pool-not-conn (seed RLS context bound to query connection)
- #3771 churn-hotspot FCM push regression tests
- #3772 voice OAuth handler error-channel test
- #3773 pm-devops-dependabot-3441-axum-test-land
- #3774 quarantine burn-down cadence + weekly metric
- #3776 pm-integration property_type i18n enum stability
- #3778 pm-scrum-master-airbnb-env-leak-flake fix
- #3780 push APNs+FCM hotspot characterization
- #3779, #3782, #3783, #3785 — ppt-web i18n + UI code-review PRs
- #3784 dispatcher unshallow fix (closes #3781)

## Blockers

- **Mobile-native KMP backlog** (create-listing-screen, inquiries-response-contract, ktor-http-timeout, sso-error-body) — cloud-runner Gradle/KMP + swagger-ui egress blocked (#2652, #2966). Carried since 2026-08. Owner: pm-devops.
- **Dispatcher stall #3357** — non-mechanical merge conflicts remain after #3718 resolver; #3784 addressed the unshallow symptom but did not verify the stall clears. Owner: pm-tech-lead.
- **i18n flip gate: mobile-RN drafts #3676 and #3677** — both stale, gate the 2026-12-01 flip. Owner: pm-frontend.

## Next actions

1. **[high]** Promote or close mobile-RN i18n drafts #3676 and #3677 before the 2026-12-01 warn→error flip — pm-frontend (deadline 2026-11-29).
2. **[high]** Verify dispatcher stall #3357 status after #3784 and resolve remaining non-mechanical conflicts — pm-tech-lead.
3. **[high]** Unblock KMP + swagger-ui egress for the cloud runner (#2652, #2966), or formally defer mobile-native track 3 — pm-devops.
4. **[medium]** Land a UC-10 follow-up (tests, i18n, screen-map) for PersonMonthsScreen.tsx (444 churn) and person_months.rs (436 churn); add a watch entry if churn continues — pm-backend/pm-frontend/pm-qa.
5. **[medium]** Triage open PRs: #3786 epic-gap-audit round 12, drafts #3787/#3788/#3789, stale #2555/#2559 (UC-ACC-05 wiring open since July) — pm-tech-lead.
6. **[medium]** Continue quarantine burn-down (201 rows, due 2027-02-19) using #3774 cadence; decompose watch modules (vote.rs, sso.rs, financial.rs, lease.rs) — pm-qa + pm-backend.

## Risks

- i18n warn→error flip lands 2026-12-01 with mobile-RN drafts unresolved → CI breaks (medium × high).
- Quarantine deadline 2027-02-19 missed — burn-down was 0 in 2 weeks before #3774 (high × medium).
- Push hotspots (apns.rs 415, fcm.rs) keep regressing without an architectural pass (medium × medium).
- Mobile-native track blocked since August with no cloud-side fix (high × medium).
- New UC-10 approval flow + high churn raises chance of RLS/authz defects (medium × medium).

## Open questions

- Are #3676/#3677 still open and stale? (State carried from 2026-11-24, not re-verified today.)
- Does #3784 fully resolve dispatcher stall #3357, or only the unshallow symptom?
- Were #3777 and #3781's sibling PRs among the 15 merged?
- Has coverage.json been rescanned for UC-10 person-month submissions? Last scan (2026-11-24T04:19Z) predates #3775.
- Are #2555 and #2559 (UC-ACC-05 wiring) still wanted, or should they be closed as superseded?

## Decisions needed

See `.research/management/decisions.md` — 2026-11-25 block.
