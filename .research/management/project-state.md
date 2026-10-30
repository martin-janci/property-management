# PPT Project State

_Generated: 2026-10-30 — routine Phase 1.6 incremental upkeep. Role focus: **pm-qa** (66-day stale rotation slot; pm-cursor idx 3 → 4). Coverage `scan_kind=upkeep`; coverage_cursor idx 0 → 1 (epic-10a OAuth Provider re-checked, 3/3 stories still done, no PRs touched OAuth surfaces). pm-scrum-master always-on._

## Delta since last routine run (2026-10-28)

- **0 PRs merged** — fully quiet merge window (cursor stays at #3183).
- **0 new issues** since cursor.
- **+6 open draft PRs** this window (#3191–#3196) — mix of dispatcher auto-fixes (`code-review-ppt-web-ui-invoice-qty-price-nan`, `code-review-api-handlers-appeal-no-status-guard`, `code-review-api-handlers-moderation-report-flood`) + 3 Dependabot group bumps (gradle-minor-patch, npm-minor-patch, kotlin). Open PR count 25 → 31.
- **Carried blockers all unchanged**: #3187 (cross-tenant assign-404 CI gate), #3168 (cargo-deny RUSTSEC), #3164 (violations add_evidence authz — PR #3171 draft), #3131 (merge pipeline), #2652 (KMP cloud), #2951 (mobile RN jest-expo).
- **Stale-PR aging bite**: 7/31 open PRs are now 60+ days old (accounting #2555/#2559 at ~70d; Dependabot #2584/#2588/#2589/#2590/#2866 at 60–103d). Reviewer-starvation pattern first called out 2026-07-30 has not cleared.

## Executive summary

- **Delivery: 49/49 stories done across 13 epics in coverage.** No status flips this run; `last_checked` refreshed on epic-10a (idx 0, OAuth Provider — all 3 stories still done, nothing in the window touched OAuth surfaces).
- **Quiet merge window + stale-PR aging** — zero PRs merged in 2 days while the open-draft queue grew by 6. The dispatcher's fix loop is producing but the merge queue is not consuming; reviewer capacity is the obvious bottleneck right now.
- **pm-qa rotation came up 66 days stale** — last run 2026-08-25. In that window the dispatcher's auto-fix loop produced ~10 code-review-* fix PRs where only 1 (#3184) ships with a regression test — test-shadow debt is now compounding. Six pm-qa next_actions queued (three high: attach test to #3188 before merge, replace announcement-fanout re-model test with real sqlx RLS test, land one worked e2e-form-submit example; three medium). Five risks added — including the new `pm-qa-test-shadow-compounding-dispatcher-fix-loop-2026-10-30` (high/high) and the `pm-qa-add-evidence-pattern-chronic-subroute-authz-gap-2026-10-30` observation that dispute #2483 → violations #3164 is a chronic pattern, not two isolated bugs.
- **New blocker on dev CI** carried: #3187 (`cross_org_assign_violation_is_not_found` — assign returns 403, expects 404) — draft fix PR #3188 exists but needs its own regression test attached in-PR before it can merge.

## Sprint progress

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · **epics_done = 4/4** at sprint level. Coverage holds 13 epics all at done.

| Epic | Sprint status | Coverage status | Shipped PRs this window |
|---|---|---|---|
| 6 — Announcements & Communication | done (6/6) | 6/6 done | — |
| 7A — Basic Document Management | done (5/5) | 5/5 done | — |
| 8A — Basic Notification Preferences | done (3/3) | 3/3 done | — |
| 10A — OAuth Provider Foundation | done (3/3) | 3/3 done · **last_checked 2026-10-30 (cursor re-check)** | — |
| 10B — Platform Administration | done (7/7) | 7/7 done | — |
| 80 — Dispute Resolution | done (3/3) | 3/3 done | — |
| 9 — 2FA/TOTP | done (1/1) | 1/1 done | — |
| 79/82/83/84/85 (extended) | — | all done | — |

## Shipped since last run (0 PRs merged)

- _None._ Fully quiet 2-day merge window.

## Blockers (0 new, 6 carried)

- **#3187** — Failing test on dev blocks backend CI: `cross_org_assign_violation_is_not_found` (assign returns 403, expects 404). Draft fix PR #3188 open; needs regression test attached in-PR. Owner: pm-security.
- **#3168** — cargo-deny RUSTSEC advisories gate every backend PR merge (carried, high). Owner: pm-security.
- **#3164** — violations `add_evidence` sub-resource ungated (carried, high); draft PR #3171 in flight. Owner: pm-backend.
- **#3131** — merge pipeline deadlocked (carried, medium). Owner: pm-backend.
- **#2652** — mobile-native/KMP cloud builds blocked (carried, high); also blocks KMP regression-test attachment path. Owner: pm-devops.
- **#2951** — mobile RN jest-expo version rot (carried, medium). Owner: pm-devops.

## What's next (top 5 from the roadmap)

1. **Fix #3187 cross-tenant assign-404 contract AND attach regression test to PR #3188 in-PR** — currently #3188 is a code-only fix; the DEC-001 pattern requires a failing-on-main test file alongside — owner: pm-security + pm-qa — **gates backend CI on dev**.
2. **Replace pure-Rust announcement cross-tenant fanout re-model test (#2484) with sqlx integration test** against the real RLS predicate — unblocks carried risk aged 99 days — owner: pm-qa.
3. **Resolve #3168 cargo-deny RUSTSEC advisories** (carried) — owner: pm-security.
4. **Land one worked e2e-form-submit smoke example (ReportFaultScreen with MSW)** co-owned with pm-frontend — DoD-sets the gate for the remaining ~12 forms, closes the 2026-10-28 fake-submit-pattern-cluster risk — owner: pm-qa + pm-frontend.
5. **Allocate explicit reviewer slot this week for the 7 PRs 60+ days old** — reviewer-starvation worsening; stale accounting PRs risk merge-conflict compounding every week — owner: pm-scrum-master.

## Role focus today: pm-qa (+ pm-scrum-master always-on)

- **pm-qa** (66-day stale rotation): The dormant period coincides with the dispatcher's fix-loop producing patches faster than the test-coverage loop catches up — specifically 7 open `code-review-*` fix PRs touch user-visible defects but only #3184 ships with regression tests. Six next_actions queued (three high, three medium); five risks added — including the new `pm-qa-test-shadow-compounding-dispatcher-fix-loop-2026-10-30` (high/high), the chronic `add_evidence`-pattern sub-route authz gap, the chronic stale-PR reviewer-starvation, no-multi-replica-harness, and KMP-tests-unrunnable-cloud. The 2026-08-25 DEC-001 decision (fix PR must contain failing-on-main test) is now 66 days unresolved — queued for promotion from advisory to CI-blocking.
- **pm-scrum-master**: Fully quiet merge window — one next_action queued (`stale-pr-reviewer-slot`) to turn the reviewer-starvation signal into an explicit weekly slot allocation. The accounting MVP trio (#2555/#2559) is at ~70 days now; the carried `accounting-mvp-trio-reviewer-starvation` risk aged into a chronic condition.

## Coverage (upkeep this run — 2026-10-30)

- **`coverage.json` refreshed via mechanical upkeep** — `scan_kind=upkeep`, `generated=2026-10-30T11:00:00Z`, no re-scan.
- **Epic re-check: epic-10a OAuth Provider (cursor idx 0)** — all 3 stories still `done`; no merged PR in the 2026-10-28..2026-10-30 window touched OAuth/token surfaces. `last_checked = 2026-10-30` stamped.
- **Merged-PR evidence check:** 0 PRs merged in this window; coverage composition unchanged.
- **`coverage_cursor` advances 0 → 1** (epic-10a → epic-10b next run).
- **`pm_cursor` advances 3 → 4** (pm-qa → pm-devops next). role_last_run["pm-qa"] = 2026-10-30.
- **Composition unchanged: 49 done · 0 partial · 0 not-started** across 13 epics. Missing UC links: 3 (UC-33.1/33.2/33.3, already queued elsewhere).
