//! Authentication integration tests (Epic 80, Story 80.2, Epic 99 Story 99.1).
//!
//! Tests all authentication endpoints end-to-end:
//! - User registration
//! - Login/logout flows
//! - Token refresh
//! - Password reset
//! - MFA flows
//! - Error responses
//!
//! Uses sqlx::test for test database isolation.

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use serde_json::{json, Value};
use sqlx::PgPool;

use crate::common::{
    cleanup_test_user, create_authenticated_user, create_authenticated_user_with_org,
    verify_user_email, TestApp, TestUser,
};

/// Test helper to create a JSON request
fn json_request(method: Method, uri: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

/// Test helper to create a request with auth header
fn auth_request(method: Method, uri: &str, token: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap()
}

/// Test helper to create a request with auth header and JSON body
fn auth_json_request(method: Method, uri: &str, token: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

// =============================================================================
// Registration Tests
// =============================================================================

#[cfg(test)]
mod registration {
    use super::*;

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_register_valid_user(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();

        // Clean up any existing test user
        cleanup_test_user(&pool, &user.email).await;

        let request = app
            .post("/api/v1/auth/register")
            .json(user.registration_body())
            .build();

        let response = app.execute(request).await;

        response.assert_status(StatusCode::CREATED);
        response.assert_json_field("message");
        // #956: `RegisterResponse` carries only a generic `message`; there is
        // deliberately no `user_id` field (anti-enumeration).

        // Cleanup
        cleanup_test_user(&pool, &user.email).await;
    }

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_register_duplicate_email(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();

        // Clean up any existing test user
        cleanup_test_user(&pool, &user.email).await;

        // Register first time
        let request1 = app
            .post("/api/v1/auth/register")
            .json(user.registration_body())
            .build();
        app.execute(request1)
            .await
            .assert_status(StatusCode::CREATED);

        // Try to register again with same email
        let request2 = app
            .post("/api/v1/auth/register")
            .json(user.registration_body())
            .build();
        let response = app.execute(request2).await;

        // #956: the already-registered path is byte-for-byte identical to a
        // fresh registration (201 + generic message) to avoid account
        // enumeration — no 409 / EMAIL_EXISTS is surfaced to the caller.
        response.assert_status(StatusCode::CREATED);
        response.assert_json_field("message");

        // Cleanup
        cleanup_test_user(&pool, &user.email).await;
    }

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_register_invalid_email(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let body = json!({
            "email": "not-an-email",
            "password": "SecurePassword123!",
            "name": "Test User"
        });

        let request = json_request(Method::POST, "/api/v1/auth/register", body);
        let response = app.execute(request).await;

        response.assert_status(StatusCode::BAD_REQUEST);
        let json = response.json_value();
        assert_eq!(json["code"].as_str().unwrap(), "INVALID_EMAIL");
    }

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_register_weak_password(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let body = json!({
            "email": "test@example.com",
            "password": "123",
            "name": "Test User"
        });

        let request = json_request(Method::POST, "/api/v1/auth/register", body);
        let response = app.execute(request).await;

        response.assert_status(StatusCode::BAD_REQUEST);
        let json = response.json_value();
        assert_eq!(json["code"].as_str().unwrap(), "VALIDATION_ERROR");
    }

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_register_empty_name(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let body = json!({
            "email": "test@example.com",
            "password": "SecurePassword123!",
            "name": ""
        });

        let request = json_request(Method::POST, "/api/v1/auth/register", body);
        let response = app.execute(request).await;

        response.assert_status(StatusCode::BAD_REQUEST);
        let json = response.json_value();
        assert_eq!(json["code"].as_str().unwrap(), "INVALID_NAME");
    }
}

// =============================================================================
// Login Tests
// =============================================================================

#[cfg(test)]
mod login {
    use super::*;

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_login_valid_credentials(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();

        // Register and verify user
        cleanup_test_user(&pool, &user.email).await;
        let reg_request = app
            .post("/api/v1/auth/register")
            .json(user.registration_body())
            .build();
        app.execute(reg_request).await;
        verify_user_email(&pool, &user.email).await;

        // Login
        let login_request = app
            .post("/api/v1/auth/login")
            .json(user.login_body())
            .build();
        let response = app.execute(login_request).await;

        response.assert_status(StatusCode::OK);
        response.assert_json_field("accessToken");
        response.assert_json_field("refreshToken");
        response.assert_json_field("expiresIn");
        response.assert_json_field("tokenType");

        let json = response.json_value();
        assert_eq!(json["tokenType"].as_str().unwrap(), "Bearer");

        // Cleanup
        cleanup_test_user(&pool, &user.email).await;
    }

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_login_invalid_password(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();

        // Register and verify user
        cleanup_test_user(&pool, &user.email).await;
        let reg_request = app
            .post("/api/v1/auth/register")
            .json(user.registration_body())
            .build();
        app.execute(reg_request).await;
        verify_user_email(&pool, &user.email).await;

        // Login with wrong password
        let body = json!({
            "email": user.email,
            "password": "WrongPassword123!"
        });

        let request = json_request(Method::POST, "/api/v1/auth/login", body);
        let response = app.execute(request).await;

        response.assert_status(StatusCode::UNAUTHORIZED);
        let json = response.json_value();
        assert_eq!(json["code"].as_str().unwrap(), "INVALID_CREDENTIALS");

        // Cleanup
        cleanup_test_user(&pool, &user.email).await;
    }

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_login_nonexistent_user(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let body = json!({
            "email": "nonexistent@example.com",
            "password": "AnyPassword123!"
        });

        let request = json_request(Method::POST, "/api/v1/auth/login", body);
        let response = app.execute(request).await;

        // Should return 401 (not 404 to prevent email enumeration)
        response.assert_status(StatusCode::UNAUTHORIZED);
        let json = response.json_value();
        assert_eq!(json["code"].as_str().unwrap(), "INVALID_CREDENTIALS");
    }

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_login_unverified_email(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();

        // Register but don't verify
        cleanup_test_user(&pool, &user.email).await;
        let reg_request = app
            .post("/api/v1/auth/register")
            .json(user.registration_body())
            .build();
        app.execute(reg_request).await;

        // Try to login without verification
        let login_request = app
            .post("/api/v1/auth/login")
            .json(user.login_body())
            .build();
        let response = app.execute(login_request).await;

        // #956: unverified-email login returns 401 (not 403) so it does not
        // become an account-enumeration oracle distinct from bad credentials.
        response.assert_status(StatusCode::UNAUTHORIZED);
        let json = response.json_value();
        assert_eq!(json["code"].as_str().unwrap(), "EMAIL_NOT_VERIFIED");

        // Cleanup
        cleanup_test_user(&pool, &user.email).await;
    }
}

// =============================================================================
// Token Refresh Tests
// =============================================================================

#[cfg(test)]
mod token_refresh {
    use super::*;

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_refresh_valid_token(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();

        // Register, verify, and login
        cleanup_test_user(&pool, &user.email).await;
        let (_, refresh_token) = create_authenticated_user(&app, &user).await;

        // Refresh token
        let body = json!({
            "refreshToken": refresh_token
        });

        let request = json_request(Method::POST, "/api/v1/auth/refresh", body);
        let response = app.execute(request).await;

        response.assert_status(StatusCode::OK);
        response.assert_json_field("accessToken");
        response.assert_json_field("expiresIn");

        // Cleanup
        cleanup_test_user(&pool, &user.email).await;
    }

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_refresh_invalid_token(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let body = json!({
            "refreshToken": "invalid-refresh-token"
        });

        let request = json_request(Method::POST, "/api/v1/auth/refresh", body);
        let response = app.execute(request).await;

        response.assert_status(StatusCode::UNAUTHORIZED);
    }

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_refresh_missing_token(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let body = json!({});

        let request = json_request(Method::POST, "/api/v1/auth/refresh", body);
        let response = app.execute(request).await;

        // A well-formed JSON body missing the required `refreshToken` field is
        // an axum `JsonDataError` → 422 Unprocessable Entity (not 400, which is
        // reserved for JSON syntax errors).
        response.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
    }

    /// Regression for #2238 / PR #2205: a present-but-empty `refresh_token=;`
    /// cookie must NOT shadow a valid token supplied in the JSON body.
    ///
    /// Before the fix, `parse_refresh_cookie` returned `Some("")` for an empty
    /// cookie, so `resolve_refresh_token` chose the empty cookie over the body
    /// token and `/auth/refresh` failed with `MISSING_TOKEN`. This is the
    /// handler-level counterpart to the `parse_refresh_cookie` unit test — it
    /// drives the real `refresh_token()` handler so a future refactor at the
    /// call site (e.g. dropping the empty-cookie guard or reordering
    /// precedence) can't silently reintroduce the bug.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn empty_cookie_does_not_shadow_body_token_on_refresh(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        cleanup_test_user(&pool, &user.email).await;

        let (_access, refresh_token) = create_authenticated_user(&app, &user).await;

        // Cookie-based clients that have "cleared" the cookie send
        // `refresh_token=;` (present but empty). The valid token lives in the
        // body. The handler must fall through to the body token, not stall.
        let request = app
            .post("/api/v1/auth/refresh")
            .header("Cookie", "refresh_token=;")
            .json(json!({ "refreshToken": refresh_token }))
            .build();
        let response = app.execute(request).await;

        response.assert_status(StatusCode::OK);

        // A successful refresh returns a rotated token pair, not MISSING_TOKEN.
        let body = response.json_value();
        assert!(
            body.get("accessToken").and_then(Value::as_str).is_some(),
            "empty cookie must not shadow the body token: expected a rotated \
             accessToken, got {body}"
        );
        let new_refresh = body
            .get("refreshToken")
            .and_then(Value::as_str)
            .expect("refresh response must carry a rotated refreshToken");
        assert_ne!(
            new_refresh, refresh_token,
            "refresh must rotate the token, not echo the input"
        );

        cleanup_test_user(&pool, &user.email).await;
    }

    /// Regression for #676: `/auth/refresh` must return a **fresh** tenant
    /// list from the database — not the membership set frozen at login time.
    ///
    /// Scenario:
    ///   1. Seed a user with active memberships in TWO organizations.
    ///   2. Log in (which previously bound the tenants list into the
    ///      response). Capture `tenants` from the login response and assert
    ///      both orgs are present.
    ///   3. Revoke the membership in org B server-side by flipping its
    ///      `organization_members.status` to `removed`.
    ///   4. Call `/auth/refresh` with the refresh token from step 2.
    ///   5. Assert the refresh response's `tenants` array contains ONLY
    ///      org A — the revoked org B must not survive.
    ///
    /// Pre-fix behaviour: `tenants` was either absent from the refresh
    /// response or carried the stale login-time list, so `deriveActiveRole`
    /// on the frontend could keep handing back a removed role until the
    /// user logged out and back in.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_refresh_returns_fresh_tenants_after_revoke(pool: PgPool) {
        use uuid::Uuid;

        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();

        // Register, verify, and login.
        cleanup_test_user(&pool, &user.email).await;
        let (_, refresh_token) = create_authenticated_user(&app, &user).await;

        // Look up the user's id so we can seed memberships against it.
        let user_id: Uuid = sqlx::query_scalar("SELECT id FROM users WHERE email = $1")
            .bind(&user.email)
            .fetch_one(&pool)
            .await
            .expect("user row exists after registration");

        // Seed two organizations and grant active membership in both.
        let org_a: Uuid = sqlx::query_scalar(
            r#"
            INSERT INTO organizations (name, slug, contact_email, status)
            VALUES ($1, $2, $3, 'active')
            RETURNING id
            "#,
        )
        .bind("Org A — kept")
        .bind(format!("org-a-{}", user.email))
        .bind(format!("org-a-{}", user.email))
        .fetch_one(&pool)
        .await
        .expect("seed org A");

        let org_b: Uuid = sqlx::query_scalar(
            r#"
            INSERT INTO organizations (name, slug, contact_email, status)
            VALUES ($1, $2, $3, 'active')
            RETURNING id
            "#,
        )
        .bind("Org B — revoked")
        .bind(format!("org-b-{}", user.email))
        .bind(format!("org-b-{}", user.email))
        .fetch_one(&pool)
        .await
        .expect("seed org B");

        for org_id in [org_a, org_b] {
            sqlx::query(
                r#"
                INSERT INTO organization_members
                    (organization_id, user_id, role_type, status, joined_at)
                VALUES ($1, $2, 'manager', 'active', NOW())
                "#,
            )
            .bind(org_id)
            .bind(user_id)
            .execute(&pool)
            .await
            .expect("seed membership");
        }

        // Sanity: a refresh BEFORE revocation must report both orgs. This
        // also exercises the rotation path so the next refresh below uses
        // the freshly-issued token.
        let pre_request = json_request(
            Method::POST,
            "/api/v1/auth/refresh",
            json!({ "refreshToken": refresh_token }),
        );
        let pre_response = app.execute(pre_request).await;
        pre_response.assert_status(StatusCode::OK);
        let pre_json = pre_response.json_value();
        let pre_tenants = pre_json["tenants"]
            .as_array()
            .expect("refresh response must include `tenants` array (#676)");
        let pre_ids: std::collections::HashSet<String> = pre_tenants
            .iter()
            .map(|t| t["tenantId"].as_str().unwrap_or_default().to_string())
            .collect();
        assert!(
            pre_ids.contains(&org_a.to_string()),
            "pre-revoke refresh tenants must contain org A; got {pre_ids:?}"
        );
        assert!(
            pre_ids.contains(&org_b.to_string()),
            "pre-revoke refresh tenants must contain org B; got {pre_ids:?}"
        );
        let rotated_refresh = pre_json["refreshToken"]
            .as_str()
            .expect("refresh response carries rotated refreshToken")
            .to_string();

        // Revoke org B directly in the DB — emulates an admin removing the
        // user from that organization mid-session.
        sqlx::query(
            r#"
            UPDATE organization_members
               SET status = 'removed'
             WHERE organization_id = $1 AND user_id = $2
            "#,
        )
        .bind(org_b)
        .bind(user_id)
        .execute(&pool)
        .await
        .expect("revoke org B membership");

        // Now refresh again. The fix MUST re-query memberships and drop
        // org B from the response.
        let post_request = json_request(
            Method::POST,
            "/api/v1/auth/refresh",
            json!({ "refreshToken": rotated_refresh }),
        );
        let post_response = app.execute(post_request).await;
        post_response.assert_status(StatusCode::OK);
        let post_json = post_response.json_value();
        let post_tenants = post_json["tenants"]
            .as_array()
            .expect("refresh response must include `tenants` array (#676)");
        let post_ids: std::collections::HashSet<String> = post_tenants
            .iter()
            .map(|t| t["tenantId"].as_str().unwrap_or_default().to_string())
            .collect();
        assert!(
            post_ids.contains(&org_a.to_string()),
            "post-revoke refresh must still contain org A; got {post_ids:?}"
        );
        assert!(
            !post_ids.contains(&org_b.to_string()),
            "post-revoke refresh MUST drop org B (revoked server-side); got {post_ids:?}"
        );

        // Cleanup
        cleanup_test_user(&pool, &user.email).await;
    }
}

// =============================================================================
// Logout Tests
// =============================================================================

#[cfg(test)]
mod logout {
    use super::*;

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_logout_valid_session(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();

        // Register, verify, and login
        cleanup_test_user(&pool, &user.email).await;
        let (access_token, refresh_token) = create_authenticated_user(&app, &user).await;

        // Logout
        let body = json!({
            "refreshToken": refresh_token
        });

        let request = auth_json_request(Method::POST, "/api/v1/auth/logout", &access_token, body);
        let response = app.execute(request).await;

        response.assert_status(StatusCode::OK);

        // Cleanup
        cleanup_test_user(&pool, &user.email).await;
    }

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_logout_without_auth(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let body = json!({
            "refreshToken": "some-token"
        });

        let request = json_request(Method::POST, "/api/v1/auth/logout", body);
        let response = app.execute(request).await;

        // `/auth/logout` has no auth extractor and is intentionally public: it
        // revokes the supplied refresh token (if found) and always returns 200,
        // so it can't be used to probe token validity (anti-enumeration).
        response.assert_status(StatusCode::OK);
    }

    /// Regression for #2238 / PR #2205: `/auth/logout` must revoke the session
    /// named by the body token even when a present-but-empty `refresh_token=;`
    /// cookie is also sent.
    ///
    /// Before the fix, `resolve_refresh_token` picked the empty cookie over the
    /// body token, so `logout` hashed the empty string, `find_by_token_hash`
    /// missed, and the real session was silently left live (logout still
    /// returned 200 for anti-enumeration, masking the no-op). We assert the
    /// session is *actually* revoked by proving the token no longer refreshes.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn empty_cookie_does_not_shadow_body_token_on_logout(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        cleanup_test_user(&pool, &user.email).await;

        let (_access, refresh_token) = create_authenticated_user(&app, &user).await;

        // Logout with an empty cookie shadowing a valid body token.
        let request = app
            .post("/api/v1/auth/logout")
            .header("Cookie", "refresh_token=;")
            .json(json!({ "refreshToken": refresh_token }))
            .build();
        app.execute(request).await.assert_status(StatusCode::OK);

        // The session must be genuinely revoked: refreshing that token now
        // fails. On the pre-fix (empty-string no-op) path logout was a no-op,
        // so this refresh would still succeed with 200 — this guards the real
        // revocation. The header carries the token so the empty-cookie path is
        // not exercised again here.
        let post = app
            .post("/api/v1/auth/refresh")
            .header("X-Refresh-Token", &refresh_token)
            .json(json!({ "refreshToken": "" }))
            .build();
        app.execute(post)
            .await
            .assert_status(StatusCode::UNAUTHORIZED);

        cleanup_test_user(&pool, &user.email).await;
    }
}

// =============================================================================
// Password Reset Tests
// =============================================================================

#[cfg(test)]
mod password_reset {
    use super::*;

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_request_password_reset(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();

        // Register and verify user
        cleanup_test_user(&pool, &user.email).await;
        let reg_request = app
            .post("/api/v1/auth/register")
            .json(user.registration_body())
            .build();
        app.execute(reg_request).await;
        verify_user_email(&pool, &user.email).await;

        // Request password reset
        let body = json!({
            "email": user.email
        });

        let request = json_request(Method::POST, "/api/v1/auth/forgot-password", body);
        let response = app.execute(request).await;

        // Should return 200 (even for nonexistent emails to prevent enumeration)
        response.assert_status(StatusCode::OK);

        // Cleanup
        cleanup_test_user(&pool, &user.email).await;
    }

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_request_password_reset_nonexistent_email(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let body = json!({
            "email": "nonexistent@example.com"
        });

        let request = json_request(Method::POST, "/api/v1/auth/forgot-password", body);
        let response = app.execute(request).await;

        // Should return 200 to prevent email enumeration
        response.assert_status(StatusCode::OK);
    }

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_reset_password_invalid_token(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let body = json!({
            "token": "invalid-reset-token",
            "newPassword": "NewSecurePassword123!"
        });

        let request = json_request(Method::POST, "/api/v1/auth/reset-password", body);
        let response = app.execute(request).await;

        // Should return 400 or 401 for invalid token
        assert!(
            response.status == StatusCode::BAD_REQUEST
                || response.status == StatusCode::UNAUTHORIZED
        );
    }

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_reset_password_weak_new_password(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let body = json!({
            "token": "some-token",
            "newPassword": "weak"
        });

        let request = json_request(Method::POST, "/api/v1/auth/reset-password", body);
        let response = app.execute(request).await;

        response.assert_status(StatusCode::BAD_REQUEST);
    }
}

// =============================================================================
// MFA Tests
// =============================================================================

#[cfg(test)]
mod mfa {
    use super::*;

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_enable_mfa(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();

        // Register, verify, and login. MFA endpoints use `RlsConnection`, which
        // requires an `X-Tenant-ID` header + an active `organization_members`
        // row, so provision an org and stamp the tenant header on the request.
        cleanup_test_user(&pool, &user.email).await;
        let (access_token, org_id) = create_authenticated_user_with_org(&app, &user, "mfa").await;

        // Enable MFA
        let request = app
            .post("/api/v1/auth/mfa/setup")
            .bearer(&access_token)
            .tenant(org_id)
            .build();
        let response = app.execute(request).await;

        response.assert_status(StatusCode::OK);
        response.assert_json_field("secret");
        // Backend uses serde rename_all = "camelCase" → qrUri (not qr_code_uri).
        // `MfaSetupResponse` carries only `secret` + `qrUri`; recovery codes are
        // issued by `/mfa/verify`, not `/mfa/setup`.
        response.assert_json_field("qrUri");

        // Cleanup
        cleanup_test_user(&pool, &user.email).await;
    }

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_verify_mfa_invalid_code(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();

        // Register, verify, and login
        cleanup_test_user(&pool, &user.email).await;
        let (access_token, _) = create_authenticated_user(&app, &user).await;

        // Setup MFA first
        let setup_request = auth_request(Method::POST, "/api/v1/auth/mfa/setup", &access_token);
        app.execute(setup_request).await;

        // Verify with invalid code
        let body = json!({
            "code": "000000"
        });

        let request =
            auth_json_request(Method::POST, "/api/v1/auth/mfa/verify", &access_token, body);
        let response = app.execute(request).await;

        response.assert_status(StatusCode::BAD_REQUEST);

        // Cleanup
        cleanup_test_user(&pool, &user.email).await;
    }

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_mfa_without_auth(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let request = Request::builder()
            .method(Method::POST)
            .uri("/api/v1/auth/mfa/setup")
            .body(Body::empty())
            .unwrap();

        let response = app.execute(request).await;

        response.assert_status(StatusCode::UNAUTHORIZED);
    }
}

// =============================================================================
// Session Management Tests
// =============================================================================

#[cfg(test)]
mod sessions {
    use super::*;

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_list_sessions(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();

        // Register, verify, and login
        cleanup_test_user(&pool, &user.email).await;
        let (access_token, _) = create_authenticated_user(&app, &user).await;

        // List sessions
        let request = auth_request(Method::GET, "/api/v1/auth/sessions", &access_token);
        let response = app.execute(request).await;

        response.assert_status(StatusCode::OK);
        response.assert_json_field("sessions");

        let json = response.json_value();
        let sessions = json["sessions"].as_array().unwrap();
        assert!(!sessions.is_empty(), "Should have at least one session");

        // Cleanup
        cleanup_test_user(&pool, &user.email).await;
    }

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_revoke_all_sessions(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();

        // Register, verify, and login
        cleanup_test_user(&pool, &user.email).await;
        let (access_token, _) = create_authenticated_user(&app, &user).await;

        // Revoke all sessions
        let request = auth_request(
            Method::POST,
            "/api/v1/auth/sessions/revoke-all",
            &access_token,
        );
        let response = app.execute(request).await;

        response.assert_status(StatusCode::OK);

        // Note: access tokens are stateless JWTs validated purely by signature
        // + expiry (no server-side denylist), and `revoke-all` only revokes
        // refresh tokens. So the existing access token keeps validating until
        // it expires — re-using it on GET /sessions still returns 200.
        let verify_request = auth_request(Method::GET, "/api/v1/auth/sessions", &access_token);
        let verify_response = app.execute(verify_request).await;

        assert_eq!(verify_response.status, StatusCode::OK);

        // Cleanup
        cleanup_test_user(&pool, &user.email).await;
    }

    // ------------------------------------------------------------------
    // Regression: revoke-all / list-sessions cookie blindness
    //
    // After the P0-12 cookie migration, ppt-web sends the refresh token ONLY
    // via the HttpOnly `refresh_token` cookie — never the `X-Refresh-Token`
    // header. `revoke_all_sessions` and `list_sessions` used to read the header
    // only, so cookie callers resolved to `None`:
    //   * revoke-all revoked the caller's OWN live session ("sign out other
    //     devices" signed the caller out too), and
    //   * list-sessions could never mark any row `isCurrent`.
    // The fix resolves the current session cookie-first / header-fallback via
    // `resolve_current_session_id`. These tests fail on `main` and pass after.
    // ------------------------------------------------------------------

    /// Log an already-registered+verified user in again to open a second
    /// concurrent session. Returns `(access_token, refresh_token)`.
    async fn login_again(app: &TestApp, user: &TestUser) -> (String, String) {
        let login_req = app
            .post("/api/v1/auth/login")
            .json(user.login_body())
            .build();
        let resp = app.execute(login_req).await;
        assert_eq!(resp.status, StatusCode::OK, "second login should succeed");
        let json = resp.json_value();
        (
            json["accessToken"]
                .as_str()
                .expect("accessToken")
                .to_string(),
            json["refreshToken"]
                .as_str()
                .expect("refreshToken")
                .to_string(),
        )
    }

    /// Cookie-based caller: revoke-all must revoke ONLY the other device and
    /// leave the caller's own session alive. On `main` the cookie is ignored,
    /// `current_session_id = None`, and BOTH sessions are revoked
    /// (`revokedCount == 2`) — the first assert below fails there.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn revoke_all_sessions_preserves_cookie_caller(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        cleanup_test_user(&pool, &user.email).await;

        // Session 1 is the caller; session 2 is another device.
        let (access1, refresh1) = create_authenticated_user(&app, &user).await;
        let (_access2, refresh2) = login_again(&app, &user).await;

        // Caller hits revoke-all as a cookie-based ppt-web client: refresh
        // token lives ONLY in the HttpOnly cookie, no X-Refresh-Token header.
        let req = app
            .post("/api/v1/auth/sessions/revoke-all")
            .bearer(&access1)
            .header("Cookie", &format!("refresh_token={}", refresh1))
            .build();
        let resp = app.execute(req).await;
        resp.assert_status(StatusCode::OK);

        let count = resp.json_value()["revokedCount"].as_u64().unwrap();
        assert_eq!(
            count, 1,
            "cookie caller must revoke only the other session (main revokes both)"
        );

        // Caller's own refresh token still works => session survived.
        let refresh_ok = app
            .post("/api/v1/auth/refresh")
            .header("Cookie", &format!("refresh_token={}", refresh1))
            .json(json!({ "refreshToken": "" }))
            .build();
        let refresh_resp = app.execute(refresh_ok).await;
        assert_eq!(
            refresh_resp.status,
            StatusCode::OK,
            "caller's own session must survive revoke-all"
        );

        // The other device's refresh token is now revoked.
        let refresh_bad = app
            .post("/api/v1/auth/refresh")
            .header("Cookie", &format!("refresh_token={}", refresh2))
            .json(json!({ "refreshToken": "" }))
            .build();
        let bad_resp = app.execute(refresh_bad).await;
        assert_eq!(
            bad_resp.status,
            StatusCode::UNAUTHORIZED,
            "the other device must be signed out"
        );

        cleanup_test_user(&pool, &user.email).await;
    }

    /// Header-based caller (mobile RN / legacy ppt-web): behavior is unchanged
    /// — revoke-all still preserves the caller's session. Passes on `main` too;
    /// guards against a regression in the fallback path.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn revoke_all_sessions_preserves_header_caller(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        cleanup_test_user(&pool, &user.email).await;

        let (access1, refresh1) = create_authenticated_user(&app, &user).await;
        let (_access2, _refresh2) = login_again(&app, &user).await;

        let req = app
            .post("/api/v1/auth/sessions/revoke-all")
            .bearer(&access1)
            .header("X-Refresh-Token", &refresh1)
            .build();
        let resp = app.execute(req).await;
        resp.assert_status(StatusCode::OK);

        let count = resp.json_value()["revokedCount"].as_u64().unwrap();
        assert_eq!(count, 1, "header caller must revoke only the other session");

        cleanup_test_user(&pool, &user.email).await;
    }

    /// No cookie and no header: neither source resolves a current session, so
    /// revoke-all revokes EVERY session for the user. Documents the
    /// intentional `None` branch of `revoke_all_user_tokens`.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn revoke_all_sessions_without_token_revokes_everything(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        cleanup_test_user(&pool, &user.email).await;

        let (access1, _refresh1) = create_authenticated_user(&app, &user).await;
        let (_access2, _refresh2) = login_again(&app, &user).await;

        let req = app
            .post("/api/v1/auth/sessions/revoke-all")
            .bearer(&access1)
            .build();
        let resp = app.execute(req).await;
        resp.assert_status(StatusCode::OK);

        let count = resp.json_value()["revokedCount"].as_u64().unwrap();
        assert_eq!(
            count, 2,
            "with no current session, all sessions are revoked"
        );

        cleanup_test_user(&pool, &user.email).await;
    }

    /// Cookie-based caller: list-sessions must mark exactly one row
    /// `isCurrent = true` and it must be the caller's session. On `main` the
    /// cookie is ignored so no row is ever current (count == 0) — fails there.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn list_sessions_marks_cookie_caller_current(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        cleanup_test_user(&pool, &user.email).await;

        let (access1, refresh1) = create_authenticated_user(&app, &user).await;
        let (_access2, _refresh2) = login_again(&app, &user).await;

        // Session 1 (the caller) is the earliest-created refresh_tokens row for
        // this user; session 2 came from `login_again`. Bind `isCurrent` to
        // that specific id so a resolver that flagged the *wrong* session would
        // fail — a bare count assertion would not catch it.
        let session1_id: uuid::Uuid = sqlx::query_scalar(
            "SELECT id FROM refresh_tokens \
             WHERE user_id = (SELECT id FROM users WHERE email = $1) \
             ORDER BY created_at ASC LIMIT 1",
        )
        .bind(&user.email)
        .fetch_one(&pool)
        .await
        .expect("session 1 row");

        let req = app
            .get("/api/v1/auth/sessions")
            .bearer(&access1)
            .header("Cookie", &format!("refresh_token={}", refresh1))
            .build();
        let resp = app.execute(req).await;
        resp.assert_status(StatusCode::OK);

        let json = resp.json_value();
        let sessions = json["sessions"].as_array().unwrap();
        assert_eq!(sessions.len(), 2, "user has two live sessions");

        let current: Vec<&Value> = sessions
            .iter()
            .filter(|s| s["isCurrent"].as_bool().unwrap_or(false))
            .collect();
        assert_eq!(
            current.len(),
            1,
            "exactly one session must be marked current for the cookie caller"
        );
        assert_eq!(
            current[0]["id"].as_str().unwrap(),
            session1_id.to_string(),
            "the current row must be the caller's own (session 1), not the other device"
        );

        cleanup_test_user(&pool, &user.email).await;
    }

    /// Header-based caller (mobile RN / legacy ppt-web): list-sessions must mark
    /// exactly one row `isCurrent = true` and it must be the caller's session,
    /// resolved via the `X-Refresh-Token` fallback. Mirrors the cookie test to
    /// guard the header branch symmetrically with the revoke-all tests. On
    /// `main` the header path already worked; this locks in identity, not just
    /// count.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn list_sessions_marks_header_caller_current(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();
        cleanup_test_user(&pool, &user.email).await;

        let (access1, refresh1) = create_authenticated_user(&app, &user).await;
        let (_access2, _refresh2) = login_again(&app, &user).await;

        let session1_id: uuid::Uuid = sqlx::query_scalar(
            "SELECT id FROM refresh_tokens \
             WHERE user_id = (SELECT id FROM users WHERE email = $1) \
             ORDER BY created_at ASC LIMIT 1",
        )
        .bind(&user.email)
        .fetch_one(&pool)
        .await
        .expect("session 1 row");

        let req = app
            .get("/api/v1/auth/sessions")
            .bearer(&access1)
            .header("X-Refresh-Token", &refresh1)
            .build();
        let resp = app.execute(req).await;
        resp.assert_status(StatusCode::OK);

        let json = resp.json_value();
        let sessions = json["sessions"].as_array().unwrap();
        assert_eq!(sessions.len(), 2, "user has two live sessions");

        let current: Vec<&Value> = sessions
            .iter()
            .filter(|s| s["isCurrent"].as_bool().unwrap_or(false))
            .collect();
        assert_eq!(
            current.len(),
            1,
            "exactly one session must be marked current for the header caller"
        );
        assert_eq!(
            current[0]["id"].as_str().unwrap(),
            session1_id.to_string(),
            "the current row must be the caller's own (session 1) via the header fallback"
        );

        cleanup_test_user(&pool, &user.email).await;
    }
}

// =============================================================================
// Error Response Tests
// =============================================================================

#[cfg(test)]
mod error_responses {
    use super::*;

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_missing_content_type(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let request = Request::builder()
            .method(Method::POST)
            .uri("/api/v1/auth/login")
            .body(Body::from(r#"{"email":"test@example.com"}"#))
            .unwrap();

        let response = app.execute(request).await;

        // Should return 415 Unsupported Media Type or 400 Bad Request
        assert!(
            response.status == StatusCode::UNSUPPORTED_MEDIA_TYPE
                || response.status == StatusCode::BAD_REQUEST
        );
    }

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_malformed_json(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let request = Request::builder()
            .method(Method::POST)
            .uri("/api/v1/auth/login")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from("not valid json"))
            .unwrap();

        let response = app.execute(request).await;

        response.assert_status(StatusCode::BAD_REQUEST);
    }

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_missing_required_fields(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let body = json!({
            "email": "test@example.com"
            // missing password
        });

        let request = json_request(Method::POST, "/api/v1/auth/login", body);
        let response = app.execute(request).await;

        // Well-formed JSON missing the required `password` field is an axum
        // `JsonDataError` → 422 (400 is reserved for JSON syntax errors, which
        // `test_malformed_json` / `test_empty_body` cover).
        response.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_empty_body(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let request = Request::builder()
            .method(Method::POST)
            .uri("/api/v1/auth/login")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::empty())
            .unwrap();

        let response = app.execute(request).await;

        response.assert_status(StatusCode::BAD_REQUEST);
    }
}
