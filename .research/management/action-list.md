# PPT Action List

_Generated: 2026-07-29 · 56 items · 28 open · 27 in-progress · 1 done (target buffer: 36 open) · pm-backend rotation slot added 6 new actions this run._

| Priority | Status | Owner | ID | Action | Dep | Source |
|---|---|---|---|---|---|---|
| high | open | pm-frontend | gap-84-1-wire-ppt-web-direct-to-s3-upload-2026-07-23 | Wire ppt-web direct-to-S3 upload via POST /api/v1/documents/upload-url: api-client binding + UploadDocument in... | none | pm-scrum-master 2026-07-23 (score=8: mvp*2 + partial*2 + fin |
| high | open | pm-frontend | gap-84-2-document-sign-page-2026-07-23-retry3 | Build signer-facing document-sign page in ppt-web against shipped signing API; flip screen-map ppt/document-si... | none | pm-scrum-master 2026-07-23 (score=8: mvp*2 + partial*2 + fin |
| high | open | pm-integration | integrations-webhook-hardening-audit-2026-07-23 | Audit all webhook handlers (booking, airbnb, esignature, layout) for timestamp/replay/idempotency parity — #24... | gh-issue-2485 | pm-scrum-master 2026-07-23 (score=7: security*2 + cross-cutt |
| high | open | pm-security | sec-layout-webhook-hmac-verify-2026-07-23 | Verify layout publish webhook uses HMAC signature verification (parity with esignature webhook) — feeds #2485 ... | gh-issue-2485 | pm-security 2026-07-23 (score=7: sec*2 + dep-blocker + arch- |
| medium | open | pm-data | data-audit-add-evidence-idor-fix-2026-07-23 | After #2483/PR #2490 lands, backfill dispute add_evidence access-audit event (who added evidence to which disp... | gh-issue-2483 | pm-data 2026-07-23 (score=6: security-adjacent + audit-compl |
| medium | open | pm-qa | sec-mobile-layout-cache-tenant-scope-test-2026-07-23 | Add mobile test asserting LAYOUT_CACHE_KEY is cleared/tenant-scoped on logout+re-login (follows #2486 fix) | gh-issue-2486 | pm-scrum-master 2026-07-23 (score=6: security*2 + mobile + d |
| medium | open | pm-security | sec-layout-webhook-integration-test-2026-07-23 | Add integration test for layout publish webhook replay/timestamp handling once #2485 fix lands (parity with ge... | gh-issue-2485 | pm-scrum-master 2026-07-23 (score=6: security*2 + dep-blocke |
| medium | open | pm-data | data-layout-publish-event-tracking-2026-07-23 | Define layout publish/webhook analytics events (published_by, layout_version, target_tenant_count) — Layout & ... | none | pm-data 2026-07-23 (score=5: KPI gap on newly-shipped featur |
| medium | open | pm-data | data-dispute-fsm-kpi-definitions-2026-07-23 | Define dispute-lifecycle KPI set (filed->mediation->resolved funnel, TTR percentiles, evidence-per-dispute) — ... | none | pm-data 2026-07-23 (score=5: KPI gap on shipped MVP epic) |
| medium | open | pm-data | data-announcement-fanout-metric-2026-07-23 | Instrument announcement fan-out with delivered/read/ack metrics per targeting scope; also feed #2484 real-SQL ... | gh-issue-2484 | pm-data 2026-07-23 (score=5: KPI gap on Epic 6 + data-integr |
| medium | open | pm-data | data-support-data-audit-event-def-2026-07-23 | Formalize support-staff read audit event schema (who viewed which tenant's diagnostics, who revoked sessions) ... | none | pm-data 2026-07-23 (score=5: reopens 2026-05-28 pm-data deci |
| medium | open | pm-data | data-fault-kpi-unification-2026-07-23 | Unify FaultStatusCount metric with owner/portfolio fault KPIs into one shared definition (open decision from 2... | none | pm-data 2026-07-23 (score=5: metric-consistency) |
| medium | open | pm-data | data-signup-funnel-tracking-2026-07-23 | Instrument signup / onboarding-tour completion funnel (10b-6) — TourOverlay hook exists, no analytics events f... | none | pm-data 2026-07-23 (score=5: onboarding KPI blindness) |
| medium | open | pm-data | data-privacy-retention-policy-2026-07-23 | Publish data-retention policy for support-data / analytics events / audit trail (append-only support_tooling_e... | none | pm-data 2026-07-23 (score=5: privacy/GDPR) |
| medium | open | pm-frontend | chore-uc-33-dispute-subuc-links-2026-07-23 | Link UC-33.1/UC-33.2/UC-33.3 (dispute sub-UCs) to the dispute screen-maps' use-cases frontmatter (missing_use_... | none | pm-scrum-master 2026-07-23 (score=5: screen-gap + easy) |
| medium | open | pm-frontend | docs-screen-map-frontmatter-epics-2026-07-23 | Backfill screen-map frontmatter epics: field so epic->screen linkage stops manufacturing orphans in coverage s... | none | pm-scrum-master 2026-07-23 (score=5: screen-gap + high-lever |
| medium | open | pm-qa | test-scheduler-target-ids-regression-2026-07-23 | Backfill regression test for scheduler malformed target_ids parse (fix in #2436) — currently no test asserts s... | none | pm-scrum-master 2026-07-23 (score=5: security-adjacent test- |
| medium | open | pm-security | sec-ammonia-supply-chain-audit-2026-07-23 | Post-#2446 (RUSTSEC-2026-0213 ammonia bump) — run a cargo-audit sweep against the workspace to catch any other... | none | pm-scrum-master 2026-07-23 (score=5: security-hygiene follow |
| medium | open | pm-scrum-master | chore-stale-pr-2433-ios-layout-2026-07-23 | Unblock PR #2433 (mobile-native iOS resolved-layout) — 2 days without update; owner ping / rebase decision | none | pm-scrum-master 2026-07-23 (score=4: stalled review) |
| medium | in-progress | pm-tech-lead | gh-issue-2483 | Follow-up: add_evidence dispute sub-resource is still cross-tenant-writable (missed by #2441/PR #2450) (Closes... | none | dispatcher-issue-ingest 2026-07-23T08:02:32Z (#2483) |
| medium | in-progress | pm-tech-lead | gh-issue-2484 | Follow-up: announcement cross-tenant fan-out guard is tested only via a pure-Rust re-model, not the real SQL (... | none | dispatcher-issue-ingest 2026-07-23T08:02:32Z (#2484) |
| medium | in-progress | pm-tech-lead | gh-issue-2485 | Follow-up: layout publish webhook has no timestamp/replay protection (PR #2431) (Closes #2485) | none | dispatcher-issue-ingest 2026-07-23T08:02:32Z (#2485) |
| medium | in-progress | pm-tech-lead | gh-issue-2486 | Follow-up: mobile LAYOUT_CACHE_KEY is not tenant-scoped and survives logout (PR #2432) (Closes #2486) | none | dispatcher-issue-ingest 2026-07-23T08:02:32Z (#2486) |
| medium | in-progress | pm-tech-lead | gh-issue-2366-retry1 | Follow-up: direct-to-S3 upload drops building_id — building-scoped documents lose their association (PR #2345)... | none | dispatcher-retry-remint 2026-07-23T10:07:50Z (retry_of=gh-is |
| low | open | pm-data | data-audit-oauth-token-usage-2026-07-23 | Add analytics tracking for OAuth token issuance/refresh/revocation events per client (Epic 10A shipped; needed... | none | pm-data 2026-07-23 (score=4: KPI gap; developer-ecosystem vi |
| low | open | pm-data | data-mobile-native-analytics-parity-2026-07-23 | Audit mobile-native (Reality KMP) event tracking against ppt-web/reality-web parity — surface funnels missing ... | none | pm-data 2026-07-23 (score=4: mobile-parity KPI gap) |
| low | open | pm-data | data-pgvector-rag-observability-2026-07-23 | Add observability for pgvector RAG retrieval quality (84-5 shipped) — retrieval latency, top-k relevance, empt... | none | pm-data 2026-07-23 (score=4: observability gap on freshly-sh |
| low | open | pm-data | data-reality-portal-listing-view-events-2026-07-23 | Add listing-view analytics events (reality-web + mobile-native) with view-source, filter-state, session contex... | none | pm-data 2026-07-23 (score=4: KPI gap on public listings surf |
| low | open | pm-devops | devops-nextest-partition-runbook-2026-07-23 | Document the nextest archive + per-test partition workflow (#2459) and the 206->8 test-binary consolidation (#... | none | pm-scrum-master 2026-07-23 (score=3: docs + follow-through o |
| low | open | pm-frontend | refactor-layout-editor-styles-followthrough-2026-07-23 | Follow-through on #2464 LayoutEditorPage style extraction — apply same pattern to remaining layout-editor comp... | none | pm-scrum-master 2026-07-23 (score=3: refactor; follows shipp |
| low | open | pm-qa | qa-verify-gate-adoption-checkin-2026-07-23 | Confirm every open PR (#2478/#2481/#2482/#2490/#2491) has run `just verify` locally (per #2444 verify gate) — ... | none | pm-scrum-master 2026-07-23 (score=3: process check) |
| low | open | pm-scrum-master | chore-triage-untriaged-issues-750s-2026-07-23 | Bulk-triage untriaged issues #749-#779 (backlog carry) — 30+ items in seen_signals list; owner assignment or b... | none | pm-scrum-master 2026-07-23 (score=3: backlog hygiene) |
| low | open | pm-devops | chore-dependabot-cargo-minor-batch-2026-07-23 | Review + merge dependabot cargo-minor-patch batch #2473 (6 crates) once CI is green; verify no lockfile drift ... | none | pm-scrum-master 2026-07-23 (score=2: routine deps) |
| low | in-progress | pm-tech-lead | refactor-churn-hotspot-ppt-dashboard-md-2026-07-21 | Churn hotspot: docs/screens/ppt/dashboard.md — 3 touches this run (Layout & Content Manager pilot integration) | none | dispatcher-backlog-refill 2026-07-22T08:10:32Z (score=1 conf |
| low | in-progress | pm-tech-lead | refactor-churn-hotspot-repo-map-md-2026-07-20 | Churn hotspot: docs/repo-map.md — 4 touches this window (per-PR route-map refresh) | none | dispatcher-backlog-refill 2026-07-22T08:10:32Z (score=1 conf |
| low | in-progress | pm-tech-lead | refactor-churn-hotspot-backend-crates-db-src-models-mod-rs-retry2 | Churn hotspot: backend/crates/db/src/models/mod.rs (12 commits in 19-day catch-up) [retry 2/2 of failed refact... | none | dispatcher-retry-remint 2026-07-22T10:05:12Z (retry_of=refac |

## Grouped by owner

### pm-backend (6, new 2026-07-29 rotation)
- !!! `pm-backend-review-acc-05-invoice-lifecycle-2026-07-29` — Review + land PR #2555 (acc-05 sent/cancelled invoice lifecycle) as prerequisite for #2558 (PDF) and #2559 (PAY-by-square QR)
- !! `pm-backend-acc-05-supply-chain-audit-2026-07-29` — Supply-chain check on new deps introduced by PR #2559 (crc32fast, lzma-rs); fold into pm-security cargo-audit sweep
- !! `pm-backend-layout-hardening-post-2478-followthrough-2026-07-29` — Integration test for publish TOCTOU + webhook replay to lock in the #2478 hardening (depends: gh-issue-2485)
- !! `pm-backend-signature-request-post-2504-e2e-2026-07-29` — Post-#2504 (BIT-313 signature-request mount fix): add e2e route test for /documents/{id}/signature-requests reachability
- !! `pm-backend-gh-issue-2557-dev-team-followups-2026-07-29` — Land test(backend) dev-team follow-ups from #2557
- ! `pm-backend-reports-rs-split-2026-07-29` — Plan module-split for backend/servers/api-server/src/routes/reports.rs (3329 LOC, runs_seen=3)

### pm-data (12)
- !! `data-audit-add-evidence-idor-fix-2026-07-23` — After #2483/PR #2490 lands, backfill dispute add_evidence access-audit event (who added evidence to which dispute) to the platform-admin support-data event stream (parity with support-data audit-read pattern)
- !! `data-layout-publish-event-tracking-2026-07-23` — Define layout publish/webhook analytics events (published_by, layout_version, target_tenant_count) — Layout & Content Manager shipped end-to-end but has no KPI hooks yet
- !! `data-dispute-fsm-kpi-definitions-2026-07-23` — Define dispute-lifecycle KPI set (filed->mediation->resolved funnel, TTR percentiles, evidence-per-dispute) — Epic 80 stories all done but no analytics dashboard exists; align with SupportDataPage metric definitions
- !! `data-announcement-fanout-metric-2026-07-23` — Instrument announcement fan-out with delivered/read/ack metrics per targeting scope; also feed #2484 real-SQL integration test data-quality check (currently pure-Rust re-model)
- !! `data-support-data-audit-event-def-2026-07-23` — Formalize support-staff read audit event schema (who viewed which tenant's diagnostics, who revoked sessions) separate from audit_read capability gate (open decision from 2026-05-28)
- !! `data-fault-kpi-unification-2026-07-23` — Unify FaultStatusCount metric with owner/portfolio fault KPIs into one shared definition (open decision from 2026-05-28)
- !! `data-signup-funnel-tracking-2026-07-23` — Instrument signup / onboarding-tour completion funnel (10b-6) — TourOverlay hook exists, no analytics events fired on complete/skip/reset
- !! `data-privacy-retention-policy-2026-07-23` — Publish data-retention policy for support-data / analytics events / audit trail (append-only support_tooling_events has no TTL yet)
- ! `data-audit-oauth-token-usage-2026-07-23` — Add analytics tracking for OAuth token issuance/refresh/revocation events per client (Epic 10A shipped; needed for platform-admin ecosystem health dashboard)
- ! `data-mobile-native-analytics-parity-2026-07-23` — Audit mobile-native (Reality KMP) event tracking against ppt-web/reality-web parity — surface funnels missing analytics hooks (listing view, search, contact-inquiry)
- ! `data-pgvector-rag-observability-2026-07-23` — Add observability for pgvector RAG retrieval quality (84-5 shipped) — retrieval latency, top-k relevance, empty-result rate — before feature adoption grows
- ! `data-reality-portal-listing-view-events-2026-07-23` — Add listing-view analytics events (reality-web + mobile-native) with view-source, filter-state, session context — foundational for realtor conversion metrics

### pm-devops (2)
- ! `devops-nextest-partition-runbook-2026-07-23` — Document the nextest archive + per-test partition workflow (#2459) and the 206->8 test-binary consolidation (#2461/#2487) as a contributor runbook so future test files land in the consolidated crates
- ! `chore-dependabot-cargo-minor-batch-2026-07-23` — Review + merge dependabot cargo-minor-patch batch #2473 (6 crates) once CI is green; verify no lockfile drift with dev

### pm-frontend (5)
- !!! `gap-84-1-wire-ppt-web-direct-to-s3-upload-2026-07-23` — Wire ppt-web direct-to-S3 upload via POST /api/v1/documents/upload-url: api-client binding + UploadDocument integration + screen-map note (84-1 partial; backend #2309 shipped)
- !!! `gap-84-2-document-sign-page-2026-07-23-retry3` — Build signer-facing document-sign page in ppt-web against shipped signing API; flip screen-map ppt/document-sign buildStatus planned->shipped; verify signature-request email delivery (84-2 partial; prior attempt failed no-PR)
- !! `chore-uc-33-dispute-subuc-links-2026-07-23` — Link UC-33.1/UC-33.2/UC-33.3 (dispute sub-UCs) to the dispute screen-maps' use-cases frontmatter (missing_use_cases in coverage.json)
- !! `docs-screen-map-frontmatter-epics-2026-07-23` — Backfill screen-map frontmatter epics: field so epic->screen linkage stops manufacturing orphans in coverage scan (systemic finding from 2026-06-23 deep scan; still present)
- ! `refactor-layout-editor-styles-followthrough-2026-07-23` — Follow-through on #2464 LayoutEditorPage style extraction — apply same pattern to remaining layout-editor components to reduce inline-style churn

### pm-integration (1)
- !!! `integrations-webhook-hardening-audit-2026-07-23` — Audit all webhook handlers (booking, airbnb, esignature, layout) for timestamp/replay/idempotency parity — #2485 shows layout webhook lacks it; treat as cross-cutting review

### pm-qa (3)
- !! `sec-mobile-layout-cache-tenant-scope-test-2026-07-23` — Add mobile test asserting LAYOUT_CACHE_KEY is cleared/tenant-scoped on logout+re-login (follows #2486 fix)
- !! `test-scheduler-target-ids-regression-2026-07-23` — Backfill regression test for scheduler malformed target_ids parse (fix in #2436) — currently no test asserts silent-parse behavior is gone
- ! `qa-verify-gate-adoption-checkin-2026-07-23` — Confirm every open PR (#2478/#2481/#2482/#2490/#2491) has run `just verify` locally (per #2444 verify gate) — enforcement or advisory-only?

### pm-scrum-master (2)
- !! `chore-stale-pr-2433-ios-layout-2026-07-23` — Unblock PR #2433 (mobile-native iOS resolved-layout) — 2 days without update; owner ping / rebase decision
- ! `chore-triage-untriaged-issues-750s-2026-07-23` — Bulk-triage untriaged issues #749-#779 (backlog carry) — 30+ items in seen_signals list; owner assignment or bulk-close

### pm-security (3)
- !!! `sec-layout-webhook-hmac-verify-2026-07-23` — Verify layout publish webhook uses HMAC signature verification (parity with esignature webhook) — feeds #2485 fix design
- !! `sec-layout-webhook-integration-test-2026-07-23` — Add integration test for layout publish webhook replay/timestamp handling once #2485 fix lands (parity with generic integration webhook test pattern)
- !! `sec-ammonia-supply-chain-audit-2026-07-23` — Post-#2446 (RUSTSEC-2026-0213 ammonia bump) — run a cargo-audit sweep against the workspace to catch any other pinned crates with open advisories

### pm-tech-lead (8)
- !! `gh-issue-2483` — Follow-up: add_evidence dispute sub-resource is still cross-tenant-writable (missed by #2441/PR #2450) (Closes #2483)
- !! `gh-issue-2484` — Follow-up: announcement cross-tenant fan-out guard is tested only via a pure-Rust re-model, not the real SQL (PR #2455) (Closes #2484)
- !! `gh-issue-2485` — Follow-up: layout publish webhook has no timestamp/replay protection (PR #2431) (Closes #2485)
- !! `gh-issue-2486` — Follow-up: mobile LAYOUT_CACHE_KEY is not tenant-scoped and survives logout (PR #2432) (Closes #2486)
- !! `gh-issue-2366-retry1` — Follow-up: direct-to-S3 upload drops building_id — building-scoped documents lose their association (PR #2345) (Closes #2366) [retry 1/2 of failed gh-issue-2366]
- ! `refactor-churn-hotspot-ppt-dashboard-md-2026-07-21` — Churn hotspot: docs/screens/ppt/dashboard.md — 3 touches this run (Layout & Content Manager pilot integration)
- ! `refactor-churn-hotspot-repo-map-md-2026-07-20` — Churn hotspot: docs/repo-map.md — 4 touches this window (per-PR route-map refresh)
- ! `refactor-churn-hotspot-backend-crates-db-src-models-mod-rs-retry2` — Churn hotspot: backend/crates/db/src/models/mod.rs (12 commits in 19-day catch-up) [retry 2/2 of failed refactor-churn-hotspot-backend-crates-db-src-models-mod-rs]
