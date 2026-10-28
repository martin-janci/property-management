# PPT Project State

_Generated: 2026-10-28 — routine Phase 1.6 incremental upkeep. Role focus: **pm-frontend** (83-day stale rotation slot; pm-cursor idx 2 → 3). Coverage `scan_kind=upkeep`; coverage_cursor idx 12 → 0 (wrap — epic-9 2FA/TOTP re-checked, 1/1 story still done, `last_checked` refreshed from 2026-07-15 to today). pm-scrum-master always-on._

## Delta since last routine run (2026-10-27)

- **1 PR merged** (#3183) — `code-review-api-handlers-violations-no-xtenant-idor-tests: cross-tenant IDOR regression suite + 404 fix for violation mutations`:
  - 498 new LOC in `backend/servers/api-server/tests/suites/violations_mutation_cross_org_idor_tests.rs` covering `assign_violation`, `add_evidence`, `delete_evidence`, `create_action`, `update_action`, `mark_action_sent`, `record_payment`, `update_appeal`, `decide_appeal` for cross-tenant 404.
  - 44 LOC on `backend/servers/api-server/src/routes/violations.rs` + 19 LOC on `backend/crates/db/src/repositories/violations.rs` — assignee-org-membership guard (ties into the add-evidence authz gap tracked in action-list).
  - **Resolves backlog items** `code-review-api-handlers-violations-no-xtenant-idor-tests` (test-gap, score 1) + `code-review-api-handlers-assign-violation-no-member-check` (bug, score 2) — both marked `done` on 2026-10-27 reconcile.
  - **Zero coverage status flips** — the merge is a defense-in-depth hardening of code already shipped under epic-80 (Dispute Resolution, 3/3 done).
- **1 new issue: #3187** — `Failing test on dev blocks backend CI: cross_org_assign_violation_is_not_found (assign returns 403, expects 404)`. Labelled bug/security/follow-up. **Fix PR #3188 already landed to review** (approved per dispatcher fast-path 2026-10-27 15:14) — contract tightening on `assign_violation` to return 404 (not 403) on cross-org assignee. Action-list carries `gh-issue-3187` (pm-security, high) until merged.
- **Dispatcher Tier-1d harvest (today 2026-10-28)** — 8 unconsumed code-review-finding signals across three segments:
  - `api-handlers` (2): `moderation-report-flood` (medium, +2 bug), `appeal-no-status-guard` (medium, +2 bug).
  - `ppt-web-ui` (3, this run's **pm-frontend focus cluster**): `invoice-vat-rate-payload-drift` (medium, +2), `invoice-qty-price-nan` (medium, +2), `invoice-feature-no-i18n` (medium, +1) — all in the invoice feature.
  - `reality-web` (3): `listing-edit-price-zero` (medium, +2), `listing-edit-i18n-sk` (medium, +1), `comparison-parse-shape` (medium, +1).
  - **These findings are in `.research/signals/2026-10-28-*-tier1d.json` but NOT yet in `backlog.json`** — recurrence of fp-null finding #24 (Tier-1d generator produces faster than the planner drains).

<sub>Prior upkeep note (archived): 2026-10-26 — pm-backend rotation; coverage_cursor 11 → 12 (epic-8a Notification Preferences re-checked).</sub>

## Executive summary

- **Delivery: 49/49 stories done across 13 epics in coverage.** No status flips this run; `last_checked` refreshed on the single epic-9 2FA/TOTP story (cursor wrap: 12 → 0 after this run — epic-9 was the final slot in the sorted epic list).
- **1 PR merged** — defense-in-depth cross-tenant IDOR test suite + assignee-org-membership guard on violations mutations. Narrow backend fix, tests included.
- **Buffer still constrained:** structural blockers are unchanged (#2652 KMP cloud builds, mobile-native-gate), but the dispatcher's current trigger (`buffer-low: claimable=7/72 — 5 open gated mobile-native, WIP over cap`) confirms the pattern — most ready plans are mobile-native-gated. 2 ready plans in `.research/plans/` remain the KMP pair (`-inquiries-response-contract`, `-create-listing-not-wired`).
- **pm-frontend rotation came up 83 days stale** — last run 2026-08-06; today's lens surfaces the **ppt-web invoice feature contract-drift cluster** (three Tier-1d findings in a single feature) as the highest-signal frontend risk this run.
- **Issue #3187 fix PR #3188 approved** — expected to land next; closes the cross_org_assign_violation CI gate.

## Sprint progress

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · **epics_done = 3/5** at sprint level (sprint-status.yaml); coverage-level all extended epics are complete.

| Epic | Sprint status | Coverage status | Shipped PRs this window |
|---|---|---|---|
| 6 — Announcements & Communication | in-progress (3/6)† | 6/6 done | — |
| 7A — Basic Document Management | in-progress (2/5)† | 5/5 done | — |
| 8A — Basic Notification Preferences | done (3/3) | 3/3 done | — |
| 10A — OAuth Provider Foundation | done (3/3) | 3/3 done | — |
| 10B — Platform Administration | in-progress (7/7)† | 7/7 done | — |
| 80 — Dispute Resolution | partial (1/3)† | 3/3 done | #3183 (cross-tenant IDOR tests + assignee-org guard on `assign_violation` — defense-in-depth) |
| 79 — API client integration | (extended) | 4/4 done | — |
| 82 — iOS Reality Portal (SwiftUI) | (extended) | 5/5 done | — |
| 83 — Portal Integrations | (extended) | 3/3 done | — |
| 84 — Documents & e-signature | (extended) | 5/5 done | — |
| 85 — Build/CI configuration | (extended) | 2/2 done | — |
| 9 — 2FA/TOTP | (extended) | 1/1 done · **last_checked 2026-10-28 (cursor re-check)** | — |
| Cross-cutting | — | — | — |

† sprint-status.yaml drift against coverage.json — queued as `pm-scrum-master-reconcile-sprint-status-vs-coverage` (low); a deep scan would resolve it authoritatively.

## Shipped since last run (1 PR merged)

- **#3183** — `code-review-api-handlers-violations-no-xtenant-idor-tests: cross-tenant IDOR regression suite + 404 fix for violation mutations` · backend · defense-in-depth epic-80

## What's next (top 5 from gap-driven ranking)

1. **[high] pm-devops** · Unblock mobile-native/KMP builds in the cloud runner (issue #2652) — _still the keystone blocker; 5 of 7 claimable plans are mobile-native-gated per today's dispatcher fire payload._
2. **[high] pm-backend** · Gate violations `add_evidence` sub-resource with authz check (gh #3164) — _same pattern-class as the just-landed #3183 cross-tenant guard; one row deeper in the violations surface._
3. **[high] pm-security** · Resolve #3187 cross_org_assign_violation CI gate (PR #3188 approved, awaiting merge) — _blocking backend CI; fast-path tracked._
4. **[medium] pm-frontend** · Address ppt-web invoice feature contract-drift cluster (VAT rate payload, qty/price NaN, i18n gap) — _three Tier-1d findings in one feature today; cluster root-cause is Decimal/number mixing + unlabelled strings._
5. **[medium] pm-tech-lead** · Portfolio/owner analytics drift fix (Closes #3104) — _owner-analytics 500s; carried._

## Blockers

- **mobile-native cloud builds (#2652)** — long-carried; the one task that would remove the gate (`pm-devops-unblock-mobile-native-cloud-builds`) is still non-claimable by the gate itself (fp-null #7 recurrence).
- **backend CI red on dev** — `cross_org_assign_violation_is_not_found` failing; fix PR #3188 approved, waiting on merge.

## Role focus today

**pm-frontend** (83 days stale). pm-scrum-master always-on.

### pm-frontend one-line summary

ppt-web invoice feature carries three concurrent Tier-1d findings (VAT rate payload drift, qty/price NaN on blank input, missing i18n keys) — single-cluster root-cause, worth batching into one PR; separately, PR #3024 (dashboard screen-map doc sync) has been draft-stale for 23 days and is the single most-stale frontend PR touching screen docs.
