//! Happy-path status-code tests for the realtors surface
//! (`reality-server/src/routes/realtors.rs`).
//!
//! Focused regression coverage for the OpenAPI-vs-runtime divergence fixed in
//! `code-review-reality-server-create-handlers-return-200-not-201`: the
//! `create_profile` and `respond_to_inquiry` POST handlers document
//! `201 Created` in their utoipa annotations but previously returned `200 OK`.
//!
//! Auth model: both endpoints use `RequestPrincipal`. On the tenant-less test
//! router a `public`/`staff` principal is rejected ("tenant not resolved"), so
//! the authenticated user is seeded with `principal_kind = 'platform'`, which
//! the extractor admits with `effective_org = None`.

use crate::common::{make_app_state, mint_token, send_json};
use axum::{http::Method, Router};
use reality_server::routes;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

fn realtors_router(pool: PgPool) -> Router {
    let state = make_app_state(pool);
    Router::new()
        .nest("/api/v1/realtors", routes::realtors::router())
        .with_state(state)
}

async fn seed_user(pool: &PgPool, tag: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'hash', $2, 'active', NOW(), 'platform')
        RETURNING id
        "#,
    )
    .bind(format!("{tag}@realtors-hp.test"))
    .bind(format!("RealtorsHP {tag}"))
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("seed_user({tag}): {e}"))
}

/// Seed an `active` listing owned by `owner` and return its id.
async fn seed_listing(pool: &PgPool, tag: &str, owner: Uuid) -> Uuid {
    let org = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO organizations (name, slug, contact_email, status)
        VALUES ($1, $2, $3, 'active')
        RETURNING id
        "#,
    )
    .bind(format!("Realtor Org {tag}"))
    .bind(format!("realtor-org-{tag}"))
    .bind(format!("{tag}@realtor-org.test"))
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("seed_org({tag}): {e}"));

    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO listings
            (organization_id, created_by, status, transaction_type,
             title, property_type, price, currency,
             street, city, postal_code, country)
        VALUES ($1, $2, 'active', 'sale',
                'Realtor Listing', 'apartment', 200000.00, 'EUR',
                'Test Street 7', 'Bratislava', '81101', 'SK')
        RETURNING id
        "#,
    )
    .bind(org)
    .bind(owner)
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("seed_listing({tag}): {e}"))
}

/// Seed an inquiry addressed to `realtor` (the authenticated caller).
async fn seed_inquiry(pool: &PgPool, listing_id: Uuid, realtor: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO listing_inquiries
            (listing_id, realtor_id, name, email, message)
        VALUES ($1, $2, 'Buyer', 'buyer@realtors-hp.test', 'Is this available?')
        RETURNING id
        "#,
    )
    .bind(listing_id)
    .bind(realtor)
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("seed_inquiry: {e}"))
}

// ── create_profile (POST /profile) ───────────────────────────────────────────

// Regression: the utoipa contract documents `201 Created` for this
// profile-creation POST; the handler must agree (it previously returned 200).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_profile_returns_201(pool: PgPool) {
    let user = seed_user(&pool, "create-profile").await;
    let token = mint_token(user);
    let app = realtors_router(pool);
    let status = send_json(
        &app,
        Method::POST,
        "/api/v1/realtors/profile",
        Some(&token),
        json!({ "bio": "Seasoned agent", "phone": "+421900000000" }),
    )
    .await;
    assert_eq!(status, 201, "create_profile must return 201 Created");
}

// ── respond_to_inquiry (POST /inquiries/{id}/respond) ────────────────────────

// Regression: the utoipa contract documents `201 Created` for this
// message-creation POST; the handler must agree (it previously returned 200).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn respond_to_inquiry_returns_201(pool: PgPool) {
    let realtor = seed_user(&pool, "respond-realtor").await;
    let listing_id = seed_listing(&pool, "respond", realtor).await;
    let inquiry_id = seed_inquiry(&pool, listing_id, realtor).await;
    let token = mint_token(realtor);
    let app = realtors_router(pool);
    let status = send_json(
        &app,
        Method::POST,
        &format!("/api/v1/realtors/inquiries/{inquiry_id}/respond"),
        Some(&token),
        json!({ "message": "Yes, it is still available." }),
    )
    .await;
    assert_eq!(status, 201, "respond_to_inquiry must return 201 Created");
}
