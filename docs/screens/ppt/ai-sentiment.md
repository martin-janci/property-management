---
id: ppt/ai-sentiment
name: Tenant Sentiment Dashboard
product: ppt
implementations:
  ppt-web:
    route: "/ai/sentiment"
    component: SentimentDashboardPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: partial
endpoints: []
relatedScreens:
  - id: ppt/ai-dashboards
    rel: parent
  - id: ppt/ai-predictive-maintenance
    rel: sibling
sharedComponents: []
diagrams: []
useCases: []
epics:
  - "13"
designSources: []
owner: pm-frontend
---

# Tenant Sentiment Dashboard

Manager-web surface for Epic 13 Story 13.2 — AI-derived sentiment trends and
alerts from tenant communications. Presentational `SentimentDashboardPage` is
wired by `routes/groups/ai-dashboards.tsx` (`SentimentDashboardRoute`) to the
`features/sentiment` hooks (`useSentimentDashboard`, `useSentimentTrends`,
`useSentimentAlerts`, `useAcknowledgeSentimentAlert`), which call the
`/api/v1/ai/sentiment/*` REST API through the shared `getApiClient()` axios
client.

- KPI rollup tiles (org 30-day average, data points, open/total alerts).
- SVG sparkline trend chart with negative-spike markers (`delta < -0.2`).
- Sentiment alerts panel with per-alert acknowledge.

## States

- **Auth gate**: renders `<AuthRequiredGate />` until `user.organizationId`.
- **Empty**: alerts panel shows "No active alerts."; trend chart shows
  "Not enough data to draw trend." when fewer than 2 points exist.
- **Loading**: per-query loading flags drive skeleton placeholders.
- **Saving**: acknowledge button disables while the mutation is pending.
- **Error**: query/mutation errors surface as `ApiError`. `apiStatus: partial`
  until the sentiment backend is verified end-to-end.

## Notes

### Specific (recent)
- 2026-06-23 — #1674: replaced hand-rolled `fetch` with `getApiClient()`
  (auth/error/retry), added the org-scoping gate, and extracted all strings into
  `sentiment.*` i18n keys (en/sk/cs/de/pl/hu).

## Agent Log
- 2026-06-23 — FrontendEngineer: created this screen-map as part of the #1674
  follow-up (Epic 13 / Story 13.2).
