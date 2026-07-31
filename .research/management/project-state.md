# PPT Project State

_Generated: 2026-07-31 — routine Phase 1.6 lightweight upkeep (Scrum Master synthesis + pm-frontend rotation slot). Coverage `scan_kind=upkeep`; pm_cursor idx 2 → 3 (pm-frontend → pm-qa next), coverage_cursor idx 4 → 5 (epic-7a re-checked, all 5 stories still done, evidence refreshed with PR #2597 on 7a-1; advances to epic-80)._

## Executive summary

- **Delivery still at 47/49 stories done, 2 partial** (84-1 direct-to-S3 wiring, 84-2 signer document-sign page). The 2026-07-30 → 2026-07-31 window shipped **19 non-dispatcher PRs** heavily weighted toward security hardening and follow-through refactor extractions — auto-review loop is genuinely converging.
- **Security cluster this window:** three untrusted-cast / XSS fixes landed the same day across ppt-web + reality-web (#2596 AML decision cast, #2600 tenant JSON XSS, #2603 viewsource cast) plus #2593 (Android SSO CSRF mint call site — closes #2574 half-wired) and #2597 (DELETE-by-file-key reference guard — closes #2573 reap-race). All 4 opened issues this window are already closed.
- **Refactor extractions land:** #2599 (schedule cadence/cron helpers out of reports.rs) and #2610 (scheduler retention/prune jobs to submodule) — the two top backend churn hotspots from previous windows both got structurally addressed. schedule_cadence.rs and voice_webhooks.rs are the churn front now (post-extraction and post-#2604 test-add respectively).
- **Frontend hygiene fixes:** #2602 repaired pre-existing frontend test failures that had blocked verify-gate for at least one window silently — indicates the verify-gate feedback loop needs escalation to first-class CI signal. Also #2601 (admin-web platform-settings + mobile-config Save-path no-ops fixed) and #2592 (WS console diagnostics gated).
- **Infra:** #2607 shipped the reality-api-client drift gate (closes #2556); #2606/#2608/#2609 tightened error-surface behavior on layout/scheduler/resolved public endpoints.
- **Open PR picture unchanged:** 15 open — 11 dependabot + accounting MVP-loop trio (#2555/#2558/#2559) still zero reviewer engagement (3rd day drafted) + 1 draft #2611 booking/mod.rs split. **Accounting trio reviewer starvation is now the top delivery-throughput blocker.**
- **84-1 unblocked** — dependency #2573 was closed by PR #2597 this window. The gap-84-1 retry chain can now be claimed without regression risk.

## Sprint progress (`_bmad-output/implementation-artifacts/sprint-status.yaml`)

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · **epics_done = 3/5** unchanged this run. Extended-scope epics (10B, 80, 81, 82, 83, 84, 85, 79, 8A, 9) folded into `coverage.json` and largely done.

| Epic | Sprint status | Coverage status (13 epics) |
|---|---|---|
| 6 — Announcements & Communication | in-progress | 6/6 stories done in coverage |
| 7A — Basic Document Management | in-progress | 5/5 stories done in coverage (7a-1 evidence refreshed via #2597) |
| 8A — Basic Notification Preferences | done | 3/3 stories done |
| 10A — OAuth Provider Foundation | done | 3/3 stories done |
| 10B — Platform Administration | in-progress | 7/7 stories done |
| 80 — Dispute Resolution | partial | 3/3 stories done in coverage; sprint-status still says partial (pending reconciliation — queued this run) |
| 84 — Documents / e-signature | (extended) | 3/5 done, 2 partial (84-1, 84-2); 84-1 unblocked this run via #2597 |
| 82 / 83 / 85 / 79 / 81 / 8a / 9 | (extended) | all done in coverage; **epic-7a re-checked this run (all 5 stories done, 7a-1 evidence refreshed with PR #2597)** |

## Shipped since last run (19 non-dispatcher PRs > #2579)

- **#2610** — refactor(api-server): extract scheduler retention/prune jobs to submodule
- **#2609** — code-review api-core: stop leaking raw sqlx/serde error text on public GET /layout/resolved
- **#2608** — code-review api-core: surface DB errors on scheduler notification-target lookups
- **#2607** — chore(api-validation): add reality-api-client drift gate (closes #2556)
- **#2606** — code-review api-core: return 500 on failed layout serialize instead of null body
- **#2605** — docs(api-server): remove stale TODO(security) headers in faults/critical_notifications
- **#2604** — test(api-server): add voice webhook signature/verification unit tests
- **#2603** — code-review reality-web: validate untrusted ?source= against ViewSource set
- **#2602** — fix(admin-web): repair pre-existing frontend test failures blocking verify gate
- **#2601** — dx: fix admin-web platform-settings + mobile-config no-op Save paths
- **#2600** — code-review reality-web: escape inlined tenant-config JSON
- **#2599** — refactor: extract schedule cadence/cron helpers from reports.rs
- **#2597** — fix(api-server): guard DELETE /documents/by-file-key against reaping a referenced object (closes #2573)
- **#2596** — code-review ppt-web: validate AML review decision before submit
- **#2595** — test(mobile-native): pin portfolio-analytics zero-view-day behaviour (no bug)
- **#2594** — test(api-server): DB-backed regression for support_tooling_events retention prune
- **#2593** — gh-issue-2574: mobile-native Android SSO initiation leg (mint CSRF nonce)
- **#2592** — fix(ppt-web): gate WebSocket console diagnostics behind import.meta.env.DEV
- **#2580** — refactor: de-duplicate platform_admin authz batch2 tests

## What's next (top 5 actions from ranked backlog)

1. **[high] Claim 84-1** direct-to-S3 wire in the next dispatcher round — dependency (#2573) cleared by PR #2597 — **owner: pm-frontend**. Adds api-client binding + UploadDocument integration + regression test; flips 84-1 partial → done.
2. **[high] Cross-cutting frontend security pattern lint/codemod** for untrusted-to-union casts and SSR string interpolation into `<script>` — #2596 + #2600 + #2603 same-window signal — **owner: pm-frontend** (mechanism decision from pm-tech-lead).
3. **[medium] Shepherd accounting MVP-loop trio** (#2555 invoice lifecycle, #2558 invoice PDF, #2559 PAY-by-square QR) — 3-day reviewer starvation — **owner: pm-tech-lead**.
4. **[medium] Frontend verify-gate hygiene** — post-#2602 silent failure window; make pnpm test failure first-class on dev push — **owner: pm-devops**.
5. **[medium] Package scoped implementer brief for 84-2** signer document-sign page — 3 prior no-PR attempts; retry4 needs a green-test anchor — **owner: pm-frontend**.

## Blockers

- **Accounting MVP-loop trio (#2555 / #2558 / #2559)** — 3rd day drafted with zero reviewer engagement; dispatcher throughput bottlenecked on reviewer capacity, not implementer capacity. Reviewer-slot policy decision from 2026-07-30 still pending. Owner: pm-tech-lead.
- **84-2 signer page retry3** — 3 no-PR implementer attempts on record; retry4 needs a scoped brief before spawn. Owner: pm-frontend.

## Role focus today: **pm-frontend** (rotation idx 2; last 2026-06-10, 52d stale) + pm-scrum-master always-on

- **pm-scrum-master** (always-on): synthesized the delivery picture. Headline = the auto-review loop shipped 19 PRs in ~24h and every issue opened this window is already closed. 84-1 is unblocked. Reviewer capacity (accounting trio) is now tighter than implementer capacity.
- **pm-frontend** (rotation): flagged the 3-in-one-day XSS/cast pattern (#2596/#2600/#2603) as a systemic gap worth a lint/codemod — that goes in the action list as high-priority. Also called out the frontend verify-gate silent-failure window (post-#2602) as a hygiene gap, and confirmed 84-1 is now ready to claim. Recommends scoping a proper implementer brief for 84-2 before spawning retry4.

## Coverage (upkeep this run — 2026-07-31)

- **`coverage.json` refreshed via mechanical upkeep** — `scan_kind=upkeep`, `generated` bumped, no re-scan.
- **Epic re-check: epic-7a** — cursor idx 4. All 5 stories still `done`; evidence entry added to 7a-1 for PR #2597 (DELETE-by-file-key reference-check guard, closes #2573 regression). `last_checked = 2026-07-31` stamped on all 5 stories.
- **Merged-PR evidence added:** 7a-1 (PR #2597 lifecycle-guard hardening), 84-1 (PR #2597 unblocks the reap-race dependency).
- **`coverage_cursor` advances 4 → 5** (epic-7a → epic-80 next run).
- **`pm_cursor` advances 2 → 3** (pm-frontend → pm-qa next run). role_last_run["pm-frontend"] = 2026-07-31.
- **Composition unchanged: 47 done · 2 partial · 0 not-started** across 13 epics. Missing UC-33.3 link queued (UC-33.1 and UC-33.2 already queued in prior runs). Zero orphan screens, zero validation errors.
