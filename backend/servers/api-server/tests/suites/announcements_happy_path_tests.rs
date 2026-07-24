//! BIT-268 wave-4 batch 4: announcements + critical-notifications happy-path tests.
//!
//! Covers partial endpoints from docs/endpoint-checklist/groups/notifications-comms.md:
//!
//! Announcements:
//! - POST   /api/v1/announcements
//! - GET    /api/v1/announcements
//! - GET    /api/v1/announcements/published
//! - GET    /api/v1/announcements/{id}
//! - PUT    /api/v1/announcements/{id}
//! - DELETE /api/v1/announcements/{id}
//! - POST   /api/v1/announcements/{id}/publish
//! - POST   /api/v1/announcements/{id}/schedule
//! - POST   /api/v1/announcements/{id}/archive
//! - POST   /api/v1/announcements/{id}/pin
//! - PATCH  /api/v1/announcements/{id}/pin
//! - GET    /api/v1/announcements/{id}/attachments
//! - POST   /api/v1/announcements/{id}/attachments
//! - DELETE /api/v1/announcements/{id}/attachments/{attachment_id}
//! - POST   /api/v1/announcements/{id}/read
//! - POST   /api/v1/announcements/{id}/acknowledge
//! - GET    /api/v1/announcements/{id}/acknowledgments
//! - GET    /api/v1/announcements/statistics
//! - GET    /api/v1/announcements/unread-count
//!
//! Critical notifications:
//! - POST   /api/v1/organizations/{org_id}/critical-notifications
//! - GET    /api/v1/organizations/{org_id}/critical-notifications
//! - GET    /api/v1/organizations/{org_id}/critical-notifications/unacknowledged
//! - POST   /api/v1/organizations/{org_id}/critical-notifications/{notification_id}/acknowledge
//! - GET    /api/v1/organizations/{org_id}/critical-notifications/{notification_id}/stats

#![allow(dead_code)]

use axum::http::StatusCode;
use chrono::Utc;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{create_manager_with_org, TestApp, TestUser};

// ---------------------------------------------------------------------------
// Seed helpers
// ---------------------------------------------------------------------------

async fn user_id_for(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(email)
        .fetch_one(pool)
        .await
        .expect("fetch user id")
}

/// Seed an active, manager-tier `user_memberships` row.
///
/// The critical-notification handlers authorize through `RequestPrincipal`,
/// whose `is_active` / `is_manager_in_org` checks read `user_memberships`
/// (not the `organization_members` table that `seed_membership` writes). The
/// role is stored PascalCase (`OrgAdmin`) so it matches `is_manager_in_org`'s
/// manager-tier set (`SuperAdmin`, `PlatformAdmin`, `OrgAdmin`, …).
async fn seed_user_membership(pool: &PgPool, user_id: Uuid, org_id: Uuid) {
    sqlx::query(
        r#"INSERT INTO user_memberships (user_id, organization_id, role)
           VALUES ($1, $2, 'OrgAdmin')
           ON CONFLICT DO NOTHING"#,
    )
    .bind(user_id)
    .bind(org_id)
    .execute(pool)
    .await
    .expect("seed user_membership");
}

/// Inject the `ResolvedTenant` request extension that `host_tenant_middleware`
/// would normally add. The bare `TestApp` router (`create_router`) does not
/// install that middleware, so `RequestPrincipal` handlers otherwise fail with
/// 403 "tenant not resolved". Mirrors `community_happy_path_tests`.
fn inject_tenant(
    mut req: axum::http::Request<axum::body::Body>,
    org_id: Uuid,
) -> axum::http::Request<axum::body::Body> {
    use api_core::middleware::host_tenant::{ResolvedTenant, TenantSource};
    req.extensions_mut().insert(ResolvedTenant {
        organization_id: org_id,
        source: TenantSource::Subdomain,
    });
    req
}

/// Seed a draft announcement and return its id.
async fn seed_draft_announcement(pool: &PgPool, org_id: Uuid, author_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO announcements (organization_id, author_id, title, content, target_type, status)
        VALUES ($1, $2, 'Test Announcement', 'Content here.', 'all'::announcement_target_type, 'draft'::announcement_status)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(author_id)
    .fetch_one(pool)
    .await
    .expect("seed draft announcement")
}

/// Seed a published announcement and return its id.
async fn seed_published_announcement(pool: &PgPool, org_id: Uuid, author_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO announcements (organization_id, author_id, title, content, target_type, status, published_at)
        VALUES ($1, $2, 'Published Announcement', 'Published content.', 'all'::announcement_target_type, 'published'::announcement_status, NOW())
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(author_id)
    .fetch_one(pool)
    .await
    .expect("seed published announcement")
}

/// Seed an attachment on an announcement and return its id.
async fn seed_attachment(pool: &PgPool, announcement_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO announcement_attachments (announcement_id, file_key, file_name, file_type, file_size)
        VALUES ($1, 'attach/test.pdf', 'test.pdf', 'application/pdf', 1024)
        RETURNING id
        "#,
    )
    .bind(announcement_id)
    .fetch_one(pool)
    .await
    .expect("seed attachment")
}

/// Seed a critical notification and return its id.
async fn seed_critical_notification(pool: &PgPool, org_id: Uuid, created_by: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO critical_notifications (organization_id, title, message, created_by)
        VALUES ($1, 'Critical Alert', 'This is critical.', $2)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(created_by)
    .fetch_one(pool)
    .await
    .expect("seed critical notification")
}

// ---------------------------------------------------------------------------
// POST /api/v1/announcements
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_announcement_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_manager_with_org(&app, &user, "ann-create").await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .post("/api/v1/announcements")
                .json(json!({
                    "title": "Hello Residents",
                    "content": "Welcome to Q3 updates.",
                    "target_type": "all"
                }))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::CREATED, "body: {}", resp.text());
    let body = resp.json_value();
    assert!(body.get("id").is_some(), "response must include id");
}

// ---------------------------------------------------------------------------
// GET /api/v1/announcements
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_announcements_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_manager_with_org(&app, &user, "ann-list").await;
    let user_id = user_id_for(&pool, &user.email).await;
    let _ = seed_draft_announcement(&pool, org_id, user_id).await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .get("/api/v1/announcements")
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    let body = resp.json_value();
    assert!(
        body.get("announcements").is_some(),
        "response must include announcements"
    );
}

// ---------------------------------------------------------------------------
// GET /api/v1/announcements/published
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_published_announcements_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_manager_with_org(&app, &user, "ann-pub-list").await;
    let user_id = user_id_for(&pool, &user.email).await;
    let _ = seed_published_announcement(&pool, org_id, user_id).await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .get("/api/v1/announcements/published")
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    let body = resp.json_value();
    assert!(
        body.get("announcements").is_some(),
        "response must include announcements"
    );
}

// ---------------------------------------------------------------------------
// GET /api/v1/announcements/{id}
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_announcement_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_manager_with_org(&app, &user, "ann-get").await;
    let user_id = user_id_for(&pool, &user.email).await;
    let ann_id = seed_draft_announcement(&pool, org_id, user_id).await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .get(&format!("/api/v1/announcements/{ann_id}"))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    let body = resp.json_value();
    assert_eq!(
        body["announcement"]["id"].as_str().unwrap_or(""),
        ann_id.to_string()
    );
}

// ---------------------------------------------------------------------------
// PUT /api/v1/announcements/{id}
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_announcement_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_manager_with_org(&app, &user, "ann-update").await;
    let user_id = user_id_for(&pool, &user.email).await;
    let ann_id = seed_draft_announcement(&pool, org_id, user_id).await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .put(&format!("/api/v1/announcements/{ann_id}"))
                .json(json!({ "title": "Updated Title" }))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
}

// ---------------------------------------------------------------------------
// POST /api/v1/announcements/{id}/publish
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn publish_announcement_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_manager_with_org(&app, &user, "ann-publish").await;
    let user_id = user_id_for(&pool, &user.email).await;
    let ann_id = seed_draft_announcement(&pool, org_id, user_id).await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .post(&format!("/api/v1/announcements/{ann_id}/publish"))
                .json(json!({}))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
}

// ---------------------------------------------------------------------------
// POST /api/v1/announcements/{id}/schedule
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn schedule_announcement_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_manager_with_org(&app, &user, "ann-sched").await;
    let user_id = user_id_for(&pool, &user.email).await;
    let ann_id = seed_draft_announcement(&pool, org_id, user_id).await;

    let scheduled_at = (Utc::now() + chrono::Duration::days(3)).to_rfc3339();

    let resp = app
        .execute(
            app.session(token, org_id)
                .post(&format!("/api/v1/announcements/{ann_id}/schedule"))
                .json(json!({ "scheduled_at": scheduled_at }))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
}

// ---------------------------------------------------------------------------
// POST /api/v1/announcements/{id}/archive
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn archive_announcement_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_manager_with_org(&app, &user, "ann-archive").await;
    let user_id = user_id_for(&pool, &user.email).await;
    let ann_id = seed_published_announcement(&pool, org_id, user_id).await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .post(&format!("/api/v1/announcements/{ann_id}/archive"))
                .json(json!({}))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
}

// ---------------------------------------------------------------------------
// POST /api/v1/announcements/{id}/pin
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn pin_announcement_post_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_manager_with_org(&app, &user, "ann-pin-post").await;
    let user_id = user_id_for(&pool, &user.email).await;
    let ann_id = seed_published_announcement(&pool, org_id, user_id).await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .post(&format!("/api/v1/announcements/{ann_id}/pin"))
                .json(json!({ "pinned": true }))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
}

// ---------------------------------------------------------------------------
// PATCH /api/v1/announcements/{id}/pin
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn pin_announcement_patch_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_manager_with_org(&app, &user, "ann-pin-patch").await;
    let user_id = user_id_for(&pool, &user.email).await;
    let ann_id = seed_published_announcement(&pool, org_id, user_id).await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .patch(&format!("/api/v1/announcements/{ann_id}/pin"))
                .json(json!({ "pinned": false }))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
}

// ---------------------------------------------------------------------------
// GET /api/v1/announcements/{id}/attachments
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_attachments_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_manager_with_org(&app, &user, "ann-att-list").await;
    let user_id = user_id_for(&pool, &user.email).await;
    let ann_id = seed_published_announcement(&pool, org_id, user_id).await;
    let _ = seed_attachment(&pool, ann_id).await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .get(&format!("/api/v1/announcements/{ann_id}/attachments"))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
}

// ---------------------------------------------------------------------------
// POST /api/v1/announcements/{id}/attachments
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn add_attachment_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_manager_with_org(&app, &user, "ann-att-add").await;
    let user_id = user_id_for(&pool, &user.email).await;
    let ann_id = seed_draft_announcement(&pool, org_id, user_id).await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .post(&format!("/api/v1/announcements/{ann_id}/attachments"))
                .json(json!({
                    "file_key": "org/files/doc.pdf",
                    "file_name": "doc.pdf",
                    "file_type": "application/pdf",
                    "file_size": 2048
                }))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::CREATED, "body: {}", resp.text());
}

// ---------------------------------------------------------------------------
// DELETE /api/v1/announcements/{id}/attachments/{attachment_id}
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn delete_attachment_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_manager_with_org(&app, &user, "ann-att-del").await;
    let user_id = user_id_for(&pool, &user.email).await;
    let ann_id = seed_draft_announcement(&pool, org_id, user_id).await;
    let att_id = seed_attachment(&pool, ann_id).await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .delete(&format!(
                    "/api/v1/announcements/{ann_id}/attachments/{att_id}"
                ))
                .build(),
        )
        .await;

    assert!(
        resp.status == StatusCode::OK || resp.status == StatusCode::NO_CONTENT,
        "expected 200 or 204, got {}: {}",
        resp.status,
        resp.text()
    );
}

// ---------------------------------------------------------------------------
// POST /api/v1/announcements/{id}/read
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn mark_read_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_manager_with_org(&app, &user, "ann-read").await;
    let user_id = user_id_for(&pool, &user.email).await;
    let ann_id = seed_published_announcement(&pool, org_id, user_id).await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .post(&format!("/api/v1/announcements/{ann_id}/read"))
                .json(json!({}))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
}

// ---------------------------------------------------------------------------
// POST /api/v1/announcements/{id}/acknowledge
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn acknowledge_announcement_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_manager_with_org(&app, &user, "ann-ack").await;
    let user_id = user_id_for(&pool, &user.email).await;
    let ann_id = seed_published_announcement(&pool, org_id, user_id).await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .post(&format!("/api/v1/announcements/{ann_id}/acknowledge"))
                .json(json!({}))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
}

// ---------------------------------------------------------------------------
// GET /api/v1/announcements/{id}/acknowledgments
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_acknowledgments_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_manager_with_org(&app, &user, "ann-ackl").await;
    let user_id = user_id_for(&pool, &user.email).await;
    let ann_id = seed_published_announcement(&pool, org_id, user_id).await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .get(&format!("/api/v1/announcements/{ann_id}/acknowledgments"))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
}

// ---------------------------------------------------------------------------
// GET /api/v1/announcements/statistics
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_statistics_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_manager_with_org(&app, &user, "ann-stats").await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .get("/api/v1/announcements/statistics")
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
}

// ---------------------------------------------------------------------------
// GET /api/v1/announcements/unread-count
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_unread_count_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_manager_with_org(&app, &user, "ann-unread").await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .get("/api/v1/announcements/unread-count")
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
}

// ---------------------------------------------------------------------------
// DELETE /api/v1/announcements/{id}
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn delete_announcement_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_manager_with_org(&app, &user, "ann-delete").await;
    let user_id = user_id_for(&pool, &user.email).await;
    let ann_id = seed_draft_announcement(&pool, org_id, user_id).await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .delete(&format!("/api/v1/announcements/{ann_id}"))
                .build(),
        )
        .await;

    assert!(
        resp.status == StatusCode::OK || resp.status == StatusCode::NO_CONTENT,
        "expected 200 or 204, got {}: {}",
        resp.status,
        resp.text()
    );
}

// ---------------------------------------------------------------------------
// POST /api/v1/organizations/{org_id}/critical-notifications
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_critical_notification_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_manager_with_org(&app, &user, "crit-create").await;
    let user_id = user_id_for(&pool, &user.email).await;
    seed_user_membership(&pool, user_id, org_id).await;

    let resp = app
        .execute(inject_tenant(
            app.session(token, org_id)
                .post(&format!(
                    "/api/v1/organizations/{org_id}/critical-notifications"
                ))
                .json(json!({
                    "title": "Urgent: Water Shutoff",
                    "message": "Water will be shut off tomorrow at 8am."
                }))
                .build(),
            org_id,
        ))
        .await;

    assert_eq!(resp.status, StatusCode::CREATED, "body: {}", resp.text());
    let body = resp.json_value();
    assert!(body.get("id").is_some(), "response must include id");
}

// ---------------------------------------------------------------------------
// GET /api/v1/organizations/{org_id}/critical-notifications
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_critical_notifications_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_manager_with_org(&app, &user, "crit-list").await;
    let user_id = user_id_for(&pool, &user.email).await;
    seed_user_membership(&pool, user_id, org_id).await;
    let _ = seed_critical_notification(&pool, org_id, user_id).await;

    let resp = app
        .execute(inject_tenant(
            app.session(token, org_id)
                .get(&format!(
                    "/api/v1/organizations/{org_id}/critical-notifications"
                ))
                .build(),
            org_id,
        ))
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
}

// ---------------------------------------------------------------------------
// GET /api/v1/organizations/{org_id}/critical-notifications/unacknowledged
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_unacknowledged_critical_notifications_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_manager_with_org(&app, &user, "crit-unack").await;
    let user_id = user_id_for(&pool, &user.email).await;
    seed_user_membership(&pool, user_id, org_id).await;
    let _ = seed_critical_notification(&pool, org_id, user_id).await;

    let resp = app
        .execute(inject_tenant(
            app.session(token, org_id)
                .get(&format!(
                    "/api/v1/organizations/{org_id}/critical-notifications/unacknowledged"
                ))
                .build(),
            org_id,
        ))
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
}

// ---------------------------------------------------------------------------
// POST /api/v1/organizations/{org_id}/critical-notifications/{id}/acknowledge
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn acknowledge_critical_notification_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_manager_with_org(&app, &user, "crit-ack").await;
    let user_id = user_id_for(&pool, &user.email).await;
    seed_user_membership(&pool, user_id, org_id).await;
    let notif_id = seed_critical_notification(&pool, org_id, user_id).await;

    let resp = app
        .execute(inject_tenant(
            app.session(token, org_id)
                .post(&format!(
                    "/api/v1/organizations/{org_id}/critical-notifications/{notif_id}/acknowledge"
                ))
                .json(json!({}))
                .build(),
            org_id,
        ))
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
}

// ---------------------------------------------------------------------------
// GET /api/v1/organizations/{org_id}/critical-notifications/{id}/stats
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_critical_notification_stats_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_manager_with_org(&app, &user, "crit-stats").await;
    let user_id = user_id_for(&pool, &user.email).await;
    seed_user_membership(&pool, user_id, org_id).await;
    let notif_id = seed_critical_notification(&pool, org_id, user_id).await;

    let resp = app
        .execute(inject_tenant(
            app.session(token, org_id)
                .get(&format!(
                    "/api/v1/organizations/{org_id}/critical-notifications/{notif_id}/stats"
                ))
                .build(),
            org_id,
        ))
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
}
