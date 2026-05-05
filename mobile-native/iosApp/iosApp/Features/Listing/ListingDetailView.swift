import SwiftUI
import shared

/// Listing detail screen for Reality Portal iOS app.
///
/// Displays full property information, photos, and contact options.
///
/// Epic 82 - Story 82.4: Listing Detail and Favorites
struct ListingDetailView: View {
    @Environment(NavigationCoordinator.self) private var coordinator
    @Environment(AuthManager.self) private var authManager
    @Environment(\.dismiss) private var dismiss

    let listingId: String

    @State private var listing: ListingDetailModel?
    @State private var isLoading = true
    @State private var isFavorite = false
    @State private var showGallery = false
    @State private var showInquirySheet = false
    @State private var errorMessage: String?

    private let listingRepository = DependencyContainer.shared.listingRepository
    private var favoritesRepository: FavoritesRepository {
        DependencyContainer.shared.makeAuthenticatedFavoritesRepository(
            sessionToken: authManager.getSessionToken()
        )
    }

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
                // Photo header
                photoHeader(listing)

                VStack(alignment: .leading, spacing: 20) {
                    // Price and title
                    priceSection(listing)

                    // Key features
                    featuresRow(listing)

                    Divider()

                    // Description
                    descriptionSection(listing)

                    Divider()

                    // Amenities
                    amenitiesGrid(listing)

                    Divider()

                    // Location
                    locationSection(listing)

                    Divider()

                    // Agent contact
                    agentCard(listing)

                    // Contact button
                    contactButton
                }
                .padding()
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

            // Map placeholder
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

            Button {
                coordinator.navigate(to: .listingMap(id: listingId))
            } label: {
                HStack {
                    Image(systemName: "map.fill")
                    Text(String(localized: "view_on_map"))
                }
                .font(.subheadline)
            }
        }
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

        // Load listing detail from KMP
        let result = await listingRepository.getListingDetail(listingId: listingId)

        if let kmpListing = result.getOrNull() {
            listing = KMPBridge.toListingDetail(kmpListing)

            // Check if this listing is favorited
            if authManager.isAuthenticated {
                await checkFavoriteStatus()
            }
        } else if let error = result.exceptionOrNull() {
            errorMessage = error.message ?? "Failed to load listing"
        }

        isLoading = false
    }

    private func checkFavoriteStatus() async {
        let result = await favoritesRepository.isFavorite(listingId: listingId)
        if let isFav = result.getOrNull() {
            isFavorite = isFav.boolValue
        }
    }

    private func toggleFavorite() async {
        guard authManager.isAuthenticated else {
            // Prompt to sign in
            coordinator.navigate(to: .login)
            return
        }

        let newFavoriteState = !isFavorite

        // Optimistic update
        withAnimation(.spring(response: 0.3, dampingFraction: 0.6)) {
            isFavorite = newFavoriteState
        }

        // Persist change
        if newFavoriteState {
            let result = await favoritesRepository.addFavorite(listingId: listingId)
            if result.exceptionOrNull() != nil {
                // Revert on error
                isFavorite = false
            }
        } else {
            let result = await favoritesRepository.removeFavorite(listingId: listingId)
            if result.exceptionOrNull() != nil {
                // Revert on error
                isFavorite = true
            }
        }
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

private struct PhotoGalleryView: View {
    let photos: [String]
    @Environment(\.dismiss) private var dismiss

    @State private var currentIndex = 0

    var body: some View {
        ZStack(alignment: .topLeading) {
            Color.black.ignoresSafeArea() // fullscreen photo viewer intentionally black

            TabView(selection: $currentIndex) {
                ForEach(photos.indices, id: \.self) { index in
                    ZStack {
                        if let url = URL(string: photos[index]) {
                            AsyncImage(url: url) { phase in
                                switch phase {
                                case .success(let image):
                                    image
                                        .resizable()
                                        .aspectRatio(contentMode: .fit)
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
}
