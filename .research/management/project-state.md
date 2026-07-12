# PPT Project State

_Generated: 2026-07-12 — daily PM rotation (Scrum Master + pm-security; routine upkeep). Coverage `scan_kind=upkeep`; pm_cursor idx 5 → 6 (pm-security → pm-data next), coverage_cursor idx 12 → 0 (epic-9 → epic-10a wrap)._

## Executive summary (2026-07-12)

- **Cursor gap closed:** last committed routine run was 2026-07-09 (LAG=85h). Between then and today, 41 PRs merged to `dev` (#2186–#2261); this run reconciles the backlog against that flood.
- **Security lens (pm-security rotation).** Two live security follow-ups landed this run: (a) reality-server SSO exchange no longer trusts client-supplied `request.roles` (#2254, closes #2249, HIGH sev — was a portal privilege-escalation surface); (b) OAuth CSRF single-use path got handler-level coverage (#2219, closes #2203). Two more security follow-up **issues** are freshly filed on merged code and waiting for pickup — #2263 (per-portal webhook receiver may fail open on empty secret after #2259 env wiring; needs `.filter(|s| !s.is_empty())` guard) and #2241 (OAuth state single-use test models atomic consume, but production Redis is non-atomic GET+DEL — TOCTOU race under concurrent replay).
- **Fresh code-review findings (Phase 1.5, backlog).** Six new `code-review-finding` signals — most severe: api-server login handler returns `ACCOUNT_SUSPENDED` **before** password check (account-enumeration oracle for suspended users, `auth.rs:699`, high-conf), and reality-server `validate_fetch_url` accepts IPv4-mapped IPv6 literals (`::ffff:169.254.169.254` → cloud metadata SSRF, `url_validator.rs:145`, high-conf). Both promote to plans this run.
- **Churn hotspots:** `api-server/src/routes/auth.rs` (repeated-churn — 3rd consecutive run in the top-N; auth surface is churning as security work lands), `crates/integrations/src/booking/mod.rs` (first-time hotspot from OTA coverage series #2230/#2231), `crates/db/src/repositories/llm_document.rs` (RAG/pgvector work #2226/#2256).
- **Delivery cadence healthy.** 41 PRs merged in 3.5 days across api-server, reality-server, ppt-web, mobile-native (KMP), plus 6 dependabot bumps. No reverts, no closed-with-changes-needed on hot code paths. Two open PRs (#2260 shard-rebalance, #2262 sitemap consistency-check) are follow-ups to #2223/#2225 and both look green.

## Sprint progress — coverage.json (upkeep pass)

Latest coverage scan is still 2026-07-07 `deep`; this run advanced only the rotating epic (epic-9 — TOTP MFA setup, still `done`). Full `scan` mode is deferred to the on-demand local `/ppt-project-management scan`.

| Epic | Status snapshot |
|---|---|
| epic-6 — Announcements | 5/6 done (unchanged since 2026-07-07) |
| epic-7a — Document Management | 3/5 partial (2 stories in flight) |
| epic-8a — Notification Preferences | 3/3 done |
| epic-9 — TOTP MFA | 1/1 done (re-checked 2026-07-12) |
| epic-10a — OAuth Provider | 3/3 done |
| epic-10b — Platform Admin | 7/7 done |
| epic-79 — Reality Portal (public) | 5/5 done |
| epic-80 — Reality Portal (portal owner) | 4/4 done |
| epic-81 — Realtor Management | 3/3 done |
| epic-82 — Mobile (Reality KMP) | 5/5 done |
| epic-83 — Rental Platform Integrations | 3/3 done (Airbnb + Booking OAuth handlers now covered) |
| epic-84 — Admin/Compliance | 3/3 done |
| epic-85 — Mobile Build Pipeline | 3/3 done |

## Actions and risks

- Action list: 48 items (31 in-progress via dispatcher, 17 open). See `action-list.json`.
- Risks: 42 items (37 open, 5 resolved). See `risks.json`.
- **New today (routine):** two backlog rows promoted to plans this run (see the daily brief).

## Role focus today

- **pm-security** — see security lens above; 2 issues, 0 blockers on-hand, watching #2263 / #2241 to ensure they land before webhook rollout.
- Rotation next run: **pm-data**.

## Historical note

- 2026-06-16 project-state snapshot (`dev` was RED at #1437) is preserved below for continuity — the current tree is green after those hotfixes landed via the July merge batch.

---

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

(top of roadmap.md, deep scan 2026-07-07 evening)

1. **[high] Reconcile sprint-status ready-for-dev -> done for 10a-1/10a-2/10a-3 — gates #481 (closed 2026-05-26), #482, #487 all verified closed; code+screens+tests shipped** — pm-backend
2. **[high] No application handler calling search_similar_documents or embedding-write flow (84-5-pgvector-rag pgvector RAG Migration)** — pm-backend
3. **[high] RAG retrieval/query service (embedding generation + similarity search) not implemented in routes/repositories (84-5-pgvector-rag pgvector RAG Migration)** — pm-backend
4. **[medium] Frontend UI not built (OrganizationsPage.tsx not found; buildStatus=planned not shipped) (10b-1-organization-management-dashboard Organization Management Dashboard)** — pm-frontend
5. **[medium] Create new schedule endpoint still stubbed out (81-1-report-schedule-editing Report Schedule Editing)** — pm-backend

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
