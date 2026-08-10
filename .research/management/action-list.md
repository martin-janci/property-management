# Action list — PPT delivery

_Generated: 2026-08-10T09:00:00Z_ · _Buffer: 36/36 open_

Rendered from `action-list.json`. Items move to `done`/`failed` only when the dispatcher / implementer / rotating role agent resolves them.

| id | action | owner | priority | source |
|---|---|---|---|---|
| code-review-mobile-native-kmp-portfolio-analytics-caps-100 | mobile-native-kmp: getPortfolioAnalytics() truncates realtor portfolio at 100 listings | pm-backend | low | dispatcher-backlog-refill 2026-08-04 |
| code-review-api-core-workflow-cond-parse-failopen | workflow_executor.rs evaluate_conditions() FAILS OPEN on unparseable condition — convert to fail-closed | pm-backend | high | tier1d-signal-bridge 2026-08-05 |
| gap-84-1-direct-s3-upload-wiring | Wire ppt-web direct-to-S3 upload via POST /documents/upload-url (84-1 partial finish) | pm-frontend | high | gap-scan 2026-08-10 |
| gap-84-2-signer-page | Build signer-facing document-sign page in ppt-web (84-2 partial finish) | pm-frontend | high | gap-scan 2026-08-10 |
| pm-qa-un-quarantine-disputes-kpis-window-validation | Un-quarantine /disputes/kpis test + reject inverted window (400) — #2575 | pm-backend | high | pm-analysis 2026-08-10 |
| pm-qa-fanout-replace-purerust-with-sqlx-integration | Replace pure-Rust announcement fan-out test with sqlx integration — #2484 | pm-backend | high | pm-analysis 2026-08-10 |
| pm-qa-reality-server-security-batch-regression-tests | Add regression tests for reality-server security batch (#2724/#2725/#2726/#2727) | pm-backend | high | pm-analysis 2026-08-10 |
| pm-qa-workflow-executor-fail-closed-rewrite | Convert workflow_executor.rs unparseable-condition branch to fail-closed | pm-backend | high | pm-analysis 2026-08-10 |
| pm-qa-android-sso-mint-call-site | Wire SsoStateStore.mint() + integration test for reality://sso callback — #2574 | pm-mobile | high | pm-analysis 2026-08-10 |
| pm-qa-webhook-hmac-parity-extend | Extend PR #2718 HMAC parity test to booking/airbnb/esignature webhooks | pm-integration | medium | pm-analysis 2026-08-10 |
| pm-scrum-master-shepherd-accounting-mvp-trio | Shepherd merge of accounting MVP trio (#2555/#2558/#2559) — 13 days idle | pm-tech-lead | medium | pm-analysis 2026-08-10 |
| pm-scrum-master-shepherd-2684-workflow-cond-fix | Shepherd merge of PR #2684 (workflow-cond-parse-failopen) | pm-tech-lead | medium | pm-analysis 2026-08-10 |
| pm-security-alexa-webhook-signature-verify | Alexa voice webhook accepts forged requests — verify_alexa_signature never checks HMAC | pm-security | medium | pm-analysis 2026-08-10 |
| pm-security-reality-web-tenant-config-inline-escape | reality-web layout.tsx inlines tenant-config JSON into `<script>` without escaping | pm-security | medium | pm-analysis 2026-08-10 |
| pm-tech-lead-follow-up-2483-add-evidence-idor-verify | Verify PR #2712 fully covers add_evidence IDOR (#2483) and close if satisfied | pm-tech-lead | medium | pm-analysis 2026-08-10 |
| pm-mobile-layout-cache-tenant-scoping | Follow-up #2486: namespace mobile LAYOUT_CACHE_KEY by tenant_id + purge on logout | pm-mobile | medium | pm-analysis 2026-08-10 |
| pm-tech-lead-oauth-state-single-use-atomic-redis | Follow-up #2241: OAuth state single-use not atomic in prod Redis (use GETDEL) | pm-tech-lead | medium | pm-analysis 2026-08-10 |
| pm-tech-lead-report-schedule-consumer-rls-fix | Follow-up #2318: report_schedules due-work consumer uses RLS no-op pool | pm-tech-lead | medium | pm-analysis 2026-08-10 |
| pm-tech-lead-direct-s3-upload-idor-hardening | Follow-up #2320: harden direct-to-S3 upload flow (IDOR/size cap/orphans) | pm-tech-lead | medium | pm-analysis 2026-08-10 |
| pm-data-layout-publish-event-tracking | Define layout publish/webhook analytics events | pm-data | medium | pm-analysis 2026-08-10 |
| pm-data-dispute-lifecycle-kpi-set | Define dispute-lifecycle KPI set (funnel + TTR + evidence-per-dispute) | pm-data | medium | pm-analysis 2026-08-10 |
| pm-data-support-staff-read-audit-event-schema | Publish support-staff read audit-event schema | pm-data | medium | pm-analysis 2026-08-10 |
| pm-data-retention-policy-support-audit | Publish data-retention policy for support_tooling_events / audit trail | pm-data | medium | pm-analysis 2026-08-10 |
| pm-data-formalize-faultstatuscount | Formalize FaultStatusCount canonical definition | pm-data | medium | pm-analysis 2026-08-10 |
| pm-data-onboarding-tour-funnel | Instrument signup/onboarding-tour completion funnel (10b-6) | pm-data | medium | pm-analysis 2026-08-10 |
| screen-map-link-uc-33-1 | Link UC-33.1 (dispute sub-UC) to a dispute screen-map | pm-frontend | low | gap-scan 2026-08-10 |
| screen-map-link-uc-33-2 | Link UC-33.2 (dispute sub-UC) to a dispute screen-map | pm-frontend | low | gap-scan 2026-08-10 |
| screen-map-link-uc-33-3 | Link UC-33.3 (dispute sub-UC) to a dispute screen-map | pm-frontend | low | gap-scan 2026-08-10 |
| pm-devops-churn-reality-server-state | Churn hotspot #1: reality-server/src/state.rs (1201 lines) — split proposal | pm-tech-lead | low | hotspot-scan 2026-08-10 |
| pm-devops-churn-reality-server-agencies | Churn hotspot #2: reality-server/src/routes/agencies.rs (624 lines) — split | pm-tech-lead | low | hotspot-scan 2026-08-10 |
| pm-devops-churn-reality-web-layout-revalidate-test | Churn hotspot #3: reality-web layout-revalidate route.test.ts (486 lines) | pm-tech-lead | low | hotspot-scan 2026-08-10 |
| pm-devops-triage-closed-not-merged-2705 | Triage closed-not-merged PR #2705 (invalid rust-toolchain 1.100.0) | pm-devops | low | pm-analysis 2026-08-10 |
| pm-tech-lead-console-warn-error-cleanup-websocket | 10 ungated console.warn/error in ppt-web websocket.ts leak diagnostics | pm-tech-lead | low | pm-analysis 2026-08-10 |
| pm-devops-batch-merge-dependabot-window | 12 dependabot PRs open — schedule batch merge window | pm-devops | low | pm-analysis 2026-08-10 |
| pm-devops-cloud-routine-cadence-recovery | Cloud routine cadence recovery — 3d gap 2026-08-07→2026-08-10; add health-check alert | pm-devops | low | pm-analysis 2026-08-10 |
| pm-tech-lead-gh-issue-2556-reality-api-client-drift-gate | gh-issue-2556: add reality-api-client drift gate (extend #2569) | pm-tech-lead | low | pm-analysis 2026-08-10 |
