//! OAuth 2.0 Authorization-Server edge-case integration tests (additive gap cover).
//!
//! The companion suites (`oauth_integration_tests.rs`, `oauth_authorization_server_test.rs`,
//! `oauth_client_registration_test.rs`, `oauth_token_introspection_rotation_test.rs`)
//! are comprehensive for the core flows and security properties. This file pins the
//! remaining narrow edges that none of the existing files exercise:
//!
//!   1. **RFC 6749 §5.1 transport headers**: the token endpoint must set BOTH
//!      `Cache-Control: no-store` AND `Pragma: no-cache` on every successful
//!      token response (both authorization-code and refresh-token grants).
//!
//!   2. **Deactivated client at /authorize**: a revoked (`is_active=false`) client
//!      must be rejected at the initial authorize GET and at the consent POST
//!      (the existing tests only pin the deactivation behaviour at /token and
//!      /introspect — the authorize-stage rejection is uncovered).
//!
//!   3. **Revoke endpoint: Basic-auth header**: RFC 7009 §2.3 / RFC 6749 §2.3.1
//!      allow client credentials to be supplied via HTTP Basic auth on the
//!      revocation endpoint. The sibling file covers form-param creds and
//!      introspect Basic-auth; this pins the revoke path.
//!
//!   4. **Public client supplying a wrong secret at /token**: the token handler
//!      has a branch for a public client that voluntarily sends `client_secret`
//!      (routes/oauth.rs:327-335). A mismatched secret must be rejected with 401
//!      `invalid_client` even though the client itself is non-confidential.
//!
//! Every test uses `#[sqlx::test(migrator = "db::MIGRATOR")]` so the schema is
//! current, and drives the full Axum router via `TestApp`.

#![allow(dead_code)]

mod common;

use api_server::services::AuthService;
use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use base64::Engine;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use common::{create_authenticated_user, TestApp, TestUser};

// ─── local helpers ───────────────────────────────────────────────────────────

/// Build a PKCE (S256) `(code_verifier, code_challenge)` pair.
fn pkce_pair() -> (String, String) {
    let verifier = format!("test-verifier-{}", Uuid::new_v4());
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let challenge = URL_SAFE_NO_PAD.encode(hasher.finalize());
    (verifier, challenge)
}

/// URL-encode a `key=value` form body.
fn form_body(pairs: &[(&str, &str)]) -> String {
    pairs
        .iter()
        .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
        .collect::<Vec<_>>()
        .join("&")
}

/// POST `application/x-www-form-urlencoded` (unauthenticated).
fn form_request(uri: &str, body: &str) -> Request<Body> {
    Request::builder()
        .method(Method::POST)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(Body::from(body.to_string()))
        .unwrap()
}

/// POST `application/x-www-form-urlencoded` with Bearer auth.
fn form_request_with_auth(uri: &str, body: &str, token: &str) -> Request<Body> {
    Request::builder()
        .method(Method::POST)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::from(body.to_string()))
        .unwrap()
}

/// GET /oauth/authorize with a Bearer token.
fn authorize_get(uri: &str, token: &str) -> Request<Body> {
    Request::builder()
        .method(Method::GET)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap()
}

/// Seed a confidential OAuth client directly in the DB.
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
            (client_id, client_secret_hash, name, redirect_uris, scopes,
             is_confidential, rotate_refresh_tokens)
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

/// Seed a public (non-confidential) OAuth client directly in the DB.
/// Returns `(client_id, redirect_uri)`.
async fn seed_public_client(pool: &PgPool) -> (String, String) {
    let client_id = format!("ci-pub-{}", &Uuid::new_v4().to_string()[..8]);
    let redirect_uri = "https://spa.example.com/callback".to_string();

    sqlx::query(
        r#"
        INSERT INTO oauth_clients
            (client_id, client_secret_hash, name, redirect_uris, scopes,
             is_confidential, rotate_refresh_tokens)
        VALUES
            ($1, 'unused-hash-for-public-client', 'CI Public',
             $2::jsonb, '["profile"]'::jsonb, false, false)
        "#,
    )
    .bind(&client_id)
    .bind(serde_json::json!([redirect_uri]).to_string())
    .execute(pool)
    .await
    .expect("seed public client");

    (client_id, redirect_uri)
}

/// Deactivate an OAuth client (simulates admin revocation).
async fn deactivate_client(pool: &PgPool, client_id: &str) {
    sqlx::query("UPDATE oauth_clients SET is_active = false WHERE client_id = $1")
        .bind(client_id)
        .execute(pool)
        .await
        .expect("deactivate client");
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

// ─── module: token_transport_headers ─────────────────────────────────────────

/// RFC 6749 §5.1 requires token responses to include BOTH `Cache-Control:
/// no-store` AND `Pragma: no-cache`. The companion suites assert the
/// `Cache-Control` header on the authorization-code path; this module pins
/// the `Pragma` header, which is NOT asserted anywhere in the existing suites,
/// for both grant types.
#[cfg(test)]
mod token_transport_headers {
    use super::*;

    /// Authorization-code grant must set both `Cache-Control: no-store` and
    /// `Pragma: no-cache` on the token response (RFC 6749 §5.1).
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_authz_code_token_response_sets_pragma_no_cache(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (user_at, _) = create_authenticated_user(&app, &user).await;
        let (client_id, client_secret, redirect_uri) = seed_confidential_client(&pool).await;
        let (verifier, challenge) = pkce_pair();

        // Consent → code
        let consent_form = form_body(&[
            ("client_id", &client_id),
            ("redirect_uri", &redirect_uri),
            ("scope", "profile"),
            ("code_challenge", &challenge),
            ("code_challenge_method", "S256"),
            ("consent", "approve"),
        ]);
        let code = app
            .execute(form_request_with_auth(
                "/api/v1/oauth/authorize",
                &consent_form,
                &user_at,
            ))
            .await
            .json_value()["code"]
            .as_str()
            .expect("code")
            .to_string();

        // Exchange code for tokens
        let token_body = form_body(&[
            ("grant_type", "authorization_code"),
            ("code", &code),
            ("redirect_uri", &redirect_uri),
            ("client_id", &client_id),
            ("client_secret", &client_secret),
            ("code_verifier", &verifier),
        ]);
        let token_resp = app
            .execute(form_request("/api/v1/oauth/token", &token_body))
            .await;
        assert_eq!(
            token_resp.status,
            StatusCode::OK,
            "token exchange must succeed. body={}",
            token_resp.text()
        );

        // Cache-Control must be no-store (already asserted in sibling suite;
        // assert here too so this test is self-contained).
        let cc = token_resp
            .headers
            .get(header::CACHE_CONTROL)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default();
        assert_eq!(
            cc, "no-store",
            "authorization-code response must set Cache-Control: no-store"
        );

        // Pragma: no-cache is NOT currently asserted in any existing test.
        let pragma = token_resp
            .headers
            .get(header::PRAGMA)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default();
        assert_eq!(
            pragma, "no-cache",
            "authorization-code response must set Pragma: no-cache (RFC 6749 §5.1)"
        );
    }

    /// Refresh-token grant must also include both headers (the token route
    /// sets them unconditionally on success, regardless of grant type).
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_refresh_token_response_sets_pragma_no_cache(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (user_at, _) = create_authenticated_user(&app, &user).await;
        let (client_id, client_secret, redirect_uri) = seed_confidential_client(&pool).await;

        let (_at, rt) =
            confidential_auth_flow(&app, &user_at, &client_id, &client_secret, &redirect_uri).await;

        let refresh_body = form_body(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", &rt),
            ("client_id", &client_id),
            ("client_secret", &client_secret),
        ]);
        let refresh_resp = app
            .execute(form_request("/api/v1/oauth/token", &refresh_body))
            .await;
        assert_eq!(
            refresh_resp.status,
            StatusCode::OK,
            "refresh must succeed. body={}",
            refresh_resp.text()
        );

        let pragma = refresh_resp
            .headers
            .get(header::PRAGMA)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default();
        assert_eq!(
            pragma, "no-cache",
            "refresh-token response must set Pragma: no-cache (RFC 6749 §5.1)"
        );

        let cc = refresh_resp
            .headers
            .get(header::CACHE_CONTROL)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default();
        assert_eq!(
            cc, "no-store",
            "refresh-token response must set Cache-Control: no-store"
        );
    }
}

// ─── module: deactivated_client_at_authorize ─────────────────────────────────

/// The companion suites pin deactivated-client rejection at `/token` and
/// `/introspect`. This module closes the remaining gap: a revoked client must
/// also be rejected at the `/authorize` GET and consent POST, preventing any
/// new authorization codes from being issued for it.
#[cfg(test)]
mod deactivated_client_at_authorize {
    use super::*;

    /// A deactivated client must be rejected at the authorize GET (`validate_authorize_request`
    /// calls `require_active_client` which checks `is_active = true`). The error
    /// code must be `invalid_client` (same as the /token path) with status 400.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_deactivated_client_rejected_at_authorize_get(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (access_token, _) = create_authenticated_user(&app, &user).await;
        let (client_id, redirect_uri) = seed_public_client(&pool).await;
        let (_verifier, challenge) = pkce_pair();

        // Deactivate the client before the authorize request.
        deactivate_client(&pool, &client_id).await;

        let uri = format!(
            "/api/v1/oauth/authorize?response_type=code&client_id={}&redirect_uri={}&scope=profile&code_challenge={}&code_challenge_method=S256",
            urlencoding::encode(&client_id),
            urlencoding::encode(&redirect_uri),
            urlencoding::encode(&challenge),
        );
        let resp = app.execute(authorize_get(&uri, &access_token)).await;
        assert_eq!(
            resp.status,
            StatusCode::BAD_REQUEST,
            "deactivated client at authorize GET must be rejected (400). body={}",
            resp.text()
        );
        assert_eq!(
            resp.json_value()["error"],
            "invalid_client",
            "error must be invalid_client for a deactivated client at authorize GET"
        );
    }

    /// A deactivated client must also be rejected at the consent POST (authorize POST).
    /// The `create_authorization_code` service method internally calls `require_active_client`,
    /// so no code can be issued for a deactivated client even if the user submits consent.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_deactivated_client_rejected_at_consent_post(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (access_token, _) = create_authenticated_user(&app, &user).await;
        let (client_id, redirect_uri) = seed_public_client(&pool).await;
        let (_verifier, challenge) = pkce_pair();

        // Deactivate the client before the consent POST.
        deactivate_client(&pool, &client_id).await;

        let consent_form = form_body(&[
            ("client_id", &client_id),
            ("redirect_uri", &redirect_uri),
            ("scope", "profile"),
            ("code_challenge", &challenge),
            ("code_challenge_method", "S256"),
            ("consent", "approve"),
        ]);
        let resp = app
            .execute(form_request_with_auth(
                "/api/v1/oauth/authorize",
                &consent_form,
                &access_token,
            ))
            .await;
        // The handler maps `OAuthServiceError` from `create_authorization_code` to 400.
        assert_eq!(
            resp.status,
            StatusCode::BAD_REQUEST,
            "deactivated client at consent POST must be rejected (400). body={}",
            resp.text()
        );
        assert_eq!(
            resp.json_value()["error"],
            "invalid_client",
            "error must be invalid_client for a deactivated client at consent POST"
        );
    }

    /// Sanity: a deactivated client must not appear in the active set, so no
    /// authorization code row should be written after the (rejected) consent attempt.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_deactivated_client_no_code_persisted(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (access_token, _) = create_authenticated_user(&app, &user).await;
        let (client_id, redirect_uri) = seed_public_client(&pool).await;
        let (_verifier, challenge) = pkce_pair();

        deactivate_client(&pool, &client_id).await;

        let consent_form = form_body(&[
            ("client_id", &client_id),
            ("redirect_uri", &redirect_uri),
            ("scope", "profile"),
            ("code_challenge", &challenge),
            ("code_challenge_method", "S256"),
            ("consent", "approve"),
        ]);
        let resp = app
            .execute(form_request_with_auth(
                "/api/v1/oauth/authorize",
                &consent_form,
                &access_token,
            ))
            .await;
        assert!(
            resp.status.is_client_error(),
            "deactivated client at consent must be rejected with 4xx. got {}",
            resp.status
        );

        // Verify no authorization code was persisted in the DB for this client.
        let code_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM oauth_authorization_codes WHERE client_id = $1",
        )
        .bind(&client_id)
        .fetch_one(&pool)
        .await
        .expect("count auth codes");
        assert_eq!(
            code_count, 0,
            "no authorization code must be persisted for a deactivated client"
        );
    }
}

// ─── module: revoke_basic_auth ────────────────────────────────────────────────

/// The introspect endpoint's Basic-auth path is covered in `oauth_integration_tests.rs`
/// (`provider_security::test_introspect_accepts_basic_auth_header`). This module
/// pins the equivalent contract for the RFC 7009 revocation endpoint, which uses
/// the same `extract_client_credentials` helper that prefers a Basic header over
/// form-body params.
#[cfg(test)]
mod revoke_basic_auth {
    use super::*;

    /// Revoking a token using HTTP Basic auth credentials (not form-body
    /// `client_id` / `client_secret`) must succeed for a confidential client.
    /// RFC 7009 §2.3 / RFC 6749 §2.3.1 explicitly allow this encoding.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_revoke_accepts_basic_auth_for_confidential_client(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (user_at, _) = create_authenticated_user(&app, &user).await;
        let (client_id, client_secret, redirect_uri) = seed_confidential_client(&pool).await;

        let (oauth_at, _rt) =
            confidential_auth_flow(&app, &user_at, &client_id, &client_secret, &redirect_uri).await;

        // Build HTTP Basic auth: base64(client_id:client_secret)
        let credentials = format!("{}:{}", client_id, client_secret);
        let basic_auth = format!("Basic {}", STANDARD.encode(credentials.as_bytes()));

        // Send only the token in the form body — credentials come from the Basic header.
        let body = form_body(&[("token", &oauth_at), ("token_type_hint", "access_token")]);
        let req = Request::builder()
            .method(Method::POST)
            .uri("/api/v1/oauth/revoke")
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .header(header::AUTHORIZATION, &basic_auth)
            .body(Body::from(body))
            .unwrap();

        let resp = app.execute(req).await;
        assert_eq!(
            resp.status,
            StatusCode::OK,
            "revoke via Basic auth must return 200. body={}",
            resp.text()
        );

        // Verify the token is now inactive via introspection.
        let intro_body = form_body(&[
            ("token", &oauth_at),
            ("client_id", &client_id),
            ("client_secret", &client_secret),
        ]);
        let intro_resp = app
            .execute(form_request("/api/v1/oauth/introspect", &intro_body))
            .await;
        assert_eq!(intro_resp.status, StatusCode::OK);
        assert_eq!(
            intro_resp.json_value()["active"],
            false,
            "token revoked via Basic auth must introspect as inactive"
        );
    }

    /// Basic auth with a WRONG client secret at the revocation endpoint must be
    /// rejected with 401 `invalid_client` — proving that a malformed Basic header
    /// is not silently ignored and does not cause the fallback to form-param parsing.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_revoke_basic_auth_wrong_secret_rejected(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (user_at, _) = create_authenticated_user(&app, &user).await;
        let (client_id, client_secret, redirect_uri) = seed_confidential_client(&pool).await;

        let (oauth_at, _rt) =
            confidential_auth_flow(&app, &user_at, &client_id, &client_secret, &redirect_uri).await;

        // Valid client_id but wrong secret in the Basic header.
        let bad_credentials = format!("{}:{}", client_id, "definitely-not-the-real-secret");
        let bad_basic = format!("Basic {}", STANDARD.encode(bad_credentials.as_bytes()));

        let body = form_body(&[("token", &oauth_at), ("token_type_hint", "access_token")]);
        let req = Request::builder()
            .method(Method::POST)
            .uri("/api/v1/oauth/revoke")
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .header(header::AUTHORIZATION, &bad_basic)
            .body(Body::from(body))
            .unwrap();

        let resp = app.execute(req).await;
        assert_eq!(
            resp.status,
            StatusCode::UNAUTHORIZED,
            "wrong Basic-auth secret at /revoke must return 401. body={}",
            resp.text()
        );
        assert_eq!(
            resp.json_value()["error"],
            "invalid_client",
            "error must be invalid_client for wrong Basic-auth credentials at /revoke"
        );
    }
}

// ─── module: public_client_token_secret_mismatch ─────────────────────────────

/// A public client is not required to supply a `client_secret`. However, if
/// it voluntarily sends one at the token endpoint, the handler validates it
/// (routes/oauth.rs:327-335 — the `else if let Some(client_secret)` branch).
/// A mismatched secret must be rejected with 401 `invalid_client`, ensuring a
/// misconfigured public client surfaces the error rather than silently succeeding.
#[cfg(test)]
mod public_client_token_secret_mismatch {
    use super::*;

    /// Public client that sends a `client_secret` with the wrong value must be
    /// rejected at the token endpoint with 401 `invalid_client`.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_public_client_wrong_secret_rejected_at_token(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (access_token, _) = create_authenticated_user(&app, &user).await;
        let (client_id, redirect_uri) = seed_public_client(&pool).await;
        let (verifier, challenge) = pkce_pair();

        // Consent → code with the public client
        let consent_form = form_body(&[
            ("client_id", &client_id),
            ("redirect_uri", &redirect_uri),
            ("scope", "profile"),
            ("code_challenge", &challenge),
            ("code_challenge_method", "S256"),
            ("consent", "approve"),
        ]);
        let code = app
            .execute(form_request_with_auth(
                "/api/v1/oauth/authorize",
                &consent_form,
                &access_token,
            ))
            .await
            .json_value()["code"]
            .as_str()
            .expect("code")
            .to_string();

        // Token exchange: public client, but with a wrong secret supplied.
        let token_body = form_body(&[
            ("grant_type", "authorization_code"),
            ("code", &code),
            ("redirect_uri", &redirect_uri),
            ("client_id", &client_id),
            ("client_secret", "wrong-secret-for-public-client"),
            ("code_verifier", &verifier),
        ]);
        let token_resp = app
            .execute(form_request("/api/v1/oauth/token", &token_body))
            .await;
        assert_eq!(
            token_resp.status,
            StatusCode::UNAUTHORIZED,
            "public client with wrong secret must be rejected (401). body={}",
            token_resp.text()
        );
        assert_eq!(
            token_resp.json_value()["error"],
            "invalid_client",
            "error must be invalid_client when a public client supplies a mismatched secret"
        );
    }

    /// Positive control: a public client that sends NO `client_secret` at the
    /// token endpoint must succeed (public-client standard behavior).
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_public_client_no_secret_succeeds_at_token(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (access_token, _) = create_authenticated_user(&app, &user).await;
        let (client_id, redirect_uri) = seed_public_client(&pool).await;
        let (verifier, challenge) = pkce_pair();

        let consent_form = form_body(&[
            ("client_id", &client_id),
            ("redirect_uri", &redirect_uri),
            ("scope", "profile"),
            ("code_challenge", &challenge),
            ("code_challenge_method", "S256"),
            ("consent", "approve"),
        ]);
        let code = app
            .execute(form_request_with_auth(
                "/api/v1/oauth/authorize",
                &consent_form,
                &access_token,
            ))
            .await
            .json_value()["code"]
            .as_str()
            .expect("code")
            .to_string();

        // Exchange without client_secret (the normal public-client path).
        let token_body = form_body(&[
            ("grant_type", "authorization_code"),
            ("code", &code),
            ("redirect_uri", &redirect_uri),
            ("client_id", &client_id),
            // no client_secret
            ("code_verifier", &verifier),
        ]);
        let token_resp = app
            .execute(form_request("/api/v1/oauth/token", &token_body))
            .await;
        assert_eq!(
            token_resp.status,
            StatusCode::OK,
            "public client without secret must succeed. body={}",
            token_resp.text()
        );
        assert!(
            token_resp.json_value()["access_token"].is_string(),
            "public client must receive an access_token"
        );
    }
}
