//! Regression test: `PortfolioAnalyticsRepository` alert-rule and alert
//! queries used columns (category, operator, scope, notify_roles, ...,
//! current_value, is_read, is_resolved, ...) that migration 00091 never
//! created, so creating or listing alert rules and alerts returned 500.
//! Migration 00242 aligns the schema with the model.

use db::models::portfolio_analytics::{
    BenchmarkCategory, ComparisonScope, CreateAlertRule, UpdateAlertRule,
};
use db::repositories::PortfolioAnalyticsRepository;
use rust_decimal::Decimal;
use sqlx::PgPool;

use crate::common::{seed_org, set_ctx};

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn alert_rule_crud_round_trips(pool: PgPool) {
    set_ctx(&pool, None, None, true).await;
    let org = seed_org(&pool, "alert-rules").await;
    let repo = PortfolioAnalyticsRepository::new(pool.clone());

    let rule = repo
        .create_alert_rule(
            org,
            CreateAlertRule {
                name: "Low occupancy".into(),
                description: None,
                metric_name: "occupancy_rate".into(),
                category: BenchmarkCategory::Occupancy,
                operator: "<".into(),
                threshold_value: Decimal::new(80, 0),
                scope: ComparisonScope::Portfolio,
                building_id: None,
                notify_roles: vec!["manager".into()],
                notify_users: vec![],
                notification_frequency: "daily".into(),
            },
        )
        .await
        .expect("create alert rule");
    assert_eq!(rule.notify_roles, vec!["manager".to_string()]);
    assert_eq!(rule.trigger_count, 0);
    assert!(rule.is_active);

    let updated = repo
        .update_alert_rule(
            rule.id,
            org,
            UpdateAlertRule {
                name: None,
                description: None,
                threshold_value: Some(Decimal::new(75, 0)),
                operator: None,
                notify_roles: None,
                notify_users: None,
                notification_frequency: None,
                is_active: Some(false),
            },
        )
        .await
        .expect("update alert rule")
        .expect("rule exists");
    assert_eq!(updated.threshold_value, Decimal::new(75, 0));

    let all = repo.list_alert_rules(org, false).await.expect("list rules");
    assert_eq!(all.len(), 1);
    let active = repo.list_alert_rules(org, true).await.expect("list active");
    assert!(active.is_empty());
    assert!(repo.delete_alert_rule(rule.id, org).await.unwrap());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn alert_list_and_stats_query_existing_columns(pool: PgPool) {
    set_ctx(&pool, None, None, true).await;
    let org = seed_org(&pool, "alert-list").await;
    let repo = PortfolioAnalyticsRepository::new(pool.clone());
    let alerts = repo
        .list_alerts(org, true, true, 10)
        .await
        .expect("list alerts");
    assert!(alerts.is_empty());
    let stats = repo.get_alert_stats(org).await.expect("alert stats");
    assert_eq!(stats.total_alerts, 0);
}

/// Deleting a rule sets `portfolio_alerts.rule_id` to NULL; listing and
/// fetching such alerts must still decode (the model fields are Option).
/// Also covers `alert_type` having a default, so an alert can be inserted
/// with only the model's columns.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn alerts_of_a_deleted_rule_still_list(pool: PgPool) {
    set_ctx(&pool, None, None, true).await;
    let org = seed_org(&pool, "alert-orphan").await;
    let repo = PortfolioAnalyticsRepository::new(pool.clone());
    let rule = repo
        .create_alert_rule(
            org,
            CreateAlertRule {
                name: "Occupancy".into(),
                description: None,
                metric_name: "occupancy_rate".into(),
                category: BenchmarkCategory::Occupancy,
                operator: "<".into(),
                threshold_value: Decimal::new(80, 0),
                scope: ComparisonScope::Portfolio,
                building_id: None,
                notify_roles: vec![],
                notify_users: vec![],
                notification_frequency: "immediate".into(),
            },
        )
        .await
        .expect("create rule");

    let alert_id: uuid::Uuid = sqlx::query_scalar(
        "INSERT INTO portfolio_alerts \
             (organization_id, rule_id, severity, title, metric_name, current_value, threshold_value) \
         VALUES ($1, $2, 'warning', 'Low occupancy', 'occupancy_rate', 72, 80) RETURNING id",
    )
    .bind(org)
    .bind(rule.id)
    .fetch_one(&pool)
    .await
    .expect("insert alert without alert_type");
    // A legacy-style alert with the nullable columns left NULL.
    sqlx::query(
        "INSERT INTO portfolio_alerts (organization_id, severity, title) \
         VALUES ($1, 'info', 'Legacy')",
    )
    .bind(org)
    .execute(&pool)
    .await
    .expect("insert legacy alert");

    assert!(repo.delete_alert_rule(rule.id, org).await.unwrap());

    let alerts = repo
        .list_alerts(org, false, false, 10)
        .await
        .expect("list alerts");
    assert_eq!(alerts.len(), 2);
    assert!(alerts.iter().all(|a| a.rule_id.is_none()));
    let fetched = repo
        .get_alert(alert_id, org)
        .await
        .expect("get alert")
        .expect("alert exists");
    assert_eq!(fetched.rule_id, None);
    assert_eq!(fetched.metric_name.as_deref(), Some("occupancy_rate"));
    assert_eq!(fetched.threshold_value, Some(Decimal::new(80, 0)));
    assert_eq!(fetched.message, None);
}

/// Legacy word operators (00091 `threshold_operator`) are mapped to the
/// symbols the model uses. The migration is idempotent, so re-applying it
/// over a legacy-shaped row exercises the backfill.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn legacy_operator_words_map_to_symbols(pool: PgPool) {
    set_ctx(&pool, None, None, true).await;
    let org = seed_org(&pool, "alert-legacy-op").await;
    for op in ["gt", "lt", "gte", "lte", "eq"] {
        sqlx::query(
            "INSERT INTO portfolio_alert_rules \
                 (organization_id, name, metric_name, threshold_operator) \
             VALUES ($1, $2, 'occupancy_rate', $2)",
        )
        .bind(org)
        .bind(op)
        .execute(&pool)
        .await
        .expect("insert legacy rule");
    }
    sqlx::raw_sql(include_str!(
        "../../migrations/00242_reconcile_portfolio_alert_rules.sql"
    ))
    .execute(&pool)
    .await
    .expect("re-apply 00242");

    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT name::text, operator::text FROM portfolio_alert_rules \
         WHERE organization_id = $1 ORDER BY name",
    )
    .bind(org)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        rows,
        vec![
            ("eq".to_string(), "=".to_string()),
            ("gt".to_string(), ">".to_string()),
            ("gte".to_string(), ">=".to_string()),
            ("lt".to_string(), "<".to_string()),
            ("lte".to_string(), "<=".to_string()),
        ]
    );
}
