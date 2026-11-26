# PPT delivery — project state

<sub>Generated: 2026-11-26 by routine Phase 1.6 (pm-scrum-master + pm-data)</sub>

## Executive summary

22 PRs landed since the last run — the most active single day in the hardening sprint. Headlined by pm-security's FORCE RLS on person_month_submissions and the submission-repo RLS-conn migration (#3797), UC-10 own-unit person-count scoping (#3792), DSA i18n dates fix (#3795), and a 14-PR code-review-finding wave across ppt-web-ui, reality-web, mobile-rn, and api-core/handlers. Five PRs are approved and sitting in the review queue (#3800, #3813, #3814, #3815, #3816) — that's the next merge wave.

## Sprint progress

- **Sprint:** Hardening & Refactor — Modules Under Watch, release-248 Quarantine Burn-down, Mobile-native Backlog
- **Workstreams done:** 0 / 3
- **Churn hotspots this run:** person_month_submissions.rs (178), person_month.rs (142), authorization.rs (120); plus person_months.rs routes (108, runs_seen 1→2), voice.rs (92), AppShell.tsx (91), maintenance-messages.ts (86)
- **Code review slice:** see routine brief — deferred to Phase 1.5 result

## Shipped since last run

- pm-security FORCE RLS on person_month_submissions + submission repo on RLS conn (#3797)
- pm-security negative-authz matrix for person-month submissions (#3801) + restrict non-manager person-month reads (#3796)
- pm-security audit of non-FORCE RLS tables + raw-pool repo bypass (#3799)
- fix(UC-10): residents see own-unit person counts only (#3792)
- fix(UC-67): DSA report card dates via active locale (#3795)
- test-gap-pr-3762: pin payment-match tiebreak at integration level (#3812)
- pm-scrum-master: watch-module decomposition triage doc (#3807)
- pm-scrum-master: promote mobile-RN i18n raw-status subset (#3803) — subset of #3676
- code-review wave: 14 findings resolved across ppt-web-ui (#3810 ConditionBuilder-0, #3811 AppShell UserMenu arrow-keys), reality-web (#3788, #3793, #3798, #3809), mobile-rn (#3787, #3789, #3805, #3806), api-core/handlers (#3802, #3808)
- Churn-hotspot fix: APNs push-type header (#3790)

## What's next (top 5)

1. **[high]** Merge the 5 approved PRs in review queue (#3800 pm-scrum uc-10-followup, #3813/#3814 screen-map drift, #3815 voice_webhooks dedupe, #3816 GuestReportSubmissionPage extract) — pm-scrum-master
2. **[high]** Continue Track 1 watch-module decomposition — PR #3815 is a start on voice_webhooks/handlers.rs; queue reports/mod.rs (~80K) and llm_document.rs (~79K) next — pm-tech-lead
3. **[high]** Track 2 (quarantine burn-down): start pm-backend's 193 rows — gated integration-test-lane proposal — pm-backend
4. **[high]** Track 3 (mobile-native): continue blocked — pm-devops to unblock KMP + swagger-ui egress in cloud runner (#2652, #2966) — pm-devops
5. **[medium]** pm-data wire-up: add quarantine-burndown weekly counter + APNs/auth negative-authz observability metrics — pm-data

## Blockers

- **Mobile-native (Track 3): cloud-runner KMP + swagger-ui egress blocked (#2652, #2966). Carried since 2026-08; 4/4 KMP items structurally unclaimable.**

## Role focus today

- pm-scrum-master (always-on) + pm-data (rotation index 6)

### pm-data one-line

Hardening sprint is producing a high rate of UC-scoped data-shape fixes (UC-10 own-unit person-counts #3792, UC-67 DSA i18n #3795, FORCE RLS on person_month_submissions + submission repo on RLS conn #3797/#3796) but the analytics surface for the quarantine burn-down and the APNs/FCM push reliability track is still thin. 193 pm-backend quarantine rows due 2027-02-19 have no week-over-week burndown metric, and the APNs eviction/push-type-header fixes (#3769, #3790) landed without an emitted counter that an alert could hang off.
