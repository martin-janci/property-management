---
id: ppt/community-marketplace-new
name: Create Marketplace Listing
product: ppt
sitemapRefs: {}
implementations:
  ppt-web:
    route: "/community/marketplace/new"
    component: CreateItemPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: partial
endpoints: []
relatedScreens:
  - id: ppt/community-marketplace
    rel: parent
sharedComponents: []
diagrams: []
useCases: []
epics:
  - Epic-27
designSources: []
owner: pm-frontend
---

# Create Marketplace Listing

Form to publish a marketplace listing in the selected building.

## Notes

### Specific (recent)
- 2026-11-04 — Submits `POST /community/buildings/{building_id}/marketplace` (title, description, category, condition, free/price as decimal string, negotiable, trade, one photo URL, location, pickup details) and opens the new listing. `endpoints:` left empty — community ops are absent from `@ppt/sitemap`. `apiStatus: partial` until verified end-to-end (backend community happy-path tests quarantined, BIT-440).

## Agent Log
- 2026-11-04 — agent: created with the route (`frontend/apps/ppt-web/src/routes/groups/community.tsx`); page in `features/community/pages/CreateItemPage.tsx`.
