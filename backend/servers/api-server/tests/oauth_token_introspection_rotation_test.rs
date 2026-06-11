//! OAuth 2.0 token introspection + refresh-rotation security tests
//! (coverage story 10a-3, additive).
//!
//! This file deliberately covers the *uncovered* surface of token management
//! that `oauth_integration_tests.rs` does not already exercise. It does NOT
//! duplicate the PKCE happy path, the basic refresh rotation, the family
//! reuse-detection happy path, or the introspection-of-revoked-token cases —
//! those already live next door.  Here we pin the shipped contract for the
//! remaining edges:
//!
//! Introspection (RFC 7662):
//!  - unknown / garbage token → `active:false`, HTTP 200 (NOT an error).
//!  - invalid client *secret* (vs the existing "missing creds" case) → 401.
//!  - cross-client binding: client B introspecting client A's token → inactive.
//!  - expired access token → inactive (distinct from a *revoked* one).
//!  - the inactive payload shape (only `active:false`; metadata fields absent/null).
//!  - introspecting an access token reports `token_type:"access_token"`.
//!
//! Refresh-token rotation security:
//!  - shipped quirk: a refresh exchange revokes only the *refresh* token; the
//!    previously-issued *access* token survives the rotation (still active).
//!  - expired refresh token → `invalid_grant` (TokenExpired path), distinct
//!    from a revoked / reused one.
//!  - unknown / garbage refresh token → `invalid_grant`.
//!  - revoke-on-reuse, observed through introspection: after a consumed token
//!    is replayed and the family is revoked, a *sibling* live token from the
//!    same family introspects as inactive.
//!
//! Every test uses `#[sqlx::test(migrator = "db::MIGRATOR")]` so the schema is
//! current, and drives the real Axum router via `TestApp`.

#[allow(dead_code)]
mod common;

use api_server::services::AuthService;
use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::{Duration, Utc};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use common::{create_authenticated_user, TestApp, TestUser};

// ─── helpers (kept local; the sibling file's copies are private to it) ──────

/// Build a PKCE (S256) code_verifier + code_challenge pair.
fn pkce_pair() -> (String, String) {
    let verifier = format!("test-verifier-{}", Uuid::new_v4());
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let challenge = URL_SAFE_NO_PAD.encode(hasher.finalize());
    (verifier, challenge)
}

/// SHA-256 hex of a token — mirrors `OAuthService::hash_token`, so a token we
/// seed directly with a known plaintext can be looked up by the service.
fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

/// URL-encode a key=value form body.
fn form_body(pairs: &[(&str, &str)]) -> String {
    pairs
        .iter()
        .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
        .collect::<Vec<_>>()
        .join("&")
}

/// POST application/x-www-form-urlencoded.
fn form_request(uri: &str, body: &str) -> Request<Body> {
    Request::builder()
        .method(Method::POST)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(Body::from(body.to_string()))
        .unwrap()
}

/// POST application/x-www-form-urlencoded with Bearer auth (consent endpoint).
fn form_request_with_auth(uri: &str, body: &str, token: &str) -> Request<Body> {
    Request::builder()
        .method(Method::POST)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::from(body.to_string()))
        .unwrap()
}

/// Seed a confidential OAuth client (rotation enabled) directly in the DB.
/// Returns `(client_id, client_secret, redirect_uri)`.
async fn seed_confidential_client(pool: &PgPool) -> (String, String, String) {
    let client_id = format!("ci-conf-{}", &Uuid::new_v4().to_string()[..8]);
    let redirect_uri = "https://app.example.com/callback".to_string();
    let plaintext_secret = "test-client-secret-very-secure-12345678";
    let auth = AuthService::new();
    let hash = auth.hash_password(plaintext_secret).expect("hash secret");

    sqlx::query(
        r#"
        INSERT INTO oauth_clients
            (client_id, client_secret_hash, name, redirect_uris, scopes, is_confidential, rotate_refresh_tokens)
        VALUES
            ($1, $2, 'CI Conf', $3::jsonb, '["profile","email"]'::jsonb, true, true)
        "#,
    )
    .bind(&client_id)
    .bind(&hash)
    .bind(serde_json::json!([redirect_uri]).to_string())
    .execute(pool)
    .await
    .expect("seed confidential client");

    (client_id, plaintext_secret.to_string(), redirect_uri)
}

/// Drive the full PKCE S256 authorization-code flow for a confidential client
/// and return `(access_token, refresh_token)`.
async fn confidential_auth_flow(
    app: &TestApp,
    user_at: &str,
    client_id: &str,
    client_secret: &str,
    redirect_uri: &str,
) -> (String, String) {
    let (verifier, challenge) = pkce_pair();
    let consent_form = form_body(&[
        ("client_id", client_id),
        ("redirect_uri", redirect_uri),
        ("scope", "profile"),
        ("code_challenge", &challenge),
        ("code_challenge_method", "S256"),
        ("consent", "approve"),
    ]);
    let code = app
        .execute(form_request_with_auth(
            "/api/v1/oauth/authorize",
            &consent_form,
            user_at,
        ))
        .await
        .json_value()["code"]
        .as_str()
        .expect("missing authorization code")
        .to_string();

    let token_body = form_body(&[
        ("grant_type", "authorization_code"),
        ("code", &code),
        ("redirect_uri", redirect_uri),
        ("client_id", client_id),
        ("client_secret", client_secret),
        ("code_verifier", &verifier),
    ]);
    let tokens = app
        .execute(form_request("/api/v1/oauth/token", &token_body))
        .await
        .json_value();

    let at = tokens["access_token"]
        .as_str()
        .expect("access_token")
        .to_string();
    let rt = tokens["refresh_token"]
        .as_str()
        .expect("refresh_token")
        .to_string();
    (at, rt)
}

/// Introspect `token` with explicit client credentials in the form body.
async fn introspect_with_creds(
    app: &TestApp,
    token: &str,
    client_id: &str,
    client_secret: &str,
) -> serde_json::Value {
    let body = form_body(&[
        ("token", token),
        ("client_id", client_id),
        ("client_secret", client_secret),
    ]);
    app.execute(form_request("/api/v1/oauth/introspect", &body))
        .await
        .json_value()
}

/// Exchange a refresh token at the token endpoint and return the raw response.
async fn refresh(
    app: &TestApp,
    refresh_token: &str,
    client_id: &str,
    client_secret: &str,
) -> common::TestResponse {
    let body = form_body(&[
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
        ("client_id", client_id),
        ("client_secret", client_secret),
    ]);
    app.execute(form_request("/api/v1/oauth/token", &body))
        .await
}

/// Seed an access token row directly with a caller-chosen plaintext and expiry.
/// Returns the plaintext token string. Used to manufacture an *expired* token
/// deterministically without waiting out a TTL.
async fn seed_access_token(
    pool: &PgPool,
    user_id: Uuid,
    client_id: &str,
    expires_at: chrono::DateTime<Utc>,
) -> String {
    let plaintext = format!("seeded-at-{}", Uuid::new_v4());
    sqlx::query(
        r#"
        INSERT INTO oauth_access_tokens
            (user_id, client_id, token_hash, scopes, expires_at)
        VALUES ($1, $2, $3, '["profile"]'::jsonb, $4)
        "#,
    )
    .bind(user_id)
    .bind(client_id)
    .bind(hash_token(&plaintext))
    .bind(expires_at)
    .execute(pool)
    .await
    .expect("seed access token");
    plaintext
}

/// Seed a refresh token row directly with a caller-chosen plaintext and expiry.
async fn seed_refresh_token(
    pool: &PgPool,
    user_id: Uuid,
    client_id: &str,
    expires_at: chrono::DateTime<Utc>,
) -> String {
    let plaintext = format!("seeded-rt-{}", Uuid::new_v4());
    sqlx::query(
        r#"
        INSERT INTO oauth_refresh_tokens
            (user_id, client_id, token_hash, scopes, expires_at)
        VALUES ($1, $2, $3, '["profile"]'::jsonb, $4)
        "#,
    )
    .bind(user_id)
    .bind(client_id)
    .bind(hash_token(&plaintext))
    .bind(expires_at)
    .execute(pool)
    .await
    .expect("seed refresh token");
    plaintext
}

// ─── module: introspection_edges ────────────────────────────────────────────

#[cfg(test)]
mod introspection_edges {
    use super::*;

    /// An unknown / garbage token must introspect as `active:false` with HTTP
    /// 200 — RFC 7662 §2.2 mandates the server NOT error on a token it doesn't
    /// recognise. (The sibling file only covers *revoked real* tokens.)
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_introspect_unknown_token_returns_inactive_200(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let (client_id, client_secret, _redirect) = seed_confidential_client(&pool).await;

        let body = form_body(&[
            ("token", "totally-made-up-opaque-token-value-xyz"),
            ("client_id", &client_id),
            ("client_secret", &client_secret),
        ]);
        let resp = app
            .execute(form_request("/api/v1/oauth/introspect", &body))
            .await;
        assert_eq!(
            resp.status,
            StatusCode::OK,
            "unknown token must NOT error — must return 200. body={}",
            resp.text()
        );
        assert_eq!(
            resp.json_value()["active"],
            false,
            "unknown token must be active=false"
        );
    }

    /// Introspection with a valid `client_id` but the WRONG `client_secret`
    /// must return 401 invalid_client. The sibling file only covers *missing*
    /// credentials; this pins the bad-secret branch.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_introspect_wrong_client_secret_returns_401(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (user_at, _) = create_authenticated_user(&app, &user).await;
        let (client_id, client_secret, redirect_uri) = seed_confidential_client(&pool).await;
        let (oauth_at, _rt) =
            confidential_auth_flow(&app, &user_at, &client_id, &client_secret, &redirect_uri).await;

        let body = form_body(&[
            ("token", &oauth_at),
            ("client_id", &client_id),
            ("client_secret", "this-is-not-the-real-secret"),
        ]);
        let resp = app
            .execute(form_request("/api/v1/oauth/introspect", &body))
            .await;
        assert_eq!(
            resp.status,
            StatusCode::UNAUTHORIZED,
            "wrong client_secret must return 401. body={}",
            resp.text()
        );
        assert_eq!(resp.json_value()["error"], "invalid_client");
    }

    /// RFC 7662 §2.2 client binding: a token issued to client A, introspected
    /// by an authenticated *different* client B, must be reported as inactive —
    /// so one client cannot enumerate another client's token metadata.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_introspect_cross_client_token_reported_inactive(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (user_at, _) = create_authenticated_user(&app, &user).await;

        // Client A mints a real access token.
        let (client_a, secret_a, redirect_a) = seed_confidential_client(&pool).await;
        let (token_a, _rt) =
            confidential_auth_flow(&app, &user_at, &client_a, &secret_a, &redirect_a).await;

        // Client B authenticates and tries to introspect client A's token.
        let (client_b, secret_b, _redirect_b) = seed_confidential_client(&pool).await;
        let intro = introspect_with_creds(&app, &token_a, &client_b, &secret_b).await;
        assert_eq!(
            intro["active"], false,
            "a token belonging to another client must be reported inactive, got {}",
            intro
        );
        // And the metadata must NOT leak across the client boundary.
        assert!(
            intro["sub"].is_null(),
            "sub must not leak to another client"
        );
        assert!(
            intro["scope"].is_null(),
            "scope must not leak to another client"
        );
    }

    /// An *expired* (not revoked) access token introspects as inactive. We seed
    /// the row directly with a past `expires_at` so the test is deterministic.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_introspect_expired_access_token_inactive(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (user_at, _) = create_authenticated_user(&app, &user).await;
        let user_id: Uuid = user_at_to_user_id(&app, &user_at).await;
        let (client_id, client_secret, _redirect) = seed_confidential_client(&pool).await;

        let expired =
            seed_access_token(&pool, user_id, &client_id, Utc::now() - Duration::hours(1)).await;

        let intro = introspect_with_creds(&app, &expired, &client_id, &client_secret).await;
        assert_eq!(
            intro["active"], false,
            "an expired access token must introspect inactive, got {}",
            intro
        );
    }

    /// The inactive introspection payload must carry ONLY `active:false`; every
    /// other RFC 7662 metadata field is absent/null (serialized as null here).
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_inactive_payload_has_no_metadata(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let (client_id, client_secret, _redirect) = seed_confidential_client(&pool).await;

        let intro = introspect_with_creds(&app, "no-such-token", &client_id, &client_secret).await;
        assert_eq!(intro["active"], false);
        for field in [
            "scope",
            "client_id",
            "username",
            "token_type",
            "exp",
            "iat",
            "sub",
        ] {
            assert!(
                intro[field].is_null(),
                "inactive payload must not expose `{}`, got {}",
                field,
                intro
            );
        }
    }

    /// A live access token introspects active with `token_type:"access_token"`.
    /// (The sibling file asserts the refresh-token branch's `token_type`; this
    /// pins the access-token branch's discriminator explicitly.)
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_introspect_access_token_type_discriminator(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (user_at, _) = create_authenticated_user(&app, &user).await;
        let (client_id, client_secret, redirect_uri) = seed_confidential_client(&pool).await;
        let (oauth_at, _rt) =
            confidential_auth_flow(&app, &user_at, &client_id, &client_secret, &redirect_uri).await;

        let intro = introspect_with_creds(&app, &oauth_at, &client_id, &client_secret).await;
        assert_eq!(intro["active"], true);
        assert_eq!(
            intro["token_type"], "access_token",
            "access-token branch must report token_type=access_token, got {}",
            intro
        );
    }
}

// ─── module: rotation_security ──────────────────────────────────────────────

#[cfg(test)]
mod rotation_security {
    use super::*;

    /// Shipped-quirk pin: refreshing rotates only the *refresh* token. The
    /// access token issued alongside the OLD refresh token is NOT revoked by
    /// the rotation and keeps introspecting active until its own TTL elapses.
    /// We pin the behaviour as shipped rather than asserting an aspirational
    /// "rotation also kills the old access token" contract.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_old_access_token_survives_refresh_rotation(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (user_at, _) = create_authenticated_user(&app, &user).await;
        let (client_id, client_secret, redirect_uri) = seed_confidential_client(&pool).await;

        let (at1, rt1) =
            confidential_auth_flow(&app, &user_at, &client_id, &client_secret, &redirect_uri).await;

        // Rotate rt1 → rt2. This succeeds and revokes rt1.
        let resp = refresh(&app, &rt1, &client_id, &client_secret).await;
        assert_eq!(
            resp.status,
            StatusCode::OK,
            "refresh must succeed. body={}",
            resp.text()
        );

        // The original access token (at1) is still introspectable as active.
        let intro = introspect_with_creds(&app, &at1, &client_id, &client_secret).await;
        assert_eq!(
            intro["active"], true,
            "shipped behaviour: the pre-rotation access token survives the refresh, got {}",
            intro
        );

        // Sanity: the old refresh token (rt1) is now dead at the token endpoint.
        let stale = refresh(&app, &rt1, &client_id, &client_secret).await;
        assert_eq!(
            stale.status,
            StatusCode::BAD_REQUEST,
            "old refresh token must be rejected post-rotation. body={}",
            stale.text()
        );
        assert_eq!(stale.json_value()["error"], "invalid_grant");
    }

    /// An *expired* refresh token must be rejected at the token endpoint with
    /// `invalid_grant` (TokenExpired path) — distinct from a revoked/reused one.
    /// Seeded directly with a past `expires_at` so the test is deterministic.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_expired_refresh_token_rejected(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (user_at, _) = create_authenticated_user(&app, &user).await;
        let user_id = user_at_to_user_id(&app, &user_at).await;
        let (client_id, client_secret, _redirect) = seed_confidential_client(&pool).await;

        let expired_rt =
            seed_refresh_token(&pool, user_id, &client_id, Utc::now() - Duration::hours(1)).await;

        let resp = refresh(&app, &expired_rt, &client_id, &client_secret).await;
        assert_eq!(
            resp.status,
            StatusCode::BAD_REQUEST,
            "expired refresh token must be rejected. body={}",
            resp.text()
        );
        assert_eq!(
            resp.json_value()["error"],
            "invalid_grant",
            "expired refresh must map to invalid_grant"
        );
    }

    /// An unknown / garbage refresh token must be rejected with `invalid_grant`
    /// (it is indistinguishable from any other unusable token, by design).
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_unknown_refresh_token_rejected(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let (client_id, client_secret, _redirect) = seed_confidential_client(&pool).await;

        let resp = refresh(
            &app,
            "never-issued-refresh-token",
            &client_id,
            &client_secret,
        )
        .await;
        assert_eq!(
            resp.status,
            StatusCode::BAD_REQUEST,
            "unknown refresh token must be rejected. body={}",
            resp.text()
        );
        assert_eq!(resp.json_value()["error"], "invalid_grant");
    }

    /// Revoke-on-reuse, observed through introspection. After a consumed token
    /// is replayed (reuse detected → whole family revoked), a *sibling* live
    /// token from the same family must now introspect as inactive. This ties
    /// the rotation-security family revocation to the introspection surface,
    /// which the sibling file only checks via the token endpoint.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_family_revocation_visible_via_introspection(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (user_at, _) = create_authenticated_user(&app, &user).await;
        let (client_id, client_secret, redirect_uri) = seed_confidential_client(&pool).await;

        let (_at1, rt1) =
            confidential_auth_flow(&app, &user_at, &client_id, &client_secret, &redirect_uri).await;

        // Rotate rt1 → rt2 (rt2 is the live sibling we'll watch).
        let rt2 = refresh(&app, &rt1, &client_id, &client_secret)
            .await
            .json_value()["refresh_token"]
            .as_str()
            .expect("rt2")
            .to_string();

        // rt2 is currently active.
        let before = introspect_with_creds(&app, &rt2, &client_id, &client_secret).await;
        assert_eq!(
            before["active"], true,
            "live sibling must start active, got {}",
            before
        );

        // Replay the consumed rt1 → reuse detection revokes the whole family.
        let replay = refresh(&app, &rt1, &client_id, &client_secret).await;
        assert_eq!(
            replay.status,
            StatusCode::BAD_REQUEST,
            "replaying consumed token must be rejected. body={}",
            replay.text()
        );
        assert_eq!(replay.json_value()["error"], "invalid_grant");

        // The live sibling rt2 must now introspect inactive — the family was revoked.
        let after = introspect_with_creds(&app, &rt2, &client_id, &client_secret).await;
        assert_eq!(
            after["active"], false,
            "sibling token must be inactive after family revocation, got {}",
            after
        );
    }
}

// ─── shared helper used by both modules ─────────────────────────────────────

/// Resolve the user UUID for a seeded authenticated test user by introspecting
/// a freshly-minted access token through the OAuth flow would be circular, so
/// we read it from the `users` table via the bearer's `sub`. The access token
/// minted by `create_authenticated_user` is a first-party JWT whose `sub` is
/// the user id; decode it without verification to extract the subject.
async fn user_at_to_user_id(_app: &TestApp, user_at: &str) -> Uuid {
    // A JWT is three base64url segments; the middle one is the claims payload.
    let payload_b64 = user_at
        .split('.')
        .nth(1)
        .expect("access token must be a JWT with a payload segment");
    let payload = URL_SAFE_NO_PAD
        .decode(payload_b64)
        .expect("JWT payload must be base64url");
    let claims: serde_json::Value =
        serde_json::from_slice(&payload).expect("JWT payload must be JSON");
    let sub = claims["sub"].as_str().expect("JWT must carry sub");
    Uuid::parse_str(sub).expect("sub must be a UUID")
}
