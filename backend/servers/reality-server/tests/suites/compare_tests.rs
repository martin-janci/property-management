//! Behaviour tests for GET /api/v1/compare, POST /api/v1/compare/{id},
//! DELETE /api/v1/compare/{id}.

use crate::common;

use axum::{http::Method, Router};
use reality_server::routes;
use sqlx::PgPool;
use uuid::Uuid;

fn compare_router(pool: PgPool) -> Router {
    common::ensure_test_env();
    let state = common::make_app_state(pool);
    Router::new()
        .nest("/api/v1/compare", routes::compare::router())
        .with_state(state)
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_compare_list_unauthenticated_returns_401(pool: PgPool) {
    let router = compare_router(pool);
    let status = common::send(&router, Method::GET, "/api/v1/compare", None).await;
    assert_eq!(status, axum::http::StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_compare_list_authenticated_returns_200(pool: PgPool) {
    let user_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO users (id, email, password_hash, name, principal_kind, status) \
         VALUES ($1, $2, 'x', 'Test User', 'platform', 'active')",
    )
    .bind(user_id)
    .bind(format!("compare-get-{}@test.internal", user_id))
    .execute(&pool)
    .await
    .expect("seed user");
    let router = compare_router(pool);
    let token = common::mint_token(user_id);
    let status = common::send(&router, Method::GET, "/api/v1/compare", Some(&token)).await;
    assert_eq!(status, axum::http::StatusCode::OK);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn add_to_compare_unauthenticated_returns_401(pool: PgPool) {
    let router = compare_router(pool);
    let listing_id = Uuid::new_v4();
    let status = common::send(
        &router,
        Method::POST,
        &format!("/api/v1/compare/{listing_id}"),
        None,
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn add_to_compare_unknown_listing_returns_404(pool: PgPool) {
    let user_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO users (id, email, password_hash, name, principal_kind, status) \
         VALUES ($1, $2, 'x', 'Test User', 'platform', 'active')",
    )
    .bind(user_id)
    .bind(format!("compare-add-{}@test.internal", user_id))
    .execute(&pool)
    .await
    .expect("seed user");
    let router = compare_router(pool);
    let token = common::mint_token(user_id);
    let listing_id = Uuid::new_v4();
    let status = common::send(
        &router,
        Method::POST,
        &format!("/api/v1/compare/{listing_id}"),
        Some(&token),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::NOT_FOUND);
}

/// Regression (TOCTOU): concurrent POSTs must never push the per-user compare
/// list past `MAX_COMPARE_LISTINGS` (4). Before the fix, `add_to_compare` read
/// `COUNT(*)` and then INSERTed in two separate statements on an autocommit
/// connection, so N simultaneous requests could all observe `count < 4` and
/// all insert — leaving up to N rows. The fix serializes per-user adds with a
/// transaction-scoped advisory lock, so the count check and insert are atomic
/// for a given user.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn add_to_compare_concurrent_never_exceeds_max(pool: PgPool) {
    use routes::compare::MAX_COMPARE_LISTINGS;

    let user_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO users (id, email, password_hash, name, principal_kind, status) \
         VALUES ($1, $2, 'x', 'Test User', 'platform', 'active')",
    )
    .bind(user_id)
    .bind(format!("compare-conc-{}@test.internal", user_id))
    .execute(&pool)
    .await
    .expect("seed user");

    // An org + owner for the listings FK chain.
    let org_id: Uuid = sqlx::query_scalar(
        "INSERT INTO organizations (name, slug, contact_email, status) \
         VALUES ('Cmp Org', $1, $2, 'active') RETURNING id",
    )
    .bind(format!("cmp-org-{}", user_id))
    .bind(format!("cmp-org-{}@test.internal", user_id))
    .fetch_one(&pool)
    .await
    .expect("seed org");

    // Seed more active listings than the cap so every concurrent request has a
    // distinct, valid listing to add (distinct listings avoid the unique
    // (user_id, listing_id) constraint masking the cap race).
    const CONCURRENCY: usize = 8;
    let mut listing_ids = Vec::with_capacity(CONCURRENCY);
    for i in 0..CONCURRENCY {
        let listing_id: Uuid = sqlx::query_scalar(
            "INSERT INTO listings \
             (organization_id, created_by, status, transaction_type, title, \
              property_type, price, currency, street, city, postal_code, country) \
             VALUES ($1, $1, 'active', 'sale', $2, 'apartment', 100000.00, 'EUR', \
                     'St 1', 'Bratislava', '81101', 'SK') RETURNING id",
        )
        .bind(org_id)
        .bind(format!("Cmp Listing {i}"))
        .fetch_one(&pool)
        .await
        .expect("seed listing");
        listing_ids.push(listing_id);
    }

    let router = compare_router(pool.clone());
    let token = common::mint_token(user_id);

    // Fire all adds concurrently.
    let mut handles = Vec::with_capacity(CONCURRENCY);
    for listing_id in listing_ids {
        let router = router.clone();
        let token = token.clone();
        handles.push(tokio::spawn(async move {
            common::send(
                &router,
                Method::POST,
                &format!("/api/v1/compare/{listing_id}"),
                Some(&token),
            )
            .await
        }));
    }

    let mut created = 0usize;
    for h in handles {
        let status = h.await.expect("task join");
        if status == axum::http::StatusCode::CREATED {
            created += 1;
        }
    }

    let final_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM compare_lists WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(&pool)
            .await
            .expect("count rows");

    assert!(
        final_count <= MAX_COMPARE_LISTINGS,
        "compare list exceeded cap under concurrency: {final_count} rows (max {MAX_COMPARE_LISTINGS})"
    );
    assert_eq!(
        created as i64, final_count,
        "number of 201 responses ({created}) must equal rows actually inserted ({final_count})"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn remove_from_compare_unauthenticated_returns_401(pool: PgPool) {
    let router = compare_router(pool);
    let listing_id = Uuid::new_v4();
    let status = common::send(
        &router,
        Method::DELETE,
        &format!("/api/v1/compare/{listing_id}"),
        None,
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn remove_from_compare_authenticated_not_in_list_returns_404(pool: PgPool) {
    let user_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO users (id, email, password_hash, name, principal_kind, status) \
         VALUES ($1, $2, 'x', 'Test User', 'platform', 'active')",
    )
    .bind(user_id)
    .bind(format!("compare-del-{}@test.internal", user_id))
    .execute(&pool)
    .await
    .expect("seed user");
    let router = compare_router(pool);
    let token = common::mint_token(user_id);
    let listing_id = Uuid::new_v4();
    let status = common::send(
        &router,
        Method::DELETE,
        &format!("/api/v1/compare/{listing_id}"),
        Some(&token),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::NOT_FOUND);
}
