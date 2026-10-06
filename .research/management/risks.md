# Risks

_Generated: 2026-10-06T06:30:00Z — refreshed from `risks.json` (routine Phase 1.6 upkeep, pm-data rotation)._

21 open risks. Two new entries this run from the pm-data lens: `silent-dropped-input-no-contract-test` (half of this week's merged PRs are this class) and `review-feedback-telemetry-dark` (reviewer notes are growing but nothing aggregates them). All prior risks carry forward.

| ID | P | I | Owner | Risk |
|---|---|---|---|---|
| `pm-devops-swagger-ui-egress-blocking-security-2026-10-01` | high | high | pm-devops | #2966 utoipa-swagger-ui egress 403 blocks every backend PR — fix in draft PR #3026 awaiting human merge |
| `pm-security-portfolio-analytics-idor-persistent-2026-10-01` | medium | high | pm-security | #2946 IDOR on portfolio_analytics still open on retry 2/2 — tier-1 security defect, parked behind #2966 |
| `pm-tech-lead-auth-header-triplicate-drift-2026-10-01` | medium | medium | pm-tech-lead | #3006 triplicated auth-header + MFA-retry in @ppt/api-client — drift risk |
| `pm-scrum-master-accounting-mvp-trio-reviewer-starvation-2026-07-30` | medium | medium | pm-tech-lead | Accounting trio #2555/#2558/#2559 idle 68d — reviewer starvation |
| `pm-data-silent-dropped-input-no-contract-test-2026-10-06` | high | medium | pm-qa | reality-web UI silently drops user input — 3 of 6 PRs this run are this class (#3019/#3021/#3022); no end-to-end data-contract assertion |
| `pm-data-review-feedback-telemetry-dark-2026-10-06` | medium | low | pm-data | reviewer_summary.verdict notes grow in assignments.json; no aggregation = trend data stays dark |
| `risk-layout-webhook-replay-2026-07-23` | medium | high | pm-security | Layout publish webhook lacks replay protection (#2485) |
| `risk-mobile-layout-cache-cross-tenant-2026-07-23` | medium | medium | pm-security | Mobile LAYOUT_CACHE_KEY not tenant-scoped (#2486) |
| `risk-dispute-add-evidence-idor-lingers-2026-07-23` | low | high | pm-security | add_evidence sub-resource cross-tenant-writable until #2490 lands |
| `risk-announcement-fanout-test-fidelity-2026-07-23` | medium | high | pm-qa | Announcement fan-out guard tested via pure-Rust re-model, not real SQL |
| `risk-analytics-blindspots-shipped-mvp-2026-07-23` | high | medium | pm-data | Shipped MVP features lack KPI instrumentation |
| `risk-data-retention-policy-missing-2026-07-23` | medium | medium | pm-data | Support-data audit trail has no TTL — GDPR risk |
| `risk-metric-definition-drift-2026-07-23` | high | medium | pm-data | FaultStatusCount metric diverges from portfolio KPIs |
| `risk-webhook-cross-integration-drift-2026-07-23` | medium | high | pm-integration | Webhook handlers across integrations lack consistent hardening |
| `pm-scrum-master-delete-by-file-key-samesorg-ref-gap-2026-07-30` | medium | high | pm-backend | PR #2571 DELETE-by-file-key missing same-org reference-check |
| `pm-scrum-master-android-sso-csrf-half-wired-2026-07-30` | high | high | pm-mobile | PR #2568 CSRF state fix is non-functional (#2574) |
| `pm-backend-disputes-kpis-window-validation-gap-2026-07-30` | medium | medium | pm-backend | Dispute KPI endpoint quarantined-test-only in main |
| `pm-scrum-master-voice-webhooks-churn-persists-2026-08-25` | medium | medium | pm-tech-lead | voice_webhooks.rs on 3rd hotspot window — latent design defect |
| `pm-qa-mobile-rn-lint-gap-hooks-i18n-2026-08-25` | high | medium | pm-frontend | Missing eslint-plugin-react-hooks + i18n lint on mobile RN |
| `pm-devops-mobile-native-cloud-starvation` | high | medium | pm-devops | Buffer starvation chronic — compounded by #2966 + #2652 |
| `pm-devops-research-land-workflow-silent-break` | low | high | pm-devops | research-land.yml silent break would strand `.research/` commits |
