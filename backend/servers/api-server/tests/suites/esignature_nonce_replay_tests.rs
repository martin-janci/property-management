//! Regression tests for the e-signature nonce replay fix (issue #673,
//! follow-up to PR #542).
//!
//! Background: PR #542 introduced `LightweightProvider::build_signing_url`
//! which binds a per-link nonce into the HMAC of a signing URL. The nonce
//! was never persisted, so even though the future `/sign` consumer would
//! verify the HMAC, it had no way to detect a replay — an attacker who
//! captured a signing URL could reuse it for the full 7-day TTL.
//!
//! This issue's fix adds the `e_signature_nonces` table with a unique
//! `(envelope_id, nonce)` constraint. `ESignatureNonceRepository::record_nonce`
//! INSERTs into that table; the second INSERT with the same pair raises
//! Postgres `unique_violation` (23505) which the repository maps to
//! `RecordNonceError::Replay`. The API layer maps that to **409 CONFLICT**.
//!
//! There is no `/sign` HTTP endpoint in the codebase yet, so this test
//! exercises the repository contract directly. The 409 status-code claim
//! in the PR body is asserted at the boundary the consumer will use:
//! `RecordNonceError::Replay` exists on the second attempt with the same
//! `(envelope_id, nonce)` pair, and the API translation is a trivial match
//! once the `/sign` handler ships.

#![allow(dead_code)]

use db::repositories::{ESignatureNonceRepository, RecordNonceError};
use sqlx::PgPool;
use uuid::Uuid;

/// Insert a minimal `signature_requests` row so the
/// `e_signature_nonces.envelope_id` FK is satisfied. Returns the request id.
///
/// This bypasses the higher-level `SignatureRequestRepository::create` because
/// it requires a real `documents` row + RLS context, which is irrelevant to
/// the nonce-replay contract under test.
async fn seed_signature_request(pool: &PgPool) -> Uuid {
    // 1. Org — match the seeding shape used by other api-server tests
    //    (caddy_ask_tests, dev_mode_tenant_tests). Slug includes a random
    //    suffix because multiple tests share the same migrator-seeded DB
    //    per `#[sqlx::test]` instance but seeded helpers may be called
    //    multiple times in a single test.
    let slug = format!("e673-org-{}", Uuid::new_v4());
    let org_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO organizations (name, slug, contact_email, status)
           VALUES ('E673 Org', $1, 'e673@example.com', 'active')
           RETURNING id"#,
    )
    .bind(&slug)
    .fetch_one(pool)
    .await
    .expect("seed org");

    // 2. User (creator)
    let user_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO users (email, password_hash, name, status, email_verified_at)
           VALUES ($1, 'test_hash', 'E673 User', 'active', NOW())
           RETURNING id"#,
    )
    .bind(format!("e673-{}@example.com", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("seed user");

    // 3. Document (signature_requests.document_id FK)
    let doc_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO documents
             (organization_id, title, category, file_key, file_name,
              mime_type, size_bytes, access_scope, created_by)
           VALUES ($1, 'E673 Doc', 'contracts', 'k', 'f.pdf',
                   'application/pdf', 1, 'organization', $2)
           RETURNING id"#,
    )
    .bind(org_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("seed document");

    // 4. Signature request — empty signers, just enough to satisfy FK.
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO signature_requests
             (document_id, organization_id, signers, created_by)
           VALUES ($1, $2, '[]'::jsonb, $3)
           RETURNING id"#,
    )
    .bind(doc_id)
    .bind(org_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("seed signature_request")
}

/// Core regression: the same `(envelope_id, nonce)` cannot be recorded twice.
/// First call succeeds; second call returns `RecordNonceError::Replay`, which
/// the API layer maps to HTTP 409 CONFLICT.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn nonce_replay_is_rejected(pool: PgPool) {
    let repo = ESignatureNonceRepository::new(pool.clone());
    let envelope_id = seed_signature_request(&pool).await;
    let nonce = Uuid::new_v4();

    // First use — the legitimate one.
    repo.record_nonce(envelope_id, nonce)
        .await
        .expect("first record_nonce must succeed");

    // Second use of the SAME (envelope_id, nonce) pair must be rejected.
    let replay = repo.record_nonce(envelope_id, nonce).await;
    assert!(
        matches!(replay, Err(RecordNonceError::Replay)),
        "Second record_nonce with same (envelope_id, nonce) must be \
         RecordNonceError::Replay (which the API layer maps to 409 CONFLICT); \
         got: {:?}",
        replay
    );

    // Sanity: the row from the legitimate first use is still present.
    assert!(
        repo.nonce_exists(envelope_id, nonce)
            .await
            .expect("nonce_exists query must succeed"),
        "Legitimate first nonce must remain on file after the replay attempt"
    );
}

/// Same nonce VALUE across two different envelopes is allowed — the unique
/// constraint is on the *pair*, not the nonce alone. Without this case a
/// regression that tightened the constraint to UNIQUE(nonce) only would slip
/// through.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn same_nonce_across_different_envelopes_is_allowed(pool: PgPool) {
    let repo = ESignatureNonceRepository::new(pool.clone());
    let envelope_a = seed_signature_request(&pool).await;
    let envelope_b = seed_signature_request(&pool).await;
    let shared_nonce = Uuid::new_v4();

    repo.record_nonce(envelope_a, shared_nonce)
        .await
        .expect("envelope A first use");
    repo.record_nonce(envelope_b, shared_nonce).await.expect(
        "envelope B with same nonce VALUE must succeed — the constraint \
             is on (envelope_id, nonce), not nonce alone",
    );
}

/// Different nonces on the SAME envelope are allowed — a single signature
/// request gets one nonce per `build_signing_url` invocation (initial invite
/// + each reminder), so multiple rows per envelope is the normal state.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn different_nonces_on_same_envelope_are_allowed(pool: PgPool) {
    let repo = ESignatureNonceRepository::new(pool.clone());
    let envelope_id = seed_signature_request(&pool).await;

    repo.record_nonce(envelope_id, Uuid::new_v4())
        .await
        .expect("first nonce on envelope");
    repo.record_nonce(envelope_id, Uuid::new_v4())
        .await
        .expect("second distinct nonce on same envelope must succeed");
    repo.record_nonce(envelope_id, Uuid::new_v4())
        .await
        .expect("third distinct nonce on same envelope must succeed");
}
