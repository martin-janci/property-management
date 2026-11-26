# pm-scrum-master

<sub>Generated: 2026-11-26 — rotating role analysis for the research routine.</sub>

## Summary

22 PRs landed since the last run — the most active single day in the hardening sprint. Headlined by pm-security's FORCE RLS on person_month_submissions and the submission-repo RLS-conn migration (#3797), UC-10 own-unit person-count scoping (#3792), DSA i18n dates fix (#3795), and a 14-PR code-review-finding wave across ppt-web-ui, reality-web, mobile-rn, and api-core/handlers. Five PRs are approved and sitting in the review queue (#3800, #3813, #3814, #3815, #3816) — that's the next merge wave.

## Next actions

- **[high · none]** Merge the 5 approved PRs in review queue (#3800 pm-scrum uc-10-followup, #3813/#3814 screen-map drift, #3815 voice_webhooks dedupe, #3816 GuestReportSubmissionPage extract)
  - DoD: all 5 PRs merged into dev; review queue drains to <=2
- **[high · none]** Continue Track 1 watch-module decomposition — PR #3815 is a start on voice_webhooks/handlers.rs; queue reports/mod.rs (~80K) and llm_document.rs (~79K) next
  - DoD: at least one more watch-module decomposition PR raised this week
- **[high · pm-backend]** Track 2 (quarantine burn-down): start pm-backend's 193 rows — gated integration-test-lane proposal
  - DoD: integration-test-lane proposal PR opened; docs/quarantine-policy.md references the lane
- **[high · pm-devops]** Track 3 (mobile-native): continue blocked — pm-devops to unblock KMP + swagger-ui egress in cloud runner (#2652, #2966)
  - DoD: issues #2652 + #2966 closed; KMP CI green in the cloud routine
- **[medium · pm-data]** pm-data wire-up: add quarantine-burndown weekly counter + APNs/auth negative-authz observability metrics
  - DoD: metrics emitted + alert thresholds documented
