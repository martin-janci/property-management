---
id: ppt/buildings-form
name: Building Create / Edit
product: ppt
sitemapRefs: {}
implementations:
  ppt-web:
    route: "/buildings/new"
    component: BuildingFormPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: complete
endpoints: []
relatedScreens: []
sharedComponents: []
diagrams: []
useCases: []
epics:
  - Epic-3
designSources: []
owner: pm-frontend
---

# Building Create / Edit

Manager-only form; also mounted at /buildings/:buildingId/edit. POST /api/v1/buildings, PUT /api/v1/buildings/{id}.

## Notes

### Specific (recent)
- 2026-11-06 — added in gap audit round 6 (P0).

## Agent Log
- 2026-11-06 — agent: created with the page.
