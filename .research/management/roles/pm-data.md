# pm-data

<sub>Generated: 2026-11-17 — rotating role analysis for the research routine.</sub>

## Summary

Static check only. ppt-web has a `trackEvent` / `registerAnalyticsSink` layer and admin-web has onboarding analytics, but I did not verify that moderation routing (PR #3562) or mobile ReportFault (PR #3567) emit events, and mobile has no obvious analytics layer. The three Phase 1.5 digest findings are reporting-correctness defects: an INNER JOIN cohort drop, no persisted abandoned state, and no digest metrics in the api-server source.

## Next actions

- **[high · backend]** Fix `find_inactive_digest_candidates` to `LEFT JOIN last_activity` with `COALESCE(la.last_active, u.created_at)`, and add a regression test for users with no refresh_tokens row.
  - DoD: users with no or pruned refresh tokens are included in the sweep; test passes.
- **[high · backend]** Add `notification_digests.abandoned_at TIMESTAMPTZ` (new migration after 00061) and set it in `report_abandoned`, so abandoned state is queryable.
  - DoD: migration merged; abandoned rows are queryable and counted; `report_abandoned` is idempotent.
- **[medium · backend]** Define digest KPIs and emit counters (`candidates_considered`, `sent`, `failed`, `abandoned`, `excluded_by_filter`) plus a per-sweep summary log or metric.
  - DoD: metrics are documented in the sprint doc or analytics spec and visible on a dashboard.
- **[medium · frontend]** Add `trackEvent` calls for `fault_report_submitted`/`failed` (mobile ReportFault) and moderation route decisions (SPA), with a name, trigger and property schema, using the existing ppt-web `trackEvent` sink; mobile needs an equivalent sink.
  - DoD: event catalogue entry exists for each; tests assert emission; no PII in properties.
- **[low · pm]** Backfill KPI definitions for shipped stories in `sprint-status.yaml` or the epic docs, and add an events/KPI column to the story template.
  - DoD: each in-progress/review story names its KPI and events.

## Risks

- **[high · high] Digest engagement/re-activation KPIs biased by INNER JOIN.** The cohort with no live refresh token (long-tail inactive users) is excluded without any record. Mitigate: LEFT JOIN fix, then backfill the cohort size and report it as `excluded_by_filter`.
- **[medium · high] Abandoned digests visible only in logs.** After log loss or a restart the failure rate is under-reported and cannot be audited. Mitigate: add `abandoned_at` and alert on count.
- **[low · medium] Advisory-lock release leak can silently skip sweep runs.** Creates unreported gaps in digest delivery data. Mitigate: detach or close the connection on release error; emit a sweep-skipped counter.
- **[medium · medium] Mobile has no evident analytics layer.** Mobile fault-report attempt and conversion funnels cannot be measured; web and mobile metric definitions may diverge. Mitigate: agree one event catalogue and a shared event schema.
- **[medium · low] Events added late to shipped features have no baseline.** Before/after comparisons are weak. Mitigate: make events part of the story definition of done.

## Open questions

- Do PR #3562 and PR #3567 add `trackEvent` calls? I did not open the diffs or `merged_prs_meta.json`.
- Is there a central event catalogue or metrics doc? I found none in the files I read.
- Does mobile have a telemetry SDK? Grep found only 21 incidental matches and no sink.
- Does any Prometheus or metrics module cover the digest service? Search of api-server metric files returned no matches.
- What are the retention and privacy rules for `notification_digests` rows and `refresh_tokens.last_used_at` (used as an activity signal)?

## Decisions needed

- Define 'abandoned' semantics and retention (soft-terminal column vs. dead-letter table) — owner: backend/architect.
- Choose the canonical analytics event schema and naming convention, and the mobile sink — owner: pm/frontend lead.
- Decide whether inactivity should be computed from `refresh_tokens` or from a dedicated `last_active_at` column — owner: backend/architect.
