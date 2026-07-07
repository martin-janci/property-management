//! Regression test for #2076 (follow-up to PR #2060) — several `LeaseRepository`
//! list/detail queries join `units u ON u.id = <table>.unit_id` and project
//! `u.designation` as the unit label. If that join column ever drifts (e.g. a
//! rename to a non-existent `u.unit_number` / `u.label`, the class of bug PR
//! #2060 fixed), the query `42703`-fails at fetch → 500 on the lease routes.
//!
//! This drives the four affected read paths through a real pool:
//!   - `list_applications_rls` (tenant-applications list)
//!   - `list_leases_rls` (leases list)
//!   - `get_expiration_overview_rls` (expiring-leases list)
//!   - `get_lease_with_details` (single-lease detail)
//!
//! and asserts both (a) no error on the units join (42703 on drift) and (b) the
//! returned unit label equals the seeded `units.designation`. It fails on the
//! pre-#2060 SQL and passes with the correct `u.designation` projection.
//!
//! It additionally pins the enum-decode fix that this test flushed out:
//! `tenant_applications.status` / `leases.status` / `leases.termination_reason`
//! are Postgres ENUM columns, while the models decode them as `String` — the
//! read paths must project `status::text` (and `find_lease_by_id_rls` must use
//! the explicit `LEASE_COLUMNS` list instead of `SELECT *`) or every fetch
//! fails with `ColumnDecode` ("mismatched types"). Seeding is done via raw SQL
//! because the create paths (`create_application_rls` / `create_lease_rls`)
//! still use `RETURNING *` and carry the same decode drift (known residual,
//! tracked in the PR body).

use chrono::{Duration, Utc};
use db::models::lease::{ApplicationListQuery, LeaseListQuery};
use db::repositories::LeaseRepository;
use sqlx::PgPool;
use uuid::Uuid;

const DESIGNATION: &str = "A-101-JOIN";

async fn seed_org(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO organizations (name, slug, contact_email, status)
        VALUES ($1, $2, $3, 'active')
        RETURNING id
        "#,
    )
    .bind(format!("Lease {slug}"))
    .bind(slug)
    .bind(format!("{slug}@lease.test"))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_building(pool: &PgPool, org: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO buildings (organization_id, name, street, city, postal_code, country, status)
        VALUES ($1, 'Join Tower', 'Street 1', 'City', '00000', 'SK', 'active')
        RETURNING id
        "#,
    )
    .bind(org)
    .fetch_one(pool)
    .await
    .expect("seed building")
}

async fn seed_unit(pool: &PgPool, building: Uuid, designation: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO units (building_id, designation) VALUES ($1, $2) RETURNING id",
    )
    .bind(building)
    .bind(designation)
    .fetch_one(pool)
    .await
    .expect("seed unit")
}

/// Seed a tenant application via raw SQL (the repo's `create_application_rls`
/// uses `RETURNING *`, which still trips the enum-decode drift on `status`).
async fn seed_application(pool: &PgPool, org: Uuid, unit: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO tenant_applications (
            organization_id, unit_id, applicant_name, applicant_email, status, submitted_at
        )
        VALUES ($1, $2, 'Jane Applicant', 'jane@lease.test', 'draft', NOW())
        RETURNING id
        "#,
    )
    .bind(org)
    .bind(unit)
    .fetch_one(pool)
    .await
    .expect("seed tenant application")
}

/// Seed an active lease via raw SQL (the repo's `create_lease_rls` uses
/// `RETURNING *`, same enum-decode drift). Ends in 20 days so it lands in the
/// expiring overview's 30-day bucket (filters status = 'active' AND
/// end_date <= today + 90d).
async fn seed_lease(pool: &PgPool, org: Uuid, unit: Uuid) -> Uuid {
    let today = Utc::now().date_naive();
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO leases (
            organization_id, unit_id, landlord_name, tenant_name, tenant_email,
            start_date, end_date, term_months, monthly_rent, security_deposit, status
        )
        VALUES ($1, $2, 'Landlord Ltd', 'Jane Applicant', 'jane@lease.test',
                $3, $4, 12, 1000, 1000, 'active')
        RETURNING id
        "#,
    )
    .bind(org)
    .bind(unit)
    .bind(today - Duration::days(300))
    .bind(today + Duration::days(20))
    .fetch_one(pool)
    .await
    .expect("seed lease")
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn lease_read_paths_project_units_designation_as_unit_label(pool: PgPool) {
    let org = seed_org(&pool, "units-join").await;
    let building = seed_building(&pool, org).await;
    let unit = seed_unit(&pool, building, DESIGNATION).await;
    seed_application(&pool, org, unit).await;
    let lease = seed_lease(&pool, org, unit).await;

    let repo = LeaseRepository::new(pool.clone());

    // 1) Tenant-applications list — joins units for `unit_name`.
    let (apps, _) = repo
        .list_applications_rls(
            &pool,
            org,
            ApplicationListQuery {
                unit_id: None,
                status: None,
                limit: None,
                offset: None,
            },
        )
        .await
        .expect("list_applications must not 42703 on the units join (#2076)");
    assert_eq!(apps.len(), 1, "the seeded application must be listed");
    assert_eq!(
        apps[0].unit_name, DESIGNATION,
        "application unit_name must equal the seeded units.designation"
    );
    assert_eq!(
        apps[0].status, "draft",
        "application status must decode via status::text"
    );

    // 2) Leases list — joins units for `unit_name`.
    let (leases, _) = repo
        .list_leases_rls(
            &pool,
            org,
            LeaseListQuery {
                unit_id: None,
                tenant_id: None,
                status: None,
                expiring_within_days: None,
                limit: None,
                offset: None,
            },
        )
        .await
        .expect("list_leases must not 42703 on the units join (#2076)");
    assert_eq!(leases.len(), 1, "the seeded lease must be listed");
    assert_eq!(
        leases[0].unit_name, DESIGNATION,
        "lease unit_name must equal the seeded units.designation"
    );
    assert_eq!(
        leases[0].status, "active",
        "lease status must decode via status::text"
    );

    // 3) Expiring-leases list — joins units for `unit_name`.
    let overview = repo
        .get_expiration_overview_rls(&pool, org)
        .await
        .expect("get_expiration_overview must not 42703 on the units join (#2076)");
    assert_eq!(
        overview.expiring_30_days.len(),
        1,
        "the active lease ending in 20 days must land in the 30-day bucket"
    );
    assert_eq!(
        overview.expiring_30_days[0].unit_name, DESIGNATION,
        "expiring-lease unit_name must equal the seeded units.designation"
    );

    // 4) Single-lease detail — joins units for `unit_name`; also pins the
    //    `LEASE_COLUMNS` enum-decode fix in `find_lease_by_id_rls`.
    #[allow(deprecated)]
    let details = repo
        .get_lease_with_details(lease)
        .await
        .expect("get_lease_with_details must not 42703 on the units join (#2076)")
        .expect("the seeded lease must be found");
    assert_eq!(
        details.unit_name, DESIGNATION,
        "lease detail unit_name must equal the seeded units.designation"
    );
    assert_eq!(
        details.lease.status, "active",
        "lease status must decode via LEASE_COLUMNS status::text"
    );
}
