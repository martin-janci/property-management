---
id: ppt/ai-dashboards
name: AI Dashboards — Module Map
product: ppt
implementations:
  ppt-web:
    route: "/ai"
    component: aiDashboardRoutes
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: partial
endpoints: []
relatedScreens:
  - id: ppt/ai-sentiment
    rel: child
  - id: ppt/ai-predictive-maintenance
    rel: child
sharedComponents: []
diagrams: []
useCases: []
epics:
  - "13"
designSources: []
owner: pm-frontend
---

# AI Dashboards — Module Map

Module-level index for the Epic 13 AI surfaces (Stories 13.2 / 13.3) in ppt-web.
Both screens are mounted by `routes/groups/ai-dashboards.tsx`
(`aiDashboardRoutes()`) and call the `/api/v1/ai/*` REST API through the shared
axios client (`apps/ppt-web/src/lib/api.ts`, `getApiClient()`), which injects the
Bearer token, transforms `ErrorResponse` → `ApiError`, handles `401`, and retries
transient `5xx`/`429`. Sibling screen-maps:
[Tenant Sentiment](./ai-sentiment.md) and
[Predictive Maintenance](./ai-predictive-maintenance.md).

## Surfaces

| Route | Page / component | Backing `/api/v1/ai/*` endpoint(s) |
| --- | --- | --- |
| `/ai/sentiment` | `SentimentDashboardPage` (route wrapper `SentimentDashboardRoute`) | `GET /sentiment/dashboard`, `GET /sentiment/trends`, `GET /sentiment/alerts`, `POST /sentiment/alerts/:id/acknowledge` |
| `/ai/predictive-maintenance` | `PredictiveMaintenancePage` (route wrapper `PredictiveMaintenanceRoute`) | `GET /equipment`, `GET /equipment/predictions`, `POST /equipment/predictions/:id/acknowledge` |

> The AI endpoints are not yet in the OpenAPI spec / generated `@ppt/api-client`,
> so the front-matter `endpoints` list stays empty and the routes are documented
> in prose above (same convention as the IoT screen-maps). Requests still route
> through `getApiClient()`, so auth/error/retry semantics apply; a `typespec`
> slice can later promote these to typed `@ppt/api-client` hooks.

## States

- **Auth gate**: both route wrappers short-circuit to `<AuthRequiredGate />`
  until `user.organizationId` is present, mirroring `routes/groups/iot.tsx`.
- **Empty**: panels render empty copy when their query returns no rows.
- **Loading**: per-query loading flags drive skeleton placeholders.
- **Saving**: acknowledge buttons disable while their mutation is pending.
- **Error**: query/mutation errors surface as `ApiError`. `apiStatus: partial`
  until the AI backend is verified end-to-end.

## Notes

### Specific (recent)
- 2026-06-23 — #1674: routed both dashboards' hooks through `getApiClient()`
  (was hand-rolled `fetch` bypassing auth/retry), added the org-scoping
  `<AuthRequiredGate />`, moved all hardcoded strings into `sentiment.*` /
  `predictive.*` i18n keys across all 6 locales, registered both routes in
  `@ppt/sitemap`, and created these screen-maps.

## Agent Log
- 2026-06-23 — FrontendEngineer: created the AI dashboards module map and child
  screen-maps as part of the #1674 follow-up (Epic 13 / Stories 13.2-13.3).
