//! Regression guard for `FaultRepository::find_by_id_for_org_rls` (#3307).
//!
//! The `update_fault` / `triage_fault` / `update_status` handlers used to load
//! a fault with the id-only `find_by_id_rls` and then reject a foreign org with
//! a hand-written `f.organization_id == tenant_id` match arm. Because the
//! `faults` RLS policy is not `FORCE`d, that Rust guard was the *only* thing
//! stopping a cross-org IDOR — a later refactor dropping the arm would silently
//! re-open it, and the id-only reader fetched the foreign row before discarding
//! it.
//!
//! The org filter now lives in SQL (`WHERE id = $1 AND organization_id = $2`).
//! This test pins that contract directly on the repo primitive: a cross-org id
//! returns `None` even with no Rust guard in sight, so the isolation can no
//! longer be lost by editing a handler.

use crate::common::seed_org;
use db::repositories::FaultRepository;
use sqlx::PgPool;
use uuid::Uuid;

async fn seed_building(pool: &PgPool, org_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO buildings (organization_id, street, city, postal_code)
        VALUES ($1, 'Test Street 1', 'Bratislava', '81101')
        RETURNING id
        "#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed building")
}

async fn seed_reporter(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'test_hash', 'IDOR Reporter', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed reporter")
}

async fn seed_fault(pool: &PgPool, org_id: Uuid, building_id: Uuid, reporter_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO faults (organization_id, building_id, reporter_id, title, description, status)
        VALUES ($1, $2, $3, 'Broken thing', 'It is broken', 'new'::fault_status)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(building_id)
    .bind(reporter_id)
    .fetch_one(pool)
    .await
    .expect("seed fault")
}

/// Own-org lookup returns the fault; cross-org lookup collapses to `None` in
/// SQL — the exact 404 the mutation handlers now rely on, with no Rust guard.
#[sqlx::test(migrations = "./migrations")]
async fn find_by_id_for_org_rls_enforces_org_in_sql(pool: PgPool) {
    let org_a = seed_org(&pool, "idor-a").await;
    let org_b = seed_org(&pool, "idor-b").await;
    let bld_a = seed_building(&pool, org_a).await;
    let rep_a = seed_reporter(&pool, "idor-a@reporters.test").await;
    let fault_a = seed_fault(&pool, org_a, bld_a, rep_a).await;

    let repo = FaultRepository::new(pool.clone());
    let mut conn = pool.acquire().await.expect("acquire");

    // Own org: the fault is visible.
    let own = repo
        .find_by_id_for_org_rls(&mut *conn, fault_a, org_a)
        .await
        .expect("find_by_id_for_org_rls own-org");
    assert!(
        own.is_some(),
        "own-org fault must be readable via find_by_id_for_org_rls"
    );
    assert_eq!(own.unwrap().organization_id, org_a);

    // Foreign org enumerating org A's fault id: SQL org filter → None (→ 404),
    // independent of any handler-side `organization_id == tenant_id` arm.
    let cross = repo
        .find_by_id_for_org_rls(&mut *conn, fault_a, org_b)
        .await
        .expect("find_by_id_for_org_rls cross-org");
    assert!(
        cross.is_none(),
        "#3307: a cross-org id must collapse to None in SQL, not rely on a \
         hand-written Rust guard in the mutation handlers"
    );
}
