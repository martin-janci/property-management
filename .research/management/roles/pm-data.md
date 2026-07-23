# Role: pm-data — 2026-07-23

> Data/analytics lens. Rotating role this run (pm_cursor rotation[6]). Static read-only. Previous run: 2026-05-28 (56 days ago).

**Summary:** Delivery has largely converged on MVP (47/49 stories done) yet almost none of the recently shipped features are instrumented for KPI/analytics. Epic 6 announcements, Epic 10A OAuth, Epic 10B platform admin, Epic 80 disputes, Epic 84 e-signature all lack basic event tracking. The recent test-backfill wave (BIT-268/BIT-557/BIT-559 in PRs #2447/#2453/#2465) raised code coverage but did NOT add analytics events; the pipeline for platform-admin audit_read is well-defined (SupportDataPage) but has not been generalized to feature-level KPIs. Two carried-over 2026-05-28 decisions (support-staff audit event schema + FaultStatusCount unification) are still open.

## Cross-reference to shipped test-backfill wave

- **Is data flowing to analytics correctly?** No — the test wave verifies code paths execute, but there is no analytics-event emission assertion helper anywhere in the workspace. Silent metric drift is possible.
- **Missing metrics for disputes epic:** Epic 80 (all 3 stories done) has zero KPI — no filed/mediation/resolved funnel, no TTR percentiles, no evidence-per-dispute counter. PR #2450 hardened access but did not add an audit event for who accessed / added evidence to which dispute; #2483 follow-up (add_evidence IDOR) also needs an audit_write event when the fix lands.
- **Missing metrics for layout epic:** Layout & Content Manager (PRs #2424–#2432, #2443, #2464, #2478) shipped end-to-end with zero KPI hooks — no `published_by`, `layout_version`, or `target_tenant_count` events on the publish path.

## Next actions

| Action | Priority | Dependency | Definition of done |
|---|---|---|---|
| Backfill dispute add_evidence access-audit event to support-data event stream once #2483/PR #2490 lands (parity with support-data audit_read pattern) | medium | pm-security (gh-issue-2483) | audit_write event emitted on evidence upload; visible in SupportDataPage |
| Define layout publish/webhook analytics events (published_by, layout_version, target_tenant_count) — Layout & Content Manager shipped end-to-end with zero KPI hooks | medium | none | event schema + emission wired in publish_layout handler; documented in support-data catalogue |
| Define dispute-lifecycle KPI set (filed->mediation->resolved funnel, TTR percentiles, evidence-per-dispute) — Epic 80 all-done, no dashboard exists | medium | pm-scrum-master (dashboard scope) | metric definitions + filed/resolved counters + p50/p95 TTR emitted; shared taxonomy with owner/portfolio KPIs |
| Instrument announcement fan-out with delivered/read/ack per targeting scope; also feed #2484 real-SQL integration data-quality check | medium | pm-qa (gh-issue-2484 test rework) | metrics visible per scope (all/building/units/roles); #2484 test exercises real SQL and asserts count matches emitted metric |
| Publish data-retention policy for support-data / analytics events / audit trail (append-only support_tooling_events has no TTL) | medium | pm-security (GDPR classification) | policy doc merged; PII-carrying tables get lifecycle SQL jobs |
| Formalize support-staff read audit event schema (who viewed which tenant's diagnostics / revoked sessions) — carried-over decision from 2026-05-28 | medium | pm-security | event schema + emit at all support-data read/revoke sites |

## Risks

| Risk | Probability | Impact | Mitigation |
|---|---|---|---|
| Shipped MVP features (Epic 6/10A/10B/80/84) lack KPI instrumentation — product decisions run blind on exactly the features that just went live | high | medium | Sequence the 7 pm-data KPI tasks now on the action-list; establish minimum-analytics DoD for future stories |
| FaultStatusCount vs owner/portfolio fault KPIs diverge (2026-05-28 open decision) — dashboards will disagree | high | medium | Land single-source-of-truth definitions in shared module; deprecate duplicates |
| Append-only support_tooling_events + audit trail have no TTL / retention policy — long-term storage + GDPR risk | medium | medium | Publish retention policy; add lifecycle jobs if PII-carrying |
| Test-backfill wave proves code paths execute but does NOT verify analytics events fire — silent metric drift possible | medium | low | Add analytics-event assertion helper; require metric-touching tests to use it |
| Mobile (RN + KMP) event tracking parity with web unknown — funnels may be blind on ~50% of traffic | medium | medium | Audit + backfill (action data-mobile-native-analytics-parity-2026-07-23) |

## Open questions

- What KPI dashboard tool does PPT use for internal metrics — Grafana over Postgres, a third-party (Amplitude/PostHog/Segment), or bespoke platform-admin pages?
- Are dispute lifecycle KPIs required by any customer contract / regulatory obligation, or purely product-internal?
- Is there a Data Protection Impact Assessment on `support_tooling_events` (support staff reading tenant data)?
- For webhook events (booking / airbnb / esignature / layout), do we emit analytics on delivery + retry + failure, or only log?
- Are the seed-data recipes stable enough to reason about analytics test fidelity (pm-data DoD depends on repeatable seeds)?

## Decisions needed

- Analytics platform choice (bespoke vs Amplitude/PostHog/Segment) — owner: pm-tech-lead + pm-data
- GDPR / retention policy for `support_tooling_events` (TTL vs indefinite) — owner: pm-security + pm-data
- Minimum-analytics DoD for new stories (blocking gate or advisory?) — owner: pm-scrum-master + pm-data
- FaultStatusCount canonical definition (support-data vs owner/portfolio KPI) — owner: pm-data (carried over from 2026-05-28)
