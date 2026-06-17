# Reality Portal iOS (SwiftUI) — Screen Map

This directory contains screen maps for the SwiftUI iOS app at
`mobile-native/iosApp/` (bundle ID `three.two.bit.ppt.reality`).

Screen maps here complement the shared `docs/screens/reality/` web screen maps.
They document iOS-specific implementation status, route bindings, and gaps.

> **Story numbering:** the "Story" column below uses the **build** story set
> (`_bmad-output/implementation-artifacts/stories/82-*.md`, e.g. 82.2 =
> Navigation, 82.3 = Home & Search). This is *not* the same numbering as
> `epics-007.md` Epic 82 (Mobile Native Completion). See
> [`docs/superpowers/plans/epic-82-story-mapping.md`](../../superpowers/plans/epic-82-story-mapping.md)
> for the reconciliation.

## Epic-82 Screen Coverage

| Screen Map | Component | Story | buildStatus |
|---|---|---|---|
| [home.md](home.md) | HomeView | 82.3 | in-progress |
| [search.md](search.md) | SearchView | 82.3 | in-progress |
| [listing-detail.md](listing-detail.md) | ListingDetailView | 82.4 | in-progress |
| [favorites.md](favorites.md) | FavoritesView | 82.4 | in-progress |
| [inquiries.md](inquiries.md) | InquiriesView | 82.5 | in-progress |
| [account.md](account.md) | AccountView | 82.5 | in-progress |
| [auth-login.md](auth-login.md) | LoginView | 82.5 | in-progress |
| [saved-searches.md](saved-searches.md) | SavedSearchesView | UC-45.2 | in-progress |
| [compare-listings.md](compare-listings.md) | CompareListingsView | UC-46.5 | in-progress |
| [realtors.md](realtors.md) | RealtorsView | UC-49.1 | in-progress |
| [agencies.md](agencies.md) | AgenciesView | UC-51.1 | in-progress |
| [navigation.md](navigation.md) | NavigationCoordinator / DeepLinkHandler / NavigationStateRestorationService | 82.2 | shipped |

## Cross-Cutting Infrastructure

| Screen Map | Concern | AC | Status |
|---|---|---|---|
| [navigation.md](navigation.md) | Navigation state preservation, URL-scheme deep-linking, auth guard | AC-4, AC-5 | verified (Coverage 82-2); `Info.plist` URL-scheme registration gap flagged for pm-mobile |

## Stub Destinations (Routes defined, views not implemented)

| Route | Destination Stub | File to create |
|---|---|---|
| Route.listingGallery(id:) | Text("Gallery for listing: \(id)") | Features/Listing/ListingGalleryView.swift |
| Route.listingMap(id:) | Text("Map for listing: \(id)") | Features/Listing/ListingMapView.swift |
| Route.inquiryDetail(id:) | Text("Inquiry: \(id)") | Features/Inquiries/InquiryDetailView.swift |
| Route.newInquiry(listingId:) | Text("New inquiry for: \(listingId)") | Features/Inquiries/NewInquiryView.swift |
| Route.profile | Text("Edit Profile") | Features/Account/ProfileView.swift |
| Route.settings | Text("Settings") | Features/Account/SettingsView.swift |
| Route.register | Text("Register") | Features/Auth/RegisterView.swift |
| Route.featuredListings | Text("Featured Listings") | Features/Home/FeaturedListingsView.swift |
| Route.searchResults(query:filters:) | Text("Search Results for: \(query)") | (reuse SearchView with params) |

## Infrastructure Audit Summary

See [gap-82-1-swiftui-audit.md](../../superpowers/plans/gap-82-1-swiftui-audit.md) for full narrative audit.
