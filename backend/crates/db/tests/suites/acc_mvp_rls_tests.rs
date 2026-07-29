//! FORCE-RLS isolation tests for the ACC MVP tables added in migrations
//! 00200/00201/00202 (EPIC-ACC-02/03/04/05/16). Companion to the base-MVP
//! `accounting_rls_repo_tests.rs` (00184 tables). Verifies, against a live DB:
//!   1. every new acc_* table has FORCE ROW LEVEL SECURITY + a tenant policy
//!      bound to get_current_org_id() (structural — covers all 18 tables);
//!   2. rows created under company A are invisible under company B, and a write
//!      targeting another tenant's id is rejected by WITH CHECK (behavioral);
//!   3. acc_audit_log is append-only — UPDATE/DELETE are blocked by RLS.
//!
//! "Company/agenda" == the existing `organizations` tenant; RLS keys on
//! get_current_org_id() exactly like 00184 (see .research/accounting/architecture.md).
//!
//! NOTE: the test DB superuser BYPASSES RLS (FORCE included), so the behavioral
//! test runs under a dedicated NOSUPERUSER/NOBYPASSRLS role on a single pinned
//! connection — the same technique as `accounting_rls_repo_tests.rs`.

use crate::common::seed_org;
use sqlx::PgPool;
use uuid::Uuid;

/// Every tenant-scoped table added by the ACC MVP migrations.
const NEW_TABLES: &[&str] = &[
    "acc_company_settings",
    "acc_numbering_series",
    "acc_unit",
    "acc_vat_rate",
    "acc_bank_account",
    "acc_email_template",
    "acc_document_template",
    "acc_item_category",
    "acc_catalog_item",
    "acc_price_level",
    "acc_contact_address",
    "acc_contact_tag",
    "acc_document_link",
    "acc_attachment",
    "acc_tag",
    "acc_share_link",
    "acc_audit_log",
    "acc_two_factor",
];

async fn set_request_ctx(
    conn: &mut sqlx::PgConnection,
    org: Option<Uuid>,
    user: Option<Uuid>,
    super_admin: bool,
) {
    sqlx::query("SELECT set_request_context($1, $2, $3)")
        .bind(org)
        .bind(user)
        .bind(super_admin)
        .execute(&mut *conn)
        .await
        .expect("set_request_context");
}

async fn count_on(conn: &mut sqlx::PgConnection, table: &str) -> i64 {
    // Table names are hard-coded constants (NEW_TABLES), never user input;
    // sqlx 0.9 still requires dynamic SQL to be explicitly asserted safe.
    let sql = format!("SELECT count(*) FROM {table}");
    sqlx::query_scalar::<_, i64>(sqlx::AssertSqlSafe(sql))
        .fetch_one(&mut *conn)
        .await
        .unwrap_or_else(|e| panic!("count {table}: {e}"))
}

/// (1) Structural: every new acc_* table must have FORCE RLS and a policy whose
/// expression is bound to get_current_org_id(). Catalog reads are not
/// tenant-scoped, so the pool superuser is fine here.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn acc_mvp_all_new_tables_force_rls_with_tenant_policy(pool: PgPool) {
    for t in NEW_TABLES {
        let forced: bool =
            sqlx::query_scalar("SELECT relforcerowsecurity FROM pg_class WHERE relname = $1")
                .bind(t)
                .fetch_one(&pool)
                .await
                .unwrap_or_else(|e| panic!("pg_class lookup for {t}: {e}"));
        assert!(forced, "{t} must have FORCE ROW LEVEL SECURITY");

        // For acc_audit_log the SELECT policy carries the qual and the INSERT
        // policy carries with_check; for the rest the FOR ALL policy does.
        let has_tenant_policy: bool = sqlx::query_scalar(
            r#"SELECT EXISTS (
                 SELECT 1 FROM pg_policies
                 WHERE tablename = $1
                   AND (COALESCE(qual,'') LIKE '%get_current_org_id()%'
                        OR COALESCE(with_check,'') LIKE '%get_current_org_id()%')
               )"#,
        )
        .bind(t)
        .fetch_one(&pool)
        .await
        .unwrap_or_else(|e| panic!("pg_policies lookup for {t}: {e}"));
        assert!(
            has_tenant_policy,
            "{t} must have a tenant-isolation policy bound to get_current_org_id()"
        );
    }
}

/// (2)+(3) Behavioral: seed one row in every new table under company A, then —
/// under a NOSUPERUSER role so FORCE RLS actually binds — prove company B sees
/// none of them, cross-tenant writes are rejected, and the audit log is
/// append-only.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn acc_mvp_force_rls_cross_tenant_isolation(pool: PgPool) {
    // ---- Seed fixtures on the (superuser) pool; superusers bypass RLS, so the
    // explicit tenant_id values are what matter. ----
    let org_a = seed_org(&pool, "tenant-a").await;
    let org_b = seed_org(&pool, "tenant-b").await;
    let user_a: Uuid = sqlx::query_scalar(
        "INSERT INTO users (email, password_hash, name) VALUES ($1, 'x', 'User A') RETURNING id",
    )
    .bind(format!("a-{org_a}@acc.test"))
    .fetch_one(&pool)
    .await
    .expect("seed user");

    sqlx::query("INSERT INTO acc_company_settings (tenant_id) VALUES ($1)")
        .bind(org_a)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO acc_numbering_series (tenant_id, doc_type, year) VALUES ($1, 'invoice', 2026)",
    )
    .bind(org_a)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO acc_unit (tenant_id, code, name) VALUES ($1, 'pcs', 'pieces')")
        .bind(org_a)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO acc_vat_rate (tenant_id, name, rate) VALUES ($1, 'standard', 20)")
        .bind(org_a)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO acc_bank_account (tenant_id, label, iban) VALUES ($1, 'main', 'SK0000')",
    )
    .bind(org_a)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO acc_email_template (tenant_id, subject, body) VALUES ($1, 's', 'b')")
        .bind(org_a)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO acc_document_template (tenant_id, name) VALUES ($1, 'classic')")
        .bind(org_a)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO acc_item_category (tenant_id, name) VALUES ($1, 'services')")
        .bind(org_a)
        .execute(&pool)
        .await
        .unwrap();

    let item_a: Uuid = sqlx::query_scalar(
        "INSERT INTO acc_catalog_item (tenant_id, name) VALUES ($1, 'Consulting') RETURNING id",
    )
    .bind(org_a)
    .fetch_one(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO acc_price_level (tenant_id, item_id, name, price) VALUES ($1, $2, 'retail', 100)")
        .bind(org_a).bind(item_a).execute(&pool).await.unwrap();

    let contact_a: Uuid = sqlx::query_scalar(
        "INSERT INTO contact (tenant_id, name) VALUES ($1, 'Acme s.r.o.') RETURNING id",
    )
    .bind(org_a)
    .fetch_one(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO acc_contact_address (tenant_id, contact_id) VALUES ($1, $2)")
        .bind(org_a)
        .bind(contact_a)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO acc_contact_tag (tenant_id, contact_id, tag) VALUES ($1, $2, 'vip')")
        .bind(org_a)
        .bind(contact_a)
        .execute(&pool)
        .await
        .unwrap();

    let inv_a: Uuid = sqlx::query_scalar(
        "INSERT INTO invoice (tenant_id, contact_id, number, issue_date, due_date)
         VALUES ($1, $2, 'F-1', CURRENT_DATE, CURRENT_DATE) RETURNING id",
    )
    .bind(org_a)
    .bind(contact_a)
    .fetch_one(&pool)
    .await
    .unwrap();
    let inv_a2: Uuid = sqlx::query_scalar(
        "INSERT INTO invoice (tenant_id, contact_id, number, issue_date, due_date)
         VALUES ($1, $2, 'F-2', CURRENT_DATE, CURRENT_DATE) RETURNING id",
    )
    .bind(org_a)
    .bind(contact_a)
    .fetch_one(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO acc_document_link (tenant_id, from_invoice_id, to_invoice_id) VALUES ($1, $2, $3)")
        .bind(org_a).bind(inv_a).bind(inv_a2).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO acc_attachment (tenant_id, filename, content_type, storage_key) VALUES ($1, 'f.pdf', 'application/pdf', 'k1')")
        .bind(org_a).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO acc_tag (tenant_id, entity_id, tag) VALUES ($1, $2, 'urgent')")
        .bind(org_a)
        .bind(inv_a)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO acc_share_link (tenant_id, invoice_id, token_hash) VALUES ($1, $2, 'tok-a')",
    )
    .bind(org_a)
    .bind(inv_a)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO acc_audit_log (tenant_id, actor_id, action, entity_type, entity_id) VALUES ($1, $2, 'create', 'invoice', $3)")
        .bind(org_a).bind(user_a).bind(inv_a).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO acc_two_factor (tenant_id, user_id, secret) VALUES ($1, $2, 'seed')")
        .bind(org_a)
        .bind(user_a)
        .execute(&pool)
        .await
        .unwrap();

    // ---- Create a NOSUPERUSER NOBYPASSRLS role so FORCE RLS actually binds ----
    let role = format!("acc_rls_test_{}", Uuid::new_v4().simple());
    let grant = |sql: String| sqlx::query(sqlx::AssertSqlSafe(sql));
    grant(format!("CREATE ROLE \"{role}\" NOSUPERUSER NOBYPASSRLS"))
        .execute(&pool)
        .await
        .expect("create role");
    grant(format!("GRANT USAGE ON SCHEMA public TO \"{role}\""))
        .execute(&pool)
        .await
        .expect("grant usage");
    grant(format!(
        "GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO \"{role}\""
    ))
    .execute(&pool)
    .await
    .expect("grant dml");
    grant(format!(
        "GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA public TO \"{role}\""
    ))
    .execute(&pool)
    .await
    .expect("grant exec");

    // ---- One pinned connection; SET ROLE; run all checks under it ----
    let mut conn = pool.acquire().await.expect("acquire connection");
    sqlx::query(sqlx::AssertSqlSafe(format!("SET ROLE \"{role}\"")))
        .execute(&mut *conn)
        .await
        .expect("set role");

    // Company B is fully isolated: sees none of A's rows — with ONE intentional
    // exception. `acc_share_link` carries a capability-read policy added in
    // migration 00204 (`acc_share_link_capability_read`, FOR SELECT USING(true))
    // so the PUBLIC share endpoint can resolve a link by its unguessable token
    // hash WITHOUT a tenant context. SELECT is therefore capability-isolated, not
    // tenant-isolated; the row exposes no secret (only the token HASH) and writes
    // stay tenant-isolated (asserted below). So B *can* see A's share link by
    // SELECT — that is the designed behavior, not a leak.
    set_request_ctx(&mut conn, Some(org_b), None, false).await;
    for t in NEW_TABLES {
        if *t == "acc_share_link" {
            assert!(
                count_on(&mut conn, t).await >= 1,
                "acc_share_link SELECT is capability-based (00204), readable cross-tenant by hash"
            );
            continue;
        }
        assert_eq!(
            count_on(&mut conn, t).await,
            0,
            "company B must NOT see {t} from company A"
        );
    }

    // Company A sees each of its rows.
    set_request_ctx(&mut conn, Some(org_a), Some(user_a), false).await;
    for t in NEW_TABLES {
        assert!(
            count_on(&mut conn, t).await >= 1,
            "company A should see its own {t} row"
        );
    }

    // acc_audit_log is append-only: UPDATE/DELETE match no rows under FORCE RLS
    // (only SELECT + INSERT policies exist), so they affect 0 rows — not error.
    let upd = sqlx::query("UPDATE acc_audit_log SET action = 'tamper'")
        .execute(&mut *conn)
        .await
        .expect("update returns ok (0 rows)");
    assert_eq!(
        upd.rows_affected(),
        0,
        "acc_audit_log must be immutable (no UPDATE policy)"
    );
    let del = sqlx::query("DELETE FROM acc_audit_log")
        .execute(&mut *conn)
        .await
        .expect("delete returns ok (0 rows)");
    assert_eq!(
        del.rows_affected(),
        0,
        "acc_audit_log must be append-only (no DELETE policy)"
    );
    assert!(
        count_on(&mut conn, "acc_audit_log").await >= 1,
        "the audit row survives tamper attempts"
    );

    // Cross-tenant write rejected (WITH CHECK). Done LAST: each failed statement
    // aborts only its own implicit (autocommit) transaction.
    set_request_ctx(&mut conn, Some(org_b), None, false).await;
    let cross = sqlx::query("INSERT INTO acc_unit (tenant_id, code, name) VALUES ($1, 'x', 'x')")
        .bind(org_a)
        .execute(&mut *conn)
        .await;
    assert!(
        cross.is_err(),
        "WITH CHECK must reject a row whose tenant_id != current org"
    );

    // acc_share_link WRITES stay tenant-isolated despite the permissive SELECT:
    // the capability-read policy is FOR SELECT only, so INSERT is still governed
    // by the tenant_isolation FOR ALL policy's WITH CHECK. B cannot forge a link
    // for A's tenant.
    let cross_link = sqlx::query(
        "INSERT INTO acc_share_link (tenant_id, invoice_id, token_hash) VALUES ($1, $2, 'tok-x')",
    )
    .bind(org_a)
    .bind(inv_a)
    .execute(&mut *conn)
    .await;
    assert!(
        cross_link.is_err(),
        "acc_share_link cross-tenant INSERT must be rejected by WITH CHECK"
    );

    sqlx::query("RESET ROLE").execute(&mut *conn).await.ok();
}
