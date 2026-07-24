//! BIT-313: happy-path coverage for the document-nested signature-request
//! endpoints (Story 7B.3 / Epic 84.2).
//!
//! These were deferred from BIT-312 because, as originally wired, the list and
//! create handlers were **unreachable**: `router()` mounted them at the bare
//! `/` under `/api/v1/signature-requests`, but both handlers extract
//! `Path(document_id): Path<Uuid>` — with no `{document_id}` segment in the
//! matched route, axum's `Path<Uuid>` extraction had nothing to bind. BIT-313
//! re-mounts them as a document sub-resource:
//!
//! - POST /api/v1/documents/{id}/signature-requests  (create_signature_request) 201
//! - GET  /api/v1/documents/{id}/signature-requests  (list_signature_requests)  200
//!
//! Setup is API-first (drive the real document handler to seed) so these tests
//! stay resilient to schema drift in the seed layer. `TestApp::new` sets
//! `RUST_ENV=development`, so the `LightweightProvider` used on the create path
//! falls back to its dev HMAC secret and signing-link generation succeeds; the
//! signer invitation email is best-effort (warn-and-continue) so it does not
//! affect the `201`.

#![allow(dead_code)]

mod common;

use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use common::{create_authenticated_user_with_org, TestApp, TestUser};

/// Create a document via the API and return its id. The org creator is an
/// `org_admin`, which clears the manager gate on document creation.
async fn create_document(app: &TestApp, token: &str, org_id: Uuid, key: &str) -> Uuid {
    let res = app
        .execute(
            app.session(token.to_string(), org_id)
                .post("/api/v1/documents")
                .json(json!({
                    "title": "Lease to sign",
                    "category": "contracts",
                    "file_key": format!("org/contracts/{key}.pdf"),
                    "file_name": format!("{key}.pdf"),
                    "mime_type": "application/pdf",
                    "size_bytes": 4096
                }))
                .build(),
        )
        .await;

    assert_eq!(res.status, StatusCode::CREATED, "body: {}", res.text());
    res.json_value()["id"]
        .as_str()
        .and_then(|s| s.parse().ok())
        .expect("created document id")
}

// ---------------------------------------------------------------------------
// signatures.rs — POST /api/v1/documents/{id}/signature-requests
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_signature_request_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "sig-create-ok").await;
    let doc_id = create_document(&app, &token, org_id, "sig-create").await;

    let res = app
        .execute(
            app.session(token, org_id)
                .post(&format!("/api/v1/documents/{doc_id}/signature-requests"))
                .json(json!({
                    "signers": [
                        { "email": "signer@example.com", "name": "Sam Signer" }
                    ],
                    "subject": "Please sign the lease"
                }))
                .build(),
        )
        .await;

    assert_eq!(res.status, StatusCode::CREATED, "body: {}", res.text());
    let body = res.json_value();
    assert!(
        body["signature_request"]["id"].is_string(),
        "response must include the created signature_request id, body: {body}"
    );
    assert_eq!(
        body["signature_request"]["document_id"].as_str(),
        Some(doc_id.to_string().as_str()),
        "signature request must be bound to the seeded document"
    );
}

// ---------------------------------------------------------------------------
// signatures.rs — GET /api/v1/documents/{id}/signature-requests
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_signature_requests_empty_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "sig-list-empty").await;
    let doc_id = create_document(&app, &token, org_id, "sig-list-empty").await;

    let res = app
        .execute(
            app.session(token, org_id)
                .get(&format!("/api/v1/documents/{doc_id}/signature-requests"))
                .build(),
        )
        .await;

    assert_eq!(res.status, StatusCode::OK, "body: {}", res.text());
    let body = res.json_value();
    assert_eq!(
        body["total"].as_i64(),
        Some(0),
        "a document with no signature requests must report total 0, body: {body}"
    );
    assert!(
        body["signature_requests"]
            .as_array()
            .is_some_and(|a| a.is_empty()),
        "signature_requests must be an empty array, body: {body}"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_signature_requests_returns_created(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "sig-list-one").await;
    let doc_id = create_document(&app, &token, org_id, "sig-list-one").await;

    // Create one signature request through the API, then list it back.
    let created = app
        .execute(
            app.session(token.clone(), org_id)
                .post(&format!("/api/v1/documents/{doc_id}/signature-requests"))
                .json(json!({
                    "signers": [
                        { "email": "signer@example.com", "name": "Sam Signer" }
                    ]
                }))
                .build(),
        )
        .await;
    assert_eq!(
        created.status,
        StatusCode::CREATED,
        "body: {}",
        created.text()
    );

    let res = app
        .execute(
            app.session(token, org_id)
                .get(&format!("/api/v1/documents/{doc_id}/signature-requests"))
                .build(),
        )
        .await;

    assert_eq!(res.status, StatusCode::OK, "body: {}", res.text());
    let body = res.json_value();
    assert_eq!(
        body["total"].as_i64(),
        Some(1),
        "list must report the one created request, body: {body}"
    );
    assert_eq!(
        body["signature_requests"][0]["document_id"].as_str(),
        Some(doc_id.to_string().as_str()),
        "listed request must belong to the seeded document, body: {body}"
    );
}
