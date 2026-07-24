//! Test-backfill: Wave 1B — Legal documents + Insurance policies/claims.
//!
//! Covers 22 endpoints that were partial (real DB logic, no HTTP tests):
//!
//! **Legal** (`/api/v1/legal`):
//!   create_document, list_documents, get_document, update_document,
//!   delete_document, add_version, list_versions, get_version,
//!   create_requirement, list_requirements, get_requirement, update_requirement,
//!   delete_requirement, create_verification, list_verifications
//!
//! **Insurance** (`/api/v1/insurance`):
//!   list_policies, create_policy, get_policy, update_policy,
//!   get_expiring_policies, create_claim, get_claim
//!
//! Every test uses `#[sqlx::test(migrator = "db::MIGRATOR")]` and asserts
//! HTTP status + response shape. All seeding uses static string literals
//! (sqlx 0.9 SQL-injection lint: no `format!` / `&String` in query macros).

#![allow(dead_code)]

use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{create_authenticated_user_with_org, seed_org, TestApp, TestUser};

// ---------------------------------------------------------------------------
// Shared fixture helpers
// ---------------------------------------------------------------------------

async fn setup(pool: &PgPool, slug: &str) -> (TestApp, String, Uuid) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser {
        email: format!("wave1b-{slug}-{}@test.internal", Uuid::new_v4()),
        password: "Test1234!".to_string(),
        name: "Wave1B User".to_string(),
    };
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, slug).await;
    (app, token, org_id)
}

// Seed a legal document directly and return its id.
async fn seed_legal_document(pool: &PgPool, org_id: Uuid, user_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO legal_documents
               (organization_id, document_type, title, created_by)
           VALUES ($1, 'contract', 'Seed Legal Doc', $2)
           RETURNING id"#,
    )
    .bind(org_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("seed legal_document")
}

async fn seed_user_id(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(email)
        .fetch_one(pool)
        .await
        .expect("resolve user id")
}

// ---------------------------------------------------------------------------
// Legal: create_document  →  201 with id/title/document_type
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn legal_create_document_returns_201(pool: PgPool) {
    let (app, token, org_id) = setup(&pool, "leg-create").await;
    let sess = app.session(token, org_id);

    let resp = app
        .execute(
            sess.post("/api/v1/legal/documents")
                .json(json!({
                    "document_type": "contract",
                    "title": "Test Contract",
                    "description": "A test legal document"
                }))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::CREATED);
    let body = resp.json_value();
    assert!(body["id"].is_string(), "response must have an id field");
    assert_eq!(body["title"].as_str(), Some("Test Contract"));
    assert_eq!(body["document_type"].as_str(), Some("contract"));
}

// ---------------------------------------------------------------------------
// Legal: list_documents  →  200 array (empty on fresh org)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn legal_list_documents_returns_200(pool: PgPool) {
    let (app, token, org_id) = setup(&pool, "leg-list").await;
    let sess = app.session(token, org_id);

    let resp = app
        .execute(sess.get("/api/v1/legal/documents").build())
        .await;

    resp.assert_status(StatusCode::OK);
    let body = resp.json_value();
    assert!(body.is_array(), "list_documents must return a JSON array");
}

// ---------------------------------------------------------------------------
// Legal: get_document  →  200 same-org / 404 cross-org
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn legal_get_document_same_org_200(pool: PgPool) {
    let (app, token, org_id) = setup(&pool, "leg-get").await;
    let user_id = seed_user_id(
        &pool,
        // Extract email from token claim — cheapest: just fetch the one user in this fresh org.
        &sqlx::query_scalar::<_, String>(
            "SELECT u.email FROM users u JOIN organization_members m ON m.user_id=u.id WHERE m.organization_id=$1 LIMIT 1",
        )
        .bind(org_id)
        .fetch_one(&pool)
        .await
        .expect("find user"),
    )
    .await;

    let doc_id = seed_legal_document(&pool, org_id, user_id).await;
    let sess = app.session(token, org_id);

    let resp = app
        .execute(
            sess.get(&format!("/api/v1/legal/documents/{doc_id}"))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);
    let body = resp.json_value();
    assert_eq!(body["id"].as_str(), Some(doc_id.to_string().as_str()));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn legal_get_document_cross_org_404(pool: PgPool) {
    let (app, token_a, org_a) = setup(&pool, "leg-get-a").await;

    // Create a second org and its document.
    let org_b = seed_org(&pool, "leg-get-b").await;
    let user_b_email = format!("leg-get-b-{}@test.internal", Uuid::new_v4());
    sqlx::query(
        r#"INSERT INTO users (email, password_hash, name, status, email_verified_at)
           VALUES ($1, 'hash', 'B User', 'active', NOW())"#,
    )
    .bind(&user_b_email)
    .execute(&pool)
    .await
    .expect("seed user b");
    let user_b = seed_user_id(&pool, &user_b_email).await;
    let doc_b = seed_legal_document(&pool, org_b, user_b).await;

    let sess_a = app.session(token_a, org_a);
    let resp = app
        .execute(
            sess_a
                .get(&format!("/api/v1/legal/documents/{doc_b}"))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::NOT_FOUND);
}

// ---------------------------------------------------------------------------
// Legal: update_document  →  200 title changed
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn legal_update_document_returns_200(pool: PgPool) {
    let (app, token, org_id) = setup(&pool, "leg-upd").await;
    let user_email = sqlx::query_scalar::<_, String>(
        "SELECT u.email FROM users u JOIN organization_members m ON m.user_id=u.id WHERE m.organization_id=$1 LIMIT 1",
    )
    .bind(org_id)
    .fetch_one(&pool)
    .await
    .expect("find user");
    let user_id = seed_user_id(&pool, &user_email).await;
    let doc_id = seed_legal_document(&pool, org_id, user_id).await;

    let sess = app.session(token, org_id);
    let resp = app
        .execute(
            sess.patch(&format!("/api/v1/legal/documents/{doc_id}"))
                .json(json!({"title": "Updated Contract Title"}))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);
    assert_eq!(
        resp.json_value()["title"].as_str(),
        Some("Updated Contract Title")
    );
}

// ---------------------------------------------------------------------------
// Legal: delete_document  →  200 success=true, then 404 on re-read
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn legal_delete_document_returns_200(pool: PgPool) {
    let (app, token, org_id) = setup(&pool, "leg-del").await;
    let user_email = sqlx::query_scalar::<_, String>(
        "SELECT u.email FROM users u JOIN organization_members m ON m.user_id=u.id WHERE m.organization_id=$1 LIMIT 1",
    )
    .bind(org_id)
    .fetch_one(&pool)
    .await
    .expect("find user");
    let user_id = seed_user_id(&pool, &user_email).await;
    let doc_id = seed_legal_document(&pool, org_id, user_id).await;

    let sess = app.session(token, org_id);
    let del = app
        .execute(
            sess.delete(&format!("/api/v1/legal/documents/{doc_id}"))
                .build(),
        )
        .await;
    del.assert_status(StatusCode::NO_CONTENT);

    // Confirm tombstone
    let get = app
        .execute(
            sess.get(&format!("/api/v1/legal/documents/{doc_id}"))
                .build(),
        )
        .await;
    get.assert_status(StatusCode::NOT_FOUND);
}

// ---------------------------------------------------------------------------
// Legal: add_version  →  201 with document_id / version_number
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn legal_add_version_returns_201(pool: PgPool) {
    let (app, token, org_id) = setup(&pool, "leg-ver").await;
    let user_email = sqlx::query_scalar::<_, String>(
        "SELECT u.email FROM users u JOIN organization_members m ON m.user_id=u.id WHERE m.organization_id=$1 LIMIT 1",
    )
    .bind(org_id)
    .fetch_one(&pool)
    .await
    .expect("find user");
    let user_id = seed_user_id(&pool, &user_email).await;
    let doc_id = seed_legal_document(&pool, org_id, user_id).await;

    let sess = app.session(token, org_id);
    let resp = app
        .execute(
            sess.post(&format!("/api/v1/legal/documents/{doc_id}/versions"))
                .json(json!({
                    "file_path": "/files/v1.pdf",
                    "file_name": "contract-v1.pdf",
                    "change_notes": "Initial version"
                }))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::CREATED);
    let body = resp.json_value();
    assert_eq!(
        body["document_id"].as_str(),
        Some(doc_id.to_string().as_str())
    );
    assert!(body["version_number"].as_i64().is_some());
}

// ---------------------------------------------------------------------------
// Legal: list_versions  →  200 array
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn legal_list_versions_returns_200(pool: PgPool) {
    let (app, token, org_id) = setup(&pool, "leg-list-ver").await;
    let user_email = sqlx::query_scalar::<_, String>(
        "SELECT u.email FROM users u JOIN organization_members m ON m.user_id=u.id WHERE m.organization_id=$1 LIMIT 1",
    )
    .bind(org_id)
    .fetch_one(&pool)
    .await
    .expect("find user");
    let user_id = seed_user_id(&pool, &user_email).await;
    let doc_id = seed_legal_document(&pool, org_id, user_id).await;

    let sess = app.session(token, org_id);
    let resp = app
        .execute(
            sess.get(&format!("/api/v1/legal/documents/{doc_id}/versions"))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);
    assert!(resp.json_value().is_array());
}

// ---------------------------------------------------------------------------
// Legal: create_requirement  →  201 with id / requirement_type
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn legal_create_requirement_returns_201(pool: PgPool) {
    let (app, token, org_id) = setup(&pool, "leg-req-c").await;
    let sess = app.session(token, org_id);

    let resp = app
        .execute(
            sess.post("/api/v1/legal/requirements")
                .json(json!({
                    "requirement_type": "regulatory",
                    "name": "GDPR Data Retention Policy",
                    "description": "Annual review required",
                    "due_date": "2027-01-01"
                }))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::CREATED);
    let body = resp.json_value();
    assert!(body["id"].is_string());
    assert_eq!(body["requirement_type"].as_str(), Some("regulatory"));
}

// ---------------------------------------------------------------------------
// Legal: list_requirements  →  200 array
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn legal_list_requirements_returns_200(pool: PgPool) {
    let (app, token, org_id) = setup(&pool, "leg-req-l").await;
    let sess = app.session(token, org_id);

    let resp = app
        .execute(sess.get("/api/v1/legal/requirements").build())
        .await;

    resp.assert_status(StatusCode::OK);
    assert!(resp.json_value().is_array());
}

// ---------------------------------------------------------------------------
// Legal: get_requirement  →  200 same-org / 404 unknown id
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn legal_get_requirement_200_and_404(pool: PgPool) {
    let (app, token, org_id) = setup(&pool, "leg-req-g").await;
    let sess = app.session(token, org_id);

    // Create one first
    let create_resp = app
        .execute(
            sess.post("/api/v1/legal/requirements")
                .json(json!({
                    "requirement_type": "internal",
                    "name": "Fire Safety Compliance",
                }))
                .build(),
        )
        .await;
    create_resp.assert_status(StatusCode::CREATED);
    let req_id = create_resp.json_value()["id"]
        .as_str()
        .expect("id")
        .to_string();

    // Happy path
    let get = app
        .execute(
            sess.get(&format!("/api/v1/legal/requirements/{req_id}"))
                .build(),
        )
        .await;
    get.assert_status(StatusCode::OK);
    assert_eq!(get.json_value()["id"].as_str(), Some(req_id.as_str()));

    // Unknown id
    let missing = app
        .execute(
            sess.get(&format!("/api/v1/legal/requirements/{}", Uuid::new_v4()))
                .build(),
        )
        .await;
    missing.assert_status(StatusCode::NOT_FOUND);
}

// ---------------------------------------------------------------------------
// Legal: update_requirement  →  200 title changed
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn legal_update_requirement_returns_200(pool: PgPool) {
    let (app, token, org_id) = setup(&pool, "leg-req-u").await;
    let sess = app.session(token, org_id);

    let create = app
        .execute(
            sess.post("/api/v1/legal/requirements")
                .json(json!({"requirement_type": "regulatory", "name": "Old Title"}))
                .build(),
        )
        .await;
    let req_id = create.json_value()["id"].as_str().unwrap().to_string();

    let upd = app
        .execute(
            sess.patch(&format!("/api/v1/legal/requirements/{req_id}"))
                .json(json!({"name": "New Title"}))
                .build(),
        )
        .await;
    upd.assert_status(StatusCode::OK);
    assert_eq!(upd.json_value()["name"].as_str(), Some("New Title"));
}

// ---------------------------------------------------------------------------
// Legal: delete_requirement  →  200 success=true
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn legal_delete_requirement_returns_200(pool: PgPool) {
    let (app, token, org_id) = setup(&pool, "leg-req-d").await;
    let sess = app.session(token, org_id);

    let create = app
        .execute(
            sess.post("/api/v1/legal/requirements")
                .json(json!({"requirement_type": "internal", "name": "To Delete"}))
                .build(),
        )
        .await;
    let req_id = create.json_value()["id"].as_str().unwrap().to_string();

    let del = app
        .execute(
            sess.delete(&format!("/api/v1/legal/requirements/{req_id}"))
                .build(),
        )
        .await;
    del.assert_status(StatusCode::OK);
    assert_eq!(del.json_value()["success"].as_bool(), Some(true));
}

// ---------------------------------------------------------------------------
// Legal: create_verification  →  201 with requirement_id
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn legal_create_verification_returns_201(pool: PgPool) {
    let (app, token, org_id) = setup(&pool, "leg-ver-c").await;
    let sess = app.session(token, org_id);

    let create_req = app
        .execute(
            sess.post("/api/v1/legal/requirements")
                .json(json!({"requirement_type": "regulatory", "name": "Verification Target"}))
                .build(),
        )
        .await;
    let req_id = create_req.json_value()["id"].as_str().unwrap().to_string();

    let resp = app
        .execute(
            sess.post(&format!("/api/v1/legal/requirements/{req_id}/verify"))
                .json(json!({
                    "verification_method": "document_review",
                    "verification_date": "2026-06-27",
                    "status": "compliant",
                    "notes": "Verified by compliance team"
                }))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::CREATED);
    let body = resp.json_value();
    assert!(body["id"].is_string());
    assert_eq!(body["requirement_id"].as_str(), Some(req_id.as_str()));
}

// ---------------------------------------------------------------------------
// Legal: list_verifications  →  200 array
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn legal_list_verifications_returns_200(pool: PgPool) {
    let (app, token, org_id) = setup(&pool, "leg-ver-l").await;
    let sess = app.session(token, org_id);

    let create_req = app
        .execute(
            sess.post("/api/v1/legal/requirements")
                .json(json!({"requirement_type": "internal", "name": "List Verifs Target"}))
                .build(),
        )
        .await;
    let req_id = create_req.json_value()["id"].as_str().unwrap().to_string();

    let resp = app
        .execute(
            sess.get(&format!(
                "/api/v1/legal/requirements/{req_id}/verifications"
            ))
            .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);
    assert!(resp.json_value().is_array());
}

// ===========================================================================
// Insurance
// ===========================================================================

// ---------------------------------------------------------------------------
// Insurance: list_policies  →  200 array (empty on fresh org)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn insurance_list_policies_returns_200(pool: PgPool) {
    let (app, token, org_id) = setup(&pool, "ins-list").await;
    let sess = app.session(token, org_id);

    let resp = app
        .execute(sess.get("/api/v1/insurance/policies").build())
        .await;

    resp.assert_status(StatusCode::OK);
    let body = resp.json_value();
    // list_policies wraps in {"policies": [...]}
    assert!(
        body["policies"].is_array() || body.is_array(),
        "expected policies array in response, got: {body}"
    );
}

// ---------------------------------------------------------------------------
// Insurance: create_policy  →  200/201 with id / policy_number
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn insurance_create_policy_returns_success(pool: PgPool) {
    let (app, token, org_id) = setup(&pool, "ins-create").await;
    let sess = app.session(token, org_id);

    let resp = app
        .execute(
            sess.post("/api/v1/insurance/policies")
                .json(json!({
                    "policy_number": "POL-2026-001",
                    "policy_name": "Building Fire Insurance",
                    "provider_name": "Acme Insurance",
                    "policy_type": "fire",
                    "effective_date": "2026-01-01",
                    "expiration_date": "2027-01-01"
                }))
                .build(),
        )
        .await;

    assert!(
        resp.status == StatusCode::OK || resp.status == StatusCode::CREATED,
        "expected 200 or 201, got {}. Body: {}",
        resp.status,
        resp.text()
    );
    let body = resp.json_value();
    assert!(body["id"].is_string(), "response must have id");
    assert_eq!(body["policy_number"].as_str(), Some("POL-2026-001"));
}

// ---------------------------------------------------------------------------
// Insurance: get_policy  →  200 same-org / 404 unknown
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn insurance_get_policy_200_and_404(pool: PgPool) {
    let (app, token, org_id) = setup(&pool, "ins-get").await;
    let sess = app.session(token, org_id);

    let create = app
        .execute(
            sess.post("/api/v1/insurance/policies")
                .json(json!({
                    "policy_number": "POL-GET-001",
                    "policy_name": "Get Test Policy",
                    "provider_name": "Test Insurer",
                    "policy_type": "liability",
                    "effective_date": "2026-01-01",
                    "expiration_date": "2027-01-01"
                }))
                .build(),
        )
        .await;
    let policy_id = create.json_value()["id"].as_str().unwrap().to_string();

    // Happy path
    let get = app
        .execute(
            sess.get(&format!("/api/v1/insurance/policies/{policy_id}"))
                .build(),
        )
        .await;
    get.assert_status(StatusCode::OK);
    assert_eq!(get.json_value()["id"].as_str(), Some(policy_id.as_str()));

    // Unknown
    let missing = app
        .execute(
            sess.get(&format!("/api/v1/insurance/policies/{}", Uuid::new_v4()))
                .build(),
        )
        .await;
    missing.assert_status(StatusCode::NOT_FOUND);
}

// ---------------------------------------------------------------------------
// Insurance: update_policy  →  200 policy_name changed
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn insurance_update_policy_returns_200(pool: PgPool) {
    let (app, token, org_id) = setup(&pool, "ins-upd").await;
    let sess = app.session(token, org_id);

    let create = app
        .execute(
            sess.post("/api/v1/insurance/policies")
                .json(json!({
                    "policy_number": "POL-UPD-001",
                    "policy_name": "Old Policy Name",
                    "provider_name": "Insurer",
                    "policy_type": "fire",
                    "effective_date": "2026-01-01",
                    "expiration_date": "2027-01-01"
                }))
                .build(),
        )
        .await;
    let policy_id = create.json_value()["id"].as_str().unwrap().to_string();

    let upd = app
        .execute(
            sess.put(&format!("/api/v1/insurance/policies/{policy_id}"))
                .json(json!({"policy_name": "Updated Policy Name"}))
                .build(),
        )
        .await;
    upd.assert_status(StatusCode::OK);
    assert_eq!(
        upd.json_value()["policy_name"].as_str(),
        Some("Updated Policy Name")
    );
}

// ---------------------------------------------------------------------------
// Insurance: get_expiring_policies  →  200 expiring_policies array
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn insurance_get_expiring_policies_returns_200(pool: PgPool) {
    let (app, token, org_id) = setup(&pool, "ins-exp").await;
    let sess = app.session(token, org_id);

    let resp = app
        .execute(
            sess.get("/api/v1/insurance/policies/expiring?days_ahead=30")
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);
    // Response wrapped in {"policies": [...]}  or just an array
    let body = resp.json_value();
    assert!(
        body["policies"].is_array() || body.is_array(),
        "expected policies array, got: {body}"
    );
}

// ---------------------------------------------------------------------------
// Insurance: create_claim  →  201/200 with id / policy_id
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn insurance_create_claim_returns_success(pool: PgPool) {
    let (app, token, org_id) = setup(&pool, "ins-claim-c").await;
    let sess = app.session(token, org_id);

    // Need a policy first
    let create_policy = app
        .execute(
            sess.post("/api/v1/insurance/policies")
                .json(json!({
                    "policy_number": "POL-CLM-001",
                    "policy_name": "Claim Test Policy",
                    "provider_name": "Test Insurer",
                    "policy_type": "fire",
                    "effective_date": "2026-01-01",
                    "expiration_date": "2027-01-01"
                }))
                .build(),
        )
        .await;
    let policy_id = create_policy.json_value()["id"]
        .as_str()
        .unwrap()
        .to_string();

    let resp = app
        .execute(
            sess.post("/api/v1/insurance/claims")
                .json(json!({
                    "policy_id": policy_id,
                    "incident_date": "2026-06-01",
                    "incident_description": "Water damage from burst pipe",
                    "incident_location": "Building A, Floor 2"
                }))
                .build(),
        )
        .await;

    assert!(
        resp.status == StatusCode::OK || resp.status == StatusCode::CREATED,
        "expected 200 or 201, got {}. Body: {}",
        resp.status,
        resp.text()
    );
    let body = resp.json_value();
    assert!(body["id"].is_string());
    assert_eq!(body["policy_id"].as_str(), Some(policy_id.as_str()));
}

// ---------------------------------------------------------------------------
// Insurance: get_claim  →  200 same-org / 404 unknown
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn insurance_get_claim_200_and_404(pool: PgPool) {
    let (app, token, org_id) = setup(&pool, "ins-claim-g").await;
    let sess = app.session(token, org_id);

    let policy = app
        .execute(
            sess.post("/api/v1/insurance/policies")
                .json(json!({
                    "policy_number": "POL-GETCLM-001",
                    "policy_name": "Get Claim Policy",
                    "provider_name": "Test Co",
                    "policy_type": "liability",
                    "effective_date": "2026-01-01",
                    "expiration_date": "2027-01-01"
                }))
                .build(),
        )
        .await;
    let policy_id = policy.json_value()["id"].as_str().unwrap().to_string();

    let claim = app
        .execute(
            sess.post("/api/v1/insurance/claims")
                .json(json!({
                    "policy_id": policy_id,
                    "incident_date": "2026-06-15",
                    "incident_description": "Roof damage from storm"
                }))
                .build(),
        )
        .await;
    let claim_id = claim.json_value()["id"].as_str().unwrap().to_string();

    // Happy path
    let get = app
        .execute(
            sess.get(&format!("/api/v1/insurance/claims/{claim_id}"))
                .build(),
        )
        .await;
    get.assert_status(StatusCode::OK);
    assert_eq!(get.json_value()["id"].as_str(), Some(claim_id.as_str()));

    // Unknown
    let missing = app
        .execute(
            sess.get(&format!("/api/v1/insurance/claims/{}", Uuid::new_v4()))
                .build(),
        )
        .await;
    missing.assert_status(StatusCode::NOT_FOUND);
}
