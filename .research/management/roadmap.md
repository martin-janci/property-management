# PPT Roadmap — upkeep 2026-07-31

## State of the project

- Stories: **47 done / 2 partial / 0 not-started** of 49 (13 epics). Unchanged since 2026-07-15 deep scan.
- Delta vs 2026-07-30 upkeep: no story-status flips this window. **21 PRs merged** (auto-review + code-review autofixes). **Both previous-run blockers closed inside 24-48h**: PR #2597 closes #2573 (DELETE-by-file-key data-loss), PR #2593 closes #2574 (Android SSO CSRF half-wired). The auto-fix loop is now formally completing its own regression cycles.
- Remaining gaps (the last 2 partial stories, both frontend slices on shipped APIs):
  1. **84-1** — ppt-web still uploads via server proxy; direct-to-S3 endpoint (#2309) has no frontend consumer. **Now unblocked** (was blocked on #2573; #2597 landed).
  2. **84-2** — signer-facing document-sign page not built. Screen-map planned, API complete. Prior attempts failed no-PR — **dispatcher retry pool at retry_3/2 exhausted**.
- New same-window follow-up (still open): **#2575** — `/disputes/kpis` has no window-ordering validation, only test is quarantined (from PR #2572).
- **P0 security cluster surfaced this run (Phase 1.5)**: voice_webhooks.rs — cross-tenant auth bypass in `authenticate_voice_user`, HMAC default-secret fallback + non-constant-time compare, Alexa signature never verified. Voice endpoints effectively unauthenticated in production. PR #2604 added tests around the broken code — **the tests do NOT fix the findings**.
- Screen coverage: 0 orphan screens · 0 validation errors · **3 missing UC links** (UC-33.1, UC-33.2, UC-33.3 — all queued into action-list this run). All 5 epic-7a stories still show `screen_refs: []` — a frontmatter-drift artifact, not a real UI gap (backfill task carried).
- Buffer: **11/36 open** (below half — 25 short). Dispatcher trigger `buffer-low: claimable=6/72` in this run confirms constraint. Refill via role next-actions + phase-2 backlog refill.

## Ranked plan

### mvp / finish-what's-started (highest score, 8)

- [high] Wire ppt-web direct-to-S3 upload via POST /api/v1/documents/upload-url — api-client binding + `UploadDocument` integration + regression test (84-1 partial) — owner: pm-frontend — **now unblocked** (#2573 closed by #2597)
- [high] Build signer-facing document-sign page in ppt-web against shipped signing API; flip screen-map `ppt/document-sign buildStatus: planned → shipped`; verify signature-request email delivery (84-2 partial) — owner: pm-frontend — **retry_3/2 pool exhausted, needs re-scope**

### P0 security (score 9, new this run)

- [high] SECURITY [P0]: fix 3 voice_webhooks findings — cross-tenant auth bypass in `authenticate_voice_user`, HMAC default-secret fallback + non-constant-time compare, Alexa signature never verified — voice endpoints unauthenticated in production; PR #2604 tests do NOT fix. Owner: pm-security. Un-quarantine or re-write the #2604 tests after the fix.

### post-merge follow-ups from this window (score 5-7)

- [medium] Fix #2575 — `/disputes/kpis` no window-ordering validation, only test quarantined — owner: pm-backend
- [medium] Add regression test for the fresh `schedule_cadence.rs` extraction (PR #2599, 889 lines) — reports.rs cron-schedule round-trip — owner: pm-backend

### security cross-cutting (score 7, carried)

- [high] Cross-cutting webhook hardening audit — booking / airbnb / esignature / layout — #2528 booking webhook is the last unresolved leg from the 2026-07-24 batch — owner: pm-integration
- [high] Verify layout publish webhook uses HMAC signature verification (parity with esignature webhook) — feeds #2485 fix design — owner: pm-security
- [medium] Post-#2446 (RUSTSEC-2026-0213 ammonia) cargo-audit sweep — owner: pm-security

### post-merge follow-ups (carried from prior windows, score 5-6)

- [medium] Follow-up #2483: add_evidence dispute sub-resource cross-tenant-writable — owner: pm-tech-lead (in-progress via PR #2490)
- [medium] Follow-up #2484: announce fan-out real-SQL integration test — owner: pm-tech-lead
- [medium] Follow-up #2485: layout publish webhook timestamp/replay protection — owner: pm-tech-lead
- [medium] Follow-up #2486: mobile LAYOUT_CACHE_KEY tenant scoping — owner: pm-tech-lead
- [medium] Follow-up #2366 (retry 1/2): direct-to-S3 upload drops building_id — owner: pm-tech-lead
- [medium] Follow-up #2528 booking webhook + Airbnb replay parity — owner: pm-tech-lead
- [medium] Follow-up #2530 instrument full signup funnel — owner: pm-tech-lead

### review + coordination (score 4-5, medium)

- [medium] Review + shepherd merge of accounting MVP-loop trio (#2555, #2558, #2559) — **3-day** reviewer starvation — owner: pm-tech-lead
- [medium] Draft ppt-web accounting-page skeleton + api-client bindings in parallel to accounting trio — owner: pm-frontend
- [medium] Cover the reality-web SSR escape parity gap (post PR #2600/#2603) — owner: pm-frontend

### pm-data KPI gap wave (score 4-5, carried)

- [medium] Define layout publish/webhook analytics events (published_by, layout_version, target_tenant_count)
- [medium] Define dispute-lifecycle KPI set (funnel + TTR percentiles + evidence-per-dispute)
- [medium] Instrument announcement fan-out with delivered/read/ack per targeting scope
- [medium] Support-staff read audit event schema
- [medium] Publish data-retention policy for support-data / audit trail
- [medium] Formalize FaultStatusCount canonical definition
- [medium] Instrument signup/onboarding-tour completion funnel (10b-6)

### Screen-map drift (score 3-4)

- [medium] Backfill screen-map frontmatter `epics:` field across `docs/screens/**` — coverage orphan detector manufactures false `screen_refs: []` on shipped epic-7a stories — owner: pm-frontend
- [medium] Link UC-33.1 to a dispute screen-map — owner: pm-frontend
- [medium] Link UC-33.2 to a dispute screen-map — owner: pm-frontend
- [medium] Link UC-33.3 to a dispute screen-map (new this run) — owner: pm-frontend

### churn hotspots + chore (score 1-2, low)

- [low] pm-backend: extract scheduler.rs retention/prune jobs to dedicated module (repeated churn, 2 windows)
- [low] Churn hotspot: routes/layout/{admin,tenant}.rs (post PR #2478/#2549 dust settling)
- [low] Churn hotspot: crates/integrations/src/booking/mod.rs (3185 lines, runs_seen=2)
- [low] Follow-up screen-map drift audit routine (carried)
- [low] Cloud routine cadence recovery — reduce 3-4d gaps (retry 2/2)
- [low] Coverage gap [phase4]: 82-1 SwiftUI Project Setup — verify and finish to done (retry 1/2)

Buffer: **11/36 open** · project at 47/49 — 25 slots short. ⚠ Buffer below half — Phase 2 backlog refill needed to complement this run's PM-driven additions. All the 84-1 blockers cleared this window; the 84-2 retry-pool exhaustion is the biggest single risk to closing the last two partials.
