---
id: ppt/settings-portal-webhooks
name: Portal Webhooks
product: ppt
sitemapRefs:
  ppt-web: ppt-settings-portal-webhooks
implementations:
  ppt-web:
    route: "/settings/portal-webhooks"
    component: PortalWebhooksPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: partial
endpoints: []
relatedScreens:
  - id: ppt/settings-integrations
    rel: sibling
sharedComponents: []
diagrams: []
useCases:
  - UC-29
epics:
  - Epic-105
designSources: []
owner: pm-frontend
---

# Portal Webhooks

Manager-facing visibility surface for inbound real-estate portal webhooks
(Gap 83-3 / Epic 105). Inbound webhooks (`POST /api/v1/webhooks/portals/*`)
record views and inquiries from external portals (Reality Portal, Sreality,
Bezrealitky, Nehnutelnosti) and update per-listing syndication state; until
Gap 83-3 there was no manager-facing surface to see any of it.

`PortalWebhooksPage` renders that read-only surface from the org-scoped
syndication read endpoint via the `@ppt/api-client` `syndication` module:

- `useSyndicationDashboard({ page, limit })` — GET
  `/api/v1/listings/syndication/dashboard` — per-listing status + org-wide
  aggregate stats. Paginated (default `limit=20`); the page renders a prev/next
  pager over the per-listing rows while the summary stats stay page-independent.

The route is manager-gated at the route level via
`<ProtectedRoute requiredRoles={[...MANAGER_ROLES]}>` (issue #2322) — the
dashboard exposes org-wide stats, per-listing inquiry counts and `last_error`
strings that residents must not read. The nav link is separately gated on
`isManagerRole`; both now draw from the same `MANAGER_ROLES` constant in
`routes/shared.tsx`.

`apiStatus: partial` — the dashboard endpoint is consumed, but the module also
exports `useOrganizationSyndicationStats`, `useListingSyndications` and
`useListingSyndicationStatus` hooks that no screen consumes yet.

## Notes

### Specific (recent)

- 2026-07-15 — `endpoints` left empty because the syndication read endpoints are
  not (yet) in the `@ppt/sitemap` endpoint catalogue (same reason as
  `settings-integrations`). Added the `ppt-settings-portal-webhooks` sitemap
  entry. Follow-up (deferred to a pm-backend/typespec task): register the four
  `routes::listings::*` syndication handlers in the api-server OpenAPI `paths()`,
  regenerate the TS client, and swap the hand-written `syndication` module to
  generated operations — the natural moment to prune the three unused hooks.

## Agent Log

- 2026-07-15 — agent: gh-issue-2322 — added the screen-map for the portal
  webhooks status page (created in PR #2294 without one); route-level
  manager gate, per-listing pagination, `enabled`-gated dashboard query, and
  cs/de/hu/pl/sk translations landed alongside.
