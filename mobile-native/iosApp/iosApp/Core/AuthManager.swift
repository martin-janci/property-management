import Foundation
import Observation
import shared

/// User information from authentication.
///
/// Epic 82 - Story 82.5: Inquiries and Account
struct User: Identifiable, Equatable {
    let id: String
    let email: String
    let name: String
    let avatarUrl: String?

    /// Initialize from KMP SsoUserInfo.
    init(from ssoUser: SsoUserInfo) {
        self.id = ssoUser.userId
        self.email = ssoUser.email
        self.name = ssoUser.name
        self.avatarUrl = ssoUser.avatarUrl
    }

    /// Initialize with explicit values.
    init(id: String, email: String, name: String, avatarUrl: String? = nil) {
        self.id = id
        self.email = email
        self.name = name
        self.avatarUrl = avatarUrl
    }
}

/// Authentication state manager for Reality Portal iOS app.
///
/// Epic 82 - Story 82.5: Inquiries and Account
/// Integrates with KMP SsoService for SSO authentication and uses
/// `KeychainService` for secure token storage (no raw Security framework calls).
@Observable
final class AuthManager {
    // MARK: - Published Properties

    /// Current authenticated user, nil if not authenticated.
    private(set) var currentUser: User?

    /// Whether authentication is currently loading.
    private(set) var isLoading: Bool = false

    /// Last authentication error message.
    private(set) var errorMessage: String?

    // MARK: - Computed Properties

    /// Whether the user is currently authenticated.
    var isAuthenticated: Bool {
        accessToken != nil && currentUser != nil
    }

    // MARK: - Private Properties

    private var accessToken: String?
    private var refreshToken: String?
    private let configuration = Configuration.shared
    private let ssoService = SsoService()

    /// Shared Keychain service — scoped to the app's bundle identifier.
    private let keychain = KeychainService(service: Configuration.shared.keychainService)

    // MARK: - Initialization

    init() {
        // Session will be restored when restoreSession() is called
    }

    // MARK: - Public Methods

    /// Login with email and password.
    /// - Parameters:
    ///   - email: User email address.
    ///   - password: User password.
    /// - Note: Reality Portal uses SSO from Property Management app. Direct login is not supported.
    func login(email: String, password: String) async throws {
        isLoading = true
        errorMessage = nil

        defer { isLoading = false }

        // Reality Portal uses SSO from Property Management app
        // Direct email/password login is not supported
        throw AuthError.ssoRequired
    }

    /// Login via SSO token from Property Management app.
    /// - Parameter token: SSO token received via deep link.
    func loginWithSsoToken(_ token: String) async throws {
        isLoading = true
        errorMessage = nil

        defer { isLoading = false }

        do {
            let result = try await withCheckedThrowingContinuation { continuation in
                Task {
                    let validationResult = await ssoService.validateAndLogin(ssoToken: token)
                    if let session = validationResult.getOrNull() {
                        continuation.resume(returning: session)
                    } else if let error = validationResult.exceptionOrNull() {
                        continuation.resume(throwing: AuthError.ssoValidationFailed(error.message ?? "Unknown error"))
                    } else {
                        continuation.resume(throwing: AuthError.ssoValidationFailed("Unknown error"))
                    }
                }
            }

            // Store the session token
            let sessionToken = result.sessionToken
            storeTokens(accessToken: sessionToken, refreshToken: "")

            // Set current user from SSO response
            currentUser = User(from: result.user)
            accessToken = sessionToken
        } catch {
            // Set error message so UI can display it to the user
            errorMessage = error.localizedDescription
            throw error
        }
    }

    /// Logout the current user.
    func logout() {
        ssoService.logout()
        accessToken = nil
        refreshToken = nil
        currentUser = nil

        // Clear from secure storage
        clearStoredTokens()
    }

    /// Restore session from stored tokens.
    func restoreSession() {
        // Load tokens from Keychain using KeychainService
        guard let storedToken = keychain.loadOptional(forKey: KeychainService.Keys.accessToken) else {
            return
        }

        accessToken = storedToken
        refreshToken = keychain.loadOptional(forKey: KeychainService.Keys.refreshToken)

        // Validate and restore the session using KMP SsoService
        Task {
            await loadCurrentUser()
        }
    }

    /// Refresh the access token using the refresh token.
    func refreshAccessToken() async throws {
        guard accessToken != nil else {
            throw AuthError.noRefreshToken
        }

        isLoading = true
        defer { isLoading = false }

        let result = try await withCheckedThrowingContinuation { continuation in
            Task {
                let refreshResult = await ssoService.refreshSession()
                if let session = refreshResult.getOrNull() {
                    continuation.resume(returning: session)
                } else if let error = refreshResult.exceptionOrNull() {
                    continuation.resume(throwing: AuthError.tokenExpired)
                } else {
                    continuation.resume(throwing: AuthError.tokenExpired)
                }
            }
        }

        // Update user info from refreshed session
        currentUser = User(
            id: result.userId,
            email: result.email,
            name: result.name,
            avatarUrl: nil
        )
    }

    /// Get the current session token for API calls.
    func getSessionToken() -> String? {
        return ssoService.getSessionToken()
    }

    // MARK: - Private Methods

    private func loadCurrentUser() async {
        guard let token = accessToken else { return }

        isLoading = true
        defer { isLoading = false }

        // Try to restore the session with stored token
        let success = await ssoService.restoreSession(token: token)

        if success {
            // Get session info
            let result = await ssoService.getSession()
            if let session = result.getOrNull() {
                currentUser = User(
                    id: session.userId,
                    email: session.email,
                    name: session.name,
                    avatarUrl: nil
                )
            }
        } else {
            // Token is invalid, clear stored credentials
            clearStoredTokens()
            accessToken = nil
            refreshToken = nil
            currentUser = nil
        }
    }

    private func storeTokens(accessToken: String, refreshToken: String) {
        self.accessToken = accessToken
        self.refreshToken = refreshToken

        // Store in Keychain via KeychainService (eliminates raw Security framework calls)
        try? keychain.save(accessToken, forKey: KeychainService.Keys.accessToken)
        if !refreshToken.isEmpty {
            try? keychain.save(refreshToken, forKey: KeychainService.Keys.refreshToken)
        }
    }

    private func clearStoredTokens() {
        try? keychain.delete(forKey: KeychainService.Keys.accessToken)
        try? keychain.delete(forKey: KeychainService.Keys.refreshToken)
    }
}

// MARK: - Auth Errors

/// Authentication-related errors.
enum AuthError: LocalizedError {
    case invalidCredentials
    case networkError
    case noRefreshToken
    case tokenExpired
    case ssoRequired
    case ssoValidationFailed(String)
    case notImplemented

    var errorDescription: String? {
        switch self {
        case .invalidCredentials:
            return "Invalid email or password"
        case .networkError:
            return "Network error. Please check your connection."
        case .noRefreshToken:
            return "Session expired. Please login again."
        case .tokenExpired:
            return "Session expired. Please login again."
        case .ssoRequired:
            return "Please use the Property Management app to sign in."
        case .ssoValidationFailed(let message):
            return "SSO login failed: \(message)"
        case .notImplemented:
            return "This feature is not yet implemented."
        }
    }
}
