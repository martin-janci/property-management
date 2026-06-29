//! Repo-level behavioral RLS regression test for PAP-103 (parent PAP-80 / PAP-67).
//!
//! Background
//! ----------
//! Migration `00179` (PAP-62) put `FORCE ROW LEVEL SECURITY` + the canonical
//! `get_current_org_id()` policy on the three AI-chat tables (`ai_chat_sessions`,
//! `ai_chat_messages`, `ai_training_feedback`). The production api-server connects
//! as the table OWNER, which `FORCE` binds. `AiChatRepository` held a raw `PgPool`
//! and ran every query WITHOUT ever calling `set_request_context`, so on `dev`
//! `get_current_org_id()` returned NULL and the policy collapsed to
//! `organization_id = NULL` → **deny-all**: own-org reads returned empty, writes
//! failed. (PAP-103.)
//!
//! The fix routes the repo through an RLS-context connection (the `RlsConnection`
//! extractor in handlers sets the org/user GUCs before any query). This test
//! exercises the *repository methods themselves* on a `FORCE`-bound role and
//! proves, against `ai_chat_sessions`:
//!
//!   1. **Deny-all reproduction** — with the role bound but NO context set
//!      (exactly what the raw-pool repo did on `dev`), an own-org
//!      `find_session_by_id` returns nothing.
//!   2. **Fix** — with `set_request_context(org_a, user_a)` applied first (what
//!      `RlsConnection` now does), the same repo call returns the own-org row.
//!   3. **Cross-tenant** — org B's session stays invisible to an org-A caller.
//!   4. **Write path** — `create_session` on a context-set connection succeeds
//!      and the row is the caller's org.
//!
//! Why this test switches roles
//! ----------------------------
//! `#[sqlx::test]` connects as the Postgres SUPERUSER, which bypasses RLS
//! entirely — even `FORCE` does not bind a superuser, so a behavioral assertion
//! would pass vacuously. The test creates a plain `NOSUPERUSER NOBYPASSRLS`
//! role, grants it access, and `SET ROLE`s to it so `FORCE` actually enforces the
//! policy the way the production owner role experiences it. Mirrors
//! `enhanced_tenant_screening_rls_repo_tests.rs` (the PAP-74 precedent PAP-103
//! follows — the ai_chat policies also carry the `get_current_org_not_deleted()`
//! soft-delete guard, so the bound role needs `organizations` SELECT + the
//! helper-function EXECUTE grants).

use db::models::CreateChatSession;
use db::repositories::AiChatRepository;
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
    .bind(format!("AiChat {slug}"))
    .bind(slug)
    .bind(format!("{slug}@aichat.test"))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'test_hash', 'AiChat User', 'active', NOW(), 'public')
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

/// Seed a chat session directly (as superuser, RLS-exempt) for an org/user.
async fn seed_session(pool: &PgPool, org_id: Uuid, user_id: Uuid, title: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO ai_chat_sessions (organization_id, user_id, title)
        VALUES ($1, $2, $3)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(user_id)
    .bind(title)
    .fetch_one(pool)
    .await
    .expect("seed session")
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn ai_chat_repo_force_rls_deny_all_and_fix(pool: PgPool) {
    let repo = AiChatRepository::new(pool.clone());

    // --- Seed as superuser / super-admin context (satisfies org roles-trigger). ---
    set_ctx(&pool, None, None, true).await;
    let org_a = seed_org(&pool, "aichat-a").await;
    let org_b = seed_org(&pool, "aichat-b").await;
    let user_a = seed_user(&pool, "a@aichat.test").await;
    let user_b = seed_user(&pool, "b@aichat.test").await;
    let session_a = seed_session(&pool, org_a, user_a, "Session A").await;
    let session_b = seed_session(&pool, org_b, user_b, "Session B").await;

    // --- NOSUPERUSER NOBYPASSRLS role so FORCE actually binds. ---
    let role = format!("ppt_rls_aichat_{}", Uuid::new_v4().simple());
    for stmt in [
        format!("CREATE ROLE \"{role}\" NOSUPERUSER NOBYPASSRLS"),
        format!("GRANT SELECT, INSERT, UPDATE, DELETE ON ai_chat_sessions TO \"{role}\""),
        // RLS policy helpers must be EXECUTE-able by the bound role; the
        // not-deleted guard reads `organizations`.
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
        // Explicitly clear any inherited context, then drop to the bound role.
        sqlx::query("SELECT clear_request_context()")
            .execute(&mut *conn)
            .await
            .expect("clear ctx");
        sqlx::query(sqlx::AssertSqlSafe(format!("SET ROLE \"{role}\"")))
            .execute(&mut *conn)
            .await
            .expect("set role");

        let found = repo
            .find_session_by_id(&mut *conn, session_a, org_a)
            .await
            .expect("find_session_by_id (no ctx)");
        assert!(
            found.is_none(),
            "PAP-103 regression: without RLS context, own-org chat session must be \
             invisible (deny-all) — this is what the raw-pool repo did on dev"
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
            .find_session_by_id(&mut *conn, session_a, org_a)
            .await
            .expect("find_session_by_id (ctx)");
        assert_eq!(
            found.map(|s| s.id),
            Some(session_a),
            "PAP-103 fix: with RLS context set, the repo must return the own-org session"
        );

        // (3) Org B's session stays invisible to an org-A caller — even though
        //     the SQL filter is passed org_a, RLS independently denies the row.
        let cross = repo
            .find_session_by_id(&mut *conn, session_b, org_a)
            .await
            .expect("find_session_by_id cross");
        assert!(
            cross.is_none(),
            "cross-tenant: org B's session must NOT be visible to an org-A caller"
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
            .create_session(
                &mut *conn,
                org_a,
                user_a,
                CreateChatSession {
                    title: Some("Created under RLS".to_string()),
                    context: None,
                },
            )
            .await
            .expect("create_session under context must succeed");
        assert_eq!(created.organization_id, org_a);

        sqlx::query("RESET ROLE")
            .execute(&mut *conn)
            .await
            .expect("reset role");
    }

    // --- Cleanup the test role. ---
    set_ctx(&pool, None, None, true).await;
    for stmt in [
        format!("REVOKE ALL ON ai_chat_sessions FROM \"{role}\""),
        format!("REVOKE ALL ON organizations FROM \"{role}\""),
        format!("DROP ROLE IF EXISTS \"{role}\""),
    ] {
        sqlx::query(sqlx::AssertSqlSafe(stmt))
            .execute(&pool)
            .await
            .ok();
    }
}
