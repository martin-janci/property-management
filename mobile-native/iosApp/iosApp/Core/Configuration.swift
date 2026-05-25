import Foundation

/// Environment configuration for Reality Portal iOS app.
///
/// Epic 82 - Story 82.1: SwiftUI Project Setup
enum Environment: String {
    case development
    case staging
    case production

    /// API base URL for each environment.
    var apiBaseUrl: String {
        switch self {
        case .development:
            return "http://localhost:8081"
        case .staging:
            return "https://staging-api.reality.example.com"
        case .production:
            return "https://api.reality.example.com"
        }
    }

    /// Deep link URL scheme.
    var urlScheme: String {
        return "realityportal"
    }

    /// Universal link domain.
    var universalLinkDomain: String {
        switch self {
        case .development:
            return "localhost"
        case .staging:
            return "staging.reality.example.com"
        case .production:
            return "reality.example.com"
        }
    }

    /// Web URL base for sharing links (Story 85.3).
    var webBaseUrl: String {
        switch self {
        case .development:
            return "http://localhost:3000"
        case .staging:
            return "https://staging.reality.example.com"
        case .production:
            return "https://reality.example.com"
        }
    }
}

/// App configuration singleton.
/// Environment values are injected at build time via xcconfig files
/// (Development.xcconfig / Staging.xcconfig / Production.xcconfig) into
/// Info.plist keys API_BASE_URL, ENVIRONMENT, and ENABLE_LOGGING.
/// This allows the same binary to be built for three environments without
/// any source-code changes — only the active Xcode scheme changes.
final class Configuration {
    static let shared = Configuration()

    // MARK: - Environment (xcconfig-injected via Info.plist)

    /// Current environment, read from Info.plist key "ENVIRONMENT"
    /// which is set by the active xcconfig (Development / Staging / Production).
    /// Falls back to `.development` so debug runs are safe even without xcconfig.
    let environment: Environment

    /// API base URL, read from Info.plist key "API_BASE_URL"
    /// (overrides the enum's hard-coded values when xcconfig is active).
    private let _apiBaseUrlOverride: String?

    /// Whether verbose logging is enabled (from Info.plist "ENABLE_LOGGING").
    let loggingEnabled: Bool

    /// Bundle identifier for the app.
    let bundleIdentifier: String

    /// App display name.
    let appName: String

    /// API base URL for current environment.
    /// Uses the xcconfig-injected override when available; falls back to enum default.
    var apiBaseUrl: String {
        _apiBaseUrlOverride ?? environment.apiBaseUrl
    }

    /// Web base URL for sharing (Story 85.3).
    var webBaseUrl: String {
        environment.webBaseUrl
    }

    /// Keychain service identifier.
    var keychainService: String {
        bundleIdentifier
    }

    // MARK: - Version Information (Story 85.2)

    /// App version from Info.plist (e.g., "0.2.194")
    var version: String {
        Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String ?? "Unknown"
    }

    /// App build number from Info.plist (e.g., "2194")
    var buildNumber: String {
        Bundle.main.infoDictionary?["CFBundleVersion"] as? String ?? "0"
    }

    /// Full version string combining version and build (e.g., "0.2.194 (2194)")
    var fullVersionString: String {
        "\(version) (\(buildNumber))"
    }

    // MARK: - Init

    private init() {
        let info = Bundle.main.infoDictionary ?? [:]

        // Read ENVIRONMENT from Info.plist (set by xcconfig)
        let envString = info["ENVIRONMENT"] as? String ?? ""
        switch envString.lowercased() {
        case "staging":
            environment = .staging
        case "production":
            environment = .production
        default:
            // Includes "development" and any missing value
            environment = .development
        }

        // Read API_BASE_URL override from Info.plist (set by xcconfig)
        let rawUrl = info["API_BASE_URL"] as? String ?? ""
        // Ignore the literal placeholder that Xcode injects when xcconfig is not connected
        _apiBaseUrlOverride = rawUrl.isEmpty || rawUrl == "$(API_BASE_URL)" ? nil : rawUrl

        // Read ENABLE_LOGGING flag
        let loggingString = info["ENABLE_LOGGING"] as? String ?? "false"
        loggingEnabled = loggingString.lowercased() == "true"

        // Bundle ID and display name from bundle itself
        bundleIdentifier = Bundle.main.bundleIdentifier ?? "three.two.bit.ppt.reality"
        appName = info["CFBundleDisplayName"] as? String
            ?? info["CFBundleName"] as? String
            ?? "Reality Portal"
    }
}
