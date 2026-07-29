//! Regression guard for the unified fault-by-status KPI definition
//! (2026-05-28 pm-data decision, `data-fault-kpi-unification-2026-07-23`).
//!
//! Background
//! ----------
//! Two code paths reported per-status fault counts from their own inline
//! `GROUP BY status` query:
//!
//! * the owner/portfolio statistics — `FaultRepository::get_statistics().by_status`
//!   (organisation-scoped, optionally building-scoped), and
//! * the platform-admin support-data diagnostics —
//!   `PlatformAdminRepository::get_support_data().fault_by_status` (global).
//!
//! Because the two SQL fragments were maintained independently, a change to one
//! (a different `WHERE`, a status renamed, a filter added) could silently make
//! the two dashboards disagree. Both now derive from the single
//! `db::repositories::fault::fault_counts_by_status` definition. This test pins
//! the contract: the global support-data counts must equal the sum of the
//! per-organisation statistics counts, status by status.

use std::collections::BTreeMap;

use crate::common::seed_org;
use db::repositories::{FaultRepository, PlatformAdminRepository};
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
        VALUES ($1, 'test_hash', 'KPI Reporter', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed reporter")
}

/// Insert `n` faults with the given `status` for one org/building/reporter.
async fn seed_faults(
    pool: &PgPool,
    org_id: Uuid,
    building_id: Uuid,
    reporter_id: Uuid,
    status: &str,
    n: i64,
) {
    for _ in 0..n {
        sqlx::query(
            r#"
            INSERT INTO faults (organization_id, building_id, reporter_id, title, description, status)
            VALUES ($1, $2, $3, 'Broken thing', 'It is broken', $4::fault_status)
            "#,
        )
        .bind(org_id)
        .bind(building_id)
        .bind(reporter_id)
        .bind(status)
        .execute(pool)
        .await
        .expect("seed fault");
    }
}

fn counts_map<I>(rows: I) -> BTreeMap<String, i64>
where
    I: IntoIterator<Item = (String, i64)>,
{
    rows.into_iter().collect()
}

/// The platform-admin (global) fault-by-status counts must exactly equal the
/// sum of the per-organisation owner/portfolio statistics counts — i.e. both
/// dashboards read from one shared definition and can never disagree.
#[sqlx::test(migrations = "./migrations")]
async fn support_data_and_owner_stats_agree_on_fault_status_counts(pool: PgPool) {
    let org_a = seed_org(&pool, "kpi-a").await;
    let org_b = seed_org(&pool, "kpi-b").await;
    let bld_a = seed_building(&pool, org_a).await;
    let bld_b = seed_building(&pool, org_b).await;
    let rep_a = seed_reporter(&pool, "kpi-a@reporters.test").await;
    let rep_b = seed_reporter(&pool, "kpi-b@reporters.test").await;

    // Org A: 2 new, 1 closed. Org B: 1 new, 3 resolved.
    seed_faults(&pool, org_a, bld_a, rep_a, "new", 2).await;
    seed_faults(&pool, org_a, bld_a, rep_a, "closed", 1).await;
    seed_faults(&pool, org_b, bld_b, rep_b, "new", 1).await;
    seed_faults(&pool, org_b, bld_b, rep_b, "resolved", 3).await;

    let fault_repo = FaultRepository::new(pool.clone());
    let admin_repo = PlatformAdminRepository::new(pool.clone());

    let stats_a = fault_repo
        .get_statistics(org_a, None)
        .await
        .expect("owner stats A");
    let stats_b = fault_repo
        .get_statistics(org_b, None)
        .await
        .expect("owner stats B");
    let support = admin_repo.get_support_data().await.expect("support data");

    // Per-org owner statistics carry exactly what was seeded.
    let map_a = counts_map(
        stats_a
            .by_status
            .iter()
            .map(|c| (c.status.clone(), c.count)),
    );
    assert_eq!(map_a.get("new"), Some(&2), "org A new");
    assert_eq!(map_a.get("closed"), Some(&1), "org A closed");
    assert_eq!(map_a.get("resolved"), None, "org A has no resolved");

    let map_b = counts_map(
        stats_b
            .by_status
            .iter()
            .map(|c| (c.status.clone(), c.count)),
    );
    assert_eq!(map_b.get("new"), Some(&1), "org B new");
    assert_eq!(map_b.get("resolved"), Some(&3), "org B resolved");

    // The shared-definition invariant: global == sum of per-org, per status.
    let global = counts_map(
        support
            .fault_by_status
            .iter()
            .map(|c| (c.status.clone(), c.count)),
    );

    let mut expected: BTreeMap<String, i64> = BTreeMap::new();
    for (status, count) in map_a.iter().chain(map_b.iter()) {
        *expected.entry(status.clone()).or_insert(0) += *count;
    }

    assert_eq!(
        global, expected,
        "support-data global fault-by-status must equal the sum of per-org owner statistics"
    );
    assert_eq!(global.get("new"), Some(&3), "global new = 2 (A) + 1 (B)");
    assert_eq!(global.get("closed"), Some(&1), "global closed = 1 (A)");
    assert_eq!(global.get("resolved"), Some(&3), "global resolved = 3 (B)");
}
