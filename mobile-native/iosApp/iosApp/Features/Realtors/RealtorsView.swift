import SwiftUI
import shared

/// Realtors directory for Reality Portal iOS (UC-49.1).
///
/// The Android side already has Realtor screens fed by a forthcoming
/// shared KMP `RealtorRepository`. The repository hasn't landed in
/// `mobile-native/shared` yet (only `listing`, `favorites`, `inquiry`
/// and `notifications` repos exist there today), so this view exists
/// for navigation parity with the placeholder Android screens but
/// renders a clear "coming soon" state.
///
/// Wire-up plan once `RealtorRepository` ships in shared:
///   1. Add `realtorRepository` to `DependencyContainer`
///   2. Replace the placeholder body with the loading/list pattern
///      used in `FavoritesView` / `InquiriesView`
///   3. Wire row taps to `coordinator.navigate(to: .realtorProfile(id:))`
struct RealtorsView: View {
    var body: some View {
        VStack(spacing: 12) {
            Image(systemName: "person.3")
                .font(.largeTitle)
                .foregroundStyle(.secondary)
            Text("Realtors directory")
                .font(.headline)
            Text(
                "Coming soon — once the shared module exposes a RealtorRepository, this screen will list realtors with their portfolio and contact options."
            )
            .font(.subheadline)
            .foregroundStyle(.secondary)
            .multilineTextAlignment(.center)
            .padding(.horizontal)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .navigationTitle("Realtors")
    }
}
