import SwiftUI
import shared

/// Agencies directory for Reality Portal iOS (UC-51.1).
///
/// Mirrors `RealtorsView` — the reality-server already exposes
/// `/api/v1/agencies`, but the shared KMP module doesn't yet wrap it
/// in an `AgencyRepository`. This view exists for navigation parity
/// and renders a clear "coming soon" state until the shared repo
/// lands.
///
/// Wire-up plan once `AgencyRepository` ships in shared:
///   1. Add `agencyRepository` to `DependencyContainer`
///   2. Replace the placeholder body with the loading/list/empty
///      pattern from `FavoritesView`
///   3. Wire row taps to `coordinator.navigate(to: .agencyDetail(id:))`
struct AgenciesView: View {
    var body: some View {
        VStack(spacing: 12) {
            Image(systemName: "building.2")
                .font(.largeTitle)
                .foregroundStyle(.secondary)
            Text("Agencies directory")
                .font(.headline)
            Text(
                "Coming soon — once the shared module exposes an AgencyRepository, this screen will list agencies, their realtors, and their listings."
            )
            .font(.subheadline)
            .foregroundStyle(.secondary)
            .multilineTextAlignment(.center)
            .padding(.horizontal)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .navigationTitle("Agencies")
    }
}
