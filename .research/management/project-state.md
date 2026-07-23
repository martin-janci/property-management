# PPT Project State

_Generated: 2026-07-23 — daily PM rotation (Scrum Master + pm-data; routine Phase 1.6 lightweight run). Coverage `scan_kind=upkeep`; pm_cursor idx 6 → 7 (pm-data done today; pm-integration next), coverage_cursor idx 0 → 1 (epic-10a re-checked, all 3 stories still done — advances to epic-10b)._

## Executive summary

- **Delivery is converging: 47/49 stories done, 2 partial** (both frontend slices on shipped backend APIs — 84-1 direct-to-S3 upload wiring, 84-2 document-sign signer page). The 2026-07-13→07-15 promotion wave (7 stories moved to done) has held; the 2026-07-21→07-23 wave is almost entirely **security hardening + test infrastructure + follow-up fixes**, not new feature work.
- **Recent security hardening wave (headline this run):** PR #2450 org-scoped 5 dispute IDOR handlers, PR #2438 fixed get_document org-scoping (closes #2422), PR #2447 (BIT-268 authz backfill), PR #2453 (import+cert-doc authz), PR #2465 (OAuth handler test backfill), PR #2446 bumped ammonia for RUSTSEC-2026-0213. Four post-merge follow-up issues opened (#2483/#2484/#2485/#2486) with #2490 already open for #2483.
- **Test infrastructure consolidation (major wave):** PR #2459 (nextest archive + per-test partition), PR #2461 (consolidate 206 test binaries → 8), PR #2487 (consolidate db+reality tests), PR #2488 (fix CI disk space). This dramatically reduces CI cost and unlocks per-test partitioning in future runs.
- **`just verify` gate landed (#2444)** plus agent hardening (#2448) — a deterministic impact-scoped pre-push gate now exists at the root; adoption on open PRs still needs a check-in (queued as `qa-verify-gate-adoption-checkin-2026-07-23`).
- **Layout & Content Manager pilot (PRs #2424–#2432) shipped end-to-end** but the follow-up review surfaced replay-protection and mobile cache-key gaps (#2485/#2486) — both queued. Zero KPI instrumentation on the shipped path — pm-data has queued layout-publish event definitions.
- **No dev-red incidents this run.** CI green. Open PRs (7): #2490 (dispute add_evidence IDOR fix — closes #2483), #2491 (npm-minor-patch), #2482 (docs repo-map tidy), #2481 (dashboard screen-map tidy), #2440 (docs-forms lease-abstraction test batch), #2478 (layout review-hardening sweep), #2433 (mobile-native iOS resolved layout — 2 days stale).
- **pm-data (rotating role, 56 days stale):** Delivered features are almost entirely uninstrumented. Epic 6/10A/10B/80/84 all lack KPI hooks. Seven pm-data next_actions added covering dispute funnel, layout publish events, announcement fan-out metrics, OAuth token telemetry, onboarding funnel, retention policy, and mobile-native analytics parity.

## Sprint progress (`_bmad-output/implementation-artifacts/sprint-status.yaml`)

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · **epics_done = 3/5** (8A, 10A both fully done — epic-10a re-verified this run; epic-6 has 5/6 stories done and epic-7a has 4/5). Extended-scope epics (10B, 80, 81, 82, 83, 84, 85, 79, 8A, 9) are folded into `coverage.json` and largely done.

| Epic | Sprint status | Coverage status (13 epics) |
|---|---|---|
| 6 — Announcements & Communication | in-progress | 6/6 stories done in coverage |
| 7A — Basic Document Management | in-progress | 5/5 stories done in coverage |
| 8A — Basic Notification Preferences | done | 3/3 stories done |
| 10A — OAuth Provider Foundation | done | 3/3 stories done (re-verified 2026-07-23) |
| 10B — Platform Administration | in-progress | 7/7 stories done |
| 80 — Dispute Resolution | partial | 3/3 stories done in coverage; sprint-status still says partial (pending reconciliation) |
| 84 — Documents / e-signature | (extended) | 3/5 done, 2 partial (84-1 direct-S3 wiring, 84-2 sign page) |
| 82 / 83 / 85 / 79 / 81 / 8a / 9 | (extended) | all done in coverage |

## Shipped since last run (top 10 of 40 merged PRs)

- **#2450** — Org-scope enforced on 5 dispute IDOR handlers (post-merge follow-up remains on add_evidence, #2483/PR #2490)
- **#2438** — get_document org-scoping fix (closes #2422)
- **#2447** — BIT-268 document authz backfill wave
- **#2465** — OAuth handler test backfill
- **#2459** — nextest archive + per-test partition (CI restructuring)
- **#2461** — Consolidate 206 test binaries → 8 (large CI cost win)
- **#2444** — `just verify` gate + layered CI
- **#2446** — Ammonia bump for RUSTSEC-2026-0213
- **#2436** — Fix scheduler malformed target_ids parse
- **#2478** — Layout review-hardening sweep (still open, near-merge)

Also merged: 10 dependabot PRs (#2468–#2477), 5 code-review fix PRs (#2455–#2458 addressing announce fan-out mock, dashboard mock/dirty check, PropInput strings), plus scope + doc cleanup (#2439/#2442/#2443/#2452/#2454/#2462/#2463/#2464/#2467/#2479/#2480/#2488).

## What's next (top 5 actions from ranked backlog)

1. **[high] Wire ppt-web direct-to-S3 upload** via POST /api/v1/documents/upload-url (84-1 partial; backend #2309 shipped) — **owner: pm-frontend**
2. **[high] Build signer-facing document-sign page in ppt-web** against shipped signing API; screen-map ppt/document-sign planned→shipped (84-2 partial; prior attempt failed) — **owner: pm-frontend**
3. **[high] Cross-cutting webhook hardening audit** (booking / airbnb / esignature / layout) — #2485 shows layout lacks replay guard, unknown parity elsewhere — **owner: pm-integration**
4. **[high] Verify layout publish webhook uses HMAC signature verification** (parity with esignature webhook) — feeds #2485 fix design — **owner: pm-security**
5. **[medium] Follow-up fixes for #2483 (add_evidence IDOR — PR #2490 open), #2484 (announce fan-out real-SQL test), #2485 (layout webhook replay), #2486 (mobile layout cache tenant-scope)** — **owners: pm-tech-lead + pm-security + pm-qa**

## Blockers

- **#2433 — mobile-native iOS resolved-layout PR stale 2 days.** Owner: pm-frontend (needs rebase / ping).
- **No product-facing KPIs on shipped MVP epics.** Owner: pm-data (7 tasks queued).
- **Retention policy for `support_tooling_events` unresolved** (open since 2026-05-28). Owner: pm-data + pm-security.

## Role focus today: **pm-data** (+ pm-scrum-master always-on)

- **pm-data** (rotation idx 6, last 2026-05-28, 56d stale): 7 new next_actions appended (dispute add_evidence audit event, layout publish events, dispute-lifecycle KPI set, announcement fan-out metrics, retention policy, support-staff read audit schema, mobile-native analytics parity). 3 new risks added (KPI blindspots on shipped MVP, retention-policy gap, metric-definition drift). See `.research/management/roles/pm-data.md`.
- **pm-scrum-master** (always-on): produced the delivery synthesis above; headline = security-hardening wave dominated the window (30 real code PRs, 10 dependabot); no dev-red; test-binary consolidation is a major CI cost win; 4 new follow-up issues from post-merge review all queued.

## Coverage (upkeep this run — 2026-07-23)

- **`coverage.json` refreshed via mechanical upkeep** (no deep re-scan). Added evidence entries to 8 stories from merged PRs (10a-1/10a-2/10a-3 for #2465, 7a-1/7a-3 for #2447/#2453/#2438, 80-1/80-2/80-3 for #2450); last_checked bumped to 2026-07-23 on those 8 plus all 3 epic-10a stories (cursor idx 0).
- **`coverage_cursor` advances 0 → 1** (epic-10a → epic-10b next run).
- **`pm_cursor` advances 6 → 7** (pm-data → pm-integration next run).
- **Composition unchanged: 47 done · 2 partial · 0 not-started** across 13 epics. Same 3 missing UC links (UC-33.x dispute sub-UCs). Zero orphan screens, zero validation errors.
- **Buffer refill:** action-list refilled from 8 open items → 36 open items (target hit). Refill mix: 2 mvp finish-what's-started (high), 4 security follow-ups (high/medium), 7 pm-data KPI tasks (medium/low), 3 QA/DevOps (low/medium), 5 chore/refactor (low), plus dependabot & triage. Score ceiling this refill = 8 (mvp partial finish-what's-started); this is the natural cap when only 2 partials remain.
