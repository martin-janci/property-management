# PPT Project State

_Generated: 2026-06-26 — daily PM rotation (Scrum Master + pm-security). Coverage `scan_kind=upkeep`; pm_cursor idx 5 → 6 (pm-security → pm-data next), coverage_cursor idx 12 → 0 (epic-9 → epic-10a next). **Routine lag: 10 days** (last run 2026-06-16 → today 2026-06-26)._

## Executive summary

- **10-day cursor lag** — 96 PRs merged into `dev` (#1567–#1853) since the last routine run, including PR #1798 (3672-file `emergency.rs` route split), PR #1713 (revert of #1690 delegation frontend per board decision BIT-213), and feature work on saved-search alert transport (#1849), saved-search cadence scheduler (#1847), and messaging group participants (#1848/#1853). Sprint-status.yaml may be stale relative to the verified story states.
- **Sprint progress** — "Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth". Epic 8A and Epic 10B are done. Epic 6 is at 5/6 stories done after 2026-06-25 verifications (6-1 + 6-3 confirmed; sprint-status.yaml still shows 3/6, needs reconciliation). Epic 7A holds at 1/5; story 7a-2 is in review with red CI (document_folder_tests FK/isolation failure). Epic 10A remains 0/3 — blocked by open security gate issues #481 + #487.
- **pm-security re-assessment (first run in 30 days)** — Phase 1.5 surfaced two production panic paths in reality-server that are not yet tracked: `.expect('OS rng failed')` at `backend/servers/reality-server/src/handlers/users/mod.rs:551` (password-reset path) and `.expect('Failed to create HTTP client')` at `src/state.rs:759` (startup). pm-security also notes issue #481 (revoked refresh tokens reusable) **may already be fixed in code** — `revoked_at IS NULL` guards present in `oauth.rs:414` and `session.rs:55` — but sprint-status still marks the gate open, unnecessarily blocking 10a-1/10a-3.
- **Issue queue** — 75 of the 83 new issues are auto-filed `follow-up`/`from-merged-review` items (post-merge auto-review). 3 are `needs-human-review`; 1 each `bug`, `dispatcher`, `ci`, `epic-18`, `research-routine`. None untriaged (no `untriaged-issue` signals fire).
- **Open code review backlog (reality-server)** — 3 new findings emitted this run; merge with prior `code-review-api-core-osrng-expect` (open) and `code-review-mobile-native-kmp-deeplink-token-not-url-decoded` (5 open) — Rust panic-path remediation is the cross-segment theme.

## Sprint progress (`_bmad-output/implementation-artifacts/sprint-status.yaml`)

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · epics_done=2/6

| Epic | Tracked status | Real status (from coverage + activity) |
|---|---|---|
| 6 — Announcements & Communication | in-progress | 5/6 stories done — 6-1 + 6-3 verified 2026-06-25; sprint-status shows 3/6 (stale) |
| 7A — Basic Document Management | in-progress | 1/5 stories done; 7a-2 stuck in review (red CI) |
| 8A — Basic Notification Preferences | done | 3/3 done |
| 10A — OAuth Provider Foundation | in-progress | 0/3 started; gate-blocked by #481, #487 |
| 10B — Platform Administration | done | 7/7 done |
| 80 — Dispute Resolution | partial | 1/3 done; 80-3 mediation party submissions in flight (#1846 draft) |

## Shipped since last run (cursor #1439, 96 merged PRs)

- **#1849** — BIT-139 email/push transport drainer for saved-search alerts (reality-server background worker)
- **#1847** — BIT-140 saved-search alert_frequency cadence (reality-server scheduler)
- **#1848** + **#1853** — BIT-206/244 messaging group participants (backend + ppt-web)
- **#1846** — BIT-80-3 mediation/dispute party submissions endpoints (Epic 80 advance, in draft)
- **#1833** — BIT-84-5 RAG view contract fix + RAG similarity-search tests (reality-server)
- **#1830/#1831/#1832/#1834/#1835** — Announcements UI iterations (Epic 6 stories 6-1..6-5)
- **#1822** — Epic ACC: accounting standalone server (:8082) foundation
- **#1798** — refactor: split api-server `emergency.rs` route into modules (3672 file changes; refactor only)
- **#1809** — accounting PAP-321 F1/F2: 404-not-500 + skip-serializing provider secrets
- **#1713** — REVERT of #1690 delegation frontend re-add per board decision BIT-213
- **#1567** + **#1568** — dependabot dependency bumps

## What's next (top 5)

1. **[high] Fix 7a-2 CI failure** (document_folder_tests FK/isolation) and re-green so folder-organization moves from review to done — pm-backend
2. **[high] Close or formally defer security issues #481 + #487** to unblock Epic 10A (10a-1, 10a-3) story pickup — pm-backend / pm-security
3. **[high] Resolve issue #480** (JWT in WebSocket query param logged) — pm-backend
4. **[high] File and fix reality-server panic paths** (`handlers/users/mod.rs:551` rng expect, `state.rs:759` http-client expect) before Epic ACC ships — rust-backend (pm-security)
5. **[medium] Reconcile sprint-status.yaml epic-6 (5/6 done) and epic-10b (7/7 done)** to match 2026-06-25 verifications — orchestrator

## Blockers

- **7a-2-folder-organization** — CI red (document_folder_tests FK/isolation); story stuck in review (owner: pm-backend)
- **10a-1 + 10a-3** — open security gates #481, #487 (owner: pm-backend / pm-security)
- **10a-2** — open security gate #482 (ProtectedRoute multi-tenant role fallback) (owner: pm-frontend)
- **sprint-status.yaml freshness** — story counts do not reflect 2026-06-25 verifications (owner: orchestrator)
- **Cloud routine cadence** — 10-day lag indicates the cloud cron may have been paused or failing; verify with operator

## Role focus today

- **pm-scrum-master** (always-on synthesis)
- **pm-security** (rotation slot — first run in 30 days)

## Per-role one-liners

- **pm-scrum-master:** Sprint at ~55% by story count across active epics. 7A is the laggard; 10A gate-blocked. Re-run scan locally to refresh coverage given 96-PR window.
- **pm-security:** Two new untracked panic paths in reality-server. #481 may already be fixed in code — verify and close. WS JWT logging (#480) needs structured-log test, not just human-only suppression comment.
