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
//! ### Cross-org IDOR guard — authenticated (RLS isolation -> 404) **[#679]**
//! - GET    /folders/{id} (org_b manager → org_a folder) → 404
//! - PUT    /folders/{id} (org_b manager → org_a folder) → 404, row unchanged
//! - DELETE /folders/{id} (org_b manager → org_a folder) → 404, row still exists
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
async fn seed_folder_f(pool: &PgPool, org: Uuid, parent: Option<Uuid>, name: &str, created_by: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO document_folders (organization_id,parent_id,name,created_by) \
         VALUES ($1,$2,$3,$4) RETURNING id",
    )
    .bind(org).bind(parent).bind(name).bind(created_by)
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
// Cross-org IDOR guard — unauthenticated (existing, **[PR#580]**)
//
// Without host_tenant_middleware, an unauthenticated request that supplies
// X-Tenant-ID for Org B is rejected (4xx) before any Org A row is mutated.
// This is the legitimate outer-gate test and is intentionally kept as-is —
// it proves the JWT gate (AuthUser) blocks anonymous callers. The authenticated
// IDOR contract is exercised by the `*_authenticated_*` tests below (#679).
// ---------------------------------------------------------------------------

/// Cross-org folder create without a Bearer token is rejected; Org A row count unchanged.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_cross_org_folder_idor_is_rejected(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let org_a = seed_org_f(&pool, "idor-a").await;
    let org_b = seed_org_f(&pool, "idor-b").await;
    let user_a = seed_user_f(&pool, "idor-user@folder-test.example").await;
    let _ = seed_folder_f(&pool, org_a, None, "OrgA Folder", user_a).await;

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
// Cross-org IDOR guard — AUTHENTICATED (new, **[issue #679]**)
//
// The existing unauth test above only proves the outer JWT gate works.
// The real IDOR scenario is: a Manager authenticated in Org B sends a valid
// Bearer token + `X-Tenant-ID: org_b` but targets a folder UUID owned by
// Org A. The request passes `AuthUser`, passes `ValidatedTenantExtractor`
// (the user IS a member of org_b), reaches `RlsConnection` which sets the
// RLS context to org_b, and the `find_folder_by_id_rls` lookup for org_a's
// folder returns `None` because the RLS policy on `document_folders` hides
// rows owned by other orgs. The handler maps `None → 404 NOT_FOUND`.
//
// For PUT/DELETE the `tenant.role.is_manager()` RBAC check fires BEFORE the
// repo lookup — to reach the RLS-isolation branch we must mint a JWT with
// `role: "manager"` so the RBAC gate passes. (Same trick as the depth-limit
// test below.) For GET there is no RBAC check; any authenticated member of
// org_b sees the same 404.
//
// We also seed an `organization_members` row in org_b so
// `ValidatedTenantExtractor` (used by `RlsConnection`) accepts the user.
// ---------------------------------------------------------------------------

/// Authenticated Manager in Org B GETs a folder owned by Org A → 404
/// (RLS hides cross-org rows; not 401, not 403).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_cross_org_folder_idor_authenticated_get_returns_404(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let org_a = seed_org_f(&pool, "idor-auth-get-a").await;
    let org_b = seed_org_f(&pool, "idor-auth-get-b").await;

    // Seed an Org A user + folder (the IDOR target).
    let user_a = seed_user_f(&pool, "owner-get@folder-test.example").await;
    let folder_in_a = seed_folder_f(&pool, org_a, None, "OrgA Folder", user_a).await;

    // Attacker: a real user with a Manager membership in Org B.
    let attacker = seed_user_f(&pool, "attacker-get@folder-test.example").await;
    seed_member_f(&pool, org_b, attacker, "manager").await;
    let token = mint_jwt_with_role(attacker, "manager");

    let resp = app.execute(
        Request::builder().method(Method::GET)
            .uri(format!("/api/v1/documents/folders/{folder_in_a}"))
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .header("X-Tenant-ID", org_b.to_string())
            .body(Body::empty()).unwrap()).await;

    assert_eq!(resp.status, StatusCode::NOT_FOUND,
        "#679: authenticated cross-org GET must hit RLS isolation and return 404, \
         got {} body: {}", resp.status, resp.text());

    // Folder still exists in Org A (bypass-RLS query via app pool).
    let still_there: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM document_folders WHERE id = $1 AND organization_id = $2")
        .bind(folder_in_a).bind(org_a)
        .fetch_one(&pool).await.expect("post-GET existence check");
    assert_eq!(still_there, 1, "Org A folder must remain untouched after cross-org GET");
}

/// Authenticated Manager in Org B PUTs (updates) a folder owned by Org A → 404
/// (RLS hides cross-org rows; row unchanged).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_cross_org_folder_idor_authenticated_update_returns_404(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let org_a = seed_org_f(&pool, "idor-auth-put-a").await;
    let org_b = seed_org_f(&pool, "idor-auth-put-b").await;

    let user_a = seed_user_f(&pool, "owner-put@folder-test.example").await;
    let folder_in_a = seed_folder_f(&pool, org_a, None, "OrgA Original", user_a).await;

    let attacker = seed_user_f(&pool, "attacker-put@folder-test.example").await;
    seed_member_f(&pool, org_b, attacker, "manager").await;
    let token = mint_jwt_with_role(attacker, "manager");

    let resp = app.execute(
        Request::builder().method(Method::PUT)
            .uri(format!("/api/v1/documents/folders/{folder_in_a}"))
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .header("X-Tenant-ID", org_b.to_string())
            .body(Body::from(r#"{"name":"Pwned by OrgB"}"#)).unwrap()).await;

    assert_eq!(resp.status, StatusCode::NOT_FOUND,
        "#679: authenticated cross-org PUT must hit RLS isolation and return 404 \
         (not 403 RBAC, not 401 auth), got {} body: {}", resp.status, resp.text());

    // Org A's folder name must be unchanged.
    let name_after: String = sqlx::query_scalar(
        "SELECT name FROM document_folders WHERE id = $1")
        .bind(folder_in_a)
        .fetch_one(&pool).await.expect("post-PUT name check");
    assert_eq!(name_after, "OrgA Original",
        "Org A folder name must not be mutated by a cross-org PUT");
}

/// Authenticated Manager in Org B DELETEs a folder owned by Org A → 404
/// (RLS hides cross-org rows; folder still exists).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_cross_org_folder_idor_authenticated_delete_returns_404(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let org_a = seed_org_f(&pool, "idor-auth-del-a").await;
    let org_b = seed_org_f(&pool, "idor-auth-del-b").await;

    let user_a = seed_user_f(&pool, "owner-del@folder-test.example").await;
    let folder_in_a = seed_folder_f(&pool, org_a, None, "OrgA Survivor", user_a).await;

    let attacker = seed_user_f(&pool, "attacker-del@folder-test.example").await;
    seed_member_f(&pool, org_b, attacker, "manager").await;
    let token = mint_jwt_with_role(attacker, "manager");

    let resp = app.execute(
        Request::builder().method(Method::DELETE)
            .uri(format!("/api/v1/documents/folders/{folder_in_a}"))
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .header("X-Tenant-ID", org_b.to_string())
            .body(Body::empty()).unwrap()).await;

    assert_eq!(resp.status, StatusCode::NOT_FOUND,
        "#679: authenticated cross-org DELETE must hit RLS isolation and return 404 \
         (not 403 RBAC, not 401 auth, not 204 success), got {} body: {}",
        resp.status, resp.text());

    // Folder must still exist in Org A (bypass-RLS query via app pool).
    let still_there: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM document_folders WHERE id = $1 AND organization_id = $2")
        .bind(folder_in_a).bind(org_a)
        .fetch_one(&pool).await.expect("post-DELETE existence check");
    assert_eq!(still_there, 1,
        "Org A folder must still exist after a cross-org DELETE attempt");
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
        let fid = seed_folder_f(&pool, org_id, parent, &format!("Level {i}"), user_id).await;
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
