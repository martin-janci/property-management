# pm-data

<sub>Generated: 2026-11-26 — rotating role analysis for the research routine.</sub>

## Summary

Hardening sprint is producing a high rate of UC-scoped data-shape fixes (UC-10 own-unit person-counts #3792, UC-67 DSA i18n #3795, FORCE RLS on person_month_submissions + submission repo on RLS conn #3797/#3796) but the analytics surface for the quarantine burn-down and the APNs/FCM push reliability track is still thin. 193 pm-backend quarantine rows due 2027-02-19 have no week-over-week burndown metric, and the APNs eviction/push-type-header fixes (#3769, #3790) landed without an emitted counter that an alert could hang off.

## Next actions

- **[high · pm-devops]** Emit a weekly quarantine-burndown counter from the quarantine-age-gate workflow (open rows + days-to-earliest-due) and surface it on the Phase 1.6 scrum-master digest — makes the 2027-02-19 cliff visible before it bites
  - DoD: metric (rows_open, days_to_earliest_due) collected on every main CI run and added to the delivery digest
- **[medium · none]** Add KPI counters for APNs push eviction + apns-push-type header path (success/failure/410) alongside the FCM parity already covered by #3771 regression tests
  - DoD: counters emitted in api-server/src/services/push/apns.rs, documented in the metrics catalog, backfilled onto the delivery dashboard
- **[medium · none]** Add negative-authz observability: count 403 responses from cross-unit person-month reads (now that #3792 narrowed scope to own-unit and #3796 added the negative-authz matrix) — flags a policy regression before customers notice
  - DoD: 403 counter per endpoint with route+role labels; dashboard panel showing baseline
- **[low · pm-backend]** Define 'abandoned' semantics (carried from 2026-11-17): either `notification_digests.abandoned_at TIMESTAMPTZ` column (new migration after 00251) or a dead-letter table; wire report_abandoned to the chosen shape
  - DoD: migration merged, report_abandoned writes the chosen shape, alert wired on abandoned_count > N/sweep
- **[low · none]** Normalize code-review-finding throughput telemetry: today's run resolved 14 code-review signals but signals/<date>.json never emitted a done-rate time series — add 'resolved_signals_per_run' so the delivery picture shows throughput
  - DoD: field in signals schema, backfilled for last 7 runs, charted in roadmap.md 'State of the project'

## Risks

- **[high · high]** Quarantine burn-down has no weekly metric; 201 rows to zero by 2027-02-19 is a schedule-visible cliff only through manual grep. CI fails LATE, not when burn-down stalls.. Mitigation: Phase 1.6 counter from workflow output; alert when weekly delta < required run-rate (currently 201/13 weeks ≈ 15.5 rows/week)
- **[medium · medium]** Push transport metrics (APNs eviction, FCM parity, apns-push-type) are code-only without counter emission — regressions surface as user complaints, not alerts.. Mitigation: Add Prometheus counters alongside the hardening PRs that just merged (#3769, #3771, #3790)
- **[medium · high]** Multi-unit authz regressions on person-month tables are now test-gated (#3796, #3801) but not metric-gated; a code path that bypasses RLS wouldn't trigger the tests until review.. Mitigation: Dashboard panel: 403 rate by route × role; alert on sudden drop (missing enforcement) or spike (misconfig)
- **[medium · medium]** Dispatcher claim+review throughput (research/management/assignments.json) has no time-to-merge histogram; today's queue has 5 approved PRs in review — if they stall the backlog grows but the digest doesn't scream.. Mitigation: Emit queue-age percentiles from the dispatcher into the digest; block new claims past a threshold

## Open questions

- Is there any metrics backend plugged into api-server (Prometheus exporter, StatsD, OpenTelemetry) or are all counters dead-ended?
- Does the quarantine-age-gate workflow emit structured output a scraper can consume, or only a boolean pass/fail?
- Who owns the cross-cutting 'delivery KPIs' dashboard — pm-data vs pm-devops?

## Decisions needed

- Standardize the metrics stack (Prometheus vs vendor) before more counters land on push/auth — owner: pm-tech-lead
- Confirm 'abandoned' semantics for notification_digests (column vs dead-letter) — carried from 2026-11-17 — owner: pm-backend
