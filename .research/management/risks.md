# Risks

_Generated: 2026-10-07T12:40:00Z — refreshed from `risks.json` (routine Phase 1.6, pm-data rotation)._

22 open risks. New entries this run: `pm-data-engagement-event-specs-missing`, `pm-data-fcm-stub-inflates-sent-count`, `pm-scrum-master-buffer-starvation-backend-heavy`.

| ID | P | I | Owner | Risk |
|---|---|---|---|---|
| `pm-devops-swagger-ui-egress-blocking-security-2026-10-01` | high | high | pm-devops | utoipa-swagger-ui egress 403 (#2966) renders api-server structurally unbuildable in the cloud runner — every backend-touching dispatcher task is CLOUD-BUILD-BLOCKED, including all 3 open cross-tenant  |
| `pm-security-portfolio-analytics-idor-persistent-2026-10-01` | medium | high | pm-security | Cross-tenant IDOR #2946 (portfolio_analytics property metrics computed org_id then discarded) is on retry 2/2 and still open — a tier-1 security defect that has survived two implementer attempts; a su |
| `pm-tech-lead-auth-header-triplicate-drift-2026-10-01` | medium | medium | pm-tech-lead | @ppt/api-client has triplicated auth-header + MFA-retry logic across fetch primitives (gdprClient, paymentMatching, remaining consumers) — surfaced by post-merge review of PR #3004 and tracked as #300 |
| `pm-scrum-master-accounting-mvp-trio-reviewer-starvation-2026-07-30` | medium | medium | pm-tech-lead | Accounting MVP-loop trio (#2555 / #2558 / #2559) has been sitting at 64d with no reviewer engagement — dispatcher stack is still starving on reviewer capacity, not implementer capacity. Risk is now ag |
| `risk-layout-webhook-replay-2026-07-23` | medium | high | pm-security | Layout publish webhook lacks timestamp/replay protection (#2485) — a captured legitimate publish can be replayed to overwrite a newer layout |
| `risk-mobile-layout-cache-cross-tenant-2026-07-23` | medium | medium | pm-security | Mobile LAYOUT_CACHE_KEY is not tenant-scoped and survives logout (#2486) — user A's layout can leak to user B on shared device or org-switch |
| `risk-dispute-add-evidence-idor-lingers-2026-07-23` | low | high | pm-security | add_evidence dispute sub-resource remains cross-tenant-writable until PR #2490 lands (#2483) — PR #2450 fixed 5 handlers but missed the sub-route |
| `risk-announcement-fanout-test-fidelity-2026-07-23` | medium | high | pm-qa | Announcement cross-tenant fan-out guard is tested only via a pure-Rust re-model, not the real SQL (#2484) — the SQL predicate could regress without the test catching it |
| `risk-analytics-blindspots-shipped-mvp-2026-07-23` | high | medium | pm-data | Multiple shipped MVP features (Epic 6, 10A, 10B, 80, 84) lack KPI instrumentation — product/business decisions run blind |
| `risk-data-retention-policy-missing-2026-07-23` | medium | medium | pm-data | Append-only support_tooling_events + support-data audit trail have ON DELETE RESTRICT but no TTL — long-term storage/GDPR compliance risk |
| `risk-metric-definition-drift-2026-07-23` | high | medium | pm-data | FaultStatusCount metric (support-data) diverges from owner/portfolio fault KPIs — dashboards will disagree with each other |
| `risk-webhook-cross-integration-drift-2026-07-23` | medium | high | pm-integration | Webhook handlers across integrations (booking, airbnb, esignature, layout) lack consistent hardening — #2485 shows layout has no replay guard; unknown parity elsewhere |
| `pm-scrum-master-delete-by-file-key-samesorg-ref-gap-2026-07-30` | medium | high | pm-backend | PR #2571 (DELETE-by-file-key) landed with a same-org reference-check gap (#2573) — an active same-org file key can be deleted out from under a live document row |
| `pm-scrum-master-android-sso-csrf-half-wired-2026-07-30` | high | high | pm-mobile | PR #2568 CSRF state fix is non-functional (#2574) — SsoStateStore.mint() has no call site so every reality://sso callback is rejected |
| `pm-backend-disputes-kpis-window-validation-gap-2026-07-30` | medium | medium | pm-backend | Dispute KPI endpoint (#2572 → #2575) is quarantined-test-only in main; a shape regression could ship undetected until reporting starts consuming it |
| `pm-scrum-master-voice-webhooks-churn-persists-2026-08-25` | medium | medium | pm-tech-lead | backend/servers/api-server/src/routes/voice_webhooks.rs is on its 3rd hotspot window running; PR #2838 centralized token encryption but scheduler/OAuth-refresh/token-vend paths still cluster — repeate |
| `pm-qa-mobile-rn-lint-gap-hooks-i18n-2026-08-25` | high | medium | pm-frontend | 3 mobile-rn PRs fixed defects that eslint-plugin-react-hooks + a no-hardcoded-strings rule would catch statically — every future mobile-rn PR risks reintroducing them |
| `pm-devops-mobile-native-cloud-starvation` | high | medium | pm-devops | Buffer starvation becomes chronic — every new mobile-native/KMP finding adds an unclaimable item to the cloud dispatcher's pool, now compounded by #2966 forcing claimable=9/72 |
| `pm-devops-research-land-workflow-silent-break` | low | high | pm-devops | `research-land.yml` is the only path routine commits reach `dev` from cloud session branches. A silent workflow break would let `.research/` commits accumulate on session branches without landing |
| `pm-data-engagement-event-specs-missing` | high | medium | pm-data | Epics 6/7A/8A shipped with no engagement/delivery event specs — adoption and notification KPIs cannot be measured and late-added tracking will not cover earlier usage. |
| `pm-data-fcm-stub-inflates-sent-count` | high | high | pm-data | Notification sent counts overstated — the FCM stub silently swallows failures (issue #484, still open), so any delivery KPI built on the current counter is wrong. 8a-3 is marked done despite open gate |
| `pm-scrum-master-buffer-starvation-backend-heavy` | high | high | pm-scrum-master | Dispatcher buffer starvation (claimable 16/72) persists because most remaining open work is backend/KMP, which cloud cannot build until #3026 is confirmed clean and #2652 lands. |
