//! Voice-assistant OAuth business logic (Epic 93: Voice Assistant & OAuth
//! Completion).
//!
//! This module holds the behaviour-preserving business logic extracted from the
//! monolithic `routes/voice_webhooks.rs` so the HTTP layer keeps only request
//! wiring:
//!
//! - **Device authentication** — `authenticate_voice_user` resolves the linked
//!   device for a platform-presented OAuth access token (indexed lookup + the
//!   constant-time `voice_device_token_matches` authority, issues #2658 /
//!   #2662).
//! - **Token persistence encryption** — `encrypt_voice_token_pair` is the single
//!   fail-closed (#765) / indexed-hash-in-lock-step (#2662) choke point shared by
//!   the exchange and refresh handlers.
//! - **OAuth error sanitisation** — `voice_oauth_error_response` keeps upstream
//!   provider detail out of client responses (code-review: oauth client error
//!   leak).
//!
//! Every function is carried over verbatim from the original route module; the
//! split changes module location and visibility only, not behaviour.

use axum::{http::StatusCode, Json};
use chrono::{DateTime, Utc};
use common::errors::ErrorResponse;
use hmac::{Hmac, KeyInit, Mac};
use integrations::{encrypt_optional_required, encrypt_required, CryptoError, IntegrationCrypto};
use sha2::Sha256;
use sqlx::PgConnection;

type HmacSha256 = Hmac<Sha256>;

/// Encrypted voice OAuth token material ready for storage:
/// `(access_ciphertext, refresh_ciphertext, access_hash)`.
pub(crate) type EncryptedVoiceTokens = (String, Option<String>, Option<Vec<u8>>);

/// Client-facing message for a failed voice OAuth authorization-code exchange.
///
/// The upstream provider error (`VoiceOAuthManager::exchange_code`) routinely
/// embeds provider endpoints, client identifiers and raw transport detail. That
/// detail is logged server-side and MUST NOT be echoed to the caller
/// (code-review: oauth client error leak). Mirrors the policy already applied
/// to the sibling exchange handler in `routes/ai/voice.rs`.
pub(crate) const OAUTH_EXCHANGE_FAILED_MESSAGE: &str = "Authorization code exchange failed";

/// Client-facing message for a failed voice OAuth token refresh. Same non-leak
/// policy as [`OAUTH_EXCHANGE_FAILED_MESSAGE`].
pub(crate) const OAUTH_REFRESH_FAILED_MESSAGE: &str = "Token refresh failed";

/// Build the client response for a failed upstream voice OAuth call (code
/// exchange or token refresh). The upstream `detail` is logged server-side;
/// callers pass a static, non-revealing `client_message` for the body.
pub(crate) fn voice_oauth_error_response(
    status: StatusCode,
    code: &'static str,
    client_message: &'static str,
    detail: impl std::fmt::Display,
) -> (StatusCode, Json<ErrorResponse>) {
    // The upstream `detail` (provider endpoints, client ids, raw transport
    // errors) is logged server-side only. The response body carries just the
    // static `client_message` — never the detail (code-review: oauth client
    // error leak).
    tracing::error!(error = %detail, error_code = code, "voice OAuth upstream call failed");
    (status, Json(ErrorResponse::new(code, client_message)))
}

/// Map a fail-closed encryption error to an HTTP 500.
///
/// Issue #765: persisted voice OAuth tokens must always be encrypted; when
/// `INTEGRATION_ENCRYPTION_KEY` is unset we refuse to store them in plaintext.
pub(crate) fn voice_encryption_required(e: CryptoError) -> (StatusCode, Json<ErrorResponse>) {
    tracing::error!(error = %e, "Refusing to store voice OAuth token without encryption");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse::new(
            "ENCRYPTION_REQUIRED",
            "Integration token encryption is not configured",
        )),
    )
}

/// Encrypt a freshly exchanged / refreshed voice OAuth token pair for storage.
///
/// `oauth_token_exchange` and `oauth_token_refresh` (real and simulated
/// branches alike) persist upstream Amazon/Google tokens the same way and must
/// stay in lock-step on two security invariants:
/// - #765: encryption is MANDATORY — [`encrypt_required`] fails closed when no
///   `INTEGRATION_ENCRYPTION_KEY` is configured, mapped here to HTTP 500 so a
///   token is never written in plaintext.
/// - #2662: the indexed `access_token_hash` is derived from the SAME plaintext
///   as the stored ciphertext, so the O(1) device lookup can never drift out of
///   sync with the encrypted access token.
///
/// Returns `(access_ciphertext, refresh_ciphertext, access_hash)`; the caller
/// supplies `expires_at`, which is provider- or simulation-specific. Keeping
/// this in one place means the four call sites cannot silently diverge on
/// either invariant.
pub(crate) fn encrypt_voice_token_pair(
    crypto: Option<&IntegrationCrypto>,
    access_token: &str,
    refresh_token: Option<&str>,
) -> Result<EncryptedVoiceTokens, (StatusCode, Json<ErrorResponse>)> {
    let access_encrypted =
        encrypt_required(crypto, access_token).map_err(voice_encryption_required)?;
    let refresh_encrypted =
        encrypt_optional_required(crypto, refresh_token).map_err(voice_encryption_required)?;
    let access_hash = voice_access_token_hash(access_token);
    Ok((access_encrypted, refresh_encrypted, access_hash))
}

/// Whether an unconfigured voice OAuth platform may fall back to minting a
/// locally-simulated token instead of failing the request.
///
/// Simulated tokens are a development-only aid. Fail-open fix (security #890):
/// in RELEASE builds an unconfigured platform must fail closed — never mint or
/// store a fake token, which would otherwise clobber the device's real stored
/// token with a dead simulated one. The build flag is threaded through as an
/// explicit argument so the release (fail-closed) path stays unit-testable —
/// test binaries always compile with `debug_assertions` on.
pub(crate) fn voice_simulated_oauth_allowed(debug_build: bool) -> bool {
    debug_build
}

/// Constant-time byte equality for secret-derived string comparisons.
///
/// Issue #2658 (#3): comparing signatures/tokens with `==` leaks a timing
/// side-channel proportional to the shared prefix length. Mirrors the
/// constant-time comparison the portal/Airbnb webhook receivers use.
pub(crate) fn ct_eq_str(a: &str, b: &str) -> bool {
    use subtle::ConstantTimeEq;
    a.as_bytes().ct_eq(b.as_bytes()).into()
}

/// Keyed HMAC-SHA256 of a voice OAuth access token, used as a deterministic,
/// indexed lookup key (`voice_assistant_devices.access_token_hash`) so
/// `authenticate_voice_user` selects the candidate device in SQL instead of
/// decrypt-and-scanning every active device for the platform (issue #2662).
///
/// Keyed with `INTEGRATION_ENCRYPTION_KEY` — the same secret that encrypts the
/// token at rest — so the persisted value is a keyed MAC, not an offline-
/// guessable plain digest. Returns `None` when the key is unavailable: write
/// paths then persist a NULL hash and the read path falls back to the linear
/// scan, so a device is never locked out.
pub(crate) fn voice_access_token_hash(access_token: &str) -> Option<Vec<u8>> {
    let key = std::env::var(integrations::ENCRYPTION_KEY_ENV).ok()?;
    let mut mac = HmacSha256::new_from_slice(key.as_bytes()).ok()?;
    mac.update(access_token.as_bytes());
    Some(mac.finalize().into_bytes().to_vec())
}

/// Whether `presented` is the access token that owns `device`: it must match
/// the device's decrypted stored token (constant-time) and not be expired.
///
/// Pure (no I/O) so the security-critical matching logic can be unit-tested
/// without a database. See `authenticate_voice_user`.
pub(crate) fn voice_device_token_matches(
    crypto: &IntegrationCrypto,
    device: &db::models::VoiceAssistantDevice,
    presented: &str,
    now: DateTime<Utc>,
) -> bool {
    let Some(encrypted) = device.access_token_encrypted.as_deref() else {
        return false;
    };
    // A token we cannot decrypt (e.g. after key rotation) is simply not a match.
    let Ok(stored) = crypto.decrypt(encrypted) else {
        return false;
    };
    if !ct_eq_str(&stored, presented) {
        return false;
    }
    // Reject an expired access token.
    match device.token_expires_at {
        Some(expires_at) => expires_at > now,
        None => true,
    }
}

/// Authenticate a voice request by matching the platform-presented OAuth access
/// token against the encrypted token stored for a linked device.
///
/// Issue #2658 (#1 — broken authentication): the previous implementation
/// ignored the access token entirely and returned *any* active device for the
/// platform, authenticating every caller as whichever user most recently used
/// the platform (horizontal privilege escalation). We now fail closed unless
/// the presented token exactly matches a device's stored token:
///   * an absent/blank token is rejected outright;
///   * the integration encryption key is required — stored tokens are
///     ciphertext, so without the key we cannot validate and refuse rather than
///     authenticate anyone;
///   * each active device's stored token is decrypted and compared to the
///     presented token in constant time, and only the owning (unexpired) device
///     is returned.
pub(crate) async fn authenticate_voice_user(
    conn: &mut PgConnection,
    access_token: &str,
    platform: &str,
) -> Result<db::models::VoiceAssistantDevice, (StatusCode, Json<ErrorResponse>)> {
    let unauthorized = || {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new(
                "INVALID_ACCESS_TOKEN",
                "Voice device not linked or access token invalid. Please complete account linking.",
            )),
        )
    };

    // Reject an absent/blank token outright.
    if access_token.trim().is_empty() {
        return Err(unauthorized());
    }

    // Stored device access tokens are encrypted at rest; without the integration
    // encryption key we cannot validate the presented token, so fail closed
    // rather than authenticate anyone.
    let crypto = IntegrationCrypto::try_from_env().ok_or_else(|| {
        tracing::error!(
            "Voice authentication unavailable: INTEGRATION_ENCRYPTION_KEY is not configured"
        );
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "ENCRYPTION_REQUIRED",
                "Voice token validation is not configured",
            )),
        )
    })?;

    let now = Utc::now();
    let db_error = || {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("DATABASE_ERROR", "Database error")),
        )
    };

    // Fast path (issue #2662): select the single candidate device by a keyed
    // HMAC-SHA256 of the presented token, resolved by the partial index on
    // (platform, access_token_hash), instead of AES-GCM-decrypting every active
    // device for the platform. The decrypt + constant-time verify below remains
    // the authority (defence in depth) — the hash only narrows the search, so a
    // hash collision or a stale/rotated row still cannot authenticate.
    let token_hash = voice_access_token_hash(access_token);
    if let Some(hash) = token_hash.as_deref() {
        let candidate = sqlx::query_as::<_, db::models::VoiceAssistantDevice>(
            r#"
            SELECT * FROM voice_assistant_devices
            WHERE platform = $1
              AND is_active = TRUE
              AND access_token_hash = $2
            LIMIT 1
            "#,
        )
        .bind(platform)
        .bind(hash)
        .fetch_optional(&mut *conn)
        .await
        .map_err(|e| {
            tracing::error!("Database error: {}", e);
            db_error()
        })?;

        if let Some(device) = candidate {
            if voice_device_token_matches(&crypto, &device, access_token, now) {
                return Ok(device);
            }
        }
    }

    // Fallback: linear decrypt-and-scan. When a hash was computed we only need
    // the rows that predate the hash column (NULL hash) — the indexed path above
    // already covered every backfilled row; when no key/hash is available we
    // scan all active token-bearing devices (legacy behaviour). Either way the
    // AES-GCM ciphertext (random nonce per encryption) cannot be matched in SQL,
    // so equality is checked after decryption.
    let scanned_hashed = token_hash.is_some();
    let devices = sqlx::query_as::<_, db::models::VoiceAssistantDevice>(
        r#"
        SELECT * FROM voice_assistant_devices
        WHERE platform = $1
          AND is_active = TRUE
          AND access_token_encrypted IS NOT NULL
          AND (NOT $2 OR access_token_hash IS NULL)
        ORDER BY last_used_at DESC NULLS LAST
        "#,
    )
    .bind(platform)
    .bind(scanned_hashed)
    .fetch_all(&mut *conn)
    .await
    .map_err(|e| {
        tracing::error!("Database error: {}", e);
        db_error()
    })?;

    devices
        .into_iter()
        .find(|device| voice_device_token_matches(&crypto, device, access_token, now))
        .ok_or_else(unauthorized)
}

#[cfg(test)]
mod tests {
    use super::*;
    use db::models::voice_platform;
    use uuid::Uuid;

    /// Regression (code-review: oauth client error leak): the client-facing
    /// body for a failed voice OAuth exchange / refresh must carry only the
    /// static, non-revealing message — never the upstream provider detail,
    /// which can embed internal hosts, client ids, or bearer credentials.
    #[test]
    fn voice_oauth_error_response_does_not_leak_upstream_detail() {
        let sensitive = "connect error: dns failure for secret-internal-host.oauth.example:443 \
             (bearer AKIA-LEAKED-SECRET)";

        for (code, message) in [
            ("OAUTH_EXCHANGE_FAILED", OAUTH_EXCHANGE_FAILED_MESSAGE),
            ("TOKEN_REFRESH_FAILED", OAUTH_REFRESH_FAILED_MESSAGE),
        ] {
            let (status, body) =
                voice_oauth_error_response(StatusCode::BAD_REQUEST, code, message, sensitive);
            assert_eq!(status, StatusCode::BAD_REQUEST);

            let json = serde_json::to_string(&body.0).expect("serialize ErrorResponse");
            assert!(
                !json.contains("secret-internal-host"),
                "leaked upstream host in {code} body: {json}"
            );
            assert!(
                !json.contains("AKIA-LEAKED-SECRET"),
                "leaked credential in {code} body: {json}"
            );
            assert!(
                !json.contains("dns failure"),
                "leaked transport detail in {code} body: {json}"
            );
            assert_eq!(body.0.code, code, "error code altered: {json}");
            assert_eq!(
                body.0.message, message,
                "client body must be exactly the sanitized message: {json}"
            );
        }
    }

    // --- voice OAuth simulated-token fallback gate (#890) ---------------

    #[test]
    fn simulated_voice_oauth_fallback_forbidden_in_release_builds() {
        // Fail-open fix (#890): when the platform OAuth client is unconfigured,
        // a RELEASE build must fail closed — never mint or store a simulated
        // refresh token (which would also clobber the device's real stored
        // token). The refresh handler gates its simulated branch on this.
        assert!(
            !voice_simulated_oauth_allowed(false),
            "release builds must fail closed, never mint a simulated voice token"
        );
    }

    #[test]
    fn simulated_voice_oauth_fallback_allowed_in_debug_builds() {
        // Development convenience is preserved: debug/test builds may still use
        // the simulated fallback when the platform is unconfigured.
        assert!(
            voice_simulated_oauth_allowed(true),
            "debug builds may use the simulated fallback"
        );
    }

    // --- ct_eq_str / voice_device_token_matches (issue #2658) -----------

    #[test]
    fn ct_eq_str_matches_only_identical_strings() {
        assert!(ct_eq_str("token-abc", "token-abc"));
        assert!(!ct_eq_str("token-abc", "token-abd"));
        assert!(!ct_eq_str("token-abc", "token-abc-longer"));
        assert!(!ct_eq_str("", "x"));
        assert!(ct_eq_str("", ""));
    }

    /// A `VoiceAssistantDevice` whose `access_token_encrypted` holds `plaintext`
    /// encrypted under `crypto`, expiring at `expires_at`.
    fn device_with_token(
        crypto: &IntegrationCrypto,
        plaintext: Option<&str>,
        expires_at: Option<DateTime<Utc>>,
    ) -> db::models::VoiceAssistantDevice {
        let now = Utc::now();
        db::models::VoiceAssistantDevice {
            id: Uuid::new_v4(),
            organization_id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            unit_id: None,
            platform: voice_platform::ALEXA.to_string(),
            device_id: "dev-1".to_string(),
            device_name: None,
            linked_at: now,
            last_used_at: None,
            access_token_encrypted: plaintext.map(|p| crypto.encrypt(p).unwrap()),
            refresh_token_encrypted: None,
            token_expires_at: expires_at,
            is_active: true,
            capabilities: serde_json::json!([]),
            // The pure predicate ignores the lookup hash (it re-derives trust
            // from the decrypted ciphertext); the DB-backed selection is covered
            // by `authenticate_voice_user_*` below.
            access_token_hash: None,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn voice_token_match_is_exact_and_rejects_mismatch_missing_and_expired() {
        // Deterministic 32-byte key; construct crypto directly (no env races).
        let crypto = IntegrationCrypto::new(&"ab".repeat(32)).expect("crypto");
        let now = Utc::now();

        // Exact match on a non-expiring token -> authenticated.
        let dev = device_with_token(&crypto, Some("real-token"), None);
        assert!(voice_device_token_matches(&crypto, &dev, "real-token", now));

        // Wrong token -> not a match (this is the broken-auth regression: the
        // old code returned this device regardless of the presented token).
        assert!(!voice_device_token_matches(&crypto, &dev, "attacker", now));

        // Device without a stored token -> never a match.
        let no_tok = device_with_token(&crypto, None, None);
        assert!(!voice_device_token_matches(
            &crypto,
            &no_tok,
            "real-token",
            now
        ));

        // Matching token but expired -> rejected.
        let expired = device_with_token(
            &crypto,
            Some("real-token"),
            Some(now - chrono::Duration::hours(1)),
        );
        assert!(!voice_device_token_matches(
            &crypto,
            &expired,
            "real-token",
            now
        ));

        // Matching token still valid in the future -> accepted.
        let future = device_with_token(
            &crypto,
            Some("real-token"),
            Some(now + chrono::Duration::hours(1)),
        );
        assert!(voice_device_token_matches(
            &crypto,
            &future,
            "real-token",
            now
        ));

        // A token encrypted under a different key can't be decrypted -> no match.
        let other = IntegrationCrypto::new(&"cd".repeat(32)).expect("crypto");
        assert!(!voice_device_token_matches(&other, &dev, "real-token", now));
    }

    // --- authenticate_voice_user (DB-backed selection path, issue #2662) ------
    //
    // Postgres-backed via `#[sqlx::test]` (runs under backend.yml CI; the cloud
    // verify gate skips it — no DATABASE_URL). This drives the real fetch +
    // selection loop the pure `voice_device_token_matches` predicate cannot
    // reach: the #2660 regression lived exactly here (the old code returned
    // *any* active device for the platform regardless of the presented token).

    /// Seed one active voice device on `platform` whose stored token is `token`
    /// (encrypted under `crypto`) and whose indexed lookup hash is the keyed
    /// HMAC of `token`, expiring at `expires_at`. Returns the new device id.
    async fn seed_voice_device(
        pool: &sqlx::PgPool,
        crypto: &IntegrationCrypto,
        platform: &str,
        token: &str,
        expires_at: Option<DateTime<Utc>>,
    ) -> Uuid {
        let id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO voice_assistant_devices (
                id, organization_id, user_id, platform, device_id,
                access_token_encrypted, access_token_hash, token_expires_at,
                is_active, capabilities
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, TRUE, '[]'::jsonb)
            "#,
        )
        .bind(id)
        .bind(Uuid::new_v4())
        .bind(Uuid::new_v4())
        .bind(platform)
        .bind(format!("dev-{id}"))
        .bind(crypto.encrypt(token).expect("encrypt seed token"))
        .bind(voice_access_token_hash(token))
        .bind(expires_at)
        .execute(pool)
        .await
        .expect("seed voice device");
        id
    }

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn authenticate_voice_user_selects_owner_and_rejects_others(pool: sqlx::PgPool) {
        // `authenticate_voice_user` reads the encryption key from the process
        // environment — both to decrypt stored tokens and to key the lookup
        // hash — so pin a deterministic key. This is the only in-crate test that
        // touches INTEGRATION_ENCRYPTION_KEY.
        let key = "ab".repeat(32); // 64 hex chars = 32 bytes
        std::env::set_var(integrations::ENCRYPTION_KEY_ENV, &key);
        let crypto = IntegrationCrypto::new(&key).expect("crypto");

        // Two devices on the same platform with distinct tokens (the priv-esc
        // guard: presenting A's token must never authenticate as B).
        let id_a = seed_voice_device(&pool, &crypto, voice_platform::ALEXA, "token-A", None).await;
        let _id_b = seed_voice_device(&pool, &crypto, voice_platform::ALEXA, "token-B", None).await;

        let mut conn = pool.acquire().await.expect("acquire connection");

        // (a) Presenting A's token resolves to exactly device A, never B.
        let dev = authenticate_voice_user(&mut conn, "token-A", voice_platform::ALEXA)
            .await
            .expect("token-A must authenticate as device A");
        assert_eq!(
            dev.id, id_a,
            "presented token-A must resolve to device A only"
        );

        // (b) A token matching no device -> 401 INVALID_ACCESS_TOKEN.
        let (status, body) =
            authenticate_voice_user(&mut conn, "token-UNKNOWN", voice_platform::ALEXA)
                .await
                .expect_err("unknown token must be rejected");
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body.0.code, "INVALID_ACCESS_TOKEN");

        // (c) An expired but otherwise-matching token is rejected.
        let past = Utc::now() - chrono::Duration::hours(1);
        seed_voice_device(
            &pool,
            &crypto,
            voice_platform::ALEXA,
            "token-EXP",
            Some(past),
        )
        .await;
        let (status, body) = authenticate_voice_user(&mut conn, "token-EXP", voice_platform::ALEXA)
            .await
            .expect_err("expired token must be rejected");
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body.0.code, "INVALID_ACCESS_TOKEN");

        // (d) An absent (empty or whitespace-only) access token is rejected
        // outright — the fail-closed early return, before any device lookup, so
        // a caller presenting no token can never be authenticated as a device.
        for absent in ["", "   "] {
            let (status, body) = authenticate_voice_user(&mut conn, absent, voice_platform::ALEXA)
                .await
                .expect_err("absent token must be rejected");
            assert_eq!(status, StatusCode::UNAUTHORIZED);
            assert_eq!(body.0.code, "INVALID_ACCESS_TOKEN");
        }

        std::env::remove_var(integrations::ENCRYPTION_KEY_ENV);
    }

    // --- voice_encryption_required (OAuth persistence fail-closed) -------

    #[test]
    fn encryption_required_maps_to_500() {
        let (status, body) =
            voice_encryption_required(CryptoError::KeyNotConfigured("no key".to_string()));
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body.0.code, "ENCRYPTION_REQUIRED");
    }

    #[test]
    fn encrypt_voice_token_pair_fails_closed_without_crypto() {
        // #765: the shared token-encryption choke point used by both the
        // exchange and refresh handlers must refuse to produce a token pair
        // (HTTP 500 ENCRYPTION_REQUIRED) when no encryption key is configured —
        // never emit a plaintext access/refresh token. Passing `Some(refresh)`
        // ensures the failure is on the access token, before the refresh branch.
        let (status, body) = encrypt_voice_token_pair(None, "access-token", Some("refresh-token"))
            .expect_err("must fail closed when crypto is unconfigured");
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body.0.code, "ENCRYPTION_REQUIRED");
    }

    #[test]
    fn encrypt_voice_token_pair_round_trips_and_stores_no_plaintext() {
        // #2838 centralization: every voice OAuth persistence site (exchange +
        // refresh, real Alexa/Google upstreams and the simulated fallback alike)
        // funnels its token pair through `encrypt_voice_token_pair`. This asserts
        // the shared choke point's round-trip and no-plaintext-at-rest guarantee
        // across all three provider shapes.
        //
        // Env-independent by design: `crypto` is built directly from a fixed key.
        // The concurrently-running `authenticate_voice_user_*` test toggles the
        // process-wide INTEGRATION_ENCRYPTION_KEY, so the returned `access_hash`
        // — which reads that env — is deliberately NOT asserted here; the stored
        // hash / plaintext lock-step (#2662) is covered end to end by the suite_8
        // `voice_oauth_token_encryption_roundtrip_tests` integration suite.
        let crypto = IntegrationCrypto::new(
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        )
        .expect("valid 32-byte key");

        // (label, access_token, refresh_token) — Alexa, Google (with and without
        // a refresh token), and the dev-only simulated fallback provider
        // (`voice_access_<platform>_<uuid>`, minted when a platform is
        // unconfigured in a debug build).
        let cases: [(&str, &str, Option<&str>); 4] = [
            (
                "alexa",
                "Atza|IQEBLjAsAhRmHjN.alexa-access-token",
                Some("Atzr|IQEBLzAsAhRmHjN.alexa-refresh-token"),
            ),
            (
                "google",
                "ya29.a0AfB_byC-google-access-token",
                Some("1//09-google-refresh-token"),
            ),
            // Google may legitimately omit a refresh token on re-consent.
            (
                "google_no_refresh",
                "ya29.a0AfB_byC-google-access-only",
                None,
            ),
            // Fallback / simulated provider (unconfigured platform, debug build).
            (
                "fallback_simulated",
                "voice_access_alexa_2f1c4e8a-0000-4000-8000-000000000001",
                Some("voice_refresh_alexa_2f1c4e8a-0000-4000-8000-000000000002"),
            ),
        ];

        for (label, access, refresh) in cases {
            let (access_ct, refresh_ct, _access_hash) =
                encrypt_voice_token_pair(Some(&crypto), access, refresh).unwrap_or_else(
                    |(s, b)| panic!("[{label}] must encrypt with a key: {s} {}", b.0.code),
                );

            // Access token: encrypted at rest, plaintext never stored.
            assert_ne!(
                access_ct, access,
                "[{label}] access ciphertext must differ from plaintext"
            );
            assert!(
                access_ct.starts_with("enc:"),
                "[{label}] stored access token must carry the enc: prefix, got {access_ct}"
            );
            assert!(
                !access_ct.contains(access),
                "[{label}] plaintext access token must never appear in the stored value"
            );
            // Round-trip: the stored ciphertext decrypts back to the original.
            assert_eq!(
                integrations::decrypt_if_available(Some(&crypto), &access_ct),
                access,
                "[{label}] access token must round-trip through decrypt"
            );

            // Refresh token: same guarantees when present; `None` passes through.
            match (refresh, refresh_ct) {
                (Some(refresh_plain), Some(refresh_ct)) => {
                    assert_ne!(
                        refresh_ct, refresh_plain,
                        "[{label}] refresh ciphertext must differ from plaintext"
                    );
                    assert!(
                        refresh_ct.starts_with("enc:"),
                        "[{label}] stored refresh token must carry the enc: prefix"
                    );
                    assert!(
                        !refresh_ct.contains(refresh_plain),
                        "[{label}] plaintext refresh token must never appear in the stored value"
                    );
                    assert_eq!(
                        integrations::decrypt_if_available(Some(&crypto), &refresh_ct),
                        refresh_plain,
                        "[{label}] refresh token must round-trip through decrypt"
                    );
                }
                (None, None) => { /* no refresh token to persist — correct */ }
                (r, c) => {
                    panic!("[{label}] refresh presence mismatch: input {r:?} -> stored {c:?}")
                }
            }
        }
    }
}
