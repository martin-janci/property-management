# Action list

_Generated: 2026-08-01T02:15:00Z_

| Status | Priority | Owner | Action | Dependency |
|--------|----------|-------|--------|------------|
| open | high | pm-frontend | Verify reality-web buildListingJsonLd escapes </script> before embedding in inline application/ld+json tag (post-#2600 a | none |
| open | high | pm-scrum-master | Fix issue #480: WS auth token leaked in query-param logs + missing re-validation after JWT expiry (severity high) | rust-backend |
| open | medium | pm-frontend | Ship frontend permission-authoring UI (AccessScopeSelector/RoleSelector/UserSelector) for story 7a-3; backend enforcemen | none |
| open | medium | pm-frontend | Close mobile parity gap for story 6-3: AnnouncementDetailScreen needs comment read+post UI (mobile currently shows count | none |
| open | medium | pm-frontend | Wire TenantLifecyclePage off hardcoded DEMO_TENANT onto the :id route param post-ui-kit refactor | none |
| open | low | pm-devops | admin-web mobile-config Save flow blocked: PATCH /api/v1/admin/mobile-config endpoint missing | none |
| open | low | pm-devops | admin-web platform-settings Save blocked: PATCH /api/v1/platform-admin/settings endpoint missing | none |
| open | low | pm-tech-lead | Churn hotspot: backend/servers/api-server/src/routes/auth.rs — 2950 lines this window (runs_seen=5, no refactor PR yet) | none |
| open | low | pm-tech-lead | Churn hotspot: backend/servers/api-server/src/routes/reports.rs — 3329 lines this window (PR #2599 extracted helpers) | none |
| in-progress | high | pm-frontend | Wire ppt-web direct-to-S3 upload via POST /api/v1/documents/upload-url: api-client binding + UploadDocument integration  | none |
| in-progress | high | pm-frontend | Build signer-facing document-sign page in ppt-web against shipped signing API; flip screen-map ppt/document-sign buildSt | none |
| in-progress | high | pm-tech-lead | ci: reality-web Docker build has failed on every run since 2026-06-17 — no frontend image published for 6 weeks (Closes  | none |
| in-progress | high | pm-integration | Audit all webhook handlers (booking, airbnb, esignature, layout) for timestamp/replay/idempotency parity — #2485 shows l | gh-issue-2485 |
| in-progress | high | pm-security | Verify layout publish webhook uses HMAC signature verification (parity with esignature webhook) — feeds #2485 fix design | gh-issue-2485 |
| in-progress | medium | pm-security | SECURITY: community.rs get_group/list_posts/get_item run unauthenticated — anonymous cross-tenant read | none |
| in-progress | medium | pm-data | Instrument announcement fan-out with delivered/read/ack metrics per targeting scope; also feed #2484 real-SQL integratio | gh-issue-2484 |
| in-progress | medium | pm-data | Define dispute-lifecycle KPI set (filed->mediation->resolved funnel, TTR percentiles, evidence-per-dispute) — Epic 80 st | none |
| in-progress | medium | pm-data | Unify FaultStatusCount metric with owner/portfolio fault KPIs into one shared definition (open decision from 2026-05-28) | none |
| in-progress | medium | pm-data | Define layout publish/webhook analytics events (published_by, layout_version, target_tenant_count) — Layout & Content Ma | none |
| in-progress | medium | pm-data | Publish data-retention policy for support-data / analytics events / audit trail (append-only support_tooling_events has  | none |
| in-progress | medium | pm-data | Formalize support-staff read audit event schema (who viewed which tenant's diagnostics, who revoked sessions) separate f | none |
| in-progress | medium | pm-frontend | Backfill screen-map frontmatter epics: field so epic->screen linkage stops manufacturing orphans in coverage scan (syste | none |
| in-progress | medium | pm-devops | SDK drift gate is effectively unenforced — api-validation.yml only fires on docs/api/**, so committed @ppt/api-client dr | none |
| in-progress | medium | pm-tech-lead | Follow-up: direct-to-S3 upload drops building_id — building-scoped documents lose their association (PR #2345) (Closes # | none |
| in-progress | medium | pm-tech-lead | Follow-up: add_evidence dispute sub-resource is still cross-tenant-writable (missed by #2441/PR #2450) (Closes #2483) | none |
| in-progress | medium | pm-tech-lead | Follow-up: announcement cross-tenant fan-out guard is tested only via a pure-Rust re-model, not the real SQL (PR #2455)  | none |
| in-progress | medium | pm-tech-lead | Follow-up: layout publish webhook has no timestamp/replay protection (PR #2431) (Closes #2485) | none |
| in-progress | medium | pm-tech-lead | Follow-up: mobile LAYOUT_CACHE_KEY is not tenant-scoped and survives logout (PR #2432) (Closes #2486) | none |
| in-progress | medium | pm-tech-lead | Follow-up: harden booking_push_notification webhook + Airbnb replay parity (PR #2499) (Closes #2528) | none |
| in-progress | medium | pm-tech-lead | Follow-up: instrument the full signup funnel, not just the onboarding tour (PR #2515) (Closes #2530) | none |
| in-progress | medium | pm-security | Post-#2446 (RUSTSEC-2026-0213 ammonia bump) — run a cargo-audit sweep against the workspace to catch any other pinned cr | none |
| in-progress | low | pm-data | Add analytics tracking for OAuth token issuance/refresh/revocation events per client (Epic 10A shipped; needed for platf | none |
| in-progress | low | pm-data | Audit mobile-native (Reality KMP) event tracking against ppt-web/reality-web parity — surface funnels missing analytics  | none |
| in-progress | low | pm-data | Add observability for pgvector RAG retrieval quality (84-5 shipped) — retrieval latency, top-k relevance, empty-result r | none |
| in-progress | low | pm-devops | Document the nextest archive + per-test partition workflow (#2459) and the 206->8 test-binary consolidation (#2461/#2487 | none |
| in-progress | low | pm-devops | Cloud routine cadence recovery — reduce 3–4d gaps between runs [retry 2/2 of failed dx-routine-lag-catchup-2026-07] | none |
| in-progress | low | pm-backend | Investigate repeated churn on backend/servers/api-server/src/services/scheduler.rs (top-churn 2 windows running) — propo | none |
| in-progress | low | pm-tech-lead | Churn hotspot: backend/crates/db/src/models/mod.rs (12 commits in 19-day catch-up) [retry 2/2 of failed refactor-churn-h | none |
| in-progress | low | pm-tech-lead | Churn hotspot: backend/crates/integrations/src/booking/mod.rs — 3626 lines this window (recently split by PR #2611) | none |
| in-progress | low | pm-tech-lead | Churn hotspot: docs/screens/ppt/dashboard.md — 3 touches this run (Layout & Content Manager pilot integration) | none |
| in-progress | low | pm-tech-lead | Churn hotspot: docs/repo-map.md — 4 touches this window (per-PR route-map refresh) | none |
| in-progress | low | pm-frontend | Follow-through on #2464 LayoutEditorPage style extraction — apply same pattern to remaining layout-editor components to  | none |
| in-progress | low | pm-qa | Screen-map drift: reality-web layout changed without docs/screens/reality/ update (PR #2600) | none |
