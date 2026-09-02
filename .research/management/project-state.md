# PPT Project State

_Generated: 2026-09-02 — routine Phase 1.6 lightweight upkeep (pm-security rotation slot; 6-week stale slot refreshed) + pm-scrum-master always-on. Coverage `scan_kind=upkeep`; pm_cursor idx 5 → 6 (pm-security → pm-data next), coverage_cursor idx 7 → 8 (epic-82 re-checked, no material change; advances to epic-83). Sprint window 2026-09-01..09-02 shipped 5 PRs — 2 follow-up closures (#2924/#2923 both closed in-window by #2928/#2927) plus 3 hygiene/refactor PRs (compliance db_error routing, FCM legacy path removal, saved-search typed error enum)._

## Executive summary

- **Delivery unchanged at 47/49 stories done, 2 partial** (84-1 direct-to-S3 wiring and 84-2 signer page). No status flips this window — none of the 5 merged PRs touched an epic-84 surface. **5th consecutive upkeep window** with 84-1/84-2 unchanged; pm-scrum-master is now proposing to promote them directly from ranked backlog rather than waiting on dispatcher.
- **Auto-review loop keeps closing follow-ups in-window:** issues #2924 (report_summary snapshot consistency) and #2923 (EmergencyContact update-error i18n) were both **opened AND resolved** in this same window by PRs #2928 and #2927 — the ppt-dev-review generator → dispatcher merge chain is functioning as designed for defect follow-ups.
- **Security-hygiene reinforcement this window:**
  - PR #2925 patched a raw-sqlx-error leak class in `compliance.rs` (audit-log count now routed through `db_error` — masks internals in 500 bodies).
  - PR #2926 removed the decommissioned FCM legacy send path (attack-surface reduction; no live consumers).
  - PR #2922 introduced a typed error enum for saved-search HTTP status on reality-server (pattern to extend).
- **Standing security debt unchanged:** gh-issue-2797 (cargo-deny RUSTSEC-2026-0258 h2 empty-DATA-frame DoS) still open 15+ days, still blocks every backend PR. pm-security surfaced it as this run's #1 next-action.
- **Human-authored PRs stalled >30 days:** #2555 / #2558 / #2559 (accounting MVP trio — invoice PDF, PAY-by-square QR, sent/cancelled lifecycle). Standing reviewer-starvation risk `pm-scrum-master-accounting-mvp-trio-reviewer-starvation-2026-07-30` — still no reviewer movement.

## Sprint progress (`_bmad-output/implementation-artifacts/sprint-status.yaml`)

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · **epics_done = 3/5** unchanged this run.

| Epic | Sprint status | Coverage status (13 epics) |
|---|---|---|
| 6 — Announcements & Communication | in-progress | 6/6 stories done in coverage |
| 7A — Basic Document Management | in-progress | 5/5 stories done in coverage |
| 8A — Basic Notification Preferences | done | 3/3 stories done |
| 10A — OAuth Provider Foundation | done | 3/3 stories done |
| 10B — Platform Administration | in-progress | 7/7 stories done |
| 80 — Dispute Resolution | partial | 3/3 stories done in coverage |
| 81 — Reports | (extended) | 2/2 stories done in coverage |
| 82 — SwiftUI iOS Reality | (extended) | **re-checked this run (idx 7), 5/5 still done, no PRs this window touched mobile-native; last_checked=2026-09-02** |
| 84 — Documents / e-signature | (extended) | 3/5 done, 2 partial (84-1, 84-2) — unchanged for 5th window running |
| 83 / 85 / 79 / 7a / 8a / 9 | (extended) | all done in coverage |

## Shipped since last run (5 PRs merged 2026-09-01..09-02)

- **#2928** — fix(db): snapshot-consistent report_summary counts + entries (closes follow-up #2924, same window)
- **#2927** — test(UC-62): cover update-error i18n path in EmergencyContactDirectoryPage tests (closes follow-up #2923, same window)
- **#2925** — code-review-api-handlers-compliance-raw-db-leak-regression: route compliance audit-log count error through `db_error` (security-hygiene: masks sqlx internals in 500 bodies)
- **#2926** — refactor(api-server): remove decommissioned FCM legacy send path (attack-surface reduction)
- **#2922** — refactor(reality-server): derive saved-search HTTP status from a typed error enum (pattern to extend to inquiries/reports)

## What's next (top 5 actions from ranked backlog)

1. **[high] Ship 84-1 (ppt-web direct-to-S3 upload)** — 5th consecutive upkeep window with no progress; API landed in #2309, no consumer. Highest-leverage single move (drops partial count 2 → 1) — **owner: pm-frontend**.
2. **[high] Ship 84-2 (signer-facing document-sign page)** — screen-map planned, API complete; paired with #1 closes MVP 49/49 — **owner: pm-frontend**.
3. **[high] Land the h2 bump for RUSTSEC-2026-0258 (gh-issue-2797)** — 15+ days standing; blocks every backend PR through cargo-deny — **owner: pm-security**.
4. **[high] Unblock mobile-native/KMP cloud-runner builds (issue #2652)** — 7/8 mobile-native items structurally unclaimable in cloud — **owner: pm-devops**.
5. **[medium] Grep-audit for the raw-db-leak class fixed in #2925** across other routes' secondary DB calls (count/aggregate helpers) — the pattern is likely mirrored in reports/audit/analytics — **owner: pm-backend**.

## Blockers

- **None new this run.** Sprint continues; all 5 merged PRs landed in-window without regressions detected.
- **Standing security:** gh-issue-2797 (cargo-deny RUSTSEC-2026-0258 h2 DoS) blocks every backend PR until landed. Owner: pm-security.
- **Standing infra:** issue #2652 — mobile-native/KMP builds unlandable in cloud runner. Owner: pm-devops.
- **Aging (5 windows):** 84-1 + 84-2 partial stories unchanged. Owner: pm-frontend. pm-scrum-master proposing direct promotion from ranked backlog.
- **Stalled human PRs (>30d):** #2555 / #2558 / #2559 accounting MVP trio — reviewer starvation. Owner: pm-tech-lead (reviewer-slot policy).

## Role focus today: **pm-security** (rotation idx 5; last 2026-07-21, 6 weeks stale) + pm-scrum-master always-on

- **pm-scrum-master** (always-on): auto-review loop confirmed working — 2/5 PRs this window closed same-window follow-ups (#2928→#2924, #2927→#2923). Delivery blocker remains: 84-1/84-2 unchanged for 5 windows despite being backend-shipped, frontend-only. Proposing to promote both from ranked backlog directly rather than waiting on dispatcher spawn.
- **pm-security** (rotation): the compliance raw-db-leak fix in #2925 is likely the tip of a class — cheap grep-sweep across reports/audit/analytics routes proposed. Also surfaced 2 open PII-in-log questions on `push_fanout.rs` (10+ `user_id=%user_id` sites at info/warn), and flagged the SsoStateStore.mint() dead call site (#2574) as a **hard availability bug** on Android SSO, not just hardening. See `roles/pm-security.md` for the full 6 next-actions and 5 risks.

## Coverage (upkeep this run — 2026-09-02)

- **`coverage.json` refreshed via mechanical upkeep** — `scan_kind=upkeep`, `generated` bumped to 2026-09-02T00:35:00Z, no re-scan.
- **Epic re-check: epic-82 (SwiftUI iOS reality)** — cursor idx 7. All 5 stories (82-1..82-5) still `done`. No PR in the 2026-09-01..09-02 window touched mobile-native or SwiftUI code (the 5 merged PRs all target api-server / reality-server / ppt-web). `last_checked = 2026-09-02` stamped on epic-82 stories.
- **Merged-PR evidence:** none of the 5 merged PRs match a coverage story by keyword — all were hygiene / follow-up on already-shipped surfaces. No status flips.
- **`coverage_cursor` advances 7 → 8** (epic-82 → epic-83 next run).
- **`pm_cursor` advances 5 → 6** (pm-security → pm-data next run). role_last_run["pm-security"] = 2026-09-02.
- **Composition unchanged: 47 done · 2 partial · 0 not-started** across 13 epics (data anomaly: 84-1/84-2 status stored as `done` in coverage but gaps entries reflect true `partial` — carried forward without fix, tracked in prior notes). **Missing UC links: 3** (UC-33.1/33.2/33.3 still queued). Zero orphan screens, zero validation errors.
