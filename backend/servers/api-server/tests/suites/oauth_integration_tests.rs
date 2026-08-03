//! OAuth 2.0 Authorization Server integration tests (Epic 10A, gap-10a-1).
//!
//! Covers:
//!  - Full PKCE S256 authorization code flow (public client)
//!  - Consent / revoke user grant
//!  - Token refresh rotation with family_id reuse detection
//!  - Authorization audit trail (OAuthAuthorize, OAuthRevoke, OAuthTokenDeniedPrincipalKind)
//!  - Security: revoked tokens rejected, PKCE plain method rejected, introspect auth enforced
//!  - Epic 10A provider security: PKCE S256 enforcement, revoked-token introspection,
//!    refresh-rotation replay, principal_kind access control, redirect URI binding,
//!    deactivated client rejection, Basic-auth introspection (pm-security-oauth-10a-security-tests)
//!
//! Every test uses `#[sqlx::test(migrator = "db::MIGRATOR")]` so the schema
//! is fully up-to-date, and uses `TestApp` so the real Axum router (with all
//! middleware) is exercised end-to-end.

#![allow(dead_code)]

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

use crate::common::{create_authenticated_user, TestApp, TestUser};

// ─── helpers ───────────────────────────────────────────────────────────────────────────

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

/// GET with Bearer auth. The authorize consent endpoint requires the caller to
/// be authenticated (issue #820), so the GET must carry the access token.
fn get_request_with_auth(uri: &str, token: &str) -> Request<Body> {
    Request::builder()
        .method(Method::GET)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::empty())
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

/// Count `oauth_token_events` rows for a given client + event_type (Epic 10A,
/// #2628). Used to assert the token-lifecycle producer actually writes analytics
/// rows — on `dev` before the producer was wired this table stayed empty for
/// every flow.
async fn count_token_events(pool: &PgPool, client_id: &str, event_type: &str) -> i64 {
    sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*) FROM oauth_token_events
        WHERE client_id = $1 AND event_type = $2
        "#,
    )
    .bind(client_id)
    .bind(event_type)
    .fetch_one(pool)
    .await
    .unwrap_or(0)
}

/// Run the full PKCE S256 authorization-code flow for a **confidential** client
/// and return `(access_token, refresh_token)`.
///
/// Shared by `refresh_rotation` and `provider_security` tests.  Using this
/// helper instead of copy-pasting the consent → token exchange pattern means
/// future changes to the authorize/token endpoints only need updating in one
/// place.
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
        .expect("missing code in confidential_auth_flow")
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
        .expect("access_token missing in confidential_auth_flow")
        .to_string();
    let rt = tokens["refresh_token"]
        .as_str()
        .expect("refresh_token missing in confidential_auth_flow")
        .to_string();
    (at, rt)
}

/// POST the consent form for a PKCE S256 authorize request and return the
/// authorization `code`.
///
/// This covers the most common test setup step: the user approves consent for
/// a given `client_id` / `redirect_uri` / `scope` / `challenge` and the test
/// immediately needs the resulting code.  Centralising it here means the form
/// field list only needs to be maintained in one place.
async fn consent_and_get_code(
    app: &TestApp,
    user_at: &str,
    client_id: &str,
    redirect_uri: &str,
    scope: &str,
    challenge: &str,
) -> String {
    let consent_form = form_body(&[
        ("client_id", client_id),
        ("redirect_uri", redirect_uri),
        ("scope", scope),
        ("code_challenge", challenge),
        ("code_challenge_method", "S256"),
        ("consent", "approve"),
    ]);
    app.execute(form_request_with_auth(
        "/api/v1/oauth/authorize",
        &consent_form,
        user_at,
    ))
    .await
    .json_value()["code"]
        .as_str()
        .expect("missing code in consent_and_get_code")
        .to_string()
}

/// POST the RFC 7009 token-revocation endpoint with client credentials in the
/// form body and return the HTTP status code.
///
/// Call sites only care about whether the revoke succeeded (200) or was
/// rejected; they can assert the returned `StatusCode` directly.
async fn revoke_rfc7009(
    app: &TestApp,
    token: &str,
    token_type_hint: &str,
    client_id: &str,
    client_secret: &str,
) -> StatusCode {
    let body = form_body(&[
        ("token", token),
        ("token_type_hint", token_type_hint),
        ("client_id", client_id),
        ("client_secret", client_secret),
    ]);
    app.execute(form_request("/api/v1/oauth/revoke", &body))
        .await
        .status
}

/// POST the RFC 7662 introspection endpoint with client credentials in the
/// form body and return the parsed JSON response body.
///
/// Most callers only need to inspect `body["active"]`; they can assert the
/// full value from the returned `serde_json::Value`.
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

// ─── module: pkce_flow ───────────────────────────────────────────────────────────────────────

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
        let get_req = get_request_with_auth(&authorize_get_uri, &access_token);
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

        let code = consent_and_get_code(
            &app,
            &access_token,
            &client_id,
            &redirect_uri,
            "profile",
            &challenge,
        )
        .await;

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
        let user = TestUser::new();
        let (access_token, _) = create_authenticated_user(&app, &user).await;
        let (client_id, redirect_uri) = seed_public_client(&pool).await;

        let uri = format!(
            "/api/v1/oauth/authorize?response_type=code&client_id={}&redirect_uri={}&scope=profile",
            urlencoding::encode(&client_id),
            urlencoding::encode(&redirect_uri),
        );
        // Authenticated (issue #820) so the request reaches PKCE validation.
        let req = get_request_with_auth(&uri, &access_token);
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

    /// Regression (issue #823, PR #908): PKCE is mandatory for the
    /// authorization-code flow for *all* clients, not just public ones
    /// (OAuth 2.1 §4.1.1 / RFC 7636). Previously `validate_authorize_request`
    /// only required `code_challenge` when `!is_confidential`, so a confidential
    /// client could obtain an authorization code with no PKCE binding — a code
    /// then exchangeable at `/token` with no `code_verifier` at all (the token
    /// path only checked the verifier `if let Some(challenge)`). A confidential
    /// client authorizing without `code_challenge` must now be rejected with
    /// 400 `invalid_request`.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_confidential_client_requires_pkce(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (access_token, _) = create_authenticated_user(&app, &user).await;
        let (client_id, _client_secret, redirect_uri) = seed_confidential_client(&pool).await;

        // No code_challenge supplied.
        let uri = format!(
            "/api/v1/oauth/authorize?response_type=code&client_id={}&redirect_uri={}&scope=profile",
            urlencoding::encode(&client_id),
            urlencoding::encode(&redirect_uri),
        );
        // Authenticated (issue #820) so the request reaches PKCE validation.
        let req = get_request_with_auth(&uri, &access_token);
        let resp = app.execute(req).await;
        assert_eq!(
            resp.status,
            StatusCode::BAD_REQUEST,
            "confidential client without PKCE must be rejected. body={}",
            resp.text()
        );
        let body = resp.json_value();
        assert_eq!(body["error"], "invalid_request");
    }

    /// Regression (issue #820): the authorize consent GET advertises bearer
    /// auth + a 401 in its OpenAPI but previously took no auth extractor, so it
    /// served the consent page (client name, scopes, redirect URI) to any
    /// anonymous caller. An unauthenticated GET must now be rejected with 401.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_authorize_get_requires_auth(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let (client_id, redirect_uri) = seed_public_client(&pool).await;
        let (_verifier, challenge) = pkce_pair();
        let uri = format!(
            "/api/v1/oauth/authorize?response_type=code&client_id={}&redirect_uri={}&scope=profile&code_challenge={}&code_challenge_method=S256",
            urlencoding::encode(&client_id),
            urlencoding::encode(&redirect_uri),
            urlencoding::encode(&challenge),
        );
        let req = Request::builder()
            .method(Method::GET)
            .uri(&uri)
            .body(Body::empty())
            .unwrap();
        let resp = app.execute(req).await;
        assert_eq!(
            resp.status,
            StatusCode::UNAUTHORIZED,
            "unauthenticated authorize GET must be rejected. body={}",
            resp.text()
        );
    }

    /// PKCE `plain` challenge method must be rejected at the token endpoint —
    /// only S256 is accepted (OAuth 2.1 §4.1.1 / RFC 7636 §4.2).
    ///
    /// The authorize endpoint stores the `code_challenge_method` value without
    /// validation and issues a code normally. The rejection happens at `/token`
    /// via `verify_pkce`, which returns `false` for any method other than S256,
    /// causing the server to return `invalid_grant`.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_pkce_plain_method_rejected(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (access_token, _) = create_authenticated_user(&app, &user).await;
        let (client_id, redirect_uri) = seed_public_client(&pool).await;
        let raw_verifier = format!("plain-verifier-{}", uuid::Uuid::new_v4());

        // Authorize with code_challenge_method=plain. The server accepts this
        // at /authorize (no method validation at consent stage).
        let consent_form = form_body(&[
            ("client_id", &client_id),
            ("redirect_uri", &redirect_uri),
            ("scope", "profile"),
            ("code_challenge", &raw_verifier), // plain: challenge IS the verifier
            ("code_challenge_method", "plain"),
            ("consent", "approve"),
        ]);
        let consent_resp = app
            .execute(form_request_with_auth(
                "/api/v1/oauth/authorize",
                &consent_form,
                &access_token,
            ))
            .await;
        assert_eq!(
            consent_resp.status,
            StatusCode::OK,
            "authorize must issue a code for plain method (validation deferred to /token). body={}",
            consent_resp.text()
        );
        let code = consent_resp.json_value()["code"]
            .as_str()
            .expect("missing code in authorize response")
            .to_string();

        // Token exchange: supply the raw verifier. verify_pkce rejects plain
        // regardless of whether verifier matches challenge, returning invalid_grant.
        let token_form = form_body(&[
            ("grant_type", "authorization_code"),
            ("code", &code),
            ("redirect_uri", &redirect_uri),
            ("client_id", &client_id),
            ("code_verifier", &raw_verifier),
        ]);
        let token_resp = app
            .execute(form_request("/api/v1/oauth/token", &token_form))
            .await;
        assert_eq!(
            token_resp.status,
            StatusCode::BAD_REQUEST,
            "plain-method code exchange must be rejected at /token. body={}",
            token_resp.text()
        );
        let err = token_resp.json_value();
        assert_eq!(
            err["error"], "invalid_grant",
            "plain method rejection must return invalid_grant (RFC 7636), got {}",
            err
        );
    }

    /// Authorization code replay must be rejected on second use.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_authorization_code_cannot_be_reused(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (access_token, _) = create_authenticated_user(&app, &user).await;
        let (client_id, client_secret, redirect_uri) = seed_confidential_client(&pool).await;
        let (verifier, challenge) = pkce_pair();

        let code = consent_and_get_code(
            &app,
            &access_token,
            &client_id,
            &redirect_uri,
            "profile",
            &challenge,
        )
        .await;

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

    /// The consent POST (`authorize_post`) must reject an unauthenticated caller
    /// with 401, symmetric with the authorize GET guard (issue #820). The
    /// endpoint advertises `bearer_auth` + a 401 in its OpenAPI and parses the
    /// acting user from the validated token (routes/oauth.rs:196-207); a missing
    /// Authorization header must be rejected *before* any authorization code is
    /// minted — otherwise an anonymous caller could forge consent on behalf of
    /// whichever user the token would have identified.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_authorize_post_requires_auth(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let (client_id, redirect_uri) = seed_public_client(&pool).await;
        let (_verifier, challenge) = pkce_pair();

        // Well-formed consent form, but NO Authorization header.
        let consent_form = form_body(&[
            ("client_id", &client_id),
            ("redirect_uri", &redirect_uri),
            ("scope", "profile"),
            ("code_challenge", &challenge),
            ("code_challenge_method", "S256"),
            ("consent", "approve"),
        ]);
        let resp = app
            .execute(form_request("/api/v1/oauth/authorize", &consent_form))
            .await;
        assert_eq!(
            resp.status,
            StatusCode::UNAUTHORIZED,
            "unauthenticated authorize POST must be rejected with 401. body={}",
            resp.text()
        );

        // And no authorization code may have been persisted as a side effect.
        let code_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM oauth_authorization_codes WHERE client_id = $1",
        )
        .bind(&client_id)
        .fetch_one(&pool)
        .await
        .expect("count auth codes");
        assert_eq!(
            code_count, 0,
            "no authorization code must be issued for an unauthenticated consent POST"
        );
    }

    /// The authorize GET must reject a `response_type` other than `code`
    /// (routes/oauth.rs:126-131). Only the authorization-code flow is supported;
    /// an implicit-flow `response_type=token` must return 400 `invalid_request`.
    /// The request carries a valid bearer token so the check under test (response
    /// type validation) is reached rather than short-circuited by the #820 auth
    /// guard, which runs first.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_authorize_get_rejects_non_code_response_type(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (access_token, _) = create_authenticated_user(&app, &user).await;
        let (client_id, redirect_uri) = seed_public_client(&pool).await;
        let (_verifier, challenge) = pkce_pair();

        // response_type=token (implicit flow) is not supported.
        let uri = format!(
            "/api/v1/oauth/authorize?response_type=token&client_id={}&redirect_uri={}&scope=profile&code_challenge={}&code_challenge_method=S256",
            urlencoding::encode(&client_id),
            urlencoding::encode(&redirect_uri),
            urlencoding::encode(&challenge),
        );
        let resp = app
            .execute(get_request_with_auth(&uri, &access_token))
            .await;
        assert_eq!(
            resp.status,
            StatusCode::BAD_REQUEST,
            "non-'code' response_type must be rejected with 400. body={}",
            resp.text()
        );
        let err = resp.json_value();
        assert_eq!(
            err["error"], "invalid_request",
            "unsupported response_type must return invalid_request, got {}",
            err
        );
    }
}

// ─── module: consent_revoke ─────────────────────────────────────────────────────────────────────────

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

    /// Regression (issue #756): the consent POST must re-validate the requested
    /// scopes against the client's registered grant before issuing a code.
    /// The public client is seeded with `["profile"]` only, so a consent POST
    /// asking for a scope outside that grant must be rejected with
    /// `invalid_scope` (400) and must NOT issue an authorization code. A request
    /// for the valid subset (`profile`) must still succeed.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_consent_post_rejects_scope_outside_client_grant(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (access_token, _) = create_authenticated_user(&app, &user).await;
        let (client_id, redirect_uri) = seed_public_client(&pool).await;
        let (_verifier, challenge) = pkce_pair();

        // `email` is NOT in the public client's `["profile"]` grant.
        let escalation_form = form_body(&[
            ("client_id", &client_id),
            ("redirect_uri", &redirect_uri),
            ("scope", "profile email"),
            ("code_challenge", &challenge),
            ("code_challenge_method", "S256"),
            ("consent", "approve"),
        ]);
        let resp = app
            .execute(form_request_with_auth(
                "/api/v1/oauth/authorize",
                &escalation_form,
                &access_token,
            ))
            .await;
        assert_eq!(
            resp.status,
            StatusCode::BAD_REQUEST,
            "requesting a scope outside the client grant must be rejected. body={}",
            resp.text()
        );
        let body = resp.json_value();
        assert_eq!(body["error"], "invalid_scope");

        // No authorization code may have been persisted for this client.
        let code_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM oauth_authorization_codes WHERE client_id = $1",
        )
        .bind(&client_id)
        .fetch_one(&pool)
        .await
        .expect("count auth codes");
        assert_eq!(
            code_count, 0,
            "no authorization code must be issued when a scope is rejected"
        );

        // The valid subset (exactly the client's grant) must still succeed.
        let valid_form = form_body(&[
            ("client_id", &client_id),
            ("redirect_uri", &redirect_uri),
            ("scope", "profile"),
            ("code_challenge", &challenge),
            ("code_challenge_method", "S256"),
            ("consent", "approve"),
        ]);
        let ok_resp = app
            .execute(form_request_with_auth(
                "/api/v1/oauth/authorize",
                &valid_form,
                &access_token,
            ))
            .await;
        assert_eq!(
            ok_resp.status,
            StatusCode::OK,
            "a consent POST for the valid scope subset must still succeed. body={}",
            ok_resp.text()
        );
        assert!(
            !ok_resp.json_value()["code"]
                .as_str()
                .unwrap_or("")
                .is_empty(),
            "a non-empty authorization code must be returned for the valid subset"
        );
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

        // Grant authorization (return code is not used further in this test)
        consent_and_get_code(
            &app,
            &access_token,
            &client_id,
            &redirect_uri,
            "profile",
            &challenge,
        )
        .await;

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

// ─── module: refresh_rotation ────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod refresh_rotation {
    use super::*;

    /// Refresh rotation: using a refresh token must yield a new access + refresh
    /// token, and the old refresh token must be invalidated.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_refresh_token_rotation(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (user_at, _) = create_authenticated_user(&app, &user).await;
        let (client_id, client_secret, redirect_uri) = seed_confidential_client(&pool).await;

        let (_at1, rt1) =
            confidential_auth_flow(&app, &user_at, &client_id, &client_secret, &redirect_uri).await;

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
            confidential_auth_flow(&app, &user_at, &client_id, &client_secret, &redirect_uri).await;

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

    /// Explicitly revoked refresh token must be rejected at the token endpoint.
    ///
    /// Distinct from the family-reuse-detection test: here we revoke a single
    /// refresh token via RFC 7009 `/revoke` and then verify the token endpoint
    /// returns `invalid_grant` — ensuring the revocation path itself is wired
    /// correctly, independent of replay/family logic.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_revoked_refresh_token_rejected_at_token_endpoint(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (user_at, _) = create_authenticated_user(&app, &user).await;
        let (client_id, client_secret, redirect_uri) = seed_confidential_client(&pool).await;

        // Obtain an initial refresh token via the full auth flow.
        let (_at, rt) =
            confidential_auth_flow(&app, &user_at, &client_id, &client_secret, &redirect_uri).await;

        // Explicitly revoke the refresh token via RFC 7009 (client-authenticated).
        let revoke_status =
            revoke_rfc7009(&app, &rt, "refresh_token", &client_id, &client_secret).await;
        assert_eq!(
            revoke_status,
            StatusCode::OK,
            "RFC 7009 revoke must return 200"
        );

        // Now attempt to use the revoked refresh token at /token — must be rejected.
        let use_revoked_body = form_body(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", &rt),
            ("client_id", &client_id),
            ("client_secret", &client_secret),
        ]);
        let rejected_resp = app
            .execute(form_request("/api/v1/oauth/token", &use_revoked_body))
            .await;
        assert_eq!(
            rejected_resp.status,
            StatusCode::BAD_REQUEST,
            "revoked refresh token must be rejected with 400. body={}",
            rejected_resp.text()
        );
        let err = rejected_resp.json_value();
        assert_eq!(
            err["error"], "invalid_grant",
            "error must be invalid_grant for a revoked refresh token, got {}",
            err
        );
    }

    /// Using a refresh token with the wrong client_id must be rejected.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_refresh_wrong_client_rejected(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (user_at, _) = create_authenticated_user(&app, &user).await;
        let (client_id, client_secret, redirect_uri) = seed_confidential_client(&pool).await;
        let (other_client_id, other_client_secret, _) = seed_confidential_client(&pool).await;

        let (_at, rt) =
            confidential_auth_flow(&app, &user_at, &client_id, &client_secret, &redirect_uri).await;

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

// ─── module: audit_trail ────────────────────────────────────────────────────────────────────────────

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

        consent_and_get_code(
            &app,
            &access_token,
            &client_id,
            &redirect_uri,
            "profile",
            &challenge,
        )
        .await;

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
        consent_and_get_code(
            &app,
            &access_token,
            &client_id,
            &redirect_uri,
            "profile",
            &challenge,
        )
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

        let (oauth_at, _rt) =
            confidential_auth_flow(&app, &user_at, &client_id, &client_secret, &redirect_uri).await;

        // Introspect with client credentials in form body
        let intro = introspect_with_creds(&app, &oauth_at, &client_id, &client_secret).await;
        assert_eq!(intro["active"], true);
        assert!(intro["sub"].is_string());
        assert_eq!(intro["client_id"], client_id);
        let scope_str = intro["scope"].as_str().unwrap_or_default();
        assert!(scope_str.contains("profile"), "scope must contain profile");
    }

    /// Token introspection without client credentials must return 401.
    ///
    /// RFC 7662 §2.1 requires that the introspection endpoint is accessible
    /// only to authorised resource servers.  Omitting client credentials must
    /// result in a 401 Unauthorized rather than leaking token metadata.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_introspect_requires_client_auth(pool: PgPool) {
        let app = TestApp::new(pool).await;

        // No client_id / client_secret — bare token lookup
        let body = form_body(&[("token", "any-opaque-token-value")]);
        let resp = app
            .execute(form_request("/api/v1/oauth/introspect", &body))
            .await;
        assert_eq!(
            resp.status,
            StatusCode::UNAUTHORIZED,
            "introspect without client credentials must return 401. body={}",
            resp.text()
        );
        let err = resp.json_value();
        assert_eq!(
            err["error"], "invalid_client",
            "error must be invalid_client, got {}",
            err
        );
    }

    /// Token introspection with a valid client_id but the WRONG client_secret
    /// must return 401 `invalid_client` (RFC 7662 §2.1). This is distinct from
    /// the no-credentials case above: here the client is known and active, but
    /// `validate_client_credentials` fails password verification
    /// (services/oauth.rs:285-289), which the handler maps to 401 — no token
    /// metadata may leak to a caller that cannot prove it is the client.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_introspect_wrong_client_secret_returns_invalid_client(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (user_at, _) = create_authenticated_user(&app, &user).await;
        let (client_id, client_secret, redirect_uri) = seed_confidential_client(&pool).await;

        // Mint a genuinely-active token so a credential bypass would be visible
        // as active=true rather than a benign inactive result.
        let (oauth_at, _rt) =
            confidential_auth_flow(&app, &user_at, &client_id, &client_secret, &redirect_uri).await;

        let intro = introspect_with_creds(
            &app,
            &oauth_at,
            &client_id,
            "totally-wrong-client-secret-value",
        )
        .await;
        // introspect_with_creds returns the parsed JSON regardless of status; the
        // handler rejects at auth time, so the body carries the OAuth error.
        assert_eq!(
            intro["error"], "invalid_client",
            "wrong client_secret at introspect must return invalid_client, got {}",
            intro
        );
        // And it must not have leaked the token's active state.
        assert!(
            intro.get("active").is_none(),
            "a rejected introspect must not report token active state, got {}",
            intro
        );
    }

    /// Token introspection of an unknown/opaque token by a validly-authenticated
    /// client must return `active=false` (RFC 7662 §2.2 —
    /// `IntrospectionResponse::inactive()`, services/oauth.rs:621). A token the
    /// server has never issued is indistinguishable from an expired/revoked one,
    /// so the endpoint reports inactive without leaking any metadata.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_introspect_unknown_token_returns_inactive(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let (client_id, client_secret, _redirect_uri) = seed_confidential_client(&pool).await;

        let intro = introspect_with_creds(
            &app,
            "an-opaque-token-that-was-never-issued",
            &client_id,
            &client_secret,
        )
        .await;
        assert_eq!(
            intro["active"], false,
            "an unknown token must introspect as active=false, got {}",
            intro
        );
        assert!(
            intro["sub"].is_null() && intro["scope"].is_null(),
            "no metadata may accompany an inactive introspection result, got {}",
            intro
        );
    }

    /// Token introspection: a live refresh token must return active=true
    /// with the correct `token_type`, `sub`, `scope`, and `client_id` fields.
    ///
    /// Complements `test_introspect_valid_token` which covers access tokens;
    /// this confirms the refresh-token branch in the service is also correct.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_introspect_live_refresh_token_active(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (user_at, _) = create_authenticated_user(&app, &user).await;
        let (client_id, client_secret, redirect_uri) = seed_confidential_client(&pool).await;
        // Use "profile email" scope (both are in the confidential client's grant).
        let (verifier, challenge) = pkce_pair();
        let code = consent_and_get_code(
            &app,
            &user_at,
            &client_id,
            &redirect_uri,
            "profile email",
            &challenge,
        )
        .await;

        let token_body = form_body(&[
            ("grant_type", "authorization_code"),
            ("code", &code),
            ("redirect_uri", &redirect_uri),
            ("client_id", &client_id),
            ("client_secret", &client_secret),
            ("code_verifier", &verifier),
        ]);
        let tokens = app
            .execute(form_request("/api/v1/oauth/token", &token_body))
            .await
            .json_value();
        let oauth_rt = tokens["refresh_token"]
            .as_str()
            .expect("refresh_token")
            .to_string();

        // Introspect the refresh token
        let body = introspect_with_creds(&app, &oauth_rt, &client_id, &client_secret).await;
        assert_eq!(body["active"], true, "live refresh token must be active");
        assert_eq!(body["client_id"], client_id);
        assert!(body["sub"].is_string(), "sub must be present");
        let scope = body["scope"].as_str().unwrap_or_default();
        assert!(scope.contains("profile"), "scope must contain profile");
        assert_eq!(
            body["token_type"], "refresh_token",
            "token_type must be refresh_token"
        );
        assert!(body["exp"].is_number(), "exp must be present");
        assert!(body["iat"].is_number(), "iat must be present");
    }

    /// Token introspection: a revoked access token must return active=false.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_introspect_revoked_token_returns_inactive(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (user_at, _) = create_authenticated_user(&app, &user).await;
        let (client_id, client_secret, redirect_uri) = seed_confidential_client(&pool).await;

        let (oauth_at, _rt) =
            confidential_auth_flow(&app, &user_at, &client_id, &client_secret, &redirect_uri).await;

        // Revoke the access token (RFC 7009, client-authenticated)
        let status =
            revoke_rfc7009(&app, &oauth_at, "access_token", &client_id, &client_secret).await;
        assert_eq!(status, StatusCode::OK);

        // Introspect — must be inactive
        let intro = introspect_with_creds(&app, &oauth_at, &client_id, &client_secret).await;
        assert_eq!(
            intro["active"], false,
            "revoked token must return active=false, got {}",
            intro
        );
    }

    /// Token introspection: a refresh token revoked via RFC 7009 must return
    /// `active=false`.  Exercises the revocation→introspect path for refresh
    /// tokens specifically (the access-token path is covered above).
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_introspect_revoked_refresh_token_inactive(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (user_at, _) = create_authenticated_user(&app, &user).await;
        let (client_id, client_secret, redirect_uri) = seed_confidential_client(&pool).await;

        let (_at, oauth_rt) =
            confidential_auth_flow(&app, &user_at, &client_id, &client_secret, &redirect_uri).await;

        // Revoke the refresh token via RFC 7009 (client-authenticated).
        let status =
            revoke_rfc7009(&app, &oauth_rt, "refresh_token", &client_id, &client_secret).await;
        assert_eq!(status, StatusCode::OK);

        // Introspect — must report active=false.
        let intro = introspect_with_creds(&app, &oauth_rt, &client_id, &client_secret).await;
        assert_eq!(
            intro["active"], false,
            "revoked refresh token must return active=false, got {}",
            intro
        );
    }
}

// ─── module: token_endpoint_validation ──────────────────────────────────────────────────────────────────────────

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

    /// SEC-001 downgrade guard: a *public* client that presents a `client_secret`
    /// at the token endpoint must be rejected with 401 `invalid_client`
    /// (routes/oauth.rs:327-340). Public clients have no usable secret hash, so
    /// the endpoint must refuse the credentialed request outright rather than
    /// routing it into `validate_client_credentials` (which would `verify_password`
    /// against an empty hash and surface a `server_error`). This rejection happens
    /// during client authentication, *before* grant processing, so a bogus `code`
    /// suffices to drive the path.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_public_client_with_secret_rejected_at_token(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let (client_id, redirect_uri) = seed_public_client(&pool).await;

        let body = form_body(&[
            ("grant_type", "authorization_code"),
            ("code", "irrelevant-the-secret-is-rejected-first"),
            ("redirect_uri", &redirect_uri),
            ("client_id", &client_id),
            // A public client must NOT authenticate with a secret.
            (
                "client_secret",
                "some-secret-a-public-client-should-not-have",
            ),
        ]);
        let resp = app
            .execute(form_request("/api/v1/oauth/token", &body))
            .await;
        assert_eq!(
            resp.status,
            StatusCode::UNAUTHORIZED,
            "public client presenting a client_secret must return 401. body={}",
            resp.text()
        );
        let err = resp.json_value();
        assert_eq!(
            err["error"], "invalid_client",
            "error must be invalid_client for a public client that supplies a secret, got {}",
            err
        );
    }
}

// ─── module: provider_security ──────────────────────────────────────────────
//
// Epic 10A OAuth provider security properties (pm-security-oauth-10a-security-tests).
//
// Focuses on three areas called out in the task brief plus supporting cases:
//   A. PKCE S256 enforcement — only S256 accepted; plain rejected end-to-end.
//   B. Revoked-token introspection — RFC 7662 must return active=false for
//      revoked access and refresh tokens; introspect requires client auth.
//   C. Refresh-rotation replay — revoked family member triggers full-family
//      revocation via the family_id mechanism.
//   D. principal_kind access control (Phase 6 C17) — a user whose principal_kind
//      is not in the client's allowed_principal_kinds list is rejected at both
//      code-exchange and refresh time.
//   E. Redirect URI binding — token exchange must reject a redirect URI that
//      differs from the one used at authorization time.
//   F. Deactivated client — a revoked (is_active=false) client is rejected at
//      introspect, token, and authorize endpoints.
//   G. Basic-auth introspection — RFC 7662 client credentials via HTTP Basic
//      header (not just form params) must be accepted.

#[cfg(test)]
mod provider_security {
    use super::*;

    // ── helpers ──────────────────────────────────────────────────────────────

    /// Seed a confidential client restricted to `staff` principal_kind only.
    ///
    /// Returns (client_id, client_secret, redirect_uri).
    async fn seed_staff_only_client(pool: &PgPool) -> (String, String, String) {
        let client_id = format!("ci-staff-{}", &Uuid::new_v4().to_string()[..8]);
        let redirect_uri = "https://staff.example.com/callback".to_string();
        let plaintext_secret = "staff-only-secret-32bytes-abcdefgh";
        let auth = AuthService::new();
        let hash = auth.hash_password(plaintext_secret).expect("hash secret");

        sqlx::query(
            r#"
            INSERT INTO oauth_clients
                (client_id, client_secret_hash, name, redirect_uris, scopes,
                 is_confidential, rotate_refresh_tokens, allowed_principal_kinds)
            VALUES
                ($1, $2, 'Staff Only Client', $3::jsonb, '["profile"]'::jsonb,
                 true, true, ARRAY['staff'])
            "#,
        )
        .bind(&client_id)
        .bind(&hash)
        .bind(serde_json::json!([redirect_uri]).to_string())
        .execute(pool)
        .await
        .expect("seed staff-only client");

        (client_id, plaintext_secret.to_string(), redirect_uri)
    }

    /// Deactivate an OAuth client by setting is_active=false.
    async fn deactivate_client(pool: &PgPool, client_id: &str) {
        sqlx::query("UPDATE oauth_clients SET is_active = false WHERE client_id = $1")
            .bind(client_id)
            .execute(pool)
            .await
            .expect("deactivate client");
    }

    // ── A. PKCE S256 enforcement ─────────────────────────────────────────────

    /// PKCE `plain` method stored during authorize must be rejected at the token
    /// endpoint.  The `verify_pkce` function in `OAuthService` only accepts S256;
    /// any other method (including "plain") returns `false`, causing the server
    /// to return `invalid_grant`.  This test drives the full HTTP path.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_pkce_plain_method_rejected_at_token_exchange(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (access_token, _) = create_authenticated_user(&app, &user).await;
        let (client_id, redirect_uri) = seed_public_client(&pool).await;
        let raw_verifier = format!("plain-verifier-{}", Uuid::new_v4());

        // Authorize with code_challenge_method=plain. The authorize endpoint
        // stores the method verbatim without validating it.
        let consent_form = form_body(&[
            ("client_id", &client_id),
            ("redirect_uri", &redirect_uri),
            ("scope", "profile"),
            ("code_challenge", &raw_verifier), // plain: challenge == verifier
            ("code_challenge_method", "plain"),
            ("consent", "approve"),
        ]);
        let consent_resp = app
            .execute(form_request_with_auth(
                "/api/v1/oauth/authorize",
                &consent_form,
                &access_token,
            ))
            .await;
        assert_eq!(
            consent_resp.status,
            StatusCode::OK,
            "authorize must accept plain at consent stage (validation deferred to /token). body={}",
            consent_resp.text()
        );
        let code = consent_resp.json_value()["code"]
            .as_str()
            .expect("code missing")
            .to_string();

        // Token exchange with the matching verifier — still rejected because
        // `verify_pkce` returns false for any method != S256.
        let token_form = form_body(&[
            ("grant_type", "authorization_code"),
            ("code", &code),
            ("redirect_uri", &redirect_uri),
            ("client_id", &client_id),
            ("code_verifier", &raw_verifier),
        ]);
        let token_resp = app
            .execute(form_request("/api/v1/oauth/token", &token_form))
            .await;
        assert_eq!(
            token_resp.status,
            StatusCode::BAD_REQUEST,
            "plain PKCE method must be rejected at token exchange. body={}",
            token_resp.text()
        );
        let err = token_resp.json_value();
        assert_eq!(
            err["error"], "invalid_grant",
            "error must be invalid_grant for rejected plain verifier, got {}",
            err
        );
    }

    /// No code_verifier supplied for a PKCE-protected code must return invalid_grant.
    /// This tests that the verifier-absence path (as opposed to wrong-verifier path)
    /// is correctly handled.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_pkce_missing_verifier_rejected(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (access_token, _) = create_authenticated_user(&app, &user).await;
        let (client_id, client_secret, redirect_uri) = seed_confidential_client(&pool).await;
        let (_verifier, challenge) = pkce_pair();

        let code = consent_and_get_code(
            &app,
            &access_token,
            &client_id,
            &redirect_uri,
            "profile",
            &challenge,
        )
        .await;

        // Omit code_verifier entirely
        let token_form = form_body(&[
            ("grant_type", "authorization_code"),
            ("code", &code),
            ("redirect_uri", &redirect_uri),
            ("client_id", &client_id),
            ("client_secret", &client_secret),
            // no code_verifier
        ]);
        let token_resp = app
            .execute(form_request("/api/v1/oauth/token", &token_form))
            .await;
        assert_eq!(
            token_resp.status,
            StatusCode::BAD_REQUEST,
            "missing code_verifier must be rejected. body={}",
            token_resp.text()
        );
        let err = token_resp.json_value();
        assert_eq!(err["error"], "invalid_grant");
    }

    /// Defense-in-depth regression (issue #823 / superseded PR #908, landed in
    /// #1025): PKCE is enforced at the *authorize* stage, but the token endpoint
    /// must independently refuse to exchange any stored authorization code that
    /// has **no** `code_challenge` bound to it — see the comment in
    /// `OAuthService::exchange_code_for_tokens`. Such a challenge-less code can
    /// only exist if it was minted before PKCE enforcement, or via a future bug
    /// that bypasses `validate_authorize_request`. Because the live authorize
    /// path now always requires a `code_challenge`, the only way to exercise
    /// this second layer is to insert the row directly.
    ///
    /// Both attempts — with and without a `code_verifier` — must be rejected
    /// with `invalid_grant`, and no tokens must be issued.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_challengeless_stored_code_not_exchangeable(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (_access_token, _) = create_authenticated_user(&app, &user).await;
        let (client_id, client_secret, redirect_uri) = seed_confidential_client(&pool).await;

        // Look up the freshly-created user's id so the seeded code references a
        // real principal (FK on oauth_authorization_codes.user_id).
        let user_id: Uuid = sqlx::query_scalar("SELECT id FROM users WHERE email = $1")
            .bind(&user.email)
            .fetch_one(&pool)
            .await
            .expect("fetch seeded user id");

        // Mint a raw authorization code and store its SHA-256 hash with a NULL
        // code_challenge — i.e. a code with no PKCE binding. This mirrors how
        // OAuthService hashes codes (`hash_token` = hex(sha256(code))).
        let raw_code = format!("legacy-code-{}", Uuid::new_v4());
        let code_hash = {
            let mut hasher = Sha256::new();
            hasher.update(raw_code.as_bytes());
            hex::encode(hasher.finalize())
        };

        sqlx::query(
            r#"
            INSERT INTO oauth_authorization_codes
                (user_id, client_id, code_hash, scopes, redirect_uri,
                 code_challenge, code_challenge_method, expires_at)
            VALUES
                ($1, $2, $3, '["profile"]'::jsonb, $4,
                 NULL, NULL, NOW() + INTERVAL '10 minutes')
            "#,
        )
        .bind(user_id)
        .bind(&client_id)
        .bind(&code_hash)
        .bind(&redirect_uri)
        .execute(&pool)
        .await
        .expect("seed challenge-less authorization code");

        // Attempt 1: exchange WITHOUT a code_verifier.
        let token_form_no_verifier = form_body(&[
            ("grant_type", "authorization_code"),
            ("code", &raw_code),
            ("redirect_uri", &redirect_uri),
            ("client_id", &client_id),
            ("client_secret", &client_secret),
        ]);
        let resp = app
            .execute(form_request("/api/v1/oauth/token", &token_form_no_verifier))
            .await;
        assert_eq!(
            resp.status,
            StatusCode::BAD_REQUEST,
            "challenge-less code must not be exchangeable (no verifier). body={}",
            resp.text()
        );
        assert_eq!(resp.json_value()["error"], "invalid_grant");

        // The first exchange attempt atomically consumed the code, so a re-seed
        // is needed to prove the second (with-verifier) path independently.
        let raw_code2 = format!("legacy-code-{}", Uuid::new_v4());
        let code_hash2 = {
            let mut hasher = Sha256::new();
            hasher.update(raw_code2.as_bytes());
            hex::encode(hasher.finalize())
        };
        sqlx::query(
            r#"
            INSERT INTO oauth_authorization_codes
                (user_id, client_id, code_hash, scopes, redirect_uri,
                 code_challenge, code_challenge_method, expires_at)
            VALUES
                ($1, $2, $3, '["profile"]'::jsonb, $4,
                 NULL, NULL, NOW() + INTERVAL '10 minutes')
            "#,
        )
        .bind(user_id)
        .bind(&client_id)
        .bind(&code_hash2)
        .bind(&redirect_uri)
        .execute(&pool)
        .await
        .expect("re-seed challenge-less authorization code");

        // Attempt 2: exchange WITH an arbitrary code_verifier. A challenge-less
        // stored code must still be rejected — the verifier has nothing valid to
        // match against, so the token endpoint refuses it rather than treating
        // "no challenge" as "PKCE not required".
        let token_form_with_verifier = form_body(&[
            ("grant_type", "authorization_code"),
            ("code", &raw_code2),
            ("redirect_uri", &redirect_uri),
            ("client_id", &client_id),
            ("client_secret", &client_secret),
            ("code_verifier", "any-verifier-the-attacker-supplies"),
        ]);
        let resp2 = app
            .execute(form_request(
                "/api/v1/oauth/token",
                &token_form_with_verifier,
            ))
            .await;
        assert_eq!(
            resp2.status,
            StatusCode::BAD_REQUEST,
            "challenge-less code must not be exchangeable (with verifier). body={}",
            resp2.text()
        );
        assert_eq!(resp2.json_value()["error"], "invalid_grant");

        // And no access tokens may have been minted for this user as a result.
        let issued: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM oauth_access_tokens WHERE user_id = $1")
                .bind(user_id)
                .fetch_one(&pool)
                .await
                .expect("count access tokens");
        assert_eq!(
            issued, 0,
            "no access token must be issued from a challenge-less authorization code"
        );
    }

    // ── B. Revoked-token introspection ───────────────────────────────────────

    /// Introspecting a revoked access token must return active=false.
    ///
    /// This is the end-to-end path: issue token → revoke via RFC 7009 →
    /// introspect → verify active=false.  Covers the Epic 10A provider's
    /// introspect_token implementation which checks is_valid() on the stored row.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_introspect_revoked_access_token_returns_inactive(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (user_at, _) = create_authenticated_user(&app, &user).await;
        let (client_id, client_secret, redirect_uri) = seed_confidential_client(&pool).await;

        let (oauth_at, _rt) =
            confidential_auth_flow(&app, &user_at, &client_id, &client_secret, &redirect_uri).await;

        // Revoke the access token via RFC 7009 (client-authenticated)
        let revoke_status =
            revoke_rfc7009(&app, &oauth_at, "access_token", &client_id, &client_secret).await;
        assert_eq!(revoke_status, StatusCode::OK, "revoke must return 200");

        // Introspect the now-revoked token
        let intro = introspect_with_creds(&app, &oauth_at, &client_id, &client_secret).await;
        assert_eq!(
            intro["active"], false,
            "revoked access token must introspect as active=false, got {}",
            intro
        );
    }

    /// Introspecting a revoked refresh token must return active=false.
    ///
    /// Covers the refresh-token branch of introspect_token which calls
    /// find_refresh_token_by_hash (only non-revoked) — a revoked row is not
    /// returned, so the server falls through to inactive().
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_introspect_revoked_refresh_token_returns_inactive(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (user_at, _) = create_authenticated_user(&app, &user).await;
        let (client_id, client_secret, redirect_uri) = seed_confidential_client(&pool).await;

        let (_at, rt) =
            confidential_auth_flow(&app, &user_at, &client_id, &client_secret, &redirect_uri).await;

        // Revoke the refresh token via RFC 7009 (client-authenticated)
        let revoke_status =
            revoke_rfc7009(&app, &rt, "refresh_token", &client_id, &client_secret).await;
        assert_eq!(revoke_status, StatusCode::OK);

        // Introspect the now-revoked refresh token
        let intro = introspect_with_creds(&app, &rt, &client_id, &client_secret).await;
        assert_eq!(
            intro["active"], false,
            "revoked refresh token must introspect as active=false, got {}",
            intro
        );
    }

    /// Introspection endpoint must reject requests without client authentication
    /// (RFC 7662 §2.1: protected resource or client must authenticate).
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_introspect_requires_client_credentials(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (user_at, _) = create_authenticated_user(&app, &user).await;
        let (client_id, client_secret, redirect_uri) = seed_confidential_client(&pool).await;

        let (oauth_at, _rt) =
            confidential_auth_flow(&app, &user_at, &client_id, &client_secret, &redirect_uri).await;

        // Introspect with no credentials — only the token field
        let body_no_creds = form_body(&[("token", &oauth_at)]);
        let resp_no_creds = app
            .execute(form_request("/api/v1/oauth/introspect", &body_no_creds))
            .await;
        assert_eq!(
            resp_no_creds.status,
            StatusCode::UNAUTHORIZED,
            "introspect without client auth must return 401. body={}",
            resp_no_creds.text()
        );
        let err = resp_no_creds.json_value();
        assert_eq!(
            err["error"], "invalid_client",
            "error must be invalid_client, got {}",
            err
        );
    }

    /// Introspection must accept client credentials via HTTP Basic auth header
    /// (RFC 7662 §2.1 / RFC 6749 §2.3.1), not just form-body params.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_introspect_accepts_basic_auth_header(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (user_at, _) = create_authenticated_user(&app, &user).await;
        let (client_id, client_secret, redirect_uri) = seed_confidential_client(&pool).await;

        let (oauth_at, _rt) =
            confidential_auth_flow(&app, &user_at, &client_id, &client_secret, &redirect_uri).await;

        // Build Basic auth header: base64(client_id:client_secret) using STANDARD engine
        let credentials = format!("{}:{}", client_id, client_secret);
        let encoded = STANDARD.encode(credentials.as_bytes());
        let basic_auth = format!("Basic {}", encoded);

        // Send only the token in the form body — credentials come from Basic header
        let body = form_body(&[("token", &oauth_at)]);
        let req = Request::builder()
            .method(Method::POST)
            .uri("/api/v1/oauth/introspect")
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .header(header::AUTHORIZATION, &basic_auth)
            .body(Body::from(body))
            .unwrap();

        let resp = app.execute(req).await;
        assert_eq!(
            resp.status,
            StatusCode::OK,
            "introspect with Basic auth must return 200. body={}",
            resp.text()
        );
        let intro = resp.json_value();
        assert_eq!(
            intro["active"], true,
            "valid token introspected via Basic auth must be active=true, got {}",
            intro
        );
        assert!(intro["sub"].is_string(), "sub must be present");
        assert_eq!(intro["client_id"], client_id);
    }

    // ── C. Refresh-rotation replay block ─────────────────────────────────────

    /// Replaying a revoked refresh token (family-reuse) must:
    ///   1. Return invalid_grant immediately.
    ///   2. Revoke ALL tokens in the same family so no other family member
    ///      can be used (defense-in-depth per RFC 9700 §5.2).
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_refresh_family_replay_revokes_entire_family(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (user_at, _) = create_authenticated_user(&app, &user).await;
        let (client_id, client_secret, redirect_uri) = seed_confidential_client(&pool).await;

        // Initial token issuance
        let (_at1, rt1) =
            confidential_auth_flow(&app, &user_at, &client_id, &client_secret, &redirect_uri).await;

        // Rotate rt1 → rt2
        let rotate_body = form_body(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", &rt1),
            ("client_id", &client_id),
            ("client_secret", &client_secret),
        ]);
        let tokens2 = app
            .execute(form_request("/api/v1/oauth/token", &rotate_body))
            .await
            .json_value();
        let rt2 = tokens2["refresh_token"].as_str().expect("rt2").to_string();

        // Rotate rt2 → rt3 (rt2 is now spent)
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

        // Replay rt1 (already spent after first rotation) — triggers family revocation
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
            "replay of revoked family member must return 400. body={}",
            replay_resp.text()
        );
        assert_eq!(replay_resp.json_value()["error"], "invalid_grant");

        // rt3 (the latest live token in the family) must also be dead now
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
            "all family members must be revoked after replay detection. body={}",
            rt3_resp.text()
        );
        assert_eq!(rt3_resp.json_value()["error"], "invalid_grant");
    }

    // ── D. principal_kind access control (Phase 6 C17) ───────────────────────

    /// A user registered via the normal staff flow has principal_kind='staff'.
    /// A client restricted to allowed_principal_kinds=['staff'] must accept them.
    /// This is the positive case for principal_kind enforcement.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_principal_kind_staff_allowed_on_staff_client(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        // Staff users are created via the normal registration flow (principal_kind='staff')
        let user = TestUser::new();
        let (user_at, _) = create_authenticated_user(&app, &user).await;
        let (client_id, client_secret, redirect_uri) = seed_staff_only_client(&pool).await;
        let (verifier, challenge) = pkce_pair();

        // Authorize — consent_and_get_code panics if code is absent, which
        // serves as an implicit 200-and-code assertion.
        let code = consent_and_get_code(
            &app,
            &user_at,
            &client_id,
            &redirect_uri,
            "profile",
            &challenge,
        )
        .await;

        // Token exchange must succeed
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
            "staff user on staff-only client must get tokens. body={}",
            token_resp.text()
        );
        assert!(token_resp.json_value()["access_token"].is_string());
    }

    /// A user whose principal_kind is not in the client's allowed_principal_kinds list
    /// must be rejected at the token endpoint (code exchange) with HTTP 403 access_denied.
    ///
    /// Strategy: INSERT a portal user with principal_kind='public' directly into the DB
    /// (the BEFORE UPDATE guard only applies to UPDATEs, not INSERTs), then generate a
    /// valid JWT for them using the same test secret that TestApp uses.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_principal_kind_mismatch_rejected_at_token_exchange(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let (client_id, client_secret, redirect_uri) = seed_staff_only_client(&pool).await;

        // Insert a portal user directly with principal_kind='public'.
        // INSERTs are not subject to the BEFORE UPDATE principal_kind guard.
        let public_user_id = Uuid::new_v4();
        let public_email = format!("portal-{}@example.com", &public_user_id.to_string()[..8]);
        let auth_svc = AuthService::new();
        let pw_hash = auth_svc.hash_password("PortalPass123!").expect("hash");
        sqlx::query(
            r#"
            INSERT INTO users
                (id, email, password_hash, name, status, principal_kind, email_verified_at)
            VALUES ($1, $2, $3, 'Portal User', 'active', 'public', NOW())
            "#,
        )
        .bind(public_user_id)
        .bind(&public_email)
        .bind(&pw_hash)
        .execute(&pool)
        .await
        .expect("insert public user");

        // Generate a valid JWT for this public user using the TestApp's
        // configured secret. Reading it back off `app.config` keeps the test
        // honest if `TestConfig` ever rotates its default — issue #704 R4.
        let jwt_svc = api_server::services::JwtService::new(&app.config.jwt_secret)
            .expect("jwt service for test");
        let public_at = jwt_svc
            .generate_access_token(public_user_id, &public_email, "Portal User", None, None)
            .expect("access token for public user");

        let (verifier, challenge) = pkce_pair();

        // Authorize — principal_kind is not checked at the consent stage.
        let code = consent_and_get_code(
            &app,
            &public_at,
            &client_id,
            &redirect_uri,
            "profile",
            &challenge,
        )
        .await;

        // Token exchange must fail with 403: principal_kind='public' is not in
        // allowed_principal_kinds=['staff'] for this client.
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
            StatusCode::FORBIDDEN,
            "principal_kind='public' on staff-only client must return 403. body={}",
            token_resp.text()
        );
        let err = token_resp.json_value();
        assert_eq!(
            err["error"], "access_denied",
            "error must be access_denied for principal_kind mismatch, got {}",
            err
        );
    }

    /// R3 (issue #704): `OAuthService::refresh_tokens` re-checks
    /// `allowed_principal_kinds` so a token issued while the user's kind was
    /// permitted cannot be refreshed once that kind is removed from the
    /// client's policy (e.g. an admin tightens access after issuance).
    ///
    /// Strategy: a normal staff user gets tokens from a client whose default
    /// `allowed_principal_kinds=['public','staff','platform']` permits staff,
    /// then we mutate the client's policy down to `['public']` only and
    /// attempt to refresh. There is no DB trigger on `oauth_clients` so the
    /// UPDATE proceeds without GUC gymnastics. The expected response is
    /// HTTP 403 with `error=access_denied`, mirroring the code-exchange path.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_principal_kind_mismatch_rejected_at_refresh(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (user_at, _) = create_authenticated_user(&app, &user).await;
        // Default allowed_principal_kinds covers 'staff', so initial issuance succeeds.
        let (client_id, client_secret, redirect_uri) = seed_confidential_client(&pool).await;

        let (_at, rt) =
            confidential_auth_flow(&app, &user_at, &client_id, &client_secret, &redirect_uri).await;

        // Tighten policy: client now only allows 'public'. The staff user's
        // kind ('staff') is no longer in the allowed set, but they hold a
        // live refresh token issued under the old policy.
        sqlx::query(
            "UPDATE oauth_clients SET allowed_principal_kinds = ARRAY['public'] WHERE client_id = $1",
        )
        .bind(&client_id)
        .execute(&pool)
        .await
        .expect("tighten allowed_principal_kinds");

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
            StatusCode::FORBIDDEN,
            "refresh with newly-disallowed principal_kind must return 403. body={}",
            refresh_resp.text()
        );
        let err = refresh_resp.json_value();
        assert_eq!(
            err["error"], "access_denied",
            "error must be access_denied at refresh principal_kind mismatch, got {}",
            err
        );
    }

    // ── E. Redirect URI binding ───────────────────────────────────────────────

    /// The redirect URI used at token exchange must exactly match the one recorded
    /// in the authorization code.  Using a different URI must return invalid_grant.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_redirect_uri_mismatch_rejected_at_token_exchange(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (access_token, _) = create_authenticated_user(&app, &user).await;
        let (client_id, client_secret, redirect_uri) = seed_confidential_client(&pool).await;
        let (verifier, challenge) = pkce_pair();

        // Authorize with the registered redirect URI
        let code = consent_and_get_code(
            &app,
            &access_token,
            &client_id,
            &redirect_uri,
            "profile",
            &challenge,
        )
        .await;

        // Token exchange with a DIFFERENT redirect URI
        let wrong_uri = "https://attacker.example.com/steal";
        let token_form = form_body(&[
            ("grant_type", "authorization_code"),
            ("code", &code),
            ("redirect_uri", wrong_uri), // ← mismatch
            ("client_id", &client_id),
            ("client_secret", &client_secret),
            ("code_verifier", &verifier),
        ]);
        let token_resp = app
            .execute(form_request("/api/v1/oauth/token", &token_form))
            .await;
        assert_eq!(
            token_resp.status,
            StatusCode::BAD_REQUEST,
            "redirect URI mismatch must be rejected. body={}",
            token_resp.text()
        );
        let err = token_resp.json_value();
        assert_eq!(
            err["error"], "invalid_grant",
            "error must be invalid_grant for redirect URI mismatch, got {}",
            err
        );
    }

    // ── F. Deactivated client ────────────────────────────────────────────────

    /// A revoked (is_active=false) client must be rejected at the token endpoint.
    /// Tokens issued before revocation cannot be refreshed via a dead client.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_deactivated_client_rejected_at_token_endpoint(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (user_at, _) = create_authenticated_user(&app, &user).await;
        let (client_id, client_secret, redirect_uri) = seed_confidential_client(&pool).await;

        // Issue initial tokens while client is active
        let (_at, rt) =
            confidential_auth_flow(&app, &user_at, &client_id, &client_secret, &redirect_uri).await;

        // Now deactivate the client (simulates admin revocation via /admin/oauth/clients/{id})
        deactivate_client(&pool, &client_id).await;

        // Attempt to use the refresh token — client is no longer active
        let refresh_body = form_body(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", &rt),
            ("client_id", &client_id),
            ("client_secret", &client_secret),
        ]);
        let refresh_resp = app
            .execute(form_request("/api/v1/oauth/token", &refresh_body))
            .await;
        // R2 (issue #704): mirror the precision of the introspect-side
        // assertion. The token endpoint looks the client up via
        // `find_active_client_by_client_id` BEFORE reaching the refresh-token
        // lookup (routes/oauth.rs:297-307), so a deactivated client must
        // produce 401 + `invalid_client` — not any 4xx. Asserting the exact
        // status + error pins the order of checks so a future regression
        // that hits the refresh-token lookup first (and returns 400
        // invalid_grant) cannot pass silently.
        assert_eq!(
            refresh_resp.status,
            StatusCode::UNAUTHORIZED,
            "deactivated client at token endpoint must return 401. body={}",
            refresh_resp.text()
        );
        let err = refresh_resp.json_value();
        assert_eq!(
            err["error"], "invalid_client",
            "error must be invalid_client for deactivated client at token endpoint, got {}",
            err
        );
    }

    /// A revoked client must be rejected at the introspection endpoint.
    /// `validate_client_credentials` calls `find_active_client_by_client_id`
    /// which filters by is_active=true, so a deactivated client cannot authenticate.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_deactivated_client_rejected_at_introspect(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (user_at, _) = create_authenticated_user(&app, &user).await;
        let (client_id, client_secret, redirect_uri) = seed_confidential_client(&pool).await;

        // Issue a token while the client is active
        let (oauth_at, _rt) =
            confidential_auth_flow(&app, &user_at, &client_id, &client_secret, &redirect_uri).await;

        // Deactivate the client
        deactivate_client(&pool, &client_id).await;

        // Introspect — client credentials must be rejected because client is inactive
        let introspect_body = form_body(&[
            ("token", &oauth_at),
            ("client_id", &client_id),
            ("client_secret", &client_secret),
        ]);
        let resp = app
            .execute(form_request("/api/v1/oauth/introspect", &introspect_body))
            .await;
        assert_eq!(
            resp.status,
            StatusCode::UNAUTHORIZED,
            "deactivated client must be rejected at introspect (got {}). body={}",
            resp.status,
            resp.text()
        );
        let err = resp.json_value();
        assert_eq!(
            err["error"], "invalid_client",
            "error must be invalid_client for deactivated client, got {}",
            err
        );
    }

    // ── C. Revocation endpoint authentication (#823 / RFC 7009 §2.1) ─────────

    /// The revocation endpoint must require client authentication. An
    /// unauthenticated caller (no client_id / client_secret) must be rejected
    /// with 401 and MUST NOT revoke the token. Before the fix the endpoint took
    /// any token from any caller and revoked it — a trivial unauthenticated DoS.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_revoke_without_client_auth_is_rejected_and_no_op(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (user_at, _) = create_authenticated_user(&app, &user).await;
        let (client_id, client_secret, redirect_uri) = seed_confidential_client(&pool).await;

        let (oauth_at, _rt) =
            confidential_auth_flow(&app, &user_at, &client_id, &client_secret, &redirect_uri).await;

        // Revoke WITHOUT any client credentials.
        let revoke_body = form_body(&[("token", &oauth_at), ("token_type_hint", "access_token")]);
        let resp = app
            .execute(form_request("/api/v1/oauth/revoke", &revoke_body))
            .await;
        assert_eq!(
            resp.status,
            StatusCode::UNAUTHORIZED,
            "unauthenticated revoke must return 401. body={}",
            resp.text()
        );

        // The token must still be active — the unauthenticated call must be a
        // no-op, not a successful revocation.
        let intro = introspect_with_creds(&app, &oauth_at, &client_id, &client_secret).await;
        assert_eq!(
            intro["active"], true,
            "token must remain active after an unauthenticated revoke attempt, got {}",
            intro
        );
    }

    /// A client must not be able to revoke another client's token. The
    /// revocation endpoint authenticates the caller, but the token belongs to a
    /// different client, so the revoke must be a no-op (200 per RFC 7009, but
    /// the victim token stays active).
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_revoke_cross_client_token_is_no_op(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (user_at, _) = create_authenticated_user(&app, &user).await;

        // Victim client issues a token.
        let (victim_id, victim_secret, victim_redirect) = seed_confidential_client(&pool).await;
        let (victim_at, _rt) =
            confidential_auth_flow(&app, &user_at, &victim_id, &victim_secret, &victim_redirect)
                .await;

        // Attacker client authenticates with its OWN valid credentials and tries
        // to revoke the victim's token.
        let (attacker_id, attacker_secret, _attacker_redirect) =
            seed_confidential_client(&pool).await;
        let cross_revoke_status = revoke_rfc7009(
            &app,
            &victim_at,
            "access_token",
            &attacker_id,
            &attacker_secret,
        )
        .await;
        assert_eq!(
            cross_revoke_status,
            StatusCode::OK,
            "RFC 7009 returns 200 even when the token is not the caller's."
        );

        // Victim token must still be active — the cross-client revoke is a no-op.
        let intro = introspect_with_creds(&app, &victim_at, &victim_id, &victim_secret).await;
        assert_eq!(
            intro["active"], true,
            "another client must not be able to revoke this token, got {}",
            intro
        );
    }

    /// Introspection must be bound to the authenticating client. A client that
    /// authenticates correctly but introspects a token belonging to a different
    /// client must get `active=false` (RFC 7662 §2.2) — no cross-client metadata
    /// leak (scope / sub / expiry).
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_introspect_cross_client_token_reports_inactive(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (user_at, _) = create_authenticated_user(&app, &user).await;

        // Owner client issues a token.
        let (owner_id, owner_secret, owner_redirect) = seed_confidential_client(&pool).await;
        let (owner_at, _rt) =
            confidential_auth_flow(&app, &user_at, &owner_id, &owner_secret, &owner_redirect).await;

        // Sanity: the owner sees its own token as active.
        let owner_intro = introspect_with_creds(&app, &owner_at, &owner_id, &owner_secret).await;
        assert_eq!(
            owner_intro["active"], true,
            "owner must see its own token active, got {}",
            owner_intro
        );

        // A different (validly authenticated) client introspects the same token.
        let (other_id, other_secret, _other_redirect) = seed_confidential_client(&pool).await;
        let other_intro = introspect_with_creds(&app, &owner_at, &other_id, &other_secret).await;
        assert_eq!(
            other_intro["active"], false,
            "a token belonging to another client must introspect as inactive, got {}",
            other_intro
        );
        assert!(
            other_intro["scope"].is_null() && other_intro["sub"].is_null(),
            "no token metadata may leak across clients, got {}",
            other_intro
        );
    }
}

// ─── module: admin_client_audit ─────────────────────────────────────────────
//
// Findings #768 / #800: privileged OAuth-client mutations on the admin surface
// must be audited. `update_client` (PATCH /api/v1/admin/oauth/clients/{id})
// previously wrote NO audit_log row at all and silently dropped the operator
// `reason` the admin-web UI requires for scope edits. These tests pin the
// regression: an `oauth_client_update` row is written, and the supplied
// `reason` is persisted into its `details`.
#[cfg(test)]
mod admin_client_audit {
    use super::*;
    use api_server::services::JwtService;

    const TEST_JWT_SECRET: &str =
        "test-secret-key-that-is-at-least-64-characters-long-for-testing-purposes";

    /// Seed a platform-principal user. Returns the new user id.
    async fn seed_platform_user(pool: &PgPool, email: &str) -> Uuid {
        sqlx::query_scalar::<_, Uuid>(
            r#"
            INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
            VALUES ($1, 'h', 'OAuth Admin Test', 'active', NOW(), 'platform')
            RETURNING id
            "#,
        )
        .bind(email)
        .fetch_one(pool)
        .await
        .expect("seed platform user")
    }

    /// Mark the user as MFA-enrolled (the capability gate's step 2.5 wall).
    async fn enroll_mfa(pool: &PgPool, user_id: Uuid) {
        sqlx::query(
            r#"
            INSERT INTO user_2fa (user_id, secret, enabled, enabled_at, backup_codes, backup_codes_remaining)
            VALUES ($1, 'unused-secret', true, NOW(), '[]'::jsonb, 0)
            ON CONFLICT (user_id) DO UPDATE SET enabled = true, enabled_at = NOW()
            "#,
        )
        .bind(user_id)
        .execute(pool)
        .await
        .expect("enroll mfa");
    }

    /// Grant `oauth_client_write` to the user with `mfa_required = false` so
    /// the capability extractor's recent-MFA recency check is skipped (we are
    /// not exercising MFA here — only the audit behaviour of update_client).
    async fn grant_oauth_client_write(pool: &PgPool, user_id: Uuid, granted_by: Uuid) {
        sqlx::query(
            r#"
            INSERT INTO capability_grants
                (user_id, capability, granted_by, expires_at, mfa_required, note)
            VALUES ($1, 'oauth_client_write', $2, NULL, false, 'ci-audit-test')
            "#,
        )
        .bind(user_id)
        .bind(granted_by)
        .execute(pool)
        .await
        .expect("grant oauth_client_write");
    }

    /// Look up the internal UUID of a seeded client by its `client_id`.
    async fn client_uuid(pool: &PgPool, client_id: &str) -> Uuid {
        sqlx::query_scalar::<_, Uuid>("SELECT id FROM oauth_clients WHERE client_id = $1")
            .bind(client_id)
            .fetch_one(pool)
            .await
            .expect("client uuid")
    }

    /// Mint a platform-kind bearer JWT validated by `TestApp`'s JWT secret.
    fn mint_platform_token(user_id: Uuid, email: &str) -> String {
        let svc = JwtService::new(TEST_JWT_SECRET).expect("jwt service");
        svc.generate_access_token_with_kind(
            user_id,
            email,
            "OAuth Admin Test",
            None,
            None,
            Some("platform".to_string()),
        )
        .expect("mint token")
    }

    /// Build a PATCH request (the test RequestBuilder has no `patch` helper).
    fn patch_request(uri: &str, token: &str, body: serde_json::Value) -> Request<Body> {
        Request::builder()
            .method(Method::PATCH)
            .uri(uri)
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string()))
            .unwrap()
    }

    /// Updating a client's scopes via the admin route must write exactly one
    /// `oauth_client_update` audit row, attributed to the acting operator,
    /// with the supplied `reason` persisted in `details.reason`.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    #[ignore = "requires postgres + migrations"]
    async fn update_client_scope_change_writes_audit_with_reason(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;

        let admin = seed_platform_user(&pool, "oauth-audit-admin@test.local").await;
        let granter = seed_platform_user(&pool, "oauth-audit-granter@test.local").await;
        enroll_mfa(&pool, admin).await;
        grant_oauth_client_write(&pool, admin, granter).await;
        let token = mint_platform_token(admin, "oauth-audit-admin@test.local");

        let (client_id, _secret, _redirect) = seed_confidential_client(&pool).await;
        let id = client_uuid(&pool, &client_id).await;

        let before = count_audit_rows(&pool, admin, "oauth_client_update").await;

        let reason = "Granting calendar scope per ticket OPS-4821";
        let req = patch_request(
            &format!("/api/v1/admin/oauth/clients/{id}"),
            &token,
            serde_json::json!({
                "name": "CI Conf (renamed)",
                "scopes": ["profile", "email", "openid"],
                "reason": reason,
            }),
        );
        let resp = app.execute(req).await;
        assert_eq!(
            resp.status,
            StatusCode::OK,
            "update_client must succeed for a granted platform principal (got {}): {}",
            resp.status,
            resp.text()
        );

        // Exactly one audit row was added for this action.
        let after = count_audit_rows(&pool, admin, "oauth_client_update").await;
        assert_eq!(
            after,
            before + 1,
            "exactly one oauth_client_update audit row must be written on update_client"
        );

        // The supplied reason is persisted into the audit row's details, and
        // the scope change is recorded.
        let row: (serde_json::Value, Option<Uuid>) = sqlx::query_as(
            r#"
            SELECT details, resource_id
            FROM audit_logs
            WHERE user_id = $1 AND action::text = 'oauth_client_update'
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(admin)
        .fetch_one(&pool)
        .await
        .expect("fetch audit row");

        assert_eq!(
            row.0.get("reason").and_then(|v| v.as_str()),
            Some(reason),
            "audit details.reason must persist the operator reason, got {}",
            row.0
        );
        assert_eq!(
            row.0.get("scopes_changed").and_then(|v| v.as_bool()),
            Some(true),
            "audit must flag scopes_changed=true, got {}",
            row.0
        );
        assert_eq!(
            row.1,
            Some(id),
            "audit resource_id must reference the updated client UUID"
        );
    }
}

// ─── module: token_usage_analytics ───────────────────────────────────────────
//
// Epic 10A (data audit) follow-up #2628. PR #2526 landed the
// `oauth_token_events` data layer (model + repository + migration) but nothing
// on the running token path called it, so the table stayed permanently empty
// and the ecosystem-health dashboard rendered all-zeros. These tests pin the
// producer wired into `OAuthService`: every issuance / refresh / revocation on
// the real `/api/v1/oauth/*` router must persist a corresponding analytics row.
// On `dev` (no producer) each assertion below fails with an observed count of 0.

#[cfg(test)]
mod token_usage_analytics {
    use super::*;

    /// A full lifecycle — authorization_code issuance, refresh, then access-token
    /// revocation — must write exactly one `issued`, one `refreshed`, and one
    /// `revoked` row for the client, and the per-client rollup must reflect them.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_token_lifecycle_writes_analytics_events(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (user_at, _) = create_authenticated_user(&app, &user).await;
        let (client_id, client_secret, redirect_uri) = seed_confidential_client(&pool).await;

        // 1) authorization_code exchange → one `issued` event.
        let (access_token, refresh_token) =
            confidential_auth_flow(&app, &user_at, &client_id, &client_secret, &redirect_uri).await;
        assert_eq!(
            count_token_events(&pool, &client_id, "issued").await,
            1,
            "authorization_code exchange must record an `issued` token-usage event"
        );

        // 2) refresh_token grant → one `refreshed` event.
        let refresh_body = form_body(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", &refresh_token),
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
        assert_eq!(
            count_token_events(&pool, &client_id, "refreshed").await,
            1,
            "refresh_token grant must record a `refreshed` token-usage event"
        );

        // 3) RFC 7009 revocation of the access token → one `revoked` event.
        let revoke_status = revoke_rfc7009(
            &app,
            &access_token,
            "access_token",
            &client_id,
            &client_secret,
        )
        .await;
        assert_eq!(
            revoke_status,
            StatusCode::OK,
            "revoke must succeed per RFC 7009"
        );
        assert_eq!(
            count_token_events(&pool, &client_id, "revoked").await,
            1,
            "revocation must record a `revoked` token-usage event"
        );

        // The per-client rollup the dashboard reads must now be non-empty and
        // consistent with the three lifecycle transitions above.
        let repo = db::repositories::OAuthTokenEventRepository::new(pool.clone());
        let since = chrono::Utc::now() - chrono::Duration::hours(1);
        let usage = repo
            .usage_per_client(since)
            .await
            .expect("usage_per_client");
        let row = usage
            .iter()
            .find(|u| u.client_id == client_id)
            .expect("client must appear in the per-client rollup");
        assert_eq!(row.issued_count, 1, "rollup issued_count");
        assert_eq!(row.refreshed_count, 1, "rollup refreshed_count");
        assert_eq!(row.revoked_count, 1, "rollup revoked_count");

        let totals = repo.totals(since).await.expect("totals");
        assert_eq!(totals.issued_count, 1);
        assert_eq!(totals.refreshed_count, 1);
        assert_eq!(totals.revoked_count, 1);
        assert_eq!(totals.active_clients, 1);
    }

    /// A best-effort recording failure must never break the OAuth flow: the
    /// issuance path still returns tokens even though we assert the analytics
    /// row is present on the happy path. This test focuses on the happy-path
    /// contract; the log-and-ignore behaviour is unit-covered by the service.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_public_client_issuance_records_event(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (user_at, _) = create_authenticated_user(&app, &user).await;
        let (client_id, redirect_uri) = seed_public_client(&pool).await;

        let (verifier, challenge) = pkce_pair();
        let code = consent_and_get_code(
            &app,
            &user_at,
            &client_id,
            &redirect_uri,
            "profile",
            &challenge,
        )
        .await;
        let token_form = form_body(&[
            ("grant_type", "authorization_code"),
            ("code", &code),
            ("redirect_uri", &redirect_uri),
            ("client_id", &client_id),
            ("code_verifier", &verifier),
        ]);
        let resp = app
            .execute(form_request("/api/v1/oauth/token", &token_form))
            .await;
        assert_eq!(
            resp.status,
            StatusCode::OK,
            "public client token exchange must succeed. body={}",
            resp.text()
        );
        assert_eq!(
            count_token_events(&pool, &client_id, "issued").await,
            1,
            "public client authorization_code exchange must record an `issued` event"
        );
    }
}

// ─── module: token_usage_read_route ──────────────────────────────────────────
//
// Router-driven HTTP coverage for `GET /api/v1/platform-admin/oauth/token-usage`
// (issue #2630 — post-merge-review follow-up on PR #2629). The sibling
// `token_usage_analytics` module above only exercises the producer and the
// repository rollups (`OAuthTokenEventRepository::{totals,usage_per_client}`)
// called *directly*; nothing drove the read *handler* through the real Axum
// router, so the response envelope, its authz walls, and its `since` decoding
// shipped with zero executing HTTP-level coverage.
//
// These cases close that gap end-to-end against the real router:
//   * super-admin + `AuditRead` → 200, and the JSON body carries the
//     camelCase envelope (`totals` / `perClient` / `since`) after one seeded
//     issuance.
//   * the two authz walls each deny with 403: the `require_capability(AuditRead)`
//     tower layer, and the in-handler `extract_super_admin_token` super-admin
//     re-check.
//   * a malformed `?since=` → 400 `INVALID_SINCE`, and a valid RFC 3339
//     `?since=` narrows the window.
//
// IG3 note: NONE of these executed on `dev` before #2630. The deny-path (403)
// assertions, the 400 `INVALID_SINCE` decode assertion, and the camelCase
// envelope assertions are entirely net-new — the prior `token_usage_analytics`
// module never constructed an HTTP request to this route.
//
// The platform-admin provisioning recipe mirrors the shipped, passing pattern
// in `admin_platform_happy_path_tests.rs`: seed a `principal_kind = 'platform'`
// user (the INSERT bypasses the BEFORE-UPDATE guard), enroll MFA, and grant the
// capability with `mfa_required = false` so the extractor's recent-MFA recency
// check is skipped. The OAuth issuance is seeded through the real router via the
// file-level `confidential_auth_flow` helper.

#[cfg(test)]
mod token_usage_read_route {
    use super::*;
    use api_server::services::JwtService;

    const TOKEN_USAGE_URL: &str = "/api/v1/platform-admin/oauth/token-usage";
    /// Matches `TestConfig::default().jwt_secret`, so tokens minted here are
    /// accepted by the `TestApp`'s `JwtService`.
    const TEST_JWT_SECRET: &str =
        "test-secret-key-that-is-at-least-64-characters-long-for-testing-purposes";

    /// Seed a platform-principal user and return its id. The INSERT bypasses the
    /// `BEFORE UPDATE` principal_kind guard, so `'platform'` can be set directly.
    async fn seed_platform_user(pool: &PgPool, email: &str) -> Uuid {
        sqlx::query_scalar::<_, Uuid>(
            r#"
            INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
            VALUES ($1, 'h', 'Issue2630 Admin', 'active', NOW(), 'platform')
            RETURNING id
            "#,
        )
        .bind(email)
        .fetch_one(pool)
        .await
        .expect("seed platform user")
    }

    /// Mark the user as MFA-enrolled (the capability gate's step-2.5 wall).
    async fn enroll_mfa(pool: &PgPool, user_id: Uuid) {
        sqlx::query(
            r#"
            INSERT INTO user_2fa (user_id, secret, enabled, enabled_at, backup_codes, backup_codes_remaining)
            VALUES ($1, 'unused-secret', true, NOW(), '[]'::jsonb, 0)
            ON CONFLICT (user_id) DO UPDATE SET enabled = true, enabled_at = NOW()
            "#,
        )
        .bind(user_id)
        .execute(pool)
        .await
        .expect("enroll mfa");
    }

    /// Grant a single named `capability` with `mfa_required = false`. `granted_by`
    /// must reference a distinct real user (FK + app-layer no-self-grant rule).
    async fn grant_capability(pool: &PgPool, user_id: Uuid, granted_by: Uuid, capability: &str) {
        sqlx::query(
            r#"
            INSERT INTO capability_grants
                (user_id, capability, granted_by, expires_at, mfa_required, note)
            VALUES ($1, $2, $3, NULL, false, 'issue-2630')
            "#,
        )
        .bind(user_id)
        .bind(capability)
        .bind(granted_by)
        .execute(pool)
        .await
        .expect("grant capability");
    }

    /// Mint a platform-kind bearer JWT validated by `TestApp`'s JWT secret, with
    /// the supplied `roles` claim (the wall under test in the deny cases).
    fn mint_token(user_id: Uuid, email: &str, roles: Option<Vec<String>>) -> String {
        let svc = JwtService::new(TEST_JWT_SECRET).expect("jwt service");
        svc.generate_access_token_with_kind(
            user_id,
            email,
            "Issue2630 Admin",
            None,
            roles,
            Some("platform".to_string()),
        )
        .expect("mint token")
    }

    /// Provision a fully-authorized platform super-admin holding `AuditRead`
    /// (platform principal + MFA + active grant + `super_admin` role) and return
    /// its bearer token — passes both the capability layer and
    /// `extract_super_admin_token`.
    async fn super_admin_with_audit_read(pool: &PgPool, label: &str) -> String {
        let email = format!("issue2630-sa-{label}-{}@test.local", Uuid::new_v4());
        let granter_email = format!("issue2630-gr-{label}-{}@test.local", Uuid::new_v4());
        let admin = seed_platform_user(pool, &email).await;
        let granter = seed_platform_user(pool, &granter_email).await;
        enroll_mfa(pool, admin).await;
        grant_capability(pool, admin, granter, "audit_read").await;
        mint_token(admin, &email, Some(vec!["super_admin".to_string()]))
    }

    /// Seed exactly one `issued` OAuth token event through the real router and
    /// return the client_id it was recorded against.
    async fn seed_one_issuance(app: &TestApp, pool: &PgPool) -> String {
        let user = TestUser::new();
        let (user_at, _) = create_authenticated_user(app, &user).await;
        let (client_id, client_secret, redirect_uri) = seed_confidential_client(pool).await;
        confidential_auth_flow(app, &user_at, &client_id, &client_secret, &redirect_uri).await;
        assert_eq!(
            count_token_events(pool, &client_id, "issued").await,
            1,
            "precondition: one issuance must be seeded through the router"
        );
        client_id
    }

    /// Case 1 — super-admin + `AuditRead` gets 200, and the JSON body carries the
    /// camelCase envelope (`totals` / `perClient` / `since`). The `#[serde(rename_all
    /// = "camelCase")]` on `OAuthTokenUsageResponse` renames `per_client` →
    /// `perClient`; assert both that the camelCase spelling is present and that the
    /// snake_case spelling is absent, so the contract is pinned in both directions.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_token_usage_super_admin_returns_camelcase_body(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let client_id = seed_one_issuance(&app, &pool).await;

        let token = super_admin_with_audit_read(&pool, "ok").await;
        let resp = app.get(TOKEN_USAGE_URL).bearer(&token).send().await;
        assert_eq!(
            resp.status,
            StatusCode::OK,
            "super-admin with AuditRead must get 200. body={}",
            resp.text()
        );

        let body = resp.json_value();
        // camelCase envelope keys are present …
        assert!(
            body.get("totals").is_some(),
            "body must carry `totals`: {body}"
        );
        assert!(
            body.get("perClient").is_some(),
            "body must carry `perClient`: {body}"
        );
        assert!(
            body.get("since").is_some(),
            "body must carry `since`: {body}"
        );
        // … and the snake_case spelling is absent (locks the camelCase rename).
        assert!(
            body.get("per_client").is_none(),
            "`per_client` must be renamed to `perClient`: {body}"
        );
        assert!(
            body["since"].is_string(),
            "`since` must serialize as an RFC 3339 string: {body}"
        );

        // The seeded issuance is reflected in the totals + per-client rollup
        // (nested structs are camelCase too).
        assert_eq!(
            body["totals"]["issuedCount"], 1,
            "totals.issuedCount: {body}"
        );
        assert_eq!(
            body["totals"]["activeClients"], 1,
            "totals.activeClients: {body}"
        );
        assert!(
            body["totals"].get("issued_count").is_none(),
            "totals must be camelCase (no `issued_count`): {body}"
        );
        let per_client = body["perClient"].as_array().expect("perClient array");
        let row = per_client
            .iter()
            .find(|c| c["clientId"] == client_id)
            .expect("seeded client must appear in perClient");
        assert_eq!(row["issuedCount"], 1, "perClient issuedCount: {row}");
    }

    /// Case 2a — a platform principal holding `AuditRead` (so the capability layer
    /// passes) but whose token carries no `super_admin` role must be rejected with
    /// 403 by the in-handler `extract_super_admin_token` re-check.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_token_usage_denied_without_super_admin_role(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;

        let email = format!("issue2630-norole-{}@test.local", Uuid::new_v4());
        let granter_email = format!("issue2630-norole-gr-{}@test.local", Uuid::new_v4());
        let admin = seed_platform_user(&pool, &email).await;
        let granter = seed_platform_user(&pool, &granter_email).await;
        enroll_mfa(&pool, admin).await;
        grant_capability(&pool, admin, granter, "audit_read").await;
        // Token carries a non-privileged role only.
        let token = mint_token(admin, &email, Some(vec!["member".to_string()]));

        let resp = app.get(TOKEN_USAGE_URL).bearer(&token).send().await;
        assert_eq!(
            resp.status,
            StatusCode::FORBIDDEN,
            "a non-super-admin caller must be rejected by extract_super_admin_token. body={}",
            resp.text()
        );
    }

    /// Case 2b — a platform `super_admin` with no `AuditRead` grant must be
    /// rejected with 403 by the `require_capability(AuditRead)` tower layer,
    /// before the handler runs.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_token_usage_denied_without_audit_read_capability(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;

        let email = format!("issue2630-nocap-{}@test.local", Uuid::new_v4());
        let admin = seed_platform_user(&pool, &email).await;
        enroll_mfa(&pool, admin).await;
        // super_admin role but NO capability grant.
        let token = mint_token(admin, &email, Some(vec!["super_admin".to_string()]));

        let resp = app.get(TOKEN_USAGE_URL).bearer(&token).send().await;
        assert_eq!(
            resp.status,
            StatusCode::FORBIDDEN,
            "a caller missing the AuditRead capability must be rejected by the layer. body={}",
            resp.text()
        );
    }

    /// Case 3a — a malformed `?since=` must be rejected with 400 and the
    /// programmatic error code `INVALID_SINCE`.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_token_usage_invalid_since_returns_400(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let token = super_admin_with_audit_read(&pool, "badsince").await;

        let resp = app
            .get(&format!("{TOKEN_USAGE_URL}?since=not-a-timestamp"))
            .bearer(&token)
            .send()
            .await;
        assert_eq!(
            resp.status,
            StatusCode::BAD_REQUEST,
            "a malformed `since` must be rejected with 400. body={}",
            resp.text()
        );
        let body = resp.json_value();
        assert_eq!(
            body["code"], "INVALID_SINCE",
            "error code must be INVALID_SINCE: {body}"
        );
    }

    /// Case 3b — a valid RFC 3339 `?since=` narrows the window: a bound in the
    /// past includes the seeded issuance (and the echoed `since` equals the
    /// supplied bound), while a bound in the future excludes it.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_token_usage_valid_since_narrows_window(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        seed_one_issuance(&app, &pool).await;
        let token = super_admin_with_audit_read(&pool, "since").await;

        // A `since` in the past includes the issuance.
        let past = (chrono::Utc::now() - chrono::Duration::hours(1)).to_rfc3339();
        let included = app
            .get(&format!(
                "{TOKEN_USAGE_URL}?since={}",
                urlencoding::encode(&past)
            ))
            .bearer(&token)
            .send()
            .await;
        assert_eq!(
            included.status,
            StatusCode::OK,
            "valid past `since` must be accepted. body={}",
            included.text()
        );
        let included_body = included.json_value();
        assert_eq!(
            included_body["totals"]["issuedCount"], 1,
            "a past `since` must include the seeded issuance: {included_body}"
        );
        // The echoed `since` reflects the caller-supplied bound, not the default.
        let echoed = included_body["since"].as_str().expect("since string");
        let echoed_dt = chrono::DateTime::parse_from_rfc3339(echoed).expect("echoed since parses");
        let supplied_dt =
            chrono::DateTime::parse_from_rfc3339(&past).expect("supplied since parses");
        assert_eq!(
            echoed_dt, supplied_dt,
            "the echoed `since` must equal the supplied bound"
        );

        // A `since` in the future excludes the issuance → window narrows to empty.
        let future = (chrono::Utc::now() + chrono::Duration::hours(1)).to_rfc3339();
        let narrowed = app
            .get(&format!(
                "{TOKEN_USAGE_URL}?since={}",
                urlencoding::encode(&future)
            ))
            .bearer(&token)
            .send()
            .await;
        assert_eq!(
            narrowed.status,
            StatusCode::OK,
            "valid future `since` must be accepted. body={}",
            narrowed.text()
        );
        let narrowed_body = narrowed.json_value();
        assert_eq!(
            narrowed_body["totals"]["issuedCount"], 0,
            "a future `since` must narrow the window to empty: {narrowed_body}"
        );
        assert!(
            narrowed_body["perClient"]
                .as_array()
                .map(|a| a.is_empty())
                .unwrap_or(false),
            "a future `since` must yield an empty perClient: {narrowed_body}"
        );
    }
}
