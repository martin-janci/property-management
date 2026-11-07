# PPT Project State

_Generated: 2026-11-07 — routine Phase 1.6 incremental upkeep. Role focus: **pm-qa** (74-day stale rotation slot; pm-cursor idx 3 → 4). Coverage `scan_kind=upkeep`; coverage_cursor idx 0 → 1 (epic-10a OAuth re-checked, 3/3 stories still done, no PRs touched OAuth surfaces). pm-scrum-master always-on._

## Delta since last routine run (2026-10-28)

- **2 PRs merged** in the last window:
  - **#3350** — reality-web shared-comparison validation (parseListingSummary at the API boundary) with a two-commit TDD split (`test:` fails on unfixed handler → `fix:` turns green). First dispatcher PR in a while to properly model IG3.
  - **#3351** — ppt-web registry pet/vehicle forms + lists routed through react-i18next across all 6 locales (en/sk/cs/de/pl/hu); pure i18n-refactor with existing locale-namespace-parity suite covering regressions.
- **0 open PRs** touched this run, **0 new issues**. Low-churn window.
- **2 action-list items resolved** → `done`:
  - `code-review-reality-web-share-cmp-unchecked-json` (PR #3350)
  - `code-review-ppt-web-ui-registry-no-i18n` (PR #3351)
- **Carried blockers unchanged**: #3187 (cross-tenant assign-404), #3168 (cargo-deny RUSTSEC), #3164 (violations add_evidence authz), #3131 (merge pipeline deadlock), #2652 (KMP cloud builds), #2951 (RN jest-expo).

## Executive summary

- **Delivery: 49/49 stories done across 13 epics in coverage.** No status flips this run; `last_checked` refreshed on the 3 epic-10a OAuth stories (10a-1/2/3) — nothing in the window touched OAuth surfaces, so re-check was negative.
- **2 merged PRs** — both frontend code-review items the dispatcher queued; neither touches a coverage-tracked story, so no coverage flip.
- **pm-qa rotation came up 74 days stale** — last run 2026-08-25. Four pm-qa next_actions queued (two medium, two low), two new risks added (IG3-not-mechanized · screen-map-drift silent accumulation). The pm-qa run **confirms** PR #3350 as the canonical dispatcher TDD contract model and proposes a CI diff-guard to lock it in.
- **No new blockers**; six carried blockers unchanged.

## Sprint progress

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · **epics_done = 4/4** at sprint level. Coverage holds 13 epics all at done.

| Epic | Sprint status | Coverage status | Shipped PRs this window |
|---|---|---|---|
| 6 — Announcements & Communication | done (6/6) | 6/6 done | — |
| 7A — Basic Document Management | done (5/5) | 5/5 done | — |
| 8A — Basic Notification Preferences | done (3/3) | 3/3 done | — |
| 10A — OAuth Provider Foundation | done (3/3) | 3/3 done · **last_checked 2026-11-07 (cursor re-check)** | — |
| 10B — Platform Administration | done (7/7) | 7/7 done | — |
| 80 — Dispute Resolution | done (3/3) | 3/3 done | — |
| 9 — 2FA/TOTP | done (1/1) | 1/1 done | — |
| 79/81/82/83/84/85 (extended) | — | all done | #3350 (reality-web), #3351 (ppt-web) — both map to extended UI surfaces, not sprint epics |

## Shipped since last run (2 PRs merged)

- **#3351** — code-review-ppt-web-ui-registry-no-i18n: ppt-web registry pet/vehicle forms + lists wired through react-i18next (all 6 locales) (merged 2026-11-07T00:27:50Z).
- **#3350** — code-review-reality-web-share-cmp-unchecked-json: reality-web ComparisonUrlHandler validates shared-comparison bodies at the API boundary (new `parseListingSummary` guard) (merged 2026-11-07T00:27:41Z). **First proper two-commit TDD split since the IG3 contract was introduced — codify in CI.**

## Blockers (6 carried, 0 new)

- **#3187** — Failing test on dev blocks backend CI: `cross_org_assign_violation_is_not_found` (assign returns 403, expects 404). Owner: pm-security. (carried, high)
- **#3168** — cargo-deny RUSTSEC advisories gate every backend PR merge (carried, high). Owner: pm-security.
- **#3164** — violations `add_evidence` sub-resource ungated (carried, high). Owner: pm-backend.
- **#3131** — merge pipeline deadlocked (carried, medium). Owner: pm-backend.
- **#2652** — mobile-native/KMP cloud builds blocked (carried, high). Owner: pm-devops.
- **#2951** — mobile RN jest-expo version rot (carried, medium). Owner: pm-devops.

## What's next (top 5 from the roadmap)

1. **Fix #3187 cross-tenant assign-404 contract** (either backend `assign_violation` returns 404 on cross-tenant, or the test's role shape is adjusted) — owner: pm-security — **gates backend CI on dev**.
2. **Adopt CI diff-guard for IG3 TDD gate on `auto-impl/*` branches** (PR #3350 models it; mechanize it before the pattern decays) — owner: pm-qa + pm-tech-lead.
3. **Wire `ReportFaultScreen.tsx` handleSubmit to real `/api/v1/faults` POST** (carried; user-visible data-loss bug) — owner: pm-frontend.
4. **Wire reality-web `/sell` wizard Publish to reality-server POST** (carried; every seller submission silently discarded) — owner: pm-frontend.
5. **Resolve #3168 cargo-deny RUSTSEC advisories** (carried) — owner: pm-security.

## Role focus today: pm-qa (+ pm-scrum-master always-on)

- **pm-qa** (74-day stale rotation): Quiet merge window but a landmark pattern: PR #3350 ships the canonical dispatcher two-commit TDD split (`test:` commit fails on unfixed handler, `fix:` turns green). Four next_actions queued — two propose test extensions on the merged PRs (`reality-web-listing-boundary-coverage`, `ppt-web-registry-i18n-parity-lock`), one proposes mechanizing the IG3 gate as a CI diff-guard, one sweeps screen-map drift on the five open `gh-issue-33xx` follow-ups. Two new risks added: IG3-not-mechanized (medium/medium), screen-map-drift silent accumulation (high/low).
- **pm-scrum-master**: Low-churn window (2 small frontend PRs); no new blockers surfaced; six carried blockers unchanged. The dispatcher backlog underflow identified on 2026-10-28 (buffer 17/36) persists and should motivate a dispatcher-side backlog-scan refresh (out of scope for this skill; flagged to the parent routine).

## Coverage (upkeep this run — 2026-11-07)

- **`coverage.json` refreshed via mechanical upkeep** — `scan_kind=upkeep`, no re-scan.
- **Epic re-check: epic-10a OAuth Provider Foundation (cursor idx 0)** — stories 10a-1/10a-2/10a-3 all still `done`; no merged PR in the 2026-10-29..2026-11-07 window touched OAuth surfaces. `last_checked = 2026-11-07` stamped on all three.
- **Merged-PR evidence check:** #3350 touches reality-web ComparisonUrlHandler (extended epic-82/portal UI, not sprint-epic); #3351 touches ppt-web registry (extended UI, not sprint-epic). Neither flips a coverage story.
- **`coverage_cursor` advances 0 → 1** (epic-10a → epic-10b next run).
- **`pm_cursor` advances 3 → 4** (pm-qa → pm-devops next). role_last_run["pm-qa"] = 2026-11-07.
- **Composition unchanged: 49 done · 0 partial · 0 not-started** across 13 epics. Missing UC links: 3 (UC-33.1/33.2/33.3, already tracked).
