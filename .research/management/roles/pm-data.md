# Role: pm-data — 2026-05-28

> Data/analytics lens. Rotating role this run (pm_cursor rotation[6]). Static read-only.

**Summary:** The only data/analytics surface to move this window is the new Support Data admin page (#635), which exposes cross-tenant tenant diagnostics (user counts, active sessions, fault-status summary) via `GET /api/v1/platform-admin/support-data` behind the `audit_read` capability. It ships with no per-view usage tracking and a FaultStatusCount metric that overlaps the owner_analytics / portfolio_performance fault KPIs — a metric-consistency and PII-access-traceability concern. No new tracking-event definitions accompanied any of the 5 merged PRs.

## Next actions

| Action | Priority | Dependency | Definition of done |
|---|---|---|---|
| Define analytics/audit tracking events for the Support Data page (#635): `support_data_viewed` (admin_user_id, tenant_count, fault_total), `support_user_searched`, `support_sessions_revoked` so support-tooling usage is auditable separately from the capability gate. | medium | none | Events defined (name/trigger/properties/owner) and emitted on each support-data read / session-revoke. |
| Reconcile the FaultStatusCount / FaultByStatusTable metric definition (SupportDataPage #635) against owner_analytics / portfolio_performance fault KPIs — buckets + counting window defined once and reused. | medium | none | One shared fault-status query/helper; a fixture test asserting all three surfaces return identical counts. |
| Document the data-retention + privacy posture for support-data session/activity reads (`get_user_sessions` / `get_user_activity`): confirm PII access is itself audit-logged and retention-bounded. | low | none | Retention + audit posture documented; cross-tenant PII reads traceable to an admin actor. |

## Risks

| Risk | Prob | Impact | Mitigation |
|---|---|---|---|
| FaultStatusCount metric divergence: the #635 support fault-by-status metric re-derived per route will disagree with owner/portfolio fault dashboards, eroding trust and complicating triage. | medium | medium | Factor the bucket+window definition into one shared helper reused by support-data, owner-analytics, portfolio routes; add an identical-counts fixture test. |
| Support Data PII access has no usage tracking: support staff read cross-tenant memberships/sessions(IP)/activity gated only by `audit_read`; no event records who viewed/revoked, so cross-tenant PII access is not independently traceable. | medium | medium | Emit `support_data_viewed` / `support_sessions_revoked` audit events (admin actor + target); bound retention on session/activity diagnostics. |

## Open questions

- Is there an existing analytics/event pipeline (`track_event`/telemetry) the new admin surfaces should hook into, or is audit logging the only sink? (`analytics`/`metric` references exist in owner_analytics.rs, portfolio_performance.rs, market_pricing.rs — but no central event tracker was confirmed this run.)
- Does `audit_read` already produce an audit-trail row per support-data fetch, or only gate access?

## Decisions needed

- Should support-staff reads through the Support Data page emit their own audit/analytics events (who viewed which tenant), and should FaultStatusCount be unified with owner/portfolio fault KPIs into one shared definition? — owner: pm-data / pm-backend.
