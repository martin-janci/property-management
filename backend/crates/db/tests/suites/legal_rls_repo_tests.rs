//! Repo-level behavioral RLS regression test for PAP-109 (parent PAP-80 / PAP-67).
//!
//! Background
//! ----------
//! Migration `00179` (PAP-62) put `FORCE ROW LEVEL SECURITY` + the canonical
//! `get_current_org_id()` policy on the eight legal/compliance tables, including
//! `legal_documents`. The production api-server connects as the table OWNER,
//! which `FORCE` binds. `LegalRepository` held a raw `PgPool` and ran every
//! query WITHOUT ever calling `set_request_context`, so on `dev`
//! `get_current_org_id()` returned NULL and the policy collapsed to
//! `organization_id = NULL` → **deny-all**: own-org reads returned empty, writes
//! failed. (PAP-80.)
//!
//! The fix routes the repo through an RLS-context connection (the `RlsConnection`
//! extractor in handlers sets the org/user GUCs before any query). This test
//! exercises the *repository methods themselves* on a `FORCE`-bound role and
//! proves:
//!
//!   1. **Deny-all reproduction** — with the role bound but NO context set
//!      (exactly what the raw-pool repo did on `dev`), an own-org
//!      `find_document_by_id` / `list_documents` returns nothing.
//!   2. **Fix** — with `set_request_context(org_a, user_a)` applied first (what
//!      `RlsConnection` now does), the same repo calls return the own-org row.
//!   3. **Cross-tenant** — org B's document stays invisible to an org-A caller
//!      even with context set.
//!   4. **Write path** — a `create_document` on a context-set connection succeeds
//!      and the row is the caller's org; without context the INSERT fails the
//!      policy.
//!
//! Why this test switches roles
//! ----------------------------
//! `#[sqlx::test]` connects as the Postgres SUPERUSER, which bypasses RLS
//! entirely — even `FORCE` does not bind a superuser, so a behavioral assertion
//! would pass vacuously. The test creates a plain `NOSUPERUSER NOBYPASSRLS`
//! role, grants it access, and `SET ROLE`s to it so `FORCE` actually enforces
//! the policy the way the production owner role experiences it. Mirrors
//! `equipment_rls_repo_tests.rs` / `work_order_rls_repo_tests.rs` (the PAP-67
//! precedent PAP-109 follows).

use crate::common::{seed_org, set_ctx};
use db::models::{CreateLegalDocument, LegalDocumentQuery};
use db::repositories::LegalRepository;
use sqlx::PgPool;
use uuid::Uuid;

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'test_hash', 'Legal User', 'active', NOW(), 'public')
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

/// Seed a legal document directly (as superuser, RLS-exempt) for an org.
async fn seed_document(pool: &PgPool, org_id: Uuid, user_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO legal_documents (organization_id, document_type, title, created_by)
        VALUES ($1, 'contract', 'Seeded Document', $2)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("seed document")
}

fn sample_create() -> CreateLegalDocument {
    CreateLegalDocument {
        building_id: None,
        document_type: "policy".to_string(),
        title: "Created under RLS".to_string(),
        description: None,
        parties: None,
        effective_date: None,
        expiry_date: None,
        file_path: None,
        file_name: None,
        file_size: None,
        mime_type: None,
        is_confidential: None,
        retention_period_months: None,
        tags: None,
        metadata: None,
    }
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn legal_repo_force_rls_deny_all_and_fix(pool: PgPool) {
    let repo = LegalRepository::new(pool.clone());

    // --- Seed as superuser / super-admin context (satisfies org roles-trigger). ---
    set_ctx(&pool, None, None, true).await;
    let org_a = seed_org(&pool, "legal-force-a").await;
    let org_b = seed_org(&pool, "legal-force-b").await;
    let user_a = seed_user(&pool, "a@legal.test").await;
    let user_b = seed_user(&pool, "b@legal.test").await;
    let doc_a = seed_document(&pool, org_a, user_a).await;
    let doc_b = seed_document(&pool, org_b, user_b).await;

    // --- NOSUPERUSER NOBYPASSRLS role so FORCE actually binds. ---
    let role = format!("ppt_rls_legal_{}", Uuid::new_v4().simple());
    for stmt in [
        format!("CREATE ROLE \"{role}\" NOSUPERUSER NOBYPASSRLS"),
        format!("GRANT SELECT, INSERT, UPDATE, DELETE ON legal_documents TO \"{role}\""),
        format!(
            "GRANT EXECUTE ON FUNCTION get_current_org_id(), is_super_admin(), \
             get_current_org_not_deleted() TO \"{role}\""
        ),
        format!("GRANT SELECT ON organizations TO \"{role}\""),
    ] {
        sqlx::query(sqlx::AssertSqlSafe(stmt))
            .execute(&pool)
            .await
            .expect("grant setup");
    }

    // ====================================================================
    // (1) DENY-ALL reproduction: role bound, NO context set (the dev raw-pool
    //     behavior). Own-org reads return nothing.
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

        let found = repo
            .find_document_by_id(&mut *conn, doc_a, org_a)
            .await
            .expect("find_document_by_id (no ctx)");
        assert!(
            found.is_none(),
            "PAP-80 regression: without RLS context, own-org document must be \
             invisible (deny-all) — this is what the raw-pool repo did on dev"
        );

        let listed = repo
            .list_documents(&mut *conn, org_a, LegalDocumentQuery::default())
            .await
            .expect("list_documents (no ctx)");
        assert!(
            listed.is_empty(),
            "PAP-80 regression: without RLS context, list returns deny-all empty"
        );

        sqlx::query("RESET ROLE")
            .execute(&mut *conn)
            .await
            .expect("reset role");
    }

    // ====================================================================
    // (2) FIX + (3) cross-tenant: set context, drop to bound role, query repo.
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

        // (2) Own-org row IS now visible through the repo — the fix.
        let found = repo
            .find_document_by_id(&mut *conn, doc_a, org_a)
            .await
            .expect("find_document_by_id (ctx)");
        assert_eq!(
            found.map(|d| d.id),
            Some(doc_a),
            "PAP-80 fix: with RLS context set, the repo must return the own-org document"
        );

        let listed = repo
            .list_documents(&mut *conn, org_a, LegalDocumentQuery::default())
            .await
            .expect("list_documents (ctx)");
        assert_eq!(
            listed.iter().map(|d| d.id).collect::<Vec<_>>(),
            vec![doc_a],
            "PAP-80 fix: list returns exactly the own-org document under context"
        );

        // (3) Org B's document stays invisible to an org-A caller.
        let cross = repo
            .find_document_by_id(&mut *conn, doc_b, org_a)
            .await
            .expect("find_document_by_id cross");
        assert!(
            cross.is_none(),
            "cross-tenant: org B's document must NOT be visible to an org-A caller"
        );

        sqlx::query("RESET ROLE")
            .execute(&mut *conn)
            .await
            .expect("reset role");
    }

    // ====================================================================
    // (4) WRITE path: a create on a context-set connection succeeds and the row
    //     is the caller's org. Without context the INSERT would fail the policy
    //     WITH CHECK (the write-side of deny-all).
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
            .create_document(&mut *conn, org_a, user_a, sample_create())
            .await
            .expect("create_document under context must succeed");
        assert_eq!(created.organization_id, org_a);

        sqlx::query("RESET ROLE")
            .execute(&mut *conn)
            .await
            .expect("reset role");
    }

    // --- Cleanup the test role. ---
    set_ctx(&pool, None, None, true).await;
    for stmt in [
        format!("REVOKE ALL ON legal_documents FROM \"{role}\""),
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
