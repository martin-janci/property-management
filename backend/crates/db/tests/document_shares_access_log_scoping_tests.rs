//! Regression test for #2243 (follow-up to #2197) — `get_share_access_log_rls`
//! must scope its read to the share's owning document, not just the share id.
//!
//! Background
//! ----------
//! `document_shares` and `document_share_access_log` are RLS-tenant-scoped only.
//! `revoke_share_rls` was hardened in #2197 to constrain its UPDATE to the
//! owning document (`WHERE id = $1 AND document_id = $2`), closing a
//! cross-document IDOR at the data layer. This test extends the identical
//! invariant to the sibling access-log read: a caller authorized for document A
//! must not be able to read the access log of a share belonging to document B in
//! the same tenant by passing A's id plus B's share_id.
//!
//! It exercises the repo method directly: reading share-B's log while naming
//! document A returns 0 rows, while naming its true owning document B returns the
//! seeded rows.

use db::models::LogShareAccess;
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
        VALUES ('Access Log Scope Org', 'access-log-scope-org', 'access-log-scope@scoping.test', 'active')
        RETURNING id
        "#,
    )
    .fetch_one(pool)
    .await
    .expect("seed org");

    let user_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ('access-log-scope@user.test', 'test_hash', 'Access Log Scope User', 'active', NOW(), 'public')
        RETURNING id
        "#,
    )
    .fetch_one(pool)
    .await
    .expect("seed user");

    let doc_a = seed_doc(pool, org_id, user_id, "Access Log Scope A", "s3/access-log-a.pdf").await;
    let doc_b = seed_doc(pool, org_id, user_id, "Access Log Scope B", "s3/access-log-b.pdf").await;
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

/// Reading share-B's access log while naming document A must return 0 rows (the
/// cross-document IDOR #2243 closes at the data layer); naming its true owning
/// document B must return the seeded log entries.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_share_access_log_rls_is_scoped_to_owning_document(pool: PgPool) {
    let (doc_a, _share_a, doc_b, share_b) = seed(&pool).await;
    let repo = DocumentRepository::new(pool.clone());

    // Two access-log rows for share-B.
    for _ in 0..2 {
        repo.log_share_access_rls(
            &pool,
            LogShareAccess {
                share_id: share_b,
                accessed_by: None,
                ip_address: Some("203.0.113.7".to_string()),
            },
        )
        .await
        .expect("seed access-log row for share-B");
    }

    // Cross-document: read share-B's log under document A → 0 rows.
    let cross = repo
        .get_share_access_log_rls(&pool, share_b, doc_a, None)
        .await
        .expect("get_share_access_log_rls must not error on a cross-document read");
    assert!(
        cross.is_empty(),
        "reading a share's access log under the wrong document must return 0 rows (#2243)"
    );

    // Correctly scoped: read share-B's log under its true document B → 2 rows.
    let scoped = repo
        .get_share_access_log_rls(&pool, share_b, doc_b, None)
        .await
        .expect("get_share_access_log_rls must not error on a correctly scoped read");
    assert_eq!(
        scoped.len(),
        2,
        "reading a share's access log under its owning document must return its rows"
    );
    assert!(
        scoped.iter().all(|entry| entry.share_id == share_b),
        "every returned access-log row must belong to the requested share"
    );
}
