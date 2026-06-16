//! Repo-level behavioral RLS regression test for PAP-76 (PAP-67 cluster).
//!
//! Background
//! ----------
//! Migration `00179` put `FORCE ROW LEVEL SECURITY` + the canonical
//! `get_current_org_id()` policy on the form tables (`forms`, `form_fields`,
//! `form_submissions`, `form_downloads`). Under `FORCE` the api-server's owner
//! connection is no longer exempt, so a query issued on a connection WITHOUT
//! `app.current_org_id` set collapses to deny-all — own-org reads return
//! empty, writes fail.
//!
//! PAP-76 converted `FormRepository` to the stateless executor pattern
//! (handlers pass `&mut **rls.conn()`), so every query runs under the
//! caller's RLS context. This test exercises the repository methods
//! themselves on a `FORCE`-bound NOSUPERUSER role and proves:
//!
//!   1. **Deny-all** — with the role bound but NO context set (exactly what
//!      the old raw-pool repo did), an own-org `FormRepository::get` /
//!      `list` returns nothing.
//!   2. **Fix** — with `set_request_context(org, user)` applied first
//!      (what `RlsConnection` does), the same calls surface the own-org row.
//!   3. **Cross-tenant** — org B's form stays invisible to an org-A context.
//!
//! Why this test switches roles
//! ----------------------------
//! `#[sqlx::test]` connects as the Postgres SUPERUSER, which bypasses RLS
//! entirely — even `FORCE` does not bind a superuser. The test creates a
//! plain `NOSUPERUSER NOBYPASSRLS` role, grants it access, and `SET ROLE`s
//! to it so `FORCE` actually enforces the policy the way the production
//! owner role experiences it. Mirrors `budget_rls_repo_tests.rs`.

use db::models::{FormListQuery, FormSubmissionParams, SubmitForm};
use db::repositories::FormRepository;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

async fn set_ctx(pool: &PgPool, org_id: Option<Uuid>, user_id: Option<Uuid>, is_super: bool) {
    sqlx::query("SELECT set_request_context($1, $2, $3)")
        .bind(org_id)
        .bind(user_id)
        .bind(is_super)
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
    .bind(format!("Form {slug}"))
    .bind(slug)
    .bind(format!("{slug}@form.test"))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'test_hash', 'Form User', 'active', NOW(), 'public')
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

/// Insert a form directly as the (RLS-exempt) superuser for an org.
async fn seed_form_with_status(
    pool: &PgPool,
    org_id: Uuid,
    user_id: Uuid,
    title: &str,
    status: &str,
) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO forms (
            organization_id, title, status, target_type, target_ids,
            require_signatures, allow_multiple_submissions, created_by
        )
        VALUES ($1, $2, $4::form_status, 'all', '[]'::jsonb, false, true, $3)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(title)
    .bind(user_id)
    .bind(status)
    .fetch_one(pool)
    .await
    .expect("seed form")
}

async fn seed_form(pool: &PgPool, org_id: Uuid, user_id: Uuid, title: &str) -> Uuid {
    seed_form_with_status(pool, org_id, user_id, title, "draft").await
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn form_repo_force_rls_deny_all_and_fix(pool: PgPool) {
    let repo = FormRepository::new(pool.clone());

    // --- Seed as superuser / super-admin context ---
    set_ctx(&pool, None, None, true).await;
    let org_a = seed_org(&pool, "force-form-a").await;
    let org_b = seed_org(&pool, "force-form-b").await;
    let user_a = seed_user(&pool, "a@form.test").await;
    let user_b = seed_user(&pool, "b@form.test").await;
    let form_a = seed_form(&pool, org_a, user_a, "Form A").await;
    let form_b = seed_form(&pool, org_b, user_b, "Form B").await;
    let published_form_a =
        seed_form_with_status(&pool, org_a, user_a, "Published Form A", "published").await;

    // --- NOSUPERUSER NOBYPASSRLS role so FORCE actually binds ---
    let role = format!("ppt_rls_form_{}", Uuid::new_v4().simple());
    for stmt in [
        format!("CREATE ROLE \"{role}\" NOSUPERUSER NOBYPASSRLS"),
        format!(
            "GRANT SELECT, INSERT, UPDATE, DELETE ON \
             forms, form_fields, form_submissions, form_downloads TO \"{role}\""
        ),
        format!(
            "GRANT EXECUTE ON FUNCTION get_current_org_id(), is_super_admin(), \
             get_current_org_not_deleted() TO \"{role}\""
        ),
        format!("GRANT SELECT ON organizations TO \"{role}\""),
        // `FormRepository::list` LEFT JOINs `users` to surface `created_by_name`,
        // so the NOSUPERUSER RLS role needs SELECT on `users` or the join trips
        // Postgres 42501 (permission denied on `users`).
        format!("GRANT SELECT ON users TO \"{role}\""),
        // `get_submission` LEFT JOINs `units` (un.designation) and `buildings`
        // (b.name) for the submission detail view, so the role likewise needs
        // SELECT on both or the join trips 42501 (permission denied on `units`).
        format!("GRANT SELECT ON units, buildings TO \"{role}\""),
    ] {
        sqlx::query(sqlx::AssertSqlSafe(stmt))
            .execute(&pool)
            .await
            .expect("grant setup");
    }

    // ====================================================================
    // (1) DENY-ALL: role bound, NO context set.
    //     Own-org reads must return nothing.
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
            .get(&mut *conn, org_a, form_a)
            .await
            .expect("get (no ctx)");
        assert!(
            found.is_none(),
            "PAP-76 regression: without RLS context, own-org form must be invisible (deny-all)"
        );

        let (listed, total) = repo
            .list(&mut conn, org_a, FormListQuery::default())
            .await
            .expect("list (no ctx)");
        assert!(
            listed.is_empty() && total == 0,
            "PAP-76 regression: without RLS context, list must return deny-all empty"
        );

        sqlx::query("RESET ROLE")
            .execute(&mut *conn)
            .await
            .expect("reset role");
    }

    // ====================================================================
    // (2) FIX + (3) cross-tenant: set context, drop to role, query repo.
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

        // (2) Own-org form IS now visible — the fix.
        let found = repo
            .get(&mut *conn, org_a, form_a)
            .await
            .expect("get (ctx)");
        assert_eq!(
            found.map(|f| f.id),
            Some(form_a),
            "PAP-76 fix: with RLS context set, the repo must return the own-org form"
        );

        let (listed, total) = repo
            .list(&mut conn, org_a, FormListQuery::default())
            .await
            .expect("list (ctx)");
        assert!(
            listed.iter().any(|f| f.id == form_a),
            "PAP-76 fix: list returns own-org form under context"
        );
        assert!(total > 0, "total must be non-zero when context is set");

        // (3) Org B's form stays invisible to an org-A caller.
        let cross = repo
            .get(&mut *conn, org_a, form_b)
            .await
            .expect("cross-tenant get");
        assert!(
            cross.is_none(),
            "cross-tenant: org B's form must NOT be visible to an org-A caller"
        );

        let submission = repo
            .submit(
                &mut *conn,
                FormSubmissionParams {
                    org_id: org_a,
                    form_id: published_form_a,
                    user_id: user_a,
                    building_id: None,
                    unit_id: None,
                    data: SubmitForm {
                        data: json!({ "q1": "answer" }),
                        attachments: None,
                        signature_data: None,
                    },
                    ip_address: None,
                    user_agent: None,
                },
            )
            .await
            .expect("submit (ctx)");
        let fetched_submission = repo
            .get_submission(&mut *conn, org_a, submission.id)
            .await
            .expect("get submission (ctx)");
        assert!(
            fetched_submission.is_some(),
            "PAP-76 fix: submit/get_submission round-trip must work under RLS context"
        );

        repo.record_download(&mut *conn, org_a, published_form_a, user_a, None, None)
            .await
            .expect("record download (ctx)");
        let downloads = repo
            .get_download_count(&mut *conn, published_form_a)
            .await
            .expect("download count (ctx)");
        assert_eq!(
            downloads, 1,
            "PAP-76 fix: record_download/get_download_count round-trip must work under RLS context"
        );

        sqlx::query("RESET ROLE")
            .execute(&mut *conn)
            .await
            .expect("reset role");
    }

    // --- Cleanup ---
    set_ctx(&pool, None, None, true).await;
    let _ = (form_b, user_b); // seeded for symmetry
    for stmt in [
        format!(
            "REVOKE ALL ON forms, form_fields, form_submissions, form_downloads FROM \"{role}\""
        ),
        format!("REVOKE ALL ON organizations FROM \"{role}\""),
        format!("REVOKE ALL ON users FROM \"{role}\""),
        format!("DROP ROLE IF EXISTS \"{role}\""),
    ] {
        sqlx::query(sqlx::AssertSqlSafe(stmt))
            .execute(&pool)
            .await
            .ok();
    }
}
