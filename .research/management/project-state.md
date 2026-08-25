# PPT Project State

_Generated: 2026-08-25 — routine Phase 1.6 lightweight upkeep (pm-qa rotation slot; 71-day stale slot refreshed) + pm-scrum-master always-on. Coverage `scan_kind=upkeep`; pm_cursor idx 3 → 4 (pm-qa → pm-devops next), coverage_cursor idx 5 → 6 (epic-80 re-checked, no material change; advances to epic-81). Sprint window 2026-08-20..08-25 shipped 13 PRs — the auto-review loop closed 7 previously-open action-list items in the same window (perfect same-window close rate)._

## Executive summary

- **Delivery still at 47/49 stories done, 2 partial** (84-1 direct-to-S3 upload wiring and 84-2 sign page). No status flips this window. The last window's 13 PRs were all follow-up hardening / auto-fix, dominated by:
  - Notifications reliability — PR #2826 (per-channel bookkeeping + bounded retry) + PR #2834 (atomic claim so quiet-hours drain delivers at-most-once under >1 replica).
  - AML dashboard UX + i18n — PR #2829 (replace window.prompt/alert with in-app dialogs) + PR #2833 (reset dialog state per assessment).
  - Facilities booking UX — PR #2828 (surface fetch/approve/reject/cancel errors) + PR #2830 (i18n the UI strings).
  - Mobile-RN correctness + localization — PR #2835 (VoteDetailScreen conditional-hooks fix) + PR #2836 (ThreadDetailScreen i18n) + PR #2837 (voice-assistant confirmation i18n).
  - Security / integrations — PR #2821 (gate direct-connect OTA credential writes to manager) + PR #2827 (neutralize CR/LF in CSV export sanitizer) + PR #2838 (centralize voice OAuth token encryption).
  - Verification-badge — PR #2825 (i18n expiry copy + de-dup the expiry-logic).
- **Auto-review loop closed 7 items in-window:** all 4 follow-up issues opened by the last post-merge review (#2822 CSV / #2823 held-notifications / #2831 quiet-hours-drain / #2832 AML-dialog) were resolved by PRs merged the same window; plus dispatcher meta #2743 (archive push ceiling + ghost retry-remint) closed upstream and 2 code-review items (facilities booking silent errors, AML prompt/alert flow). Same-window close rate: 7/7.
- **Zero new open PRs** — dispatcher stack is drained. Buffer sits at 17/36 open, below-half not because the queue is stale but because coverage is 47/49; the ranker has only 5 gap candidates left in the ranked pool.
- **No new blockers** — sprint continues in-progress with no red flags. Main aging risk is the 2 partial 84-x frontend stories (unchanged for 3 windows).

## Sprint progress (`_bmad-output/implementation-artifacts/sprint-status.yaml`)

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · **epics_done = 3/5** unchanged this run. Extended-scope epics (10B, 80, 81, 82, 83, 84, 85, 79, 8A, 9) folded into `coverage.json` and largely done.

| Epic | Sprint status | Coverage status (13 epics) |
|---|---|---|
| 6 — Announcements & Communication | in-progress | 6/6 stories done in coverage |
| 7A — Basic Document Management | in-progress | 5/5 stories done in coverage |
| 8A — Basic Notification Preferences | done | 3/3 stories done (reliability re-hardened this window via #2826 + #2834) |
| 10A — OAuth Provider Foundation | done | 3/3 stories done |
| 10B — Platform Administration | in-progress | 7/7 stories done (AML dialog UX re-hardened via #2829 + #2833) |
| 80 — Dispute Resolution | partial | 3/3 stories done in coverage; **re-checked this run (idx 5), no material change, last_checked=2026-08-25** |
| 84 — Documents / e-signature | (extended) | 3/5 done, 2 partial (84-1, 84-2) — unchanged |
| 82 / 83 / 85 / 79 / 81 / 8a / 9 | (extended) | all done in coverage |

## Shipped since last run (13 PRs merged 2026-08-20..08-25)

- **#2821** — code-review api-handlers booking-connect: gate direct-connect OTA credential writes to manager role (non-manager hijack fix)
- **#2825** — gh-issue-2824: i18n VerificationBadge expiry copy + de-duplicate expiry logic
- **#2826** — gh-issue-2823: per-channel bookkeeping + bounded retry for held-notification drain
- **#2827** — gh-issue-2822: neutralize CR/LF in CSV export sanitizer
- **#2828** — code-review ppt-web-ui facilities-booking silent-errors: surface booking fetch/approve/reject/cancel errors
- **#2829** — code-review ppt-web-ui AML-dashboard: replace window.prompt/alert decision flow with in-app dialogs + i18n
- **#2830** — code-review ppt-web-ui facilities-booking hardcoded-i18n: i18n facilities booking UI strings
- **#2833** — gh-issue-2832: reset AML EDD/Review dialog state per assessment
- **#2834** — gh-issue-2831: atomic claim so quiet-hours drain delivers held notifications at-most-once
- **#2835** — code-review mobile-rn VoteDetailScreen conditional-hooks: run hooks before returns
- **#2836** — code-review mobile-rn ThreadDetailScreen hardcoded-en: localize UI
- **#2837** — code-review mobile-rn voice-cmd hardcoded-en: localize voice-assistant confirmations
- **#2838** — churn-hotspot voice_webhooks.rs (retry1): centralize voice OAuth token encryption

## What's next (top 5 actions from ranked backlog)

1. **[high] Wire ppt-web direct-to-S3 upload (84-1 partial)** — POST /api/v1/documents/upload-url consumer + regression test — **owner: pm-frontend**. Backend has been shipped since 2026-07-30; this is the highest-leverage single move (drops the partial count 2 → 1).
2. **[high] Build signer-facing document-sign page (84-2 partial)** — flip screen-map ppt/document-sign buildStatus planned → shipped — **owner: pm-frontend**. Closes 49/49 MVP delivery when paired with #1.
3. **[high] Resolve gh-issue-2797** — cargo-deny RUSTSEC-2026-0258 (h2 empty-DATA-frame DoS) blocks every backend PR — **owner: pm-security**. Standing since 2026-08-18; still in-progress.
4. **[high] Add >1-replica concurrency integration test for quiet-hours drain atomic claim** (#2834 → #2831 closure gap) — assert at-most-once under 2 racing api-server replicas — **owner: pm-qa**.
5. **[high] Add authz regression test for direct-connect OTA credential writes** (#2821) — assert non-manager is rejected at the booking-connect endpoint (was hijackable) — **owner: pm-qa**.

## Blockers

- **None new this run.** Sprint continues with no red flags; auto-review loop is closing all in-window follow-ups.
- **Standing:** gh-issue-2797 (cargo-deny RUSTSEC-2026-0258 h2 DoS) blocks every backend PR until landed. Owner: pm-security.
- **Aging:** 84-1 + 84-2 partial stories unchanged for 3 upkeep windows — no new dispatcher attempts this window. Owner: pm-frontend.

## Role focus today: **pm-qa** (rotation idx 3; last 2026-06-15, 71d stale) + pm-scrum-master always-on

- **pm-scrum-master** (always-on): produced the delivery synthesis above. Headline = the auto-review loop closed 7 previously-open items in the same 5-day window it opened them — the loop's compensating-transaction property is holding. Reviewer capacity is comfortable (0 new open PRs). Aging 84-x partials are now the top delivery lever.
- **pm-qa** (rotation): flagged 7 regression / concurrency / authz tests that would guard the fixes shipped this window from re-regressing — the atomic-claim concurrency test (#2834) and the booking-connect authz test (#2821) are `high` priority; the mobile-rn conditional-hooks / CSV fuzz / voice-OAuth round-trip / AML dialog-state / VerificationBadge i18n snapshot tests are `medium`/`low`. Also flagged the mobile-rn lint-config gap that would have caught 3 of this window's PRs statically (proposed as a pm-frontend action).

## Coverage (upkeep this run — 2026-08-25)

- **`coverage.json` refreshed via mechanical upkeep** — `scan_kind=upkeep`, `generated` bumped to 2026-08-25T04:24:00Z, no re-scan.
- **Epic re-check: epic-80** — cursor idx 5. All 3 stories still `done`. No PR in the 2026-08-20..08-25 window touched dispute routes/handlers/screens; evidence entry appended to 80-1 noting the negative check. `last_checked = 2026-08-25` stamped on all 3 stories.
- **Merged-PR evidence:** none of the 13 merged PRs match a coverage story by keyword — all were code-review / follow-up hardening. No status flips.
- **`coverage_cursor` advances 5 → 6** (epic-80 → epic-81 next run).
- **`pm_cursor` advances 3 → 4** (pm-qa → pm-devops next run). role_last_run["pm-qa"] = 2026-08-25.
- **Composition unchanged: 47 done · 2 partial · 0 not-started** across 13 epics. **Missing UC links: 1** (UC-33.3 queued this run; UC-33.1/33.2 already queued). Zero orphan screens, zero validation errors.
