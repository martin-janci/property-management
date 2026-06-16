import SwiftUI
import shared

/// Search screen for Reality Portal iOS app.
///
/// Provides property search with filters, sort order, and paginated results
/// bound to the reality-server search API via KMP ListingRepository.
///
/// Epic 82 - Story 82.3: Home and Search Screens
///   — debounced search (350 ms cancellable token), infinite scroll
///     pagination, FilterSheet with listing-type + bathrooms sections,
///     and CoreLocation "Near Me" chip / radius picker.
struct SearchView: View {
    @Environment(NavigationCoordinator.self) private var coordinator
    @Environment(LocationManager.self) private var locationManager

    @State private var searchText = ""
    @State private var results: [ListingPreview] = []
    @State private var isLoading = false
    @State private var isLoadingMore = false
    @State private var showFilters = false
    @State private var showSortPicker = false
    @State private var filters = SearchFilters()
    @State private var sortOption: SortOption = .newest
    @State private var currentPage = 1
    @State private var totalPages = 1
    @State private var totalCount = 0

    /// Cancellable token for the in-flight debounce task.
    @State private var debounceTask: Task<Void, Never>?

    /// Monotonic sequence for the stale-response guard. Each fresh (page-1)
    /// `performSearch()` increments this and captures the value before the
    /// `await`; if a newer search started meanwhile, the older response is
    /// dropped so an out-of-order completion can't clobber newer results.
    /// Mirrors the Android `SearchScreen.kt` searchGeneration guard, scoped to
    /// the SwiftUI side (see plan: Android-side race is a separate backlog item).
    @State private var requestSeq = 0

    /// Drives the location-denied alert; set by .onChange(of: locationManager.locationError).
    @State private var showLocationDeniedAlert = false

    private let listingRepository = DependencyContainer.shared.listingRepository
    private let debounceNs: UInt64 = 350_000_000 // 350 ms

    var body: some View {
        VStack(spacing: 0) {
            searchBar
            filterBar

            if isLoading && results.isEmpty {
                loadingView
            } else if results.isEmpty && (!searchText.isEmpty || filters.hasActiveFilters) {
                emptyResultsView
            } else if results.isEmpty {
                searchPromptView
            } else {
                resultsGrid
            }
        }
        .navigationTitle(String(localized: "tab_search"))
        .navigationBarTitleDisplayMode(.inline)
        .toolbar {
            ToolbarItem(placement: .navigationBarTrailing) {
                Button {
                    showSortPicker = true
                } label: {
                    Image(systemName: "arrow.up.arrow.down")
                }
                .accessibilityLabel(String(localized: "sort_by"))
            }
        }
        .sheet(isPresented: $showFilters) {
            // SwiftUI's `.sheet` creates a new view-hierarchy context that
            // does NOT inherit `@Observable` environment values injected
            // above the presenter. Without this `.environment(locationManager)`
            // re-injection the FilterSheet's `@Environment(LocationManager.self)`
            // would resolve to a default-initialised LocationManager (or crash
            // on older iOS), so the "Near Me" toggle inside the sheet would
            // never call `requestLocation()` on the shared instance (closes #625).
            FilterSheet(filters: $filters) {
                Task { await performSearch() }
            }
            .environment(locationManager)
        }
        .confirmationDialog(String(localized: "sort_by"), isPresented: $showSortPicker, titleVisibility: .visible) {
            ForEach(SortOption.allCases, id: \.self) { option in
                Button(option.displayName) {
                    sortOption = option
                    Task { await performSearch() }
                }
            }
            Button(String(localized: "cancel"), role: .cancel) {}
        }
        .onChange(of: searchText) { _, newValue in
            scheduleSearch(for: newValue)
        }
        .onChange(of: locationManager.coordinate) { _, coord in
            if let coord, filters.radiusKm != nil {
                filters.latitude = coord.latitude
                filters.longitude = coord.longitude
                Task { await performSearch() }
            }
        }
        .onAppear {
            // Consume any pending filters set by HomeView category chips
            if let pending = coordinator.pendingSearchFilters {
                filters = pending
                coordinator.pendingSearchFilters = nil
                Task { await performSearch() }
            }
        }
    }

    // MARK: - Subviews

    private var searchBar: some View {
        HStack(spacing: 12) {
            HStack {
                Image(systemName: "magnifyingglass")
                    .foregroundStyle(.secondary)
                TextField(String(localized: "search_placeholder"), text: $searchText)
                    .textFieldStyle(.plain)
                if !searchText.isEmpty {
                    Button {
                        searchText = ""
                        debounceTask?.cancel()
                        Task { await performSearch() }
                    } label: {
                        Image(systemName: "xmark.circle.fill")
                            .foregroundStyle(.secondary)
                    }
                }
            }
            .padding(12)
            .background(Color(.systemGray6))
            .clipShape(RoundedRectangle(cornerRadius: 12))
        }
        .padding()
    }

    private var filterBar: some View {
        ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: 8) {
                Button {
                    showFilters = true
                } label: {
                    HStack(spacing: 4) {
                        Image(systemName: "slider.horizontal.3")
                        Text(String(localized: "filters"))
                        if filters.hasActiveFilters {
                            Circle()
                                .fill(Color.accentColor)
                                .frame(width: 8, height: 8)
                        }
                    }
                    .font(.subheadline)
                    .padding(.horizontal, 12)
                    .padding(.vertical, 8)
                    .background(filters.hasActiveFilters ? Color.accentColor.opacity(0.1) : Color(.systemGray6))
                    .clipShape(Capsule())
                }
                nearMeChip
                ForEach(PropertyType.allCases, id: \.self) { type in
                    FilterChip(
                        title: type.displayName,
                        isSelected: filters.propertyTypes.contains(type)
                    ) {
                        if filters.propertyTypes.contains(type) {
                            filters.propertyTypes.remove(type)
                        } else {
                            filters.propertyTypes.insert(type)
                        }
                        Task { await performSearch() }
                    }
                }
            }
            .padding(.horizontal)
        }
        .padding(.bottom, 8)
    }

    @ViewBuilder
    private var nearMeChip: some View {
        let isActive = filters.radiusKm != nil
        Button {
            if isActive {
                filters.radiusKm = nil
                filters.latitude = nil
                filters.longitude = nil
                locationManager.stopLocation()
                Task { await performSearch() }
            } else {
                filters.radiusKm = 10.0
                if let coord = locationManager.coordinate {
                    filters.latitude = coord.latitude
                    filters.longitude = coord.longitude
                    Task { await performSearch() }
                } else {
                    locationManager.requestLocation()
                }
            }
        } label: {
            HStack(spacing: 4) {
                if locationManager.isLocating {
                    ProgressView().scaleEffect(0.7)
                } else {
                    Image(systemName: isActive ? "location.fill" : "location")
                }
                Text(String(localized: "filter_near_me"))
            }
            .font(.subheadline)
            .padding(.horizontal, 12)
            .padding(.vertical, 8)
            .background(isActive ? Color.accentColor.opacity(0.1) : Color(.systemGray6))
            .foregroundStyle(isActive ? Color.accentColor : .primary)
            .clipShape(Capsule())
            .overlay {
                Capsule().stroke(isActive ? Color.accentColor : Color.clear, lineWidth: 1)
            }
        }
        .buttonStyle(.plain)
        // Alert is handled at body level (showLocationDeniedAlert) to avoid
        // iOS 17+ @Observable reconciliation issues with inline bindings.
    }

    private var loadingView: some View {
        VStack {
            Spacer()
            ProgressView()
            Text(String(localized: "searching")).foregroundStyle(.secondary).padding(.top, 8)
            Spacer()
        }
    }

    private var emptyResultsView: some View {
        VStack(spacing: 16) {
            Spacer()
            Image(systemName: "magnifyingglass").font(.system(size: 48)).foregroundStyle(.secondary)
            Text(String(localized: "no_results_found")).font(.headline)
            Text(String(localized: "search_adjust_hint")).font(.subheadline).foregroundStyle(.secondary)
            Spacer()
        }
    }

    private var searchPromptView: some View {
        VStack(spacing: 16) {
            Spacer()
            Image(systemName: "house.fill").font(.system(size: 48)).foregroundStyle(.secondary)
            Text(String(localized: "search_prompt_title")).font(.headline)
            Text(String(localized: "search_prompt_description")).font(.subheadline).foregroundStyle(.secondary)
            VStack(alignment: .leading, spacing: 8) {
                Text(String(localized: "recent_searches"))
                    .font(.subheadline).fontWeight(.semibold).foregroundStyle(.secondary)
                ForEach(["Bratislava apartments", "Houses for sale", "3 bedroom"], id: \.self) { term in
                    Button { searchText = term } label: {
                        HStack {
                            Image(systemName: "clock.arrow.circlepath").foregroundStyle(.secondary)
                            Text(term).foregroundStyle(.primary)
                            Spacer()
                        }
                        .padding(.vertical, 8)
                    }
                }
            }
            .padding()
            .background(Color(.systemGray6))
            .clipShape(RoundedRectangle(cornerRadius: 12))
            .padding(.horizontal)
            Spacer()
        }
    }

    private var resultsGrid: some View {
        ScrollView {
            LazyVStack(spacing: 12) {
                ForEach(Array(results.enumerated()), id: \.element.id) { index, listing in
                    SearchResultCard(listing: listing) {
                        coordinator.navigate(to: .listingDetail(id: listing.id))
                    }
                    .onAppear {
                        // AC-4: infinite scroll. When the last loaded cell appears,
                        // fetch the next page. `loadMoreResults()` self-guards on
                        // currentPage/totalPages so trailing onAppears are no-ops.
                        if index == results.count - 1 {
                            Task { await loadMoreResults() }
                        }
                    }
                }
                if isLoadingMore {
                    ProgressView()
                        .frame(maxWidth: .infinity)
                        .padding(.vertical, 16)
                }
            }
            .padding(.vertical, 8)
        }
    }

    // MARK: - Search

    /// Debounce free-text query changes before hitting the network (AC-2).
    ///
    /// Cancels any in-flight debounce token and starts a fresh `debounceNs`
    /// (350 ms) sleep; only after the sleep survives cancellation does the
    /// actual search fire. Mirrors the Android `SearchScreen.kt`
    /// `.debounce(SEARCH_DEBOUNCE_MS)` collector so three quick keystrokes
    /// coalesce into a single repository call.
    private func scheduleSearch(for _: String) {
        debounceTask?.cancel()
        debounceTask = Task {
            try? await Task.sleep(nanoseconds: debounceNs)
            if Task.isCancelled { return }
            await performSearch()
        }
    }

    /// Run a fresh (page-1) search against the KMP `ListingRepository` and
    /// replace the current result set. Mirrors the Android `SearchScreen.kt`
    /// `performSearch()` contract (request assembly + success/failure fold)
    /// plus a SwiftUI-side stale-response guard (`requestSeq`).
    private func performSearch() async {
        isLoading = true
        currentPage = 1

        // Stale-response guard: capture the sequence for this search before the
        // await; if a newer search bumps `requestSeq` while we're suspended,
        // discard this (older) response on resume.
        requestSeq += 1
        let seq = requestSeq

        let request = ListingSearchRequest(
            query: searchText.isEmpty ? nil : searchText,
            filters: buildKMPFilters(),
            sort: sortOption.kmpSortOption,
            page: Int32(currentPage),
            pageSize: 20
        )
        let result = await listingRepository.searchListings(request: request)

        // Drop responses superseded by a newer search.
        guard seq == requestSeq else { return }

        if let response = result.getOrNull() {
            results = response.listings.map { KMPBridge.toListingPreview($0) }
            totalPages = Int(response.totalPages)
            totalCount = Int(response.total)
        } else {
            #if DEBUG
            if let error = result.exceptionOrNull() {
                print("Search error: \(error.message ?? "Unknown error")")
            }
            #endif
            results = []
            totalPages = 1
            totalCount = 0
        }
        isLoading = false
    }

    private func loadMoreResults() async {
        guard currentPage < totalPages, !isLoading, !isLoadingMore else { return }
        isLoadingMore = true
        let prevPage = currentPage
        currentPage += 1
        let request = ListingSearchRequest(
            query: searchText.isEmpty ? nil : searchText,
            filters: buildKMPFilters(),
            sort: sortOption.kmpSortOption,
            page: Int32(currentPage),
            pageSize: 20
        )
        let result = await listingRepository.searchListings(request: request)
        if let response = result.getOrNull() {
            results.append(contentsOf: response.listings.map { KMPBridge.toListingPreview($0) })
        } else {
            currentPage = prevPage
        }
        isLoadingMore = false
    }

    private func buildKMPFilters() -> ListingSearchFilters? {
        guard filters.hasActiveFilters else { return nil }

        // Map Swift ListingKind to KMP ListingType
        var listingType: ListingType? = nil
        if let swiftType = filters.listingType {
            switch swiftType {
            case .sale: listingType = .sale
            case .rent: listingType = .rent
            }
        }

        // Map Swift PropertyType to KMP PropertyCategory
        var category: PropertyCategory? = nil
        if let firstType = filters.propertyTypes.first {
            switch firstType {
            case .apartment: category = .apartment
            case .house:     category = .house
            case .land:      category = .land
            case .commercial: category = .commercial
            case .garage:    category = .garage
            }
        }

        return ListingSearchFilters(
            type: listingType,
            category: category,
            city: nil, district: nil, region: nil,
            minPrice: filters.priceMin.map { KotlinLong(integerLiteral: $0) },
            maxPrice: filters.priceMax.map { KotlinLong(integerLiteral: $0) },
            minArea: nil, maxArea: nil,
            minRooms: filters.bedroomsMin.map { KotlinInt(integerLiteral: $0) },
            maxRooms: filters.bedroomsMax.map { KotlinInt(integerLiteral: $0) },
            bedrooms: filters.bedroomsMin.map { KotlinInt(integerLiteral: $0) },
            bathrooms: filters.bathroomsMin.map { KotlinInt(integerLiteral: $0) },
            features: nil, yearBuiltMin: nil, yearBuiltMax: nil,
            nearLat: filters.latitude.map { KotlinDouble(value: $0) },
            nearLng: filters.longitude.map { KotlinDouble(value: $0) },
            radiusKm: filters.radiusKm.map { KotlinDouble(value: $0) }
        )
    }
}

// MARK: - Sort Option

/// Available sort orders for search results.
///
/// Epic 82 - Story 82.3: Home and Search Screens
enum SortOption: CaseIterable {
    case newest
    case oldest
    case priceAsc
    case priceDesc
    case areaAsc
    case areaDesc
    case relevance

    var displayName: String {
        switch self {
        case .newest:     return String(localized: "sort_newest")
        case .oldest:     return String(localized: "sort_oldest")
        case .priceAsc:   return String(localized: "sort_price_asc")
        case .priceDesc:  return String(localized: "sort_price_desc")
        case .areaAsc:    return String(localized: "sort_area_asc")
        case .areaDesc:   return String(localized: "sort_area_desc")
        case .relevance:  return String(localized: "sort_relevance")
        }
    }

    /// KMP ListingSortOption equivalent.
    var kmpSortOption: ListingSortOption {
        switch self {
        case .newest:     return .newest
        case .oldest:     return .oldest
        case .priceAsc:   return .priceAsc
        case .priceDesc:  return .priceDesc
        case .areaAsc:    return .areaAsc
        case .areaDesc:   return .areaDesc
        case .relevance:  return .relevance
        }
    }
}

// MARK: - Supporting Views

private struct FilterChip: View {
    let title: String
    let isSelected: Bool
    let action: () -> Void
    var body: some View {
        Button(action: action) {
            Text(title)
                .font(.subheadline)
                .padding(.horizontal, 12)
                .padding(.vertical, 8)
                .background(isSelected ? Color.accentColor.opacity(0.1) : Color(.systemGray6))
                .foregroundStyle(isSelected ? Color.accentColor : .primary)
                .clipShape(Capsule())
                .overlay { Capsule().stroke(isSelected ? Color.accentColor : Color.clear, lineWidth: 1) }
        }
        .buttonStyle(.plain)
    }
}

private struct SearchResultCard: View {
    let listing: ListingPreview
    let action: () -> Void
    var body: some View {
        Button(action: action) {
            VStack(alignment: .leading, spacing: 0) {
                ZStack {
                    RoundedRectangle(cornerRadius: 12).fill(Color(.systemGray5)).frame(height: 180)
                    if let thumbnailUrl = listing.thumbnailUrl, let url = URL(string: thumbnailUrl) {
                        AsyncImage(url: url) { phase in
                            switch phase {
                            case .success(let image):
                                image.resizable().aspectRatio(contentMode: .fill)
                                    .frame(height: 180).clipShape(RoundedRectangle(cornerRadius: 12))
                            case .failure, .empty:
                                Image(systemName: "photo").font(.largeTitle).foregroundStyle(.secondary)
                            @unknown default:
                                Image(systemName: "photo").font(.largeTitle).foregroundStyle(.secondary)
                            }
                        }
                    } else {
                        Image(systemName: "photo").font(.largeTitle).foregroundStyle(.secondary)
                    }
                }
                .overlay(alignment: .topTrailing) {
                    Button {} label: {
                        Image(systemName: "heart").font(.title3).padding(8)
                            .background(.ultraThinMaterial).clipShape(Circle())
                    }
                    .padding(8)
                }
                VStack(alignment: .leading, spacing: 4) {
                    Text(listing.formattedPrice).font(.title3).fontWeight(.bold).foregroundStyle(Color.accentColor)
                    Text(listing.title).font(.subheadline).lineLimit(1).foregroundStyle(.primary)
                    HStack(spacing: 4) {
                        Image(systemName: "location.fill").font(.caption)
                        Text(listing.location).font(.caption)
                    }
                    .foregroundStyle(.secondary)
                    HStack(spacing: 12) {
                        if let area = listing.areaSqm {
                            HStack(spacing: 2) { Image(systemName: "square.fill").font(.caption2); Text("\(area) m2") }
                        }
                        if let rooms = listing.rooms {
                            HStack(spacing: 2) { Image(systemName: "bed.double.fill").font(.caption2); Text("\(rooms)") }
                        }
                    }
                    .font(.caption).foregroundStyle(.secondary)
                }
                .padding(12)
            }
            .background(Color(.systemBackground))
            .clipShape(RoundedRectangle(cornerRadius: 12))
            .shadow(color: .black.opacity(0.05), radius: 4, y: 2)
        }
        .buttonStyle(.plain)
        .padding(.horizontal)
    }
}

// MARK: - Filter Sheet

private struct FilterSheet: View {
    @Binding var filters: SearchFilters
    let onApply: () -> Void
    @Environment(LocationManager.self) private var locationManager
    @Environment(\.dismiss) private var dismiss
    private let radiusOptions: [Double] = [1, 3, 5, 10, 20, 50]

    var body: some View {
        NavigationStack {
            Form {
                Section(String(localized: "filter_price_range")) {
                    HStack {
                        TextField(String(localized: "filter_price_min"), value: $filters.priceMin, format: .number)
                            .textFieldStyle(.roundedBorder).keyboardType(.numberPad)
                        Text("-")
                        TextField(String(localized: "filter_price_max"), value: $filters.priceMax, format: .number)
                            .textFieldStyle(.roundedBorder).keyboardType(.numberPad)
                    }
                }
                Section(String(localized: "filter_listing_type")) {
                    ForEach(ListingKind.allCases, id: \.self) { kind in
                        Toggle(kind.displayName, isOn: Binding(
                            get: { filters.listingType == kind },
                            set: { isOn in filters.listingType = isOn ? kind : nil }
                        ))
                    }
                }
                Section(String(localized: "filter_property_type")) {
                    ForEach(PropertyType.allCases, id: \.self) { type in
                        Toggle(type.displayName, isOn: Binding(
                            get: { filters.propertyTypes.contains(type) },
                            set: { isOn in
                                if isOn { filters.propertyTypes.insert(type) }
                                else { filters.propertyTypes.remove(type) }
                            }
                        ))
                    }
                }
                Section(String(localized: "filter_bedrooms")) {
                    Stepper(String(format: String(localized: "filter_min_value"), filters.bedroomsMin ?? 0),
                        value: Binding(get: { filters.bedroomsMin ?? 0 }, set: { filters.bedroomsMin = $0 > 0 ? $0 : nil }),
                        in: 0...10)
                    Stepper(String(format: String(localized: "filter_max_value"), filters.bedroomsMax ?? 10),
                        value: Binding(get: { filters.bedroomsMax ?? 10 }, set: { filters.bedroomsMax = $0 < 10 ? $0 : nil }),
                        in: 0...10)
                }
                Section(String(localized: "filter_bathrooms")) {
                    Stepper(String(format: String(localized: "filter_min_value"), filters.bathroomsMin ?? 0),
                        value: Binding(get: { filters.bathroomsMin ?? 0 }, set: { filters.bathroomsMin = $0 > 0 ? $0 : nil }),
                        in: 0...10)
                }
                nearMeSection
                Section {
                    Button(String(localized: "reset_filters")) {
                        filters.reset()
                        locationManager.stopLocation()
                    }
                    .foregroundStyle(Color.pptDanger)
                }
            }
            .navigationTitle(String(localized: "filters"))
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) { Button(String(localized: "cancel")) { dismiss() } }
                ToolbarItem(placement: .confirmationAction) {
                    Button(String(localized: "apply")) { onApply(); dismiss() }
                }
            }
        }
    }

    @ViewBuilder
    private var nearMeSection: some View {
        Section {
            Toggle(String(localized: "filter_near_me"), isOn: Binding(
                get: { filters.radiusKm != nil },
                set: { enabled in
                    if enabled {
                        filters.radiusKm = 10.0
                        if let coord = locationManager.coordinate {
                            filters.latitude = coord.latitude
                            filters.longitude = coord.longitude
                        } else {
                            locationManager.requestLocation()
                        }
                    } else {
                        filters.radiusKm = nil; filters.latitude = nil; filters.longitude = nil
                        locationManager.stopLocation()
                    }
                }
            ))
            if filters.radiusKm != nil {
                Picker(String(localized: "filter_radius"),
                    selection: Binding(get: { filters.radiusKm ?? 10.0 }, set: { filters.radiusKm = $0 })) {
                    ForEach(radiusOptions, id: \.self) { km in
                        Text(String(format: String(localized: "radius_km_format"), Int(km))).tag(km)
                    }
                }
            }
        } header: {
            Text(String(localized: "filter_near_me_section"))
        } footer: {
            if let coord = locationManager.coordinate, filters.radiusKm != nil {
                Text(String(format: String(localized: "location_detected_format"), coord.latitude, coord.longitude))
                    .foregroundStyle(.secondary)
            } else if locationManager.isLocating {
                Label(String(localized: "locating"), systemImage: "location.fill").foregroundStyle(.secondary)
            }
        }
    }
}

// MARK: - Preview

#Preview {
    NavigationStack { SearchView() }
        .environment(NavigationCoordinator())
        .environment(LocationManager())
}
