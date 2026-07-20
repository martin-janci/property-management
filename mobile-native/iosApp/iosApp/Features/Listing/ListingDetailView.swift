import MapKit
import SwiftUI
import shared

/// Listing detail screen for Reality Portal iOS app.
///
/// Displays full property information, photos, and contact options.
///
/// Favorites state is driven by the app-wide `FavoritesService` so that
/// toggling a favorite here is immediately reflected in `FavoritesView`
/// without requiring a separate network call or manual refresh.
///
/// Section order and visibility are driven by the shared resolved layout
/// (`ResolvedLayoutScreen`). The layout is seeded from the compiled default
/// (`LayoutDefaults.shared.listingDetail`) and replaced at most once
/// during the initial load — no mid-view swaps after content renders.
///
/// Epic 82 - Story 82.4: Listing Detail and Favorites
struct ListingDetailView: View {
    @Environment(NavigationCoordinator.self) private var coordinator
    @Environment(AuthManager.self) private var authManager
    @Environment(FavoritesService.self) private var favoritesService
    @Environment(\.dismiss) private var dismiss

    let listingId: String

    @State private var listing: ListingDetailModel?
    @State private var isLoading = true
    @State private var showGallery = false
    @State private var showInquirySheet = false
    @State private var errorMessage: String?

    /// Resolved layout seeded from the compiled default. Replaced at most once
    /// during `loadListing()` before `isLoading` flips to false.
    @State private var layout: ResolvedLayoutScreen = LayoutDefaults.shared.listingDetail

    /// Derived from `FavoritesService` — always in sync with `FavoritesView`.
    private var isFavorite: Bool {
        favoritesService.isFavorite(listingId: listingId)
    }

    private let listingRepository = DependencyContainer.shared.listingRepository
    private let layoutRepository = DependencyContainer.shared.layoutRepository

    var body: some View {
        Group {
            if isLoading {
                loadingView
            } else if let listing = listing {
                listingContent(listing)
            } else {
                errorView
            }
        }
        .navigationBarTitleDisplayMode(.inline)
        .toolbar {
            ToolbarItem(placement: .navigationBarTrailing) {
                HStack(spacing: 16) {
                    favoriteButton
                    shareButton
                }
            }
        }
        .sheet(isPresented: $showInquirySheet) {
            NewInquirySheet(listingId: listingId)
        }
        .fullScreenCover(isPresented: $showGallery) {
            PhotoGalleryView(photos: listing?.photos ?? [])
        }
        .task {
            await loadListing()
        }
    }

    // MARK: - Subviews

    private var loadingView: some View {
        VStack(spacing: 16) {
            ProgressView()
            Text(String(localized: "loading_listing"))
                .foregroundStyle(.secondary)
        }
    }

    private var errorView: some View {
        VStack(spacing: 16) {
            Image(systemName: "exclamationmark.triangle")
                .font(.largeTitle)
                .foregroundStyle(Color.pptWarning)
            Text(errorMessage ?? String(localized: "failed_to_load_listing"))
                .foregroundStyle(.secondary)
            Button(String(localized: "retry")) {
                Task { await loadListing() }
            }
        }
    }

    private func listingContent(_ listing: ListingDetailModel) -> some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 0) {
                // Photo header is rendered outside the padded content area so
                // it bleeds edge-to-edge. It is always rendered (it corresponds
                // to gallery.v1 but its position is structural, not dispatched).
                photoHeader(listing)

                VStack(alignment: .leading, spacing: 20) {
                    managedSections(listing)

                    // locationSection (map) is UNMANAGED — always rendered after
                    // all managed sections, regardless of layout order.
                    // A divider precedes it only when a managed section rendered.
                    if !buildRenderPlan().isEmpty {
                        Divider()
                    }
                    locationSection(listing)
                }
                .padding()
            }
        }
    }

    /// Represents a single renderable slot in the managed section list.
    ///
    /// The render plan is computed eagerly (outside the ViewBuilder) so that
    /// `needsDivider` can be determined without mutating local state inside the
    /// result builder — which SwiftUI's result builder does not support.
    private enum LayoutSlot {
        case priceSection
        case featuresRow
        case descriptionSection
        case amenitiesGrid
        case agentContact
        case placeholder
    }

    /// Builds the ordered render plan from the resolved layout, skipping no-ops
    /// and computing `needsDivider` for each slot up-front.
    private func buildRenderPlan() -> [(slot: LayoutSlot, needsDivider: Bool)] {
        // Kotlin List<ResolvedLayoutSection> bridges as a typed Swift array —
        // no cast needed (see DependencyContainer's KMP consumers). The @State
        // seed guarantees this is never empty-by-accident.
        let sections = layout.sections
        var plan: [(slot: LayoutSlot, needsDivider: Bool)] = []
        var didRenderManaged = false

        for section in sections {
            // gallery.v1 is structural (rendered as photoHeader); skip.
            if section.type == "gallery.v1" { continue }

            let slot: LayoutSlot?
            if section.isPlaceholder {
                slot = .placeholder
            } else if section.isVisible {
                switch section.type {
                case "listing-header.v1": slot = .priceSection
                case "key-details.v1":   slot = .featuresRow
                case "description.v1":   slot = .descriptionSection
                case "features.v1":      slot = .amenitiesGrid
                case "agent-contact.v1": slot = .agentContact
                default:                 slot = nil  // additional-info.v1, resources.v1, unknown
                }
            } else {
                slot = nil  // hidden
            }

            if let resolved = slot {
                plan.append((slot: resolved, needsDivider: didRenderManaged))
                didRenderManaged = true
            }
        }

        return plan
    }

    /// Renders the managed sections in layout order with dividers between adjacent visible ones.
    ///
    /// Section mapping (plan §Architecture):
    ///   gallery.v1          → photoHeader  (structural — handled above; skipped here)
    ///   listing-header.v1   → priceSection
    ///   key-details.v1      → featuresRow
    ///   description.v1      → descriptionSection
    ///   features.v1         → amenitiesGrid
    ///   agent-contact.v1    → agentCard + contactButton (both gated together)
    ///   additional-info.v1  → no-op
    ///   resources.v1        → no-op
    ///   unknown             → no-op
    ///   placeholder         → LayoutPlaceholderSection (one block)
    private func managedSections(_ listing: ListingDetailModel) -> some View {
        // Build the render plan outside the ViewBuilder closure so we can use
        // imperative logic (var, for-loop) without conflicting with the result
        // builder's expression-based syntax.
        let plan = buildRenderPlan()
        return ForEach(0..<plan.count, id: \.self) { index in
            let entry = plan[index]
            if entry.needsDivider {
                Divider()
            }
            switch entry.slot {
            case .priceSection:
                priceSection(listing)
            case .featuresRow:
                featuresRow(listing)
            case .descriptionSection:
                descriptionSection(listing)
            case .amenitiesGrid:
                amenitiesGrid(listing)
            case .agentContact:
                agentCard(listing)
                contactButton
            case .placeholder:
                LayoutPlaceholderSection()
            }
        }
    }

    private func photoHeader(_ listing: ListingDetailModel) -> some View {
        Button {
            showGallery = true
        } label: {
            ZStack(alignment: .bottomTrailing) {
                // Main image placeholder or async image
                ZStack {
                    Rectangle()
                        .fill(Color(.systemGray5))
                        .frame(height: 280)

                    if let firstPhoto = listing.photos.first,
                       let url = URL(string: firstPhoto) {
                        AsyncImage(url: url) { phase in
                            switch phase {
                            case .success(let image):
                                image
                                    .resizable()
                                    .aspectRatio(contentMode: .fill)
                                    .frame(height: 280)
                                    .clipped()
                            case .failure, .empty:
                                Image(systemName: "photo")
                                    .font(.largeTitle)
                                    .foregroundStyle(.secondary)
                            @unknown default:
                                Image(systemName: "photo")
                                    .font(.largeTitle)
                                    .foregroundStyle(.secondary)
                            }
                        }
                    } else {
                        Image(systemName: "photo")
                            .font(.largeTitle)
                            .foregroundStyle(.secondary)
                    }
                }

                // Photo count badge
                HStack(spacing: 4) {
                    Image(systemName: "photo.on.rectangle")
                    Text("\(listing.photos.count)")
                }
                .font(.caption)
                .fontWeight(.medium)
                .padding(.horizontal, 8)
                .padding(.vertical, 4)
                .background(.ultraThinMaterial)
                .clipShape(Capsule())
                .padding(12)
            }
        }
        .buttonStyle(.plain)
    }

    private func priceSection(_ listing: ListingDetailModel) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(listing.formattedPrice)
                .font(.title)
                .fontWeight(.bold)
                .foregroundStyle(Color.accentColor)

            Text(listing.title)
                .font(.title3)
                .fontWeight(.semibold)

            HStack(spacing: 4) {
                Image(systemName: "location.fill")
                    .font(.caption)
                Text(listing.address)
                    .font(.subheadline)
            }
            .foregroundStyle(.secondary)
        }
    }

    private func featuresRow(_ listing: ListingDetailModel) -> some View {
        HStack(spacing: 24) {
            if let area = listing.areaSqm {
                FeatureItem(icon: "square.fill", value: "\(area)", unit: "m2")
            }
            if let rooms = listing.rooms {
                FeatureItem(icon: "bed.double.fill", value: "\(rooms)", unit: "rooms")
            }
            if let bathrooms = listing.bathrooms {
                FeatureItem(icon: "shower.fill", value: "\(bathrooms)", unit: "baths")
            }
        }
        .padding()
        .background(Color(.systemGray6))
        .clipShape(RoundedRectangle(cornerRadius: 12))
    }

    private func descriptionSection(_ listing: ListingDetailModel) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(String(localized: "description"))
                .font(.headline)

            Text(listing.description)
                .font(.body)
                .foregroundStyle(.secondary)
        }
    }

    private func amenitiesGrid(_ listing: ListingDetailModel) -> some View {
        VStack(alignment: .leading, spacing: 12) {
            Text(String(localized: "amenities"))
                .font(.headline)

            LazyVGrid(columns: [
                GridItem(.flexible()),
                GridItem(.flexible())
            ], spacing: 12) {
                ForEach(listing.amenities, id: \.self) { amenity in
                    HStack(spacing: 8) {
                        Image(systemName: "checkmark.circle.fill")
                            .foregroundStyle(Color.pptSuccess)
                        Text(amenity)
                            .font(.subheadline)
                        Spacer()
                    }
                }
            }
        }
    }

    private func locationSection(_ listing: ListingDetailModel) -> some View {
        VStack(alignment: .leading, spacing: 12) {
            Text(String(localized: "location"))
                .font(.headline)

            // Inline map preview when coordinates are available, otherwise a
            // graceful placeholder. Tapping either opens the full-screen
            // ListingMapView. The preview is intentionally non-interactive
            // (no gestures) so the parent ScrollView keeps vertical scrolling.
            Button {
                coordinator.navigate(to: .listingMap(id: listingId))
            } label: {
                ZStack(alignment: .bottomLeading) {
                    if let coordinate = coordinate(for: listing) {
                        Map(
                            initialPosition: .region(
                                MKCoordinateRegion(
                                    center: coordinate,
                                    span: MKCoordinateSpan(latitudeDelta: 0.01, longitudeDelta: 0.01)
                                )
                            ),
                            interactionModes: []
                        ) {
                            Marker(listing.title, coordinate: coordinate)
                                .tint(Color.accentColor)
                        }
                        .frame(height: 160)
                        .clipShape(RoundedRectangle(cornerRadius: 12))
                        .allowsHitTesting(false)
                    } else {
                        RoundedRectangle(cornerRadius: 12)
                            .fill(Color(.systemGray5))
                            .frame(height: 160)
                            .overlay {
                                VStack {
                                    Image(systemName: "map")
                                        .font(.largeTitle)
                                        .foregroundStyle(.secondary)
                                    Text(listing.address)
                                        .font(.caption)
                                        .foregroundStyle(.secondary)
                                }
                            }
                    }

                    Label(String(localized: "view_on_map"), systemImage: "map.fill")
                        .font(.caption)
                        .fontWeight(.medium)
                        .padding(.horizontal, 10)
                        .padding(.vertical, 6)
                        .background(.ultraThinMaterial)
                        .clipShape(Capsule())
                        .padding(12)
                }
            }
            .buttonStyle(.plain)
        }
    }

    /// Builds a `CLLocationCoordinate2D` only when both lat/long are present.
    private func coordinate(for listing: ListingDetailModel) -> CLLocationCoordinate2D? {
        guard let lat = listing.latitude, let lon = listing.longitude else {
            return nil
        }
        return CLLocationCoordinate2D(latitude: lat, longitude: lon)
    }

    private func agentCard(_ listing: ListingDetailModel) -> some View {
        VStack(alignment: .leading, spacing: 12) {
            Text(String(localized: "contact_agent"))
                .font(.headline)

            HStack(spacing: 12) {
                // Agent avatar placeholder
                Circle()
                    .fill(Color(.systemGray4))
                    .frame(width: 56, height: 56)
                    .overlay {
                        Text(listing.agentName.prefix(1).uppercased())
                            .font(.title3)
                            .fontWeight(.semibold)
                            .foregroundStyle(.secondary)
                    }

                VStack(alignment: .leading, spacing: 4) {
                    Text(listing.agentName)
                        .font(.subheadline)
                        .fontWeight(.semibold)

                    Text(listing.agentPhone ?? String(localized: "contact_via_inquiry"))
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }

                Spacer()

                if let phone = listing.agentPhone {
                    Button {
                        // Call agent
                        if let url = URL(string: "tel:\(phone)") {
                            UIApplication.shared.open(url)
                        }
                    } label: {
                        Image(systemName: "phone.fill")
                            .padding(12)
                            .background(Color.pptSuccess)
                            .foregroundStyle(.white)
                            .clipShape(Circle())
                    }
                }
            }
            .padding()
            .background(Color(.systemGray6))
            .clipShape(RoundedRectangle(cornerRadius: 12))
        }
    }

    private var contactButton: some View {
        Button {
            showInquirySheet = true
        } label: {
            HStack {
                Image(systemName: "envelope.fill")
                Text(String(localized: "send_inquiry"))
            }
            .frame(maxWidth: .infinity)
            .padding()
            .background(Color.accentColor)
            .foregroundStyle(.white)
            .clipShape(RoundedRectangle(cornerRadius: 12))
        }
        .padding(.top, 8)
    }

    private var favoriteButton: some View {
        Button {
            Task {
                await toggleFavorite()
            }
        } label: {
            Image(systemName: isFavorite ? "heart.fill" : "heart")
                .foregroundStyle(isFavorite ? Color.pptDanger : Color.primary)
                .animation(.spring(response: 0.3, dampingFraction: 0.6), value: isFavorite)
        }
    }

    private var shareButton: some View {
        ShareLink(
            item: URL(string: "\(Configuration.shared.webBaseUrl)/listing/\(listingId)")!,
            subject: Text(listing?.title ?? String(localized: "property")),
            message: Text(listing?.formattedPrice ?? "")
        ) {
            Image(systemName: "square.and.arrow.up")
        }
    }

    // MARK: - Data Loading

    private func loadListing() async {
        isLoading = true
        errorMessage = nil

        // Fetch layout and listing concurrently so both settle before the
        // managed sections appear. The layout never throws domain-wise but
        // surfaces as `async throws` in Swift due to KMP bridging — wrap in
        // `try?` and fall back to the already-seeded default on nil.
        async let layoutFetch: ResolvedLayoutScreen? = try? await layoutRepository.getListingDetailLayout()
        async let listingFetch = listingRepository.getListingDetail(listingId: listingId)

        let (fetchedLayout, result) = await (layoutFetch, listingFetch)

        if let resolved = fetchedLayout {
            layout = resolved
        }

        if let kmpListing = result.getOrNull() {
            listing = KMPBridge.toListingDetail(kmpListing)
        } else if let error = result.exceptionOrNull() {
            errorMessage = error.message ?? "Failed to load listing"
        }

        isLoading = false
    }

    private func toggleFavorite() async {
        guard authManager.isAuthenticated else {
            // Prompt to sign in — store the intended destination so the user
            // is returned here after successful SSO login.
            coordinator.pendingDestination = .listingDetail(id: listingId)
            coordinator.navigate(to: .login)
            return
        }

        // Delegate toggle to FavoritesService. The service applies an
        // optimistic update so isFavorite reflects the new state immediately.
        await favoritesService.toggle(listingId: listingId)
    }
}

// MARK: - Supporting Views

private struct FeatureItem: View {
    let icon: String
    let value: String
    let unit: String

    var body: some View {
        VStack(spacing: 4) {
            Image(systemName: icon)
                .font(.title3)
                .foregroundStyle(.secondary)
            Text(value)
                .font(.headline)
            Text(unit)
                .font(.caption)
                .foregroundStyle(.secondary)
        }
        .frame(maxWidth: .infinity)
    }
}

// MARK: - New Inquiry Sheet

private struct NewInquirySheet: View {
    let listingId: String
    @Environment(\.dismiss) private var dismiss
    @Environment(AuthManager.self) private var authManager

    @State private var message = ""
    @State private var contactPreference = ContactPreference.email
    @State private var isSending = false
    @State private var errorMessage: String?

    private var inquiryRepository: InquiryRepository {
        DependencyContainer.shared.makeAuthenticatedInquiryRepository(
            sessionToken: authManager.getSessionToken()
        )
    }

    var body: some View {
        NavigationStack {
            Form {
                Section(String(localized: "inquiry_your_message")) {
                    TextEditor(text: $message)
                        .frame(minHeight: 150)
                }

                Section(String(localized: "inquiry_contact_preference")) {
                    Picker(String(localized: "inquiry_prefer_contact"), selection: $contactPreference) {
                        Text(String(localized: "email")).tag(ContactPreference.email)
                        Text(String(localized: "phone")).tag(ContactPreference.phone)
                        Text(String(localized: "either")).tag(ContactPreference.either)
                    }
                }

                if let error = errorMessage {
                    Section {
                        Text(error)
                            .foregroundStyle(Color.pptDanger)
                    }
                }
            }
            .navigationTitle(String(localized: "send_inquiry"))
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button(String(localized: "cancel")) {
                        dismiss()
                    }
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button(String(localized: "send")) {
                        Task {
                            await sendInquiry()
                        }
                    }
                    .disabled(message.isEmpty || isSending)
                }
            }
        }
    }

    private func sendInquiry() async {
        isSending = true
        errorMessage = nil

        let request = CreateInquiryRequest(
            listingId: listingId,
            message: message,
            name: nil,
            email: nil,
            phone: nil
        )

        let result = await inquiryRepository.createInquiry(request: request)

        if result.getOrNull() != nil {
            dismiss()
        } else if let error = result.exceptionOrNull() {
            errorMessage = error.message ?? "Failed to send inquiry"
        }

        isSending = false
    }
}

private enum ContactPreference: String, CaseIterable {
    case email
    case phone
    case either
}

// MARK: - Photo Gallery View

/// Full-screen photo gallery with swipe-to-advance and pinch-to-zoom.
///
/// Each photo page manages its own zoom state so that navigating away from a
/// zoomed page (by swiping left/right) resets zoom for the next page.
/// A double-tap on a zoomed photo resets zoom; a double-tap when at 1x zooms
/// to 2.5x. Pinch is clamped to [1x, 5x] to prevent excessive magnification.
private struct PhotoGalleryView: View {
    let photos: [String]
    @Environment(\.dismiss) private var dismiss

    @State private var currentIndex = 0

    var body: some View {
        ZStack(alignment: .topLeading) {
            Color.black.ignoresSafeArea() // fullscreen photo viewer intentionally black

            TabView(selection: $currentIndex) {
                ForEach(photos.indices, id: \.self) { index in
                    ZoomablePhotoPage(photoUrl: photos[index])
                        .tag(index)
                }
            }
            .tabViewStyle(.page(indexDisplayMode: .never))

            // Close button
            Button {
                dismiss()
            } label: {
                Image(systemName: "xmark.circle.fill")
                    .font(.title)
                    .foregroundStyle(.white)
                    .padding()
            }

            // Photo counter
            Text("\(currentIndex + 1) / \(photos.count)")
                .foregroundStyle(.white)
                .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .bottom)
                .padding()
        }
    }
}

// MARK: - Zoomable Photo Page

/// A single photo page with pinch-to-zoom and double-tap-to-zoom support.
///
/// Zoom state is held locally so that swiping to another page starts fresh at 1x.
private struct ZoomablePhotoPage: View {
    let photoUrl: String

    // Current committed scale factor (set when the gesture ends).
    @State private var scale: CGFloat = 1.0
    // Live delta from the active MagnificationGesture (resets on each gesture start).
    @State private var gestureScale: CGFloat = 1.0
    // Offset for panning when zoomed.
    @State private var offset: CGSize = .zero
    @State private var gestureOffset: CGSize = .zero

    private let minScale: CGFloat = 1.0
    private let maxScale: CGFloat = 5.0
    private let doubleTapZoom: CGFloat = 2.5

    /// The combined effective scale including the in-progress gesture delta.
    private var effectiveScale: CGFloat {
        min(max(scale * gestureScale, minScale), maxScale)
    }

    var body: some View {
        ZStack {
            if let url = URL(string: photoUrl) {
                AsyncImage(url: url) { phase in
                    switch phase {
                    case .success(let image):
                        image
                            .resizable()
                            .aspectRatio(contentMode: .fit)
                            .scaleEffect(effectiveScale)
                            .offset(x: offset.width + gestureOffset.width,
                                    y: offset.height + gestureOffset.height)
                            // Magnification is always active. The drag gesture
                            // is mask-disabled at 1x so the parent TabView's
                            // swipe-to-page recognizer wins horizontal swipes
                            // when the photo is not zoomed in (closes #618).
                            // Without this, DragGesture competed with TabView
                            // even though `gestureOffset` was guarded — the
                            // recognizer still consumed touches arbitrarily.
                            .gesture(magnificationGesture)
                            .gesture(
                                dragGesture,
                                including: scale > minScale ? .all : .none
                            )
                            .onTapGesture(count: 2) {
                                handleDoubleTap()
                            }
                            .animation(.interactiveSpring(), value: scale)
                            .animation(.interactiveSpring(), value: offset)
                    case .failure, .empty:
                        Image(systemName: "photo")
                            .font(.largeTitle)
                            .foregroundStyle(.secondary)
                    @unknown default:
                        Image(systemName: "photo")
                            .font(.largeTitle)
                            .foregroundStyle(.secondary)
                    }
                }
            } else {
                Rectangle()
                    .fill(Color(.systemGray5))
                    .overlay {
                        Image(systemName: "photo")
                            .font(.largeTitle)
                            .foregroundStyle(.secondary)
                    }
            }
        }
    }

    // MARK: - Gestures

    private var magnificationGesture: some Gesture {
        MagnificationGesture()
            .onChanged { value in
                gestureScale = value
            }
            .onEnded { value in
                let newScale = min(max(scale * value, minScale), maxScale)
                scale = newScale
                gestureScale = 1.0
                // When zoomed back to 1x reset any accumulated pan offset.
                if scale <= minScale {
                    withAnimation(.spring()) {
                        offset = .zero
                    }
                }
            }
    }

    private var dragGesture: some Gesture {
        DragGesture()
            .onChanged { value in
                // Only allow panning when zoomed in.
                guard scale > minScale else { return }
                gestureOffset = value.translation
            }
            .onEnded { value in
                guard scale > minScale else { return }
                offset = CGSize(
                    width: offset.width + value.translation.width,
                    height: offset.height + value.translation.height
                )
                gestureOffset = .zero
            }
    }

    // MARK: - Double Tap

    private func handleDoubleTap() {
        withAnimation(.spring(response: 0.35, dampingFraction: 0.75)) {
            if scale > minScale {
                // Already zoomed -- reset to fit.
                scale = minScale
                offset = .zero
            } else {
                scale = doubleTapZoom
            }
        }
    }
}

// MARK: - Preview Data

/// ListingDetail type alias for backwards compatibility
typealias ListingDetail = ListingDetailModel

// MARK: - Preview

#Preview {
    NavigationStack {
        ListingDetailView(listingId: "1")
    }
    .environment(NavigationCoordinator())
    .environment(AuthManager())
    .environment(FavoritesService())
}
