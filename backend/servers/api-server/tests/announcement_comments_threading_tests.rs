//! Integration tests for announcement comment threading (Epic 6, Story 6.3).
//!
//! Coverage 6-3: exercises the comment endpoints end-to-end through the HTTP
//! router (`TestApp`), asserting the full create / list / nested-reply / delete
//! flow plus:
//!
//!   * **Threading depth** — replies are limited to one level (a reply to a
//!     reply is rejected with 400).
//!   * **Auth** — every comment endpoint mounts `AuthUser`; an unauthenticated
//!     request is rejected with 401 at the extractor before the handler runs.
//!
//! These tests are additive and DB-backed (`#[sqlx::test]`). The parent
//! announcement is seeded directly via SQL (mirroring the prerequisite-row
//! seeding in `faults_cross_org_idor_tests.rs`) because the HTTP create+publish
//! flow is a separate, heavier surface (manager RBAC + publish lifecycle) that
//! is orthogonal to the comment-threading behaviour under test here.
//!
//! **RLS tenant isolation is asserted separately**, at the repository/DB layer,
//! in `crates/db/tests/announcement_comments_rls_tests.rs`. It cannot be
//! demonstrated through this `#[sqlx::test]` HTTP harness: `#[sqlx::test]`
//! connects as the `postgres` *superuser*, and Postgres exempts superusers from
//! RLS entirely (see the "Create non-superuser test role" step in
//! `.github/workflows/backend.yml` and the `TEST_DATABASE_URL` /
//! `rls_test_runner` harness in `crates/db/tests/rls_penetration_tests.rs`).
//! Under a superuser connection the comment routes' `RlsConnection` GUC is a
//! no-op, so a cross-tenant request would (mis)leadingly succeed here even
//! though the policy blocks it in production. The DB-layer test connects as the
//! non-superuser role so the `announcement_comments` policy (migration 00016)
//! actually runs.

#![allow(dead_code)]

#[allow(dead_code)]
mod common;

use axum::http::StatusCode;
use common::{create_authenticated_user_with_org, TestApp, TestUser};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

/// Resolve a user's id from their email (seeded by `create_authenticated_user`).
async fn user_id_for(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(email)
        .fetch_one(pool)
        .await
        .expect("resolve user id")
}

/// Seed a *published*, comments-enabled announcement owned by `org_id` /
/// `author_id` and return its id.
///
/// Inserted directly (not via the HTTP create+publish path) so the test stays
/// focused on the comment surface. `status = 'published'` satisfies
/// `Announcement::is_published()`, which `create_comment` requires; the RLS
/// policy on `announcements` keys off `organization_id`.
async fn seed_published_announcement(pool: &PgPool, org_id: Uuid, author_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO announcements (
            organization_id, author_id, title, content,
            target_type, status, published_at, comments_enabled
        )
        VALUES (
            $1, $2, 'Threaded discussion', 'Please discuss below.',
            'all'::announcement_target_type, 'published'::announcement_status,
            NOW(), TRUE
        )
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(author_id)
    .fetch_one(pool)
    .await
    .expect("seed published announcement")
}

// ---------------------------------------------------------------------------
// (1) Happy path: create top-level comment, nested reply, list (threaded),
//     then delete.
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn comment_threading_create_reply_list_delete(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let user = TestUser::new();
    let (token, org) = create_authenticated_user_with_org(&app, &user, "thread-a").await;
    let author_id = user_id_for(&app.pool, &user.email).await;
    let ann = seed_published_announcement(&app.pool, org, author_id).await;

    // --- create a top-level comment ---
    let create_resp = app
        .execute(
            app.post(&format!("/api/v1/announcements/{ann}/comments"))
                .bearer(&token)
                .header("X-Tenant-ID", &org.to_string())
                .json(json!({ "content": "Top-level comment" }))
                .build(),
        )
        .await;
    create_resp.assert_status(StatusCode::CREATED);
    let parent: Value = create_resp.json_value();
    let parent_id = parent["id"].as_str().expect("comment id").to_string();
    assert!(
        parent["parent_id"].is_null(),
        "top-level comment has no parent"
    );

    // --- create a nested reply (one level deep) ---
    let reply_resp = app
        .execute(
            app.post(&format!("/api/v1/announcements/{ann}/comments"))
                .bearer(&token)
                .header("X-Tenant-ID", &org.to_string())
                .json(json!({ "content": "A reply", "parent_id": parent_id }))
                .build(),
        )
        .await;
    reply_resp.assert_status(StatusCode::CREATED);
    let reply: Value = reply_resp.json_value();
    let reply_id = reply["id"].as_str().expect("reply id").to_string();
    assert_eq!(
        reply["parent_id"].as_str(),
        Some(parent_id.as_str()),
        "reply must point at the parent comment"
    );

    // --- list comments: one top-level entry carrying the reply as a child ---
    let list_resp = app
        .execute(
            app.get(&format!("/api/v1/announcements/{ann}/comments"))
                .bearer(&token)
                .header("X-Tenant-ID", &org.to_string())
                .build(),
        )
        .await;
    list_resp.assert_status(StatusCode::OK);
    let list: Value = list_resp.json_value();
    assert_eq!(
        list["count"].as_u64(),
        Some(1),
        "exactly one top-level comment expected, got {list}"
    );
    let top = &list["comments"][0];
    assert_eq!(top["id"].as_str(), Some(parent_id.as_str()));
    let replies = top["replies"].as_array().expect("replies array present");
    assert_eq!(
        replies.len(),
        1,
        "the reply must be nested under its parent"
    );
    assert_eq!(replies[0]["id"].as_str(), Some(reply_id.as_str()));

    // --- delete the reply (author deleting own comment) ---
    let del_resp = app
        .execute(
            app.delete(&format!("/api/v1/announcements/{ann}/comments/{reply_id}"))
                .bearer(&token)
                .header("X-Tenant-ID", &org.to_string())
                .build(),
        )
        .await;
    del_resp.assert_status(StatusCode::OK);

    // --- deleting the same comment again is rejected (already deleted) ---
    let del_again = app
        .execute(
            app.delete(&format!("/api/v1/announcements/{ann}/comments/{reply_id}"))
                .bearer(&token)
                .header("X-Tenant-ID", &org.to_string())
                .build(),
        )
        .await;
    del_again.assert_status(StatusCode::BAD_REQUEST);
}

// ---------------------------------------------------------------------------
// (2) Nesting depth: replying to a reply is rejected (max depth = 2).
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn reply_to_a_reply_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let user = TestUser::new();
    let (token, org) = create_authenticated_user_with_org(&app, &user, "depth").await;
    let author_id = user_id_for(&app.pool, &user.email).await;
    let ann = seed_published_announcement(&app.pool, org, author_id).await;

    let parent = app
        .execute(
            app.post(&format!("/api/v1/announcements/{ann}/comments"))
                .bearer(&token)
                .header("X-Tenant-ID", &org.to_string())
                .json(json!({ "content": "root" }))
                .build(),
        )
        .await;
    parent.assert_status(StatusCode::CREATED);
    let parent_id = parent.json_value()["id"]
        .as_str()
        .expect("parent id")
        .to_string();

    let reply = app
        .execute(
            app.post(&format!("/api/v1/announcements/{ann}/comments"))
                .bearer(&token)
                .header("X-Tenant-ID", &org.to_string())
                .json(json!({ "content": "reply", "parent_id": parent_id }))
                .build(),
        )
        .await;
    reply.assert_status(StatusCode::CREATED);
    let reply_id = reply.json_value()["id"]
        .as_str()
        .expect("reply id")
        .to_string();

    // Replying to the reply must be rejected — only one level of nesting.
    let nested = app
        .execute(
            app.post(&format!("/api/v1/announcements/{ann}/comments"))
                .bearer(&token)
                .header("X-Tenant-ID", &org.to_string())
                .json(json!({ "content": "too deep", "parent_id": reply_id }))
                .build(),
        )
        .await;
    nested.assert_status(StatusCode::BAD_REQUEST);
}

// ---------------------------------------------------------------------------
// (3) Auth: every comment endpoint requires authentication (401 with no token).
//     (Cross-tenant RLS isolation is asserted in the DB-layer companion file —
//      see the module doc comment for why it cannot live in this harness.)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn comment_endpoints_require_auth(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let ann = Uuid::new_v4();
    let comment = Uuid::new_v4();

    // List (GET)
    let list = app
        .execute(
            app.get(&format!("/api/v1/announcements/{ann}/comments"))
                .build(),
        )
        .await;
    list.assert_status(StatusCode::UNAUTHORIZED);

    // Create (POST)
    let create = app
        .execute(
            app.post(&format!("/api/v1/announcements/{ann}/comments"))
                .json(json!({ "content": "hello" }))
                .build(),
        )
        .await;
    create.assert_status(StatusCode::UNAUTHORIZED);

    // Delete (DELETE)
    let delete = app
        .execute(
            app.delete(&format!("/api/v1/announcements/{ann}/comments/{comment}"))
                .build(),
        )
        .await;
    delete.assert_status(StatusCode::UNAUTHORIZED);
}
