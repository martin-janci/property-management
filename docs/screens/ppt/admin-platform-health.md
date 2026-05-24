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
  - ppt/admin-agencies
  - ppt/admin-oauth-clients
sharedComponents: []
diagrams: []
useCases:
  - UC-17
epics:
  - Epic-10B-3
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
go through `@ppt/api-client`'s `apiRequest` helper, which carries the MFA
challenge-response interceptor.

### Specific (recent)

- 2026-05-24 — agent: gap-10b-3-admin-health-ui — replaced inline `fetchJson`
  helper with `apiRequest`-based hooks in `@ppt/api-client` admin module;
  removed `token` prop threading; biome + typecheck green.

## Agent Log

<!-- newest entries on top -->
- 2026-05-24 — agent: gap-10b-3-admin-health-ui — MUST fix: replaced fetchJson
  with useHealthDashboard / useHealthAlerts / useAcknowledgeAlert /
  useMetricHistory / useUpdateHealthThreshold hooks; all routes through
  apiRequest MFA interceptor; token prop removed from MetricHistoryPanel and
  ThresholdsTable; created screen-map.
- 2026-05-24 — agent: gap-10b-3-admin-health-ui — initial implementation of
  PlatformHealthPage with dashboard, alerts, thresholds, and metric history
  drill-down; admin-web App.tsx route wired at platform/health.
