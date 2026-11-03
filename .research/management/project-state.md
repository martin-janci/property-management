# PPT Project State

_Generated: 2026-11-03 — routine Phase 1.6 incremental upkeep. Role focus: **pm-qa** (70-day stale rotation slot; pm-cursor idx 3 → 4). Coverage `scan_kind=upkeep`; coverage_cursor idx 0 → 1 (epic-10a OAuth Provider Foundation re-checked, all 3 stories still done, no PRs touched OAuth). pm-scrum-master always-on._

## Delta since last routine run (2026-11-02)

- **5 PRs merged** (#3216–#3220) — dispatcher batch landing follow-ups to the 2026-11-02 window: one screen-map doc reconcile (#3216), one reality-web numeric-coercion helper extraction (#3217), two ppt-web i18n migrations (#3218 EvidenceUploader + #3220 community feature), and one reality-server public-API hardening (#3219 price-map ILIKE escape). All merged in the same 50-second burst on 2026-11-02 12:36 UTC.
- **1 new backlog surface each**: #3217 drifted the reality screen-map doc one touch (follow-up to #3212's _status work; `screen-map-drift-pr-3217-reality` logged, score=2, likely closed by one doc Agent Log line). #3220 lit up `ItemForm.tsx` as a first-time churn hotspot (127 lines in one PR — one-shot i18n migration, not an instability pattern).
- **Repeated-churn three-strike confirmed**: `VoteDetailScreen.test.tsx` crossed `runs_seen 2→3` (PRs #3189/#3209/#3213 within 7d). Phase 1.5 rotating review of **mobile-rn** segment surfaced the matching behavioural-test gap (`code-review-mobile-rn-votedetail-ballot-untested`, score 1) — the test file is churning on edge-case rendering but has zero behavioural coverage of the ballot logic itself.
- **Phase 1.5 (mobile-rn) also surfaced two i18n defects**: `AnnouncementDetailScreen` renders raw backend status enum on a primary visual pill (score 2 — same anti-pattern as the already-fixed `OutagesScreen` from PR #3106); `VoteDetailScreen` interpolates raw status into an English-only `defaultValue` template (score 1 — the `voting.notActive` key is absent from every locale).
- **New backlog repeated-churn**: `violations_role_authz_tests.rs` fired first-time repeated-churn (PR #3171 seeded, PR #3184 added 285 lines of coverage). Signal is informational — regression suite churn reflecting the violations mutation surface, not test instability.
- **Carried blockers unchanged** (and the recurrent #3215 self-approval wedge still the dominant one): #3215 (self-approval merge wedge, 10 PRs stuck in review), #3168 (cargo-deny RUSTSEC advisories), #2652 (KMP cloud builds), #2951 (RN jest-expo), #3131 (merge pipeline deadlock).

## Executive summary

- **Delivery: 49/49 stories done across 13 epics in coverage.** No status flips this run; `last_checked` refreshed on epic-10a (idx 0, OAuth Provider Foundation — all 3 stories still done, no PRs in the window touched OAuth surfaces).
- **5 PRs merged** — follow-ups and i18n catch-up to the 2026-11-02 window. All resolve existing backlog rows that were already marked `done` by yesterday's run. No new critical blockers surfaced.
- **pm-qa rotation came up 70 days stale** — last run 2026-08-25. The backlog has accumulated **test coverage gaps** across mobile-rn (3-strike on VoteDetailScreen.test.tsx; behavioural ballot tests missing) and api-handlers (first-time repeated-churn on violations role-authz tests). The pattern is "regression tests grow around merged security/bug fixes while behavioural coverage of the same features stays thin" — queued as a pm-qa next_action (`behavioural-vs-regression-coverage-audit`).
- **No new blockers on dev CI**: screen-map CI remains green on main (last run 2026-11-02T12:36 UTC on dev); the one 2026-10-28 blocker (#3187 cross-tenant assign-404) landed via PR #3188 earlier and is now closed.

## Sprint progress

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · **epics_done = 4/4** at sprint level (all four named sprint epics are status: done in sprint-status.yaml). Coverage holds 13 epics all at done. The repository is in post-sprint stabilization mode — PRs this window are code-review follow-ups, hotspot cleanups, and i18n catch-up, not new feature stories.

| Epic | Sprint status | Coverage status | Shipped PRs this window |
|---|---|---|---|
| 6 — Announcements & Communication | done (6/6) | 6/6 done | — |
| 7A — Basic Document Management | done (5/5) | 5/5 done | — |
| 8A — Basic Notification Preferences | done (3/3) | 3/3 done | — |
| 10A — OAuth Provider Foundation | done (3/3) | 3/3 done · **last_checked 2026-11-03 (cursor re-check)** | — |
| 10B — Platform Administration | done (7/7) | 7/7 done | — |
| 80 — Dispute Resolution | done (3/3) | 3/3 done | #3218 (EvidenceUploader i18n) |
| 9 — 2FA/TOTP | done (1/1) | 1/1 done | — |
| 79/82/83/84/85 (extended) | — | all done | #3217 reality-web listing-edit helper; #3219 reality-server price-map escape; #3220 ppt-web community i18n |

## Shipped since last run (5 PRs merged)

- **#3216** — docs(reality-web): reconcile account-listing-edit screen-map with PR #3212 (docs-only, closes `screen-map-drift-pr-3212-reality`).
- **#3217** — repeated-churn reality-web listing-edit: extract numeric coercion into tested `_number` helper (3 files; closes the `repeated-churn-...edit/page.tsx` row).
- **#3218** — code-review ppt-web-ui: i18n EvidenceUploader validate errors + drop-zone + placeholder (8 files, 6 locales + component + test).
- **#3219** — churn-hotspot price_map.rs: escape LIKE metacharacters in city filter (1 file; closes an `ILIKE` wildcard-smuggling class on the public, unauthenticated `/price-map` endpoint).
- **#3220** — code-review ppt-web-ui: internationalize ppt-web community feature (23 files; adds a 323-key `community.*` namespace across all six locales).

## Blockers (0 new, 5 carried)

- **#3215** — dispatcher merge pipeline wedged: approved PRs cannot be merged because the dispatcher bot cannot approve its own PRs; 10 active assignments sit in terminal `review` state (carried, high). Owner: operator (needs a second GitHub approver identity or a gate relaxation).
- **#3168** — cargo-deny RUSTSEC advisories gate every backend PR merge (carried, high). Owner: pm-security.
- **#3131** — merge pipeline deadlock (carried, medium). Owner: pm-backend.
- **#2652** — mobile-native/KMP cloud builds blocked (carried, high). Owner: pm-devops.
- **#2951** — mobile RN jest-expo version rot (carried, medium). Owner: pm-devops.

## What's next (top 5 from the roadmap)

1. **Resolve #3215 self-approval wedge** — unblocks 10 approved PRs stuck in `review`; the dominant delivery blocker right now. Owner: operator.
2. **Fix `AnnouncementDetailScreen` status pill i18n** (same pattern as the already-landed `OutagesScreen` fix PR #3106; non-English users see raw English status on a primary badge). Owner: pm-frontend.
3. **Add behavioural tests for VoteDetailScreen ballot logic** (`buildAnswers`, `RankedQuestionCard.move`, `isAnswerComplete`, `castMutation` payload shape) — pairs with the three-strike repeated-churn on the test file. Owner: pm-qa.
4. **Audit the mobile-rn status-enum rendering pattern cluster** (OutagesScreen already fixed, AnnouncementDetailScreen now surfaced, VoteDetailScreen notActive) — one lint rule or one shared helper would stop the recurrence. Owner: pm-frontend / pm-qa.
5. **Resolve #3168 cargo-deny RUSTSEC advisories** (carried) — Owner: pm-security.

## Role focus today: pm-qa (+ pm-scrum-master always-on)

- **pm-qa** (70-day stale rotation): The dormant period masked a growing cluster of "regression tests on top of merged fixes without behavioural coverage of the feature itself" — explicit in the three-strike `VoteDetailScreen.test.tsx` churn (edge-case render tests growing while ballot logic ships untested) and the first-time repeated-churn on `violations_role_authz_tests.rs` (coverage expands as merged security fixes compound). Two pm-qa next_actions queued (both medium): `behavioural-vs-regression-coverage-audit` and `mobile-rn-status-enum-render-lint-rule`. One new medium risk on the rotation cadence itself (`pm-qa-rotation-70d-stale-2026-11-03`).
- **pm-scrum-master**: 5 merged PRs this window — all follow-ups to the 2026-11-02 batch (no new epics, no sprint delta). The window is NOT quiet (5 new backlog rows, 3 code-review findings, 1 screen-map-drift, 2 three-strike cumulative re-emits). One next_action queued (`unblock-3215-self-approval-wedge`) to escalate the self-approval wedge as the dominant delivery blocker.

## Coverage (upkeep this run — 2026-11-03)

- **`coverage.json` refreshed via mechanical upkeep** — `scan_kind=upkeep`, `generated=2026-11-03T03:10:00Z`, no re-scan.
- **Epic re-check: epic-10a OAuth Provider Foundation (cursor idx 0)** — all 3 stories (`OAuth 2.0 Authorization Server`, `OAuth Client Registration`, `OAuth Token Management`) still `done`; no merged PR in the 2026-11-02..2026-11-03 window touched OAuth surfaces. `last_checked = 2026-11-03` stamped.
- **Merged-PR evidence check:** 5 merged PRs this window — all target follow-up / i18n / hotspot surfaces that do not map to a specific coverage story at the sub-feature granularity; no story-status flips.
- **`coverage_cursor` advances 0 → 1** (epic-10a → epic-10b next run).
- **`pm_cursor` advances 3 → 4** (pm-qa → pm-devops next). role_last_run["pm-qa"] = 2026-11-03.
- **Composition unchanged: 49 done · 0 partial · 0 not-started** across 13 epics. Missing UC links: 3 (UC-33.1/33.2/33.3, already queued elsewhere).
