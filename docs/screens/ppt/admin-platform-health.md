---
id: ppt/admin-platform-health
name: Platform Health Monitoring (Admin)
product: ppt
sitemapRefs: {}
implementations:
  ppt-web:
    route: platform/health
    component: PlatformHealthPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: complete
endpoints:
  - get_health_dashboard
  - get_health_alerts
  - acknowledge_health_alert
  - get_metric_history
  - update_health_threshold
relatedScreens:
  - id: ppt/admin-oauth-clients
    rel: sibling
sharedComponents: []
diagrams: []
useCases:
  - UC-17
epics:
  - Epic-10B
designSources: []
owner: pm-frontend
---

## Functionality Checklist

- [w] Dashboard section: current metrics grid with status badges (normal / warning / critical)
- [w] Drill into metric history via slide-over panel with 5 time-range options (1h / 6h / 24h / 7d / 30d)
- [w] Stats summary (min / max / avg / count) above time-series data-points table
- [w] Active alerts table with toggle between "Active only" and "All alerts"
- [w] Acknowledge alert action gated by `site_settings_write` capability
- [w] Metric thresholds table with inline edit dialog (warning + critical values)
- [w] Threshold edit gated by `site_settings_write` capability
- [w] Auto-refresh every 60 seconds; manual Refresh button
- [w] Error banner if dashboard load fails

## States

- **Loading**: "Loading…" text in metrics section
- **Error**: Red alert banner with i18n key `admin.health.loadError`
- **Empty metrics**: "No metrics recorded yet." message
- **No active alerts**: "No active alerts." message
- **No thresholds**: "No thresholds configured." message

## Notes

### Broader context

Part of Epic 10B Story 10B.3. Provides the super-admin operator view of platform
health. Backend routes live under `/api/v1/platform-admin/health/*`. All requests
go through `@ppt/api-client`'s shared `authenticatedFetchJson` factory
(`lib/fetch.ts`), which carries the MFA challenge-response interceptor.

### Specific (recent)

- 2026-05-27 — agent: gap-10b-3-health-ui-mfa-fix — MFA intercept baked into
  shared `authenticatedFetchJson`; `admin/api.ts` local `apiRequest` removed;
  `lib/mfa-handler.ts` created; `admin/mfa-handler.ts` re-exports for BC.
- 2026-05-24 — agent: gap-10b-3-admin-health-ui — replaced inline `fetchJson`
  helper with `apiRequest`-based hooks in `@ppt/api-client` admin module;
  removed `token` prop threading; biome + typecheck green.

## Agent Log
- 2026-07-13 — agent: gap-screens-normalize-frontmatter — normalized story-id-style epic ref(s) Epic-10B-3 → Epic-10B (strip story suffix); /screens validate clean.

<!-- newest entries on top -->
- 2026-05-28 — agent: gap-10b-3-health-ui-mfa-bypass-fix — added unit tests
  for `authenticatedFetchJson` MFA interception (`lib/fetch.test.ts`): covers
  401 mfa_required → handler called → retry-once, handler returns false →
  throw, repeated 401 after retry → no loop, non-mfa 401 → handler not called.
  Added regression test to `PlatformHealthPage.test.tsx` verifying dashboard
  401 triggers MFA handler and retries (PR #471 bypass regression guard).
- 2026-05-27 — agent: gap-10b-3-health-ui-mfa-fix — added 401 mfa_required
  intercept to shared `authenticatedFetchJson` in `lib/fetch.ts`; refactored
  `admin/api.ts` to use it (removed duplicate local `apiRequest`); created
  `lib/mfa-handler.ts` as canonical home; `admin/mfa-handler.ts` now re-
  exports from lib for backward compatibility.
- 2026-05-24 — agent: gap-10b-3-admin-health-ui — MUST fix: replaced fetchJson
  with useHealthDashboard / useHealthAlerts / useAcknowledgeAlert /
  useMetricHistory / useUpdateHealthThreshold hooks; all routes through
  apiRequest MFA interceptor; token prop removed from MetricHistoryPanel and
  ThresholdsTable; created screen-map.
- 2026-05-24 — agent: gap-10b-3-admin-health-ui — initial implementation of
  PlatformHealthPage with dashboard, alerts, thresholds, and metric history
  drill-down; admin-web App.tsx route wired at platform/health.
