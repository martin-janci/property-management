//! Shared fixtures for the OAuth 2.0 integration test flows.
//!
//! Extracted verbatim from the former monolithic
//! `oauth_integration_tests.rs` so the per-flow modules (`authorize`,
//! `token`, `introspect_revoke`, `grants`, `admin_analytics`) can share the
//! PKCE / client-seed / introspect helpers. Behaviour is unchanged; the
//! helpers are merely re-scoped to `pub(crate)`.

pub(crate) use api_server::services::AuthService;
pub(crate) use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
pub(crate) use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
pub(crate) use base64::Engine;
pub(crate) use sha2::{Digest, Sha256};
pub(crate) use sqlx::PgPool;
pub(crate) use uuid::Uuid;

pub(crate) use crate::common::{create_authenticated_user, TestApp, TestUser};

// ─── helpers ───────────────────────────────────────────────────────────────────────────

/// Build a PKCE (S256) code_verifier + code_challenge pair.
pub(crate) fn pkce_pair() -> (String, String) {
    let verifier = format!("test-verifier-{}", Uuid::new_v4());
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let challenge = URL_SAFE_NO_PAD.encode(hasher.finalize());
    (verifier, challenge)
}

/// URL-encode a key=value form body.
pub(crate) fn form_body(pairs: &[(&str, &str)]) -> String {
    pairs
        .iter()
        .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
        .collect::<Vec<_>>()
        .join("&")
}

/// POST application/x-www-form-urlencoded.
pub(crate) fn form_request(uri: &str, body: &str) -> Request<Body> {
    Request::builder()
        .method(Method::POST)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(Body::from(body.to_string()))
        .unwrap()
}

/// POST application/x-www-form-urlencoded with Bearer auth.
pub(crate) fn form_request_with_auth(uri: &str, body: &str, token: &str) -> Request<Body> {
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
pub(crate) fn get_request_with_auth(uri: &str, token: &str) -> Request<Body> {
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
pub(crate) async fn seed_confidential_client(pool: &PgPool) -> (String, String, String) {
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
pub(crate) async fn seed_public_client(pool: &PgPool) -> (String, String) {
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
pub(crate) async fn count_audit_rows(pool: &PgPool, user_id: Uuid, action: &str) -> i64 {
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
pub(crate) async fn count_token_events(pool: &PgPool, client_id: &str, event_type: &str) -> i64 {
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

/// Wait until at least `expected` `oauth_token_events` rows exist for
/// `client_id` + `event_type`, or panic after a short timeout.
///
/// Since #2643 the token-usage recording is written by a **detached background
/// task** off the hot token path, so the row may not be visible the instant the
/// HTTP token response returns. Polling here removes that race while still
/// proving the recording is preserved on the happy path (no dropped writes) —
/// a genuinely dropped write makes this time out and fail.
pub(crate) async fn await_token_event_count(
    pool: &PgPool,
    client_id: &str,
    event_type: &str,
    expected: i64,
) {
    // ~2s budget (100 × 20ms) — comfortably covers a local INSERT round-trip
    // while still failing fast if the event is never recorded.
    for _ in 0..100 {
        if count_token_events(pool, client_id, event_type).await >= expected {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    panic!(
        "timed out waiting for {expected} `{event_type}` token-usage event(s) for client \
         {client_id}; observed {} — the detached recorder dropped the write",
        count_token_events(pool, client_id, event_type).await,
    );
}

/// Run the full PKCE S256 authorization-code flow for a **confidential** client
/// and return `(access_token, refresh_token)`.
///
/// Shared by `refresh_rotation` and `provider_security` tests.  Using this
/// helper instead of copy-pasting the consent → token exchange pattern means
/// future changes to the authorize/token endpoints only need updating in one
/// place.
pub(crate) async fn confidential_auth_flow(
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
pub(crate) async fn consent_and_get_code(
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
pub(crate) async fn revoke_rfc7009(
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
pub(crate) async fn introspect_with_creds(
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
