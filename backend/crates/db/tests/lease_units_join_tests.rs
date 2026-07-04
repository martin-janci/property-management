//! Regression test for #2076 (follow-up to PR #2060) — several `LeaseRepository`
//! list/detail queries join `units u ON u.id = <table>.unit_id` and project
//! `u.designation` as the unit label. If that join column ever drifts (e.g. a
//! rename to a non-existent `u.unit_number` / `u.label`, the class of bug PR
//! #2060 fixed), the query `42703`-fails at fetch → 500 on the lease routes.
//!
//! This drives the four affected read paths through a real pool:
//!   - `list_applications` (tenant-applications list)
//!   - `list_leases` (leases list)
//!   - `get_expiration_overview_rls` (expiring-leases list)
//!   - `get_lease_with_details` (single-lease detail)
//! and asserts both (a) no 42703 on the units join and (b) the returned unit
//! label equals the seeded `units.designation`. It fails on the pre-#2060 SQL
//! and passes with the correct `u.designation` projection.

use chrono::{Duration, Utc};
use db::models::lease::{ApplicationListQuery, CreateApplication, CreateLease, LeaseListQuery};
use db::repositories::LeaseRepository;
use rust_decimal::Decimal;
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

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn lease_read_paths_project_units_designation_as_unit_label(pool: PgPool) {
    let org = seed_org(&pool, "units-join").await;
    let building = seed_building(&pool, org).await;
    let unit = seed_unit(&pool, building, DESIGNATION).await;

    let repo = LeaseRepository::new(pool.clone());

    // Seed a tenant application on the unit.
    repo.create_application_rls(
        &pool,
        org,
        CreateApplication {
            unit_id: unit,
            applicant_name: "Jane Applicant".to_string(),
            applicant_email: "jane@lease.test".to_string(),
            applicant_phone: None,
            date_of_birth: None,
            national_id: None,
            current_address: None,
            current_landlord_name: None,
            current_landlord_phone: None,
            current_rent_amount: None,
            current_tenancy_start: None,
            employer_name: None,
            employer_phone: None,
            job_title: None,
            employment_start: None,
            monthly_income: None,
            desired_move_in: None,
            desired_lease_term_months: None,
            proposed_rent: None,
            co_applicants: None,
            source: None,
            referral_code: None,
        },
    )
    .await
    .expect("seed tenant application");

    // Seed a lease on the same unit, ending soon so it shows in the expiring
    // overview (which filters status = 'active' AND end_date <= today + 90d).
    let today = Utc::now().date_naive();
    let lease = repo
        .create_lease_rls(
            &pool,
            org,
            Uuid::new_v4(),
            CreateLease {
                unit_id: unit,
                application_id: None,
                template_id: None,
                landlord_user_id: None,
                landlord_name: "Landlord Ltd".to_string(),
                landlord_address: None,
                tenant_user_id: None,
                tenant_name: "Jane Applicant".to_string(),
                tenant_email: "jane@lease.test".to_string(),
                tenant_phone: None,
                occupants: None,
                start_date: today - Duration::days(300),
                end_date: today + Duration::days(20),
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
        .expect("seed lease");

    // Activate the lease so the expiring-overview filter (status = 'active') keeps it.
    sqlx::query("UPDATE leases SET status = 'active' WHERE id = $1")
        .bind(lease.id)
        .execute(&pool)
        .await
        .expect("activate lease");

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

    // 4) Single-lease detail — joins units for `unit_name`.
    #[allow(deprecated)]
    let details = repo
        .get_lease_with_details(lease.id)
        .await
        .expect("get_lease_with_details must not 42703 on the units join (#2076)")
        .expect("the seeded lease must be found");
    assert_eq!(
        details.unit_name, DESIGNATION,
        "lease detail unit_name must equal the seeded units.designation"
    );
}
