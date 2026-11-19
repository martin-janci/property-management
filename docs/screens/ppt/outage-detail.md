---
id: ppt/outage-detail
name: Outage Detail
product: ppt
sitemapRefs: {}
implementations:
  ppt-web:
    route: "/outages/:outageId"
    component: ViewOutagePage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: stub
endpoints: []
relatedScreens:
  - id: ppt/outages
    rel: parent
sharedComponents: []
diagrams: []
useCases: []
epics: []
designSources: []
owner: pm-frontend
---

# Outage Detail

Stubbed by team audit on 2026-05-18. Route exists in code; flesh out useCases, epics, and redesign notes when known.

## Notes

### Specific (recent)
- 2026-11-18 — **auth guard (PR #3606)**: `/outages/:outageId` is now wrapped in `<ProtectedRoute>` (auth-only, not role-gated) in `routes/groups/outages.tsx` — previously a bare `<Route>` that rendered the detail chrome for an anonymous deep-link. Any affected signed-in user can read outage detail; an anonymous deep-link is now redirected to `/login` (return URL captured). `buildStatus`/`apiStatus` unchanged. Guard regression-covered by `routes/groups/core-route-groups-auth-guard.route.test.tsx`.
- 2026-05-18 — audit: stub created from `frontend/apps/ppt-web/src/App.tsx:468`.

## Agent Log
- 2026-11-18 — agent: screen-map-drift-pr-3606-ppt — reconciled with merged PR #3606: the previously-bare `/outages/:outageId` detail route is now wrapped in `<ProtectedRoute>` (auth-only) in `routes/groups/outages.tsx`; anonymous deep-links redirect to `/login` (return URL captured). No route/component/buildStatus/apiStatus change — docs-only.
- 2026-05-18 — agent: created stub for unmapped route.
