# PPT Roadmap — upkeep 2026-08-09

## State of the project

- Stories: **47 done / 2 partial / 0 not-started** of 49 (13 epics). Unchanged since 2026-07-15 deep scan.
- Delta vs 2026-07-30 upkeep: no story-status flips. 19 PRs merged this 44h window (dispatcher/auto-fix dominant): two P0-follow-up issues closed (`#2703` SSRF via PR #2710, `#2704` memory-DoS via PR #2707, `#2612` scheduled-notification via PR #2714), one long-open security gap closed (community reads via PR #2722), the announcement fan-out real-SQL test-fidelity risk **partially satisfied** by PR #2723's dedicated 295-LOC suite, and two admin-web no-op Save paths unblocked (`#2716` platform-settings, `#2717` mobile-config). Five churn refactors on auth/layout/reports (2711/2713/2715/2720/2721) without new tests — legitimate (behaviour preserved) but pattern to watch.
- Remaining gaps (unchanged):
  1. **84-1** — ppt-web direct-to-S3 upload still server-proxied. Still blocked-on `#2573` reference-check fix.
  2. **84-2** — signer-facing document-sign page not built (screen-map planned, API complete).
- Screen coverage: 0 orphan screens · 0 validation errors · 3 missing UC links (UC-33.x).
- QA note (2026-08-09): file-count "hotfix-no-test" heuristic is producing false positives — all three fix PRs this window ship inline `#[cfg(test)]` regression tests. Real gap is #2547 (carried) and layout webhook replay-guard (#2485 still open despite PR #2718 pinning body-binding parity).

## Ranked plan

### mvp / finish-what's-started (highest score, 8)

- [high] Wire ppt-web direct-to-S3 upload via POST /api/v1/documents/upload-url — 84-1 partial → done — owner: pm-frontend — **blocked-on: #2573**
- [high] Build signer-facing document-sign page in ppt-web against shipped signing API; flip screen-map ppt/document-sign buildStatus planned→shipped (84-2 partial) — owner: pm-frontend

### post-merge follow-ups (score 7-8, high)

- [high] Fix #2573 — DELETE /documents/by-file-key same-org reference-check gap (blocks 84-1) — owner: pm-backend
- [high] Fix #2574 — Android SSO SsoStateStore.mint() call-site — owner: pm-mobile
- [medium] Fix #2575 — /disputes/kpis window-ordering validation + un-quarantine test — owner: pm-backend
- [medium] PR #2547 scheduler retention prune regression test (carried) — owner: pm-backend/pm-qa

### security cross-cutting (score 7)

- [medium] Layout webhook nonce+timestamp replay-guard (#2485) — PR #2718 only pinned body-binding parity — owner: pm-security
- [high] Cross-cutting webhook hardening audit — booking / airbnb / esignature / layout — #2528 remains — owner: pm-integration
- [medium] Alexa voice webhook signature verification stub — owner: pm-security

### post-merge follow-ups (carried from prior windows, score 5-6)

- [medium] Follow-up #2483: add_evidence dispute IDOR write-guard (audit landed via #2712; write-guard still tracked) — owner: pm-tech-lead
- [medium] Follow-up #2486: mobile LAYOUT_CACHE_KEY tenant scoping — owner: pm-tech-lead
- [medium] Follow-up #2366 (retry 2/2): direct-to-S3 upload drops building_id — owner: pm-tech-lead
- [medium] Follow-up #2241 (retry 2): OAuth state single-use not atomic in prod Redis — owner: pm-tech-lead
- [medium] Follow-up #2318 (retry 2): report-schedule due-work consumer RLS no-op — owner: pm-tech-lead
- [medium] Follow-up #2320 (retry 2): direct-to-S3 upload hardening (IDOR, size cap, orphans) — owner: pm-tech-lead

### QA + heuristic hygiene (new, score 5)

- [medium] Widen routine hotfix-no-test heuristic to count inline #[cfg(test)] / #[tokio::test] / #[sqlx::test] — 3 false-positives this window — owner: pm-tech-lead
- [low] Spot-check #2723 announcement_fanout_metrics_tests.rs RLS predicate coverage; close risk-announcement-fanout-test-fidelity — owner: pm-qa

### review + coordination (score 4-5)

- [medium] Review + shepherd merge of accounting MVP-loop trio (#2555/#2558/#2559) — 2+ day reviewer starvation (carried) — owner: pm-tech-lead
- [medium] repeated-churn: auth.rs (runs_seen=5, 2950 lines) — #2715 deduped boilerplate but module-split still open — owner: pm-tech-lead

### pm-data KPI gap wave (score 4-5, carried)

- [medium] Define layout publish/webhook analytics events
- [medium] Define dispute-lifecycle KPI set (funnel + TTR + evidence-per-dispute)
- [medium] Support-staff read audit event schema
- [medium] Publish data-retention policy for support-data / audit trail
- [medium] Formalize FaultStatusCount canonical definition
- [medium] Instrument signup/onboarding-tour completion funnel (10b-6)

### Screen-map drift (score 3-4)

- [medium] Link UC-33.1 to a dispute screen-map — owner: pm-frontend
- [medium] Link UC-33.2 to a dispute screen-map — owner: pm-frontend

### churn hotspots + chore (score 1-2, low)

- [low] Churn hotspot: workflow_executor.rs (457 lines this window) — evaluate_conditions fail-open on unparseable — owner: pm-backend
- [low] Churn hotspot: services/scheduler.rs (347 lines) — retention/prune extraction candidate — owner: pm-backend
- [low] Churn hotspots: layout/tenant.rs (2nd run) + layout/admin.rs (extract further after #2711/#2713 dedupe) — owner: pm-backend
- [low] Follow-up screen-map drift: PR #2497 reality-web/app/api/layout-revalidate/route.ts — owner: pm-qa
- [low] WebSocket not re-authed on token rotation — owner: pm-backend
- [low] SECURITY: reality-web layout.tsx inlines tenant-config JSON w/o escaping — owner: pm-security
- [low] AmlDashboardPage casts raw window.prompt text into review-decision union — owner: pm-backend
- [low] PortfolioAnalytics: caps realtor portfolio at 100 listings — owner: pm-backend
- [low] Triage closed-not-merged PRs #2385/#2387/#2489/#2705 (dependabot supersedes) — owner: pm-devops

Buffer: **6 open + 7 done this window** in action-list — dispatcher is landing PRs faster than gaps open (7 resolved, 4 new items). Project holds at 47/49; the 2 remaining partials both depend on the same unblock (#2573).
