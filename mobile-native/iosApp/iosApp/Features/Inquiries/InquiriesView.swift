import SwiftUI
import shared

/// Inquiries screen for Reality Portal iOS app.
///
/// Displays user's sent inquiries and conversations.
///
/// Epic 82 - Story 82.5: Inquiries and Account
struct InquiriesView: View {
    @Environment(NavigationCoordinator.self) private var coordinator
    @Environment(AuthManager.self) private var authManager

    @State private var inquiries: [InquiryPreview] = []
    @State private var isLoading = true
    @State private var errorMessage: String?

    private var inquiryRepository: InquiryRepository {
        DependencyContainer.shared.makeAuthenticatedInquiryRepository(
            sessionToken: authManager.getSessionToken()
        )
    }

    var body: some View {
        Group {
            if !authManager.isAuthenticated {
                notAuthenticatedView
            } else if isLoading {
                loadingView
            } else if inquiries.isEmpty {
                emptyView
            } else {
                inquiriesListView
            }
        }
        .navigationTitle("Inquiries")
        .refreshable {
            await loadInquiries()
        }
        .task {
            if authManager.isAuthenticated {
                await loadInquiries()
            }
        }
    }

    // MARK: - Subviews

    private var notAuthenticatedView: some View {
        VStack(spacing: 24) {
            Spacer()

            Image(systemName: "envelope.fill")
                .font(.system(size: 64))
                .foregroundStyle(.secondary)

            Text("Sign in to see your inquiries")
                .font(.headline)

            Text("Track your property inquiries and conversations with agents.")
                .font(.subheadline)
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
                .padding(.horizontal, 32)

            Button {
                coordinator.navigate(to: .login)
            } label: {
                Text("Sign In")
                    .frame(maxWidth: .infinity)
                    .padding()
                    .background(Color.accentColor)
                    .foregroundStyle(.white)
                    .clipShape(RoundedRectangle(cornerRadius: 12))
            }
            .padding(.horizontal, 32)

            Spacer()
        }
    }

    private var loadingView: some View {
        VStack(spacing: 16) {
            Spacer()
            ProgressView()
            Text("Loading inquiries...")
                .foregroundStyle(.secondary)
            Spacer()
        }
    }

    private var emptyView: some View {
        VStack(spacing: 24) {
            Spacer()

            Image(systemName: "envelope.open")
                .font(.system(size: 64))
                .foregroundStyle(.secondary)

            Text("No inquiries yet")
                .font(.headline)

            Text("When you send an inquiry about a property, it will appear here.")
                .font(.subheadline)
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
                .padding(.horizontal, 32)

            Button {
                coordinator.selectedTab = .search
            } label: {
                Text("Browse Listings")
                    .frame(maxWidth: .infinity)
                    .padding()
                    .background(Color.accentColor)
                    .foregroundStyle(.white)
                    .clipShape(RoundedRectangle(cornerRadius: 12))
            }
            .padding(.horizontal, 32)

            Spacer()
        }
    }

    private var inquiriesListView: some View {
        ScrollView {
            LazyVStack(spacing: 12) {
                ForEach(inquiries) { inquiry in
                    InquiryCard(inquiry: inquiry) {
                        coordinator.navigate(to: .inquiryDetail(id: inquiry.id))
                    }
                }
            }
            .padding()
        }
    }

    // MARK: - Data Loading

    private func loadInquiries() async {
        isLoading = true
        errorMessage = nil

        // Load inquiries from KMP shared module
        let result = await inquiryRepository.getInquiries(page: 1, pageSize: 50, status: nil)

        if let response = result.getOrNull() {
            inquiries = response.inquiries.map { KMPBridge.toInquiryPreview($0) }
        } else if let error = result.exceptionOrNull() {
            errorMessage = error.message ?? "Failed to load inquiries"
            #if DEBUG
            print("Inquiries error: \(errorMessage ?? "Unknown")")
            #endif
        }

        isLoading = false
    }
}

// MARK: - Supporting Views

private struct InquiryCard: View {
    let inquiry: InquiryPreview
    let onTap: () -> Void

    var body: some View {
        Button(action: onTap) {
            HStack(spacing: 12) {
                // Listing image placeholder
                RoundedRectangle(cornerRadius: 8)
                    .fill(Color(.systemGray5))
                    .frame(width: 60, height: 60)
                    .overlay {
                        Image(systemName: "photo")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }

                VStack(alignment: .leading, spacing: 4) {
                    HStack {
                        Text(inquiry.listingTitle)
                            .font(.subheadline)
                            .fontWeight(.semibold)
                            .lineLimit(1)
                            .foregroundStyle(.primary)

                        Spacer()

                        statusBadge
                    }

                    Text(inquiry.lastMessage)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                        .lineLimit(2)

                    Text(inquiry.formattedDate)
                        .font(.caption2)
                        .foregroundStyle(.tertiary)
                }

                if inquiry.hasUnread {
                    Circle()
                        .fill(Color.accentColor)
                        .frame(width: 10, height: 10)
                }
            }
            .padding()
            .background(Color(.systemBackground))
            .clipShape(RoundedRectangle(cornerRadius: 12))
            .shadow(color: .black.opacity(0.05), radius: 4, y: 2)
        }
        .buttonStyle(.plain)
    }

    private var statusBadge: some View {
        Text(inquiry.status.displayName)
            .font(.caption2)
            .fontWeight(.medium)
            .padding(.horizontal, 6)
            .padding(.vertical, 2)
            .background(inquiry.status.backgroundColor)
            .foregroundStyle(inquiry.status.foregroundColor)
            .clipShape(Capsule())
    }
}

// MARK: - Preview Data

struct InquiryPreview: Identifiable {
    let id: String
    let listingId: String
    let listingTitle: String
    let lastMessage: String
    let status: InquiryStatus
    let date: Date
    let hasUnread: Bool

    var formattedDate: String {
        let formatter = RelativeDateTimeFormatter()
        formatter.unitsStyle = .abbreviated
        return formatter.localizedString(for: date, relativeTo: Date())
    }

    #if DEBUG
    /// Sample data for SwiftUI previews only - excluded from production builds (Story 85.1)
    static let samples: [InquiryPreview] = [
        InquiryPreview(
            id: "1",
            listingId: "1",
            listingTitle: "Modern Apartment in City Center",
            lastMessage: "Thank you for your interest. The property is still available for viewing.",
            status: .replied,
            date: Date().addingTimeInterval(-3600),
            hasUnread: true
        ),
        InquiryPreview(
            id: "2",
            listingId: "2",
            listingTitle: "Family House with Garden",
            lastMessage: "Hi, I would like to schedule a viewing for this property.",
            status: .pending,
            date: Date().addingTimeInterval(-86400),
            hasUnread: false
        ),
        InquiryPreview(
            id: "3",
            listingId: "3",
            listingTitle: "Cozy Studio Near Park",
            lastMessage: "The property has been sold. Thank you for your interest.",
            status: .closed,
            date: Date().addingTimeInterval(-172800),
            hasUnread: false
        ),
    ]
    #endif
}

enum InquiryStatus: String {
    case pending
    case replied
    case closed

    var displayName: String {
        switch self {
        case .pending: return "Pending"
        case .replied: return "Replied"
        case .closed: return "Closed"
        }
    }

    var backgroundColor: Color {
        switch self {
        case .pending: return InquiryStatusColors.pending.bg
        case .replied: return InquiryStatusColors.replied.bg
        case .closed:  return InquiryStatusColors.closed.bg
        }
    }

    var foregroundColor: Color {
        switch self {
        case .pending: return InquiryStatusColors.pending.ink
        case .replied: return InquiryStatusColors.replied.ink
        case .closed:  return InquiryStatusColors.closed.ink
        }
    }
}

// MARK: - Preview

#Preview {
    NavigationStack {
        InquiriesView()
    }
    .environment(NavigationCoordinator())
    .environment(AuthManager())
}
