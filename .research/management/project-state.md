# PPT Project State

_Generated: 2026-07-31 — routine Phase 1.6 lightweight upkeep (Scrum Master synthesis + pm-frontend rotation slot). Coverage `scan_kind=upkeep`; pm_cursor idx 2 → 3 (pm-frontend → pm-qa next), coverage_cursor idx 4 → 5 (epic-7a re-checked, all 5 stories still done, evidence refreshed with PR #2597 on 7a-1)._

## Executive summary

- **Delivery still at 47/49 stories done, 2 partial** (the 84-1 direct-to-S3 upload wiring and 84-2 sign page). Both same-window regressions from the 2026-07-30 run are **closed inside 24-48h**: PR #2597 guards DELETE-by-file-key (closes #2573 data-loss), PR #2593 wires Android SSO mint() at deep-link entry (closes #2574 CSRF blocker). The auto-fix loop is now formally closing its own regression cycles.
- **21 PRs merged this window** (2026-07-30 → 2026-07-31). Mix of same-window follow-up fixes (#2597, #2593, #2602, #2594) + code-review autofixes (#2578, #2592, #2595, #2596, #2600, #2603, #2605, #2606, #2608, #2609) + de-dup / refactor (#2579, #2580, #2599, #2601) + drift-gate CI (#2607) + screen-map reconciliation (#2577). PR #2599 extracted a fresh 889-line `schedule_cadence.rs` from reports.rs — a churn-hotspot fix in flight.
- **84-1 is now unblocked** — the DELETE-by-file-key reference-check gap that blocked client wiring is closed. The frontend team can now safely ship the direct-to-S3 client without risk of orphaning shared file-keys.
- **P0 SECURITY CLUSTER surfaced this run (Phase 1.5)**: three high-severity findings in `voice_webhooks.rs` — (a) cross-tenant auth bypass in `authenticate_voice_user`, (b) HMAC default-secret fallback + non-constant-time compare, (c) Alexa signature never verified (`_signature` bound-and-ignored). Voice endpoints are effectively unauthenticated in production. **PR #2604 added unit tests around the broken code — the tests do NOT fix the findings**. Now the top backlog priority; owner: pm-security.
- **Open PRs (3): accounting MVP-loop trio** (#2555 invoice lifecycle, #2558 invoice PDF, #2559 PAY-by-square QR) — draft-ready since 2026-07-28, **now 3 days without reviewer engagement**. Dispatcher trigger `buffer-low: claimable=6/72` symptom of reviewer-capacity constraint, not implementer capacity.
- **Only one same-window follow-up still open**: #2575 dispute-KPI window validation.

## Sprint progress (`_bmad-output/implementation-artifacts/sprint-status.yaml`)

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · **epics_done = 3/5** unchanged this run. Extended-scope epics (10B, 80, 81, 82, 83, 84, 85, 79, 8A, 9) folded into `coverage.json` and largely done.

| Epic | Sprint status | Coverage status (13 epics) |
|---|---|---|
| 6 — Announcements & Communication | in-progress | 6/6 stories done in coverage |
| 7A — Basic Document Management | in-progress | **5/5 stories done — evidence refreshed this run** (7a-1 gains PR #2597 note) |
| 8A — Basic Notification Preferences | done | 3/3 stories done |
| 10A — OAuth Provider Foundation | done | 3/3 stories done |
| 10B — Platform Administration | in-progress | 7/7 stories done |
| 80 — Dispute Resolution | partial | 3/3 stories done in coverage; sprint-status still says partial (pending reconciliation) |
| 84 — Documents / e-signature | (extended) | 3/5 done, 2 partial (84-1 direct-S3 wiring — now unblocked, 84-2 sign page — retry pool exhausted) |
| 82 / 83 / 85 / 79 / 81 / 8a / 9 | (extended) | all done in coverage |

## Shipped since last run (21 PRs > #2576)

- **#2609** stop leaking raw sqlx/serde error text on public GET /layout/resolved
- **#2608** surface DB errors on scheduler notification-target lookups (was silent)
- **#2607** add reality-api-client drift gate (closes #2556)
- **#2606** return 500 on failed layout serialize instead of null body
- **#2605** remove stale TODO(security) gate comments (faults / critical_notifications)
- **#2604** unit tests for voice_webhooks.rs (⚠ does NOT fix the 3 P0 findings)
- **#2603** validate untrusted ?source= against ViewSource set (reality-web)
- **#2602** fix pre-existing frontend test failures blocking the verify gate (closes #2598)
- **#2601** fix no-op Save paths in admin-web platform-settings + mobile-config
- **#2600** escape inlined tenant-config JSON in reality-web layout.tsx (XSS)
- **#2599** extract schedule cadence/cron helpers from reports.rs (new 889-line module)
- **#2597** guard DELETE /documents/by-file-key against reaping still-referenced object (**closes #2573 data-loss blocker**)
- **#2596** validate AML review decision before submit (ppt-web)
- **#2595** portfolio-analytics keep days with inquiries but zero views (mobile-native)
- **#2594** DB-backed regression test for support_tooling_events retention prune (closes hotfix-no-test)
- **#2593** wire Android SSO initiation leg (mint CSRF nonce) (**closes #2574 CSRF blocker**)
- **#2592** gate WebSocket console diagnostics behind import.meta.env.DEV
- **#2580** de-duplicate platform_admin authz batch2 tests
- **#2579** de-duplicate org-property authz backfill tests
- **#2578** re-auth WebSocket on token rotation instead of leaving stale socket
- **#2577** reconcile listing-detail screen-map with layout-revalidate hardening

## What's next (top 5 actions from ranked backlog)

1. **[high, P0] Fix voice_webhooks security cluster** — 3 findings from Phase 1.5, all high severity; endpoints unauthenticated in production. PR #2604 added tests but did NOT fix. Owner: **pm-security**.
2. **[high] Wire 84-1 direct-to-S3 in ppt-web** — POST /documents/upload-url consumer; blocker #2573 cleared by #2597. Owner: **pm-frontend**.
3. **[high] Build 84-2 signer-facing document-sign page** — retry_3/2 pool exhausted; needs specialist re-scope. Owner: **pm-frontend / pm-tech-lead**.
4. **[medium] Fix #2575** — `/disputes/kpis` window-ordering validation + un-quarantine the KPIs test. Owner: **pm-backend**.
5. **[medium] Break accounting MVP-loop trio reviewer starvation** (#2555 / #2558 / #2559 — 3 days) — assign reviewer slot or split into smaller PRs. Owner: **pm-tech-lead**.

## Blockers

- **Voice-webhook security cluster (Phase 1.5 findings)** — 3 P0 findings, no PR opened yet; endpoints effectively unauthenticated in production. Owner: **pm-security / pm-backend**.
- **Accounting MVP-loop trio (#2555 / #2558 / #2559)** — 3-day reviewer starvation; blocks the accounting story pipeline entirely. Owner: **pm-tech-lead**.
- **84-2 signer-page retry_3/2 pool exhausted** — needs specialist re-scope before another dispatcher attempt. Owner: **pm-frontend / pm-tech-lead**.

## Role focus today: **pm-frontend** (rotation idx 2; last 2026-06-10, 51d stale) + pm-scrum-master always-on

- **pm-scrum-master** (always-on): produced the delivery synthesis above. Headline = the auto-review loop shipped 21 PRs in ≤2 days and closed BOTH of its previous-run regressions inside 24-48h — the loop is genuinely converging. New P0 security cluster on voice_webhooks is the top new backlog item. Reviewer capacity remains the binding constraint (accounting trio 3-day stall).
- **pm-frontend** (rotation, first run in 51 days): 84-1 unblocked, 84-2 retry-exhausted, 3 UC-33.x links + screen-map frontmatter epics backfill = single high-leverage docs PR. Flagged the accounting MVP-loop trio's implicit frontend dependency (need ppt-web accounting-page skeleton drafted in parallel to avoid sequential integration wait). Confirmed no frontend surface is affected by the P0 voice-webhook cluster, but any future voice-UI story should not be scoped until closed.

## Coverage (upkeep this run — 2026-07-31)

- **`coverage.json` refreshed via mechanical upkeep** — `scan_kind=upkeep`, `generated` bumped, no re-scan.
- **Epic re-check: epic-7a** — cursor idx 4. All 5 stories still `done`; evidence entry added to 7a-1 for PR #2597 (DELETE-by-file-key reference-check guard, closes #2573 same-org data-loss regression). `last_checked = 2026-07-31` stamped on all 5 stories.
- **`coverage_cursor` advances 4 → 5** (epic-7a → next epic).
- **`pm_cursor` advances 2 → 3** (pm-frontend → pm-qa next run). role_last_run["pm-frontend"] = 2026-07-31.
- **Composition unchanged: 47 done · 2 partial · 0 not-started** across 13 epics. Same 3 missing UC links (UC-33.1/2/3 — all 3 now queued into action-list this run). Zero orphan screens, zero validation errors.
- **Buffer health: ⚠ 11/36 open (below half)** — Phase 2 backlog refill needed to complement PM-driven additions; 6 new open items added this run (voice-webhook P0, UC-33.3 link, reality-web SSR escape audit, schedule_cadence regression test, accounting-web skeleton).
