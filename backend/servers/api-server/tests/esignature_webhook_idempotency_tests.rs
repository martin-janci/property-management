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
//! | W3 | Webhook for a `declined` / `expired` / `cancelled` (non-`completed`) terminal request → 200 ack, no signer mutation |
//!
//! The webhook endpoint is unauthenticated; authority comes from the
//! `X-Webhook-Secret` header validated against `ESIGN_WEBHOOK_SECRET`. We pin
//! that env var so the handler accepts our forged-but-authentic deliveries.

#![allow(dead_code)]

#[allow(dead_code)]
mod common;

use axum::http::StatusCode;
use common::{seed_org, TestApp, TestResponse};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

/// Shared webhook secret pinned for the duration of the test process.
const TEST_WEBHOOK_SECRET: &str = "esign-webhook-idempotency-test-secret-0123456789";

/// Provider slug used for the seeded requests. The `lightweight` provider is the
/// in-tree default and needs no external configuration.
const PROVIDER: &str = "lightweight";

/// Idempotent-ack message the terminal-state guard returns. Pinned here so the
/// assertion drifts loudly if `routes/signatures.rs` changes the copy.
const ALREADY_FINALIZED_MSG: &str = "Signature request already finalized; webhook ignored";

/// A seeded terminal signature request plus the identifiers a webhook payload
/// must carry to address it.
struct TerminalRequest {
    /// `signature_requests.id` — used to read back state after the webhook.
    id: Uuid,
    /// Provider-side request id the webhook references.
    provider_request_id: String,
    /// Email of the single seeded signer.
    signer_email: String,
}

/// Ensure `ESIGN_WEBHOOK_SECRET` is set before the handler reads it.
fn ensure_webhook_secret() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        std::env::set_var("ESIGN_WEBHOOK_SECRET", TEST_WEBHOOK_SECRET);
    });
}

/// Seed user + document + a signature request in the given status with a single
/// signer in the given signer status. The org is provisioned via the shared
/// `common::seed_org` helper so this file no longer carries its own org insert.
async fn seed_terminal_request(
    pool: &PgPool,
    status: &str,
    signer_status: &str,
) -> TerminalRequest {
    let org_id = seed_org(pool, "esign-wh").await;

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
    .bind(PROVIDER)
    .bind(&provider_request_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("seed signature_request");

    TerminalRequest {
        id: request_id,
        provider_request_id,
        signer_email,
    }
}

/// POST a webhook delivery for `PROVIDER` with the pinned secret header.
async fn post_webhook(app: &TestApp, payload: Value) -> TestResponse {
    app.execute(
        app.post(&format!("/api/v1/signature-requests/webhook/{PROVIDER}"))
            .header("x-webhook-secret", TEST_WEBHOOK_SECRET)
            .json(payload)
            .build(),
    )
    .await
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

/// Read the `signed_document_id` linked to a request (None when no signed
/// document has been stored).
async fn signed_document_id(pool: &PgPool, request_id: Uuid) -> Option<Uuid> {
    sqlx::query_scalar(r#"SELECT signed_document_id FROM signature_requests WHERE id = $1"#)
        .bind(request_id)
        .fetch_one(pool)
        .await
        .expect("read signed_document_id")
}

// ===========================================================================
// W1 — duplicate `completed` webhook on a completed request is a no-op
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn duplicate_completed_webhook_on_terminal_request_is_noop(pool: PgPool) {
    ensure_webhook_secret();
    let app = TestApp::new(pool.clone()).await;

    // Request already finalized as `completed`, signer already `signed`.
    let req = seed_terminal_request(&pool, "completed", "signed").await;
    assert_eq!(
        signer_status(&pool, req.id).await,
        "signed",
        "precondition: signer is already signed"
    );

    let resp = post_webhook(
        &app,
        json!({
            "event_type": "completed",
            "provider_request_id": req.provider_request_id,
            "signer_email": req.signer_email,
            "signer_status": "signed",
            "signed_document_url": "https://provider.example/signed.pdf"
        }),
    )
    .await;

    // Idempotent acknowledgement: 200, success, and the guard message so the
    // provider stops retrying.
    resp.assert_status(StatusCode::OK);
    let body = resp.json_value();
    assert_eq!(body.get("success"), Some(&json!(true)));
    assert_eq!(
        body.get("message"),
        Some(&json!(ALREADY_FINALIZED_MSG)),
        "expected the terminal-state guard message, got: {body}"
    );

    // No side effects: signer status unchanged, no signed document linked.
    assert_eq!(
        signer_status(&pool, req.id).await,
        "signed",
        "duplicate webhook must not mutate signer status"
    );
    assert!(
        signed_document_id(&pool, req.id).await.is_none(),
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
    let req = seed_terminal_request(&pool, "completed", "signed").await;

    let resp = post_webhook(
        &app,
        json!({
            "event_type": "signer_declined",
            "provider_request_id": req.provider_request_id,
            "signer_email": req.signer_email,
            "signer_status": "declined",
            "decline_reason": "changed my mind"
        }),
    )
    .await;

    resp.assert_status(StatusCode::OK);

    assert_eq!(
        signer_status(&pool, req.id).await,
        "signed",
        "terminal request must not re-transition signer status on a late event"
    );
}

// ===========================================================================
// W3 — the guard covers every terminal request status, not just `completed`
// ===========================================================================
//
// `SignatureRequestStatus::is_terminal()` is true for completed / declined /
// expired / cancelled. The W1/W2 cases above only exercise the `completed`
// branch; this case asserts the guard also short-circuits webhooks for the
// other three terminal request statuses (the task's "voided" family). A
// duplicate provider delivery for such a request must be acknowledged with a
// 200 idempotent ack and must not mutate the signer roster.
async fn terminal_status_acks_without_side_effects(pool: PgPool, request_status: &str) {
    ensure_webhook_secret();
    let app = TestApp::new(pool.clone()).await;

    // Seed a request that is already in the given terminal status. The signer
    // is left `pending` so that any (erroneous) re-application of the webhook's
    // `signer_status` would be observable as a flip to `signed`.
    let req = seed_terminal_request(&pool, request_status, "pending").await;
    assert_eq!(
        signer_status(&pool, req.id).await,
        "pending",
        "precondition: signer starts pending for status {request_status}"
    );

    let resp = post_webhook(
        &app,
        json!({
            "event_type": "completed",
            "provider_request_id": req.provider_request_id,
            "signer_email": req.signer_email,
            "signer_status": "signed",
            "signed_document_url": "https://provider.example/signed.pdf"
        }),
    )
    .await;

    // Idempotent acknowledgement regardless of which terminal status we are in.
    resp.assert_status(StatusCode::OK);
    let body = resp.json_value();
    assert_eq!(body.get("success"), Some(&json!(true)));
    assert_eq!(
        body.get("message"),
        Some(&json!(ALREADY_FINALIZED_MSG)),
        "expected the terminal-state guard message for status {request_status}, got: {body}"
    );

    // No side effects: signer roster untouched, no signed document stored.
    assert_eq!(
        signer_status(&pool, req.id).await,
        "pending",
        "webhook on a {request_status} request must not mutate signer status"
    );
    assert!(
        signed_document_id(&pool, req.id).await.is_none(),
        "webhook on a {request_status} request must not store a signed document"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn webhook_on_declined_request_is_noop(pool: PgPool) {
    terminal_status_acks_without_side_effects(pool, "declined").await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn webhook_on_expired_request_is_noop(pool: PgPool) {
    terminal_status_acks_without_side_effects(pool, "expired").await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn webhook_on_cancelled_request_is_noop(pool: PgPool) {
    terminal_status_acks_without_side_effects(pool, "cancelled").await;
}
