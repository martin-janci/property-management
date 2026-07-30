# PPT Roadmap — upkeep 2026-07-30

## State of the project

- Stories: **47 done / 2 partial / 0 not-started** of 49 (13 epics). Unchanged since 2026-07-15 deep scan.
- Delta vs 2026-07-27 upkeep: no story-status flips this window. 17 PRs merged (dominated by follow-up hardening + auto-fix loop): all of the 2026-07-24 post-merge-review batch is now closed except #2528 (booking webhook parity). Two same-day merges (#2571 DELETE-by-file-key, #2568 Android SSO CSRF) each spawned a follow-up issue (#2573, #2574) — the auto-review loop is catching regressions inside 24h.
- Remaining gaps (the last 2 partial stories, both frontend slices on shipped APIs):
  1. **84-1** — ppt-web still uploads via server proxy; direct-to-S3 endpoint (#2309) has no frontend consumer. Now blocked on #2573 reference-check fix landing first.
  2. **84-2** — signer-facing document-sign page not built (screen-map planned, API complete); prior implementer attempt failed no-PR.
- Screen coverage: 0 orphan screens · 0 validation errors · 3 missing UC links (UC-33.x dispute sub-UCs — 2 queued this run, 1 remaining).
- Buffer: **36/36 open** (refilled from 31/36 open pre-run — 6 new items, 1 marked done by PR #2576).

## Ranked plan

### mvp / finish-what's-started (highest score, 8)

- [high] Wire ppt-web direct-to-S3 upload via POST /api/v1/documents/upload-url — api-client binding + UploadDocument integration + regression test (84-1 partial) — owner: pm-frontend — **blocked-on: #2573 reference-check fix**
- [high] Build signer-facing document-sign page in ppt-web against shipped signing API; flip screen-map ppt/document-sign buildStatus planned→shipped; verify signature-request email delivery (84-2 partial) — owner: pm-frontend

### post-merge follow-ups from this window (score 7-8, high priority)

- [high] Fix #2573 — DELETE /documents/by-file-key can delete a still-referenced object within the same org (data-loss regression from PR #2571) — owner: pm-backend
- [high] Fix #2574 — Android SSO CSRF guard half-wired: SsoStateStore.mint() has no call site so every reality://sso callback is rejected (regression from PR #2568) — owner: pm-mobile
- [medium] Fix #2575 — /disputes/kpis has no window-ordering validation, only test is quarantined (from PR #2572) — owner: pm-backend
- [medium] Add regression test for PR #2547 scheduler retention prune (still flagged hotfix-no-test) — owner: pm-backend/pm-qa

### security cross-cutting (score 7, carried)

- [high] Cross-cutting webhook hardening audit — booking / airbnb / esignature / layout — #2528 booking webhook is the last unresolved leg from the 2026-07-24 batch — owner: pm-integration
- [medium] Alexa voice webhook accepts forged requests — verify_alexa_signature never checks the signature (SECURITY) — owner: pm-security

### post-merge follow-ups (carried from prior windows, score 5-6)

- [medium] Follow-up #2483: add_evidence dispute sub-resource cross-tenant-writable — owner: pm-tech-lead (in-progress via PR #2490)
- [medium] Follow-up #2484: announce fan-out real-SQL integration test — owner: pm-tech-lead
- [medium] Follow-up #2485: layout publish webhook timestamp/replay protection — owner: pm-tech-lead
- [medium] Follow-up #2486: mobile LAYOUT_CACHE_KEY tenant scoping — owner: pm-tech-lead
- [medium] Follow-up #2366 (retry 2/2): direct-to-S3 upload drops building_id — owner: pm-tech-lead
- [medium] Follow-up #2241 (retry 2): OAuth state single-use not atomic in prod Redis — owner: pm-tech-lead
- [medium] Follow-up #2318 (retry 2): report-schedule due-work consumer RLS no-op — owner: pm-tech-lead
- [medium] Follow-up #2320 (retry 2): harden direct-to-S3 upload flow (IDOR at registration, size cap, orphans) — owner: pm-tech-lead

### review + coordination (score 4-5, medium)

- [medium] Review + shepherd merge of accounting MVP-loop trio (#2555, #2558, #2559) — 2-day reviewer starvation — owner: pm-tech-lead
- [medium] repeated-churn: auth.rs (runs_seen=4, 2950 lines, 2nd-largest route file) — plan a module split — owner: pm-tech-lead

### pm-data KPI gap wave (score 4-5, carried)

- [medium] Define layout publish/webhook analytics events (published_by, layout_version, target_tenant_count)
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

- [low] Investigate services/scheduler.rs churn — extract retention/prune jobs to dedicated module — owner: pm-backend
- [low] Churn hotspot: routes/reports.rs (3329 lines this window)
- [low] Churn hotspot: crates/integrations/src/booking/mod.rs (3185 lines this window)
- [low] Churn hotspot: platform_admin_authz_batch2/org_property_authz_backfill/infra_ops_authz_backfill (BIT-268/557/559 test backfill triage)
- [low] Follow-up screen-map drift: PR #2497 reality-web/app/api/layout-revalidate/route.ts w/o docs/screens/reality/ update — owner: pm-qa
- [low] Follow-up: 10 ungated console.warn/error in ppt-web websocket.ts leak diagnostics in prod — owner: pm-tech-lead
- [low] Follow-up: WebSocket not re-authed on token rotation — owner: pm-backend
- [low] SECURITY: reality-web layout.tsx inlines tenant-config JSON into `<script>` without escaping — owner: pm-security
- [low] AmlDashboardPage casts raw window.prompt text into review-decision union — owner: pm-backend
- [low] PortfolioAnalytics inquiriesTrend drops days with inquiries but zero views — owner: pm-backend
- [low] reality-web listingAnalytics.ts casts untrusted ?source= to ViewSource union — owner: pm-tech-lead
- [low] Stale TODO(security) headers in faults.rs / critical_notifications.rs — owner: pm-devops
- [low] admin-web platform-settings + mobile-config Save paths are permanent no-ops — owner: pm-devops
- [low] Triage closed-not-merged PRs #2385/#2387/#2489 (dependabot supersedes) — owner: pm-devops/pm-tech-lead
- [low] Cloud routine cadence recovery — reduce 3–4d gaps between runs (retry 2/2)
- [low] gh-issue-2556: add reality-api-client drift gate (deferred from #2487) — owner: pm-tech-lead

Buffer: **36/36 open** · project at 47/49 — backlog is again converging on the 2 remaining frontend slices plus a healthy pipeline of freshly-caught follow-ups from this window's own PRs. The auto-review loop is doing its job.
