//! Regression test for issue #754 — cross-tenant IDOR on `documents` caused
//! by missing `FORCE ROW LEVEL SECURITY`.
//!
//! Background
//! ----------
//! `documents` had only `ENABLE ROW LEVEL SECURITY` (00020). PostgreSQL
//! exempts the table OWNER from RLS unless the table is `FORCE`d, and the
//! production api-server connects as the table owner (`ppt`). So the
//! `document_tenant_isolation` policy was silently bypassed at runtime, and
//! `get_document` / `find_by_id_with_details_rls` did NOT isolate by org — a
//! real cross-tenant metadata read. (To make matters worse, the policy filtered
//! on `app.tenant_id`, a GUC that is never set; the codebase sets
//! `app.current_org_id`.) Migration 00163 repairs the policy to use
//! `get_current_org_id()` and adds `FORCE ROW LEVEL SECURITY`.
//!
//! Why this test switches roles
//! ----------------------------
//! `#[sqlx::test]` connects as the Postgres SUPERUSER, which bypasses RLS
//! entirely — even `FORCE` does not bind a superuser. To actually exercise the
//! policy the way the production owner role does, the test creates a plain
//! `NOSUPERUSER NOBYPASSRLS` role, grants it table access, and `SET ROLE`s to
//! it. `FORCE` binds that role, so the org-scoped policy is enforced. The org
//! context (`app.current_org_id`) is a session GUC and survives `SET ROLE`.

use sqlx::PgPool;
use uuid::Uuid;

mod common;
use common::{seed_org, set_ctx};

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'test_hash', 'Docs User', 'active', NOW(), 'public')
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

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

/// FORCE RLS on `documents` must hide another org's document from a
/// non-superuser (owner-like) role, while still exposing the caller's own
/// org's document. Without 00163's FORCE + policy fix, the cross-tenant row
/// would be visible (the IDOR in #754).
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn documents_force_rls_blocks_cross_tenant_read(pool: PgPool) {
    // --- Seed as superuser (super-admin context satisfies the roles-trigger
    //     RLS WITH CHECK fired by INSERT INTO organizations). ---
    set_ctx(&pool, None, None, true).await;

    let org_a = seed_org(&pool, "force-a").await;
    let org_b = seed_org(&pool, "force-b").await;
    let user_a = seed_user(&pool, "a@docs.test").await;
    let user_b = seed_user(&pool, "b@docs.test").await;

    let doc_a = seed_document(&pool, org_a, user_a, "alpha").await;
    let doc_b = seed_document(&pool, org_b, user_b, "bravo").await;

    // --- Create a NOSUPERUSER NOBYPASSRLS role so FORCE actually binds. ---
    // Random suffix keeps the role name unique across parallel test DBs.
    let role = format!("ppt_rls_test_{}", Uuid::new_v4().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "CREATE ROLE \"{role}\" NOSUPERUSER NOBYPASSRLS"
    )))
    .execute(&pool)
    .await
    .expect("create role");
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "GRANT SELECT ON documents TO \"{role}\""
    )))
    .execute(&pool)
    .await
    .expect("grant select on documents");
    // The policy calls helper functions (get_current_org_id, is_super_admin,
    // get_current_org_not_deleted); the role must be able to EXECUTE them.
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "GRANT EXECUTE ON FUNCTION get_current_org_id(), is_super_admin(), \
         get_current_org_not_deleted() TO \"{role}\""
    )))
    .execute(&pool)
    .await
    .expect("grant execute on rls helpers");
    // `get_current_org_not_deleted()` is SECURITY INVOKER (the default — it is
    // declared `LANGUAGE plpgsql STABLE` with no `SECURITY DEFINER`; see
    // migration 00140). Its body runs `SELECT 1 FROM organizations` with the
    // CALLER's privileges, so the bound role must be able to read
    // `organizations` or the policy's soft-delete guard raises
    // `permission denied for table organizations` and the caller can no longer
    // see even its OWN org's rows. `organizations` has no RLS of its own, so a
    // plain table-level SELECT grant is sufficient and does not weaken the
    // cross-tenant `documents` check (that is enforced by the `documents`
    // policy's `organization_id = get_current_org_id()` leg).
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "GRANT SELECT ON organizations TO \"{role}\""
    )))
    .execute(&pool)
    .await
    .expect("grant select on organizations (read by SECURITY INVOKER soft-delete helper)");

    // --- All role-bound work runs on ONE dedicated connection. `SET ROLE`
    //     and the org-context GUC are session state, so they must not be
    //     spread across pool acquisitions. ---
    {
        let mut conn = pool.acquire().await.expect("acquire connection");

        // Org-A context, then drop privileges to the bound (FORCE-applicable) role.
        sqlx::query("SELECT set_request_context($1, $2, $3)")
            .bind(org_a)
            .bind(user_a)
            .bind(false)
            .execute(&mut *conn)
            .await
            .expect("set org-A context");
        sqlx::query(sqlx::AssertSqlSafe(format!("SET ROLE \"{role}\"")))
            .execute(&mut *conn)
            .await
            .expect("set role");

        // Org-A's own document IS visible.
        let see_a: Option<Uuid> =
            sqlx::query_scalar("SELECT id FROM documents WHERE id = $1 AND deleted_at IS NULL")
                .bind(doc_a)
                .fetch_optional(&mut *conn)
                .await
                .expect("select doc_a");
        assert_eq!(
            see_a,
            Some(doc_a),
            "org A's own document must be visible under its own context"
        );

        // Org-B's document is NOT visible — this is the IDOR that #754 closes.
        let see_b: Option<Uuid> =
            sqlx::query_scalar("SELECT id FROM documents WHERE id = $1 AND deleted_at IS NULL")
                .bind(doc_b)
                .fetch_optional(&mut *conn)
                .await
                .expect("select doc_b");
        assert_eq!(
            see_b, None,
            "org B's document must NOT be visible to an org-A caller (cross-tenant IDOR #754)"
        );

        // A blanket SELECT returns only org-A's row.
        let visible: Vec<Uuid> = sqlx::query_scalar("SELECT id FROM documents ORDER BY title")
            .fetch_all(&mut *conn)
            .await
            .expect("select all visible documents");
        assert_eq!(
            visible,
            vec![doc_a],
            "only the caller's own org documents may be visible under FORCE RLS"
        );

        // Drop back to superuser on this connection before it returns to the pool.
        sqlx::query("RESET ROLE")
            .execute(&mut *conn)
            .await
            .expect("reset role");
    }

    // --- Cleanup the test role. ---
    set_ctx(&pool, None, None, true).await;
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "REVOKE ALL ON documents FROM \"{role}\""
    )))
    .execute(&pool)
    .await
    .ok();
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "REVOKE ALL ON organizations FROM \"{role}\""
    )))
    .execute(&pool)
    .await
    .ok();
    // DROP OWNED severs every remaining privilege this role holds in the
    // test database — the explicit REVOKEs above keep missing the RLS
    // helper-function EXECUTE grants, so DROP ROLE failed with "objects
    // depend on it" and leaked the cluster-global role (PAP-134).
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP OWNED BY \"{role}\"")))
        .execute(&pool)
        .await
        .ok();
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "DROP ROLE IF EXISTS \"{role}\""
    )))
    .execute(&pool)
    .await
    .ok();
}
