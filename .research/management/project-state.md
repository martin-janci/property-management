# PPT Project State

_Generated: 2026-06-16 — daily PM rotation (Scrum Master + pm-devops; routine refresh). Coverage `scan_kind=upkeep`; pm_cursor idx 4 → 5 (pm-security next), coverage_cursor idx 11 → 12 (epic-8a → epic-9)._

## Executive summary

- **`dev` backend is RED (issue #1437, critical).** PR #1426 merged despite breaking compile; ALL backend CI gates are now broken on `dev` until #1435 or #1436 lands. This is the **second dev-red incident in 14 days** (cf. #1332 unblocked 2026-06-14 via #1379) and exposes a structural gap: `backend.yml` runs on PR but not on push, so a merge that conflicts with `main`/peer PRs can break compile silently after-the-fact. pm-devops is filing this as the headline blocker.
- **Test-coverage hardening flood: 4+ pm-qa PRs landed** this run, clearing high-priority follow-up gaps from the 2026-06-14 post-merge review: #1393 (Booking.com OAuth/CSRF coverage → closes #1362/#1374), #1394 (document presigned-URL minting/expiry + access-gate → closes #1377), #1395 (realtime preference-sync publish leg → closes #1376), #1417 (vote NaN fuzz guard → closes Phase 1.5 finding). All four queued action-list items now status=done via dev-reconcile.
- **Mobile delivery momentum:** #1385-#1389 env-setup/iOS native, #1391 (FilterSheet Near Me Android/shared parity), #1401 (iOS CoreLocation Near Me), #1402 (navigation-state preservation AC-4). Five gap-82 coverage items closed via dev-reconcile.
- **DevOps state-of-the-stack (pm-devops rotation):** Mobile EAS workflows (`eas-build-android.yml` / `eas-build-ios.yml`) are now present in `.github/workflows/` (cleared from 2026-05-27 backlog as draft-only). `app-tsx-merge-queue.yml` exists. Pre-push fmt/clippy gate (#1431) merged — but is **local-hook-only** and would not have caught #1426. `security-test-gate.yml` enforcement status still unconfirmed.
- **New issues this run (13):** #1403-#1413 + #1422 (post-merge-review follow-ups, all labeled `follow-up` + `from-merged-review`) plus CRITICAL #1437.
- **Stale drafts still need a call:** #1316 (verify-document-folder-organization-backend-promote), #1197 (test-oauth-authorization-server-integration ~6d), #988 (epic-scale).

## Sprint progress (`_bmad-output/implementation-artifacts/sprint-status.yaml`)

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · epics_done=1/5 (8A only) — but **8A.3 publish-leg tests now in (#1395)**, only mobile-push leg gates final 8A promotion.

| Epic | Tracked status | Real status (from coverage + activity) |
|---|---|---|
| 6 — Announcements & Communication | in-progress | 1/6 stories complete (6-6); web UI for 6-2/6-3/6-4 still in flight |
| 7A — Basic Document Management | in-progress | 0/5 stories complete; #1316 stale; presigned-URL coverage landed (#1394) |
| 8A — Basic Notification Preferences | **near-done** | 8a-1/8a-2 done; 8a-3 publish-leg tests landed (#1395), only mobile-push (FCM/APNs) leg open |
| 10A — OAuth Provider Foundation | in-progress | 0/3 stories complete; #1197 OAuth integration test draft stale ~6d; #1388 token-exchange serde tests landed |
| 10B — Platform Administration | in-progress | 5/7 stories complete (coverage 2026-05-29) |
| 82 — Mobile (Reality KMP) | in-progress | 5 gap-82 items closed this run via PRs #1391/#1401/#1402/#1386 |
| 85 — Mobile Build Pipeline | in-progress | EAS workflow files NOW in repo; green-status verification still owed |

## Shipped since last run (cursor #1384, 26 PRs)

- **#1393** — Booking.com OAuth handler / CSRF / secure-credential-replacement coverage (closes #1362, #1374) [pm-qa]
- **#1394** — Document presigned-URL minting/expiry + access-gate allow-path tests (closes #1377) [pm-qa]
- **#1395** — CI-executable coverage for realtime preference-sync publish leg (closes #1376) [pm-qa]
- **#1417** — NaN-weight fuzz guard for vote winner selection (Phase 1.5 finding) [pm-qa]
- **#1388** — Airbnb OAuth token-exchange serde unit tests [pm-backend]
- **#1397** — Forms hardening [pm-backend]
- **#1426** — Backend feature merge — **BROKE DEV COMPILE (see issue #1437)** [pm-backend]
- **#1430** — Work-orders org-gate [pm-backend]
- **#1431** — Pre-push fmt gate (local hook) [pm-devops]
- **#1432** — Stale RLS baseline reset [pm-backend]
- **#1385-#1389** — Mobile env-setup / iOS native [pm-frontend]
- **#1386** — Mobile navigation auth guard (AC-5) evidence [pm-frontend]
- **#1391** — FilterSheet Near Me location filter (Android/shared parity) [pm-frontend]
- **#1401** — iOS CoreLocation Near Me integration (story 82.3) [pm-frontend]
- **#1402** — Navigation state preservation (AC-4) proof [pm-frontend]

## What's next (top 5 actions)

1. **[high] URGENT: Land #1435 or #1436 to restore `dev` backend compile** (issue #1437) — pm-devops + pm-backend. Until this lands every backend PR's CI is red regardless of its own quality.
2. **[high] Add `cargo check --workspace --tests` smoke gate on `dev` push** (not just PR) — pm-devops + pm-backend. Would have caught #1426 → #1437 before propagation.
3. **[high] Triage remaining open follow-up issues #1403-#1413 + #1422** (post-merge-review) — pm-scrum-master. Yesterday's pm-qa rotation cleared 4 of #1360-#1377 via merged PRs; new batch needs owner assignment.
4. **[medium] Confirm EAS mobile workflows green on workflow_dispatch** — pm-devops. Both files now exist; pins/secrets verification still owed.
5. **[medium] Decide stale draft PRs #1316 (~3d), #1197 (~7d), #988 (epic-scale)** — pm-scrum-master. Promote, rebase, or close.

## Blockers

- **#1437 — `dev` backend compile broken (CRITICAL).** Owner: pm-devops + pm-backend. Lands as #1435 or #1436.
- **EAS mobile pipeline unverified.** Owner: pm-devops. Workflow files present, green-status not confirmed.
- **`security-test-gate.yml` enforcement.** Owner: pm-devops + pm-qa. Still possibly advisory-only on `dev`.
- **Stale drafts #1316/#1197/#988.** Owner: pm-scrum-master. No movement >2d.

## Role focus today: **pm-devops** (+ pm-scrum-master always-on)

- **pm-devops** (rotation idx 4, last 2026-05-27, 20d stale): 6 new next_actions appended to `action-list.json`; 4 new risks appended to `risks.json`; 3 new decisions in `decisions.md`. Full role JSON in `.research/management/roles/pm-devops.md`. Headline: dev-CI discipline failure (#1437) + EAS workflow files now present but unverified + pre-push gate local-only.
- **pm-scrum-master** (always-on): produced the delivery synthesis above; headline = `dev` red blocks all backend CI; pm-qa coverage flood mostly cleared 18 follow-up issues from 2026-06-14; mobile location-filter slice complete.

## Coverage (deep scan — 2026-06-23)

- **`coverage.json` fully regenerated** (`scan_kind=deep`) — supersedes the `scan_kind=upkeep` note in the header above and the rotating per-epic upkeep cursor (was "next: epic-9"; all epics are now freshly classified). All 13 epics with story files rescanned in parallel → **49 stories: 37 done · 12 partial · 0 not-started**. Full ranked plan in `roadmap.md`.
- **Dominant signal — promotion lag, not missing code:** ~8 stories (6-1…6-5, 79-1, 10b-5, 10b-7) are code-complete with merged PRs but stuck at sprint-status `ready-for-dev`/`review`; they need reconciliation + sign-off, not new implementation.
- **Genuinely unfinished slices:** 84-5 pgvector RAG retrieval/query service (migration only), 80-3 mediation party-submission endpoints (unwired), 80-2 dispute redesign 5-step wizard + i18n, 6-3/6-4 mobile comments/pinned UI, 79-1/79-2 e2e (79-2 security-sensitive: SSO/JWT/cookie).
- **Systemic screen-map drift:** 0 of ~120 screen-maps populate frontmatter `epics:` → epic→screen linkage impossible; this manufactures the 29 "orphan" screens (really out-of-scan-scope: 13 of 25 epics). Backfilling `epics:` is the single highest-leverage fix. Also: 2 orphan epics (epic-85 env/build, epic-8a NotificationSettingsPage), 4 missing UC links (UC-10/29/33/40).
- **Top coverage actions** — *secondary to the #1437 dev-red incident in "What's next" above*: (1) reconcile 79-2 auth-flow → done with pm-security SSO/JWT/cookie sign-off; (2) reconcile 10b-5 support-data-access → done with retention/access-audit check; (3) finish & promote announcements 6-1/6-2/6-5; (4) complete 79-1 e2e verification; (5) backfill screen-map `epics:` frontmatter.
