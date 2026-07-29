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
//! fails with `ColumnDecode` ("mismatched types").
//!
//! #2169 (follow-up to PR #2150): the residual `SELECT *` in
//! `get_lease_with_details_rls` and the `RETURNING *` in every lease write
//! method (`create_lease_rls` / `update_lease_rls` / `terminate_lease_rls` /
//! `renew_lease_rls` / `mark_sent_for_signature_rls`) carried the same drift and
//! 500'd GET /leases/{id} and all lease mutations. The second test in this file
//! (`lease_rls_detail_and_write_paths_decode_enum_status`) drives those exact
//! paths through the repo — no raw-SQL seeding — and asserts the enum-backed
//! `status` / `termination_reason` decode as text.

use crate::common::seed_org;
use chrono::{Duration, Utc};
use db::models::lease::{ApplicationListQuery, CreateLease, LeaseListQuery, TerminateLease};
use db::repositories::LeaseRepository;
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

const DESIGNATION: &str = "A-101-JOIN";

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

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO users (email, password_hash, name) VALUES ($1, 'x', 'Lease User') RETURNING id",
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

/// #2169 regression. Drives the RLS single-lease detail read and the lease
/// write paths (create + terminate) that previously used `SELECT *` /
/// `RETURNING *` and 500'd with `ColumnDecode` because `leases.status`
/// (`lease_status`) and `leases.termination_reason` (`termination_reason`) are
/// Postgres ENUMs decoded as `String`. Pre-fix, `get_lease_with_details_rls`
/// and every mutation fail at fetch; post-fix, the enum columns decode as text.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn lease_rls_detail_and_write_paths_decode_enum_status(pool: PgPool) {
    let org = seed_org(&pool, "rls-detail").await;
    let building = seed_building(&pool, org).await;
    let unit = seed_unit(&pool, building, "B-202-RLS").await;
    let user = seed_user(&pool, "creator@lease.test").await;

    let repo = LeaseRepository::new(pool.clone());
    let today = Utc::now().date_naive();

    // 1) create_lease_rls — INSERT ... RETURNING LEASE_COLUMNS. Pre-fix this
    //    used `RETURNING *` and failed to decode the `status` enum.
    let created = repo
        .create_lease_rls(
            &pool,
            org,
            user,
            CreateLease {
                unit_id: unit,
                application_id: None,
                template_id: None,
                landlord_user_id: None,
                landlord_name: "Landlord Ltd".to_string(),
                landlord_address: None,
                tenant_user_id: None,
                tenant_name: "Jane Tenant".to_string(),
                tenant_email: "jane@lease.test".to_string(),
                tenant_phone: None,
                occupants: None,
                start_date: today - Duration::days(10),
                end_date: today + Duration::days(355),
                term_months: 12,
                is_fixed_term: Some(true),
                monthly_rent: Decimal::new(1000, 0),
                security_deposit: Decimal::new(1000, 0),
                deposit_held_by: None,
                rent_due_day: Some(1),
                late_fee_amount: None,
                late_fee_grace_days: None,
                utilities_included: None,
                parking_spaces: None,
                storage_units: None,
                pets_allowed: None,
                pet_deposit: None,
                max_occupants: None,
                smoking_allowed: None,
                notes: None,
            },
        )
        .await
        .expect("create_lease_rls must decode the status enum (RETURNING LEASE_COLUMNS, #2169)");
    assert_eq!(
        created.status, "draft",
        "newly created lease status must decode via status::text"
    );

    // 2) get_lease_with_details_rls — the exact #2169 500. Pre-fix used
    //    `SELECT *` and failed at fetch on the `status` enum.
    let details = repo
        .get_lease_with_details_rls(&pool, created.id)
        .await
        .expect("get_lease_with_details_rls must not ColumnDecode on status (#2169)")
        .expect("the created lease must be found");
    assert_eq!(
        details.lease.status, "draft",
        "RLS detail lease status must decode via LEASE_COLUMNS status::text"
    );

    // 3) terminate_lease_rls — UPDATE ... RETURNING LEASE_COLUMNS. Exercises the
    //    `termination_reason` enum decode as well as `status`.
    let terminated = repo
        .terminate_lease_rls(
            &pool,
            created.id,
            user,
            TerminateLease {
                termination_reason: "mutual_agreement".to_string(),
                termination_notes: Some("closed early".to_string()),
                effective_date: Some(today),
            },
        )
        .await
        .expect("terminate_lease_rls must decode status + termination_reason enums (#2169)");
    assert_eq!(
        terminated.status, "terminated",
        "terminated lease status must decode via status::text"
    );
    assert_eq!(
        terminated.termination_reason.as_deref(),
        Some("mutual_agreement"),
        "termination_reason enum must decode via termination_reason::text"
    );
}
