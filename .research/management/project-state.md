# PPT Project State

_Generated: 2026-08-24 — routine Phase 1.6 lightweight upkeep (pm-qa rotation slot). Coverage `scan_kind=upkeep`; `pm_cursor` idx 3 → 4 (pm-qa → pm-devops next), `coverage_cursor` idx 5 → 6 (epic-80 re-checked, no status change; advances to epic-81). Window 2026-08-22T03:03:20Z → 2026-08-24T03:12:08Z: 13 PRs merged, 6 issues opened-and-closed._

## Executive summary

- **Coverage closed: 49/49 stories done, 0 partial, 0 not-started** — the first fully-closed map. Both carried `partial` stories turned out to be **stale, not open**:
  - **84-1 (S3 presigned URLs)** — `DocumentUpload.tsx` calls `useUploadDocumentDirect()`; `@ppt/api-client documents/api.ts::uploadDocumentDirect` chains `POST /api/v1/documents/upload-url` → S3 `PUT` → register, pinned by `documents/api.test.ts`.
  - **84-2 (e-signature)** — `DocumentSignPage` routed at `/sign`; `DocumentSignaturePanel` + token-hygiene + i18n-parity tests present; screen-map `ppt/document-sign` `buildStatus: shipped`.
- **The milestone is hollow, and that is the real finding.** 8 of the 13 PRs merged this window (AML/compliance, facilities booking, verification badge, voice assistant) map to **no coverage story at all**. The 49-story map no longer describes where code is changing, and the gap ranker now has zero story candidates — buffer sits at **20/36 open**.
- **Delivery pattern of the window: rework.** 5 of 13 merged PRs were `from-merged-review` follow-ups, and **two of them fixed PRs merged in the same window**: #2826 → #2831 → #2834 (held-notification drain) and #2829 → #2832 → #2833 (AML decision dialogs). `quiet_hours_drain.rs` and `AmlDashboardPage.tsx` were each patched **twice inside 48 hours**.
- **pm-qa root cause: the gate is under-catching by test *level*, not test *count*.** #2826 shipped migration `00234` + repository changes with **8 pure in-process `#[test]` and zero `#[sqlx::test]`** — the multi-replica race it introduced was invisible to every test it added; #2834 then added exactly the two DB-backed cases that catch it. #2829 shipped 4 happy-path `it()` cases with no dialog-remount case; #2833 then added exactly those two. `pr-reviewer-prompt.md` instructs the reviewer to **"Skim"** test files and has no risk-class → required-test-level notion, so both PRs read as well-tested.
- **Rework rate is trending hard:** post-merge review `with_issues/prs_scanned` went **0/52** (2026-08-06→08-14) → **8/36 = 22 %** (2026-08-20→08-23); this window's `from-merged-review` share is **5/13 = 38 %**. Nothing currently tracks it.
- **Open PRs (17), and the signal is buried:** 13 are dependabot, none touched since last run. Human work is 4 stale PRs — #2555 / #2558 / #2559 (UC-ACC-05 accounting, **26 days**, zero reviewer engagement) and draft #2744 (dispatcher un-wedge, 10 days).

## Sprint progress (`_bmad-output/implementation-artifacts/sprint-status.yaml`)

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · **epics_done = 3/5** unchanged this run.

| Epic | Sprint status | Coverage status (13 epics, 49 stories) |
|---|---|---|
| 6 — Announcements & Communication | in-progress | 6/6 done (6-5 evidence refreshed via #2836 mobile ThreadDetail i18n) |
| 7A — Basic Document Management | in-progress | 5/5 done |
| 8A — Basic Notification Preferences | done | 3/3 done (8a-2 evidence refreshed via #2826/#2834) |
| 10A — OAuth Provider Foundation | done | 3/3 done |
| 10B — Platform Administration | in-progress | 7/7 done (10b-5 evidence refreshed via #2827 CSV sanitizer) |
| 80 — Dispute Resolution | **partial (STALE)** | **3/3 done — re-checked this run**; sprint-status header contradicts its own `development_status` |
| 84 — Documents / e-signature | (extended) | **5/5 done — 84-1 and 84-2 cleared as stale-partial this run** |
| 81 / 82 / 83 / 85 / 79 / 8a / 9 | (extended) | all done (81-2 + 83-2 evidence refreshed via #2827 / #2821) |

## Shipped since last run (13 PRs, all merged into `dev`, all `post-merge-reviewed`)

- **#2838** — churn-hotspot: centralize voice OAuth token encryption (`voice_webhooks.rs`)
- **#2837** — mobile RN: localize voice-assistant confirmation/error strings
- **#2836** — mobile RN: localize `ThreadDetailScreen` UI strings
- **#2835** — mobile RN: run `VoteDetailScreen` hooks before the `voteId` early return
- **#2834** — gh-issue-2831: atomic claim so the quiet-hours drain delivers held notifications at-most-once across replicas *(fixes #2826)*
- **#2833** — gh-issue-2832: reset AML EDD/Review dialog state per assessment *(fixes #2829)*
- **#2830** — i18n facilities booking UI strings
- **#2829** — replace AML dashboard prompt/alert decision flow with in-app dialogs + i18n *(spawned #2832)*
- **#2828** — surface ppt-web facilities booking fetch/approve/reject/cancel errors
- **#2827** — gh-issue-2822: neutralize CR/LF in the CSV export sanitizer
- **#2826** — gh-issue-2823: per-channel bookkeeping + bounded retry for the held-notification drain *(spawned #2831)*
- **#2825** — gh-issue-2824: i18n `VerificationBadge` expiry copy + de-duplicate expiry logic
- **#2821** — gate direct-connect OTA credential writes on manager role (booking connect non-manager hijack)

Issues: 6 opened and **all closed** this window — #2822, #2823, #2824, #2831, #2832 (`follow-up` + `from-merged-review`) and #2743 (`bug,follow-up,infra,dispatcher`).

## What's next (top 5 actions from ranked backlog)

1. **[high] Run the LOCAL `/ppt-project-management scan`** — coverage is 49/49 done and most merged work falls outside the map; the gap ranker has no candidates and the buffer is 20/36 — **owner: pm-tech-lead**.
2. **[high] Resolve the UC-ACC-05 accounting trio (#2555 / #2558 / #2559)** — 26 days, zero reviewer engagement; review-and-merge or close-and-re-plan — **owner: pm-tech-lead**.
3. **[high] Gate migration PRs on a DB-backed test** — any diff touching `backend/crates/db/migrations/**` must add ≥1 `#[sqlx::test]`; re-run against #2826's diff it blocks, against #2834's it passes — **owner: pm-qa**.
4. **[high] Add a risk-class → required-test-level table to `pr-reviewer-prompt.md`** and carve concurrency / cross-process / component-lifecycle changes out of its "Skim test files" rule — **owner: pm-qa**.
5. **[medium] Require a dialog-remount ("re-open for a different subject") test** for every ppt-web dialog holding `useState`; backfill `ReviewAssessmentDialog` and `InitiateEddDialog`, which have no test file at all — **owner: pm-qa**.

## Blockers

- **UC-ACC-05 accounting trio (#2555 / #2558 / #2559)** — 26 days open, zero reviewer engagement, untouched since the last run. The accounting/invoice MVP loop cannot advance. Owner: pm-tech-lead.
- **Coverage/planning inputs exhausted** — 49/49 done leaves the gap ranker with no story candidates while 8 of 13 merged PRs fall outside the map; action-list buffer at 20/36 and unfillable from coverage. Owner: pm-tech-lead.
- **Pre-merge review gate under-catching** — 5 of 13 merged PRs were `from-merged-review` follow-ups; two same-window regressions meant two files were patched twice in 48h. Owner: pm-qa.

## Role focus today: **pm-qa** (rotation idx 3; last 2026-06-15, 70d stale) + pm-scrum-master always-on

- **pm-scrum-master** (always-on): produced the delivery synthesis above. Headline = coverage closed at 49/49, but the milestone is hollow — planning inputs are exhausted and review capacity, not implementer capacity, is the binding constraint (26-day accounting trio, 38 % rework rate).
- **pm-qa** (rotation): the pre-merge gate is under-catching by **test level**, not test count. Every regressing PR this window *did* ship tests — they were just at the wrong level for the risk introduced (in-process unit where multi-replica DB was needed; happy-path render where remount was needed). Both defect classes are enumerable and mechanically gateable. Also flags `voice_webhooks.rs` crypto centralization (#2838, 1 file, 1 test marker, repeat churn hotspot) as the most likely next link in the chain, and `ppt-web/features/compliance` (3 pages + 7 components behind one test file) as the thinnest test floor on the highest-churn regulated surface. Full analysis in `roles/pm-qa.md`.

## Coverage (upkeep this run — 2026-08-24)

- **`coverage.json` refreshed via mechanical upkeep** — `scan_kind=upkeep`, `generated=2026-08-24T03:12:08Z`, no re-scan.
- **Epic re-check: epic-80 (Dispute Resolution)** — cursor idx 5. All 3 stories still `done`; `routes/disputes.rs`, four ppt-web dispute pages with tests, and 3 screen-maps present. **Drift recorded:** `sprint-status.yaml` `epics.epic-80` header still says `status: partial / stories_completed: 1` while its own `development_status` lists all three `done`. `last_checked = 2026-08-24` stamped on all 3.
- **Status flips: 2** — `84-1-s3-presigned-urls` and `84-2-esignature-email`, both `partial → done` (stale, verified shipped). Screen-map `ppt/document-sign` `buildStatus` corrected `planned → shipped` in the coverage snapshot.
- **Merged-PR evidence added:** 84-4 + 8a-2 (#2826/#2834), 83-2 (#2821), 10b-5 + 81-2 (#2827), 6-5 (#2836). No other status changes.
- **`coverage_cursor` advances 5 → 6** (epic-80 → epic-81 next run). 13 distinct epics.
- **`pm_cursor` advances 3 → 4** (pm-qa → pm-devops next run). `role_last_run["pm-qa"] = 2026-08-24`.
- **Composition: 49 done · 0 partial · 0 not-started** across 13 epics. 3 missing UC links remain (UC-33.1/33.2/33.3 — all 3 queued into `action-list.json` this run). Zero orphan epics, zero orphan screens, zero validation errors.
- **Buffer: 20/36 open** — the shortfall is map exhaustion, not triage backlog; refill must come from a deep `scan` or the dispatcher's Tier-1d dev-review generator.
