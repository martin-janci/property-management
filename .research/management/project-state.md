# PPT Project State

_Generated: 2026-08-13 — routine Phase 1.6 lightweight upkeep (pm-qa rotation slot). Coverage `scan_kind=upkeep`; pm_cursor idx 3 → 4 (pm-qa → pm-devops next), coverage_cursor idx 5 → 6 (epic-80 re-checked, no material change; advances to epic-81)._

## Executive summary

- **Very quiet 4-hour window.** Zero PRs merged since the previous routine at 2026-08-12T22:45:00Z; one new issue (#2743) filed by the dispatcher itself surfaces two previously-fixed defects that have recurred:
  1. `.research/management/*-archive.json` are back over the 64KiB MCP inline-push ceiling (~638KiB + ~660KiB) — a recurrence of #1162. Every dispatcher terminal transition is one merge away from being unable to persist.
  2. `retry-remint.sh` re-minted `code-review-api-handlers-voice-webhook-default-secret` even though it already landed on dev under PR #2660 (different id) — a recurrence of #2460's cross-issue-id blind spot.
- **Story-level truth vs epic-level rollup.** The active sprint's `sprint-status.yaml` epic block still shows epic-6/7a/10b `in-progress` and epic-80 `partial` even though every underlying story is already `done` in `development_status`. Tracking-hygiene reconciliation is queued.
- **Claim buffer starvation continues.** Six action-list items remain open but only one is meaningfully claimable — 4 KMP items are unlandable in the cloud runner (Gradle/AGP 403), 1 is stem-blocked by the quarantined PR #2684, 1 is an investigative closed-not-merged retry. Alternate landing paths for the KMP block are queued as a top-level action.
- **Fresh code-review finding this run.** Rotating expert review of `api-handlers` surfaced `migration.rs` handlers persisting a hardcoded mock URL as `file_path` and returning it as `download_url` — Epic 66 export feature is non-functional end-to-end. Score 2, medium confidence, vector bug.

## Sprint progress (`_bmad-output/implementation-artifacts/sprint-status.yaml`)

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · **epics_done = 2/6** at the rolled-up level, but story-level detail is much further along (see below and pm-qa/pm-scrum-master decisions to reconcile).

| Epic | Sprint status | Coverage status (13 epics) |
|---|---|---|
| 6 — Announcements & Communication | in-progress | 6/6 stories done in coverage |
| 7A — Basic Document Management | in-progress | 5/5 stories done |
| 8A — Basic Notification Preferences | done | 3/3 stories done |
| 10A — OAuth Provider Foundation | done | 3/3 stories done |
| 10B — Platform Administration | in-progress | 7/7 stories done |
| 80 — Dispute Resolution | partial | 3/3 stories done (this run: coverage upkeep bumped `last_checked` to 2026-08-13 for all three; no status flips) |
| 84 — Documents / e-signature | (extended) | 3/5 done, 2 partial (84-1 direct-S3 wiring, 84-2 sign page) |
| 82 / 83 / 85 / 79 / 81 / 8a / 9 | (extended) | all done in coverage |

## Shipped since last run

_None. Zero PRs merged since 2026-08-12T22:45:00Z._

## What's next (top 5 actions from ranked backlog)

1. **[high] Fix dispatcher archive-oversize regression (#2743)** — shard/prune `.research/management/*-archive.json` back under the 64KiB MCP ceiling. Blocks all dispatcher terminal transitions. Owner: pm-tech-lead.
2. **[high] Fix `retry-remint.sh` ghost-retry dedup** — also-landed-on-dev signal in the failed row's `implementer_summary` should suppress re-mint (closes the #2460 blind spot). Owner: pm-tech-lead.
3. **[high] Triage quarantined PR #2684** — fix_rounds=3 exhausted; classify test-shard 1-4 failure as flake or regression, un-quarantine or drop. Owner: pm-qa / pm-backend.
4. **[high] Wire ppt-web direct-to-S3 upload flow (story 84-1)** — POST /documents/upload-url consumer + UploadDocument integration. Owner: pm-frontend.
5. **[high] Build signer-facing document-sign page in ppt-web (story 84-2)** — scope tightly, prior gap-84-2 attempt failed with no PR. Owner: pm-frontend.

## Blockers

- **Dispatcher infra (issue #2743)** — both `.research/management/*-archive.json` files (~638KiB + ~660KiB) again exceed the 64KiB MCP push ceiling. Every dispatcher terminal transition (assignment `merged`/`failed`, action-list item `done`/`dropped`) is one merge away from being unable to persist. Owner: pm-tech-lead.
- **Claim buffer** — starved of landable work: 4 KMP items structurally unlandable in the cloud runner, 1 stem-blocked by quarantined PR #2684, 1 investigative closed-not-merged. Owner: pm-mobile-native + pm-tech-lead.
- **PR #2684 (workflow-cond-parse-failopen)** — quarantined, CI red on test-shard 1-4, fix_rounds=3 exhausted, in review >5 days. Auto-loop cannot self-recover. Owner: pm-tech-lead.

## Role focus today: **pm-qa** (rotation idx 3; last 2026-06-15, 59d stale) + pm-scrum-master always-on

- **pm-scrum-master** (always-on): headline blocker is the dispatcher archive-oversize recurrence (#2743) — treat this as the number-one delivery-flow issue this cycle. Reviewer capacity remains the tighter constraint than implementer capacity (accounting trio still starving after 15 days).
- **pm-qa** (rotation): flags PR #2684's quarantine as a policy gap (fix_rounds=3 exhausted with no defined un-quarantine owner), the untested `SsoService.kt` KMP auth surface as a security-critical test gap, and the sprint-status epic-level rollup as stale relative to story-level done markers.

## Coverage (upkeep this run — 2026-08-13)

- **`coverage.json` refreshed via mechanical upkeep** — `scan_kind=upkeep`, `generated=2026-08-13T03:03:00Z`, no re-scan.
- **Epic re-check: epic-80** — cursor idx 5. All 3 stories still `done`; `last_checked = 2026-08-13` stamped on 80-1, 80-2, 80-3. No status flips.
- **`coverage_cursor` advances 5 → 6** (epic-80 → epic-81 next run).
- **`pm_cursor` advances 3 → 4** (pm-qa → pm-devops next run). `role_last_run["pm-qa"] = 2026-08-13`.
- **Composition unchanged:** 47 done · 2 partial · 0 not-started across 13 epics.

## Code review slice (Phase 1.5)

- **Segment reviewed:** api-handlers (oldest-unreviewed; tie broken by fallback preference; last review 2026-08-10).
- **Experts:** rust + security.
- **Findings:** 1 medium-severity — `migration.rs` export flow persists a hardcoded mock URL and returns it as `download_url`, leaving Epic 66 export feature non-functional end-to-end for any platform admin invoking `/export/{id}` or `/export/{id}/download`.
- **Next segment:** api-core or reality-server (both tied at 2026-08-10 — will pick next run).
