---
id: ppt/iot-dashboard
name: IoT / Smart-Building Dashboard
product: ppt
implementations:
  ppt-web:
    route: "/iot"
    component: IotDashboardPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: partial
endpoints: []
relatedScreens: []
sharedComponents: []
diagrams: []
useCases: []
epics:
  - Epic-14
designSources: []
owner: pm-frontend
---

# IoT / Smart-Building Dashboard

The manager-web front door for the Epic 14 IoT backend (FR71–75). Backend
modules + migrations were complete but had **zero UI on any platform**
([PAP-18](/PAP/issues/PAP-18) gap #3); PAP-22 builds and mounts the web surface.

Presentational `IotDashboardPage` is wired by `routes/groups/iot.tsx` to the
`@ppt/api-client` IoT module (`useIotDashboard`, `useSensors`,
`useSensorReadings`, `useAcknowledgeAlert`, `useResolveAlert`):

- **FR71** — device/sensor list (`SensorListPanel`).
- **FR72** — per-device telemetry readings (`TelemetryPanel`).
- **FR74** — threshold alerts with acknowledge/resolve (`AlertsPanel`).
- **FR75** — KPI rollup tiles (`IotStatCard`).

## States

- **Empty**: rendered when an org has no provisioned sensors — panels show empty
  copy.
- **Loading**: per-panel loading flags from the api-client query hooks
  (`dashboardLoading`, `sensorsLoading`, `readingsLoading`).
- **Error**: query errors surface as empty panels today; `apiStatus: partial`
  until verified end-to-end against the live IoT backend.

## Notes

### Specific (recent)
- 2026-06-08 — PAP-22: built the IoT feature dir + `@ppt/api-client` IoT module,
  mounted `iotRoutes()` in `AppRoutes.tsx` on `/iot`, and created this
  screen-map entry. `/screens validate` green.

## Agent Log
- 2026-06-08 — CTO: created on route mount (PAP-22, Epic 14 / FR71–75).
