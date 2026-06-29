//! Happy-path integration tests for notification preference granular endpoints
//! (BIT-303 batch 2, ~17 endpoints from Epic 8B / Epic 29).
//!
//! Endpoints covered:
//! - GET  /granular/events
//! - PUT  /granular/events/{event_type}
//! - POST /granular/events/reset
//! - PUT  /granular/events/category/{category}
//! - GET  /granular/schedule
//! - PUT  /granular/schedule
//! - GET  /granular/roles
//! - GET  /granular/roles/{role}
//! - PUT  /granular/roles/{role}
//! - DELETE /granular/roles/{role}
//! - POST /granular/roles/{role}/apply
//! - GET  /granular/groups
//! - GET  /granular/groups/{group_id}
//! - DELETE /granular/groups/{group_id}
//! - POST /granular/groups/{group_id}/read
//! - POST /granular/groups/read-all
//! - GET  /granular/digests

mod common;

use axum::http::StatusCode;
use common::{create_authenticated_user_with_org, TestApp, TestUser};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

const BASE: &str = "/api/v1/users/me/notification-preferences/granular";

// ============================================================================
// Helpers
// ============================================================================

const INSERT_GROUP: &str = r#"INSERT INTO notification_groups
               (user_id, group_key, entity_type, entity_id, title)
           VALUES ($1, $2, 'announcement', gen_random_uuid(), 'Test group')
           RETURNING id"#;

const INSERT_NOTIF: &str = r#"INSERT INTO grouped_notifications
               (group_id, user_id, event_type, title, body)
           VALUES ($1, $2, 'announcement_created', 'Hello', 'Body')
           RETURNING id"#;

const SELECT_USER_ID: &str = "SELECT id FROM users WHERE email = $1";

/// Insert a notification group (parent) plus one grouped notification (child).
/// Returns `(group_id, notification_id)`.
async fn seed_notification_group(pool: &PgPool, user_id: Uuid, _org_id: Uuid) -> (Uuid, Uuid) {
    let group_key = format!("test_{}", Uuid::new_v4());
    let group_id: Uuid = sqlx::query_scalar(INSERT_GROUP)
        .bind(user_id)
        .bind(&group_key)
        .fetch_one(pool)
        .await
        .expect("seed notification group");

    let notif_id: Uuid = sqlx::query_scalar(INSERT_NOTIF)
        .bind(group_id)
        .bind(user_id)
        .fetch_one(pool)
        .await
        .expect("seed grouped notification");

    (group_id, notif_id)
}

// ============================================================================
// G1 — GET /granular/events returns a list with category summaries
// ============================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_event_preferences_returns_ok(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org) = create_authenticated_user_with_org(&app, &user, "g1").await;
    let sess = app.session(token, org);

    let resp = app
        .execute(sess.get(&format!("{BASE}/events")).build())
        .await;

    resp.assert_status(StatusCode::OK);
    resp.assert_json_field("preferences");
    resp.assert_json_field("categories");
}

// ============================================================================
// G2 — PUT /granular/events/{event_type} updates a single event preference
// ============================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_event_preference_returns_ok(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org) = create_authenticated_user_with_org(&app, &user, "g2").await;
    let sess = app.session(token, org);

    let resp = app
        .execute(
            sess.put(&format!("{BASE}/events/fault_created"))
                .json(json!({
                    "pushEnabled": false,
                    "emailEnabled": true,
                    "inAppEnabled": true
                }))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);
    resp.assert_json_field("eventType");
}

// ============================================================================
// G3 — PUT /granular/events/{event_type} with unknown type → 404
// ============================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_event_preference_unknown_type_returns_404(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org) = create_authenticated_user_with_org(&app, &user, "g3").await;
    let sess = app.session(token, org);

    let resp = app
        .execute(
            sess.put(&format!("{BASE}/events/nonexistent_event_xyz"))
                .json(json!({
                    "pushEnabled": false,
                    "emailEnabled": false,
                    "inAppEnabled": true
                }))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::NOT_FOUND);
}

// ============================================================================
// G4 — POST /granular/events/reset resets all event preferences
// ============================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn reset_event_preferences_returns_ok(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org) = create_authenticated_user_with_org(&app, &user, "g4").await;
    let sess = app.session(token, org);

    // First, update one preference so there is something to reset.
    app.execute(
        sess.put(&format!("{BASE}/events/fault_created"))
            .json(json!({
                "pushEnabled": false,
                "emailEnabled": false,
                "inAppEnabled": false
            }))
            .build(),
    )
    .await;

    let resp = app
        .execute(sess.post(&format!("{BASE}/events/reset")).build())
        .await;

    resp.assert_status(StatusCode::OK);
    resp.assert_json_field("preferences");
}

// ============================================================================
// G5 — PUT /granular/events/category/{category} updates a whole category
// ============================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_category_preferences_returns_ok(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org) = create_authenticated_user_with_org(&app, &user, "g5").await;
    let sess = app.session(token, org);

    let resp = app
        .execute(
            sess.put(&format!("{BASE}/events/category/fault"))
                .json(json!({
                    "pushEnabled": true,
                    "emailEnabled": false,
                    "inAppEnabled": true
                }))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);
    resp.assert_json_field("preferences");
}

// ============================================================================
// G6 — PUT /granular/events/category/{category} with invalid category → 400
// ============================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_category_preferences_invalid_category_returns_400(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org) = create_authenticated_user_with_org(&app, &user, "g6").await;
    let sess = app.session(token, org);

    let resp = app
        .execute(
            sess.put(&format!("{BASE}/events/category/notacategory"))
                .json(json!({
                    "pushEnabled": true,
                    "emailEnabled": true,
                    "inAppEnabled": true
                }))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::BAD_REQUEST);
}

// ============================================================================
// G7 — GET /granular/schedule returns schedule (default if none set)
// ============================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_schedule_returns_ok(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org) = create_authenticated_user_with_org(&app, &user, "g7").await;
    let sess = app.session(token, org);

    let resp = app
        .execute(sess.get(&format!("{BASE}/schedule")).build())
        .await;

    resp.assert_status(StatusCode::OK);
    resp.assert_json_field("schedule");
    resp.assert_json_field("isCurrentlyQuiet");
}

// ============================================================================
// G8 — PUT /granular/schedule persists quiet-hours settings
// ============================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_schedule_returns_ok(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org) = create_authenticated_user_with_org(&app, &user, "g8").await;
    let sess = app.session(token, org);

    let resp = app
        .execute(
            sess.put(&format!("{BASE}/schedule"))
                .json(json!({
                    "quietHoursEnabled": true,
                    "quietHoursStart": "22:00",
                    "quietHoursEnd": "08:00",
                    "timezone": "Europe/Bratislava",
                    "weekendQuietHoursEnabled": false,
                    "digestEnabled": false
                }))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);
    resp.assert_json_field("schedule");

    let body = resp.json_value();
    assert_eq!(
        body["schedule"]["quietHoursEnabled"],
        json!(true),
        "G8: quietHoursEnabled must be persisted as true"
    );
}

// ============================================================================
// G9 — GET /granular/roles lists role defaults (empty list initially)
// ============================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_role_defaults_returns_ok(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org) = create_authenticated_user_with_org(&app, &user, "g9").await;
    let sess = app.session(token, org);

    let resp = app
        .execute(sess.get(&format!("{BASE}/roles")).build())
        .await;

    resp.assert_status(StatusCode::OK);
    resp.assert_json_field("roleDefaults");
}

// ============================================================================
// G10 — PUT /granular/roles/{role} upserts defaults (manager only)
// ============================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_role_defaults_as_manager_returns_ok(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org) = create_authenticated_user_with_org(&app, &user, "g10").await;
    let sess = app.session(token, org);

    let resp = app
        .execute(
            sess.put(&format!("{BASE}/roles/manager"))
                .json(json!({ "eventPreferences": {} }))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);
    resp.assert_json_field("role");
}

// ============================================================================
// G11 — GET /granular/roles/{role} retrieves defaults (returns 404 if none)
// ============================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_role_defaults_returns_ok_after_upsert(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org) = create_authenticated_user_with_org(&app, &user, "g11").await;
    let sess = app.session(token, org);

    // Seed a role default so the GET has something to return.
    app.execute(
        sess.put(&format!("{BASE}/roles/manager"))
            .json(json!({ "eventPreferences": {} }))
            .build(),
    )
    .await
    .assert_status(StatusCode::OK);

    let resp = app
        .execute(sess.get(&format!("{BASE}/roles/manager")).build())
        .await;

    resp.assert_status(StatusCode::OK);
    resp.assert_json_field("role");
}

// ============================================================================
// G12 — DELETE /granular/roles/{role} removes defaults
// ============================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn delete_role_defaults_returns_ok(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org) = create_authenticated_user_with_org(&app, &user, "g12").await;
    let sess = app.session(token, org);

    // Upsert first so there is something to delete.
    app.execute(
        sess.put(&format!("{BASE}/roles/manager"))
            .json(json!({ "eventPreferences": {} }))
            .build(),
    )
    .await
    .assert_status(StatusCode::OK);

    let resp = app
        .execute(sess.delete(&format!("{BASE}/roles/manager")).build())
        .await;

    resp.assert_status(StatusCode::NO_CONTENT);
}

// ============================================================================
// G13 — POST /granular/roles/{role}/apply applies defaults to calling user
// ============================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn apply_role_defaults_returns_ok(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org) = create_authenticated_user_with_org(&app, &user, "g13").await;
    let sess = app.session(token, org);

    // Upsert role defaults first.
    app.execute(
        sess.put(&format!("{BASE}/roles/manager"))
            .json(json!({ "eventPreferences": {} }))
            .build(),
    )
    .await
    .assert_status(StatusCode::OK);

    let resp = app
        .execute(sess.post(&format!("{BASE}/roles/manager/apply")).build())
        .await;

    resp.assert_status(StatusCode::OK);
}

// ============================================================================
// G14 — GET /granular/groups returns grouped notifications list
// ============================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_notification_groups_returns_ok(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org) = create_authenticated_user_with_org(&app, &user, "g14").await;

    let user_id: Uuid = sqlx::query_scalar(SELECT_USER_ID)
        .bind(&user.email)
        .fetch_one(&pool)
        .await
        .expect("resolve user id");

    seed_notification_group(&pool, user_id, org).await;

    let sess = app.session(token, org);
    let resp = app
        .execute(sess.get(&format!("{BASE}/groups")).build())
        .await;

    resp.assert_status(StatusCode::OK);
    resp.assert_json_field("groups");
}

// ============================================================================
// G15 — GET /granular/groups/{group_id} returns a single group
// ============================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_notification_group_returns_ok(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org) = create_authenticated_user_with_org(&app, &user, "g15").await;

    let user_id: Uuid = sqlx::query_scalar(SELECT_USER_ID)
        .bind(&user.email)
        .fetch_one(&pool)
        .await
        .expect("resolve user id");

    let (group_id, _) = seed_notification_group(&pool, user_id, org).await;

    let sess = app.session(token, org);
    let resp = app
        .execute(sess.get(&format!("{BASE}/groups/{group_id}")).build())
        .await;

    resp.assert_status(StatusCode::OK);
    resp.assert_json_field("group");
}

// ============================================================================
// G16 — DELETE /granular/groups/{group_id} removes a group
// ============================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn delete_notification_group_returns_ok(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org) = create_authenticated_user_with_org(&app, &user, "g16").await;

    let user_id: Uuid = sqlx::query_scalar(SELECT_USER_ID)
        .bind(&user.email)
        .fetch_one(&pool)
        .await
        .expect("resolve user id");

    let (group_id, _) = seed_notification_group(&pool, user_id, org).await;

    let sess = app.session(token, org);
    let resp = app
        .execute(sess.delete(&format!("{BASE}/groups/{group_id}")).build())
        .await;

    resp.assert_status(StatusCode::NO_CONTENT);
}

// ============================================================================
// G17 — POST /granular/groups/{group_id}/read marks a group read
// ============================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn mark_group_read_returns_ok(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org) = create_authenticated_user_with_org(&app, &user, "g17").await;

    let user_id: Uuid = sqlx::query_scalar(SELECT_USER_ID)
        .bind(&user.email)
        .fetch_one(&pool)
        .await
        .expect("resolve user id");

    let (group_id, _) = seed_notification_group(&pool, user_id, org).await;

    let sess = app.session(token, org);
    let resp = app
        .execute(sess.post(&format!("{BASE}/groups/{group_id}/read")).build())
        .await;

    resp.assert_status(StatusCode::NO_CONTENT);
}

// ============================================================================
// G18 — POST /granular/groups/read-all marks all groups read
// ============================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn mark_all_groups_read_returns_ok(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org) = create_authenticated_user_with_org(&app, &user, "g18").await;

    let user_id: Uuid = sqlx::query_scalar(SELECT_USER_ID)
        .bind(&user.email)
        .fetch_one(&pool)
        .await
        .expect("resolve user id");

    seed_notification_group(&pool, user_id, org).await;
    seed_notification_group(&pool, user_id, org).await;

    let sess = app.session(token, org);
    let resp = app
        .execute(sess.post(&format!("{BASE}/groups/read-all")).build())
        .await;

    resp.assert_status(StatusCode::OK);
}

// ============================================================================
// G19 — GET /granular/digests returns digest list
// ============================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_digests_returns_ok(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org) = create_authenticated_user_with_org(&app, &user, "g19").await;
    let sess = app.session(token, org);

    let resp = app
        .execute(sess.get(&format!("{BASE}/digests")).build())
        .await;

    // handler returns Vec<NotificationDigest> directly (a JSON array)
    resp.assert_status(StatusCode::OK);
}
