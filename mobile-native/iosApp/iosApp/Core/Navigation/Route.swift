import Foundation

/// Navigation routes for Reality Portal iOS app.
///
/// Epic 82 - Story 82.2: Navigation and Routing
enum Route: Hashable {
    // MARK: - Home Tab

    /// Home screen with featured listings.
    case home

    /// Featured listings section.
    case featuredListings

    // MARK: - Search Tab

    /// Search screen.
    case search

    /// Search results with query and optional filters.
    case searchResults(query: String, filters: SearchFilters?)

    // MARK: - Listing

    /// Listing detail screen.
    case listingDetail(id: String)

    /// Full-screen photo gallery for a listing.
    case listingGallery(id: String)

    /// Map view for a listing location.
    case listingMap(id: String)

    // MARK: - Favorites Tab

    /// Favorites list screen.
    case favorites

    // MARK: - Inquiries Tab

    /// Inquiries list screen.
    case inquiries

    /// Inquiry conversation detail.
    case inquiryDetail(id: String)

    /// New inquiry form for a listing.
    case newInquiry(listingId: String)

    // MARK: - Account Tab

    /// Account screen.
    case account

    /// Profile editing screen.
    case profile

    /// Settings screen.
    case settings

    /// Login screen.
    case login

    /// Registration screen.
    case register

    // MARK: - Saved searches / Compare / Realtors / Agencies (Android parity)

    /// User's saved searches (UC-45.2).
    case savedSearches

    /// Side-by-side listing comparison (UC-46.5). Carries the listing
    /// IDs to compare so the destination view doesn't need to read
    /// from a side-channel.
    case compareListings(ids: [String])

    /// Realtor directory (UC-49.1).
    case realtors

    /// Agency directory (UC-51.1).
    case agencies

    /// Whether this route requires the user to be authenticated before it
    /// can be navigated to. Used by the deep-link entry point to gate
    /// universal/custom-scheme links: unauthenticated users are bounced
    /// to `.login` (with the intended destination stored on the coordinator
    /// as `pendingDestination`) instead of landing inside a protected view.
    var requiresAuth: Bool {
        switch self {
        case .favorites,
             .inquiries, .inquiryDetail, .newInquiry,
             .account, .profile, .settings,
             .savedSearches:
            return true
        case .home, .featuredListings,
             .search, .searchResults,
             .listingDetail, .listingGallery, .listingMap,
             .login, .register,
             .compareListings,
             .realtors, .agencies:
            return false
        }
    }
}

// MARK: - Search Filters

/// Search filter options.
///
/// Epic 82 - Story 82.3: Home and Search Screens
struct SearchFilters: Hashable {
    var priceMin: Int?
    var priceMax: Int?
    var propertyTypes: Set<PropertyType> = []
    var bedroomsMin: Int?
    var bedroomsMax: Int?
    var bathroomsMin: Int?
    var radiusKm: Double?
    var latitude: Double?
    var longitude: Double?

    /// Whether any filters are active.
    var hasActiveFilters: Bool {
        priceMin != nil ||
        priceMax != nil ||
        !propertyTypes.isEmpty ||
        bedroomsMin != nil ||
        bedroomsMax != nil ||
        bathroomsMin != nil ||
        radiusKm != nil
    }

    /// Reset all filters to default values.
    mutating func reset() {
        priceMin = nil
        priceMax = nil
        propertyTypes = []
        bedroomsMin = nil
        bedroomsMax = nil
        bathroomsMin = nil
        radiusKm = nil
        latitude = nil
        longitude = nil
    }
}

/// Property types for filtering.
enum PropertyType: String, CaseIterable, Hashable {
    case apartment
    case house
    case land
    case commercial
    case garage

    var displayName: String {
        switch self {
        case .apartment: return "Apartment"
        case .house: return "House"
        case .land: return "Land"
        case .commercial: return "Commercial"
        case .garage: return "Garage"
        }
    }

    var iconName: String {
        switch self {
        case .apartment: return "building.2"
        case .house: return "house"
        case .land: return "leaf"
        case .commercial: return "building"
        case .garage: return "car"
        }
    }
}
