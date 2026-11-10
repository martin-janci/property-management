//! Regression guard for `FaultRepository::list` filters and sorting.
//!
//! Two defects lived in the dynamic SQL of this method:
//!
//! 1. The enum filters were written as `status = ANY($n)::fault_status[]`,
//!    which casts the boolean result instead of the bound array, so any
//!    `status` / `priority` / `category` filter failed in Postgres and the
//!    handler answered 500.
//! 2. `sort_by` / `sort_order` were interpolated into `ORDER BY` unchecked, so
//!    a query string such as `sort_by=(SELECT password_hash FROM users ...)`
//!    was executed as SQL.

use crate::common::seed_org;
use db::models::FaultListQuery;
use db::repositories::FaultRepository;
use sqlx::PgPool;
use uuid::Uuid;

async fn seed(pool: &PgPool, slug: &str) -> Uuid {
    let org = seed_org(pool, slug).await;
    let building: Uuid = sqlx::query_scalar(
        "INSERT INTO buildings (organization_id, street, city, postal_code)
         VALUES ($1, 'Test Street 1', 'Bratislava', '81101') RETURNING id",
    )
    .bind(org)
    .fetch_one(pool)
    .await
    .expect("seed building");
    let reporter: Uuid = sqlx::query_scalar(
        "INSERT INTO users (email, password_hash, name, status, email_verified_at)
         VALUES ($1, 'test_hash', 'Reporter', 'active', NOW()) RETURNING id",
    )
    .bind(format!("{slug}@reporters.test"))
    .fetch_one(pool)
    .await
    .expect("seed reporter");
    for (title, status, priority, category) in [
        ("B leak", "new", "high", "plumbing"),
        ("A light", "resolved", "low", "electrical"),
    ] {
        sqlx::query(
            "INSERT INTO faults (organization_id, building_id, reporter_id, title, description,
                                 status, priority, category)
             VALUES ($1, $2, $3, $4, 'desc', $5::fault_status, $6::fault_priority, $7::fault_category)",
        )
        .bind(org)
        .bind(building)
        .bind(reporter)
        .bind(title)
        .bind(status)
        .bind(priority)
        .bind(category)
        .execute(pool)
        .await
        .expect("seed fault");
    }
    org
}

#[sqlx::test(migrations = "./migrations")]
async fn list_filters_by_status_priority_and_category(pool: PgPool) {
    let org = seed(&pool, "fault-filter").await;
    let repo = FaultRepository::new(pool.clone());

    let by_status = repo
        .list(
            org,
            FaultListQuery {
                status: Some(vec!["new".to_string()]),
                ..Default::default()
            },
        )
        .await
        .expect("status filter must not fail");
    assert_eq!(by_status.len(), 1);
    assert_eq!(by_status[0].title, "B leak");

    let by_priority = repo
        .list(
            org,
            FaultListQuery {
                priority: Some(vec!["low".to_string(), "high".to_string()]),
                ..Default::default()
            },
        )
        .await
        .expect("priority filter must not fail");
    assert_eq!(by_priority.len(), 2);

    let by_category = repo
        .list(
            org,
            FaultListQuery {
                category: Some(vec!["electrical".to_string()]),
                ..Default::default()
            },
        )
        .await
        .expect("category filter must not fail");
    assert_eq!(by_category.len(), 1);
    assert_eq!(by_category[0].title, "A light");
}

#[sqlx::test(migrations = "./migrations")]
async fn list_sorts_by_allowlisted_columns_only(pool: PgPool) {
    let org = seed(&pool, "fault-sort").await;
    let repo = FaultRepository::new(pool.clone());

    let by_title = repo
        .list(
            org,
            FaultListQuery {
                sort_by: Some("title".to_string()),
                sort_order: Some("asc".to_string()),
                ..Default::default()
            },
        )
        .await
        .expect("sort by title");
    assert_eq!(by_title[0].title, "A light");

    // An injected expression must never be executed: it falls back to the
    // default ordering. `pg_sleep` in a scalar subquery would otherwise run
    // (and a multi-row subquery would raise), so success plus a fast return
    // proves the value never reached the SQL text.
    let started = std::time::Instant::now();
    let injected = repo
        .list(
            org,
            FaultListQuery {
                sort_by: Some("(SELECT 1 FROM pg_sleep(3))".to_string()),
                sort_order: Some("DESC, (SELECT password_hash FROM users)".to_string()),
                ..Default::default()
            },
        )
        .await
        .expect("injected sort must be ignored, not executed");
    assert_eq!(injected.len(), 2);
    assert!(
        started.elapsed() < std::time::Duration::from_secs(2),
        "injected sort expression was executed"
    );
}
