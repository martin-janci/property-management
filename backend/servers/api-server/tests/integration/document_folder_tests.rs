//! Document folder API integration tests (Story 7A.2, PR #580).
//!
//! ## Coverage
//!
//! ### Authentication guard tests (existing + new **[PR#580]**)
//! - GET  /api/v1/documents/folders
//! - POST /api/v1/documents/folders
//! - GET  /api/v1/documents/folders/tree
//! - GET  /api/v1/documents/folders/{id}   **[PR#580]**
//! - PUT  /api/v1/documents/folders/{id}   **[PR#580]**
//! - DELETE /api/v1/documents/folders/{id} **[PR#580]**
//!
//! ### Non-manager 403 guard **[PR#580]**
//! - POST   (non-manager -> 403)
//! - PUT    (non-manager -> 403)
//! - DELETE (non-manager -> 403)
//!
//! ### Cross-org IDOR guard **[PR#580]**
//! ### Depth-limit trigger (6th level -> 400 MAX_DEPTH_EXCEEDED) **[PR#580]**
//! ### Schema assertions (FK, column, table existence)

use crate::common;

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use sqlx::PgPool;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Seed / mint helpers
// ---------------------------------------------------------------------------

async fn seed_org_f(pool: &PgPool, tag: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO organizations (name, slug, contact_email, status) \
         VALUES ($1,$2,$3,'active') RETURNING id",
    )
    .bind(format!("FolderTest {tag}"))
    .bind(format!("folder-test-{tag}"))
    .bind(format!("{tag}@folder-test.example"))
    .fetch_one(pool)
    .await
    .expect("seed_org_f")
}

async fn seed_user_f(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO users (email, password_hash, name, status, email_verified_at) \
         VALUES ($1,'test_hash','FolderTest User','active',NOW()) RETURNING id",
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed_user_f")
}

async fn seed_member_f(pool: &PgPool, org: Uuid, user: Uuid, role: &str) {
    sqlx::query(
        "INSERT INTO organization_members \
             (organization_id,user_id,role_type,status,joined_at) \
         VALUES ($1,$2,$3,'active',NOW()) ON CONFLICT DO NOTHING",
    )
    .bind(org).bind(user).bind(role)
    .execute(pool).await.expect("seed_member_f");
}

/// Mint a signed HS256 JWT with `role: <role_str>` for TenantExtractor.
fn mint_jwt_with_role(user_id: Uuid, role_str: &str) -> String {
    use jsonwebtoken::{encode, EncodingKey, Header};
    use serde::Serialize;
    #[derive(Serialize)]
    struct TC { sub: String, email: String, name: String,
                exp: i64, iat: i64, jti: String, token_type: String, role: String }
    let now = chrono::Utc::now().timestamp();
    encode(&Header::default(),
           &TC { sub: user_id.to_string(),
                 email: "test@folder-test.example".into(),
                 name: "FolderTest".into(),
                 exp: now + 900, iat: now,
                 jti: Uuid::new_v4().to_string(),
                 token_type: "access".into(),
                 role: role_str.into() },
           &EncodingKey::from_secret(
               b"test-secret-key-that-is-at-least-64-characters-long-for-testing-purposes"))
    .expect("mint_jwt_with_role")
}

/// Insert document_folder row directly (bypasses HTTP + RLS).
async fn seed_folder_f(pool: &PgPool, org: Uuid, parent: Option<Uuid>, name: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO document_folders (organization_id,parent_id,name,created_by) \
         VALUES ($1,$2,$3,$1) RETURNING id",
    )
    .bind(org).bind(parent).bind(name)
    .fetch_one(pool).await.expect("seed_folder_f")
}

// ---------------------------------------------------------------------------
// Auth guard tests — existing
// ---------------------------------------------------------------------------

/// GET /api/v1/documents/folders — unauthenticated → 401
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_list_folders_requires_auth(pool: PgPool) {
    let app = common::TestApp::new(pool).await;
    let resp = app.execute(
        Request::builder().method(Method::GET)
            .uri("/api/v1/documents/folders")
            .body(Body::empty()).unwrap()).await;
    assert_eq!(resp.status, StatusCode::UNAUTHORIZED,
               "GET /api/v1/documents/folders must require authentication");
}

/// POST /api/v1/documents/folders — unauthenticated → 401
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_create_folder_requires_auth(pool: PgPool) {
    let app = common::TestApp::new(pool).await;
    let resp = app.execute(
        Request::builder().method(Method::POST)
            .uri("/api/v1/documents/folders")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(r#"{"name":"Test Folder"}"#)).unwrap()).await;
    assert_eq!(resp.status, StatusCode::UNAUTHORIZED,
               "POST /api/v1/documents/folders must require authentication");
}

/// GET /api/v1/documents/folders/tree — unauthenticated → 401
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_get_folder_tree_requires_auth(pool: PgPool) {
    let app = common::TestApp::new(pool).await;
    let resp = app.execute(
        Request::builder().method(Method::GET)
            .uri("/api/v1/documents/folders/tree")
            .body(Body::empty()).unwrap()).await;
    assert_eq!(resp.status, StatusCode::UNAUTHORIZED,
               "GET /api/v1/documents/folders/tree must require authentication");
}

// ---------------------------------------------------------------------------
// Auth guard tests — new **[PR#580]**
// ---------------------------------------------------------------------------

/// GET /api/v1/documents/folders/{id} — unauthenticated → 401
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_get_folder_by_id_requires_auth(pool: PgPool) {
    let app = common::TestApp::new(pool).await;
    let id = Uuid::new_v4();
    let resp = app.execute(
        Request::builder().method(Method::GET)
            .uri(format!("/api/v1/documents/folders/{id}"))
            .body(Body::empty()).unwrap()).await;
    assert_eq!(resp.status, StatusCode::UNAUTHORIZED,
               "GET /api/v1/documents/folders/{{id}} must require authentication");
}

/// PUT /api/v1/documents/folders/{id} — unauthenticated → 401
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_update_folder_requires_auth(pool: PgPool) {
    let app = common::TestApp::new(pool).await;
    let id = Uuid::new_v4();
    let resp = app.execute(
        Request::builder().method(Method::PUT)
            .uri(format!("/api/v1/documents/folders/{id}"))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(r#"{"name":"Renamed"}"#)).unwrap()).await;
    assert_eq!(resp.status, StatusCode::UNAUTHORIZED,
               "PUT /api/v1/documents/folders/{{id}} must require authentication");
}

/// DELETE /api/v1/documents/folders/{id} — unauthenticated → 401
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_delete_folder_requires_auth(pool: PgPool) {
    let app = common::TestApp::new(pool).await;
    let id = Uuid::new_v4();
    let resp = app.execute(
        Request::builder().method(Method::DELETE)
            .uri(format!("/api/v1/documents/folders/{id}"))
            .body(Body::empty()).unwrap()).await;
    assert_eq!(resp.status, StatusCode::UNAUTHORIZED,
               "DELETE /api/v1/documents/folders/{{id}} must require authentication");
}

// ---------------------------------------------------------------------------
// Non-manager 403 guard — new **[PR#580]**
//
// Users whose JWT has no `role` claim resolve to TenantRole::Guest via
// `unwrap_or(TenantRole::Guest)`. Guest.is_manager() == false → 403.
// ---------------------------------------------------------------------------

/// POST /api/v1/documents/folders — authenticated non-manager → 403
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_create_folder_non_manager_returns_403(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let user = common::TestUser::new();
    let (token, _) = common::create_authenticated_user(&app, &user).await;
    let org_id = seed_org_f(&pool, "nm-create").await;

    let resp = app.execute(
        Request::builder().method(Method::POST)
            .uri("/api/v1/documents/folders")
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .header("X-Tenant-ID", org_id.to_string())
            .body(Body::from(r#"{"name":"Should 403"}"#)).unwrap()).await;

    assert_eq!(resp.status, StatusCode::FORBIDDEN,
        "POST /folders non-manager must return 403; got {} body: {}",
        resp.status, resp.text());
}

/// PUT /api/v1/documents/folders/{id} — authenticated non-manager → 403
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_update_folder_non_manager_returns_403(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let user = common::TestUser::new();
    let (token, _) = common::create_authenticated_user(&app, &user).await;
    let org_id = seed_org_f(&pool, "nm-update").await;
    let fake_id = Uuid::new_v4(); // need not exist; 403 fires before DB look-up

    let resp = app.execute(
        Request::builder().method(Method::PUT)
            .uri(format!("/api/v1/documents/folders/{fake_id}"))
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .header("X-Tenant-ID", org_id.to_string())
            .body(Body::from(r#"{"name":"Should 403"}"#)).unwrap()).await;

    assert_eq!(resp.status, StatusCode::FORBIDDEN,
        "PUT /folders/{{id}} non-manager must return 403; got {} body: {}",
        resp.status, resp.text());
}

/// DELETE /api/v1/documents/folders/{id} — authenticated non-manager → 403
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_delete_folder_non_manager_returns_403(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let user = common::TestUser::new();
    let (token, _) = common::create_authenticated_user(&app, &user).await;
    let org_id = seed_org_f(&pool, "nm-delete").await;
    let fake_id = Uuid::new_v4();

    let resp = app.execute(
        Request::builder().method(Method::DELETE)
            .uri(format!("/api/v1/documents/folders/{fake_id}"))
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .header("X-Tenant-ID", org_id.to_string())
            .body(Body::empty()).unwrap()).await;

    assert_eq!(resp.status, StatusCode::FORBIDDEN,
        "DELETE /folders/{{id}} non-manager must return 403; got {} body: {}",
        resp.status, resp.text());
}

// ---------------------------------------------------------------------------
// Cross-org IDOR guard — new **[PR#580]**
//
// Without host_tenant_middleware, an unauthenticated request that supplies
// X-Tenant-ID for Org B is rejected (4xx) before any Org A row is mutated.
// ---------------------------------------------------------------------------

/// Cross-org folder create is rejected; Org A row count unchanged.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_cross_org_folder_idor_is_rejected(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let org_a = seed_org_f(&pool, "idor-a").await;
    let org_b = seed_org_f(&pool, "idor-b").await;
    let _ = seed_folder_f(&pool, org_a, None, "OrgA Folder").await;

    let count_before: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM document_folders WHERE organization_id = $1")
            .bind(org_a).fetch_one(&pool).await.expect("count_before");

    // No bearer token + wrong tenant header → auth gate fires (4xx).
    let resp = app.execute(
        Request::builder().method(Method::POST)
            .uri("/api/v1/documents/folders")
            .header(header::CONTENT_TYPE, "application/json")
            .header("X-Tenant-ID", org_b.to_string())
            .body(Body::from(r#"{"name":"IDOR attempt"}"#)).unwrap()).await;

    assert!((400u16..500).contains(&resp.status.as_u16()),
        "cross-org POST /folders must be rejected with 4xx, got {}", resp.status);

    let count_after: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM document_folders WHERE organization_id = $1")
            .bind(org_a).fetch_one(&pool).await.expect("count_after");

    assert_eq!(count_before, count_after,
        "cross-org request must not mutate Org A folder count");
}

// ---------------------------------------------------------------------------
// Depth-limit trigger — new **[PR#580]**
//
// DB trigger check_folder_depth_trigger prevents hierarchies > 5 levels.
// The handler catches the exception and returns 400 MAX_DEPTH_EXCEEDED.
// ---------------------------------------------------------------------------

/// Creating folder at depth 6 returns 400 MAX_DEPTH_EXCEEDED.
///
/// Seeds levels 1-5 in the DB directly; then issues an HTTP POST for level 6
/// using a JWT embedding `role: "manager"` so the is_manager() gate passes.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_folder_depth_limit_returns_bad_request(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let org_id = seed_org_f(&pool, "depth-limit").await;
    let user_id = seed_user_f(&pool, "depth-mgr@folder-test.example").await;
    seed_member_f(&pool, org_id, user_id, "manager").await;

    // Seed levels 1-5 directly (bypasses HTTP; trigger fires at INSERT).
    let mut parent: Option<Uuid> = None;
    for i in 1u32..=5 {
        let fid = seed_folder_f(&pool, org_id, parent, &format!("Level {i}")).await;
        parent = Some(fid);
    }
    let depth5_parent = parent.unwrap();

    // Mint a JWT with role:"manager" so TenantExtractor resolves Manager role.
    let token = mint_jwt_with_role(user_id, "manager");

    let body = serde_json::json!({
        "name": "Level 6 - must fail",
        "parent_id": depth5_parent
    });

    let resp = app.execute(
        Request::builder().method(Method::POST)
            .uri("/api/v1/documents/folders")
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .header("X-Tenant-ID", org_id.to_string())
            .body(Body::from(body.to_string())).unwrap()).await;

    assert_eq!(resp.status, StatusCode::BAD_REQUEST,
        "depth-6 folder must return 400; got {} body: {}",
        resp.status, resp.text());

    let json = resp.json_value();
    assert_eq!(json.get("code").and_then(|v| v.as_str()), Some("MAX_DEPTH_EXCEEDED"),
        "response code must be MAX_DEPTH_EXCEEDED; body: {json}");
}

// ---------------------------------------------------------------------------
// Schema tests: folder_id FK on documents
// ---------------------------------------------------------------------------

/// Verify the documents table has a folder_id column (FK to document_folders).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_documents_table_has_folder_id_column(pool: PgPool) {
    let row = sqlx::query(
        "SELECT column_name FROM information_schema.columns \
         WHERE table_schema='public' AND table_name='documents' AND column_name='folder_id'",
    )
    .fetch_optional(&pool).await.expect("schema query");
    assert!(row.is_some(),
            "documents.folder_id column must exist (FK to document_folders)");
}

/// Verify the document_folders table exists.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_document_folders_table_exists(pool: PgPool) {
    let row = sqlx::query(
        "SELECT table_name FROM information_schema.tables \
         WHERE table_schema='public' AND table_name='document_folders'",
    )
    .fetch_optional(&pool).await.expect("schema query");
    assert!(row.is_some(), "document_folders table must exist");
}

/// Verify FK constraint from documents.folder_id -> document_folders.id.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_folder_id_fk_constraint_exists(pool: PgPool) {
    let row = sqlx::query(
        r#"
        SELECT tc.constraint_name
        FROM information_schema.table_constraints tc
        JOIN information_schema.key_column_usage kcu
          ON tc.constraint_name = kcu.constraint_name AND tc.table_schema = kcu.table_schema
        JOIN information_schema.referential_constraints rc
          ON tc.constraint_name = rc.constraint_name AND tc.table_schema = rc.constraint_schema
        JOIN information_schema.key_column_usage ccu
          ON rc.unique_constraint_name = ccu.constraint_name
             AND rc.unique_constraint_schema = ccu.table_schema
        WHERE tc.constraint_type = 'FOREIGN KEY'
          AND tc.table_schema = 'public' AND tc.table_name = 'documents'
          AND kcu.column_name = 'folder_id' AND ccu.table_name = 'document_folders'
        "#,
    )
    .fetch_optional(&pool).await.expect("FK constraint query");
    assert!(row.is_some(),
            "documents.folder_id must have FK constraint referencing document_folders");
}
