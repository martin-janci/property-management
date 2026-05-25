import Foundation

/// Environment configuration for Reality Portal iOS app.
///
/// Epic 82 - Story 82.1: SwiftUI Project Setup
enum Environment: String {
    case development
    case staging
    case production

    var apiBaseUrl: String {
        switch self {
        case .development: return "http://localhost:8081"
        case .staging:     return "https://staging-api.reality.example.com"
        case .production:  return "https://api.reality.example.com"
        }
    }

    var urlScheme: String { return "realityportal" }

    var universalLinkDomain: String {
        switch self {
        case .development: return "localhost"
        case .staging:     return "staging.reality.example.com"
        case .production:  return "reality.example.com"
        }
    }

    var webBaseUrl: String {
        switch self {
        case .development: return "http://localhost:3000"
        case .staging:     return "https://staging.reality.example.com"
        case .production:  return "https://reality.example.com"
        }
    }
}

/// App configuration singleton.
/// Environment values are injected at build time via xcconfig files
/// (Development.xcconfig / Staging.xcconfig / Production.xcconfig) into
/// Info.plist keys API_BASE_URL, ENVIRONMENT, and ENABLE_LOGGING.
/// Epic 85 - Story 85.2: Build Configuration by Environment
final class Configuration {
    static let shared = Configuration()

    let environment: Environment
    private let _apiBaseUrlOverride: String?
    let loggingEnabled: Bool
    let bundleIdentifier: String
    let appName: String

    var apiBaseUrl: String { _apiBaseUrlOverride ?? environment.apiBaseUrl }
    var webBaseUrl: String { environment.webBaseUrl }
    var keychainService: String { bundleIdentifier }

    var version: String { Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String ?? "Unknown" }
    var buildNumber: String { Bundle.main.infoDictionary?["CFBundleVersion"] as? String ?? "0" }
    var fullVersionString: String { "\(version) (\(buildNumber))" }

    private init() {
        let info = Bundle.main.infoDictionary ?? [:]
        let envString = info["ENVIRONMENT"] as? String ?? ""
        switch envString.lowercased() {
        case "staging":    environment = .staging
        case "production": environment = .production
        default:           environment = .development
        }
        let rawUrl = info["API_BASE_URL"] as? String ?? ""
        _apiBaseUrlOverride = rawUrl.isEmpty || rawUrl == "$(API_BASE_URL)" ? nil : rawUrl
        let loggingStr = info["ENABLE_LOGGING"] as? String ?? "false"
        loggingEnabled = loggingStr.lowercased() == "true"
        bundleIdentifier = Bundle.main.bundleIdentifier ?? "three.two.bit.ppt.reality"
        appName = info["CFBundleDisplayName"] as? String ?? info["CFBundleName"] as? String ?? "Reality Portal"
    }
}
