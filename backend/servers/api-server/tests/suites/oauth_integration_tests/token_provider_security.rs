//! OAuth token-endpoint flow tests — Epic 10A provider security properties.
//!
//! Split from the former monolithic `oauth_integration_tests.rs`
//! (by OAuth flow). Test bodies are unchanged.

pub(crate) use super::fixtures::*;

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
