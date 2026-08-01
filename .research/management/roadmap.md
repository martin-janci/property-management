# PPT Roadmap — upkeep 2026-08-01

## State of the project

- Stories: **47 done / 2 partial / 0 not-started** of 49 (13 epics). Unchanged since 2026-07-15 deep scan.
- Delta vs 2026-07-30 upkeep: no story-status flips this window. 5 PRs merged (all post-merge hygiene): the auto-review loop closed three in-progress items (gh-issue-2485 via PR #2614, test-gap-screen-map-drift-pr-2600-reality via PR #2618, pm-backend-scheduler-rs-refactor-extract-jobs partially via PR #2613). One new follow-up issue landed (#2612, fire-once notifications) — pending triage.
- Remaining gaps (the last 2 partial stories, both frontend slices on shipped APIs):
  1. **84-1** — ppt-web still uploads via server proxy; direct-to-S3 endpoint (#2309) has no frontend consumer. **Still blocked on gh-issue-2573**, now 3 days without backend churn.
  2. **84-2** — signer-facing document-sign page not built (screen-map planned, API complete); prior implementer attempts have failed 3× as single-squash — **needs slice-split** into route/manifest → capture → verify+delivery.
- Screen coverage: 0 orphan screens · 0 validation errors · 3 missing UC links (UC-33.x — 2 queued 2026-07-30, UC-33.3 queued this run).
- Buffer: **10 open / 31 in-progress** (44 items total after 3 marked done + 6 new). The "in-progress" pile is inflated by dispatcher polling; the true fresh-open queue is 10 items — half-empty relative to the 36-slot buffer.

## Ranked plan

### mvp / finish-what's-started (highest score, 8)

- [high] **Split 84-2 signer sign page into 3 mergeable slices** — (1) `/sign/:token` route + fetch signer manifest, (2) canvas signature capture + PATCH signature-request, (3) verification & delivery-confirmation UI. Retry3 as single squash failed — go incremental with per-slice screen-map flip. Owner: pm-frontend.
- [high] Wire ppt-web direct-to-S3 upload via POST /api/v1/documents/upload-url — api-client binding + UploadDocument integration + regression test (84-1 partial) — owner: pm-frontend — **blocked-on: gh-issue-2573 (still open 3 days)**

### unblock + escalation (score 7-8, high priority)

- [high] Fix gh-issue-2573 — DELETE /documents/by-file-key can delete a still-referenced object within the same org (data-loss regression from PR #2571) — owner: pm-backend — **blocking 84-1**
- [high] Escalate accounting MVP-loop trio (#2555 / #2558 / #2559) — 4-day reviewer starvation (up from 2 days last run); apply DEC-107 reviewer-slot policy or split — owner: pm-tech-lead
- [high] Fix gh-issue-2574 — Android SSO CSRF guard half-wired (SsoStateStore.mint() has no call site) — owner: pm-mobile
- [medium] Fix gh-issue-2575 — /disputes/kpis has no window-ordering validation, only test is quarantined — owner: pm-backend
- [medium] Triage new open issue #2612 (fire-once notifications) — decide owner_role, scope — owner: pm-tech-lead

### security cross-cutting (score 7, carried)

- [high] Cross-cutting webhook hardening audit — booking / airbnb / esignature / layout — layout leg now covered by PR #2614; remaining legs still open — owner: pm-integration
- [medium] Alexa voice webhook accepts forged requests — verify_alexa_signature never checks the signature (SECURITY) — owner: pm-security

### post-merge follow-ups (carried from prior windows, score 5-6)

- [medium] Follow-up #2483: add_evidence dispute sub-resource cross-tenant-writable — owner: pm-tech-lead (in-progress via PR #2490)
- [medium] Follow-up #2484: announce fan-out real-SQL integration test — owner: pm-tech-lead
- [medium] Follow-up #2486: mobile LAYOUT_CACHE_KEY tenant scoping — owner: pm-tech-lead
- [medium] Follow-up #2366 (retry 2/2): direct-to-S3 upload drops building_id — owner: pm-tech-lead
- [medium] Follow-up #2241 (retry 2): OAuth state single-use not atomic in prod Redis — owner: pm-tech-lead
- [medium] Follow-up #2318 (retry 2): report-schedule due-work consumer RLS no-op — owner: pm-tech-lead
- [medium] Follow-up #2320 (retry 2): harden direct-to-S3 upload flow (IDOR at registration, size cap, orphans) — owner: pm-tech-lead
- [medium] Follow-up #2528: booking webhook parity — owner: pm-tech-lead
- [medium] Follow-up #2530: full signup funnel instrumentation — owner: pm-tech-lead

### frontend consolidation (new this run, score 5)

- [medium] **Extend #2616's ui-kit consolidation pattern to ppt-web** — audit top-10 duplicated Spinner/EmptyState/Button variants (per code-review-ppt-web-ui-duplicated-spinner-markup) and land as one deprecation PR — owner: pm-frontend

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
- [medium] **Link UC-33.3 to a dispute screen-map** (missing_use_cases from coverage; last of the UC-33.x wave) — owner: pm-frontend

### churn hotspots + chore (score 1-2, low)

- [low] repeated-churn: auth.rs (runs_seen=5, 2950 lines this window)
- [low] repeated-churn: crates/integrations/src/booking/mod.rs (3626 lines; PR #2611 split, PR #2619 draft continues pattern)
- [low] repeated-churn: routes/reports.rs (3329 lines this window; PR #2599 extracted helpers)
- [low] Investigate services/scheduler.rs residual churn — retention/prune jobs still not extracted (vote-lifecycle done via PR #2613)
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
- [low] Cloud routine cadence recovery — reduce 3–4d gaps between runs (retry 2/2)
- [low] gh-issue-2556: add reality-api-client drift gate (deferred from #2487) — owner: pm-tech-lead
- [low] Add regression test for PR #2547 scheduler retention prune (still flagged hotfix-no-test) — owner: pm-backend/pm-qa

Buffer: **10 fresh-open / 31 in-progress (total 41 active)** · project at 47/49 — auto-review loop closed 3 items and shipped 5 PRs of hygiene. Real blocker is the **frontend `partial` chain** (84-1 backend-blocked, 84-2 retry-pattern-blocked) — not implementer or reviewer capacity per se, but a coordination pass. ⚠ Fresh-open buffer below half (10/36) — dispatcher should consider a `scan` refresh soon.
