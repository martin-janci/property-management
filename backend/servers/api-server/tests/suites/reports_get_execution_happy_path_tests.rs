//! Happy-path integration test for `GET /api/v1/reports/executions/{id}`
//! (`reports::get_execution`) — BIT-420 (BIT-417 bucket B).
//!
//! The existing coverage (`report_schedule_sibling_scope_tests.rs`) only asserts
//! the cross-tenant 404 (IDOR) path for this handler; there was no positive 200
//! test for a same-org caller resolving a real execution. This adds it.
//!
//! Auth model: the handler takes a `RlsConnection` (resolved from the
//! `X-Tenant-ID` header + an active `organization_members` row); after resolving
//! the caller's org it scopes the lookup via `get_execution_scoped(id, org)`.
//! A same-org member therefore receives a 200 with the execution payload.
//! Seeding mirrors `report_execution_download_retry_e2e_tests.rs` (schedule +
//! completed execution).

#![allow(dead_code)]

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{create_authenticated_user, seed_membership, seed_org, TestApp, TestUser};

async fn seed_schedule(pool: &PgPool, org_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO report_schedules
            (report_id, organization_id, name, frequency, time, timezone, format, recipients, is_active, status)
        VALUES
            (gen_random_uuid(), $1, 'BIT420 Schedule', 'weekly', '09:00', 'UTC', 'pdf',
             '["owner@example.com"]', true, 'active')
        RETURNING id
        "#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed report schedule")
}

async fn seed_completed_execution(pool: &PgPool, schedule_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO report_executions
            (schedule_id, status, started_at, completed_at, file_key, file_name, file_size, created_at)
        VALUES
            ($1, 'completed', NOW(), NOW(), 'reports/2026/bit420.pdf', 'bit420.pdf', 2048, NOW())
        RETURNING id
        "#,
    )
    .bind(schedule_id)
    .fetch_one(pool)
    .await
    .expect("seed completed execution")
}

async fn user_id_for(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(email)
        .fetch_one(pool)
        .await
        .expect("fetch user id by email")
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_execution_same_org_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org = seed_org(&pool, "getexec").await;
    let schedule = seed_schedule(&pool, org).await;
    let exec = seed_completed_execution(&pool, schedule).await;

    // Same-org authenticated member.
    let user = TestUser::new();
    let (token, _) = create_authenticated_user(&app, &user).await;
    let user_id = user_id_for(&pool, &user.email).await;
    // get_execution has no role gate — any active membership satisfies the
    // RlsConnection extractor; `org_admin` is the canonical seed role.
    seed_membership(&pool, org, user_id, "org_admin").await;

    let req = Request::builder()
        .method(Method::GET)
        .uri(format!("/api/v1/reports/executions/{exec}"))
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header("X-Tenant-ID", org.to_string())
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::empty())
        .expect("build get_execution request");

    let resp = app.execute(req).await;
    resp.assert_status(StatusCode::OK);

    let body = resp.json_value();
    let returned_id = body
        .get("id")
        .and_then(|v| v.as_str())
        .expect("execution payload must carry an id");
    assert_eq!(
        returned_id,
        exec.to_string(),
        "get_execution must return the seeded execution, body={body}"
    );
}
