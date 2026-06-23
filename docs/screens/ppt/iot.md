---
id: ppt/iot
name: IoT / Smart Building — Module Map
product: ppt
implementations:
  ppt-web:
    route: "/iot"
    component: iotRoutes
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: partial
endpoints: []
relatedScreens:
  - id: ppt/iot-dashboard
    rel: child
  - id: ppt/iot-sensors
    rel: child
sharedComponents:
  - SensorStatusBadge
diagrams: []
useCases: []
epics:
  - "14"
designSources: []
owner: pm-frontend
---

# IoT / Smart Building — Module Map

Module-level index for the Epic 14 IoT / Smart-Building surface (FR71–FR75) in
ppt-web. Every IoT screen is mounted by `routes/groups/iot.tsx` (`iotRoutes()`)
and wired to the `@ppt/api-client` IoT module
(`frontend/packages/api-client/src/iot/{api,hooks,types,index}.ts`) over the
`/api/v1/iot/*` REST API. Sibling screen-maps:
[IoT dashboard](./iot-dashboard.md) and
[IoT sensors](./iot-sensors.md).

## Surfaces

| Route | Page / component | Backing `/api/v1/iot/*` endpoint(s) |
| --- | --- | --- |
| `/iot` | `IotDashboardPage` (route wrapper `IotDashboardPageRoute`) | `GET /dashboard`, `GET /sensors`, `GET /sensors/:id/readings`, `POST /alerts/:aid/acknowledge`, `POST /alerts/:aid/resolve` |
| `/iot/sensors` | `SensorListPage` (lazy `IotSensorListPage`) | `GET /sensors`, `DELETE /sensors/:id` |
| `/iot/sensors/new` | `SensorFormPage` `mode="create"` (lazy `IotSensorFormPage`) | `POST /sensors` |
| `/iot/sensors/:sensorId/edit` | `SensorFormPage` `mode="edit"` | `GET /sensors/:id`, `PUT /sensors/:id` |
| `/iot/sensors/:sensorId/thresholds` | _planned_ — thresholds config UI ([BIT-148](/BIT/issues/BIT-148)) | `GET/POST /sensors/:id/thresholds`, `PUT/DELETE /sensors/thresholds/:tid` |
| `/iot/alerts` | `IotAlertsPage` (route wrapper `IotAlertsPageRoute`) | `GET /dashboard` (`recent_alerts`), `POST /alerts/:aid/acknowledge`, `POST /alerts/:aid/resolve` |

> Endpoint ids are not registered in `@ppt/sitemap` for IoT yet, so the
> front-matter `endpoints` list stays empty and the routes are documented in
> prose above (same convention as the sibling IoT screen-maps).

## Alerts page (`/iot/alerts`)

Standalone full-page view of threshold-breach alerts (FR74), reachable from the
top nav ("Sensor Alerts"), the command palette ("Go to Sensor Alerts"), and
alongside the dashboard's inline `AlertsPanel`.

- **Source (v1):** alerts come from the dashboard rollup
  (`useIotDashboard().recent_alerts`). The presentational `IotAlertsPage`
  takes the alert array + handlers in, so a dedicated cross-sensor alerts list
  endpoint can replace that source later without touching the component. The
  per-sensor `useSensorAlerts` hook remains available for sensor-scoped views.
- **Table columns:** severity badge, sensor name (resolved via `useSensors`),
  message, triggered value / threshold, triggered timestamp, lifecycle state,
  and ack/resolve actions.
- **Filters (client-side):** state (open / acknowledged / resolved) and
  severity (critical / warning / info).
- **Actions:** `useAcknowledgeSensorAlert` and `useResolveSensorAlert` mutations
  acknowledge / resolve in place; the affected row disables while the mutation
  is in flight and the IoT caches invalidate on success.

## States

- **Empty**: each surface renders empty copy when its query returns no rows; the
  alerts page shows "No alerts match these filters. All clear." when the filter
  set yields nothing.
- **Loading**: per-query spinners (dashboard rollup, sensor list, alert list).
- **Saving**: ack/resolve and sensor CRUD buttons disable while their mutation
  is pending.
- **Error**: ack/resolve and sensor CRUD mutation failures surface as error
  toasts (`iot.acknowledgeFailed` / `iot.resolveFailed` / the CRUD failure keys)
  from the `routes/groups/iot.tsx` wrappers. The alerts page additionally renders
  a distinct error panel with a Retry affordance when its source query
  (`useIotDashboard`) fails (`iot.alerts.loadError` + `common.retry`), so a hard
  load failure is not misreported as the "All clear" empty state. `apiStatus:
  partial` until the IoT backend is verified end-to-end.

## Notes

### Specific (recent)
- 2026-06-21 — [BIT-149](/BIT/issues/BIT-149) (parent
  [BIT-146](/BIT/issues/BIT-146), Epic 14): added the standalone alerts page
  (`/iot/alerts`, `IotAlertsPage`) with ack/resolve + state/severity filters,
  the nav + command-palette links, `iot.*` i18n keys across all 6 languages, and
  this module-level screen-map. Thresholds UI
  ([BIT-148](/BIT/issues/BIT-148)) remains a later increment.

## Agent Log
- 2026-06-21 — FrontendEngineer: created the IoT module map with the standalone
  alerts page increment (BIT-149, Epic 14 / FR74).
- 2026-06-23 — agent: surfaced ack/resolve mutation failures as toasts on the
  alerts + dashboard wrappers and added a distinct `isError` panel with Retry to
  `IotAlertsPage` (issue #1669); reconciled the States > Error section. Deferred:
  the dedicated cross-sensor alerts list endpoint + `recent_alerts` source limit.
