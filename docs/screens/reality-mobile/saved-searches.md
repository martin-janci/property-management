---
id: reality-mobile/saved-searches
name: Saved Searches (iOS SwiftUI)
product: reality-mobile
sitemapRefs:
  reality-web: reality-saved-searches
implementations:
  ios-swiftui:
    component: SavedSearchesView
    route: Route.savedSearches (pushed onto accountPath)
    buildStatus: in-progress
    redesignStatus: not-started
    apiStatus: partial
endpoints:
  - saved_searches_list
  - saved_searches_delete
relatedScreens:
  - id: reality/saved-searches
    rel: web-counterpart
  - id: reality-mobile/account
    rel: parent
  - id: reality-mobile/search
    rel: sibling
sharedComponents: []
diagrams: []
useCases:
  - UC-45
epics:
  - Epic-82
designSources: []
owner: reality-frontend
---

## Functionality Checklist

- [x] [m] Load saved searches via `FavoritesRepository.getSavedSearches()`
- [x] [m] List of saved-search rows: name, filter summary, alert toggle
- [x] [m] Alert toggle → `FavoritesRepository.toggleSavedSearchAlert(id:enabled:)`
- [x] [m] Delete saved search (swipe-to-delete or button) → `FavoritesRepository.deleteSavedSearch(id:)`
- [x] [m] Loading state
- [x] [m] Empty state (magnifying-glass icon + "no_saved_searches" message)
- [x] [m] Error state
- [ ] [m] Navigate to search results from saved search (not implemented — no route from SavedSearch to SearchView with pre-filled filters)
- [ ] [m] Create saved search from current search (would be triggered from SearchView — not wired)

## States

- **Loading**: ProgressView.
- **Error**: Warning icon + message + Retry.
- **Empty**: Magnifying-glass SF symbol + "no_saved_searches" + description.
- **Success**: List of SavedSearchRow items with name, filter chips, and alert toggle.

## Notes

### Broader context

UC-45.2 — user's personal saved searches with alert notifications. Navigated to via `Route.savedSearches` which `NavigationCoordinator` places under the Account tab stack. Feature parity with the Android `SavedSearchesScreen` composable.

### Specific (recent)

- Accessible from AccountView's navigation list; also navigable via deep link `realityportal://account/saved-searches`.
- Alert toggle fires async KMP call and updates list in-place on success.
- ⚠️ **Sitemap gap (#581):** the alert-toggle UI calls `PATCH /api/v1/saved-searches/{id}`, but no `saved_searches_update` (or `saved_searches_toggle_alert`) operationId exists in `@ppt/sitemap` yet — so it is intentionally omitted from `endpoints` above. Restore the endpoint here once the sitemap is extended.

## Agent Log

- 2026-05-25 — agent: created screen map from audit of mobile-native/iosApp/iosApp/Features/SavedSearches/SavedSearchesView.swift (UC-45.2). Launch-from-saved-search gap noted.
- 2026-05-28 — agent: fix(#581) added missing Agent Log entry that PR #554 omitted (CLAUDE.md screen-map Rule A.3); flagged dropped operationIds on home + saved-searches in Notes > Specific (recent) pending @ppt/sitemap extension.
