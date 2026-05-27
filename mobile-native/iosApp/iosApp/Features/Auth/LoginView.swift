import SwiftUI

/// Login screen for Reality Portal iOS app.
///
/// Provides SSO login via Property Management app and email/password login.
///
/// Epic 82 - Story 82.5: Inquiries and Account
struct LoginView: View {
    @Environment(NavigationCoordinator.self) private var coordinator
    @Environment(AuthManager.self) private var authManager
    @Environment(\.dismiss) private var dismiss

    @State private var email = ""
    @State private var password = ""
    @State private var showPassword = false
    @State private var isLoading = false
    @State private var errorMessage: String?

    var body: some View {
        NavigationStack {
            ScrollView {
                VStack(spacing: 32) {
                    // Header
                    headerSection

                    // SSO Login
                    ssoLoginSection

                    // Divider
                    dividerSection

                    // Email/Password Login
                    emailLoginSection

                    // Error message
                    if let error = errorMessage {
                        errorBanner(error)
                    }

                    // Register link
                    registerSection
                }
                .padding()
            }
            .navigationTitle(String(localized: "sign_in"))
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button(String(localized: "cancel")) {
                        dismiss()
                    }
                }
            }
        }
    }

    // MARK: - Subviews

    private var headerSection: some View {
        VStack(spacing: 16) {
            Image(systemName: "house.fill")
                .font(.system(size: 48))
                .foregroundStyle(Color.accentColor)

            Text(String(localized: "welcome_to_reality_portal"))
                .font(.title2)
                .fontWeight(.bold)

            Text(String(localized: "sign_in_description"))
                .font(.subheadline)
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
        }
        .padding(.top, 16)
    }

    private var ssoLoginSection: some View {
        VStack(spacing: 16) {
            Button {
                loginWithSso()
            } label: {
                HStack(spacing: 12) {
                    Image(systemName: "building.2.fill")
                    Text(String(localized: "sign_in_with_pm"))
                }
                .frame(maxWidth: .infinity)
                .padding()
                .background(Color.accentColor)
                .foregroundStyle(.white)
                .clipShape(RoundedRectangle(cornerRadius: 12))
            }
            .disabled(isLoading)

            Text(String(localized: "sso_description"))
                .font(.caption)
                .foregroundStyle(.secondary)
        }
    }

    private var dividerSection: some View {
        HStack {
            Rectangle()
                .fill(Color(.separator))
                .frame(height: 1)

            Text(String(localized: "or"))
                .font(.subheadline)
                .foregroundStyle(.secondary)
                .padding(.horizontal, 16)

            Rectangle()
                .fill(Color(.separator))
                .frame(height: 1)
        }
    }

    private var emailLoginSection: some View {
        VStack(spacing: 16) {
            // Email field
            VStack(alignment: .leading, spacing: 8) {
                Text(String(localized: "email"))
                    .font(.subheadline)
                    .fontWeight(.medium)

                TextField(String(localized: "enter_email"), text: $email)
                    .textFieldStyle(.plain)
                    .textContentType(.emailAddress)
                    .keyboardType(.emailAddress)
                    .autocapitalization(.none)
                    .autocorrectionDisabled()
                    .padding()
                    .background(Color(.systemGray6))
                    .clipShape(RoundedRectangle(cornerRadius: 12))
            }

            // Password field
            VStack(alignment: .leading, spacing: 8) {
                Text(String(localized: "password"))
                    .font(.subheadline)
                    .fontWeight(.medium)

                HStack {
                    Group {
                        if showPassword {
                            TextField(String(localized: "enter_password"), text: $password)
                        } else {
                            SecureField(String(localized: "enter_password"), text: $password)
                        }
                    }
                    .textFieldStyle(.plain)
                    .textContentType(.password)
                    .autocapitalization(.none)
                    .autocorrectionDisabled()

                    Button {
                        showPassword.toggle()
                    } label: {
                        Image(systemName: showPassword ? "eye.slash.fill" : "eye.fill")
                            .foregroundStyle(.secondary)
                    }
                }
                .padding()
                .background(Color(.systemGray6))
                .clipShape(RoundedRectangle(cornerRadius: 12))
            }

            // Forgot password
            HStack {
                Spacer()
                Button(String(localized: "forgot_password")) {
                    // Open forgot password flow
                }
                .font(.subheadline)
            }

            // Login button
            Button {
                Task {
                    await loginWithEmail()
                }
            } label: {
                HStack(spacing: 8) {
                    if isLoading {
                        ProgressView()
                            .tint(.white)
                    }
                    Text(String(localized: "sign_in"))
                }
                .frame(maxWidth: .infinity)
                .padding()
                .background(canLogin ? Color.accentColor : Color(.systemGray4))
                .foregroundStyle(.white)
                .clipShape(RoundedRectangle(cornerRadius: 12))
            }
            .disabled(!canLogin || isLoading)
        }
    }

    private func errorBanner(_ message: String) -> some View {
        HStack(spacing: 8) {
            Image(systemName: "exclamationmark.triangle.fill")
                .foregroundStyle(Color.pptWarning)
            Text(message)
                .font(.subheadline)
            Spacer()
        }
        .padding()
        .background(Color.pptWarningLight)
        .clipShape(RoundedRectangle(cornerRadius: 12))
    }

    private var registerSection: some View {
        HStack(spacing: 4) {
            Text(String(localized: "no_account_prompt"))
                .foregroundStyle(.secondary)
            Button(String(localized: "create_account")) {
                coordinator.navigate(to: .register)
            }
        }
        .font(.subheadline)
    }

    // MARK: - Computed Properties

    private var canLogin: Bool {
        !email.isEmpty && !password.isEmpty && email.contains("@")
    }

    // MARK: - Actions

    private func loginWithSso() {
        let state = authManager.beginSsoFlow()
        let urlString = "propertymanagement://sso?callback=realityportal://sso&state=\(state)"
        if let url = URL(string: urlString) {
            UIApplication.shared.open(url)
        }
    }

    private func loginWithEmail() async {
        isLoading = true
        errorMessage = nil

        do {
            try await authManager.login(email: email, password: password)
            dismiss()

            // Navigate to pending destination if any
            if let pendingDestination = coordinator.pendingDestination {
                coordinator.navigate(to: pendingDestination)
                coordinator.pendingDestination = nil
            }
        } catch {
            errorMessage = error.localizedDescription
        }

        isLoading = false
    }
}

// MARK: - Preview

#Preview {
    LoginView()
        .environment(NavigationCoordinator())
        .environment(AuthManager())
}
