# pm-scrum-master — 2026-11-02

_Role report rendered from the always-on Scrum Master pass of the research routine (Phase 1.6)._

## Summary

All six sprint epics and all 49 coverage stories are done. The last 5 merged PRs were hardening, test and i18n work. The real queue is the backlog (7 open, 2 ready, 5 stalled-review), which is too thin to refill the 7/72 action buffer without new signal scans.

## Sprint progress

- Sprint: **Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth**
- Epics done: **6 / 6**

## Shipped since last run (5 PRs)

- #3209 — fix ranked-ballot a11y labels (closes #3206)
- #3210 — violations cross-org 404 pin on `get_rule` / `get_action` / `get_appeal`
- #3211 — Accounting Invoice i18n + drop native `alert` / `confirm`
- #3212 — reality-web listing-edit `_status` helper extracted with tests
- #3214 — `compliance_wave1b_tests` fixture consolidation

## Next actions (top 6)

| Priority | Action | Owner |
|---|---|---|
| high | Triage stalled PRs #3075 (violations number race), #3108 (owner-analytics 500), #2555 / #2559 (accounting invoice lifecycle + QR), #3119 (NewsScreen i18n): merge, rework or close each | pm-tech-lead |
| high | Run fresh per-package code-review signal scans and a default-mode re-rank to refill the action buffer | none |
| medium | Fix `file_appeal` status guard and rate-limit / dedup `/moderation/report` | pm-security |
| medium | Add `HttpTimeout` to the shared Ktor client in mobile-native | none |
| medium | Implement the two ready KMP plans (create-listing wiring, InquiriesResponse contract) | none |
| low | Refresh `sprint-status.yaml` sprint name/goal (still shows Dec-2025 Epic 6/7A/8A/10A sprint); reconcile `test_hardening_batch` | none |

## Risks (top 5)

| Risk | P | I | Mitigation |
|---|---|---|---|
| Action buffer starvation (7/72): open + ready backlog is only 9 items, coverage has no gaps to feed it | high | medium | Run new signal scans; broaden the scan vectors; do not rely on coverage upkeep |
| Churn-hotspot items keep spawning small test-consolidation PRs on the same four areas (low-value throughput) | high | low | Pause churn-hotspot generation for those files until #3213 and #3208 land |
| Five stalled PRs, some open 10–15 days, including the violations race and the owner-analytics 500 | medium | medium | Force a merge-or-close decision this cycle |
| Violations number `COUNT(*)+1` race and appeal re-file with no status guard remain open in the module that just received hardening PRs | medium | medium | Land #3075 and the appeal guard together |
| `sprint-status.yaml` is stale (sprint name from 2025) — downstream reports may mislead | medium | low | Refresh the sprint metadata |

## Blockers

- **PR #3075 / backlog `violations-number-count-race`** — stalled review; the fix is needed before more violations hardening (owner: pm-tech-lead)
- **PR #3108 — owner-analytics portfolio summary 500** — stalled review on a schema-drift fix for a user-facing 500 (owner: rust-backend)
- **PRs #2555 / #2559 — accounting invoice lifecycle + QR** — open 13–15 days, awaiting human judgement (owner: pm-product-owner)

## Decisions needed

- Merge, rework or close stalled PRs #2555, #2559, #3075, #3108, #3119 — owner: pm-tech-lead
- Approve a new scan cycle (scan vectors + packages) to refill the action buffer — owner: pm-product-owner
- Cap churn-hotspot item generation for test-consolidation files — owner: pm-scrum-master

## Open questions

- Does #3213 (draft) fully close the open `VoteDetailScreen` churn-hotspot backlog item?
- Is #3208 awaiting review or CI, and is it related to stalled PR #3119 (same NewsScreen / i18n defaultValue fallback pattern)?
- Is `test_hardening_batch` (`thb-2026-05-25`) still in-progress? Items #480 and #484 look unreconciled.
- Are UC-33.1 to UC-33.3 (flagged missing in coverage `screen_gaps`) intentionally unscoped?
- Why is `pm_cursor.role_last_run.pm-scrum-master` unset, and does the dispatcher need it?
