---
id: ppt/community-marketplace-detail
name: Marketplace Listing Detail
product: ppt
sitemapRefs: {}
implementations:
  ppt-web:
    route: "/community/marketplace/:itemId"
    component: ItemDetailPage
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

# Marketplace Listing Detail

A single marketplace listing with a contact-the-seller form.

## Notes

### Specific (recent)
- 2026-11-04 — Loads `GET /community/marketplace/{id}`; non-sellers send `POST /community/marketplace/{id}/inquiries` (message + optional offer). Sellers see their listing read-only (no edit/withdraw/mark-sold/inquiry-list endpoints). `endpoints:` left empty — community ops are absent from `@ppt/sitemap`. `apiStatus: partial` until verified end-to-end (backend community happy-path tests quarantined, BIT-440).

## Agent Log
- 2026-11-04 — agent: created with the route (`frontend/apps/ppt-web/src/routes/groups/community.tsx`); page in `features/community/pages/ItemDetailPage.tsx`.
