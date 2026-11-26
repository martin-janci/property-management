# PPT delivery — roadmap

<sub>Generated: 2026-11-26 · scan_kind=upkeep (coverage epic re-check: epic-9)</sub>

## State of the project

- **Coverage:** 49/49 stories done across 13 epics (upkeep tick on epic-9 this run; all 10b/10a stories remained `done`).
- **Active sprint:** Hardening & Refactor — three tracks (modules-under-watch, quarantine burn-down, mobile-native).
- **Top 3 gaps (hardening-driven, not coverage-driven):**
  1. Track 1 (modules-under-watch): 6 oversized modules, 0 decomposed. voice_webhooks/handlers.rs refactor started in draft PR #3815.
  2. Track 2 (quarantine burn-down): 0/201 rows closed; 2027-02-19 cliff; needs weekly metric (pm-data risk).
  3. Track 3 (mobile-native KMP): 4/4 items structurally unclaimable until #2652 + #2966 are unblocked.
- **Screen coverage:** 0 stories without screen-map · 0 orphan epics · 0 orphan screens · 0 missing UC links (no drift from Phase 1.6 upkeep; routine Phase 1 flagged 3 reality-web drift signals this run — see brief).

## Ranked plan

### Hardening (active sprint, no phase weight because post-mvp)

- [high] Merge the 5 approved PRs in review queue (#3800 pm-scrum uc-10-followup, #3813/#3814 screen-map drift, #3815 voice_webhooks dedupe, #3816 GuestReportSubmissionPage extract) — owner: pm-scrum-master — why: hardening-sprint next step
- [high] Continue Track 1 watch-module decomposition — PR #3815 is a start on voice_webhooks/handlers.rs; queue reports/mod.rs (~80K) and llm_document.rs (~79K) next — owner: pm-tech-lead — why: hardening-sprint next step
- [high] Track 2 (quarantine burn-down): start pm-backend's 193 rows — gated integration-test-lane proposal — owner: pm-backend — why: hardening-sprint next step
- [high] Track 3 (mobile-native): continue blocked — pm-devops to unblock KMP + swagger-ui egress in cloud runner (#2652, #2966) — owner: pm-devops — why: hardening-sprint next step
- [medium] pm-data wire-up: add quarantine-burndown weekly counter + APNs/auth negative-authz observability metrics — owner: pm-data — why: hardening-sprint next step
- [high] Emit a weekly quarantine-burndown counter from the quarantine-age-gate workflow (open rows + days-to-earliest-due) and surface it on the Phase 1.6 scrum-master digest — makes the 2027-02-19 cliff visible before it bites — owner: pm-data — why: observability/KPI gap
- [medium] Add KPI counters for APNs push eviction + apns-push-type header path (success/failure/410) alongside the FCM parity already covered by #3771 regression tests — owner: pm-data — why: observability/KPI gap
- [medium] Add negative-authz observability: count 403 responses from cross-unit person-month reads (now that #3792 narrowed scope to own-unit and #3796 added the negative-authz matrix) — flags a policy regression before customers notice — owner: pm-data — why: observability/KPI gap
- [low] Define 'abandoned' semantics (carried from 2026-11-17): either `notification_digests.abandoned_at TIMESTAMPTZ` column (new migration after 00251) or a dead-letter table; wire report_abandoned to the chosen shape — owner: pm-data — why: observability/KPI gap
- [low] Normalize code-review-finding throughput telemetry: today's run resolved 14 code-review signals but signals/<date>.json never emitted a done-rate time series — add 'resolved_signals_per_run' so the delivery picture shows throughput — owner: pm-data — why: observability/KPI gap

Buffer: 51/36 open · 0 candidates ranked but unqueued (hardening sprint runs off next_actions, not candidate ranking).
