# PPT Project State

_Generated: 2026-11-07 — routine Phase 1.6 incremental upkeep. Role focus: **pm-qa** (74-day stale rotation slot; pm-cursor idx 3 → 4). Coverage `scan_kind=upkeep`; coverage_cursor idx 0 → 1 (epic-10a OAuth re-checked, all 3 stories still done, no PRs touched OAuth surfaces). pm-scrum-master always-on._

## Delta since last routine run (2026-11-06 14:58 UTC)

- **2 PRs merged** (#3350, #3351) — both low-priority technical-debt fixes queued as code-review action items, both now flipped to `done` in action-list.json.
  - **#3350** `code-review-reality-web-share-cmp-unchecked-json` — validates shared-comparison listing bodies at the API boundary; **has test file** (`ComparisonUrlHandler.test.tsx` + `listingSummarySchema.ts`).
  - **#3351** `code-review-ppt-web-ui-registry-no-i18n` — wires registry pet/vehicle forms + lists through react-i18next (4 components + 6 locale JSONs); **NO test file**.
  - Neither PR maps to a coverage story — both are refactor/debt items tracked in action-list, not story-delivery.
- **Planner buffer-low signal (6/72 claimable)** — all open non-dep-blocked work is mobile-native-gated on #2652. The dispatcher's 3-implementer pool (≈36 slots/day) is starving. **QA-visible release-readiness risk** (see pm-qa risks below).
- **Carried blockers unchanged**: #3187 (cross-tenant assign-404 contract, 10 days carried, still UNRESOLVED:pm-security), #3168 (cargo-deny RUSTSEC), #3164 (violations add_evidence authz), #3131 (merge pipeline deadlock), #2652 (KMP cloud builds), #2951 (RN jest-expo).
- **Zero new or updated issues** this window; zero open PRs touched since last run.

## Executive summary

- **Delivery: 49/49 stories done across 13 epics in coverage.** No status flips this run; `last_checked` refreshed on epic-10a (idx 0, OAuth Provider Foundation — stories 10a-1/2/3 still done, nothing in the window touched the OAuth surfaces).
- **2 PRs merged, both technical-debt** — the mix (1 of 2 shipped with a test) is the same test-shadow pattern pm-qa flagged on **2026-08-25**, now 74 days unresolved. The CI diff-guard decision proposal is still unformalized.
- **pm-qa rotation came up 74 days stale** — last run 2026-08-25. The window since has produced ≥7 dispatcher-authored fixes without regression tests; the backlog has queued three "wire-but-no-POST" bugs that a QA pattern-sweep would catch. Six pm-qa next_actions queued (two high, four medium); two new risks (`pm-qa-test-shadow-pattern-persistent-2026-11-07` high/medium, `pm-qa-planner-starvation-mobile-native-gating-2026-11-07` high/high).
- **Planner buffer at 6/72 claimable** — mobile-native structural gate (#2652) is the single release-gate; pm-scrum-master queued an off-rotation Tier-1b backlog scan scoped to backend+frontend+reality vectors.

## Sprint progress

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · **epics_done = 4/4** at sprint level · Coverage holds 13 epics all at done.

| Epic | Sprint status | Coverage status | Shipped PRs this window |
|---|---|---|---|
| 6 — Announcements & Communication | done (6/6) | 6/6 done | — |
| 7A — Basic Document Management | done (5/5) | 5/5 done | — |
| 8A — Basic Notification Preferences | done (3/3) | 3/3 done | — |
| 10A — OAuth Provider Foundation | done (3/3) | 3/3 done · **last_checked 2026-11-07 (cursor re-check)** | — |
| 10B — Platform Administration | done (7/7) | 7/7 done | — |
| 80 — Dispute Resolution | done (3/3) | 3/3 done | — |
| 9 — 2FA/TOTP | done (1/1) | 1/1 done | — |
| 79/81/82/83/84/85 (extended) | — | all done | — |

## Shipped since last run (2 PRs merged)

- **#3350** — code-review-reality-web-share-cmp-unchecked-json: validate shared-comparison listing bodies at the API boundary. Touches `ComparisonUrlHandler.tsx` + `ComparisonUrlHandler.test.tsx` + `listingSummarySchema.ts`. **Has test file.** Action-list item flipped to `done`.
- **#3351** — code-review-ppt-web-ui-registry-no-i18n: wire registry pet/vehicle forms + lists through react-i18next (4 ppt-web components + all 6 locale JSONs). **No test file.** Action-list item flipped to `done`; pm-qa queued a Vitest i18n regression backfill (`pm-qa-test-shadow-regression-registry-i18n-2026-11-07`).

## Blockers (0 new, 6 carried)

- **#3187** — Failing test on dev blocks backend CI: `cross_org_assign_violation_is_not_found` (carried 10 days, still UNRESOLVED:pm-security). Owner: pm-security.
- **#3168** — cargo-deny RUSTSEC advisories gate every backend PR merge (carried, high). Owner: pm-security.
- **#3164** — violations `add_evidence` sub-resource ungated (carried, high). Owner: pm-backend.
- **#3131** — merge pipeline deadlocked (carried, medium). Owner: pm-backend.
- **#2652** — mobile-native/KMP cloud builds blocked (carried, high; now also the SINGLE release-gate for all mobile-native QA work and the root cause of the planner-starvation signal). Owner: pm-devops.
- **#2951** — mobile RN jest-expo version rot (carried, medium). Owner: pm-devops.

## What's next (top 5 from the roadmap)

1. **Dispatcher Tier-1b off-rotation scan (backend + frontend + reality vectors)** to break the mobile-native gating and refill the planner from 6/72 to ≥36 — owner: pm-scrum-master / pm-devops — **gates the dispatcher's implementer pool**.
2. **Escalate planner buffer-low as a QA-visible release-readiness risk** — publish a release-blocker view distinct from the planner's buffer-health view (mobile-native items surface as release blockers, not just planner blockers) — owner: pm-qa.
3. **Confirm #3187 cross-tenant assign-404 contract status** on dev CI (10 days carried, UNRESOLVED:pm-security sentinel needs to be resolved to a concrete action-list id) — owner: pm-qa / pm-security.
4. **Backfill Vitest i18n regression test for PR #3351 registry components** — closes the test-shadow debt this window — owner: pm-qa (dep: pm-frontend).
5. **Resolve #3168 cargo-deny RUSTSEC advisories** (carried, gates every backend merge) — owner: pm-security.

## Role focus today: pm-qa (+ pm-scrum-master always-on)

- **pm-qa** (74-day stale rotation): The dormant period produced ≥7 dispatcher-authored fixes without regression tests. Six next_actions queued (two high, four medium); two new risks (test-shadow-pattern-persistent high/medium, planner-starvation-mobile-native-gating high/high). The 2026-08-25 lint-gates decision (eslint-plugin-react-hooks + i18next/no-literal-string in `frontend/apps/mobile`) is now 74 days unresolved — the UNRESOLVED:pm-qa sentinel on the pm-frontend row is specifically the pm-qa warn-only-for-one-sprint sign-off. Unblocking that is a one-decision lever.
- **pm-scrum-master**: Quiet window on merge volume (2 debt PRs) but the planner-health signal is the headline — 6/72 claimable, all remaining open work mobile-native-gated. One next_action queued (`pm-scrum-master-planner-refill-backend-frontend-reality-2026-11-07`) to establish a parallel non-mobile queue.

## Coverage (upkeep this run — 2026-11-07)

- **`coverage.json` refreshed via mechanical upkeep** — `scan_kind=upkeep`, `generated=2026-11-07T11:00:00Z`, no re-scan.
- **Epic re-check: epic-10a OAuth Provider Foundation (cursor idx 0)** — stories 10a-1/10a-2/10a-3 still `done`; no merged PR in the 2026-11-06..2026-11-07 window touched OAuth routes/handlers. `last_checked = 2026-11-07` stamped on all 3 stories; evidence lines appended.
- **Merged-PR evidence check:** #3350 (reality-web share-cmp) + #3351 (ppt-web registry i18n) touch frontend UI only — neither maps to a coverage story; no status flips.
- **`coverage_cursor` advances 0 → 1** (epic-10a → epic-10b next run).
- **`pm_cursor` advances 3 → 4** (pm-qa → pm-devops next). role_last_run["pm-qa"] = 2026-11-07.
- **Composition unchanged: 49 done · 0 partial · 0 not-started** across 13 epics. Missing UC links: 3 (UC-33.1/33.2/33.3, already queued elsewhere).

## Buffer / planner health

- **claimable = 6 / 72** (floor 36) — critical. Mobile-native structural gate (#2652) is the choke-point.
- action-list.json after this run: **17 open · 8 in-progress · 2 done** (2 fresh-done are the 2 flipped items).
- Two poisoned-sentinel rows remain non-claimable until their referenced decisions land: `pm-frontend-mobile-rn-lint-gates-adopt` (UNRESOLVED:pm-qa — resolvable by pm-qa accepting warn-only-for-one-sprint), `pm-scrum-master-violations-xtenant-404-contract-sweep` (UNRESOLVED:pm-security — needs #3187 investigation).

## Role summaries (this run)

- **pm-scrum-master**: 2-PR window, both technical-debt; planner starvation is the actionable headline — queued an off-rotation backlog-refill action scoped to non-mobile vectors.
- **pm-qa** (rotating): test-shadow pattern recurring after 74 days; planner-gating mobile-native items is a release-readiness risk, not just a planner-buffer risk; six actions queued across test-backfill, release-view, and decision-formalization leverage.
