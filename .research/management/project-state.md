# PPT Project State

_Generated: 2026-11-10 — routine Phase 1.6 incremental upkeep. Role focus: **pm-devops** (74-day stale rotation slot; pm-cursor idx 4 → 5). pm-scrum-master always-on. Coverage `scan_kind=upkeep`; coverage_cursor idx 1 → 2 (epic-10b re-checked, still 7/7 done)._

## Delta since last routine run (2026-11-09 22:53 UTC)

- **1 PR merged** in ~4h: **#3371** (reality-server `list_import_jobs` total fix, with regression test). All code-review reland — no scope drift.
- **6 fresh draft PRs** opened by the dispatcher for new Phase-1.5 findings: #3372 (reality-web realtor-api JSON guard), #3373 (ppt-web-ui payment-match silent fail), #3374 (reality-server SSO callback DB err leak), #3375 (ppt-web-ui buildings error raw-text), #3376 (ppt-web-ui DocumentSearch raw-error i18n), #3377 (reality-server price_map mode validation). All drafts, 0 reviews on most, ≤1 on two.
- **Issue #3357** (dispatcher pipeline stall) still open — primary delivery blocker.
- **No new issues** this window.
- **No action-list items moved to done this run** — PR #3371's slug does not match any open action.

## Executive summary

- **Delivery: 49/49 stories done across 13 epics in coverage.** No status flips; `last_checked` refreshed on epic-10b (idx 1, Platform Administration — 7/7 still done).
- **CI pipeline is healthy** — 0 failures across 15 named workflows in the recent window (pm-devops reading).
- **Dispatcher pipeline health remains the real blocker.** #3357 still open; the 6-draft burst shows claim-generation is working today, but the "stuck relands" tail from the previous window is not resolved.
- **pm-devops rotation came up 74 days stale** — last run 2026-08-31. Three devops actions queued; `deploy-server.yml` dormancy flagged as medium-prob / high-impact risk.

## Sprint progress

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · **epics_done = 4/4** at sprint level · 49/49 stories done in coverage.

| Epic | Sprint status | Coverage status | Shipped PRs this window |
|---|---|---|---|
| 6 — Announcements & Communication | done (6/6) | 6/6 done | — |
| 7A — Basic Document Management | done (5/5) | 5/5 done | — |
| 8A — Basic Notification Preferences | done (3/3) | 3/3 done | — |
| 10A — OAuth Provider Foundation | done (3/3) | 3/3 done | — |
| 10B — Platform Administration | done (7/7) | 7/7 done · **last_checked 2026-11-10 (cursor re-check)** | — |
| 80 — Dispute Resolution | done (3/3) | 3/3 done | — |

## Shipped since last run (1 PR merged)

- **#3371** — code-review-reality-server-imports-total-page-len: fix import-jobs total to report full count, not page length (merged 2026-11-10 00:36 UTC). Shipped with regression test `list_import_jobs_total_is_full_count_not_page_len` in `imports_happy_path_tests.rs`.

## What's next (top 5 from the ranked plan)

1. **[high] pm-scrum-master** — Triage #3357: rebase the 3 blocked PRs OR adjust Tier-1d signal sourcing to stop emitting dead-end vectors. Why: the dispatcher cannot self-heal; still the primary delivery blocker.
2. **[high] pm-scrum-master** — Sweep remaining violations mutation handlers for the 403-vs-404 cross-tenant contract. Why: carried from 2026-10-28; pattern audit still open across moderation.rs / aml_dsa.rs.
3. **[high] pm-devops** — Unblock mobile-native/KMP cloud builds (structural gap persists even though #2652 closed). Why: 5+ backlog items permanently unclaimable in cloud; retitle action to drop the stale issue reference.
4. **[medium] pm-devops** — Verify `deploy-server.yml` dormancy — last run 2026-09-27 (~44d). Why: medium-prob / high-impact risk of cred rotation silently breaking it.
5. **[medium] pm-scrum-master** — Monitor the 6 new draft PRs (#3372-#3377) through CI and review; retire any failing Band-C gates early.

## Role focus today

**pm-devops** (rotating, slot 4) + **pm-scrum-master** (always-on). Both role summaries written to `.research/management/roles/`.

## Blockers

- **#3357** dispatcher pipeline stall — 3 approved PRs blocked on non-mechanical conflicts + Tier-1d signals dead-end. Owner: pm-scrum-master.
- **mobile-native-kmp backlog (5+ items)** — structural cloud-runner gap persists; items remain structurally unclaimable even after parent issue #2652 closed. Owner: pm-devops.

## Carried risks (unchanged)

- Three concurrent frontend/mobile code-review bugs (pre-11-09) suggest end-to-end submit-smoke is missing — see `.research/management/risks.md`.
- pm-frontend rotation 83-day stale (last run 2026-08-06) — rotation-staleness bias proposal still open.
- Dispatcher planner starvation (now at 6-10 claimable non-KMP; this run's 6 drafts mitigate short-term).
- **(new)** `deploy-server.yml` dormant ~44 days — may be silently broken.
- **(new)** Action-list cites closed issue #2652; retitle needed.
