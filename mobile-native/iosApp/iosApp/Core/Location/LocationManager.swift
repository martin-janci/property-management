import CoreLocation
import Foundation
import Observation

/// Manages CoreLocation access for the Reality Portal iOS app.
///
/// Requests "when in use" authorisation, publishes the user's last known
/// coordinate, and exposes a human-readable authorisation-status string so
/// views can show appropriate prompts.
///
/// Epic 82 - Story 82.3: Home and Search Screens – CoreLocation integration
@Observable
final class LocationManager: NSObject {
    // MARK: - Observable state

    /// The user's most recently observed coordinate, or nil if unavailable.
    private(set) var coordinate: CLLocationCoordinate2D?

    /// Current authorisation status (refreshed whenever it changes).
    private(set) var authorizationStatus: CLAuthorizationStatus = .notDetermined

    /// true while the manager is actively fetching a location fix.
    private(set) var isLocating = false

    /// Non-nil when location services fail.
    private(set) var locationError: String?

    // MARK: - Private

    private let manager: CLLocationManager

    /// Set to true only when the user explicitly requests location (requestLocation()).
    /// Guards the auto-fetch in locationManagerDidChangeAuthorization so we don't
    /// eagerly fire a location fix on every app launch after permission is granted.
    private var wantsLocation = false

    // MARK: - Init

    override init() {
        manager = CLLocationManager()
        super.init()
        manager.delegate = self
        manager.desiredAccuracy = kCLLocationAccuracyHundredMeters
        manager.distanceFilter = 500
        authorizationStatus = manager.authorizationStatus
    }

    // MARK: - Public API

    /// Request permission and start a one-shot location update.
    func requestLocation() {
        locationError = nil
        wantsLocation = true
        switch manager.authorizationStatus {
        case .authorizedWhenInUse, .authorizedAlways:
            fetchLocation()
        case .notDetermined:
            manager.requestWhenInUseAuthorization()
        case .denied, .restricted:
            wantsLocation = false
            locationError = String(localized: "location_access_denied")
        @unknown default:
            wantsLocation = false
            locationError = String(localized: "location_unavailable")
        }
    }

    /// Stop location updates.
    func stopLocation() {
        manager.stopUpdatingLocation()
        isLocating = false
        wantsLocation = false
    }

    /// Clear any outstanding location error (call from alert dismiss action).
    func clearLocationError() {
        locationError = nil
    }

    // MARK: - Private

    private func fetchLocation() {
        isLocating = true
        manager.requestLocation()
    }
}

// MARK: - CLLocationManagerDelegate

extension LocationManager: CLLocationManagerDelegate {
    func locationManager(
        _ manager: CLLocationManager,
        didUpdateLocations locations: [CLLocation]
    ) {
        isLocating = false
        wantsLocation = false
        guard let loc = locations.last else { return }
        coordinate = loc.coordinate
    }

    func locationManager(_ manager: CLLocationManager, didFailWithError error: Error) {
        isLocating = false
        let clError = error as? CLError
        if clError?.code != .locationUnknown {
            locationError = error.localizedDescription
        }
    }

    func locationManagerDidChangeAuthorization(_ manager: CLLocationManager) {
        authorizationStatus = manager.authorizationStatus
        switch manager.authorizationStatus {
        case .authorizedWhenInUse, .authorizedAlways:
            // Only auto-fetch if the user explicitly requested location — not on
            // every app launch after permission was previously granted.
            if !isLocating && wantsLocation { fetchLocation() }
        case .denied, .restricted:
            locationError = String(localized: "location_access_denied")
            isLocating = false
        default:
            break
        }
    }
}
