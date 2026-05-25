---
id: reality-mobile/listing-detail
name: Listing Detail (iOS SwiftUI)
product: reality
sitemapRefs:
  reality-web: reality-listing-detail
implementations:
  ios-swiftui:
    component: ListingDetailView
    route: Route.listingDetail(id:)
    buildStatus: in-progress
    redesignStatus: not-started
    apiStatus: partial
endpoints:
  - GET /api/v1/listings/{id}
  - POST /api/v1/favorites
  - DELETE /api/v1/favorites/{id}
relatedScreens:
  - id: reality/listing-detail
    rel: web-counterpart
  - id: reality-mobile/home
    rel: parent
  - id: reality-mobile/search
    rel: parent
  - id: reality-mobile/favorites
    rel: sibling
  - id: reality-mobile/compare-listings
    rel: child
sharedComponents:
  - AsyncImage
diagrams: []
useCases:
  - UC-31
  - UC-44
  - UC-46
epics:
  - "82"
designSources: []
owner: reality-frontend
---

## Functionality Checklist

- [x] [m] Load listing detail via KMP `listingRepository.getListingById(id:)`
- [x] [m] Photo gallery horizontal scroll (AsyncImage with fallback)
- [x] [m] Price (formatted currency), title, address display
- [x] [m] Area sqm, rooms, bathrooms, floor metrics row
- [x] [m] Amenities list
- [x] [m] Agent info section (name, phone)
- [x] [m] Map section (latitude/longitude if available)
- [x] [m] Favorite toggle button (heart icon, Route.favorites integration)
- [x] [m] "Contact Agent" button → Route.newInquiry(listingId:)
- [x] [m] "Compare" button → Route.compareListings(ids:)
- [x] [m] Share button (system share sheet)
- [x] [m] Loading state (ProgressView in NavigationStack)
- [x] [m] Error state with retry
- [ ] [m] Full-screen photo gallery (Route.listingGallery in Route enum, NavigationCoordinator case handled, but destination is stub Text view)
- [ ] [m] Map full-screen expansion (Route.listingMap destination is stub Text view)
- [ ] [m] Price history chart (not implemented)

## States

- **Loading**: ProgressView while detail fetched.
- **Error**: Error message + Retry button.
- **Success**: Full listing content scrollable.

## Notes

### Broader context

Detail view accessible from HomeView (featured/recent cards), SearchView (search results), FavoritesView (saved items). Accepts `listingId: String` parameter. Maps KMP `ListingDetail` → Swift `ListingDetailModel` via `KMPBridge.toListingDetail()`.

### Specific (recent)

- Favorite toggle calls `favoritesRepository.addFavorite` / `removeFavorite` via `DependencyContainer.shared.favoritesRepository`. No optimistic update currently — state refreshed after server response.
- `Route.listingGallery` and `Route.listingMap` are registered in both `Route.swift` and `NavigationCoordinator.navigate()` but destination views in `MainTabView.destinationView()` are stub `Text` placeholders.

## Agent Log

- 2026-05-25 — agent: created screen map from audit of mobile-native/iosApp/iosApp/Features/Listing/ListingDetailView.swift (epic-82 story 82.4). Gallery and map stub gaps noted.
