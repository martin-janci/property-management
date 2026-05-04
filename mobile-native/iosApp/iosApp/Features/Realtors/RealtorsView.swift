import SwiftUI
import shared

/// Realtors directory for Reality Portal iOS (UC-49.1).
///
/// reality-server doesn't expose a directory listing endpoint yet (only
/// `/realtors/profile` and `/realtors/{user_id}/profile`), so this view
/// derives the realtor list from the agency tree:
///
///   1. Load all agencies via `AgencyRepository.listAgencies()`.
///   2. For each agency, load its members.
///   3. Filter members to those whose role marks them as a realtor and
///      hydrate the union as `RealtorEntry` rows.
///
/// When reality-server eventually adds `GET /api/v1/realtors`, this
/// view can switch to a single call against `RealtorRepository` — the
/// row UI is shape-compatible.
struct RealtorsView: View {
    @Environment(NavigationCoordinator.self) private var coordinator
    @Environment(AuthManager.self) private var authManager

    @State private var entries: [RealtorEntry] = []
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
                ProgressView().frame(maxWidth: .infinity, maxHeight: .infinity)
            } else if let errorMessage {
                errorView(errorMessage)
            } else if entries.isEmpty {
                emptyView
            } else {
                realtorList
            }
        }
        .navigationTitle("Realtors")
        .refreshable { await loadRealtors() }
        .task { await loadRealtors() }
    }

    private func errorView(_ message: String) -> some View {
        VStack(spacing: 12) {
            Image(systemName: "exclamationmark.triangle")
                .font(.largeTitle)
                .foregroundStyle(.secondary)
            Text("Couldn't load realtors")
                .font(.headline)
            Text(message)
                .font(.subheadline)
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
                .padding(.horizontal)
            Button("Retry") { Task { await loadRealtors() } }
                .buttonStyle(.borderedProminent)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private var emptyView: some View {
        VStack(spacing: 12) {
            Image(systemName: "person.3")
                .font(.largeTitle)
                .foregroundStyle(.secondary)
            Text("No realtors found")
                .font(.headline)
            Text("Realtors will appear here as agencies onboard their teams.")
                .font(.subheadline)
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
                .padding(.horizontal)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private var realtorList: some View {
        List(entries) { entry in
            row(for: entry)
        }
        .listStyle(.insetGrouped)
    }

    @ViewBuilder
    private func row(for entry: RealtorEntry) -> some View {
        HStack(spacing: 12) {
            avatar(url: entry.photoUrl)
            VStack(alignment: .leading, spacing: 4) {
                Text(entry.displayName).font(.headline)
                Text(entry.agencyName)
                    .font(.subheadline)
                    .foregroundStyle(.secondary)
                if let email = entry.email, !email.isEmpty {
                    Text(email)
                        .font(.caption)
                        .foregroundStyle(.tertiary)
                }
            }
            Spacer()
        }
        .contentShape(Rectangle())
        .onTapGesture {
            coordinator.navigate(to: .searchResults(query: entry.displayName, filters: nil))
        }
    }

    @ViewBuilder
    private func avatar(url: String?) -> some View {
        if let url, let parsed = URL(string: url) {
            AsyncImage(url: parsed) { image in
                image.resizable().scaledToFill()
            } placeholder: {
                Image(systemName: "person.crop.circle")
                    .foregroundStyle(.secondary)
            }
            .frame(width: 44, height: 44)
            .clipShape(Circle())
        } else {
            Image(systemName: "person.crop.circle")
                .font(.title)
                .foregroundStyle(.secondary)
                .frame(width: 44, height: 44)
        }
    }

    private func loadRealtors() async {
        isLoading = true
        errorMessage = nil
        // Fan out over agencies to collect their members; merge by
        // user id so the same realtor doesn't show up twice if
        // reality-server later adds a multi-agency model.
        // KMP repos surface Kotlin `Result` to Swift, not async-throws,
        // so we unwrap with getOrNull / exceptionOrNull.
        let agenciesResult = await agencyRepository.listAgencies()
        guard let agenciesResp = agenciesResult.getOrNull() else {
            errorMessage = agenciesResult.exceptionOrNull()?.message ?? "Failed to load agencies"
            isLoading = false
            return
        }
        var collected: [String: RealtorEntry] = [:]
        for agency in agenciesResp.agencies {
            let membersResult = await agencyRepository.listMembers(agencyId: agency.id)
            let members = membersResult.getOrNull()?.members ?? []
            for member in members where isRealtor(role: member.role) {
                let entry = RealtorEntry(
                    id: "\(agency.id):\(member.userId)",
                    userId: member.userId,
                    displayName: member.name?.nonEmpty ?? member.email?.nonEmpty ?? "Realtor",
                    email: member.email,
                    photoUrl: member.photoUrl,
                    agencyName: agency.name
                )
                collected[member.userId] = entry
            }
        }
        entries = collected.values.sorted { $0.displayName < $1.displayName }
        isLoading = false
    }

    private func isRealtor(role: String) -> Bool {
        let normalised = role.lowercased()
        return normalised.contains("realtor") || normalised == "agent" || normalised == "member"
    }
}

private struct RealtorEntry: Identifiable {
    let id: String
    let userId: String
    let displayName: String
    let email: String?
    let photoUrl: String?
    let agencyName: String
}

private extension Optional where Wrapped == String {
    var nonEmpty: String? {
        guard let self else { return nil }
        return self.isEmpty ? nil : self
    }
}
