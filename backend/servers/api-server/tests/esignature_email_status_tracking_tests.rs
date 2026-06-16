//! Integration tests for the e-signature email-send + status-tracking flow
//! (Epic 84.2, coverage 84-2).
//!
//! The existing `esignature_*` suites cover the webhook idempotency guard
//! (terminal-state replays) and the signer-facing `/sign` consumer. What was
//! NOT yet pinned end-to-end is the *status-tracking lifecycle* a signature
//! request moves through as provider webhooks land:
//!
//! ```text
//!   send  ─▶ sent ─▶ viewed ─▶ signed     (request: pending → in_progress → completed)
//!                          └─▶ declined   (request: declined)
//! ```
//!
//! These tests POST provider webhooks to
//! `/api/v1/signature-requests/webhook/{provider}` and assert that each signer
//! status transition is **persisted** (JSONB `signers[*].status`), that the
//! derived **request status** advances correctly, that the linked document's
//! `signature_status` tracks alongside, and that the persisted state is
//! **surfaced** back through `GET /api/v1/signature-requests/{id}` via
//! `signer_counts`.
//!
//! | Case | Flow | Expected |
//! |------|------|----------|
//! | T1 | sent → viewed → signed (single signer) | request: pending → pending → completed; doc: pending → signed |
//! | T2 | sent → declined | signer declined; request: declined; doc: none |
//! | T3 | two signers: one signs, request goes in_progress; both sign → completed | partial then completed; counts surfaced |
//! | T4 | viewed transition is persisted and surfaced via GET signer_counts | viewed=1 |
//!
//! Harness mirrors `esignature_webhook_idempotency_tests.rs`: the webhook
//! endpoint is unauthenticated and authority comes from the `X-Webhook-Secret`
//! header validated against `ESIGN_WEBHOOK_SECRET`, which we pin so the handler
//! accepts our forged-but-authentic deliveries. Email sending is disabled in
//! the `TestApp` default config (`EmailService` uses the no-op Log transport),
//! so the requester completion/decline email send returns `Ok` without an SMTP
//! server — exercising the send code path without external dependencies.

#[allow(dead_code)]
mod common;

use axum::http::StatusCode;
use common::{create_authenticated_user_with_org, TestApp, TestResponse, TestUser};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

/// Shared webhook secret pinned for the duration of the test process.
const TEST_WEBHOOK_SECRET: &str = "esign-email-status-tracking-test-secret-0123456789";

/// Provider slug used for the seeded requests. The `lightweight` provider is the
/// in-tree default and needs no external configuration.
const PROVIDER: &str = "lightweight";

/// A seeded signature request plus the identifiers a webhook payload must carry
/// to address it, and the auth context needed to read it back through the API.
struct SeededRequest {
    /// `signature_requests.id` — used to read back state and call GET.
    id: Uuid,
    /// `documents.id` linked to the request — used to read `signature_status`.
    document_id: Uuid,
    /// Provider-side request id the webhook references.
    provider_request_id: String,
    /// Emails of the seeded signers, in roster order.
    signer_emails: Vec<String>,
    /// Bearer access token for the org-admin requester (for the GET surface).
    access_token: String,
    /// Organization id (the `X-Tenant-ID` header value for the GET surface).
    org_id: Uuid,
}

/// Ensure `ESIGN_WEBHOOK_SECRET` is set before the handler reads it.
fn ensure_webhook_secret() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        std::env::set_var("ESIGN_WEBHOOK_SECRET", TEST_WEBHOOK_SECRET);
    });
}

/// Register + log in an org-admin, then seed a document + a `pending` signature
/// request (with `signer_count` signers, each starting in the `sent` status —
/// the invitation email has been dispatched, the start of the status-tracking
/// lifecycle) under that user's org.
///
/// The request's `created_by` is the authenticated user so the GET surface
/// (which runs under `RlsConnection` → requires Bearer + `X-Tenant-ID` +
/// membership) can read it back.
async fn seed_pending_request(app: &TestApp, signer_count: usize) -> SeededRequest {
    let pool = &app.pool;
    let user = TestUser::new();
    let (access_token, org_id) =
        create_authenticated_user_with_org(app, &user, "esign-status").await;

    let user_id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(&user.email)
        .fetch_one(pool)
        .await
        .expect("resolve user id");

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

    // Link the document to a (placeholder) signature request so the
    // `documents.signature_status` write inside `update_signer_status` (which
    // targets `WHERE signature_request_id = $1`) has a row to hit. We set the
    // FK below after we know the request id.
    let signer_emails: Vec<String> = (0..signer_count)
        .map(|i| format!("signer-{i}-{}@example.com", Uuid::new_v4()))
        .collect();
    let signers: Vec<Value> = signer_emails
        .iter()
        .enumerate()
        .map(|(i, email)| {
            json!({
                "email": email,
                "name": format!("Signer {i}"),
                "order": i,
                "status": "sent"
            })
        })
        .collect();
    let signers = json!(signers);

    let provider_request_id = format!("prov-req-{}", Uuid::new_v4());

    let request_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO signature_requests
             (document_id, organization_id, status, subject, message, signers,
              provider, provider_request_id, created_by)
           VALUES ($1, $2, 'pending'::signature_request_status, 'Please sign the lease',
                   'Sign at your earliest convenience', $3, $4, $5, $6)
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

    // Point the document at the request and mark it pending, mirroring what
    // `SignatureRequestRepository::create` does on the real send path. The
    // status-tracking writes downstream key off `signature_request_id`.
    sqlx::query(
        r#"UPDATE documents
           SET signature_status = 'pending', signature_request_id = $1
           WHERE id = $2"#,
    )
    .bind(request_id)
    .bind(doc_id)
    .execute(pool)
    .await
    .expect("link document to signature request");

    SeededRequest {
        id: request_id,
        document_id: doc_id,
        provider_request_id,
        signer_emails,
        access_token,
        org_id,
    }
}

/// POST a signer-status webhook delivery for `PROVIDER` with the pinned secret.
async fn post_signer_event(
    app: &TestApp,
    provider_request_id: &str,
    event_type: &str,
    signer_email: &str,
    signer_status: &str,
    extra: Value,
) -> TestResponse {
    let mut payload = json!({
        "event_type": event_type,
        "provider_request_id": provider_request_id,
        "signer_email": signer_email,
        "signer_status": signer_status,
    });
    if let (Some(obj), Some(extra_obj)) = (payload.as_object_mut(), extra.as_object()) {
        for (k, v) in extra_obj {
            obj.insert(k.clone(), v.clone());
        }
    }
    app.execute(
        app.post(&format!("/api/v1/signature-requests/webhook/{PROVIDER}"))
            .header("x-webhook-secret", TEST_WEBHOOK_SECRET)
            .json(payload)
            .build(),
    )
    .await
}

/// Read the persisted status of the signer at `idx` in the JSONB roster.
async fn signer_status_at(pool: &PgPool, request_id: Uuid, idx: usize) -> String {
    sqlx::query_scalar::<_, String>(
        r#"SELECT signers->$2->>'status' FROM signature_requests WHERE id = $1"#,
    )
    .bind(request_id)
    .bind(idx as i32)
    .fetch_one(pool)
    .await
    .expect("read signer status")
}

/// Read the persisted request status (`pending` / `in_progress` / ...).
async fn request_status(pool: &PgPool, request_id: Uuid) -> String {
    sqlx::query_scalar::<_, String>(r#"SELECT status::text FROM signature_requests WHERE id = $1"#)
        .bind(request_id)
        .fetch_one(pool)
        .await
        .expect("read request status")
}

/// Read the linked document's `signature_status`.
async fn document_signature_status(pool: &PgPool, document_id: Uuid) -> String {
    sqlx::query_scalar::<_, String>(r#"SELECT signature_status FROM documents WHERE id = $1"#)
        .bind(document_id)
        .fetch_one(pool)
        .await
        .expect("read document signature_status")
}

/// GET the request through the authenticated API and return the `signer_counts`
/// object. The `GET /{id}` handler runs under `RlsConnection`, so it needs the
/// requester's Bearer token plus the `X-Tenant-ID` header.
async fn get_signer_counts(app: &TestApp, req: &SeededRequest) -> Value {
    let resp = app
        .execute(
            app.get(&format!("/api/v1/signature-requests/{}", req.id))
                .bearer(&req.access_token)
                .header("X-Tenant-ID", &req.org_id.to_string())
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);
    resp.json_value()
        .get("signer_counts")
        .cloned()
        .expect("signer_counts in GET response")
}

// ===========================================================================
// T1 — sent → viewed → signed lifecycle (single signer)
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn sent_viewed_signed_lifecycle_persists_and_surfaces(pool: PgPool) {
    ensure_webhook_secret();
    let app = TestApp::new(pool.clone()).await;

    let req = seed_pending_request(&app, 1).await;
    let email = req.signer_emails[0].clone();

    // Precondition: signer emailed (sent), request pending, doc pending.
    assert_eq!(signer_status_at(&pool, req.id, 0).await, "sent");
    assert_eq!(request_status(&pool, req.id).await, "pending");
    assert_eq!(
        document_signature_status(&pool, req.document_id).await,
        "pending"
    );

    // (a) viewed — non-terminal, request stays pending.
    post_signer_event(
        &app,
        &req.provider_request_id,
        "viewed",
        &email,
        "viewed",
        json!({}),
    )
    .await
    .assert_status(StatusCode::OK);

    assert_eq!(
        signer_status_at(&pool, req.id, 0).await,
        "viewed",
        "viewed transition must persist on the signer"
    );
    assert_eq!(
        request_status(&pool, req.id).await,
        "pending",
        "a viewed-but-unsigned request stays pending"
    );

    // Surfaced through GET: counts show viewed=1.
    let counts = get_signer_counts(&app, &req).await;
    assert_eq!(counts.get("viewed"), Some(&json!(1)));
    assert_eq!(counts.get("signed"), Some(&json!(0)));
    assert_eq!(counts.get("total"), Some(&json!(1)));

    // (b) signed — single signer, so the request completes.
    post_signer_event(
        &app,
        &req.provider_request_id,
        "completed",
        &email,
        "signed",
        json!({ "signed_document_url": "https://provider.example/signed.pdf" }),
    )
    .await
    .assert_status(StatusCode::OK);

    assert_eq!(signer_status_at(&pool, req.id, 0).await, "signed");
    assert_eq!(
        request_status(&pool, req.id).await,
        "completed",
        "all signers signed → request completed"
    );
    assert_eq!(
        document_signature_status(&pool, req.document_id).await,
        "signed",
        "completed request flips the document to signed"
    );

    // Surfaced through GET: signed=1, viewed back to 0.
    let counts = get_signer_counts(&app, &req).await;
    assert_eq!(counts.get("signed"), Some(&json!(1)));
    assert_eq!(counts.get("viewed"), Some(&json!(0)));
}

// ===========================================================================
// T2 — sent → declined lifecycle (single signer)
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn declined_lifecycle_persists_and_surfaces(pool: PgPool) {
    ensure_webhook_secret();
    let app = TestApp::new(pool.clone()).await;

    let req = seed_pending_request(&app, 1).await;
    let email = req.signer_emails[0].clone();

    post_signer_event(
        &app,
        &req.provider_request_id,
        "signer_declined",
        &email,
        "declined",
        json!({ "decline_reason": "terms unacceptable" }),
    )
    .await
    .assert_status(StatusCode::OK);

    assert_eq!(signer_status_at(&pool, req.id, 0).await, "declined");
    assert_eq!(
        request_status(&pool, req.id).await,
        "declined",
        "a declining signer moves the request to declined"
    );
    assert_eq!(
        document_signature_status(&pool, req.document_id).await,
        "none",
        "declined request resets the document signature_status to none"
    );

    // Decline reason is persisted on the signer.
    let reason: Option<String> = sqlx::query_scalar(
        r#"SELECT signers->0->>'declined_reason' FROM signature_requests WHERE id = $1"#,
    )
    .bind(req.id)
    .fetch_one(&pool)
    .await
    .expect("read declined_reason");
    assert_eq!(reason.as_deref(), Some("terms unacceptable"));

    // Surfaced through GET: declined=1.
    let counts = get_signer_counts(&app, &req).await;
    assert_eq!(counts.get("declined"), Some(&json!(1)));
    assert_eq!(counts.get("signed"), Some(&json!(0)));
}

// ===========================================================================
// T3 — two signers: first signs (in_progress), second signs (completed)
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn multi_signer_progress_to_completed_persists_and_surfaces(pool: PgPool) {
    ensure_webhook_secret();
    let app = TestApp::new(pool.clone()).await;

    let req = seed_pending_request(&app, 2).await;
    let first = req.signer_emails[0].clone();
    let second = req.signer_emails[1].clone();

    // First signer signs → request in_progress, document partial.
    post_signer_event(
        &app,
        &req.provider_request_id,
        "signer_signed",
        &first,
        "signed",
        json!({}),
    )
    .await
    .assert_status(StatusCode::OK);

    assert_eq!(signer_status_at(&pool, req.id, 0).await, "signed");
    assert_eq!(signer_status_at(&pool, req.id, 1).await, "sent");
    assert_eq!(
        request_status(&pool, req.id).await,
        "in_progress",
        "one of two signers signed → in_progress"
    );
    assert_eq!(
        document_signature_status(&pool, req.document_id).await,
        "partial",
        "in_progress request marks the document partial"
    );

    // Surfaced through GET: signed=1, sent=1, total=2.
    let counts = get_signer_counts(&app, &req).await;
    assert_eq!(counts.get("signed"), Some(&json!(1)));
    assert_eq!(counts.get("sent"), Some(&json!(1)));
    assert_eq!(counts.get("total"), Some(&json!(2)));

    // Second signer signs → request completed.
    post_signer_event(
        &app,
        &req.provider_request_id,
        "completed",
        &second,
        "signed",
        json!({ "signed_document_url": "https://provider.example/signed.pdf" }),
    )
    .await
    .assert_status(StatusCode::OK);

    assert_eq!(signer_status_at(&pool, req.id, 1).await, "signed");
    assert_eq!(
        request_status(&pool, req.id).await,
        "completed",
        "both signers signed → completed"
    );
    assert_eq!(
        document_signature_status(&pool, req.document_id).await,
        "signed"
    );

    let counts = get_signer_counts(&app, &req).await;
    assert_eq!(counts.get("signed"), Some(&json!(2)));
    assert_eq!(counts.get("sent"), Some(&json!(0)));
}

// ===========================================================================
// T4 — two signers: one declines after the other signed → request declined
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn multi_signer_decline_overrides_in_progress(pool: PgPool) {
    ensure_webhook_secret();
    let app = TestApp::new(pool.clone()).await;

    let req = seed_pending_request(&app, 2).await;
    let first = req.signer_emails[0].clone();
    let second = req.signer_emails[1].clone();

    // First signer signs → in_progress.
    post_signer_event(
        &app,
        &req.provider_request_id,
        "signer_signed",
        &first,
        "signed",
        json!({}),
    )
    .await
    .assert_status(StatusCode::OK);
    assert_eq!(request_status(&pool, req.id).await, "in_progress");

    // Second signer declines → a single decline moves the whole request to
    // declined, regardless of the already-collected signature.
    post_signer_event(
        &app,
        &req.provider_request_id,
        "signer_declined",
        &second,
        "declined",
        json!({ "decline_reason": "needs legal review" }),
    )
    .await
    .assert_status(StatusCode::OK);

    assert_eq!(signer_status_at(&pool, req.id, 0).await, "signed");
    assert_eq!(signer_status_at(&pool, req.id, 1).await, "declined");
    assert_eq!(
        request_status(&pool, req.id).await,
        "declined",
        "any decline takes precedence over a partial signed state"
    );
    assert_eq!(
        document_signature_status(&pool, req.document_id).await,
        "none"
    );

    // Surfaced through GET: one signed, one declined.
    let counts = get_signer_counts(&app, &req).await;
    assert_eq!(counts.get("signed"), Some(&json!(1)));
    assert_eq!(counts.get("declined"), Some(&json!(1)));
}
