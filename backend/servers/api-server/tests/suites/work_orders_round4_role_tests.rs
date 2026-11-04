//! Round-4 regressions (audit4-operations) for `/api/v1/work-orders`.
//!
//! Work-order and schedule mutations are manager-only (role from the
//! `organization_members` row); start / complete / hold also admit the
//! assignee. A plain `resident` member gets 403 and nothing changes.

#![allow(dead_code)]

use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{seed_membership, seed_org, TestApp, TestConfig};

async fn member(pool: &PgPool, org: Uuid, role: &str) -> (String, Uuid) {
    use api_server::services::JwtService;
    let email = format!("wo-r4-{}@test.internal", Uuid::new_v4());
    let id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO users (email, password_hash, name, status, email_verified_at)
           VALUES ($1, 'test_hash', 'WO R4', 'active', NOW()) RETURNING id"#,
    )
    .bind(&email)
    .fetch_one(pool)
    .await
    .unwrap();
    seed_membership(pool, org, id, role).await;
    let config = TestConfig::default();
    let token = JwtService::new(&config.jwt_secret)
        .unwrap()
        .generate_access_token(id, &email, "WO R4", None, None)
        .unwrap();
    (token, id)
}

async fn seed_building(pool: &PgPool, org_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO buildings (organization_id, street, city, postal_code, country)
           VALUES ($1, 'WO St 1', 'Bratislava', '81101', 'Slovakia') RETURNING id"#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn seed_work_order(
    pool: &PgPool,
    org_id: Uuid,
    building_id: Uuid,
    created_by: Uuid,
    assigned_to: Option<Uuid>,
) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO work_orders
               (organization_id, building_id, title, description, created_by, assigned_to)
           VALUES ($1, $2, 'Original title', 'Original description', $3, $4)
           RETURNING id"#,
    )
    .bind(org_id)
    .bind(building_id)
    .bind(created_by)
    .bind(assigned_to)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn wo_count(pool: &PgPool, id: Uuid) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM work_orders WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn resident_cannot_mutate_work_orders(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "wor4r").await;
    let (_tm, mgr) = member(&pool, org, "manager").await;
    let (token, resident) = member(&pool, org, "resident").await;
    let building = seed_building(&pool, org).await;
    let wo = seed_work_order(&pool, org, building, mgr, None).await;
    let tenant = org.to_string();

    let cases = vec![
        app.post("/api/v1/work-orders").json(json!({
            "organization_id": org,
            "building_id": building,
            "title": "Injected",
            "description": "x",
        })),
        app.patch(&format!("/api/v1/work-orders/{wo}"))
            .json(json!({ "title": "pwned" })),
        app.delete(&format!("/api/v1/work-orders/{wo}")),
        app.post(&format!("/api/v1/work-orders/{wo}/assign"))
            .json(json!({ "assigned_to": resident })),
        app.post(&format!("/api/v1/work-orders/{wo}/start")),
        app.post(&format!(
            "/api/v1/work-orders/schedules/process-due?organization_id={org}"
        )),
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
    assert_eq!(wo_count(&pool, wo).await, 1);
    let (title, assigned): (String, Option<Uuid>) =
        sqlx::query_as("SELECT title, assigned_to FROM work_orders WHERE id = $1")
            .bind(wo)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(title, "Original title");
    assert_eq!(assigned, None);
    let created: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM work_orders WHERE organization_id = $1 AND title = 'Injected'",
    )
    .bind(org)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(created, 0);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn assignee_can_start_assigned_work_order(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "wor4a").await;
    let (_tm, mgr) = member(&pool, org, "manager").await;
    let (token, tech) = member(&pool, org, "resident").await;
    let building = seed_building(&pool, org).await;
    let wo = seed_work_order(&pool, org, building, mgr, Some(tech)).await;

    let resp = app
        .execute(
            app.post(&format!("/api/v1/work-orders/{wo}/start"))
                .bearer(&token)
                .header("X-Tenant-ID", &org.to_string())
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "{}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn manager_can_delete_own_org_work_order_but_not_foreign(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "wor4x").await;
    let org_b = seed_org(&pool, "wor4y").await;
    let (token_a, mgr_a) = member(&pool, org_a, "manager").await;
    let (token_b, _) = member(&pool, org_b, "manager").await;
    let building = seed_building(&pool, org_a).await;
    let wo = seed_work_order(&pool, org_a, building, mgr_a, None).await;

    let resp = app
        .execute(
            app.delete(&format!("/api/v1/work-orders/{wo}"))
                .bearer(&token_b)
                .header("X-Tenant-ID", &org_b.to_string())
                .build(),
        )
        .await;
    assert!(
        resp.status == StatusCode::NOT_FOUND || resp.status == StatusCode::FORBIDDEN,
        "cross-org delete must be rejected: {} {}",
        resp.status,
        resp.text()
    );
    assert_eq!(wo_count(&pool, wo).await, 1);

    let resp = app
        .execute(
            app.delete(&format!("/api/v1/work-orders/{wo}"))
                .bearer(&token_a)
                .header("X-Tenant-ID", &org_a.to_string())
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::NO_CONTENT, "{}", resp.text());
}
