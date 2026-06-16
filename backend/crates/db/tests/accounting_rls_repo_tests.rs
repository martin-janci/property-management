//! RLS isolation tests for native Accounting MVP (PAP-206).

use db::models::accounting::{Contact, Invoice};
use db::repositories::accounting::AccountingRepository;
use sqlx::PgPool;
use uuid::Uuid;

async fn set_ctx(pool: &PgPool, org_id: Option<Uuid>, user_id: Option<Uuid>, is_super_admin: bool) {
    sqlx::query("SELECT set_request_context($1, $2, $3)")
        .bind(org_id)
        .bind(user_id)
        .bind(is_super_admin)
        .execute(pool)
        .await
        .expect("set_request_context");
}

async fn seed_org(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO organizations (name, slug, contact_email, status)
        VALUES ($1, $2, $3, 'active')
        RETURNING id
        "#,
    )
    .bind(format!("Accounting {slug}"))
    .bind(slug)
    .bind(format!("{slug}@accounting.test"))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

#[sqlx::test(migrator = "db::MIGRATOR")]
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
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "DROP OWNED BY \"{role}\""
    )))
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
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "DROP OWNED BY \"{role}\""
    )))
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
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "DROP OWNED BY \"{role}\""
    )))
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
async fn accounting_payment_match_force_rls_blocks_cross_tenant_read(pool: PgPool) {
    set_ctx(&pool, None, None, true).await;
    let org_a = seed_org(&pool, "org-a-match").await;
    let org_b = seed_org(&pool, "org-b-match").await;

    // Org A setup
    let contact_a =
        sqlx::query_scalar::<_, Uuid>("INSERT INTO contact (tenant_id, name) VALUES ($1, $2) RETURNING id")
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
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "DROP OWNED BY \"{role}\""
    )))
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
async fn accounting_invoice_item_force_rls_blocks_cross_tenant_read(pool: PgPool) {
    set_ctx(&pool, None, None, true).await;
    let org_a = seed_org(&pool, "org-a-item").await;
    let org_b = seed_org(&pool, "org-b-item").await;

    // Org A
    let contact_a =
        sqlx::query_scalar::<_, Uuid>("INSERT INTO contact (tenant_id, name) VALUES ($1, $2) RETURNING id")
            .bind(org_a)
            .bind("C-A")
            .fetch_one(&pool)
            .await
            .unwrap();
    let inv_a = sqlx::query_scalar::<_, Uuid>("INSERT INTO invoice (tenant_id, contact_id, number, issue_date, due_date, currency) VALUES ($1, $2, 'I-A', NOW(), NOW(), 'CZK') RETURNING id").bind(org_a).bind(contact_a).fetch_one(&pool).await.unwrap();
    let _item_a = sqlx::query_scalar::<_, Uuid>("INSERT INTO invoice_item (invoice_id, tenant_id, description) VALUES ($1, $2, 'Item A') RETURNING id").bind(inv_a).bind(org_a).fetch_one(&pool).await.unwrap();

    // Org B
    let contact_b =
        sqlx::query_scalar::<_, Uuid>("INSERT INTO contact (tenant_id, name) VALUES ($1, $2) RETURNING id")
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
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "DROP OWNED BY \"{role}\""
    )))
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
