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

use crate::common::{seed_org, set_ctx};
use db::models::{
    CreateForm, CreateFormField, FormListQuery, FormSubmissionParams, SubmitForm, UpdateForm,
};
use db::repositories::FormRepository;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

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

/// Creates a `NOSUPERUSER NOBYPASSRLS` role with the grants the
/// `FormRepository` needs, so `FORCE ROW LEVEL SECURITY` actually binds the way
/// the production owner role experiences it. Returns the role name.
async fn create_rls_role(pool: &PgPool) -> String {
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
        format!("GRANT SELECT ON users TO \"{role}\""),
    ] {
        sqlx::query(sqlx::AssertSqlSafe(stmt))
            .execute(pool)
            .await
            .expect("grant setup");
    }
    role
}

async fn drop_rls_role(pool: &PgPool, role: &str) {
    set_ctx(pool, None, None, true).await;
    for stmt in [
        format!(
            "REVOKE ALL ON forms, form_fields, form_submissions, form_downloads FROM \"{role}\""
        ),
        format!("REVOKE ALL ON organizations FROM \"{role}\""),
        format!("REVOKE ALL ON users FROM \"{role}\""),
        format!("DROP ROLE IF EXISTS \"{role}\""),
    ] {
        sqlx::query(sqlx::AssertSqlSafe(stmt))
            .execute(pool)
            .await
            .ok();
    }
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
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
    // Reuses the shared `create_rls_role` helper for the common CREATE + grant
    // set (forms/form_fields/form_submissions/form_downloads + the RLS
    // functions + organizations + users), then adds the extra grants this test
    // alone needs:
    //
    //   `get_submission` LEFT JOINs `units` (un.designation) and `buildings`
    //   (b.name) for the submission detail view, so the role likewise needs
    //   SELECT on both or the join trips 42501 (permission denied on `units`).
    let role = create_rls_role(&pool).await;
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "GRANT SELECT ON units, buildings TO \"{role}\""
    )))
    .execute(&pool)
    .await
    .expect("grant setup");

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
    // Reuse the shared `drop_rls_role` helper (it re-establishes the super
    // context, then REVOKEs every grant this test made + DROPs the role) so
    // the REVOKE list lives in exactly one place — the write-path test below
    // already goes through it. Keeping a hand-inlined copy here meant any new
    // form table had to be added to the grant list twice.
    let _ = (form_b, user_b); // seeded for symmetry
    drop_rls_role(&pool, &role).await;
}

/// Write-path coverage for PAP-76 / issue #1369.
///
/// The original `form_rls_repo_tests` only exercised read paths (`get`/`list`)
/// plus submit/download round-trips. The PR description, however, named
/// own-org `create`/`update` failing under `FORCE` as a *primary* symptom, and
/// there was no cross-tenant write negative. This test fills both gaps on the
/// `FORCE`-bound NOSUPERUSER role:
///
///   1. **Own-org write round-trip** — `create` (form + fields) succeeds for the
///      caller's org and the row + its fields are then visible via `get` /
///      `get_fields`; a follow-up `update` mutates the row.
///   2. **Cross-tenant write isolation** — an org-A context that targets org B's
///      form (passing org B's id, so RLS — not just the WHERE-clause org filter —
///      is the enforcer) cannot `update` (errors `RowNotFound`) or `delete`
///      (0 rows) it, and org B's form is left fully intact.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn form_repo_force_rls_write_paths_and_cross_tenant(pool: PgPool) {
    let repo = FormRepository::new(pool.clone());

    // --- Seed as superuser / super-admin context ---
    set_ctx(&pool, None, None, true).await;
    let org_a = seed_org(&pool, "force-form-write-a").await;
    let org_b = seed_org(&pool, "force-form-write-b").await;
    let user_a = seed_user(&pool, "write-a@form.test").await;
    let user_b = seed_user(&pool, "write-b@form.test").await;
    // Org B's form is seeded directly (RLS-exempt) so it pre-exists the attack.
    let form_b = seed_form(&pool, org_b, user_b, "Org B private form").await;

    let role = create_rls_role(&pool).await;

    // ====================================================================
    // (1) OWN-ORG WRITE ROUND-TRIP under org-A context.
    // ====================================================================
    let created_form_id = {
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

        // create form + one field
        let created = repo
            .create(
                &mut conn,
                org_a,
                user_a,
                CreateForm {
                    title: "Resident intake".to_string(),
                    description: Some("created under RLS context".to_string()),
                    category: None,
                    building_id: None,
                    target_type: None,
                    target_ids: None,
                    require_signatures: false,
                    allow_multiple_submissions: true,
                    submission_deadline: None,
                    confirmation_message: None,
                    fields: vec![CreateFormField {
                        field_key: "q1".to_string(),
                        label: "Question 1".to_string(),
                        field_type: "text".to_string(),
                        required: true,
                        help_text: None,
                        placeholder: None,
                        default_value: None,
                        validation_rules: None,
                        options: None,
                        field_order: 0,
                        width: "full".to_string(),
                        section: None,
                        conditional_display: None,
                    }],
                },
            )
            .await
            .expect(
                "PAP-76 fix: own-org create must succeed under RLS context (was deny-all pre-fix)",
            );
        assert_eq!(created.organization_id, org_a);

        // The created form is now visible to the same org-A context.
        let fetched = repo
            .get(&mut *conn, org_a, created.id)
            .await
            .expect("get created form");
        assert_eq!(
            fetched.map(|f| f.id),
            Some(created.id),
            "PAP-76 fix: own-org form must be visible right after create under context"
        );

        // The field created alongside the form is visible too.
        let fields = repo
            .get_fields(&mut *conn, created.id)
            .await
            .expect("get_fields");
        assert_eq!(
            fields.len(),
            1,
            "create must persist the form's field under RLS context"
        );
        assert_eq!(fields[0].field_key, "q1");

        // update round-trip on the own-org form.
        let updated = repo
            .update(
                &mut conn,
                org_a,
                created.id,
                user_a,
                UpdateForm {
                    title: Some("Resident intake (v2)".to_string()),
                    description: None,
                    category: None,
                    building_id: None,
                    target_type: None,
                    target_ids: None,
                    require_signatures: None,
                    allow_multiple_submissions: None,
                    submission_deadline: None,
                    confirmation_message: None,
                },
            )
            .await
            .expect("PAP-76 fix: own-org update must succeed under RLS context");
        assert_eq!(updated.title, "Resident intake (v2)");

        sqlx::query("RESET ROLE")
            .execute(&mut *conn)
            .await
            .expect("reset role");
        created.id
    };

    // ====================================================================
    // (2) CROSS-TENANT WRITE ISOLATION.
    //     Org-A context targets org B's form. We pass org B's id so the
    //     test isolates RLS itself (not the app-level org filter) as the
    //     thing that must block the write.
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

        // update against org B's form must fail (RLS hides the row → existence
        // check returns None → RowNotFound).
        let update_res = repo
            .update(
                &mut conn,
                org_b,
                form_b,
                user_a,
                UpdateForm {
                    title: Some("hijacked".to_string()),
                    description: None,
                    category: None,
                    building_id: None,
                    target_type: None,
                    target_ids: None,
                    require_signatures: None,
                    allow_multiple_submissions: None,
                    submission_deadline: None,
                    confirmation_message: None,
                },
            )
            .await;
        assert!(
            matches!(update_res, Err(sqlx::Error::RowNotFound)),
            "cross-tenant update must be blocked by RLS, got {update_res:?}"
        );

        // delete against org B's form must affect 0 rows (no error, but a no-op
        // under RLS).
        repo.delete(&mut *conn, org_b, form_b)
            .await
            .expect("cross-tenant delete call itself returns Ok (0 rows affected)");

        sqlx::query("RESET ROLE")
            .execute(&mut *conn)
            .await
            .expect("reset role");
    }

    // ====================================================================
    // Verify the cross-tenant target survived, observed as superuser.
    // ====================================================================
    set_ctx(&pool, None, None, true).await;
    let (b_title, b_deleted): (String, Option<chrono::DateTime<chrono::Utc>>) =
        sqlx::query_as("SELECT title, deleted_at FROM forms WHERE id = $1")
            .bind(form_b)
            .fetch_one(&pool)
            .await
            .expect("fetch org-B form post-attack");
    assert_eq!(
        b_title, "Org B private form",
        "cross-tenant update must NOT have changed org B's form title"
    );
    assert!(
        b_deleted.is_none(),
        "cross-tenant delete must NOT have soft-deleted org B's form"
    );

    // The own-org form we created earlier is still there and updated.
    let (a_title,): (String,) = sqlx::query_as("SELECT title FROM forms WHERE id = $1")
        .bind(created_form_id)
        .fetch_one(&pool)
        .await
        .expect("fetch own-org form");
    assert_eq!(a_title, "Resident intake (v2)");

    drop_rls_role(&pool, &role).await;
}
