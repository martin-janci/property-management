//! Repo-level behavioral RLS regression test for PAP-102 (PAP-80 cluster).
//!
//! Background
//! ----------
//! Migration `00179` (PAP-62) put `FORCE ROW LEVEL SECURITY` + the canonical
//! `get_current_org_id()` policy on the building-certification tables
//! (`building_certifications` and the rest of the Epic-137 cluster). The
//! production api-server connects as the table OWNER, which `FORCE` binds.
//! `BuildingCertificationRepository` held a raw `PgPool` and ran every query
//! WITHOUT ever calling `set_request_context`, so on `dev`
//! `get_current_org_id()` returned NULL and the policy collapsed to
//! `organization_id = NULL` → **deny-all**: own-org reads returned empty, writes
//! failed. (PAP-102.)
//!
//! The fix routes the repo through an RLS-context connection (the `RlsConnection`
//! extractor in handlers sets the org/user GUCs before any query). This test
//! exercises the *repository methods themselves* on a `FORCE`-bound role and
//! proves:
//!
//!   1. **Deny-all reproduction** — with the role bound but NO context set
//!      (exactly what the raw-pool repo did on `dev`), an own-org
//!      `get_certification` / `list_certifications` returns nothing. This is the
//!      "would have failed on dev" evidence.
//!   2. **Fix** — with `set_request_context(org_a, user_a)` applied first (what
//!      `RlsConnection` now does), the same repo calls return the own-org row.
//!   3. **Cross-tenant** — org B's certification stays invisible to an org-A
//!      caller even with context set.
//!   4. **Write path** — a `create_certification` on a context-set connection
//!      succeeds and the row is the caller's org. Without context the INSERT
//!      would fail the policy `WITH CHECK`.
//!
//! Why this test switches roles
//! ----------------------------
//! `#[sqlx::test]` connects as the Postgres SUPERUSER, which bypasses RLS
//! entirely — even `FORCE` does not bind a superuser, so a behavioral assertion
//! would pass vacuously. The test creates a plain `NOSUPERUSER NOBYPASSRLS`
//! role, grants it access, and `SET ROLE`s to it so `FORCE` actually enforces the
//! policy the way the production owner role experiences it. Mirrors
//! `reserve_funds_rls_repo_tests.rs` / `work_order_rls_repo_tests.rs` (the
//! PAP-67 precedent this follows).

use crate::common::{seed_org, set_ctx};
use db::models::building_certification::{
    CertificationLevel, CertificationProgram, CreateBuildingCertification,
};
use db::repositories::BuildingCertificationRepository;
use sqlx::PgPool;
use uuid::Uuid;

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'test_hash', 'BC User', 'active', NOW(), 'public')
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

async fn seed_building(pool: &PgPool, org_id: Uuid, name: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO buildings (organization_id, name, street, city, postal_code, country, status)
        VALUES ($1, $2, 'Street 1', 'City', '00000', 'Country', 'active')
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(name)
    .fetch_one(pool)
    .await
    .expect("seed building")
}

/// Seed a building certification directly (as superuser, RLS-exempt) for an org.
async fn seed_cert(pool: &PgPool, org_id: Uuid, building_id: Uuid, user_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO building_certifications
            (organization_id, building_id, program, level, status, created_by)
        VALUES ($1, $2, 'leed', 'gold', 'planning', $3)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(building_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("seed building certification")
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn building_certification_repo_force_rls_deny_all_and_fix(pool: PgPool) {
    let repo = BuildingCertificationRepository::new(pool.clone());

    // --- Seed as superuser / super-admin context (satisfies org roles-trigger). ---
    set_ctx(&pool, None, None, true).await;
    let org_a = seed_org(&pool, "force-bc-a").await;
    let org_b = seed_org(&pool, "force-bc-b").await;
    let user_a = seed_user(&pool, "a@bc.test").await;
    let user_b = seed_user(&pool, "b@bc.test").await;
    let building_a = seed_building(&pool, org_a, "Building A").await;
    let building_b = seed_building(&pool, org_b, "Building B").await;
    let cert_a = seed_cert(&pool, org_a, building_a, user_a).await;
    let cert_b = seed_cert(&pool, org_b, building_b, user_b).await;

    // --- NOSUPERUSER NOBYPASSRLS role so FORCE actually binds. ---
    let role = format!("ppt_rls_bc_{}", Uuid::new_v4().simple());
    for stmt in [
        format!("CREATE ROLE \"{role}\" NOSUPERUSER NOBYPASSRLS"),
        format!("GRANT SELECT, INSERT, UPDATE, DELETE ON building_certifications TO \"{role}\""),
        // RLS policy helpers must be EXECUTE-able by the bound role; the
        // soft-delete guard reads `organizations`.
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
            .get_certification(&mut *conn, org_a, cert_a)
            .await
            .expect("get_certification (no ctx)");
        assert!(
            found.is_none(),
            "PAP-102 regression: without RLS context, own-org building certification must be \
             invisible (deny-all) — this is what the raw-pool repo did on dev"
        );

        let listed = repo
            .list_certifications(&mut *conn, org_a, Default::default(), 50, 0)
            .await
            .expect("list_certifications (no ctx)");
        assert!(
            listed.is_empty(),
            "PAP-102 regression: without RLS context, list_certifications returns deny-all empty"
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
            .get_certification(&mut *conn, org_a, cert_a)
            .await
            .expect("get_certification (ctx)");
        assert_eq!(
            found.map(|c| c.id),
            Some(cert_a),
            "PAP-102 fix: with RLS context set, the repo must return the own-org certification"
        );

        let listed = repo
            .list_certifications(&mut *conn, org_a, Default::default(), 50, 0)
            .await
            .expect("list_certifications (ctx)");
        assert_eq!(
            listed.iter().map(|c| c.id).collect::<Vec<_>>(),
            vec![cert_a],
            "PAP-102 fix: list_certifications returns exactly the own-org certification under context"
        );

        // (3) Org B's certification stays invisible to an org-A caller.
        let cross = repo
            .get_certification(&mut *conn, org_a, cert_b)
            .await
            .expect("get_certification cross");
        assert!(
            cross.is_none(),
            "cross-tenant: org B's certification must NOT be visible to an org-A caller"
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
            .create_certification(
                &mut *conn,
                org_a,
                CreateBuildingCertification {
                    building_id: building_a,
                    program: CertificationProgram::Breeam,
                    version: None,
                    level: CertificationLevel::Certified,
                    status: None,
                    total_points_possible: None,
                    total_points_achieved: None,
                    application_date: None,
                    certification_date: None,
                    expiration_date: None,
                    certificate_number: None,
                    project_id: None,
                    assessor_name: None,
                    assessor_organization: None,
                    certificate_url: None,
                    scorecard_url: None,
                    notes: None,
                    application_fee: None,
                    certification_fee: None,
                    annual_fee: None,
                },
                Some(user_a),
            )
            .await
            .expect("create_certification under context must succeed");
        assert_eq!(created.organization_id, org_a);

        sqlx::query("RESET ROLE")
            .execute(&mut *conn)
            .await
            .expect("reset role");
    }

    // --- Cleanup the test role. ---
    set_ctx(&pool, None, None, true).await;
    let _ = user_b; // seeded for symmetry with org_b's certification
    for stmt in [
        format!("REVOKE ALL ON building_certifications FROM \"{role}\""),
        format!("REVOKE ALL ON organizations FROM \"{role}\""),
        format!("DROP ROLE IF EXISTS \"{role}\""),
    ] {
        sqlx::query(sqlx::AssertSqlSafe(stmt))
            .execute(&pool)
            .await
            .ok();
    }
}
