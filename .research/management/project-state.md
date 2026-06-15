# PPT Project State

_Generated: 2026-06-15 — daily PM rotation (Scrum Master + pm-qa; routine refresh). Coverage `scan_kind=upkeep`; pm_cursor idx 3 → 4 (pm-devops next), coverage_cursor idx 10 → 11 (epic-85 → epic-8a)._

## Executive summary

- **Dev CI unblocked.** PR #1379 (martin-janci, merged 2026-06-14T22:19Z) ended a 3-day red streak on the dev-wide backend `test` job by fixing 3 production regressions: `status::text AS status` casts across 9 form repo queries, COALESCE on NOT NULL booleans in registry, and a cross-tenant defense-in-depth guard on `documents/core.rs` download/preview. The forms RLS test got a `GRANT SELECT ON users` to its RLS role. **Issue #1332 should be closeable once green is observed on a fresh main build.**
- **Per-env mobile icons shipped (#1383, gap-85-2).** `app.config.ts` (+93) plus a new `app.config.icon.test.ts` (+124) and assets README give Property Management mobile DEV/STG visual differentiation. Coverage row `85-2-build-configuration` refreshed; remaining gaps: iOS xcconfig/schemes, multi-size icon generation, build scripts.
- **Follow-up issue flood: 18 new issues #1360-#1377 from yesterday's post-merge review,** all labeled `follow-up` + `from-merged-review`. Concentrated themes: missing test coverage (RLS write/download, IDOR cross-org, OAuth/CSRF, realtime sync publish leg, document presigned URL minting), atomicity bugs (record_payment, record_reserve_transaction), front-end races (dispute draft autosave, iOS SearchView pagination), and test-discipline tooling (canonical seed_membership helper, tenant-aware RLS request helper, pre-push fmt/clippy gate). Plus dispatcher meta-issue #1380 (stale gap-scan buffer + Tier-2 escalation endpoint misconfigured).
- **Stale drafts to decide:** #1316 (verify-document-folder-organization-backend-promote, 1.8d) and #1197 (test-oauth-authorization-server-integration, 5.9d) — promote, rebase, or close. **Phase 1.5 code-review:** `vote.rs:1765` partial_cmp().unwrap() panic on NaN weighted votes (medium, reachable from `/votes/{id}/results`) — fuzz test already tracked.

## Sprint progress (`_bmad-output/implementation-artifacts/sprint-status.yaml`)

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · epics_done=1/5 (8A only)

| Epic | Tracked status | Real status (from coverage + activity) |
|---|---|---|
| 6 — Announcements & Communication | in-progress | 1/6 stories complete (6-6); 6-1 in review |
| 7A — Basic Document Management | in-progress | 0/5 stories complete; #1316 draft for 7a-2 verify, stale |
| 8A — Basic Notification Preferences | **done** | 3/3 stories complete; backend realtime sync confirmed PR #472 |
| 10A — OAuth Provider Foundation | in-progress | 0/3 stories complete; #1197 OAuth test integration draft stale |
| 10B — Platform Administration | in-progress | 5/7 stories complete (coverage 2026-05-29) |
| 85 — Mobile Build Pipeline | in-progress | gap-85-2 DEV/STG icons shipped #1383 — partial → partial (icons gap closed) |

## Shipped since last run (cursor #1359)

- **#1379** — `fix: unblock dev-wide backend test job (#1332) — 3 production regressions` (form.rs status::text casts, registry COALESCE, documents/core.rs cross-tenant guard, forms RLS GRANT SELECT)
- **#1383** — `feat: per-environment app icon variants (DEV/STG badges) (mobile)` (gap-85-2)
- **#1382, #1384** — research/dispatcher state PRs (`.research/management/` only)

## What's next (top 5 actions)

1. **[high] Triage 18 follow-up issues #1360-#1377** — pm-scrum-master; assign owner or close as won't-fix. Backlog grows faster than burn-down without this.
2. **[high] Close issue #1332 if dev CI now green** after PR #1379 unblock — pm-scrum-master + pm-devops.
3. **[high] Add regression test for record_payment non-atomic check-then-insert (#1361)** — pm-qa + pm-backend; concurrent double-pay vector.
4. **[high] Add NaN-weight fuzz test for /votes/{id}/results** (Phase 1.5 finding) — pm-qa + pm-backend.
5. **[medium] Decide on stale draft PRs #1316 (1.8d) and #1197 (5.9d)** — promote, rebase or close. pm-scrum-master.

## Blockers

- **#1380 dispatcher meta-issue** — stale gap-scan buffer feeds no-op claims; Tier-2 escalation endpoint misconfigured. Owner: pm-devops (or dispatcher owner).
- **18 follow-up issues #1360-#1377 lack owner** — opened by post-merge bot; no human triage yet. Owner: pm-scrum-master.

## Role focus today: **pm-qa**

- pm-qa (rotation idx 3, last 2026-05-25, ~21d stale): 5 new next_actions appended to `action-list.json`; 5 new risks appended to `risks.json`; 3 new decisions in `decisions.md`. Full role JSON in `.research/management/roles/pm-qa.md`.
- pm-scrum-master (always-on): produced the delivery synthesis above; headline = dev CI unblocked + follow-up issue flood + mobile icons shipped.

## Coverage upkeep

- **epic-85 (rotation idx 10) refreshed** in `coverage.json`:
  - `85-1-environment-variables`: added `app.config.ts present via PR #1383` to evidence; removed `app.config.ts for Expo not present` from gaps. Status stays `partial`.
  - `85-2-build-configuration`: added per-env DEV/STG icon evidence from PR #1383; removed `app icon variants with DEV/STG badges not created` from gaps; confidence upgraded `medium → high`. Status stays `partial` (iOS xcconfig, multi-size icons, build scripts still open).
- Next epic to refresh: **epic-8a** (coverage_cursor idx 11).
