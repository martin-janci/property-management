import Foundation
import shared

/// Dependency injection container for Reality Portal iOS app.
///
/// Provides centralized access to shared dependencies and services.
///
/// Epic 82 - Story 82.1: SwiftUI Project Setup
final class DependencyContainer {
    // MARK: - Singleton

    static let shared = DependencyContainer()

    // MARK: - Configuration

    let configuration: Configuration

    // MARK: - KMP Services

    /// Listing repository for fetching property listings.
    lazy var listingRepository: ListingRepository = {
        ListingRepository(
            baseUrl: configuration.apiBaseUrl,
            sessionToken: nil,
            client: HttpClientProvider.shared.client
        )
    }()

    /// Favorites repository for managing user favorites.
    lazy var favoritesRepository: FavoritesRepository = {
        FavoritesRepository(
            baseUrl: configuration.apiBaseUrl,
            sessionToken: nil,
            client: HttpClientProvider.shared.client
        )
    }()

    /// Inquiry repository for managing property inquiries.
    lazy var inquiryRepository: InquiryRepository = {
        InquiryRepository(
            baseUrl: configuration.apiBaseUrl,
            sessionToken: nil,
            client: HttpClientProvider.shared.client
        )
    }()

    /// Agency directory repository.
    lazy var agencyRepository: AgencyRepository = {
        AgencyRepository(
            baseUrl: configuration.apiBaseUrl,
            sessionToken: nil,
            client: HttpClientProvider.shared.client
        )
    }()

    /// Realtor directory repository.
    lazy var realtorRepository: RealtorRepository = {
        RealtorRepository(
            baseUrl: configuration.apiBaseUrl,
            sessionToken: nil,
            client: HttpClientProvider.shared.client
        )
    }()

    /// Layout repository for fetching resolved layout configurations.
    lazy var layoutRepository: LayoutRepository = {
        LayoutRepository(
            baseUrl: configuration.apiBaseUrl,
            client: HttpClientProvider.shared.client
        )
    }()

    /// SSO service for authentication.
    lazy var ssoService: SsoService = {
        SsoService()
    }()

    /// API client for general API operations.
    lazy var apiClient: ApiClient = {
        ApiClient(
            baseUrl: configuration.apiBaseUrl,
            accessToken: nil,
            tenantId: nil
        )
    }()

    // MARK: - Initialization

    private init() {
        self.configuration = Configuration.shared
    }

    // MARK: - Factory Methods

    /// Create an AuthManager instance.
    func makeAuthManager() -> AuthManager {
        AuthManager()
    }

    /// Create a NavigationCoordinator instance.
    func makeNavigationCoordinator() -> NavigationCoordinator {
        NavigationCoordinator()
    }

    /// Create an authenticated listing repository with the given session token.
    func makeAuthenticatedListingRepository(sessionToken: String?) -> ListingRepository {
        ListingRepository(
            baseUrl: configuration.apiBaseUrl,
            sessionToken: sessionToken,
            client: HttpClientProvider.shared.client
        )
    }

    /// Create an authenticated favorites repository with the given session token.
    func makeAuthenticatedFavoritesRepository(sessionToken: String?) -> FavoritesRepository {
        FavoritesRepository(
            baseUrl: configuration.apiBaseUrl,
            sessionToken: sessionToken,
            client: HttpClientProvider.shared.client
        )
    }

    /// Create an authenticated inquiry repository with the given session token.
    func makeAuthenticatedInquiryRepository(sessionToken: String?) -> InquiryRepository {
        InquiryRepository(
            baseUrl: configuration.apiBaseUrl,
            sessionToken: sessionToken,
            client: HttpClientProvider.shared.client
        )
    }

    /// Create an authenticated agency repository with the given session token.
    func makeAuthenticatedAgencyRepository(sessionToken: String?) -> AgencyRepository {
        AgencyRepository(
            baseUrl: configuration.apiBaseUrl,
            sessionToken: sessionToken,
            client: HttpClientProvider.shared.client
        )
    }

    /// Create an authenticated realtor repository with the given session token.
    func makeAuthenticatedRealtorRepository(sessionToken: String?) -> RealtorRepository {
        RealtorRepository(
            baseUrl: configuration.apiBaseUrl,
            sessionToken: sessionToken,
            client: HttpClientProvider.shared.client
        )
    }

    /// Create an authenticated API client with the given access token.
    func makeAuthenticatedApiClient(accessToken: String?) -> ApiClient {
        ApiClient(
            baseUrl: configuration.apiBaseUrl,
            accessToken: accessToken,
            tenantId: nil
        )
    }

    // MARK: - Cleanup

    /// Clean up resources when app terminates.
    func cleanup() {
        HttpClientProvider.shared.close()
    }
}

// MARK: - KMP Bridge

/// Bridge for accessing KMP shared module types in Swift.
///
/// This namespace contains type aliases and helper functions
/// for working with Kotlin types from Swift.
enum KMPBridge {
    // MARK: - Type Aliases

    typealias KMPListingSummary = ListingSummary
    typealias KMPListingDetail = shared.ListingDetail
    typealias KMPListingSearchRequest = ListingSearchRequest
    typealias KMPListingSearchResponse = ListingSearchResponse
    typealias KMPFeaturedListingsResponse = FeaturedListingsResponse
    typealias KMPRecentListingsResponse = RecentListingsResponse
    typealias KMPFavoritesResponse = FavoritesResponse
    typealias KMPFavoriteEntry = FavoriteEntry
    typealias KMPInquiriesResponse = InquiriesResponse
    typealias KMPInquiry = Inquiry
    typealias KMPCreateInquiryRequest = CreateInquiryRequest
    typealias KMPCreateInquiryResponse = CreateInquiryResponse

    // MARK: - Conversion Helpers

    /// Convert KMP ListingSummary to Swift ListingPreview.
    static func toListingPreview(_ kmpListing: ListingSummary) -> ListingPreview {
        ListingPreview(
            id: kmpListing.id,
            title: kmpListing.title,
            price: Int(kmpListing.price),
            currency: kmpListing.currency,
            location: kmpListing.address.city,
            areaSqm: kmpListing.areaSqm.map { Int($0) },
            rooms: kmpListing.rooms.map { Int($0.int32Value) },
            thumbnailUrl: kmpListing.primaryImage?.thumbnailUrl
        )
    }

    /// Convert KMP ListingDetail to Swift ListingDetail.
    static func toListingDetail(_ kmpListing: shared.ListingDetail) -> ListingDetailModel {
        ListingDetailModel(
            id: kmpListing.id,
            title: kmpListing.title,
            price: Int(kmpListing.price),
            currency: kmpListing.currency,
            address: kmpListing.address.street.map { "\($0), \(kmpListing.address.city)" } ?? kmpListing.address.city,
            description: kmpListing.description_,
            areaSqm: Int(kmpListing.areaSqm),
            rooms: kmpListing.rooms.map { Int($0.int32Value) },
            bathrooms: kmpListing.bathrooms.map { Int($0.int32Value) },
            amenities: kmpListing.features,
            photos: kmpListing.images.map { $0.url },
            agentName: kmpListing.realtor?.name ?? "Unknown Agent",
            agentPhone: kmpListing.realtor?.phone,
            latitude: kmpListing.address.coordinates?.latitude,
            longitude: kmpListing.address.coordinates?.longitude
        )
    }

    /// Convert KMP Inquiry to Swift InquiryPreview.
    static func toInquiryPreview(_ kmpInquiry: Inquiry) -> InquiryPreview {
        let status: InquiryStatusSwift
        switch kmpInquiry.status {
        case .pending:
            status = .pending
        case .responded:
            status = .replied
        case .closed:
            status = .closed
        }

        return InquiryPreview(
            id: kmpInquiry.id,
            listingId: kmpInquiry.listingId,
            listingTitle: kmpInquiry.listing?.title ?? "Unknown Listing",
            lastMessage: kmpInquiry.responses.last?.message ?? kmpInquiry.message,
            status: status,
            date: ISO8601DateFormatter().date(from: kmpInquiry.updatedAt) ?? Date(),
            hasUnread: kmpInquiry.status == .responded && kmpInquiry.responses.isEmpty == false
        )
    }
}

// MARK: - Swift Models for Views

/// Swift model for listing detail, mapped from KMP ListingDetail.
struct ListingDetailModel {
    let id: String
    let title: String
    let price: Int
    let currency: String
    let address: String
    let description: String
    let areaSqm: Int?
    let rooms: Int?
    let bathrooms: Int?
    let amenities: [String]
    let photos: [String]
    let agentName: String
    let agentPhone: String?
    let latitude: Double?
    let longitude: Double?

    var formattedPrice: String {
        let formatter = NumberFormatter()
        formatter.numberStyle = .currency
        formatter.currencyCode = currency
        formatter.maximumFractionDigits = 0
        return formatter.string(from: NSNumber(value: price)) ?? "\(price) \(currency)"
    }

    #if DEBUG
    /// Sample data for SwiftUI previews only - excluded from production builds (Story 85.1)
    static let sample = ListingDetailModel(
        id: "1",
        title: "Modern Apartment in City Center",
        price: 250000,
        currency: "EUR",
        address: "Sturova 8, 811 02 Bratislava, Slovakia",
        description: "Beautiful modern apartment located in the heart of Bratislava.",
        areaSqm: 85,
        rooms: 3,
        bathrooms: 2,
        amenities: ["Balcony", "Parking", "Elevator"],
        photos: ["photo1", "photo2", "photo3"],
        agentName: "Maria Kovacova",
        agentPhone: "+421 900 123 456",
        latitude: 48.1486,
        longitude: 17.1077
    )
    #endif
}

/// Swift enum for inquiry status, used in views.
typealias InquiryStatusSwift = InquiryStatus
