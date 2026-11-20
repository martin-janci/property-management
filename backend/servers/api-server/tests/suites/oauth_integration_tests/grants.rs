//! OAuth user-grant consent / list / revoke tests.
//!
//! Split from the former monolithic `oauth_integration_tests.rs`
//! (by OAuth flow). Test bodies are unchanged.

pub(crate) use super::fixtures::*;

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
