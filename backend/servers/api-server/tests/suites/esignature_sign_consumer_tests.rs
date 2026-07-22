//! HTTP-level tests for the signer-facing `/sign` consumer endpoint
//! (issue #761 part 2).
//!
//! These exercise the full router (`create_router`) end to end:
//!
//!   GET  /api/v1/signatures/sign?token=...  — render context (no consume)
//!   POST /api/v1/signatures/sign?token=...  — record signature (consumes)
//!
//! Invariants asserted:
//!
//! | Case | Expected |
//! |------|----------|
//! | S1 | A valid link renders context, then signs once (200 → completed) |
//! | S2 | Replaying the POST after a successful sign is rejected (409) |
//! | S3 | A token with a tampered HMAC is rejected (401) |
//! | S4 | An expired token is rejected (410) |
//! | S5 | A nonce that was never issued is rejected (401) |
//!
//! The endpoint is unauthenticated — all authority is in the HMAC token. The
//! server's `ESIGN_PROVIDER` reads `ESIGN_TOKEN_SECRET` from the environment
//! once (LazyLock); we pin that secret here and mint tokens with a provider
//! that shares it, so server-side verification matches.

#![allow(dead_code)]

use std::time::Duration;

use crate::common::TestApp;
use axum::http::StatusCode;
use db::repositories::ESignatureNonceRepository;
use integrations::{LightweightConfig, LightweightProvider};
use sqlx::PgPool;
use uuid::Uuid;

/// Fixed HMAC secret shared by the in-test token minter and the server's
/// `ESIGN_PROVIDER`. ≥32 bytes to clear the production floor.
const TEST_ESIGN_SECRET: &str = "esign-sign-consumer-test-secret-0123456789";

/// Ensure `ESIGN_TOKEN_SECRET` is set before the server's `ESIGN_PROVIDER`
/// LazyLock is first touched (i.e. before the first `/sign` request).
fn ensure_esign_secret() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        if std::env::var("ESIGN_TOKEN_SECRET").is_err() {
            std::env::set_var("ESIGN_TOKEN_SECRET", TEST_ESIGN_SECRET);
        }
    });
}

/// Build a provider that mints tokens with the same secret the server uses.
/// `ttl` lets a test mint an already-expiring token (S4).
fn minter(ttl_secs: u64) -> LightweightProvider {
    LightweightProvider::new(LightweightConfig {
        token_secret: TEST_ESIGN_SECRET.as_bytes().to_vec(),
        previous_token_secret: None,
        base_url: "http://localhost:3000".to_string(),
        webhook_url: None,
        token_ttl_secs: ttl_secs,
    })
}

/// Seed org + user + document + a pending signature request with one signer.
/// Returns (request_id, org_id, signer_email).
async fn seed_request_with_signer(pool: &PgPool, signer_email: &str) -> (Uuid, Uuid, String) {
    let slug = format!("esign-sign-{}", Uuid::new_v4());
    let org_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO organizations (name, slug, contact_email, status)
           VALUES ('ESign Sign Org', $1, 'esign-sign@example.com', 'active')
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
    .bind(format!("esign-{}@example.com", Uuid::new_v4()))
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

    // Single signer, JSONB roster. `status: sent` mirrors a freshly-emailed
    // invitation; `is_complete()` is false so signing is allowed.
    let signers = serde_json::json!([{
        "email": signer_email,
        "name": "Alice Buyer",
        "order": 0,
        "status": "sent"
    }]);

    let request_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO signature_requests
             (document_id, organization_id, status, subject, message, signers, created_by)
           VALUES ($1, $2, 'pending', 'Please sign the lease',
                   'Sign at your earliest convenience', $3, $4)
           RETURNING id"#,
    )
    .bind(doc_id)
    .bind(org_id)
    .bind(&signers)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("seed signature_request");

    (request_id, org_id, signer_email.to_string())
}

/// Mint a signing token for the request and record its nonce as ISSUED
/// (mirrors what `create_signature_request` / `send_reminder` do at send
/// time). Returns the encoded token string.
async fn issue_token(
    pool: &PgPool,
    provider: &LightweightProvider,
    request_id: Uuid,
    org_id: Uuid,
    signer_email: &str,
) -> String {
    let signed = provider
        .build_signing_url(
            signer_email,
            &request_id.to_string(),
            &org_id.to_string(),
            "sent",
        )
        .expect("build signing url");

    ESignatureNonceRepository::new(pool.clone())
        .record_nonce(request_id, signed.nonce)
        .await
        .expect("record nonce (issue)");

    // Extract the encoded token from the `?token=` query.
    signed
        .url
        .split("token=")
        .nth(1)
        .expect("token in url")
        .to_string()
}

// ===========================================================================
// S1 + S2 — happy path renders + signs once; replay rejected
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn valid_link_renders_and_signs_once_then_replay_rejected(pool: PgPool) {
    ensure_esign_secret();
    let app = TestApp::new(pool.clone()).await;
    let email = "alice@example.com";
    let (request_id, org_id, _) = seed_request_with_signer(&pool, email).await;
    let token = issue_token(&pool, &minter(3600), request_id, org_id, email).await;

    // GET render context — read-only, no consume.
    let get_resp = app
        .execute(
            app.get(&format!("/api/v1/signatures/sign?token={token}"))
                .build(),
        )
        .await;
    get_resp.assert_status(StatusCode::OK);
    let ctx = get_resp.json_value();
    assert_eq!(ctx["request_id"], request_id.to_string());
    assert_eq!(ctx["signer_email"], email);

    // A second GET must STILL be OK — GET never consumes the nonce.
    app.execute(
        app.get(&format!("/api/v1/signatures/sign?token={token}"))
            .build(),
    )
    .await
    .assert_status(StatusCode::OK);

    // POST — record the signature. One signer total, so the request completes.
    let post_resp = app
        .execute(
            app.post(&format!("/api/v1/signatures/sign?token={token}"))
                .json(serde_json::json!({ "typed_name": "Alice Buyer" }))
                .build(),
        )
        .await;
    post_resp.assert_status(StatusCode::OK);
    assert_eq!(post_resp.json_value()["status"], "completed");

    // S2: replay the SAME token → 409 (nonce already consumed).
    app.execute(
        app.post(&format!("/api/v1/signatures/sign?token={token}"))
            .json(serde_json::json!({}))
            .build(),
    )
    .await
    .assert_status(StatusCode::CONFLICT);

    // And the request is recorded as completed in the DB.
    let status: String = sqlx::query_scalar::<_, String>(
        "SELECT status::text FROM signature_requests WHERE id = $1",
    )
    .bind(request_id)
    .fetch_one(&pool)
    .await
    .expect("read request status");
    assert_eq!(status, "completed");
}

// ===========================================================================
// S3 — tampered HMAC rejected
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn tampered_token_rejected(pool: PgPool) {
    ensure_esign_secret();
    let app = TestApp::new(pool.clone()).await;
    let email = "bob@example.com";
    let (request_id, org_id, _) = seed_request_with_signer(&pool, email).await;
    let token = issue_token(&pool, &minter(3600), request_id, org_id, email).await;

    // Flip the last character of the base64url payload — the MAC no longer
    // matches the (decoded) fields.
    let mut chars: Vec<char> = token.chars().collect();
    let last = chars.len() - 1;
    chars[last] = if chars[last] == 'A' { 'B' } else { 'A' };
    let tampered: String = chars.into_iter().collect();

    app.execute(
        app.get(&format!("/api/v1/signatures/sign?token={tampered}"))
            .build(),
    )
    .await
    .assert_status(StatusCode::UNAUTHORIZED);
}

// ===========================================================================
// S4 — expired token rejected (410 GONE)
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn expired_token_rejected(pool: PgPool) {
    ensure_esign_secret();
    let app = TestApp::new(pool.clone()).await;
    let email = "carol@example.com";
    let (request_id, org_id, _) = seed_request_with_signer(&pool, email).await;

    // ttl = 0 → expires_at == issue time. Sleep just past it so verify sees
    // `now > expires_at`.
    let token = issue_token(&pool, &minter(0), request_id, org_id, email).await;
    tokio::time::sleep(Duration::from_millis(1100)).await;

    app.execute(
        app.get(&format!("/api/v1/signatures/sign?token={token}"))
            .build(),
    )
    .await
    .assert_status(StatusCode::GONE);
}

// ===========================================================================
// S5 — valid HMAC but nonce never issued → rejected
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn unissued_nonce_rejected(pool: PgPool) {
    ensure_esign_secret();
    let app = TestApp::new(pool.clone()).await;
    let email = "dave@example.com";
    let (request_id, org_id, _) = seed_request_with_signer(&pool, email).await;

    // Mint a perfectly valid token but do NOT record its nonce — simulating a
    // forged-but-somehow-signed link, or one whose issuance row was never
    // written. The consumer must fail closed.
    let provider = minter(3600);
    let signed = provider
        .build_signing_url(email, &request_id.to_string(), &org_id.to_string(), "sent")
        .expect("build url");
    let token = signed.url.split("token=").nth(1).unwrap().to_string();

    // GET → 401 (nonce not issued).
    app.execute(
        app.get(&format!("/api/v1/signatures/sign?token={token}"))
            .build(),
    )
    .await
    .assert_status(StatusCode::UNAUTHORIZED);

    // POST → also 401.
    app.execute(
        app.post(&format!("/api/v1/signatures/sign?token={token}"))
            .json(serde_json::json!({}))
            .build(),
    )
    .await
    .assert_status(StatusCode::UNAUTHORIZED);
}
