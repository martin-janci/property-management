#![allow(dead_code)]

//! Wave 1B test-backfill: Compliance / AML / EDD / DSA / Moderation / Audit
//! partial endpoints (BIT-264, parent BIT-258).
//!
//! Covers 26 previously-partial endpoints:
//!
//! **AML** (4):
//!   GET /api/v1/aml-dsa/aml/assessments
//!   GET /api/v1/aml-dsa/aml/assessments/{id}
//!   GET /api/v1/aml-dsa/aml/country-risks
//!   GET /api/v1/aml-dsa/aml/thresholds
//!
//! **EDD** (7):
//!   POST /api/v1/aml-dsa/edd
//!   GET  /api/v1/aml-dsa/edd/{id}
//!   POST /api/v1/aml-dsa/edd/{id}/documents
//!   POST /api/v1/aml-dsa/edd/{id}/documents/{doc_id}/verify
//!   POST /api/v1/aml-dsa/edd/{id}/notes
//!   POST /api/v1/aml-dsa/edd/{id}/complete
//!   GET  /api/v1/aml-dsa/edd/pending
//!
//! **DSA** (5):
//!   GET  /api/v1/aml-dsa/dsa/reports
//!   POST /api/v1/aml-dsa/dsa/reports
//!   GET  /api/v1/aml-dsa/dsa/reports/{id}
//!   POST /api/v1/aml-dsa/dsa/reports/{id}/publish
//!   GET  /api/v1/aml-dsa/dsa/metrics
//!
//! **Moderation** (7):
//!   GET  /api/v1/aml-dsa/moderation/queue
//!   GET  /api/v1/aml-dsa/moderation/queue/stats
//!   GET  /api/v1/aml-dsa/moderation/cases/{id}
//!   POST /api/v1/aml-dsa/moderation/cases/{id}/assign
//!   POST /api/v1/aml-dsa/moderation/cases/{id}/unassign
//!   POST /api/v1/aml-dsa/moderation/cases/{id}/appeal/decide
//!   GET  /api/v1/aml-dsa/moderation/templates
//!
//! **Compliance audit logs** (4):
//!   GET /api/v1/compliance/audit-logs
//!   GET /api/v1/compliance/audit-logs/summary
//!   GET /api/v1/compliance/audit-logs/user/{user_id}
//!   GET /api/v1/compliance/audit-logs/integrity
//!
//! Primary assertion: the **role gate** (the dominant blast-radius risk for
//! these high-sensitivity endpoints) rejects under-privileged callers with 403.
//! Secondary assertions: authorised callers get 200 with the expected body shape
//! (keys present), and idempotent lookups return 404 for unknown IDs.

use axum::http::StatusCode;
use chrono::Utc;
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{seed_membership, seed_org, TestApp};

// ---------------------------------------------------------------------------
// JWT helpers — mirrors the pattern in aml_dsa_audit_logging_tests.rs
// ---------------------------------------------------------------------------

const JWT_SECRET: &str = "test-secret-key-that-is-at-least-64-characters-long-for-testing-purposes";

#[derive(Serialize)]
struct TestClaims {
    sub: Uuid,
    exp: i64,
    iat: i64,
    token_type: String,
    tenant_id: Option<Uuid>,
    role: Option<String>,
    email: String,
    name: String,
}

/// Mint an access JWT for `user_id` with the given org and role.
fn mint_token(user_id: Uuid, org_id: Option<Uuid>, role: Option<&str>) -> String {
    let now = Utc::now().timestamp();
    let claims = TestClaims {
        sub: user_id,
        iat: now,
        exp: now + 3600,
        token_type: "access".to_string(),
        tenant_id: org_id,
        role: role.map(str::to_string),
        email: format!("{user_id}@wave1b.test"),
        name: "Wave1B Test User".to_string(),
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(JWT_SECRET.as_bytes()),
    )
    .expect("mint jwt")
}

// ---------------------------------------------------------------------------
// DB seed helpers
// ---------------------------------------------------------------------------

// `seed_org` + `seed_membership` are the shared harness fixtures
// (`crate::common`, issue #1090) — this suite used to re-declare its own
// near-identical copies. Compliance-domain seeds below stay local.

async fn seed_user_with_principal(pool: &PgPool, label: &str, principal_kind: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
           VALUES ($1, 'x', 'Wave1B User', 'active', NOW(), $2) RETURNING id"#,
    )
    .bind(format!("{label}-{}@wave1b.test", Uuid::new_v4()))
    .bind(principal_kind)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

async fn seed_user(pool: &PgPool, label: &str) -> Uuid {
    // principal_kind 'staff' is the default for agency/tenant users
    seed_user_with_principal(pool, label, "staff").await
}

/// Seed an AML assessment and return its id.
async fn seed_aml_assessment(pool: &PgPool, org_id: Uuid) -> Uuid {
    let party = Uuid::new_v4();
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO aml_risk_assessments
               (organization_id, party_id, party_type)
           VALUES ($1, $2, 'individual') RETURNING id"#,
    )
    .bind(org_id)
    .bind(party)
    .fetch_one(pool)
    .await
    .expect("seed aml assessment")
}

/// Seed an EDD record linked to an assessment and return its id.
async fn seed_edd(pool: &PgPool, org_id: Uuid, assessment_id: Uuid, initiated_by: Uuid) -> Uuid {
    let party = Uuid::new_v4();
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO edd_records
               (aml_assessment_id, organization_id, party_id, initiated_by)
           VALUES ($1, $2, $3, $4) RETURNING id"#,
    )
    .bind(assessment_id)
    .bind(org_id)
    .bind(party)
    .bind(initiated_by)
    .fetch_one(pool)
    .await
    .expect("seed edd")
}

/// Seed a moderation case and return its id.
async fn seed_moderation_case(pool: &PgPool, org_id: Uuid, owner_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO moderation_cases
               (content_type, content_id, content_owner_id, organization_id,
                report_source, status, priority)
           VALUES ('listing', $1, $2, $3, 'user', 'pending', 3) RETURNING id"#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed moderation case")
}

// ---------------------------------------------------------------------------
// Caller fixtures
//
// Every case below needs one of exactly two callers, so the setup lives in a
// fixture instead of being re-inlined per test (this suite had ~50 copies of
// the same 3-5 line preamble, which is what kept it churning):
//
//   * `unprivileged_app` — a staff user with no org/role in the token. Used by
//     the `*_requires_*_role` negatives: the role gate rejects before any
//     handler logic runs, so no org/seed data is needed.
//   * `authorized_app` — a staff user who is an active `role` member of a fresh
//     org, with a token carrying both. Used by positive/edge cases (and by the
//     DSA negatives, which are rejected later by the platform-principal gate
//     because the seeded user's `principal_kind` is still `staff`).
//
// Adding a new endpoint: give it its own section and build the caller with one
// of these fixtures — do not re-inline `TestApp::new` + seed + `mint_token`.
// ---------------------------------------------------------------------------

/// Under-privileged caller: a fresh staff user, token with no org and no role.
/// Returns `(app, token)`.
async fn unprivileged_app(pool: &PgPool, label: &str) -> (TestApp, String) {
    let app = TestApp::new(pool.clone()).await;
    let user_id = seed_user(pool, label).await;
    let token = mint_token(user_id, None, None);
    (app, token)
}

/// An authorized caller and the identifiers a case may need. Destructure only
/// the fields the test uses and ignore the rest with `..`, e.g.
/// `let Caller { app, token, org_id, .. } = authorized_app(..).await;`.
struct Caller {
    app: TestApp,
    token: String,
    /// Tenant scope — pass to `.tenant(org_id)` on requests.
    org_id: Uuid,
    /// The caller's own user id — used where a seeded row references the author.
    user_id: Uuid,
}

/// Authorized caller: a fresh org, a staff user who is an active member of it
/// with `role`, and a token carrying both the org and `role`.
async fn authorized_app(pool: &PgPool, label: &str, role: &str) -> Caller {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(pool, label).await;
    let user_id = seed_user(pool, label).await;
    seed_membership(pool, org_id, user_id, role).await;
    let token = mint_token(user_id, Some(org_id), Some(role));
    Caller {
        app,
        token,
        org_id,
        user_id,
    }
}

// ===========================================================================
// AML: list assessments
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_aml_assessments_requires_compliance_role(pool: PgPool) {
    let (app, token) = unprivileged_app(&pool, "unpriv").await;

    let resp = app
        .get("/api/v1/aml-dsa/aml/assessments")
        .bearer(&token)
        .build();
    let status = app.execute(resp).await.status;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_aml_assessments_compliance_user_gets_paginated_response(pool: PgPool) {
    let Caller {
        app, token, org_id, ..
    } = authorized_app(&pool, "aml-list", "platform_admin").await;

    // Seed one assessment so the list is non-empty
    seed_aml_assessment(&pool, org_id).await;

    let resp = app
        .get("/api/v1/aml-dsa/aml/assessments")
        .bearer(&token)
        .tenant(org_id)
        .build();
    let r = app.execute(resp).await;
    assert_eq!(r.status, StatusCode::OK);
    let body = r.json_value();
    assert!(body["assessments"].is_array(), "expected assessments array");
    assert!(body["total"].is_number(), "expected total count");
}

// ===========================================================================
// AML: get assessment
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_aml_assessment_requires_compliance_role(pool: PgPool) {
    let (app, token) = unprivileged_app(&pool, "unpriv-get").await;

    let unknown = Uuid::new_v4();
    let resp = app
        .get(&format!("/api/v1/aml-dsa/aml/assessments/{unknown}"))
        .bearer(&token)
        .build();
    assert_eq!(app.execute(resp).await.status, StatusCode::FORBIDDEN);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_aml_assessment_returns_404_for_unknown_id(pool: PgPool) {
    let Caller {
        app, token, org_id, ..
    } = authorized_app(&pool, "aml-get", "platform_admin").await;

    let unknown = Uuid::new_v4();
    let resp = app
        .get(&format!("/api/v1/aml-dsa/aml/assessments/{unknown}"))
        .bearer(&token)
        .tenant(org_id)
        .build();
    assert_eq!(app.execute(resp).await.status, StatusCode::NOT_FOUND);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_aml_assessment_returns_body_shape_for_own_assessment(pool: PgPool) {
    let Caller {
        app, token, org_id, ..
    } = authorized_app(&pool, "aml-get-ok", "platform_admin").await;

    let assessment_id = seed_aml_assessment(&pool, org_id).await;

    let resp = app
        .get(&format!("/api/v1/aml-dsa/aml/assessments/{assessment_id}"))
        .bearer(&token)
        .tenant(org_id)
        .build();
    let r = app.execute(resp).await;
    assert_eq!(r.status, StatusCode::OK);
    let body = r.json_value();
    assert_eq!(body["id"].as_str().unwrap(), assessment_id.to_string());
    assert!(body["risk_score"].is_number());
    assert!(body["status"].is_string());
}

// ===========================================================================
// AML: country risks
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_country_risks_requires_compliance_role(pool: PgPool) {
    let (app, token) = unprivileged_app(&pool, "unpriv-cr").await;
    let resp = app
        .get("/api/v1/aml-dsa/aml/country-risks")
        .bearer(&token)
        .build();
    assert_eq!(app.execute(resp).await.status, StatusCode::FORBIDDEN);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_country_risks_returns_array(pool: PgPool) {
    let Caller {
        app, token, org_id, ..
    } = authorized_app(&pool, "cr", "platform_admin").await;

    let resp = app
        .get("/api/v1/aml-dsa/aml/country-risks")
        .bearer(&token)
        .tenant(org_id)
        .build();
    let r = app.execute(resp).await;
    assert_eq!(r.status, StatusCode::OK);
    assert!(r.json_value().is_array());
}

// ===========================================================================
// AML: thresholds
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_aml_thresholds_requires_compliance_role(pool: PgPool) {
    let (app, token) = unprivileged_app(&pool, "unpriv-thr").await;
    let resp = app
        .get("/api/v1/aml-dsa/aml/thresholds")
        .bearer(&token)
        .build();
    assert_eq!(app.execute(resp).await.status, StatusCode::FORBIDDEN);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_aml_thresholds_returns_threshold_fields(pool: PgPool) {
    let Caller {
        app, token, org_id, ..
    } = authorized_app(&pool, "thr", "platform_admin").await;

    let resp = app
        .get("/api/v1/aml-dsa/aml/thresholds")
        .bearer(&token)
        .tenant(org_id)
        .build();
    let r = app.execute(resp).await;
    assert_eq!(r.status, StatusCode::OK);
    let body = r.json_value();
    assert!(body["transaction_threshold_eur"].is_number());
    assert!(body["review_threshold_score"].is_number());
    // Sanity check: statutory threshold is €10 000
    assert_eq!(body["transaction_threshold_eur"].as_i64().unwrap(), 10_000);
}

// ===========================================================================
// EDD: initiate
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn initiate_edd_requires_compliance_role(pool: PgPool) {
    let (app, token) = unprivileged_app(&pool, "unpriv-edd").await;

    let body = json!({
        "aml_assessment_id": Uuid::new_v4(),
        "party_id": Uuid::new_v4(),
        "documents_requested": ["passport"]
    });
    let resp = app
        .post("/api/v1/aml-dsa/edd")
        .bearer(&token)
        .json(body)
        .build();
    assert_eq!(app.execute(resp).await.status, StatusCode::FORBIDDEN);
}

#[ignore = "EDD initiation returns 500 - server-side bug, tracked separately"]
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn initiate_edd_creates_record_and_returns_body_shape(pool: PgPool) {
    let Caller {
        app, token, org_id, ..
    } = authorized_app(&pool, "edd-init", "platform_admin").await;

    let assessment_id = seed_aml_assessment(&pool, org_id).await;
    let party_id = Uuid::new_v4();

    let body = json!({
        "aml_assessment_id": assessment_id,
        "party_id": party_id,
        "documents_requested": ["passport", "proof_of_funds"]
    });
    let resp = app
        .post("/api/v1/aml-dsa/edd")
        .bearer(&token)
        .tenant(org_id)
        .json(body)
        .build();
    let r = app.execute(resp).await;
    assert_eq!(r.status, StatusCode::OK);
    let body = r.json_value();
    assert!(body["id"].is_string());
    assert_eq!(
        body["aml_assessment_id"].as_str().unwrap(),
        assessment_id.to_string()
    );
    assert!(body["status"].is_string());
    assert!(body["documents_requested"].is_array());
}

// ===========================================================================
// EDD: get record
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_edd_record_requires_compliance_role(pool: PgPool) {
    let (app, token) = unprivileged_app(&pool, "unpriv-edd-get").await;
    let unknown = Uuid::new_v4();
    let resp = app
        .get(&format!("/api/v1/aml-dsa/edd/{unknown}"))
        .bearer(&token)
        .build();
    assert_eq!(app.execute(resp).await.status, StatusCode::FORBIDDEN);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_edd_record_returns_404_for_unknown_id(pool: PgPool) {
    let Caller {
        app, token, org_id, ..
    } = authorized_app(&pool, "edd-get-404", "platform_admin").await;
    let unknown = Uuid::new_v4();
    let resp = app
        .get(&format!("/api/v1/aml-dsa/edd/{unknown}"))
        .bearer(&token)
        .tenant(org_id)
        .build();
    assert_eq!(app.execute(resp).await.status, StatusCode::NOT_FOUND);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_edd_record_returns_own_record(pool: PgPool) {
    let Caller {
        app,
        token,
        org_id,
        user_id,
    } = authorized_app(&pool, "edd-get-ok", "platform_admin").await;

    let assessment_id = seed_aml_assessment(&pool, org_id).await;
    let edd_id = seed_edd(&pool, org_id, assessment_id, user_id).await;

    let resp = app
        .get(&format!("/api/v1/aml-dsa/edd/{edd_id}"))
        .bearer(&token)
        .tenant(org_id)
        .build();
    let r = app.execute(resp).await;
    assert_eq!(r.status, StatusCode::OK);
    let body = r.json_value();
    assert_eq!(body["id"].as_str().unwrap(), edd_id.to_string());
    assert!(body["documents_received"].is_array());
    assert!(body["compliance_notes"].is_array());
}

// ===========================================================================
// EDD: upload document
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn upload_edd_document_requires_compliance_role(pool: PgPool) {
    let (app, token) = unprivileged_app(&pool, "unpriv-edd-doc").await;
    let unknown = Uuid::new_v4();
    let body = json!({
        "document_type": "passport",
        "original_filename": "passport.pdf",
        "file_path": "uploads/passport.pdf",
        "file_size_bytes": 12345,
        "mime_type": "application/pdf"
    });
    let resp = app
        .post(&format!("/api/v1/aml-dsa/edd/{unknown}/documents"))
        .bearer(&token)
        .json(body)
        .build();
    assert_eq!(app.execute(resp).await.status, StatusCode::FORBIDDEN);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn upload_edd_document_uploads_and_returns_doc_shape(pool: PgPool) {
    let Caller {
        app,
        token,
        org_id,
        user_id,
    } = authorized_app(&pool, "edd-doc-ok", "platform_admin").await;

    let assessment_id = seed_aml_assessment(&pool, org_id).await;
    let edd_id = seed_edd(&pool, org_id, assessment_id, user_id).await;

    let body = json!({
        "document_type": "passport",
        "original_filename": "passport.pdf",
        "file_path": "uploads/edd/passport.pdf",
        "file_size_bytes": 10240,
        "mime_type": "application/pdf"
    });
    let resp = app
        .post(&format!("/api/v1/aml-dsa/edd/{edd_id}/documents"))
        .bearer(&token)
        .tenant(org_id)
        .json(body)
        .build();
    let r = app.execute(resp).await;
    assert_eq!(r.status, StatusCode::OK);
    let body = r.json_value();
    assert!(body["id"].is_string());
    assert_eq!(body["document_type"].as_str().unwrap(), "passport");
    assert_eq!(body["verification_status"].as_str().unwrap(), "pending");
}

// ===========================================================================
// EDD: verify document
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn verify_edd_document_requires_compliance_role(pool: PgPool) {
    let (app, token) = unprivileged_app(&pool, "unpriv-edd-verify").await;
    let edd_id = Uuid::new_v4();
    let doc_id = Uuid::new_v4();
    let body = json!({ "status": "verified" });
    let resp = app
        .post(&format!(
            "/api/v1/aml-dsa/edd/{edd_id}/documents/{doc_id}/verify"
        ))
        .bearer(&token)
        .json(body)
        .build();
    assert_eq!(app.execute(resp).await.status, StatusCode::FORBIDDEN);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn verify_edd_document_updates_status(pool: PgPool) {
    let Caller {
        app,
        token,
        org_id,
        user_id,
    } = authorized_app(&pool, "edd-verify-ok", "platform_admin").await;

    let assessment_id = seed_aml_assessment(&pool, org_id).await;
    let edd_id = seed_edd(&pool, org_id, assessment_id, user_id).await;

    // Upload a doc first
    let upload_body = json!({
        "document_type": "utility_bill",
        "original_filename": "bill.pdf",
        "file_path": "uploads/edd/bill.pdf",
        "file_size_bytes": 5000,
        "mime_type": "application/pdf"
    });
    let upload_resp = app
        .post(&format!("/api/v1/aml-dsa/edd/{edd_id}/documents"))
        .bearer(&token)
        .tenant(org_id)
        .json(upload_body)
        .build();
    let upload_r = app.execute(upload_resp).await;
    assert_eq!(upload_r.status, StatusCode::OK);
    let doc_id_str = upload_r.json_value()["id"].as_str().unwrap().to_string();

    // Now verify it
    let verify_body = json!({ "status": "verified" });
    let resp = app
        .post(&format!(
            "/api/v1/aml-dsa/edd/{edd_id}/documents/{doc_id_str}/verify"
        ))
        .bearer(&token)
        .tenant(org_id)
        .json(verify_body)
        .build();
    let r = app.execute(resp).await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(
        r.json_value()["verification_status"].as_str().unwrap(),
        "verified"
    );
}

// ===========================================================================
// EDD: add note
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn add_edd_note_requires_compliance_role(pool: PgPool) {
    let (app, token) = unprivileged_app(&pool, "unpriv-edd-note").await;
    let unknown = Uuid::new_v4();
    let body = json!({ "content": "Test note" });
    let resp = app
        .post(&format!("/api/v1/aml-dsa/edd/{unknown}/notes"))
        .bearer(&token)
        .json(body)
        .build();
    assert_eq!(app.execute(resp).await.status, StatusCode::FORBIDDEN);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn add_edd_note_stores_note_and_returns_body(pool: PgPool) {
    let Caller {
        app,
        token,
        org_id,
        user_id,
    } = authorized_app(&pool, "edd-note-ok", "platform_admin").await;

    let assessment_id = seed_aml_assessment(&pool, org_id).await;
    let edd_id = seed_edd(&pool, org_id, assessment_id, user_id).await;

    let body = json!({ "content": "Verified source of funds via bank statement." });
    let resp = app
        .post(&format!("/api/v1/aml-dsa/edd/{edd_id}/notes"))
        .bearer(&token)
        .tenant(org_id)
        .json(body)
        .build();
    let r = app.execute(resp).await;
    assert_eq!(r.status, StatusCode::OK);
    let body = r.json_value();
    assert!(body["id"].is_string());
    assert_eq!(
        body["content"].as_str().unwrap(),
        "Verified source of funds via bank statement."
    );

    // DB side-effect: note appended to compliance_notes JSONB array
    let notes: serde_json::Value = sqlx::query_scalar(
        "SELECT COALESCE(compliance_notes, '[]'::jsonb) FROM edd_records WHERE id = $1",
    )
    .bind(edd_id)
    .fetch_one(&pool)
    .await
    .expect("fetch notes");
    assert_eq!(
        notes.as_array().unwrap().len(),
        1,
        "note must be persisted in JSONB array"
    );
}

// ===========================================================================
// EDD: complete
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn complete_edd_requires_compliance_role(pool: PgPool) {
    let (app, token) = unprivileged_app(&pool, "unpriv-edd-comp").await;
    let unknown = Uuid::new_v4();
    let resp = app
        .post(&format!("/api/v1/aml-dsa/edd/{unknown}/complete"))
        .bearer(&token)
        .build();
    assert_eq!(app.execute(resp).await.status, StatusCode::FORBIDDEN);
}

#[ignore = "EDD complete returns 500 - server-side bug, tracked separately"]
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn complete_edd_transitions_status_to_completed(pool: PgPool) {
    let Caller {
        app,
        token,
        org_id,
        user_id,
    } = authorized_app(&pool, "edd-complete-ok", "platform_admin").await;

    let assessment_id = seed_aml_assessment(&pool, org_id).await;
    let edd_id = seed_edd(&pool, org_id, assessment_id, user_id).await;

    let resp = app
        .post(&format!("/api/v1/aml-dsa/edd/{edd_id}/complete"))
        .bearer(&token)
        .tenant(org_id)
        .build();
    let r = app.execute(resp).await;
    assert_eq!(r.status, StatusCode::OK);
    let body = r.json_value();
    assert_eq!(body["id"].as_str().unwrap(), edd_id.to_string());
    assert_eq!(body["status"].as_str().unwrap(), "completed");

    // DB side-effect: completed_at set
    let completed_at: Option<chrono::DateTime<Utc>> =
        sqlx::query_scalar("SELECT completed_at FROM edd_records WHERE id = $1")
            .bind(edd_id)
            .fetch_one(&pool)
            .await
            .expect("fetch edd");
    assert!(completed_at.is_some(), "completed_at must be set");
}

// ===========================================================================
// EDD: list pending
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_pending_edd_requires_compliance_role(pool: PgPool) {
    let (app, token) = unprivileged_app(&pool, "unpriv-edd-pend").await;
    let resp = app
        .get("/api/v1/aml-dsa/edd/pending")
        .bearer(&token)
        .build();
    assert_eq!(app.execute(resp).await.status, StatusCode::FORBIDDEN);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_pending_edd_returns_own_org_records(pool: PgPool) {
    let Caller {
        app,
        token,
        org_id,
        user_id,
    } = authorized_app(&pool, "edd-pend-ok", "platform_admin").await;
    let other_org = seed_org(&pool, "edd-pend-other").await;

    let my_assessment = seed_aml_assessment(&pool, org_id).await;
    let _my_edd = seed_edd(&pool, org_id, my_assessment, user_id).await;

    // Other org's EDD — must NOT appear
    let other_user = seed_user(&pool, "other-edd-user").await;
    let other_assessment = seed_aml_assessment(&pool, other_org).await;
    let _other_edd = seed_edd(&pool, other_org, other_assessment, other_user).await;

    let resp = app
        .get("/api/v1/aml-dsa/edd/pending")
        .bearer(&token)
        .tenant(org_id)
        .build();
    let r = app.execute(resp).await;
    assert_eq!(r.status, StatusCode::OK);
    let arr = r.json_value();
    assert!(arr.is_array());
    // Ensure no cross-org leakage: the other_user's EDD belongs to other_org
    // and must not appear in the result set. Verify by checking initiated_by.
    for record in arr.as_array().unwrap() {
        let initiated_by_str = record["initiated_by"].as_str().unwrap_or("");
        assert_ne!(
            initiated_by_str,
            other_user.to_string(),
            "cross-org EDD record leaked into response"
        );
    }
}

// ===========================================================================
// DSA: non-platform callers are rejected on all DSA endpoints
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_dsa_reports_requires_platform_role(pool: PgPool) {
    // The token carries platform_admin, but the seeded user's principal_kind
    // stays `staff`, so the `RequestPrincipal` platform gate rejects with 403.
    let Caller { app, token, .. } = authorized_app(&pool, "dsa-list", "platform_admin").await;

    let resp = app
        .get("/api/v1/aml-dsa/dsa/reports")
        .bearer(&token)
        .build();
    // RequestPrincipal gates on principal_kind column, not JWT role
    assert_eq!(app.execute(resp).await.status, StatusCode::FORBIDDEN);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn generate_dsa_report_requires_platform_role(pool: PgPool) {
    let Caller { app, token, .. } = authorized_app(&pool, "dsa-gen", "platform_admin").await;

    let body = json!({
        "period_start": "2025-01-01T00:00:00Z",
        "period_end": "2025-03-31T23:59:59Z"
    });
    let resp = app
        .post("/api/v1/aml-dsa/dsa/reports")
        .bearer(&token)
        .json(body)
        .build();
    assert_eq!(app.execute(resp).await.status, StatusCode::FORBIDDEN);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_dsa_report_requires_platform_role(pool: PgPool) {
    let Caller { app, token, .. } = authorized_app(&pool, "dsa-get", "platform_admin").await;

    let unknown = Uuid::new_v4();
    let resp = app
        .get(&format!("/api/v1/aml-dsa/dsa/reports/{unknown}"))
        .bearer(&token)
        .build();
    assert_eq!(app.execute(resp).await.status, StatusCode::FORBIDDEN);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn publish_dsa_report_requires_platform_role(pool: PgPool) {
    let Caller { app, token, .. } = authorized_app(&pool, "dsa-pub", "platform_admin").await;

    let unknown = Uuid::new_v4();
    let resp = app
        .post(&format!("/api/v1/aml-dsa/dsa/reports/{unknown}/publish"))
        .bearer(&token)
        .build();
    assert_eq!(app.execute(resp).await.status, StatusCode::FORBIDDEN);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_dsa_metrics_requires_platform_role(pool: PgPool) {
    let Caller { app, token, .. } = authorized_app(&pool, "dsa-metrics", "platform_admin").await;

    let resp = app
        .get("/api/v1/aml-dsa/dsa/metrics")
        .bearer(&token)
        .build();
    assert_eq!(app.execute(resp).await.status, StatusCode::FORBIDDEN);
}

// ===========================================================================
// Moderation: queue
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_moderation_queue_requires_moderator_role(pool: PgPool) {
    let (app, token) = unprivileged_app(&pool, "unpriv-queue").await;
    let resp = app
        .get("/api/v1/aml-dsa/moderation/queue")
        .bearer(&token)
        .build();
    assert_eq!(app.execute(resp).await.status, StatusCode::FORBIDDEN);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_moderation_queue_manager_gets_paginated_cases(pool: PgPool) {
    let Caller {
        app, token, org_id, ..
    } = authorized_app(&pool, "queue-ok", "manager").await;

    let owner_id = seed_user(&pool, "case-owner-q").await;
    seed_moderation_case(&pool, org_id, owner_id).await;

    let resp = app
        .get("/api/v1/aml-dsa/moderation/queue")
        .bearer(&token)
        .tenant(org_id)
        .build();
    let r = app.execute(resp).await;
    assert_eq!(r.status, StatusCode::OK);
    let body = r.json_value();
    assert!(body["cases"].is_array(), "expected cases array in response");
}

// ===========================================================================
// Moderation: queue stats
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_moderation_stats_requires_moderator_role(pool: PgPool) {
    let (app, token) = unprivileged_app(&pool, "unpriv-stats").await;
    let resp = app
        .get("/api/v1/aml-dsa/moderation/queue/stats")
        .bearer(&token)
        .build();
    assert_eq!(app.execute(resp).await.status, StatusCode::FORBIDDEN);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_moderation_stats_returns_stats_shape(pool: PgPool) {
    let Caller {
        app, token, org_id, ..
    } = authorized_app(&pool, "stats-ok", "manager").await;

    let resp = app
        .get("/api/v1/aml-dsa/moderation/queue/stats")
        .bearer(&token)
        .tenant(org_id)
        .build();
    let r = app.execute(resp).await;
    assert_eq!(r.status, StatusCode::OK);
    let body = r.json_value();
    assert!(body["pending_count"].is_number(), "expected pending_count");
    assert!(
        body["under_review_count"].is_number(),
        "expected under_review_count"
    );
}

// ===========================================================================
// Moderation: get case
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_moderation_case_requires_moderator_role(pool: PgPool) {
    let (app, token) = unprivileged_app(&pool, "unpriv-case").await;
    let unknown = Uuid::new_v4();
    let resp = app
        .get(&format!("/api/v1/aml-dsa/moderation/cases/{unknown}"))
        .bearer(&token)
        .build();
    assert_eq!(app.execute(resp).await.status, StatusCode::FORBIDDEN);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_moderation_case_returns_case_shape(pool: PgPool) {
    let Caller {
        app, token, org_id, ..
    } = authorized_app(&pool, "case-get-ok", "manager").await;

    let owner_id = seed_user(&pool, "case-owner-get").await;
    let case_id = seed_moderation_case(&pool, org_id, owner_id).await;

    let resp = app
        .get(&format!("/api/v1/aml-dsa/moderation/cases/{case_id}"))
        .bearer(&token)
        .tenant(org_id)
        .build();
    let r = app.execute(resp).await;
    assert_eq!(r.status, StatusCode::OK);
    let body = r.json_value();
    assert_eq!(body["id"].as_str().unwrap(), case_id.to_string());
    assert!(body["status"].is_string());
    assert!(body["priority"].is_number());
}

// ===========================================================================
// Moderation: assign case
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn assign_moderation_case_requires_moderator_role(pool: PgPool) {
    let (app, token) = unprivileged_app(&pool, "unpriv-assign").await;
    let unknown = Uuid::new_v4();
    let body = json!({ "assignee_id": Uuid::new_v4() });
    let resp = app
        .post(&format!(
            "/api/v1/aml-dsa/moderation/cases/{unknown}/assign"
        ))
        .bearer(&token)
        .json(body)
        .build();
    assert_eq!(app.execute(resp).await.status, StatusCode::FORBIDDEN);
}

// ===========================================================================
// Moderation: unassign case (regression for #3163)
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn unassign_moderation_case_requires_moderator_role(pool: PgPool) {
    // Regression for #3163: the `/moderation/cases/{id}/unassign` route was
    // missing from `routes::aml_dsa::router()`, so the client's
    // `unassignModerationCase` call 404'd at runtime. A *missing* route yields
    // 404; a *mounted-but-gated* route yields 403. Asserting 403 here pins the
    // route's existence so it can't silently regress to a 404 again.
    let (app, token) = unprivileged_app(&pool, "unpriv-unassign").await;
    let unknown = Uuid::new_v4();
    let resp = app
        .post(&format!(
            "/api/v1/aml-dsa/moderation/cases/{unknown}/unassign"
        ))
        .bearer(&token)
        .build();
    assert_eq!(app.execute(resp).await.status, StatusCode::FORBIDDEN);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn unassign_moderation_case_clears_assignee_and_reverts_status(pool: PgPool) {
    let Caller {
        app, token, org_id, ..
    } = authorized_app(&pool, "unassign-ok", "manager").await;

    let owner_id = seed_user(&pool, "case-owner-unassign").await;
    let case_id = seed_moderation_case(&pool, org_id, owner_id).await;

    // Assign to self first: status moves pending -> under_review, assignee set.
    let assign = app
        .post(&format!(
            "/api/v1/aml-dsa/moderation/cases/{case_id}/assign"
        ))
        .bearer(&token)
        .tenant(org_id)
        .json(json!({}))
        .build();
    assert_eq!(app.execute(assign).await.status, StatusCode::OK);

    // Unassign: clears the assignee and reverts under_review -> pending.
    let unassign = app
        .post(&format!(
            "/api/v1/aml-dsa/moderation/cases/{case_id}/unassign"
        ))
        .bearer(&token)
        .tenant(org_id)
        .build();
    let r = app.execute(unassign).await;
    assert_eq!(r.status, StatusCode::OK);
    let body = r.json_value();
    assert_eq!(body["id"].as_str().unwrap(), case_id.to_string());
    assert!(body["assigned_to_name"].is_null());

    // DB is the source of truth: assignee cleared, status back to pending
    // (PG enum text is snake_case regardless of the JSON serialization).
    let (assigned_to, status): (Option<Uuid>, String) =
        sqlx::query_as("SELECT assigned_to, status::text FROM moderation_cases WHERE id = $1")
            .bind(case_id)
            .fetch_one(&pool)
            .await
            .expect("fetch case");
    assert!(assigned_to.is_none(), "assigned_to should be cleared");
    assert_eq!(status, "pending", "status should revert to pending");
}

// Regression for #3174: a moderator-role caller hitting an unknown (or
// cross-org) case id must get 404, not 500. The repo's `UPDATE ... RETURNING`
// yields `RowNotFound` when the `id` + `organization_id` predicate matches no
// row; previously the handler mapped *every* repo error to 500. The
// `*_requires_moderator_role` tests above only reach the role gate before the
// lookup, so they don't pin this.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn assign_moderation_case_unknown_id_returns_404(pool: PgPool) {
    let Caller {
        app, token, org_id, ..
    } = authorized_app(&pool, "assign-404", "manager").await;

    let unknown = Uuid::new_v4();
    // Empty body => assign to self, so the is_member check is skipped and the
    // request reaches the repo lookup for the unknown id.
    let resp = app
        .post(&format!(
            "/api/v1/aml-dsa/moderation/cases/{unknown}/assign"
        ))
        .bearer(&token)
        .tenant(org_id)
        .json(json!({}))
        .build();
    assert_eq!(app.execute(resp).await.status, StatusCode::NOT_FOUND);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn unassign_moderation_case_unknown_id_returns_404(pool: PgPool) {
    let Caller {
        app, token, org_id, ..
    } = authorized_app(&pool, "unassign-404", "manager").await;

    let unknown = Uuid::new_v4();
    let resp = app
        .post(&format!(
            "/api/v1/aml-dsa/moderation/cases/{unknown}/unassign"
        ))
        .bearer(&token)
        .tenant(org_id)
        .build();
    assert_eq!(app.execute(resp).await.status, StatusCode::NOT_FOUND);
}

// ===========================================================================
// Moderation: decide appeal
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn decide_appeal_requires_moderator_role(pool: PgPool) {
    let (app, token) = unprivileged_app(&pool, "unpriv-appeal").await;
    let unknown = Uuid::new_v4();
    let body = json!({ "decision": "upheld", "rationale": "Justified." });
    let resp = app
        .post(&format!(
            "/api/v1/aml-dsa/moderation/cases/{unknown}/appeal/decide"
        ))
        .bearer(&token)
        .json(body)
        .build();
    assert_eq!(app.execute(resp).await.status, StatusCode::FORBIDDEN);
}

// ===========================================================================
// Moderation: action templates
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_action_templates_requires_moderator_role(pool: PgPool) {
    let (app, token) = unprivileged_app(&pool, "unpriv-tmpl").await;
    let resp = app
        .get("/api/v1/aml-dsa/moderation/templates")
        .bearer(&token)
        .build();
    assert_eq!(app.execute(resp).await.status, StatusCode::FORBIDDEN);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_action_templates_returns_array(pool: PgPool) {
    let Caller {
        app, token, org_id, ..
    } = authorized_app(&pool, "tmpl-ok", "manager").await;

    let resp = app
        .get("/api/v1/aml-dsa/moderation/templates")
        .bearer(&token)
        .tenant(org_id)
        .build();
    let r = app.execute(resp).await;
    assert_eq!(r.status, StatusCode::OK);
    assert!(r.json_value().is_array(), "expected template array");
}

// ===========================================================================
// Compliance audit logs: all require super_admin
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_audit_logs_requires_super_admin(pool: PgPool) {
    // Even platform_admin is not enough — only super_admin passes
    let Caller { app, token, .. } = authorized_app(&pool, "audit-403", "platform_admin").await;

    let resp = app
        .get("/api/v1/compliance/audit-logs")
        .bearer(&token)
        .build();
    assert_eq!(app.execute(resp).await.status, StatusCode::FORBIDDEN);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_audit_logs_super_admin_gets_paginated_response(pool: PgPool) {
    let Caller { app, token, .. } = authorized_app(&pool, "audit-ok", "super_admin").await;

    let resp = app
        .get("/api/v1/compliance/audit-logs")
        .bearer(&token)
        .build();
    let r = app.execute(resp).await;
    assert_eq!(r.status, StatusCode::OK);
    let body = r.json_value();
    assert!(body["logs"].is_array(), "expected logs array");
    assert!(body["total"].is_number(), "expected total");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_audit_log_summary_requires_super_admin(pool: PgPool) {
    let (app, token) = unprivileged_app(&pool, "unpriv-summary").await;
    let resp = app
        .get("/api/v1/compliance/audit-logs/summary")
        .bearer(&token)
        .build();
    assert_eq!(app.execute(resp).await.status, StatusCode::FORBIDDEN);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_audit_log_summary_super_admin_returns_summary(pool: PgPool) {
    let Caller { app, token, .. } = authorized_app(&pool, "audit-sum-ok", "super_admin").await;

    let resp = app
        .get("/api/v1/compliance/audit-logs/summary")
        .bearer(&token)
        .build();
    let r = app.execute(resp).await;
    assert_eq!(r.status, StatusCode::OK);
    let body = r.json_value();
    assert!(body["total_entries"].is_number());
    assert!(body["action_counts"].is_array());
    assert!(body["integrity_verified"].is_boolean());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_user_audit_logs_requires_super_admin(pool: PgPool) {
    let (app, token) = unprivileged_app(&pool, "unpriv-user-logs").await;
    let target = Uuid::new_v4();
    let resp = app
        .get(&format!("/api/v1/compliance/audit-logs/user/{target}"))
        .bearer(&token)
        .build();
    assert_eq!(app.execute(resp).await.status, StatusCode::FORBIDDEN);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_user_audit_logs_super_admin_returns_filtered_logs(pool: PgPool) {
    let Caller { app, token, .. } = authorized_app(&pool, "user-log-ok", "super_admin").await;

    let target_user = Uuid::new_v4();
    let resp = app
        .get(&format!("/api/v1/compliance/audit-logs/user/{target_user}"))
        .bearer(&token)
        .build();
    let r = app.execute(resp).await;
    assert_eq!(r.status, StatusCode::OK);
    let body = r.json_value();
    assert!(body["logs"].is_array());
}

/// Regression: `get_user_audit_logs` must return the TRUE row count for the
/// user in `total`, not the length of the current page. Previously the
/// handler set `total = log_responses.len()`, so with 3 rows and a page size
/// of 2 the paginator saw `total = 2` and hid page 2 — a compliance surface
/// where audit rows silently disappeared. Seed 3 rows, ask for a page of 2,
/// and assert the page holds 2 while `total` reports 3.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_user_audit_logs_total_reflects_full_count_not_page(pool: PgPool) {
    let Caller { app, token, .. } = authorized_app(&pool, "user-log-total", "super_admin").await;

    let target_user = seed_user(&pool, "audited-user-total").await;
    for _ in 0..3 {
        sqlx::query("INSERT INTO audit_logs (user_id, action) VALUES ($1, 'login')")
            .bind(target_user)
            .execute(&pool)
            .await
            .expect("seed audit log");
    }

    let resp = app
        .get(&format!(
            "/api/v1/compliance/audit-logs/user/{target_user}?limit=2&offset=0"
        ))
        .bearer(&token)
        .build();
    let r = app.execute(resp).await;
    assert_eq!(r.status, StatusCode::OK);
    let body = r.json_value();

    assert_eq!(
        body["logs"].as_array().map(|a| a.len()),
        Some(2),
        "page should hold the requested limit of 2 rows"
    );
    assert_eq!(
        body["total"].as_i64(),
        Some(3),
        "total must be the full user row count (3), not the page length"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn verify_audit_integrity_requires_super_admin(pool: PgPool) {
    let (app, token) = unprivileged_app(&pool, "unpriv-integrity").await;
    let resp = app
        .get("/api/v1/compliance/audit-logs/integrity")
        .bearer(&token)
        .build();
    assert_eq!(app.execute(resp).await.status, StatusCode::FORBIDDEN);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn verify_audit_integrity_super_admin_returns_integrity_report(pool: PgPool) {
    let Caller { app, token, .. } = authorized_app(&pool, "integrity-ok", "super_admin").await;

    let resp = app
        .get("/api/v1/compliance/audit-logs/integrity")
        .bearer(&token)
        .build();
    let r = app.execute(resp).await;
    assert_eq!(r.status, StatusCode::OK);
    let body = r.json_value();
    assert!(
        body["verified"].is_boolean(),
        "expected 'verified' boolean field"
    );
}

// ===========================================================================
// EDD write endpoints: authorised caller + unknown id -> 404 (not 500)
// ===========================================================================
//
// The `*_requires_compliance_role` cases above use an UNDER-privileged token,
// so they stop at the role gate (403) before the record lookup ever runs — they
// cannot pin what an *authorised* compliance officer sees when the `edd_id` is
// unknown or belongs to another org. Each EDD write handler resolves the record
// with `edd_repo.get_edd(edd_id, org_id)` and maps the `None` case to 404; any
// repo error maps to 500. These tests pin the clean 404 so a future refactor
// that collapses "not found" into the 500 arm (the exact class of bug #3174
// fixed for moderation `assign`/`unassign`) is caught. The lookup runs for an
// org-scoped id, so a cross-org record is equally invisible (404), which also
// guards tenant isolation on the write path without duplicating the dedicated
// cross-org IDOR suite's read-path coverage.

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn upload_edd_document_unknown_edd_returns_404(pool: PgPool) {
    let Caller {
        app, token, org_id, ..
    } = authorized_app(&pool, "edd-doc-404", "platform_admin").await;

    // Body must pass `validate_edd_document` (which runs BEFORE the lookup) so
    // the request reaches `get_edd` and surfaces the 404, not a 400.
    let unknown = Uuid::new_v4();
    let body = json!({
        "document_type": "passport",
        "original_filename": "passport.pdf",
        "file_path": "uploads/edd/passport.pdf",
        "file_size_bytes": 10240,
        "mime_type": "application/pdf"
    });
    let resp = app
        .post(&format!("/api/v1/aml-dsa/edd/{unknown}/documents"))
        .bearer(&token)
        .tenant(org_id)
        .json(body)
        .build();
    assert_eq!(app.execute(resp).await.status, StatusCode::NOT_FOUND);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn verify_edd_document_unknown_edd_returns_404(pool: PgPool) {
    let Caller {
        app, token, org_id, ..
    } = authorized_app(&pool, "edd-verify-404", "platform_admin").await;

    // The EDD-record lookup gates before the document is ever touched, so an
    // unknown `edd_id` must 404 regardless of the (also-unknown) `doc_id`.
    let unknown_edd = Uuid::new_v4();
    let unknown_doc = Uuid::new_v4();
    let body = json!({ "status": "verified" });
    let resp = app
        .post(&format!(
            "/api/v1/aml-dsa/edd/{unknown_edd}/documents/{unknown_doc}/verify"
        ))
        .bearer(&token)
        .tenant(org_id)
        .json(body)
        .build();
    assert_eq!(app.execute(resp).await.status, StatusCode::NOT_FOUND);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn add_edd_note_unknown_edd_returns_404(pool: PgPool) {
    let Caller {
        app, token, org_id, ..
    } = authorized_app(&pool, "edd-note-404", "platform_admin").await;

    // Content must be non-empty to clear `validate_text_field` (runs before the
    // lookup) so the request reaches `get_edd` and 404s rather than 400s.
    let unknown = Uuid::new_v4();
    let body = json!({ "content": "regression: note against a non-existent EDD record" });
    let resp = app
        .post(&format!("/api/v1/aml-dsa/edd/{unknown}/notes"))
        .bearer(&token)
        .tenant(org_id)
        .json(body)
        .build();
    assert_eq!(app.execute(resp).await.status, StatusCode::NOT_FOUND);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn complete_edd_unknown_edd_returns_404(pool: PgPool) {
    let Caller {
        app, token, org_id, ..
    } = authorized_app(&pool, "edd-complete-404", "platform_admin").await;

    // NOTE: the `complete_edd` *happy path* is `#[ignore]`d for a server-side
    // 500 in the repo-level `complete_edd` call. The unknown-id lookup fails
    // earlier, at `get_edd().ok_or(NOT_FOUND)`, so this path is 404 and pins
    // that the 500 bug lives strictly in the state-transition call, not the
    // existence check.
    let unknown = Uuid::new_v4();
    let resp = app
        .post(&format!("/api/v1/aml-dsa/edd/{unknown}/complete"))
        .bearer(&token)
        .tenant(org_id)
        .build();
    assert_eq!(app.execute(resp).await.status, StatusCode::NOT_FOUND);
}

// ===========================================================================
// DSA: platform-principal positive paths (first authorised-caller coverage)
// ===========================================================================
//
// Every DSA case above is negative-only — it proves a tenant-scoped principal
// (JWT role platform_admin but `users.principal_kind = 'staff'`) is rejected by
// the `RequestPrincipal` platform gate with 403. None exercises the authorised
// side, so a regression that broke the platform-principal accept arm (or the
// read itself) would pass unnoticed. A user whose `principal_kind = 'platform'`
// reaching these handlers with no resolved tenant hits the
// `(None, Platform) => effective_org = None` arm and is allowed; these two
// read-only endpoints then return 200 against an empty dataset.

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_dsa_reports_platform_principal_returns_array(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    // `principal_kind = 'platform'` is what the DSA gate requires; the JWT role
    // is irrelevant because `RequestPrincipal` re-derives the kind from the
    // trusted `users` column and trusts only `sub` from the token. No tenant
    // header: a platform operator has no org context.
    let user_id = seed_user_with_principal(&pool, "plat-dsa-list", "platform").await;
    let token = mint_token(user_id, None, None);

    let resp = app
        .get("/api/v1/aml-dsa/dsa/reports")
        .bearer(&token)
        .build();
    let r = app.execute(resp).await;
    assert_eq!(r.status, StatusCode::OK);
    assert!(
        r.json_value().is_array(),
        "expected a DSA reports array for a platform principal"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_dsa_metrics_platform_principal_returns_metrics(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user_id = seed_user_with_principal(&pool, "plat-dsa-metrics", "platform").await;
    let token = mint_token(user_id, None, None);

    let resp = app
        .get("/api/v1/aml-dsa/dsa/metrics")
        .bearer(&token)
        .build();
    let r = app.execute(resp).await;
    assert_eq!(r.status, StatusCode::OK);
    let body = r.json_value();
    assert!(body["pending_cases"].is_number(), "expected pending_cases");
    assert!(
        body["sla_compliance_rate"].is_number(),
        "expected sla_compliance_rate"
    );
    // Empty dataset: no cases => the handler's `else` branch reports full SLA.
    assert_eq!(
        body["sla_compliance_rate"].as_f64(),
        Some(100.0),
        "empty queue must report 100% SLA compliance"
    );
}
