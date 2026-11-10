//! Gap audit #32: built-in automation templates rendered their English
//! `name` in every locale. Migration 00243 gives each system template a stable
//! `slug` that ppt-web translates by; the repository must expose it.

use db::repositories::AutomationRepository;
use sqlx::PgPool;

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn system_templates_expose_stable_slugs(pool: PgPool) {
    let repo = AutomationRepository::new(pool.clone());
    let templates = repo.list_templates().await.expect("list templates");

    let mut slugs: Vec<String> = templates
        .iter()
        .filter(|t| t.is_system)
        .map(|t| t.slug.clone().expect("system template has a slug"))
        .collect();
    slugs.sort();
    assert_eq!(
        slugs,
        vec![
            "high_priority_fault_escalation",
            "monthly_statement_generation",
            "overdue_payment_reminder",
            "weekly_maintenance_report",
        ]
    );

    let first = &templates[0];
    let fetched = repo
        .get_template(first.id)
        .await
        .expect("get template")
        .expect("template exists");
    assert_eq!(fetched.slug, first.slug);
}
