//! Document folder API integration tests (Story 7A.2, PR #580).
//!
//! NOTE: This binary lives at `tests/document_folder_tests.rs` (root level) so
//! that Cargo actually compiles and runs it. The previous copy under
//! `tests/integration/document_folder_tests.rs` was orphaned — nothing declared
//! `mod integration;`, so that whole subtree never built and these tests never
//! ran in CI. Moved here (story 7A.2 backend verification) following the same
//! `mod common;` convention every other root-level integration bin uses, and
//! extended with positive AC happy-path coverage (create / move / delete).
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
//! ### Positive AC happy-path coverage (create / move / delete / cycle-guard) **[7A.2 verify]**

#[allow(dead_code)]
mod common;

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
    // Embed a UUID fragment so slug/contact_email are globally unique within the
    // test run. Without this, parallel `#[sqlx::test]` workers that happen to
    // share the same Postgres instance (or a reused template DB) collide on the
    // organizations UNIQUE(slug) and UNIQUE(contact_email) constraints, causing
    // spurious test failures. Pattern mirrors common::seed_org.
    let uid = Uuid::new_v4();
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO organizations (name, slug, contact_email, status) \
         VALUES ($1,$2,$3,'active') RETURNING id",
    )
    .bind(format!("FolderTest {tag}"))
    .bind(format!("folder-test-{tag}-{uid}"))
    .bind(format!("{tag}-{uid}@folder-test.example"))
    .fetch_one(pool)
    .await
    .expect("seed_org_f")
}

/// Seed a user with a unique email derived from `tag` + a UUID fragment.
///
/// Using hardcoded email strings caused spurious UNIQUE-constraint panics when
/// the same test binary was re-run against a non-ephemeral Postgres (or when the
/// test database was not properly dropped after a failed run). UUID-suffixed
/// addresses guarantee per-invocation uniqueness regardless of teardown state.
///
/// `principal_kind` is set explicitly to `'staff'` to match the green sibling
/// integration tests (e.g. `admin_mfa_*_tests.rs`, `principal_platform_host_tests.rs`).
/// The `RequestPrincipal` extractor reads `users.principal_kind` to classify the
/// caller; agency-staff endpoints (document folders) require a `staff` principal,
/// so the seed must pin it rather than relying on the column default.
async fn seed_user_f(pool: &PgPool, tag: &str) -> Uuid {
    let uid = Uuid::new_v4();
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind) \
         VALUES ($1,'test_hash','FolderTest User','active',NOW(),'staff') RETURNING id",
    )
    .bind(format!("{tag}-{uid}@folder-test.example"))
    .fetch_one(pool)
    .await
    .expect("seed_user_f")
}

/// Mint a signed HS256 JWT with `role: <role_str>` for TenantExtractor.
fn mint_jwt_with_role(user_id: Uuid, role_str: &str) -> String {
    use jsonwebtoken::{encode, EncodingKey, Header};
    use serde::Serialize;
    #[derive(Serialize)]
    struct TC {
        sub: String,
        email: String,
        name: String,
        exp: i64,
        iat: i64,
        jti: String,
        token_type: String,
        role: String,
    }
    let now = chrono::Utc::now().timestamp();
    encode(
        &Header::default(),
        &TC {
            sub: user_id.to_string(),
            email: "test@folder-test.example".into(),
            name: "FolderTest".into(),
            exp: now + 900,
            iat: now,
            jti: Uuid::new_v4().to_string(),
            token_type: "access".into(),
            role: role_str.into(),
        },
        &EncodingKey::from_secret(
            b"test-secret-key-that-is-at-least-64-characters-long-for-testing-purposes",
        ),
    )
    .expect("mint_jwt_with_role")
}

/// Insert document_folder row directly (bypasses HTTP + RLS).
async fn seed_folder_f(
    pool: &PgPool,
    org: Uuid,
    parent: Option<Uuid>,
    name: &str,
    created_by: Uuid,
) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO document_folders (organization_id,parent_id,name,created_by) \
         VALUES ($1,$2,$3,$4) RETURNING id",
    )
    .bind(org)
    .bind(parent)
    .bind(name)
    .bind(created_by)
    .fetch_one(pool)
    .await
    .expect("seed_folder_f")
}

/// Insert a `documents` row directly (Story 7A.1 schema: title/file_key/…).
/// Bypasses HTTP + RLS; `folder` may be NULL (root) or a folder UUID.
async fn seed_document_f(
    pool: &PgPool,
    org: Uuid,
    folder: Option<Uuid>,
    title: &str,
    created_by: Uuid,
) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO documents \
             (organization_id, folder_id, title, file_key, file_name, mime_type, size_bytes, created_by) \
         VALUES ($1,$2,$3,$4,$5,'application/pdf',1024,$6) RETURNING id",
    )
    .bind(org).bind(folder).bind(title)
    .bind(format!("s3/{title}.pdf")).bind(format!("{title}.pdf"))
    .bind(created_by)
    .fetch_one(pool).await.expect("seed_document_f")
}

// ---------------------------------------------------------------------------
// Auth guard tests — existing
// ---------------------------------------------------------------------------

/// GET /api/v1/documents/folders — unauthenticated → 401
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_list_folders_requires_auth(pool: PgPool) {
    let app = common::TestApp::new(pool).await;
    let resp = app
        .execute(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/documents/folders")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::UNAUTHORIZED,
        "GET /api/v1/documents/folders must require authentication"
    );
}

/// POST /api/v1/documents/folders — unauthenticated → 401
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_create_folder_requires_auth(pool: PgPool) {
    let app = common::TestApp::new(pool).await;
    let resp = app
        .execute(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/documents/folders")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"name":"Test Folder"}"#))
                .unwrap(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::UNAUTHORIZED,
        "POST /api/v1/documents/folders must require authentication"
    );
}

/// GET /api/v1/documents/folders/tree — unauthenticated → 401
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_get_folder_tree_requires_auth(pool: PgPool) {
    let app = common::TestApp::new(pool).await;
    let resp = app
        .execute(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/documents/folders/tree")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::UNAUTHORIZED,
        "GET /api/v1/documents/folders/tree must require authentication"
    );
}

// ---------------------------------------------------------------------------
// Auth guard tests — new **[PR#580]**
// ---------------------------------------------------------------------------

/// GET /api/v1/documents/folders/{id} — unauthenticated → 401
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_get_folder_by_id_requires_auth(pool: PgPool) {
    let app = common::TestApp::new(pool).await;
    let id = Uuid::new_v4();
    let resp = app
        .execute(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/api/v1/documents/folders/{id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::UNAUTHORIZED,
        "GET /api/v1/documents/folders/{{id}} must require authentication"
    );
}

/// PUT /api/v1/documents/folders/{id} — unauthenticated → 401
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_update_folder_requires_auth(pool: PgPool) {
    let app = common::TestApp::new(pool).await;
    let id = Uuid::new_v4();
    let resp = app
        .execute(
            Request::builder()
                .method(Method::PUT)
                .uri(format!("/api/v1/documents/folders/{id}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"name":"Renamed"}"#))
                .unwrap(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::UNAUTHORIZED,
        "PUT /api/v1/documents/folders/{{id}} must require authentication"
    );
}

/// DELETE /api/v1/documents/folders/{id} — unauthenticated → 401
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_delete_folder_requires_auth(pool: PgPool) {
    let app = common::TestApp::new(pool).await;
    let id = Uuid::new_v4();
    let resp = app
        .execute(
            Request::builder()
                .method(Method::DELETE)
                .uri(format!("/api/v1/documents/folders/{id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::UNAUTHORIZED,
        "DELETE /api/v1/documents/folders/{{id}} must require authentication"
    );
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

    let resp = app
        .execute(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/documents/folders")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header("X-Tenant-ID", org_id.to_string())
                .body(Body::from(r#"{"name":"Should 403"}"#))
                .unwrap(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "POST /folders non-manager must return 403; got {} body: {}",
        resp.status,
        resp.text()
    );
}

/// PUT /api/v1/documents/folders/{id} — authenticated non-manager → 403
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_update_folder_non_manager_returns_403(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let user = common::TestUser::new();
    let (token, _) = common::create_authenticated_user(&app, &user).await;
    let org_id = seed_org_f(&pool, "nm-update").await;
    let fake_id = Uuid::new_v4(); // need not exist; 403 fires before DB look-up

    let resp = app
        .execute(
            Request::builder()
                .method(Method::PUT)
                .uri(format!("/api/v1/documents/folders/{fake_id}"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header("X-Tenant-ID", org_id.to_string())
                .body(Body::from(r#"{"name":"Should 403"}"#))
                .unwrap(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "PUT /folders/{{id}} non-manager must return 403; got {} body: {}",
        resp.status,
        resp.text()
    );
}

/// DELETE /api/v1/documents/folders/{id} — authenticated non-manager → 403
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_delete_folder_non_manager_returns_403(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let user = common::TestUser::new();
    let (token, _) = common::create_authenticated_user(&app, &user).await;
    let org_id = seed_org_f(&pool, "nm-delete").await;
    let fake_id = Uuid::new_v4();

    let resp = app
        .execute(
            Request::builder()
                .method(Method::DELETE)
                .uri(format!("/api/v1/documents/folders/{fake_id}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header("X-Tenant-ID", org_id.to_string())
                .body(Body::empty())
                .unwrap(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "DELETE /folders/{{id}} non-manager must return 403; got {} body: {}",
        resp.status,
        resp.text()
    );
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
    let user_a = seed_user_f(&pool, "idor-a-owner").await;
    let _ = seed_folder_f(&pool, org_a, None, "OrgA Folder", user_a).await;

    let count_before: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM document_folders WHERE organization_id = $1")
            .bind(org_a)
            .fetch_one(&pool)
            .await
            .expect("count_before");

    // No bearer token + wrong tenant header → auth gate fires (4xx).
    let resp = app
        .execute(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/documents/folders")
                .header(header::CONTENT_TYPE, "application/json")
                .header("X-Tenant-ID", org_b.to_string())
                .body(Body::from(r#"{"name":"IDOR attempt"}"#))
                .unwrap(),
        )
        .await;

    assert!(
        (400u16..500).contains(&resp.status.as_u16()),
        "cross-org POST /folders must be rejected with 4xx, got {}",
        resp.status
    );

    let count_after: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM document_folders WHERE organization_id = $1")
            .bind(org_a)
            .fetch_one(&pool)
            .await
            .expect("count_after");

    assert_eq!(
        count_before, count_after,
        "cross-org request must not mutate Org A folder count"
    );
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
    let user_a = seed_user_f(&pool, "idor-auth-get-a-owner").await;
    let folder_in_a = seed_folder_f(&pool, org_a, None, "OrgA Folder", user_a).await;

    // Attacker: a real user with a Manager membership in Org B.
    let attacker = seed_user_f(&pool, "idor-auth-get-b-attacker").await;
    common::seed_membership(&pool, org_b, attacker, "manager").await;
    let token = mint_jwt_with_role(attacker, "manager");

    let resp = app
        .execute(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/api/v1/documents/folders/{folder_in_a}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header("X-Tenant-ID", org_b.to_string())
                .body(Body::empty())
                .unwrap(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::NOT_FOUND,
        "#679: authenticated cross-org GET must hit RLS isolation and return 404, \
         got {} body: {}",
        resp.status,
        resp.text()
    );

    // Folder still exists in Org A (bypass-RLS query via app pool).
    let still_there: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM document_folders WHERE id = $1 AND organization_id = $2",
    )
    .bind(folder_in_a)
    .bind(org_a)
    .fetch_one(&pool)
    .await
    .expect("post-GET existence check");
    assert_eq!(
        still_there, 1,
        "Org A folder must remain untouched after cross-org GET"
    );
}

/// Authenticated Manager in Org B PUTs (updates) a folder owned by Org A → 404
/// (RLS hides cross-org rows; row unchanged).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_cross_org_folder_idor_authenticated_update_returns_404(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let org_a = seed_org_f(&pool, "idor-auth-put-a").await;
    let org_b = seed_org_f(&pool, "idor-auth-put-b").await;

    let user_a = seed_user_f(&pool, "idor-auth-put-a-owner").await;
    let folder_in_a = seed_folder_f(&pool, org_a, None, "OrgA Original", user_a).await;

    let attacker = seed_user_f(&pool, "idor-auth-put-b-attacker").await;
    common::seed_membership(&pool, org_b, attacker, "manager").await;
    let token = mint_jwt_with_role(attacker, "manager");

    let resp = app
        .execute(
            Request::builder()
                .method(Method::PUT)
                .uri(format!("/api/v1/documents/folders/{folder_in_a}"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header("X-Tenant-ID", org_b.to_string())
                .body(Body::from(r#"{"name":"Pwned by OrgB"}"#))
                .unwrap(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::NOT_FOUND,
        "#679: authenticated cross-org PUT must hit RLS isolation and return 404 \
         (not 403 RBAC, not 401 auth), got {} body: {}",
        resp.status,
        resp.text()
    );

    // Org A's folder name must be unchanged.
    let name_after: String = sqlx::query_scalar("SELECT name FROM document_folders WHERE id = $1")
        .bind(folder_in_a)
        .fetch_one(&pool)
        .await
        .expect("post-PUT name check");
    assert_eq!(
        name_after, "OrgA Original",
        "Org A folder name must not be mutated by a cross-org PUT"
    );
}

/// Authenticated Manager in Org B DELETEs a folder owned by Org A → 404
/// (RLS hides cross-org rows; folder still exists).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_cross_org_folder_idor_authenticated_delete_returns_404(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let org_a = seed_org_f(&pool, "idor-auth-del-a").await;
    let org_b = seed_org_f(&pool, "idor-auth-del-b").await;

    let user_a = seed_user_f(&pool, "idor-auth-del-a-owner").await;
    let folder_in_a = seed_folder_f(&pool, org_a, None, "OrgA Survivor", user_a).await;

    let attacker = seed_user_f(&pool, "idor-auth-del-b-attacker").await;
    common::seed_membership(&pool, org_b, attacker, "manager").await;
    let token = mint_jwt_with_role(attacker, "manager");

    let resp = app
        .execute(
            Request::builder()
                .method(Method::DELETE)
                .uri(format!("/api/v1/documents/folders/{folder_in_a}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header("X-Tenant-ID", org_b.to_string())
                .body(Body::empty())
                .unwrap(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::NOT_FOUND,
        "#679: authenticated cross-org DELETE must hit RLS isolation and return 404 \
         (not 403 RBAC, not 401 auth, not 204 success), got {} body: {}",
        resp.status,
        resp.text()
    );

    // Folder must still exist in Org A (bypass-RLS query via app pool).
    let still_there: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM document_folders WHERE id = $1 AND organization_id = $2",
    )
    .bind(folder_in_a)
    .bind(org_a)
    .fetch_one(&pool)
    .await
    .expect("post-DELETE existence check");
    assert_eq!(
        still_there, 1,
        "Org A folder must still exist after a cross-org DELETE attempt"
    );
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
    let user_id = seed_user_f(&pool, "depth-limit-mgr").await;
    common::seed_membership(&pool, org_id, user_id, "manager").await;

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

    let resp = app
        .execute(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/documents/folders")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header("X-Tenant-ID", org_id.to_string())
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::BAD_REQUEST,
        "depth-6 folder must return 400; got {} body: {}",
        resp.status,
        resp.text()
    );

    let json = resp.json_value();
    assert_eq!(
        json.get("code").and_then(|v| v.as_str()),
        Some("MAX_DEPTH_EXCEEDED"),
        "response code must be MAX_DEPTH_EXCEEDED; body: {json}"
    );
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
    .fetch_optional(&pool)
    .await
    .expect("schema query");
    assert!(
        row.is_some(),
        "documents.folder_id column must exist (FK to document_folders)"
    );
}

/// Verify the document_folders table exists.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_document_folders_table_exists(pool: PgPool) {
    let row = sqlx::query(
        "SELECT table_name FROM information_schema.tables \
         WHERE table_schema='public' AND table_name='document_folders'",
    )
    .fetch_optional(&pool)
    .await
    .expect("schema query");
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
    .fetch_optional(&pool)
    .await
    .expect("FK constraint query");
    assert!(
        row.is_some(),
        "documents.folder_id must have FK constraint referencing document_folders"
    );
}

// ---------------------------------------------------------------------------
// Positive AC-path coverage — new **[story 7A.2 verify]**
//
// The tests above prove the *guards* (auth 401, RBAC 403, RLS 404, depth 400,
// schema). They do NOT prove the happy paths in the story's acceptance
// criteria actually work end-to-end. These tests close that gap:
//   AC-1: a manager creating a folder -> 201 + row persisted (folder tree).
//   AC-2: a manager moving a document into a folder -> 200 + folder_id updated.
//   AC-3: deleting a folder containing documents detaches the documents to
//         root (folder_id = NULL) and soft-deletes the folder.
//
// NB on AC-3 / cascade: the RLS delete path (`delete_folder_rls`) always moves
// contained documents to root and soft-deletes the folder, regardless of the
// `cascade` flag the handler accepts — i.e. the "delete all contents" branch
// is not wired through the RLS handler (documents are never hard-removed via
// this endpoint). The test asserts the implemented behaviour (detach-to-root)
// so the contract is pinned; the unimplemented hard-cascade is recorded in the
// story verification notes rather than silently asserted.
// ---------------------------------------------------------------------------

/// AC-1: authenticated manager creates a folder -> 201, and the folder is
/// persisted (queryable in the org). Exercises the create happy path that the
/// 403/401 guard tests above cannot reach.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_create_folder_manager_succeeds(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let org_id = seed_org_f(&pool, "create-ok").await;
    let user_id = seed_user_f(&pool, "create-ok-mgr").await;
    common::seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_jwt_with_role(user_id, "manager");

    let resp = app
        .execute(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/documents/folders")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header("X-Tenant-ID", org_id.to_string())
                .body(Body::from(
                    r#"{"name":"Contracts","description":"Top-level"}"#,
                ))
                .unwrap(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "AC-1: manager create folder must return 201; got {} body: {}",
        resp.status,
        resp.text()
    );

    let json = resp.json_value();
    let new_id = json
        .get("id")
        .and_then(|v| v.as_str())
        .expect("create response must carry the new folder id");
    let new_id = Uuid::parse_str(new_id).expect("folder id must be a UUID");

    // Folder persisted under the org (bypass-RLS query via app pool).
    let persisted: (Uuid, String) = sqlx::query_as(
        "SELECT organization_id, name FROM document_folders \
         WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(new_id)
    .fetch_one(&pool)
    .await
    .expect("created folder must be persisted");
    assert_eq!(
        persisted.0, org_id,
        "folder must belong to the caller's org"
    );
    assert_eq!(
        persisted.1, "Contracts",
        "folder name must match the request"
    );
}

/// AC-2: a manager moves a document into a folder -> 200, and the document's
/// `folder_id` reflects the new location.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_move_document_into_folder_succeeds(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let org_id = seed_org_f(&pool, "move-ok").await;
    let user_id = seed_user_f(&pool, "move-ok-mgr").await;
    common::seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_jwt_with_role(user_id, "manager");

    let folder = seed_folder_f(&pool, org_id, None, "Destination", user_id).await;
    let doc = seed_document_f(&pool, org_id, None, "report", user_id).await;

    let body = serde_json::json!({ "folder_id": folder });
    let resp = app
        .execute(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/api/v1/documents/{doc}/move"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header("X-Tenant-ID", org_id.to_string())
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "AC-2: manager move document must return 200; got {} body: {}",
        resp.status,
        resp.text()
    );

    // folder_id now points at the destination folder.
    let folder_after: Option<Uuid> =
        sqlx::query_scalar("SELECT folder_id FROM documents WHERE id = $1")
            .bind(doc)
            .fetch_one(&pool)
            .await
            .expect("doc must exist after move");
    assert_eq!(
        folder_after,
        Some(folder),
        "AC-2: document.folder_id must equal the destination folder after move"
    );
}

/// AC-3: deleting a folder that contains documents detaches those documents to
/// root (folder_id = NULL) and soft-deletes the folder. Default request (no
/// body) -> non-cascade; the RLS delete path moves contents to root.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_delete_folder_detaches_documents_to_root(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let org_id = seed_org_f(&pool, "del-detach").await;
    let user_id = seed_user_f(&pool, "del-detach-mgr").await;
    common::seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_jwt_with_role(user_id, "manager");

    let folder = seed_folder_f(&pool, org_id, None, "ToDelete", user_id).await;
    let doc = seed_document_f(&pool, org_id, Some(folder), "kept-doc", user_id).await;

    let resp = app
        .execute(
            Request::builder()
                .method(Method::DELETE)
                .uri(format!("/api/v1/documents/folders/{folder}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header("X-Tenant-ID", org_id.to_string())
                .body(Body::empty())
                .unwrap(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::NO_CONTENT,
        "AC-3: manager delete folder must return 204; got {} body: {}",
        resp.status,
        resp.text()
    );

    // Folder soft-deleted.
    let folder_deleted_at: Option<chrono::DateTime<chrono::Utc>> =
        sqlx::query_scalar("SELECT deleted_at FROM document_folders WHERE id = $1")
            .bind(folder)
            .fetch_one(&pool)
            .await
            .expect("folder row must still exist (soft delete)");
    assert!(
        folder_deleted_at.is_some(),
        "AC-3: deleted folder must be soft-deleted (deleted_at set)"
    );

    // Document survives and is detached to root.
    let (doc_folder, doc_deleted): (Option<Uuid>, Option<chrono::DateTime<chrono::Utc>>) =
        sqlx::query_as("SELECT folder_id, deleted_at FROM documents WHERE id = $1")
            .bind(doc)
            .fetch_one(&pool)
            .await
            .expect("document must survive folder delete");
    assert_eq!(
        doc_folder, None,
        "AC-3: document in a deleted folder must be detached to root (folder_id = NULL)"
    );
    assert!(
        doc_deleted.is_none(),
        "AC-3: deleting a folder must NOT delete its documents (no hard cascade)"
    );
}

/// Hierarchy integrity: a manager cannot move a folder into one of its own
/// descendants (would create a cycle) -> 400 CIRCULAR_REFERENCE. Backs the
/// `is_descendant_of_rls` guard in the update handler.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_update_folder_into_descendant_is_rejected(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let org_id = seed_org_f(&pool, "circular").await;
    let user_id = seed_user_f(&pool, "circular-mgr").await;
    common::seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_jwt_with_role(user_id, "manager");

    // root -> child  (try to set root.parent = child => cycle)
    let root = seed_folder_f(&pool, org_id, None, "Root", user_id).await;
    let child = seed_folder_f(&pool, org_id, Some(root), "Child", user_id).await;

    let body = serde_json::json!({ "parent_id": child });
    let resp = app
        .execute(
            Request::builder()
                .method(Method::PUT)
                .uri(format!("/api/v1/documents/folders/{root}"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header("X-Tenant-ID", org_id.to_string())
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::BAD_REQUEST,
        "moving a folder into its own descendant must return 400; got {} body: {}",
        resp.status,
        resp.text()
    );
    let json = resp.json_value();
    assert_eq!(
        json.get("code").and_then(|v| v.as_str()),
        Some("CIRCULAR_REFERENCE"),
        "response code must be CIRCULAR_REFERENCE; body: {json}"
    );

    // root.parent_id unchanged (still root-level).
    let parent_after: Option<Uuid> =
        sqlx::query_scalar("SELECT parent_id FROM document_folders WHERE id = $1")
            .bind(root)
            .fetch_one(&pool)
            .await
            .expect("root folder must still exist");
    assert_eq!(
        parent_after, None,
        "rejected circular update must not mutate the folder's parent"
    );
}

// ---------------------------------------------------------------------------
// Move-to-root + tri-state parent_id — #1589 finding 1
// ---------------------------------------------------------------------------

/// PUT /folders/{id} with `{"parent_id": null}` must DETACH a nested folder to
/// the top level. The old `parent_id = COALESCE($n, parent_id)` ignored an
/// explicit null, so the move returned 200 while the folder stayed under its
/// parent (a silent no-op). The tri-state fix makes null set parent_id = NULL.
/// Fails-on-`dev` (the COALESCE leaves parent unchanged); passes with the fix.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_update_folder_move_to_root_detaches(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let org = seed_org_f(&pool, "move-root").await;
    let user = seed_user_f(&pool, "move-root-mgr").await;
    common::seed_membership(&pool, org, user, "manager").await;
    let token = mint_jwt_with_role(user, "manager");

    let parent = seed_folder_f(&pool, org, None, "Parent", user).await;
    let child = seed_folder_f(&pool, org, Some(parent), "Child", user).await;

    let resp = app
        .execute(
            Request::builder()
                .method(Method::PUT)
                .uri(format!("/api/v1/documents/folders/{child}"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header("X-Tenant-ID", org.to_string())
                .body(Body::from(r#"{"parent_id":null}"#))
                .unwrap(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "move-to-root must return 200; got {} body: {}",
        resp.status,
        resp.text()
    );

    let parent_after: Option<Uuid> =
        sqlx::query_scalar("SELECT parent_id FROM document_folders WHERE id = $1")
            .bind(child)
            .fetch_one(&pool)
            .await
            .expect("post-move parent check");
    assert_eq!(
        parent_after, None,
        "child must be detached to root (parent_id IS NULL), not silently left \
         under its parent (#1589)"
    );
}

/// Guards the tri-state's "absent" arm: a name-only update must NOT touch
/// parent_id (the COALESCE→CASE change must not regress this).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_update_folder_name_only_preserves_parent(pool: PgPool) {
    let app = common::TestApp::new(pool.clone()).await;
    let org = seed_org_f(&pool, "name-only").await;
    let user = seed_user_f(&pool, "name-only-mgr").await;
    common::seed_membership(&pool, org, user, "manager").await;
    let token = mint_jwt_with_role(user, "manager");

    let parent = seed_folder_f(&pool, org, None, "Parent2", user).await;
    let child = seed_folder_f(&pool, org, Some(parent), "Child2", user).await;

    let resp = app
        .execute(
            Request::builder()
                .method(Method::PUT)
                .uri(format!("/api/v1/documents/folders/{child}"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header("X-Tenant-ID", org.to_string())
                .body(Body::from(r#"{"name":"Renamed"}"#))
                .unwrap(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "name-only update must return 200; got {} body: {}",
        resp.status,
        resp.text()
    );

    let parent_after: Option<Uuid> =
        sqlx::query_scalar("SELECT parent_id FROM document_folders WHERE id = $1")
            .bind(child)
            .fetch_one(&pool)
            .await
            .expect("post-rename parent check");
    assert_eq!(
        parent_after,
        Some(parent),
        "a name-only update must leave parent_id unchanged (absent != null)"
    );
}
