//! Round-4 regressions (audit4-operations) for `/api/v1/marketplace`.
//!
//! * `get_manager_dashboard` verifies the caller manages the query org
//!   (non-member 404, resident 403);
//! * `list_reviews` only shows non-approved reviews to managers of the
//!   filtered org;
//! * RFQ create / delete are manager-only;
//! * `submit_quote` requires a live invitation (404 otherwise);
//! * platform-admin moderation routes resolve platform status from
//!   `users.principal_kind`, so a real platform principal with a plain login
//!   token gets through.

#![allow(dead_code)]

use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{seed_membership, seed_org, TestApp, TestConfig};

async fn user_of_kind(pool: &PgPool, kind: &str) -> (String, Uuid) {
    use api_server::services::JwtService;
    let email = format!("mkt-r4-{}@test.internal", Uuid::new_v4());
    let id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
           VALUES ($1, 'test_hash', 'Mkt R4', 'active', NOW(), $2) RETURNING id"#,
    )
    .bind(&email)
    .bind(kind)
    .fetch_one(pool)
    .await
    .unwrap();
    let config = TestConfig::default();
    let token = JwtService::new(&config.jwt_secret)
        .unwrap()
        .generate_access_token(id, &email, "Mkt R4", None, None)
        .unwrap();
    (token, id)
}

async fn user(pool: &PgPool) -> (String, Uuid) {
    user_of_kind(pool, "staff").await
}

async fn seed_provider(pool: &PgPool, user_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO service_provider_profiles
               (user_id, company_name, contact_name, contact_email, status)
           VALUES ($1, 'R4 Provider', 'Contact', $2, 'active') RETURNING id"#,
    )
    .bind(user_id)
    .bind(format!("{}@mkt-r4.test", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn seed_rfq(pool: &PgPool, org_id: Uuid, created_by: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO rfqs (organization_id, created_by, title, description, service_category, status)
           VALUES ($1, $2, 'Roof', 'Fix roof', 'roofing', 'sent') RETURNING id"#,
    )
    .bind(org_id)
    .bind(created_by)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn seed_review(
    pool: &PgPool,
    provider: Uuid,
    org: Uuid,
    reviewer: Uuid,
    status: &str,
) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO provider_reviews
               (provider_id, reviewer_id, organization_id, quality_rating, timeliness_rating,
                communication_rating, value_rating, overall_rating, review_title, status)
           VALUES ($1, $2, $3, 1, 1, 1, 1, 1, 'R4 review', $4) RETURNING id"#,
    )
    .bind(provider)
    .bind(reviewer)
    .bind(org)
    .bind(status)
    .fetch_one(pool)
    .await
    .unwrap()
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn manager_dashboard_requires_manager_of_query_org(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "mkr4a").await;
    let org_b = seed_org(&pool, "mkr4b").await;
    let (token_mgr, mgr) = user(&pool).await;
    seed_membership(&pool, org_a, mgr, "manager").await;
    let (token_res, res) = user(&pool).await;
    seed_membership(&pool, org_a, res, "resident").await;
    let (token_out, out) = user(&pool).await;
    seed_membership(&pool, org_b, out, "manager").await;
    let uri = format!("/api/v1/marketplace/dashboard?organization_id={org_a}");

    let resp = app.execute(app.get(&uri).bearer(&token_out).build()).await;
    assert_eq!(resp.status, StatusCode::NOT_FOUND, "{}", resp.text());
    let resp = app.execute(app.get(&uri).bearer(&token_res).build()).await;
    assert_eq!(resp.status, StatusCode::FORBIDDEN, "{}", resp.text());
    let resp = app.execute(app.get(&uri).bearer(&token_mgr).build()).await;
    assert_eq!(resp.status, StatusCode::OK, "{}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_reviews_hides_unmoderated_reviews_from_non_managers(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "mkr4r").await;
    let org_b = seed_org(&pool, "mkr4s").await;
    let (token_mgr, mgr) = user(&pool).await;
    seed_membership(&pool, org_a, mgr, "manager").await;
    let (token_out, out) = user(&pool).await;
    seed_membership(&pool, org_b, out, "manager").await;
    let (_tp, prov_user) = user(&pool).await;
    let provider = seed_provider(&pool, prov_user).await;
    let pending = seed_review(&pool, provider, org_a, mgr, "pending").await;

    for uri in [
        "/api/v1/marketplace/reviews?status=pending".to_string(),
        format!("/api/v1/marketplace/reviews?organization_id={org_a}&status=pending"),
    ] {
        let resp = app.execute(app.get(&uri).bearer(&token_out).build()).await;
        assert_eq!(resp.status, StatusCode::OK, "{}", resp.text());
        assert!(
            !resp.text().contains(&pending.to_string()),
            "{uri}: a non-manager must not see a pending review"
        );
    }

    let resp = app
        .execute(
            app.get(&format!(
                "/api/v1/marketplace/reviews?organization_id={org_a}&status=pending"
            ))
            .bearer(&token_mgr)
            .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "{}", resp.text());
    assert!(resp.text().contains(&pending.to_string()));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn rfq_mutations_are_manager_only(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "mkr4q").await;
    let (_tm, mgr) = user(&pool).await;
    seed_membership(&pool, org, mgr, "manager").await;
    let (token_res, res) = user(&pool).await;
    seed_membership(&pool, org, res, "resident").await;
    let rfq = seed_rfq(&pool, org, mgr).await;

    let resp = app
        .execute(
            app.post("/api/v1/marketplace/rfqs")
                .json(json!({
                    "organization_id": org,
                    "title": "Injected",
                    "description": "x",
                    "service_category": "roofing",
                    "provider_ids": [],
                }))
                .bearer(&token_res)
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::FORBIDDEN, "{}", resp.text());

    let resp = app
        .execute(
            app.delete(&format!("/api/v1/marketplace/rfqs/{rfq}"))
                .bearer(&token_res)
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::FORBIDDEN, "{}", resp.text());
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM rfqs WHERE organization_id = $1")
        .bind(org)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(n, 1);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn submit_quote_requires_invitation(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "mkr4i").await;
    let (_tm, mgr) = user(&pool).await;
    seed_membership(&pool, org, mgr, "manager").await;
    let rfq = seed_rfq(&pool, org, mgr).await;
    let (token_p, prov_user) = user(&pool).await;
    let provider = seed_provider(&pool, prov_user).await;

    let submit = || {
        app.post("/api/v1/marketplace/quotes")
            .json(json!({ "rfq_id": rfq, "price": "999.00" }))
            .bearer(&token_p)
            .build()
    };
    let resp = app.execute(submit()).await;
    assert_eq!(resp.status, StatusCode::NOT_FOUND, "{}", resp.text());

    sqlx::query("INSERT INTO rfq_invitations (rfq_id, provider_id) VALUES ($1, $2)")
        .bind(rfq)
        .bind(provider)
        .execute(&pool)
        .await
        .unwrap();
    let resp = app.execute(submit()).await;
    assert_eq!(resp.status, StatusCode::CREATED, "{}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn platform_principal_can_list_verifications_with_login_token(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (token_admin, _) = user_of_kind(&pool, "platform").await;
    let (token_staff, _) = user(&pool).await;

    let resp = app
        .execute(
            app.get("/api/v1/marketplace/verifications")
                .bearer(&token_admin)
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "{}", resp.text());

    let resp = app
        .execute(
            app.get("/api/v1/marketplace/verifications")
                .bearer(&token_staff)
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::FORBIDDEN, "{}", resp.text());
}
