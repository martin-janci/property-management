---
id: ppt/buildings-detail
name: Building Detail
product: ppt
sitemapRefs: {}
implementations:
  ppt-web:
    component: BuildingDetailPage
    buildStatus: shipped
    redesignStatus: n/a
    apiStatus: partial
  mobile:
    buildStatus: n/a
    redesignStatus: n/a
    apiStatus: n/a
endpoints: []
epics:
  - Epic-3
  - Epic-81
sharedComponents:
  - status-pill
  - stat-card
  - location-map
useCases:
  - UC-15
diagrams: []
owner: pm-frontend
---

## Overview

Detail view for a single building in the PPT admin portal. Route
`/buildings/:buildingId`, rendered by `BuildingDetailPage`
(`frontend/apps/ppt-web/src/features/buildings/pages/BuildingDetailPage.tsx`).
Loads the building, its floors, units, and common areas from the buildings API.

## Functionality Checklist

<!-- tag with [w] / [m] / [w,m] / [-] -->

### Header card
- [x] [w] Hero image (falls back to placeholder when `photoUrl` is unsafe/absent)
- [x] [w] Name + composed address line + status pill
- [x] [w] Optional description
- [x] [w] Stat grid: Type / Units / Floors / Year Built / Total Area

### Location map (Story 3.1 AC3)
- [x] [w] Renders `BuildingLocationMap` immediately under the header card
- [x] [w] Shows a keyless OpenStreetMap embed centred on the building with a marker
- [x] [w] "Open in OpenStreetMap" link opens the point in a new tab
- [x] [w] Coordinate readout (lat, lng to 6 dp) under the map
- [x] [w] Renders nothing when the building has no resolved coordinates (graceful
  degradation while geocoding is unconfigured/pending)

### Floors / Units / Common Areas
- [x] [w] Floors list with per-floor unit counts (loading + empty states)
- [x] [w] Units section (`UnitsSection`)
- [x] [w] Common areas grid with type + description + area (loading + empty states)

## Data Flow / API Notes

Endpoints (not yet registered in `@ppt/sitemap`, so listed here in prose):
`GET /api/v1/buildings/:id`, `GET /api/v1/buildings/:id/floors`,
`GET /api/v1/buildings/:id/common-areas`.

- Coordinates are geocoded and stored server-side (Story 3.1 AC3, backend
  [BIT-200](/BIT/issues/BIT-200) / PR #1691). The API serializes them as
  top-level `latitude` / `longitude` fields.
- The client `Building` model exposes coordinates as nested
  `location: GeoLocation`. The buildings API client normalizes the flat
  backend shape into `location` (`packages/api-client/src/buildings/api.ts:
  normalizeBuilding`), tolerating numeric strings and out-of-range/missing
  values, so the page can rely on `building.location` regardless of transport.
- The map uses the OpenStreetMap keyless embed (`/export/embed.html`) — no API
  key and no extra dependency, so it works in CI and offline dev.

## Agent Log
- 2026-08-18 — agent: screen-map-drift-pr-2647 — noted i18n update in PR #2647: `BuildingDetailPageRoute` in `frontend/apps/ppt-web/src/routes/groups/buildings.tsx` now renders its missing-param fallback via `t('errors.buildingNotFound', 'Building not found')` (was a hardcoded English literal); locale bundles gained the `errors.buildingNotFound` key across en/sk/cs/de/hu/pl. No route or component change.
