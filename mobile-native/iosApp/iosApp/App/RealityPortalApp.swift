import SwiftUI
import UIKit
import shared

/// Main entry point for Reality Portal iOS app.
///
/// Owns the five long-lived app-wide objects (`navigationCoordinator`,
/// `authManager`, `favoritesService`, `pushNotificationManager`,
/// `restorationService`) and wires them together:
///
/// 1. On launch (`onAppear`) — restore auth session, seed `FavoritesService`,
///    configure push notifications, then restore nav state.
/// 2. On background (`scenePhase == .background`) — save nav state.
/// 3. On `onOpenURL` — parse SSO callbacks vs. navigation deep links and
///    delegate accordingly.
///
/// Epic 82 - Story 82.1: SwiftUI Project Setup
/// Epic 82 - Story 82.2: Navigation and Routing (deep link + state restoration)
/// Epic 82 - Story 82.4: Listing Detail and Favorites (FavoritesService injection)
/// Epic 82 - Story 82.5: Inquiries and Account (PushNotificationManager)
@main
struct RealityPortalApp: App {
    // MARK: - State Objects

    @State private var navigationCoordinator = NavigationCoordinator()
    @State private var authManager: AuthManager
    @State private var favoritesService = FavoritesService()
    @State private var pushNotificationManager = PushNotificationManager(
        keychainService: KeychainService(service: Configuration.shared.keychainService)
    )
    @State private var locationManager = LocationManager()
    @Environment(\.scenePhase) private var scenePhase

    private let restorationService: NavigationStateRestorationService
    private let deepLinkHandler = DeepLinkHandler()

    init() {
        let restoration = NavigationStateRestorationService()
        self.restorationService = restoration
        // AuthManager needs the restoration service so logout can wipe the
        // persisted navigation state (otherwise protected stacks would survive
        // sign-out and only be dropped by the next launch's auth-gating).
        self._authManager = State(initialValue: AuthManager(restorationService: restoration))
    }

    // MARK: - App Body

    var body: some Scene {
        WindowGroup {
            MainTabView()
                .environment(navigationCoordinator)
                .environment(authManager)
                .environment(favoritesService)
                .environment(pushNotificationManager)
                .environment(locationManager)
                .onOpenURL { url in
                    handleIncomingURL(url)
                }
                .onAppear {
                    configureApp()
                }
        }
        .onChange(of: scenePhase) { _, newPhase in
            handleScenePhaseChange(newPhase)
        }
    }

    // MARK: - Configuration

    private func configureApp() {
        // Log configuration for debugging
        #if DEBUG
        print("Reality Portal iOS App")
        print("Environment: \(Configuration.shared.environment.rawValue)")
        print("API Base URL: \(Configuration.shared.apiBaseUrl)")
        #endif

        // Restore user session first so we know auth state before restoring
        // navigation (protected tabs are skipped when not authenticated).
        authManager.restoreSession()

        // Seed FavoritesService with the current session token so that the
        // favorites list is available immediately after launch.
        Task {
            await favoritesService.configure(sessionToken: authManager.getSessionToken())
        }

        // Configure push notifications — re-reads system authorization status
        // and re-registers with APNs if already authorized.
        Task {
            await pushNotificationManager.configure()
        }

        // Listen for APNs registration requests from PushNotificationManager.
        // The manager posts this notification to avoid importing UIKit itself.
        NotificationCenter.default.addObserver(
            forName: .pushNotificationManagerRequestsRegistration,
            object: nil,
            queue: .main
        ) { _ in
            UIApplication.shared.registerForRemoteNotifications()
        }

        // Restore navigation state from the previous launch.
        restorationService.restore(
            into: navigationCoordinator,
            isAuthenticated: authManager.isAuthenticated
        )
    }

    // MARK: - Scene Phase Handling

    private func handleScenePhaseChange(_ phase: ScenePhase) {
        switch phase {
        case .background:
            // Persist navigation state so the user returns to the same screen.
            restorationService.save(coordinator: navigationCoordinator)
            #if DEBUG
            print("App moved to background — navigation state saved")
            #endif
        case .inactive:
            break
        case .active:
            #if DEBUG
            print("App became active")
            #endif
        @unknown default:
            break
        }
    }

    // MARK: - APNs Token Handling

    /// Forward an APNs device token to the `PushNotificationManager`.
    ///
    /// In a pure SwiftUI lifecycle app, hook this via `UIApplicationDelegateAdaptor`
    /// if an explicit AppDelegate is added. The notification-center bridge in
    /// `configureApp()` handles APNs registration requests from the service layer.
    func didRegisterForRemoteNotifications(deviceToken: Data) {
        pushNotificationManager.didRegisterForRemoteNotifications(deviceToken: deviceToken)
    }

    /// Forward an APNs registration failure to the `PushNotificationManager`.
    func didFailToRegisterForRemoteNotifications(error: Error) {
        pushNotificationManager.didFailToRegisterForRemoteNotifications(error: error)
    }

    // MARK: - Incoming URL Handling

    /// Routes an incoming URL to either SSO authentication or navigation.
    ///
    /// Auth-gating: routes whose `requiresAuth` is `true` are intercepted
    /// when the user is not authenticated — the intended destination is
    /// stashed on the coordinator as `pendingDestination` and the user is
    /// bounced to `.login`. `AuthManager` replays the destination after a
    /// successful sign-in / SSO callback.
    private func handleIncomingURL(_ url: URL) {
        let result = deepLinkHandler.parse(url)
        switch result {
        case .ssoCallback(let token, let state):
            handleSsoCallback(token: token, state: state)
        case .route(let route):
            if route.requiresAuth && !authManager.isAuthenticated {
                navigationCoordinator.pendingDestination = route
                navigationCoordinator.navigate(to: .login)
            } else {
                navigationCoordinator.navigate(to: route)
            }
        case .unrecognized:
            #if DEBUG
            print("Unrecognised deep link: \(url)")
            #endif
        }
    }

    private func handleSsoCallback(token: String, state: String?) {
        // CSRF protection: the `state` value must match the nonce the app
        // minted before opening the SSO browser flow. AuthManager owns the
        // nonce; reject the callback here before touching the token.
        guard authManager.consumeSsoState(state) else {
            #if DEBUG
            print("SSO callback rejected: state mismatch (CSRF)")
            #endif
            navigationCoordinator.navigate(to: .login)
            return
        }
        Task { @MainActor in
            do {
                try await authManager.loginWithSsoToken(token)

                // Seed favorites now that we have a valid session.
                await favoritesService.configure(sessionToken: authManager.getSessionToken())

                // Navigate to pending destination if any
                if let pendingDestination = navigationCoordinator.pendingDestination {
                    navigationCoordinator.navigate(to: pendingDestination)
                    navigationCoordinator.pendingDestination = nil
                }
            } catch {
                // Log error for debugging
                #if DEBUG
                print("SSO login failed: \(error.localizedDescription)")
                #endif

                // Navigate to login with error state so user sees feedback
                // The AuthManager.errorMessage will be set by the login method
                navigationCoordinator.navigate(to: .login)
            }
        }
    }
}
