//! Regression test for BIT-359 — `get_shares_rls` decoded the Postgres enum
//! `document_share_type` straight into the Rust `String` field
//! `DocumentShare.share_type`. The `GET /documents/{id}/shares` listing endpoint
//! (`servers/api-server/src/routes/documents/shares.rs:95`) calls this repo
//! method; its `SELECT s.*` returned the raw `document_share_type` OID, and
//! sqlx 0.9 refuses to decode a user-defined ENUM into `String`, so the read
//! panicked at runtime. Every sibling query in the same file already casts
//! `share_type::text AS share_type`; only this `SELECT s.*` path forgot.
//!
//! The fix replaces `SELECT s.*` with an explicit column list that casts
//! `s.share_type::text AS share_type`. This test seeds a `'link'` share and
//! reads it back through `get_shares_rls`; it fails on `main` (decode error)
//! and passes once the cast is in place.

use db::repositories::DocumentRepository;
use sqlx::PgPool;
use uuid::Uuid;

/// Seed org -> user -> document -> `'link'` share under a super-admin RLS
/// context (which satisfies the `WITH CHECK` legs on the inserts). Returns
/// `(document_id, share_id)`.
async fn seed(pool: &PgPool) -> (Uuid, Uuid) {
    sqlx::query("SELECT set_request_context($1, $2, $3)")
        .bind(Option::<Uuid>::None)
        .bind(Option::<Uuid>::None)
        .bind(true)
        .execute(pool)
        .await
        .expect("set super-admin context");

    let org_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO organizations (name, slug, contact_email, status)
        VALUES ('Share Enum Org', 'share-enum-org', 'share-enum@decode.test', 'active')
        RETURNING id
        "#,
    )
    .fetch_one(pool)
    .await
    .expect("seed org");

    let user_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ('share-enum@user.test', 'test_hash', 'Share Enum User', 'active', NOW(), 'public')
        RETURNING id
        "#,
    )
    .fetch_one(pool)
    .await
    .expect("seed user");

    let document_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO documents (
            organization_id, title, category, file_key, file_name, mime_type,
            size_bytes, created_by
        )
        VALUES ($1, 'Share Enum Doc', 'other', 's3/share-enum.pdf', 'share-enum.pdf',
                'application/pdf', 1024, $2)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("seed document");

    // Bind the enum column with a literal cast — `'link'::document_share_type` —
    // rather than a bound `String`, which sqlx 0.9 would reject on the encode
    // side (a separate strictness from the decode regression under test).
    let share_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO document_shares (document_id, share_type, shared_by, share_token)
        VALUES ($1, 'link'::document_share_type, $2, $3)
        RETURNING id
        "#,
    )
    .bind(document_id)
    .bind(user_id)
    .bind(format!("tok{}", Uuid::new_v4().simple()))
    .fetch_one(pool)
    .await
    .expect("seed link share");

    (document_id, share_id)
}

/// `get_shares_rls` must decode the `document_share_type` enum into the
/// `share_type: String` field. On `main` the `SELECT s.*` path panics on
/// decode; after the `::text` cast it returns the seeded `'link'` share.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_shares_rls_decodes_document_share_type_enum(pool: PgPool) {
    let (document_id, share_id) = seed(&pool).await;
    let repo = DocumentRepository::new(pool.clone());

    let shares = repo
        .get_shares_rls(&pool, document_id)
        .await
        .expect("get_shares_rls must not fail decoding the document_share_type enum (BIT-359)");

    assert_eq!(shares.len(), 1, "the one seeded share must be returned");
    assert_eq!(shares[0].share.id, share_id);
    assert_eq!(
        shares[0].share.share_type, "link",
        "share_type enum must decode to its text value"
    );
    assert_eq!(shares[0].document_title, "Share Enum Doc");
}
