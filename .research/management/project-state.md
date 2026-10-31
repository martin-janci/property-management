# PPT Project State

_Generated: 2026-10-31 — routine Phase 1.6 incremental upkeep. Role focus: **pm-qa** (67-day stale rotation slot; pm-cursor idx 3 → 4). Coverage `scan_kind=upkeep`; coverage_cursor idx 0 → 1 (epic-10a OAuth re-checked, all three stories still done). pm-scrum-master always-on._

## Delta since last routine run (2026-10-28)

- **15 PRs merged** (#3185–#3205, 10 code-review fixes + 3 pm-* queue items + 2 compliance-tests). All dispatcher-driven; no new epic/story deliveries — all 13 epics remain 49/49 done.
- **1 new issue** (#3206, labeled `follow-up` + `from-merged-review`) — follow-up on PR #3189 (ranked-ballot a11y labels render raw i18n keys due to missing `voting.moveUp/moveDown/rankHint` keys + no-op `t() ?? literal` fallback pattern). Already triaged — not promotion-eligible as untriaged, has a clear owner (mobile-rn).
- **Churn pattern shift:** compliance_wave1b_tests.rs is now at **repeated-churn threshold** (runs_seen=1 going to 2 after this run, +171 lines in #3204); listing-edit `page.tsx` and VoteDetailScreen test are new top-3 hotspots this run.
- **Buffer-low trigger** (`claimable=10/72`): dispatcher claim pool hit the threshold while action-list total open is 30. Suggests UNRESOLVED-sentinel items are blocking claim; pm-scrum-master follow-up queued.
- **Carried blockers unchanged**: #3168 (cargo-deny RUSTSEC advisories), #2652 (KMP cloud builds), #2951 (RN jest-expo), #3131 (merge pipeline deadlock).

## Executive summary

- **Delivery: 49/49 stories done across 13 epics in coverage.** No status flips; `last_checked` refreshed on epic-10a (OAuth authorization-server/client-registration/token-management).
- **15 PRs merged in 72h** — a dispatcher-driven code-review sweep across ppt-web (6), reality-web (3), reality-server (2), mobile-rn (1), api-server (1), + 2 compliance tests. Average PR is small-focused (one bug, one test), consistent with the research-routine-to-dispatcher pipeline operating as designed.
- **pm-qa rotation came up 67d stale.** Finding this run: amber release-readiness signal — delivery healthy but 15 open code-review-finding items and 2 infra blockers.
- **New backlog cluster:** rotating-review turned up a reality-web i18n gap on 4 authed pages (account/profile/password + realtor/profile) **plus** 15 raw `next/link` imports bypassing the next-intl Link wrapper. Both findings share the same bug class as the just-shipped PR #3203 (listing-edit i18n) — likely one PR could sweep the account cluster.

## Sprint progress

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · **epics_done = 4/4** at sprint level. Coverage holds 13 epics all at done.

| Epic | Sprint status | Coverage status | Shipped PRs this window |
|---|---|---|---|
| 6 — Announcements & Communication | done (6/6) | 6/6 done | — |
| 7A — Basic Document Management | done (5/5) | 5/5 done | — |
| 8A — Basic Notification Preferences | done (3/3) | 3/3 done | — |
| 10A — OAuth Provider Foundation | done (3/3) | 3/3 done · **last_checked 2026-10-31 (cursor re-check)** | — |
| 10B — Platform Administration | done (7/7) | 7/7 done | #3188 (violations xtenant 404) |
| 80 — Dispute Resolution | done (3/3) | 3/3 done | — |
| 9/79/82/83/84/85 (extended) | — | all done | — |

## Shipped since last run (15 PRs)

See `roles/pm-scrum-master.md` for the full list. Highlights:

- **#3188** closes the dev-CI blocker #3187 — `assign_violation` now returns 404 (not 403) cross-tenant.
- **#3203** localises the reality-web listing-edit wizard across all six locales (sk/cs/de/en/pl/hu) — the first material i18n fix in the authed reality-web surface.
- **#3186** wires IoT WebSocket `hasGivenUp` state into the dashboard UI — closes a UX dead-end.
- **#3185** fixes the mount-time-token-bake bug in ppt-web neighbors hooks — stale Bearer after refresh is gone.
- **#3204** adds +171 lines to compliance_wave1b regression coverage — compliance surface is being hardened iteratively.

## What's next (top 5)

1. **medium** — Promote `code-review-reality-web-account-pages-no-i18n` to a plan (sweep profile/password/account + realtor/profile; migrate 15 `next/link` imports to `@/i18n/routing`). _owner: pm-frontend_
2. **medium** — Investigate buffer-low: dispatcher claimable=10/72 while action-list has 30 open. Likely UNRESOLVED-sentinel blockage. _owner: pm-scrum-master_
3. **medium** — Add QA regression tests for the reality-web authed i18n sweep once backend ships messages. _owner: pm-qa_
4. **medium** — Follow up on #3206 ranked-ballot a11y i18n (dead `?? literal` fallback pattern). _owner: pm-frontend_
5. **low** — Close out #3171 (add_evidence manager gate PR, open 1d). _owner: pm-backend_

## Blockers

- **mobile-native/KMP cloud builds (#2652)** — blocks dispatcher claim pool. _owner: pm-devops_
- **React Native jest-expo cloud harness (#2951)** — mobile-rn cloud-verify-blocked. _owner: pm-devops_

## Role focus today

**pm-qa** (rotation) + **pm-scrum-master** (always-on).

### pm-qa summary (one-line)

Amber release-readiness: delivery healthy, 15 open code-review findings + 2 infra blockers. New reality-web authed-pages i18n cluster risk logged (high prob × medium impact).
