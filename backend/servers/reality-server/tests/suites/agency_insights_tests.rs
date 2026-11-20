//! Agency dashboard data and member management
//! (`routes/agency_insights.rs`, mounted in the agencies router).

use crate::common::{make_app_state, mint_token, send, send_json};
use axum::{
    body::Body,
    http::{header, Method, Request},
    Router,
};
use reality_server::routes;
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

fn app(pool: PgPool) -> Router {
    Router::new()
        .nest("/api/v1/agencies", routes::agencies::router())
        .with_state(make_app_state(pool))
}

async fn get_json(app: &Router, path: &str, token: &str) -> (u16, Value) {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(path)
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status().as_u16();
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

async fn seed_user(pool: &PgPool, tag: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'hash', $2, 'active', NOW(), 'public')
        RETURNING id
        "#,
    )
    .bind(format!("{tag}@agency-insights.test"))
    .bind(format!("Realtor {tag}"))
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn seed_member(pool: &PgPool, agency: Uuid, user: Uuid, role: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO reality_agency_members (agency_id, user_id, role) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(agency)
    .bind(user)
    .bind(role)
    .fetch_one(pool)
    .await
    .unwrap()
}

struct Fixture {
    agency: Uuid,
    owner: Uuid,
    owner_member: Uuid,
    realtor: Uuid,
    realtor_member: Uuid,
    outsider: Uuid,
}

async fn fixture(pool: &PgPool) -> Fixture {
    let agency = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO reality_agencies (name, slug, email)
         VALUES ('Insights', 'insights', 'a@agency-insights.test') RETURNING id",
    )
    .fetch_one(pool)
    .await
    .unwrap();
    let owner = seed_user(pool, "owner").await;
    let realtor = seed_user(pool, "realtor").await;
    let outsider = seed_user(pool, "outsider").await;
    let owner_member = seed_member(pool, agency, owner, "owner").await;
    let realtor_member = seed_member(pool, agency, realtor, "realtor").await;

    let org = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO organizations (name, slug, contact_email, status)
         VALUES ('Insights Org', 'insights-org', 'o@agency-insights.test', 'active') RETURNING id",
    )
    .fetch_one(pool)
    .await
    .unwrap();
    for (status, price) in [("active", "100000"), ("sold", "200000")] {
        let listing = sqlx::query_scalar::<_, Uuid>(
            r#"
            INSERT INTO listings
                (organization_id, created_by, status, transaction_type, title, property_type,
                 price, currency, street, city, postal_code, country)
            VALUES ($1, $2, $3, 'sale', 'Insights Listing', 'apartment', $4::numeric, 'EUR',
                    'Street 1', 'Bratislava', '81101', 'SK')
            RETURNING id
            "#,
        )
        .bind(org)
        .bind(realtor)
        .bind(status)
        .bind(price)
        .fetch_one(pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO listing_inquiries (listing_id, realtor_id, name, email, message)
             VALUES ($1, $2, 'Buyer', 'b@agency-insights.test', 'Hi')",
        )
        .bind(listing)
        .bind(realtor)
        .execute(pool)
        .await
        .unwrap();
    }
    Fixture {
        agency,
        owner,
        owner_member,
        realtor,
        realtor_member,
        outsider,
    }
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn stats_listings_performance_for_members_only(pool: PgPool) {
    let f = fixture(&pool).await;
    let app = app(pool);
    let member = mint_token(f.realtor);

    let (status, stats) = get_json(
        &app,
        &format!("/api/v1/agencies/{}/stats", f.agency),
        &member,
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(stats["totalListings"], 2);
    assert_eq!(stats["activeListings"], 1);
    assert_eq!(stats["soldListings"], 1);
    assert_eq!(stats["totalInquiries"], 2);
    assert_eq!(stats["totalRealtors"], 2);
    assert_eq!(stats["conversionRate"], 0.5);

    let (status, page) = get_json(
        &app,
        &format!("/api/v1/agencies/{}/listings?status=active", f.agency),
        &member,
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(page["total"], 1);
    assert_eq!(page["listings"][0]["realtorName"], "Realtor realtor");

    let (status, series) = get_json(
        &app,
        &format!("/api/v1/agencies/{}/performance?interval=month", f.agency),
        &member,
    )
    .await;
    assert_eq!(status, 200);
    let created: i64 = series
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["listings"].as_i64().unwrap())
        .sum();
    assert_eq!(created, 2);

    let (status, rstats) = get_json(
        &app,
        &format!(
            "/api/v1/agencies/{}/members/{}/stats",
            f.agency, f.realtor_member
        ),
        &member,
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(rstats["closedDeals"], 1);

    let outsider = mint_token(f.outsider);
    let (status, _) = get_json(
        &app,
        &format!("/api/v1/agencies/{}/stats", f.agency),
        &outsider,
    )
    .await;
    assert_eq!(status, 403, "non-members must not read agency stats");
    let (status, _) = get_json(
        &app,
        &format!("/api/v1/agencies/{}/performance?interval=year", f.agency),
        &member,
    )
    .await;
    assert_eq!(status, 400);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn member_management_requires_manager_and_keeps_an_owner(pool: PgPool) {
    let f = fixture(&pool).await;
    let app = app(pool.clone());
    let owner = mint_token(f.owner);
    let realtor = mint_token(f.realtor);
    let member_path = |id: Uuid| format!("/api/v1/agencies/{}/members/{id}", f.agency);

    // A plain realtor cannot manage members.
    let status = send_json(
        &app,
        Method::PUT,
        &member_path(f.owner_member),
        Some(&realtor),
        json!({ "role": "realtor" }),
    )
    .await;
    assert_eq!(status, 403);

    // The only owner cannot demote themselves.
    let status = send_json(
        &app,
        Method::PUT,
        &member_path(f.owner_member),
        Some(&owner),
        json!({ "role": "realtor" }),
    )
    .await;
    assert_eq!(status, 400);

    let status = send_json(
        &app,
        Method::PUT,
        &member_path(f.realtor_member),
        Some(&owner),
        json!({ "role": "manager" }),
    )
    .await;
    assert_eq!(status, 200);

    let status = send_json(
        &app,
        Method::PUT,
        &member_path(f.realtor_member),
        Some(&owner),
        json!({ "role": "boss" }),
    )
    .await;
    assert_eq!(status, 400);

    let status = send(
        &app,
        Method::DELETE,
        &member_path(f.realtor_member),
        Some(&owner),
    )
    .await;
    assert_eq!(status, 204);
    let active: bool =
        sqlx::query_scalar("SELECT is_active FROM reality_agency_members WHERE id = $1")
            .bind(f.realtor_member)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(!active, "removed member is kept as inactive");

    let status = send(
        &app,
        Method::DELETE,
        &member_path(Uuid::new_v4()),
        Some(&owner),
    )
    .await;
    assert_eq!(status, 404);
}
