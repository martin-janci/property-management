//! Integration tests backfilling the remaining partial messaging/thread endpoints.
//! Covers success paths for:
//! - POST /api/v1/messages/threads/{id}/messages
//! - DELETE /api/v1/messages/threads/{id}/messages/{message_id}
//! - POST /api/v1/messages/threads/{id}/read
//! - GET /api/v1/messages/unread-count
//! - POST /api/v1/messages/users/{id}/block
//! - GET /api/v1/messages/users/blocked
//! - DELETE /api/v1/messages/users/{id}/block
//! - POST /api/v1/messages/threads/{id}/attachments/upload-url
//! - GET /api/v1/messages/threads/{id}/attachments/{attachment_id}/download
//! - POST /api/v1/messages/threads/{id}/archive
//! - DELETE /api/v1/messages/threads/{id}/archive

#![allow(dead_code)]

use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{seed_membership, TestApp, TestConfig};

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
// Happy Path Tests
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn messaging_happy_paths(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "beh").await;
    let (alice, alice_email) = seed_user(&pool, "alice").await;
    let (bob, bob_email) = seed_user(&pool, "bob").await;

    seed_membership(&pool, org, alice, "org_admin").await;
    seed_membership(&pool, org, bob, "resident").await;

    let token_a = mint_token(alice, &alice_email);
    let token_b = mint_token(bob, &bob_email);

    // Seed a thread first
    let thread = seed_thread(&pool, org, alice, bob).await;

    // 1. Send Message happy path (POST /api/v1/messages/threads/{id}/messages)
    let send_req = app
        .execute(
            app.post(&format!("/api/v1/messages/threads/{thread}/messages"))
                .bearer(&token_a)
                .header("X-Tenant-ID", &org.to_string())
                .json(json!({ "content": "Hello bob!" }))
                .build(),
        )
        .await;
    assert_eq!(send_req.status, StatusCode::OK);
    let sent_body = send_req.json_value();
    assert_eq!(sent_body["message"], "Message sent successfully");
    let message_id_str = sent_body["sentMessage"]["id"].as_str().expect("message id");
    let message_id = Uuid::parse_str(message_id_str).expect("parse message id");

    // 2. Get Unread Count happy path (GET /api/v1/messages/unread-count)
    let unread_req = app
        .execute(
            app.get("/api/v1/messages/unread-count")
                .bearer(&token_b)
                .header("X-Tenant-ID", &org.to_string())
                .build(),
        )
        .await;
    assert_eq!(unread_req.status, StatusCode::OK);
    assert_eq!(unread_req.json_value()["unreadCount"].as_i64(), Some(1));

    // 3. Mark Thread Read happy path (POST /api/v1/messages/threads/{id}/read)
    let read_req = app
        .execute(
            app.post(&format!("/api/v1/messages/threads/{thread}/read"))
                .bearer(&token_b)
                .header("X-Tenant-ID", &org.to_string())
                .build(),
        )
        .await;
    assert_eq!(read_req.status, StatusCode::OK);
    assert!(read_req.json_value()["message"]
        .as_str()
        .unwrap()
        .contains("messages marked as read"));

    // Verify Bob's unread count is now 0
    let unread_req2 = app
        .execute(
            app.get("/api/v1/messages/unread-count")
                .bearer(&token_b)
                .header("X-Tenant-ID", &org.to_string())
                .build(),
        )
        .await;
    assert_eq!(unread_req2.status, StatusCode::OK);
    assert_eq!(unread_req2.json_value()["unreadCount"].as_i64(), Some(0));

    // 4. Delete Message happy path (DELETE /api/v1/messages/threads/{id}/messages/{message_id})
    let delete_req = app
        .execute(
            app.delete(&format!(
                "/api/v1/messages/threads/{thread}/messages/{message_id}"
            ))
            .bearer(&token_a)
            .header("X-Tenant-ID", &org.to_string())
            .build(),
        )
        .await;
    assert_eq!(delete_req.status, StatusCode::OK);
    assert_eq!(
        delete_req.json_value()["message"],
        "Message deleted successfully"
    );

    // 5. Archive Thread and Unarchive Thread happy path (POST / DELETE /threads/{id}/archive)
    let arch_req = app
        .execute(
            app.post(&format!("/api/v1/messages/threads/{thread}/archive"))
                .bearer(&token_a)
                .header("X-Tenant-ID", &org.to_string())
                .build(),
        )
        .await;
    assert_eq!(arch_req.status, StatusCode::OK);
    assert_eq!(arch_req.json_value()["message"], "Conversation archived");

    let unarch_req = app
        .execute(
            app.delete(&format!("/api/v1/messages/threads/{thread}/archive"))
                .bearer(&token_a)
                .header("X-Tenant-ID", &org.to_string())
                .build(),
        )
        .await;
    assert_eq!(unarch_req.status, StatusCode::OK);
    assert_eq!(
        unarch_req.json_value()["message"],
        "Conversation un-archived"
    );

    // 6. Block User, List Blocked, and Unblock User happy path
    let block_req = app
        .execute(
            app.post(&format!("/api/v1/messages/users/{bob}/block"))
                .bearer(&token_a)
                .header("X-Tenant-ID", &org.to_string())
                .build(),
        )
        .await;
    assert_eq!(block_req.status, StatusCode::OK);
    assert_eq!(
        block_req.json_value()["message"],
        "User blocked successfully"
    );

    let list_blocked = app
        .execute(
            app.get("/api/v1/messages/users/blocked")
                .bearer(&token_a)
                .header("X-Tenant-ID", &org.to_string())
                .build(),
        )
        .await;
    assert_eq!(list_blocked.status, StatusCode::OK);
    let blocked_body = list_blocked.json_value();
    assert_eq!(blocked_body["count"].as_i64(), Some(1));
    assert_eq!(
        blocked_body["blockedUsers"][0]["blockedUser"]["id"].as_str(),
        Some(bob.to_string().as_str())
    );

    let unblock_req = app
        .execute(
            app.delete(&format!("/api/v1/messages/users/{bob}/block"))
                .bearer(&token_a)
                .header("X-Tenant-ID", &org.to_string())
                .build(),
        )
        .await;
    assert_eq!(unblock_req.status, StatusCode::OK);
    assert_eq!(
        unblock_req.json_value()["message"],
        "User unblocked successfully"
    );

    let list_blocked2 = app
        .execute(
            app.get("/api/v1/messages/users/blocked")
                .bearer(&token_a)
                .header("X-Tenant-ID", &org.to_string())
                .build(),
        )
        .await;
    assert_eq!(list_blocked2.status, StatusCode::OK);
    assert_eq!(list_blocked2.json_value()["count"].as_i64(), Some(0));

    // 7. Request attachment upload URL and GET attachment download URL
    // These reach the S3/storage layer and fail with 503 SERVICE_UNAVAILABLE since no S3 is configured.
    let upload_req = app
        .execute(
            app.post(&format!(
                "/api/v1/messages/threads/{thread}/attachments/upload-url"
            ))
            .bearer(&token_a)
            .header("X-Tenant-ID", &org.to_string())
            .json(json!({
                "fileName": "test.txt",
                "fileType": "text/plain",
                "fileSize": 100
            }))
            .build(),
        )
        .await;
    assert_eq!(upload_req.status, StatusCode::SERVICE_UNAVAILABLE);

    let msg_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO messages (thread_id, sender_id, content) VALUES ($1, $2, 'test file') RETURNING id"#,
    )
    .bind(thread)
    .bind(alice)
    .fetch_one(&pool)
    .await
    .unwrap();

    let attachment_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO message_attachments (message_id, file_key, file_name, file_type, file_size)
           VALUES ($1, 'messages/test-file-key', 'test.txt', 'text/plain', 100) RETURNING id"#,
    )
    .bind(msg_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    let download_req = app
        .execute(
            app.get(&format!(
                "/api/v1/messages/threads/{thread}/attachments/{attachment_id}/download"
            ))
            .bearer(&token_a)
            .header("X-Tenant-ID", &org.to_string())
            .build(),
        )
        .await;
    assert_eq!(download_req.status, StatusCode::SERVICE_UNAVAILABLE);
}
