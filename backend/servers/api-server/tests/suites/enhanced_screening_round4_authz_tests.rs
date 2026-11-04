//! Round-4 regressions (audit4-operations) for `/api/v1/tenant-screening`.
//!
//! * Every screening route is manager-only (credit / background / eviction
//!   PII, provider credentials, queue, reports): a `resident` member gets 403.
//! * By-id mutations are org-scoped in SQL (`delete_provider_config`,
//!   `update_provider_status`, `update_queue_item_status`, `delete_risk_model`)
//!   and `get_pending_queue_items` only returns the caller's org rows — the app
//!   connects as a role that bypasses RLS, so the explicit predicate matters.
//! * Create payloads referencing another org's `screening_id` /
//!   `provider_config_id` are rejected with 404.

#![allow(dead_code)]

use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{seed_membership, seed_org, TestApp, TestConfig};

async fn member(pool: &PgPool, org: Uuid, role: &str) -> String {
    use api_server::services::JwtService;
    let email = format!("ets-r4-{}@test.internal", Uuid::new_v4());
    let id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO users (email, password_hash, name, status, email_verified_at)
           VALUES ($1, 'test_hash', 'Screening R4', 'active', NOW()) RETURNING id"#,
    )
    .bind(&email)
    .fetch_one(pool)
    .await
    .unwrap();
    seed_membership(pool, org, id, role).await;
    let config = TestConfig::default();
    JwtService::new(&config.jwt_secret)
        .unwrap()
        .generate_access_token(id, &email, "Screening R4", None, None)
        .unwrap()
}

async fn seed_screening(pool: &PgPool, org_id: Uuid) -> Uuid {
    let building_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO buildings (organization_id, street, city, postal_code, country)
           VALUES ($1, 'Screening St 1', 'Bratislava', '81101', 'Slovakia') RETURNING id"#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .unwrap();
    let unit_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO units (
               building_id, designation, floor, unit_type,
               size_sqm, rooms, ownership_share, occupancy_status, status, settings
           )
           VALUES ($1, '1A', 1, 'apartment', 50.0, 2, 100.00, 'unknown', 'active', '{}'::jsonb)
           RETURNING id"#,
    )
    .bind(building_id)
    .fetch_one(pool)
    .await
    .unwrap();
    let application_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO tenant_applications (
               organization_id, unit_id, applicant_name, applicant_email, status
           )
           VALUES ($1, $2, 'Applicant', $3, 'draft') RETURNING id"#,
    )
    .bind(org_id)
    .bind(unit_id)
    .bind(format!("{}@applicant.test", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .unwrap();
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO tenant_screenings (application_id, organization_id, screening_type, status)
           VALUES ($1, $2, 'credit_check', 'completed') RETURNING id"#,
    )
    .bind(application_id)
    .bind(org_id)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn seed_provider(pool: &PgPool, org_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO screening_provider_configs (organization_id, provider_name, provider_type)
           VALUES ($1, $2, 'credit_bureau') RETURNING id"#,
    )
    .bind(org_id)
    .bind(format!("Bureau {}", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn seed_queue_item(pool: &PgPool, org_id: Uuid, screening_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO screening_request_queue (screening_id, organization_id, check_type)
           VALUES ($1, $2, 'credit_bureau') RETURNING id"#,
    )
    .bind(screening_id)
    .bind(org_id)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn count(pool: &PgPool, sql: &'static str, id: Uuid) -> i64 {
    sqlx::query_scalar(sql)
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn resident_is_denied_every_screening_surface(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "etsr").await;
    let token = member(&pool, org, "resident").await;
    let screening = seed_screening(&pool, org).await;
    let provider = seed_provider(&pool, org).await;
    let tenant = org.to_string();

    let cases = vec![
        app.get(&format!("/api/v1/tenant-screening/credit/{screening}")),
        app.get(&format!("/api/v1/tenant-screening/results/{screening}")),
        app.get("/api/v1/tenant-screening/queue"),
        app.post("/api/v1/tenant-screening/queue").json(json!({
            "screening_id": screening,
            "check_type": "CreditBureau",
        })),
        app.delete(&format!("/api/v1/tenant-screening/providers/{provider}")),
        app.get(&format!("/api/v1/tenant-screening/reports/{screening}")),
    ];
    for rb in cases {
        let req = rb.bearer(&token).header("X-Tenant-ID", &tenant).build();
        let uri = req.uri().to_string();
        let resp = app.execute(req).await;
        assert_eq!(
            resp.status,
            StatusCode::FORBIDDEN,
            "{uri} must be manager-only: {}",
            resp.text()
        );
    }
    assert_eq!(
        count(
            &pool,
            "SELECT COUNT(*) FROM screening_provider_configs WHERE id = $1",
            provider
        )
        .await,
        1
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn cross_org_manager_cannot_touch_foreign_screening_rows(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "etsa").await;
    let org_b = seed_org(&pool, "etsb").await;
    let token_b = member(&pool, org_b, "manager").await;
    let screening_a = seed_screening(&pool, org_a).await;
    let provider_a = seed_provider(&pool, org_a).await;
    let queue_a = seed_queue_item(&pool, org_a, screening_a).await;
    let tenant_b = org_b.to_string();

    // Delete foreign provider config → 404, row survives.
    let resp = app
        .execute(
            app.delete(&format!("/api/v1/tenant-screening/providers/{provider_a}"))
                .bearer(&token_b)
                .header("X-Tenant-ID", &tenant_b)
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::NOT_FOUND, "{}", resp.text());
    assert_eq!(
        count(
            &pool,
            "SELECT COUNT(*) FROM screening_provider_configs WHERE id = $1",
            provider_a
        )
        .await,
        1
    );

    // Update foreign provider status → 404.
    let resp = app
        .execute(
            app.put(&format!(
                "/api/v1/tenant-screening/providers/{provider_a}/status"
            ))
            .json(json!({ "status": "Active" }))
            .bearer(&token_b)
            .header("X-Tenant-ID", &tenant_b)
            .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::NOT_FOUND, "{}", resp.text());

    // Update foreign queue item status → 404, status unchanged.
    let resp = app
        .execute(
            app.put(&format!("/api/v1/tenant-screening/queue/{queue_a}/status"))
                .json(json!({ "status": "completed" }))
                .bearer(&token_b)
                .header("X-Tenant-ID", &tenant_b)
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::NOT_FOUND, "{}", resp.text());
    let status: String =
        sqlx::query_scalar("SELECT status FROM screening_request_queue WHERE id = $1")
            .bind(queue_a)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(status, "pending");

    // Pending queue listing does not include org A's item.
    let resp = app
        .execute(
            app.get("/api/v1/tenant-screening/queue")
                .bearer(&token_b)
                .header("X-Tenant-ID", &tenant_b)
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "{}", resp.text());
    assert!(
        !resp.text().contains(&queue_a.to_string()),
        "org B must not see org A's queue items"
    );

    // Create a queue item pointing at org A's screening → 404, nothing inserted.
    let resp = app
        .execute(
            app.post("/api/v1/tenant-screening/queue")
                .json(json!({ "screening_id": screening_a, "check_type": "CreditBureau" }))
                .bearer(&token_b)
                .header("X-Tenant-ID", &tenant_b)
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::NOT_FOUND, "{}", resp.text());
    assert_eq!(
        count(
            &pool,
            "SELECT COUNT(*) FROM screening_request_queue WHERE screening_id = $1",
            screening_a
        )
        .await,
        1
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn owner_org_manager_manages_own_screening_rows(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "etso").await;
    let token = member(&pool, org, "manager").await;
    let screening = seed_screening(&pool, org).await;
    let provider = seed_provider(&pool, org).await;
    let queue = seed_queue_item(&pool, org, screening).await;
    let tenant = org.to_string();

    let resp = app
        .execute(
            app.put(&format!("/api/v1/tenant-screening/queue/{queue}/status"))
                .json(json!({ "status": "completed" }))
                .bearer(&token)
                .header("X-Tenant-ID", &tenant)
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "{}", resp.text());

    let resp = app
        .execute(
            app.post("/api/v1/tenant-screening/queue")
                .json(json!({ "screening_id": screening, "check_type": "CreditBureau" }))
                .bearer(&token)
                .header("X-Tenant-ID", &tenant)
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::CREATED, "{}", resp.text());

    let resp = app
        .execute(
            app.delete(&format!("/api/v1/tenant-screening/providers/{provider}"))
                .bearer(&token)
                .header("X-Tenant-ID", &tenant)
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::NO_CONTENT, "{}", resp.text());
}
