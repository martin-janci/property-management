---
id: ppt/iot-sensors
name: IoT Sensors — Registry, Register & Edit
product: ppt
implementations:
  ppt-web:
    route: "/iot/sensors"
    component: SensorListPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: partial
endpoints:
  - "GET /api/v1/iot/sensors"
  - "POST /api/v1/iot/sensors"
  - "GET /api/v1/iot/sensors/{id}"
  - "PUT /api/v1/iot/sensors/{id}"
  - "DELETE /api/v1/iot/sensors/{id}"
relatedScreens:
  - ppt/iot-dashboard
sharedComponents:
  - SensorStatusBadge
diagrams: []
useCases: []
epics:
  - "14"
designSources: []
owner: pm-frontend
---

# IoT Sensors — Registry, Register & Edit

The device-management surface for the Epic 14 IoT backend (FR71). Complements
the read-only [IoT dashboard](./iot-dashboard.md) with full sensor CRUD entry
points, mounted by `routes/groups/iot.tsx` on the `@ppt/api-client` IoT module
(`useSensors`, `useSensor`, `useCreateSensor`, `useUpdateSensor`,
`useDeleteSensor`, `useBuildings`).

## Routes

- `/iot/sensors` — `SensorListPage`: a table of every registered device with
  status, location and last-reading columns, plus register / edit / delete
  actions.
- `/iot/sensors/new` — `SensorFormPage` (`mode="create"`): register a new device
  against a building. `organization_id` is pinned to the caller's tenant
  server-side; `created_by` comes from the authenticated user.
- `/iot/sensors/:sensorId/edit` — `SensorFormPage` (`mode="edit"`): update an
  existing device. Identity fields (building, type, serial number) are
  read-only because the api-server `UpdateSensor` shape does not accept them;
  `status` becomes editable instead.

## States

- **Empty**: list shows empty copy + a register CTA when the org has no sensors.
- **Loading**: per-query loading flags drive spinners (list, form prefill,
  building options).
- **Saving**: submit/delete buttons disable and show progress while the
  mutation is in flight.
- **Error**: create/update/delete failures surface as toasts; `apiStatus:
  partial` until verified end-to-end against the live IoT backend.

## Notes

### Specific (recent)
- 2026-06-21 — [BIT-146](/BIT/issues/BIT-146): added the sensor registry +
  register/edit pages and the `@ppt/api-client` create/update/delete layer
  (`createSensor`/`updateSensor`/`deleteSensor` + hooks). Nav link + command
  palette entries added. Thresholds UI ([BIT-148](/BIT/issues/BIT-148)) and a
  dedicated alerts page ([BIT-149](/BIT/issues/BIT-149)) follow as later
  increments.

## Agent Log
- 2026-06-21 — FrontendEngineer: created with the sensor-CRUD increment
  (BIT-146, Epic 14 / FR71).
