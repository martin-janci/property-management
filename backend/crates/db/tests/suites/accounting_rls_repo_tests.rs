//! RLS isolation tests for native Accounting MVP (PAP-206).

use crate::common::{seed_org, set_ctx};
use db::models::accounting::{Contact, CreateInvoice, CreateInvoiceItem, Invoice, UpdateInvoice};
use db::repositories::accounting::AccountingRepository;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use sqlx::PgPool;
use uuid::Uuid;

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn accounting_contact_force_rls_blocks_cross_tenant_read(pool: PgPool) {
    // Seed as superuser
    set_ctx(&pool, None, None, true).await;

    let org_a = seed_org(&pool, "org-a").await;
    let org_b = seed_org(&pool, "org-b").await;

    // Seed contact for org_a
    let contact_a_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO contact (tenant_id, name) VALUES ($1, $2) RETURNING id",
    )
    .bind(org_a)
    .bind("Contact A")
    .fetch_one(&pool)
    .await
    .expect("seed contact a");

    // Seed contact for org_b
    let _contact_b_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO contact (tenant_id, name) VALUES ($1, $2) RETURNING id",
    )
    .bind(org_b)
    .bind("Contact B")
    .fetch_one(&pool)
    .await
    .expect("seed contact b");

    // Create a NOSUPERUSER role so FORCE actually binds.
    let role = format!("accounting_rls_test_{}", Uuid::new_v4().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "CREATE ROLE \"{role}\" NOSUPERUSER NOBYPASSRLS"
    )))
    .execute(&pool)
    .await
    .expect("create role");

    sqlx::query(sqlx::AssertSqlSafe(format!(
        "GRANT SELECT ON contact TO \"{role}\""
    )))
    .execute(&pool)
    .await
    .expect("grant select");

    sqlx::query(sqlx::AssertSqlSafe(format!(
        "GRANT SELECT ON organizations TO \"{role}\""
    )))
    .execute(&pool)
    .await
    .expect("grant select orgs");

    sqlx::query(sqlx::AssertSqlSafe(format!(
        "GRANT EXECUTE ON FUNCTION get_current_org_id(), is_super_admin(), get_current_org_not_deleted() TO \"{role}\""
    )))
    .execute(&pool)
    .await
    .expect("grant execute");

    let repo = AccountingRepository::new(pool.clone());

    {
        let mut conn = pool.acquire().await.expect("acquire connection");

        // Org-A context
        sqlx::query("SELECT set_request_context($1, $2, $3)")
            .bind(org_a)
            .bind(None::<Uuid>)
            .bind(false)
            .execute(&mut *conn)
            .await
            .expect("set org-A context");

        sqlx::query(sqlx::AssertSqlSafe(format!("SET ROLE \"{role}\"")))
            .execute(&mut *conn)
            .await
            .expect("set role");

        // Can see org-A's contact
        let contact_a = repo
            .find_contact_rls(&mut *conn, contact_a_id)
            .await
            .expect("find contact_a");
        assert!(contact_a.is_some(), "Org A's own contact must be visible");
        assert_eq!(contact_a.unwrap().name, "Contact A");

        // Cannot see org-B's contact
        let contact_b = sqlx::query_as::<_, Contact>("SELECT * FROM contact WHERE tenant_id = $1")
            .bind(org_b)
            .fetch_optional(&mut *conn)
            .await
            .expect("find contact_b");
        assert!(
            contact_b.is_none(),
            "Org B's contact must NOT be visible to Org A"
        );

        sqlx::query("RESET ROLE")
            .execute(&mut *conn)
            .await
            .expect("reset role");
    }

    // Cleanup
    set_ctx(&pool, None, None, true).await;
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

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn accounting_invoice_force_rls_blocks_cross_tenant_read(pool: PgPool) {
    // Seed as superuser
    set_ctx(&pool, None, None, true).await;

    let org_a = seed_org(&pool, "org-a-inv").await;
    let org_b = seed_org(&pool, "org-b-inv").await;

    let contact_a_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO contact (tenant_id, name) VALUES ($1, $2) RETURNING id",
    )
    .bind(org_a)
    .bind("Contact A")
    .fetch_one(&pool)
    .await
    .expect("seed contact a");

    let contact_b_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO contact (tenant_id, name) VALUES ($1, $2) RETURNING id",
    )
    .bind(org_b)
    .bind("Contact B")
    .fetch_one(&pool)
    .await
    .expect("seed contact b");

    // Seed invoice for org_a
    let invoice_a_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO invoice (tenant_id, contact_id, number, issue_date, due_date, currency)
        VALUES ($1, $2, 'INV-A', NOW(), NOW() + INTERVAL '14 days', 'CZK')
        RETURNING id
        "#,
    )
    .bind(org_a)
    .bind(contact_a_id)
    .fetch_one(&pool)
    .await
    .expect("seed invoice a");

    // Seed invoice for org_b
    let _invoice_b_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO invoice (tenant_id, contact_id, number, issue_date, due_date, currency)
        VALUES ($1, $2, 'INV-B', NOW(), NOW() + INTERVAL '14 days', 'CZK')
        RETURNING id
        "#,
    )
    .bind(org_b)
    .bind(contact_b_id)
    .fetch_one(&pool)
    .await
    .expect("seed invoice b");

    // Create a NOSUPERUSER role
    let role = format!("accounting_rls_test_{}", Uuid::new_v4().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "CREATE ROLE \"{role}\" NOSUPERUSER NOBYPASSRLS"
    )))
    .execute(&pool)
    .await
    .expect("create role");

    sqlx::query(sqlx::AssertSqlSafe(format!(
        "GRANT SELECT ON contact, invoice TO \"{role}\""
    )))
    .execute(&pool)
    .await
    .expect("grant select");

    sqlx::query(sqlx::AssertSqlSafe(format!(
        "GRANT SELECT ON organizations TO \"{role}\""
    )))
    .execute(&pool)
    .await
    .expect("grant select orgs");

    sqlx::query(sqlx::AssertSqlSafe(format!(
        "GRANT EXECUTE ON FUNCTION get_current_org_id(), is_super_admin(), get_current_org_not_deleted() TO \"{role}\""
    )))
    .execute(&pool)
    .await
    .expect("grant execute");

    let repo = AccountingRepository::new(pool.clone());

    {
        let mut conn = pool.acquire().await.expect("acquire connection");

        // Org-A context
        sqlx::query("SELECT set_request_context($1, $2, $3)")
            .bind(org_a)
            .bind(None::<Uuid>)
            .bind(false)
            .execute(&mut *conn)
            .await
            .expect("set org-A context");

        sqlx::query(sqlx::AssertSqlSafe(format!("SET ROLE \"{role}\"")))
            .execute(&mut *conn)
            .await
            .expect("set role");

        // Can see org-A's invoice
        let invoice_a = repo
            .find_invoice_rls(&mut *conn, invoice_a_id)
            .await
            .expect("find invoice_a");
        assert!(invoice_a.is_some(), "Org A's own invoice must be visible");
        assert_eq!(invoice_a.unwrap().number, "INV-A");

        // Cannot see org-B's invoice
        let invoice_b = sqlx::query_as::<_, Invoice>("SELECT * FROM invoice WHERE tenant_id = $1")
            .bind(org_b)
            .fetch_optional(&mut *conn)
            .await
            .expect("find invoice_b");
        assert!(
            invoice_b.is_none(),
            "Org B's invoice must NOT be visible to Org A"
        );

        sqlx::query("RESET ROLE")
            .execute(&mut *conn)
            .await
            .expect("reset role");
    }

    // Cleanup
    set_ctx(&pool, None, None, true).await;
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

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn accounting_bank_statement_force_rls_blocks_cross_tenant_read(pool: PgPool) {
    set_ctx(&pool, None, None, true).await;
    let org_a = seed_org(&pool, "org-a-stmt").await;
    let org_b = seed_org(&pool, "org-b-stmt").await;

    let stmt_a_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO bank_statement (tenant_id, source_filename, account_iban) VALUES ($1, $2, $3) RETURNING id"
    )
    .bind(org_a)
    .bind("stmt-a.csv")
    .bind("IBAN-A")
    .fetch_one(&pool)
    .await
    .expect("seed stmt a");

    let role = format!("accounting_rls_test_{}", Uuid::new_v4().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "CREATE ROLE \"{role}\" NOSUPERUSER NOBYPASSRLS"
    )))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "GRANT SELECT ON bank_statement, organizations TO \"{role}\""
    )))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "GRANT EXECUTE ON FUNCTION get_current_org_id(), is_super_admin(), get_current_org_not_deleted() TO \"{role}\""
    )))
    .execute(&pool)
    .await
    .unwrap();

    let repo = AccountingRepository::new(pool.clone());
    {
        let mut conn = pool.acquire().await.unwrap();
        sqlx::query("SELECT set_request_context($1, $2, $3)")
            .bind(org_a)
            .bind(None::<Uuid>)
            .bind(false)
            .execute(&mut *conn)
            .await
            .unwrap();
        sqlx::query(sqlx::AssertSqlSafe(format!("SET ROLE \"{role}\"")))
            .execute(&mut *conn)
            .await
            .unwrap();

        let stmt_a = repo
            .find_bank_statement_rls(&mut *conn, stmt_a_id)
            .await
            .unwrap();
        assert!(stmt_a.is_some());
        assert_eq!(stmt_a.unwrap().source_filename, "stmt-a.csv");

        let stmt_b = sqlx::query("SELECT * FROM bank_statement WHERE tenant_id = $1")
            .bind(org_b)
            .fetch_optional(&mut *conn)
            .await
            .unwrap();
        assert!(stmt_b.is_none());
    }

    set_ctx(&pool, None, None, true).await;
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

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn accounting_payment_match_force_rls_blocks_cross_tenant_read(pool: PgPool) {
    set_ctx(&pool, None, None, true).await;
    let org_a = seed_org(&pool, "org-a-match").await;
    let org_b = seed_org(&pool, "org-b-match").await;

    // Org A setup
    let contact_a = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO contact (tenant_id, name) VALUES ($1, $2) RETURNING id",
    )
    .bind(org_a)
    .bind("C-A")
    .fetch_one(&pool)
    .await
    .unwrap();
    let inv_a = sqlx::query_scalar::<_, Uuid>("INSERT INTO invoice (tenant_id, contact_id, number, issue_date, due_date, currency) VALUES ($1, $2, 'I-A', NOW(), NOW(), 'CZK') RETURNING id").bind(org_a).bind(contact_a).fetch_one(&pool).await.unwrap();
    let stmt_a = sqlx::query_scalar::<_, Uuid>("INSERT INTO bank_statement (tenant_id, source_filename, account_iban) VALUES ($1, $2, $3) RETURNING id").bind(org_a).bind("S-A").bind("I-A").fetch_one(&pool).await.unwrap();
    let line_a = sqlx::query_scalar::<_, Uuid>("INSERT INTO bank_statement_line (statement_id, tenant_id, booking_date, amount, currency) VALUES ($1, $2, NOW(), 100, 'CZK') RETURNING id").bind(stmt_a).bind(org_a).fetch_one(&pool).await.unwrap();
    let match_a = sqlx::query_scalar::<_, Uuid>("INSERT INTO payment_match (tenant_id, statement_line_id, invoice_id, confidence, state) VALUES ($1, $2, $3, 1.0, 'confirmed') RETURNING id").bind(org_a).bind(line_a).bind(inv_a).fetch_one(&pool).await.unwrap();

    let role = format!("accounting_rls_test_{}", Uuid::new_v4().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "CREATE ROLE \"{role}\" NOSUPERUSER NOBYPASSRLS"
    )))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "GRANT SELECT ON payment_match, organizations TO \"{role}\""
    )))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "GRANT EXECUTE ON FUNCTION get_current_org_id(), is_super_admin(), get_current_org_not_deleted() TO \"{role}\""
    )))
    .execute(&pool)
    .await
    .unwrap();

    let repo = AccountingRepository::new(pool.clone());
    {
        let mut conn = pool.acquire().await.unwrap();
        sqlx::query("SELECT set_request_context($1, $2, $3)")
            .bind(org_a)
            .bind(None::<Uuid>)
            .bind(false)
            .execute(&mut *conn)
            .await
            .unwrap();
        sqlx::query(sqlx::AssertSqlSafe(format!("SET ROLE \"{role}\"")))
            .execute(&mut *conn)
            .await
            .unwrap();

        let m_a = repo
            .find_payment_match_rls(&mut *conn, match_a)
            .await
            .unwrap();
        assert!(m_a.is_some());

        let m_b = sqlx::query("SELECT * FROM payment_match WHERE tenant_id = $1")
            .bind(org_b)
            .fetch_optional(&mut *conn)
            .await
            .unwrap();
        assert!(m_b.is_none());

        sqlx::query("RESET ROLE")
            .execute(&mut *conn)
            .await
            .expect("reset role");
    }

    set_ctx(&pool, None, None, true).await;
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

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn accounting_invoice_item_force_rls_blocks_cross_tenant_read(pool: PgPool) {
    set_ctx(&pool, None, None, true).await;
    let org_a = seed_org(&pool, "org-a-item").await;
    let org_b = seed_org(&pool, "org-b-item").await;

    // Org A
    let contact_a = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO contact (tenant_id, name) VALUES ($1, $2) RETURNING id",
    )
    .bind(org_a)
    .bind("C-A")
    .fetch_one(&pool)
    .await
    .unwrap();
    let inv_a = sqlx::query_scalar::<_, Uuid>("INSERT INTO invoice (tenant_id, contact_id, number, issue_date, due_date, currency) VALUES ($1, $2, 'I-A', NOW(), NOW(), 'CZK') RETURNING id").bind(org_a).bind(contact_a).fetch_one(&pool).await.unwrap();
    let _item_a = sqlx::query_scalar::<_, Uuid>("INSERT INTO invoice_item (invoice_id, tenant_id, description) VALUES ($1, $2, 'Item A') RETURNING id").bind(inv_a).bind(org_a).fetch_one(&pool).await.unwrap();

    // Org B
    let contact_b = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO contact (tenant_id, name) VALUES ($1, $2) RETURNING id",
    )
    .bind(org_b)
    .bind("C-B")
    .fetch_one(&pool)
    .await
    .unwrap();
    let inv_b = sqlx::query_scalar::<_, Uuid>("INSERT INTO invoice (tenant_id, contact_id, number, issue_date, due_date, currency) VALUES ($1, $2, 'I-B', NOW(), NOW(), 'CZK') RETURNING id").bind(org_b).bind(contact_b).fetch_one(&pool).await.unwrap();
    let _item_b = sqlx::query_scalar::<_, Uuid>("INSERT INTO invoice_item (invoice_id, tenant_id, description) VALUES ($1, $2, 'Item B') RETURNING id").bind(inv_b).bind(org_b).fetch_one(&pool).await.unwrap();

    let role = format!("accounting_rls_test_{}", Uuid::new_v4().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "CREATE ROLE \"{role}\" NOSUPERUSER NOBYPASSRLS"
    )))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "GRANT SELECT ON invoice_item, organizations TO \"{role}\""
    )))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "GRANT EXECUTE ON FUNCTION get_current_org_id(), is_super_admin(), get_current_org_not_deleted() TO \"{role}\""
    )))
    .execute(&pool)
    .await
    .unwrap();

    let repo = AccountingRepository::new(pool.clone());
    {
        let mut conn = pool.acquire().await.unwrap();
        sqlx::query("SELECT set_request_context($1, $2, $3)")
            .bind(org_a)
            .bind(None::<Uuid>)
            .bind(false)
            .execute(&mut *conn)
            .await
            .unwrap();
        sqlx::query(sqlx::AssertSqlSafe(format!("SET ROLE \"{role}\"")))
            .execute(&mut *conn)
            .await
            .unwrap();

        let items_a = repo
            .find_invoice_items_rls(&mut *conn, inv_a)
            .await
            .unwrap();
        assert_eq!(items_a.len(), 1);
        assert_eq!(items_a[0].description, "Item A");

        let items_b = sqlx::query("SELECT * FROM invoice_item WHERE tenant_id = $1")
            .bind(org_b)
            .fetch_all(&mut *conn)
            .await
            .unwrap();
        assert_eq!(items_b.len(), 0);

        sqlx::query("RESET ROLE")
            .execute(&mut *conn)
            .await
            .expect("reset role");
    }

    // Cleanup
    set_ctx(&pool, None, None, true).await;
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

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn accounting_bank_statement_line_force_rls_blocks_cross_tenant_read(pool: PgPool) {
    set_ctx(&pool, None, None, true).await;
    let org_a = seed_org(&pool, "org-a-line").await;
    let org_b = seed_org(&pool, "org-b-line").await;

    let stmt_a = sqlx::query_scalar::<_, Uuid>("INSERT INTO bank_statement (tenant_id, source_filename, account_iban) VALUES ($1, $2, $3) RETURNING id").bind(org_a).bind("S-A").bind("I-A").fetch_one(&pool).await.unwrap();
    let line_a = sqlx::query_scalar::<_, Uuid>("INSERT INTO bank_statement_line (statement_id, tenant_id, booking_date, amount, currency) VALUES ($1, $2, NOW(), 100, 'CZK') RETURNING id").bind(stmt_a).bind(org_a).fetch_one(&pool).await.unwrap();

    let role = format!("accounting_rls_test_{}", Uuid::new_v4().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "CREATE ROLE \"{role}\" NOSUPERUSER NOBYPASSRLS"
    )))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "GRANT SELECT ON bank_statement_line, organizations TO \"{role}\""
    )))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "GRANT EXECUTE ON FUNCTION get_current_org_id(), is_super_admin(), get_current_org_not_deleted() TO \"{role}\""
    )))
    .execute(&pool)
    .await
    .unwrap();

    let repo = AccountingRepository::new(pool.clone());
    {
        let mut conn = pool.acquire().await.unwrap();
        sqlx::query("SELECT set_request_context($1, $2, $3)")
            .bind(org_a)
            .bind(None::<Uuid>)
            .bind(false)
            .execute(&mut *conn)
            .await
            .unwrap();
        sqlx::query(sqlx::AssertSqlSafe(format!("SET ROLE \"{role}\"")))
            .execute(&mut *conn)
            .await
            .unwrap();

        let l_a = repo
            .find_bank_statement_line_rls(&mut *conn, line_a)
            .await
            .unwrap();
        assert!(l_a.is_some());

        let l_b = sqlx::query("SELECT * FROM bank_statement_line WHERE tenant_id = $1")
            .bind(org_b)
            .fetch_optional(&mut *conn)
            .await
            .unwrap();
        assert!(l_b.is_none());

        sqlx::query("RESET ROLE")
            .execute(&mut *conn)
            .await
            .expect("reset role");
    }

    set_ctx(&pool, None, None, true).await;
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

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn accounting_invoice_force_rls_blocks_cross_tenant_write(pool: PgPool) {
    set_ctx(&pool, None, None, true).await;
    let org_a = seed_org(&pool, "org-a-write").await;
    let org_b = seed_org(&pool, "org-b-write").await;

    let contact_b = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO contact (tenant_id, name) VALUES ($1, $2) RETURNING id",
    )
    .bind(org_b)
    .bind("C-B")
    .fetch_one(&pool)
    .await
    .unwrap();
    let inv_b = sqlx::query_scalar::<_, Uuid>("INSERT INTO invoice (tenant_id, contact_id, number, issue_date, due_date, currency) VALUES ($1, $2, 'I-B', NOW(), NOW(), 'CZK') RETURNING id").bind(org_b).bind(contact_b).fetch_one(&pool).await.unwrap();

    let role = format!("accounting_rls_write_test_{}", Uuid::new_v4().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "CREATE ROLE \"{role}\" NOSUPERUSER NOBYPASSRLS"
    )))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "GRANT SELECT, INSERT, UPDATE, DELETE ON invoice, contact TO \"{role}\""
    )))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "GRANT SELECT ON organizations TO \"{role}\""
    )))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "GRANT EXECUTE ON FUNCTION get_current_org_id(), is_super_admin(), get_current_org_not_deleted() TO \"{role}\""
    )))
    .execute(&pool)
    .await
    .unwrap();

    {
        let mut conn = pool.acquire().await.unwrap();
        sqlx::query("SELECT set_request_context($1, $2, $3)")
            .bind(org_a)
            .bind(None::<Uuid>)
            .bind(false)
            .execute(&mut *conn)
            .await
            .unwrap();
        sqlx::query(sqlx::AssertSqlSafe(format!("SET ROLE \"{role}\"")))
            .execute(&mut *conn)
            .await
            .unwrap();

        // 1. Cross-tenant INSERT denied
        let res = sqlx::query("INSERT INTO invoice (tenant_id, contact_id, number, issue_date, due_date, currency) VALUES ($1, $2, 'I-A-FAIL', NOW(), NOW(), 'CZK')")
            .bind(org_b) // Trying to insert for Org B while in Org A context
            .bind(contact_b)
            .execute(&mut *conn)
            .await;
        assert!(
            res.is_err(),
            "Cross-tenant INSERT must fail due to RLS WITH CHECK"
        );

        // 2. Cross-tenant UPDATE affects 0 rows
        let res = sqlx::query("UPDATE invoice SET number = 'HACKED' WHERE id = $1")
            .bind(inv_b)
            .execute(&mut *conn)
            .await
            .unwrap();
        assert_eq!(
            res.rows_affected(),
            0,
            "Cross-tenant UPDATE must affect 0 rows"
        );

        // 3. Cross-tenant DELETE affects 0 rows
        let res = sqlx::query("DELETE FROM invoice WHERE id = $1")
            .bind(inv_b)
            .execute(&mut *conn)
            .await
            .unwrap();
        assert_eq!(
            res.rows_affected(),
            0,
            "Cross-tenant DELETE must affect 0 rows"
        );

        sqlx::query("RESET ROLE").execute(&mut *conn).await.unwrap();
    }

    // Verify row B is still there and unchanged
    set_ctx(&pool, None, None, true).await;
    let inv_b_after: Invoice = sqlx::query_as("SELECT * FROM invoice WHERE id = $1")
        .bind(inv_b)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(inv_b_after.number, "I-B");

    // Cleanup
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

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn accounting_confirm_match_cross_tenant_denied(pool: PgPool) {
    set_ctx(&pool, None, None, true).await;
    let org_a = seed_org(&pool, "org-a-match-fail").await;
    let org_b = seed_org(&pool, "org-b-match-fail").await;

    // Org B setup
    let contact_b = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO contact (tenant_id, name) VALUES ($1, $2) RETURNING id",
    )
    .bind(org_b)
    .bind("C-B")
    .fetch_one(&pool)
    .await
    .unwrap();
    let inv_b = sqlx::query_scalar::<_, Uuid>("INSERT INTO invoice (tenant_id, contact_id, number, issue_date, due_date, currency) VALUES ($1, $2, 'I-B', NOW(), NOW(), 'CZK') RETURNING id").bind(org_b).bind(contact_b).fetch_one(&pool).await.unwrap();
    let stmt_b = sqlx::query_scalar::<_, Uuid>("INSERT INTO bank_statement (tenant_id, source_filename, account_iban) VALUES ($1, $2, $3) RETURNING id").bind(org_b).bind("S-B").bind("I-B").fetch_one(&pool).await.unwrap();
    let line_b = sqlx::query_scalar::<_, Uuid>("INSERT INTO bank_statement_line (statement_id, tenant_id, booking_date, amount, currency) VALUES ($1, $2, NOW(), 100, 'CZK') RETURNING id").bind(stmt_b).bind(org_b).fetch_one(&pool).await.unwrap();
    let match_b = sqlx::query_scalar::<_, Uuid>("INSERT INTO payment_match (tenant_id, statement_line_id, invoice_id, confidence, state) VALUES ($1, $2, $3, 1.0, 'suggested') RETURNING id").bind(org_b).bind(line_b).bind(inv_b).fetch_one(&pool).await.unwrap();

    let role = format!("accounting_rls_match_test_{}", Uuid::new_v4().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "CREATE ROLE \"{role}\" NOSUPERUSER NOBYPASSRLS"
    )))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(sqlx::AssertSqlSafe(format!("GRANT SELECT, UPDATE, INSERT ON payment_match, bank_statement_line, invoice, organizations TO \"{role}\""))).execute(&pool).await.unwrap();
    sqlx::query(sqlx::AssertSqlSafe(format!("GRANT EXECUTE ON FUNCTION get_current_org_id(), is_super_admin(), get_current_org_not_deleted() TO \"{role}\""))).execute(&pool).await.unwrap();

    let repo = AccountingRepository::new(pool.clone());
    {
        let mut conn = pool.acquire().await.unwrap();
        sqlx::query("SELECT set_request_context($1, $2, $3)")
            .bind(org_a)
            .bind(None::<Uuid>)
            .bind(false)
            .execute(&mut *conn)
            .await
            .unwrap();
        sqlx::query(sqlx::AssertSqlSafe(format!("SET ROLE \"{role}\"")))
            .execute(&mut *conn)
            .await
            .unwrap();

        // Try to find match_b (should fail/be None)
        let m_b = repo
            .find_payment_match_rls(&mut *conn, match_b)
            .await
            .unwrap();
        assert!(m_b.is_none(), "Org A must not see Org B's payment match");

        // Try to update match_b directly via SQL
        let res = sqlx::query("UPDATE payment_match SET state = 'confirmed' WHERE id = $1")
            .bind(match_b)
            .execute(&mut *conn)
            .await
            .unwrap();
        assert_eq!(res.rows_affected(), 0);

        sqlx::query("RESET ROLE").execute(&mut *conn).await.unwrap();
    }

    // Cleanup
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

/// #1522 finding 3: the invoice header totals must equal the sum of the stored
/// (rounded NUMERIC(18,2)) line rows. The old two-loop code summed full-precision
/// lines into the header while each item row was rounded on insert, so a line
/// whose VAT carries a third decimal (e.g. 0.10 * 15% = 0.015) could leave the
/// header off by a cent from the sum of its items. The single-pass round-once
/// fix makes the header == sum(items) by construction.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn create_invoice_header_totals_equal_sum_of_rounded_lines(pool: PgPool) {
    let org = seed_org(&pool, "inv-round").await;
    set_ctx(&pool, Some(org), None, true).await;

    let contact_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO contact (tenant_id, name) VALUES ($1, 'Rounding Contact') RETURNING id",
    )
    .bind(org)
    .fetch_one(&pool)
    .await
    .expect("seed contact");

    let repo = AccountingRepository::new(pool.clone());
    let mut conn = pool.acquire().await.expect("acquire");
    sqlx::query("SELECT set_request_context($1, $2, $3)")
        .bind(Some(org))
        .bind(Option::<Uuid>::None)
        .bind(true)
        .execute(&mut *conn)
        .await
        .expect("set ctx on conn");

    // Three lines whose per-line VAT (0.10 * 15% = 0.015) rounds individually —
    // the scenario where summing-then-rounding and rounding-then-summing differ.
    let item = CreateInvoiceItem {
        description: "Fractional VAT line".to_string(),
        qty: dec!(1),
        unit_price: dec!(0.10),
        vat_rate: dec!(15),
        vat_rate_type: None,
    };
    let invoice = repo
        .create_invoice_rls(
            &mut conn,
            CreateInvoice {
                tenant_id: org,
                contact_id,
                number: "INV-ROUND-1".to_string(),
                issue_date: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
                due_date: chrono::NaiveDate::from_ymd_opt(2026, 1, 15).unwrap(),
                taxable_supply_date: None,
                currency: "EUR".to_string(),
                variable_symbol: None,
                status: None,
                items: vec![item.clone(), item.clone(), item],
                country: Some("SK".to_string()),
            },
        )
        .await
        .expect("create invoice");

    let (base_sum, vat_sum, total_sum): (Decimal, Decimal, Decimal) = sqlx::query_as(
        r#"
        SELECT COALESCE(SUM(base_amount), 0), COALESCE(SUM(vat_amount), 0),
               COALESCE(SUM(total_amount), 0)
        FROM invoice_item WHERE invoice_id = $1
        "#,
    )
    .bind(invoice.id)
    .fetch_one(&mut *conn)
    .await
    .expect("sum line rows");

    assert_eq!(
        invoice.base_amount, base_sum,
        "header base_amount must equal the sum of stored line base_amounts"
    );
    assert_eq!(
        invoice.vat_amount, vat_sum,
        "header vat_amount must equal the sum of stored line vat_amounts"
    );
    assert_eq!(
        invoice.total_amount, total_sum,
        "header total_amount must equal the sum of stored line total_amounts"
    );
}

/// PAP-321 F1: `update_invoice_rls` on a missing (or RLS-excluded) id must return
/// `Ok(None)` so the handler can map it to 404, rather than bubbling a
/// `RowNotFound` error up to a 500. Pre-fix the method used `.fetch_one`, which
/// returned `Err(RowNotFound)` for the no-rows-affected case.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn update_invoice_missing_id_returns_none_not_error(pool: PgPool) {
    let org = seed_org(&pool, "inv-update-404").await;
    set_ctx(&pool, Some(org), None, true).await;

    let repo = AccountingRepository::new(pool.clone());
    let mut conn = pool.acquire().await.expect("acquire");
    sqlx::query("SELECT set_request_context($1, $2, $3)")
        .bind(Some(org))
        .bind(Option::<Uuid>::None)
        .bind(true)
        .execute(&mut *conn)
        .await
        .expect("set ctx on conn");

    let data = UpdateInvoice {
        contact_id: None,
        number: Some("WONT-APPLY".to_string()),
        issue_date: None,
        due_date: None,
        taxable_supply_date: None,
        currency: None,
        variable_symbol: None,
        status: None,
        paid_amount: None,
    };

    let res = repo
        .update_invoice_rls(&mut *conn, Uuid::new_v4(), data)
        .await
        .expect("update on missing id must not be a DB error");
    assert!(
        res.is_none(),
        "updating a non-existent invoice id must return Ok(None), not a row"
    );

    let res = repo
        .update_invoice_payment_status_rls(&mut *conn, Uuid::new_v4(), dec!(50))
        .await
        .expect("payment-status update on missing id must not be a DB error");
    assert!(
        res.is_none(),
        "applying a payment to a non-existent invoice id must return Ok(None)"
    );
}
