# PPT Roadmap — upkeep 2026-08-10

## State of the project

- Stories: **47 done / 2 partial / 0 not-started** of 49 (13 epics). Unchanged since 2026-07-15 deep scan (last real flip was 80-3 on 2026-06-25).
- Delta vs 2026-08-07 upkeep: no story-status flips this window; **22 PRs merged** and **3 tracked issues closed** (#2703 SSRF, #2704 DoS, #2612 notification durability). The dispatcher's auto-fix + post-merge-review loops are now visibly retiring the reality-server auth security batch (#2724/#2725/#2726/#2727 all landed the code fix this window).
- Remaining coverage gaps (the last 2 partial stories, both frontend slices on shipped APIs):
  1. **84-1** — ppt-web still uploads via server proxy; direct-to-S3 endpoint has no frontend consumer (blocked-behind: #2320 upload hardening).
  2. **84-2** — signer-facing document-sign page not built (screen-map planned, API complete); prior implementer attempt failed no-PR.
- Screen coverage: 0 orphan screens · 0 validation errors · 3 missing UC links (UC-33.1/2/3 dispute sub-UCs — all 3 queued this run).
- **Dispatcher buffer starvation** (routing signal): dispatcher-side open buffer at 1/72; local buffer went 2/36 pre-run → 36/36 post-refill. 34 new items surfaced this run.

## Ranked plan

### mvp / finish-what's-started (highest score, 8)

- [high] Wire ppt-web direct-to-S3 upload via POST /api/v1/documents/upload-url — api-client binding + UploadDocument integration + regression test (84-1 partial) — owner: pm-frontend — **soft-dep: #2320 upload-flow hardening**
- [high] Build signer-facing document-sign page in ppt-web; flip screen-map ppt/document-sign buildStatus planned→shipped; verify signature-request email delivery (84-2 partial) — owner: pm-frontend

### post-merge-review follow-ups from this window (score 7-8, high priority)

- [high] Un-quarantine /disputes/kpis test + add window_start <= window_end validation (#2575 open 10+ days) — owner: pm-backend
- [high] Add failing-on-main regression tests for reality-server security batch (#2725 password-reset transport, #2726 SSO session-invalidate error swallow, #2727 agency-members unauth IDOR, #2724 db_error leak) — owner: pm-backend/pm-qa — why: code fixes shipped but no evidence of matching negative tests
- [high] Convert workflow_executor.rs unparseable-condition branch to fail-closed (tier1d signal — separate from #2708 NaN guard) — owner: pm-backend
- [high] Wire Android SsoStateStore.mint() at reality://sso deep-link entry + happy-path integration test (#2574 half-wired regression from #2568) — owner: pm-mobile
- [high] Replace pure-Rust announcement fan-out test with sqlx integration test exercising real RLS predicate (#2484 unresolved) — owner: pm-backend/pm-qa

### security cross-cutting (score 6-7)

- [medium] Extend PR #2718 HMAC parity test to booking / airbnb / esignature webhooks (only layout leg landed) — owner: pm-integration
- [medium] Alexa voice webhook accepts forged requests — verify_alexa_signature never checks HMAC (SECURITY carried) — owner: pm-security
- [medium] reality-web layout.tsx inlines tenant-config JSON into `<script>` without escaping — owner: pm-security

### post-merge follow-ups (carried, score 5-6)

- [medium] Verify PR #2712 fully covers add_evidence cross-tenant IDOR (#2483) — owner: pm-tech-lead
- [medium] Follow-up #2486: mobile LAYOUT_CACHE_KEY tenant scoping — owner: pm-mobile
- [medium] Follow-up #2241 (retry 2): OAuth state single-use not atomic in prod Redis — owner: pm-tech-lead
- [medium] Follow-up #2318 (retry 2): report_schedule due-work consumer RLS no-op — owner: pm-tech-lead
- [medium] Follow-up #2320 (retry 2): harden direct-to-S3 upload flow (IDOR/size cap/orphans) — owner: pm-tech-lead

### review + coordination (score 4-5, medium)

- [medium] Shepherd merge of accounting MVP trio (#2555, #2558, #2559) — 13 days idle — owner: pm-tech-lead
- [medium] Shepherd merge of human-authored PR #2684 (workflow-cond-parse-failopen) — owner: pm-tech-lead

### pm-data KPI gap wave (score 4-5, carried)

- [medium] Define layout publish/webhook analytics events (published_by, layout_version, target_tenant_count)
- [medium] Define dispute-lifecycle KPI set (funnel + TTR percentiles + evidence-per-dispute)
- [medium] Support-staff read audit event schema
- [medium] Publish data-retention policy for support_tooling_events / audit trail
- [medium] Formalize FaultStatusCount canonical definition
- [medium] Instrument signup/onboarding-tour completion funnel (10b-6)

_Note: announcement fan-out instrumentation was retired this run — PR #2723 landed the metric (may need scope-expansion follow-up per open question)._

### Screen-map drift (score 3-4)

- [low] Link UC-33.1 to a dispute screen-map (missing_use_cases from coverage) — owner: pm-frontend
- [low] Link UC-33.2 to a dispute screen-map (missing_use_cases from coverage) — owner: pm-frontend
- [low] Link UC-33.3 to a dispute screen-map (missing_use_cases from coverage — third leg queued this run) — owner: pm-frontend

### churn hotspots + chore (score 1-2, low)

- [low] Churn hotspot #1: reality-server/src/state.rs (1201 lines) — draft module split proposal — owner: pm-tech-lead
- [low] Churn hotspot #2: reality-server/src/routes/agencies.rs (624 lines) — split — owner: pm-tech-lead
- [low] Churn hotspot #3: reality-web layout-revalidate route.test.ts (486 lines) — extract fixtures — owner: pm-tech-lead
- [low] Triage closed-not-merged PR #2705 (dependabot rust-toolchain 1.100.0 nonexistent) — owner: pm-devops
- [low] 10 ungated console.warn/error in ppt-web websocket.ts leak diagnostics — owner: pm-tech-lead
- [low] 12 dependabot PRs open — schedule batch merge window — owner: pm-devops
- [low] Cloud routine cadence recovery — 3d gap 2026-08-07 → 2026-08-10 — owner: pm-devops
- [low] gh-issue-2556: add reality-api-client drift gate (extend #2569 pattern) — owner: pm-tech-lead

Buffer: **36/36 open** · 0 candidates ranked but unqueued · project at 47/49 — the huge burn cleared 22 PRs but produced only 3 fresh follow-up issues (all closed same-window as their source PRs), so the queue after refill is dominated by pre-existing pm-data KPI work + this window's test-coverage debt on the security batch.
