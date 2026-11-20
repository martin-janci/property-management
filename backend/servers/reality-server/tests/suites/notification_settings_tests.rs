//! Portal notification settings: push-token registration, per-topic
//! preferences, and dismissing a favorite alert.

use crate::common::{make_app_state, mint_token, send, send_json};
use axum::{http::Method, Router};
use reality_server::routes;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

fn app(pool: PgPool) -> Router {
    let state = make_app_state(pool);
    Router::new()
        .nest("/api/v1/users", routes::users::router())
        .nest("/api/v1/favorites", routes::favorites::router())
        .with_state(state)
}

async fn seed_public_user(pool: &PgPool, tag: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'hash', 'Notif User', 'active', NOW(), 'public')
        RETURNING id
        "#,
    )
    .bind(format!("{tag}@notif.test"))
    .fetch_one(pool)
    .await
    .expect("seed user")
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn push_token_register_and_unregister(pool: PgPool) {
    let user = seed_public_user(&pool, "push").await;
    let token = mint_token(user);
    let app = app(pool.clone());

    let bad = send_json(
        &app,
        Method::POST,
        "/api/v1/users/me/push-tokens",
        Some(&token),
        json!({ "token": "abc", "platform": "web" }),
    )
    .await;
    assert_eq!(bad, 400, "unknown platform is rejected");

    let anon = send_json(
        &app,
        Method::POST,
        "/api/v1/users/me/push-tokens",
        None,
        json!({ "token": "abc", "platform": "android" }),
    )
    .await;
    assert_eq!(anon, 401);

    for _ in 0..2 {
        let ok = send_json(
            &app,
            Method::POST,
            "/api/v1/users/me/push-tokens",
            Some(&token),
            json!({ "token": "fcm-token-1", "platform": "android", "device_id": "pixel" }),
        )
        .await;
        assert_eq!(ok, 200, "registration is idempotent");
    }
    let (count, platform): (i64, String) =
        sqlx::query_as("SELECT COUNT(*), MIN(platform) FROM device_push_tokens WHERE user_id = $1")
            .bind(user)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 1);
    assert_eq!(platform, "fcm");

    let removed = send(
        &app,
        Method::DELETE,
        "/api/v1/users/me/push-tokens/fcm-token-1",
        Some(&token),
    )
    .await;
    assert_eq!(removed, 204);
    let left: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM device_push_tokens WHERE user_id = $1")
            .bind(user)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(left, 0);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn notification_preferences_default_then_saved(pool: PgPool) {
    let user = seed_public_user(&pool, "prefs").await;
    let other = seed_public_user(&pool, "prefs-other").await;
    let token = mint_token(user);
    let app = app(pool.clone());

    let get = send(
        &app,
        Method::GET,
        "/api/v1/users/me/notification-preferences",
        Some(&token),
    )
    .await;
    assert_eq!(get, 200);

    let put = send_json(
        &app,
        Method::PUT,
        "/api/v1/users/me/notification-preferences",
        Some(&token),
        json!({
            "new_listings": false,
            "price_drops": true,
            "inquiry_responses": true,
            "listing_updates": false,
            "marketing": true
        }),
    )
    .await;
    assert_eq!(put, 200);

    let repo = db::repositories::RealityPortalRepository::new(pool.clone());
    let saved = repo.get_notification_preferences(user).await.unwrap();
    assert!(!saved.new_listings);
    assert!(!saved.listing_updates);
    assert!(saved.marketing);
    let untouched = repo.get_notification_preferences(other).await.unwrap();
    assert_eq!(
        untouched,
        db::models::PortalNotificationPreferences::default()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn delete_favorite_alert_is_owner_scoped(pool: PgPool) {
    let user = seed_public_user(&pool, "alert").await;
    let other = seed_public_user(&pool, "alert-other").await;
    let org = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO organizations (name, slug, contact_email, status)
         VALUES ('Notif Org', 'notif-org', 'org@notif.test', 'active') RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let listing = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO listings
            (organization_id, created_by, status, transaction_type, title, property_type,
             price, currency, street, city, postal_code, country)
        VALUES ($1, $2, 'active', 'sale', 'Alert Listing', 'apartment', 100000.00, 'EUR',
                'Street 1', 'Bratislava', '81101', 'SK')
        RETURNING id
        "#,
    )
    .bind(org)
    .bind(user)
    .fetch_one(&pool)
    .await
    .unwrap();
    let favorite = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO portal_favorites (user_id, listing_id) VALUES ($1, $2) RETURNING id",
    )
    .bind(user)
    .bind(listing)
    .fetch_one(&pool)
    .await
    .unwrap();
    let alert = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO favorite_alert_queue (favorite_id, user_id, listing_id, alert_type, old_price, new_price, currency)
        VALUES ($1, $2, $3, 'price_change', 100000, 90000, 'EUR')
        RETURNING id
        "#,
    )
    .bind(favorite)
    .bind(user)
    .bind(listing)
    .fetch_one(&pool)
    .await
    .unwrap();
    let app = app(pool.clone());

    let foreign = send(
        &app,
        Method::DELETE,
        &format!("/api/v1/favorites/alerts/{alert}"),
        Some(&mint_token(other)),
    )
    .await;
    assert_eq!(foreign, 404, "another user's alert must not be deletable");

    let own = send(
        &app,
        Method::DELETE,
        &format!("/api/v1/favorites/alerts/{alert}"),
        Some(&mint_token(user)),
    )
    .await;
    assert_eq!(own, 204);
}
