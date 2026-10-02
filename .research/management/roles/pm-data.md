# Role: pm-data — 2026-10-02

> Data/analytics lens. Rotating role this run (pm_cursor rotation[6]). Static read-only. Previous run: 2026-07-23 (71 days ago — longest-stale slot in rotation).

**Summary:** Business KPI counters (`faults_created_total`, `votes_cast_total`, `organizations_created_total`, `auth_login_total`) are declared via `describe_counter!` in `backend/servers/api-server/src/observability.rs:272-282`, but a repo-wide grep finds no emit site. They will export nothing. No canonical domain-event schema or tracking plan exists; the only `fault_reported`/`vote_cast`/`payment_completed` matches are in a db test file. Only `reality-server` listing-view analytics (`portal_track_listing_view`) is genuinely instrumented. This run was static and limited to ~5 reads; dashboard, soft-delete, and timezone/currency normalisation checks were not performed.

## Carried-over 2026-07-23 items still open

- FaultStatusCount canonical definition — still not landed
- Retention/TTL policy for `support_tooling_events` + audit trail — still not published
- Dispute lifecycle KPIs (filed → mediation → resolved, TTR p50/p95) — still absent

## Next actions

| Action | Priority | Dependency | Definition of done |
|---|---|---|---|
| Wire emit sites for the 4 declared KPI counters, or remove the dead `describe_counter!` blocks | high | none | Counters increment in `create_fault`, `cast_vote`, `create_org`, and login handlers; `/metrics` shows non-zero values |
| Author a canonical domain-event catalogue (name, trigger, props, owner) | high | pm-tech-lead | `docs/data/event-catalog.md` merged, covering fault, vote, payment, booking, dispute, announcement events |
| Define a fault-SLA metric: histogram of time to triage + resolve, labelled by priority | medium | none | One `FaultStatusCount` definition in a shared module; support-data and owner/portfolio KPIs both use it |
| Add a metrics-assertion test helper and require it for KPI-touching handlers | medium | pm-qa | Helper in shared test utils; at least 3 handlers assert emission |
| Add Stripe webhook `payment_completed`/`payment_failed` counters with idempotency labels | medium | pm-security | Counters emitted on the webhook path; no PII in labels |
| Carry over: publish retention policy for `support_tooling_events` + audit trail; add dispute lifecycle KPIs (filed→mediation→resolved, TTR p50/p95) | medium | pm-security | Policy doc merged; dispute metric definitions documented |

## Risks

| Risk | Probability | Impact | Mitigation |
|---|---|---|---|
| Declared-but-never-emitted KPI counters make dashboards look healthy while showing zero — teams may wrongly assume tracking exists | high | high | Wire the emit sites or remove the describes; add CI check that every described metric has an emit site |
| No event catalogue means web, mobile, and reality surfaces will name/shape events inconsistently; funnels will diverge | high | medium | Publish the catalogue; make it a story DoD item |
| 2026-07-23 open items unresolved: FaultStatusCount definitions diverge, `support_tooling_events` has no TTL (GDPR) | medium | medium | Decide the canonical definition and retention policy this sprint |
| Delivery signals #3009–#3011 (toast, auth-header refactor, error-state UI) added no client-side error/UX telemetry; MFA-retry unification has no retry/failure counter | medium | low | Add `auth_refresh` + `mfa_retry` counters in the shared api-client or the server |
| Dispatcher git-stash bug #3012 may lose uncommitted agent work; no pipeline-health metric exists to detect it | medium | low | Log and count dispatcher isolation failures |

## Decisions needed

- Analytics platform choice (Prometheus-only vs product analytics SDK) — owner: pm-tech-lead
- Minimum-analytics DoD for stories: blocking gate or advisory — owner: pm-scrum-master
- `FaultStatusCount` canonical definition — owner: pm-data
- Retention + TTL policy for `support_tooling_events` and audit data — owner: pm-security

## Open questions

- Which analytics platform is in use (Prometheus/Grafana only, or also PostHog/Amplitude)? Is a client-side event SDK present in ppt-web or mobile?
- Is there a metrics scrape and dashboard config (Grafana JSON) anywhere in the repo or infra? Not verified this run.
- Do payments and bookings have any emit sites? The Stripe webhook is in the backend CLAUDE.md but was not read.
- Are timezone (UTC) and currency (minor units, ISO code) normalisation enforced at the schema level? Not checked this run.
- Is soft-delete (`deleted_at`) coverage consistent across critical tables? Not checked; only announcement comments confirmed soft-deleted.
