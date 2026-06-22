//! Behavioral RLS regression test for PAP-171 (defense-in-depth follow-up to
//! PAP-167 / PAP-150).
//!
//! Background
//! ----------
//! Migration `00102` ENABLEd RLS on the api-ecosystem catalog
//! (`marketplace_integrations`, …) and developer-portal
//! (`developer_accounts`, `developer_api_keys`, `developer_sandboxes`) tables
//! and attached correct policies, but never FORCEd them. The production
//! api-server connects as the table OWNER, which RLS exempts unless FORCE is
//! set — so those policies were never a real backstop and authorization was
//! enforced only in the handlers (`is_platform_admin()` gate for catalog
//! writes, `user_id` owner check for developer rows). Migration `00180`
//! (PAP-171) adds `FORCE ROW LEVEL SECURITY`, and the handlers were routed from
//! `acquire_public` to `acquire_with_rls(user / super-admin context)` so the
//! policies are actually exercised.
//!
//! This test proves, on a `FORCE`-bound NOSUPERUSER role (the way the
//! production owner experiences it):
//!
//!   1. **Cross-user isolation** — under user A's context, A sees only A's own
//!      developer account / API key / sandbox; user B's credential-class rows
//!      are invisible (a future handler bug cannot leak them across users).
//!      Exercised both with raw SQL counts and through the repository method
//!      the handler uses (`list_developer_api_keys`).
//!   2. **Deny-all without context** — with no RLS context set (what
//!      `acquire_public` would leave), the developer tables collapse to
//!      deny-all, confirming FORCE binds.
//!   3. **Catalog write authorization** — a super-admin context can INSERT into
//!      `marketplace_integrations` (platform-admin catalog writes still pass)
//!      while a plain user context is denied by the policy WITH CHECK.
//!   4. **Catalog public read** — the `USING (TRUE)` read policy keeps the
//!      catalog publicly readable even under FORCE with no super-admin context.
//!
//! Why this test switches roles
//! ----------------------------
//! `#[sqlx::test]` connects as the Postgres SUPERUSER, which bypasses RLS
//! entirely — even FORCE does not bind a superuser, so a behavioral assertion
//! would pass vacuously. The test creates a plain `NOSUPERUSER NOBYPASSRLS`
//! role, grants it table access, and `SET ROLE`s to it so FORCE enforces the
//! policy. Mirrors `api_ecosystem_rls_repo_tests.rs` (the PAP-110 precedent).

use db::repositories::ApiEcosystemRepository;
use sqlx::PgPool;
use uuid::Uuid;

async fn set_ctx(pool: &PgPool, user_id: Option<Uuid>, is_super_admin: bool) {
    sqlx::query("SELECT set_request_context($1, $2, $3)")
        .bind(Option::<Uuid>::None) // no org — developer/catalog tables are user/global-scoped
        .bind(user_id)
        .bind(is_super_admin)
        .execute(pool)
        .await
        .expect("set_request_context");
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'test_hash', 'Dev User', 'active', NOW(), 'public')
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

async fn seed_developer_account(pool: &PgPool, user_id: Uuid, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO developer_accounts (user_id, email, status)
        VALUES ($1, $2, 'approved')
        RETURNING id
        "#,
    )
    .bind(user_id)
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed developer account")
}

async fn seed_api_key(pool: &PgPool, developer_id: Uuid, name: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO developer_api_keys (developer_id, name, key_prefix, key_hash)
        VALUES ($1, $2, 'ppt_live', 'sha256:deadbeef')
        RETURNING id
        "#,
    )
    .bind(developer_id)
    .bind(name)
    .fetch_one(pool)
    .await
    .expect("seed api key")
}

async fn seed_sandbox(pool: &PgPool, developer_id: Uuid, name: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO developer_sandboxes (developer_id, name)
        VALUES ($1, $2)
        RETURNING id
        "#,
    )
    .bind(developer_id)
    .bind(name)
    .fetch_one(pool)
    .await
    .expect("seed sandbox")
}

async fn seed_oauth_app(pool: &PgPool, developer_id: Uuid, client_id: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO developer_oauth_apps
            (developer_id, name, client_id, client_secret_hash, redirect_uris)
        VALUES ($1, 'Test OAuth App', $2, 'sha256:secret', ARRAY['https://example.test/cb'])
        RETURNING id
        "#,
    )
    .bind(developer_id)
    .bind(client_id)
    .fetch_one(pool)
    .await
    .expect("seed oauth app")
}

async fn seed_oauth_grant(pool: &PgPool, oauth_app_id: Uuid, user_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO developer_oauth_grants (oauth_app_id, user_id, scopes)
        VALUES ($1, $2, ARRAY['read'])
        RETURNING id
        "#,
    )
    .bind(oauth_app_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("seed oauth grant")
}

async fn seed_usage_log(pool: &PgPool, api_key_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO api_key_usage_logs (api_key_id, endpoint, method, status_code)
        VALUES ($1, '/v1/ping', 'GET', 200)
        RETURNING id
        "#,
    )
    .bind(api_key_id)
    .fetch_one(pool)
    .await
    .expect("seed usage log")
}

/// Fail-on-`dev` guard (IG3): the eleven catalog + developer-portal tables must
/// carry `FORCE ROW LEVEL SECURITY`. A plain non-owner role (as the behavioral
/// test below uses) is bound by ENABLE alone, so only this catalog-metadata
/// assertion distinguishes the migration being present from absent — it fails
/// before migrations `00180`/`00181` and passes after. `#[sqlx::test]` runs as
/// a superuser, so a behavioral owner-bypass assertion would pass vacuously;
/// asserting `pg_class.relforcerowsecurity` is the robust check (mirrors
/// `epic11_org_guc_rls_tests.rs` / `notification_rls_guc_tests.rs`).
///
/// The three OAuth/usage tables (`developer_oauth_apps`, `developer_oauth_grants`,
/// `api_key_usage_logs`) were forced in `00181` — `developer_oauth_apps` holds
/// `client_secret_hash` (credential-class), making it the highest-value target.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn api_ecosystem_catalog_and_developer_tables_are_force_rls(pool: PgPool) {
    let tables = [
        "marketplace_integrations",
        "connectors",
        "connector_actions",
        "api_documentation",
        "api_code_samples",
        "developer_accounts",
        "developer_api_keys",
        "developer_sandboxes",
        // Added by migration 00181 (GH #1303 — remaining 00102 owner-bypass gap):
        "developer_oauth_apps",
        "developer_oauth_grants",
        "api_key_usage_logs",
    ];

    for table in tables {
        let forced: bool = sqlx::query_scalar(
            "SELECT relforcerowsecurity FROM pg_class WHERE relname = $1 AND relkind = 'r'",
        )
        .bind(table)
        .fetch_one(&pool)
        .await
        .unwrap_or_else(|e| panic!("query relforcerowsecurity for {table}: {e}"));

        assert!(
            forced,
            "PAP-171/GH-1303 regression: table `{table}` must be FORCE ROW LEVEL SECURITY \
             (migrations 00180/00181) so the owner app role is not exempt from its RLS policy"
        );

        // RLS must also remain enabled (FORCE without ENABLE is meaningless).
        let enabled: bool =
            sqlx::query_scalar("SELECT relrowsecurity FROM pg_class WHERE relname = $1")
                .bind(table)
                .fetch_one(&pool)
                .await
                .unwrap_or_else(|e| panic!("query relrowsecurity for {table}: {e}"));
        assert!(enabled, "table `{table}` must have RLS enabled");
    }
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn developer_portal_force_rls_cross_user_isolation(pool: PgPool) {
    let repo = ApiEcosystemRepository::new(pool.clone());

    // --- Seed as superuser. ---
    set_ctx(&pool, None, true).await;
    let user_a = seed_user(&pool, "force-a@dev.test").await;
    let user_b = seed_user(&pool, "force-b@dev.test").await;
    let dev_a = seed_developer_account(&pool, user_a, "force-a@dev.test").await;
    let dev_b = seed_developer_account(&pool, user_b, "force-b@dev.test").await;
    let key_a = seed_api_key(&pool, dev_a, "A live key").await;
    let key_b = seed_api_key(&pool, dev_b, "B confidential key").await;
    let _sandbox_a = seed_sandbox(&pool, dev_a, "A sandbox").await;
    let _sandbox_b = seed_sandbox(&pool, dev_b, "B sandbox").await;
    // OAuth apps + grants + usage logs for A and B — the three tables FORCEd by
    // migration 00181/00182 (GH #1303). Their policy predicates differ:
    // oauth_apps scopes by developer_id->user, oauth_grants by user_id directly,
    // and api_key_usage_logs by api_key_id->developer_api_keys->developer_accounts->user.
    let app_a = seed_oauth_app(&pool, dev_a, "client-a").await;
    let app_b = seed_oauth_app(&pool, dev_b, "client-b").await;
    let _grant_a = seed_oauth_grant(&pool, app_a, user_a).await;
    let grant_b = seed_oauth_grant(&pool, app_b, user_b).await;
    let _usage_a = seed_usage_log(&pool, key_a).await;
    let usage_b = seed_usage_log(&pool, key_b).await;

    // --- NOSUPERUSER NOBYPASSRLS role so FORCE actually binds. ---
    let role = format!("ppt_rls_devportal_{}", Uuid::new_v4().simple());
    for stmt in [
        format!("CREATE ROLE \"{role}\" NOSUPERUSER NOBYPASSRLS"),
        format!("GRANT SELECT, INSERT, UPDATE, DELETE ON developer_accounts TO \"{role}\""),
        format!("GRANT SELECT, INSERT, UPDATE, DELETE ON developer_api_keys TO \"{role}\""),
        format!("GRANT SELECT, INSERT, UPDATE, DELETE ON developer_sandboxes TO \"{role}\""),
        format!("GRANT SELECT, INSERT, UPDATE, DELETE ON developer_oauth_apps TO \"{role}\""),
        format!("GRANT SELECT, INSERT, UPDATE, DELETE ON developer_oauth_grants TO \"{role}\""),
        format!("GRANT SELECT, INSERT, UPDATE, DELETE ON api_key_usage_logs TO \"{role}\""),
        format!("GRANT SELECT, INSERT, UPDATE, DELETE ON marketplace_integrations TO \"{role}\""),
    ] {
        sqlx::query(sqlx::AssertSqlSafe(stmt))
            .execute(&pool)
            .await
            .expect("grant setup");
    }

    // ====================================================================
    // (2) DENY-ALL without context: role bound, NO context set (what
    //     `acquire_public` leaves). Developer tables collapse to deny-all.
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

        let visible: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM developer_api_keys")
            .fetch_one(&mut *conn)
            .await
            .expect("count keys (no ctx)");
        assert_eq!(
            visible, 0,
            "without RLS context the developer_api_keys table must be deny-all under FORCE"
        );

        // The three tables FORCEd by 00181/00182 (GH #1303) also collapse to
        // deny-all without a context, mirroring developer_api_keys.
        for (table, label) in [
            ("developer_oauth_apps", "OAuth apps"),
            ("developer_oauth_grants", "OAuth grants"),
            ("api_key_usage_logs", "API key usage logs"),
        ] {
            let n: i64 =
                sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT COUNT(*) FROM {table}")))
                    .fetch_one(&mut *conn)
                    .await
                    .unwrap_or_else(|e| panic!("count {table} (no ctx): {e}"));
            assert_eq!(
                n, 0,
                "without RLS context `{label}` ({table}) must be deny-all under FORCE"
            );
        }

        sqlx::query("RESET ROLE")
            .execute(&mut *conn)
            .await
            .expect("reset role");
    }

    // ====================================================================
    // (1) CROSS-USER ISOLATION: under user A's context, A sees only A's rows.
    // ====================================================================
    {
        let mut conn = pool.acquire().await.expect("acquire");
        sqlx::query("SELECT set_request_context($1, $2, $3)")
            .bind(Option::<Uuid>::None)
            .bind(user_a)
            .bind(false)
            .execute(&mut *conn)
            .await
            .expect("set user-A ctx");
        sqlx::query(sqlx::AssertSqlSafe(format!("SET ROLE \"{role}\"")))
            .execute(&mut *conn)
            .await
            .expect("set role");

        // Raw-SQL visibility: exactly one account / key / sandbox — all A's.
        let accounts: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM developer_accounts")
            .fetch_one(&mut *conn)
            .await
            .expect("count accounts");
        assert_eq!(
            accounts, 1,
            "user A must see only their own developer account"
        );

        let keys: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM developer_api_keys")
            .fetch_one(&mut *conn)
            .await
            .expect("count keys");
        assert_eq!(keys, 1, "user A must see only their own API key");

        let sandboxes: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM developer_sandboxes")
            .fetch_one(&mut *conn)
            .await
            .expect("count sandboxes");
        assert_eq!(sandboxes, 1, "user A must see only their own sandbox");

        // The three tables FORCEd by 00181/00182 (GH #1303): under A's context,
        // A sees exactly their own row in each.
        for (table, label) in [
            ("developer_oauth_apps", "OAuth app"),
            ("developer_oauth_grants", "OAuth grant"),
            ("api_key_usage_logs", "API key usage log"),
        ] {
            let n: i64 =
                sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT COUNT(*) FROM {table}")))
                    .fetch_one(&mut *conn)
                    .await
                    .unwrap_or_else(|e| panic!("count {table} under A: {e}"));
            assert_eq!(n, 1, "user A must see only their own {label} ({table})");
        }

        // Probe B's OAuth app / grant / usage-log row by id → invisible. This is
        // the predicate-correctness assertion the catalog-metadata check can't
        // make: a wrong join column on api_key_usage_logs (which reaches user
        // identity via developer_api_keys -> developer_accounts) would pass the
        // FORCE-presence check yet still leak B's rows here.
        for (table, id) in [
            ("developer_oauth_apps", app_b),
            ("developer_oauth_grants", grant_b),
            ("api_key_usage_logs", usage_b),
        ] {
            let cross: Option<Uuid> = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
                "SELECT id FROM {table} WHERE id = $1"
            )))
            .bind(id)
            .fetch_optional(&mut *conn)
            .await
            .unwrap_or_else(|e| panic!("probe B {table}: {e}"));
            assert!(
                cross.is_none(),
                "user A must NOT read user B's row in `{table}` by id"
            );
        }

        // Probing B's account / key by id surfaces nothing.
        let cross_account: Option<Uuid> =
            sqlx::query_scalar("SELECT id FROM developer_accounts WHERE id = $1")
                .bind(dev_b)
                .fetch_optional(&mut *conn)
                .await
                .expect("probe B account");
        assert!(
            cross_account.is_none(),
            "user A must NOT be able to read user B's developer account by id"
        );

        let cross_key: Option<Uuid> =
            sqlx::query_scalar("SELECT id FROM developer_api_keys WHERE id = $1")
                .bind(key_b)
                .fetch_optional(&mut *conn)
                .await
                .expect("probe B key");
        assert!(
            cross_key.is_none(),
            "user A must NOT be able to read user B's API key by id"
        );

        // Through the repository method the handler uses: asking for B's keys by
        // developer_id returns empty under A's context (RLS backstops the
        // handler's owner check).
        let cross_repo_keys = repo
            .list_developer_api_keys(&mut *conn, dev_b)
            .await
            .expect("list B keys under A ctx");
        assert!(
            cross_repo_keys.is_empty(),
            "list_developer_api_keys(dev_b) under user A's context must be empty — \
             the RLS backstop, not just the handler check, hides cross-user keys"
        );

        // Own keys are visible through the same repo method.
        let own_repo_keys = repo
            .list_developer_api_keys(&mut *conn, dev_a)
            .await
            .expect("list A keys under A ctx");
        assert_eq!(
            own_repo_keys.len(),
            1,
            "user A must see their own API key via the repo"
        );

        // ── WRITE path (#1594). The policies are `FOR ALL USING (...)` with no
        // explicit WITH CHECK, so Postgres reuses USING as the write check too.
        // Assert that holds — otherwise a future FOR SELECT + weak write policy
        // split would let A mutate B's rows while the read probes above stay green.
        //
        // Asymmetry to keep in mind: UPDATE/DELETE of a row hidden by USING
        // silently affects 0 rows (NOT an error); an INSERT that violates the
        // (implicit) WITH CHECK raises an error.
        let upd = sqlx::query("UPDATE developer_oauth_apps SET name = 'hijack' WHERE id = $1")
            .bind(app_b)
            .execute(&mut *conn)
            .await
            .expect("update B oauth app under A must run (RLS hides the row, not error)");
        assert_eq!(
            upd.rows_affected(),
            0,
            "user A must NOT UPDATE user B's oauth app (USING hides it → 0 rows)"
        );

        let del = sqlx::query("DELETE FROM developer_oauth_grants WHERE id = $1")
            .bind(grant_b)
            .execute(&mut *conn)
            .await
            .expect("delete B oauth grant under A must run (RLS hides the row, not error)");
        assert_eq!(
            del.rows_affected(),
            0,
            "user A must NOT DELETE user B's oauth grant (USING hides it → 0 rows)"
        );

        let del_log = sqlx::query("DELETE FROM api_key_usage_logs WHERE id = $1")
            .bind(usage_b)
            .execute(&mut *conn)
            .await
            .expect("delete B usage log under A must run (RLS hides the row, not error)");
        assert_eq!(
            del_log.rows_affected(),
            0,
            "user A must NOT DELETE user B's api_key_usage_log (USING hides it → 0 rows)"
        );

        // INSERT attributed to B violates the (implicit) WITH CHECK → error.
        let ins = sqlx::query(
            "INSERT INTO developer_oauth_grants (oauth_app_id, user_id, scopes) \
             VALUES ($1, $2, ARRAY['read'])",
        )
        .bind(app_b)
        .bind(user_b)
        .execute(&mut *conn)
        .await;
        assert!(
            ins.is_err(),
            "user A must NOT INSERT an oauth grant owned by user B (WITH CHECK denies)"
        );

        sqlx::query("RESET ROLE")
            .execute(&mut *conn)
            .await
            .expect("reset role");
    }

    // ====================================================================
    // (3a) CATALOG WRITE under super-admin context succeeds.
    // ====================================================================
    {
        let mut conn = pool.acquire().await.expect("acquire");
        sqlx::query("SELECT set_request_context($1, $2, $3)")
            .bind(Option::<Uuid>::None)
            .bind(Option::<Uuid>::None)
            .bind(true) // is_super_admin
            .execute(&mut *conn)
            .await
            .expect("set super-admin ctx");
        sqlx::query(sqlx::AssertSqlSafe(format!("SET ROLE \"{role}\"")))
            .execute(&mut *conn)
            .await
            .expect("set role");

        let admin_insert = sqlx::query(
            r#"
            INSERT INTO marketplace_integrations (slug, name, description, vendor_name)
            VALUES ('pap171-admin', 'PAP-171 Admin Integration', 'desc', 'Vendor')
            "#,
        )
        .execute(&mut *conn)
        .await;
        assert!(
            admin_insert.is_ok(),
            "platform-admin (super-admin context) catalog write must still pass under FORCE: {admin_insert:?}"
        );

        sqlx::query("RESET ROLE")
            .execute(&mut *conn)
            .await
            .expect("reset role");
    }

    // ====================================================================
    // (4) CATALOG public read + (3b) non-admin write denied, under a plain
    //     user context.
    // ====================================================================
    {
        let mut conn = pool.acquire().await.expect("acquire");
        sqlx::query("SELECT set_request_context($1, $2, $3)")
            .bind(Option::<Uuid>::None)
            .bind(user_a)
            .bind(false)
            .execute(&mut *conn)
            .await
            .expect("set plain user ctx");
        sqlx::query(sqlx::AssertSqlSafe(format!("SET ROLE \"{role}\"")))
            .execute(&mut *conn)
            .await
            .expect("set role");

        // (4) Public read: USING (TRUE) keeps the catalog readable even with no
        //     super-admin context — the admin-inserted row is visible.
        let catalog_visible: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM marketplace_integrations")
                .fetch_one(&mut *conn)
                .await
                .expect("count catalog");
        assert!(
            catalog_visible >= 1,
            "the catalog public-read policy must keep marketplace_integrations readable under FORCE"
        );

        // (3b) Plain user context: catalog write is denied by the policy WITH CHECK.
        let user_insert = sqlx::query(
            r#"
            INSERT INTO marketplace_integrations (slug, name, description, vendor_name)
            VALUES ('pap171-user', 'PAP-171 User Integration', 'desc', 'Vendor')
            "#,
        )
        .execute(&mut *conn)
        .await;
        assert!(
            user_insert.is_err(),
            "a non-super-admin catalog write must be denied by the FORCE-RLS write policy"
        );

        sqlx::query("RESET ROLE")
            .execute(&mut *conn)
            .await
            .expect("reset role");
    }

    // --- Cleanup the test role. ---
    set_ctx(&pool, None, true).await;
    for stmt in [
        format!("REVOKE ALL ON developer_accounts FROM \"{role}\""),
        format!("REVOKE ALL ON developer_api_keys FROM \"{role}\""),
        format!("REVOKE ALL ON developer_sandboxes FROM \"{role}\""),
        format!("REVOKE ALL ON developer_oauth_apps FROM \"{role}\""),
        format!("REVOKE ALL ON developer_oauth_grants FROM \"{role}\""),
        format!("REVOKE ALL ON api_key_usage_logs FROM \"{role}\""),
        format!("REVOKE ALL ON marketplace_integrations FROM \"{role}\""),
        format!("DROP OWNED BY \"{role}\""),
        format!("DROP ROLE IF EXISTS \"{role}\""),
    ] {
        sqlx::query(sqlx::AssertSqlSafe(stmt))
            .execute(&pool)
            .await
            .ok();
    }
}
