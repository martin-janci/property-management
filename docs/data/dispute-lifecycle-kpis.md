# Dispute Lifecycle — KPI Definitions

> Scope: analytics KPIs for the Epic 77 dispute-resolution lifecycle
> (`filed → mediation → resolved` funnel, time-to-resolution percentiles,
> evidence-per-dispute).
> Audience: pm-data / analytics, product owners for the Disputes feature,
> anyone building a disputes dashboard or scheduled metrics job.
> Grounded in code as of branch
> `auto-impl/data-dispute-fsm-kpi-definitions-2026-07-160b1a55`.

This document is a **definitions reference**, not a query cookbook: every KPI is
pinned to the concrete columns that back it so a dashboard, a materialised view,
or a scheduled export computes the same number. SQL sketches are illustrative
(Postgres dialect) and omit RLS / `organization_id` scoping, which every real
query must add.

## Source of truth (schema)

The lifecycle is implemented under Epic 77. The tables and columns the KPIs read
from:

| Table | Migration | Key columns for KPIs |
|-------|-----------|----------------------|
| `disputes` | `00076_create_disputes.sql`, `00160_disputes_mediation_notes.sql` | `id`, `organization_id`, `building_id`, `category`, `priority`, `status`, `filed_by`, `assigned_to`, `created_at`, `resolved_at`, `updated_at` |
| `dispute_activities` | `00076` | `dispute_id`, `activity_type`, `description`, `metadata` (JSONB), `created_at` |
| `dispute_evidence` | `00076` | `dispute_id`, `uploaded_by`, `size_bytes`, `created_at` |
| `mediation_sessions` | `00076` | `dispute_id`, `status`, `scheduled_at`, `duration_minutes` |
| `dispute_resolutions` | `00076` | `dispute_id`, `status`, `proposed_at`, `accepted_at`, `implemented_at` |
| `escalations` | `00076` | `dispute_id`, `resolved`, `created_at`, `resolved_at` |

Status/category/priority constant values are defined in
`backend/crates/db/src/models/disputes.rs` (`dispute_status`, `dispute_category`,
`dispute_priority`, `activity_type`). Analytics must use those literal strings —
do not invent new ones.

## The state machine (FSM)

`disputes.status` is a single VARCHAR holding the **current** state. Valid states
and transitions come from `dispute_state_machine` in `models/disputes.rs`:

```
                withdrawn ◄─────┐
                                │
  filed ──► under_review ──► mediation ──► resolved ──► closed
    │             │  ▲  │         │            ▲
    │             │  │  └────────►│            │
    ├──► awaiting_response ───────┘            │
    │             │                            │
    └──► withdrawn │                           │
                   └──► escalated ─────────────┘
                              └──► closed
```

State reference (`dispute_status::ALL`):

| State | Kind | Meaning |
|-------|------|---------|
| `filed` | entry | Just created; not yet triaged. |
| `under_review` | active | Manager is triaging. |
| `awaiting_response` | active | Waiting on a party. |
| `mediation` | active | In mediation sessions. |
| `escalated` | active | Escalated for non-compliance / severity. |
| `resolved` | terminal-ish | Resolution reached; `resolved_at` set. Can still → `closed`. |
| `withdrawn` | terminal | Complainant withdrew. |
| `closed` | terminal | Archived; no further transitions. |

Allowed transitions (authoritative — mirror of `is_valid_transition`):

| From | To |
|------|----|
| `filed` | `under_review`, `awaiting_response`, `withdrawn` |
| `under_review` | `mediation`, `awaiting_response`, `resolved`, `escalated` |
| `awaiting_response` | `under_review`, `mediation`, `resolved`, `withdrawn` |
| `mediation` | `under_review`, `resolved`, `escalated` |
| `escalated` | `under_review`, `resolved`, `closed` |
| `resolved` | `closed` |
| `withdrawn`, `closed` | — (terminal) |

**Key modelling fact for the funnel:** a dispute does **not** have to pass
through `mediation` to reach `resolved` (`under_review → resolved`,
`awaiting_response → resolved`, and `escalated → resolved` are all valid). So
`mediation` is an *optional* funnel stage, not a mandatory gate. The funnel KPI
therefore measures **stage penetration**, not a strict sequential drop-off.

### Reconstructing stage history (important caveat)

`disputes.status` only holds the latest state. To know whether a now-`closed`
dispute ever *passed through* `mediation`, you need the transition history in
`dispute_activities`.

Every status change writes one row with `activity_type = 'status_changed'`
(constant `activity_type::STATUS_CHANGED`) — see `DisputeRepository::update_status`
and `resolve_dispute` in `repositories/dispute.rs`. **However**, at the time of
writing the transition is recorded only in the human-readable `description`
field, e.g.:

```
Status changed from 'under_review' to 'mediation'
Status changed from 'under_review' to 'mediation': reason text
```

and `metadata` is passed as `None` (no structured `from`/`to` JSON). There are
also dedicated activity types `activity_type::ESCALATED` and
`activity_type::CLOSED`.

Consequences for analytics:

- **"Currently in stage X"** metrics are exact and cheap (`WHERE status = 'X'`).
- **"Ever reached stage X"** metrics require either parsing the `description`
  string or treating a terminal state as evidence a prerequisite state occurred.
  Both are fragile. This is called out again under
  [Open questions](#open-questions--recommendations) as the recommended schema
  fix (a structured `to_status` in `metadata`, or a first-class
  `dispute_status_transitions` table).

Until that fix lands, the funnel KPI below is defined against **two tiers**:
a *robust* definition using only `disputes.status` + `resolved_at`, and an
*enriched* definition that additionally mines `dispute_activities.description`.
Dashboards should prefer the robust tier and label the enriched tier as
best-effort.

## KPI catalogue

Naming convention: `dispute.<area>.<metric>`. Snapshots are computed over a
**cohort** — the set of disputes whose `created_at` falls in the reporting window
`[window_start, window_end)` — unless a KPI states otherwise. All time-window
filters use `created_at` (filing time) so a dispute stays in exactly one cohort.

---

### 1. Lifecycle funnel — `dispute.funnel.*`

**Question:** of the disputes filed in a period, how many reached mediation, and
how many reached resolution?

Because `mediation` is optional (see FSM note), the funnel is reported as three
**stage-penetration** counts plus the conversion ratios between them:

| Metric | Definition |
|--------|------------|
| `dispute.funnel.filed` | Count of disputes in the cohort (denominator). |
| `dispute.funnel.reached_mediation` | Cohort disputes that entered `mediation` at any point. |
| `dispute.funnel.reached_resolved` | Cohort disputes that reached `resolved` (or `closed` via `resolved`). |
| `dispute.funnel.mediation_rate` | `reached_mediation / filed`. |
| `dispute.funnel.resolution_rate` | `reached_resolved / filed`. |
| `dispute.funnel.mediation_to_resolution_rate` | `reached_resolved_via_mediation / reached_mediation` (enriched tier only). |

Supporting terminal-outcome breakdown (mutually exclusive, sums to `filed` once
all cohort disputes are terminal):

| Metric | `disputes.status` |
|--------|-------------------|
| `dispute.funnel.outcome.resolved` | `resolved` or `closed` with `resolved_at IS NOT NULL` |
| `dispute.funnel.outcome.withdrawn` | `withdrawn` |
| `dispute.funnel.outcome.closed_unresolved` | `closed` with `resolved_at IS NULL` |
| `dispute.funnel.outcome.open` | anything not terminal (`filed`, `under_review`, `awaiting_response`, `mediation`, `escalated`) |

**Robust tier** — `reached_resolved` (`mediation` penetration not available
without history):

```sql
SELECT
  count(*)                                                   AS filed,
  count(*) FILTER (WHERE resolved_at IS NOT NULL)            AS reached_resolved,
  count(*) FILTER (WHERE status = 'mediation')               AS currently_in_mediation
FROM disputes
WHERE created_at >= :window_start AND created_at < :window_end;
```

**Enriched tier** — `reached_mediation` via the activity log:

```sql
WITH cohort AS (
  SELECT id FROM disputes
  WHERE created_at >= :window_start AND created_at < :window_end
),
mediation_hits AS (
  SELECT DISTINCT dispute_id
  FROM dispute_activities
  WHERE activity_type = 'status_changed'
    AND description LIKE '%to ''mediation''%'
)
SELECT
  (SELECT count(*) FROM cohort)                                   AS filed,
  (SELECT count(*) FROM cohort c
     JOIN mediation_hits m ON m.dispute_id = c.id)                AS reached_mediation;
```

Recommended dashboard dimensions: `category`, `priority`, `building_id`,
`assigned_to` (mediator/manager). Rates are undefined (report `null`, not `0`)
when the denominator is `0`.

---

### 2. Time-to-resolution (TTR) percentiles — `dispute.ttr.*`

**Question:** how long does a dispute take to resolve, at the median and in the
tail?

**TTR** of a resolved dispute = `resolved_at − created_at`. `resolved_at` is set
exactly once, by `DisputeRepository::resolve_dispute`, when the dispute enters
`resolved`. Only disputes with `resolved_at IS NOT NULL` enter the TTR
population; unresolved / withdrawn disputes are excluded (they have no
resolution duration).

| Metric | Definition |
|--------|------------|
| `dispute.ttr.count` | Number of resolved disputes in the cohort (TTR sample size). |
| `dispute.ttr.p50` | 50th percentile of TTR (median). |
| `dispute.ttr.p90` | 90th percentile of TTR. |
| `dispute.ttr.p95` | 95th percentile of TTR. |
| `dispute.ttr.mean` | Arithmetic mean (report alongside p50 to expose skew; do not use alone). |
| `dispute.ttr.max` | Slowest resolution in the cohort. |

Unit: report in **hours** as the canonical unit (dashboards may render days).
Use continuous percentiles (`percentile_cont`) so interpolation is stable on
small samples.

```sql
SELECT
  count(*)                                                              AS ttr_count,
  percentile_cont(0.50) WITHIN GROUP (ORDER BY resolved_at - created_at) AS p50,
  percentile_cont(0.90) WITHIN GROUP (ORDER BY resolved_at - created_at) AS p90,
  percentile_cont(0.95) WITHIN GROUP (ORDER BY resolved_at - created_at) AS p95,
  avg(resolved_at - created_at)                                          AS mean,
  max(resolved_at - created_at)                                          AS max
FROM disputes
WHERE resolved_at IS NOT NULL
  AND created_at >= :window_start AND created_at < :window_end;
```

Notes and pitfalls:

- **Cohort by `created_at`, not `resolved_at`.** Bucketing by filing date keeps
  the TTR sample aligned with the funnel cohort. Be aware this introduces
  **right-censoring**: recently-filed disputes that are still open are excluded,
  which biases a young cohort's percentiles *downward* (only the fast ones have
  resolved yet). For trend dashboards, only compare cohorts old enough that
  ≥ ~p95 of disputes would have resolved, or annotate the censoring.
- A percentile over `interval` values is well-defined in Postgres; keep the
  subtraction inside `ORDER BY` (do not pre-cast to seconds unless you need a
  numeric type downstream).
- Segment the same percentiles by `priority` (`urgent` disputes are the SLA
  story) and by `category`.

**Stage-latency variant (enriched, optional):** `filed → resolved` can be split
into `filed → mediation` and `mediation → resolved` legs by diffing
`dispute_activities.created_at` for the relevant `status_changed` rows. Same
percentile method, same history caveat as the funnel.

---

### 3. Evidence per dispute — `dispute.evidence.*`

**Question:** how much evidence is attached to a dispute, and how many disputes
have none?

Evidence rows live in `dispute_evidence` (one row per uploaded file). Count is
per `dispute_id`. Disputes with zero evidence contribute a `0` to the
distribution — they must be included via a LEFT JOIN, not dropped by an INNER
JOIN (a common undercount bug).

| Metric | Definition |
|--------|------------|
| `dispute.evidence.mean_per_dispute` | Mean evidence count across cohort disputes (zeros included). |
| `dispute.evidence.p50_per_dispute` | Median evidence count. |
| `dispute.evidence.p90_per_dispute` | 90th percentile evidence count. |
| `dispute.evidence.zero_evidence_rate` | Share of cohort disputes with **no** evidence. |
| `dispute.evidence.total_files` | Total evidence files across the cohort. |
| `dispute.evidence.total_bytes` | Sum of `size_bytes` (storage-footprint signal). |

```sql
WITH per_dispute AS (
  SELECT d.id,
         count(e.id)              AS evidence_count,
         coalesce(sum(e.size_bytes), 0) AS bytes
  FROM disputes d
  LEFT JOIN dispute_evidence e ON e.dispute_id = d.id
  WHERE d.created_at >= :window_start AND d.created_at < :window_end
  GROUP BY d.id
)
SELECT
  avg(evidence_count)                                             AS mean_per_dispute,
  percentile_cont(0.50) WITHIN GROUP (ORDER BY evidence_count)    AS p50_per_dispute,
  percentile_cont(0.90) WITHIN GROUP (ORDER BY evidence_count)    AS p90_per_dispute,
  avg((evidence_count = 0)::int)::float                           AS zero_evidence_rate,
  sum(evidence_count)                                             AS total_files,
  sum(bytes)                                                      AS total_bytes
FROM per_dispute;
```

Recommended cut: split `mean_per_dispute` and `zero_evidence_rate` by
`category` (e.g. `damage` disputes should skew evidence-heavy; a high
`zero_evidence_rate` on `damage` is an actionable data-quality signal) and by
funnel outcome (does more evidence correlate with resolution?).

---

### 4. Supporting operational KPIs (secondary)

These round out the lifecycle picture and reuse the same cohort/source rules.

| Metric | Definition | Source |
|--------|------------|--------|
| `dispute.aging.open_by_stage` | Current open disputes grouped by `status`, with age = `now() − created_at` percentiles per stage. | `disputes` where status not terminal |
| `dispute.rate.withdrawal` | `withdrawn / filed` for the cohort. | `disputes.status` |
| `dispute.rate.escalation` | Cohort disputes that ever hit `escalated` / `filed`. | `dispute_activities` (enriched) or `escalations` |
| `dispute.mediation.sessions_per_disputed` | Mean `mediation_sessions` rows per dispute that reached mediation. | `mediation_sessions` |
| `dispute.mediation.no_show_or_cancel_rate` | Share of `mediation_sessions` with `status` in cancelled/no-show states. | `mediation_sessions.status` |
| `dispute.resolution.acceptance_rate` | `dispute_resolutions` with `status = 'accepted'` / total proposed. | `dispute_resolutions.status` |
| `dispute.enforcement.open_escalations` | `escalations` with `resolved = false`. | `escalations` |

## Computation & operating notes

- **Tenant scoping:** every query above omits `organization_id` for brevity.
  Production analytics MUST scope by `organization_id` (and respect RLS if run
  through the app role rather than an analytics role).
- **Soft deletes:** if the disputes tables participate in the soft-delete RLS
  filter (`00140_rls_soft_delete_filter.sql`), confirm whether analytics should
  count soft-deleted disputes. Default recommendation: **exclude** them from
  operational KPIs.
- **Refresh cadence:** these are reporting metrics, not hot-path. A daily
  materialised view (or a scheduled export) keyed by cohort week/month is
  sufficient; percentiles do not need to be real-time.
- **Null-denominator rule:** every rate returns `null` (not `0`) when its
  denominator is `0`, so an empty cohort is visibly "no data" rather than a
  misleading `0%`.
- **Percentile method:** use `percentile_cont` everywhere for consistency;
  switch to `percentile_disc` only if a metric must return an actually-observed
  value (none of the above require it).

## Open questions & recommendations

1. **Structured status-transition history (highest-value fix).** The funnel and
   stage-latency KPIs are currently best-effort because transitions are only in
   free-text `dispute_activities.description` (`metadata` is `None`). Recommend
   one of: (a) write `{"from": "...", "to": "..."}` into
   `dispute_activities.metadata` on every `status_changed` (small change in
   `DisputeRepository::update_status` / `resolve_dispute`), or (b) add a
   first-class `dispute_status_transitions(dispute_id, from_status, to_status,
   changed_at, changed_by)` table. Either makes "ever reached stage X" exact and
   retires the fragile `LIKE '%to ''mediation''%'` heuristic.
2. **No `mediation`-entered timestamp on `disputes`.** Only `created_at` and
   `resolved_at` are first-class. Once (1) exists, `filed → mediation` latency
   becomes exact; until then it is derived from the activity log.
3. **Right-censoring of TTR by cohort.** Documented above; confirm the dashboard
   either waits for cohorts to mature or annotates in-flight cohorts, to avoid
   reporting artificially fast percentiles for the current month.
4. **Soft-delete inclusion policy** for disputes analytics is unconfirmed (see
   Computation notes) — verify against the RLS soft-delete migration before
   productionising.
5. **Withdrawn/closed-unresolved handling in rates.** The funnel outcome
   breakdown treats `closed` with `resolved_at IS NULL` as *unresolved*; confirm
   this matches how the product defines a "successful" dispute outcome.
