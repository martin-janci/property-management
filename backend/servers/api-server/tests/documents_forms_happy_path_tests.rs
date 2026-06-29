//! BIT-358 Wave 7: documents-forms happy-path (2xx) backfill.
//!
//! Most of the documents/forms surface in
//! `docs/endpoint-checklist/groups/documents-forms.md` is already covered by
//! existing happy-path suites:
//! - core CRUD / versions / shares -> `documents_core_crud_tests.rs`
//! - intelligence + templates      -> `documents_intelligence_templates_tests.rs`
//! - folders (list/tree/get/create/move/delete) -> `document_folder_tests.rs`
//! - upload + create->read roundtrip -> `document_upload_tests.rs`
//! - download / preview presign      -> `document_download_preview_tests.rs`
//! - share public access             -> `document_share_access_tests.rs`
//! - ALL `/api/v1/forms` endpoints (crud/fields/types/submissions, publish,
//!   archive, statistics, available, reorder, review, download) ->
//!   `lease_abstraction_forms_tests.rs`
//! - same-org submit / cross-org IDOR -> `form_cross_org_idor_tests.rs`
//!
//! This file closes the two remaining documents-core endpoints that had NO
//! dedicated seed-based happy-path 2xx assertion of their own:
//! - GET  /api/v1/documents/{id}        (get_document)  -> 200, body.document
//! - POST /api/v1/documents/{id}/move   (move_document) -> 200, body.document
//!
//! Both mirror `documents_core_crud_tests.rs` exactly (same `mod common;`
//! helpers, `create_authenticated_user_with_org`, `#[sqlx::test]` migrator,
//! seed pattern, and request/assert shape) to minimise blind-CI compile risk.
//!
//! Intentionally NOT covered here (would need a live external provider or are
//! already exercised elsewhere — see headers above):
//! - POST /api/v1/documents/{id}/ai-summarize (ai_summarize_document): calls a
//!   live AI provider after fetching extracted text; cannot return 2xx in blind
//!   CI without that provider (same rationale as the intelligence suite skip).
//! - POST /api/v1/documents/upload: multipart file upload, covered by
//!   `document_upload_tests.rs`.

#![allow(dead_code)]

mod common;

use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use common::{create_authenticated_user_with_org, TestApp, TestUser};

// ---------------------------------------------------------------------------
// Seed helpers (mirrors documents_core_crud_tests.rs)
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

async fn seed_folder(pool: &PgPool, org_id: Uuid, user_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO document_folders (organization_id, parent_id, name, created_by) \
         VALUES ($1, NULL, 'Destination', $2) RETURNING id",
    )
    .bind(org_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("seed folder")
}

// ---------------------------------------------------------------------------
// documents/core.rs — GET /api/v1/documents/{id}
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_document_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "doc-get-ok").await;
    let user_id = user_id_for(&pool, &user.email).await;
    let doc_id = seed_document(&pool, org_id, user_id).await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .get(&format!("/api/v1/documents/{doc_id}"))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    let body = resp.json_value();
    assert!(
        body.get("document").is_some(),
        "response must include document key"
    );
}

// ---------------------------------------------------------------------------
// documents/core.rs — POST /api/v1/documents/{id}/move
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn move_document_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "doc-move-ok").await;
    let user_id = user_id_for(&pool, &user.email).await;
    // created_by == auth user, so the creator authorization branch passes.
    let doc_id = seed_document(&pool, org_id, user_id).await;
    let folder_id = seed_folder(&pool, org_id, user_id).await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .post(&format!("/api/v1/documents/{doc_id}/move"))
                .json(json!({ "folder_id": folder_id }))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    let body = resp.json_value();
    assert!(
        body.get("document").is_some(),
        "response must include document key"
    );

    // Confirm the move persisted.
    let moved_to: Option<Uuid> =
        sqlx::query_scalar::<_, Option<Uuid>>("SELECT folder_id FROM documents WHERE id = $1")
            .bind(doc_id)
            .fetch_one(&pool)
            .await
            .expect("fetch folder_id");
    assert_eq!(moved_to, Some(folder_id), "document must be in the folder");
}
