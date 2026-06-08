//! OAuth 2.0 Authorization-Server integration tests — error paths & audit trail
//! (Coverage 10a-1).
//!
//! The companion suite `oauth_integration_tests.rs` already exercises the PKCE
//! happy path, refresh rotation, consent/revoke, and introspection. This file
//! deliberately does NOT duplicate that. It pins the ACTUAL shipped contract of
//! the authorization-server error paths and the authorization audit trail that
//! the companion suite leaves uncovered:
//!
//!   - authorize GET error paths: unknown client, wrong response_type,
//!     redirect-URI mismatch (note the shipped quirk: redirect-URI mismatch at
//!     /authorize maps to `invalid_request`, NOT `invalid_redirect_uri`).
//!   - authorize POST (consent) quirks: a confidential client still issues a
//!     code, and `create_authorization_code` does NOT re-validate redirect_uri
//!     at consent time — the binding check is deferred to /token (RFC 6749
//!     §5.2). Both quirks are pinned, not "fixed".
//!   - token endpoint error paths: missing client_id, unknown client,
//!     unsupported grant_type, code-exchange redirect-URI mismatch
//!     (`invalid_grant`), missing code (`invalid_grant`), and the SEC-001
//!     contract that a confidential client cannot be downgraded to public by
//!     omitting its secret (401 `invalid_client`).
//!   - audit trail: the `oauth_authorize` row carries the client_id + scopes in
//!     its `details`, and a DENIED consent emits NO `oauth_authorize` row.
//!
//! Every test uses `#[sqlx::test(migrator = "db::MIGRATOR")]` so the schema is
//! fully up to date, and `TestApp` so the real Axum router (all middleware) is
//! exercised end to end. Seeding/harness helpers mirror
//! `oauth_integration_tests.rs` rather than re-deriving them.

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

// ─── helpers (mirrors oauth_integration_tests.rs) ────────────────────────────

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

/// POST `application/x-www-form-urlencoded`.
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

/// GET with Bearer auth. The authorize endpoint requires the caller to be
/// authenticated (issue #820), so the GET must carry the access token.
fn get_request_with_auth(uri: &str, token: &str) -> Request<Body> {
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
/// Returns `(client_id, redirect_uri)`.
async fn seed_public_client(pool: &PgPool) -> (String, String) {
    let client_id = format!("ci-pub-{}", &Uuid::new_v4().to_string()[..8]);
    let redirect_uri = "https://spa.example.com/callback".to_string();
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

/// Count `audit_logs` rows by user + action (compares against the `::text`
/// serialization, i.e. the `#[sqlx(rename = …)]` value such as `oauth_authorize`).
async fn count_audit_rows(pool: &PgPool, user_id: Uuid, action: &str) -> i64 {
    sqlx::query_scalar::<_, i64>(
        r#"SELECT COUNT(*) FROM audit_logs WHERE user_id = $1 AND action::text = $2"#,
    )
    .bind(user_id)
    .bind(action)
    .fetch_one(pool)
    .await
    .unwrap_or(0)
}

/// Resolve a seeded user's id by email.
async fn user_id_by_email(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(email)
        .fetch_one(pool)
        .await
        .expect("find user id")
}

// ─── module: authorize_errors ────────────────────────────────────────────────

#[cfg(test)]
mod authorize_errors {
    use super::*;

    /// authorize GET for an unknown client_id must be rejected. The service maps
    /// `require_active_client` failure to `InvalidClient("Client not found")`,
    /// which the handler returns as 400 `invalid_client`.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_authorize_unknown_client_rejected(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (access_token, _) = create_authenticated_user(&app, &user).await;
        let (_verifier, challenge) = pkce_pair();

        let uri = format!(
            "/api/v1/oauth/authorize?response_type=code&client_id={}&redirect_uri={}&scope=profile&code_challenge={}&code_challenge_method=S256",
            urlencoding::encode("no-such-client-id"),
            urlencoding::encode("https://spa.example.com/callback"),
            urlencoding::encode(&challenge),
        );
        let resp = app
            .execute(get_request_with_auth(&uri, &access_token))
            .await;
        assert_eq!(
            resp.status,
            StatusCode::BAD_REQUEST,
            "unknown client must be rejected. body={}",
            resp.text()
        );
        assert_eq!(resp.json_value()["error"], "invalid_client");
    }

    /// authorize GET with `response_type` other than `code` must be rejected
    /// with 400 `invalid_request` (the handler validates this before touching
    /// the service).
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_authorize_wrong_response_type_rejected(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (access_token, _) = create_authenticated_user(&app, &user).await;
        let (client_id, redirect_uri) = seed_public_client(&pool).await;
        let (_verifier, challenge) = pkce_pair();

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
            "response_type!=code must be rejected. body={}",
            resp.text()
        );
        assert_eq!(resp.json_value()["error"], "invalid_request");
    }

    /// authorize GET with a redirect_uri not registered for the client must be
    /// rejected. SHIPPED QUIRK (pinned, not fixed): the service's
    /// `InvalidRedirectUri` maps to OAuthError `invalid_request` (see the
    /// `From<OAuthServiceError>` impl), so the error code is `invalid_request`
    /// — NOT a dedicated `invalid_redirect_uri`. Status is 400.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_authorize_redirect_uri_mismatch_rejected(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (access_token, _) = create_authenticated_user(&app, &user).await;
        let (client_id, _registered_redirect) = seed_public_client(&pool).await;
        let (_verifier, challenge) = pkce_pair();

        let uri = format!(
            "/api/v1/oauth/authorize?response_type=code&client_id={}&redirect_uri={}&scope=profile&code_challenge={}&code_challenge_method=S256",
            urlencoding::encode(&client_id),
            urlencoding::encode("https://attacker.example.com/callback"),
            urlencoding::encode(&challenge),
        );
        let resp = app
            .execute(get_request_with_auth(&uri, &access_token))
            .await;
        assert_eq!(
            resp.status,
            StatusCode::BAD_REQUEST,
            "redirect_uri mismatch must be rejected. body={}",
            resp.text()
        );
        // Quirk pin: redirect mismatch surfaces as invalid_request, not a
        // dedicated invalid_redirect_uri error code.
        assert_eq!(
            resp.json_value()["error"],
            "invalid_request",
            "redirect mismatch maps to invalid_request (shipped quirk)"
        );
    }

    /// authorize GET missing the PKCE `code_challenge` must be rejected with 400
    /// `invalid_request` even for a confidential client (OAuth 2.1 — PKCE is
    /// mandatory for all clients). This complements the companion suite's
    /// public-client variant by asserting the error code on the GET path.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_authorize_missing_pkce_rejected(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (access_token, _) = create_authenticated_user(&app, &user).await;
        let (client_id, _secret, redirect_uri) = seed_confidential_client(&pool).await;

        let uri = format!(
            "/api/v1/oauth/authorize?response_type=code&client_id={}&redirect_uri={}&scope=profile",
            urlencoding::encode(&client_id),
            urlencoding::encode(&redirect_uri),
        );
        let resp = app
            .execute(get_request_with_auth(&uri, &access_token))
            .await;
        assert_eq!(
            resp.status,
            StatusCode::BAD_REQUEST,
            "missing PKCE must be rejected. body={}",
            resp.text()
        );
        assert_eq!(resp.json_value()["error"], "invalid_request");
    }
}

// ─── module: consent_quirks ──────────────────────────────────────────────────

#[cfg(test)]
mod consent_quirks {
    use super::*;

    /// SHIPPED QUIRK (pinned, not fixed): the consent POST
    /// (`create_authorization_code`) re-validates scopes against the client
    /// grant but does NOT re-validate `redirect_uri`. So a consent POST whose
    /// `redirect_uri` differs from the one the client registered still issues a
    /// code at /authorize. The redirect-URI binding is only enforced later at
    /// /token (RFC 6749 §5.2). This test pins that a code IS issued here.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_consent_does_not_revalidate_redirect_uri(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (access_token, _) = create_authenticated_user(&app, &user).await;
        let (client_id, _registered_redirect) = seed_public_client(&pool).await;
        let (_verifier, challenge) = pkce_pair();

        // Submit consent with a redirect_uri the client never registered.
        let consent_form = form_body(&[
            ("client_id", &client_id),
            ("redirect_uri", "https://unregistered.example.com/cb"),
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
        assert_eq!(
            resp.status,
            StatusCode::OK,
            "consent POST issues a code without re-validating redirect_uri (shipped quirk). body={}",
            resp.text()
        );
        assert!(
            !resp.json_value()["code"].as_str().unwrap_or("").is_empty(),
            "an authorization code must be returned"
        );
    }

    /// A confidential client successfully obtains an authorization code at the
    /// consent POST (the redeem flow itself is covered in the companion suite —
    /// here we just confirm the consent POST issues a code for a confidential
    /// client). The companion suite focuses public-client error paths.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_consent_confidential_client_issues_code(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (access_token, _) = create_authenticated_user(&app, &user).await;
        let (client_id, _secret, redirect_uri) = seed_confidential_client(&pool).await;
        let (_verifier, challenge) = pkce_pair();

        let consent_form = form_body(&[
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
                &consent_form,
                &access_token,
            ))
            .await;
        assert_eq!(resp.status, StatusCode::OK, "body={}", resp.text());
        assert!(!resp.json_value()["code"].as_str().unwrap_or("").is_empty());
    }

    /// Consent POST without a bearer token must be rejected with 401 — the
    /// handler authenticates the caller before processing consent.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_consent_post_requires_auth(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
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
        let resp = app
            .execute(form_request("/api/v1/oauth/authorize", &consent_form))
            .await;
        assert_eq!(
            resp.status,
            StatusCode::UNAUTHORIZED,
            "unauthenticated consent POST must be rejected. body={}",
            resp.text()
        );
    }
}

// ─── module: token_errors ────────────────────────────────────────────────────

#[cfg(test)]
mod token_errors {
    use super::*;

    /// Token request with no `client_id` must be rejected with 400
    /// `invalid_request` ("client_id required") — the handler extracts client_id
    /// before any grant processing.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_token_missing_client_id_rejected(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let body = form_body(&[
            ("grant_type", "authorization_code"),
            ("code", "irrelevant"),
            ("redirect_uri", "https://spa.example.com/callback"),
            // no client_id
        ]);
        let resp = app
            .execute(form_request("/api/v1/oauth/token", &body))
            .await;
        assert_eq!(
            resp.status,
            StatusCode::BAD_REQUEST,
            "missing client_id must be rejected. body={}",
            resp.text()
        );
        assert_eq!(resp.json_value()["error"], "invalid_request");
    }

    /// Token request with an unknown `client_id` must be rejected with 401
    /// `invalid_client` — the client lookup fails before grant processing.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_token_unknown_client_rejected(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let body = form_body(&[
            ("grant_type", "authorization_code"),
            ("code", "irrelevant"),
            ("redirect_uri", "https://spa.example.com/callback"),
            ("client_id", "no-such-client"),
        ]);
        let resp = app
            .execute(form_request("/api/v1/oauth/token", &body))
            .await;
        assert_eq!(
            resp.status,
            StatusCode::UNAUTHORIZED,
            "unknown client must be rejected. body={}",
            resp.text()
        );
        assert_eq!(resp.json_value()["error"], "invalid_client");
    }

    /// SEC-001 contract: a confidential client cannot be downgraded to a public
    /// client by simply omitting `client_secret`. The token endpoint looks the
    /// client up first and, finding it confidential, requires a secret —
    /// returning 401 `invalid_client` when none is supplied.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_token_confidential_client_without_secret_rejected(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let (client_id, _secret, redirect_uri) = seed_confidential_client(&pool).await;

        let body = form_body(&[
            ("grant_type", "authorization_code"),
            ("code", "irrelevant"),
            ("redirect_uri", &redirect_uri),
            ("client_id", &client_id),
            // confidential client, but no client_secret
        ]);
        let resp = app
            .execute(form_request("/api/v1/oauth/token", &body))
            .await;
        assert_eq!(
            resp.status,
            StatusCode::UNAUTHORIZED,
            "confidential client without secret must be rejected (SEC-001). body={}",
            resp.text()
        );
        assert_eq!(resp.json_value()["error"], "invalid_client");
    }

    /// Token request with an unsupported `grant_type` must be rejected with 400
    /// `invalid_request`. The client is valid; only the grant type is bad. (The
    /// client must authenticate first, hence a confidential client + secret.)
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_token_unsupported_grant_type_rejected(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let (client_id, client_secret, _redirect_uri) = seed_confidential_client(&pool).await;

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
            "unsupported grant_type must be rejected. body={}",
            resp.text()
        );
        assert_eq!(resp.json_value()["error"], "invalid_request");
    }

    /// Code exchange where the `redirect_uri` does not match the one bound to
    /// the authorization code must be rejected. SHIPPED CONTRACT (RFC 6749
    /// §5.2): redirect-URI mismatch at /token is `invalid_grant`, NOT
    /// `invalid_request`. Status 400.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_token_code_exchange_redirect_mismatch_invalid_grant(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (access_token, _) = create_authenticated_user(&app, &user).await;
        let (client_id, client_secret, redirect_uri) = seed_confidential_client(&pool).await;
        let (verifier, challenge) = pkce_pair();

        // Mint a code bound to the registered redirect_uri.
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

        // Exchange with a DIFFERENT redirect_uri.
        let token_body = form_body(&[
            ("grant_type", "authorization_code"),
            ("code", &code),
            ("redirect_uri", "https://app.example.com/other-callback"),
            ("client_id", &client_id),
            ("client_secret", &client_secret),
            ("code_verifier", &verifier),
        ]);
        let resp = app
            .execute(form_request("/api/v1/oauth/token", &token_body))
            .await;
        assert_eq!(
            resp.status,
            StatusCode::BAD_REQUEST,
            "redirect mismatch at /token must be rejected. body={}",
            resp.text()
        );
        // Contract pin: §5.2 mismatch is invalid_grant, not invalid_request.
        assert_eq!(resp.json_value()["error"], "invalid_grant");
    }

    /// SHIPPED QUIRK (pinned): an `authorization_code` grant with NO `code`
    /// parameter is rejected as `invalid_grant`, not `invalid_request`. The
    /// service treats a missing code the same as an unredeemable one
    /// (`exchange_code_for_tokens` returns `InvalidGrant` when `code` is absent).
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_token_authorization_code_missing_code_invalid_grant(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let (client_id, client_secret, redirect_uri) = seed_confidential_client(&pool).await;

        let body = form_body(&[
            ("grant_type", "authorization_code"),
            ("redirect_uri", &redirect_uri),
            ("client_id", &client_id),
            ("client_secret", &client_secret),
            // no `code`
        ]);
        let resp = app
            .execute(form_request("/api/v1/oauth/token", &body))
            .await;
        assert_eq!(
            resp.status,
            StatusCode::BAD_REQUEST,
            "missing code must be rejected. body={}",
            resp.text()
        );
        // Quirk pin: missing code is invalid_grant (not invalid_request).
        assert_eq!(resp.json_value()["error"], "invalid_grant");
    }
}

// ─── module: audit_trail ─────────────────────────────────────────────────────

#[cfg(test)]
mod audit_trail {
    use super::*;

    /// The `oauth_authorize` audit row written on a successful consent must
    /// carry the `client_id` and the granted `scopes` in its `details` JSON.
    /// The companion suite only asserts the row count; this pins the payload.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_authorize_audit_details_carry_client_and_scopes(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (access_token, _) = create_authenticated_user(&app, &user).await;
        let (client_id, redirect_uri) = seed_public_client(&pool).await;
        let (_verifier, challenge) = pkce_pair();
        let user_id = user_id_by_email(&pool, &user.email).await;

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
        assert_eq!(resp.status, StatusCode::OK, "body={}", resp.text());

        // Inspect the persisted audit row's details payload.
        let details: serde_json::Value = sqlx::query_scalar::<_, serde_json::Value>(
            r#"SELECT details FROM audit_logs
               WHERE user_id = $1 AND action::text = 'oauth_authorize'
               ORDER BY created_at DESC LIMIT 1"#,
        )
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .expect("audit row must exist");

        assert_eq!(
            details["client_id"], client_id,
            "audit details must record the client_id"
        );
        let scopes = details["scopes"]
            .as_array()
            .expect("scopes must be an array in audit details");
        assert!(
            scopes.iter().any(|s| s == "profile"),
            "audit details scopes must include the granted scope, got {}",
            details
        );
    }

    /// A DENIED consent must NOT emit an `oauth_authorize` audit row — the
    /// handler short-circuits with 403 `access_denied` before reaching the
    /// code-creation + audit block.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_denied_consent_emits_no_authorize_audit_row(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        let (access_token, _) = create_authenticated_user(&app, &user).await;
        let (client_id, redirect_uri) = seed_public_client(&pool).await;
        let (_verifier, challenge) = pkce_pair();
        let user_id = user_id_by_email(&pool, &user.email).await;

        let before = count_audit_rows(&pool, user_id, "oauth_authorize").await;

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
        assert_eq!(resp.json_value()["error"], "access_denied");

        let after = count_audit_rows(&pool, user_id, "oauth_authorize").await;
        assert_eq!(
            after, before,
            "a denied consent must not write an oauth_authorize audit row"
        );
    }
}
