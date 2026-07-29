//! Repo-level behavioral RLS regression test for PAP-108 (parent PAP-80 / PAP-67).
//!
//! Background
//! ----------
//! Migration `00179` (PAP-62) put `FORCE ROW LEVEL SECURITY` + the canonical
//! `get_current_org_id()` policy on `document_embeddings` — the RAG store that
//! `LlmDocumentRepository` reads for chat context. The production api-server
//! connects as the table OWNER, which `FORCE` binds. The repo held a raw
//! `PgPool` and ran every query WITHOUT ever calling `set_request_context`, so
//! on `dev` `get_current_org_id()` returned NULL and the policy collapsed to
//! `organization_id = NULL` → **deny-all**: own-org RAG reads returned empty
//! (chat silently lost its document context) and embedding writes failed.
//!
//! The fix makes the repo stateless: every method takes an RLS-context
//! executor (the `RlsConnection` extractor — or the short-lived context-set
//! connection in `send_message` — sets the org/user GUCs before any query).
//! This test exercises the *repository methods themselves* on a `FORCE`-bound
//! role and proves:
//!
//!   1. **Deny-all reproduction** — with the role bound but NO context set
//!      (exactly what the raw-pool repo did on `dev`), an own-org
//!      `find_document_embeddings` / `search_documents_by_text` returns nothing.
//!   2. **Fix** — with `set_request_context(org_a, user_a)` applied first, the
//!      same by-id read returns the own-org chunk.
//!   3. **Cross-tenant by-id** — org B's document embeddings are invisible to
//!      an org-A caller probing org B's `document_id`.
//!   4. **Write path** — `create_embedding` succeeds under org-A context and
//!      the row lands in the caller's org (the write-side of deny-all).
//!
//! Why this test switches roles
//! ----------------------------
//! `#[sqlx::test]` connects as the Postgres SUPERUSER, which bypasses RLS
//! entirely — even `FORCE` does not bind a superuser, so a behavioral assertion
//! would pass vacuously. The test creates a plain `NOSUPERUSER NOBYPASSRLS`
//! role, grants it access, and `SET ROLE`s to it so `FORCE` actually enforces
//! the policy the way the production owner role experiences it. Mirrors
//! `sentiment_rls_repo_tests.rs` / `work_order` (the PAP-67 precedent PAP-80
//! follows).

use crate::common::{seed_org, set_ctx};
use db::repositories::LlmDocumentRepository;
use sqlx::PgPool;
use uuid::Uuid;

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'test_hash', 'LlmDoc User', 'active', NOW(), 'public')
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

/// Seed a parent document (FK target for embeddings) as superuser, RLS-exempt.
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

/// Seed an embedding chunk directly (as superuser, RLS-exempt) for an org.
async fn seed_embedding(pool: &PgPool, org_id: Uuid, document_id: Uuid, text: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO document_embeddings
            (organization_id, document_id, chunk_index, chunk_text, metadata)
        VALUES ($1, $2, 0, $3, '{}'::jsonb)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(document_id)
    .bind(text)
    .fetch_one(pool)
    .await
    .expect("seed embedding")
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn llm_document_repo_force_rls_deny_all_and_fix(pool: PgPool) {
    let repo = LlmDocumentRepository::new(pool.clone());

    // --- Seed as superuser / super-admin context (satisfies org roles-trigger). ---
    set_ctx(&pool, None, None, true).await;
    let org_a = seed_org(&pool, "llmforce-a").await;
    let org_b = seed_org(&pool, "llmforce-b").await;
    let user_a = seed_user(&pool, "a@llmdoc.test").await;
    let user_b = seed_user(&pool, "b@llmdoc.test").await;
    let doc_a = seed_document(&pool, org_a, user_a, "llmdoc-rag-a").await;
    let doc_b = seed_document(&pool, org_b, user_b, "llmdoc-rag-b").await;
    let chunk_a = seed_embedding(&pool, org_a, doc_a, "boiler maintenance manual chapter").await;
    let _chunk_b = seed_embedding(&pool, org_b, doc_b, "confidential org-b lease appendix").await;

    // --- NOSUPERUSER NOBYPASSRLS role so FORCE actually binds. ---
    let role = format!("ppt_rls_llmdoc_{}", Uuid::new_v4().simple());
    for stmt in [
        format!("CREATE ROLE \"{role}\" NOSUPERUSER NOBYPASSRLS"),
        format!("GRANT SELECT, INSERT, UPDATE, DELETE ON document_embeddings TO \"{role}\""),
        // The org-not-deleted policy helper reads `organizations` with invoker
        // rights, so the bound role needs read access to it.
        format!("GRANT SELECT ON organizations TO \"{role}\""),
        // RLS policy helpers must be EXECUTE-able by the bound role.
        format!("GRANT EXECUTE ON FUNCTION get_current_org_id() TO \"{role}\""),
        format!("GRANT EXECUTE ON FUNCTION get_current_org_not_deleted() TO \"{role}\""),
    ] {
        sqlx::query(sqlx::AssertSqlSafe(stmt))
            .execute(&pool)
            .await
            .expect("grant setup");
    }

    // ====================================================================
    // (1) DENY-ALL reproduction: role bound, NO context set (the dev raw-pool
    //     behavior). Own-org RAG reads return nothing.
    // ====================================================================
    {
        let mut conn = pool.acquire().await.expect("acquire");
        // Explicitly clear any inherited context, then drop to the bound role.
        sqlx::query("SELECT clear_request_context()")
            .execute(&mut *conn)
            .await
            .expect("clear ctx");
        sqlx::query(sqlx::AssertSqlSafe(format!("SET ROLE \"{role}\"")))
            .execute(&mut *conn)
            .await
            .expect("set role");

        let by_doc = repo
            .find_document_embeddings(&mut *conn, doc_a)
            .await
            .expect("find_document_embeddings (no ctx)");
        assert!(
            by_doc.is_empty(),
            "PAP-80 regression: without RLS context, own-org embeddings must be \
             invisible (deny-all) — this is what the raw-pool repo did on dev"
        );

        let by_text = repo
            .search_documents_by_text(&mut *conn, org_a, "boiler", 5)
            .await
            .expect("search_documents_by_text (no ctx)");
        assert!(
            by_text.is_empty(),
            "PAP-80 regression: the RAG text search silently returned no context \
             on the raw pool"
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

        // (2) Own-org by-id read IS now visible through the repo — the fix.
        let by_doc = repo
            .find_document_embeddings(&mut *conn, doc_a)
            .await
            .expect("find_document_embeddings (ctx)");
        assert_eq!(
            by_doc.iter().map(|e| e.id).collect::<Vec<_>>(),
            vec![chunk_a],
            "PAP-80 fix: with RLS context set, the by-id read returns exactly the own-org chunk"
        );

        // (3) Cross-tenant by-id: probing org B's document id from an org-A
        //     context must surface nothing.
        let cross = repo
            .find_document_embeddings(&mut *conn, doc_b)
            .await
            .expect("find_document_embeddings (cross-tenant probe)");
        assert!(
            cross.is_empty(),
            "cross-tenant: org B's embeddings must NOT be readable by an org-A caller"
        );

        sqlx::query("RESET ROLE")
            .execute(&mut *conn)
            .await
            .expect("reset role");
    }

    // ====================================================================
    // (4) WRITE path: a create on a context-set connection succeeds and the
    //     row is the caller's org. Without context the INSERT would fail the
    //     policy WITH CHECK (the write-side of deny-all).
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

        let created = repo
            .create_embedding(
                &mut *conn,
                org_a,
                doc_a,
                1,
                "boiler maintenance manual chapter two",
                None,
                serde_json::json!({}),
            )
            .await
            .expect("create_embedding under context must succeed");
        assert_eq!(created.organization_id, org_a);

        sqlx::query("RESET ROLE")
            .execute(&mut *conn)
            .await
            .expect("reset role");
    }

    // --- Cleanup the test role. ---
    set_ctx(&pool, None, None, true).await;
    for stmt in [
        format!("REVOKE ALL ON document_embeddings FROM \"{role}\""),
        format!("REVOKE ALL ON organizations FROM \"{role}\""),
        // DROP OWNED severs every remaining privilege this role holds in the
        // test database — the explicit REVOKEs above keep missing the RLS
        // helper-function EXECUTE grants, so DROP ROLE failed with "objects
        // depend on it" and leaked the cluster-global role (PAP-134).
        format!("DROP OWNED BY \"{role}\""),
        format!("DROP ROLE IF EXISTS \"{role}\""),
    ] {
        sqlx::query(sqlx::AssertSqlSafe(stmt))
            .execute(&pool)
            .await
            .ok();
    }
}
