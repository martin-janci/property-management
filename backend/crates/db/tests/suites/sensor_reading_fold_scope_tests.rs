//! Behavioral test for folding the IoT org-scope guard INTO the reading-ingest
//! writes (issue #3305, follow-up to PR #3289).
//!
//! `add_reading` / `add_batch_readings` used to run a separate
//! `ensure_in_org` (`SELECT EXISTS(... sensors WHERE id=$1 AND
//! organization_id=$2)`) before every write — an extra DB round-trip on the
//! hot single-ingest path plus a check-then-insert TOCTOU window. The fix folds
//! the guard into the INSERT itself (`INSERT ... SELECT ... WHERE EXISTS(...)`),
//! so a cross-org or missing sensor writes zero rows and is reported back as
//! `None` (→ 404) in a single statement.
//!
//! This test runs as the Postgres superuser, which bypasses RLS entirely. That
//! is deliberate: it isolates the *explicit* `organization_id` predicate folded
//! into the write from the RLS policy — the "holds even when the connection's
//! role bypasses RLS" guarantee the IoT scope helpers document. On the pre-fix
//! code the writes took no `org_id` and inserted unconditionally, so the
//! cross-org cases below would have written rows.

use db::models::{CreateSensorReading, SingleReading};
use db::repositories::SensorRepository;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{seed_org, set_ctx};

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'test_hash', 'Fold User', 'active', NOW(), 'public')
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

async fn seed_building(pool: &PgPool, org_id: Uuid, street: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO buildings (organization_id, street, city, postal_code)
        VALUES ($1, $2, 'Bratislava', '81101')
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(street)
    .fetch_one(pool)
    .await
    .expect("seed building")
}

async fn seed_sensor(pool: &PgPool, org_id: Uuid, building_id: Uuid, created_by: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO sensors (organization_id, building_id, name, sensor_type, created_by)
        VALUES ($1, $2, 'fold-temp', 'temperature', $3)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(building_id)
    .bind(created_by)
    .fetch_one(pool)
    .await
    .expect("seed sensor")
}

async fn count_readings(pool: &PgPool, sensor_id: Uuid) -> i64 {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM sensor_readings WHERE sensor_id = $1")
        .bind(sensor_id)
        .fetch_one(pool)
        .await
        .expect("count readings")
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn reading_writes_fold_org_scope_into_insert(pool: PgPool) {
    set_ctx(&pool, None, None, true).await;

    let org_a = seed_org(&pool, "fold-a").await;
    let org_b = seed_org(&pool, "fold-b").await;
    let user_a = seed_user(&pool, "a@fold.test").await;
    let bldg_a = seed_building(&pool, org_a, "1 Fold St").await;
    let sensor_a = seed_sensor(&pool, org_a, bldg_a, user_a).await;

    let repo = SensorRepository::new();
    let mut conn = pool.acquire().await.expect("acquire connection");

    let reading = |sensor_id: Uuid| CreateSensorReading {
        sensor_id,
        value: 21.5,
        unit: "C".to_string(),
        quality: None,
        raw_data: None,
        timestamp: None,
    };

    // --- single ingest, correct org → inserted. ---
    let ok = repo
        .create_reading(&mut conn, org_a, reading(sensor_a))
        .await
        .expect("create_reading (own org)");
    assert!(
        ok.is_some(),
        "a reading for an in-org sensor must be inserted"
    );
    assert_eq!(count_readings(&pool, sensor_a).await, 1);

    // --- single ingest, wrong org → zero rows, reported as None (→ 404). ---
    let cross = repo
        .create_reading(&mut conn, org_b, reading(sensor_a))
        .await
        .expect("create_reading (cross org must not error)");
    assert!(
        cross.is_none(),
        "a reading for a sensor outside the caller's org must NOT be inserted"
    );
    assert_eq!(
        count_readings(&pool, sensor_a).await,
        1,
        "the cross-org single ingest must not have written a row"
    );

    // --- batch ingest, correct org → inserted. ---
    let single = SingleReading {
        value: 22.0,
        unit: "C".to_string(),
        quality: None,
        timestamp: chrono::Utc::now(),
    };
    let ok = repo
        .create_batch_readings(&mut conn, sensor_a, org_a, vec![single.clone()])
        .await
        .expect("create_batch_readings (own org)");
    assert_eq!(ok, Some(1), "an in-org batch must insert its rows");
    assert_eq!(count_readings(&pool, sensor_a).await, 2);

    // --- batch ingest, wrong org → zero rows, reported as None (→ 404). ---
    let cross = repo
        .create_batch_readings(&mut conn, sensor_a, org_b, vec![single])
        .await
        .expect("create_batch_readings (cross org must not error)");
    assert!(
        cross.is_none(),
        "a batch for a sensor outside the caller's org must NOT be inserted"
    );
    assert_eq!(
        count_readings(&pool, sensor_a).await,
        2,
        "the cross-org batch ingest must not have written rows"
    );
}
