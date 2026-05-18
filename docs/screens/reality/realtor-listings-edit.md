---
id: reality/realtor-listings-edit
name: Realtor Edit Listing
product: reality
sitemapRefs: {}
implementations:
  reality-web:
    route: "/realtor/listings/[id]/edit"
    component: RealtorListingEditPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: stub
endpoints: []
relatedScreens:
  - id: reality/realtor-listings
    rel: parent
sharedComponents: []
diagrams: []
useCases: []
epics: []
designSources: []
owner: reality-frontend
---

# Realtor Edit Listing

Stubbed by team audit on 2026-05-18. Route exists in code; flesh out useCases, epics, and redesign notes when known.

Note: an existing `reality/listing-edit` screen-map may overlap. Humans should reconcile whether one entry should cover the realtor edit route.

## Notes

### Specific (recent)
- 2026-05-18 — audit: stub created from `frontend/apps/reality-web/src/app/[locale]/realtor/listings/[id]/edit/page.tsx`.

## Agent Log
- 2026-05-18 — agent: created stub for unmapped route.
