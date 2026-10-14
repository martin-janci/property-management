# pm-data — 2026-10-14 rotation slot

> Role: Data / analytics engineer lens. Static read only.
> Pm-cursor rotation idx 6 → 7. Previous run: 2026-07-23 (82-day gap).

## Summary

The 26-PR observation window hardened the public portal /sell + /report + /profile funnels and the ppt-web community/documents/rentals mutation paths — but **not one of those PRs added a single `trackEvent` call**, even though `reality-web/src/lib/analytics.ts` and `ppt-web/src/lib/analytics.ts` already provide a transport-agnostic event bus. Analytics blindspots on shipped MVP features (risk `risk-analytics-blindspots-shipped-mvp-2026-07-23`) are now compounded by a data-health incident on the portfolio-analytics endpoint (#3078 / open PR #3081: schema drift causing live 500s on `property-metrics`), meaning realtor dashboards are currently returning partial/erroring data in production until #3081 lands.

## Next actions

| # | Action | Priority | Dependency | Definition of done |
|---|---|---|---|---|
| 1 | Instrument the reality-web **/sell wizard funnel** — emit `listing.sell_wizard.step_shown`, `listing.sell_wizard.step_completed`, `listing.sell_wizard.publish_submitted`, `listing.sell_wizard.publish_succeeded`, `listing.sell_wizard.publish_failed` with the server-side error class (405/422/5xx) from PRs #3044 / #3056 / #3066. Owner: pm-data (spec) + pm-frontend (wire). | high | none | 5 events emitted, dataLayer contract documented in `docs/analytics/reality-sell-funnel.md`, unit test asserts no PII in properties |
| 2 | Instrument the reality-web **/report submission flow** — emit `report.form_shown`, `report.submitted`, `report.submit_succeeded` (with returned report id from PR #3046), `report.submit_failed` (with error class from PR #3067). | high | none | 4 events emitted, dataLayer contract documented, screen-map `reality/report` agent-log entry linking to the spec |
| 3 | Treat **portfolio-analytics property-metrics 500 (#3078 / PR #3081)** as a data-health incident, not just a code bug — add a dashboard alert on `realtor.portfolio_metrics.5xx_rate > 1%` and a backfill plan for any dashboards that silently rendered zeros during the drift window. | high | pm-tech-lead (PR #3081 land) | Alert configured; retro note in `decisions.md`; `data_health_check` row in action-list for monthly schema-drift audit |
| 4 | Define single-source-of-truth **currency metric** after PR #3051 fixed hardcoded EUR on leases — publish `lease.currency` + `payment.currency` dimensions in the shared metric catalog and deprecate any dashboard that computed revenue without a currency key. | medium | pm-frontend | `docs/analytics/metrics.md` lists the canonical currency keys; 1 duplicate currency series removed from a reporting dashboard |
| 5 | Add a **mutation-error funnel** cross-cutting event (`client.mutation.error_surfaced`) on the ppt-web mutations that PRs #3048 / #3049 / #3050 now surface. Without it, we see error toasts fire but have no funnel for "mutation attempted → mutation surfaced error → user retried / abandoned". | medium | pm-frontend | 1 shared helper + 3 call sites (community group join/leave, documents, rentals); downstream dashboard tile wired in realtor-analytics |
| 6 | Author **minimum-analytics DoD** for all new stories (even internal admin screens) — one page: event name convention, required properties, PII rules, screen-map agent-log stamp. Stop shipping MVP surfaces without instrumentation. | medium | pm-tech-lead | `docs/analytics/story-dod.md` merged; added as checklist item to `.github/pull_request_template.md` |

## Risks

| # | Risk | Prob | Impact | Mitigation |
|---|---|---|---|---|
| 1 | **Live data-health incident**: portfolio-analytics property-metrics 500 (schema drift, open PR #3081) silently returns errors to realtor dashboards — realtors making portfolio decisions from an empty / partial view. | high | high | Expedite #3081 review; backfill-time communication to realtors; add schema-drift CI gate (compare `sqlx` prepared queries against live schema). |
| 2 | **Reality-portal funnel blindspot**: PRs #3044 / #3056 / #3064 / #3066 / #3067 hardened the /sell + /report happy-path but we still cannot measure success rate, drop-off step, or error-class distribution on either funnel — the whole point of the fixes (did we actually improve conversion?) is unmeasurable. | high | medium | Ship actions #1 and #2 in the same sprint as the next /sell or /report change. |
| 3 | **Metric-definition drift (carried, now reinforced)**: PR #3051 fixed one hardcoded EUR currency in leases. Grepping shows at least two other call sites still compute revenue with implicit currency. FaultStatusCount metric already drifts between support-data and portfolio dashboards. Any shared "revenue" or "fault" number across the admin surface is suspect. | high | medium | Action #4 catalog + delete-after-migrate on the duplicates. |
| 4 | **PII leakage via unreviewed trackEvent properties**: the current analytics bus accepts an arbitrary `Record<string, unknown>` — no schema, no PII scrubber. Once we start wiring events from the /sell wizard (which carries postalCode, address fragments), easy to leak PII into the dataLayer. | medium | high | Action #6 DoD must require a reviewed property-list; add a Biome rule / codemod that forbids spreading request bodies into trackEvent properties. |
| 5 | **Data-retention policy still unpublished (carried)**: `support_tooling_events` + support-data audit trail are append-only with no TTL; GDPR surface area grows every week. | medium | medium | Publish retention policy; action in last pm-data run still open — needs promotion to committed. |

## Open questions

- Which deploy-time analytics tag manager (PostHog / Segment / GTM / nothing) is actually injected into reality-web in staging + prod? The code says "injected at deploy time, if at all" — we need to know "at all" before writing event specs.
- Is there a shared metric-catalog repo / page outside this codebase? If not, action #4 and #6 both create it in `docs/analytics/`.
- Is portfolio-analytics property-metrics the only endpoint on the dashboard with live schema drift, or should #3081 trigger a sweep of every `sqlx` prepared query that materialises into a realtor dashboard?

## Decisions needed

- **Pick a canonical analytics transport** (PostHog self-hosted vs Segment vs GTM-only) — owner: pm-tech-lead + pm-devops. Without this, the trackEvent calls land but no one knows where the events go.
- **Classify portfolio-analytics #3081 as a data-health incident vs a code bug** — owner: pm-tech-lead. Determines whether we backfill dashboards or just land the PR.
