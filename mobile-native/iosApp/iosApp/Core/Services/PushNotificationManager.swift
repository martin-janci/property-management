import Foundation
import UserNotifications
import Observation

/// Notification category identifiers for Reality Portal push notifications.
///
/// Epic 82 - Story 82.5: Inquiries and Account
enum NotificationCategory: String {
    case inquiryReply = "INQUIRY_REPLY"
    case newListing   = "NEW_LISTING"
    case priceDrop    = "PRICE_DROP"
    case savedSearch  = "SAVED_SEARCH"
}

/// Notification preference keys — one per toggle in `AccountView`.
///
/// These keys are persisted in `UserDefaults` (preferences, not secrets).
/// The actual APNs device token is stored in the Keychain via `KeychainService`.
enum NotificationPreferenceKey: String, CaseIterable {
    case newListings        = "pref_notif_new_listings"
    case priceDrops         = "pref_notif_price_drops"
    case inquiryResponses   = "pref_notif_inquiry_responses"
    case marketing          = "pref_notif_marketing"
}

/// Manages APNs registration, permission requests, device-token persistence,
/// and per-category notification preferences for the Reality Portal iOS app.
///
/// **Architecture contract:**
/// - Created once in `RealityPortalApp` and injected as an `@Environment` value.
/// - The APNs device token (hex string) is stored in the Keychain via
///   `KeychainService` under `KeychainService.Keys.pushDeviceToken`.
/// - Notification *preference* booleans (toggles) are stored in `UserDefaults`
///   because they are not secrets and need no extra protection.
/// - Callers observe `@Observable` properties; no delegate pattern needed from
///   SwiftUI views.
///
/// **Lifecycle:**
/// 1. `RealityPortalApp.configureApp()` calls `configure()` once.
/// 2. `configure()` re-checks the current `UNUserNotificationCenter` status
///    and, if already authorised, re-registers with APNs.
/// 3. Call `requestAuthorization()` from the Account screen when the user
///    enables their first notification toggle.
/// 4. When AppDelegate / scene callbacks deliver a device token call
///    `didRegisterForRemoteNotifications(deviceToken:)`.
///
/// Epic 82 - Story 82.5: Inquiries and Account
@Observable
final class PushNotificationManager {

    // MARK: - Observable State

    /// Whether the user has granted push notification permission.
    private(set) var isAuthorized: Bool = false

    /// Whether an authorization request or APNs registration is in flight.
    private(set) var isRequestingPermission: Bool = false

    /// The raw APNs device token as a hex string, or `nil` if not yet registered.
    private(set) var deviceToken: String?

    /// Per-category preference values (true = enabled).
    private(set) var preferences: [NotificationPreferenceKey: Bool] = [:]

    // MARK: - Private

    private let keychainService: KeychainService
    private let notificationCenter: UNUserNotificationCenter

    // MARK: - Initialization

    /// Creates a `PushNotificationManager`.
    ///
    /// - Parameters:
    ///   - keychainService: The shared `KeychainService` instance used to
    ///     persist the APNs device token.
    ///   - notificationCenter: Defaults to `.current()`; injectable for testing.
    init(
        keychainService: KeychainService,
        notificationCenter: UNUserNotificationCenter = .current()
    ) {
        self.keychainService = keychainService
        self.notificationCenter = notificationCenter

        // Load the cached device token from Keychain.
        deviceToken = keychainService.loadOptional(forKey: KeychainService.Keys.pushDeviceToken)

        // Load preferences from UserDefaults, defaulting to reasonable values.
        loadPreferences()
    }

    // MARK: - Public API

    /// Configure the manager on app launch.
    ///
    /// Re-reads the current `UNAuthorizationStatus` so the UI reflects the
    /// real system state (the user may have changed it in Settings).
    /// If already authorized, re-registers with APNs so the token stays fresh.
    func configure() async {
        let settings = await notificationCenter.notificationSettings()
        let authorized = settings.authorizationStatus == .authorized
            || settings.authorizationStatus == .provisional

        await MainActor.run { isAuthorized = authorized }

        if authorized {
            await registerCategories()
            await MainActor.run {
                // Re-register with APNs via the notification bridge.
                NotificationCenter.default.post(
                    name: .pushNotificationManagerRequestsRegistration,
                    object: nil
                )
            }
        }
    }

    /// Request notification authorization from the user.
    ///
    /// Should be called the first time the user enables a notification toggle
    /// in `AccountView`. Safe to call multiple times — if already authorized
    /// the system returns immediately.
    ///
    /// - Returns: Whether authorization was granted.
    @discardableResult
    func requestAuthorization() async -> Bool {
        guard !isRequestingPermission else { return isAuthorized }

        await MainActor.run { isRequestingPermission = true }
        defer { Task { @MainActor in isRequestingPermission = false } }

        do {
            let granted = try await notificationCenter.requestAuthorization(
                options: [.alert, .badge, .sound]
            )
            await MainActor.run { isAuthorized = granted }

            if granted {
                await registerCategories()
                await MainActor.run {
                    NotificationCenter.default.post(
                        name: .pushNotificationManagerRequestsRegistration,
                        object: nil
                    )
                }
            }
            return granted
        } catch {
            #if DEBUG
            print("[PushNotificationManager] requestAuthorization failed: \(error.localizedDescription)")
            #endif
            return false
        }
    }

    /// Called by the app delegate / scene delegate when APNs delivers a device token.
    ///
    /// Converts the raw `Data` to a lowercase hex string and persists it in the
    /// Keychain under `KeychainService.Keys.pushDeviceToken`.
    ///
    /// - Parameter data: The raw device token `Data` from the APNs callback.
    func didRegisterForRemoteNotifications(deviceToken data: Data) {
        let hexToken = data.map { String(format: "%02x", $0) }.joined()

        // Persist in Keychain — overwrite any stale token.
        do {
            try keychainService.save(hexToken, forKey: KeychainService.Keys.pushDeviceToken)
        } catch {
            #if DEBUG
            print("[PushNotificationManager] Keychain save failed: \(error.localizedDescription)")
            #endif
        }

        deviceToken = hexToken

        #if DEBUG
        print("[PushNotificationManager] Device token registered: \(hexToken)")
        #endif
    }

    /// Called when APNs registration fails.
    ///
    /// - Parameter error: The registration error.
    func didFailToRegisterForRemoteNotifications(error: Error) {
        #if DEBUG
        print("[PushNotificationManager] APNs registration failed: \(error.localizedDescription)")
        #endif
    }

    /// Returns whether the given preference key is enabled.
    func isEnabled(_ key: NotificationPreferenceKey) -> Bool {
        preferences[key] ?? defaultValue(for: key)
    }

    /// Toggles the given preference and persists it to `UserDefaults`.
    ///
    /// If enabling any notification while not authorized, this also triggers
    /// an authorization request.
    func setEnabled(_ enabled: Bool, for key: NotificationPreferenceKey) async {
        // Request permission when the user turns on any notification.
        if enabled && !isAuthorized {
            let granted = await requestAuthorization()
            guard granted else {
                // Authorization denied — don't persist the change.
                #if DEBUG
                print("[PushNotificationManager] Permission denied — preference not saved for \(key.rawValue)")
                #endif
                return
            }
        }

        preferences[key] = enabled
        UserDefaults.standard.set(enabled, forKey: key.rawValue)
    }

    /// Clears the stored device token from the Keychain.
    ///
    /// Call on logout so the server is notified the token is stale (via the
    /// calling code — this method only clears local storage).
    func clearDeviceToken() {
        try? keychainService.delete(forKey: KeychainService.Keys.pushDeviceToken)
        deviceToken = nil
    }

    // MARK: - Private

    private func loadPreferences() {
        var loaded: [NotificationPreferenceKey: Bool] = [:]
        for key in NotificationPreferenceKey.allCases {
            if UserDefaults.standard.object(forKey: key.rawValue) != nil {
                loaded[key] = UserDefaults.standard.bool(forKey: key.rawValue)
            } else {
                loaded[key] = defaultValue(for: key)
            }
        }
        preferences = loaded
    }

    private func defaultValue(for key: NotificationPreferenceKey) -> Bool {
        switch key {
        case .newListings:      return true
        case .priceDrops:       return true
        case .inquiryResponses: return true
        case .marketing:        return false
        }
    }

    private func registerCategories() async {
        let inquiryAction = UNNotificationAction(
            identifier: "VIEW_INQUIRY",
            title: "View Inquiry",
            options: [.foreground]
        )
        let inquiryCategory = UNNotificationCategory(
            identifier: NotificationCategory.inquiryReply.rawValue,
            actions: [inquiryAction],
            intentIdentifiers: [],
            options: [.customDismissAction]
        )

        let viewListingAction = UNNotificationAction(
            identifier: "VIEW_LISTING",
            title: "View Listing",
            options: [.foreground]
        )
        let newListingCategory = UNNotificationCategory(
            identifier: NotificationCategory.newListing.rawValue,
            actions: [viewListingAction],
            intentIdentifiers: [],
            options: []
        )
        let priceDropCategory = UNNotificationCategory(
            identifier: NotificationCategory.priceDrop.rawValue,
            actions: [viewListingAction],
            intentIdentifiers: [],
            options: []
        )
        let savedSearchCategory = UNNotificationCategory(
            identifier: NotificationCategory.savedSearch.rawValue,
            actions: [viewListingAction],
            intentIdentifiers: [],
            options: []
        )

        notificationCenter.setNotificationCategories([
            inquiryCategory,
            newListingCategory,
            priceDropCategory,
            savedSearchCategory,
        ])
    }
}

// MARK: - Notification Name Extension

extension Notification.Name {
    /// Posted by `PushNotificationManager` when it wants the app entry point
    /// to call `UIApplication.shared.registerForRemoteNotifications()`.
    ///
    /// `RealityPortalApp.configureApp()` installs a `NotificationCenter` observer
    /// for this name on the main queue and calls the UIKit API from there,
    /// keeping the service layer free of UIKit imports.
    static let pushNotificationManagerRequestsRegistration = Notification.Name(
        "three.two.bit.ppt.reality.PushNotificationManagerRequestsRegistration"
    )

    /// Posted by `AppDelegate` when APNs delivers a device token. `userInfo`
    /// carries the raw token under the `"deviceToken"` key (`Data`).
    static let pushNotificationManagerDidRegister = Notification.Name(
        "three.two.bit.ppt.reality.PushNotificationManagerDidRegister"
    )

    /// Posted by `AppDelegate` when APNs registration fails. `userInfo` carries
    /// the failure under the `"error"` key (`Error`).
    static let pushNotificationManagerDidFailToRegister = Notification.Name(
        "three.two.bit.ppt.reality.PushNotificationManagerDidFailToRegister"
    )
}
