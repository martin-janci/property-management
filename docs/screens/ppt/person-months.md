---
id: ppt/person-months
name: Person-months
product: ppt
implementations:
  ppt-web:
    route: "/buildings/:buildingId/person-months"
    component: PersonMonthsPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: complete
endpoints: []
relatedScreens:
  - id: ppt/buildings-detail
    rel: parent
sharedComponents: []
diagrams: []
useCases:
  - UC-10
epics:
  - Epic-3
designSources: []
owner: pm-frontend
---

# Person-months

Person-month tracking (resident counts per unit/month, used for shared-utility
billing). Wired into ppt-web `AppRoutes.tsx` by BIT-190 (Epic 3, Story 3.5). The
`features/person-months/` pages were already built but unrouted and had no
api-client; this work authored the `@ppt/api-client` person-months module
(unit + building endpoints incl. `calculate`) and the route group that wires the
pages to it.

## Routes

- `/buildings/:buildingId/person-months` — `PersonMonthsPage`: building-level
  summary, one card per unit for the selected year/month.
- `/buildings/:buildingId/person-months/bulk` — `BulkEntryPage`: enter counts for
  every unit in one period, submitted via the `/person-months/bulk` endpoint.
- `/buildings/:buildingId/units/:unitId/person-months` — `UnitPersonMonthsPage`:
  per-unit history + yearly summary chart, with per-entry delete.
- `/buildings/:buildingId/units/:unitId/person-months/edit/:year/:month` —
  `EditPersonMonthPage`: add/edit a single entry (upsert). When no entry exists
  for the period, the count is pre-populated from the unit's resident history via
  the `calculate` endpoint (Story 3.5 auto-suggest).

Reachable from the building detail page via a "Person-months" quick link.

## States

- **Loading**: spinners while building/units/entries queries are in flight.
- **Empty**: units with no entry render a card with count 0; unit history shows
  an empty-state when a year has no entries.
- **Error**: mutations surface failures via toast; the bulk results panel lists
  per-unit failures returned by the backend.

## Notes

### Specific (recent)
- **Endpoints not catalogued**: the person-months endpoints have no `operationId`
  in `@ppt/sitemap`, so `endpoints: []` keeps `/screens validate` green; the URLs
  are: `GET|POST /api/v1/buildings/{building_id}/person-months[/bulk|/summary]`
  and `GET|POST|PUT|DELETE /api/v1/buildings/{building_id}/units/{unit_id}/person-months[/{id}|/yearly|/calculate]`.
  Reference the operationIds here once the sitemap catalogue is extended.
- 2026-06-22 — BIT-190: authored `@ppt/api-client/person-months` (api + hooks +
  types), added the `person-months` route group, mounted it in `AppRoutes.tsx`,
  and added a building-detail entry point. Building-level views also list units
  via `/api/v1/units?buildingId=` because the person-months endpoints only return
  units that already have an entry.

## Agent Log
- 2026-07-13 — agent: gap-screens-normalize-frontmatter — normalized story-id-style epic ref(s) Epic-3-5 → Epic-3 (strip story suffix); /screens validate clean.
- 2026-06-22 — FrontendEngineer: created on route+api-client wiring (BIT-190).
- 2026-07-09 — agent: linked UC-10 (Person-Months, docs/use-cases.md) in useCases frontmatter (gap-screens-link-uc-10).
