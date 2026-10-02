# PPT Roadmap — upkeep 2026-10-02

_Rebuilt from `coverage.json` (49/49 stories done across 13 epics) + current `action-list.json` + sprint-status.yaml. 21h since last routine run — on schedule._

## State of the project

- **Stories: 49/49 done** across 13 epics. No status flips this cycle.
- Delta vs 2026-10-01 upkeep: **3 PRs merged** since #3005 (#3009 RegistryRulesPage toast / #3010 api-client auth-header unified / #3011 facilities error UI). Zero coverage status flips (surfaces already done). epic-83 `last_checked` bumped to 2026-10-02 by the coverage-cursor re-check.
- **Buffer-low signal continues: `claimable=15/22`** (floor 36). Dropped from 72 total → 22 total open as prior refills consumed; 7 dep-blocked. The payload that triggered this routine fire was `buffer-low: claimable=15/72 — refill planner` (based on prior cycle snapshot). Root cause has shifted from **implementer-starved → reviewer-starved**: 11/11 active tasks sit in `review`.
- Screen coverage: 0 orphan screens · 0 validation errors · 3 known missing UC links (UC-33.1/33.2/33.3 already queued elsewhere).
- Open PRs: 26 total; 7 touched this cycle (all dependabot bulk). 2 non-dependabot stalled are the accounting trio (#2559 / #2555) at 65d.

## Epic status (short)

- **Epic 6 — Announcements & Communication:** 6/6 done in coverage; sprint-status still shows in-progress (drift — safe to re-reconcile next deep scan).
- **Epic 7A — Basic Document Management:** 5/5 done.
- **Epic 8A — Basic Notification Preferences:** 3/3 done.
- **Epic 10A — OAuth Provider Foundation:** 3/3 done.
- **Epic 10B — Platform Administration:** 7/7 done.
- **Epic 80 — Dispute Resolution:** 3/3 done in coverage (sprint-status still shows partial).
- **Epic 81 — Reports:** 2/2 done.
- **Epic 82 — iOS Reality Portal (SwiftUI):** 5/5 done. (`last_checked = 2026-10-01` from prior cursor pass.)
- **Epic 83 — Portal Integrations:** 3/3 done. **Re-checked this run (cursor idx 8).** No merged PR in the 2026-10-01..10-02 window touched portal-integration surfaces; `last_checked = 2026-10-02` stamped on all 3 stories.
- **Epic 84 — Documents & e-signature:** 5/5 done.
- **Epic 85 — Build/CI configuration:** 2/2 done.
- **Epic 79 — API client integration:** 4/4 done. PR #3010 (auth-header + MFA-retry unified) is new evidence on 79-1.
- **Epic 9 — 2FA/TOTP:** 1/1 done.

## Ranked plan

### Phase 1 (MVP, highest priority) — Infra unblocks + security retries

- **[high]** Unblock utoipa-swagger-ui egress (#2966) — single largest buffer lever; gates all 3 open IDOR retries.
- **[high]** Drain review queue: 11/11 active tasks in review (new this run) — reviewer capacity now the bottleneck.
- **[high]** Cross-tenant IDOR portfolio_analytics (#2946 retry 2/2) — blocked behind #2966.
- **[high]** Cross-tenant IDOR violation comments/evidence/payments (#2944 retry 1/2) — blocked behind #2966.
- **[high]** Cross-tenant IDOR portfolio_properties (#2945 retry 1/2) — blocked behind #2966.
- **[high]** Triage #3012 git-stash isolation bug before next parallel claim batch.
- **[high]** Unblock mobile-native/KMP cloud builds (#2652) — gates 7 KMP code-review items.

### Phase 2 — Data/analytics instrumentation (new this run)

- **[high]** Wire emit sites for the 4 declared KPI counters in `backend/servers/api-server/src/observability.rs:272-282` — or remove the dead describes. Dashboards silently show zero.
- **[high]** Author canonical domain-event catalogue (`docs/data/event-catalog.md`) covering fault, vote, payment, booking, dispute, announcement events.
- **[medium]** Define a fault-SLA metric; shared `FaultStatusCount` definition.
- **[medium]** Metrics-assertion test helper; require ≥3 KPI handlers to use it.
- **[medium]** Stripe webhook `payment_completed`/`payment_failed` counters with idempotency labels.

### Phase 3 — Reality-web correctness (Tier-1d findings this run)

- **[high]** `/sell` wizard discards every submission — wire Publish to the generated create-listing mutation. Promoted to `plans/code-review-reality-web-sell-wizard-no-persist.md`.
- Keep in backlog (score 2, medium confidence): `/report` page doesn't POST (`code-review-reality-web-report-page-no-submit`), `/profile` renders MOCK_* (`code-review-reality-web-profile-mock-data`).

### Phase 4 — Housekeeping

- Stalled accounting trio (#2559, #2555) at 65d — reviewer slot overdue.
- Triage dependabot bulk (#3008 / #3007 / #2981 / #2972 / #2962 / #2961 / #2957) — merge npm/cargo minors; defer KMP/compose until #2652 unblocks.
- Repeated-churn: `backend/servers/api-server/src/routes/integrations/webhook.rs` — currently CLOUD-BUILD-BLOCKED behind #2966.
- Mobile RN meter-reading numeric validation gap (#code-review-mobile-rn-meter-reading-no-numeric-validation) — blocked behind #2951 jest-expo rot.
- Reality-server duplicated URL-validation helper — merge into shared crate.
- `logout()` AUTHED_QUERY_KEY_ROOTS allow-list gap (session leak).

Buffer: **15 claimable / 22 open** (floor 36) — buffer-low. Primary lever is now **drain the review queue** (11 active, 0 claimed last cycle); secondary lever is **#2966 unblock** which cascades through security.
