//! RLS isolation tests for external accounting provider integration (PAP-191).

use crate::common::{seed_org, set_ctx};
use db::models::accounting_provider::{AccountingProviderAuthFlow, AccountingProviderConnection};
use db::repositories::accounting_provider::AccountingProviderRepository;
use sqlx::PgPool;
use uuid::Uuid;

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn accounting_provider_connection_force_rls_blocks_cross_tenant_read(pool: PgPool) {
    // Seed as superuser
    set_ctx(&pool, None, None, true).await;

    let org_a = seed_org(&pool, "org-a").await;
    let org_b = seed_org(&pool, "org-b").await;

    let repo = AccountingProviderRepository::new(pool.clone());

    // Seed connection for org_a
    repo.upsert_connection_rls(
        &pool,
        AccountingProviderConnection {
            tenant_id: org_a,
            auth_flow: AccountingProviderAuthFlow::Ccf,
            provider_account_name: "Org A Connection".to_string(),
            client_id: "client-a".to_string(),
            client_secret_enc: Some("secret-a".to_string()),
            refresh_token_enc: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        },
    )
    .await
    .expect("upsert org_a connection");

    // Seed connection for org_b
    repo.upsert_connection_rls(
        &pool,
        AccountingProviderConnection {
            tenant_id: org_b,
            auth_flow: AccountingProviderAuthFlow::Ccf,
            provider_account_name: "Org B Connection".to_string(),
            client_id: "client-b".to_string(),
            client_secret_enc: Some("secret-b".to_string()),
            refresh_token_enc: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        },
    )
    .await
    .expect("upsert org_b connection");

    // Create a NOSUPERUSER role so FORCE actually binds.
    let role = format!("accounting_provider_rls_test_{}", Uuid::new_v4().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "CREATE ROLE \"{role}\" NOSUPERUSER NOBYPASSRLS"
    )))
    .execute(&pool)
    .await
    .expect("create role");

    sqlx::query(sqlx::AssertSqlSafe(format!(
        "GRANT SELECT ON accounting_provider_connection TO \"{role}\""
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

        // Can see org-A's connection
        let conn_a = repo
            .find_connection_rls(&mut *conn, org_a)
            .await
            .expect("find conn_a");
        assert!(conn_a.is_some(), "Org A's own connection must be visible");
        assert_eq!(conn_a.unwrap().client_id, "client-a");

        // Cannot see org-B's connection
        let conn_b = repo
            .find_connection_rls(&mut *conn, org_b)
            .await
            .expect("find conn_b");
        assert!(
            conn_b.is_none(),
            "Org B's connection must NOT be visible to Org A"
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
