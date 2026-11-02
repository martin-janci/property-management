# PPT Project State

_Generated: 2026-11-02 — routine Phase 1.6 incremental upkeep. Role focus: **pm-qa** (69-day stale rotation slot; pm-cursor idx 3 → 4). Coverage `scan_kind=upkeep`; `coverage_cursor idx 0 → 1` (epic-10a re-checked, 3 stories still done, no OAuth-touching PRs). pm-scrum-master always-on._

## Delta since last routine run (2026-11-01)

- **5 PRs merged** — all hardening / test / i18n work, nothing moves a story:
  - **#3209** fix ranked-ballot a11y labels (Closes #3206)
  - **#3210** violations cross-org 404 pin on `get_rule` / `get_action` / `get_appeal`
  - **#3211** Accounting Invoice i18n + drop native `alert` / `confirm`
  - **#3212** reality-web listing-edit `_status` helper extracted + tested
  - **#3214** `compliance_wave1b_tests.rs` fixture consolidation
- **Open PRs touched (2):** #3213 (draft, VoteDetailScreen test stabilization), #3208 (vote-detail dead i18n fallback replacement)
- **Buffer-low alert:** dispatcher reports claimable = 7/72. Backlog `open` + `ready` is only 9 items, and coverage has no gaps to feed the ranker — the only way to refill is fresh per-package signal scans (see next_actions).
- **No new issues; no reverts; no stalled-review signals** (all four candidates already in `state.seen_signals`).

## Executive summary

- **Delivery: 49/49 stories done across 13 epics in coverage.** No status flips this run; `last_checked` refreshed on epic-10a (idx 0, OAuth 2.0 Authorization Server / Client Registration / Token Management — all still done, nothing in the window touched OAuth surfaces).
- **5 PRs merged** — all test-bearing. None introduces new TODO/FIXME, no reverts, no dependabot. The churn is tied across four areas (reality-web listing-edit, ppt-web accounting, mobile VoteDetailScreen, backend violations/compliance tests) → test-consolidation churn, not feature churn.
- **pm-qa rotation came up 69 days stale** — last run 2026-08-25. Three test-adjacent risks surfaced: sprint-tracking staleness (QA can't gate without acceptance criteria), closed hardening issues (#480/#483/#484) possibly without regression tests, KMP contract gap on the two ready plans.
- **Buffer-low (7/72):** the Scrum Master and pm-qa agree — fresh signal scans + a re-rank are the fix. Coverage is already drained; the real queue is the backlog + PR stream.

## Sprint progress

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · **epics_done = 6/6** at sprint level. Coverage holds 13 epics all at done.

| Epic | Sprint status | Coverage status | Shipped PRs this window |
|---|---|---|---|
| 6 — Announcements & Communication | done (6/6) | 6/6 done | — |
| 7A — Basic Document Management | done (5/5) | 5/5 done | — |
| 8A — Basic Notification Preferences | done (3/3) | 3/3 done | — |
| 10A — OAuth Provider Foundation | done (3/3) | 3/3 done · **last_checked 2026-11-02 (cursor re-check)** | — |
| 10B — Platform Administration | done (7/7) | 7/7 done | #3210 (violations xtenant 404 pin) |
| 80 — Dispute Resolution | done (3/3) | 3/3 done | — |
| 79/82/83/84/85 (extended) | — | all done | — |

## Shipped since last run (5 PRs merged)

- **#3209** — fix ranked-ballot a11y labels/hint rendering raw i18n keys (closes #3206). Mobile voting UX.
- **#3210** — churn-hotspot/violations: pin cross-org 404 on `get_rule`/`get_action`/`get_appeal` reads. Backend hardening.
- **#3211** — code-review-ppt-web-ui-invoice-feature-no-i18n: i18n the Accounting Invoice feature + drop native alert/confirm. Replaced the pattern across accounting; same pattern survives in `compliance/ContentModerationPage` and `news/NewsListPage` → new backlog rows this run.
- **#3212** — churn-hotspot reality-web listing-edit: extract tested `_status` helper. Fires `screen-map-drift-pr-3212-reality` because the PR touched reality-web routes without updating `docs/screens/reality/`. (The screen doc already exists; drift is strict per-PR update, so it's a benign-but-tracked signal.)
- **#3214** — repeated-churn `compliance_wave1b_tests.rs`: consolidate suite fixtures.

## What's next (top 5)

1. **[high]** Triage stalled PRs #3075 / #3108 / #2555 / #2559 / #3119 — merge, rework or close each (owner: pm-tech-lead).
2. **[high]** Run fresh per-package code-review signal scans + default-mode re-rank to refill the 7/72 action buffer (owner: pm-scrum-master).
3. **[high]** Reconcile `test_hardening_batch` in `sprint-status.yaml` (#480/#483/#484 closed on GitHub 2026-05-26 but still listed open); set batch to `done` or record why they were closed (owner: pm-qa).
4. **[high]** Add contract tests for `code-review-mobile-native-kmp-inquiries-response-contract` before merging its fix — KMP tests decode a real reality-server inquiry response and fail on the current `page_size` vs `limit` mismatch (owner: pm-qa).
5. **[medium]** Add negative state-transition tests (moderation-report-flood, appeal-no-status-guard) + a Ktor timeout test (`http-client-no-timeout`) to each fix PR (owner: pm-qa).

## Role focus today

- **pm-scrum-master** (always-on): action-buffer refill is the dominant risk; 5 stalled PRs need triage; sprint metadata is stale.
- **pm-qa** (rotating, 69-day stale catch-up): sprint tracking is stale, test_hardening_batch has drifted from GitHub state, KMP contract tests missing, moderation/appeal negative tests missing.

## Blockers

- **PR #3075 / backlog `code-review-api-handlers-violations-number-count-race`** — stalled review on a correctness fix in the module that just received hardening (owner: pm-tech-lead).
- **PR #3108 — owner-analytics portfolio summary 500** — stalled review on a schema-drift fix for a user-facing 500 (owner: rust-backend).
- **PRs #2555 / #2559 — accounting invoice lifecycle + QR** — open 13–15 days, awaiting a human judgement call (owner: pm-product-owner).

Carried: #3168 (cargo-deny RUSTSEC advisories), #2652 (KMP cloud builds), #2951 (RN jest-expo), #3164 (violations add_evidence authz), #3131 (merge pipeline deadlock).
