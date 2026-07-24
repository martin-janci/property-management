//! Integration tests for Epic 66: Platform Migration & Data Import endpoints.
//!
//! Asserts real database effects and round-trips for templates, import jobs,
//! job validation errors, and exports.

#![allow(dead_code)]

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{create_authenticated_user, seed_membership, seed_org, TestApp, TestUser};
use db::models::{
    ApproveImportResponse, ImportJobStatus, ImportJobStatusResponse, ImportPreviewResult,
    MigrationExportStatus, MigrationExportStatusResponse,
};

#[derive(serde::Deserialize)]
struct TemplateDetailResponse {
    name: String,
    field_mappings: Vec<serde_json::Value>,
}

/// Helper to create a request with Bearer Auth and Tenant header
fn request(
    token: &str,
    tenant_id: Uuid,
    method: Method,
    uri: &str,
    body: Option<serde_json::Value>,
) -> Request<Body> {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header("X-Tenant-ID", tenant_id.to_string());

    if body.is_some() {
        builder = builder.header(header::CONTENT_TYPE, "application/json");
    }

    let req_body = match body {
        Some(val) => Body::from(val.to_string()),
        None => Body::empty(),
    };

    builder.body(req_body).unwrap()
}

/// Mint an access token carrying the `platform_admin` role and a `tenant_id`
/// claim pinned to `org_id`.
///
/// `require_platform_admin` checks `AuthUser::is_platform_admin()` (the `role`
/// JWT claim). The `/auth/login` flow mints tokens with `role = None`
/// ("roles set when org context is selected"), so a login token always 403s on
/// this gate. We mint directly, mirroring `migration_pagination_tests.rs`.
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
            name: "Migration DB Admin".into(),
        },
        &EncodingKey::from_secret(
            b"test-secret-key-that-is-at-least-64-characters-long-for-testing-purposes",
        ),
    )
    .expect("mint platform-admin token")
}

/// Provision a platform admin member for RLS.
async fn create_platform_admin(app: &TestApp, user: &TestUser, slug: &str) -> (String, Uuid) {
    let (_login_token, _refresh) = create_authenticated_user(app, user).await;

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

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_import_template_lifecycle(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, tenant_id) = create_platform_admin(&app, &user, "migtemp").await;

    // 1. List templates initially (should contain the seeded system templates)
    let list_req = request(
        &token,
        tenant_id,
        Method::GET,
        "/api/v1/migration/templates",
        None,
    );
    let list_resp = app.execute(list_req).await;
    assert_eq!(list_resp.status, StatusCode::OK);
    let list_json = list_resp.json_value();
    assert!(list_json["total"].as_i64().unwrap() >= 3); // minimum 3 system templates seeded

    // 2. Create custom import template
    let new_template = serde_json::json!({
        "name": "Custom Building Import",
        "description": "Custom mappings for building import",
        "data_type": "buildings",
        "field_mappings": [
            {
                "field_name": "name",
                "display_label": "Building Name",
                "column_header": "BName",
                "data_type": "string",
                "validation": {
                    "required": true
                }
            }
        ]
    });

    let create_req = request(
        &token,
        tenant_id,
        Method::POST,
        "/api/v1/migration/templates",
        Some(new_template),
    );
    let create_resp = app.execute(create_req).await;
    assert_eq!(create_resp.status, StatusCode::OK);
    let created = create_resp.json_value();
    let template_id = Uuid::parse_str(created["id"].as_str().unwrap()).unwrap();

    // Verify it is persisted in the database
    let db_exists: (bool,) =
        sqlx::query_as("SELECT EXISTS(SELECT 1 FROM import_templates WHERE id = $1)")
            .bind(template_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(db_exists.0, "Template should be persisted in database");

    // 3. Retrieve detailed template
    let get_req = request(
        &token,
        tenant_id,
        Method::GET,
        &format!("/api/v1/migration/templates/{template_id}"),
        None,
    );
    let get_resp = app.execute(get_req).await;
    assert_eq!(get_resp.status, StatusCode::OK);
    let detail: TemplateDetailResponse = get_resp.json();
    assert_eq!(detail.name, "Custom Building Import");
    assert_eq!(detail.field_mappings.len(), 1);

    // 4. Update the template
    let update_body = serde_json::json!({
        "name": "Updated Custom Import",
        "description": "Updated description"
    });
    let update_req = request(
        &token,
        tenant_id,
        Method::PUT,
        &format!("/api/v1/migration/templates/{template_id}"),
        Some(update_body),
    );
    let update_resp = app.execute(update_req).await;
    assert_eq!(update_resp.status, StatusCode::OK);

    // Assert update persisted in DB
    let db_name: (String,) = sqlx::query_as("SELECT name FROM import_templates WHERE id = $1")
        .bind(template_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(db_name.0, "Updated Custom Import");

    // 5. Duplicate template
    let duplicate_req = request(
        &token,
        tenant_id,
        Method::POST,
        &format!("/api/v1/migration/templates/{template_id}/duplicate"),
        None,
    );
    let duplicate_resp = app.execute(duplicate_req).await;
    assert_eq!(duplicate_resp.status, StatusCode::OK);
    let dup_json = duplicate_resp.json_value();
    let dup_id = Uuid::parse_str(dup_json["id"].as_str().unwrap()).unwrap();
    assert_ne!(dup_id, template_id);

    // 6. Delete template
    let delete_req = request(
        &token,
        tenant_id,
        Method::DELETE,
        &format!("/api/v1/migration/templates/{template_id}"),
        None,
    );
    let delete_resp = app.execute(delete_req).await;
    assert_eq!(delete_resp.status, StatusCode::NO_CONTENT);

    // Assert deleted in DB
    let db_exists_after: (bool,) =
        sqlx::query_as("SELECT EXISTS(SELECT 1 FROM import_templates WHERE id = $1)")
            .bind(template_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(
        !db_exists_after.0,
        "Template should be removed from database"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_import_job_execution_flow(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, tenant_id) = create_platform_admin(&app, &user, "migjob").await;

    // Retrieve system template to initiate a job
    let list_req = request(
        &token,
        tenant_id,
        Method::GET,
        "/api/v1/migration/templates/system",
        None,
    );
    let list_resp = app.execute(list_req).await;
    assert_eq!(list_resp.status, StatusCode::OK);
    let list_json = list_resp.json_value();
    let template_id = Uuid::parse_str(list_json[0]["id"].as_str().unwrap()).unwrap();

    // Create a mock import job manually in DB
    let job_id = Uuid::new_v4();
    sqlx::query(
        r#"INSERT INTO import_jobs (
            id, organization_id, template_id, status, original_filename,
            file_path, file_size_bytes, total_rows, created_by
        ) VALUES ($1, $2, $3, 'pending', 'test_data.csv', 'imports/test_data.csv', 1024, 100,
          (SELECT id FROM users LIMIT 1))"#,
    )
    .bind(job_id)
    .bind(tenant_id)
    .bind(template_id)
    .execute(&pool)
    .await
    .unwrap();

    // 1. Get status of job
    let status_req = request(
        &token,
        tenant_id,
        Method::GET,
        &format!("/api/v1/migration/import/jobs/{job_id}"),
        None,
    );
    let status_resp = app.execute(status_req).await;
    assert_eq!(status_resp.status, StatusCode::OK);
    let status: ImportJobStatusResponse = status_resp.json();
    assert_eq!(status.status, ImportJobStatus::Pending);

    // 2. Validate job
    let validate_req = request(
        &token,
        tenant_id,
        Method::POST,
        &format!("/api/v1/migration/import/jobs/{job_id}/validate"),
        None,
    );
    let validate_resp = app.execute(validate_req).await;
    assert_eq!(validate_resp.status, StatusCode::OK);
    let validated_status: ImportJobStatusResponse = validate_resp.json();
    assert_eq!(validated_status.status, ImportJobStatus::Validated);

    // 3. Retrieve preview
    let preview_req = request(
        &token,
        tenant_id,
        Method::GET,
        &format!("/api/v1/migration/import/jobs/{job_id}/preview"),
        None,
    );
    let preview_resp = app.execute(preview_req).await;
    assert_eq!(preview_resp.status, StatusCode::OK);
    let preview: ImportPreviewResult = preview_resp.json();
    assert_eq!(preview.job_id, job_id);
    assert_eq!(preview.issues.len(), 2); // validated inserts 2 mock errors

    // 4. Retrieve job errors
    let errors_req = request(
        &token,
        tenant_id,
        Method::GET,
        &format!("/api/v1/migration/import/jobs/{job_id}/errors"),
        None,
    );
    let errors_resp = app.execute(errors_req).await;
    assert_eq!(errors_resp.status, StatusCode::OK);
    let errors_json = errors_resp.json_value();
    assert_eq!(errors_json["total_errors"].as_i64().unwrap(), 2);

    // 5. Approve import
    let approve_body = serde_json::json!({
        "job_id": job_id,
        "acknowledge_warnings": true
    });
    let approve_req = request(
        &token,
        tenant_id,
        Method::POST,
        &format!("/api/v1/migration/import/jobs/{job_id}/approve"),
        Some(approve_body),
    );
    let approve_resp = app.execute(approve_req).await;
    assert_eq!(approve_resp.status, StatusCode::OK);
    let approved: ApproveImportResponse = approve_resp.json();
    assert_eq!(approved.status, ImportJobStatus::Importing);

    // Verify database state updated
    let db_status: (String,) = sqlx::query_as("SELECT status::text FROM import_jobs WHERE id = $1")
        .bind(job_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(db_status.0, "importing");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_migration_exports(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, tenant_id) = create_platform_admin(&app, &user, "migexp").await;

    // 1. Request export
    let export_body = serde_json::json!({
        "categories": ["buildings", "units"],
        "privacy_options": {
            "anonymize_personal_data": true,
            "mask_financial_data": true,
            "exclude_document_contents": false,
            "hash_identifiers": false
        }
    });

    let export_req = request(
        &token,
        tenant_id,
        Method::POST,
        "/api/v1/migration/export",
        Some(export_body),
    );
    let export_resp = app.execute(export_req).await;
    assert_eq!(export_resp.status, StatusCode::OK);
    let export_json = export_resp.json_value();
    let export_id = Uuid::parse_str(export_json["export_id"].as_str().unwrap()).unwrap();

    // Verify it exists in DB as pending
    let db_status: (String,) =
        sqlx::query_as("SELECT status::text FROM migration_exports WHERE id = $1")
            .bind(export_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(db_status.0, "pending");

    // 2. Get export status (auto-transitions to ready in mock setup)
    let status_req = request(
        &token,
        tenant_id,
        Method::GET,
        &format!("/api/v1/migration/export/{export_id}"),
        None,
    );
    let status_resp = app.execute(status_req).await;
    assert_eq!(status_resp.status, StatusCode::OK);
    let status: MigrationExportStatusResponse = status_resp.json();
    assert_eq!(status.status, MigrationExportStatus::Ready);
    assert!(status.download_url.is_some());

    // 3. Download the export
    let download_req = request(
        &token,
        tenant_id,
        Method::GET,
        &format!("/api/v1/migration/export/{export_id}/download"),
        None,
    );
    let download_resp = app.execute(download_req).await;
    assert_eq!(download_resp.status, StatusCode::OK);

    // Verify download count is incremented in DB
    let db_download_count: (i32,) =
        sqlx::query_as("SELECT download_count FROM migration_exports WHERE id = $1")
            .bind(export_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        db_download_count.0, 1,
        "Download count should be 1 after download"
    );
}
