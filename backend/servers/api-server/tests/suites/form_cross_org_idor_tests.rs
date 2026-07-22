//! Regression tests for form submit/download org isolation (#1369).
//!
//! The forms routes now run under `RlsConnection`, so the caller's effective
//! org comes from the authenticated membership-backed `X-Tenant-ID` context.
//! These tests prove the write/download endpoints honor that scope:
//! same-org submit still works, while cross-org submit/download attempts
//! return 404 and leave the target org's rows untouched.

#![allow(dead_code)]

use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{create_authenticated_user_with_org, TestApp, TestUser};

async fn user_id_for(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(email)
        .fetch_one(pool)
        .await
        .expect("fetch user id")
}

async fn seed_published_form(pool: &PgPool, org_id: Uuid, created_by: Uuid, title: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO forms (
            organization_id, title, status, target_type, target_ids,
            require_signatures, allow_multiple_submissions, created_by
        )
        VALUES ($1, $2, 'published', 'all', '[]'::jsonb, false, true, $3)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(title)
    .bind(created_by)
    .fetch_one(pool)
    .await
    .expect("seed published form")
}

async fn submission_count(pool: &PgPool, form_id: Uuid) -> i64 {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM form_submissions WHERE form_id = $1")
        .bind(form_id)
        .fetch_one(pool)
        .await
        .expect("count submissions")
}

async fn download_count(pool: &PgPool, form_id: Uuid) -> i64 {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM form_downloads WHERE form_id = $1")
        .bind(form_id)
        .fetch_one(pool)
        .await
        .expect("count downloads")
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn submit_form_same_org_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "forms-submit-ok").await;
    let user_id = user_id_for(&pool, &user.email).await;
    let form_id = seed_published_form(&pool, org_id, user_id, "Resident intake").await;

    let response = app
        .execute(
            app.session(token, org_id)
                .post(&format!("/api/v1/forms/{form_id}/submit"))
                .json(json!({ "data": { "q1": "answer" } }))
                .build(),
        )
        .await;

    assert_eq!(
        response.status,
        StatusCode::CREATED,
        "same-org submit must succeed"
    );
    assert_eq!(
        submission_count(&pool, form_id).await,
        1,
        "same-org submit must insert exactly one submission"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn submit_form_cross_org_is_not_found_and_does_not_insert(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let owner = TestUser::new();
    let (_owner_token, org_a) =
        create_authenticated_user_with_org(&app, &owner, "forms-submit-owner").await;
    let owner_id = user_id_for(&pool, &owner.email).await;
    let form_id = seed_published_form(&pool, org_a, owner_id, "Org A only").await;

    let attacker = TestUser::new();
    let (attacker_token, org_b) =
        create_authenticated_user_with_org(&app, &attacker, "forms-submit-attacker").await;

    let response = app
        .execute(
            app.session(attacker_token, org_b)
                .post(&format!("/api/v1/forms/{form_id}/submit"))
                .json(json!({ "data": { "q1": "answer" } }))
                .build(),
        )
        .await;

    assert_eq!(
        response.status,
        StatusCode::NOT_FOUND,
        "cross-org submit must be hidden behind org scope, got {} body={}",
        response.status,
        response.text()
    );
    assert_eq!(
        submission_count(&pool, form_id).await,
        0,
        "cross-org submit must not insert a submission"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn download_form_cross_org_is_not_found_and_does_not_insert(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let owner = TestUser::new();
    let (_owner_token, org_a) =
        create_authenticated_user_with_org(&app, &owner, "forms-download-owner").await;
    let owner_id = user_id_for(&pool, &owner.email).await;
    let form_id = seed_published_form(&pool, org_a, owner_id, "Downloadable form").await;

    let attacker = TestUser::new();
    let (attacker_token, org_b) =
        create_authenticated_user_with_org(&app, &attacker, "forms-download-attacker").await;

    let response = app
        .execute(
            app.session(attacker_token, org_b)
                .post(&format!("/api/v1/forms/{form_id}/download"))
                .build(),
        )
        .await;

    assert_eq!(
        response.status,
        StatusCode::NOT_FOUND,
        "cross-org download must be hidden behind org scope, got {} body={}",
        response.status,
        response.text()
    );
    assert_eq!(
        download_count(&pool, form_id).await,
        0,
        "cross-org download must not insert a download row"
    );
}
