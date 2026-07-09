//! Regression test for #2197 — `revoke_share_rls` must scope its UPDATE to the
//! share's owning document, not just the share id.
//!
//! Background
//! ----------
//! `document_shares` RLS only scopes to the tenant. The revoke handler
//! (`servers/api-server/src/routes/documents/shares.rs`) authorizes against the
//! path *document* id but historically looked the share up (and revoked it)
//! purely by `share_id`. A creator of document A could therefore revoke a share
//! of document B in the same tenant by passing A's id (to satisfy the creator
//! check) plus B's share_id — a cross-document IDOR. PR #2181 added an explicit
//! `share.document_id == id` handler guard; #2197 pushes the same invariant into
//! the data layer so a future caller cannot reintroduce the vulnerability by
//! dropping the guard.
//!
//! This test exercises the repo method directly: revoking share-B while naming
//! document A affects 0 rows and leaves the share active, while naming its true
//! owning document affects exactly 1 row and revokes it.

use db::repositories::DocumentRepository;
use sqlx::PgPool;
use uuid::Uuid;

/// Seed one org, one user, two documents (A, B), each carrying a `'link'`
/// share, under a super-admin RLS context (which satisfies the `WITH CHECK`
/// legs on the inserts). Returns `(doc_a, share_a, doc_b, share_b)`.
async fn seed(pool: &PgPool) -> (Uuid, Uuid, Uuid, Uuid) {
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
        VALUES ('Revoke Scope Org', 'revoke-scope-org', 'revoke-scope@scoping.test', 'active')
        RETURNING id
        "#,
    )
    .fetch_one(pool)
    .await
    .expect("seed org");

    let user_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ('revoke-scope@user.test', 'test_hash', 'Revoke Scope User', 'active', NOW(), 'public')
        RETURNING id
        "#,
    )
    .fetch_one(pool)
    .await
    .expect("seed user");

    let doc_a = seed_doc(pool, org_id, user_id, "Revoke Scope A", "s3/revoke-a.pdf").await;
    let doc_b = seed_doc(pool, org_id, user_id, "Revoke Scope B", "s3/revoke-b.pdf").await;
    let share_a = seed_share(pool, doc_a, user_id).await;
    let share_b = seed_share(pool, doc_b, user_id).await;

    (doc_a, share_a, doc_b, share_b)
}

async fn seed_doc(pool: &PgPool, org_id: Uuid, user_id: Uuid, title: &str, key: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO documents (
            organization_id, title, category, file_key, file_name, mime_type,
            size_bytes, created_by
        )
        VALUES ($1, $2, 'other', $3, $4, 'application/pdf', 1024, $5)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(title)
    .bind(key)
    .bind(format!("{title}.pdf"))
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("seed document")
}

async fn seed_share(pool: &PgPool, document_id: Uuid, shared_by: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO document_shares (document_id, share_type, shared_by, share_token)
        VALUES ($1, 'link'::document_share_type, $2, $3)
        RETURNING id
        "#,
    )
    .bind(document_id)
    .bind(shared_by)
    .bind(format!("tok{}", Uuid::new_v4().simple()))
    .fetch_one(pool)
    .await
    .expect("seed link share")
}

async fn is_revoked(pool: &PgPool, share_id: Uuid) -> bool {
    sqlx::query_scalar::<_, bool>(
        "SELECT revoked_at IS NOT NULL FROM document_shares WHERE id = $1",
    )
    .bind(share_id)
    .fetch_one(pool)
    .await
    .expect("read revoked_at")
}

/// Revoking share-B while naming document A must affect 0 rows and leave the
/// share active (the cross-document IDOR #2197 closes at the data layer);
/// naming its true owning document B must affect 1 row and revoke it.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn revoke_share_rls_is_scoped_to_owning_document(pool: PgPool) {
    let (doc_a, _share_a, doc_b, share_b) = seed(&pool).await;
    let repo = DocumentRepository::new(pool.clone());

    // Cross-document: revoke share-B under document A → 0 rows, still active.
    let affected = repo
        .revoke_share_rls(&pool, share_b, doc_a)
        .await
        .expect("revoke_share_rls must not error on a cross-document revoke");
    assert_eq!(
        affected, 0,
        "revoking a share under the wrong document must affect 0 rows (#2197)"
    );
    assert!(
        !is_revoked(&pool, share_b).await,
        "a cross-document revoke must NOT revoke the share"
    );

    // Correctly scoped: revoke share-B under its true document B → 1 row.
    let affected = repo
        .revoke_share_rls(&pool, share_b, doc_b)
        .await
        .expect("revoke_share_rls must not error on a correctly scoped revoke");
    assert_eq!(
        affected, 1,
        "revoking a share under its owning document must affect exactly 1 row"
    );
    assert!(
        is_revoked(&pool, share_b).await,
        "a correctly scoped revoke must revoke the share"
    );
}
