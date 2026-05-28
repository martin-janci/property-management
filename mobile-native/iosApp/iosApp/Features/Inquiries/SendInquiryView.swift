import SwiftUI
import shared

/// Compose and send a new property inquiry.
///
/// Presents a form with the user's name, e-mail, optional phone number, and a
/// free-text message.  Pre-fills name and e-mail from `AuthManager.currentUser`
/// when the user is signed in.
///
/// Endpoint: POST /api/v1/inquiries
///
/// Epic 82 - Story 82.5: Inquiries and Account
struct SendInquiryView: View {
    let listingId: String

    @Environment(NavigationCoordinator.self) private var coordinator
    @Environment(AuthManager.self) private var authManager
    @Environment(\.dismiss) private var dismiss

    @State private var name = ""
    @State private var email = ""
    @State private var phone = ""
    @State private var message = ""
    @State private var isSending = false
    @State private var errorMessage: String?
    @State private var didSend = false

    private var inquiryRepository: InquiryRepository {
        DependencyContainer.shared.makeAuthenticatedInquiryRepository(
            sessionToken: authManager.getSessionToken()
        )
    }

    private var isFormValid: Bool {
        !name.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty &&
        !email.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty &&
        email.contains("@") &&
        !message.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
    }

    var body: some View {
        NavigationStack {
            Form {
                // Contact details section
                Section(String(localized: "section_contact_details")) {
                    TextField(String(localized: "label_name_required"), text: $name)
                        .textContentType(.name)
                        .autocorrectionDisabled()

                    TextField(String(localized: "label_email_required"), text: $email)
                        .textContentType(.emailAddress)
                        .keyboardType(.emailAddress)
                        .textInputAutocapitalization(.never)
                        .autocorrectionDisabled()

                    TextField(String(localized: "label_phone_optional"), text: $phone)
                        .textContentType(.telephoneNumber)
                        .keyboardType(.phonePad)
                }

                // Message section
                Section(String(localized: "section_your_message")) {
                    TextField(String(localized: "inquiry_message_placeholder"), text: $message, axis: .vertical)
                        .lineLimit(4...8)
                        .frame(minHeight: 80, alignment: .topLeading)
                }

                // Error feedback
                if let errorMessage {
                    Section {
                        Label(errorMessage, systemImage: "exclamationmark.circle.fill")
                            .foregroundStyle(.red)
                            .font(.callout)
                    }
                }

                // Submit button
                Section {
                    Button {
                        Task { await send() }
                    } label: {
                        HStack {
                            Spacer()
                            if isSending {
                                ProgressView()
                            } else {
                                Text(String(localized: "send_inquiry"))
                                    .fontWeight(.semibold)
                            }
                            Spacer()
                        }
                    }
                    .disabled(!isFormValid || isSending)
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
            }
            .alert(String(localized: "inquiry_sent"), isPresented: $didSend) {
                Button(String(localized: "ok")) {
                    dismiss()
                    coordinator.selectedTab = .inquiries
                }
            } message: {
                Text(String(localized: "inquiry_sent_confirmation"))
            }
            .onAppear {
                prefillFromAuthManager()
            }
        }
    }

    // MARK: - Actions

    private func prefillFromAuthManager() {
        guard let user = authManager.currentUser else { return }
        name = user.name
        email = user.email
    }

    private func send() async {
        let trimmedMessage = message.trimmingCharacters(in: .whitespacesAndNewlines)
        let trimmedName    = name.trimmingCharacters(in: .whitespacesAndNewlines)
        let trimmedEmail   = email.trimmingCharacters(in: .whitespacesAndNewlines)
        let trimmedPhone   = phone.trimmingCharacters(in: .whitespacesAndNewlines)

        guard !trimmedMessage.isEmpty, !trimmedName.isEmpty, !trimmedEmail.isEmpty else { return }

        isSending = true
        errorMessage = nil

        let request = CreateInquiryRequest(
            listingId: listingId,
            message: trimmedMessage,
            name: trimmedName,
            email: trimmedEmail,
            phone: trimmedPhone.isEmpty ? nil : trimmedPhone
        )

        let result = await inquiryRepository.createInquiry(request: request)

        if result.getOrNull() != nil {
            didSend = true
        } else if let error = result.exceptionOrNull() {
            errorMessage = error.message ?? String(localized: "error_generic")
        }

        isSending = false
    }
}

// MARK: - Localisation Keys

// These keys are already present in Localizable.strings; listed here for quick
// discovery by grepping this file.
//  "section_contact_details"        – Contact Details section header
//  "inquiry_message_placeholder"    – Message field placeholder
//  "inquiry_sent_confirmation"      – Body of the success alert

// MARK: - Preview

#Preview {
    SendInquiryView(listingId: "lst-preview")
        .environment(NavigationCoordinator())
        .environment(AuthManager())
}
