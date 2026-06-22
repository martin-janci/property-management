---
id: ppt/notification-analytics
name: Notification Analytics
product: ppt
implementations:
  ppt-web:
    route: /notifications/analytics
    component: NotificationAnalyticsPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: complete
  mobile:
    buildStatus: n/a
    redesignStatus: n/a
    apiStatus: n/a
endpoints: []
relatedScreens: []
useCases:
  - UC-01
sharedComponents:
  - kpi-card
  - data-table
  - channel-filter
  - time-window-filter
epics: []
diagrams: []
owner: pm-frontend
---

## Functionality Checklist

<!-- tag with [w] / [m] / [w,m] / [-] -->

### Filters (web)
- [x] [w] Time-window selector (`15m` / `24h` / `7d` / `30d`) → `after` query param
- [x] [w] Channel selector (all / `push` / `email` / `in_app`) → `channel` query param
- [x] [w] Operator-only gating — non-operators see an access notice (mirrors `isManagerRole`)

### KPIs (web)
- [x] [w] Attempted (`sent + failed`)
- [x] [w] Sent
- [x] [w] Failed (highlighted red when > 0)
- [x] [w] Aggregate failure rate (highlighted red when the alert is firing)

### Alert (web)
- [x] [w] Prominent `role="alert"` banner when `alert.firing` (>5% failure rate over ≥ min attempts)

### Per-channel table (web)
- [x] [w] One row per channel: sent / delivered / failed / skipped / opened / clicked / failure rate
- [x] [w] Per-row failure rate highlighted when above the alert threshold
- [x] [w] `opened` / `clicked` render now (currently 0 — engagement tracking is a later increment)

## States

- **Empty**: when no channel had activity in the window (`by_channel` is empty), a single-line "No notification activity in the selected window." is shown (filters + totals retained).
- **Loading**: centered spinner with `aria-busy` while `useNotificationAnalytics` is fetching.
- **Error**: inline `role="alert"` danger tile ("Failed to load notification analytics") + `common.retry` button wired to `refetch()`.
- **Forbidden**: non-operators (and a backend `403` when the principal lacks `audit_read`) see an amber `role="alert"` access notice and no data/table.

## Notes

### Broader context

Operator dashboard for notification delivery (Story 2B-C.3, PM #969 gap 4,
BIT-214). Consumes `GET /api/v1/admin/notifications/analytics` (capability-gated
`audit_read`) shipped in BIT-180 (PR #1710): per-channel delivery/failure rates
over a selectable window plus a >5% failure-rate alert. Follow-up of the
notification delivery-tracking persistence layer.

### Specific (recent)

- **Endpoint not catalogued**: the admin notifications analytics endpoint has no
  `operationId` in `@ppt/sitemap` and is not in the generated `@ppt/api-client`
  (it follows the `admin/audit` precedent), so `endpoints: []` here to keep
  `/screens validate` green and the hook calls the REST path directly via
  TanStack Query. Add a `notifications_analytics` operationId to the sitemap and
  reference it when the catalogue is extended.
- **Route not in sitemap**: `/notifications/analytics` is not in `@ppt/sitemap`
  `pptWebRoutes`, so `sitemapRefs` is omitted. Reachable from the main nav via an
  operator-only "Notification Analytics" link.
- Auth: the bearer token is read from `localStorage` (`ppt_access_token`) in the
  fetch helper, matching `authApiClient` / `gdprClient`, because the handler
  requires authentication.

## Agent Log

<!-- newest entries on top -->

- 2026-06-22 — agent: BIT-214 (FrontendEngineer) — created from scratch for Story 2B-C.3. New `/notifications/analytics` route + `NotificationAnalyticsPage` + `useNotificationAnalytics` (direct REST, `admin/audit` precedent) + `routes/groups/notifications.tsx` + operator-only nav link. Per-channel table, totals KPIs, firing >5% failure-rate alert banner, channel + time-window filters. ppt-web buildStatus shipped / apiStatus complete; mobile n/a. Page tests added (6 cases); typecheck (own files) + biome + vitest green.
