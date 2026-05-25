//! OAuth 2.0 Authorization Server integration tests (Epic 10A, gap-10a-1).
//!
//! Covers:
//!  - Full PKCE S256 authorization code flow (public client)
//!  - Consent / revoke user grant
//!  - Token refresh rotation with family_id reuse detection
//!  - Authorization audit trail (OAuthAuthorize, OAuthRevoke, OAuthTokenDeniedPrincipalKind)
//!
//! Every test uses `#[sqlx::test(migrator = "db::MIGRATOR")]` so the schema
//! is fully up-to-date, and uses `TestApp` so the real Axum router (with all
//! middleware) is exercised end-to-end.

#[allow(dead_code)]
mod common;

use api_server::services::AuthService;
use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use common::{create_authenticated_user, TestApp, TestUser};

// ─── helpers ────────────────────────────────────────────────────────────────

/// Build a PKCE (S256) code_verifier + code_challenge pair.
fn pkce_pair() -> (String, String) {
    let verifier = format!("test-verifier-{}", Uuid::new_v4());
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let challenge = URL_SAFE_NO_PAD.encode(hasher.finalize());
    (verifier, challenge)
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

/// POST application/x-www-form-urlencoded with Bearer auth.
fn form_request_with_auth(uri: &str, body: &str, token: &str) -> Request<Body> {
    Request::builder()
        .method(Method::POST)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::from(body.to_string()))
        .unwrap()
}

/// Seed a confidential OAuth client directly in the DB.
///
/// Returns `(client_id, client_secret, redirect_uri)`.
async fn seed_confidential_client(pool: &PgPool) -> (String, String, String) {
    let client_id = format!("ci-conf-{}", &Uuid::new_v4().to_string()[..8]);
    let redirect_uri = "https://app.example.com/callback".to_string();
    // Use a known plaintext secret so tests can authenticate.
    let plaintext_secret = "test-client-secret-very-secure-12345678";
    // Hash with argon2id (the same algorithm auth_service uses in production).
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

/// Seed a public (non-confidential) OAuth client directly in the DB.
///
/// Returns `(client_id, redirect_uri)`.
async fn seed_public_client(pool: &PgPool) -> (String, String) {
    let client_id = format!("ci-pub-{}", &Uuid::new_v4().to_string()[..8]);
    let redirect_uri = "https://spa.example.com/callback".to_string();
    // Public clients have no meaningful secret; use a placeholder hash.
    let hash = "unused-hash-for-public-client";

    sqlx::query(
        r#"
        INSERT INTO oauth_clients
            (client_id, client_secret_hash, name, redirect_uris, scopes, is_confidential, rotate_refresh_tokens)
        VALUES
            ($1, $2, 'CI Public', $3::jsonb, '["profile"]'::jsonb, false, false)
        "#,
    )
    .bind(&client_id)
    .bind(hash)
    .bind(serde_json::json!([redirect_uri]).to_string())
    .execute(pool)
    .await
    .expect("seed public client");

    (client_id, redirect_uri)
}

/// Count audit log rows by user + action.
async fn count_audit_rows(pool: &PgPool, user_id: Uuid, action: &str) -> i64 {
    sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*) FROM audit_logs
        WHERE user_id = $1 AND action::text = $2
        "#,
    )
    .bind(user_id)
    .bind(action)
    .fetch_one(pool)
    .await
    .unwrap_or(0)
}

// ─── module: pkce_flow ──────────────────────────────────────────────────────

#[cfg(test)]
mod pkce_flow {
    use super::*;

    /// Happy-path: full PKCE S256 authorization code → token exchange for a
    /// public client. Verifies the authorize GET, authorize POST (consent),
    /// and token POST all succeed and that an access token is returned without
    /// a refresh token (public-client behaviour).
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_full_pkce_s256_public_client(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (access_token, _) = create_authenticated_user(&app, &user).await;
        let (client_id, redirect_uri) = seed_public_client(&pool).await;
        let (verifier, challenge) = pkce_pair();
        let state_param = "csrf-state-abc";
        let scope = "profile";

        // 1. GET /authorize — validate request and get consent page data
        let authorize_get_uri = format!(
            "/api/v1/oauth/authorize?response_type=code&client_id={}&redirect_uri={}&scope={}&state={}&code_challenge={}&code_challenge_method=S256",
            urlencoding::encode(&client_id),
            urlencoding::encode(&redirect_uri),
            urlencoding::encode(scope),
            urlencoding::encode(state_param),
            urlencoding::encode(&challenge),
        );
        let get_req = Request::builder()
            .method(Method::GET)
            .uri(&authorize_get_uri)
            .body(Body::empty())
            .unwrap();
        let get_resp = app.execute(get_req).await;
        assert_eq!(
            get_resp.status,
            StatusCode::OK,
            "authorize GET should return 200. body={}",
            get_resp.text()
        );
        let consent_data = get_resp.json_value();
        assert_eq!(consent_data["clientId"], client_id);
        assert!(consent_data["scopes"].is_array());

        // 2. POST /authorize — user approves consent
        let consent_form = form_body(&[
            ("client_id", &client_id),
            ("redirect_uri", &redirect_uri),
            ("scope", scope),
            ("state", state_param),
            ("code_challenge", &challenge),
            ("code_challenge_method", "S256"),
            ("consent", "approve"),
        ]);
        let consent_req =
            form_request_with_auth("/api/v1/oauth/authorize", &consent_form, &access_token);
        let consent_resp = app.execute(consent_req).await;
        assert_eq!(
            consent_resp.status,
            StatusCode::OK,
            "authorize POST should return 200. body={}",
            consent_resp.text()
        );
        let auth_resp = consent_resp.json_value();
        let code = auth_resp["code"]
            .as_str()
            .expect("missing code")
            .to_string();
        assert_eq!(auth_resp["state"], state_param);

        // 3. POST /token — exchange code for tokens
        let token_form = form_body(&[
            ("grant_type", "authorization_code"),
            ("code", &code),
            ("redirect_uri", &redirect_uri),
            ("client_id", &client_id),
            ("code_verifier", &verifier),
        ]);
        let token_req = form_request("/api/v1/oauth/token", &token_form);
        let token_resp = app.execute(token_req).await;
        assert_eq!(
            token_resp.status,
            StatusCode::OK,
            "token exchange should return 200. body={}",
            token_resp.text()
        );
        let tokens = token_resp.json_value();
        assert!(
            tokens["access_token"].is_string(),
            "access_token must be present"
        );
        assert_eq!(tokens["token_type"], "Bearer");
        assert!(tokens["expires_in"].is_number());
        // Public clients must NOT receive a refresh token
        assert!(
            tokens
                .get("refresh_token")
                .map(|v| v.is_null())
                .unwrap_or(true),
            "public client must not receive refresh_token, got: {}",
            tokens
        );
    }

    /// Full PKCE S256 flow for a confidential client — should receive both
    /// access_token and refresh_token.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_full_pkce_s256_confidential_client(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (access_token, _) = create_authenticated_user(&app, &user).await;
        let (client_id, client_secret, redirect_uri) = seed_confidential_client(&pool).await;
        let (verifier, challenge) = pkce_pair();

        // POST /authorize — consent
        let consent_form = form_body(&[
            ("client_id", &client_id),
            ("redirect_uri", &redirect_uri),
            ("scope", "profile email"),
            ("state", "csrf-xyz"),
            ("code_challenge", &challenge),
            ("code_challenge_method", "S256"),
            ("consent", "approve"),
        ]);
        let consent_req =
            form_request_with_auth("/api/v1/oauth/authorize", &consent_form, &access_token);
        let consent_resp = app.execute(consent_req).await;
        assert_eq!(consent_resp.status, StatusCode::OK);
        let code = consent_resp.json_value()["code"]
            .as_str()
            .expect("missing code")
            .to_string();

        // POST /token
        let token_form = form_body(&[
            ("grant_type", "authorization_code"),
            ("code", &code),
            ("redirect_uri", &redirect_uri),
            ("client_id", &client_id),
            ("client_secret", &client_secret),
            ("code_verifier", &verifier),
        ]);
        let token_resp = app
            .execute(form_request("/api/v1/oauth/token", &token_form))
            .await;
        assert_eq!(
            token_resp.status,
            StatusCode::OK,
            "token exchange should return 200. body={}",
            token_resp.text()
        );
        let tokens = token_resp.json_value();
        assert!(tokens["access_token"].is_string());
        assert!(
            tokens["refresh_token"].is_string(),
            "confidential client must receive refresh_token"
        );

        // Response must carry cache-control headers (RFC 6749 §5.1)
        let cc = token_resp
            .headers
            .get(header::CACHE_CONTROL)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default();
        assert_eq!(
            cc, "no-store",
            "token response must set Cache-Control: no-store"
        );
    }

    /// Wrong PKCE verifier must be rejected (InvalidCodeVerifier).
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_pkce_wrong_verifier_rejected(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (access_token, _) = create_authenticated_user(&app, &user).await;
        let (client_id, redirect_uri) = seed_public_client(&pool).await;
        let (_verifier, challenge) = pkce_pair();

        let consent_form = form_body(&[
            ("client_id", &client_id),
            ("redirect_uri", &redirect_uri),
            ("scope", "profile"),
            ("code_challenge", &challenge),
            ("code_challenge_method", "S256"),
            ("consent", "approve"),
        ]);
        let consent_req =
            form_request_with_auth("/api/v1/oauth/authorize", &consent_form, &access_token);
        let code = app.execute(consent_req).await.json_value()["code"]
            .as_str()
            .expect("missing code")
            .to_string();

        // Supply a WRONG verifier
        let token_form = form_body(&[
            ("grant_type", "authorization_code"),
            ("code", &code),
            ("redirect_uri", &redirect_uri),
            ("client_id", &client_id),
            ("code_verifier", "wrong-verifier-does-not-match-challenge"),
        ]);
        let token_resp = app
            .execute(form_request("/api/v1/oauth/token", &token_form))
            .await;
        assert_eq!(
            token_resp.status,
            StatusCode::BAD_REQUEST,
            "wrong verifier must be rejected. body={}",
            token_resp.text()
        );
        let body = token_resp.json_value();
        assert_eq!(body["error"], "invalid_grant");
    }

    /// Public client without code_challenge must be rejected.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_public_client_requires_pkce(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let (client_id, redirect_uri) = seed_public_client(&pool).await;

        let uri = format!(
            "/api/v1/oauth/authorize?response_type=code&client_id={}&redirect_uri={}&scope=profile",
            urlencoding::encode(&client_id),
            urlencoding::encode(&redirect_uri),
        );
        let req = Request::builder()
            .method(Method::GET)
            .uri(&uri)
            .body(Body::empty())
            .unwrap();
        let resp = app.execute(req).await;
        assert_eq!(
            resp.status,
            StatusCode::BAD_REQUEST,
            "public client without PKCE must be rejected. body={}",
            resp.text()
        );
        let body = resp.json_value();
        assert_eq!(body["error"], "invalid_request");
    }

    /// Authorization code replay must be rejected on second use.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_authorization_code_cannot_be_reused(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (access_token, _) = create_authenticated_user(&app, &user).await;
        let (client_id, client_secret, redirect_uri) = seed_confidential_client(&pool).await;
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
            .expect("missing code")
            .to_string();

        let make_token_req = || {
            let body = form_body(&[
                ("grant_type", "authorization_code"),
                ("code", &code),
                ("redirect_uri", &redirect_uri),
                ("client_id", &client_id),
                ("client_secret", &client_secret),
                ("code_verifier", &verifier),
            ]);
            form_request("/api/v1/oauth/token", &body)
        };

        // First use → should succeed
        let first = app.execute(make_token_req()).await;
        assert_eq!(first.status, StatusCode::OK, "first use must succeed");

        // Second use → must be rejected
        let second = app.execute(make_token_req()).await;
        assert_eq!(
            second.status,
            StatusCode::BAD_REQUEST,
            "code replay must be rejected. body={}",
            second.text()
        );
        let body = second.json_value();
        assert_eq!(body["error"], "invalid_grant");
    }
}

// ─── module: consent_revoke ──────────────────────────────────────────────────

#[cfg(test)]
mod consent_revoke {
    use super::*;

    /// User denying consent must return access_denied.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_consent_denied_returns_access_denied(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (access_token, _) = create_authenticated_user(&app, &user).await;
        let (client_id, redirect_uri) = seed_public_client(&pool).await;
        let (_verifier, challenge) = pkce_pair();

        let consent_form = form_body(&[
            ("client_id", &client_id),
            ("redirect_uri", &redirect_uri),
            ("scope", "profile"),
            ("code_challenge", &challenge),
            ("code_challenge_method", "S256"),
            ("consent", "deny"),
        ]);
        let resp = app
            .execute(form_request_with_auth(
                "/api/v1/oauth/authorize",
                &consent_form,
                &access_token,
            ))
            .await;
        assert_eq!(
            resp.status,
            StatusCode::FORBIDDEN,
            "denied consent must return 403. body={}",
            resp.text()
        );
        let body = resp.json_value();
        assert_eq!(body["error"], "access_denied");
    }

    /// After granting, the user can list their grants and then revoke the grant.
    /// After revocation the grant must no longer appear.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_user_can_list_and_revoke_grant(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (access_token, _) = create_authenticated_user(&app, &user).await;
        let (client_id, redirect_uri) = seed_public_client(&pool).await;
        let (_verifier, challenge) = pkce_pair();

        // Grant authorization
        let consent_form = form_body(&[
            ("client_id", &client_id),
            ("redirect_uri", &redirect_uri),
            ("scope", "profile"),
            ("code_challenge", &challenge),
            ("code_challenge_method", "S256"),
            ("consent", "approve"),
        ]);
        let consent_resp = app
            .execute(form_request_with_auth(
                "/api/v1/oauth/authorize",
                &consent_form,
                &access_token,
            ))
            .await;
        assert_eq!(consent_resp.status, StatusCode::OK);

        // List grants
        let list_req = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/oauth/grants")
            .header(header::AUTHORIZATION, format!("Bearer {}", access_token))
            .body(Body::empty())
            .unwrap();
        let list_resp = app.execute(list_req).await;
        assert_eq!(
            list_resp.status,
            StatusCode::OK,
            "list grants must return 200. body={}",
            list_resp.text()
        );
        let grants = list_resp.json_value();
        let arr = grants.as_array().expect("expected array of grants");
        assert!(
            arr.iter().any(|g| g["clientId"] == client_id),
            "grant for {} must appear in list",
            client_id
        );

        // Revoke the grant
        let revoke_req = Request::builder()
            .method(Method::DELETE)
            .uri(format!("/api/v1/oauth/grants/{}", client_id))
            .header(header::AUTHORIZATION, format!("Bearer {}", access_token))
            .body(Body::empty())
            .unwrap();
        let revoke_resp = app.execute(revoke_req).await;
        assert_eq!(
            revoke_resp.status,
            StatusCode::NO_CONTENT,
            "revoke grant must return 204. body={}",
            revoke_resp.text()
        );

        // List again — grant must be gone
        let list_req2 = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/oauth/grants")
            .header(header::AUTHORIZATION, format!("Bearer {}", access_token))
            .body(Body::empty())
            .unwrap();
        let list_resp2 = app.execute(list_req2).await;
        let grants2 = list_resp2.json_value();
        let arr2 = grants2.as_array().expect("expected array of grants");
        assert!(
            !arr2.iter().any(|g| g["clientId"] == client_id),
            "revoked grant must not appear in list"
        );
    }

    /// Revoking a non-existent grant must return 404.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_revoke_nonexistent_grant_returns_404(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (access_token, _) = create_authenticated_user(&app, &user).await;

        let revoke_req = Request::builder()
            .method(Method::DELETE)
            .uri("/api/v1/oauth/grants/nonexistent-client-id")
            .header(header::AUTHORIZATION, format!("Bearer {}", access_token))
            .body(Body::empty())
            .unwrap();
        let resp = app.execute(revoke_req).await;
        assert_eq!(
            resp.status,
            StatusCode::NOT_FOUND,
            "revoking nonexistent grant must return 404. body={}",
            resp.text()
        );
    }

    /// Unauthenticated access to grants list must be rejected.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_grants_list_requires_auth(pool: PgPool) {
        let app = TestApp::new(pool).await;
        let req = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/oauth/grants")
            .body(Body::empty())
            .unwrap();
        let resp = app.execute(req).await;
        assert!(
            resp.status.is_client_error(),
            "unauthenticated grants list must be rejected with 4xx, got {}",
            resp.status
        );
    }
}

// ─── module: refresh_rotation ────────────────────────────────────────────────

#[cfg(test)]
mod refresh_rotation {
    use super::*;

    /// Helper: run the full auth flow for a confidential client and return the
    /// initial access_token + refresh_token.
    async fn auth_flow(
        app: &TestApp,
        access_token: &str,
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
                access_token,
            ))
            .await
            .json_value()["code"]
            .as_str()
            .expect("missing code")
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

    /// Refresh rotation: using a refresh token must yield a new access + refresh
    /// token, and the old refresh token must be invalidated.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_refresh_token_rotation(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (user_at, _) = create_authenticated_user(&app, &user).await;
        let (client_id, client_secret, redirect_uri) = seed_confidential_client(&pool).await;

        let (_at1, rt1) =
            auth_flow(&app, &user_at, &client_id, &client_secret, &redirect_uri).await;

        // Use rt1 to get a new token set
        let refresh_body = form_body(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", &rt1),
            ("client_id", &client_id),
            ("client_secret", &client_secret),
        ]);
        let refresh_resp = app
            .execute(form_request("/api/v1/oauth/token", &refresh_body))
            .await;
        assert_eq!(
            refresh_resp.status,
            StatusCode::OK,
            "first refresh must succeed. body={}",
            refresh_resp.text()
        );
        let tokens2 = refresh_resp.json_value();
        let rt2 = tokens2["refresh_token"].as_str().expect("rt2").to_string();
        assert_ne!(rt1, rt2, "rotated token must differ from original");

        // rt1 must now be invalid (already rotated/revoked)
        let stale_body = form_body(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", &rt1),
            ("client_id", &client_id),
            ("client_secret", &client_secret),
        ]);
        let stale_resp = app
            .execute(form_request("/api/v1/oauth/token", &stale_body))
            .await;
        assert_eq!(
            stale_resp.status,
            StatusCode::BAD_REQUEST,
            "reusing old refresh token must be rejected. body={}",
            stale_resp.text()
        );
        let err = stale_resp.json_value();
        assert_eq!(
            err["error"], "invalid_grant",
            "error must be invalid_grant, got {}",
            err
        );
    }

    /// family_id reuse detection: replaying a revoked refresh token from the
    /// same family must revoke the entire family and return invalid_grant
    /// (TokenReuseDetected).
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_refresh_token_family_reuse_detection(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (user_at, _) = create_authenticated_user(&app, &user).await;
        let (client_id, client_secret, redirect_uri) = seed_confidential_client(&pool).await;

        // Issue initial tokens
        let (_at1, rt1) =
            auth_flow(&app, &user_at, &client_id, &client_secret, &redirect_uri).await;

        // Rotate once: rt1 → rt2
        let rotate1_body = form_body(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", &rt1),
            ("client_id", &client_id),
            ("client_secret", &client_secret),
        ]);
        let tokens2 = app
            .execute(form_request("/api/v1/oauth/token", &rotate1_body))
            .await
            .json_value();
        let rt2 = tokens2["refresh_token"].as_str().expect("rt2").to_string();

        // Rotate again: rt2 → rt3 (valid)
        let rotate2_body = form_body(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", &rt2),
            ("client_id", &client_id),
            ("client_secret", &client_secret),
        ]);
        let tokens3 = app
            .execute(form_request("/api/v1/oauth/token", &rotate2_body))
            .await
            .json_value();
        let rt3 = tokens3["refresh_token"].as_str().expect("rt3").to_string();

        // Now REPLAY rt1 (already revoked) — triggers family revocation
        let replay_body = form_body(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", &rt1),
            ("client_id", &client_id),
            ("client_secret", &client_secret),
        ]);
        let replay_resp = app
            .execute(form_request("/api/v1/oauth/token", &replay_body))
            .await;
        assert_eq!(
            replay_resp.status,
            StatusCode::BAD_REQUEST,
            "replay of revoked family token must be rejected. body={}",
            replay_resp.text()
        );
        let err = replay_resp.json_value();
        assert_eq!(err["error"], "invalid_grant");

        // rt3 (the latest live token in the same family) must ALSO be dead now
        let rt3_body = form_body(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", &rt3),
            ("client_id", &client_id),
            ("client_secret", &client_secret),
        ]);
        let rt3_resp = app
            .execute(form_request("/api/v1/oauth/token", &rt3_body))
            .await;
        assert_eq!(
            rt3_resp.status,
            StatusCode::BAD_REQUEST,
            "all tokens in the family must be revoked after replay detection. body={}",
            rt3_resp.text()
        );
        let err3 = rt3_resp.json_value();
        assert_eq!(err3["error"], "invalid_grant");
    }

    /// Missing refresh_token in refresh request must return invalid_request.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_refresh_missing_token_returns_invalid_request(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let (client_id, client_secret, _redirect_uri) = seed_confidential_client(&pool).await;

        let body = form_body(&[
            ("grant_type", "refresh_token"),
            ("client_id", &client_id),
            ("client_secret", &client_secret),
            // No refresh_token field
        ]);
        let resp = app
            .execute(form_request("/api/v1/oauth/token", &body))
            .await;
        assert_eq!(
            resp.status,
            StatusCode::BAD_REQUEST,
            "missing refresh_token must return 400. body={}",
            resp.text()
        );
        let err = resp.json_value();
        assert_eq!(err["error"], "invalid_request");
    }

    /// Using a refresh token with the wrong client_id must be rejected.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_refresh_wrong_client_rejected(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (user_at, _) = create_authenticated_user(&app, &user).await;
        let (client_id, client_secret, redirect_uri) = seed_confidential_client(&pool).await;
        let (other_client_id, other_client_secret, _) = seed_confidential_client(&pool).await;

        let (_at, rt) = auth_flow(&app, &user_at, &client_id, &client_secret, &redirect_uri).await;

        // Try to use rt with a different client
        let body = form_body(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", &rt),
            ("client_id", &other_client_id),
            ("client_secret", &other_client_secret),
        ]);
        let resp = app
            .execute(form_request("/api/v1/oauth/token", &body))
            .await;
        assert!(
            resp.status.is_client_error(),
            "wrong client must be rejected with 4xx, got {}. body={}",
            resp.status,
            resp.text()
        );
    }
}

// ─── module: audit_trail ─────────────────────────────────────────────────────

#[cfg(test)]
mod audit_trail {
    use super::*;

    /// Approving consent must emit an `oauth_authorize` audit log row.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_authorize_creates_audit_log(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (access_token, _) = create_authenticated_user(&app, &user).await;
        let (client_id, redirect_uri) = seed_public_client(&pool).await;
        let (_verifier, challenge) = pkce_pair();

        // Retrieve the user_id from the DB for later audit check
        let user_id: Uuid = sqlx::query_scalar("SELECT id FROM users WHERE email = $1")
            .bind(&user.email)
            .fetch_one(&pool)
            .await
            .expect("find user");

        let before = count_audit_rows(&pool, user_id, "oauth_authorize").await;

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
        assert_eq!(resp.status, StatusCode::OK);

        let after = count_audit_rows(&pool, user_id, "oauth_authorize").await;
        assert_eq!(
            after,
            before + 1,
            "one oauth_authorize audit row must be added after consent"
        );
    }

    /// Revoking a user grant must emit an `oauth_revoke` audit log row.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_revoke_grant_creates_audit_log(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (access_token, _) = create_authenticated_user(&app, &user).await;
        let (client_id, redirect_uri) = seed_public_client(&pool).await;
        let (_verifier, challenge) = pkce_pair();

        // Grant first
        let consent_form = form_body(&[
            ("client_id", &client_id),
            ("redirect_uri", &redirect_uri),
            ("scope", "profile"),
            ("code_challenge", &challenge),
            ("code_challenge_method", "S256"),
            ("consent", "approve"),
        ]);
        app.execute(form_request_with_auth(
            "/api/v1/oauth/authorize",
            &consent_form,
            &access_token,
        ))
        .await;

        let user_id: Uuid = sqlx::query_scalar("SELECT id FROM users WHERE email = $1")
            .bind(&user.email)
            .fetch_one(&pool)
            .await
            .expect("find user");

        let before = count_audit_rows(&pool, user_id, "oauth_revoke").await;

        // Revoke
        let revoke_req = Request::builder()
            .method(Method::DELETE)
            .uri(format!("/api/v1/oauth/grants/{}", client_id))
            .header(header::AUTHORIZATION, format!("Bearer {}", access_token))
            .body(Body::empty())
            .unwrap();
        let revoke_resp = app.execute(revoke_req).await;
        assert_eq!(revoke_resp.status, StatusCode::NO_CONTENT);

        let after = count_audit_rows(&pool, user_id, "oauth_revoke").await;
        assert_eq!(
            after,
            before + 1,
            "one oauth_revoke audit row must be added after grant revocation"
        );
    }

    /// Token introspection: a valid access token must return active=true with
    /// the correct sub/scope/client_id fields.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_introspect_valid_token(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (user_at, _) = create_authenticated_user(&app, &user).await;
        let (client_id, client_secret, redirect_uri) = seed_confidential_client(&pool).await;
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
                &user_at,
            ))
            .await
            .json_value()["code"]
            .as_str()
            .expect("missing code")
            .to_string();

        let token_body = form_body(&[
            ("grant_type", "authorization_code"),
            ("code", &code),
            ("redirect_uri", &redirect_uri),
            ("client_id", &client_id),
            ("client_secret", &client_secret),
            ("code_verifier", &verifier),
        ]);
        let oauth_at = app
            .execute(form_request("/api/v1/oauth/token", &token_body))
            .await
            .json_value()["access_token"]
            .as_str()
            .expect("access_token")
            .to_string();

        // Introspect with client credentials in form body
        let introspect_body = form_body(&[
            ("token", &oauth_at),
            ("client_id", &client_id),
            ("client_secret", &client_secret),
        ]);
        let introspect_resp = app
            .execute(form_request("/api/v1/oauth/introspect", &introspect_body))
            .await;
        assert_eq!(
            introspect_resp.status,
            StatusCode::OK,
            "introspect must return 200. body={}",
            introspect_resp.text()
        );
        let intro = introspect_resp.json_value();
        assert_eq!(intro["active"], true);
        assert!(intro["sub"].is_string());
        assert_eq!(intro["client_id"], client_id);
        let scope_str = intro["scope"].as_str().unwrap_or_default();
        assert!(scope_str.contains("profile"), "scope must contain profile");
    }

    /// Token introspection: a revoked access token must return active=false.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_introspect_revoked_token_returns_inactive(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (user_at, _) = create_authenticated_user(&app, &user).await;
        let (client_id, client_secret, redirect_uri) = seed_confidential_client(&pool).await;
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
                &user_at,
            ))
            .await
            .json_value()["code"]
            .as_str()
            .expect("code")
            .to_string();

        let token_body = form_body(&[
            ("grant_type", "authorization_code"),
            ("code", &code),
            ("redirect_uri", &redirect_uri),
            ("client_id", &client_id),
            ("client_secret", &client_secret),
            ("code_verifier", &verifier),
        ]);
        let oauth_at = app
            .execute(form_request("/api/v1/oauth/token", &token_body))
            .await
            .json_value()["access_token"]
            .as_str()
            .expect("access_token")
            .to_string();

        // Revoke the access token (RFC 7009)
        let revoke_body = form_body(&[("token", &oauth_at), ("token_type_hint", "access_token")]);
        let revoke_resp = app
            .execute(form_request("/api/v1/oauth/revoke", &revoke_body))
            .await;
        assert_eq!(revoke_resp.status, StatusCode::OK);

        // Introspect — must be inactive
        let introspect_body = form_body(&[
            ("token", &oauth_at),
            ("client_id", &client_id),
            ("client_secret", &client_secret),
        ]);
        let intro = app
            .execute(form_request("/api/v1/oauth/introspect", &introspect_body))
            .await
            .json_value();
        assert_eq!(
            intro["active"], false,
            "revoked token must return active=false, got {}",
            intro
        );
    }
}

// ─── module: token_endpoint_validation ───────────────────────────────────────

#[cfg(test)]
mod token_endpoint_validation {
    use super::*;

    /// Unknown grant_type must return invalid_request.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_unsupported_grant_type(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let (client_id, client_secret, _) = seed_confidential_client(&pool).await;

        let body = form_body(&[
            ("grant_type", "client_credentials"),
            ("client_id", &client_id),
            ("client_secret", &client_secret),
        ]);
        let resp = app
            .execute(form_request("/api/v1/oauth/token", &body))
            .await;
        assert_eq!(
            resp.status,
            StatusCode::BAD_REQUEST,
            "unsupported grant must return 400. body={}",
            resp.text()
        );
        let err = resp.json_value();
        assert_eq!(err["error"], "invalid_request");
    }

    /// Missing client_id must return invalid_request.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_missing_client_id_returns_invalid_request(pool: PgPool) {
        let app = TestApp::new(pool).await;
        let body = form_body(&[
            ("grant_type", "authorization_code"),
            ("code", "some-code"),
            ("redirect_uri", "https://example.com/cb"),
        ]);
        let resp = app
            .execute(form_request("/api/v1/oauth/token", &body))
            .await;
        assert_eq!(
            resp.status,
            StatusCode::BAD_REQUEST,
            "missing client_id must return 400. body={}",
            resp.text()
        );
        let err = resp.json_value();
        assert_eq!(err["error"], "invalid_request");
    }

    /// Unknown client_id must return invalid_client.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_unknown_client_id_returns_invalid_client(pool: PgPool) {
        let app = TestApp::new(pool).await;
        let body = form_body(&[
            ("grant_type", "authorization_code"),
            ("code", "some-code"),
            ("redirect_uri", "https://example.com/cb"),
            ("client_id", "totally-unknown-client"),
        ]);
        let resp = app
            .execute(form_request("/api/v1/oauth/token", &body))
            .await;
        assert_eq!(
            resp.status,
            StatusCode::UNAUTHORIZED,
            "unknown client must return 401. body={}",
            resp.text()
        );
        let err = resp.json_value();
        assert_eq!(err["error"], "invalid_client");
    }

    /// Confidential client with wrong secret must return invalid_client.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_wrong_client_secret_returns_invalid_client(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let (client_id, _correct_secret, _redirect_uri) = seed_confidential_client(&pool).await;

        let body = form_body(&[
            ("grant_type", "authorization_code"),
            ("code", "any-code"),
            ("redirect_uri", "https://app.example.com/callback"),
            ("client_id", &client_id),
            ("client_secret", "WRONG-SECRET"),
        ]);
        let resp = app
            .execute(form_request("/api/v1/oauth/token", &body))
            .await;
        assert_eq!(
            resp.status,
            StatusCode::UNAUTHORIZED,
            "wrong secret must return 401. body={}",
            resp.text()
        );
        let err = resp.json_value();
        assert_eq!(err["error"], "invalid_client");
    }

    /// Confidential client with missing secret must return invalid_client.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_confidential_client_missing_secret_returns_invalid_client(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let (client_id, _correct_secret, _redirect_uri) = seed_confidential_client(&pool).await;

        // Note: no client_secret field
        let body = form_body(&[
            ("grant_type", "authorization_code"),
            ("code", "any-code"),
            ("redirect_uri", "https://app.example.com/callback"),
            ("client_id", &client_id),
        ]);
        let resp = app
            .execute(form_request("/api/v1/oauth/token", &body))
            .await;
        assert_eq!(
            resp.status,
            StatusCode::UNAUTHORIZED,
            "confidential client missing secret must return 401. body={}",
            resp.text()
        );
        let err = resp.json_value();
        assert_eq!(err["error"], "invalid_client");
    }
}
