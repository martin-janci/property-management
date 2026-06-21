---
id: ppt/iot-thresholds
name: IoT Sensor Thresholds
product: ppt
implementations:
  ppt-web:
    route: "/iot/sensors/:sensorId/thresholds"
    component: SensorThresholdPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: partial
endpoints: []
relatedScreens:
  - id: ppt/iot-sensors
    rel: parent
  - id: ppt/iot-dashboard
    rel: sibling
sharedComponents: []
diagrams: []
useCases: []
epics:
  - "14"
designSources: []
owner: pm-frontend
---

# IoT Sensor Thresholds

Per-sensor threshold configuration surface (Epic 14, FR73). Reached from the
**Sensors & Devices** list via the Thresholds row action.

## Routes

- `/iot/sensors/:sensorId/thresholds` — `SensorThresholdPage`: lists all alert
  rules for the sensor and provides an inline form to add, edit, or delete rules.

## API

Backend endpoints (not yet registered in `@ppt/sitemap`, so `endpoints: []`):

- `GET  /api/v1/iot/sensors/{id}/thresholds` → `{ thresholds: SensorThreshold[] }`
- `POST /api/v1/iot/sensors/{id}/thresholds` → `SensorThreshold`
- `PUT  /api/v1/iot/sensors/thresholds/{tid}` → `SensorThreshold`
- `DELETE /api/v1/iot/sensors/thresholds/{tid}` → 204

Fields per threshold: `metric`, `comparison` (>, >=, <, <=, ==, !=),
`warning_value`, `warning_high`, `critical_value`, `critical_high`,
`enabled`, `alert_cooldown_minutes`.

## States

- **Empty**: copy + CTA to add first rule.
- **Loading**: spinner while fetching the threshold list.
- **Inline form**: shown for create and edit without a full page transition.
- **Enabled toggle**: clicking the badge calls `PUT …/thresholds/{tid}` with
  `{ enabled: !current }` — no confirmation dialog.
- **Delete**: `window.confirm` guard → `DELETE …/thresholds/{tid}`.

## Notes

### Specific (recent)
- 2026-06-21 — [BIT-146](/BIT/issues/BIT-146): created threshold config UI as
  the second increment of the IoT module (Epic 14 / FR73).

## Agent Log
- 2026-06-21 — FrontendEngineer: created (BIT-146, Epic 14 / FR73).
