---
id: reality/account-listing-edit
name: Edit Listing (Account)
product: reality
sitemapRefs: {}
implementations:
  reality-web:
    route: "/account/listings/[id]/edit"
    component: AccountListingEditPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: unknown
endpoints: []
relatedScreens:
  - id: reality/account
    rel: parent
  - id: reality/listing-edit
    rel: sibling
sharedComponents: []
diagrams: []
useCases: []
epics: []
designSources: []
owner: reality-frontend
---

# Edit Listing (Account)

Stubbed by team audit on 2026-05-18. Route exists in code; flesh out useCases, epics, and redesign notes when known.

Note: distinct from `reality/listing-edit` (which appears to be the realtor variant). Disambiguation needed when humans add detail.

## Notes

### Specific (recent)
- 2026-05-18 — audit: stub created from `frontend/apps/reality-web/src/app/[locale]/account/listings/[id]/edit/page.tsx`.

## Agent Log
- 2026-05-18 — agent: created stub for unmapped route.
