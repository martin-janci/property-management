# PPT Project State

_Generated: 2026-07-29 — routine Phase 1.6 default rotating run (Scrum Master synthesis + pm-backend deep-dive; pm-backend last ran 2026-06-06 — 53 days stale). Coverage `scan_kind=upkeep`; pm_cursor idx 1 → 2 (pm-backend → pm-frontend next), coverage_cursor idx 3 → 4 (epic-79 re-checked cheaply, all 4 stories still done; advances to epic-7a next)._

## Executive summary

- **Delivery holds at 47/49 stories done, 2 partial** (84-1 direct-to-S3 upload wiring, 84-2 signer-facing document-sign page). One net follow-up resolved this window: **#2549 closed #2532** (layout publish/webhook/revalidate event emission wired end-to-end).
- **Shipped this run (7 merged PRs):** #2504 signature-request mount fix (BIT-313), #2478 layout review-hardening sweep (authz, publish TOCTOU, webhook replay, defensive rendering), #2549 layout event emission + sink (closes #2532), #2553 AuthContext cold-boot stale-role fix, #2433 mobile-native iOS listing detail through shared resolved layout (stale PR unblocked), #2491 npm-minor-patch bump. #2554 was research/dispatcher self-refill.
- **Backend delivery cluster forming on accounting-server:** three new PRs opened 2026-07-28 by the same author against the accounting slice — #2555 (invoice sent/cancelled lifecycle, UC-ACC-05.17), #2558 (invoice PDF render, UC-ACC-05.9), #2559 (PAY-by-square QR endpoint, UC-ACC-05.8). All three CI-clean, mergeable, awaiting first review. Phase-A "close the MVP loop" work per the accounting portal analysis. **#2555 introduces the `InvoiceStatus::Sent`/`Cancelled` enum variants #2558/#2559 depend on — sequence #2555 first.**
- **Post-merge review pipeline still healthy:** #2532 closed by #2549 this window; #2528 (booking webhook hardening) still open; new post-merge issues #2557 (backend dev-team follow-ups) + #2556 (CI reality-api-client drift gate) opened.
- **No dev-red incidents in the window.** CI green.

## Sprint progress (`_bmad-output/implementation-artifacts/sprint-status.yaml`)

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · **epics_done = 3/5** unchanged this run.

| Epic | Sprint status | Coverage status (13 epics) |
|---|---|---|
| 6 — Announcements & Communication | in-progress | 6/6 stories done in coverage |
| 7A — Basic Document Management | in-progress | 5/5 stories done in coverage |
| 8A — Basic Notification Preferences | done | 3/3 stories done |
| 10A — OAuth Provider Foundation | done | 3/3 stories done |
| 10B — Platform Administration | in-progress | 7/7 stories done |
| 80 — Dispute Resolution | partial | 3/3 stories done in coverage (sprint-status stale) |
| 79 — ppt-web integration | (extended, re-checked this run) | 4/4 done; #2553 refreshed AuthContext evidence |
| 82 — Reality iOS SwiftUI | (extended) | 5/5 done; #2433 refreshed listing-detail evidence |
| 84 — Documents / e-signature | (extended) | 3/5 done, 2 partial (84-1, 84-2) |
| 83 / 85 / 81 / 8a / 9 | (extended) | all done in coverage |

## Shipped since last run (7 merged PRs since 2026-07-28T03:20Z)

- **#2549** — gh-issue-2532: layout publish/webhook/revalidate event emission + sink (closes #2532)
- **#2478** — fix(layout): review-hardening sweep — authz, publish TOCTOU, webhook replay, defensive rendering
- **#2504** — fix(api-server): signature-request list/create unreachable — mount as document sub-resource (BIT-313)
- **#2553** — code-review-ppt-web-core-authctx-init-stale-role: route AuthContext cold-boot init through refreshTokenInternal
- **#2433** — feat(mobile-native): iOS listing detail renders through the shared resolved layout (stale PR unblocked)
- **#2491** — chore(deps): npm-minor-patch group across 1 directory with 5 updates
- **#2554** — chore(research): refill starved dispatcher stack (routine self)

## What's next (top 5 actions — anchored to the ranked backlog & open PRs)

1. **[high] Review + land PR #2555 (accounting invoice sent/cancelled lifecycle, UC-ACC-05.17)** — prerequisite for #2558/#2559 (both depend on the `InvoiceStatus` enum variants #2555 introduces) — **owner: pm-backend**
2. **[high] Then review PR #2558 (invoice PDF render) + PR #2559 (PAY-by-square QR)** — same author cluster; rebase after #2555 merges — **owner: pm-backend**
3. **[high] Wire ppt-web direct-to-S3 upload via POST /api/v1/documents/upload-url (84-1 partial; #2309 shipped)** — **owner: pm-frontend**
4. **[high] Build signer-facing document-sign page in ppt-web (84-2 partial; #2306 screen-map planned)** — **owner: pm-frontend**
5. **[medium] Add api-server regression test for #2547 scheduler retention prune (hotfix-no-test)** + integration test for layout webhook TOCTOU/replay hardening from #2478 — **owner: pm-backend / pm-qa**

## Blockers

- **PR #2482** — refactor churn-hotspot repo-map.md now 6 days stale (since 2026-07-23). Owner: pm-tech-lead (rebase or close).
- **#2547 landed without an api-server test** (hotfix-no-test signal, still open on action-list as `bug-hotfix-no-test-pr-2547`). Owner: pm-qa / pm-backend.
- **`docs/screens/mobile/` still empty** — mobile screen-map coverage remains at 0 files; drift detection deliberately excludes mobile until seeded. Owner: pm-frontend / pm-mobile.

## Role focus today: **pm-backend** (rotation idx 1; last 2026-06-06, 53 days stale) + pm-scrum-master always-on

- **pm-scrum-master (always-on):** headline this run = post-merge follow-up loop resolved #2532 (via #2549) and the layout hardening sweep (#2478) landed cleanly; new accounting-server slice cluster (three PRs same day, same author) needs a scheduled review pass; backend rotation role is on stale-refresh so backend gets the deep-dive.
- **pm-backend deep-dive:** three-PR accounting cluster is the near-term delivery focus, needs merge-order enforced (#2555 first); layout hardening + event-emission wave landed but is not test-locked; auth.rs (2950 LOC, runs_seen=4) and reports.rs (3329 LOC, runs_seen=3) growing past the safe-review threshold. New supply-chain vector: PR #2559 pulls in `crc32fast` + `lzma-rs`. See `roles/pm-backend.md` for full JSON.

## Coverage (upkeep this run — 2026-07-29)

- **`coverage.json` refreshed via mechanical upkeep** — added merged-PR evidence to 79-2 (#2553), 82-4 (#2433), 84-2 (#2504); refreshed `last_checked=2026-07-29` on all epic-79 stories (rotating epic re-check).
- **`coverage_cursor` advances 3 → 4** (epic-79 → epic-7a next run).
- **`pm_cursor` advances 1 → 2** (pm-backend → pm-frontend next run).
- **Composition unchanged: 47 done · 2 partial · 0 not-started** across 13 epics. Same 3 missing UC links (UC-33.x dispute sub-UCs). Zero orphan screens, zero validation errors.
