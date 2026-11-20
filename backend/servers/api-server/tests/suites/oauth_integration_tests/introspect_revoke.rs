//! OAuth introspection / revocation + authorize-audit tests.
//!
//! Split from the former monolithic `oauth_integration_tests.rs`
//! (by OAuth flow). Test bodies are unchanged.

pub(crate) use super::fixtures::*;

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
