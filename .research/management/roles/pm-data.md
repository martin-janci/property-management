# Role: pm-data — 2026-10-04

> Data/analytics lens. Rotating role this run (pm_cursor rotation[6]). Static read-only. Previous run: 2026-07-23 (73 days ago). Incremental refresh — not a scan.

**Summary:** No material delivery against pm-data recommendations since the 2026-07-23 pass. Of ~70 PRs merged in the intervening window, zero added event emission, event-schema definitions, or lifecycle policies for `support_tooling_events`. Every previously-identified gap (Epic 6/10A/10B/80/84 uninstrumented, FaultStatusCount dual-definition, retention policy unpublished, mobile analytics parity unknown) remains open; the gap has grown as more features (meter OCR, voice/webhooks, workflow-automation, reality-web `/sell` + `/report`, portfolio_analytics) have shipped without instrumentation. One new concern surfaces this run: `portfolio_analytics` is both a security IDOR (#2946) **and** a metric-correctness defect — the computed-then-discarded `org_id` can corrupt metric values independently of the auth leak, and the correctness check does not land with the IDOR fix.

## What changed since 2026-07-23 (concrete, not speculative)

- **PR #3005** (webhook.rs split into 6 submodules) — zero analytics change; pure refactor.
- **PR #2744** (dispatcher archive-push) — infra tooling only.
- No PR in the window added a `tracker.track(...)` emit, an `audit_write` to `support_tooling_events`, or a Grafana/metrics schema update visible to a grep across `backend/crates/` or `frontend/apps/`.
- The three new reality-web features shipped since 07-23 (`/sell` wizard, `/report` page, PriceAlerts mark-read) all currently have user-visible defects on the action-list and none emit analytics events — these are the next tracking candidates when the platform decision lands.

## Next actions (6)

| Action | Priority | Dependency | Definition of done |
|---|---|---|---|
| Pick the analytics platform (bespoke Postgres+Grafana vs Amplitude vs PostHog vs Segment) — strict upstream of every KPI backfill | high | pm-tech-lead | platform selected; ADR merged into `docs/`; downstream tasks unblocked |
| Verify `portfolio_analytics` metric correctness independent of the #2946 auth fix — the computed-then-discarded `org_id` can produce commingled numbers on main even before the IDOR is exploited | medium | pm-security (#2946) | sqlx integration test on seed data asserts per-org cardinality; metric values match expected |
| Audit mobile (RN + KMP) analytics event parity vs web for Epic 6/10A/10B/80/84 — carried from 2026-07-23, 73 days older | medium | none | parity matrix doc; funnel-blindspot list with ownership |
| Force decision on Minimum-Analytics DoD (binding gate vs formally drop) — 73-day-old proposal | medium | pm-scrum-master | decision merged into `decisions.md`; sprint-status gate updated or proposal retired |
| Define layout publish/webhook analytics events (`published_by`, `layout_version`, `target_tenant_count`) — carried | medium | analytics-platform choice | event schema + emission wired in `publish_layout` handler; documented in support-data catalogue |
| Define dispute-lifecycle KPI set (filed → mediation → resolved funnel, TTR p50/p95, evidence-per-dispute) — carried | medium | analytics-platform choice | metric definitions + counters + percentiles emitted; shared taxonomy with owner/portfolio KPIs |

## Risks (new or materially aged this run)

| Risk | Probability | Impact | Mitigation |
|---|---|---|---|
| `portfolio_analytics` is both a security IDOR AND a metric-correctness defect; metric values may already be wrong on main even without an attacker | medium | medium | Independent cross-tenant comparison on seed data; sqlx integration test as part of #2946 fix PR |
| Mobile analytics parity still unknown 73 days after the first call; funnels for 5 shipped MVP epics likely blind on ~50% of traffic | medium | medium | Mobile-analytics parity audit (sequenced); shared event taxonomy before new mobile tracking is written |
| No analytics-platform choice made; every KPI backfill action is structurally blocked by this one upstream decision | high | medium | Force the decision this cycle; unblock the 4 downstream KPI tasks |
| Carried: FaultStatusCount dual-definition drift vs owner/portfolio fault KPIs (dashboards will disagree) | high | medium | Land single-source metric definitions in shared module; deprecate duplicates |
| Carried: `support_tooling_events` + audit trail have no TTL / retention policy (GDPR + long-term storage risk) | medium | medium | Publish retention policy; add lifecycle jobs for PII-carrying tables |

## Open questions

- Which analytics platform does PPT target — bespoke Postgres+Grafana, Amplitude, PostHog, Segment? (Blocks every downstream backfill.)
- Is there a DPIA on `support_tooling_events` (support staff reading tenant data)? Carried from 2026-07-23.
- Are dispute-lifecycle KPIs required by customer contract / regulatory obligation, or purely product-internal?
- Do webhook events (booking / airbnb / esignature / layout) emit analytics on delivery + retry + failure, or only log? Carried.
- Are seed-data recipes stable enough to reason about analytics test fidelity? The missing-seed-recipe gap in `ppt-db-migrations` suggests not.

## Decisions needed

- Analytics-platform choice (bespoke vs Amplitude/PostHog/Segment) — owner: pm-tech-lead + pm-data. **Blocking.**
- Minimum-Analytics DoD: binding gate vs formally dropped — owner: pm-scrum-master + pm-data. **Decision-force this cycle.**
- GDPR / retention policy for `support_tooling_events` (TTL vs indefinite) — owner: pm-security + pm-data. Carried.
- FaultStatusCount canonical definition — owner: pm-data. Carried from 2026-05-28.
