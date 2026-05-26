import Foundation
import Security

/// Errors that can occur during Keychain operations.
///
/// Epic 82 - Story 82.5: Inquiries and Account
enum KeychainError: LocalizedError {
    case encodingFailed
    case saveFailed(OSStatus)
    case loadFailed(OSStatus)
    case deleteFailed(OSStatus)
    case itemNotFound
    case unexpectedData

    var errorDescription: String? {
        switch self {
        case .encodingFailed:
            return "Failed to encode data for Keychain storage."
        case .saveFailed(let status):
            return "Keychain save failed with status: \(status)."
        case .loadFailed(let status):
            return "Keychain load failed with status: \(status)."
        case .deleteFailed(let status):
            return "Keychain delete failed with status: \(status)."
        case .itemNotFound:
            return "Item not found in Keychain."
        case .unexpectedData:
            return "Unexpected data format in Keychain."
        }
    }
}

/// Secure token storage backed by the iOS Keychain.
///
/// All tokens (auth tokens, push device tokens) are stored with
/// `kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly` so they survive
/// app restarts but cannot be restored on a new device from an iCloud
/// backup — important for session-bound tokens.
///
/// Usage:
/// ```swift
/// let keychain = KeychainService(service: "three.two.bit.ppt.reality")
///
/// // Store
/// try keychain.save("my-token", forKey: "access_token")
///
/// // Load
/// let token = try keychain.load(forKey: "access_token")
///
/// // Delete
/// try keychain.delete(forKey: "access_token")
///
/// // Check existence
/// let exists = keychain.contains(key: "access_token")
/// ```
///
/// Epic 82 - Story 82.5: Inquiries and Account
final class KeychainService {
    // MARK: - Properties

    /// The service identifier used to scope all Keychain items for this app.
    let service: String

    // MARK: - Initialization

    /// Creates a new `KeychainService` scoped to the given service identifier.
    ///
    /// - Parameter service: The Keychain service name (typically the bundle ID).
    init(service: String) {
        self.service = service
    }

    // MARK: - Public API

    /// Saves a string value to the Keychain under the given key.
    ///
    /// If a value already exists for the key it is replaced atomically.
    ///
    /// - Parameters:
    ///   - value: The string to store.
    ///   - key: The account key to store it under.
    /// - Throws: `KeychainError` on failure.
    func save(_ value: String, forKey key: String) throws {
        guard let data = value.data(using: .utf8) else {
            throw KeychainError.encodingFailed
        }

        // Delete any existing item first so SecItemAdd is clean.
        try? delete(forKey: key)

        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: key,
            kSecValueData as String: data,
            // Item survives device restarts but is NOT migrated to new devices.
            kSecAttrAccessible as String: kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly
        ]

        let status = SecItemAdd(query as CFDictionary, nil)
        guard status == errSecSuccess else {
            throw KeychainError.saveFailed(status)
        }
    }

    /// Loads a string value from the Keychain for the given key.
    ///
    /// - Parameter key: The account key to look up.
    /// - Returns: The stored string.
    /// - Throws: `KeychainError.itemNotFound` when the key has no entry;
    ///           `KeychainError.loadFailed` for other OS-level errors.
    func load(forKey key: String) throws -> String {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: key,
            kSecReturnData as String: true,
            kSecMatchLimit as String: kSecMatchLimitOne
        ]

        var result: AnyObject?
        let status = SecItemCopyMatching(query as CFDictionary, &result)

        guard status != errSecItemNotFound else {
            throw KeychainError.itemNotFound
        }

        guard status == errSecSuccess else {
            throw KeychainError.loadFailed(status)
        }

        guard let data = result as? Data, let value = String(data: data, encoding: .utf8) else {
            throw KeychainError.unexpectedData
        }

        return value
    }

    /// Returns the stored string, or `nil` if the key has no entry.
    ///
    /// This is a convenience wrapper over `load(forKey:)` that swallows
    /// `itemNotFound` and propagates other errors silently (logging them in
    /// DEBUG builds).
    ///
    /// - Parameter key: The account key to look up.
    /// - Returns: The stored string or `nil`.
    func loadOptional(forKey key: String) -> String? {
        do {
            return try load(forKey: key)
        } catch KeychainError.itemNotFound {
            return nil
        } catch {
            #if DEBUG
            print("[KeychainService] loadOptional(\(key)) failed: \(error.localizedDescription)")
            #endif
            return nil
        }
    }

    /// Deletes the Keychain entry for the given key.
    ///
    /// A no-op (does not throw) when the key has no entry.
    ///
    /// - Parameter key: The account key to delete.
    /// - Throws: `KeychainError.deleteFailed` for unexpected OS errors.
    func delete(forKey key: String) throws {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: key
        ]

        let status = SecItemDelete(query as CFDictionary)

        // errSecItemNotFound is not an error — the item simply doesn't exist.
        guard status == errSecSuccess || status == errSecItemNotFound else {
            throw KeychainError.deleteFailed(status)
        }
    }

    /// Returns whether an item exists in the Keychain for the given key.
    ///
    /// - Parameter key: The account key to check.
    /// - Returns: `true` when an item is found.
    func contains(key: String) -> Bool {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: key,
            kSecMatchLimit as String: kSecMatchLimitOne,
            kSecReturnAttributes as String: false
        ]

        let status = SecItemCopyMatching(query as CFDictionary, nil)
        return status == errSecSuccess
    }

    /// Removes **all** Keychain items for this service.
    ///
    /// Intended for use on logout to wipe all stored credentials.
    func deleteAll() {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service
        ]
        SecItemDelete(query as CFDictionary)
    }
}

// MARK: - Well-known Keys

extension KeychainService {
    /// Standard key constants used across the app.
    ///
    /// Using string constants (rather than raw literals) prevents typos and
    /// makes it easy to audit all Keychain access in one place.
    enum Keys {
        /// JWT access token for the Reality Portal SSO session.
        static let accessToken = "reality_portal_access_token"

        /// Refresh token for the Reality Portal SSO session.
        static let refreshToken = "reality_portal_refresh_token"

        /// APNs device token (hex string) registered with the server.
        static let pushDeviceToken = "reality_portal_push_device_token"
    }
}
