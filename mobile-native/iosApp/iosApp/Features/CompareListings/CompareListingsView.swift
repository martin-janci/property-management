import SwiftUI
import shared

/// Side-by-side listing comparison (UC-46.5).
///
/// Brings the iOS surface up to parity with the Android compare flow.
/// Pure-UI feature: takes a set of listing IDs collected from
/// elsewhere (favorites, search results) and renders the listings'
/// price / area / room counts in a horizontally-scrollable grid so
/// the user can pick between them.
///
/// The KMP `ListingRepository` already exposes
/// `getListingDetail(id:)`; this view fans those calls out in
/// parallel via TaskGroup and renders whatever returns.
struct CompareListingsView: View {
    @Environment(NavigationCoordinator.self) private var coordinator
    @Environment(AuthManager.self) private var authManager

    /// Listing IDs to compare. Up to 4 fit comfortably on a phone screen.
    let listingIds: [String]

    @State private var listings: [ListingDetail] = []
    @State private var isLoading = true
    @State private var errorMessage: String?

    private var listingRepository: ListingRepository {
        DependencyContainer.shared.makeAuthenticatedListingRepository(
            sessionToken: authManager.getSessionToken()
        )
    }

    var body: some View {
        Group {
            if isLoading {
                ProgressView().frame(maxWidth: .infinity, maxHeight: .infinity)
            } else if let errorMessage {
                errorView(errorMessage)
            } else if listings.isEmpty {
                emptyView
            } else {
                comparisonGrid
            }
        }
        .navigationTitle("Compare")
        .task { await loadListings() }
    }

    private func errorView(_ message: String) -> some View {
        VStack(spacing: 12) {
            Image(systemName: "exclamationmark.triangle")
                .font(.largeTitle)
                .foregroundStyle(.secondary)
            Text("Couldn't load listings")
                .font(.headline)
            Text(message)
                .font(.subheadline)
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
                .padding(.horizontal)
            Button("Retry") { Task { await loadListings() } }
                .buttonStyle(.borderedProminent)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private var emptyView: some View {
        VStack(spacing: 12) {
            Image(systemName: "rectangle.on.rectangle.slash")
                .font(.largeTitle)
                .foregroundStyle(.secondary)
            Text("Nothing to compare")
                .font(.headline)
            Text("Pick at least two listings from search or favorites to compare them here.")
                .font(.subheadline)
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
                .padding(.horizontal)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private var comparisonGrid: some View {
        ScrollView(.horizontal) {
            HStack(alignment: .top, spacing: 12) {
                ForEach(listings, id: \.id) { listing in
                    column(for: listing)
                }
            }
            .padding()
        }
    }

    @ViewBuilder
    private func column(for listing: ListingDetail) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(listing.title)
                .font(.headline)
                .lineLimit(2)
            Text(FormatUtils.shared.formatPrice(amount: Int64(truncating: listing.price), currency: listing.currency))
                .font(.title3)
                .foregroundStyle(.tint)
            Divider()
            stat(label: "Area", value: "\(Int(listing.areaSqm)) m²")
            if let rooms = listing.rooms {
                stat(label: "Rooms", value: "\(rooms)")
            }
            if let bedrooms = listing.bedrooms {
                stat(label: "Bedrooms", value: "\(bedrooms)")
            }
            if let bathrooms = listing.bathrooms {
                stat(label: "Bathrooms", value: "\(bathrooms)")
            }
            if let yearBuilt = listing.yearBuilt {
                stat(label: "Built", value: "\(yearBuilt)")
            }
            stat(label: "City", value: listing.address.city)
            Spacer()
            Button("Open") {
                coordinator.navigate(to: .listingDetail(id: listing.id))
            }
            .buttonStyle(.bordered)
        }
        .frame(width: 220)
        .padding()
        .background(.thinMaterial)
        .clipShape(RoundedRectangle(cornerRadius: 12))
    }

    @ViewBuilder
    private func stat(label: String, value: String) -> some View {
        HStack {
            Text(label).foregroundStyle(.secondary)
            Spacer()
            Text(value).fontWeight(.medium)
        }
        .font(.subheadline)
    }

    private func loadListings() async {
        isLoading = true
        errorMessage = nil
        var collected: [ListingDetail] = []
        var firstError: String?
        // Fetch in parallel; preserve order so the UI matches the input
        // sequence the user picked.
        await withTaskGroup(of: (Int, Result<ListingDetail, Error>).self) { group in
            for (index, id) in listingIds.enumerated() {
                group.addTask {
                    do {
                        let detail = try await listingRepository.getListingDetail(listingId: id)
                        return (index, .success(detail))
                    } catch {
                        return (index, .failure(error))
                    }
                }
            }
            var indexed: [(Int, ListingDetail)] = []
            for await (index, result) in group {
                switch result {
                case .success(let listing):
                    indexed.append((index, listing))
                case .failure(let error):
                    if firstError == nil {
                        firstError = error.localizedDescription
                    }
                }
            }
            collected = indexed.sorted { $0.0 < $1.0 }.map { $0.1 }
        }
        listings = collected
        if collected.isEmpty, let firstError {
            errorMessage = firstError
        }
        isLoading = false
    }
}
