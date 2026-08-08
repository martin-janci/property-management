# PPT Roadmap — upkeep 2026-08-08

## State of the project

- Stories: **47 done / 2 partial / 0 not-started** of 49 (13 epics). Coverage-view epics done: **12/13** (only epic-84 has partial stories — 84-1, 84-2).
- Delta vs 2026-08-06 upkeep: no story-status flips this window. 7 PRs merged since #2702 — heavy on code-review clears (#2706, #2707, #2708, #2709, #2712) + two layout-hotspot refactors (#2711, #2713). Auto-fix loop closed #2704 (memory-DoS) in <24h and is now working #2703 (SSRF, draft PR #2710).
- Remaining gaps (the last 2 partial stories, both frontend slices on shipped APIs):
  1. **84-1** — ppt-web still uploads via server proxy; direct-to-S3 endpoint (#2309) has no frontend consumer. Blocked on #2573 same-org reference-check fix.
  2. **84-2** — signer-facing document-sign page not built (screen-map planned, API complete); prior implementer attempt failed no-PR.
- Screen coverage: 0 orphan screens · 0 validation errors · 3 missing UC links (UC-33.x dispute sub-UCs — 2 queued, 1 remaining).
- Coverage cursor: idx 5 → 6 (epic-80 re-checked this run, no material change; advances to epic-81 next). PR #2712 evidence added to 80-1 dispute-audit trail.

## Ranked plan

### mvp / finish-what's-started (highest score, 8)

- [high] Wire ppt-web direct-to-S3 upload via POST /api/v1/documents/upload-url — api-client binding + UploadDocument integration + regression test (84-1 partial) — owner: pm-frontend — **blocked-on: #2573 reference-check fix**
- [high] Build signer-facing document-sign page in ppt-web against shipped signing API; flip screen-map ppt/document-sign buildStatus planned→shipped; verify signature-request email delivery (84-2 partial) — owner: pm-frontend

### security / this-window follow-ups (score 7-8, high priority)

- [high] Merge PR #2710 + resolver-spoof regression test — closes #2703 (SSRF DNS-rebinding TOCTOU, live vuln in workflow api_call.rs) — owner: pm-tech-lead
- [high] Sequence-lock #2696 (inquiry-email seam) with `code-review-reality-server-inquiry-notify-route-wiring` — merge as a pair; solo #2696 ships silent-success — owner: pm-backend / pm-qa
- [high] Backfill regression tests for the two 2026-08-07 hotfix-no-test slips: #2707 (body-cap) and #2712 (add_evidence audit) — owner: pm-qa
- [high] Vendor the utoipa-swagger-ui zip so api-server tests can run locally (every refactor PR this window defers to CI) — owner: pm-devops

### carried post-merge follow-ups (score 5-7, medium)

- [medium] Fix #2573 — DELETE /documents/by-file-key can delete a still-referenced object within the same org (blocks 84-1) — owner: pm-backend
- [medium] Fix #2574 — Android SSO CSRF guard half-wired (SsoStateStore.mint() has no call site) — owner: pm-mobile
- [medium] Fix #2575 — /disputes/kpis has no window-ordering validation, only test is quarantined — owner: pm-backend
- [medium] Follow-up #2483: add_evidence dispute sub-resource cross-tenant-writable — owner: pm-tech-lead (in-progress via PR #2490)
- [medium] Follow-up #2484: announce fan-out real-SQL integration test — owner: pm-qa (replaces pure-Rust model)
- [medium] Follow-up #2485: layout publish webhook timestamp/replay protection — owner: pm-tech-lead
- [medium] Follow-up #2486: mobile LAYOUT_CACHE_KEY tenant scoping — owner: pm-tech-lead
- [medium] Cross-cutting webhook hardening audit — booking / airbnb / esignature / layout (#2528 booking webhook still open) — owner: pm-integration
- [medium] Review + shepherd merge of accounting MVP-loop trio (#2555, #2558, #2559) — 2-day reviewer starvation carried from 2026-07-30 — owner: pm-tech-lead
- [medium] Reconcile sprint-status.yaml — epics 6/7a/10b/80 show coverage=done, sprint-yaml still in-progress — owner: pm-scrum-master

### pm-data KPI gap wave (score 4-5, carried)

- [medium] Define layout publish/webhook analytics events
- [medium] Define dispute-lifecycle KPI set (funnel + TTR percentiles + evidence-per-dispute)
- [medium] Instrument announcement fan-out with delivered/read/ack per targeting scope
- [medium] Support-staff read audit event schema
- [medium] Publish data-retention policy for support-data / audit trail
- [medium] Formalize FaultStatusCount canonical definition
- [medium] Instrument signup/onboarding-tour completion funnel (10b-6)

### Screen-map drift (score 3-4)

- [medium] Link UC-33.1 to a dispute screen-map (missing_use_cases from coverage) — owner: pm-frontend
- [medium] Link UC-33.2 to a dispute screen-map (missing_use_cases from coverage) — owner: pm-frontend

### churn hotspots + chore (score 1-2, low)

- [low] Churn hotspot: routes/auth.rs (2950 lines, runs_seen=5) — retry1 in-progress — owner: pm-tech-lead
- [low] Churn hotspot: routes/reports.rs (3329 lines) — retry1 open — owner: pm-tech-lead
- [low] Churn hotspot: services/scheduler.rs / crates/integrations/src/booking/mod.rs
- [low] Follow-up: WebSocket not re-authed on token rotation — owner: pm-backend
- [low] SECURITY: reality-web layout.tsx inlines tenant-config JSON without escaping — owner: pm-security
- [low] AmlDashboardPage casts raw window.prompt text into review-decision union — owner: pm-backend
- [low] PortfolioAnalytics inquiriesTrend drops days with inquiries but zero views — owner: pm-backend
- [low] reality-web listingAnalytics.ts casts untrusted ?source= to ViewSource union — owner: pm-tech-lead
- [low] Stale TODO(security) headers in faults.rs / critical_notifications.rs — owner: pm-devops
- [low] admin-web platform-settings + mobile-config Save paths are permanent no-ops — owner: pm-devops
- [low] gh-issue-2556: add reality-api-client drift gate — owner: pm-tech-lead

Buffer: **18/36 open** · project at 47/49 — buffer is at half-full; a full deep `scan` would refill it. This run is intentional-upkeep only (per task instructions: the deep scan is LOCAL-only, not appropriate for the cloud routine). Below-half buffer warning fires; recommend queuing a scan run when a human is at the terminal.
