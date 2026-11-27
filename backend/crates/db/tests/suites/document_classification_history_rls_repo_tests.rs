//! Repo-level behavioral RLS regression test for the AI/OCR document-intelligence
//! classification-history read.
//!
//! Background
//! ----------
//! `document_classification_history` is under `FORCE ROW LEVEL SECURITY` with the
//! canonical `get_current_org_id()` tenant-isolation policy (migration 00179 /
//! #754): a row is visible only when the owning document's `organization_id`
//! equals the request org context. The production api-server connects as the
//! table OWNER, which `FORCE` binds.
//!
//! `DocumentRepository::get_classification_history` ran the read on the shared
//! `PgPool` — a connection that never has `set_request_context` applied. On a
//! `FORCE`-bound role that means `get_current_org_id()` is NULL and the policy
//! collapses to deny-all: the caller's own AI classification history silently
//! comes back empty, and the query bypasses the per-request tenant context (the
//! AI/OCR RLS-bypass pattern). The GET `/documents/{id}/classification/history`
//! handler holds the request's org-scoped `RlsConnection` yet issued the read on
//! the context-less pool.
//!
//! The fix adds `get_classification_history_rls`, which takes the caller's
//! org-scoped executor, and points the handler at it. This test exercises the
//! repository method on a `FORCE`-bound role and proves:
//!
//!   1. **Deny-all reproduction** — bound role, NO context set (exactly what the
//!      raw-pool read did): an own-org read returns nothing.
//!   2. **Fix** — with `set_request_context(org_a, user_a)` first, the same read
//!      returns the own-org history row.
//!   3. **Cross-tenant by-id** — probing org B's document id from an org-A
//!      context surfaces nothing.
//!
//! Why this test switches roles
//! ----------------------------
//! `#[sqlx::test]` connects as the Postgres SUPERUSER, which bypasses RLS
//! entirely — even `FORCE` does not bind a superuser, so a behavioral assertion
//! would pass vacuously. The test creates a plain `NOSUPERUSER NOBYPASSRLS` role
//! and `SET ROLE`s to it so `FORCE` enforces the policy the way the production
//! owner role experiences it. Mirrors `llm_document_rls_repo_tests.rs`.

use crate::common::{seed_org, set_ctx};
use db::repositories::DocumentRepository;
use sqlx::PgPool;
use uuid::Uuid;

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'test_hash', 'ClassHist User', 'active', NOW(), 'public')
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

/// Seed a parent document (FK target for the history row) as superuser, RLS-exempt.
async fn seed_document(pool: &PgPool, org_id: Uuid, created_by: Uuid, title: &str) -> Uuid {
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
    .bind(format!("s3/{title}.pdf"))
    .bind(format!("{title}.pdf"))
    .bind(created_by)
    .fetch_one(pool)
    .await
    .expect("seed document")
}

/// Seed a classification-history row directly (as superuser, RLS-exempt).
async fn seed_history(pool: &PgPool, document_id: Uuid, category: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO document_classification_history
            (document_id, predicted_category, confidence)
        VALUES ($1, $2, 0.9)
        RETURNING id
        "#,
    )
    .bind(document_id)
    .bind(category)
    .fetch_one(pool)
    .await
    .expect("seed classification history")
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_classification_history_rls_force_rls_deny_all_and_fix(pool: PgPool) {
    let repo = DocumentRepository::new(pool.clone());

    // --- Seed as super-admin context (satisfies the org roles-trigger). ---
    set_ctx(&pool, None, None, true).await;
    let org_a = seed_org(&pool, "classhist-a").await;
    let org_b = seed_org(&pool, "classhist-b").await;
    let user_a = seed_user(&pool, "a@classhist.test").await;
    let user_b = seed_user(&pool, "b@classhist.test").await;
    let doc_a = seed_document(&pool, org_a, user_a, "classhist-doc-a").await;
    let doc_b = seed_document(&pool, org_b, user_b, "classhist-doc-b").await;
    let hist_a = seed_history(&pool, doc_a, "contracts").await;
    let _hist_b = seed_history(&pool, doc_b, "invoices").await;

    // --- NOSUPERUSER NOBYPASSRLS role so FORCE actually binds. ---
    let role = format!("ppt_rls_classhist_{}", Uuid::new_v4().simple());
    for stmt in [
        format!("CREATE ROLE \"{role}\" NOSUPERUSER NOBYPASSRLS"),
        format!("GRANT SELECT ON document_classification_history TO \"{role}\""),
        // The policy's EXISTS subquery reads `documents` (also FORCE RLS), so the
        // bound role needs read access to it and to the org-not-deleted helper's
        // `organizations` table.
        format!("GRANT SELECT ON documents TO \"{role}\""),
        format!("GRANT SELECT ON organizations TO \"{role}\""),
        format!("GRANT EXECUTE ON FUNCTION get_current_org_id() TO \"{role}\""),
        format!("GRANT EXECUTE ON FUNCTION get_current_org_not_deleted() TO \"{role}\""),
    ] {
        sqlx::query(sqlx::AssertSqlSafe(stmt))
            .execute(&pool)
            .await
            .expect("grant setup");
    }

    // ====================================================================
    // (1) DENY-ALL reproduction: role bound, NO context set (the raw-pool
    //     behavior). The own-org classification history returns nothing.
    // ====================================================================
    {
        let mut conn = pool.acquire().await.expect("acquire");
        sqlx::query("SELECT clear_request_context()")
            .execute(&mut *conn)
            .await
            .expect("clear ctx");
        sqlx::query(sqlx::AssertSqlSafe(format!("SET ROLE \"{role}\"")))
            .execute(&mut *conn)
            .await
            .expect("set role");

        let rows = repo
            .get_classification_history_rls(&mut *conn, doc_a)
            .await
            .expect("get_classification_history_rls (no ctx)");
        assert!(
            rows.is_empty(),
            "regression: without RLS context, own-org classification history must be \
             invisible (deny-all) — this is what the raw-pool read did in production"
        );

        sqlx::query("RESET ROLE")
            .execute(&mut *conn)
            .await
            .expect("reset role");
    }

    // ====================================================================
    // (2) FIX + (3) cross-tenant by-id: set context, drop to bound role, query.
    // ====================================================================
    {
        let mut conn = pool.acquire().await.expect("acquire");
        sqlx::query("SELECT set_request_context($1, $2, $3)")
            .bind(org_a)
            .bind(user_a)
            .bind(false)
            .execute(&mut *conn)
            .await
            .expect("set org-A ctx");
        sqlx::query(sqlx::AssertSqlSafe(format!("SET ROLE \"{role}\"")))
            .execute(&mut *conn)
            .await
            .expect("set role");

        // (2) Own-org read IS now visible through the repo — the fix.
        let rows = repo
            .get_classification_history_rls(&mut *conn, doc_a)
            .await
            .expect("get_classification_history_rls (ctx)");
        assert_eq!(
            rows.iter().map(|h| h.id).collect::<Vec<_>>(),
            vec![hist_a],
            "fix: with RLS context set, the read returns exactly the own-org history row"
        );

        // (3) Cross-tenant by-id: probing org B's document id from an org-A
        //     context must surface nothing.
        let cross = repo
            .get_classification_history_rls(&mut *conn, doc_b)
            .await
            .expect("get_classification_history_rls (cross-tenant probe)");
        assert!(
            cross.is_empty(),
            "cross-tenant: org B's classification history must NOT be readable by an org-A caller"
        );

        sqlx::query("RESET ROLE")
            .execute(&mut *conn)
            .await
            .expect("reset role");
    }

    // --- Cleanup the test role. ---
    set_ctx(&pool, None, None, true).await;
    for stmt in [
        format!("REVOKE ALL ON document_classification_history FROM \"{role}\""),
        format!("REVOKE ALL ON documents FROM \"{role}\""),
        format!("REVOKE ALL ON organizations FROM \"{role}\""),
        format!("DROP OWNED BY \"{role}\""),
        format!("DROP ROLE IF EXISTS \"{role}\""),
    ] {
        sqlx::query(sqlx::AssertSqlSafe(stmt))
            .execute(&pool)
            .await
            .ok();
    }
}
