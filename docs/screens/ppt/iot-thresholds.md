---
id: ppt/iot-thresholds
name: IoT Thresholds — Per-sensor Alert Rules & Templates
product: ppt
implementations:
  ppt-web:
    route: "/iot/sensors/:sensorId/thresholds"
    component: IotThresholdConfigPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: partial
endpoints: []
relatedScreens:
  - id: ppt/iot-sensors
    rel: parent
  - id: ppt/iot-dashboard
    rel: sibling
sharedComponents:
  - ThresholdForm
  - ThresholdList
diagrams: []
useCases: []
epics:
  - "14"
designSources: []
owner: pm-frontend
---

# IoT Thresholds — Per-sensor Alert Rules & Templates

The per-sensor alert-threshold configuration surface for the Epic 14 IoT
backend (FR73). Reached from the [IoT sensors registry](./iot-sensors.md) via
the per-row **Thresholds** action, mounted by `routes/groups/iot.tsx` on the
`@ppt/api-client` IoT threshold layer (`useThresholds`, `useCreateThreshold`,
`useUpdateThreshold`, `useDeleteThreshold`, `useThresholdTemplates`,
`useApplyTemplate`, plus `useSensor` for context).

## Routes

- `/iot/sensors/:sensorId/thresholds` — `IotThresholdConfigPage`: lists the
  sensor's configured threshold rules and provides add / edit / delete and an
  enable toggle, plus an **Apply template** picker scoped to the sensor's
  `sensor_type`.

## API

Mounted on `/api/v1/iot/sensors`:

- `GET /{id}/thresholds` — list a sensor's thresholds.
- `POST /{id}/thresholds` — create a threshold rule.
- `PUT /thresholds/{thresholdId}` — update a threshold rule.
- `DELETE /thresholds/{thresholdId}` — delete a threshold rule.
- `GET /templates?sensor_type=…` — list reusable threshold templates.
- `POST /templates/{templateId}/apply` — apply a template to a sensor,
  creating a threshold (`{ sensor_id }`).

(These IoT endpoints are not yet registered in `@ppt/sitemap`, so `endpoints`
is empty and the URLs are documented here in prose.)

## States

- **Empty**: the list shows empty copy prompting the operator to add a
  threshold or apply a template.
- **Loading**: per-query loading flags drive spinners (sensor context,
  threshold list, template options).
- **Saving**: the add/edit form and the apply-template / toggle / delete
  buttons disable and show progress while their mutation is in flight.
- **Error**: create / update / delete / apply failures surface as toasts;
  `apiStatus: partial` until verified end-to-end against the live IoT backend.

## Notes

### Specific (recent)
- 2026-06-21 — [BIT-148](/BIT/issues/BIT-148) (parent
  [BIT-146](/BIT/issues/BIT-146), Epic 14 / FR73): added the threshold
  configuration page (`IotThresholdConfigPage`), `ThresholdForm` +
  `ThresholdList` components, the `@ppt/api-client` threshold + template layer
  (`listThresholds`/`createThreshold`/`updateThreshold`/`deleteThreshold`/
  `listThresholdTemplates`/`applyTemplate` + hooks), the route and a
  Thresholds entry point on the sensor registry. A dedicated alerts page
  ([BIT-149](/BIT/issues/BIT-149)) follows as a later increment.

## Agent Log
- 2026-06-21 — FrontendEngineer: created with the threshold-config increment
  (BIT-148, Epic 14 / FR73).
