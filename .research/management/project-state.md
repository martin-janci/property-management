# PPT Project State

_Generated: 2026-09-26 — routine Phase 1.6 lightweight upkeep (pm-security rotation slot; 67-day stale slot refreshed) + pm-scrum-master always-on. Coverage `scan_kind=upkeep`; pm_cursor idx 5 → 6 (pm-security → pm-data next), coverage_cursor idx 7 → 8 (epic-82 re-checked, no material change; advances to epic-83). Sprint window 2026-09-08..09-26 shipped **0 PRs** — 18 days without a merge to `dev`._

## Executive summary

- **Delivery unchanged at 47/49 stories done, 2 partial** (84-1 direct-to-S3 upload wiring, 84-2 sign page). No status flips this window. **Zero PRs merged in 18 days** — last merge was #2956 on 2026-09-08.
- **Sprint story-complete, tracking-hygiene gap.** Every story in the active sprint's epics (6, 7a, 8a, 10a, 10b, 80) reads `done` in `_bmad-output/implementation-artifacts/sprint-status.yaml`'s `development_status` block, but the `epics:` summary block at the top is stale (epic-6 says `3/6`, epic-7a `2/5`, epic-10b/80 status fields don't match their per-story counts). Reconciliation queued as `pm-scrum-master-reconcile-stale-epic-counts`.
- **Auto-review + dispatcher loop still ticking but no throughput.** Since 2026-09-08 all dispatcher activity has been Tier-1d generator kicks and buffer-refill telemetry; every open backlog item is either mobile-native/KMP (7/8 items — structurally unclaimable in the cloud runner due to the AGP/Gradle egress gate) or waiting on a cloud-build unblock. `GC3-buffer-bounds=FAIL (record-only)` is now chronic.
- **Two high-severity security fixes stuck in retry loop.** #2944 (cross-tenant IDOR in violation records) and #2945 (portfolio_properties read+write missing org scope) have failed twice on the same cloud-build blocker (cited as #2652). Their retry-1 drafts (#2977, #2976) remain open with `mergeable_state: unstable`. **Critical mismatch:** #2652's actual body describes a mobile-native/KMP `dl.google.com` egress 403, not the swagger-ui/api-server egress that supposedly blocks these retries — the wrong root-cause issue is being tracked. Both #2944 and #2945 are marked `closed` on GitHub despite the fixes being unmerged.
- **New auth-bypass this run:** #2978 — ppt-web AI-chat + OCR feature hooks call the API via raw `fetch()` with no `Authorization` header. Draft fix #2979 exists (`mergeable_state: dirty`). Fails-closed today (401 behind `<ProtectedRoute>`) but same anti-pattern as the already-fixed #486. Add lint/test guard alongside the merge.
- **59-day-old accounting PRs still stalled:** #2555, #2559 (feat(acc)) remain unreviewed since 2026-07-30 with no owner assigned. Merge, close, or rescope decision needed from pm-tech-lead.

## Sprint progress (`_bmad-output/implementation-artifacts/sprint-status.yaml`)

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · **story-level `done` across all sprint epics; summary block stale**. Extended-scope epics folded into `coverage.json`.

| Epic | Sprint status (stale) | Actual story-level | Coverage status (13 epics) |
|---|---|---|---|
| 6 — Announcements & Communication | in-progress (3/6) | done (6/6) | 6/6 stories done in coverage |
| 7A — Basic Document Management | in-progress (2/5) | done (5/5) | 5/5 stories done in coverage |
| 8A — Basic Notification Preferences | done | done | 3/3 stories done |
| 10A — OAuth Provider Foundation | done | done | 3/3 stories done |
| 10B — Platform Administration | in-progress | done | 7/7 stories done |
| 80 — Dispute Resolution | partial | done | 3/3 stories done in coverage |
| 82 — (extended) | (extended) | done | 2/2 stories done in coverage; **re-checked this run (idx 7), no material change, last_checked=2026-09-26** |
| 84 — Documents / e-signature | (extended) | 3/5 done, 2 partial | 3/5 done, 2 partial (84-1, 84-2) — unchanged |
| 81 / 83 / 85 / 79 / 7a / 8a / 9 | (extended) | all done | all done in coverage |

## Shipped since last run (0 PRs merged in 18d window since 2026-09-08)

- _nothing_ — every commit on `dev` since #2956 merged (2026-09-08) has been `.research/` chore commits from the daily routine + the dispatcher planning loop. Zero application-code merges.

## What's next (top actions from the ranked action list)

1. **[high]** Move PR #2979 (gh-issue-2978 AI-chat auth-header fix) out of draft into review and merge. Owner: pm-frontend.
2. **[high]** Unblock cloud-runner builds for api-server (swagger-ui egress 403 per issue #2652) so #2944/#2945 retries can complete. Owner: pm-devops.
3. **[high]** Reconcile #2652's actual body (mobile-native/KMP `dl.google.com` egress) against the claimed api-server/swagger-ui blocker so the correct root cause is tracked. Owner: pm-devops.
4. **[high]** Land the cross-tenant IDOR fixes #2944 / #2945 once the cloud build is green. Owner: pm-security.
5. **[medium]** Reconcile stale epic-level completion counts (epic-6, 7a, 10b, 80) in `sprint-status.yaml` against `development_status` entries. Owner: pm-scrum-master.

## Blockers

- **gh-issue-2944 / gh-issue-2945 (cross-tenant IDOR fixes)** — backend api-server unbuildable in cloud runner; two failed retries; wrong root-cause issue (#2652) tracked. Owner: pm-devops.
- **PR #2979 (gh-issue-2978 fix)** — still in draft, `mergeable_state: dirty`. Owner: pm-frontend.
- **PR #2555 / #2559** — 59 days old with no review movement, no owner assigned. Owner: pm-tech-lead.

## Role focus today

- **pm-scrum-master** (always-on synthesis) — summary above.
- **pm-security** (rotating role; last-run 2026-07-21 → 2026-09-26) — see `roles/pm-security.md`.

## Coverage upkeep

- **Merged PRs mapped to stories:** 0 (no merges this window).
- **Epic re-checked (rotating slot `coverage_cursor.next_index=7`):** epic-82 — no material change; `last_checked=2026-09-26`.
- **Cursor advance:** `coverage_cursor.next_index 7 → 8` (epic-83 up next).
- **`scan_kind`:** `upkeep` (no deep re-scan; the authoritative full rebuild is the on-demand local `/ppt-project-management scan`).
