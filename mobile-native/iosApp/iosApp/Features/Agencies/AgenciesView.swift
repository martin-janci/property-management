import SwiftUI
import shared

/// Agencies directory for Reality Portal iOS (UC-51.1).
///
/// Reads from the shared `AgencyRepository` (now wired in
/// `DependencyContainer`). Tapping a row navigates to the listing
/// search filtered by that agency, since iOS doesn't yet have a
/// dedicated agency-detail screen.
struct AgenciesView: View {
    @Environment(NavigationCoordinator.self) private var coordinator
    @Environment(AuthManager.self) private var authManager

    @State private var agencies: [Agency] = []
    @State private var isLoading = true
    @State private var errorMessage: String?

    private var agencyRepository: AgencyRepository {
        DependencyContainer.shared.makeAuthenticatedAgencyRepository(
            sessionToken: authManager.getSessionToken()
        )
    }

    var body: some View {
        Group {
            if isLoading {
                loadingView
            } else if let errorMessage {
                errorView(errorMessage)
            } else if agencies.isEmpty {
                emptyView
            } else {
                agenciesList
            }
        }
        .navigationTitle("Agencies")
        .refreshable { await loadAgencies() }
        .task { await loadAgencies() }
    }

    private var loadingView: some View {
        ProgressView()
            .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private func errorView(_ message: String) -> some View {
        VStack(spacing: 12) {
            Image(systemName: "exclamationmark.triangle")
                .font(.largeTitle)
                .foregroundStyle(.secondary)
            Text("Couldn't load agencies")
                .font(.headline)
            Text(message)
                .font(.subheadline)
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
                .padding(.horizontal)
            Button("Retry") { Task { await loadAgencies() } }
                .buttonStyle(.borderedProminent)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private var emptyView: some View {
        VStack(spacing: 12) {
            Image(systemName: "building.2")
                .font(.largeTitle)
                .foregroundStyle(.secondary)
            Text("No agencies yet")
                .font(.headline)
            Text("Real estate agencies will appear here as they join the platform.")
                .font(.subheadline)
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
                .padding(.horizontal)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private var agenciesList: some View {
        List(agencies, id: \.id) { agency in
            row(for: agency)
        }
        .listStyle(.insetGrouped)
    }

    @ViewBuilder
    private func row(for agency: Agency) -> some View {
        HStack(spacing: 12) {
            agencyLogo(url: agency.logoUrl)
            VStack(alignment: .leading, spacing: 4) {
                Text(agency.name).font(.headline)
                if let city = agency.city, !city.isEmpty {
                    Text(city)
                        .font(.subheadline)
                        .foregroundStyle(.secondary)
                }
                if let website = agency.website, !website.isEmpty {
                    Text(website)
                        .font(.caption)
                        .foregroundStyle(.tint)
                        .lineLimit(1)
                }
            }
            Spacer()
            Image(systemName: "chevron.right").foregroundStyle(.tertiary)
        }
        .contentShape(Rectangle())
        .onTapGesture {
            // Drop into Search filtered by this agency's slug as the
            // free-text query — keeps the navigation graph minimal
            // until a dedicated agency-detail screen ships.
            coordinator.navigate(to: .searchResults(query: agency.name, filters: nil))
        }
    }

    @ViewBuilder
    private func agencyLogo(url: String?) -> some View {
        if let url, let parsed = URL(string: url) {
            AsyncImage(url: parsed) { image in
                image.resizable().scaledToFit()
            } placeholder: {
                Image(systemName: "building.2")
                    .foregroundStyle(.secondary)
            }
            .frame(width: 44, height: 44)
            .clipShape(RoundedRectangle(cornerRadius: 8))
        } else {
            Image(systemName: "building.2")
                .font(.title2)
                .foregroundStyle(.secondary)
                .frame(width: 44, height: 44)
        }
    }

    private func loadAgencies() async {
        isLoading = true
        errorMessage = nil
        let result = await agencyRepository.listAgencies()
        if let response = result.getOrNull() {
            agencies = response.agencies
        } else if let error = result.exceptionOrNull() {
            errorMessage = error.message ?? "Failed to load agencies"
        }
        isLoading = false
    }
}
