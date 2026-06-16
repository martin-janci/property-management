import Foundation

/// Parses incoming URLs (custom scheme + universal links) into typed ``Route``
/// values, decoupled from navigation so it can be unit-tested without any
/// SwiftUI state.
///
/// **Supported custom-scheme URLs** (`realityportal://`):
///
/// | URL | Route |
/// |-----|-------|
/// | `realityportal://sso?token=<t>` | SSO callback — handled by caller, returns `nil` |
/// | `realityportal://listing/<id>` | `.listingDetail(id:)` |
/// | `realityportal://listing/<id>/gallery` | `.listingGallery(id:)` |
/// | `realityportal://listing/<id>/map` | `.listingMap(id:)` |
/// | `realityportal://search?q=<query>` | `.searchResults(query:filters:)` |
/// | `realityportal://favorites` | `.favorites` |
/// | `realityportal://inquiries` | `.inquiries` |
/// | `realityportal://inquiries/<id>` | `.inquiryDetail(id:)` |
/// | `realityportal://inquiries/new?listingId=<id>` | `.newInquiry(listingId:)` |
/// | `realityportal://account` | `.account` |
/// | `realityportal://account/profile` | `.profile` |
/// | `realityportal://account/settings` | `.settings` |
/// | `realityportal://realtors` | `.realtors` |
/// | `realityportal://agencies` | `.agencies` |
/// | `realityportal://saved-searches` | `.savedSearches` |
/// | `realityportal://compare?ids=<id1>,<id2>` | `.compareListings(ids:)` |
///
/// Universal links (`https://reality.example.com/...`) follow the same path
/// mapping as the custom scheme.
///
/// Epic 82 - Story 82.2: Navigation and Routing
final class DeepLinkHandler {

    // MARK: - Public Interface

    /// Result of parsing a deep link URL.
    enum ParseResult {
        /// A navigation route was extracted — navigate to it.
        case route(Route)
        /// An SSO callback was received — caller should process the token.
        ///
        /// `state` is the CSRF nonce returned by the SSO server. Callers MUST
        /// compare it against the value that was minted and stored locally
        /// before the SSO browser flow opened. A mismatch (or missing value)
        /// must reject the callback.
        case ssoCallback(token: String, state: String?)
        /// The URL could not be parsed; ignore it.
        case unrecognized
    }

    // MARK: - Universal-link allow-list

    /// Hosts permitted for `https://` / `http://` universal links.
    ///
    /// Defence-in-depth: iOS only routes hosts present in the AASA / app
    /// entitlement to `onOpenURL`, but the parser still rejects unknown
    /// hosts so a stray `https://attacker.example/listing/1` from a
    /// share-sheet or paste never reaches the router.
    static let allowedUniversalLinkHosts: Set<String> = [
        "reality.example.com",         // production (placeholder until AASA wired)
        "staging.reality.example.com"  // staging — matches Environment.staging.universalLinkDomain
    ]

    /// Parse any incoming URL into a ``ParseResult``.
    ///
    /// - Parameter url: The URL received via `onOpenURL` or the scene delegate.
    /// - Returns: A ``ParseResult`` that the caller should act on.
    func parse(_ url: URL) -> ParseResult {
        guard let components = URLComponents(url: url, resolvingAgainstBaseURL: true) else {
            return .unrecognized
        }

        // SSO callback — must be tested before the path router because the
        // host is "sso", which doesn't fit the path-component model.
        if components.host == "sso" {
            return parseSsoCallback(components)
        }

        // For universal links (http/https), enforce a host allow-list.
        if let scheme = components.scheme?.lowercased(), scheme == "http" || scheme == "https" {
            guard let host = components.host?.lowercased(),
                  Self.allowedUniversalLinkHosts.contains(host) else {
                return .unrecognized
            }
        }

        let pathSegments = components.path
            .split(separator: "/", omittingEmptySubsequences: true)
            .map(String.init)

        guard let first = pathSegments.first else {
            // Root URL → home
            return .route(.home)
        }

        if let route = parseRoute(first: first, rest: Array(pathSegments.dropFirst()), queryItems: components.queryItems ?? []) {
            return .route(route)
        }

        return .unrecognized
    }

    // MARK: - Private Helpers

    private func parseSsoCallback(_ components: URLComponents) -> ParseResult {
        let items = components.queryItems ?? []
        guard let token = items.first(where: { $0.name == "token" })?.value, !token.isEmpty else {
            return .unrecognized
        }
        let state = items.first(where: { $0.name == "state" })?.value
        return .ssoCallback(token: token, state: state)
    }

    /// Route the first path segment and its tail.
    private func parseRoute(first: String, rest: [String], queryItems: [URLQueryItem]) -> Route? {
        switch first {
        case "listing":
            return parseListingRoute(rest: rest, queryItems: queryItems)

        case "search":
            let query = queryItems.first(where: { $0.name == "q" })?.value ?? ""
            let filters = parseSearchFilters(from: queryItems)
            return .searchResults(query: query, filters: filters)

        case "favorites":
            return .favorites

        case "inquiries":
            return parseInquiriesRoute(rest: rest, queryItems: queryItems)

        case "account":
            return parseAccountRoute(rest: rest)

        case "realtors":
            return .realtors

        case "agencies":
            return .agencies

        case "saved-searches", "savedSearches":
            return .savedSearches

        case "compare":
            return parseCompareRoute(queryItems: queryItems)

        case "home":
            return .home

        default:
            return nil
        }
    }

    // MARK: Listing

    private func parseListingRoute(rest: [String], queryItems: [URLQueryItem]) -> Route? {
        guard let id = rest.first, !id.isEmpty else { return nil }
        let sub = rest.dropFirst().first
        switch sub {
        case "gallery":
            return .listingGallery(id: id)
        case "map":
            return .listingMap(id: id)
        default:
            return .listingDetail(id: id)
        }
    }

    // MARK: Inquiries

    private func parseInquiriesRoute(rest: [String], queryItems: [URLQueryItem]) -> Route? {
        guard let first = rest.first else { return .inquiries }
        if first == "new" {
            if let listingId = queryItems.first(where: { $0.name == "listingId" })?.value, !listingId.isEmpty {
                return .newInquiry(listingId: listingId)
            }
            return nil
        }
        return .inquiryDetail(id: first)
    }

    // MARK: Account

    private func parseAccountRoute(rest: [String]) -> Route {
        switch rest.first {
        case "profile":
            return .profile
        case "settings":
            return .settings
        default:
            return .account
        }
    }

    // MARK: Compare

    private func parseCompareRoute(queryItems: [URLQueryItem]) -> Route? {
        guard let idsString = queryItems.first(where: { $0.name == "ids" })?.value else { return nil }
        let ids = idsString.split(separator: ",").map(String.init).filter { !$0.isEmpty }
        guard ids.count >= 2 else { return nil }
        return .compareListings(ids: ids)
    }

    // MARK: Search Filters

    private func parseSearchFilters(from queryItems: [URLQueryItem]) -> SearchFilters? {
        var filters = SearchFilters()
        var hasAny = false

        if let v = queryItems.first(where: { $0.name == "priceMin" })?.value, let n = Int(v) {
            filters.priceMin = n; hasAny = true
        }
        if let v = queryItems.first(where: { $0.name == "priceMax" })?.value, let n = Int(v) {
            filters.priceMax = n; hasAny = true
        }
        if let v = queryItems.first(where: { $0.name == "bedroomsMin" })?.value, let n = Int(v) {
            filters.bedroomsMin = n; hasAny = true
        }
        if let v = queryItems.first(where: { $0.name == "bedroomsMax" })?.value, let n = Int(v) {
            filters.bedroomsMax = n; hasAny = true
        }
        if let v = queryItems.first(where: { $0.name == "bathroomsMin" })?.value, let n = Int(v) {
            filters.bathroomsMin = n; hasAny = true
        }
        if let v = queryItems.first(where: { $0.name == "radius" })?.value, let d = Double(v) {
            filters.radiusKm = d; hasAny = true
        }
        if let v = queryItems.first(where: { $0.name == "lat" })?.value, let d = Double(v) {
            filters.latitude = d; hasAny = true
        }
        if let v = queryItems.first(where: { $0.name == "lon" })?.value, let d = Double(v) {
            filters.longitude = d; hasAny = true
        }
        if let v = queryItems.first(where: { $0.name == "types" })?.value {
            let typeStrings = v.split(separator: ",").map(String.init)
            let types = typeStrings.compactMap { PropertyType(rawValue: $0) }
            if !types.isEmpty {
                filters.propertyTypes = Set(types)
                hasAny = true
            }
        }

        return hasAny ? filters : nil
    }
}
