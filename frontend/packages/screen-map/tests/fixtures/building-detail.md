---
id: ppt/building-detail
name: Building Detail
product: ppt
sitemapRefs:
  ppt-web: ppt-building-detail
  mobile: mobile-building-detail-screen
implementations:
  ppt-web:
    route: /buildings/:id
    component: BuildingDetailPage
    buildStatus: shipped
    redesignStatus: applied
    apiStatus: complete
  mobile:
    screen: BuildingDetailScreen
    buildStatus: shipped
    redesignStatus: in-progress
    apiStatus: complete
endpoints:
  - building_get
  - building_update
  - units_list
relatedScreens:
  - { id: ppt/buildings-list, rel: parent }
  - { id: ppt/building-edit, rel: action }
sharedComponents:
  - BuildingHeader
  - UnitsTable
diagrams:
  - { ref: docs/sequence-diagrams.md#building-detail-load, kind: sequence }
useCases: [UC-12, UC-13]
epics: [Epic-15]
owner: pm-frontend
lastReview: '2026-05-04'
---

## Functionality Checklist

- [x] [w,m] View building info
- [ ] [m] Edit building info (planned)

## Notes

### Broader context
Header card pattern is shared with `reality/property-detail`.

## Agent Log

- 2026-05-07 — agent: initial seed.
