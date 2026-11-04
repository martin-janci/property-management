//! Round-4 regressions (audit4-operations) for `/api/v1/rentals`.
//!
//! OTA connection / booking / calendar / guest / iCal / authority-report
//! mutations and the guest-PII reads (check-in reminders, reports) are
//! manager-only, with the role taken from the `organization_members` row. A
//! `resident` member gets 403 and nothing changes; a manager succeeds; a
//! user who is not a member of the `X-Tenant-ID` org is rejected too.

#![allow(dead_code)]

use axum::http::StatusCode;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{seed_membership, seed_org, TestApp, TestConfig};

async fn user(pool: &PgPool) -> (String, Uuid) {
    use api_server::services::JwtService;
    let email = format!("rent-r4-{}@test.internal", Uuid::new_v4());
    let id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO users (email, password_hash, name, status, email_verified_at)
           VALUES ($1, 'test_hash', 'Rentals R4', 'active', NOW()) RETURNING id"#,
    )
    .bind(&email)
    .fetch_one(pool)
    .await
    .unwrap();
    let config = TestConfig::default();
    let token = JwtService::new(&config.jwt_secret)
        .unwrap()
        .generate_access_token(id, &email, "Rentals R4", None, None)
        .unwrap();
    (token, id)
}

async fn seed_connection(pool: &PgPool, org_id: Uuid) -> Uuid {
    let building = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO buildings (organization_id, street, city, postal_code, country)
           VALUES ($1, 'Rent St 1', 'Bratislava', '81101', 'Slovakia') RETURNING id"#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .unwrap();
    let unit = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO units (building_id, designation) VALUES ($1, 'RT-1') RETURNING id"#,
    )
    .bind(building)
    .fetch_one(pool)
    .await
    .unwrap();
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO rental_platform_connections (organization_id, unit_id, platform, is_active)
           VALUES ($1, $2, 'airbnb', true) RETURNING id"#,
    )
    .bind(org_id)
    .bind(unit)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn connection_count(pool: &PgPool, id: Uuid) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM rental_platform_connections WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn resident_and_non_member_are_denied(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "rtr4r").await;
    let (token_r, resident) = user(&pool).await;
    seed_membership(&pool, org, resident, "resident").await;
    let (token_x, outsider) = user(&pool).await;
    let other = seed_org(&pool, "rtr4o").await;
    seed_membership(&pool, other, outsider, "manager").await;
    let conn = seed_connection(&pool, org).await;
    let tenant = org.to_string();

    for token in [&token_r, &token_x] {
        for rb in [
            app.delete(&format!("/api/v1/rentals/connections/{conn}")),
            app.get("/api/v1/rentals/reports"),
            app.get("/api/v1/rentals/checkin-reminders"),
        ] {
            let req = rb.bearer(token).header("X-Tenant-ID", &tenant).build();
            let uri = req.uri().to_string();
            let resp = app.execute(req).await;
            assert_eq!(
                resp.status,
                StatusCode::FORBIDDEN,
                "{uri} must be manager-only: {}",
                resp.text()
            );
        }
    }
    assert_eq!(connection_count(&pool, conn).await, 1);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn manager_can_read_reports_and_delete_connection(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "rtr4m").await;
    let (token, mgr) = user(&pool).await;
    seed_membership(&pool, org, mgr, "manager").await;
    let conn = seed_connection(&pool, org).await;
    let tenant = org.to_string();

    let resp = app
        .execute(
            app.get("/api/v1/rentals/reports")
                .bearer(&token)
                .header("X-Tenant-ID", &tenant)
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "{}", resp.text());

    let resp = app
        .execute(
            app.delete(&format!("/api/v1/rentals/connections/{conn}"))
                .bearer(&token)
                .header("X-Tenant-ID", &tenant)
                .build(),
        )
        .await;
    assert!(resp.status.is_success(), "{}", resp.text());
    assert_eq!(connection_count(&pool, conn).await, 0);
}
