//! BIT-558 wave-10: documents-forms batch 2 happy-path (2xx) tests.
//!
//! The documents-forms group is already broadly covered on `dev`
//! (`documents_forms_happy_path_tests.rs` = forms; `documents_core_crud_tests.rs`,
//! `document_folder_tests.rs`, `document_upload_tests.rs`,
//! `document_download_preview_tests.rs`, `document_share_access_tests.rs`,
//! `documents_intelligence_templates_tests.rs` = documents; `signatures_legal_docs_tests.rs`,
//! `legal_notices_audit_tests.rs`, `legal_insurance_wave1b_tests.rs` = signatures/legal;
//! `lease_abstraction_forms_tests.rs` = lease-abstraction documents/extractions).
//!
//! The only endpoints in the whole group that had NO happy-path (2xx) test
//! anywhere were the three lease-abstraction *import* endpoints. This file closes
//! that final gap:
//!
//! - POST /api/v1/lease-abstraction/extractions/{id}/validate (validate_import)   200
//! - POST /api/v1/lease-abstraction/extractions/{id}/import   (import_to_lease)    200
//! - GET  /api/v1/lease-abstraction/imports/{id}             (get_import)          200
//!
//! These handlers use the `AuthUser` extractor and read the organisation from the
//! JWT `tenant_id` claim (NOT the `X-Tenant-ID` header), so we mint a token that
//! carries the claim — the same on-the-wire shape used by
//! `accounting_happy_path_tests.rs`. Rows are seeded directly (the upload/extract
//! path needs S3 + an LLM), which the repo reads back through org-scoped joins.

use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{create_authenticated_user_with_org, TestApp, TestUser};

/// Mint an access token carrying a `tenant_id` claim, matching the on-the-wire
/// shape the `AuthUser` extractor expects (lease-abstraction handlers resolve the
/// org from `user.tenant_id`).
fn mint_tenant_token(user_id: Uuid, org_id: Uuid) -> String {
    use jsonwebtoken::{encode, EncodingKey, Header};
    use serde::Serialize;

    #[derive(Serialize)]
    struct Claims {
        sub: String,
        exp: i64,
        iat: i64,
        token_type: String,
        tenant_id: String,
        role: Option<String>,
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
            role: None,
            email: "lease-abstraction-batch2@test.example".into(),
            name: "Lease Abstraction Batch2".into(),
        },
        &EncodingKey::from_secret(
            b"test-secret-key-that-is-at-least-64-characters-long-for-testing-purposes",
        ),
    )
    .expect("mint tenant token")
}

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

async fn seed_building(pool: &PgPool, org_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO buildings (organization_id, street, city, postal_code, country) \
         VALUES ($1, 'Lease Street', 'Bratislava', '81101', 'Slovakia') RETURNING id",
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed building")
}

async fn seed_unit(pool: &PgPool, building_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO units (building_id, designation, floor) \
         VALUES ($1, 'A1', 1) RETURNING id",
    )
    .bind(building_id)
    .fetch_one(pool)
    .await
    .expect("seed unit")
}

/// Seed a completed lease_document (avoids the S3 upload path).
async fn seed_lease_document(pool: &PgPool, org_id: Uuid, user_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO lease_documents \
         (organization_id, uploaded_by, file_name, file_size_bytes, mime_type, storage_path, status) \
         VALUES ($1, $2, 'lease.pdf', 102400, 'application/pdf', 'abstractions/test/lease.pdf', 'completed') \
         RETURNING id",
    )
    .bind(org_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("seed lease_document")
}

/// Seed a lease_extraction with the fields `validate_import` treats as required
/// (`tenant_name`, `lease_start_date`, `monthly_rent`) so validation can pass.
/// `review_status` is caller-controlled (import requires `approved`).
async fn seed_lease_extraction(pool: &PgPool, document_id: Uuid, review_status: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO lease_extractions \
         (document_id, tenant_name, lease_start_date, lease_end_date, monthly_rent, \
          overall_confidence, fields_extracted, fields_flagged, review_status) \
         VALUES ($1, 'Alice Tenant', '2026-01-01', '2026-12-31', 1200.00, 95.00, 5, 0, $2) \
         RETURNING id",
    )
    .bind(document_id)
    .bind(review_status)
    .fetch_one(pool)
    .await
    .expect("seed lease_extraction")
}

/// Seed a lease_imports row directly (status defaults to 'pending').
async fn seed_lease_import(pool: &PgPool, extraction_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO lease_imports (extraction_id) VALUES ($1) RETURNING id",
    )
    .bind(extraction_id)
    .fetch_one(pool)
    .await
    .expect("seed lease_import")
}

// ---------------------------------------------------------------------------
// POST /api/v1/lease-abstraction/extractions/{id}/validate
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn validate_import_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (_login, org_id) = create_authenticated_user_with_org(&app, &user, "la-validate").await;
    let user_id = user_id_for(&pool, &user.email).await;
    let token = mint_tenant_token(user_id, org_id);

    let building_id = seed_building(&pool, org_id).await;
    let unit_id = seed_unit(&pool, building_id).await;
    let doc_id = seed_lease_document(&pool, org_id, user_id).await;
    let ext_id = seed_lease_extraction(&pool, doc_id, "pending").await;

    let resp = app
        .execute(
            app.post(&format!(
                "/api/v1/lease-abstraction/extractions/{ext_id}/validate"
            ))
            .bearer(&token)
            .json(&json!({ "unit_id": unit_id }))
            .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    let body = resp.json_value();
    assert_eq!(
        body["can_import"].as_bool(),
        Some(true),
        "extraction has all required fields, so import must be allowed; body: {body}"
    );
}

// ---------------------------------------------------------------------------
// POST /api/v1/lease-abstraction/extractions/{id}/import
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn import_to_lease_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (_login, org_id) = create_authenticated_user_with_org(&app, &user, "la-import").await;
    let user_id = user_id_for(&pool, &user.email).await;
    let token = mint_tenant_token(user_id, org_id);

    let building_id = seed_building(&pool, org_id).await;
    let unit_id = seed_unit(&pool, building_id).await;
    let doc_id = seed_lease_document(&pool, org_id, user_id).await;
    // import_to_lease requires the extraction to be human-approved first.
    let ext_id = seed_lease_extraction(&pool, doc_id, "approved").await;

    let resp = app
        .execute(
            app.post(&format!(
                "/api/v1/lease-abstraction/extractions/{ext_id}/import"
            ))
            .bearer(&token)
            .json(&json!({ "unit_id": unit_id }))
            .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    let body = resp.json_value();
    assert_eq!(
        body["success"].as_bool(),
        Some(true),
        "import must report success; body: {body}"
    );
}

// ---------------------------------------------------------------------------
// GET /api/v1/lease-abstraction/imports/{id}
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_import_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (_login, org_id) = create_authenticated_user_with_org(&app, &user, "la-get-import").await;
    let user_id = user_id_for(&pool, &user.email).await;
    let token = mint_tenant_token(user_id, org_id);

    let doc_id = seed_lease_document(&pool, org_id, user_id).await;
    let ext_id = seed_lease_extraction(&pool, doc_id, "approved").await;
    let import_id = seed_lease_import(&pool, ext_id).await;

    let resp = app
        .execute(
            app.get(&format!("/api/v1/lease-abstraction/imports/{import_id}"))
                .bearer(&token)
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    let body = resp.json_value();
    assert_eq!(
        body["id"].as_str(),
        Some(import_id.to_string().as_str()),
        "returned import id must match the seeded one; body: {body}"
    );
}
