---
id: ppt/iot-alerts
name: IoT Sensor Alerts
product: ppt
implementations:
  ppt-web:
    route: "/iot/alerts"
    component: SensorAlertsPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: partial
endpoints: []
relatedScreens:
  - id: ppt/iot-sensors
    rel: sibling
  - id: ppt/iot-dashboard
    rel: sibling
  - id: ppt/iot-thresholds
    rel: sibling
sharedComponents: []
diagrams: []
useCases: []
epics:
  - "14"
designSources: []
owner: pm-frontend
---

# IoT Sensor Alerts

Org-wide sensor alert management surface (Epic 14, FR74). Reachable from the
**Sensors & Devices** list header ("Alerts" button) and from the command
palette (`nav-iot-alerts`).

## Routes

- `/iot/alerts` — `SensorAlertsPage`: a table of all threshold alerts across
  all sensors in the organisation with acknowledge and resolve actions.

## API

Backend endpoints (not yet registered in `@ppt/sitemap`, so `endpoints: []`):

- `GET  /api/v1/iot/sensors/alerts` → `{ alerts: SensorAlert[] }`
  (The backend list-alerts handler accepts `AlertQuery` params: `sensor_id`,
  `building_id`, `severity`, `resolved`, `acknowledged`, `from_time`, `to_time`,
  `limit`, `offset`.)
- `POST /api/v1/iot/sensors/alerts/{aid}/acknowledge` → `SensorAlert`
- `POST /api/v1/iot/sensors/alerts/{aid}/resolve` → `SensorAlert`

Alert fields: `id`, `sensor_id`, `threshold_id`, `severity` (warning | critical),
`triggered_value`, `threshold_value`, `message`, `triggered_at`, `resolved_at`,
`acknowledged_by`, `acknowledged_at`.

## States

- **Empty**: copy confirming all sensors are within thresholds.
- **Loading**: spinner.
- **Open alerts**: Ack + Resolve buttons enabled.
- **Acknowledged alerts**: only Resolve button shown.
- **Resolved alerts**: resolved timestamp shown; no action buttons.

## Notes

### Specific (recent)
- 2026-06-21 — [BIT-146](/BIT/issues/BIT-146): created dedicated alerts page
  as the third increment of the IoT module (Epic 14 / FR74).

## Agent Log
- 2026-06-21 — FrontendEngineer: created (BIT-146, Epic 14 / FR74).
