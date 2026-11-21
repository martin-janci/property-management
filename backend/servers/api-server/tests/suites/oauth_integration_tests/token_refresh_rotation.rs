//! OAuth token-endpoint flow tests — refresh-token rotation.
//!
//! Split from the former monolithic `oauth_integration_tests.rs`
//! (by OAuth flow). Test bodies are unchanged.

pub(crate) use super::fixtures::*;

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
