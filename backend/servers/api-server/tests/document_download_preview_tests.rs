//! Integration tests for the authenticated document download / preview routes
//! (Story 7A.4):
//!
//! - `GET /api/v1/documents/{id}/download` — presigned attachment URL
//! - `GET /api/v1/documents/{id}/preview`  — presigned inline URL
//!
//! These pin the *backend security & routing* contract that the frontend
//! preview/download UI (AC-1 PDF preview, AC-2 image preview, AC-3 unsupported
//! → direct download) sits on top of. Story task 6.2 ("backend tests for
//! permission check on download") and 6.3 are covered here at the HTTP layer;
//! the in-memory `document_access_allowed` gate already has unit coverage in
//! `routes/documents/document_access_test.rs`, and SigV4 presigning has unit
//! coverage in `integrations::storage`.
//!
//! # Design note — why we assert 503 on the "happy" path
//!
//! `TestApp::new` leaves `storage_service = None` in `AppState` (no S3 in CI).
//! Both handlers reach the presigner only AFTER (a) auth, (b) the RLS document
//! lookup, and (c) the access gate have all passed; with no storage configured
//! that final step returns `503 STORAGE_NOT_CONFIGURED`. So a 503 here is a
//! positive signal: it proves the request cleared auth + RLS + the access gate
//! and was about to presign. The denial paths (404 / 401 / 400) short-circuit
//! BEFORE storage, so they are asserted exactly. This keeps the suite
//! deterministic and DB-only — no live S3 dependency.

#[allow(dead_code)]
mod common;

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use sqlx::PgPool;
use uuid::Uuid;

use common::{
    cleanup_test_user, create_authenticated_user, seed_membership, seed_org, TestApp, TestUser,
};

// ============================================================================
// Seed helpers
// ============================================================================

async fn user_id_for(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(email)
        .fetch_one(pool)
        .await
        .expect("user_id_for")
}

/// Insert a `documents` row directly and return its id. `access_scope`,
/// `mime_type`, and `access_target_ids` are parameterised so a single helper
/// covers the previewable / non-previewable / scoped fixtures.
#[allow(clippy::too_many_arguments)]
async fn seed_document(
    pool: &PgPool,
    org_id: Uuid,
    created_by: Uuid,
    title: &str,
    mime_type: &str,
    file_name: &str,
    access_scope: &str,
    access_target_ids: serde_json::Value,
) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO documents \
             (organization_id, title, category, file_key, file_name, mime_type, \
              size_bytes, access_scope, access_target_ids, created_by) \
         VALUES ($1, $2, 'reports', $3, $4, $5, 1024, \
                 $6::document_access_scope, $7, $8) \
         RETURNING id",
    )
    .bind(org_id)
    .bind(title)
    .bind(format!("{org_id}/test/{}.bin", Uuid::new_v4()))
    .bind(file_name)
    .bind(mime_type)
    .bind(access_scope)
    .bind(access_target_ids)
    .bind(created_by)
    .fetch_one(pool)
    .await
    .expect("seed_document")
}

/// Build an authenticated GET request for the given path + tenant header.
fn get_request(path: &str, token: &str, tenant: Uuid) -> Request<Body> {
    Request::builder()
        .method(Method::GET)
        .uri(path)
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header("X-Tenant-ID", tenant.to_string())
        .body(Body::empty())
        .unwrap()
}

// ============================================================================
// T1 — Auth guard: unauthenticated download/preview → 401
// ============================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_download_requires_auth(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let request = Request::builder()
        .method(Method::GET)
        .uri(format!("/api/v1/documents/{}/download", Uuid::new_v4()))
        .body(Body::empty())
        .unwrap();
    let response = app.execute(request).await;
    assert_eq!(
        response.status,
        StatusCode::UNAUTHORIZED,
        "unauthenticated download must return 401"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_preview_requires_auth(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let request = Request::builder()
        .method(Method::GET)
        .uri(format!("/api/v1/documents/{}/preview", Uuid::new_v4()))
        .body(Body::empty())
        .unwrap();
    let response = app.execute(request).await;
    assert_eq!(
        response.status,
        StatusCode::UNAUTHORIZED,
        "unauthenticated preview must return 401"
    );
}

// ============================================================================
// T2 — Not found: authenticated, valid org, unknown document id → 404
// ============================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_download_unknown_document_is_404(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;
    let (token, _) = create_authenticated_user(&app, &user).await;
    let user_id = user_id_for(&pool, &user.email).await;
    let org_id = seed_org(&pool, &Uuid::new_v4().to_string()[..8]).await;
    seed_membership(&pool, org_id, user_id, "org_admin").await;

    let response = app
        .execute(get_request(
            &format!("/api/v1/documents/{}/download", Uuid::new_v4()),
            &token,
            org_id,
        ))
        .await;

    assert_eq!(
        response.status,
        StatusCode::NOT_FOUND,
        "download of a non-existent document must return 404, got {} body {}",
        response.status,
        response.text()
    );
    cleanup_test_user(&pool, &user.email).await;
}

// ============================================================================
// T3 — Cross-tenant RLS isolation: a manager of org B cannot download a
//      document that lives in org A. The RLS GUC scopes `find_by_id_rls`, so
//      the document is invisible → 404 (not 403, to avoid existence leak).
// ============================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_download_cross_tenant_document_is_404(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    // Org A + its creator user (the document owner).
    let owner = TestUser::new();
    cleanup_test_user(&pool, &owner.email).await;
    let (_owner_token, _) = create_authenticated_user(&app, &owner).await;
    let owner_id = user_id_for(&pool, &owner.email).await;
    let org_a = seed_org(&pool, &Uuid::new_v4().to_string()[..8]).await;
    seed_membership(&pool, org_a, owner_id, "org_admin").await;
    let doc_id = seed_document(
        &pool,
        org_a,
        owner_id,
        "Org A confidential",
        "application/pdf",
        "secret.pdf",
        "organization",
        serde_json::json!([]),
    )
    .await;

    // Org B + an unrelated manager who is NOT a member of org A.
    let intruder = TestUser::new();
    cleanup_test_user(&pool, &intruder.email).await;
    let (intruder_token, _) = create_authenticated_user(&app, &intruder).await;
    let intruder_id = user_id_for(&pool, &intruder.email).await;
    let org_b = seed_org(&pool, &Uuid::new_v4().to_string()[..8]).await;
    seed_membership(&pool, org_b, intruder_id, "org_admin").await;

    // Intruder asks for org A's document while scoped to org B.
    let response = app
        .execute(get_request(
            &format!("/api/v1/documents/{doc_id}/download"),
            &intruder_token,
            org_b,
        ))
        .await;

    assert_eq!(
        response.status,
        StatusCode::NOT_FOUND,
        "cross-tenant download must be hidden by RLS (404), got {} body {}",
        response.status,
        response.text()
    );

    cleanup_test_user(&pool, &owner.email).await;
    cleanup_test_user(&pool, &intruder.email).await;
}

// ============================================================================
// T4 — Access gate (non-manager): a `users`-scoped document NOT shared with
//      the requesting tenant is denied → 404. Guards the
//      `document_access_allowed` branch wired into the handler.
// ============================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_download_users_scoped_denies_unlisted_tenant(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    // Creator (manager) seeds a users-scoped doc shared with nobody.
    let creator = TestUser::new();
    cleanup_test_user(&pool, &creator.email).await;
    let (_ctok, _) = create_authenticated_user(&app, &creator).await;
    let creator_id = user_id_for(&pool, &creator.email).await;
    let org_id = seed_org(&pool, &Uuid::new_v4().to_string()[..8]).await;
    seed_membership(&pool, org_id, creator_id, "org_admin").await;
    let doc_id = seed_document(
        &pool,
        org_id,
        creator_id,
        "Shared with no one",
        "application/pdf",
        "private.pdf",
        "users",
        serde_json::json!([Uuid::new_v4().to_string()]), // some other user
    )
    .await;

    // A plain tenant in the same org — not the creator, not in the share list.
    let tenant = TestUser::new();
    cleanup_test_user(&pool, &tenant.email).await;
    let (tenant_token, _) = create_authenticated_user(&app, &tenant).await;
    let tenant_id = user_id_for(&pool, &tenant.email).await;
    seed_membership(&pool, org_id, tenant_id, "tenant").await;

    let response = app
        .execute(get_request(
            &format!("/api/v1/documents/{doc_id}/download"),
            &tenant_token,
            org_id,
        ))
        .await;

    assert_eq!(
        response.status,
        StatusCode::NOT_FOUND,
        "users-scoped doc must be denied (404) to an unlisted tenant, got {} body {}",
        response.status,
        response.text()
    );

    cleanup_test_user(&pool, &creator.email).await;
    cleanup_test_user(&pool, &tenant.email).await;
}

// ============================================================================
// T5 — Preview of an unsupported type → 400 PREVIEW_NOT_SUPPORTED.
//      This is the backend half of AC-3 (unsupported types fall back to direct
//      download). The 400 short-circuits BEFORE storage, so it is deterministic
//      even with no S3 configured.
// ============================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_preview_unsupported_type_returns_400(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;
    let (token, _) = create_authenticated_user(&app, &user).await;
    let user_id = user_id_for(&pool, &user.email).await;
    let org_id = seed_org(&pool, &Uuid::new_v4().to_string()[..8]).await;
    seed_membership(&pool, org_id, user_id, "org_admin").await;

    // text/plain is NOT in Document::supports_preview's allow-list.
    let doc_id = seed_document(
        &pool,
        org_id,
        user_id,
        "Plain notes",
        "text/plain",
        "notes.txt",
        "organization",
        serde_json::json!([]),
    )
    .await;

    let response = app
        .execute(get_request(
            &format!("/api/v1/documents/{doc_id}/preview"),
            &token,
            org_id,
        ))
        .await;

    assert_eq!(
        response.status,
        StatusCode::BAD_REQUEST,
        "preview of an unsupported type must return 400, got {} body {}",
        response.status,
        response.text()
    );
    assert_eq!(
        response.json_value()["code"].as_str(),
        Some("PREVIEW_NOT_SUPPORTED"),
        "error code must be PREVIEW_NOT_SUPPORTED, body {}",
        response.text()
    );

    cleanup_test_user(&pool, &user.email).await;
}

// ============================================================================
// T6 — Happy path clears the gate: an accessible, previewable document reaches
//      the presigner. With no S3 configured in test, that surfaces as
//      503 STORAGE_NOT_CONFIGURED — proving auth + RLS lookup + access gate
//      (+ supports_preview for preview) all passed. Asserts both routes.
// ============================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_download_accessible_document_reaches_presigner(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;
    let (token, _) = create_authenticated_user(&app, &user).await;
    let user_id = user_id_for(&pool, &user.email).await;
    let org_id = seed_org(&pool, &Uuid::new_v4().to_string()[..8]).await;
    seed_membership(&pool, org_id, user_id, "org_admin").await;
    let doc_id = seed_document(
        &pool,
        org_id,
        user_id,
        "Readable report",
        "application/pdf",
        "report.pdf",
        "organization",
        serde_json::json!([]),
    )
    .await;

    let response = app
        .execute(get_request(
            &format!("/api/v1/documents/{doc_id}/download"),
            &token,
            org_id,
        ))
        .await;

    assert_eq!(
        response.status,
        StatusCode::SERVICE_UNAVAILABLE,
        "accessible download with no S3 configured must reach the presigner and \
         return 503 (gate passed), got {} body {}",
        response.status,
        response.text()
    );
    assert_eq!(
        response.json_value()["code"].as_str(),
        Some("STORAGE_NOT_CONFIGURED"),
        "error code must be STORAGE_NOT_CONFIGURED, body {}",
        response.text()
    );

    cleanup_test_user(&pool, &user.email).await;
}

// ============================================================================
// T7 — Access-gate ALLOW path (non-manager): a `users`-scoped document shared
//      WITH the requesting tenant must clear the gate and reach the presigner
//      → 503. This is the positive mirror of T4 (which asserts the deny side),
//      proving the `document_access_allowed` "users" branch grants — not just
//      denies — at the HTTP layer for a non-manager caller. (#1377)
// ============================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_download_users_scoped_allows_listed_tenant(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    // Creator (manager) seeds a users-scoped doc shared with `tenant` below.
    let creator = TestUser::new();
    cleanup_test_user(&pool, &creator.email).await;
    let (_ctok, _) = create_authenticated_user(&app, &creator).await;
    let creator_id = user_id_for(&pool, &creator.email).await;
    let org_id = seed_org(&pool, &Uuid::new_v4().to_string()[..8]).await;
    seed_membership(&pool, org_id, creator_id, "org_admin").await;

    // The tenant who WILL be on the share list.
    let tenant = TestUser::new();
    cleanup_test_user(&pool, &tenant.email).await;
    let (tenant_token, _) = create_authenticated_user(&app, &tenant).await;
    let tenant_id = user_id_for(&pool, &tenant.email).await;
    seed_membership(&pool, org_id, tenant_id, "tenant").await;

    let doc_id = seed_document(
        &pool,
        org_id,
        creator_id,
        "Shared with the tenant",
        "application/pdf",
        "shared.pdf",
        "users",
        serde_json::json!([tenant_id.to_string()]), // tenant IS listed
    )
    .await;

    let response = app
        .execute(get_request(
            &format!("/api/v1/documents/{doc_id}/download"),
            &tenant_token,
            org_id,
        ))
        .await;

    assert_eq!(
        response.status,
        StatusCode::SERVICE_UNAVAILABLE,
        "users-scoped doc shared with the tenant must clear the gate and reach \
         the presigner (503), got {} body {}",
        response.status,
        response.text()
    );
    assert_eq!(
        response.json_value()["code"].as_str(),
        Some("STORAGE_NOT_CONFIGURED"),
        "gate-passed path must reach the presigner, body {}",
        response.text()
    );

    cleanup_test_user(&pool, &creator.email).await;
    cleanup_test_user(&pool, &tenant.email).await;
}

// ============================================================================
// T8 — Access-gate ALLOW path (non-manager creator): the creator of a document
//      keeps access even when the share scope would not otherwise grant it.
//      A `tenant`-role user who authored the doc must clear the gate → 503.
//      Pins the "creator-always-allowed wins over scope" branch end-to-end. (#1377)
// ============================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_preview_non_manager_creator_reaches_presigner(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    // A non-manager (tenant) who authors their own previewable document.
    let author = TestUser::new();
    cleanup_test_user(&pool, &author.email).await;
    let (author_token, _) = create_authenticated_user(&app, &author).await;
    let author_id = user_id_for(&pool, &author.email).await;
    let org_id = seed_org(&pool, &Uuid::new_v4().to_string()[..8]).await;
    seed_membership(&pool, org_id, author_id, "tenant").await;

    // users-scoped, shared with nobody but the author themselves authored it.
    let doc_id = seed_document(
        &pool,
        org_id,
        author_id,
        "My own upload",
        "image/png",
        "mine.png",
        "users",
        serde_json::json!([Uuid::new_v4().to_string()]), // someone else, not the author
    )
    .await;

    let response = app
        .execute(get_request(
            &format!("/api/v1/documents/{doc_id}/preview"),
            &author_token,
            org_id,
        ))
        .await;

    assert_eq!(
        response.status,
        StatusCode::SERVICE_UNAVAILABLE,
        "the document creator (even as a non-manager) must clear the gate and \
         reach the presigner (503), got {} body {}",
        response.status,
        response.text()
    );
    assert_eq!(
        response.json_value()["code"].as_str(),
        Some("STORAGE_NOT_CONFIGURED"),
        "creator path must reach the presigner, body {}",
        response.text()
    );

    cleanup_test_user(&pool, &author.email).await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_preview_accessible_document_reaches_presigner(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;
    let (token, _) = create_authenticated_user(&app, &user).await;
    let user_id = user_id_for(&pool, &user.email).await;
    let org_id = seed_org(&pool, &Uuid::new_v4().to_string()[..8]).await;
    seed_membership(&pool, org_id, user_id, "org_admin").await;
    let doc_id = seed_document(
        &pool,
        org_id,
        user_id,
        "Previewable image",
        "image/png",
        "photo.png",
        "organization",
        serde_json::json!([]),
    )
    .await;

    let response = app
        .execute(get_request(
            &format!("/api/v1/documents/{doc_id}/preview"),
            &token,
            org_id,
        ))
        .await;

    assert_eq!(
        response.status,
        StatusCode::SERVICE_UNAVAILABLE,
        "accessible preview of a supported type with no S3 must reach the \
         presigner and return 503 (gate + supports_preview passed), got {} body {}",
        response.status,
        response.text()
    );
    assert_eq!(
        response.json_value()["code"].as_str(),
        Some("STORAGE_NOT_CONFIGURED"),
        "error code must be STORAGE_NOT_CONFIGURED, body {}",
        response.text()
    );

    cleanup_test_user(&pool, &user.email).await;
}
