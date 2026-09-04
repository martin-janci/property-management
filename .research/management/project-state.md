# PPT Project State

_Generated: 2026-09-04 — routine Phase 1.6 lightweight upkeep (pm-security rotation slot; last-run 2026-06-01, 95d stale slot refreshed) + pm-scrum-master always-on. Coverage `scan_kind=upkeep`; pm_cursor idx 5 → 6 (pm-security → pm-data next), coverage_cursor idx 7 → 8 (epic-82 re-checked, no material change; advances to epic-83). Sprint window 2026-09-01T18:38Z..2026-09-04T14:30Z shipped 8 in-cursor PRs (#2922, #2925–#2928, #2931, #2932, #2935) + 5 late-merged (#2673, #2585, #2586, #2583, #2558)._

## Executive summary

- **Delivery still at 47/49 stories done, 2 partial** (84-1 direct-to-S3 upload wiring and 84-2 sign page). No status flips this window. In-cursor merges (8 PRs) were exclusively `code-review-*` and follow-up-test PRs from the dispatcher's dev-team loop:
  - Backend correctness — PR #2922 (typed-error saved-search HTTP status), PR #2925 (compliance raw-db-error-leak regression closed at compliance.rs:319), PR #2926 (retire decommissioned FCM legacy send path), PR #2928 (snapshot-consistent report_summary counts+entries).
  - Follow-up test hardening — PR #2927 (i18n update-error path in EmergencyContactDirectoryPage), PR #2931 (report_summary test fails on pre-fix code), PR #2932 (e2e 403 agent-review self-review guard).
  - Dependency noise — PR #2935 (dependabot npm-minor-patch, 40 updates).
- **Auto-review loop still working:** 4 of the 8 in-cursor PRs closed follow-up issues (#2923/#2924/#2929/#2930) opened by the from-merged-review pass — the loop is now self-consuming, PR → merged → review-issue → follow-up PR → merged.
- **Dispatcher payload this run: `buffer-low: claimable=8/72 — all remaining backlog is mobile-native-gated`.** The 4 open + 1 ready backlog items are all `mobile-native-kmp`, unclaimable in cloud (issue #2652 correctly gates them at claim time). The routine's response this run is to seed **cloud-landable** vectors — the Phase 1.5 review picked reality-web (skipping saturated mobile-native-kmp) and found the reality-web password-flows-501 stubs, surfaced below.
- **One new fact-grade finding this run (pm-security lens):** reality-web `/account/password`, `/auth/forgot-password`, `/auth/reset-password` render UI and call auth-api client stubs that unconditionally throw `AuthApiError(501, 'NOT_IMPLEMENTED')`. All three self-serve credential flows are dead-ends for real portal users. High confidence (call-path traced). Vector: `bug` (borderline `security`).
- **No new blockers.** Standing gh-issue-2797 (cargo-deny RUSTSEC-2026-0258 h2 DoS) still holds; owner pm-security.

## Sprint progress (`_bmad-output/implementation-artifacts/sprint-status.yaml`)

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · **epics_done = 3/5** unchanged this run. Extended-scope epics (10B, 80, 81, 82, 83, 84, 85, 79, 8A, 9) folded into `coverage.json` and largely done.

| Epic | Sprint status | Coverage status (13 epics) |
|---|---|---|
| 6 — Announcements & Communication | in-progress | 6/6 stories done in coverage |
| 7A — Basic Document Management | in-progress | 5/5 stories done in coverage |
| 8A — Basic Notification Preferences | done | 3/3 stories done |
| 10A — OAuth Provider Foundation | done | 3/3 stories done |
| 10B — Platform Administration | in-progress | 7/7 stories done |
| 80 — Dispute Resolution | partial | 3/3 stories done in coverage |
| 81 — Reports | (extended) | 2/2 stories done in coverage |
| 82 — iOS KMP MVP | (extended) | 5/5 stories done in coverage; **re-checked this run (idx 7), no material change, last_checked=2026-09-04** |
| 84 — Documents / e-signature | (extended) | 3/5 done, 2 partial (84-1, 84-2) — unchanged |
| 83 / 85 / 79 / 7a / 8a / 9 | (extended) | all done in coverage |

## Shipped since last run

**In-cursor (PR # > 2921):** 8 PRs — #2922, #2925, #2926, #2927, #2928, #2931, #2932, #2935 (see role file `roles/pm-scrum-master.md` for details).

**Late-merged (PR # ≤ 2921, cursor filter excluded these):** #2673 (ktor deps), #2585/#2586/#2583 (rust deps), #2558 (feat(acc) invoice PDF render UC-ACC-05.9) — all merged 2026-09-04 07:13–18Z in a dependabot-queue drain sweep.

## What's next (top 5 actions from ranked backlog)

1. **[high] Wire (or feature-flag) 3 reality-web password client stubs** — this run's fresh pm-security finding; 3 UI pages call functions that throw 501 unconditionally, so real-user self-serve credential flows are dead — **owner: pm-security**.
2. **[high] Resolve gh-issue-2797** — cargo-deny RUSTSEC-2026-0258 (h2 empty-DATA-frame DoS) blocks every backend PR — **owner: pm-security**. Standing since 2026-08-18.
3. **[high] Wire ppt-web direct-to-S3 upload (84-1 partial)** — POST /api/v1/documents/upload-url consumer + regression test — **owner: pm-frontend**. Drops partial count 2 → 1.
4. **[high] Build signer-facing document-sign page (84-2 partial)** — flip screen-map ppt/document-sign buildStatus planned → shipped — **owner: pm-frontend**. Closes 49/49 MVP when paired with #3.
5. **[high] Standing: pm-devops-unblock-mobile-native-cloud-builds** (issue #2652 — architectural gate at claim time is stable and correct; open only for the day cloud KMP builds become desirable) — **owner: pm-devops**.

## Blockers

- **None new this run.** Sprint continues with no red flags.
- **Standing:** gh-issue-2797 (cargo-deny RUSTSEC-2026-0258 h2 DoS) blocks every backend PR until landed. Owner: pm-security.
- **Standing infra:** issue #2652 — mobile-native/KMP builds unlandable in cloud runner. Owner: pm-devops (surfaced as this run's #5 action).
- **Aging:** 84-1 + 84-2 partial stories unchanged for 5 upkeep windows. Owner: pm-frontend.

## Role focus today: **pm-security** (rotation idx 5; last 2026-06-01, 95d stale) + pm-scrum-master always-on

- **pm-scrum-master** (always-on): the dispatcher's from-merged-review loop is now self-consuming (4 of 8 in-cursor PRs closed loop-generated follow-up issues #2923/#2924/#2929/#2930). Sprint delivery mechanism sound; the buffer-low condition is entirely on the supply side (mobile-native gate), not throughput.
- **pm-security** (rotation): the reality-web password-flow 501-stub finding is the top new action-list item this run. See `roles/pm-security.md` for the full write-up and dependency question surfaced for pm-tech-lead (reality-server vs. api-server SSO ownership of password reset).

## Coverage (upkeep this run — 2026-09-04)

- **`coverage.json` refreshed via mechanical upkeep** — `scan_kind=upkeep`, `generated` bumped to 2026-09-04T14:30:00Z, no re-scan.
- **Epic re-check: epic-82 (iOS KMP MVP)** — cursor idx 7. All 5 stories (82-1 SwiftUI setup, 82-2 navigation, 82-3 home/search, 82-4 listing/favorites, 82-5 inquiries/account) still `done`. No PR in the 2026-09-01..09-04 window touched iOS KMP routes/screens; `last_checked = 2026-09-04` stamped on all 5.
- **Merged-PR evidence:** none of the 8 in-cursor PRs match a coverage story by keyword — all were correctness hardening of already-shipped surfaces. No status flips.
- **`coverage_cursor` advances 7 → 8** (epic-82 → epic-83 next run).
- **`pm_cursor` advances 5 → 6** (pm-security → pm-data next run). role_last_run["pm-security"] = 2026-09-04.
- **Composition unchanged: 47 done · 2 partial · 0 not-started** across 13 epics. **Missing UC links: 3** (UC-33.1/33.2/33.3 already queued). Zero orphan screens, zero validation errors.
