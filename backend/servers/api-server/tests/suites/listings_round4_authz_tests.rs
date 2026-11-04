//! Round-4 regressions (audit4-operations) for `/api/v1/listings`.
//!
//! * Listings now use `ValidatedTenantExtractor`: a user who is not a member
//!   of the `X-Tenant-ID` org is rejected (previously `TenantExtractor` took
//!   the header at face value, so any user could read/modify any org's
//!   listings).
//! * Mutations need an agent-tier role or above from the DB membership; a
//!   `resident` member gets 403.
//! * `create_from_unit` with another org's unit id is 404 (no foreign unit /
//!   building data copied into the caller's listing).

#![allow(dead_code)]

use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{seed_membership, seed_org, TestApp, TestConfig};

async fn user(pool: &PgPool) -> (String, Uuid) {
    use api_server::services::JwtService;
    let email = format!("lst-r4-{}@test.internal", Uuid::new_v4());
    let id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO users (email, password_hash, name, status, email_verified_at)
           VALUES ($1, 'test_hash', 'Listings R4', 'active', NOW()) RETURNING id"#,
    )
    .bind(&email)
    .fetch_one(pool)
    .await
    .unwrap();
    let config = TestConfig::default();
    let token = JwtService::new(&config.jwt_secret)
        .unwrap()
        .generate_access_token(id, &email, "Listings R4", None, None)
        .unwrap();
    (token, id)
}

async fn seed_listing(pool: &PgPool, org_id: Uuid, user_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO listings
               (organization_id, created_by, status, transaction_type, title,
                property_type, size_sqm, rooms, street, city, postal_code,
                country, latitude, longitude, price, currency)
           VALUES ($1, $2, 'active', 'sale', 'R4 Listing', 'apartment',
                   75.5, 3, 'Main 1', 'Bratislava', '81101', 'SK',
                   48.1486, 17.1077, 100000, 'EUR')
           RETURNING id"#,
    )
    .bind(org_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn seed_unit(pool: &PgPool, org_id: Uuid) -> Uuid {
    let building_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO buildings (organization_id, street, city, postal_code, country)
           VALUES ($1, 'Secret St 9', 'Bratislava', '81101', 'Slovakia') RETURNING id"#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .unwrap();
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO units (
               building_id, designation, floor, unit_type,
               size_sqm, rooms, ownership_share, occupancy_status, status, settings
           )
           VALUES ($1, '9Z', 1, 'apartment', 50.0, 2, 100.00, 'unknown', 'active', '{}'::jsonb)
           RETURNING id"#,
    )
    .bind(building_id)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn listing_count(pool: &PgPool, org: Uuid) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM listings WHERE organization_id = $1")
        .bind(org)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn non_member_cannot_use_foreign_tenant_header(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "lsr4a").await;
    let org_b = seed_org(&pool, "lsr4b").await;
    let (_ta, user_a) = user(&pool).await;
    seed_membership(&pool, org_a, user_a, "manager").await;
    let (token_b, user_b) = user(&pool).await;
    seed_membership(&pool, org_b, user_b, "manager").await;
    let listing_a = seed_listing(&pool, org_a, user_a).await;

    for rb in [
        app.get("/api/v1/listings"),
        app.get(&format!("/api/v1/listings/{listing_a}")),
        app.delete(&format!("/api/v1/listings/{listing_a}")),
    ] {
        let req = rb
            .bearer(&token_b)
            .header("X-Tenant-ID", &org_a.to_string())
            .build();
        let uri = req.uri().to_string();
        let resp = app.execute(req).await;
        assert_eq!(
            resp.status,
            StatusCode::FORBIDDEN,
            "{uri}: a non-member must not act on org A via X-Tenant-ID: {}",
            resp.text()
        );
    }
    assert_eq!(listing_count(&pool, org_a).await, 1);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn resident_cannot_delete_listing_but_manager_can(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "lsr4r").await;
    let (token_m, mgr) = user(&pool).await;
    seed_membership(&pool, org, mgr, "manager").await;
    let (token_r, res) = user(&pool).await;
    seed_membership(&pool, org, res, "resident").await;
    let listing = seed_listing(&pool, org, mgr).await;
    let tenant = org.to_string();

    let resp = app
        .execute(
            app.delete(&format!("/api/v1/listings/{listing}"))
                .bearer(&token_r)
                .header("X-Tenant-ID", &tenant)
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::FORBIDDEN, "{}", resp.text());
    assert_eq!(listing_count(&pool, org).await, 1);

    let resp = app
        .execute(
            app.get(&format!("/api/v1/listings/{listing}"))
                .bearer(&token_r)
                .header("X-Tenant-ID", &tenant)
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "resident may read: {}",
        resp.text()
    );

    // Only draft listings are deletable; the role gate runs before that rule.
    sqlx::query("UPDATE listings SET status = 'draft' WHERE id = $1")
        .bind(listing)
        .execute(&pool)
        .await
        .unwrap();
    let resp = app
        .execute(
            app.delete(&format!("/api/v1/listings/{listing}"))
                .bearer(&token_m)
                .header("X-Tenant-ID", &tenant)
                .build(),
        )
        .await;
    assert!(resp.status.is_success(), "{}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_from_foreign_unit_is_not_found(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "lsr4u").await;
    let org_b = seed_org(&pool, "lsr4v").await;
    let (token_b, user_b) = user(&pool).await;
    seed_membership(&pool, org_b, user_b, "manager").await;
    let unit_a = seed_unit(&pool, org_a).await;

    let resp = app
        .execute(
            app.post("/api/v1/listings/from-unit")
                .json(json!({
                    "unit_id": unit_a,
                    "transaction_type": "sale",
                    "price": "100000",
                }))
                .bearer(&token_b)
                .header("X-Tenant-ID", &org_b.to_string())
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::NOT_FOUND, "{}", resp.text());
    assert_eq!(listing_count(&pool, org_b).await, 0);
}
