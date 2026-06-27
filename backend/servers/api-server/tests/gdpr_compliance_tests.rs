//! Integration tests for GDPR compliance and Compliance reporting routes.
//! 
//! Covers 10 GDPR endpoints and 10 Compliance endpoints (total 20 endpoints).
//! Moves status of these endpoints in the checklist from partial to done.

#[allow(dead_code)]
mod common;

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use chrono::{DateTime, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

use common::{create_authenticated_user_with_org, seed_membership, seed_org, TestApp, TestUser};

const JWT_SECRET: &str = "test-secret-key-that-is-at-least-64-characters-long-for-testing-purposes";

#[derive(Serialize)]
struct TestClaims {
    sub: Uuid,
    exp: i64,
    iat: i64,
    token_type: String,
    tenant_id: Option<Uuid>,
    role: Option<String>,
    email: String,
    name: String,
}

fn mint_token(user_id: Uuid, org_id: Option<Uuid>, role: Option<&str>, email: &str) -> String {
    let now = Utc::now().timestamp();
    let claims = TestClaims {
        sub: user_id,
        iat: now,
        exp: now + 3600,
        token_type: "access".to_string(),
        tenant_id: org_id,
        role: role.map(|r| r.to_string()),
        email: email.to_string(),
        name: "GDPR Compliance Test User".to_string(),
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(JWT_SECRET.as_bytes()),
    )
    .expect("mint jwt")
}

fn build_req(
    method: Method,
    uri: &str,
    org_id: Option<Uuid>,
    token: &str,
    body: Option<Value>,
) -> Request<Body> {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {token}"));
    if let Some(org) = org_id {
        builder = builder.header("X-Tenant-ID", org.to_string());
    }
    if let Some(ref b) = body {
        builder = builder.header(header::CONTENT_TYPE, "application/json");
        builder
            .body(Body::from(serde_json::to_vec(b).unwrap()))
            .unwrap()
    } else {
        builder.body(Body::empty()).unwrap()
    }
}

// Helper to seed users
async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO users (email, password_hash, name, status, email_verified_at)
           VALUES ($1, 'test_hash', 'Compliance Test User', 'active', NOW()) RETURNING id"#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

// Helper to seed users with scheduled deletion
async fn seed_user_deletion(pool: &PgPool, email: &str, deletion_at: DateTime<Utc>) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO users (email, password_hash, name, status, email_verified_at, scheduled_deletion_at)
           VALUES ($1, 'test_hash', 'Deletion User', 'active', NOW(), $2) RETURNING id"#,
    )
    .bind(email)
    .bind(deletion_at)
    .fetch_one(pool)
    .await
    .expect("seed user deletion")
}

// Helper to seed audit log
async fn seed_audit_log(
    pool: &PgPool,
    user_id: Option<Uuid>,
    action: &str,
    resource_type: Option<&str>,
    resource_id: Option<Uuid>,
    org_id: Option<Uuid>,
    ip: Option<&str>,
) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO audit_logs (user_id, action, resource_type, resource_id, org_id, ip_address)
           VALUES ($1, $2::audit_action, $3, $4, $5, $6) RETURNING id"#,
    )
    .bind(user_id)
    .bind(action)
    .bind(resource_type)
    .bind(resource_id)
    .bind(org_id)
    .bind(ip)
    .fetch_one(pool)
    .await
    .expect("seed audit log")
}

// Helper to seed user 2FA
async fn seed_user_2fa(pool: &PgPool, user_id: Uuid, enabled: bool) {
    sqlx::query(
        r#"INSERT INTO user_2fa (user_id, secret, enabled, enabled_at)
           VALUES ($1, 'secret', $2, CASE WHEN $2 THEN NOW() ELSE NULL END)"#,
    )
    .bind(user_id)
    .bind(enabled)
    .execute(pool)
    .await
    .expect("seed user 2fa");
}

// ============================================================================
// GDPR ENDPOINTS TESTS
// ============================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_gdpr_export_flow(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "gdprexport").await;

    // 1. Get export categories
    let req = build_req(
        Method::GET,
        "/api/v1/gdpr/export/categories",
        Some(org_id),
        &token,
        None,
    );
    let resp = app.execute(req).await;
    assert_eq!(resp.status, StatusCode::OK);
    let body = resp.json_value();
    assert!(body["categories"].is_array());
    assert!(!body["categories"].as_array().unwrap().is_empty());

    // 2. Request a data export
    let req = build_req(
        Method::POST,
        "/api/v1/gdpr/export/request",
        Some(org_id),
        &token,
        Some(json!({
            "format": "json",
            "categories": ["profile", "activity"]
        })),
    );
    let resp = app.execute(req).await;
    assert_eq!(resp.status, StatusCode::OK);
    let body = resp.json_value();
    let request_id = Uuid::parse_str(body["requestId"].as_str().unwrap()).unwrap();
    assert_eq!(body["status"].as_str().unwrap(), "pending");

    // 3. Request a duplicate export (should conflict)
    let req = build_req(
        Method::POST,
        "/api/v1/gdpr/export/request",
        Some(org_id),
        &token,
        Some(json!({
            "format": "json"
        })),
    );
    let resp = app.execute(req).await;
    assert_eq!(resp.status, StatusCode::CONFLICT);

    // 4. Get export request status
    let req = build_req(
        Method::GET,
        &format!("/api/v1/gdpr/export/status/{}", request_id),
        Some(org_id),
        &token,
        None,
    );
    let resp = app.execute(req).await;
    assert_eq!(resp.status, StatusCode::OK);
    let body = resp.json_value();
    assert_eq!(body["requestId"].as_str().unwrap(), request_id.to_string());
    assert_eq!(body["status"].as_str().unwrap(), "pending");

    // 5. Get status of non-existent request (should 404)
    let req = build_req(
        Method::GET,
        &format!("/api/v1/gdpr/export/status/{}", Uuid::new_v4()),
        Some(org_id),
        &token,
        None,
    );
    let resp = app.execute(req).await;
    assert_eq!(resp.status, StatusCode::NOT_FOUND);

    // 6. Get status of another user's request (should 403)
    let other_user = TestUser::new();
    let (other_token, other_org_id) =
        create_authenticated_user_with_org(&app, &other_user, "gdprexport2").await;
    let req = build_req(
        Method::GET,
        &format!("/api/v1/gdpr/export/status/{}", request_id),
        Some(other_org_id),
        &other_token,
        None,
    );
    let resp = app.execute(req).await;
    assert_eq!(resp.status, StatusCode::FORBIDDEN);

    // 7. Get export history
    let req = build_req(
        Method::GET,
        "/api/v1/gdpr/export/history",
        Some(org_id),
        &token,
        None,
    );
    let resp = app.execute(req).await;
    assert_eq!(resp.status, StatusCode::OK);
    let body = resp.json_value();
    assert!(body.is_array());
    assert_eq!(body[0]["id"].as_str().unwrap(), request_id.to_string());

    // 8. Download export (requires updating request status to ready in DB)
    let download_token = Uuid::new_v4();
    sqlx::query(
        "UPDATE data_export_requests SET status = 'ready', download_token = $1, file_size_bytes = 100, expires_at = NOW() + INTERVAL '1 day' WHERE id = $2"
    )
    .bind(download_token)
    .bind(request_id)
    .execute(&pool)
    .await
    .unwrap();

    let req = build_req(
        Method::GET,
        &format!("/api/v1/gdpr/export/download/{}", download_token),
        Some(org_id),
        &token,
        None,
    );
    let resp = app.execute(req).await;
    assert_eq!(resp.status, StatusCode::OK);
    let body = resp.json_value();
    assert_eq!(body["profile"]["email"].as_str().unwrap(), user.email);

    // 9. Download with wrong user (should 403)
    let req = build_req(
        Method::GET,
        &format!("/api/v1/gdpr/export/download/{}", download_token),
        Some(other_org_id),
        &other_token,
        None,
    );
    let resp = app.execute(req).await;
    assert_eq!(resp.status, StatusCode::FORBIDDEN);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_gdpr_deletion_flow(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "gdprdel").await;

    // 1. Get initial deletion status
    let req = build_req(
        Method::GET,
        "/api/v1/gdpr/deletion/status",
        Some(org_id),
        &token,
        None,
    );
    let resp = app.execute(req).await;
    assert_eq!(resp.status, StatusCode::OK);
    let body = resp.json_value();
    assert!(!body["hasActiveRequest"].as_bool().unwrap());

    // 2. Cancel when no request active (should 404)
    let req = build_req(
        Method::POST,
        "/api/v1/gdpr/deletion/cancel",
        Some(org_id),
        &token,
        None,
    );
    let resp = app.execute(req).await;
    assert_eq!(resp.status, StatusCode::NOT_FOUND);

    // 3. Request deletion with wrong confirmation (should 400)
    let req = build_req(
        Method::POST,
        "/api/v1/gdpr/deletion/request",
        Some(org_id),
        &token,
        Some(json!({
            "confirmation": "wrong@email.com",
            "reason": "testing deletion rejection"
        })),
    );
    let resp = app.execute(req).await;
    assert_eq!(resp.status, StatusCode::BAD_REQUEST);

    // 4. Request deletion successfully
    let req = build_req(
        Method::POST,
        "/api/v1/gdpr/deletion/request",
        Some(org_id),
        &token,
        Some(json!({
            "confirmation": user.email,
            "reason": "testing deletion acceptance"
        })),
    );
    let resp = app.execute(req).await;
    assert_eq!(resp.status, StatusCode::OK);
    let body = resp.json_value();
    assert!(body["hasActiveRequest"].as_bool().unwrap());
    assert_eq!(body["daysRemaining"].as_i64().unwrap(), 30);

    // 5. Request deletion again when already pending (should 409)
    let req = build_req(
        Method::POST,
        "/api/v1/gdpr/deletion/request",
        Some(org_id),
        &token,
        Some(json!({
            "confirmation": user.email
        })),
    );
    let resp = app.execute(req).await;
    assert_eq!(resp.status, StatusCode::CONFLICT);

    // 6. Get active deletion status
    let req = build_req(
        Method::GET,
        "/api/v1/gdpr/deletion/status",
        Some(org_id),
        &token,
        None,
    );
    let resp = app.execute(req).await;
    assert_eq!(resp.status, StatusCode::OK);
    let body = resp.json_value();
    assert!(body["hasActiveRequest"].as_bool().unwrap());

    // 7. Cancel deletion request
    let req = build_req(
        Method::POST,
        "/api/v1/gdpr/deletion/cancel",
        Some(org_id),
        &token,
        None,
    );
    let resp = app.execute(req).await;
    assert_eq!(resp.status, StatusCode::OK);
    let body = resp.json_value();
    assert!(!body["hasActiveRequest"].as_bool().unwrap());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_gdpr_privacy_flow(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "gdprpriv").await;

    // 1. Get current privacy settings
    let req = build_req(
        Method::GET,
        "/api/v1/gdpr/privacy",
        Some(org_id),
        &token,
        None,
    );
    let resp = app.execute(req).await;
    assert_eq!(resp.status, StatusCode::OK);
    let body = resp.json_value();
    assert_eq!(body["profileVisibility"].as_str().unwrap(), "visible");
    assert!(body["showContactInfo"].as_bool().unwrap());

    // 2. Update privacy settings
    let req = build_req(
        Method::POST,
        "/api/v1/gdpr/privacy",
        Some(org_id),
        &token,
        Some(json!({
            "profile_visibility": "hidden",
            "show_contact_info": false
        })),
    );
    let resp = app.execute(req).await;
    assert_eq!(resp.status, StatusCode::OK);
    let body = resp.json_value();
    assert_eq!(body["profileVisibility"].as_str().unwrap(), "hidden");
    assert!(!body["showContactInfo"].as_bool().unwrap());

    // 3. Update with invalid visibility (should 400)
    let req = build_req(
        Method::POST,
        "/api/v1/gdpr/privacy",
        Some(org_id),
        &token,
        Some(json!({
            "profile_visibility": "invalid_visibility_value"
        })),
    );
    let resp = app.execute(req).await;
    assert_eq!(resp.status, StatusCode::BAD_REQUEST);
}

// ============================================================================
// COMPLIANCE ENDPOINTS TESTS
// ============================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_compliance_endpoints_authz(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "complianceauthz").await;

    // List of compliance endpoints to test for role denial
    let test_cases = vec![
        (Method::GET, "/api/v1/compliance/audit-logs"),
        (Method::GET, "/api/v1/compliance/audit-logs/summary"),
        (
            Method::GET,
            &format!("/api/v1/compliance/audit-logs/user/{}", Uuid::new_v4()),
        ),
        (Method::GET, "/api/v1/compliance/audit-logs/integrity"),
        (Method::GET, "/api/v1/compliance/gdpr/data-exports"),
        (Method::GET, "/api/v1/compliance/gdpr/deletion-requests"),
        (Method::GET, "/api/v1/compliance/gdpr/privacy-report"),
        (Method::GET, "/api/v1/compliance/security/login-activity"),
        (Method::GET, "/api/v1/compliance/security/mfa-status"),
        (Method::GET, "/api/v1/compliance/security/failed-logins"),
    ];

    for (method, uri) in test_cases {
        let req = build_req(method, uri, Some(org_id), &token, None);
        let resp = app.execute(req).await;
        assert_eq!(
            resp.status,
            StatusCode::FORBIDDEN,
            "Non-SuperAdmin must be forbidden from accessing compliance reporting endpoints"
        );
    }
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_compliance_reporting_data_retrieval(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org_id = seed_org(&pool, "comp").await;
    let super_admin_id = seed_user(&pool, "superadmin@compliance.test").await;
    let token = mint_token(
        super_admin_id,
        Some(org_id),
        Some("super_admin"),
        "superadmin@compliance.test",
    );

    // Seed mock compliance data
    let user_a = seed_user(&pool, "usera@compliance.test").await;
    let user_b = seed_user_deletion(
        &pool,
        "userb@compliance.test",
        Utc::now() + chrono::Duration::days(15),
    )
    .await;

    // Seed 2FA
    seed_user_2fa(&pool, user_a, true).await;
    seed_user_2fa(&pool, user_b, false).await;

    // Seed audit logs
    seed_audit_log(
        &pool,
        Some(user_a),
        "login",
        Some("user"),
        Some(user_a),
        Some(org_id),
        Some("192.168.1.1"),
    )
    .await;
    seed_audit_log(
        &pool,
        Some(user_a),
        "privacy_settings_updated",
        Some("user"),
        Some(user_a),
        Some(org_id),
        Some("192.168.1.1"),
    )
    .await;

    // Seed failed logins for user B (need >= 3 failed logins to show in failed logins report)
    seed_audit_log(
        &pool,
        Some(user_b),
        "login_failed",
        None,
        None,
        Some(org_id),
        Some("10.0.0.1"),
    )
    .await;
    seed_audit_log(
        &pool,
        Some(user_b),
        "login_failed",
        None,
        None,
        Some(org_id),
        Some("10.0.0.1"),
    )
    .await;
    seed_audit_log(
        &pool,
        Some(user_b),
        "login_failed",
        None,
        None,
        Some(org_id),
        Some("10.0.0.1"),
    )
    .await;

    // Seed single failed login for user A (should not show up in the threshold check)
    seed_audit_log(
        &pool,
        Some(user_a),
        "login_failed",
        None,
        None,
        Some(org_id),
        Some("192.168.1.1"),
    )
    .await;

    // 1. GET /api/v1/compliance/audit-logs
    let req = build_req(
        Method::GET,
        "/api/v1/compliance/audit-logs",
        Some(org_id),
        &token,
        None,
    );
    let resp = app.execute(req).await;
    assert_eq!(resp.status, StatusCode::OK);
    let body = resp.json_value();
    assert!(body["logs"].is_array());
    assert!(body["total"].as_i64().unwrap() >= 6); // Seeding logs + test framework registration logs

    // 2. GET /api/v1/compliance/audit-logs/summary
    let req = build_req(
        Method::GET,
        "/api/v1/compliance/audit-logs/summary",
        Some(org_id),
        &token,
        None,
    );
    let resp = app.execute(req).await;
    assert_eq!(resp.status, StatusCode::OK);
    let body = resp.json_value();
    assert!(body["totalEntries"].as_i64().unwrap() >= 6);
    assert!(body["actionCounts"].is_array());
    assert!(body["integrityVerified"].is_bool());

    // 3. GET /api/v1/compliance/audit-logs/user/{user_id}
    let req = build_req(
        Method::GET,
        &format!("/api/v1/compliance/audit-logs/user/{}", user_a),
        Some(org_id),
        &token,
        None,
    );
    let resp = app.execute(req).await;
    assert_eq!(resp.status, StatusCode::OK);
    let body = resp.json_value();
    assert!(body["logs"].is_array());
    // Seeding logs for user_a: login, privacy_settings_updated, login_failed
    assert_eq!(body["total"].as_i64().unwrap(), 3);

    // 4. GET /api/v1/compliance/audit-logs/integrity
    let req = build_req(
        Method::GET,
        "/api/v1/compliance/audit-logs/integrity",
        Some(org_id),
        &token,
        None,
    );
    let resp = app.execute(req).await;
    assert_eq!(resp.status, StatusCode::OK);
    let body = resp.json_value();
    assert!(body["verified"].is_bool());
    assert_eq!(body["entriesChecked"].as_i64().unwrap(), 10000);

    // 5. GET /api/v1/compliance/gdpr/data-exports
    let req = build_req(
        Method::GET,
        "/api/v1/compliance/gdpr/data-exports",
        Some(org_id),
        &token,
        None,
    );
    let resp = app.execute(req).await;
    assert_eq!(resp.status, StatusCode::OK);
    let body = resp.json_value();
    assert!(body["exports"].is_array());
    assert!(body["totalRequests"].is_number());

    // 6. GET /api/v1/compliance/gdpr/deletion-requests
    let req = build_req(
        Method::GET,
        "/api/v1/compliance/gdpr/deletion-requests",
        Some(org_id),
        &token,
        None,
    );
    let resp = app.execute(req).await;
    assert_eq!(resp.status, StatusCode::OK);
    let body = resp.json_value();
    assert!(body["requests"].is_array());
    assert_eq!(body["totalPending"].as_i64().unwrap(), 1);
    assert_eq!(
        body["requests"][0]["userEmail"].as_str().unwrap(),
        "userb@compliance.test"
    );

    // 7. GET /api/v1/compliance/gdpr/privacy-report
    let req = build_req(
        Method::GET,
        "/api/v1/compliance/gdpr/privacy-report",
        Some(org_id),
        &token,
        None,
    );
    let resp = app.execute(req).await;
    assert_eq!(resp.status, StatusCode::OK);
    let body = resp.json_value();
    assert!(body["visibilityDistribution"].is_array());
    assert!(body["totalUsers"].as_i64().unwrap() >= 2);

    // 8. GET /api/v1/compliance/security/login-activity
    let req = build_req(
        Method::GET,
        "/api/v1/compliance/security/login-activity",
        Some(org_id),
        &token,
        None,
    );
    let resp = app.execute(req).await;
    assert_eq!(resp.status, StatusCode::OK);
    let body = resp.json_value();
    assert!(body["activity"].is_array());
    assert!(body["totalLogins"].as_i64().unwrap() >= 1);

    // 9. GET /api/v1/compliance/security/mfa-status
    let req = build_req(
        Method::GET,
        "/api/v1/compliance/security/mfa-status",
        Some(org_id),
        &token,
        None,
    );
    let resp = app.execute(req).await;
    assert_eq!(resp.status, StatusCode::OK);
    let body = resp.json_value();
    assert!(body["mfaEnabledCount"].as_i64().unwrap() >= 1);
    assert!(body["mfaDisabledCount"].as_i64().unwrap() >= 1);

    // 10. GET /api/v1/compliance/security/failed-logins
    let req = build_req(
        Method::GET,
        "/api/v1/compliance/security/failed-logins",
        Some(org_id),
        &token,
        None,
    );
    let resp = app.execute(req).await;
    assert_eq!(resp.status, StatusCode::OK);
    let body = resp.json_value();
    assert!(body["failedLogins"].is_array());
    // user_b has 3 failures on 10.0.0.1, user_a has only 1 failure (filtered out by >=3 threshold)
    assert_eq!(body["failedLogins"].as_array().unwrap().len(), 1);
    assert_eq!(
        body["failedLogins"][0]["userId"].as_str().unwrap(),
        user_b.to_string()
    );
    assert_eq!(
        body["failedLogins"][0]["ipAddress"].as_str().unwrap(),
        "10.0.0.1"
    );
    assert_eq!(body["failedLogins"][0]["attemptCount"].as_i64().unwrap(), 3);
}
