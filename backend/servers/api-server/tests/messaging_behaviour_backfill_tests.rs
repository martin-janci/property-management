//! Integration tests backfilling the remaining partial messaging/thread endpoints.
//! Covers success paths for:
//! - POST /api/v1/messages/threads/{id}/messages        (send_message)
//! - DELETE /api/v1/messages/threads/{id}/messages/{id} (delete_message)
//! - POST /api/v1/messages/threads/{id}/read            (mark_thread_read)
//! - GET  /api/v1/messages/unread-count                 (get_unread_count)
//! - POST /api/v1/messages/users/{id}/block             (block_user)
//! - GET  /api/v1/messages/users/blocked                (list_blocked_users)
//! - DELETE /api/v1/messages/users/{id}/block           (unblock_user)
//! - POST /api/v1/messages/threads/{id}/attachments/upload-url  (request_attachment_upload_url)
//! - GET  /api/v1/messages/threads/{id}/attachments/{id}/download (get_attachment_download_url)
//! - POST /api/v1/messages/threads/{id}/archive         (archive_thread — success path)
//! - DELETE /api/v1/messages/threads/{id}/archive       (unarchive_thread)

#[allow(dead_code)]
mod common;

use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use common::{seed_membership, TestApp, TestConfig};

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

async fn seed_org(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO organizations (name, slug, contact_email, status)
        VALUES ($1, $2, $3, 'active') RETURNING id
        "#,
    )
    .bind(format!("MsgBeh Org {slug}"))
    .bind(format!("msgbeh-org-{slug}-{}", Uuid::new_v4()))
    .bind(format!("{slug}-{}@msgbeh.test", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool, label: &str) -> (Uuid, String) {
    let email = format!("{label}-{}@msgbeh.test", Uuid::new_v4());
    let id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'test_hash', 'MsgBeh User', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(&email)
    .fetch_one(pool)
    .await
    .expect("seed user");
    (id, email)
}

async fn seed_thread(pool: &PgPool, org: Uuid, a: Uuid, b: Uuid) -> Uuid {
    let mut ids = [a, b];
    ids.sort();
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO message_threads (organization_id, participant_ids)
        VALUES ($1, $2)
        RETURNING id
        "#,
    )
    .bind(org)
    .bind(&ids[..])
    .fetch_one(pool)
    .await
    .expect("seed thread")
}

fn mint_token(user_id: Uuid, email: &str) -> String {
    use api_server::services::JwtService;
    let config = TestConfig::default();
    let jwt = JwtService::new(&config.jwt_secret).expect("jwt service");
    jwt.generate_access_token(user_id, email, "MsgBeh User", None, None)
        .expect("mint access token")
}

// ---------------------------------------------------------------------------
// Happy-path: send, delete, read, unread-count
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn messaging_send_delete_read_unread(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "sdr").await;
    let (alice, alice_email) = seed_user(&pool, "alice-sdr").await;
    let (bob, bob_email) = seed_user(&pool, "bob-sdr").await;

    seed_membership(&pool, org, alice, "org_admin").await;
    seed_membership(&pool, org, bob, "resident").await;

    let tok_a = mint_token(alice, &alice_email);
    let tok_b = mint_token(bob, &bob_email);
    let thread = seed_thread(&pool, org, alice, bob).await;

    // send_message → 200, returns sentMessage.id
    let send = app
        .execute(
            app.post(&format!("/api/v1/messages/threads/{thread}/messages"))
                .bearer(&tok_a)
                .header("X-Tenant-ID", &org.to_string())
                .json(json!({ "content": "Hello bob!" }))
                .build(),
        )
        .await;
    assert_eq!(send.status, StatusCode::OK);
    let send_body = send.json_value();
    assert_eq!(send_body["message"], "Message sent successfully");
    let msg_id = Uuid::parse_str(
        send_body["sentMessage"]["id"]
            .as_str()
            .expect("sentMessage.id"),
    )
    .expect("parse msg id");

    // get_unread_count for bob → 1
    let unread = app
        .execute(
            app.get("/api/v1/messages/unread-count")
                .bearer(&tok_b)
                .header("X-Tenant-ID", &org.to_string())
                .build(),
        )
        .await;
    assert_eq!(unread.status, StatusCode::OK);
    assert_eq!(unread.json_value()["unreadCount"].as_i64(), Some(1));

    // mark_thread_read for bob → 200
    let read = app
        .execute(
            app.post(&format!("/api/v1/messages/threads/{thread}/read"))
                .bearer(&tok_b)
                .header("X-Tenant-ID", &org.to_string())
                .build(),
        )
        .await;
    assert_eq!(read.status, StatusCode::OK);
    assert!(
        read.json_value()["message"]
            .as_str()
            .unwrap_or("")
            .contains("messages marked as read"),
        "expected 'messages marked as read' in response"
    );

    // unread count for bob should now be 0
    let unread2 = app
        .execute(
            app.get("/api/v1/messages/unread-count")
                .bearer(&tok_b)
                .header("X-Tenant-ID", &org.to_string())
                .build(),
        )
        .await;
    assert_eq!(unread2.status, StatusCode::OK);
    assert_eq!(unread2.json_value()["unreadCount"].as_i64(), Some(0));

    // delete_message → 200
    let del = app
        .execute(
            app.delete(&format!(
                "/api/v1/messages/threads/{thread}/messages/{msg_id}"
            ))
            .bearer(&tok_a)
            .header("X-Tenant-ID", &org.to_string())
            .build(),
        )
        .await;
    assert_eq!(del.status, StatusCode::OK);
    assert_eq!(del.json_value()["message"], "Message deleted successfully");
}

// ---------------------------------------------------------------------------
// Happy-path: archive / unarchive thread
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn messaging_archive_unarchive(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "arc").await;
    let (alice, alice_email) = seed_user(&pool, "alice-arc").await;
    let (bob, _bob_email) = seed_user(&pool, "bob-arc").await;

    seed_membership(&pool, org, alice, "org_admin").await;
    seed_membership(&pool, org, bob, "resident").await;

    let tok_a = mint_token(alice, &alice_email);
    let thread = seed_thread(&pool, org, alice, bob).await;

    // archive_thread → 200
    let arch = app
        .execute(
            app.post(&format!("/api/v1/messages/threads/{thread}/archive"))
                .bearer(&tok_a)
                .header("X-Tenant-ID", &org.to_string())
                .build(),
        )
        .await;
    assert_eq!(arch.status, StatusCode::OK);
    assert_eq!(arch.json_value()["message"], "Conversation archived");

    // unarchive_thread → 200
    let unarch = app
        .execute(
            app.delete(&format!("/api/v1/messages/threads/{thread}/archive"))
                .bearer(&tok_a)
                .header("X-Tenant-ID", &org.to_string())
                .build(),
        )
        .await;
    assert_eq!(unarch.status, StatusCode::OK);
    assert_eq!(unarch.json_value()["message"], "Conversation un-archived");
}

// ---------------------------------------------------------------------------
// Happy-path: block / list-blocked / unblock
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn messaging_block_list_unblock(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "blk").await;
    let (alice, alice_email) = seed_user(&pool, "alice-blk").await;
    let (bob, _bob_email) = seed_user(&pool, "bob-blk").await;

    seed_membership(&pool, org, alice, "org_admin").await;
    seed_membership(&pool, org, bob, "resident").await;

    let tok_a = mint_token(alice, &alice_email);

    // block_user → 200
    let block = app
        .execute(
            app.post(&format!("/api/v1/messages/users/{bob}/block"))
                .bearer(&tok_a)
                .header("X-Tenant-ID", &org.to_string())
                .build(),
        )
        .await;
    assert_eq!(block.status, StatusCode::OK);
    assert_eq!(block.json_value()["message"], "User blocked successfully");

    // list_blocked_users → 200, count=1, blockedId matches bob
    let list = app
        .execute(
            app.get("/api/v1/messages/users/blocked")
                .bearer(&tok_a)
                .header("X-Tenant-ID", &org.to_string())
                .build(),
        )
        .await;
    assert_eq!(list.status, StatusCode::OK);
    let list_body = list.json_value();
    assert_eq!(list_body["count"].as_i64(), Some(1));
    assert_eq!(
        list_body["blockedUsers"][0]["blockedId"].as_str(),
        Some(bob.to_string().as_str())
    );

    // unblock_user → 200
    let unblock = app
        .execute(
            app.delete(&format!("/api/v1/messages/users/{bob}/block"))
                .bearer(&tok_a)
                .header("X-Tenant-ID", &org.to_string())
                .build(),
        )
        .await;
    assert_eq!(unblock.status, StatusCode::OK);
    assert_eq!(
        unblock.json_value()["message"],
        "User unblocked successfully"
    );

    // list_blocked_users after unblock → count=0
    let list2 = app
        .execute(
            app.get("/api/v1/messages/users/blocked")
                .bearer(&tok_a)
                .header("X-Tenant-ID", &org.to_string())
                .build(),
        )
        .await;
    assert_eq!(list2.status, StatusCode::OK);
    assert_eq!(list2.json_value()["count"].as_i64(), Some(0));
}

// ---------------------------------------------------------------------------
// Storage-layer: attachment upload-url + download-url
// Reaches S3 layer; returns 503 in the no-S3 test harness.
// Proves the auth/participant gate is passed and the route is reachable.
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn messaging_attachment_storage_routes(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "att").await;
    let (alice, alice_email) = seed_user(&pool, "alice-att").await;
    let (bob, _bob_email) = seed_user(&pool, "bob-att").await;

    seed_membership(&pool, org, alice, "org_admin").await;
    seed_membership(&pool, org, bob, "resident").await;

    let tok_a = mint_token(alice, &alice_email);
    let thread = seed_thread(&pool, org, alice, bob).await;

    // request_attachment_upload_url → 503 (reaches S3, which is absent)
    let upload = app
        .execute(
            app.post(&format!(
                "/api/v1/messages/threads/{thread}/attachments/upload-url"
            ))
            .bearer(&tok_a)
            .header("X-Tenant-ID", &org.to_string())
            .json(json!({
                "fileName": "report.pdf",
                "fileType": "application/pdf",
                "fileSize": 102400
            }))
            .build(),
        )
        .await;
    assert_eq!(upload.status, StatusCode::SERVICE_UNAVAILABLE);

    // Seed a message + attachment so download URL can be requested
    let msg_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO messages (thread_id, sender_id, content) VALUES ($1, $2, 'attach test') RETURNING id"#,
    )
    .bind(thread)
    .bind(alice)
    .fetch_one(&pool)
    .await
    .expect("seed message");

    let att_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO message_attachments (message_id, file_key, file_name, file_type, file_size)
        VALUES ($1, 'messages/test-key', 'report.pdf', 'application/pdf', 102400)
        RETURNING id
        "#,
    )
    .bind(msg_id)
    .fetch_one(&pool)
    .await
    .expect("seed attachment");

    // get_attachment_download_url → 503 (reaches S3, which is absent)
    let download = app
        .execute(
            app.get(&format!(
                "/api/v1/messages/threads/{thread}/attachments/{att_id}/download"
            ))
            .bearer(&tok_a)
            .header("X-Tenant-ID", &org.to_string())
            .build(),
        )
        .await;
    assert_eq!(download.status, StatusCode::SERVICE_UNAVAILABLE);
}
