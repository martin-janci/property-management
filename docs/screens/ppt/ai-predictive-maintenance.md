---
id: ppt/ai-predictive-maintenance
name: Predictive Maintenance Dashboard
product: ppt
implementations:
  ppt-web:
    route: "/ai/predictive-maintenance"
    component: PredictiveMaintenancePage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: partial
endpoints: []
relatedScreens:
  - id: ppt/ai-dashboards
    rel: parent
  - id: ppt/ai-sentiment
    rel: sibling
sharedComponents: []
diagrams: []
useCases: []
epics:
  - "13"
designSources: []
owner: pm-frontend
---

# Predictive Maintenance Dashboard

Manager-web surface for Epic 13 Story 13.3 — AI-powered equipment health
monitoring and failure predictions. Presentational `PredictiveMaintenancePage`
is wired by `routes/groups/ai-dashboards.tsx` (`PredictiveMaintenanceRoute`) to
the `features/predictive-maintenance` hooks (`useEquipmentList`,
`useMaintenancePredictions`, `useAcknowledgeMaintenancePrediction`), which call
the `/api/v1/ai/equipment/*` REST API through the shared `getApiClient()` axios
client.

- KPI rollup tiles (total equipment, needs-maintenance, high-risk open,
  total predictions).
- Equipment list with status badges; selecting an item scopes the predictions.
- Predictions list with risk bands, predicted-failure ETA, confidence, and
  per-prediction acknowledge.

## States

- **Auth gate**: renders `<AuthRequiredGate />` until `user.organizationId`.
- **Empty**: equipment list shows "No equipment found."; predictions list shows
  "No high-risk predictions."
- **Loading**: per-query loading flags drive skeleton placeholders.
- **Saving**: acknowledge button disables while the mutation is pending.
- **Error**: query/mutation errors surface as `ApiError`. `apiStatus: partial`
  until the equipment backend is verified end-to-end.

## Notes

### Specific (recent)
- 2026-06-23 — #1674: replaced hand-rolled `fetch` with `getApiClient()`
  (auth/error/retry), added the org-scoping gate, and extracted all strings into
  `predictive.*` i18n keys (en/sk/cs/de/pl/hu), including translated equipment
  status / risk-band labels.

## Agent Log
- 2026-06-23 — FrontendEngineer: created this screen-map as part of the #1674
  follow-up (Epic 13 / Story 13.3).
