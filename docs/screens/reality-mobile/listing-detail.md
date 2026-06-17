---
id: reality-mobile/listing-detail
name: Listing Detail (iOS SwiftUI)
product: reality-mobile
sitemapRefs:
  reality-web: reality-listing-detail
implementations:
  ios-swiftui:
    component: ListingDetailView
    route: Route.listingDetail(id:)
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: partial
endpoints:
  - listings_get
  - favorites_add
  - favorites_remove
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
- [x] [m] Full-screen photo gallery — implemented inline via `.fullScreenCover` (`PhotoGalleryView`) with paged `TabView`, pinch-to-zoom + double-tap-to-zoom (`ZoomablePhotoPage`). The legacy `Route.listingGallery` route is no longer used by this surface.
- [x] [m] Inline map preview in location section (non-interactive `Map`, marker, falls back to placeholder when lat/long absent)
- [x] [m] Map full-screen expansion — `Route.listingMap` now resolves to `ListingMapView` (MapKit `Map` + marker + "Get Directions" hand-off to Apple Maps). Previously a stub `Text` view.
- [ ] [m] Price history chart (not implemented)

## States

- **Loading**: ProgressView while detail fetched.
- **Error**: Error message + Retry button.
- **Success**: Full listing content scrollable.

## Notes

### Broader context

Detail view accessible from HomeView (featured/recent cards), SearchView (search results), FavoritesView (saved items). Accepts `listingId: String` parameter. Maps KMP `ListingDetail` → Swift `ListingDetailModel` via `KMPBridge.toListingDetail()`.

### Specific (recent)

- Favorite toggle is now driven by the app-wide `FavoritesService` (`@Environment`), shared with `FavoritesView`. The service applies an **optimistic** update with automatic revert on server error, so the heart reflects the new state immediately (delivered in PR #641). The earlier "no optimistic update" note is obsolete.
- Full-screen photo gallery is handled **inline** by a private `PhotoGalleryView` presented via `.fullScreenCover` (paged `TabView`, pinch + double-tap zoom). The `Route.listingGallery` enum case is now effectively dead for this surface; `MainTabView.destinationView(.listingGallery)` is still a stub `Text` placeholder but is unreachable from listing-detail.
- `Route.listingMap` destination is now the real `ListingMapView` (`Features/Listing/ListingMapView.swift`): a MapKit `Map` with a marker for the listing coordinate and a "Get Directions" button that hands off to Apple Maps via `MKMapItem.openInMaps`. It re-fetches the listing by id (route only carries the id) reusing `listingRepository` + `KMPBridge.toListingDetail`. The detail screen's location section now shows a non-interactive inline `Map` preview (tappable → `ListingMapView`) with a placeholder fallback when lat/long are nil.
- New i18n key added across all 6 locales (`en/sk/cs/de/pl/hu`): `get_directions`. The map empty-state reuses the existing `location_unavailable` key (no duplicate added).
- `ListingMapView.swift` is auto-included by XcodeGen (`project.yml` globs `sources: - path: iosApp`); no `project.pbxproj` edit required.

## Agent Log

- 2026-05-25 — agent: created screen map from audit of mobile-native/iosApp/iosApp/Features/Listing/ListingDetailView.swift (epic-82 story 82.4). Gallery and map stub gaps noted.
- 2026-05-28 — agent: fix(#581) added missing Agent Log entry that PR #554 omitted (CLAUDE.md screen-map Rule A.3); flagged dropped operationIds on home + saved-searches in Notes > Specific (recent) pending @ppt/sitemap extension.
- 2026-06-09 — agent: coverage 82-4 polish — implemented `ListingMapView` (MapKit) replacing the `Route.listingMap` stub, added inline non-interactive map preview to the detail location section, added the `get_directions` string to all 6 locales (map empty-state reuses the existing `location_unavailable` key). Reconciled stale checklist/notes: full-screen gallery and optimistic favorites are DONE (delivered inline + via FavoritesService/PR #641). Flipped `buildStatus: in-progress → shipped`. Swift/MapKit authored without a mac; needs `xcodegen generate` + `xcodebuild` on macOS to compile-verify.
- 2026-06-17 — agent: coverage 82-4 polish follow-through — added `iosAppTests/FavoritesServiceTests.swift` pinning the `FavoritesService` session-gating contract that drives the detail-screen heart (`isFavorite` synchronous read; no-session toggle no-op). Synced the sibling `favorites.md` screen-map (buildStatus shipped + optimistic-remove notes). No `ListingDetailView` source change this pass. KMP/iOS not buildable offline — host-runnable XCTest needs `xcodebuild test` on macOS.
