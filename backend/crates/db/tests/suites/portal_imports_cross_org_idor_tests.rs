//! Cross-tenant IDOR regression tests for reality-portal import jobs and feed
//! subscriptions (PAP-142).
//!
//! Before the fix, the by-id repo methods keyed only on `id`, so any
//! authenticated portal user could read or mutate another user's import job
//! (`get/update/start/cancel_import_job`) or another agency's feed
//! (`get/update_feed_subscription`, `trigger_feed_sync`) by guessing the id.
//! The handlers in `reality-server/src/routes/imports.rs` now thread the
//! verified `principal.user_id` (import jobs key on `user_id`; feeds key on
//! `agency_id`) and the repo methods scope the WHERE clause accordingly.
//!
//! The CI pool runs as a superuser that bypasses FORCE RLS, so these tests
//! exercise the application-layer ownership keying directly — exactly the gap
//! RLS would not catch in CI.
//!
//! Run with:
//! ```bash
//! cargo test -p db --test portal_imports_cross_org_idor_tests
//! ```

// NB: the reality-portal import-job models are re-exported under `Portal`-prefixed
// aliases (`db::models::CreateImportJob` is a *different* struct from another domain).
use db::models::{
    CreateFeedSubscription, CreatePortalImportJob, UpdateFeedSubscription, UpdatePortalImportJob,
};
use db::repositories::RealityPortalRepository;
use sqlx::PgPool;
use uuid::Uuid;

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'test_hash', 'Portal User', 'active', NOW(), 'public')
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

async fn seed_agency(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO reality_agencies (name, slug, email, status)
        VALUES ($1, $2, $3, 'verified')
        RETURNING id
        "#,
    )
    .bind(format!("Agency {slug}"))
    .bind(slug)
    .bind(format!("{slug}@agency.test"))
    .fetch_one(pool)
    .await
    .expect("seed agency")
}

#[sqlx::test]
async fn import_job_by_id_is_scoped_to_owning_user(pool: PgPool) {
    let user_a = seed_user(&pool, "import-a@idor.test").await;
    let user_b = seed_user(&pool, "import-b@idor.test").await;

    let repo = RealityPortalRepository::new(pool.clone());

    // user_a creates an import job.
    let job = repo
        .create_import_job(
            user_a,
            CreatePortalImportJob {
                agency_id: None,
                source_type: "csv".to_string(),
                source_url: None,
                source_filename: Some("a.csv".to_string()),
            },
        )
        .await
        .expect("create import job");

    // --- user_b must NOT be able to read user_a's job by id ---
    let leaked = repo.get_import_job(job.id, user_b).await.expect("get call");
    assert!(
        leaked.is_none(),
        "user_b must not read user_a's import job (IDOR PAP-142)"
    );

    // --- user_b must NOT be able to mutate it ---
    let updated = repo
        .update_import_job(
            job.id,
            user_b,
            UpdatePortalImportJob {
                source_url: Some("https://evil.test".to_string()),
                source_filename: None,
            },
        )
        .await;
    assert!(
        matches!(updated, Err(sqlx::Error::RowNotFound)),
        "user_b must not update user_a's import job (IDOR PAP-142), got {updated:?}"
    );

    let started = repo.start_import_job(job.id, user_b).await;
    assert!(
        matches!(started, Err(sqlx::Error::RowNotFound)),
        "user_b must not start user_a's import job (IDOR PAP-142), got {started:?}"
    );

    let cancelled = repo.cancel_import_job(job.id, user_b).await;
    assert!(
        matches!(cancelled, Err(sqlx::Error::RowNotFound)),
        "user_b must not cancel user_a's import job (IDOR PAP-142), got {cancelled:?}"
    );

    // Row must be untouched by the IDOR attempts.
    let (status, source_filename): (String, Option<String>) =
        sqlx::query_as("SELECT status, source_filename FROM portal_import_jobs WHERE id = $1")
            .bind(job.id)
            .fetch_one(&pool)
            .await
            .expect("select job");
    assert_eq!(status, "pending", "status must be unchanged");
    assert_eq!(
        source_filename.as_deref(),
        Some("a.csv"),
        "source_filename must be unchanged"
    );

    // --- the owner still has full access ---
    let owned = repo
        .get_import_job(job.id, user_a)
        .await
        .expect("owner get");
    assert!(owned.is_some(), "owner must read its own import job");

    let started = repo
        .start_import_job(job.id, user_a)
        .await
        .expect("owner start");
    assert_eq!(started.status, "processing", "owner must start its own job");
}

#[sqlx::test]
async fn feed_subscription_by_id_is_scoped_to_owning_agency(pool: PgPool) {
    let agency_a = seed_agency(&pool, "feed-idor-a").await;
    let agency_b = seed_agency(&pool, "feed-idor-b").await;

    let repo = RealityPortalRepository::new(pool.clone());

    // agency_a creates a feed subscription.
    let feed = repo
        .create_feed_subscription(
            agency_a,
            CreateFeedSubscription {
                name: "A feed".to_string(),
                feed_url: "https://a.test/feed.xml".to_string(),
                feed_type: None,
                sync_interval: None,
            },
        )
        .await
        .expect("create feed");

    // --- agency_b must NOT read agency_a's feed by id ---
    let leaked = repo
        .get_feed_subscription(feed.id, agency_b)
        .await
        .expect("get call");
    assert!(
        leaked.is_none(),
        "agency_b must not read agency_a's feed (IDOR PAP-142)"
    );

    // --- agency_b must NOT mutate it ---
    let updated = repo
        .update_feed_subscription(
            feed.id,
            agency_b,
            UpdateFeedSubscription {
                name: Some("hijacked".to_string()),
                feed_url: None,
                feed_type: None,
                sync_interval: None,
                is_active: Some(false),
            },
        )
        .await;
    assert!(
        matches!(updated, Err(sqlx::Error::RowNotFound)),
        "agency_b must not update agency_a's feed (IDOR PAP-142), got {updated:?}"
    );

    let synced = repo.trigger_feed_sync(feed.id, agency_b).await;
    assert!(
        matches!(synced, Err(sqlx::Error::RowNotFound)),
        "agency_b must not sync agency_a's feed (IDOR PAP-142), got {synced:?}"
    );

    // Row must be untouched.
    let name: String = sqlx::query_scalar("SELECT name FROM feed_subscriptions WHERE id = $1")
        .bind(feed.id)
        .fetch_one(&pool)
        .await
        .expect("select feed");
    assert_eq!(
        name, "A feed",
        "feed name must be unchanged after IDOR attempt"
    );

    // --- the owning agency still has full access ---
    let owned = repo
        .get_feed_subscription(feed.id, agency_a)
        .await
        .expect("owner get");
    assert!(owned.is_some(), "owner agency must read its own feed");

    let synced = repo
        .trigger_feed_sync(feed.id, agency_a)
        .await
        .expect("owner sync");
    assert!(
        synced.last_sync_at.is_some(),
        "owner agency must sync its own feed"
    );
}
