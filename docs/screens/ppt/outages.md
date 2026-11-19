---
id: ppt/outages
name: Outages
product: ppt
sitemapRefs: {}
implementations:
  ppt-web:
    route: "/outages"
    component: OutagesPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: stub
endpoints: []
relatedScreens: []
sharedComponents: []
diagrams: []
useCases: []
epics: []
designSources: []
owner: pm-frontend
---

# Outages

Stubbed by team audit on 2026-05-18. Route exists in code; flesh out useCases, epics, and redesign notes when known.

## Notes

### Specific (recent)
- 2026-11-18 — **auth guard (PR #3606)**: `/outages` is now wrapped in `<ProtectedRoute>` (auth-only, not role-gated) in `routes/groups/outages.tsx` — previously a bare `<Route>`. Its inline `<AuthRequiredGate>` only covers the authenticated-but-no-org edge case and never redirects, so an anonymous visitor rendered the page chrome; the `<ProtectedRoute>` wrapper now redirects an anonymous visitor to `/login` (return URL captured). Residents read outages affecting them and managers the full list, but both must be signed in; manager-only create/edit actions remain gated in-page. `buildStatus`/`apiStatus` unchanged. Guard regression-covered by `routes/groups/core-route-groups-auth-guard.route.test.tsx`.
- 2026-05-18 — audit: stub created from `frontend/apps/ppt-web/src/App.tsx:465`.

## Agent Log

- 2026-11-18 — agent: screen-map-drift-pr-3606-ppt — reconciled with merged PR #3606: the previously-bare `/outages` list route is now wrapped in `<ProtectedRoute>` (auth-only) in `routes/groups/outages.tsx`; the inline non-redirecting `<AuthRequiredGate>` was not an auth guard, so anonymous visitors now redirect to `/login` (return URL captured). No route/component/buildStatus/apiStatus change — docs-only.
- 2026-11-04 — agent: r3-nav-roles — create button and card Edit only for managers (route omits the handlers for residents) (audit r3 #14).
- 2026-05-18 — agent: created stub for unmapped route.
