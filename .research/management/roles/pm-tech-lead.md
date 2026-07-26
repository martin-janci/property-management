# pm-tech-lead role output

_Generated: 2026-07-26_

## Summary

Architecture is functionally converging (47/49 stories done) but three route/integration files (auth.rs, reports.rs, booking/mod.rs — all 3000+ lines) are absorbing repeat churn without a refactor decision, and this window's test-restoration wave plus the #2547 scheduler test-gap point to a systemic pattern of shipping without durable test coverage for background jobs and god-files.

## Next actions

- **[high]** Open a refactor RFC to split auth.rs (2950 lines, 4th repeat-churn cycle - now also touched by draft PR #2553 cold-boot fix) into session/OAuth/MFA submodules before the next auth-adjacent epic lands
  - _dependency_: pm-backend
  - _definition of done_: RFC or module-split PR reviewed; auth.rs responsibilities documented
- **[medium]** Apply the same review to reports.rs (3329 lines, 3rd repeat-churn cycle, growing further via epic-6 SQL-backed dispute KPIs)
  - _dependency_: pm-backend
  - _definition of done_: Decision recorded: split now vs. defer, with size trigger threshold
- **[high]** Establish a standard test pattern for scheduler-fired jobs (services/scheduler.rs) - retention prune (#2547), auto-unpin, favorite-alerts, notification triggers all lack a verified 'job actually fires at runtime' test
  - _dependency_: pm-backend
  - _definition of done_: Injectable-clock or trigger-once harness merged and applied to #2547 + at least one prior job
- **[medium]** Confirm whether Wave J1's un-quarantine-then-re-quarantine-11 (PR #2511) reflects real flakiness vs. a one-off, before continuing to burn down the BIT-5xx/6xx quarantine backlog
  - _dependency_: pm-qa
  - _definition of done_: Root-cause note on the 11 re-quarantined tests; backlog burn-down net-positive confirmed
- **[medium]** Assess booking/mod.rs as a first-time hot spot against the already-roadmapped cross-cutting webhook hardening audit - determine if a shared OTA-provider abstraction is needed to stop per-provider file bloat
  - _dependency_: pm-integration
  - _definition of done_: Finding folded into the webhook-hardening audit scope
- **[medium]** Track draft PR #2553 (AuthContext cold-boot bypass, scope_drift=true) to merge with explicit awareness it's landing on top of the auth.rs debt, not in isolation
  - _dependency_: pm-frontend
  - _definition of done_: PR reviewed with auth-surface-wide regression check, not just its own test

## Risks

- **[high×medium]** auth.rs and reports.rs are god-files absorbing repeat cross-epic churn (OAuth, MFA, cold-boot fixes on auth.rs; dispute KPIs on reports.rs) with no refactor decision made
  - _mitigation_: Bound file growth with a module-split threshold; assign clear sub-ownership
- **[medium×medium]** booking/mod.rs (3185 lines) newly hot despite 83-2 marked done - could signal instability reintroduced into finished integration code
  - _mitigation_: Fold into the already-planned cross-cutting webhook/integration audit
- **[medium×high]** Test-restoration wave un-quarantines then re-quarantines tests in the same PR (#2511: 48 restored, 11 re-quarantined) - may be masking real flakiness under a 'quarantine debt paid down' narrative
  - _mitigation_: Require root-cause note per re-quarantined test, not silent re-ignore
- **[high×high]** Recurring cross-tenant IDOR patches across independent domains (documents #2438/#2447, disputes #2450/#2483, groups #2548) suggest no centralized authz-enforcement check at handler registration
  - _mitigation_: Add a lint/macro or checklist gate requiring explicit org-scope proof for any new tenant-scoped handler
- **[medium×medium]** #2547 scheduler-firing test gap is not isolated - auto-unpin, favorite-alerts, notification-triggers jobs likely share the same 'hard to test at runtime' blind spot
  - _mitigation_: Build one reusable scheduler test harness rather than one-off fixes per job

## Open questions

- Is there an existing decision/owner for splitting auth.rs and reports.rs, or is this repeat churn purely accretive feature work with no architecture checkpoint?
- What is the net trend of the quarantine backlog (BIT-3xx-6xx) - is Wave J1's 11 re-quarantined tests a net loss against the ~200 restored, or noise?
- Is a shared OTA/integration-provider abstraction planned for booking/airbnb/portal-webhooks, or is each provider file independently growing toward the same 3000+ line ceiling as auth.rs/reports.rs?
- Does any test currently assert that services/scheduler.rs actually registers and fires the retention-prune job at startup, or is #2547 entirely unverified in CI?

## Decisions needed

- Set a concrete refactor trigger/threshold for route files exceeding ~2500 lines (auth.rs, reports.rs, booking/mod.rs all past it) - owner: pm-tech-lead
- Standardize a test harness pattern (injectable clock / trigger-once mode) for scheduler-invoked jobs before more scheduled features ship - owner: pm-backend
- Decide whether to mandate an authz/org-scoping lint or macro check at handler registration to stop the recurring cross-tenant IDOR patch cycle - owner: pm-security

