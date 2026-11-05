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
