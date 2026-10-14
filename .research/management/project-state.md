# PPT Project State

_Generated: 2026-10-14 — routine Phase 1.6 upkeep (pm-data rotation slot; pm-cursor idx 6 → 7) + pm-scrum-master always-on. Coverage `scan_kind=upkeep`; coverage_cursor idx 8 → 9 (epic-83 re-checked, no material change). 6-day lag since last routine run (2026-10-08)._

## Executive summary

- **Delivery: 49/49 stories done across 13 epics in coverage.** No status flips this window; the window shipped quality + bugfix work on top of an already-done feature set.
- **26 PRs merged** since the 2026-10-08 cursor (PR#3043..#3080) — the broadest window in weeks. Themes: (a) reality-web /sell + /report funnel hardening (#3044 /#3046 /#3056 /#3058 /#3064 /#3066 /#3067); (b) ppt-web community/groups/documents/rentals mutation-error surfacing (#3045 /#3048 /#3049 /#3050); (c) screen-map drift reconciliations (#3053 /#3054 /#3055 /#3058 /#3059); (d) churn-hotspot test coverage + small refactors (#3060..#3063 /#3065 /#3068); (e) auth-fetch triplication finally folded into `bearerAuthorization` (#3071 — closes the #3006 risk); (f) infra ENOSPC fix for the dispatcher (#3080 — closes #3073).
- **No coverage status flips.** All 49 stories still `done`; `last_checked` stamped on all three epic-83 stories (cursor upkeep slot).
- **New open issues to watch**: #3076 (ammonia / RUSTSEC-2021-0157 — blocks all backend PRs — open PR #3077 still draft `needs-human-review`), #3078 (portfolio-analytics property-metrics 500 schema drift — open PR #3081 draft), #3079 (SSRF IPv4-compatible IPv6 bypass — open PR #3082 draft), #3069 (ViolationRepository::list_notifications cross-tenant IDOR — open PR #3070 draft).
- **Role focus today: pm-data** — flagged a live data-health incident on portfolio-analytics (#3078) and a complete absence of event tracking on the reality-web /sell + /report funnels PR#3044..#3067 just hardened.

## Sprint progress

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · **epics_done = 3/5** at sprint level (sprint-status.yaml); coverage-level all 13 epics are complete.

| Epic | Sprint status | Coverage status | Shipped PRs this window |
|---|---|---|---|
| 6 — Announcements & Communication | in-progress (3/6)† | 6/6 done | — |
| 7A — Basic Document Management | in-progress (2/5)† | 5/5 done | #3049 (gate documents routes on organizationId) |
| 8A — Basic Notification Preferences | done (3/3) | 3/3 done | — |
| 10A — OAuth Provider Foundation | done (3/3) | 3/3 done | — |
| 10B — Platform Administration | in-progress (7/7)† | 7/7 done | — |
| 80 — Dispute Resolution | partial (1/3)† | 3/3 done | — |
| 79 — API client integration | (extended) | 4/4 done | #3071 (auth-fetch folded into shared bearerAuthorization — closes #3006) |
| 83 — Portal Integrations | (extended) | 3/3 done · **last_checked 2026-10-14 (cursor re-check)** | — |
| Cross-cutting | — | — | #3080 (dispatcher ENOSPC: share one Cargo target across worktrees) |

† sprint-status.yaml drift against coverage.json — no action required this run; deep scan will reconcile.

## Shipped since last run (26 PRs merged since cursor)

**Reality-web /sell + /report + /profile funnel (8)**: #3044 Publish 405 fix · #3046 reports_submit sitemap + id surfacing · #3047 /profile i18n via next-intl · #3056 /sell area→sizeSqm + postalCode (closes #3052) · #3058 /report screen-map reconcile · #3064 SyncSchedule edit-form seed · #3066 /sell publish wiring harden · #3067 /report submit wiring + tests.

**ppt-web community + documents + rentals + leases + groups (6)**: #3045 group-not-found reuses errors.groupNotFound/common.back · #3048 community group join/leave mutation-errors surfaced · #3049 documents routes gated on organizationId · #3050 rentals mutations surface non-auth errors · #3051 lease currency from config (not hardcoded EUR) · #3068 community group-detail route-coverage.

**Screen-map drift reconciliations (5)**: #3053 /sell · #3054 community-group-detail · #3055 /profile · #3058 /report · #3059 toast route-group i18n.

**Churn-hotspot test + refactor (5)**: #3060 CreateRulePage mutation-fail test · #3061 dashboardPathForRole helper extract · #3062 CategoryFilter type alias · #3063 FeaturedListings error-surface tests · #3065 realtor-api id encoding + test coverage.

**Cross-cutting (3)**: #3043 SyncSchedule save-error banner reset · #3071 last auth-header fold into shared bearerAuthorization (closes #3006) · #3080 dispatcher ENOSPC: one shared Cargo target across worktrees (closes #3073).

## What's next (top 5 from the ranked plan)

1. **[high] Unblock backend CI by landing #3077** (`ammonia` → 4.1.7 clearing RUSTSEC-2021-0157) — gates every backend-touching PR including all open security retries. Owner: pm-security / pm-devops.
2. **[high] Land #3081 (portfolio-analytics property-metrics 500 — schema drift + IDOR regression tests)** — live data-health incident on realtor dashboards. Owner: pm-tech-lead.
3. **[high] Land #3082 (SSRF IPv4-compatible IPv6 bypass)** — security retry open as `gh-issue-3079`. Owner: pm-security.
4. **[high] Instrument reality-web /sell + /report funnels** — PRs #3044..#3067 hardened them but zero `trackEvent` calls were added; the trackEvent bus already exists. Owner: pm-data (spec) + pm-frontend (wire).
5. **[high] Land #3070 (ViolationRepository::list_notifications org scoping)** — same IDOR class as #2944 — open as `gh-issue-3069`. Owner: pm-tech-lead.

## Open PR landscape (17 open)

- **Security retries open (6 drafts, all `needs-human-review`)**: #3077 ammonia bump · #3070 ViolationRepository scope · #3072 violations role authz · #3074 record_payment atomic · #3075 violation numbering race · #3081 portfolio-analytics + IDOR · #3082 SSRF IPv6. Reviewer bandwidth is the gating factor.
- **Dependabot bulk (8)**: #2957 tower-http · #2961 kotlin group · #2962 compose-bom · #2972 ktor group · #2981 cargo-minor-patch bulk · #3007 gradle-minor-patch · #3037 tokio patch · #3038 npm-minor-patch bulk.
- **Older stalled (3)**: #3024 ppt-web dashboard screen-map reconcile · #3028 add_to_compare TOCTOU.

## Blockers (carried; one new)

- **#3076 — ammonia / RUSTSEC-2021-0157** (NEW): blocks every backend PR until #3077 merges. Owner: pm-security + pm-devops.
- **#2966** — api-server utoipa-swagger-ui egress 403: still open. Owner: pm-devops.
- **#2951** — mobile RN jest-expo version rot. Owner: pm-devops.
- **#2652** — mobile-native/KMP builds unlandable in cloud runner. Owner: pm-devops.

## Noteworthy resolved this window

- **#3006** (triplicated auth-header + MFA-retry drift) — resolved by PR #3071 folding the last call site into shared `bearerAuthorization`. One carried-risk retires.
- **#3073** (dispatcher ENOSPC on verify) — resolved by PR #3080 sharing one Cargo target across worktrees.
- **#3052** (/sell wizard payload mismatch) — resolved by PR #3056.
- **#3042** (reports sitemap registration + id surface) — resolved by PR #3046.

## Role analysis — pm-data (2026-10-14)

**Top 3 findings**

1. **Live data-health incident, under-prioritised**: portfolio-analytics `property-metrics` endpoint is returning 500s in production (#3078 / open PR #3081, schema drift). Realtor dashboards are currently rendering partial/erroring data — this is a data-health incident, not just a code bug. Treat it as a backfill + alert exercise once the fix lands.
2. **Zero funnel instrumentation on the two flows we just hardened**: PRs #3044 /#3046 /#3056 /#3064 /#3066 /#3067 fixed the reality-web /sell + /report happy-path. Grepping `trackEvent` in `apps/reality-web/src/app/[locale]/sell/page.tsx` + `.../report/page.tsx` finds **zero** calls. The `lib/analytics.ts` bus exists and is used elsewhere; wiring these funnels costs ≤ a day. Without it, we cannot measure whether the fixes actually improved conversion.
3. **Metric-definition drift reinforced**: PR #3051 fixed one hardcoded EUR currency on leases but at least two sibling call sites still compute revenue without an explicit currency key. FaultStatusCount drift (carried risk from 2026-05-28) remains open. "Revenue" and "fault" numbers across admin + portfolio dashboards still don't reconcile.

See `roles/pm-data.md` for the full return + 6 next_actions.
