//! OAuth token-endpoint flow tests — grant/parameter validation.
//!
//! Split from the former monolithic `oauth_integration_tests.rs`
//! (by OAuth flow). Test bodies are unchanged.

pub(crate) use super::fixtures::*;

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
