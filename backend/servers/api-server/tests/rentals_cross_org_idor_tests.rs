//! Regression tests for the cross-tenant IDOR fix on the Epic 18 short-term
//! rental routes (issue #804).
//!
//! Audit history: the `/api/v1/rentals/*` handlers had two classes of bug:
//!
//!   - **P0 — unauthenticated by-id reads.** `get_connection`, `get_booking`,
//!     `get_booking_with_guests`, `get_guest` and `get_report` took NO auth
//!     extractor at all, so any anonymous caller could exfiltrate OAuth tokens,
//!     full booking records and guest PII by guessing a UUID.
//!   - **P1 — mutations ignored the tenant.** `update_*` / `delete_*` /
//!     `register_guest` / `submit_report` extracted the tenant but discarded it
//!     (`_tenant`) and addressed rows by id with no `organization_id` predicate,
//!     so org B could read or mutate org A's rental data.
//!
//! Every rental table carries `organization_id`, so the fix adds
//! `*_belongs_to_org` EXISTS guards (and a `unit_belongs_to_org` building-owner
//! check) that the handlers call before reading/mutating; a foreign/unknown id
//! is collapsed to `404`. Mirrors the `_belongs_to_org` idiom used by the
//! fault / work-order / meter routes.
//!
//! Why repo-layer and not HTTP-layer: `TestApp` mounts the router WITHOUT
//! `host_tenant_middleware`, so `TenantExtractor` can never resolve a tenant in
//! tests (see `ai_llm_cross_tenant_idor_tests.rs`'s caveat). The IDOR fix lives
//! in the repository's WHERE clause, so the contract is asserted there directly.

use db::repositories::RentalRepository;
use sqlx::PgPool;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

async fn seed_org(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO organizations (name, slug, contact_email, status)
        VALUES ($1, $2, $3, 'active') RETURNING id
        "#,
    )
    .bind(format!("RentalIDOR Org {slug}"))
    .bind(format!("rental-idor-org-{slug}"))
    .bind(format!("{slug}@rental-idor.test"))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_building(pool: &PgPool, org_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO buildings (organization_id, street, city, postal_code)
        VALUES ($1, 'Rental St 1', 'Bratislava', '81101') RETURNING id
        "#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed building")
}

async fn seed_unit(pool: &PgPool, building_id: Uuid, designation: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO units (building_id, designation) VALUES ($1, $2) RETURNING id"#,
    )
    .bind(building_id)
    .bind(designation)
    .fetch_one(pool)
    .await
    .expect("seed unit")
}

async fn seed_connection(pool: &PgPool, org_id: Uuid, unit_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO rental_platform_connections (organization_id, unit_id, platform)
        VALUES ($1, $2, 'airbnb') RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(unit_id)
    .fetch_one(pool)
    .await
    .expect("seed connection")
}

async fn seed_booking(pool: &PgPool, org_id: Uuid, unit_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO rental_bookings
            (organization_id, unit_id, platform, guest_name, check_in, check_out)
        VALUES ($1, $2, 'airbnb', 'Jane Doe', '2026-06-01', '2026-06-05') RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(unit_id)
    .fetch_one(pool)
    .await
    .expect("seed booking")
}

async fn seed_guest(pool: &PgPool, org_id: Uuid, booking_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO rental_guests (organization_id, booking_id, first_name, last_name)
        VALUES ($1, $2, 'Jane', 'Doe') RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(booking_id)
    .fetch_one(pool)
    .await
    .expect("seed guest")
}

async fn seed_report(pool: &PgPool, org_id: Uuid, building_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO rental_guest_reports
            (organization_id, building_id, report_type, period_start, period_end,
             authority_code, authority_name, report_format)
        VALUES ($1, $2, 'monthly', '2026-06-01', '2026-06-30',
                'SK_UHUL', 'Authority', 'pdf') RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(building_id)
    .fetch_one(pool)
    .await
    .expect("seed report")
}

async fn seed_calendar_block(pool: &PgPool, org_id: Uuid, unit_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO rental_calendar_blocks
            (organization_id, unit_id, block_start, block_end, reason)
        VALUES ($1, $2, '2026-06-10', '2026-06-12', 'maintenance') RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(unit_id)
    .fetch_one(pool)
    .await
    .expect("seed calendar block")
}

async fn seed_ical_feed(pool: &PgPool, org_id: Uuid, unit_id: Uuid, token: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO rental_ical_feeds (organization_id, unit_id, feed_name, feed_token)
        VALUES ($1, $2, 'Feed', $3) RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(unit_id)
    .bind(token)
    .fetch_one(pool)
    .await
    .expect("seed ical feed")
}

// ---------------------------------------------------------------------------
// (1) Every rental resource guard is tenant-scoped — org B cannot resolve any
//     of org A's resources. These guards back the handler authorization checks
//     in front of the previously-unauthenticated reads and the mutation paths.
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn rental_resources_are_tenant_scoped(pool: PgPool) {
    let repo = RentalRepository::new(pool.clone());

    let org_a = seed_org(&pool, "a").await;
    let org_b = seed_org(&pool, "b").await;
    let building_a = seed_building(&pool, org_a).await;
    let unit_a = seed_unit(&pool, building_a, "3B").await;

    let connection = seed_connection(&pool, org_a, unit_a).await;
    let booking = seed_booking(&pool, org_a, unit_a).await;
    let guest = seed_guest(&pool, org_a, booking).await;
    let report = seed_report(&pool, org_a, building_a).await;
    let block = seed_calendar_block(&pool, org_a, unit_a).await;
    let feed = seed_ical_feed(&pool, org_a, unit_a, "tok-a").await;

    // Same-org guards all succeed.
    assert!(repo
        .connection_belongs_to_org(connection, org_a)
        .await
        .unwrap());
    assert!(repo.booking_belongs_to_org(booking, org_a).await.unwrap());
    assert!(repo.guest_belongs_to_org(guest, org_a).await.unwrap());
    assert!(repo.report_belongs_to_org(report, org_a).await.unwrap());
    assert!(repo
        .calendar_block_belongs_to_org(block, org_a)
        .await
        .unwrap());
    assert!(repo.ical_feed_belongs_to_org(feed, org_a).await.unwrap());

    // Cross-org guards all fail (the IDOR is blocked → handler maps to 404).
    assert!(!repo
        .connection_belongs_to_org(connection, org_b)
        .await
        .unwrap());
    assert!(!repo.booking_belongs_to_org(booking, org_b).await.unwrap());
    assert!(!repo.guest_belongs_to_org(guest, org_b).await.unwrap());
    assert!(!repo.report_belongs_to_org(report, org_b).await.unwrap());
    assert!(!repo
        .calendar_block_belongs_to_org(block, org_b)
        .await
        .unwrap());
    assert!(!repo.ical_feed_belongs_to_org(feed, org_b).await.unwrap());

    // Sanity: the unscoped lookup the vulnerable pre-fix handlers performed
    // does return the connection regardless of org (the leak the guard closes).
    let unscoped: Option<Uuid> =
        sqlx::query_scalar("SELECT id FROM rental_platform_connections WHERE id = $1")
            .bind(connection)
            .fetch_optional(&pool)
            .await
            .unwrap();
    assert_eq!(unscoped, Some(connection));
}

// ---------------------------------------------------------------------------
// (2) Unit ownership is resolved through the building's org — a unit in org A's
//     building is not addressable by org B (guards the unit-keyed read endpoints
//     and the client-supplied unit_id on create).
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn unit_belongs_to_org_checks_building_owner(pool: PgPool) {
    let repo = RentalRepository::new(pool.clone());

    let org_a = seed_org(&pool, "ua").await;
    let org_b = seed_org(&pool, "ub").await;
    let building_a = seed_building(&pool, org_a).await;
    let building_b = seed_building(&pool, org_b).await;
    let unit_a = seed_unit(&pool, building_a, "1A").await;
    let unit_b = seed_unit(&pool, building_b, "1B").await;

    assert!(
        repo.unit_belongs_to_org(unit_a, org_a).await.unwrap(),
        "org A owns the unit in its own building"
    );
    assert!(
        !repo.unit_belongs_to_org(unit_a, org_b).await.unwrap(),
        "org B must NOT be able to address org A's unit"
    );
    assert!(
        repo.unit_belongs_to_org(unit_b, org_b).await.unwrap(),
        "org B owns its own unit"
    );
    assert!(
        !repo.unit_belongs_to_org(unit_b, org_a).await.unwrap(),
        "org A must NOT be able to address org B's unit"
    );
}
