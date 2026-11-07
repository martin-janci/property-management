# PPT Project State

_Generated: 2026-11-07 — routine Phase 1.6 role-rotation (pm_cursor idx 3 → 4). Role focus: **pm-qa** (74-day stale rotation slot). Coverage `scan_kind=upkeep`; coverage_cursor idx 0 → 1 (epic-10a re-checked; all 49/49 stories remain `done` — candidate pipeline exhausted). pm-scrum-master always-on._

## Delta since last routine run (2026-11-06 14:58 UTC)

- **2 PRs merged** (#3350, #3351) — both dispatcher-landed code-review follow-ups:
  - **#3350** code-review-reality-web-share-cmp-unchecked-json (+112/-1, 3 files) — validates shared-comparison listing bodies at the API boundary, closes the raw `as Promise<ListingSummary>` cast with a new `listingSummarySchema.ts` parser. Backlog row `code-review-reality-web-share-cmp-unchecked-json` → `done` this run (resolution check passed: evidence-gone + PR verbatim ID match).
  - **#3351** code-review-ppt-web-ui-registry-no-i18n (+921/-160, 10 files = 6 locale JSONs + 4 registry components) — wires PetRegistrationForm/List + VehicleRegistrationForm/List through react-i18next, 0 remaining hardcoded placeholders. Backlog row `code-review-ppt-web-ui-registry-no-i18n` → `done` this run.
- **0 open PRs touched** since cursor. **0 new/updated issues.** **0 closed-not-merged PRs.**
- **Dispatcher buffer-low alert (trigger payload):** `claimable=6/72 honest-claimable=0 (all remaining backlog mobile-native-gated or dep-poisoned); coverage exhausted — refill planner`. Confirmed by this run: no open backlog item meets the score≥3 readiness gate; the lone open `security` row (`code-review-ppt-web-core-refresh-token-localstorage`, score 2) is `confidence: medium`, so the fast-track does not fire.
- **Carried blockers unverified** since 2026-10-28: #3187 (failing dev test), #3168 (cargo-deny advisories), #3131 (merge pipeline deadlock), #2652 (KMP cloud builds), #2951 (RN jest-expo). This run's action list queues a re-verify as high-priority.

## Executive summary

- **Delivery: 49/49 stories done across 13 epics in coverage; sprint 'Epic 6, 7A, 8A & 10A' is 4/4 done.** Candidate pipeline exhausted — the next step is a full coverage scan + new sprint, not more backlog pulls.
- **2 PRs merged** — both are dispatcher-driven cleanup of carried backlog rows. No new feature scope landed.
- **pm-qa rotation came up 74 days stale** (last run 2026-08-25). The backlog of QA observations surfaces six high-priority test gaps: cross-tenant regression for violations `add_evidence` (#3164), portfolio_analytics cross-tenant IDOR (#2946), announcement fan-out RLS integration test (replacing pure-Rust re-model #2484), Android SSO CSRF callback (#2574) + mobile layout-cache (#2486), dispute KPI test unquarantine (#2575), and the mobile-rn lint gate + submit-wiring smoke tests.
- **Carried risk: 83-day-old mobile-rn lint gap** (eslint-plugin-react-hooks + i18next/no-literal-string) still unmitigated — the fake-submit cluster (ReportFault / reality-web /sell / VoteDetail ranked ballot) points at the same defect class. Blocking vs warn-only adoption remains the open decision.
- **Action-list refilled**: 11 → 23 open (12 new items from pm-scrum-master + pm-qa). Buffer still below the 36-task target; reaching it requires the full coverage scan the SM action queued.

## Sprint progress

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · **epics_done = 4/4** at sprint level (all four named sprint epics are `done` in sprint-status.yaml). Coverage holds 13 epics, all 49/49 stories done.

| Epic | Sprint status | Coverage status | Shipped PRs this window |
|---|---|---|---|
| 6 — Announcements & Communication | done (6/6) | 6/6 done | — |
| 7A — Basic Document Management | done (5/5) | 5/5 done | — |
| 8A — Basic Notification Preferences | done (3/3) | 3/3 done | — |
| 10A — OAuth Provider Foundation | done (3/3) | 3/3 done | — |
| 10B — Platform Administration | done (7/7) | 7/7 done | — |
| 80 — Dispute Resolution | done (3/3) | 3/3 done | — |
| 9 — 2FA/TOTP | done (1/1) | 1/1 done | — |
| 79/82/83/84/85 (extended) | — | all done | — |
| 10A (re-checked this run) | done (3/3) | 3/3 done · **last_checked 2026-11-07 (cursor re-check)** | — |

## Shipped since last run (2 PRs merged)

- **#3350** — code-review-reality-web-share-cmp-unchecked-json: validate shared-comparison listing bodies at the API boundary (merged 2026-11-07, +112/-1). Closes the raw-cast gap at `components/comparison/ComparisonUrlHandler.tsx:47`.
- **#3351** — code-review-ppt-web-ui-registry-no-i18n: wire registry pet/vehicle forms + lists through react-i18next (merged 2026-11-07, +921/-160). All four registry components now `useTranslation()`, 0 hardcoded placeholders.

## What's next (top 5 from action list)

1. **[high · pm-scrum-master]** Run a full coverage scan + plan a new sprint — sprint is 4/4 done, 49/49 stories done, candidate pipeline exhausted. Local-only (scan mode); SM-flagged as the primary unblock for the buffer-low condition.
2. **[high · pm-scrum-master]** Unblock the 3 stalled dispatcher claims (quarantined PR #3294 security, PR #3301 review, PR #3333 review).
3. **[high · pm-scrum-master]** Re-verify carried blockers #3187/#3168/#3131 against live gh state.
4. **[high · pm-qa]** Cross-tenant regression test for violations `add_evidence` (#3164), covering dispute `add_evidence` (#2483/#2490) in the same sweep.
5. **[high · pm-qa]** Failing-first test for portfolio_analytics cross-tenant IDOR (#2946).

## Blockers

| Item | Reason | Owner |
|---|---|---|
| Dispatcher claim `code-review-ppt-web-core-refresh-token-localstorage` (PR #3294) | Quarantined security finding; quarantine reason not provided in state | pm-security |
| Dispatcher claims PR #3301 (reality-server inquiry dead-FK classify) and PR #3333 (screen-map drift) | Both in review and awaiting merge >7 days | pm-backend |
| Mobile-native-gated backlog (#2652 KMP cloud builds, #2951 RN jest-expo) | Carried from 2026-10-28; gates most of the remaining backlog so honest-claimable=0. **Not re-verified this run.** | pm-devops |
| #3187 / #3168 / #3131 (failing dev test, cargo-deny advisories, merge-pipeline deadlock) | Carried from 2026-10-28; current status unverified | pm-security |

## Role focus today

**Roles run:** `pm-scrum-master` (always-on), `pm-qa` (rotating; last run 2026-08-25). Per-role one-line summary:

- **pm-scrum-master** — Sprint is complete; candidate pipeline exhausted; 3 claims stalled >7 days. Primary ask: full coverage scan + new sprint + re-verify the carried-blocker trio.
- **pm-qa** — Test fidelity + test-shadow debt are the open QA risk. Six test gaps flagged (cross-tenant IDOR regressions × 3, Android SSO CSRF, dispute KPI un-quarantine, mobile-rn lint + submit-smoke). Mobile-rn lint gap remains 83 days unmitigated.
