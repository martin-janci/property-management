//! BIT-309 (BIT-303 batch 3): news/articles happy-path integration tests.
//!
//! Covers all partial endpoints from
//! docs/endpoint-checklist/groups/notifications-comms.md:
//!
//! - GET    /api/v1/news                                  list_articles
//! - POST   /api/v1/news                                  create_article
//! - GET    /api/v1/news/{id}                             get_article
//! - PUT    /api/v1/news/{id}                             update_article
//! - DELETE /api/v1/news/{id}                             delete_article
//! - POST   /api/v1/news/{id}/publish                     publish_article
//! - POST   /api/v1/news/{id}/archive                     archive_article
//! - POST   /api/v1/news/{id}/restore                     restore_article
//! - POST   /api/v1/news/{id}/pin                         pin_article
//! - GET    /api/v1/news/{id}/media                       list_media
//! - POST   /api/v1/news/{id}/media                       add_media
//! - DELETE /api/v1/news/{id}/media/{media_id}            delete_media
//! - POST   /api/v1/news/{id}/reactions                   toggle_reaction
//! - GET    /api/v1/news/{id}/reactions/counts            get_reaction_counts
//! - GET    /api/v1/news/{id}/comments                    list_comments
//! - POST   /api/v1/news/{id}/comments                    create_comment
//! - PUT    /api/v1/news/{id}/comments/{comment_id}       update_comment
//! - DELETE /api/v1/news/{id}/comments/{comment_id}       delete_comment
//! - POST   /api/v1/news/{id}/comments/{comment_id}/moderate  moderate_comment
//! - GET    /api/v1/news/{id}/comments/{comment_id}/replies   list_comment_replies
//! - POST   /api/v1/news/{id}/view                        record_view
//! - GET    /api/v1/news/statistics                       get_statistics

#![allow(dead_code)]

use crate::common::{create_manager_with_org, TestApp, TestUser};
use axum::http::StatusCode;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

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

/// Seed a draft article directly via SQL, returning its id.
async fn seed_article(pool: &PgPool, org_id: Uuid, author_id: Uuid, title: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO news_articles \
         (organization_id, author_id, title, content, status) \
         VALUES ($1, $2, $3, 'Test article content.', 'draft') \
         RETURNING id",
    )
    .bind(org_id)
    .bind(author_id)
    .bind(title)
    .fetch_one(pool)
    .await
    .expect("seed article")
}

/// Seed a published article.
async fn seed_published_article(pool: &PgPool, org_id: Uuid, author_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO news_articles \
         (organization_id, author_id, title, content, status, published_at) \
         VALUES ($1, $2, 'Published article', 'Content.', 'published', NOW()) \
         RETURNING id",
    )
    .bind(org_id)
    .bind(author_id)
    .fetch_one(pool)
    .await
    .expect("seed published article")
}

/// Seed article media, returning its id.
async fn seed_article_media(pool: &PgPool, article_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO article_media \
         (article_id, media_type, file_key, file_name, mime_type, file_size) \
         VALUES ($1, 'image', 'test/img.jpg', 'img.jpg', 'image/jpeg', 1024) \
         RETURNING id",
    )
    .bind(article_id)
    .fetch_one(pool)
    .await
    .expect("seed article media")
}

/// Seed an article comment, returning its id.
async fn seed_comment(pool: &PgPool, article_id: Uuid, user_id: Uuid, content: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO article_comments (article_id, user_id, content) \
         VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(article_id)
    .bind(user_id)
    .bind(content)
    .fetch_one(pool)
    .await
    .expect("seed comment")
}

// ---------------------------------------------------------------------------
// POST /api/v1/news — create_article
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_article_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org) = create_manager_with_org(&app, &user, "news-create").await;

    let resp = app
        .execute(
            app.post("/api/v1/news")
                .bearer(&token)
                .header("X-Tenant-ID", &org.to_string())
                .json(json!({
                    "title": "My News Article",
                    "content": "Some article content."
                }))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::CREATED);
    let body: Value = resp.json_value();
    assert!(body["id"].as_str().is_some(), "id must be present");
    assert_eq!(
        body["message"].as_str(),
        Some("Article created successfully")
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_article_requires_auth(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let resp = app
        .execute(
            app.post("/api/v1/news")
                .json(json!({"title": "x", "content": "y"}))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::UNAUTHORIZED);
}

// ---------------------------------------------------------------------------
// GET /api/v1/news — list_articles
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_articles_returns_articles(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org) = create_manager_with_org(&app, &user, "news-list").await;
    let author_id = user_id_for(&app.pool, &user.email).await;

    seed_article(&app.pool, org, author_id, "Article A").await;
    seed_article(&app.pool, org, author_id, "Article B").await;

    let resp = app
        .execute(
            app.get("/api/v1/news")
                .bearer(&token)
                .header("X-Tenant-ID", &org.to_string())
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json_value();
    let count = body["count"].as_u64().unwrap_or(0);
    assert!(count >= 2, "expected at least 2 articles, got {count}");
}

// ---------------------------------------------------------------------------
// GET /api/v1/news/{id} — get_article
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_article_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org) = create_manager_with_org(&app, &user, "news-get").await;
    let author_id = user_id_for(&app.pool, &user.email).await;

    let article_id = seed_article(&app.pool, org, author_id, "Detail article").await;

    let resp = app
        .execute(
            app.get(&format!("/api/v1/news/{article_id}"))
                .bearer(&token)
                .header("X-Tenant-ID", &org.to_string())
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json_value();
    assert_eq!(
        body["article"]["id"].as_str(),
        Some(article_id.to_string().as_str())
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_article_not_found(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org) = create_manager_with_org(&app, &user, "news-get-nf").await;

    let resp = app
        .execute(
            app.get(&format!("/api/v1/news/{}", Uuid::new_v4()))
                .bearer(&token)
                .header("X-Tenant-ID", &org.to_string())
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::NOT_FOUND);
}

// ---------------------------------------------------------------------------
// PUT /api/v1/news/{id} — update_article
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_article_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org) = create_manager_with_org(&app, &user, "news-update").await;
    let author_id = user_id_for(&app.pool, &user.email).await;

    let article_id = seed_article(&app.pool, org, author_id, "Original title").await;

    let resp = app
        .execute(
            app.put(&format!("/api/v1/news/{article_id}"))
                .bearer(&token)
                .header("X-Tenant-ID", &org.to_string())
                .json(json!({"title": "Updated title"}))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json_value();
    assert_eq!(body["article"]["title"].as_str(), Some("Updated title"));
}

// ---------------------------------------------------------------------------
// POST /api/v1/news/{id}/publish — publish_article
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn publish_article_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org) = create_manager_with_org(&app, &user, "news-publish").await;
    let author_id = user_id_for(&app.pool, &user.email).await;

    let article_id = seed_article(&app.pool, org, author_id, "Draft to publish").await;

    let resp = app
        .execute(
            app.post(&format!("/api/v1/news/{article_id}/publish"))
                .bearer(&token)
                .header("X-Tenant-ID", &org.to_string())
                .json(json!({}))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json_value();
    assert_eq!(body["article"]["status"].as_str(), Some("published"));
}

// ---------------------------------------------------------------------------
// POST /api/v1/news/{id}/archive — archive_article
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn archive_article_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org) = create_manager_with_org(&app, &user, "news-archive").await;
    let author_id = user_id_for(&app.pool, &user.email).await;

    let article_id = seed_published_article(&app.pool, org, author_id).await;

    let resp = app
        .execute(
            app.post(&format!("/api/v1/news/{article_id}/archive"))
                .bearer(&token)
                .header("X-Tenant-ID", &org.to_string())
                .json(json!({}))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json_value();
    assert_eq!(body["article"]["status"].as_str(), Some("archived"));
}

// ---------------------------------------------------------------------------
// POST /api/v1/news/{id}/restore — restore_article
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn restore_article_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org) = create_manager_with_org(&app, &user, "news-restore").await;
    let author_id = user_id_for(&app.pool, &user.email).await;

    // Seed archived article directly
    let article_id: Uuid = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO news_articles \
         (organization_id, author_id, title, content, status, archived_at) \
         VALUES ($1, $2, 'Archived article', 'Content.', 'archived', NOW()) \
         RETURNING id",
    )
    .bind(org)
    .bind(author_id)
    .fetch_one(&app.pool)
    .await
    .expect("seed archived article");

    let resp = app
        .execute(
            app.post(&format!("/api/v1/news/{article_id}/restore"))
                .bearer(&token)
                .header("X-Tenant-ID", &org.to_string())
                .json(json!({}))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json_value();
    assert_eq!(body["article"]["status"].as_str(), Some("draft"));
}

// ---------------------------------------------------------------------------
// POST /api/v1/news/{id}/pin — pin_article
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn pin_article_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org) = create_manager_with_org(&app, &user, "news-pin").await;
    let author_id = user_id_for(&app.pool, &user.email).await;

    let article_id = seed_article(&app.pool, org, author_id, "Pin me").await;

    let resp = app
        .execute(
            app.post(&format!("/api/v1/news/{article_id}/pin"))
                .bearer(&token)
                .header("X-Tenant-ID", &org.to_string())
                .json(json!({"pinned": true}))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json_value();
    assert_eq!(body["article"]["pinned"].as_bool(), Some(true));
}

// ---------------------------------------------------------------------------
// GET /api/v1/news/{id}/media — list_media
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_media_returns_media(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org) = create_manager_with_org(&app, &user, "news-list-media").await;
    let author_id = user_id_for(&app.pool, &user.email).await;

    let article_id = seed_article(&app.pool, org, author_id, "Media article").await;
    seed_article_media(&app.pool, article_id).await;

    let resp = app
        .execute(
            app.get(&format!("/api/v1/news/{article_id}/media"))
                .bearer(&token)
                .header("X-Tenant-ID", &org.to_string())
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json_value();
    let media = body["media"].as_array().expect("media array");
    assert_eq!(media.len(), 1);
}

// ---------------------------------------------------------------------------
// POST /api/v1/news/{id}/media — add_media
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn add_media_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org) = create_manager_with_org(&app, &user, "news-add-media").await;
    let author_id = user_id_for(&app.pool, &user.email).await;

    let article_id = seed_article(&app.pool, org, author_id, "Media article 2").await;

    let resp = app
        .execute(
            app.post(&format!("/api/v1/news/{article_id}/media"))
                .bearer(&token)
                .header("X-Tenant-ID", &org.to_string())
                .json(json!({
                    "media_type": "image",
                    "file_key": "uploads/photo.jpg",
                    "file_name": "photo.jpg",
                    "mime_type": "image/jpeg",
                    "file_size": 2048
                }))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::CREATED);
    let body: Value = resp.json_value();
    assert!(body["id"].as_str().is_some());
    assert_eq!(body["media_type"].as_str(), Some("image"));
}

// ---------------------------------------------------------------------------
// DELETE /api/v1/news/{id}/media/{media_id} — delete_media
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn delete_media_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org) = create_manager_with_org(&app, &user, "news-del-media").await;
    let author_id = user_id_for(&app.pool, &user.email).await;

    let article_id = seed_article(&app.pool, org, author_id, "Media article 3").await;
    let media_id = seed_article_media(&app.pool, article_id).await;

    let resp = app
        .execute(
            app.delete(&format!("/api/v1/news/{article_id}/media/{media_id}"))
                .bearer(&token)
                .header("X-Tenant-ID", &org.to_string())
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);
}

// ---------------------------------------------------------------------------
// POST /api/v1/news/{id}/reactions — toggle_reaction
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn toggle_reaction_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org) = create_manager_with_org(&app, &user, "news-react").await;
    let author_id = user_id_for(&app.pool, &user.email).await;

    let article_id = seed_article(&app.pool, org, author_id, "React article").await;

    let resp = app
        .execute(
            app.post(&format!("/api/v1/news/{article_id}/reactions"))
                .bearer(&token)
                .header("X-Tenant-ID", &org.to_string())
                .json(json!({"reaction": "like"}))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json_value();
    assert_eq!(body["added"].as_bool(), Some(true));
    assert!(body["reaction_counts"].is_object());
}

// ---------------------------------------------------------------------------
// GET /api/v1/news/{id}/reactions/counts — get_reaction_counts
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_reaction_counts_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org) = create_manager_with_org(&app, &user, "news-react-counts").await;
    let author_id = user_id_for(&app.pool, &user.email).await;

    let article_id = seed_article(&app.pool, org, author_id, "Counts article").await;

    let resp = app
        .execute(
            app.get(&format!("/api/v1/news/{article_id}/reactions/counts"))
                .bearer(&token)
                .header("X-Tenant-ID", &org.to_string())
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);
}

// ---------------------------------------------------------------------------
// GET /api/v1/news/{id}/comments — list_comments
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_comments_returns_comments(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org) = create_manager_with_org(&app, &user, "news-list-cmts").await;
    let author_id = user_id_for(&app.pool, &user.email).await;

    let article_id = seed_article(&app.pool, org, author_id, "Comment article").await;
    seed_comment(&app.pool, article_id, author_id, "First comment").await;

    let resp = app
        .execute(
            app.get(&format!("/api/v1/news/{article_id}/comments"))
                .bearer(&token)
                .header("X-Tenant-ID", &org.to_string())
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json_value();
    let count = body["count"].as_u64().unwrap_or(0);
    assert!(count >= 1, "expected at least 1 comment, got {count}");
}

// ---------------------------------------------------------------------------
// POST /api/v1/news/{id}/comments — create_comment
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_comment_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org) = create_manager_with_org(&app, &user, "news-cmt-create").await;
    let author_id = user_id_for(&app.pool, &user.email).await;

    let article_id = seed_article(&app.pool, org, author_id, "Commentable article").await;

    let resp = app
        .execute(
            app.post(&format!("/api/v1/news/{article_id}/comments"))
                .bearer(&token)
                .header("X-Tenant-ID", &org.to_string())
                .json(json!({"content": "Great article!"}))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::CREATED);
    let body: Value = resp.json_value();
    assert!(body["id"].as_str().is_some());
    assert_eq!(body["content"].as_str(), Some("Great article!"));
}

// ---------------------------------------------------------------------------
// PUT /api/v1/news/{id}/comments/{comment_id} — update_comment
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_comment_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org) = create_manager_with_org(&app, &user, "news-cmt-update").await;
    let author_id = user_id_for(&app.pool, &user.email).await;

    let article_id = seed_article(&app.pool, org, author_id, "Article for edit").await;
    let comment_id = seed_comment(&app.pool, article_id, author_id, "Original text").await;

    let resp = app
        .execute(
            app.put(&format!("/api/v1/news/{article_id}/comments/{comment_id}"))
                .bearer(&token)
                .header("X-Tenant-ID", &org.to_string())
                .json(json!({"content": "Edited text"}))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json_value();
    assert_eq!(body["content"].as_str(), Some("Edited text"));
}

// ---------------------------------------------------------------------------
// DELETE /api/v1/news/{id}/comments/{comment_id} — delete_comment
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn delete_comment_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org) = create_manager_with_org(&app, &user, "news-cmt-delete").await;
    let author_id = user_id_for(&app.pool, &user.email).await;

    let article_id = seed_article(&app.pool, org, author_id, "Article for delete cmt").await;
    let comment_id = seed_comment(&app.pool, article_id, author_id, "To be deleted").await;

    let resp = app
        .execute(
            app.delete(&format!("/api/v1/news/{article_id}/comments/{comment_id}"))
                .bearer(&token)
                .header("X-Tenant-ID", &org.to_string())
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);
}

// ---------------------------------------------------------------------------
// POST /api/v1/news/{id}/comments/{comment_id}/moderate — moderate_comment
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn moderate_comment_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org) = create_manager_with_org(&app, &user, "news-cmt-moderate").await;
    let author_id = user_id_for(&app.pool, &user.email).await;

    let article_id = seed_article(&app.pool, org, author_id, "Moderated article").await;
    let comment_id = seed_comment(&app.pool, article_id, author_id, "Offensive comment").await;

    let resp = app
        .execute(
            app.post(&format!(
                "/api/v1/news/{article_id}/comments/{comment_id}/moderate"
            ))
            .bearer(&token)
            .header("X-Tenant-ID", &org.to_string())
            .json(json!({"delete": true, "reason": "Violates guidelines"}))
            .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);
}

// ---------------------------------------------------------------------------
// GET /api/v1/news/{id}/comments/{comment_id}/replies — list_comment_replies
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_comment_replies_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org) = create_manager_with_org(&app, &user, "news-cmt-replies").await;
    let author_id = user_id_for(&app.pool, &user.email).await;

    let article_id = seed_article(&app.pool, org, author_id, "Reply article").await;
    let parent_id = seed_comment(&app.pool, article_id, author_id, "Parent comment").await;

    // Seed a reply
    sqlx::query(
        "INSERT INTO article_comments (article_id, user_id, content, parent_id) \
         VALUES ($1, $2, 'A reply', $3)",
    )
    .bind(article_id)
    .bind(author_id)
    .bind(parent_id)
    .execute(&app.pool)
    .await
    .expect("seed reply");

    let resp = app
        .execute(
            app.get(&format!(
                "/api/v1/news/{article_id}/comments/{parent_id}/replies"
            ))
            .bearer(&token)
            .header("X-Tenant-ID", &org.to_string())
            .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json_value();
    let count = body["count"].as_u64().unwrap_or(0);
    assert_eq!(count, 1, "expected exactly 1 reply, got {count}");
}

// ---------------------------------------------------------------------------
// POST /api/v1/news/{id}/view — record_view
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn record_view_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org) = create_manager_with_org(&app, &user, "news-view").await;
    let author_id = user_id_for(&app.pool, &user.email).await;

    let article_id = seed_article(&app.pool, org, author_id, "View article").await;

    let resp = app
        .execute(
            app.post(&format!("/api/v1/news/{article_id}/view"))
                .bearer(&token)
                .header("X-Tenant-ID", &org.to_string())
                .json(json!({"duration_seconds": 30}))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);
}

// ---------------------------------------------------------------------------
// GET /api/v1/news/statistics — get_statistics
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_statistics_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org) = create_manager_with_org(&app, &user, "news-stats").await;

    let resp = app
        .execute(
            app.get("/api/v1/news/statistics")
                .bearer(&token)
                .header("X-Tenant-ID", &org.to_string())
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);
    let body: Value = resp.json_value();
    assert!(body["statistics"].is_object(), "statistics object expected");
}

// ---------------------------------------------------------------------------
// DELETE /api/v1/news/{id} — delete_article
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn delete_article_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org) = create_manager_with_org(&app, &user, "news-delete").await;
    let author_id = user_id_for(&app.pool, &user.email).await;

    let article_id = seed_article(&app.pool, org, author_id, "To be deleted").await;

    let resp = app
        .execute(
            app.delete(&format!("/api/v1/news/{article_id}"))
                .bearer(&token)
                .header("X-Tenant-ID", &org.to_string())
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);

    // Subsequent GET returns 404
    let get_resp = app
        .execute(
            app.get(&format!("/api/v1/news/{article_id}"))
                .bearer(&token)
                .header("X-Tenant-ID", &org.to_string())
                .build(),
        )
        .await;
    get_resp.assert_status(StatusCode::NOT_FOUND);
}
