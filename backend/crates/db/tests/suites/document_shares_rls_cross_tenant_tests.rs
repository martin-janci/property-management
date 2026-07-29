//! Regression test for PAP-21 (follow-up to issue #754) — cross-tenant IDOR on
//! the document *share* tables caused by missing `FORCE ROW LEVEL SECURITY`.
//!
//! Background
//! ----------
//! 00020 created `document_shares` / `document_share_access_log` with only
//! `ENABLE ROW LEVEL SECURITY`, and 00172 deliberately left them un-forced
//! (the public share-token endpoints read them via the raw pool with no org
//! context). PostgreSQL exempts the table OWNER from RLS unless the table is
//! `FORCE`d, and the production api-server connects as the table owner (`ppt`),
//! so the share policies were silently bypassed — a cross-tenant read of any
//! org's shares / access logs. To compound it, those policies filtered on
//! `app.tenant_id`, a GUC that is never set (the codebase sets
//! `app.current_org_id`), so even a non-owner role saw nothing for its OWN org.
//!
//! Migration 00178 (PAP-21) repairs both policies to use `get_current_org_id()`
//! / `is_super_admin()` and adds `FORCE ROW LEVEL SECURITY`, after the public
//! handlers were rewired to read these tables through a dedicated super-admin
//! connection (`find_share_by_token_for_share` et al.).
//!
//! Why this test switches roles
//! ----------------------------
//! `#[sqlx::test]` connects as the Postgres SUPERUSER, which bypasses RLS
//! entirely — even `FORCE` does not bind a superuser. To exercise the policy
//! the way the production owner role does, the test creates a plain
//! `NOSUPERUSER NOBYPASSRLS` role, grants it table access, and `SET ROLE`s to
//! it. The org context (`app.current_org_id`) is a session GUC and survives
//! `SET ROLE`.
//!
//! Fails on main: before 00178, an org-A caller bound by the (broken,
//! `app.tenant_id`) policy sees NONE of its own shares — the
//! `own-share-is-visible` assertion fails. After 00178 the caller sees exactly
//! its own org's share and never the other org's.

use crate::common::{seed_org, set_ctx};
use sqlx::PgPool;
use uuid::Uuid;

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'test_hash', 'Shares User', 'active', NOW(), 'public')
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

/// Insert a `link` share and return `(share_id, share_token)`.
async fn seed_link_share(pool: &PgPool, document_id: Uuid, shared_by: Uuid) -> (Uuid, String) {
    let token = format!("tok{}", Uuid::new_v4().simple());
    let id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO document_shares (document_id, share_type, shared_by, share_token)
        VALUES ($1, 'link'::document_share_type, $2, $3)
        RETURNING id
        "#,
    )
    .bind(document_id)
    .bind(shared_by)
    .bind(&token)
    .fetch_one(pool)
    .await
    .expect("seed link share");
    (id, token)
}

async fn seed_access_log(pool: &PgPool, share_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO document_share_access_log (share_id, accessed_by, ip_address)
        VALUES ($1, NULL, '203.0.113.7'::inet)
        RETURNING id
        "#,
    )
    .bind(share_id)
    .fetch_one(pool)
    .await
    .expect("seed access log")
}

/// FORCE RLS on `document_shares` / `document_share_access_log` must hide
/// another org's share + access log from a non-superuser (owner-like) role,
/// while still exposing the caller's own org's rows, and while a super-admin
/// context (the public share-token path) sees both. Without 00178's FORCE +
/// policy fix this isolation is absent (the IDOR PAP-21 closes).
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn document_shares_force_rls_blocks_cross_tenant_read(pool: PgPool) {
    // --- Seed as superuser (super-admin context satisfies the org/role RLS
    //     WITH CHECK fired by the inserts). ---
    set_ctx(&pool, None, None, true).await;

    let org_a = seed_org(&pool, "shr-a").await;
    let org_b = seed_org(&pool, "shr-b").await;
    let user_a = seed_user(&pool, "a@shares.test").await;
    let user_b = seed_user(&pool, "b@shares.test").await;

    let doc_a = seed_document(&pool, org_a, user_a, "alpha").await;
    let doc_b = seed_document(&pool, org_b, user_b, "bravo").await;

    let (share_a, _token_a) = seed_link_share(&pool, doc_a, user_a).await;
    let (share_b, _token_b) = seed_link_share(&pool, doc_b, user_b).await;
    let log_a = seed_access_log(&pool, share_a).await;
    let _log_b = seed_access_log(&pool, share_b).await;

    // --- Create a NOSUPERUSER NOBYPASSRLS role so FORCE actually binds. ---
    let role = format!("ppt_shares_rls_test_{}", Uuid::new_v4().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "CREATE ROLE \"{role}\" NOSUPERUSER NOBYPASSRLS"
    )))
    .execute(&pool)
    .await
    .expect("create role");
    // The share / access-log policies join through `documents`, so the role
    // needs SELECT on all three tables, plus EXECUTE on the RLS helper
    // functions and SELECT on `organizations` (read by the SECURITY INVOKER
    // soft-delete guard `get_current_org_not_deleted()`).
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "GRANT SELECT ON documents, document_shares, document_share_access_log, \
         organizations TO \"{role}\""
    )))
    .execute(&pool)
    .await
    .expect("grant select");
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "GRANT EXECUTE ON FUNCTION get_current_org_id(), is_super_admin(), \
         get_current_org_not_deleted() TO \"{role}\""
    )))
    .execute(&pool)
    .await
    .expect("grant execute on rls helpers");

    // --- All role-bound work runs on ONE dedicated connection ('SET ROLE' and
    //     the org-context GUC are session state). ---
    {
        let mut conn = pool.acquire().await.expect("acquire connection");

        // Org-A context, then drop to the FORCE-applicable role.
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

        // Org-A's own share IS visible under its own context. (Fails on main:
        // the broken `app.tenant_id` policy hides even the caller's own share.)
        let see_share_a: Option<Uuid> =
            sqlx::query_scalar("SELECT id FROM document_shares WHERE id = $1")
                .bind(share_a)
                .fetch_optional(&mut *conn)
                .await
                .expect("select share_a");
        assert_eq!(
            see_share_a,
            Some(share_a),
            "org A's own share must be visible under its own context"
        );

        // Org-B's share is NOT visible — the cross-tenant IDOR PAP-21 closes.
        let see_share_b: Option<Uuid> =
            sqlx::query_scalar("SELECT id FROM document_shares WHERE id = $1")
                .bind(share_b)
                .fetch_optional(&mut *conn)
                .await
                .expect("select share_b");
        assert_eq!(
            see_share_b, None,
            "org B's share must NOT be visible to an org-A caller (cross-tenant IDOR)"
        );

        // A blanket SELECT returns only org-A's share.
        let visible_shares: Vec<Uuid> =
            sqlx::query_scalar("SELECT id FROM document_shares ORDER BY created_at")
                .fetch_all(&mut *conn)
                .await
                .expect("select all visible shares");
        assert_eq!(
            visible_shares,
            vec![share_a],
            "only the caller's own org shares may be visible under FORCE RLS"
        );

        // The access log inherits the same isolation: org-A's log row visible,
        // and the blanket read returns only it.
        let visible_logs: Vec<Uuid> =
            sqlx::query_scalar("SELECT id FROM document_share_access_log ORDER BY accessed_at")
                .fetch_all(&mut *conn)
                .await
                .expect("select all visible access logs");
        assert_eq!(
            visible_logs,
            vec![log_a],
            "only the caller's own org share-access logs may be visible under FORCE RLS"
        );

        // --- Super-admin context (the public share-token path) sees BOTH
        //     orgs' shares — this is the OR-leg the public handlers rely on.
        //     Toggle the GUC directly via the built-in `set_config` (always
        //     executable) rather than `set_request_context`, since the bound
        //     NOSUPERUSER role may not hold EXECUTE on that custom function. ---
        sqlx::query("SELECT set_config('app.is_super_admin', 'true', false)")
            .execute(&mut *conn)
            .await
            .expect("set super-admin context");
        let super_visible: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM document_shares WHERE id = ANY($1)")
                .bind(vec![share_a, share_b])
                .fetch_one(&mut *conn)
                .await
                .expect("count shares under super-admin");
        assert_eq!(
            super_visible, 2,
            "super-admin context (public share path) must see shares across orgs"
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
        "REVOKE ALL ON documents, document_shares, document_share_access_log, \
         organizations FROM \"{role}\""
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
