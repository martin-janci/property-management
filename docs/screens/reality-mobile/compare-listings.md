---
id: reality-mobile/compare-listings
name: Compare Listings (iOS SwiftUI)
product: reality
sitemapRefs:
  reality-web: reality-compare-properties
implementations:
  ios-swiftui:
    component: CompareListingsView
    route: Route.compareListings(ids:)
    buildStatus: in-progress
    redesignStatus: not-started
    apiStatus: partial
endpoints:
  - GET /api/v1/listings/{id} (called for each listing ID)
relatedScreens:
  - id: reality/compare-properties
    rel: web-counterpart
  - id: reality-mobile/listing-detail
    rel: parent
  - id: reality-mobile/favorites
    rel: parent
sharedComponents: []
diagrams: []
useCases:
  - UC-46
epics:
  - "82"
designSources: []
owner: reality-frontend
---

## Functionality Checklist

- [x] [m] Accept `listingIds: [String]` parameter
- [x] [m] Load each listing detail via KMP `listingRepository.getListingById(id:)`
- [x] [m] Horizontally-scrollable comparison grid: price, area, rooms, bathrooms per listing
- [x] [m] Column per listing with header (title + thumbnail)
- [x] [m] Tap listing column → Route.listingDetail(id:)
- [x] [m] Loading state
- [x] [m] Error state (if any listing fails to load)
- [ ] [m] Add/remove listings from compare set (UI for selection not implemented in this view)
- [ ] [m] Maximum listing count enforcement (no cap currently)

## States

- **Loading**: ProgressView while listings load in parallel.
- **Error**: Error message for listings that failed to load (partial failure shows loaded ones).
- **Success**: Horizontal scroll grid comparing loaded listings.

## Notes

### Broader context

UC-46.5 cross-tab feature. `NavigationCoordinator.navigate(to: .compareListings)` appends to `currentPath` (whatever tab the user is in), preserving tab context. Listing IDs collected by caller (ListingDetailView's "Compare" button).

### Specific (recent)

- Route carries listing IDs as associated value `[String]` — comparison set is stateless per navigation push.
- Loading uses `async let` / `await` parallel fetch for all listing IDs.

## Agent Log

- 2026-05-25 — agent: created screen map from audit of mobile-native/iosApp/iosApp/Features/CompareListings/CompareListingsView.swift (UC-46.5). Add/remove UI gap noted.
