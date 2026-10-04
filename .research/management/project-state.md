# PPT Project State

_Generated: 2026-10-04 — routine Phase 1.6 upkeep (pm-data rotation slot; pm-cursor idx 6 → 7) + pm-scrum-master always-on. Coverage `scan_kind=upkeep`; coverage_cursor idx 8 → 9 (epic-83 to be re-checked next run). 3-day lag since last routine run (2026-10-01)._

## Executive summary

- **Delivery: 49/49 stories done across 13 epics in coverage** (unchanged). Sprint-status.yaml still shows drift (epics 6/7A/80 in-progress at the sprint level, 10B in-progress despite 7/7 story completion). No regressions this window.
- **2 PRs merged** since last run (both land outside the normal PR-cursor advance because PR# ≤ 3015):
  - **#3005** — behaviour-preserving split of `backend/servers/api-server/src/routes/integrations/webhook.rs` (2799 → 77 lines) into 6 per-receiver submodules. Dispatcher-spawned retry 1/2; carries `post-merge-reviewed` + `needs-human-review`.
  - **#2744** — dispatcher archive-push un-wedging + retry-remint cross-id ghost fix (closes #2743). Research tooling only.
- **No new blockers, no new noteworthy issues.** All four infra blockers (#2966 swagger-ui egress, #2652 KMP cloud builds, #2951 jest-expo rot, and the IDOR retry parking lot behind #2966) carry forward unchanged.
- **Buffer-low condition persists.** The scrum-master read remains: dispatcher stack is reviewer-starved on the accounting trio and infra-starved on #2966. PR #3005 landing confirms the refactor-retry path works; it does **not** change the #2966 verdict (PR #3005's description leaves the egress block flagged as still open).
- **pm-data rotating-role refresh:** carried-forward analytics gaps still apply — Epic 6/10A/10B/80/84 remain uninstrumented; FaultStatusCount dual-definition decision is 70+ days unresolved; `support_tooling_events` retention policy still unpublished.

## Sprint progress

Current sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** · **epics_done = 2/4** at sprint-row level (sprint-status.yaml); coverage-level all extended epics are complete. (Sprint-status.yaml records 6 epic rows; 2 are strictly `done` — 8A and 10A; 10B is `in-progress` despite 7/7 story completion; 6, 7A, 80 are `in-progress`/`partial` though coverage says all stories done.)

| Epic | Sprint status | Coverage status | Shipped PRs this window |
|---|---|---|---|
| 6 — Announcements & Communication | in-progress (3/6)† | 6/6 done | — |
| 7A — Basic Document Management | in-progress (2/5)† | 5/5 done | — |
| 8A — Basic Notification Preferences | done (3/3) | 3/3 done | — |
| 10A — OAuth Provider Foundation | done (3/3) | 3/3 done | — |
| 10B — Platform Administration | in-progress (7/7)† | 7/7 done | — |
| 80 — Dispute Resolution | partial (1/3)† | 3/3 done | — |
| 79 — API client integration | (extended) | 4/4 done | — |
| Infra / tooling | n/a | n/a | #3005 (webhook.rs split), #2744 (dispatcher archive-push un-wedge) |

† sprint-status.yaml drift against coverage.json — no action required this run; deep scan will reconcile.

## Shipped since last run (2 PRs merged — 2026-10-03)

- **#3005** — `refactor-repeated-churn-integrations-webhook-retry1`: pure-move split of `routes/integrations/webhook.rs` into `webhook/subscriptions.rs` + 5 per-receiver submodules. Was retry 1/2 of a failed refactor; the retry succeeded. Churn-hotspot pressure on the file is now relieved (post-split: 77 lines in the shim; 6 small modules replace the 2799-line hot file). Carries `needs-human-review`.
- **#2744** — `fix(dispatcher): un-wedge oversize-archive push (#1162) + close retry-remint cross-id ghost (#2460)`: `.research/` scripts + prompt logic + tests only; no runtime state files touched. Closes #2743. Carries `needs-human-review`.

## Open PR landscape (24 total)

- **Non-dependabot human-authored open:**
  - **#2969** — needs-human-review, not landed; aging.
  - **#2967** — draft.
  - **#2902** — draft (long-stalled).
  - **#2559** — accounting MVP: PAY by Square slice; **64+ days** reviewer-starved.
  - **#2555** — accounting MVP: invoice lifecycle slice; **64+ days** reviewer-starved.
- **Dispatcher review-loop PRs** — rolling, normal cadence.
- **Dependabot** — bulk, unchanged, auto-merge via `auto-approve.yml`.

## Blockers (carried, none new)

- **#2966** — api-server utoipa-swagger-ui egress 403 (top blocker; still open; PR #3005's description reiterates it is unresolved). Owner: pm-devops.
- **#2951** — mobile RN jest-expo version rot. Owner: pm-devops.
- **#2946 / #2944 / #2945** — cross-tenant IDOR retries parked behind #2966. Owner: pm-security.
- **#2652** — mobile-native/KMP builds unlandable in cloud runner. Owner: pm-devops.

## Churn hotspot — post-refactor update

- `backend/servers/api-server/src/routes/integrations/webhook.rs` dropped from 2799 → 77 lines after PR #3005. Successor files are the six `webhook/*.rs` submodules; the next churn window will measure them individually, which is the design intent.
- **Watch item (new):** validate that the next 14-day churn window doesn't re-concentrate inside one of the six submodules (would indicate the split missed the real churn vector). Owner: pm-tech-lead; action-list item added.

## Role analysis — pm-data (2026-10-04)

Rotating role, 73 days since last run. Full detail at `.research/management/roles/pm-data.md`. Summary of this run:

**What changed since 2026-07-23:** Nothing material from the pm-data lens. Of the 70+ PRs merged in the intervening window, zero added KPI/analytics event emission, event-schema definitions, or lifecycle policies for `support_tooling_events`. Every carried pm-data action remains open; the gap list has only grown as more features (meter OCR, voice/webhooks, workflow-automation, reality-web /sell + /report, portfolio_analytics) have shipped without instrumentation.

**Risks (unchanged from 2026-07-23):** The 5 pm-data risks from the previous cycle remain in `risks.json` — this run adds **one new risk** on `portfolio_analytics` metric correctness given the still-open IDOR retry parking lot (the computed-then-discarded `org_id` means the metric itself may already be wrong on main even before the auth fix lands) and **one** on mobile-rn/KMP event-tracking parity (now 50% of traffic plausibly blind, 73 days older than the previous call).

**Next pm-data actions (6):** carried forward with sequencing re-confirmed given no delivery against them since July — see `roles/pm-data.md` and the merged rows in `action-list.json`.

## Decisions needed (new this run)

- **(2026-10-04, pm-scrum-master):** PR #3005 and PR #2744 both carry `needs-human-review`. Define the SLA for `needs-human-review` label clearance — is 48h the ceiling, or do these sit indefinitely? Both PRs are already merged, so this is about audit-trail hygiene, not blocking anything. Owner: pm-tech-lead.
- **(2026-10-04, pm-data):** The 2026-07-23 Minimum-Analytics-DoD decision is still open 73 days later. Either promote to binding gate (every new story must define a business event + audit event + KPI dashboard link) or formally drop. Treating it as advisory indefinitely is the worst of both worlds. Owner: pm-scrum-master + pm-data.
- **(2026-10-04, pm-data):** Analytics-platform choice (bespoke vs Amplitude/PostHog/Segment) remains unmade and now blocks any investment in the KPI backfill — teams can't instrument until the destination is picked. Owner: pm-tech-lead + pm-data.

## Coverage (upkeep this run — 2026-10-04)

- **`coverage.json` not rewritten this run** — incremental, not scan. `coverage_cursor` advances 8 → 9 (next re-check will target the epic at the new cursor position).
- **`pm_cursor` advances 6 → 7** (pm-data → pm-integration next run). `role_last_run["pm-data"] = 2026-10-04`; `role_last_run["pm-scrum-master"] = 2026-10-04`.
- **Composition unchanged:** 49 done · 0 partial · 0 not-started across 13 epics.
