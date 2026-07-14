# Story 82.4: Listing Detail and Favorites

Status: done

## Story

As a **Reality Portal iOS user**,
I want to **view detailed listing information and save favorites**,
So that **I can make informed decisions and track interesting properties**.

## Acceptance Criteria

1. **AC-1: Listing Detail View**
   - Given I tap on a listing
   - When the detail screen loads
   - Then I see full listing information (photos, description, features)
   - And agent/owner contact information is displayed
   - And I can scroll through all details

2. **AC-2: Photo Gallery**
   - Given I am viewing listing details
   - When I tap on the main photo
   - Then a full-screen gallery opens
   - And I can swipe between photos
   - And pinch-to-zoom is supported

3. **AC-3: Add/Remove Favorites**
   - Given I am viewing a listing
   - When I tap the heart icon
   - Then the listing is added to my favorites
   - And the heart icon fills in
   - And I can tap again to remove

4. **AC-4: Favorites List**
   - Given I am on the Favorites tab
   - When I view my favorites
   - Then all saved listings are displayed
   - And I can remove listings with swipe
   - And I can tap to view details

5. **AC-5: Share Listing**
   - Given I am viewing a listing
   - When I tap the share button
   - Then the iOS share sheet appears
   - And I can share via messages, email, or social apps

## Tasks / Subtasks

- [ ] Task 1: Create Listing Detail Screen (AC: 1)
  - [ ] 1.1 Create `/mobile-native/iosApp/iosApp/Features/Listing/ListingDetailView.swift`
  - [ ] 1.2 Create photo header with gallery preview
  - [ ] 1.3 Add price and key details section
  - [ ] 1.4 Add full description section
  - [ ] 1.5 Add features/amenities grid
  - [ ] 1.6 Add location map section
  - [ ] 1.7 Add agent contact card
  - [ ] 1.8 Create ListingDetailViewModel with KMP

- [ ] Task 2: Create Photo Gallery (AC: 2)
  - [ ] 2.1 Create `/mobile-native/iosApp/iosApp/Features/Listing/PhotoGalleryView.swift`
  - [ ] 2.2 Implement full-screen photo viewer
  - [ ] 2.3 Add swipe navigation between photos
  - [ ] 2.4 Add pinch-to-zoom gesture
  - [ ] 2.5 Add close button and photo counter

- [ ] Task 3: Implement Favorites Functionality (AC: 3)
  - [ ] 3.1 Create `/mobile-native/iosApp/iosApp/Features/Favorites/FavoritesViewModel.swift`
  - [ ] 3.2 Add favorite toggle mutation via KMP
  - [ ] 3.3 Sync favorites state across views
  - [ ] 3.4 Persist favorites locally for offline
  - [ ] 3.5 Add haptic feedback on toggle

- [ ] Task 4: Create Favorites Screen (AC: 4)
  - [ ] 4.1 Create `/mobile-native/iosApp/iosApp/Features/Favorites/FavoritesView.swift`
  - [ ] 4.2 Display favorites in grid layout
  - [ ] 4.3 Add swipe-to-delete gesture
  - [ ] 4.4 Add empty state for no favorites
  - [ ] 4.5 Implement pull-to-refresh

- [ ] Task 5: Implement Share Feature (AC: 5)
  - [ ] 5.1 Create share URL generator
  - [ ] 5.2 Add share button to listing detail
  - [ ] 5.3 Integrate with iOS ShareLink
  - [ ] 5.4 Include listing image in share preview

## Dev Notes

### Architecture Requirements
- ViewModels integrate with KMP use cases
- Local favorites cache for offline access
- Optimistic UI updates for favorite toggle
- Shared components for listing display

### Technical Specifications
- Photo gallery: TabView with page style
- Zoom: 1x to 4x scale
- Share URL format: `https://reality.example.com/listing/{id}`
- Local storage: UserDefaults for favorites cache

### Listing Detail Layout
```swift
struct ListingDetailView: View {
    @StateObject var viewModel: ListingDetailViewModel
    @Environment(\.dismiss) var dismiss

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 0) {
                // Photo gallery header
                PhotoHeader(
                    photos: viewModel.listing.photos,
                    onTap: { viewModel.showGallery = true }
                )

                VStack(alignment: .leading, spacing: 20) {
                    // Price and title
                    PriceSection(listing: viewModel.listing)

                    // Key features
                    FeaturesRow(listing: viewModel.listing)

                    Divider()

                    // Description
                    DescriptionSection(text: viewModel.listing.description)

                    Divider()

                    // Amenities
                    AmenitiesGrid(amenities: viewModel.listing.amenities)

                    Divider()

                    // Location map
                    LocationSection(coordinate: viewModel.listing.coordinate)

                    Divider()

                    // Agent contact
                    AgentCard(agent: viewModel.listing.agent)
                }
                .padding()
            }
        }
        .toolbar {
            ToolbarItem(placement: .navigationBarTrailing) {
                HStack {
                    FavoriteButton(
                        isFavorite: viewModel.isFavorite,
                        action: viewModel.toggleFavorite
                    )
                    ShareLink(item: viewModel.shareURL)
                }
            }
        }
        .fullScreenCover(isPresented: $viewModel.showGallery) {
            PhotoGalleryView(photos: viewModel.listing.photos)
        }
    }
}
```

### Photo Gallery Implementation
```swift
struct PhotoGalleryView: View {
    let photos: [String]
    @State private var currentIndex = 0
    @State private var scale: CGFloat = 1.0
    @Environment(\.dismiss) var dismiss

    var body: some View {
        ZStack(alignment: .topLeading) {
            Color.black.ignoresSafeArea()

            TabView(selection: $currentIndex) {
                ForEach(photos.indices, id: \.self) { index in
                    ZoomableImage(url: photos[index])
                        .tag(index)
                }
            }
            .tabViewStyle(.page(indexDisplayMode: .never))

            // Close button
            Button(action: { dismiss() }) {
                Image(systemName: "xmark.circle.fill")
                    .font(.title)
                    .foregroundColor(.white)
            }
            .padding()

            // Photo counter
            Text("\(currentIndex + 1) / \(photos.count)")
                .foregroundColor(.white)
                .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .bottom)
                .padding()
        }
    }
}
```

### Favorites Integration
```swift
class FavoritesViewModel: ObservableObject {
    private let favoritesUseCase: FavoritesUseCase // From KMP

    @Published var favorites: [Listing] = []
    @Published var isLoading = false

    func loadFavorites() async {
        isLoading = true
        do {
            let kotlinFavorites = try await favoritesUseCase.getFavorites()
            favorites = kotlinFavorites.map { Listing(from: $0) }
        } catch {
            // Handle error
        }
        isLoading = false
    }

    func toggleFavorite(listingId: String) async {
        // Optimistic update
        if let index = favorites.firstIndex(where: { $0.id == listingId }) {
            favorites.remove(at: index)
        }

        do {
            try await favoritesUseCase.toggleFavorite(id: listingId)
        } catch {
            // Revert on error
            await loadFavorites()
        }
    }
}
```

### File List (to create)

**Create:**
- `/mobile-native/iosApp/iosApp/Features/Listing/ListingDetailView.swift`
- `/mobile-native/iosApp/iosApp/Features/Listing/ListingDetailViewModel.swift`
- `/mobile-native/iosApp/iosApp/Features/Listing/PhotoGalleryView.swift`
- `/mobile-native/iosApp/iosApp/Features/Listing/Components/PhotoHeader.swift`
- `/mobile-native/iosApp/iosApp/Features/Listing/Components/PriceSection.swift`
- `/mobile-native/iosApp/iosApp/Features/Listing/Components/FeaturesRow.swift`
- `/mobile-native/iosApp/iosApp/Features/Listing/Components/AmenitiesGrid.swift`
- `/mobile-native/iosApp/iosApp/Features/Listing/Components/AgentCard.swift`
- `/mobile-native/iosApp/iosApp/Features/Favorites/FavoritesView.swift`
- `/mobile-native/iosApp/iosApp/Features/Favorites/FavoritesViewModel.swift`
- `/mobile-native/iosApp/iosApp/Features/Shared/FavoriteButton.swift`
- `/mobile-native/iosApp/iosApp/Features/Shared/ZoomableImage.swift`

### Dependencies
- Story 82.2 (Navigation and Routing) - Navigation to detail
- Story 82.3 (Home and Search) - Listing card component

### References
- [Reference: mobile-native/androidApp/.../screens/ListingDetailScreen.kt]
- [Reference: mobile-native/androidApp/.../screens/FavoritesScreen.kt]
