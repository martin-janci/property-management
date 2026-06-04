//! Regression tests for `bug-esignature-webhook-idempotency-guard` (Story 84.2).
//!
//! E-signature providers deliver webhooks at-least-once, so a single
//! `completed` / `declined` event can arrive multiple times. Before the
//! terminal-state idempotency guard, a duplicate delivery for an
//! already-finalized signature request would re-run all side effects:
//! re-store the signed document, churn signer status, and re-fire the
//! requester completion/decline email.
//!
//! These tests POST a webhook to
//! `/api/v1/signature-requests/webhook/{provider}` for a request that is
//! already in a terminal state (`completed`) and assert:
//!
//! | Case | Expected |
//! |------|----------|
//! | W1 | Duplicate `completed` webhook → 200, "already finalized" ack, no signer mutation |
//! | W2 | Duplicate `declined` signer event on a terminal request → 200, signer status untouched |
//!
//! The webhook endpoint is unauthenticated; authority comes from the
//! `X-Webhook-Secret` header validated against `ESIGN_WEBHOOK_SECRET`. We pin
//! that env var so the handler accepts our forged-but-authentic deliveries.

#[allow(dead_code)]
mod common;

use axum::http::StatusCode;
use common::TestApp;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

/// Shared webhook secret pinned for the duration of the test process.
const TEST_WEBHOOK_SECRET: &str = "esign-webhook-idempotency-test-secret-0123456789";

/// Ensure `ESIGN_WEBHOOK_SECRET` is set before the handler reads it.
fn ensure_webhook_secret() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        std::env::set_var("ESIGN_WEBHOOK_SECRET", TEST_WEBHOOK_SECRET);
    });
}

/// Seed org + user + document + a signature request in the given status with a
/// single signer in the given signer status. Returns
/// (request_id, provider, provider_request_id, signer_email).
async fn seed_terminal_request(
    pool: &PgPool,
    status: &str,
    signer_status: &str,
) -> (Uuid, String, String, String) {
    let slug = format!("esign-wh-{}", Uuid::new_v4());
    let org_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO organizations (name, slug, contact_email, status)
           VALUES ('ESign WH Org', $1, 'esign-wh@example.com', 'active')
           RETURNING id"#,
    )
    .bind(&slug)
    .fetch_one(pool)
    .await
    .expect("seed org");

    let user_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO users (email, password_hash, name, status, email_verified_at)
           VALUES ($1, 'test_hash', 'ESign User', 'active', NOW())
           RETURNING id"#,
    )
    .bind(format!("esign-wh-{}@example.com", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("seed user");

    let doc_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO documents
             (organization_id, title, category, file_key, file_name,
              mime_type, size_bytes, access_scope, created_by)
           VALUES ($1, 'Lease Agreement', 'contracts', 'k', 'lease.pdf',
                   'application/pdf', 1, 'organization', $2)
           RETURNING id"#,
    )
    .bind(org_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("seed document");

    let signer_email = format!("signer-{}@example.com", Uuid::new_v4());
    let signers = json!([{
        "email": signer_email,
        "name": "Alice Buyer",
        "order": 0,
        "status": signer_status
    }]);

    let provider = "lightweight".to_string();
    let provider_request_id = format!("prov-req-{}", Uuid::new_v4());

    let request_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO signature_requests
             (document_id, organization_id, status, subject, message, signers,
              provider, provider_request_id, created_by)
           VALUES ($1, $2, $3::signature_request_status, 'Please sign the lease',
                   'Sign at your earliest convenience', $4, $5, $6, $7)
           RETURNING id"#,
    )
    .bind(doc_id)
    .bind(org_id)
    .bind(status)
    .bind(&signers)
    .bind(&provider)
    .bind(&provider_request_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("seed signature_request");

    (request_id, provider, provider_request_id, signer_email)
}

/// Read the current signer status for the single seeded signer.
async fn signer_status(pool: &PgPool, request_id: Uuid) -> String {
    sqlx::query_scalar::<_, String>(
        r#"SELECT signers->0->>'status' FROM signature_requests WHERE id = $1"#,
    )
    .bind(request_id)
    .fetch_one(pool)
    .await
    .expect("read signer status")
}

// ===========================================================================
// W1 — duplicate `completed` webhook on a completed request is a no-op
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn duplicate_completed_webhook_on_terminal_request_is_noop(pool: PgPool) {
    ensure_webhook_secret();
    let app = TestApp::new(pool.clone()).await;

    // Request already finalized as `completed`, signer already `signed`.
    let (request_id, provider, provider_request_id, signer_email) =
        seed_terminal_request(&pool, "completed", "signed").await;

    let before = signer_status(&pool, request_id).await;
    assert_eq!(before, "signed", "precondition: signer is already signed");

    let resp = app
        .execute(
            app.post(&format!("/api/v1/signature-requests/webhook/{}", provider))
                .header("x-webhook-secret", TEST_WEBHOOK_SECRET)
                .json(json!({
                    "event_type": "completed",
                    "provider_request_id": provider_request_id,
                    "signer_email": signer_email,
                    "signer_status": "signed",
                    "signed_document_url": "https://provider.example/signed.pdf"
                }))
                .build(),
        )
        .await;

    // Idempotent acknowledgement: 200, success, and the guard message so the
    // provider stops retrying.
    resp.assert_status(StatusCode::OK);
    let body = resp.json_value();
    assert_eq!(body.get("success"), Some(&json!(true)));
    assert_eq!(
        body.get("message"),
        Some(&json!(
            "Signature request already finalized; webhook ignored"
        )),
        "expected the terminal-state guard message, got: {body}"
    );

    // No side effects: signer status unchanged, no signed document linked.
    assert_eq!(
        signer_status(&pool, request_id).await,
        "signed",
        "duplicate webhook must not mutate signer status"
    );
    let signed_doc: Option<Uuid> =
        sqlx::query_scalar(r#"SELECT signed_document_id FROM signature_requests WHERE id = $1"#)
            .bind(request_id)
            .fetch_one(&pool)
            .await
            .expect("read signed_document_id");
    assert!(
        signed_doc.is_none(),
        "duplicate webhook must not store a signed document on a terminal request"
    );
}

// ===========================================================================
// W2 — a signer `declined` event on an already-terminal request is ignored
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn signer_event_on_terminal_request_does_not_retransition(pool: PgPool) {
    ensure_webhook_secret();
    let app = TestApp::new(pool.clone()).await;

    // Request finalized as `completed`; signer recorded as `signed`. A late /
    // duplicate `declined` event must not flip the signer back.
    let (request_id, provider, provider_request_id, signer_email) =
        seed_terminal_request(&pool, "completed", "signed").await;

    let resp = app
        .execute(
            app.post(&format!("/api/v1/signature-requests/webhook/{}", provider))
                .header("x-webhook-secret", TEST_WEBHOOK_SECRET)
                .json(json!({
                    "event_type": "signer_declined",
                    "provider_request_id": provider_request_id,
                    "signer_email": signer_email,
                    "signer_status": "declined",
                    "decline_reason": "changed my mind"
                }))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);

    assert_eq!(
        signer_status(&pool, request_id).await,
        "signed",
        "terminal request must not re-transition signer status on a late event"
    );
}
