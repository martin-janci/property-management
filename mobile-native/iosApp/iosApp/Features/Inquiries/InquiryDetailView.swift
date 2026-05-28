import SwiftUI
import shared

/// Full conversation thread for a single inquiry.
///
/// Shows the original user message and all realtor replies, with a
/// reply-compose area at the bottom so the user can add follow-up
/// messages to an open inquiry.
///
/// Endpoint: GET /api/v1/inquiries/{id}
/// Reply:    POST /api/v1/inquiries/{id}/replies
///
/// Epic 82 - Story 82.5: Inquiries and Account
struct InquiryDetailView: View {
    let inquiryId: String

    @Environment(NavigationCoordinator.self) private var coordinator
    @Environment(AuthManager.self) private var authManager

    @State private var inquiry: Inquiry?
    @State private var isLoading = true
    @State private var errorMessage: String?
    @State private var replyText = ""
    @State private var isSendingReply = false

    private var inquiryRepository: InquiryRepository {
        DependencyContainer.shared.makeAuthenticatedInquiryRepository(
            sessionToken: authManager.getSessionToken()
        )
    }

    var body: some View {
        Group {
            if isLoading {
                loadingView
            } else if let errorMessage {
                errorView(message: errorMessage)
            } else if let inquiry {
                conversationView(inquiry: inquiry)
            }
        }
        .navigationTitle(String(localized: "inquiry_detail_title"))
        .navigationBarTitleDisplayMode(.inline)
        .task {
            await loadInquiry()
        }
    }

    // MARK: - Subviews

    private var loadingView: some View {
        VStack(spacing: 16) {
            Spacer()
            ProgressView()
            Text(String(localized: "loading"))
                .foregroundStyle(.secondary)
            Spacer()
        }
    }

    private func errorView(message: String) -> some View {
        VStack(spacing: 24) {
            Spacer()
            Image(systemName: "exclamationmark.triangle")
                .font(.system(size: 48))
                .foregroundStyle(.secondary)
            Text(message)
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
                .padding(.horizontal, 32)
            Button(String(localized: "retry")) {
                Task { await loadInquiry() }
            }
            .buttonStyle(.bordered)
            Spacer()
        }
    }

    private func conversationView(inquiry: Inquiry) -> some View {
        VStack(spacing: 0) {
            ScrollView {
                LazyVStack(spacing: 12) {
                    // Listing context header
                    if let listing = inquiry.listing {
                        listingContextCard(listing: listing, status: inquiry.status)
                    }

                    // Original user message
                    MessageBubble(
                        message: inquiry.message,
                        timestamp: inquiry.createdAt,
                        isFromUser: true
                    )

                    // Realtor responses
                    ForEach(inquiry.responses, id: \.id) { response in
                        MessageBubble(
                            message: response.message,
                            timestamp: response.createdAt,
                            isFromUser: false,
                            senderName: response.realtor?.name
                        )
                    }

                    // Bottom spacer
                    Color.clear.frame(height: 8)
                }
                .padding(.horizontal)
                .padding(.top, 8)
            }

            // Reply composer — only for open inquiries
            if inquiry.status != .closed {
                replyComposer
            }
        }
    }

    private func listingContextCard(listing: ListingSummary, status: InquiryStatus) -> some View {
        HStack(spacing: 12) {
            // Thumbnail placeholder
            RoundedRectangle(cornerRadius: 8)
                .fill(Color(.systemGray5))
                .frame(width: 56, height: 56)
                .overlay {
                    Image(systemName: "photo")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }

            VStack(alignment: .leading, spacing: 4) {
                Text(listing.title)
                    .font(.subheadline)
                    .fontWeight(.semibold)
                    .lineLimit(1)

                Text(listing.address.city)
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }

            Spacer()

            // Status pill
            let swiftStatus = swiftInquiryStatus(from: status)
            Text(swiftStatus.displayName)
                .font(.caption2)
                .fontWeight(.medium)
                .padding(.horizontal, 8)
                .padding(.vertical, 4)
                .background(swiftStatus.backgroundColor)
                .foregroundStyle(swiftStatus.foregroundColor)
                .clipShape(Capsule())
        }
        .padding()
        .background(Color(.secondarySystemBackground))
        .clipShape(RoundedRectangle(cornerRadius: 12))
    }

    private var replyComposer: some View {
        VStack(spacing: 0) {
            Divider()
            HStack(alignment: .bottom, spacing: 12) {
                TextField(String(localized: "inquiry_reply_placeholder"), text: $replyText, axis: .vertical)
                    .lineLimit(1...5)
                    .textFieldStyle(.plain)
                    .padding(.horizontal, 12)
                    .padding(.vertical, 10)
                    .background(Color(.secondarySystemBackground))
                    .clipShape(RoundedRectangle(cornerRadius: 20))

                Button {
                    Task { await sendReply() }
                } label: {
                    if isSendingReply {
                        ProgressView()
                            .frame(width: 36, height: 36)
                    } else {
                        Image(systemName: "arrow.up.circle.fill")
                            .font(.system(size: 36))
                            .foregroundStyle(replyText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
                                ? Color(.systemGray4)
                                : Color.accentColor)
                    }
                }
                .disabled(replyText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty || isSendingReply)
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 8)
            .background(Color(.systemBackground))
        }
    }

    // MARK: - Data Loading

    private func loadInquiry() async {
        isLoading = true
        errorMessage = nil

        let result = await inquiryRepository.getInquiry(inquiryId: inquiryId)

        if let fetched = result.getOrNull() {
            inquiry = fetched
        } else if let error = result.exceptionOrNull() {
            errorMessage = error.message ?? String(localized: "error_generic")
        }

        isLoading = false
    }

    private func sendReply() async {
        let trimmed = replyText.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return }

        isSendingReply = true
        let result = await inquiryRepository.replyToInquiry(
            inquiryId: inquiryId,
            message: trimmed
        )
        isSendingReply = false

        if result.getOrNull() != nil {
            replyText = ""
            // Reload the full thread to show the new reply.
            await loadInquiry()
        } else if let error = result.exceptionOrNull() {
            errorMessage = error.message ?? String(localized: "error_generic")
        }
    }

    // MARK: - Helpers

    /// Converts the KMP InquiryStatus enum to the Swift InquiryStatus used by UI.
    ///
    /// Name mismatch is intentional: the KMP shared layer uses `.responded` (past-tense
    /// server terminology), while the Swift UI layer uses `.replied` (user-facing label
    /// that matches `InquiryStatus.displayName`). If a new KMP status is added, a
    /// compile-time exhaustive-switch warning on the `InquiryStatus` side in
    /// `InquiriesView.swift` will surface the gap before it reaches production.
    private func swiftInquiryStatus(from kmpStatus: InquiryStatus) -> InquiryStatusSwift {
        switch kmpStatus {
        case .pending:   return .pending
        case .responded: return .replied   // KMP name differs from Swift UI name — see doc comment above
        case .closed:    return .closed
        }
    }
}

// MARK: - Message Bubble

/// A single message bubble — right-aligned for user messages, left-aligned for replies.
///
/// Epic 82 - Story 82.5: Inquiries and Account
private struct MessageBubble: View {
    let message: String
    let timestamp: String
    let isFromUser: Bool
    var senderName: String? = nil

    private var bubbleColor: Color {
        isFromUser ? Color.accentColor : Color(.secondarySystemBackground)
    }

    private var textColor: Color {
        isFromUser ? .white : .primary
    }

    var body: some View {
        HStack {
            if isFromUser { Spacer(minLength: 48) }

            VStack(alignment: isFromUser ? .trailing : .leading, spacing: 4) {
                if let senderName, !isFromUser {
                    Text(senderName)
                        .font(.caption)
                        .fontWeight(.semibold)
                        .foregroundStyle(.secondary)
                }

                Text(message)
                    .font(.callout)
                    .foregroundStyle(textColor)
                    .padding(.horizontal, 14)
                    .padding(.vertical, 10)
                    .background(bubbleColor)
                    .clipShape(RoundedRectangle(cornerRadius: 18))
                    .cornerRadius(isFromUser ? 4 : 18, corners: isFromUser ? .bottomRight : .bottomLeft)

                Text(formattedDate(from: timestamp))
                    .font(.caption2)
                    .foregroundStyle(.tertiary)
            }

            if !isFromUser { Spacer(minLength: 48) }
        }
    }

    // ISO8601DateFormatter is expensive to allocate (bootstraps locale/calendar/timezone data).
    // Using a static instance avoids a fresh allocation for every bubble on every redraw.
    private static let iso8601Formatter: ISO8601DateFormatter = {
        let f = ISO8601DateFormatter()
        f.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
        return f
    }()

    private func formattedDate(from iso8601: String) -> String {
        guard let date = Self.iso8601Formatter.date(from: iso8601) else { return iso8601 }
        let formatter = RelativeDateTimeFormatter()
        formatter.unitsStyle = .abbreviated
        return formatter.localizedString(for: date, relativeTo: Date())
    }
}

// MARK: - RoundedCorner helper

private extension View {
    func cornerRadius(_ radius: CGFloat, corners: UIRectCorner) -> some View {
        clipShape(RoundedCorner(radius: radius, corners: corners))
    }
}

private struct RoundedCorner: Shape {
    var radius: CGFloat
    var corners: UIRectCorner

    func path(in rect: CGRect) -> Path {
        let path = UIBezierPath(
            roundedRect: rect,
            byRoundingCorners: corners,
            cornerRadii: CGSize(width: radius, height: radius)
        )
        return Path(path.cgPath)
    }
}

// MARK: - Localization Keys

extension String {
    static let inquiryDetailTitle = "inquiry_detail_title"
    static let inquiryReplyPlaceholder = "inquiry_reply_placeholder"
}

// MARK: - Preview

#Preview {
    NavigationStack {
        InquiryDetailView(inquiryId: "inq-preview")
    }
    .environment(NavigationCoordinator())
    .environment(AuthManager())
}
