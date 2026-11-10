//! Regression tests for the Energy & Sustainability schema (Epic 65).
//!
//! `EnergyRepository` queried six tables (energy_performance_certificates,
//! carbon_emissions, carbon_targets, sustainability_scores,
//! building_benchmarks, benchmark_alerts) and six enum types that no
//! migration created, so every `/api/v1/energy` endpoint failed with
//! `relation "..." does not exist`. It also summed a non-existent
//! `units.floor_area` column (the real column is `size_sqm`). Migration
//! 00241 adds the schema; these tests drive every repository write path and
//! the `ON CONFLICT` upserts against it.

use chrono::NaiveDate;
use db::models::energy::{
    BenchmarkAlertSeverity, BenchmarkAlertsQuery, BenchmarkMetricType, BenchmarkQuery,
    CalculateBenchmark, CreateBenchmarkAlert, CreateCarbonEmission, CreateCarbonTarget,
    CreateEnergyPerformanceCertificate, CreateSustainabilityScore, EmissionSourceType,
    EnergyRating, HeatingType, InsulationRating, SustainabilityFilter, UpdateSustainabilityScore,
};
use db::repositories::EnergyRepository;
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{seed_org, set_ctx};

struct Fixture {
    org: Uuid,
    user: Uuid,
    building: Uuid,
    unit: Uuid,
    listing: Uuid,
}

async fn seed(pool: &PgPool) -> Fixture {
    set_ctx(pool, None, None, true).await;
    let org = seed_org(pool, "energy-schema").await;
    let user: Uuid = sqlx::query_scalar(
        "INSERT INTO users (email, password_hash, name, status, email_verified_at) \
         VALUES ('energy@schema.test', 'h', 'U', 'active', NOW()) RETURNING id",
    )
    .fetch_one(pool)
    .await
    .unwrap();
    let building: Uuid = sqlx::query_scalar(
        "INSERT INTO buildings (organization_id, name, street, city, postal_code, country) \
         VALUES ($1, 'B', 'S', 'C', '00000', 'SK') RETURNING id",
    )
    .bind(org)
    .fetch_one(pool)
    .await
    .unwrap();
    let unit: Uuid = sqlx::query_scalar(
        "INSERT INTO units (building_id, designation, size_sqm) VALUES ($1, '1', 50) RETURNING id",
    )
    .bind(building)
    .fetch_one(pool)
    .await
    .unwrap();
    let listing: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO listings (
            organization_id, created_by, status, transaction_type, title,
            property_type, street, city, postal_code, price
        )
        VALUES ($1, $2, 'active', 'rent', 'Test', 'apartment', 'S', 'C', '00000', 100)
        RETURNING id
        "#,
    )
    .bind(org)
    .bind(user)
    .fetch_one(pool)
    .await
    .unwrap();
    Fixture {
        org,
        user,
        building,
        unit,
        listing,
    }
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn epc_create_list_and_distribution(pool: PgPool) {
    let f = seed(&pool).await;
    let repo = EnergyRepository::new(pool.clone());
    let epc = repo
        .create_epc(
            f.org,
            f.user,
            CreateEnergyPerformanceCertificate {
                unit_id: f.unit,
                rating: EnergyRating::B,
                certificate_number: Some("EPC-1".into()),
                valid_until: NaiveDate::from_ymd_opt(2030, 1, 1),
                annual_energy_kwh: Some(Decimal::new(12000, 0)),
                annual_co2_kg: None,
                primary_energy_kwh_per_sqm: None,
                issued_by: None,
                issued_at: None,
                document_url: None,
                notes: None,
            },
        )
        .await
        .expect("create EPC");
    assert_eq!(epc.rating, EnergyRating::B);
    assert_eq!(epc.organization_id, f.org);

    let list = repo
        .list_epcs_for_building(f.building, 10, 0)
        .await
        .expect("list EPCs");
    assert_eq!(list.total, 1);
    let dist = repo
        .get_rating_distribution(f.building)
        .await
        .expect("rating distribution");
    assert_eq!(dist.len(), 1);
    assert_eq!(dist[0].count, 1);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn carbon_emission_and_target_upserts(pool: PgPool) {
    let f = seed(&pool).await;
    let repo = EnergyRepository::new(pool.clone());
    let emission = |kg: i64| CreateCarbonEmission {
        building_id: f.building,
        year: 2026,
        month: 3,
        source_type: EmissionSourceType::Electricity,
        co2_kg: Decimal::new(kg, 0),
        energy_kwh: None,
        notes: None,
    };
    repo.create_emission(f.org, f.user, emission(100))
        .await
        .expect("create emission");
    // Same (building, year, month, source) must upsert, not duplicate.
    let updated = repo
        .create_emission(f.org, f.user, emission(250))
        .await
        .expect("upsert emission");
    assert_eq!(updated.co2_kg, Decimal::new(250, 0));
    let list = repo
        .list_emissions(f.building, Some(2026), 10, 0)
        .await
        .expect("list emissions");
    assert_eq!(list.total, 1);

    for target in [1000, 900] {
        repo.set_carbon_target(
            f.org,
            CreateCarbonTarget {
                building_id: f.building,
                year: 2026,
                target_co2_kg: Decimal::new(target, 0),
                baseline_co2_kg: None,
                baseline_year: None,
                notes: None,
            },
        )
        .await
        .expect("upsert carbon target");
    }

    let dash = repo
        .get_carbon_dashboard(f.building, 2026)
        .await
        .expect("carbon dashboard (reads units.size_sqm)");
    assert_eq!(dash.total_co2_kg, Decimal::new(250, 0));
    assert_eq!(dash.co2_per_sqm, Decimal::new(5, 0));
    assert_eq!(dash.target_co2_kg, Some(Decimal::new(900, 0)));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn sustainability_score_upsert_update_and_filter(pool: PgPool) {
    let f = seed(&pool).await;
    let repo = EnergyRepository::new(pool.clone());
    let base = CreateSustainabilityScore {
        listing_id: f.listing,
        has_solar: true,
        has_heat_pump: false,
        insulation_rating: InsulationRating::Good,
        heating_type: HeatingType::HeatPump,
        has_ev_charging: None,
        has_rainwater_harvesting: None,
        has_smart_thermostat: None,
        has_led_lighting: None,
        has_double_glazing: None,
        renewable_energy_percentage: Some(40),
        annual_energy_cost_estimate: None,
        energy_rating: Some(EnergyRating::B),
        notes: None,
    };
    repo.upsert_sustainability_score(f.org, base.clone())
        .await
        .expect("insert score");
    repo.upsert_sustainability_score(f.org, base)
        .await
        .expect("ON CONFLICT (listing_id) upsert");

    let updated = repo
        .update_sustainability_score(
            f.listing,
            UpdateSustainabilityScore {
                has_solar: None,
                has_heat_pump: Some(true),
                insulation_rating: None,
                heating_type: None,
                has_ev_charging: None,
                has_rainwater_harvesting: None,
                has_smart_thermostat: None,
                has_led_lighting: None,
                has_double_glazing: None,
                renewable_energy_percentage: None,
                annual_energy_cost_estimate: None,
                energy_rating: None,
                notes: None,
            },
        )
        .await
        .expect("update score");
    assert!(updated.has_heat_pump);

    let hits = repo
        .filter_listings_by_sustainability(
            f.org,
            SustainabilityFilter {
                has_solar: Some(true),
                min_energy_rating: Some(EnergyRating::C),
                ..Default::default()
            },
            10,
            0,
        )
        .await
        .expect("filter listings");
    assert_eq!(hits, vec![f.listing]);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn benchmark_and_alert_lifecycle(pool: PgPool) {
    let f = seed(&pool).await;
    let repo = EnergyRepository::new(pool.clone());
    let period = |m| NaiveDate::from_ymd_opt(2026, m, 1).unwrap();
    let bench = repo
        .calculate_benchmark(
            f.org,
            CalculateBenchmark {
                building_id: f.building,
                metric_type: BenchmarkMetricType::CarbonPerSqm,
                period_start: period(1),
                period_end: period(12),
                region: None,
                building_type: None,
            },
        )
        .await
        .expect("calculate benchmark");
    assert_eq!(bench.building_id, f.building);

    let alert = repo
        .create_benchmark_alert(
            f.org,
            CreateBenchmarkAlert {
                building_id: f.building,
                metric_type: BenchmarkMetricType::CarbonPerSqm,
                current_value: Decimal::new(10, 0),
                benchmark_value: Decimal::new(5, 0),
                deviation_percentage: Decimal::new(100, 0),
                severity: BenchmarkAlertSeverity::Critical,
                message: None,
            },
        )
        .await
        .expect("create alert");
    let resolved = repo
        .resolve_benchmark_alert(alert.id, f.user)
        .await
        .expect("resolve alert");
    assert!(resolved.is_resolved);

    let alerts = repo
        .list_benchmark_alerts(
            f.building,
            BenchmarkAlertsQuery {
                unresolved_only: false,
                severity: None,
                limit: 10,
                offset: 0,
            },
        )
        .await
        .expect("list alerts");
    assert_eq!(alerts.total, 1);
    assert_eq!(alerts.alerts.len(), 1);
    let dash = repo
        .get_benchmark_dashboard(f.building, BenchmarkQuery::default())
        .await
        .expect("benchmark dashboard");
    assert_eq!(dash.benchmarks.len(), 1);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn energy_tables_have_rls_enabled(pool: PgPool) {
    let unprotected: Vec<String> = sqlx::query_scalar(
        "SELECT relname::text FROM pg_class WHERE relname = ANY($1) AND NOT relrowsecurity",
    )
    .bind(vec![
        "energy_performance_certificates",
        "carbon_emissions",
        "carbon_targets",
        "sustainability_scores",
        "building_benchmarks",
        "benchmark_alerts",
    ])
    .fetch_all(&pool)
    .await
    .unwrap();
    assert!(unprotected.is_empty(), "RLS disabled on {unprotected:?}");
    let policies: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pg_policies WHERE policyname LIKE '%_tenant_isolation' \
         AND tablename IN ('energy_performance_certificates','carbon_emissions','carbon_targets', \
         'sustainability_scores','building_benchmarks','benchmark_alerts')",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(policies, 6);
}
