import UIKit
import UserNotifications

/// UIKit application delegate for the otherwise-pure-SwiftUI Reality Portal app.
///
/// SwiftUI's `App` lifecycle does not expose the APNs callbacks
/// (`application(_:didRegisterForRemoteNotificationsWithDeviceToken:)`) nor a
/// place to set the `UNUserNotificationCenter` delegate. Without a delegate,
/// foreground notifications are silently dropped (no banner/sound) and taps on
/// notifications are never routed. Issue #698 finding 2.
///
/// This delegate is installed via `@UIApplicationDelegateAdaptor` in
/// `RealityPortalApp`. It keeps the service layer (`PushNotificationManager`)
/// UIKit-free by forwarding events through `NotificationCenter` — the same
/// decoupling pattern already used for APNs registration requests.
final class AppDelegate: NSObject, UIApplicationDelegate, UNUserNotificationCenterDelegate {

    func application(
        _ application: UIApplication,
        didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]? = nil
    ) -> Bool {
        // Become the notification-center delegate so foreground presentation
        // and tap-handling callbacks fire. PushNotificationManager owns
        // category registration + preferences; this object only bridges the
        // OS-level delegate callbacks.
        UNUserNotificationCenter.current().delegate = self
        return true
    }

    // MARK: - APNs token callbacks

    func application(
        _ application: UIApplication,
        didRegisterForRemoteNotificationsWithDeviceToken deviceToken: Data
    ) {
        NotificationCenter.default.post(
            name: .pushNotificationManagerDidRegister,
            object: nil,
            userInfo: ["deviceToken": deviceToken]
        )
    }

    func application(
        _ application: UIApplication,
        didFailToRegisterForRemoteNotificationsWithError error: Error
    ) {
        NotificationCenter.default.post(
            name: .pushNotificationManagerDidFailToRegister,
            object: nil,
            userInfo: ["error": error]
        )
    }

    // MARK: - UNUserNotificationCenterDelegate

    /// Present banners/sounds even when the app is in the foreground.
    func userNotificationCenter(
        _ center: UNUserNotificationCenter,
        willPresent notification: UNNotification,
        withCompletionHandler completionHandler: @escaping (UNNotificationPresentationOptions) -> Void
    ) {
        completionHandler([.banner, .list, .sound, .badge])
    }

    /// Handle taps on a delivered notification. Routing is left to the app's
    /// navigation layer; for now we simply acknowledge so the system does not
    /// log an unhandled-response warning.
    func userNotificationCenter(
        _ center: UNUserNotificationCenter,
        didReceive response: UNNotificationResponse,
        withCompletionHandler completionHandler: @escaping () -> Void
    ) {
        completionHandler()
    }
}
