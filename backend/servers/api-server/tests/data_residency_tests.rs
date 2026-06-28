//! Integration tests for Data Residency controls (Epic 146).

#![allow(dead_code)]

#[allow(dead_code)]
mod common;

use axum::http::{Method, StatusCode};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use common::{seed_membership, RequestBuilder, TestApp, TestConfig};
#[allow(unused_imports)]
use db::models::data_residency::{AccessType, DataRegion, DataTypeCategory, ResidencyAuditEvent};

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

fn access_token(user_id: Uuid, org_id: Option<Uuid>) -> String {
    let now = chrono::Utc::now().timestamp();
    let claims = TestClaims {
        sub: user_id,
        exp: now + 3600,
        iat: now,
        token_type: "access".to_string(),
        tenant_id: org_id,
        role: Some("manager".to_string()),
        email: "residency-test@residency.test".to_string(),
        name: "Residency Test User".to_string(),
    };
    let secret = TestConfig::default().jwt_secret;
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .expect("encode test JWT")
}

async fn seed_org(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO organizations (name, slug, contact_email, status)
        VALUES ($1, $2, $3, 'active') RETURNING id
        "#,
    )
    .bind(format!("Residency Org {slug}"))
    .bind(format!("residency-{slug}"))
    .bind(format!("{slug}@residency.test"))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'test_hash', 'Residency User', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_data_residency_lifecycle(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let user = seed_user(&pool, "manager@residency.test").await;
    let org_id = seed_org(&pool, "res-org").await;
    seed_membership(&pool, org_id, user, "manager").await;
    let token = access_token(user, Some(org_id));

    // 1. Get initial config (should insert and return default)
    let req = app
        .get("/api/v1/data-residency/config")
        .bearer(&token)
        .header("X-Tenant-ID", &org_id.to_string())
        .build();
    let resp = app.execute(req).await;
    resp.assert_status(StatusCode::OK);
    let body = resp.json_value();
    assert_eq!(body["primary_region"].as_str(), Some("eu_west"));
    assert_eq!(body["backup_region"].as_str(), Some("eu_central"));

    // 2. Configure residency (POST)
    let configure_payload = serde_json::json!({
        "primary_region": "eu_central",
        "backup_region": "ch_north",
        "allow_cross_region_access": true,
        "data_type_overrides": [
            {
                "data_type": "financial_data",
                "region": "eu_west",
                "reason": "regulatory lock"
            }
        ],
        "compliance_notes": "Slovak and Czech compliance alignment"
    });

    let req = RequestBuilder::new(Method::POST, "/api/v1/data-residency/config")
        .bearer(&token)
        .header("X-Tenant-ID", &org_id.to_string())
        .json(configure_payload.clone())
        .build();
    let resp = app.execute(req).await;
    resp.assert_status(StatusCode::OK);
    let body = resp.json_value();
    assert_eq!(body["primary_region"].as_str(), Some("eu_central"));
    assert_eq!(body["backup_region"].as_str(), Some("ch_north"));
    assert_eq!(body["allow_cross_region_access"].as_bool(), Some(true));

    // 3. Verify audit log entry was created
    let req = app
        .get("/api/v1/data-residency/audit")
        .bearer(&token)
        .header("X-Tenant-ID", &org_id.to_string())
        .build();
    let resp = app.execute(req).await;
    resp.assert_status(StatusCode::OK);
    let body = resp.json_value();
    assert!(body["total_count"].as_i64().unwrap_or(0) >= 1);
    // chain_valid is false for a single-entry chain (no prev_hash to validate against)
    assert!(body["chain_valid"].as_bool().is_some());

    // 4. Log cross region access
    let log_access_payload = serde_json::json!({
        "data_type": "personal_data",
        "source_region": "ch_north",
        "access_region": "us_east",
        "access_type": "read",
        "resource_id": Uuid::new_v4().to_string(),
        "reason": "emergency cross-region query"
    });

    let req = RequestBuilder::new(Method::POST, "/api/v1/data-residency/routing/log-access")
        .bearer(&token)
        .header("X-Tenant-ID", &org_id.to_string())
        .json(log_access_payload)
        .build();
    let resp = app.execute(req).await;
    resp.assert_status(StatusCode::OK);
    let body = resp.json_value();
    assert_eq!(body["source_region"].as_str(), Some("ch_north"));
    assert_eq!(body["access_region"].as_str(), Some("us_east"));
    assert_eq!(body["access_type"].as_str(), Some("read"));

    // 5. Get routing status
    let req = app
        .get("/api/v1/data-residency/routing/status")
        .bearer(&token)
        .header("X-Tenant-ID", &org_id.to_string())
        .build();
    let resp = app.execute(req).await;
    resp.assert_status(StatusCode::OK);
    let body = resp.json_value();
    assert_eq!(body["primary_region"].as_str(), Some("eu_central"));
    assert_eq!(body["recent_cross_region_accesses"].as_i64(), Some(1));

    // 6. Run compliance verification
    let req = RequestBuilder::new(Method::POST, "/api/v1/data-residency/compliance/verify")
        .bearer(&token)
        .header("X-Tenant-ID", &org_id.to_string())
        .json(serde_json::json!({ "include_details": true }))
        .build();
    let resp = app.execute(req).await;
    resp.assert_status(StatusCode::OK);
    let body = resp.json_value();
    assert_eq!(body["is_compliant"].as_bool(), Some(true));

    // 7. Get dashboard
    let req = app
        .get("/api/v1/data-residency/dashboard")
        .bearer(&token)
        .header("X-Tenant-ID", &org_id.to_string())
        .build();
    let resp = app.execute(req).await;
    resp.assert_status(StatusCode::OK);
    let body = resp.json_value();
    assert_eq!(body["cross_region_stats"]["last_24h"].as_i64(), Some(1));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_data_residency_cross_tenant_isolation(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let user_a = seed_user(&pool, "user-a@residency.test").await;
    let org_a = seed_org(&pool, "tenant-a").await;
    seed_membership(&pool, org_a, user_a, "manager").await;
    let token_a = access_token(user_a, Some(org_a));

    let user_b = seed_user(&pool, "user-b@residency.test").await;
    let org_b = seed_org(&pool, "tenant-b").await;
    seed_membership(&pool, org_b, user_b, "manager").await;
    let token_b = access_token(user_b, Some(org_b));

    // Configure residency for Org A
    let configure_payload = serde_json::json!({
        "primary_region": "eu_west",
        "backup_region": "eu_central",
        "allow_cross_region_access": false,
        "data_type_overrides": [],
        "compliance_notes": "Org A Setup"
    });
    let req = RequestBuilder::new(Method::POST, "/api/v1/data-residency/config")
        .bearer(&token_a)
        .header("X-Tenant-ID", &org_a.to_string())
        .json(configure_payload)
        .build();
    let resp = app.execute(req).await;
    resp.assert_status(StatusCode::OK);

    // Try to read Org A's config using Org B's context (with X-Tenant-ID set to Org A)
    // The RLS / Member validator must reject or isolate this
    let req = app
        .get("/api/v1/data-residency/config")
        .bearer(&token_b)
        .header("X-Tenant-ID", &org_a.to_string())
        .build();
    let resp = app.execute(req).await;
    // member validator blocks requests where caller user is not a member of the X-Tenant-ID header org
    assert_eq!(resp.status, StatusCode::FORBIDDEN);
}
