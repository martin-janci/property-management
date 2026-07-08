---
id: reality-mobile/agencies
name: Agencies Directory (iOS SwiftUI)
product: reality-mobile
sitemapRefs:
  reality-web: reality-agency
implementations:
  ios-swiftui:
    component: AgenciesView
    route: Route.agencies (pushed onto searchPath)
    buildStatus: in-progress
    redesignStatus: not-started
    apiStatus: partial
endpoints: []
relatedScreens:
  - id: reality/agent-profile
    rel: web-counterpart
  - id: reality-mobile/search
    rel: parent
  - id: reality-mobile/realtors
    rel: sibling
  - id: reality-mobile/listing-detail
    rel: child
sharedComponents: []
diagrams: []
useCases:
  - UC-51
epics:
  - Epic-82
designSources: []
owner: reality-frontend
---

## Functionality Checklist

- [x] [m] Load agencies via `AgencyRepository.listAgencies()`
- [x] [m] Agency list rows: name, description, realtor count, logo
- [x] [m] Tap row → navigate to search results filtered by agency (no dedicated agency detail screen)
- [x] [m] Loading state
- [x] [m] Empty state
- [x] [m] Error state
- [ ] [m] Dedicated agency detail screen (no Route.agencyDetail defined)
- [ ] [m] Agency branding page (web: agency-branding.md — not on iOS yet)

## States

- **Loading**: ProgressView.
- **Error**: Error message + Retry.
- **Empty**: "No agencies found" message.
- **Success**: List of AgencyRowCard items.

## Notes

### Broader context

UC-51.1 agency directory. Placed under the Search tab by `NavigationCoordinator`. Tapping an agency navigates to `Route.searchResults` filtered by `agencyId` — no dedicated agency profile screen exists on iOS yet. `DependencyContainer.shared.agencyRepository` is wired via KMP `AgencyRepository`.

### Specific (recent)

- No `Route.agencyDetail` in `Route.swift` — post-epic-82 follow-up needed.
- `AgencyRepository.listAgencies()` maps to `shared.AgenciesResponse`.

## Agent Log

- 2026-05-25 — agent: created screen map from audit of mobile-native/iosApp/iosApp/Features/Agencies/AgenciesView.swift (UC-51.1). Missing agency-detail route noted.
- 2026-05-28 — agent: fix(#581) added missing Agent Log entry that PR #554 omitted (CLAUDE.md screen-map Rule A.3); flagged dropped operationIds on home + saved-searches in Notes > Specific (recent) pending @ppt/sitemap extension.
