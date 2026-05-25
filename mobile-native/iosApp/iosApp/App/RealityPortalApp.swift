import SwiftUI
import shared

/// Main entry point for Reality Portal iOS app.
///
/// Owns the three long-lived app-wide objects (`navigationCoordinator`,
/// `authManager`, `restorationService`) and wires them together:
///
/// 1. On launch (`onAppear`) — restore auth session, then restore nav state.
/// 2. On background (`scenePhase == .background`) — save nav state.
/// 3. On `onOpenURL` — parse SSO callbacks vs. navigation deep links and
///    delegate accordingly.
///
/// Epic 82 - Story 82.1: SwiftUI Project Setup
/// Epic 82 - Story 82.2: Navigation and Routing (deep link + state restoration)
@main
struct RealityPortalApp: App {
    // MARK: - State Objects

    @State private var navigationCoordinator = NavigationCoordinator()
    @State private var authManager = AuthManager()
    @Environment(\.scenePhase) private var scenePhase

    private let restorationService = NavigationStateRestorationService()
    private let deepLinkHandler = DeepLinkHandler()

    // MARK: - App Body

    var body: some Scene {
        WindowGroup {
            MainTabView()
                .environment(navigationCoordinator)
                .environment(authManager)
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

    // MARK: - Incoming URL Handling

    /// Routes an incoming URL to either SSO authentication or navigation.
    private func handleIncomingURL(_ url: URL) {
        let result = deepLinkHandler.parse(url)
        switch result {
        case .ssoCallback(let token, _):
            handleSsoCallback(token: token)
        case .route(let route):
            navigationCoordinator.navigate(to: route)
        case .unrecognized:
            #if DEBUG
            print("Unrecognised deep link: \(url)")
            #endif
        }
    }

    private func handleSsoCallback(token: String) {
        Task { @MainActor in
            do {
                try await authManager.loginWithSsoToken(token)

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
