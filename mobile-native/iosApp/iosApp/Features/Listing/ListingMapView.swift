import MapKit
import SwiftUI
import shared

/// Full-screen map for a single listing.
///
/// Reached from `ListingDetailView`'s "View on Map" button via
/// `Route.listingMap(id:)`. Previously the `Route.listingMap` destination in
/// `MainTabView.destinationView()` was a stub `Text` placeholder; this view is
/// the real implementation (Epic 82 - Story 82.4 polish).
///
/// The listing detail is re-fetched here (rather than threading the already
/// loaded `ListingDetailModel` through the navigation route) because
/// `Route.listingMap` only carries the listing id, keeping the route
/// `Hashable`/`Codable` for state restoration. The fetch reuses the same
/// `listingRepository` + `KMPBridge` mapping path as `ListingDetailView`.
struct ListingMapView: View {
    let listingId: String

    @State private var listing: ListingDetailModel?
    @State private var isLoading = true
    @State private var errorMessage: String?
    @State private var cameraPosition: MapCameraPosition = .automatic

    private let listingRepository = DependencyContainer.shared.listingRepository

    /// Roughly a few city blocks so the pin has useful surrounding context.
    private let spanDegrees = 0.01

    var body: some View {
        Group {
            if isLoading {
                ProgressView()
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else if let coordinate = listingCoordinate {
                mapContent(coordinate: coordinate)
            } else {
                unavailableView
            }
        }
        .navigationTitle(String(localized: "location"))
        .navigationBarTitleDisplayMode(.inline)
        .task {
            await loadListing()
        }
    }

    // MARK: - Subviews

    private func mapContent(coordinate: CLLocationCoordinate2D) -> some View {
        Map(position: $cameraPosition) {
            Marker(listing?.title ?? String(localized: "property"), coordinate: coordinate)
                .tint(Color.accentColor)
        }
        .mapControls {
            MapUserLocationButton()
            MapCompass()
            MapScaleView()
        }
        .safeAreaInset(edge: .bottom) {
            directionsBar(coordinate: coordinate)
        }
        .ignoresSafeArea(edges: .bottom)
    }

    private func directionsBar(coordinate: CLLocationCoordinate2D) -> some View {
        Button {
            openInMaps(coordinate: coordinate)
        } label: {
            HStack {
                Image(systemName: "arrow.triangle.turn.up.right.diamond.fill")
                Text(String(localized: "get_directions"))
            }
            .frame(maxWidth: .infinity)
            .padding()
            .background(Color.accentColor)
            .foregroundStyle(.white)
            .clipShape(RoundedRectangle(cornerRadius: 12))
        }
        .padding()
        .background(.ultraThinMaterial)
    }

    private var unavailableView: some View {
        VStack(spacing: 16) {
            Image(systemName: "mappin.slash")
                .font(.largeTitle)
                .foregroundStyle(.secondary)
            Text(errorMessage ?? String(localized: "location_unavailable"))
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
        }
        .padding()
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    // MARK: - Derived

    private var listingCoordinate: CLLocationCoordinate2D? {
        guard let lat = listing?.latitude, let lon = listing?.longitude else {
            return nil
        }
        return CLLocationCoordinate2D(latitude: lat, longitude: lon)
    }

    // MARK: - Actions

    /// Hands off to Apple Maps for turn-by-turn directions to the listing.
    private func openInMaps(coordinate: CLLocationCoordinate2D) {
        let placemark = MKPlacemark(coordinate: coordinate)
        let item = MKMapItem(placemark: placemark)
        item.name = listing?.title ?? String(localized: "property")
        item.openInMaps(launchOptions: [
            MKLaunchOptionsDirectionsModeKey: MKLaunchOptionsDirectionsModeDriving
        ])
    }

    // MARK: - Data Loading

    private func loadListing() async {
        isLoading = true
        errorMessage = nil

        let result = await listingRepository.getListingDetail(listingId: listingId)

        if let kmpListing = result.getOrNull() {
            let model = KMPBridge.toListingDetail(kmpListing)
            listing = model
            if let lat = model.latitude, let lon = model.longitude {
                cameraPosition = .region(
                    MKCoordinateRegion(
                        center: CLLocationCoordinate2D(latitude: lat, longitude: lon),
                        span: MKCoordinateSpan(latitudeDelta: spanDegrees, longitudeDelta: spanDegrees)
                    )
                )
            }
        } else if let error = result.exceptionOrNull() {
            errorMessage = error.message ?? String(localized: "failed_to_load_listing")
        }

        isLoading = false
    }
}

// MARK: - Preview

#if DEBUG
#Preview {
    NavigationStack {
        ListingMapView(listingId: "1")
    }
}
#endif
