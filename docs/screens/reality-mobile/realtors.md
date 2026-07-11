---
id: reality-mobile/realtors
name: Realtors Directory (iOS SwiftUI)
product: reality-mobile
sitemapRefs: {}
implementations:
  ios-swiftui:
    component: RealtorsView
    route: Route.realtors (pushed onto searchPath)
    buildStatus: in-progress
    redesignStatus: not-started
    apiStatus: partial
endpoints: []
relatedScreens:
  - id: reality/realtor
    rel: web-counterpart
  - id: reality-mobile/search
    rel: parent
  - id: reality-mobile/agencies
    rel: sibling
sharedComponents: []
diagrams: []
useCases:
  - UC-49
epics:
  - Epic-82
designSources: []
owner: reality-frontend
---

## Functionality Checklist

- [x] [m] Load realtors derived from agency tree via `AgencyRepository.listAgencies()`
- [x] [m] Realtor list rows: name, agency name, contact info
- [x] [m] Tap row → navigate to listings filtered by realtor (search tab with agent filter)
- [x] [m] Loading state
- [x] [m] Empty state
- [x] [m] Error state
- [ ] [m] Dedicated realtor profile screen (Route.realtors navigates to search — no dedicated realtor detail)
- [ ] [m] reality-server `/realtors` directory endpoint not available; data derived from agency tree workaround

## States

- **Loading**: ProgressView.
- **Error**: Error message + Retry.
- **Empty**: "No realtors found" message.
- **Success**: List of realtor rows.

## Notes

### Broader context

UC-49.1 realtor directory. `NavigationCoordinator.navigate(to: .realtors)` places this under the Search tab. reality-server exposes only `/realtors/profile` (own profile) and `/realtors/{user_id}/profile` (individual lookup), not a directory listing endpoint — so this view derives realtors from the agency tree (lists agencies → extracts realtors).

### Specific (recent)

- This is a known API gap: a `GET /api/v1/realtors` directory endpoint is missing. Workaround sources realtors from agency data. Follow-up needed in reality-server.
- `NavigationCoordinator` places `.realtors` and `.agencies` under the Search tab so user can switch between "search by listing" and "search by realtor/agency".

## Agent Log

- 2026-05-25 — agent: created screen map from audit of mobile-native/iosApp/iosApp/Features/Realtors/RealtorsView.swift (UC-49.1). API gap (no directory endpoint) documented.
- 2026-05-28 — agent: fix(#581) added missing Agent Log entry that PR #554 omitted (CLAUDE.md screen-map Rule A.3); flagged dropped operationIds on home + saved-searches in Notes > Specific (recent) pending @ppt/sitemap extension.
