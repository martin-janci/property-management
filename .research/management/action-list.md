# Action List

<sub>Generated: 2026-07-26T05:20:00Z</sub>

| Status | Priority | Owner | ID | Action |
|--------|----------|-------|-----|--------|
| open | high | pm-backend | `pm-backend-establish-a-standard-test-pattern-for-sc` | Establish a standard test pattern for scheduler-fired jobs (services/scheduler.rs) - retention prune (#2547), auto-unpin |
| open | high | pm-backend | `pm-backend-open-a-refactor-rfc-to-split-auth-rs-295` | Open a refactor RFC to split auth.rs (2950 lines, 4th repeat-churn cycle - now also touched by draft PR #2553 cold-boot  |
| open | high | pm-scrum-master | `pm-scrum-master-get-reviewer-sign-off-and-merge-pr-2553` | Get reviewer sign-off and merge PR #2553 (AuthContext cold-boot init bypassing refreshTokenInternal fix — reviewer_summa |
| open | high | pm-scrum-master | `pm-scrum-master-land-the-two-in-progress-mvp-gap-closure` | Land the two in-progress MVP gap-closure tasks: direct-to-S3 upload wiring (84-1) and signer-facing document-sign page ( |
| open | high | pm-scrum-master | `pm-scrum-master-merge-already-approved-pr-2549-layout-pu` | Merge already-approved PR #2549 (layout publish/webhook event wiring) to close issue #2532 |
| open | medium | pm-tech-lead | `gh-issue-2532` | Follow-up: wire layout publish/webhook event emission (spec-only, no producers) (PR #2494) (Closes #2532) |
| open | medium | pm-backend | `pm-backend-apply-the-same-review-to-reports-rs-3329` | Apply the same review to reports.rs (3329 lines, 3rd repeat-churn cycle, growing further via epic-6 SQL-backed dispute K |
| open | medium | pm-frontend | `pm-frontend-track-draft-pr-2553-authcontext-cold-boo` | Track draft PR #2553 (AuthContext cold-boot bypass, scope_drift=true) to merge with explicit awareness it's landing on t |
| open | medium | pm-integration | `pm-integration-assess-booking-mod-rs-as-a-first-time-ho` | Assess booking/mod.rs as a first-time hot spot against the already-roadmapped cross-cutting webhook hardening audit - de |
| open | medium | pm-qa | `pm-qa-confirm-whether-wave-j1-s-un-quarantine` | Confirm whether Wave J1's un-quarantine-then-re-quarantine-11 (PR #2511) reflects real flakiness vs. a one-off, before c |
| open | low | pm-backend | `code-review-ppt-web-core-authctx-init-stale-role` | AuthContext init bypasses refreshTokenInternal → stale role on cold-boot refresh (#574 fix gap) |
| open | low | pm-backend | `code-review-ppt-web-core-ws-token-rotation-stale` | WebSocket not re-authed on token rotation — connect() early-return leaves live socket on old token |
| open | low | pm-tech-lead | `code-review-ppt-web-core-ws-ungated-console` | 10 ungated console.warn/error in ppt-web websocket.ts leak diagnostics in prod |
| open | low | pm-tech-lead | `refactor-churn-hotspot-infra-ops-authz-backfill-2026-07-23` | Churn hotspot: infra_ops_authz_backfill_tests.rs — 364 lines this run (BIT-268 test backfill) |
| open | low | pm-tech-lead | `refactor-churn-hotspot-org-property-authz-backfill-2026-07-23` | Churn hotspot: org_property_authz_backfill_tests.rs — 412 lines this run (BIT-268/BIT-559 authz salvage) |
| open | low | pm-tech-lead | `refactor-churn-hotspot-platform-admin-authz-batch2-2026-07-23` | Churn hotspot: platform_admin_authz_batch2_tests.rs — 417 lines this run (BIT-557 test backfill) |
| open | low | pm-qa | `screen-map-drift-pr-2497-reality` | screen-map drift: PR #2497 touched reality-web/app/api/layout-revalidate/route.ts w/o docs/screens/reality/ |
| open | low | pm-tech-lead | `triage-closed-not-merged-pr-2489` | PR #2489 closed unmerged: dependabot npm-minor-patch (5→4 update group) superseded by #2491 |
| in-progress | high | pm-frontend | `gap-84-1-wire-ppt-web-direct-to-s3-upload-2026-07-23` | Wire ppt-web direct-to-S3 upload via POST /api/v1/documents/upload-url: api-client binding + UploadDocument integration  |
| in-progress | high | pm-frontend | `gap-84-2-document-sign-page-2026-07-23-retry3` | Build signer-facing document-sign page in ppt-web against shipped signing API; flip screen-map ppt/document-sign buildSt |
| in-progress | high | pm-integration | `integrations-webhook-hardening-audit-2026-07-23` | Audit all webhook handlers (booking, airbnb, esignature, layout) for timestamp/replay/idempotency parity — #2485 shows l |
| in-progress | high | pm-security | `sec-layout-webhook-hmac-verify-2026-07-23` | Verify layout publish webhook uses HMAC signature verification (parity with esignature webhook) — feeds #2485 fix design |
| in-progress | medium | pm-security | `code-review-api-handlers-community-unauthenticated-reads` | SECURITY: community.rs get_group/list_posts/get_item run unauthenticated — anonymous cross-tenant read |
| in-progress | medium | pm-data | `data-announcement-fanout-metric-2026-07-23` | Instrument announcement fan-out with delivered/read/ack metrics per targeting scope; also feed #2484 real-SQL integratio |
| in-progress | medium | pm-data | `data-dispute-fsm-kpi-definitions-2026-07-23` | Define dispute-lifecycle KPI set (filed->mediation->resolved funnel, TTR percentiles, evidence-per-dispute) — Epic 80 st |
| in-progress | medium | pm-data | `data-fault-kpi-unification-2026-07-23` | Unify FaultStatusCount metric with owner/portfolio fault KPIs into one shared definition (open decision from 2026-05-28) |
| in-progress | medium | pm-data | `data-layout-publish-event-tracking-2026-07-23` | Define layout publish/webhook analytics events (published_by, layout_version, target_tenant_count) — Layout & Content Ma |
| in-progress | medium | pm-data | `data-privacy-retention-policy-2026-07-23` | Publish data-retention policy for support-data / analytics events / audit trail (append-only support_tooling_events has  |
| in-progress | medium | pm-data | `data-support-data-audit-event-def-2026-07-23` | Formalize support-staff read audit event schema (who viewed which tenant's diagnostics, who revoked sessions) separate f |
| in-progress | medium | pm-frontend | `docs-screen-map-frontmatter-epics-2026-07-23` | Backfill screen-map frontmatter epics: field so epic->screen linkage stops manufacturing orphans in coverage scan (syste |
| in-progress | medium | pm-tech-lead | `gh-issue-2366-retry1` | Follow-up: direct-to-S3 upload drops building_id — building-scoped documents lose their association (PR #2345) (Closes # |
| in-progress | medium | pm-tech-lead | `gh-issue-2483` | Follow-up: add_evidence dispute sub-resource is still cross-tenant-writable (missed by #2441/PR #2450) (Closes #2483) |
| in-progress | medium | pm-tech-lead | `gh-issue-2484` | Follow-up: announcement cross-tenant fan-out guard is tested only via a pure-Rust re-model, not the real SQL (PR #2455)  |
| in-progress | medium | pm-tech-lead | `gh-issue-2485` | Follow-up: layout publish webhook has no timestamp/replay protection (PR #2431) (Closes #2485) |
| in-progress | medium | pm-tech-lead | `gh-issue-2486` | Follow-up: mobile LAYOUT_CACHE_KEY is not tenant-scoped and survives logout (PR #2432) (Closes #2486) |
| in-progress | medium | pm-tech-lead | `gh-issue-2528` | Follow-up: harden booking_push_notification webhook + Airbnb replay parity (PR #2499) (Closes #2528) |
| in-progress | medium | pm-security | `sec-ammonia-supply-chain-audit-2026-07-23` | Post-#2446 (RUSTSEC-2026-0213 ammonia bump) — run a cargo-audit sweep against the workspace to catch any other pinned cr |
| in-progress | low | pm-data | `data-audit-oauth-token-usage-2026-07-23` | Add analytics tracking for OAuth token issuance/refresh/revocation events per client (Epic 10A shipped; needed for platf |
| in-progress | low | pm-data | `data-mobile-native-analytics-parity-2026-07-23` | Audit mobile-native (Reality KMP) event tracking against ppt-web/reality-web parity — surface funnels missing analytics  |
| in-progress | low | pm-devops | `devops-nextest-partition-runbook-2026-07-23` | Document the nextest archive + per-test partition workflow (#2459) and the 206->8 test-binary consolidation (#2461/#2487 |
| in-progress | low | pm-tech-lead | `refactor-churn-hotspot-backend-crates-db-src-models-mod-rs-retry2` | Churn hotspot: backend/crates/db/src/models/mod.rs (12 commits in 19-day catch-up) [retry 2/2 of failed refactor-churn-h |
| in-progress | low | pm-tech-lead | `refactor-churn-hotspot-ppt-dashboard-md-2026-07-21` | Churn hotspot: docs/screens/ppt/dashboard.md — 3 touches this run (Layout & Content Manager pilot integration) |
| in-progress | low | pm-tech-lead | `refactor-churn-hotspot-repo-map-md-2026-07-20` | Churn hotspot: docs/repo-map.md — 4 touches this window (per-PR route-map refresh) |
| in-progress | low | pm-frontend | `refactor-layout-editor-styles-followthrough-2026-07-23` | Follow-through on #2464 LayoutEditorPage style extraction — apply same pattern to remaining layout-editor components to  |
| done | medium | pm-tech-lead | `gh-issue-2530` | Follow-up: instrument the full signup funnel, not just the onboarding tour (PR #2515) (Closes #2530) |
