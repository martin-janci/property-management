//! Voice webhook signature / token verification (Story 93.3).
//!
//! Behaviour-preserving extraction from the monolithic `voice_webhooks.rs`. This
//! module owns every cryptographic verification surface the mounted handlers
//! delegate to:
//!
//! - **Alexa** — certificate-chain-based request signing: cert-URL validation,
//!   timestamp (anti-replay) validation, cert-chain fetch (shared timeout-bounded
//!   client + TTL cache), and the pure leaf-validity / SAN / RSA-SHA1 body
//!   signature check (`verify_alexa_cert_and_signature`, issue #2668).
//! - **Google** — RS256 ID-token verification against Google's JWKS
//!   (`/oauth2/v3/certs`, 1h cache) with exact `aud` / trusted `iss` / `exp`
//!   enforcement; fails closed on every path.
//! - **HMAC** — the shared-secret `/verify` "google" branch
//!   (`verify_hmac_signature`), fail-closed on an unset/blank secret, with the
//!   constant-time compare reused from `services::voice_oauth`.

use axum::http::HeaderMap;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use chrono::{DateTime, Utc};
use hmac::{Hmac, KeyInit, Mac};
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use serde::Deserialize;
use sha2::Sha256;
use std::sync::OnceLock;
use tokio::sync::RwLock;

use crate::services::voice_oauth::ct_eq_str;

type HmacSha256 = Hmac<Sha256>;

// ============================================================================
// Alexa Skills Kit signature verification
// ============================================================================

/// Verify an Alexa Skills Kit request signature (issue #2668 — SECURITY).
///
/// The previous implementation validated only the certificate URL format and
/// the request timestamp and then returned `Ok(())` — the `Signature` header
/// was bound to `_signature` and never checked, so **any** request carrying a
/// well-formed `SignatureCertChainUrl` and a fresh timestamp was accepted
/// regardless of whether it was actually signed by Amazon. That let an attacker
/// forge arbitrary voice commands (report faults, read announcements, etc.) for
/// any linked device.
///
/// We now implement Amazon's documented verification
/// (<https://developer.amazon.com/docs/custom-skills/host-a-custom-skill-as-a-web-service.html#verifying-that-the-request-was-sent-by-alexa>):
///
/// 1. Validate the `SignatureCertChainUrl` (scheme/host/port/path — existing
///    `validate_alexa_cert_url`). This constrains the fetch to Amazon's S3
///    bucket over TLS, so the certificate origin is authenticated by the
///    transport.
/// 2. Validate the request timestamp is within 150 seconds (anti-replay).
/// 3. Fetch the PEM certificate chain from the (validated) URL, with a small
///    in-process TTL cache so a burst of requests does not re-fetch.
/// 4. Validate the signing (leaf) certificate: it is currently within its
///    validity window and its Subject Alternative Names include
///    `echo-api.amazon.com`.
/// 5. Verify the base64-decoded `Signature` header is a valid PKCS#1 v1.5
///    RSA-SHA1 signature of the **raw request body** under the leaf
///    certificate's public key.
///
/// Any failure returns `Err(_)`; the caller maps that to `401 Unauthorized`.
pub(crate) async fn verify_alexa_signature(headers: &HeaderMap, body: &[u8]) -> Result<(), String> {
    // Get required headers. Alexa header names are case-insensitive; `HeaderMap`
    // lookups already normalise case.
    let cert_url = headers
        .get("SignatureCertChainUrl")
        .and_then(|v| v.to_str().ok())
        .ok_or("Missing SignatureCertChainUrl header")?;

    let signature_b64 = headers
        .get("Signature")
        .and_then(|v| v.to_str().ok())
        .ok_or("Missing Signature header")?;

    // Step 1: Validate certificate URL format (host/scheme/port/path).
    validate_alexa_cert_url(cert_url)?;

    // Step 2: Validate timestamp from request body (replay protection).
    validate_alexa_timestamp(body)?;

    // Decode the presented signature before doing any network work.
    let signature = BASE64
        .decode(signature_b64.trim().as_bytes())
        .map_err(|e| format!("Signature is not valid base64: {e}"))?;

    // Step 3: Fetch the certificate chain (validated URL only), with caching.
    let cert_pem = fetch_alexa_cert_chain(cert_url).await?;

    // Steps 4 & 5: leaf validity + SAN + RSA-SHA1 body signature.
    verify_alexa_cert_and_signature(&cert_pem, &signature, body, Utc::now())?;

    tracing::info!(cert_url = %cert_url, "Alexa request signature verified");
    Ok(())
}

/// TTL for cached Alexa signing certificates. Amazon rotates the `echo.api`
/// certificate infrequently; caching by URL for an hour avoids re-fetching on
/// every request without pinning a stale cert for long.
const ALEXA_CERT_CACHE_TTL: std::time::Duration = std::time::Duration::from_secs(3600);

/// Process-local cache of fetched certificate-chain PEM bytes keyed by URL.
#[allow(clippy::type_complexity)]
static ALEXA_CERT_CACHE: std::sync::OnceLock<
    std::sync::Mutex<
        std::collections::HashMap<String, (std::time::Instant, std::sync::Arc<Vec<u8>>)>,
    >,
> = std::sync::OnceLock::new();

/// Time budget for establishing the TCP+TLS connection to S3 when fetching the
/// Alexa signing certificate.
const ALEXA_CERT_CONNECT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

/// Overall time budget for a single certificate fetch (connect + response
/// body). Bounds how long a slow S3 can stall the webhook handler.
const ALEXA_CERT_REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

/// Shared HTTP client for Alexa certificate fetches.
///
/// Built once (process-wide) with explicit connect and request timeouts and
/// reused for every fetch. Reusing a single client keeps the connection pool
/// and TLS session cache warm and, crucially, prevents a burst of cold-cache
/// requests from spawning an unbounded number of independent TLS clients.
static ALEXA_CERT_HTTP_CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();

/// Return the shared, timeout-bounded HTTP client used to fetch Alexa signing
/// certificates, initialising it on first use.
fn alexa_cert_http_client() -> &'static reqwest::Client {
    ALEXA_CERT_HTTP_CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .connect_timeout(ALEXA_CERT_CONNECT_TIMEOUT)
            .timeout(ALEXA_CERT_REQUEST_TIMEOUT)
            .build()
            // A builder failure means the TLS backend could not initialise; a
            // default client would fail the same way, so fall back to it rather
            // than panicking inside a request handler.
            .unwrap_or_else(|_| reqwest::Client::new())
    })
}

/// Fetch the PEM certificate chain from a (previously validated) Alexa cert
/// URL, memoising the result for [`ALEXA_CERT_CACHE_TTL`].
///
/// The URL is only ever reached after [`validate_alexa_cert_url`], so it always
/// points at `https://s3.amazonaws.com/echo.api/…`; TLS authenticates the S3
/// origin. Returns the raw PEM bytes (one or more concatenated certificates).
///
/// The fetch goes through a shared client ([`alexa_cert_http_client`]) with
/// explicit connect/request timeouts so a slow or unresponsive S3 cannot stall
/// the handler indefinitely.
async fn fetch_alexa_cert_chain(url: &str) -> Result<std::sync::Arc<Vec<u8>>, String> {
    let cache =
        ALEXA_CERT_CACHE.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()));

    // Fast path: a fresh cache entry.
    if let Ok(guard) = cache.lock() {
        if let Some((fetched_at, bytes)) = guard.get(url) {
            if fetched_at.elapsed() < ALEXA_CERT_CACHE_TTL {
                return Ok(bytes.clone());
            }
        }
    }

    let response = alexa_cert_http_client()
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch Alexa certificate: {e}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "Alexa certificate fetch returned HTTP {}",
            response.status()
        ));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read Alexa certificate body: {e}"))?
        .to_vec();
    let bytes = std::sync::Arc::new(bytes);

    if let Ok(mut guard) = cache.lock() {
        guard.insert(url.to_string(), (std::time::Instant::now(), bytes.clone()));
    }

    Ok(bytes)
}

/// Validate the Alexa signing certificate and verify the request-body signature.
///
/// Pure (no I/O): takes the fetched PEM chain, the decoded signature bytes, the
/// raw request body, and the current time, so the security-critical logic is
/// unit-testable without a network or a real Amazon certificate. Splitting the
/// fetch out of this function is what lets the regression tests exercise the
/// forged-signature path against a locally generated certificate.
///
/// Checks, in order:
/// 1. The leaf (first) certificate parses.
/// 2. `now` is within the leaf's `notBefore..=notAfter` window.
/// 3. The leaf's Subject Alternative Names include `echo-api.amazon.com`.
/// 4. `signature` is a valid PKCS#1 v1.5 RSA-SHA1 signature of `body` under the
///    leaf's public key.
fn verify_alexa_cert_and_signature(
    cert_pem: &[u8],
    signature: &[u8],
    body: &[u8],
    now: DateTime<Utc>,
) -> Result<(), String> {
    use rsa::pkcs8::DecodePublicKey;
    use rsa::{Pkcs1v15Sign, RsaPublicKey};
    use sha1::{Digest, Sha1};
    use x509_cert::der::{Decode, DecodePem, Encode};
    use x509_cert::ext::pkix::{name::GeneralName, SubjectAltName};
    use x509_cert::Certificate;

    // (1) Parse the leaf certificate (first PEM block in the chain).
    let cert = Certificate::from_pem(cert_pem)
        .map_err(|e| format!("Failed to parse Alexa certificate: {e}"))?;
    let tbs = &cert.tbs_certificate;

    // (2) Validity window.
    let now_s = now.timestamp();
    if now_s < 0 {
        return Err("Current time is before the Unix epoch".to_string());
    }
    let now_s = now_s as u64;
    let not_before = tbs.validity.not_before.to_unix_duration().as_secs();
    let not_after = tbs.validity.not_after.to_unix_duration().as_secs();
    if now_s < not_before {
        return Err("Alexa certificate is not yet valid".to_string());
    }
    if now_s > not_after {
        return Err("Alexa certificate has expired".to_string());
    }

    // (3) Subject Alternative Name must include echo-api.amazon.com.
    // OID 2.5.29.17 = id-ce-subjectAltName.
    let mut san_ok = false;
    if let Some(extensions) = tbs.extensions.as_ref() {
        for ext in extensions.iter() {
            if ext.extn_id.to_string() != "2.5.29.17" {
                continue;
            }
            let san = SubjectAltName::from_der(ext.extn_value.as_bytes())
                .map_err(|e| format!("Failed to parse certificate SAN: {e}"))?;
            for general_name in san.0.iter() {
                if let GeneralName::DnsName(dns) = general_name {
                    if dns.as_str().eq_ignore_ascii_case("echo-api.amazon.com") {
                        san_ok = true;
                        break;
                    }
                }
            }
            if san_ok {
                break;
            }
        }
    }
    if !san_ok {
        return Err("Alexa certificate SAN does not include echo-api.amazon.com".to_string());
    }

    // (4) Verify the RSA-SHA1 signature over the raw request body.
    let spki_der = tbs
        .subject_public_key_info
        .to_der()
        .map_err(|e| format!("Failed to encode certificate public key: {e}"))?;
    let public_key = RsaPublicKey::from_public_key_der(&spki_der)
        .map_err(|e| format!("Certificate public key is not RSA: {e}"))?;

    let digest = Sha1::digest(body);
    public_key
        .verify(Pkcs1v15Sign::new::<Sha1>(), &digest, signature)
        .map_err(|_| "Alexa request signature does not match request body".to_string())?;

    Ok(())
}

/// Validate Alexa certificate URL format.
/// The URL must:
/// - Use HTTPS protocol
/// - Use host s3.amazonaws.com with path /echo.api/
/// - Use port 443 (default)
fn validate_alexa_cert_url(url: &str) -> Result<(), String> {
    let parsed = reqwest::Url::parse(url).map_err(|e| format!("Invalid certificate URL: {}", e))?;

    // Check protocol
    if parsed.scheme() != "https" {
        return Err("Certificate URL must use HTTPS".to_string());
    }

    // Check host
    let host = parsed.host_str().ok_or("Certificate URL missing host")?;

    if host != "s3.amazonaws.com" {
        return Err(format!(
            "Certificate URL host must be s3.amazonaws.com, got: {}",
            host
        ));
    }

    // Check port (must be 443 or default)
    if let Some(port) = parsed.port() {
        if port != 443 {
            return Err(format!("Certificate URL must use port 443, got: {}", port));
        }
    }

    // Check path starts with /echo.api/
    let path = parsed.path();
    if !path.starts_with("/echo.api/") {
        return Err(format!(
            "Certificate URL path must start with /echo.api/, got: {}",
            path
        ));
    }

    Ok(())
}

/// Validate Alexa request timestamp.
/// The timestamp must be within 150 seconds of current time.
fn validate_alexa_timestamp(body: &[u8]) -> Result<(), String> {
    // Parse the body to extract timestamp
    #[derive(Deserialize)]
    struct AlexaRequest {
        request: AlexaRequestTimestamp,
    }

    #[derive(Deserialize)]
    struct AlexaRequestTimestamp {
        timestamp: String,
    }

    let request: AlexaRequest = serde_json::from_slice(body)
        .map_err(|e| format!("Failed to parse Alexa request: {}", e))?;

    // Parse timestamp (ISO 8601 format)
    let timestamp = chrono::DateTime::parse_from_rfc3339(&request.request.timestamp)
        .map_err(|e| format!("Invalid timestamp format: {}", e))?;

    let now = Utc::now();
    let diff = now.signed_duration_since(timestamp);

    // Must be within 150 seconds (past or future)
    if diff.num_seconds().abs() > 150 {
        return Err(format!(
            "Request timestamp too old or too new: {} seconds difference",
            diff.num_seconds()
        ));
    }

    Ok(())
}

// ============================================================================
// Google Actions ID-token verification
// ============================================================================

/// Google's OAuth2 JWKS endpoint — RS256 signing certificates for Google-issued
/// ID tokens (used by Google Assistant / Actions account linking).
const GOOGLE_CERTS_URL: &str = "https://www.googleapis.com/oauth2/v3/certs";

/// Accepted `iss` values for a Google-issued ID token.
const GOOGLE_ISSUERS: [&str; 2] = ["https://accounts.google.com", "accounts.google.com"];

/// How long a fetched JWKS is trusted before a refresh. Google rotates signing
/// keys on the order of days; a 1h TTL keeps the hot path off the network while
/// still picking up rotations promptly (a rotated-in `kid` also forces an
/// immediate refresh regardless of TTL — see `google_decoding_key`).
const GOOGLE_JWKS_TTL: std::time::Duration = std::time::Duration::from_secs(3600);

/// One JWK entry from Google's `/oauth2/v3/certs` response (RSA public key).
#[derive(Clone, Deserialize)]
struct GoogleJwk {
    kid: String,
    n: String,
    e: String,
    #[allow(dead_code)]
    alg: Option<String>,
}

#[derive(Clone, Deserialize)]
struct GoogleJwks {
    keys: Vec<GoogleJwk>,
}

struct JwksCache {
    keys: Vec<GoogleJwk>,
    fetched_at: std::time::Instant,
}

/// Process-wide JWKS cache (1h TTL) and shared HTTP client (5s timeout).
static GOOGLE_JWKS_CACHE: OnceLock<RwLock<Option<JwksCache>>> = OnceLock::new();
static GOOGLE_JWKS_HTTP: OnceLock<reqwest::Client> = OnceLock::new();

fn google_jwks_cache() -> &'static RwLock<Option<JwksCache>> {
    GOOGLE_JWKS_CACHE.get_or_init(|| RwLock::new(None))
}

fn google_jwks_http() -> &'static reqwest::Client {
    GOOGLE_JWKS_HTTP.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            // Mirror `alexa_cert_http_client`: a builder failure means the TLS
            // backend could not initialise, and a default client would fail the
            // same way — fall back to it rather than panicking inside the
            // request-serving `verify_google_request` path.
            .unwrap_or_else(|_| reqwest::Client::new())
    })
}

/// Fetch and parse Google's current JWKS. `error_for_status()` is deliberate:
/// a 4xx/5xx JSON error body would otherwise parse into an empty `keys[]` and
/// surface later as a confusing "unknown kid" instead of the real HTTP failure.
async fn fetch_google_jwks() -> Result<Vec<GoogleJwk>, String> {
    let resp = google_jwks_http()
        .get(GOOGLE_CERTS_URL)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch Google signing certs: {e}"))?
        .error_for_status()
        .map_err(|e| format!("Google signing certs endpoint returned an error: {e}"))?;
    let jwks: GoogleJwks = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse Google signing certs: {e}"))?;
    Ok(jwks.keys)
}

/// Resolve the RSA public [`DecodingKey`] for a `kid`, using the cached JWKS
/// when fresh and it contains the key, otherwise refreshing from Google.
/// Mirrors the JWKS resolver in `deploy-server`'s OIDC validator.
async fn google_decoding_key(kid: &str) -> Result<DecodingKey, String> {
    // Fast path: cached, fresh, and contains the requested kid.
    {
        let guard = google_jwks_cache().read().await;
        if let Some(cache) = guard.as_ref() {
            if cache.fetched_at.elapsed() < GOOGLE_JWKS_TTL {
                if let Some(k) = cache.keys.iter().find(|k| k.kid == kid) {
                    return DecodingKey::from_rsa_components(&k.n, &k.e)
                        .map_err(|e| format!("Invalid Google JWK for kid {kid}: {e}"));
                }
            }
        }
    }

    // Refresh: stale cache OR the kid was rotated in and isn't cached yet.
    let fresh = fetch_google_jwks().await?;
    let key = fresh
        .iter()
        .find(|k| k.kid == kid)
        .ok_or_else(|| format!("Unknown Google signing key id: {kid}"))
        .and_then(|k| {
            DecodingKey::from_rsa_components(&k.n, &k.e)
                .map_err(|e| format!("Invalid Google JWK for kid {kid}: {e}"))
        })?;
    *google_jwks_cache().write().await = Some(JwksCache {
        keys: fresh,
        fetched_at: std::time::Instant::now(),
    });
    Ok(key)
}

/// Verify a Google-issued ID token's RS256 signature and registered claims
/// against `key`. Pure and synchronous so it is unit-testable without a live
/// JWKS fetch. FAILS CLOSED: any bad signature, wrong audience (`aud` must
/// equal `expected_project` exactly), untrusted issuer, or expired/not-yet-valid
/// token yields `Err`.
fn verify_google_id_token(
    token: &str,
    expected_project: &str,
    key: &DecodingKey,
) -> Result<(), String> {
    let mut validation = Validation::new(Algorithm::RS256);
    // Exact audience match against the configured Actions project id.
    validation.set_audience(&[expected_project]);
    validation.set_issuer(&GOOGLE_ISSUERS);
    validation.validate_exp = true;
    validation.validate_nbf = true;

    // jsonwebtoken base64url-decodes the segments itself (no STANDARD-alphabet
    // mis-decode) and enforces signature + aud + iss + exp/nbf.
    decode::<serde_json::Value>(token, key, &validation)
        .map(|_| ())
        .map_err(|e| format!("Google ID token verification failed: {e}"))
}

/// Verify a Google Actions request (Epic 93 / Story 93.3).
///
/// The request MUST carry a Google-issued ID token in the `Authorization:
/// Bearer <jwt>` header. The token's RS256 signature is verified against
/// Google's published JWKS (`/oauth2/v3/certs`, cached 1h), and its `aud`
/// claim must equal `GOOGLE_ACTIONS_PROJECT_ID`, its `iss` must be a Google
/// issuer, and it must be unexpired.
///
/// FAILS CLOSED on every path: an unset `GOOGLE_ACTIONS_PROJECT_ID`, a missing
/// or malformed `Authorization` header, or any signature/claim failure all
/// return `Err` (mapped by the caller to `401`).
pub(crate) async fn verify_google_request(headers: &HeaderMap) -> Result<(), String> {
    // FAIL CLOSED: the audience we verify against must be configured.
    let expected_project = std::env::var("GOOGLE_ACTIONS_PROJECT_ID")
        .map_err(|_| "GOOGLE_ACTIONS_PROJECT_ID is not configured".to_string())?;

    // FAIL CLOSED: a Google-signed Bearer ID token is required.
    let auth_header = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| "Missing Authorization header".to_string())?;
    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or_else(|| "Invalid Authorization header format".to_string())?;

    // Fail fast — and off the network — on a structurally invalid token.
    let header =
        decode_header(token).map_err(|e| format!("Invalid Google ID token header: {e}"))?;
    if header.alg != Algorithm::RS256 {
        return Err(format!(
            "Unexpected Google ID token algorithm: {:?}",
            header.alg
        ));
    }
    let kid = header
        .kid
        .ok_or_else(|| "Google ID token missing key id (kid)".to_string())?;

    let key = google_decoding_key(&kid).await?;
    verify_google_id_token(token, &expected_project, &key)?;

    tracing::info!(project_id = %expected_project, "Google Actions ID token verified");
    Ok(())
}

// ============================================================================
// HMAC-SHA256 shared-secret verification
// ============================================================================

/// Verify HMAC-SHA256 signature.
///
/// Issue #2658 (#2): fails **closed** when `VOICE_WEBHOOK_SECRET` is unset
/// **or empty**. The previous `unwrap_or("default_secret")` verified against a
/// literal present in source, letting anyone forge signatures; the sibling
/// receivers (`PORTAL_WEBHOOK_SECRET`, `AIRBNB_WEBHOOK_SECRET`,
/// `STRIPE_WEBHOOK_SECRET`) read the secret with `unwrap_or_default()` and then
/// reject a blank value as "not configured" (see `routes/integrations/webhook.rs`),
/// so an empty env var can never key the HMAC. We mirror that here: unset and
/// empty both fail closed.
pub(crate) fn verify_hmac_signature(signature: &str, body: &str) -> Result<bool, String> {
    let secret = std::env::var("VOICE_WEBHOOK_SECRET").unwrap_or_default();
    if secret.is_empty() {
        return Err("VOICE_WEBHOOK_SECRET is not configured".to_string());
    }

    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|e| format!("Invalid HMAC key: {}", e))?;

    mac.update(body.as_bytes());

    let expected = BASE64.encode(mac.finalize().into_bytes());

    // Constant-time compare (issue #2658 #3) instead of `signature == expected`.
    Ok(ct_eq_str(signature, &expected))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    fn alexa_body_with_timestamp(ts: &str) -> Vec<u8> {
        format!(r#"{{"request":{{"timestamp":"{ts}"}}}}"#).into_bytes()
    }

    // --- validate_alexa_cert_url ----------------------------------------

    #[test]
    fn cert_url_accepts_canonical_amazon_url() {
        assert!(
            validate_alexa_cert_url("https://s3.amazonaws.com/echo.api/echo-api-cert-12.pem")
                .is_ok()
        );
    }

    #[test]
    fn cert_url_accepts_explicit_port_443() {
        assert!(validate_alexa_cert_url("https://s3.amazonaws.com:443/echo.api/cert.pem").is_ok());
    }

    #[test]
    fn cert_url_rejects_non_https_scheme() {
        let err = validate_alexa_cert_url("http://s3.amazonaws.com/echo.api/cert.pem").unwrap_err();
        assert!(err.contains("HTTPS"), "unexpected error: {err}");
    }

    #[test]
    fn cert_url_rejects_wrong_host() {
        let err =
            validate_alexa_cert_url("https://evil.example.com/echo.api/cert.pem").unwrap_err();
        assert!(err.contains("s3.amazonaws.com"), "unexpected error: {err}");
    }

    #[test]
    fn cert_url_rejects_non_443_port() {
        let err =
            validate_alexa_cert_url("https://s3.amazonaws.com:8443/echo.api/cert.pem").unwrap_err();
        assert!(err.contains("443"), "unexpected error: {err}");
    }

    #[test]
    fn cert_url_rejects_wrong_path_prefix() {
        let err = validate_alexa_cert_url("https://s3.amazonaws.com/evil/cert.pem").unwrap_err();
        assert!(err.contains("/echo.api/"), "unexpected error: {err}");
    }

    #[test]
    fn cert_url_rejects_unparseable_url() {
        assert!(validate_alexa_cert_url("not a url").is_err());
    }

    // --- validate_alexa_timestamp ---------------------------------------

    #[test]
    fn timestamp_accepts_now() {
        let body = alexa_body_with_timestamp(&Utc::now().to_rfc3339());
        assert!(validate_alexa_timestamp(&body).is_ok());
    }

    #[test]
    fn timestamp_rejects_too_old() {
        let old = (Utc::now() - chrono::Duration::seconds(600)).to_rfc3339();
        let body = alexa_body_with_timestamp(&old);
        let err = validate_alexa_timestamp(&body).unwrap_err();
        assert!(err.contains("too old or too new"), "unexpected: {err}");
    }

    #[test]
    fn timestamp_rejects_far_future() {
        let future = (Utc::now() + chrono::Duration::seconds(600)).to_rfc3339();
        let body = alexa_body_with_timestamp(&future);
        assert!(validate_alexa_timestamp(&body).is_err());
    }

    #[test]
    fn timestamp_rejects_bad_format() {
        let body = alexa_body_with_timestamp("not-a-timestamp");
        let err = validate_alexa_timestamp(&body).unwrap_err();
        assert!(
            err.contains("Invalid timestamp format"),
            "unexpected: {err}"
        );
    }

    #[test]
    fn timestamp_rejects_unparseable_json() {
        let err = validate_alexa_timestamp(b"{ not json").unwrap_err();
        assert!(err.contains("Failed to parse"), "unexpected: {err}");
    }

    // --- verify_alexa_signature -----------------------------------------

    #[tokio::test]
    async fn alexa_signature_rejects_missing_cert_header() {
        let mut headers = HeaderMap::new();
        headers.insert("signature", HeaderValue::from_static("sig"));
        let body = alexa_body_with_timestamp(&Utc::now().to_rfc3339());
        let err = verify_alexa_signature(&headers, &body).await.unwrap_err();
        assert!(err.contains("SignatureCertChainUrl"), "unexpected: {err}");
    }

    #[tokio::test]
    async fn alexa_signature_rejects_missing_signature_header() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "signaturecertchainurl",
            HeaderValue::from_static("https://s3.amazonaws.com/echo.api/cert.pem"),
        );
        let body = alexa_body_with_timestamp(&Utc::now().to_rfc3339());
        let err = verify_alexa_signature(&headers, &body).await.unwrap_err();
        assert!(err.contains("Signature header"), "unexpected: {err}");
    }

    #[tokio::test]
    async fn alexa_signature_rejects_bad_cert_url() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "signaturecertchainurl",
            HeaderValue::from_static("https://evil.example.com/echo.api/cert.pem"),
        );
        headers.insert("signature", HeaderValue::from_static("sig"));
        let body = alexa_body_with_timestamp(&Utc::now().to_rfc3339());
        assert!(verify_alexa_signature(&headers, &body).await.is_err());
    }

    // The Alexa cert fetch must go through a single shared, timeout-bounded
    // client rather than building a fresh `reqwest::Client` per call — a burst
    // of cold-cache requests would otherwise spawn unbounded TLS clients.
    // Guards against a regression back to `reqwest::Client::new()` per fetch.
    #[test]
    fn alexa_cert_http_client_is_shared() {
        let first = alexa_cert_http_client();
        let second = alexa_cert_http_client();
        assert!(
            std::ptr::eq(first, second),
            "expected a single process-wide Alexa cert HTTP client"
        );
    }

    // Sanity-check the fetch timeouts are ordered sensibly (connect budget is a
    // subset of the overall request budget) and are actually bounded.
    #[test]
    fn alexa_cert_timeouts_are_bounded() {
        assert!(ALEXA_CERT_CONNECT_TIMEOUT <= ALEXA_CERT_REQUEST_TIMEOUT);
        assert!(!ALEXA_CERT_REQUEST_TIMEOUT.is_zero());
    }

    // Regression for issue #2668: a valid cert URL + fresh timestamp is NO
    // LONGER sufficient. The old code returned Ok() here without ever checking
    // the signature; the request-body signature verification now lives in
    // `verify_alexa_cert_and_signature`, exercised end-to-end below against a
    // locally generated certificate. (The `verify_alexa_signature` wrapper adds
    // a live cert fetch over the network, so its happy path is covered by an
    // integration harness, not this unit test.)

    // A self-signed RSA-2048 certificate whose SAN is `echo-api.amazon.com`,
    // valid 2026..2126 (generated with `openssl req -x509 -sha1`), plus its
    // PKCS#8 private key — used to sign a body the way Alexa's edge would.
    const ALEXA_TEST_CERT_PEM: &str = "-----BEGIN CERTIFICATE-----\n\
MIIDPzCCAiegAwIBAgIUc05E5fmM11dnpbFxoswJSfvairYwDQYJKoZIhvcNAQEF\n\
BQAwHjEcMBoGA1UEAwwTZWNoby1hcGkuYW1hem9uLmNvbTAgFw0yNjA4MDYwODI3\n\
MDlaGA8yMTI2MDcxMzA4MjcwOVowHjEcMBoGA1UEAwwTZWNoby1hcGkuYW1hem9u\n\
LmNvbTCCASIwDQYJKoZIhvcNAQEBBQADggEPADCCAQoCggEBAJMYjkpium0M0ozF\n\
xb2Cb+9G2KtRxsN15YOYfUpp1yWdE08R+ZJbXqk3Q4bP7axRJtDiR/a9OlsvEcJS\n\
twELoCkforsFQyJ53d5UCzPdLj2iOhXpg0jtv0Gh3u7Fg27n/ugqEhHaJukSjCiX\n\
4lao8t1Wd9lkxLhOEUCP6D7a9NYmIfu5ut3MqmyKeOfAAVBOjaVhS374Zq3+0OnY\n\
wSimT4+9FoTZMfeVmyVzyByyR5C3CSG9+VZq3sG86AJdhZ786ko7Dlsi/XppTQe+\n\
CYiYWb+gPEVoN8ezopr15cgAMREUNUKHajQBHbD8FEHkTAPnZqT0wrxD3MLleuKp\n\
pc2j/wkCAwEAAaNzMHEwHQYDVR0OBBYEFKYUUOrdDzvE0xV9fZJ+kDY6csU2MB8G\n\
A1UdIwQYMBaAFKYUUOrdDzvE0xV9fZJ+kDY6csU2MA8GA1UdEwEB/wQFMAMBAf8w\n\
HgYDVR0RBBcwFYITZWNoby1hcGkuYW1hem9uLmNvbTANBgkqhkiG9w0BAQUFAAOC\n\
AQEADWxe+1VEnu98Q4Xb7egrV/DUkSZLEfM9H2gSqux3NYDzjRa0/7nivMQCZGSw\n\
IgRswmQDAV+djLUNQcmONqVgc8VkdHsqeZb5zgr6eEiNqHgIkwGnz41OS3oz05Qb\n\
AlFFJU8TsobjnOQ9//Qt/ECtlEBpmretbGld1dxJ0cNo4mpLtCBjXkb9uBFeiwCy\n\
yUjA/5nfAXELnHCTHNN1YFC1CNBQQk9bFHwQEAc9OgWvMIojbcoXbRwZpUOaQOlE\n\
I6tLJePCUV9CHXNgSRBEPWCbufVHtFtyJQisGBSNUMzVPMQHbBouYUOmvju7H/bc\n\
i+jshHpYblmQyitGXulqctcnWA==\n\
-----END CERTIFICATE-----\n";

    const ALEXA_TEST_KEY_PEM: &str = "-----BEGIN PRIVATE KEY-----\n\
MIIEvAIBADANBgkqhkiG9w0BAQEFAASCBKYwggSiAgEAAoIBAQCTGI5KYrptDNKM\n\
xcW9gm/vRtirUcbDdeWDmH1KadclnRNPEfmSW16pN0OGz+2sUSbQ4kf2vTpbLxHC\n\
UrcBC6ApH6K7BUMied3eVAsz3S49ojoV6YNI7b9Bod7uxYNu5/7oKhIR2ibpEowo\n\
l+JWqPLdVnfZZMS4ThFAj+g+2vTWJiH7ubrdzKpsinjnwAFQTo2lYUt++Gat/tDp\n\
2MEopk+PvRaE2TH3lZslc8gcskeQtwkhvflWat7BvOgCXYWe/OpKOw5bIv16aU0H\n\
vgmImFm/oDxFaDfHs6Ka9eXIADERFDVCh2o0AR2w/BRB5EwD52ak9MK8Q9zC5Xri\n\
qaXNo/8JAgMBAAECggEAAaTb/UHseGN07HYZkKBHM5GeAmR/SqgJX5lvaArLXjwg\n\
JGl3ObmmnZCfWHUVNdBxNRMxr59733Dvm2rXfli0tT+euYVJ6RYbLXzmDvy1OOLf\n\
OudhgbSHh0/MSSyhBwxV0DQygBYpNRW8G4g0CJjU4GCoUFWPcHRjPg6bBMMndUe9\n\
kWOB2kZ3o4oQkJCgQ0/93VnYYk2i7mWjrUNbpqhcLX2+GzzrecGnPbbF2528IKRL\n\
GAUn9VdkwvzP8fGWOpq9qa6iWw6kkteCoQLZoHR/hN1XaQhGN2HhqjKu43gxmm16\n\
eNa4nm+xfoe2f3eHHqkVZ+RkJHKfxMtO7HHBXKfcoQKBgQDHcmRYyo33erlu28gg\n\
vINI3I7D9IqRiByrtnlpnDADtEhorcJFkZXtJuV+14HxK5nBl042bY4hSS52Y3UL\n\
poW0ibpiOVUp46PvNs7grqTBew1WX5/Cf+zebbLChHOAJi6lfouWQaCwUmNn7qs7\n\
rh+lxtcOMb+dB/yDyhvq/nBn6QKBgQC8zhKsfpPgvgwgw104zk5NJ6Pyvnvn3TRq\n\
AB0YuBpgAYWRJMXomBurmB2wv7PfBduBEiXKKazHH7n3b74U+7lGhq+ydU7K/Nd2\n\
tgtC5klKo0B6cFtPZ+0URhJ7clrYobRcFnVTax/dhvNNah8J8vGvi90BTL/34GWe\n\
p6iFLSuKIQKBgFDuyG2HdGhycoDbyrAODzAn3/8AYqJ/mzLKzyXd7VXzeFaR+/2D\n\
AFXFrOb1yJL24GPAZEqN1lkHe0UrQrnBjwwdv3ZQUZC4ATP3B6gA9nZU2qqsDwY8\n\
JwBzf1CTstLTq6YYXchRRUWHiTMJlI6ZL9pzf50Q7vJn5T4Na5rGORLRAoGAfSnH\n\
q16GPfj/JUEeLahmtDNRNn0cuwsj0hmdMGPr6DVaDGxqXtVnkovXMvMDFRhW+evD\n\
7Y9PIPphWC1Vv6dYne5vz0iBIYQYenQYZxMvBzHObtzJS4zD2CrT2c5ndzFL1bh1\n\
swVTLJJn/KwbQ4cwvYVkz5XHtVWnSFQxHYhiUsECgYBTJ6GXGXWyVgCCZMEsJ52z\n\
dsIyiBSr+2cG/1bvncg2zhl45m6krGTplUO9p4K8sSwmrQxGe13nbtbijNeLuQ0D\n\
FSMLvIzM5qOlblU6uGMSaCW3G6n4FKJo/Vqm8frVJP7wvywYy0AA32smETJyqFN0\n\
dQPD++LeIpOChiX4Ru2uMQ==\n\
-----END PRIVATE KEY-----\n";

    // Sign `body` exactly as Alexa's edge does: PKCS#1 v1.5 RSA over SHA-1(body).
    fn alexa_sign(body: &[u8]) -> Vec<u8> {
        use rsa::pkcs8::DecodePrivateKey;
        use rsa::{Pkcs1v15Sign, RsaPrivateKey};
        use sha1::{Digest, Sha1};
        let key = RsaPrivateKey::from_pkcs8_pem(ALEXA_TEST_KEY_PEM).expect("load test key");
        let digest = Sha1::digest(body);
        key.sign(Pkcs1v15Sign::new::<Sha1>(), &digest)
            .expect("sign body")
    }

    #[test]
    fn alexa_cert_and_signature_accepts_valid_signature() {
        let body = br#"{"request":{"type":"LaunchRequest"}}"#;
        let sig = alexa_sign(body);
        assert!(
            verify_alexa_cert_and_signature(ALEXA_TEST_CERT_PEM.as_bytes(), &sig, body, Utc::now())
                .is_ok(),
            "a body signed with the cert's key must verify"
        );
    }

    #[test]
    fn alexa_cert_and_signature_rejects_forged_signature() {
        // Attacker presents a body they control with a bogus signature — the
        // exact forgery the old code accepted. Must be rejected.
        let body = br#"{"request":{"type":"IntentRequest","intent":{"name":"evil"}}}"#;
        let forged = vec![0u8; 256]; // right length, wrong signature
        let err = verify_alexa_cert_and_signature(
            ALEXA_TEST_CERT_PEM.as_bytes(),
            &forged,
            body,
            Utc::now(),
        )
        .unwrap_err();
        assert!(err.contains("does not match"), "unexpected: {err}");
    }

    #[test]
    fn alexa_cert_and_signature_rejects_tampered_body() {
        // Valid signature for one body must not authenticate a different body.
        let signed_body = br#"{"request":{"type":"LaunchRequest"}}"#;
        let sig = alexa_sign(signed_body);
        let tampered_body = br#"{"request":{"type":"LaunchRequest"} }"#; // one byte differs
        let err = verify_alexa_cert_and_signature(
            ALEXA_TEST_CERT_PEM.as_bytes(),
            &sig,
            tampered_body,
            Utc::now(),
        )
        .unwrap_err();
        assert!(err.contains("does not match"), "unexpected: {err}");
    }

    #[test]
    fn alexa_cert_and_signature_rejects_expired_certificate() {
        // The test cert is valid 2026..2126; evaluate it far in the future.
        let body = br#"{"request":{"type":"LaunchRequest"}}"#;
        let sig = alexa_sign(body);
        let far_future = Utc::now() + chrono::Duration::days(365 * 200);
        let err =
            verify_alexa_cert_and_signature(ALEXA_TEST_CERT_PEM.as_bytes(), &sig, body, far_future)
                .unwrap_err();
        assert!(err.contains("expired"), "unexpected: {err}");
    }

    #[test]
    fn alexa_cert_and_signature_rejects_unparseable_cert() {
        let body = br#"{"request":{"type":"LaunchRequest"}}"#;
        let sig = alexa_sign(body);
        assert!(verify_alexa_cert_and_signature(
            b"-----BEGIN CERTIFICATE-----\nnope\n",
            &sig,
            body,
            Utc::now()
        )
        .is_err());
    }

    // --- verify_google_request (fail-closed JWT verification) -----------
    //
    // Regression coverage for the fail-open bug: `verify_google_request` used
    // to return Ok(()) for an unset project id, a missing auth header, and a
    // forged bearer whose base64 payload merely *contained* the project id
    // (with no signature check), and it decoded JWT payloads with the STANDARD
    // (padded) base64 alphabet so real URL-safe Google tokens silently fell
    // through to Ok(()). It now verifies the RS256 signature + aud/iss/exp and
    // fails closed on every error path.

    // Fixed RSA test keypair (2048-bit) used to mint + verify signed tokens in
    // these unit tests. NOT a real Google key — the production path resolves the
    // decoding key from Google's live JWKS via `google_decoding_key`.
    const TEST_RSA_PRIV_PEM: &str = "-----BEGIN PRIVATE KEY-----
MIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQC7p2rNa9yZ9llX
ZmRH2tWIayNP4yIdGP72CzZx8/YxOx03BbqKTNmcfpShjw9AuaWENi0sWXe9j1TX
RR/JZ1Cff2qDHwsFfdwAwWraFtKFrFAziLG9yZXicxxY9fotLHOzEGwnPqZo4JqK
APqVW/0bZzQLIpYALCM7Snj/DA2axsrlHH1Usi5UqEjaXQUpagwGRVpyR6eS6/Sg
u/BkU50/xMZCyyv1/0ebeDKxOy3I5j+hxLW/reJZaNIOHEzZwXmARjah8b0L0/yP
C6h5Y1Xg7Rn53qBw+DI03Z2HEwFf9cldO15X0B9XQiqJ+ZdGi2Jj6eq3oOBp812w
1SIzp+V/AgMBAAECggEAAQP5l/2qxp/cAT+/Rdmb/jkA3+siwXWOgQEJMmSs7byc
KTK8El4yxJ4Kv9++UriuefYGbeRYuYs6XPqKyX7oTh9VEZCWxq4qWqFcAAIk8YQ/
4DIv2WRrOJEsPhmsA5fnUw4WXRVXC9+V9oPlge7ALT3JvPsFmh/4W4HJAIMC2oDm
iB1fsl0jq+rk+k0JYIy11yRcI6Dz6CxMzrZtkl6h66a9LXUpwfqVSPUwuHilXCwp
PpwNGja/Qi+91DFAM3AjAHy85uSzg+wV2thCKvS+OF45z2MuElXnWDbQKinW6ige
tl8lbvQkIvDTpPAecSokYsV1ZI7LX5gqT4GlO1ImQQKBgQDmgDEsdBuIJP1lJarL
NeL2/t9n9Nhs7FoxPy+qKy8+7F6W98wJzrXXCxcK6NhKSw6RmPunrFx3ha3i6aPp
hu/855ihB0ZTkvNHQxCKmHPDW0wrrN6u77ANMYQa+Uwbs/DDbe/5Bj24ECcYqwHk
7womofS/DewMsabouuk+ymxmWwKBgQDQacynrub8CjTUaCyvLqcKjIv/+Mv0i3j+
zO1Fpb8bO9NM/FCKtnB3oddIRINAQRNJ3nd1V8HrHZhGxOTbIIeLi2RmtmJ0kjBE
gVAKPDAqvhaybfgnRKsFpl0WDzeu0UmwaC2zrsBLEXpoLRfnT2JHnA4THBj3h4yG
hVoIP1VOrQKBgQDJx7DEZIPxg8gbcoT4TZz5chbqb0nC2IkAEXtNcW5znAIWEKia
cU14CepLD5jAOMJxLMYoe1ea/fhB6xwlg421DJztYmvrH3o+iPQDEABPJS4iEbwC
0iqA8jbeUhyRJ819l1D6477F0cYX7yPCYIu3VBHn6m0Yk7A0jeM/p36LfwKBgFhf
5KZeJhhOA6TmH7yRHcf9XQhH6cRiuAXjw+E6rVTRA4Kro0OOpRY1jGJamwVOEu3J
5gHeGp6mSAIKT7kTjCaCDyr2v70KmGkUJGqSpyIYxOsYcpfEKHkW2HYYMdZxbLvf
ETIWMfgjCzLNnEs7gEM5S0aTLYsY8V/BgDHrGTNpAoGBANeF2qEjS1NyJ99mJcGj
Y5TPLyQot9d9hEkIpNT2aAFnUb0baqJ0j1eyN6NvOCUAdXcqskj4S5G5kIJEHQxP
mR3t5mHnqglfXy8CSVtCuIuBxzPf32fQq8XZo0JtE6oKfbd0jnsAX61NYhnF7LDO
mFM80HjvdsbTWHj9n2QgMU5B
-----END PRIVATE KEY-----";
    const TEST_RSA_PUB_PEM: &str = "-----BEGIN PUBLIC KEY-----
MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAu6dqzWvcmfZZV2ZkR9rV
iGsjT+MiHRj+9gs2cfP2MTsdNwW6ikzZnH6UoY8PQLmlhDYtLFl3vY9U10UfyWdQ
n39qgx8LBX3cAMFq2hbShaxQM4ixvcmV4nMcWPX6LSxzsxBsJz6maOCaigD6lVv9
G2c0CyKWACwjO0p4/wwNmsbK5Rx9VLIuVKhI2l0FKWoMBkVackenkuv0oLvwZFOd
P8TGQssr9f9Hm3gysTstyOY/ocS1v63iWWjSDhxM2cF5gEY2ofG9C9P8jwuoeWNV
4O0Z+d6gcPgyNN2dhxMBX/XJXTteV9AfV0IqifmXRotiY+nqt6DgafNdsNUiM6fl
fwIDAQAB
-----END PUBLIC KEY-----";

    const GOOGLE_ISS: &str = "https://accounts.google.com";

    fn test_decoding_key() -> DecodingKey {
        DecodingKey::from_rsa_pem(TEST_RSA_PUB_PEM.as_bytes()).unwrap()
    }

    // Mint an RS256-signed JWT with the test private key. `exp_offset` is
    // seconds relative to now (negative => already expired).
    fn signed_google_token(iss: &str, aud: &str, exp_offset: i64) -> String {
        use jsonwebtoken::{encode, EncodingKey, Header};
        let now = Utc::now().timestamp();
        let claims = serde_json::json!({
            "iss": iss,
            "aud": aud,
            "sub": "1234567890",
            "iat": now,
            "exp": now + exp_offset,
        });
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some("test-kid".to_string());
        let key = EncodingKey::from_rsa_pem(TEST_RSA_PRIV_PEM.as_bytes()).unwrap();
        encode(&header, &claims, &key).unwrap()
    }

    // A properly-signed, correctly-audienced, unexpired Google ID token is
    // accepted. Also proves URL-safe base64url JWT segments decode for real
    // (the old STANDARD-alphabet decode silently failed and fell through).
    #[test]
    fn google_id_token_valid_signature_accepted() {
        let token = signed_google_token(GOOGLE_ISS, "my-project-123", 3600);
        assert!(verify_google_id_token(&token, "my-project-123", &test_decoding_key()).is_ok());
    }

    // A forged bearer whose payload merely *contains* the project id but whose
    // signature does not verify is rejected (the core of the old fail-open bug).
    #[test]
    fn google_id_token_forged_bearer_rejected() {
        let good = signed_google_token(GOOGLE_ISS, "my-project-123", 3600);
        let parts: Vec<&str> = good.split('.').collect();
        // Keep the real header + the payload (still carries aud=my-project-123),
        // but replace the signature with garbage.
        let forged = format!("{}.{}.AAAAAAAAAAAAAAAA", parts[0], parts[1]);
        let err = verify_google_id_token(&forged, "my-project-123", &test_decoding_key())
            .expect_err("forged signature must be rejected");
        assert!(err.contains("verification failed"), "unexpected: {err}");
    }

    // Signature is valid but the audience is a different project => rejected
    // (exact project-id match, not a substring contains()).
    #[test]
    fn google_id_token_wrong_project_rejected() {
        let token = signed_google_token(GOOGLE_ISS, "someone-elses-project", 3600);
        assert!(verify_google_id_token(&token, "my-project-123", &test_decoding_key()).is_err());
    }

    // Signature + audience valid but the token is expired => rejected.
    #[test]
    fn google_id_token_expired_rejected() {
        let token = signed_google_token(GOOGLE_ISS, "my-project-123", -3600);
        assert!(verify_google_id_token(&token, "my-project-123", &test_decoding_key()).is_err());
    }

    // Signature + audience valid but issued by an untrusted issuer => rejected.
    #[test]
    fn google_id_token_untrusted_issuer_rejected() {
        let token = signed_google_token("https://evil.example.com", "my-project-123", 3600);
        assert!(verify_google_id_token(&token, "my-project-123", &test_decoding_key()).is_err());
    }

    // GOOGLE_ACTIONS_PROJECT_ID is process-global and read only by
    // `verify_google_request`; confining every mutation of it to this single
    // test keeps the assertions race-free under the parallel test runner. All
    // branches here fail closed *before* any JWKS network fetch.
    #[tokio::test]
    async fn google_request_fails_closed_on_bad_input() {
        // Unset project id => fail closed (previously returned Ok).
        std::env::remove_var("GOOGLE_ACTIONS_PROJECT_ID");
        let err = verify_google_request(&HeaderMap::new())
            .await
            .expect_err("must fail closed when project id unset");
        assert!(err.contains("not configured"), "unexpected: {err}");

        std::env::set_var("GOOGLE_ACTIONS_PROJECT_ID", "my-project-123");

        // Missing Authorization header => rejected (previously returned Ok).
        let err = verify_google_request(&HeaderMap::new())
            .await
            .expect_err("missing auth header must be rejected");
        assert!(err.contains("Missing Authorization"), "unexpected: {err}");

        // Malformed Authorization (no "Bearer " prefix) => rejected.
        let mut bad_scheme = HeaderMap::new();
        bad_scheme.insert("authorization", HeaderValue::from_static("Basic abc"));
        let err = verify_google_request(&bad_scheme)
            .await
            .expect_err("non-bearer scheme must be rejected");
        assert!(
            err.contains("Authorization header format"),
            "unexpected: {err}"
        );

        // Structurally invalid bearer token => rejected before any network I/O.
        let mut bad_jwt = HeaderMap::new();
        bad_jwt.insert("authorization", HeaderValue::from_static("Bearer only.two"));
        assert!(verify_google_request(&bad_jwt).await.is_err());

        std::env::remove_var("GOOGLE_ACTIONS_PROJECT_ID");
    }

    // NOTE: `verify_hmac_signature` reads the process-global `VOICE_WEBHOOK_SECRET`.
    // Its assertions live alongside the mounted `/verify` "google" branch in the
    // parent module's `hmac_and_verify_endpoint_google_branch` test so every
    // mutation of that env var stays in a single test and never races the
    // parallel runner.
}
