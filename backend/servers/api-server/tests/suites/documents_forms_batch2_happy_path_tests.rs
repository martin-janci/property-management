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

/// Seed a lease_extraction that is MISSING the required-for-import fields
/// (`tenant_name` and `monthly_rent` are NULL). `validate_import` must then
/// report `can_import: false` with a non-empty error list. `lease_start_date`
/// is kept populated so the two NULLs are the only validation errors.
async fn seed_lease_extraction_missing_required(pool: &PgPool, document_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO lease_extractions \
         (document_id, tenant_name, lease_start_date, lease_end_date, monthly_rent, \
          overall_confidence, fields_extracted, fields_flagged, review_status) \
         VALUES ($1, NULL, '2026-01-01', '2026-12-31', NULL, 95.00, 3, 2, 'pending') \
         RETURNING id",
    )
    .bind(document_id)
    .fetch_one(pool)
    .await
    .expect("seed lease_extraction with missing required fields")
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
            .json(json!({ "unit_id": unit_id }))
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
            .json(json!({ "unit_id": unit_id }))
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

// ===========================================================================
// Negative cases (#2537 follow-up): cross-tenant IDOR + error branches.
//
// The lease-abstraction import handlers resolve the org exclusively from the
// JWT `tenant_id` claim and gate every read through `get_extraction_for_org`
// (extraction -> document -> organization join) and a unit -> building ->
// organization ownership check. The cases below exercise those guards plus the
// import state machine (`review_status`) and the validation error path.
// ===========================================================================

/// Seed a second organisation with its own admin user and return
/// `(org_id, building_id, unit_id)`. Used to prove that resources belonging to
/// one org are invisible / rejected when acted on under a different org's token.
async fn seed_second_org(app: &TestApp, pool: &PgPool, slug: &str) -> (Uuid, Uuid, Uuid) {
    let other_user = TestUser::new();
    let (_login, org_id) = create_authenticated_user_with_org(app, &other_user, slug).await;
    let building_id = seed_building(pool, org_id).await;
    let unit_id = seed_unit(pool, building_id).await;
    (org_id, building_id, unit_id)
}

/// Cross-tenant IDOR: an extraction seeded under org A must be a 404 (not a 403)
/// when validated with an org-B token — `get_extraction_for_org` filters by the
/// caller's org, so org B simply cannot see the row.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn validate_import_cross_tenant_returns_404(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    // Org A owns the extraction + a valid unit.
    let user_a = TestUser::new();
    let (_login_a, org_a) = create_authenticated_user_with_org(&app, &user_a, "la-idor-a").await;
    let user_a_id = user_id_for(&pool, &user_a.email).await;
    let building_a = seed_building(&pool, org_a).await;
    let unit_a = seed_unit(&pool, building_a).await;
    let doc_id = seed_lease_document(&pool, org_a, user_a_id).await;
    let ext_id = seed_lease_extraction(&pool, doc_id, "pending").await;

    // Org B: different org, its own admin — token carries org B's tenant_id.
    let user_b = TestUser::new();
    let (_login_b, org_b) = create_authenticated_user_with_org(&app, &user_b, "la-idor-b").await;
    let user_b_id = user_id_for(&pool, &user_b.email).await;
    let token_b = mint_tenant_token(user_b_id, org_b);

    let resp = app
        .execute(
            app.post(&format!(
                "/api/v1/lease-abstraction/extractions/{ext_id}/validate"
            ))
            .bearer(&token_b)
            .json(json!({ "unit_id": unit_a }))
            .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::NOT_FOUND,
        "org B must not see org A's extraction (404, not 403/200); body: {}",
        resp.text()
    );
}

/// Cross-tenant IDOR on the import read path: an import row seeded under org A
/// is a 404 for org B.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_import_cross_tenant_returns_404(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    // Org A owns the document/extraction/import chain.
    let user_a = TestUser::new();
    let (_login_a, org_a) =
        create_authenticated_user_with_org(&app, &user_a, "la-idor-imp-a").await;
    let user_a_id = user_id_for(&pool, &user_a.email).await;
    let doc_id = seed_lease_document(&pool, org_a, user_a_id).await;
    let ext_id = seed_lease_extraction(&pool, doc_id, "approved").await;
    let import_id = seed_lease_import(&pool, ext_id).await;

    // Org B tries to read org A's import by id.
    let user_b = TestUser::new();
    let (_login_b, org_b) =
        create_authenticated_user_with_org(&app, &user_b, "la-idor-imp-b").await;
    let user_b_id = user_id_for(&pool, &user_b.email).await;
    let token_b = mint_tenant_token(user_b_id, org_b);

    let resp = app
        .execute(
            app.get(&format!("/api/v1/lease-abstraction/imports/{import_id}"))
                .bearer(&token_b)
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::NOT_FOUND,
        "org B must not read org A's import (404); body: {}",
        resp.text()
    );
}

/// Cross-org unit: the extraction belongs to the caller's org, but the supplied
/// `unit_id` belongs to a *different* org's building — the unit-ownership guard
/// must reject with 403 (not silently import into a foreign building).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn validate_import_foreign_unit_returns_403(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    // Caller's org owns the extraction.
    let user = TestUser::new();
    let (_login, org_id) = create_authenticated_user_with_org(&app, &user, "la-foreign-unit").await;
    let user_id = user_id_for(&pool, &user.email).await;
    let token = mint_tenant_token(user_id, org_id);
    let doc_id = seed_lease_document(&pool, org_id, user_id).await;
    let ext_id = seed_lease_extraction(&pool, doc_id, "pending").await;

    // A second org owns the unit the caller tries to target.
    let (_other_org, _other_building, foreign_unit) =
        seed_second_org(&app, &pool, "la-foreign-unit-other").await;

    let resp = app
        .execute(
            app.post(&format!(
                "/api/v1/lease-abstraction/extractions/{ext_id}/validate"
            ))
            .bearer(&token)
            .json(json!({ "unit_id": foreign_unit }))
            .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "unit from another org must be rejected with 403; body: {}",
        resp.text()
    );
}

/// Import state machine: an extraction that has not been human-approved
/// (`review_status = 'pending'`) cannot be imported — the handler returns 409
/// CONFLICT before touching the lease tables.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn import_pending_extraction_returns_409(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let user = TestUser::new();
    let (_login, org_id) =
        create_authenticated_user_with_org(&app, &user, "la-import-pending").await;
    let user_id = user_id_for(&pool, &user.email).await;
    let token = mint_tenant_token(user_id, org_id);

    let building_id = seed_building(&pool, org_id).await;
    let unit_id = seed_unit(&pool, building_id).await;
    let doc_id = seed_lease_document(&pool, org_id, user_id).await;
    // Not approved — the review_status gate fires before any import work.
    let ext_id = seed_lease_extraction(&pool, doc_id, "pending").await;

    let resp = app
        .execute(
            app.post(&format!(
                "/api/v1/lease-abstraction/extractions/{ext_id}/import"
            ))
            .bearer(&token)
            .json(json!({ "unit_id": unit_id }))
            .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::CONFLICT,
        "a pending (un-approved) extraction must not import (409); body: {}",
        resp.text()
    );
}

/// Validation error path: an extraction missing required fields (NULL
/// `tenant_name` and `monthly_rent`) validates as `can_import: false` with a
/// non-empty issues list naming the offending fields.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn validate_import_missing_fields_reports_issues(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let user = TestUser::new();
    let (_login, org_id) =
        create_authenticated_user_with_org(&app, &user, "la-validate-missing").await;
    let user_id = user_id_for(&pool, &user.email).await;
    let token = mint_tenant_token(user_id, org_id);

    let building_id = seed_building(&pool, org_id).await;
    let unit_id = seed_unit(&pool, building_id).await;
    let doc_id = seed_lease_document(&pool, org_id, user_id).await;
    let ext_id = seed_lease_extraction_missing_required(&pool, doc_id).await;

    let resp = app
        .execute(
            app.post(&format!(
                "/api/v1/lease-abstraction/extractions/{ext_id}/validate"
            ))
            .bearer(&token)
            .json(json!({ "unit_id": unit_id }))
            .build(),
        )
        .await;

    // The request itself is well-formed (200) — the *payload* reports the
    // extraction cannot be imported.
    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    let body = resp.json_value();
    assert_eq!(
        body["can_import"].as_bool(),
        Some(false),
        "extraction is missing required fields, so import must be disallowed; body: {body}"
    );
    let issues = body["errors"]
        .as_array()
        .expect("validation response must carry an errors array");
    assert!(
        !issues.is_empty(),
        "missing required fields must produce a non-empty issues array; body: {body}"
    );
    let fields: Vec<&str> = issues.iter().filter_map(|i| i["field"].as_str()).collect();
    assert!(
        fields.contains(&"tenant_name") && fields.contains(&"monthly_rent"),
        "issues must name the missing fields (tenant_name, monthly_rent); got {fields:?}"
    );
}
