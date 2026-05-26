//! Document folder API integration tests (Story 7A.2).
//!
//! Verifies that POST /api/v1/documents/folders and
//! GET /api/v1/documents/folders are correctly wired and
//! require authentication. The `folder_id` FK on the
//! `documents` table is checked via schema assertions.
//!
//! Focuses on the route contract rather than full DB round-trips
//! to keep tests fast and free of sqlx::test DB setup.

use crate::common;

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use sqlx::PgPool;

// =============================================================================
// Authentication guard tests
// =============================================================================

/// GET /api/v1/documents/folders — unauthenticated → 401
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_list_folders_requires_auth(pool: PgPool) {
    let app = common::TestApp::new(pool).await;

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/documents/folders")
        .body(Body::empty())
        .unwrap();

    let response = app.execute(request).await;

    assert_eq!(
        response.status,
        StatusCode::UNAUTHORIZED,
        "GET /api/v1/documents/folders must require authentication"
    );
}

/// POST /api/v1/documents/folders — unauthenticated → 401
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_create_folder_requires_auth(pool: PgPool) {
    let app = common::TestApp::new(pool).await;

    let request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/documents/folders")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{"name":"Test Folder"}"#))
        .unwrap();

    let response = app.execute(request).await;

    assert_eq!(
        response.status,
        StatusCode::UNAUTHORIZED,
        "POST /api/v1/documents/folders must require authentication"
    );
}

/// GET /api/v1/documents/folders/tree — unauthenticated → 401
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_get_folder_tree_requires_auth(pool: PgPool) {
    let app = common::TestApp::new(pool).await;

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/documents/folders/tree")
        .body(Body::empty())
        .unwrap();

    let response = app.execute(request).await;

    assert_eq!(
        response.status,
        StatusCode::UNAUTHORIZED,
        "GET /api/v1/documents/folders/tree must require authentication"
    );
}

// =============================================================================
// Schema tests: folder_id FK on documents
// =============================================================================

/// Verify the documents table has a folder_id column (FK to document_folders).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_documents_table_has_folder_id_column(pool: PgPool) {
    let row = sqlx::query(
        r#"
        SELECT column_name, data_type, is_nullable
        FROM information_schema.columns
        WHERE table_schema = 'public'
          AND table_name   = 'documents'
          AND column_name  = 'folder_id'
        "#,
    )
    .fetch_optional(&pool)
    .await
    .expect("schema query failed");

    assert!(
        row.is_some(),
        "documents.folder_id column must exist (FK to document_folders)"
    );
}

/// Verify the document_folders table exists.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_document_folders_table_exists(pool: PgPool) {
    let row = sqlx::query(
        r#"
        SELECT table_name
        FROM information_schema.tables
        WHERE table_schema = 'public'
          AND table_name   = 'document_folders'
        "#,
    )
    .fetch_optional(&pool)
    .await
    .expect("schema query failed");

    assert!(
        row.is_some(),
        "document_folders table must exist"
    );
}

/// Verify foreign key constraint from documents.folder_id → document_folders.id.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_folder_id_fk_constraint_exists(pool: PgPool) {
    let row = sqlx::query(
        r#"
        SELECT tc.constraint_name
        FROM information_schema.table_constraints tc
        JOIN information_schema.key_column_usage kcu
          ON tc.constraint_name = kcu.constraint_name
          AND tc.table_schema   = kcu.table_schema
        JOIN information_schema.referential_constraints rc
          ON tc.constraint_name = rc.constraint_name
          AND tc.table_schema   = rc.constraint_schema
        JOIN information_schema.key_column_usage ccu
          ON rc.unique_constraint_name = ccu.constraint_name
          AND rc.unique_constraint_schema = ccu.table_schema
        WHERE tc.constraint_type = 'FOREIGN KEY'
          AND tc.table_schema    = 'public'
          AND tc.table_name      = 'documents'
          AND kcu.column_name    = 'folder_id'
          AND ccu.table_name     = 'document_folders'
        "#,
    )
    .fetch_optional(&pool)
    .await
    .expect("FK constraint query failed");

    assert!(
        row.is_some(),
        "documents.folder_id must have a FOREIGN KEY constraint referencing document_folders"
    );
}
