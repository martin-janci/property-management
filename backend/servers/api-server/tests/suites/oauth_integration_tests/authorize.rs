//! OAuth authorization-endpoint flow tests (PKCE S256 authorize).
//!
//! Split from the former monolithic `oauth_integration_tests.rs`
//! (by OAuth flow). Test bodies are unchanged.

pub(crate) use super::fixtures::*;

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
