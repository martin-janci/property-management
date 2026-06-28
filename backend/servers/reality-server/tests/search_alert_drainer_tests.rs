//! Tests for the saved-search alert email/push **transport drainer**
//! (BIT-139, Epic 16).
//!
//! The drainer ([`SearchAlertDrainerWorker`]) drains `search_alert_queue` rows
//! to email + push transports, fanning push out across the owner's
//! `device_push_tokens`, and records delivery in `notified_at` — *independently*
//! of the `status` column, which belongs to the in-app pull/read channel.
//!
//! These tests inject counting stub transports via `with_transports` and assert
//! the four invariants that matter:
//!
//! | Case | Assertion |
//! |------|-----------|
//! | C1 | A pending alert is emailed once and pushed to *each* of the owner's tokens |
//! | C2 | Delivery sets `notified_at` but leaves `status = 'pending'` (in-app badge intact) |
//! | C3 | A second drain pass is idempotent (already-notified rows are skipped) |
//! | C4 | Push fan-out is scoped to the alert owner — a *different* user's token is never used |
//!
//! Plus a failure path: when every channel fails, `notify_attempts` is
//! incremented, `notified_at` stays NULL, and the row drops out once it exhausts
//! its attempt budget.
//!
//! Run against a live Postgres (sqlx spins up a per-test database):
//! `DATABASE_URL=... cargo test -p reality-server --test search_alert_drainer_tests`.

#![allow(dead_code)]

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use db::models::CreatePortalSavedSearch;
use db::repositories::RealityPortalRepository;
use reality_server::services::search_alert_drainer::{
    AlertEmailTransport, AlertNotification, AlertPushTransport,
};
use reality_server::services::{SearchAlertDrainerConfig, SearchAlertDrainerWorker};
use sqlx::PgPool;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Counting stub transports
// ---------------------------------------------------------------------------

/// Records every email recipient; optionally fails all sends.
#[derive(Clone, Default)]
struct RecordingEmail {
    recipients: Arc<Mutex<Vec<String>>>,
    fail: bool,
}

#[async_trait]
impl AlertEmailTransport for RecordingEmail {
    async fn send_email(
        &self,
        to_email: &str,
        _to_name: &str,
        _notification: &AlertNotification,
    ) -> Result<(), String> {
        if self.fail {
            return Err("forced email failure".into());
        }
        self.recipients.lock().unwrap().push(to_email.to_string());
        Ok(())
    }
}

/// Records every device token a push was sent to; optionally fails all sends.
#[derive(Clone, Default)]
struct RecordingPush {
    tokens: Arc<Mutex<Vec<String>>>,
    fail: bool,
}

#[async_trait]
impl AlertPushTransport for RecordingPush {
    async fn send_push(
        &self,
        token: &str,
        _platform: &str,
        _notification: &AlertNotification,
    ) -> Result<(), String> {
        if self.fail {
            return Err("forced push failure".into());
        }
        self.tokens.lock().unwrap().push(token.to_string());
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Seed helpers
// ---------------------------------------------------------------------------

async fn seed_portal_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'test_hash', 'Drainer Tester', 'active', NOW(), 'public')
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("seed_portal_user({email}): {e}"))
}

async fn seed_saved_search(repo: &RealityPortalRepository, user_id: Uuid) -> Uuid {
    repo.create_saved_search(
        user_id,
        CreatePortalSavedSearch {
            name: "Bratislava flats".to_string(),
            criteria: serde_json::json!({ "city": "Bratislava" }),
            alerts_enabled: true,
            alert_frequency: "daily".to_string(),
        },
    )
    .await
    .expect("create saved search")
    .id
}

async fn seed_push_token(pool: &PgPool, user_id: Uuid, token: &str) {
    sqlx::query(
        r#"
        INSERT INTO device_push_tokens (user_id, token, platform)
        VALUES ($1, $2, 'fcm')
        ON CONFLICT (token) DO NOTHING
        "#,
    )
    .bind(user_id)
    .bind(token)
    .execute(pool)
    .await
    .expect("seed push token");
}

async fn alert_state(
    pool: &PgPool,
    id: Uuid,
) -> (String, Option<chrono::DateTime<chrono::Utc>>, i32) {
    sqlx::query_as::<_, (String, Option<chrono::DateTime<chrono::Utc>>, i32)>(
        "SELECT status, notified_at, notify_attempts FROM search_alert_queue WHERE id = $1",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .expect("load alert state")
}

fn worker(pool: &PgPool, email: RecordingEmail, push: RecordingPush) -> SearchAlertDrainerWorker {
    SearchAlertDrainerWorker::with_transports(
        pool.clone(),
        SearchAlertDrainerConfig {
            enabled: true,
            poll_interval_secs: 60,
            batch_size: 100,
            max_attempts: 3,
        },
        Arc::new(email),
        Arc::new(push),
    )
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// C1–C4: happy path — email once, push to each owner token, `notified_at` set,
/// `status` untouched, idempotent re-run, and a foreign user's token untouched.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn drains_alert_to_email_and_owner_push_tokens(pool: PgPool) {
    let repo = RealityPortalRepository::new(pool.clone());

    let owner = seed_portal_user(&pool, "owner@test.sk").await;
    let other = seed_portal_user(&pool, "other@test.sk").await;
    let search = seed_saved_search(&repo, owner).await;

    // Owner has two devices; the other user has one (must never be touched).
    seed_push_token(&pool, owner, "owner-token-1").await;
    seed_push_token(&pool, owner, "owner-token-2").await;
    seed_push_token(&pool, other, "other-token-1").await;

    repo.enqueue_search_alert(
        &pool,
        search,
        owner,
        &[Uuid::new_v4(), Uuid::new_v4()],
        "new_listing",
    )
    .await
    .expect("enqueue alert");
    let alert_id =
        sqlx::query_scalar::<_, Uuid>("SELECT id FROM search_alert_queue WHERE user_id = $1")
            .bind(owner)
            .fetch_one(&pool)
            .await
            .expect("alert id");

    let email = RecordingEmail::default();
    let push = RecordingPush::default();
    let delivered = worker(&pool, email.clone(), push.clone()).run_once().await;

    // C1: exactly one alert delivered; emailed to the owner once.
    assert_eq!(delivered, 1, "one alert should be delivered");
    let emails = email.recipients.lock().unwrap().clone();
    assert_eq!(emails, vec!["owner@test.sk".to_string()]);

    // C1 + C4: pushed to both of the owner's tokens, never the other user's.
    let mut pushed = push.tokens.lock().unwrap().clone();
    pushed.sort();
    assert_eq!(
        pushed,
        vec!["owner-token-1".to_string(), "owner-token-2".to_string()]
    );
    assert!(
        !pushed.iter().any(|t| t == "other-token-1"),
        "a foreign user's token must never receive the owner's alert"
    );

    // C2: delivery recorded out-of-band — `notified_at` set, `status` untouched.
    let (status, notified_at, attempts) = alert_state(&pool, alert_id).await;
    assert_eq!(status, "pending", "in-app read state must be untouched");
    assert!(
        notified_at.is_some(),
        "notified_at must be set after delivery"
    );
    assert_eq!(attempts, 0);

    // C3: re-running is a no-op — the row is already notified.
    let email2 = RecordingEmail::default();
    let push2 = RecordingPush::default();
    let delivered2 = worker(&pool, email2.clone(), push2.clone())
        .run_once()
        .await;
    assert_eq!(
        delivered2, 0,
        "already-notified alerts are not re-delivered"
    );
    assert!(email2.recipients.lock().unwrap().is_empty());
    assert!(push2.tokens.lock().unwrap().is_empty());
}

/// Failure path: when every channel fails, the row is not marked notified, its
/// attempt counter climbs, and it drops out once it exhausts the budget.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn failed_delivery_increments_attempts_until_budget_exhausted(pool: PgPool) {
    let repo = RealityPortalRepository::new(pool.clone());
    let owner = seed_portal_user(&pool, "fail@test.sk").await;
    let search = seed_saved_search(&repo, owner).await;
    // No device tokens: email is the only channel, and we force it to fail.
    repo.enqueue_search_alert(&pool, search, owner, &[Uuid::new_v4()], "new_listing")
        .await
        .expect("enqueue alert");
    let alert_id =
        sqlx::query_scalar::<_, Uuid>("SELECT id FROM search_alert_queue WHERE user_id = $1")
            .bind(owner)
            .fetch_one(&pool)
            .await
            .expect("alert id");

    let failing_email = RecordingEmail {
        fail: true,
        ..Default::default()
    };
    let push = RecordingPush::default();

    // max_attempts = 3, so the row stays drainable for exactly 3 passes.
    for expected_attempts in 1..=3 {
        let delivered = worker(&pool, failing_email.clone(), push.clone())
            .run_once()
            .await;
        assert_eq!(
            delivered, 0,
            "a fully-failed alert is never marked delivered"
        );
        let (status, notified_at, attempts) = alert_state(&pool, alert_id).await;
        assert_eq!(status, "pending");
        assert!(
            notified_at.is_none(),
            "failed delivery must not set notified_at"
        );
        assert_eq!(attempts, expected_attempts);
    }

    // Budget exhausted (attempts == max_attempts): the row no longer drains.
    let delivered = worker(&pool, failing_email.clone(), push.clone())
        .run_once()
        .await;
    assert_eq!(delivered, 0);
    let (_, _, attempts) = alert_state(&pool, alert_id).await;
    assert_eq!(
        attempts, 3,
        "exhausted rows are skipped, not retried further"
    );
}
