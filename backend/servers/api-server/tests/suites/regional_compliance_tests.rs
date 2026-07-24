//! Integration tests for Regional Compliance controls (Epic 72).

#![allow(dead_code)]

use axum::http::{Method, StatusCode};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{seed_membership, RequestBuilder, TestApp, TestConfig};
use db::models::regional_compliance::*;

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
        email: "regional-test@compliance.test".to_string(),
        name: "Regional Test User".to_string(),
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
    .bind(format!("Regional Org {slug}"))
    .bind(format!("regional-{slug}"))
    .bind(format!("{slug}@regional.test"))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'test_hash', 'Regional User', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

async fn seed_building(pool: &PgPool, org_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO buildings (organization_id, street, city, postal_code, country)
        VALUES ($1, 'Hlavna 1', 'Bratislava', '81101', 'Slovakia')
        RETURNING id
        "#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed building")
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_jurisdiction_lifecycle(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let user = seed_user(&pool, "manager@compliance.test").await;
    let org_id = seed_org(&pool, "comp-org").await;
    seed_membership(&pool, org_id, user, "manager").await;
    let token = access_token(user, Some(org_id));

    // 1. Get initial jurisdiction (should default to slovakia)
    let req = app
        .get("/api/v1/regional-compliance/jurisdiction")
        .bearer(&token)
        .header("X-Tenant-ID", &org_id.to_string())
        .build();
    let resp = app.execute(req).await;
    resp.assert_status(StatusCode::OK);
    let body: Jurisdiction = serde_json::from_value(resp.json_value()).unwrap();
    assert_eq!(body, Jurisdiction::Slovakia);

    // 2. Set jurisdiction to czechia (PUT)
    let set_payload = serde_json::json!({
        "jurisdiction": "czechia"
    });

    let req = RequestBuilder::new(Method::PUT, "/api/v1/regional-compliance/jurisdiction")
        .bearer(&token)
        .header("X-Tenant-ID", &org_id.to_string())
        .json(set_payload)
        .build();
    let resp = app.execute(req).await;
    resp.assert_status(StatusCode::OK);
    let body: Jurisdiction = serde_json::from_value(resp.json_value()).unwrap();
    assert_eq!(body, Jurisdiction::Czechia);

    // 3. Get updated jurisdiction
    let req = app
        .get("/api/v1/regional-compliance/jurisdiction")
        .bearer(&token)
        .header("X-Tenant-ID", &org_id.to_string())
        .build();
    let resp = app.execute(req).await;
    resp.assert_status(StatusCode::OK);
    let body: Jurisdiction = serde_json::from_value(resp.json_value()).unwrap();
    assert_eq!(body, Jurisdiction::Czechia);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_slovak_voting_config_lifecycle(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let user = seed_user(&pool, "manager@compliance.test").await;
    let org_id = seed_org(&pool, "voting-org").await;
    let building_id = seed_building(&pool, org_id).await;
    seed_membership(&pool, org_id, user, "manager").await;
    let token = access_token(user, Some(org_id));

    // 1. Get initial voting config (should 404)
    let req = app
        .get(&format!(
            "/api/v1/regional-compliance/slovak/voting/config/{}",
            building_id
        ))
        .bearer(&token)
        .header("X-Tenant-ID", &org_id.to_string())
        .build();
    let resp = app.execute(req).await;
    resp.assert_status(StatusCode::NOT_FOUND);

    // 2. Configure Slovak voting (POST)
    let config_payload = serde_json::json!({
        "building_id": building_id,
        "enabled": true,
        "default_decision_type": "simple_majority",
        "use_ownership_weight": true,
        "min_notice_days": 15,
        "allow_proxy_voting": true
    });

    let req = RequestBuilder::new(
        Method::POST,
        "/api/v1/regional-compliance/slovak/voting/config",
    )
    .bearer(&token)
    .header("X-Tenant-ID", &org_id.to_string())
    .json(config_payload)
    .build();
    let resp = app.execute(req).await;
    resp.assert_status(StatusCode::OK);
    let body: SlovakVotingConfig = serde_json::from_value(resp.json_value()).unwrap();
    assert_eq!(body.building_id, building_id);
    assert!(body.enabled);
    assert_eq!(body.min_notice_days, 15);

    // 3. Get updated voting config
    let req = app
        .get(&format!(
            "/api/v1/regional-compliance/slovak/voting/config/{}",
            building_id
        ))
        .bearer(&token)
        .header("X-Tenant-ID", &org_id.to_string())
        .build();
    let resp = app.execute(req).await;
    resp.assert_status(StatusCode::OK);
    let body: SlovakVotingConfig = serde_json::from_value(resp.json_value()).unwrap();
    assert_eq!(body.min_notice_days, 15);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_gdpr_consent_lifecycle(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let user = seed_user(&pool, "gdpr-user@compliance.test").await;
    let org_id = seed_org(&pool, "gdpr-org").await;
    seed_membership(&pool, org_id, user, "manager").await;
    let token = access_token(user, Some(org_id));

    // 1. Get initial status (should have default essential consent granted because it is required)
    let req = app
        .get("/api/v1/regional-compliance/slovak/gdpr/consent/status")
        .bearer(&token)
        .header("X-Tenant-ID", &org_id.to_string())
        .build();
    let resp = app.execute(req).await;
    resp.assert_status(StatusCode::OK);
    let body: GdprConsentStatus = serde_json::from_value(resp.json_value()).unwrap();
    let essential = body
        .consents
        .iter()
        .find(|c| c.category == GdprConsentCategory::Essential)
        .unwrap();
    assert!(essential.granted);

    // 2. Grant Marketing consent
    let record_payload = serde_json::json!({
        "category": "marketing",
        "granted": true,
        "consent_version": "1.2"
    });
    let req = RequestBuilder::new(
        Method::POST,
        "/api/v1/regional-compliance/slovak/gdpr/consent",
    )
    .bearer(&token)
    .header("X-Tenant-ID", &org_id.to_string())
    .json(record_payload)
    .build();
    let resp = app.execute(req).await;
    resp.assert_status(StatusCode::OK);

    // 3. Verify in status
    let req = app
        .get("/api/v1/regional-compliance/slovak/gdpr/consent/status")
        .bearer(&token)
        .header("X-Tenant-ID", &org_id.to_string())
        .build();
    let resp = app.execute(req).await;
    resp.assert_status(StatusCode::OK);
    let body: GdprConsentStatus = serde_json::from_value(resp.json_value()).unwrap();
    let marketing = body
        .consents
        .iter()
        .find(|c| c.category == GdprConsentCategory::Marketing)
        .unwrap();
    assert!(marketing.granted);
    assert_eq!(marketing.consent_version.as_deref(), Some("1.2"));

    // 4. Withdraw Marketing consent
    let withdraw_payload = serde_json::json!({
        "category": "marketing",
        "granted": false,
        "consent_version": "1.2"
    });
    let req = RequestBuilder::new(
        Method::POST,
        "/api/v1/regional-compliance/slovak/gdpr/consent/withdraw",
    )
    .bearer(&token)
    .header("X-Tenant-ID", &org_id.to_string())
    .json(withdraw_payload)
    .build();
    let resp = app.execute(req).await;
    resp.assert_status(StatusCode::OK);

    // 5. Verify withdrawn in status
    let req = app
        .get("/api/v1/regional-compliance/slovak/gdpr/consent/status")
        .bearer(&token)
        .header("X-Tenant-ID", &org_id.to_string())
        .build();
    let resp = app.execute(req).await;
    resp.assert_status(StatusCode::OK);
    let body: GdprConsentStatus = serde_json::from_value(resp.json_value()).unwrap();
    let marketing = body
        .consents
        .iter()
        .find(|c| c.category == GdprConsentCategory::Marketing)
        .unwrap();
    assert!(!marketing.granted);
}

/// #1906 finding-5: compliance-config WRITES are manager-gated. A non-manager
/// member of the org must be refused (403), while a manager succeeds — proving
/// the gate derives from the trusted DB role, not the (here manager-claiming)
/// token, and is not a no-context pass.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn configure_writes_require_manager(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "rolegate-org").await;
    let building_id = seed_building(&pool, org_id).await;

    let config_payload = serde_json::json!({
        "building_id": building_id,
        "enabled": true,
        "default_decision_type": "simple_majority",
        "use_ownership_weight": true,
        "min_notice_days": 15,
        "allow_proxy_voting": true
    });

    // Non-manager (resident) member → 403, even though the token claims manager.
    let resident = seed_user(&pool, "resident@compliance.test").await;
    seed_membership(&pool, org_id, resident, "resident").await;
    let resident_token = access_token(resident, Some(org_id));
    let req = RequestBuilder::new(
        Method::POST,
        "/api/v1/regional-compliance/slovak/voting/config",
    )
    .bearer(&resident_token)
    .header("X-Tenant-ID", &org_id.to_string())
    .json(config_payload.clone())
    .build();
    let resp = app.execute(req).await;
    resp.assert_status(StatusCode::FORBIDDEN);

    // Manager member → allowed (2xx), proving the gate isn't a blanket deny.
    let manager = seed_user(&pool, "manager@compliance.test").await;
    seed_membership(&pool, org_id, manager, "manager").await;
    let manager_token = access_token(manager, Some(org_id));
    let req = RequestBuilder::new(
        Method::POST,
        "/api/v1/regional-compliance/slovak/voting/config",
    )
    .bearer(&manager_token)
    .header("X-Tenant-ID", &org_id.to_string())
    .json(config_payload)
    .build();
    let resp = app.execute(req).await;
    resp.assert_status(StatusCode::OK);
}
