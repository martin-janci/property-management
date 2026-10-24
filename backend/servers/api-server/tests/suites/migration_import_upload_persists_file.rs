//! Regression (Epic 66, Story 66.2): `POST /api/v1/migration/import/upload`
//! must persist the uploaded bytes to object storage and fail closed when no
//! storage service is configured.
//!
//! Before the fix, `upload_import_file` read the multipart body into a
//! discarded `_file_data` buffer, fabricated a `file_path` string that pointed
//! at no real object, and returned `200 OK` with a freshly-created import_job
//! row — silently losing every uploaded file (data loss on bulk import). The
//! downstream validate/approve flow then operated on a storage key that never
//! existed.
//!
//! The default `TestApp` is built with `storage_service: None`, so this test
//! exercises the fail-closed branch end to end: with no storage configured the
//! handler MUST reject the upload with `503 SERVICE_UNAVAILABLE` and MUST NOT
//! create a dangling import_job row.
//!
//! This is the "fails-before-fix" proof: before the fix the stub ignored
//! `storage_service` entirely and returned `200` with a persisted job row; the
//! two assertions below (status == 503, zero job rows) both failed. After the
//! fix they pass.

#![allow(dead_code)]

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{create_authenticated_user, seed_membership, seed_org, TestApp, TestUser};

/// Mint an access token carrying the `platform_admin` role and a `tenant_id`
/// claim pinned to `org_id` (mirrors `migration_db_tests::mint_platform_admin_token`).
fn mint_platform_admin_token(user_id: Uuid, org_id: Uuid, email: &str) -> String {
    use jsonwebtoken::{encode, EncodingKey, Header};
    use serde::Serialize;

    #[derive(Serialize)]
    struct Claims {
        sub: String,
        exp: i64,
        iat: i64,
        token_type: String,
        tenant_id: String,
        role: String,
        email: String,
        name: String,
    }

    let now = chrono::Utc::now().timestamp();
    encode(
        &Header::default(),
        &Claims {
            sub: user_id.to_string(),
            exp: now + 900,
            iat: now,
            token_type: "access".into(),
            tenant_id: org_id.to_string(),
            role: "platform_admin".into(),
            email: email.into(),
            name: "Migration Upload Admin".into(),
        },
        &EncodingKey::from_secret(
            b"test-secret-key-that-is-at-least-64-characters-long-for-testing-purposes",
        ),
    )
    .expect("mint platform-admin token")
}

/// Register + org-seed a platform-admin member so `RlsConnection` resolves the
/// tenant, and return `(token, org_id)`.
async fn create_platform_admin(app: &TestApp, user: &TestUser, slug: &str) -> (String, Uuid) {
    create_authenticated_user(app, user).await;

    let user_id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(&user.email)
        .fetch_one(&app.pool)
        .await
        .expect("resolve user id");

    let org_id = seed_org(&app.pool, slug).await;
    seed_membership(&app.pool, org_id, user_id, "platform_admin").await;

    let token = mint_platform_admin_token(user_id, org_id, &user.email);
    (token, org_id)
}

/// Grab a seeded system template id to drive a well-formed upload request.
async fn first_system_template_id(app: &TestApp, token: &str, org_id: Uuid) -> Uuid {
    let req = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/migration/templates/system")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header("X-Tenant-ID", org_id.to_string())
        .body(Body::empty())
        .unwrap();
    let resp = app.execute(req).await;
    assert_eq!(resp.status, StatusCode::OK, "list system templates");
    let json = resp.json_value();
    Uuid::parse_str(json[0]["id"].as_str().expect("system template id")).unwrap()
}

/// With no storage service configured, the upload must fail closed (503) and
/// leave no import_job row behind.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn upload_import_file_fails_closed_without_storage(pool: PgPool) {
    // Default TestApp carries `storage_service: None`.
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_platform_admin(&app, &user, "migupload").await;
    let template_id = first_system_template_id(&app, &token, org_id).await;

    let csv = "name,address\nTest Building,1 Main St\n";
    let boundary = "bit420migrationuploadboundary";
    let body = format!(
        "--{boundary}\r\n\
         Content-Disposition: form-data; name=\"template_id\"\r\n\
         \r\n\
         {template_id}\r\n\
         --{boundary}\r\n\
         Content-Disposition: form-data; name=\"file\"; filename=\"buildings.csv\"\r\n\
         Content-Type: text/csv\r\n\
         \r\n\
         {csv}\r\n\
         --{boundary}--\r\n"
    );

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/migration/import/upload")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header("X-Tenant-ID", org_id.to_string())
        .header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(Body::from(body))
        .expect("build multipart upload request");

    let resp = app.execute(req).await;

    // Fails-before-fix proof #1: the stub returned 200 OK here.
    assert_eq!(
        resp.status,
        StatusCode::SERVICE_UNAVAILABLE,
        "upload must fail closed with 503 when storage is unconfigured, got {}",
        resp.status
    );

    // Fails-before-fix proof #2: the stub persisted a dangling job row.
    let job_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM import_jobs WHERE organization_id = $1")
            .bind(org_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        job_count.0, 0,
        "no import_job row may be created when the file was not stored"
    );
}
