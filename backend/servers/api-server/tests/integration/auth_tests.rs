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

mod common;

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use serde_json::{json, Value};
use sqlx::PgPool;

use common::{cleanup_test_user, create_authenticated_user, verify_user_email, TestApp, TestUser};

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

    #[sqlx::test]
    async fn test_register_valid_user(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();

        // Clean up any existing test user
        cleanup_test_user(&pool, &user.email).await;

        let request = app
            .post("/api/v1/auth/register")
            .json(&user.registration_body())
            .build();

        let response = app.execute(request).await;

        response.assert_status(StatusCode::CREATED);
        response.assert_json_field("message");
        response.assert_json_field("user_id");

        // Cleanup
        cleanup_test_user(&pool, &user.email).await;
    }

    #[sqlx::test]
    async fn test_register_duplicate_email(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();

        // Clean up any existing test user
        cleanup_test_user(&pool, &user.email).await;

        // Register first time
        let request1 = app
            .post("/api/v1/auth/register")
            .json(&user.registration_body())
            .build();
        app.execute(request1).await.assert_status(StatusCode::CREATED);

        // Try to register again with same email
        let request2 = app
            .post("/api/v1/auth/register")
            .json(&user.registration_body())
            .build();
        let response = app.execute(request2).await;

        response.assert_status(StatusCode::CONFLICT);
        let json = response.json_value();
        assert_eq!(json["code"].as_str().unwrap(), "EMAIL_EXISTS");

        // Cleanup
        cleanup_test_user(&pool, &user.email).await;
    }

    #[sqlx::test]
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

    #[sqlx::test]
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

    #[sqlx::test]
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

    #[sqlx::test]
    async fn test_login_valid_credentials(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();

        // Register and verify user
        cleanup_test_user(&pool, &user.email).await;
        let reg_request = app
            .post("/api/v1/auth/register")
            .json(&user.registration_body())
            .build();
        app.execute(reg_request).await;
        verify_user_email(&pool, &user.email).await;

        // Login
        let login_request = app
            .post("/api/v1/auth/login")
            .json(&user.login_body())
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

    #[sqlx::test]
    async fn test_login_invalid_password(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();

        // Register and verify user
        cleanup_test_user(&pool, &user.email).await;
        let reg_request = app
            .post("/api/v1/auth/register")
            .json(&user.registration_body())
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

    #[sqlx::test]
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

    #[sqlx::test]
    async fn test_login_unverified_email(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();

        // Register but don't verify
        cleanup_test_user(&pool, &user.email).await;
        let reg_request = app
            .post("/api/v1/auth/register")
            .json(&user.registration_body())
            .build();
        app.execute(reg_request).await;

        // Try to login without verification
        let login_request = app
            .post("/api/v1/auth/login")
            .json(&user.login_body())
            .build();
        let response = app.execute(login_request).await;

        response.assert_status(StatusCode::FORBIDDEN);
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

    #[sqlx::test]
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

    #[sqlx::test]
    async fn test_refresh_invalid_token(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let body = json!({
            "refreshToken": "invalid-refresh-token"
        });

        let request = json_request(Method::POST, "/api/v1/auth/refresh", body);
        let response = app.execute(request).await;

        response.assert_status(StatusCode::UNAUTHORIZED);
    }

    #[sqlx::test]
    async fn test_refresh_missing_token(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let body = json!({});

        let request = json_request(Method::POST, "/api/v1/auth/refresh", body);
        let response = app.execute(request).await;

        response.assert_status(StatusCode::BAD_REQUEST);
    }
}

// =============================================================================
// Logout Tests
// =============================================================================

#[cfg(test)]
mod logout {
    use super::*;

    #[sqlx::test]
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

    #[sqlx::test]
    async fn test_logout_without_auth(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let body = json!({
            "refreshToken": "some-token"
        });

        let request = json_request(Method::POST, "/api/v1/auth/logout", body);
        let response = app.execute(request).await;

        response.assert_status(StatusCode::UNAUTHORIZED);
    }
}

// =============================================================================
// Password Reset Tests
// =============================================================================

#[cfg(test)]
mod password_reset {
    use super::*;

    #[sqlx::test]
    async fn test_request_password_reset(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();

        // Register and verify user
        cleanup_test_user(&pool, &user.email).await;
        let reg_request = app
            .post("/api/v1/auth/register")
            .json(&user.registration_body())
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

    #[sqlx::test]
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

    #[sqlx::test]
    async fn test_reset_password_invalid_token(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let body = json!({
            "token": "invalid-reset-token",
            "password": "NewSecurePassword123!"
        });

        let request = json_request(Method::POST, "/api/v1/auth/reset-password", body);
        let response = app.execute(request).await;

        // Should return 400 or 401 for invalid token
        assert!(
            response.status == StatusCode::BAD_REQUEST
                || response.status == StatusCode::UNAUTHORIZED
        );
    }

    #[sqlx::test]
    async fn test_reset_password_weak_new_password(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let body = json!({
            "token": "some-token",
            "password": "weak"
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

    #[sqlx::test]
    async fn test_enable_mfa(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();

        // Register, verify, and login
        cleanup_test_user(&pool, &user.email).await;
        let (access_token, _) = create_authenticated_user(&app, &user).await;

        // Enable MFA
        let request = auth_request(Method::POST, "/api/v1/auth/mfa/setup", &access_token);
        let response = app.execute(request).await;

        response.assert_status(StatusCode::OK);
        response.assert_json_field("secret");
        response.assert_json_field("qr_code_uri");
        response.assert_json_field("backup_codes");

        // Cleanup
        cleanup_test_user(&pool, &user.email).await;
    }

    #[sqlx::test]
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

        let request = auth_json_request(Method::POST, "/api/v1/auth/mfa/verify", &access_token, body);
        let response = app.execute(request).await;

        response.assert_status(StatusCode::BAD_REQUEST);

        // Cleanup
        cleanup_test_user(&pool, &user.email).await;
    }

    #[sqlx::test]
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

    #[sqlx::test]
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
        assert!(sessions.len() >= 1, "Should have at least one session");

        // Cleanup
        cleanup_test_user(&pool, &user.email).await;
    }

    #[sqlx::test]
    async fn test_revoke_all_sessions(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();

        // Register, verify, and login
        cleanup_test_user(&pool, &user.email).await;
        let (access_token, _) = create_authenticated_user(&app, &user).await;

        // Revoke all sessions
        let request = auth_request(Method::POST, "/api/v1/auth/sessions/revoke-all", &access_token);
        let response = app.execute(request).await;

        response.assert_status(StatusCode::OK);

        // Old token should no longer work
        let verify_request = auth_request(Method::GET, "/api/v1/auth/sessions", &access_token);
        let verify_response = app.execute(verify_request).await;

        assert_eq!(verify_response.status, StatusCode::UNAUTHORIZED);

        // Cleanup
        cleanup_test_user(&pool, &user.email).await;
    }
}

// =============================================================================
// Error Response Tests
// =============================================================================

#[cfg(test)]
mod error_responses {
    use super::*;

    #[sqlx::test]
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

    #[sqlx::test]
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

    #[sqlx::test]
    async fn test_missing_required_fields(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let body = json!({
            "email": "test@example.com"
            // missing password
        });

        let request = json_request(Method::POST, "/api/v1/auth/login", body);
        let response = app.execute(request).await;

        response.assert_status(StatusCode::BAD_REQUEST);
    }

    #[sqlx::test]
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
