# PPT Roadmap — upkeep 2026-08-09

## State of the project

- Stories: **47 done / 2 partial / 0 not-started** of 49 (13 epics). Composition unchanged since 2026-07-15 deep scan.
- Delta vs 2026-08-06 upkeep: no story-status flips this window. **18 PRs merged** in a single 24h burst — 3 issues closed (#2703/#2704/#2612), 4 security-adjacent (SSRF/DoS/NaN/IDOR), 1 PR quarantined (#2684, fix_rounds=3 exhausted). 84-1 and 84-2 remain the only 2 partial stories (both frontend on shipped APIs).
- **Remaining gaps (the 2 partial stories):**
  1. **84-1** — ppt-web still uploads via server proxy; direct-to-S3 endpoint (#2309) has no frontend consumer. Prior implementer attempt failed no-PR.
  2. **84-2** — signer-facing document-sign page not built (screen-map planned, API complete); prior implementer attempt failed no-PR.
- **Screen coverage:** 0 orphan screens · 0 validation errors · 3 missing UC links (UC-33.x dispute sub-UCs — all 3 queued this run).
- **Buffer: 49/36 open** — refilled from 2/36 pre-run (dispatcher trigger claimable=1/72). 3 in-progress, 1 failed/quarantined.

## Ranked plan

### mvp / finish-what's-started (highest score, 8)

- [high] Wire ppt-web direct-to-S3 upload via POST /api/v1/documents/upload-url — api-client binding + UploadDocument integration + regression test (84-1 partial) — owner: pm-frontend
- [high] Build signer-facing document-sign page in ppt-web against shipped signing API; flip screen-map ppt/document-sign buildStatus planned->shipped; verify signature-request email delivery (84-2 partial) — owner: pm-frontend

### Quarantine + in-review triage (high priority)

- [high] Human triage of PR #2684 (workflow_cond_parse_failopen) — CI test-shard(1-4) RED after 3 respawns; classify as flake vs real regression — owner: pm-tech-lead
- [medium] Shepherd 3 in-review reality-server PRs (#2724 db-error-leak, #2725 password-reset transport, #2726 sso-session-invalidate) — verify gate blocked in cloud by utoipa-swagger-ui egress — owner: pm-tech-lead

### post-merge test-coverage backfill (score 7, high — pm-qa focus)

- [high] SSRF DNS-rebinding regression test for PR #2710 — resolve-then-connect race guarded by DNS pin — owner: pm-security
- [medium] DoS body-cap regression test for PR #2707 workflow api_call — oversized body reject with 413 — owner: pm-backend
- [medium] Workflow NaN condition reject unit test for PR #2708 — evaluate_conditions on NaN operand returns error — owner: pm-backend
- [medium] Scheduled notification retry regression test for PR #2714 (closes #2612) — retry backoff + terminal state — owner: pm-backend
- [medium] Codify #2718 no-fix HMAC-parity outcome as webhook-parity policy — owner: pm-tech-lead

### security cross-cutting (score 6-7)

- [medium] Post-merge security regression sweep: 4 security-adjacent PRs this window (#2707/#2708/#2710/#2722) — verify each has co-committed regression test — owner: pm-security
- [medium] Extend #2722 IDOR fix pattern: sweep for handlers missing principal extractor — owner: pm-security
- [medium] SECURITY: Alexa voice webhook accepts forged requests (verify_alexa_signature is a no-op) — owner: pm-security
- [medium] SECURITY: reality-web layout.tsx inlines tenant-config JSON into <script> without escaping — owner: pm-security
- [medium] Cross-cutting webhook hardening: booking + Airbnb parity (#2528) — owner: pm-integration

### post-merge follow-ups (carried, score 5-6)

- [medium] Follow-up #2484 real-SQL RLS integration test for announcement cross-tenant fan-out (counterpart to #2723 metrics) — owner: pm-backend
- [medium] Follow-up #2486 mobile LAYOUT_CACHE_KEY tenant scoping + regression test — owner: pm-mobile
- [medium] Follow-up #2483 verify PR #2712 closes add_evidence sub-resource IDOR (needs subroute-authz regression) — owner: pm-backend
- [medium] Follow-up #2530 full signup funnel instrumentation — owner: pm-data
- [medium] Retry 3 gh-issue-2241 OAuth state single-use non-atomic in prod Redis — owner: pm-backend
- [medium] Retry 3 gh-issue-2318 report-schedule due-work RLS no-op — owner: pm-backend
- [medium] Retry 3 gh-issue-2320 direct-S3 IDOR + size cap + orphans — owner: pm-backend
- [medium] Retry 2 gh-issue-2366 direct-S3 upload drops building_id — owner: pm-backend

### pm-data KPI gap wave (score 4-5, carried + refreshed)

- [medium] Define layout publish/webhook analytics events — owner: pm-data
- [medium] Define dispute-lifecycle KPI set (funnel + TTR + evidence-per-dispute) — owner: pm-data
- [medium] Support-staff read audit event schema — owner: pm-data
- [medium] Formalize FaultStatusCount canonical definition — owner: pm-data
- [medium] Publish data-retention policy for support-data + audit trail — owner: pm-data

### DX / CI (score 4-5)

- [medium] Cloud-egress fix: mirror utoipa-swagger-ui build deps into allow-list (blocks local verify for #2724/#2725/#2726) — owner: pm-devops
- [medium] Test-shard fragility audit — sample 5 recent runs for shard-affinity failures — owner: pm-devops
- [low] CI: add reality-api-client drift gate (#2556) — owner: pm-devops

### Churn hotspots this window (score 2-3, low)

- [low] Churn hotspot: backend/servers/api-server/src/routes/reports/mod.rs — evaluate whether helper extraction (#2720) is complete — owner: pm-backend
- [low] Churn hotspot: backend/servers/api-server/src/services/scheduler/mod.rs — propose retention/prune job extraction — owner: pm-backend
- [low] Churn hotspot: backend/servers/api-server/src/routes/integrations/webhook.rs — post-#2718 HMAC-parity handler consolidation opportunity — owner: pm-backend

### Screen-map drift (score 3-4)

- [medium] Link UC-33.1 to a dispute screen-map — owner: pm-frontend
- [medium] Link UC-33.2 to a dispute screen-map — owner: pm-frontend
- [medium] Link UC-33.3 to a dispute screen-map — owner: pm-frontend

### code-review carryover (score 1-2, low)

- [low] ppt-web WebSocket not re-authed on token rotation — owner: pm-backend
- [low] 10 ungated console.warn/error in ppt-web websocket.ts leak diagnostics in prod — owner: pm-frontend
- [low] reality-web listingAnalytics.ts casts untrusted ?source= — owner: pm-backend
- [low] AmlDashboardPage casts raw window.prompt text into review-decision union — owner: pm-backend
- [low] PortfolioAnalytics inquiriesTrend drops days with inquiries but zero views — owner: pm-backend
- [low] mobile-native-kmp getPortfolioAnalytics() truncates portfolio at 100 listings — owner: pm-backend
- [low] Stale TODO(security) headers in faults.rs / critical_notifications.rs — owner: pm-devops
- [low] test-gap: voice_webhooks.rs (1148 lines, 6 mounted endpoints) has no tests — owner: pm-qa

Buffer: **49/36 open** · project at 47/49 — backlog refilled from a 2-open floor; still converging on the 2 remaining frontend slices plus a fresh post-merge follow-up wave from this window's 18-PR merge burst.
