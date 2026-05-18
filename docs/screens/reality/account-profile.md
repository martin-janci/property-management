---
id: reality/account-profile
name: Account Profile
product: reality
sitemapRefs: {}
implementations:
  reality-web:
    route: "/account/profile"
    component: AccountProfilePage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: unknown
endpoints: []
relatedScreens:
  - id: reality/account
    rel: parent
sharedComponents: []
diagrams: []
useCases: []
epics: []
designSources: []
owner: reality-frontend
---

# Account Profile

Stubbed by team audit on 2026-05-18. Route exists in code; flesh out useCases, epics, and redesign notes when known.

Note: a separate `reality/profile` screen-map exists; this one is specifically `/account/profile` (account-management variant) — disambiguate with humans later.

## Notes

### Specific (recent)
- 2026-05-18 — audit: stub created from `frontend/apps/reality-web/src/app/[locale]/account/profile/page.tsx`.

## Agent Log
- 2026-05-18 — agent: created stub for unmapped route.
