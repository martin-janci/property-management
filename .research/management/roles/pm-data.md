# pm-data — 2026-11-28

## Summary

The notification group-key fix (#3863) is verified in code, so in-app grouping
is now deterministic. The Hardening sprint has no KPI or data-quality
instrumentation. `.research/management/*.json`, `coverage.json` and the
quarantine registry were not opened this pass.

## Exec summary

In `backend/servers/api-server/src/services/notification.rs`, `in_app_group_key()`
returns `(notification, entity_id)` for entity-bound notifications and
`(event_type, Uuid::nil())` for entity-less ones. Regression tests sit at lines
~1070-1100. That removes the random-UUID singleton groups that distorted
grouped-notification counts, but it creates a new unbounded per-user,
per-event-type group. Separately, the three Hardening workstreams (0/6, 0/201,
0/4) are tracked only as hand-edited counters in `sprint-status.yaml`, with no
machine-readable burn-down series and no `coverage.json` entries.

## Findings

- **VERIFIED:** `in_app_group_key` is deterministic. Tests `in_app_group_key_is_deterministic_for_entity_less_notifications` and `in_app_group_key_separates_event_types_and_honours_entity_id`. The flagged item `code-review-api-core-notification-random-group-key` can be closed on this evidence.
- **NEW EDGE:** entity-less notifications of one event type now share one group per user (`UNIQUE (user_id, group_key)` + key `event_type_<nil-uuid>`). `notification_count` grows for the life of the account unless groups are reset on read or archived.
- **LEGACY DATA:** rows written before #3863 keep random-UUID group keys (singleton groups each). Pre- and post-2026-11-28 metrics are not comparable without a cutoff.
- **DEDUP CAVEAT** (notification.rs:170-175): send-dedup cache looks in-process. Multi-replica deployments can still double-deliver. Not verified further.
- **HARDENING TRACKING GAP:** workstreams exist only as `items_completed` counters in `sprint-status.yaml`. No time series → no burn-down rate against 2027-02-19 (needed ≈17 rows/week).
- **QUARANTINE METRIC DEFINITION:** 193 of 201 rows are "requires Postgres test DB; run with --ignored". Deleting versus moving to a gated integration lane gives different curves. "prefer fix+delete over bumping `due`" is not formally codified in the metric.
- **FINANCIAL KPI:** #3886 (payment matcher skips cross-currency invoices) fixes false matches, but skipped invoices now surface as unmatched and will inflate unmatched-payments/overdue KPIs unless tracked as a distinct reason.
- **PRIVACY:** error-leak sanitation PRs (#3855–#3890) reduce PII exposure in API errors. No data-retention changes in Hardening scope.

## Next actions

- **[high]** Define grouped-notification retention/reset semantics for nil-id event-type groups; record a pre/post-#3863 cutoff for notification-group metrics — owner `pm-backend`.
- **[high]** Add a machine-readable Hardening burn-down series (date, workstream, rows_total, rows_deleted, rows_converted_to_gated_lane, due_bumps) generated from `.github/quarantine/registry.toml` and appended daily — owner `pm-devops`.
- **[medium]** Audit `.research/management/*.json` churn by diffing two consecutive dispatcher runs for non-deterministic fields (timestamps, random ids, ordering); make writes idempotent/sorted — owner `pm-scrum-master`.

## Risks

- **[medium/medium]** Entity-less notification groups become unbounded per-user counters (`nil-uuid, event_type` key). Skews unread/grouped-count KPIs and can produce a very large `notification_count` or UX/query hot spot. _Mitigation:_ reset/archive on mark-read or after TTL; add count-reset test; monitor p99 `notification_count` per group.
- **[high/medium]** Burn-down of 201 quarantined tests invisible until 2027-02-19 age gate fails CI; `due` can be bumped; 193 DB-lane rows likely move in late batch so a linear trend will mislead. _Mitigation:_ daily series with explicit `due-bump` counter; interim milestone (≥50% by 2027-01-15) with alert if weekly rate drops below ≈17.

## Metrics proposed

- `quarantine_rows_remaining` (by owner), daily, with required-vs-actual burn rate and projected zero date.
- `quarantine_rows_deleted` vs `quarantine_rows_converted_to_gated_lane` vs `quarantine_due_bumps` (anti-gaming counter, target 0).
- `modules_under_watch`: LOC per module, weekly, target decreasing trend with `just verify` green.
- `notification_groups_per_user` and p99 `notification_count` per group, split by nil-id event-type groups vs entity groups, with pre/post-#3863 cutoff.
- `notification_delivery_dedup_hits` vs duplicate deliveries observed, to validate dedup reliability across replicas.
- `payment_matcher_skipped_cross_currency_total` (counter, with invoice and payment currency labels) so #3886 skips surface as an explicit reason rather than silent unmatched.
- `research_artifact_churn`: lines changed per run in `.research/management/*.json` (target ~0 on no-op runs).
- `mobile-native` backlog: days blocked per item (4 items, blocked on #2652/#2966), as a proxy for cloud-runner unblock urgency.

## Open questions

- Does the DB function behind `notification_groups` (migration 00061) or a scheduler reset, archive or expire groups? If not, how large can a nil-id event-type group grow?
- Was any backfill or cleanup planned for pre-#3863 singleton groups with random keys?
- Is the send-dedup cache in `notification.rs` shared across replicas (Redis) or per-process?
- Do `.research/management/coverage.json` and related files hold fields that change on every run?
- Which events or counters, if any, report payment-matcher skips from #3886?

## Decisions needed

- Metric definition for quarantine burn-down: count rows as done only when deleted, or also when moved to a gated integration lane? — owner `pm-scrum-master` with `pm-backend`.
- Retention or reset policy for entity-less notification groups, and whether to backfill or clean legacy random-key groups — owner `pm-backend`.
- Whether `coverage.json` should model Hardening workstreams, or `sprint-status.yaml` remains the sole source and `coverage.json` is retired for this sprint — owner operator.
