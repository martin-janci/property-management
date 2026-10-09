# PPT Project State

_Generated: 2026-10-09 — routine Phase 1.6 upkeep (pm-data rotation slot; pm-cursor idx 6 → 7) + pm-scrum-master always-on. Coverage `scan_kind=upkeep`; coverage_cursor idx 8 → 9 (epic-83 re-checked, no material change; advances to epic-84). 1-day lag since last routine run (2026-10-08 Phase 1 data)._

## Executive summary

- **Delivery: 49/49 stories done across 13 epics in coverage.** Sprint-status.yaml still shows some epics in-progress/partial (6, 7A, 10B, 80) — carried sprint-level drift against coverage.json truth; no new regressions.
- **6 PRs merged** in the 2026-10-08 bugfix dispatcher wave: #3043 (SyncSchedule banner reset), #3044 (/sell Publish 405 fix), #3045 (group-detail not-found i18n reuse), #3046 (/report sitemap + success id), #3047 (reality-web /profile next-intl locale), #3048 (community group join/leave error surface). All closed `gh-issue-30**` follow-ups (#3029, #3039-#3042). Zero coverage status flips (all surfaces already shipped).
- **New analytics-debt signal (pm-data run):** the 2026-10-08 wave shows our bug detection is reviewer-driven not metric-driven — e.g. #3044 /sell publish 405 was a user-visible funnel defect that silently blocked every listing publish for an unknown duration because no publish-success KPI exists. Same shape across /report submit, community-group join/leave, and SyncSchedule flows. Funnel-event schema is now the top data-track lever.
- **Infra blockers unchanged (carried):** #2966 utoipa-swagger-ui egress, #2652 KMP cloud build, #2951 jest-expo rot — all still gate the dispatcher queue (and the three IDOR retries).
- **Action-list buffer: 42 open / 36 floor** — above target (+6 pm-data rows this run). No buffer-low warning.

## Sprint progress

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · **epics_done = 3/5** at sprint level (sprint-status.yaml); coverage-level all extended epics are complete.

| Epic | Sprint status | Coverage status | Shipped PRs this window |
|---|---|---|---|
| 6 — Announcements & Communication | in-progress (3/6)† | 6/6 done | — |
| 7A — Basic Document Management | in-progress (2/5)† | 5/5 done | — |
| 8A — Basic Notification Preferences | done (3/3) | 3/3 done | — |
| 10A — OAuth Provider Foundation | done (3/3) | 3/3 done | — |
| 10B — Platform Administration | in-progress (7/7)† | 7/7 done | — |
| 80 — Dispute Resolution | partial (1/3)† | 3/3 done | — |
| 82 — iOS Reality Portal (SwiftUI) | (extended) | 5/5 done | #3047 (reality-web /profile locale — adjacent to 82-5 inquiries/account) |
| 83 — Portal Integrations | (extended) | 3/3 done · **last_checked 2026-10-09 (cursor re-check)** | #3043 (SyncSchedule banner — adjacent realtor-import UX) |
| Cross-cutting (reality-web + ppt-web bugfix wave) | — | — | #3044 (/sell Publish 405), #3045 (group-detail i18n), #3046 (/report sitemap + id), #3048 (community group join/leave error surface) |

† sprint-status.yaml drift against coverage.json — no action required this run; deep scan will reconcile.

## Shipped since last run (6 PRs merged 2026-10-08)

- **#3043** — SyncSchedule save-error banner reset on re-edit (realtor-import flow) — closes gh-issue-3029
- **#3044** — /sell wizard Publish 405 fix (createListing() POSTs to GET-only route) — closes gh-issue-3039
- **#3045** — group-detail not-found reuses errors.groupNotFound/common.back (i18n reuse) — closes gh-issue-3040
- **#3046** — register reports_submit in @ppt/sitemap + surface report id on /report success — closes gh-issue-3042
- **#3047** — route reality-web /profile copy through next-intl (fixes hardcoded Slovak in a 4-locale app) — closes gh-issue-3041
- **#3048** — surface community group join/leave mutation errors (ppt-web routes/groups/community.tsx)

## What's next (top 5 from the roadmap)

1. [high] **#2966 utoipa-swagger-ui egress unblock** — gates every backend PR including 3 IDOR retries — owner: **pm-devops** (carried)
2. [high] **Analytics platform decision** (bespoke pg vs vendor) — 78d open; blocks every emission wiring — owner: **pm-tech-lead** (new this run)
3. [high] **Define cross-platform funnel_step + funnel_fail schema** across reality-web + ppt-web + mobile — would make the next /sell-publish-class defect metric-visible — owner: **pm-data** (new this run)
4. [high] **#2946 portfolio_analytics IDOR fix (retry 2/2)** — one-line fix; blocked only by #2966 cloud build — owner: **pm-security** (carried)
5. [high] **Land the `analytics-schema-foundations-2026-10-09` meta-task** collapsing the 6 open 2026-07-23 pm-data items — owner: **pm-data** (new this run)

## Blockers (carried, none new)

- **#2966** — api-server utoipa-swagger-ui egress 403 (top blocker). Owner: pm-devops.
- **#2951** — mobile RN jest-expo version rot. Blocks mobile-rn code-review items from landing. Owner: pm-devops.
- **#2946** — portfolio_analytics cross-tenant IDOR (retry 2/2 draft open; blocked by #2966). Owner: pm-security.
- **#2652** — mobile-native/KMP builds unlandable in cloud runner. Owner: pm-devops.

## Role focus today: pm-data (rotation idx 6 → 7)

**Headline:** 2026-10-08 bugfix wave reinforces that our bug detection is reviewer-driven, not metric-driven. The /sell publish-405 defect silently blocked every listing publish for an unknown duration because no `listing_publish_success` KPI exists. Same funnel-instrumentation gap across /report, community-groups, SyncSchedule.

**Carried pain:** 0 of 6 pm-data action-list items from 2026-07-23 have been promoted in 78 days — ranker-fairness issue where structural `medium` items from always-structural roles never win a slot. This run's fix: collapse the six into a single `high`-scored meta-task (`analytics-schema-foundations-2026-10-09`) so the dispatcher can claim them as a batch.

**Six new rows added to action-list.json:**
1. `pm-data-funnel-event-schema-2026-10-09` (high) — cross-platform funnel_step + funnel_fail schema
2. `pm-data-analytics-schema-foundations-meta-2026-10-09` (high) — meta-collapse of 6 open items
3. `pm-data-screen-map-instrumentation-status-2026-10-09` (medium) — new frontmatter field + backfill 10
4. `pm-data-analytics-platform-decision-2026-10-09` (high) — force a DEC entry
5. `pm-data-support-tooling-retention-policy-2026-10-09` (medium) — carried retention gap
6. `pm-data-mobile-event-parity-audit-2026-10-09` (medium) — baseline before new instrumentation

**Four new risks added to risks.json:**
- `pm-data-funnel-instrumentation-gap-2026-10-09` (high/medium)
- `pm-data-analytics-platform-decision-stale-2026-10-09` (high/medium)
- `pm-data-pm-ranker-fairness-2026-10-09` (medium/medium)
- `pm-data-screen-map-instrumentation-blindspot-2026-10-09` (medium/medium)

**Four decisions appended to decisions.md** — analytics platform (force DEC), collapse-into-meta pattern (manual vs ranker rule), screen-map `instrumentationStatus` schema, and reviewer-driven vs metric-driven detection pattern.

## Coverage (upkeep this run — 2026-10-09)

- **`coverage.json` refreshed via mechanical upkeep** — `scan_kind=upkeep`, `generated=2026-10-09T04:30:00Z`, no re-scan.
- **Epic re-check: epic-83 (cursor idx 8)** — all 3 stories (83-1/2/3) still `done`; no PR in the 2026-10-02..10-09 window touched the booking/airbnb adapters or portal-webhook surfaces; PR #3043 noted on 83-1 (SyncSchedule UX adjacent to realtor-import). `last_checked = 2026-10-09` stamped on all 3.
- **Merged-PR evidence appended** on 82-5-inquiries-account (PR #3047 /profile locale adjacent) and 83-1-airbnb-integration (PR #3043 SyncSchedule banner adjacent). No status flips.
- **`coverage_cursor` advances 8 → 9** (epic-83 → epic-84 next run).
- **`pm_cursor` advances 6 → 7** (pm-data → pm-integration in rotation). role_last_run["pm-data"] = 2026-10-09.
- **Composition unchanged: 49 done · 0 partial · 0 not-started** across 13 epics. Missing UC links: 3 (UC-33.1/33.2/33.3, already queued).
