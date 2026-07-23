//! Repository round-trip guard for the `mark_favorite_alert_read` idempotency
//! contract (#2050, follow-up to #1999 finding-2 / PR #2043).
//!
//! Why this test exists
//! --------------------
//! PR #2043 remediated #1999 finding-2 ("the idempotency test was `#[ignore]`d")
//! by adding a *pure unit test* of `mark_read_outcome_status`
//! (`reality-server/src/routes/favorites.rs`), which only exercises the
//! `FavoriteAlertReadOutcome -> StatusCode` **mapping**. The behavior the finding
//! was actually about — that the *repository* returns `AlreadyRead` (not
//! `NotFound`) when a client re-marks an already-read alert — lives in the
//! `exists_count` / `flipped_count` CTE split in `mark_favorite_alert_read`
//! (`crates/db/src/repositories/reality_portal.rs`) and had **no running guard**:
//! the only integration test that covered it,
//! `mark_favorite_alert_read_is_idempotent` in reality-server, is still
//! `#[ignore]`d under the BIT-351 quarantine and never runs on the PR gate.
//!
//! This test gives that CTE branch a guard that actually runs on `cargo test -p
//! db` (i.e. the backend.yml gate, which has Postgres). It asserts the three-way
//! outcome **directly against the DB**, independent of the route mapping and of
//! the `FavoriteAlertWorker` — the pending alert is inserted straight into
//! `favorite_alert_queue` rather than produced via a price-drop worker tick, so a
//! regression in the CTE (e.g. dropping the `existing` branch so an already-read
//! row collapses to `NotFound`) fails here even while the mapping unit test still
//! passes.
//!
//! `#[sqlx::test]` connects as the Postgres SUPERUSER, which bypasses RLS
//! entirely — so `favorite_alert_queue`'s FORCE RLS policy does not hide rows and
//! no tenant context needs to be set for this behavioral (non-isolation) check.

use db::repositories::{FavoriteAlertReadOutcome, RealityPortalRepository};
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::seed_org;

async fn seed_portal_user(pool: &PgPool, email: &str) -> Uuid {
    // portal_users was merged into `users` (migration 00132); portal users are
    // rows with principal_kind='public'.
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'test_hash', 'Favorite User', 'active', NOW(), 'public')
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed portal user")
}

async fn seed_listing(pool: &PgPool, org_id: Uuid, created_by: Uuid, title: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO listings (
            organization_id, created_by, transaction_type, title, property_type,
            street, city, postal_code, country, price, status
        )
        VALUES (
            $1, $2, 'sale', $3, 'apartment',
            'Main St', 'Bratislava', '81101', 'SK', 100000, 'active'
        )
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(created_by)
    .bind(title)
    .fetch_one(pool)
    .await
    .expect("seed listing")
}

/// Insert one `pending` price-change alert straight into the queue, bypassing the
/// `FavoriteAlertWorker`, and return its id.
async fn seed_pending_alert(
    pool: &PgPool,
    favorite_id: Uuid,
    user_id: Uuid,
    listing_id: Uuid,
) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO favorite_alert_queue (
            favorite_id, user_id, listing_id, alert_type,
            old_price, new_price, currency, change_percentage, status
        )
        VALUES ($1, $2, $3, 'price_change', 100000, 80000, 'EUR', -20.00, 'pending')
        RETURNING id
        "#,
    )
    .bind(favorite_id)
    .bind(user_id)
    .bind(listing_id)
    .fetch_one(pool)
    .await
    .expect("seed pending alert")
}

/// pending -> `Flipped`, second mark -> `AlreadyRead`, unknown id -> `NotFound`.
///
/// Guards the `exists_count` / `flipped_count` CTE split directly, so the
/// AlreadyRead-vs-NotFound idempotency branch has a failing-on-regression guard
/// independent of the route's `FavoriteAlertReadOutcome -> StatusCode` mapping.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn mark_favorite_alert_read_repo_round_trip_is_idempotent(pool: PgPool) {
    let repo = RealityPortalRepository::new(pool.clone());

    let org_id = seed_org(&pool, "fav-idem-org").await;
    let user_id = seed_portal_user(&pool, "fav-idem@test.sk").await;
    let listing_id = seed_listing(&pool, org_id, user_id, "Idempotency flat").await;

    let favorite = repo
        .add_favorite(user_id, listing_id, None)
        .await
        .expect("add favorite");
    let alert_id = seed_pending_alert(&pool, favorite.id, user_id, listing_id).await;

    // First mark flips pending -> sent.
    assert_eq!(
        repo.mark_favorite_alert_read(alert_id, user_id)
            .await
            .expect("first mark"),
        FavoriteAlertReadOutcome::Flipped,
        "a pending alert must flip to Flipped on first mark",
    );

    // Second mark is idempotent — the row still exists for the user but is
    // already read, so it must be AlreadyRead (route -> 204), NOT NotFound.
    assert_eq!(
        repo.mark_favorite_alert_read(alert_id, user_id)
            .await
            .expect("second mark"),
        FavoriteAlertReadOutcome::AlreadyRead,
        "re-marking an already-read alert must be AlreadyRead, not NotFound",
    );

    // A genuinely-absent id is NotFound (route -> 404, no existence leak).
    assert_eq!(
        repo.mark_favorite_alert_read(Uuid::new_v4(), user_id)
            .await
            .expect("unknown id mark"),
        FavoriteAlertReadOutcome::NotFound,
        "an unknown alert id must be NotFound",
    );

    // An alert owned by a different user must also collapse to NotFound for the
    // wrong caller (no existence leak), and must remain markable by its owner.
    let other_user = seed_portal_user(&pool, "fav-idem-other@test.sk").await;
    let other_favorite = repo
        .add_favorite(other_user, listing_id, None)
        .await
        .expect("add favorite (other user)");
    let other_alert = seed_pending_alert(&pool, other_favorite.id, other_user, listing_id).await;

    assert_eq!(
        repo.mark_favorite_alert_read(other_alert, user_id)
            .await
            .expect("cross-user mark"),
        FavoriteAlertReadOutcome::NotFound,
        "another user's alert must be NotFound for the wrong caller",
    );
    assert_eq!(
        repo.mark_favorite_alert_read(other_alert, other_user)
            .await
            .expect("owner mark"),
        FavoriteAlertReadOutcome::Flipped,
        "the owning user must still be able to flip their own alert",
    );
}
