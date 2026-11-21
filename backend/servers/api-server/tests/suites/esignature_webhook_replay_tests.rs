//! Regression tests for the e-signature webhook replay/idempotency guard
//! (code-review finding: `api handlers esign webhook replay`).
//!
//! The webhook receiver at
//! `/api/v1/signature-requests/webhook/{provider}` authenticates deliveries with
//! a STATIC shared secret (`ESIGN_WEBHOOK_SECRET`, constant-time compared). A
//! static bearer secret proves authenticity but NOT freshness: a captured valid
//! delivery — identical bytes plus the secret header — can be replayed
//! indefinitely. The pre-existing terminal-state idempotency guard
//! (`SignatureRequestStatus::is_terminal()`) only absorbs replays once the
//! request is finalized; a delivery replayed while the request is still
//! `pending` / `in_progress` slips past it and re-runs its side effects on every
//! duplicate.
//!
//! The fix adds event-fingerprint dedupe: the first delivery of an event claims
//! a short-TTL key (Redis `SET NX` in production) and is processed; a replay
//! carrying the same fingerprint within the window is acknowledged 200 with a
//! "Duplicate webhook delivery ignored" message and NO side effects.
//!
//! Production dedupe runs against Redis, which CI does not wire, so these tests
//! install the in-memory `WebhookDedupeStore` via
//! `TestApp::with_webhook_dedupe_store` — the same Redis-in-prod /
//! in-memory-in-CI pattern the OAuth `state` store uses — and exercise the
//! handler's dedupe branch deterministically.

#![allow(dead_code)]

use crate::common::{seed_org, TestApp, TestResponse};
use axum::http::StatusCode;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

const TEST_WEBHOOK_SECRET: &str = "esign-webhook-replay-test-secret-0123456789";
const PROVIDER: &str = "lightweight";

/// Message the dedupe guard returns for a replayed delivery. Pinned so the
/// assertion drifts loudly if `routes/signatures.rs` changes the copy.
const DUPLICATE_MSG: &str = "Duplicate webhook delivery ignored";

fn ensure_webhook_secret() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        std::env::set_var("ESIGN_WEBHOOK_SECRET", TEST_WEBHOOK_SECRET);
    });
}

/// Seed a NON-terminal (`in_progress`) signature request with a single pending
/// signer, returning `(request_id, provider_request_id)`.
///
/// `in_progress` is deliberately non-terminal so the delivery does NOT short out
/// on the terminal-state guard — the replay must be caught by the new dedupe
/// guard instead.
async fn seed_non_terminal_request(pool: &PgPool) -> (Uuid, String) {
    let org_id = seed_org(pool, "esign-replay").await;

    let user_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO users (email, password_hash, name, status, email_verified_at)
           VALUES ($1, 'test_hash', 'ESign User', 'active', NOW())
           RETURNING id"#,
    )
    .bind(format!("esign-replay-{}@example.com", Uuid::new_v4()))
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

    let signers = json!([{
        "email": format!("signer-{}@example.com", Uuid::new_v4()),
        "name": "Alice Buyer",
        "order": 0,
        "status": "pending"
    }]);

    let provider_request_id = format!("prov-req-{}", Uuid::new_v4());

    let request_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO signature_requests
             (document_id, organization_id, status, subject, message, signers,
              provider, provider_request_id, created_by)
           VALUES ($1, $2, 'in_progress'::signature_request_status,
                   'Please sign the lease', 'Sign soon', $3, $4, $5, $6)
           RETURNING id"#,
    )
    .bind(doc_id)
    .bind(org_id)
    .bind(&signers)
    .bind(PROVIDER)
    .bind(&provider_request_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("seed signature_request");

    (request_id, provider_request_id)
}

async fn post_webhook(app: &TestApp, payload: &Value) -> TestResponse {
    app.execute(
        app.post(&format!("/api/v1/signature-requests/webhook/{PROVIDER}"))
            .header("x-webhook-secret", TEST_WEBHOOK_SECRET)
            .json(payload.clone())
            .build(),
    )
    .await
}

async fn request_status(pool: &PgPool, id: Uuid) -> String {
    sqlx::query_scalar::<_, String>(r#"SELECT status::text FROM signature_requests WHERE id = $1"#)
        .bind(id)
        .fetch_one(pool)
        .await
        .expect("read request status")
}

// ===========================================================================
// R1 — a replayed (duplicate) delivery on a NON-terminal request is deduped
// ===========================================================================
//
// This is the core finding. On a request that is still `in_progress`, the
// terminal-state guard does NOT fire, so WITHOUT the dedupe guard the second
// (replayed) delivery is processed again and returns the normal success copy.
// WITH the dedupe guard the replay is acknowledged 200 with the duplicate
// message and no side effects.

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn replayed_delivery_on_non_terminal_request_is_deduped(pool: PgPool) {
    ensure_webhook_secret();
    let (app, _store) = TestApp::with_webhook_dedupe_store(pool.clone()).await;

    let (request_id, provider_request_id) = seed_non_terminal_request(&pool).await;

    // A non-finalizing event (no signer_email/signer_status, not completed/voided)
    // so processing leaves the request non-terminal — the replay cannot be
    // absorbed by the terminal-state guard and must hit the dedupe guard.
    let payload = json!({
        "event_type": "delivered",
        "provider_request_id": provider_request_id,
    });

    // First delivery: processed normally.
    let first = post_webhook(&app, &payload).await;
    first.assert_status(StatusCode::OK);
    let first_body = first.json_value();
    assert_eq!(first_body.get("success"), Some(&json!(true)));
    assert_ne!(
        first_body.get("message"),
        Some(&json!(DUPLICATE_MSG)),
        "first delivery must be processed, not deduped: {first_body}"
    );

    // Precondition: still non-terminal, so the terminal-state guard is NOT what
    // catches the replay below.
    assert_eq!(
        request_status(&pool, request_id).await,
        "in_progress",
        "request must remain non-terminal after a non-finalizing event"
    );

    // Byte-for-byte replay: must be recognised as a duplicate and ignored.
    let replay = post_webhook(&app, &payload).await;
    replay.assert_status(StatusCode::OK);
    let replay_body = replay.json_value();
    assert_eq!(replay_body.get("success"), Some(&json!(true)));
    assert_eq!(
        replay_body.get("message"),
        Some(&json!(DUPLICATE_MSG)),
        "replayed delivery on a non-terminal request must be deduped, got: {replay_body}"
    );
}

// ===========================================================================
// R2 — a genuinely distinct event on the same request is NOT deduped
// ===========================================================================
//
// Dedupe keys on an event fingerprint, so two different events for the same
// request must each be processed (the guard must not swallow legitimate
// follow-up deliveries).

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn distinct_event_on_same_request_is_not_deduped(pool: PgPool) {
    ensure_webhook_secret();
    let (app, _store) = TestApp::with_webhook_dedupe_store(pool.clone()).await;

    let (_request_id, provider_request_id) = seed_non_terminal_request(&pool).await;

    let delivered = json!({
        "event_type": "delivered",
        "provider_request_id": provider_request_id,
    });
    let viewed = json!({
        "event_type": "viewed",
        "provider_request_id": provider_request_id,
    });

    // Two distinct event types => distinct fingerprints => both processed.
    let first = post_webhook(&app, &delivered).await;
    first.assert_status(StatusCode::OK);
    assert_ne!(
        first.json_value().get("message"),
        Some(&json!(DUPLICATE_MSG)),
        "first event must be processed"
    );

    let second = post_webhook(&app, &viewed).await;
    second.assert_status(StatusCode::OK);
    assert_ne!(
        second.json_value().get("message"),
        Some(&json!(DUPLICATE_MSG)),
        "a DIFFERENT event must not be deduped against the first"
    );
}
