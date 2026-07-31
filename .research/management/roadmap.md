# PPT Roadmap — upkeep 2026-07-31

## State of the project

- Stories: **47 done / 2 partial / 0 not-started** of 49 (13 epics). Unchanged since 2026-07-15 deep scan.
- Delta vs 2026-07-30 upkeep: no story-status flips this window. 19 non-dispatcher PRs merged (heavily weighted to security hardening + follow-through refactors): #2596/#2600/#2603 same-window XSS/cast fixes (pattern signal), #2597 (DELETE-by-file-key ref-check guard closes #2573 — **unblocks 84-1**), #2599 (schedule cadence extraction), #2610 (scheduler retention/prune extraction — closes 2 action-list items), #2607 (reality-api-client drift gate — closes #2556). All 4 issues opened this window already closed. Auto-review loop clearly converging.
- Remaining gaps (the last 2 partial stories, both frontend slices on shipped APIs):
  1. **84-1** — ppt-web still uploads via server proxy; direct-to-S3 endpoint (#2309) has no frontend consumer. **Unblocked this run** — dependency #2573 (reap-race regression) was closed by PR #2597. Ready to claim.
  2. **84-2** — signer-facing document-sign page not built (screen-map planned, API complete); 3 prior implementer attempts failed no-PR.
- Screen coverage: 0 orphan screens · 0 validation errors · 3 missing UC links (UC-33.x dispute sub-UCs — all 3 now queued as of this run).
- Buffer: **23/36 open** (refilled from 3/36 open pre-run — 20 new items added, 2 marked done by PR #2610).

## Ranked plan

### mvp / finish-what's-started (highest score, 8)

- [high] **Claim 84-1** direct-to-S3 wire — dependency (#2573) cleared by PR #2597 — pm-frontend consumer wiring (api-client binding + UploadDocument integration + regression test) — owner: pm-frontend — **NOW UNBLOCKED**
- [high] Build signer-facing document-sign page in ppt-web against shipped signing API; flip screen-map ppt/document-sign buildStatus planned→shipped (84-2 partial; 3 prior no-PR attempts — needs scoped brief before retry4) — owner: pm-frontend

### window-driven cross-cutting (score 8, high priority)

- [high] Cross-cutting frontend security pattern: lint/codemod for untrusted-to-union casts + SSR string interpolation into `<script>` — #2596 (AML decision) + #2600 (tenant JSON) + #2603 (viewsource) all same window — owner: pm-frontend

### post-merge follow-ups from this window (score 5-7)

- [medium] Frontend verify-gate hygiene — pnpm test failure should be first-class dev-push signal (post-#2602 silent-failure window) — owner: pm-devops
- [medium] Reality-web SSR injection pattern audit — sweep remaining string-interpolation-into-HTML + URL-search-params casts (post-#2600 / #2603) — owner: pm-security
- [medium] SECURITY: sweep other public GET routes for sqlx/serde error text bleed — pattern from #2606 (admin.rs null body) + #2609 (resolved.rs raw error) — owner: pm-security
- [medium] Add integration test for KMP Android SSO initiation → callback happy path — #2593 wired mint(), but the half-wire regression pattern needs a test that would have caught the original — owner: pm-qa

### security cross-cutting (score 7, carried)

- [high] Cross-cutting webhook hardening audit — booking / airbnb / esignature / layout — #2528 booking webhook is the last unresolved leg — owner: pm-integration
- [medium] Alexa voice webhook accepts forged requests — verify_alexa_signature never checks the signature (SECURITY) — owner: pm-security

### post-merge follow-ups (carried from prior windows, score 5-6)

- [medium] Follow-up #2483: add_evidence dispute sub-resource cross-tenant-writable — owner: pm-tech-lead (in-progress via PR #2490)
- [medium] Follow-up #2484: announcement fan-out real-SQL integration test — owner: pm-tech-lead
- [medium] Follow-up #2485: layout publish webhook timestamp/replay protection — owner: pm-tech-lead
- [medium] Follow-up #2486: mobile LAYOUT_CACHE_KEY tenant scoping — owner: pm-tech-lead
- [medium] Follow-up #2366 (retry 1/2): direct-to-S3 upload drops building_id — owner: pm-tech-lead
- [medium] Follow-up #2528: booking webhook / Airbnb replay parity — owner: pm-tech-lead
- [medium] Follow-up #2530: full signup funnel instrumentation — owner: pm-tech-lead
- [medium] Follow-up #2575: /disputes/kpis window-ordering + un-quarantine test — owner: pm-tech-lead

### review + coordination (score 4-5, medium)

- [medium] Review + shepherd merge of accounting MVP-loop trio (#2555, #2558, #2559) — 3-day reviewer starvation — owner: pm-tech-lead
- [medium] Frontend audit of accounting MVP-loop trio consumer wiring so review can proceed in one pass — owner: pm-frontend
- [medium] Repeated-churn auth.rs (runs_seen=4, 2950 lines) — plan module split — owner: pm-tech-lead

### pm-data KPI gap wave (score 4-5, carried, all in-progress)

- [medium] Layout publish/webhook analytics events (published_by, layout_version, target_tenant_count)
- [medium] Dispute-lifecycle KPI set (funnel + TTR percentiles + evidence-per-dispute)
- [medium] Announcement fan-out delivered/read/ack per targeting scope
- [medium] Support-staff read audit event schema
- [medium] Data-retention policy for support-data / audit trail
- [medium] FaultStatusCount canonical definition
- [low] Mobile-native (Reality KMP) analytics parity audit
- [low] OAuth token issuance/refresh/revocation event tracking

### Screen-map drift (score 3-4)

- [medium] Backfill screen-map frontmatter `epics:` field so epic→screen linkage stops manufacturing orphans in coverage scans — owner: pm-frontend (in-progress)
- [medium] Link UC-33.1 to a dispute screen-map — owner: pm-frontend
- [medium] Link UC-33.2 to a dispute screen-map — owner: pm-frontend
- [medium] Link UC-33.3 to a dispute screen-map — owner: pm-frontend (**new this run**)
- [low] Screen-map drift: PR #2497 reality-web layout-revalidate route w/o docs/screens/reality/ — owner: pm-qa

### coverage gap-scan (score 3-5, new this run)

- [medium] Build ppt-web permission-authoring UI for documents (AccessScopeSelector / RoleSelector / UserSelector) — 7a-3 gap — owner: pm-frontend
- [low] Mobile UI for critical notifications (8a-2 gap) — owner: pm-frontend
- [low] FCM/APNs OS integration for notification preference push on mobile (8a-3 gap) — owner: pm-frontend
- [low] Sync docs/screens/reality/inquiries.md + account.md buildStatus from in-progress → shipped (82-5 gap) — owner: pm-frontend
- [low] Reconcile sprint-status.yaml 10a-1/10a-2/10a-3 ready-for-dev → done — owner: pm-tech-lead

### churn hotspots + chore (score 1-3, low)

- [low] Churn hotspot: routes/reports/schedule_cadence.rs — post-#2599 further cron helper extraction — owner: pm-backend
- [low] Churn hotspot: routes/voice_webhooks.rs — post-#2604 signature helper sharing with esignature/booking — owner: pm-backend
- [low] Churn hotspot: admin-web MobileConfigPage.tsx — extract Save-mutation hooks into shared platform-settings pattern — owner: pm-frontend
- [low] Churn hotspot: crates/integrations/src/booking/mod.rs (post-PR-#2176 tail; split in-progress via draft #2611)
- [low] Follow-up: 10 ungated console.warn/error in ppt-web websocket.ts (in-progress)
- [low] Follow-up: WebSocket not re-authed on token rotation — owner: pm-backend
- [low] Wire drift-gate outcome into research-routine signals (follows #2607) — owner: pm-devops
- [low] Sweep ppt-web imports that could benefit from shared reality-api-client (follows #2607/#2556) — owner: pm-frontend
- [low] Cloud routine cadence recovery — reduce 3–4d gaps between runs (retry 2/2)

Buffer: **23/36 open** · project at 47/49 — buffer refilled from 3/36 pre-run via 20 gap-scan adds. Dispatcher backlog is well-fed (72 rows across all sources per trigger note); the routine-management buffer is trailing but adequate.
