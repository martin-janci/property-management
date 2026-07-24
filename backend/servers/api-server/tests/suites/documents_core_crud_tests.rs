//! BIT-268 wave-4 batch 1: documents core CRUD, versions, and shares happy-path tests.
//!
//! Covers partial endpoints from docs/endpoint-checklist/groups/documents-forms.md:
//! - POST   /api/v1/documents           (create_document)
//! - GET    /api/v1/documents           (list_documents)
//! - PUT    /api/v1/documents/{id}      (update_document)
//! - DELETE /api/v1/documents/{id}      (delete_document)
//! - PUT    /api/v1/documents/{id}/access (update_document_access)
//! - GET    /api/v1/documents/{id}/versions (get_version_history)
//! - POST   /api/v1/documents/{id}/versions (create_version)
//! - GET    /api/v1/documents/{id}/versions/{version_id} (get_version)
//! - POST   /api/v1/documents/{id}/versions/{version_id}/restore (restore_version)
//! - GET    /api/v1/documents/{id}/shares (list_shares)
//! - POST   /api/v1/documents/{id}/shares (create_share)
//! - DELETE /api/v1/documents/{id}/shares/{share_id} (revoke_share)

#![allow(dead_code)]

use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{
    create_authenticated_user, create_authenticated_user_with_org, seed_membership, seed_org,
    TestApp, TestUser,
};

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

async fn seed_document(pool: &PgPool, org_id: Uuid, user_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO documents \
         (organization_id, title, category, file_key, file_name, mime_type, size_bytes, created_by) \
         VALUES ($1, 'Test Doc', 'other', 'test/key.pdf', 'test.pdf', 'application/pdf', 1024, $2) \
         RETURNING id",
    )
    .bind(org_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("seed document")
}

// document_versions is a VIEW over the documents table (migration 00035).
// Versions are stored as additional rows in `documents` with parent_document_id
// pointing to the root document.
async fn seed_document_version(
    pool: &PgPool,
    root_doc_id: Uuid,
    org_id: Uuid,
    user_id: Uuid,
) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO documents \
         (organization_id, title, category, file_key, file_name, mime_type, size_bytes, \
          created_by, parent_document_id, version_number, is_current_version) \
         VALUES ($1, 'Test Doc v2', 'other', 'test/v2.pdf', 'v2.pdf', 'application/pdf', 2048, \
                 $2, $3, 2, true) \
         RETURNING id",
    )
    .bind(org_id)
    .bind(user_id)
    .bind(root_doc_id)
    .fetch_one(pool)
    .await
    .expect("seed document version")
}

async fn seed_document_share(pool: &PgPool, document_id: Uuid, shared_by: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO document_shares \
         (document_id, share_type, shared_by) \
         VALUES ($1, 'link', $2) \
         RETURNING id",
    )
    .bind(document_id)
    .bind(shared_by)
    .fetch_one(pool)
    .await
    .expect("seed document share")
}

// ---------------------------------------------------------------------------
// documents/core.rs — POST /api/v1/documents
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_document_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "doc-create-ok").await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .post("/api/v1/documents")
                .json(json!({
                    "title": "Contract 2026",
                    "category": "contracts",
                    "file_key": format!("{org_id}/contracts/2026.pdf"),
                    "file_name": "2026.pdf",
                    "mime_type": "application/pdf",
                    "size_bytes": 4096
                }))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::CREATED, "body: {}", resp.text());
    let body = resp.json_value();
    assert!(body.get("id").is_some(), "response must include id");
}

// ---------------------------------------------------------------------------
// documents/core.rs — GET /api/v1/documents
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_documents_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "doc-list-ok").await;
    let user_id = user_id_for(&pool, &user.email).await;
    let _ = seed_document(&pool, org_id, user_id).await;

    let resp = app
        .execute(app.session(token, org_id).get("/api/v1/documents").build())
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    let body = resp.json_value();
    assert!(
        body.get("documents").is_some(),
        "response must include documents key"
    );
}

// ---------------------------------------------------------------------------
// documents/core.rs — PUT /api/v1/documents/{id}
// ---------------------------------------------------------------------------

#[ignore = "BIT-351 quarantine: schema/route not implemented (BIT-571)"]
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_document_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "doc-update-ok").await;
    let user_id = user_id_for(&pool, &user.email).await;
    let doc_id = seed_document(&pool, org_id, user_id).await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .put(&format!("/api/v1/documents/{doc_id}"))
                .json(json!({ "title": "Updated Title" }))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
}

// ---------------------------------------------------------------------------
// documents/core.rs — DELETE /api/v1/documents/{id}
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn delete_document_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "doc-delete-ok").await;
    let user_id = user_id_for(&pool, &user.email).await;
    let doc_id = seed_document(&pool, org_id, user_id).await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .delete(&format!("/api/v1/documents/{doc_id}"))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::NO_CONTENT, "body: {}", resp.text());

    let exists: bool = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM documents WHERE id = $1 AND deleted_at IS NULL)",
    )
    .bind(doc_id)
    .fetch_one(&pool)
    .await
    .expect("check deleted");
    assert!(!exists, "document must be soft-deleted");
}

// ---------------------------------------------------------------------------
// documents/core.rs — PUT /api/v1/documents/{id}/access
// ---------------------------------------------------------------------------

#[ignore = "BIT-351 quarantine: schema/route not implemented (BIT-571)"]
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_document_access_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "doc-access-ok").await;
    let user_id = user_id_for(&pool, &user.email).await;
    let doc_id = seed_document(&pool, org_id, user_id).await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .put(&format!("/api/v1/documents/{doc_id}/access"))
                .json(json!({ "access_scope": "organization" }))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
}

// ---------------------------------------------------------------------------
// documents/versions.rs — GET /api/v1/documents/{id}/versions
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_version_history_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "doc-ver-hist-ok").await;
    let user_id = user_id_for(&pool, &user.email).await;
    let doc_id = seed_document(&pool, org_id, user_id).await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .get(&format!("/api/v1/documents/{doc_id}/versions"))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    let body = resp.json_value();
    assert!(
        body.get("history").is_some(),
        "response must include history key"
    );
}

// ---------------------------------------------------------------------------
// documents/versions.rs — POST /api/v1/documents/{id}/versions
// ---------------------------------------------------------------------------

#[ignore = "BIT-351 quarantine: schema/route not implemented (BIT-571)"]
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_version_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) =
        create_authenticated_user_with_org(&app, &user, "doc-ver-create-ok").await;
    let user_id = user_id_for(&pool, &user.email).await;
    let doc_id = seed_document(&pool, org_id, user_id).await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .post(&format!("/api/v1/documents/{doc_id}/versions"))
                .json(json!({
                    "file_key": format!("{org_id}/docs/v2.pdf"),
                    "file_name": "v2.pdf",
                    "mime_type": "application/pdf",
                    "size_bytes": 8192
                }))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::CREATED, "body: {}", resp.text());
    let body = resp.json_value();
    assert!(body.get("id").is_some(), "response must include version id");
}

// ---------------------------------------------------------------------------
// documents/versions.rs — GET /api/v1/documents/{id}/versions/{version_id}
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_version_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "doc-ver-get-ok").await;
    let user_id = user_id_for(&pool, &user.email).await;
    let doc_id = seed_document(&pool, org_id, user_id).await;
    let version_id = seed_document_version(&pool, doc_id, org_id, user_id).await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .get(&format!("/api/v1/documents/{doc_id}/versions/{version_id}"))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    let body = resp.json_value();
    assert!(
        body.get("version").is_some(),
        "response must include version key"
    );
}

// ---------------------------------------------------------------------------
// documents/versions.rs — POST /api/v1/documents/{id}/versions/{version_id}/restore
// ---------------------------------------------------------------------------

#[ignore = "BIT-351 quarantine: schema/route not implemented (BIT-571)"]
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn restore_version_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) =
        create_authenticated_user_with_org(&app, &user, "doc-ver-restore-ok").await;
    let user_id = user_id_for(&pool, &user.email).await;
    let doc_id = seed_document(&pool, org_id, user_id).await;
    let version_id = seed_document_version(&pool, doc_id, org_id, user_id).await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .post(&format!(
                    "/api/v1/documents/{doc_id}/versions/{version_id}/restore"
                ))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::CREATED, "body: {}", resp.text());
}

// ---------------------------------------------------------------------------
// documents/shares.rs — GET /api/v1/documents/{id}/shares
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_shares_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) =
        create_authenticated_user_with_org(&app, &user, "doc-shares-list-ok").await;
    let user_id = user_id_for(&pool, &user.email).await;
    let doc_id = seed_document(&pool, org_id, user_id).await;
    let _ = seed_document_share(&pool, doc_id, user_id).await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .get(&format!("/api/v1/documents/{doc_id}/shares"))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    let body = resp.json_value();
    assert!(
        body.get("shares").is_some(),
        "response must include shares key"
    );
}

// ---------------------------------------------------------------------------
// documents/shares.rs — POST /api/v1/documents/{id}/shares
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_share_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) =
        create_authenticated_user_with_org(&app, &user, "doc-share-create-ok").await;
    let user_id = user_id_for(&pool, &user.email).await;
    let doc_id = seed_document(&pool, org_id, user_id).await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .post(&format!("/api/v1/documents/{doc_id}/shares"))
                .json(json!({ "share_type": "link" }))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::CREATED, "body: {}", resp.text());
    let body = resp.json_value();
    assert!(body.get("id").is_some(), "response must include share id");
}

// ---------------------------------------------------------------------------
// documents/shares.rs — DELETE /api/v1/documents/{id}/shares/{share_id}
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn revoke_share_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) =
        create_authenticated_user_with_org(&app, &user, "doc-share-revoke-ok").await;
    let user_id = user_id_for(&pool, &user.email).await;
    let doc_id = seed_document(&pool, org_id, user_id).await;
    let share_id = seed_document_share(&pool, doc_id, user_id).await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .delete(&format!("/api/v1/documents/{doc_id}/shares/{share_id}"))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::NO_CONTENT, "body: {}", resp.text());

    // revoke_share soft-deletes by setting revoked_at; the row still exists.
    let revoked: bool = sqlx::query_scalar::<_, bool>(
        "SELECT revoked_at IS NOT NULL FROM document_shares WHERE id = $1",
    )
    .bind(share_id)
    .fetch_one(&pool)
    .await
    .expect("check revoked_at");
    assert!(revoked, "share must have revoked_at set after revocation");
}

/// Regression (code-review-api-handlers-shares-revoke-doc-mismatch): a share
/// may only be revoked through the document it actually belongs to.
///
/// `revoke_share` authorizes the caller against the *path* document `id`
/// (creator-or-manager), but previously looked up and revoked the share purely
/// by `share_id` with no check that the share belonged to that document. RLS on
/// `document_shares` only scopes to the tenant, so a non-manager who created
/// document A could revoke a share of a *different* creator's document B in the
/// same org by passing A's id (to satisfy the creator check) plus B's share_id
/// — a cross-document IDOR. The handler now guards `share.document_id == id`
/// and returns 404 otherwise.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn revoke_share_rejects_cross_document_share(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    // Attacker: a NON-manager (resident) member of the org who owns document A.
    // A resident is not a manager, so it must rely on the created_by branch of
    // the authorization check — exactly the branch the IDOR abused.
    let attacker = TestUser::new();
    let (token, _refresh) = create_authenticated_user(&app, &attacker).await;
    let attacker_id = user_id_for(&pool, &attacker.email).await;
    let org_id = seed_org(&pool, "doc-share-revoke-idor").await;
    seed_membership(&pool, org_id, attacker_id, "resident").await;
    let doc_a = seed_document(&pool, org_id, attacker_id).await;

    // Victim: a different creator's document B in the SAME org, carrying a share.
    let victim = TestUser::new();
    let (_vtoken, _vrefresh) = create_authenticated_user(&app, &victim).await;
    let victim_id = user_id_for(&pool, &victim.email).await;
    let doc_b = seed_document(&pool, org_id, victim_id).await;
    let share_b = seed_document_share(&pool, doc_b, victim_id).await;

    // Attacker passes document A's id (passes the creator check) but document
    // B's share_id — the cross-document revoke must be rejected with 404.
    let resp = app
        .execute(
            app.session(token, org_id)
                .delete(&format!("/api/v1/documents/{doc_a}/shares/{share_b}"))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::NOT_FOUND, "body: {}", resp.text());

    // Document B's share must remain active (revoked_at still NULL).
    let revoked: bool = sqlx::query_scalar::<_, bool>(
        "SELECT revoked_at IS NOT NULL FROM document_shares WHERE id = $1",
    )
    .bind(share_b)
    .fetch_one(&pool)
    .await
    .expect("check revoked_at");
    assert!(
        !revoked,
        "cross-document revoke must NOT have revoked document B's share"
    );
}
