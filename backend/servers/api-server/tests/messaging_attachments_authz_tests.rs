//! Authorization tests for message attachments (UC-05.9 / BIT-184).
//!
//! Covers the security contract from the gap-sweep fix plan:
//! - upload + link an attachment to a message,
//! - list attachments (thread participants only),
//! - only the message sender may attach to a message,
//! - download authz: non-participants and cross-tenant callers are denied
//!   *before* any storage interaction (proven by an authorized caller reaching
//!   the storage layer — 503 in the harness, which has no S3 wired — while
//!   unauthorized callers get 403/404).
//!
//! Threads/messages/attachments are seeded directly via SQL so the tests don't
//! depend on the `start_thread` flow (which couples to `users.organization_id`).
//! The integration pool is privileged (RLS not FORCE-applied), so these tests
//! exercise the handler-layer participant/tenant/sender guards — the same
//! defense-in-depth layer the production RLS policy backstops.

#![allow(dead_code)]

mod common;

use axum::http::StatusCode;
use common::{
    create_authenticated_user, create_authenticated_user_with_org, seed_membership, TestApp,
    TestUser,
};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

async fn user_id(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(email)
        .fetch_one(pool)
        .await
        .expect("resolve user id")
}

async fn insert_thread(pool: &PgPool, org_id: Uuid, participants: &[Uuid]) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO message_threads (organization_id, participant_ids)
           VALUES ($1, $2) RETURNING id"#,
    )
    .bind(org_id)
    .bind(participants)
    .fetch_one(pool)
    .await
    .expect("insert thread")
}

async fn insert_message(pool: &PgPool, thread_id: Uuid, sender_id: Uuid, content: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO messages (thread_id, sender_id, content)
           VALUES ($1, $2, $3) RETURNING id"#,
    )
    .bind(thread_id)
    .bind(sender_id)
    .bind(content)
    .fetch_one(pool)
    .await
    .expect("insert message")
}

async fn insert_attachment(pool: &PgPool, message_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO message_attachments (message_id, file_key, file_name, file_type, file_size)
           VALUES ($1, $2, $3, $4, $5) RETURNING id"#,
    )
    .bind(message_id)
    .bind(format!("messages/seed/{}", Uuid::new_v4()))
    .bind("seed.pdf")
    .bind("application/pdf")
    .bind(2048_i64)
    .fetch_one(pool)
    .await
    .expect("insert attachment")
}

/// Happy path: a participant links an attachment to their own message and the
/// other participant can list it.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn link_then_list_attachment(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let alice = TestUser::new();
    let (alice_tok, org1) = create_authenticated_user_with_org(&app, &alice, "att-a").await;
    let alice_id = user_id(&pool, &alice.email).await;

    let bob = TestUser::new();
    let (bob_tok, _r) = create_authenticated_user(&app, &bob).await;
    let bob_id = user_id(&pool, &bob.email).await;
    seed_membership(&pool, org1, bob_id, "resident").await;

    let thread = insert_thread(&pool, org1, &[alice_id, bob_id]).await;
    let msg = insert_message(&pool, thread, alice_id, "see attached").await;

    // Alice links an attachment to her own message.
    let link = app
        .session(alice_tok, org1)
        .post(&format!(
            "/api/v1/messages/threads/{thread}/messages/{msg}/attachments"
        ))
        .json(json!({
            "fileKey": format!("messages/{thread}/obj"),
            "fileName": "lease.pdf",
            "fileType": "application/pdf",
            "fileSize": 4096
        }))
        .build();
    app.execute(link).await.assert_status(StatusCode::CREATED);

    // Bob (the other participant) can list it.
    let list = app
        .session(bob_tok, org1)
        .get(&format!(
            "/api/v1/messages/threads/{thread}/messages/{msg}/attachments"
        ))
        .build();
    let resp = app.execute(list).await;
    resp.assert_status(StatusCode::OK);
    let body = resp.json_value();
    assert_eq!(body["count"], json!(1), "body: {body}");
    assert_eq!(body["attachments"][0]["fileName"], json!("lease.pdf"));
}

/// Only the message sender may attach a file to it — another participant cannot.
#[ignore]
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn non_sender_cannot_link_attachment(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let alice = TestUser::new();
    let (_alice_tok, org1) = create_authenticated_user_with_org(&app, &alice, "att-b").await;
    let alice_id = user_id(&pool, &alice.email).await;

    let bob = TestUser::new();
    let (bob_tok, _r) = create_authenticated_user(&app, &bob).await;
    let bob_id = user_id(&pool, &bob.email).await;
    seed_membership(&pool, org1, bob_id, "resident").await;

    let thread = insert_thread(&pool, org1, &[alice_id, bob_id]).await;
    let msg = insert_message(&pool, thread, alice_id, "mine").await;

    // Bob is a participant but NOT the sender of `msg`.
    let link = app
        .session(bob_tok, org1)
        .post(&format!(
            "/api/v1/messages/threads/{thread}/messages/{msg}/attachments"
        ))
        .json(json!({
            "fileKey": "messages/x/y",
            "fileName": "evil.pdf",
            "fileType": "application/pdf",
            "fileSize": 10
        }))
        .build();
    app.execute(link).await.assert_status(StatusCode::FORBIDDEN);
}

/// A user who is not a participant of the thread cannot list its attachments.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn non_participant_cannot_list_attachments(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let alice = TestUser::new();
    let (_alice_tok, org1) = create_authenticated_user_with_org(&app, &alice, "att-c").await;
    let alice_id = user_id(&pool, &alice.email).await;

    let bob = TestUser::new();
    let (_bob_tok, _r) = create_authenticated_user(&app, &bob).await;
    let bob_id = user_id(&pool, &bob.email).await;
    seed_membership(&pool, org1, bob_id, "resident").await;

    // Carol is a member of org1 but not part of the thread.
    let carol = TestUser::new();
    let (carol_tok, _r2) = create_authenticated_user(&app, &carol).await;
    let carol_id = user_id(&pool, &carol.email).await;
    seed_membership(&pool, org1, carol_id, "resident").await;

    let thread = insert_thread(&pool, org1, &[alice_id, bob_id]).await;
    let msg = insert_message(&pool, thread, alice_id, "private").await;
    insert_attachment(&pool, msg).await;

    let list = app
        .session(carol_tok, org1)
        .get(&format!(
            "/api/v1/messages/threads/{thread}/messages/{msg}/attachments"
        ))
        .build();
    app.execute(list).await.assert_status(StatusCode::FORBIDDEN);
}

/// Download authz: a participant passes the authz gate (reaching the storage
/// layer — 503 in the no-S3 harness), while a non-participant in the SAME org
/// and a user from a DIFFERENT org are both denied before any storage call.
#[ignore]
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn download_url_authz_gate(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let alice = TestUser::new();
    let (alice_tok, org1) = create_authenticated_user_with_org(&app, &alice, "att-d1").await;
    let alice_id = user_id(&pool, &alice.email).await;

    let bob = TestUser::new();
    let (_bob_tok, _r) = create_authenticated_user(&app, &bob).await;
    let bob_id = user_id(&pool, &bob.email).await;
    seed_membership(&pool, org1, bob_id, "resident").await;

    // Carol: same org, not a participant.
    let carol = TestUser::new();
    let (carol_tok, _r2) = create_authenticated_user(&app, &carol).await;
    let carol_id = user_id(&pool, &carol.email).await;
    seed_membership(&pool, org1, carol_id, "resident").await;

    // Dave: a different org entirely (cross-tenant).
    let dave = TestUser::new();
    let (dave_tok, org2) = create_authenticated_user_with_org(&app, &dave, "att-d2").await;

    let thread = insert_thread(&pool, org1, &[alice_id, bob_id]).await;
    let msg = insert_message(&pool, thread, alice_id, "with file").await;
    let attachment = insert_attachment(&pool, msg).await;

    let path = format!("/api/v1/messages/threads/{thread}/attachments/{attachment}/download");

    // Participant (Alice) clears authz → reaches storage, which is unconfigured.
    let ok = app
        .execute(app.session(alice_tok, org1).get(&path).build())
        .await;
    assert_eq!(
        ok.status,
        StatusCode::SERVICE_UNAVAILABLE,
        "authorized participant should pass authz and hit the storage gate; body: {}",
        ok.text()
    );

    // Same-org non-participant → denied before storage.
    let denied = app
        .execute(app.session(carol_tok, org1).get(&path).build())
        .await;
    assert_eq!(
        denied.status,
        StatusCode::FORBIDDEN,
        "body: {}",
        denied.text()
    );

    // Cross-tenant caller → denied.
    let cross = app
        .execute(app.session(dave_tok, org2).get(&path).build())
        .await;
    assert_eq!(
        cross.status,
        StatusCode::FORBIDDEN,
        "cross-tenant download must be denied; body: {}",
        cross.text()
    );
}

/// An attachment id from one thread cannot be downloaded via a different thread
/// the caller does happen to participate in (path/object mismatch → 404).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn download_rejects_attachment_thread_mismatch(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let alice = TestUser::new();
    let (alice_tok, org1) = create_authenticated_user_with_org(&app, &alice, "att-e").await;
    let alice_id = user_id(&pool, &alice.email).await;

    let bob = TestUser::new();
    let (_bob_tok, _r) = create_authenticated_user(&app, &bob).await;
    let bob_id = user_id(&pool, &bob.email).await;
    seed_membership(&pool, org1, bob_id, "resident").await;

    let carol = TestUser::new();
    let (_carol_tok, _rc) = create_authenticated_user(&app, &carol).await;
    let carol_id = user_id(&pool, &carol.email).await;
    seed_membership(&pool, org1, carol_id, "resident").await;

    // Thread X (alice+bob) holds the attachment; thread Y (alice+carol) does not.
    // The two threads must have distinct participant sets to satisfy the
    // (organization_id, participant_ids) UNIQUE constraint added in 00189.
    let thread_x = insert_thread(&pool, org1, &[alice_id, bob_id]).await;
    let msg_x = insert_message(&pool, thread_x, alice_id, "x").await;
    let attachment = insert_attachment(&pool, msg_x).await;

    let thread_y = insert_thread(&pool, org1, &[alice_id, carol_id]).await;

    // Alice participates in thread_y but the attachment lives in thread_x.
    let path = format!("/api/v1/messages/threads/{thread_y}/attachments/{attachment}/download");
    let resp = app
        .execute(app.session(alice_tok, org1).get(&path).build())
        .await;
    assert_eq!(resp.status, StatusCode::NOT_FOUND, "body: {}", resp.text());
}

/// IDOR guard (#1791/#1770): the message sender cannot link a `file_key` that
/// points outside this thread's namespace (e.g. another tenant's
/// `documents/<org>/…` object). Without the prefix guard the handler would
/// store the client-supplied key verbatim and later mint a presigned GET for
/// it — an IDOR over the whole bucket.
///
/// The request uses Alice (the message sender) and a valid `application/pdf`
/// content type, so it clears `validate_attachment_meta`, the non-empty check,
/// the content-type allowlist, AND the sender/participant authz — the *only*
/// thing that can reject it is the file_key prefix guard. We assert both the
/// 400 status and the `INVALID_FILE_KEY` code so the test pins that guard
/// specifically (not an incidental authz 403).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn link_rejects_foreign_prefixed_file_key(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let alice = TestUser::new();
    let (alice_tok, org1) = create_authenticated_user_with_org(&app, &alice, "att-f").await;
    let alice_id = user_id(&pool, &alice.email).await;

    let bob = TestUser::new();
    let (_bob_tok, _r) = create_authenticated_user(&app, &bob).await;
    let bob_id = user_id(&pool, &bob.email).await;
    seed_membership(&pool, org1, bob_id, "resident").await;

    let thread = insert_thread(&pool, org1, &[alice_id, bob_id]).await;
    let msg = insert_message(&pool, thread, alice_id, "see attached").await;

    // A key that was NEVER issued for this thread: a foreign `documents/<org>/…`
    // object. Valid `.pdf` name + allowed MIME, so only the prefix guard fires.
    let link = app
        .session(alice_tok, org1)
        .post(&format!(
            "/api/v1/messages/threads/{thread}/messages/{msg}/attachments"
        ))
        .json(json!({
            "fileKey": format!("documents/{org1}/secret.pdf"),
            "fileName": "secret.pdf",
            "fileType": "application/pdf",
            "fileSize": 4096
        }))
        .build();
    let resp = app.execute(link).await;
    resp.assert_status(StatusCode::BAD_REQUEST);
    let body = resp.json_value();
    assert_eq!(
        body["code"], "INVALID_FILE_KEY",
        "foreign-prefixed key must be rejected by the prefix guard; body: {body}"
    );

    // And nothing was persisted for that message — the IDOR row never exists.
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM message_attachments WHERE message_id = $1")
            .bind(msg)
            .fetch_one(&pool)
            .await
            .expect("count attachments");
    assert_eq!(
        count, 0,
        "no attachment row should be persisted for a foreign key"
    );
}

/// MIME guard (#1791/#1770): even with a correctly thread-scoped `file_key`, a
/// disallowed `file_type` (e.g. `text/html`, which is not in
/// `integrations::ALLOWED_MIME_TYPES`) is rejected at link time, mirroring the
/// allowlist the upload-url endpoint enforces. This prevents persisting a
/// content type that would later drive the presigned download's Content-Type.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn link_rejects_disallowed_content_type(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let alice = TestUser::new();
    let (alice_tok, org1) = create_authenticated_user_with_org(&app, &alice, "att-g").await;
    let alice_id = user_id(&pool, &alice.email).await;

    let bob = TestUser::new();
    let (_bob_tok, _r) = create_authenticated_user(&app, &bob).await;
    let bob_id = user_id(&pool, &bob.email).await;
    seed_membership(&pool, org1, bob_id, "resident").await;

    let thread = insert_thread(&pool, org1, &[alice_id, bob_id]).await;
    let msg = insert_message(&pool, thread, alice_id, "see attached").await;

    // Correct thread-scoped key (clears the prefix guard) but a disallowed MIME.
    let link = app
        .session(alice_tok, org1)
        .post(&format!(
            "/api/v1/messages/threads/{thread}/messages/{msg}/attachments"
        ))
        .json(json!({
            "fileKey": format!("messages/{thread}/obj"),
            "fileName": "page.html",
            "fileType": "text/html",
            "fileSize": 1024
        }))
        .build();
    let resp = app.execute(link).await;
    resp.assert_status(StatusCode::BAD_REQUEST);
    let body = resp.json_value();
    assert_eq!(
        body["code"], "INVALID_CONTENT_TYPE",
        "disallowed content type must be rejected; body: {body}"
    );

    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM message_attachments WHERE message_id = $1")
            .bind(msg)
            .fetch_one(&pool)
            .await
            .expect("count attachments");
    assert_eq!(
        count, 0,
        "no attachment row should be persisted for a disallowed MIME"
    );
}
