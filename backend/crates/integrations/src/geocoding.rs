//! Geocoding integration (Story 3.1 AC3, BIT-200).
//!
//! Converts a building's street address into latitude/longitude coordinates so
//! the building can be displayed on a map.
//!
//! # Graceful degradation (feature-flagged)
//!
//! The service is **disabled by default**. It only makes external HTTP calls
//! when a provider *and* the credentials it needs are configured via
//! environment variables. When unconfigured, [`GeocodingService::geocode`]
//! returns `Ok(None)` without touching the network, so:
//!
//! - CI and offline development never make external calls, and
//! - building create/update keep working, storing `NULL` coordinates.
//!
//! This keeps API keys out of the repository: an operator opts in per
//! deployment by setting the env vars below.
//!
//! ## Configuration
//!
//! - `GEOCODING_PROVIDER` — `google`, `mapbox`, or `nominatim` (alias `osm`).
//!   Anything else (or unset) disables geocoding.
//! - `GEOCODING_API_KEY` — required for `google`/`mapbox`. Optional for
//!   `nominatim` (most OSM endpoints are keyless).
//! - `GEOCODING_BASE_URL` — optional endpoint override. Required for
//!   `nominatim` (no default is hard-coded so we never hammer the public OSM
//!   service unintentionally).

use rust_decimal::Decimal;
use serde::Deserialize;
use std::time::Duration;

/// Resolved geographic coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Coordinates {
    pub latitude: Decimal,
    pub longitude: Decimal,
}

/// Errors that can occur while geocoding.
///
/// Note: callers on the building create/update path treat these as
/// non-fatal — a building is still persisted with `NULL` coordinates when
/// geocoding fails, so a flaky provider never blocks a write.
#[derive(Debug, thiserror::Error)]
pub enum GeocodingError {
    #[error("geocoding request failed: {0}")]
    Request(String),
    #[error("geocoding response could not be parsed: {0}")]
    Parse(String),
}

/// Supported geocoding providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeocodingProvider {
    Google,
    Mapbox,
    /// Nominatim / OSM-compatible endpoint (keyless; requires a base URL).
    Nominatim,
}

impl GeocodingProvider {
    fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "google" => Some(Self::Google),
            "mapbox" => Some(Self::Mapbox),
            "nominatim" | "osm" => Some(Self::Nominatim),
            _ => None,
        }
    }
}

/// Resolved, validated configuration for an *enabled* geocoder.
#[derive(Debug, Clone)]
struct GeocodingConfig {
    provider: GeocodingProvider,
    api_key: String,
    base_url: Option<String>,
}

/// Address-to-coordinates service.
///
/// Cheap to clone (wraps a `reqwest::Client`, which is internally `Arc`'d).
#[derive(Clone)]
pub struct GeocodingService {
    /// `None` when geocoding is disabled (the default).
    config: Option<GeocodingConfig>,
    client: reqwest::Client,
}

impl std::fmt::Debug for GeocodingService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never print the API key.
        f.debug_struct("GeocodingService")
            .field("enabled", &self.config.is_some())
            .field("provider", &self.config.as_ref().map(|c| c.provider))
            .finish()
    }
}

impl GeocodingService {
    /// Build a service from environment variables. Returns a disabled service
    /// (no-op) when the configuration is absent or incomplete — this never
    /// panics, so it is safe to call unconditionally at startup.
    pub fn from_env() -> Self {
        let config = Self::config_from_env();
        if let Some(cfg) = &config {
            tracing::info!(provider = ?cfg.provider, "Geocoding enabled");
        } else {
            tracing::info!("Geocoding disabled (no provider configured); buildings stored without coordinates");
        }
        Self::with_config(config)
    }

    fn with_config(config: Option<GeocodingConfig>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(8))
            // Nominatim's usage policy requires a descriptive User-Agent.
            .user_agent("property-management/geocoding")
            .build()
            .unwrap_or_default();
        Self { config, client }
    }

    fn config_from_env() -> Option<GeocodingConfig> {
        let provider = GeocodingProvider::parse(&std::env::var("GEOCODING_PROVIDER").ok()?)?;
        let api_key = std::env::var("GEOCODING_API_KEY").unwrap_or_default();
        let base_url = std::env::var("GEOCODING_BASE_URL")
            .ok()
            .filter(|s| !s.trim().is_empty());

        match provider {
            GeocodingProvider::Google | GeocodingProvider::Mapbox => {
                if api_key.trim().is_empty() {
                    tracing::warn!(
                        provider = ?provider,
                        "GEOCODING_PROVIDER set but GEOCODING_API_KEY is empty; geocoding disabled"
                    );
                    return None;
                }
            }
            GeocodingProvider::Nominatim => {
                if base_url.is_none() {
                    tracing::warn!(
                        "GEOCODING_PROVIDER=nominatim requires GEOCODING_BASE_URL; geocoding disabled"
                    );
                    return None;
                }
            }
        }

        Some(GeocodingConfig {
            provider,
            api_key,
            base_url,
        })
    }

    /// Whether geocoding is actually configured. When `false`, [`Self::geocode`]
    /// always returns `Ok(None)`.
    pub fn is_enabled(&self) -> bool {
        self.config.is_some()
    }

    /// Geocode a free-form address into coordinates.
    ///
    /// Returns `Ok(None)` when geocoding is disabled, when the address is blank,
    /// or when the provider finds no match. Returns `Err` only on a transport or
    /// parse failure (which callers on the write path should log and ignore).
    pub async fn geocode(&self, address: &str) -> Result<Option<Coordinates>, GeocodingError> {
        let Some(cfg) = &self.config else {
            return Ok(None);
        };
        if address.trim().is_empty() {
            return Ok(None);
        }

        let url = build_request_url(cfg, address);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| GeocodingError::Request(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(GeocodingError::Request(format!(
                "provider returned HTTP {}",
                resp.status()
            )));
        }

        let body = resp
            .text()
            .await
            .map_err(|e| GeocodingError::Request(e.to_string()))?;

        match cfg.provider {
            GeocodingProvider::Google => parse_google(&body),
            GeocodingProvider::Mapbox => parse_mapbox(&body),
            GeocodingProvider::Nominatim => parse_nominatim(&body),
        }
    }
}

impl Default for GeocodingService {
    /// A disabled (no-op) service. Useful in tests.
    fn default() -> Self {
        Self::with_config(None)
    }
}

/// Build the provider request URL for an address.
fn build_request_url(cfg: &GeocodingConfig, address: &str) -> String {
    let encoded = urlencoding::encode(address);
    match cfg.provider {
        GeocodingProvider::Google => {
            let base = cfg
                .base_url
                .as_deref()
                .unwrap_or("https://maps.googleapis.com/maps/api/geocode/json");
            format!("{base}?address={encoded}&key={}", cfg.api_key)
        }
        GeocodingProvider::Mapbox => {
            let base = cfg
                .base_url
                .as_deref()
                .unwrap_or("https://api.mapbox.com/geocoding/v5/mapbox.places");
            format!("{base}/{encoded}.json?limit=1&access_token={}", cfg.api_key)
        }
        GeocodingProvider::Nominatim => {
            // base_url is guaranteed present for Nominatim (see config_from_env).
            let base = cfg.base_url.as_deref().unwrap_or_default();
            let key_param = if cfg.api_key.trim().is_empty() {
                String::new()
            } else {
                format!("&key={}", cfg.api_key)
            };
            format!("{base}/search?q={encoded}&format=json&limit=1{key_param}")
        }
    }
}

fn coords_from_f64(lat: f64, lng: f64) -> Result<Coordinates, GeocodingError> {
    let latitude = Decimal::from_f64_retain(lat)
        .ok_or_else(|| GeocodingError::Parse(format!("invalid latitude: {lat}")))?;
    let longitude = Decimal::from_f64_retain(lng)
        .ok_or_else(|| GeocodingError::Parse(format!("invalid longitude: {lng}")))?;
    Ok(Coordinates {
        latitude,
        longitude,
    })
}

// ---- Provider response parsers (pure; unit-tested) ----

#[derive(Deserialize)]
struct GoogleResponse {
    results: Vec<GoogleResult>,
}
#[derive(Deserialize)]
struct GoogleResult {
    geometry: GoogleGeometry,
}
#[derive(Deserialize)]
struct GoogleGeometry {
    location: GoogleLocation,
}
#[derive(Deserialize)]
struct GoogleLocation {
    lat: f64,
    lng: f64,
}

fn parse_google(body: &str) -> Result<Option<Coordinates>, GeocodingError> {
    let parsed: GoogleResponse =
        serde_json::from_str(body).map_err(|e| GeocodingError::Parse(e.to_string()))?;
    match parsed.results.into_iter().next() {
        Some(r) => Ok(Some(coords_from_f64(
            r.geometry.location.lat,
            r.geometry.location.lng,
        )?)),
        None => Ok(None),
    }
}

#[derive(Deserialize)]
struct MapboxResponse {
    features: Vec<MapboxFeature>,
}
#[derive(Deserialize)]
struct MapboxFeature {
    /// Mapbox returns `[longitude, latitude]`.
    center: Vec<f64>,
}

fn parse_mapbox(body: &str) -> Result<Option<Coordinates>, GeocodingError> {
    let parsed: MapboxResponse =
        serde_json::from_str(body).map_err(|e| GeocodingError::Parse(e.to_string()))?;
    match parsed.features.into_iter().next() {
        Some(f) => {
            if f.center.len() < 2 {
                return Err(GeocodingError::Parse(
                    "mapbox feature missing center coordinates".to_string(),
                ));
            }
            // center = [lng, lat]
            Ok(Some(coords_from_f64(f.center[1], f.center[0])?))
        }
        None => Ok(None),
    }
}

#[derive(Deserialize)]
struct NominatimResult {
    lat: String,
    lon: String,
}

fn parse_nominatim(body: &str) -> Result<Option<Coordinates>, GeocodingError> {
    let parsed: Vec<NominatimResult> =
        serde_json::from_str(body).map_err(|e| GeocodingError::Parse(e.to_string()))?;
    match parsed.into_iter().next() {
        Some(r) => {
            let lat: f64 = r
                .lat
                .parse()
                .map_err(|_| GeocodingError::Parse(format!("invalid latitude: {}", r.lat)))?;
            let lng: f64 = r
                .lon
                .parse()
                .map_err(|_| GeocodingError::Parse(format!("invalid longitude: {}", r.lon)))?;
            Ok(Some(coords_from_f64(lat, lng)?))
        }
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::prelude::ToPrimitive;

    fn approx(d: Decimal, expected: f64) {
        let v = d.to_f64().unwrap();
        assert!((v - expected).abs() < 1e-6, "expected ~{expected}, got {v}");
    }

    #[test]
    fn disabled_when_provider_unset() {
        let svc = GeocodingService::default();
        assert!(!svc.is_enabled());
    }

    #[tokio::test]
    async fn disabled_service_returns_none_without_network() {
        let svc = GeocodingService::default();
        let out = svc.geocode("Hlavna 1, Bratislava").await.unwrap();
        assert_eq!(out, None);
    }

    #[test]
    fn provider_parse_aliases() {
        assert_eq!(GeocodingProvider::parse("Google"), Some(GeocodingProvider::Google));
        assert_eq!(GeocodingProvider::parse("MAPBOX"), Some(GeocodingProvider::Mapbox));
        assert_eq!(GeocodingProvider::parse("osm"), Some(GeocodingProvider::Nominatim));
        assert_eq!(GeocodingProvider::parse("nominatim"), Some(GeocodingProvider::Nominatim));
        assert_eq!(GeocodingProvider::parse("bogus"), None);
    }

    #[test]
    fn parse_google_ok() {
        let body = r#"{
            "results": [
                {"geometry": {"location": {"lat": 48.1485965, "lng": 17.1077477}}}
            ],
            "status": "OK"
        }"#;
        let c = parse_google(body).unwrap().unwrap();
        approx(c.latitude, 48.1485965);
        approx(c.longitude, 17.1077477);
    }

    #[test]
    fn parse_google_empty_is_none() {
        let body = r#"{"results": [], "status": "ZERO_RESULTS"}"#;
        assert_eq!(parse_google(body).unwrap(), None);
    }

    #[test]
    fn parse_mapbox_ok() {
        // Mapbox center is [lng, lat].
        let body = r#"{"features": [{"center": [17.1077477, 48.1485965]}]}"#;
        let c = parse_mapbox(body).unwrap().unwrap();
        approx(c.latitude, 48.1485965);
        approx(c.longitude, 17.1077477);
    }

    #[test]
    fn parse_mapbox_empty_is_none() {
        let body = r#"{"features": []}"#;
        assert_eq!(parse_mapbox(body).unwrap(), None);
    }

    #[test]
    fn parse_nominatim_ok() {
        let body = r#"[{"lat": "48.1485965", "lon": "17.1077477"}]"#;
        let c = parse_nominatim(body).unwrap().unwrap();
        approx(c.latitude, 48.1485965);
        approx(c.longitude, 17.1077477);
    }

    #[test]
    fn parse_nominatim_empty_is_none() {
        assert_eq!(parse_nominatim("[]").unwrap(), None);
    }

    #[test]
    fn build_url_encodes_address() {
        let cfg = GeocodingConfig {
            provider: GeocodingProvider::Google,
            api_key: "secret".to_string(),
            base_url: None,
        };
        let url = build_request_url(&cfg, "Hlavná 1, Bratislava");
        assert!(url.contains("address=Hlavn%C3%A1%201"));
        assert!(url.contains("key=secret"));
    }
}
