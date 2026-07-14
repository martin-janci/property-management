# Story 82.3: Home and Search Screens

Status: done

## Story

As a **Reality Portal iOS user**,
I want to **browse and search property listings**,
So that **I can discover properties that match my needs**.

## Acceptance Criteria

1. **AC-1: Home Screen Layout**
   - Given I open the app
   - When the home screen loads
   - Then I see featured listings in a carousel
   - And recent/popular listings in a grid
   - And quick search categories

2. **AC-2: Search Functionality**
   - Given I am on the search tab
   - When I enter a search query
   - Then listings matching the query are shown
   - And results update as I type (debounced)

3. **AC-3: Search Filters**
   - Given I want to refine search results
   - When I open the filter panel
   - Then I can filter by price, location, type, bedrooms
   - And filters are applied to results immediately

4. **AC-4: Listing Grid**
   - Given search results are displayed
   - When viewing the grid
   - Then each listing shows image, price, title, location
   - And I can tap to view details
   - And infinite scroll loads more results

5. **AC-5: Location-based Search**
   - Given I grant location permission
   - When I enable "Near Me" search
   - Then listings are sorted by distance
   - And a map view option is available

## Tasks / Subtasks

- [ ] Task 1: Create Home Screen (AC: 1)
  - [ ] 1.1 Create `/mobile-native/iosApp/iosApp/Features/Home/HomeView.swift`
  - [ ] 1.2 Create featured listings carousel component
  - [ ] 1.3 Create listing grid section
  - [ ] 1.4 Add quick category buttons
  - [ ] 1.5 Implement pull-to-refresh
  - [ ] 1.6 Create HomeViewModel using shared KMP code

- [ ] Task 2: Create Search Screen (AC: 2, 3)
  - [ ] 2.1 Create `/mobile-native/iosApp/iosApp/Features/Search/SearchView.swift`
  - [ ] 2.2 Implement search bar with debounced input
  - [ ] 2.3 Create SearchViewModel with KMP integration
  - [ ] 2.4 Show search suggestions
  - [ ] 2.5 Display recent searches

- [ ] Task 3: Create Filter Sheet (AC: 3)
  - [ ] 3.1 Create `/mobile-native/iosApp/iosApp/Features/Search/FilterSheet.swift`
  - [ ] 3.2 Add price range slider
  - [ ] 3.3 Add property type picker
  - [ ] 3.4 Add bedroom/bathroom counters
  - [ ] 3.5 Add location radius filter
  - [ ] 3.6 Implement apply/reset buttons

- [ ] Task 4: Create Listing Card Component (AC: 4)
  - [ ] 4.1 Create `/mobile-native/iosApp/iosApp/Features/Shared/ListingCard.swift`
  - [ ] 4.2 Display listing image with async loading
  - [ ] 4.3 Show price, title, location
  - [ ] 4.4 Add favorite button overlay
  - [ ] 4.5 Handle image loading states

- [ ] Task 5: Implement Infinite Scroll (AC: 4)
  - [ ] 5.1 Add pagination to search results
  - [ ] 5.2 Detect scroll to bottom
  - [ ] 5.3 Load next page automatically
  - [ ] 5.4 Show loading indicator at bottom

- [ ] Task 6: Add Location-based Features (AC: 5)
  - [ ] 6.1 Request location permission
  - [ ] 6.2 Get current location
  - [ ] 6.3 Add "Near Me" toggle
  - [ ] 6.4 Sort results by distance
  - [ ] 6.5 Create map view alternative

## Dev Notes

### Architecture Requirements
- Use KMP shared module for API calls
- ViewModels wrap KMP use cases
- Async image loading with Kingfisher
- Location services with CoreLocation

### Technical Specifications
- Search debounce: 300ms
- Pagination size: 20 items
- Image placeholder: Gradient or skeleton
- Grid columns: 2 (portrait), 3 (landscape)

### Home Screen Layout
```swift
struct HomeView: View {
    @StateObject var viewModel: HomeViewModel

    var body: some View {
        ScrollView {
            VStack(spacing: 24) {
                // Search bar (navigates to Search tab)
                SearchBarButton()

                // Featured carousel
                FeaturedCarousel(listings: viewModel.featured)

                // Categories
                CategoryGrid(categories: viewModel.categories)

                // Recent listings
                ListingSection(
                    title: "Recently Added",
                    listings: viewModel.recent
                )

                // Popular listings
                ListingSection(
                    title: "Popular in Your Area",
                    listings: viewModel.popular
                )
            }
        }
        .refreshable {
            await viewModel.refresh()
        }
    }
}
```

### Search Integration with KMP
```swift
class SearchViewModel: ObservableObject {
    private let searchUseCase: SearchListingsUseCase // From KMP

    @Published var query = ""
    @Published var results: [Listing] = []
    @Published var isLoading = false

    init() {
        // Debounced search
        $query
            .debounce(for: .milliseconds(300), scheduler: RunLoop.main)
            .sink { [weak self] query in
                Task { await self?.search(query) }
            }
            .store(in: &cancellables)
    }

    func search(_ query: String) async {
        isLoading = true
        do {
            let kotlinResults = try await searchUseCase.execute(query: query)
            results = kotlinResults.map { Listing(from: $0) }
        } catch {
            // Handle error
        }
        isLoading = false
    }
}
```

### Filter Model
```swift
struct SearchFilters: Equatable {
    var priceMin: Int?
    var priceMax: Int?
    var propertyTypes: Set<PropertyType> = []
    var bedroomsMin: Int?
    var bedroomsMax: Int?
    var bathroomsMin: Int?
    var radiusKm: Double?
    var location: CLLocationCoordinate2D?
}
```

### File List (to create)

**Create:**
- `/mobile-native/iosApp/iosApp/Features/Home/HomeView.swift`
- `/mobile-native/iosApp/iosApp/Features/Home/HomeViewModel.swift`
- `/mobile-native/iosApp/iosApp/Features/Home/Components/FeaturedCarousel.swift`
- `/mobile-native/iosApp/iosApp/Features/Home/Components/CategoryGrid.swift`
- `/mobile-native/iosApp/iosApp/Features/Search/SearchView.swift`
- `/mobile-native/iosApp/iosApp/Features/Search/SearchViewModel.swift`
- `/mobile-native/iosApp/iosApp/Features/Search/FilterSheet.swift`
- `/mobile-native/iosApp/iosApp/Features/Shared/ListingCard.swift`
- `/mobile-native/iosApp/iosApp/Features/Shared/ListingGrid.swift`
- `/mobile-native/iosApp/iosApp/Features/Shared/SearchBar.swift`

### Image Loading
```swift
import Kingfisher

struct ListingImage: View {
    let url: String

    var body: some View {
        KFImage(URL(string: url))
            .placeholder {
                Rectangle()
                    .fill(Color.gray.opacity(0.2))
                    .overlay(ProgressView())
            }
            .resizable()
            .aspectRatio(16/9, contentMode: .fill)
    }
}
```

### Dependencies
- Story 82.1 (SwiftUI Project Setup) - Project structure
- Story 82.2 (Navigation and Routing) - Navigation integration

### References
- [Reference: mobile-native/androidApp/.../screens/HomeScreen.kt]
- [Reference: mobile-native/androidApp/.../screens/SearchScreen.kt]
