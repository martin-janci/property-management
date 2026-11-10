//! Regression test: `ppt-seed --force` failed with
//! `audit_logs is append-only: UPDATE rejected` because deleting the seed
//! users/organizations cascades `ON DELETE SET NULL` into append-only tables.
//! The cleanup now RETAINS users and orgs referenced by append-only tables
//! (`audit_logs`, `layout_change_events`, `support_tooling_events`) and the
//! re-seed reuses them, so no immutable row is ever mutated and the audit
//! hash chain still verifies.

use db::models::{AuditAction, CreateAuditLog};
use db::repositories::{AuditLogRepository, LayoutChangeEventKind, LayoutRepository};
use db::seed::{SeedConfig, SeedRunner};
use sqlx::PgPool;

fn config(force: bool) -> SeedConfig {
    SeedConfig {
        admin_email: "admin@demo-property.test".into(),
        admin_password: "Seed-Force-Test-1!".into(),
        include_sample_data: false,
        force,
    }
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn force_reseed_keeps_append_only_rows_and_audit_chain_intact(pool: PgPool) {
    let first = SeedRunner::new(pool.clone(), config(false))
        .run()
        .await
        .expect("initial seed");

    // Normal use leaves an audit row and a layout change event by the admin.
    let audit = AuditLogRepository::new(pool.clone());
    let row = audit
        .create(CreateAuditLog {
            user_id: Some(first.admin_user_id),
            action: AuditAction::Login,
            resource_type: Some("user".into()),
            resource_id: Some(first.admin_user_id),
            org_id: Some(first.organization_id),
            details: None,
            old_values: None,
            new_values: None,
            ip_address: None,
            user_agent: None,
        })
        .await
        .expect("insert audit row");
    // Precondition: a freshly written chain verifies (this also guards the
    // microsecond truncation in `AuditLogRepository::create`).
    assert!(audit.verify_integrity(None).await.expect("verify"));
    LayoutRepository::new()
        .record_change_event(
            &pool,
            LayoutChangeEventKind::Published,
            Some("ppt/dashboard"),
            None,
            Some(first.admin_user_id),
            &serde_json::json!({}),
        )
        .await
        .expect("insert layout change event");

    let second = SeedRunner::new(pool.clone(), config(true))
        .run()
        .await
        .expect("--force re-seed must succeed with existing append-only rows");
    assert!(second.cleanup_stats.is_some());

    // The referenced admin and org were reused, not deleted and recreated.
    assert_eq!(second.admin_user_id, first.admin_user_id);
    assert_eq!(second.organization_id, first.organization_id);

    // The audit row is untouched and the hash chain still verifies.
    let (user_id, org_id): (Option<uuid::Uuid>, Option<uuid::Uuid>) =
        sqlx::query_as("SELECT user_id, org_id FROM audit_logs WHERE id = $1")
            .bind(row.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(user_id, Some(first.admin_user_id));
    assert_eq!(org_id, Some(first.organization_id));
    assert!(
        audit.verify_integrity(None).await.expect("verify"),
        "audit hash chain must verify after --force"
    );
    let published_by: Option<uuid::Uuid> = sqlx::query_scalar(
        "SELECT published_by FROM layout_change_events WHERE screen = 'ppt/dashboard'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(published_by, Some(first.admin_user_id));

    // A third forced run still works (idempotent reuse).
    SeedRunner::new(pool.clone(), config(true))
        .run()
        .await
        .expect("second --force re-seed");

    // The append-only triggers were never disabled.
    let err = sqlx::query("UPDATE audit_logs SET resource_type = 'x'")
        .execute(&pool)
        .await
        .expect_err("UPDATE on audit_logs must still be rejected");
    assert!(err.to_string().contains("append-only"), "{err}");
    assert!(sqlx::query("DELETE FROM audit_logs")
        .execute(&pool)
        .await
        .is_err());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn force_reseed_without_audit_rows_still_recreates_everything(pool: PgPool) {
    let first = SeedRunner::new(pool.clone(), config(false))
        .run()
        .await
        .expect("initial seed");
    let second = SeedRunner::new(pool.clone(), config(true))
        .run()
        .await
        .expect("--force re-seed");
    // Unreferenced seed rows are still deleted and recreated.
    assert_ne!(second.organization_id, first.organization_id);
    assert_ne!(second.admin_user_id, first.admin_user_id);
}

/// Gap audit #38: GET /layout/resolved/ppt/dashboard answered 404 "unknown
/// screen" because nothing published that screen. The seed now publishes the
/// baseline layout; it must resolve against the seeded `web` manifest, and
/// re-seeding must not clobber an admin-published version.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn seed_publishes_resolvable_ppt_dashboard_layout(pool: PgPool) {
    SeedRunner::new(pool.clone(), config(false))
        .run()
        .await
        .expect("seed");

    let repo = db::repositories::LayoutRepository::new();
    let mut conn = pool.acquire().await.unwrap();
    let cfg = repo
        .get_config(&mut *conn, "ppt/dashboard")
        .await
        .unwrap()
        .expect("ppt/dashboard config seeded");
    let base: layout_core::ScreenConfig =
        serde_json::from_value(cfg.published.expect("published")).unwrap();
    let manifest_row = repo
        .get_manifest(&mut *conn, "web")
        .await
        .unwrap()
        .expect("web manifest seeded");
    let manifest: layout_core::RegistryManifest =
        serde_json::from_value(manifest_row.manifest).unwrap();
    let resolved = layout_core::resolve(
        &base,
        layout_core::Platform::Web,
        None,
        &Default::default(),
        &manifest,
    );
    let types: Vec<_> = resolved
        .sections
        .iter()
        .map(|s| s.section_type.0.as_str())
        .collect();
    assert_eq!(types, vec!["dashboard-stats.v1", "action-queue.v1"]);

    // Admin publishes v2; a forced re-seed keeps it.
    sqlx::query("UPDATE layout_configs SET published_version = 2 WHERE screen = 'ppt/dashboard'")
        .execute(&pool)
        .await
        .unwrap();
    SeedRunner::new(pool.clone(), config(true))
        .run()
        .await
        .expect("re-seed");
    let v: i32 = sqlx::query_scalar(
        "SELECT published_version FROM layout_configs WHERE screen = 'ppt/dashboard'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(v, 2);
}

/// A database seeded before the layout existed gets it on the next plain
/// `ppt-seed` run, even though that run stops with `AlreadySeeded`.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn already_seeded_database_still_gets_dashboard_layout(pool: PgPool) {
    SeedRunner::new(pool.clone(), config(false))
        .run()
        .await
        .expect("seed");
    sqlx::query("DELETE FROM layout_config_versions WHERE screen = 'ppt/dashboard'")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM layout_configs WHERE screen = 'ppt/dashboard'")
        .execute(&pool)
        .await
        .unwrap();

    let again = SeedRunner::new(pool.clone(), config(false)).run().await;
    assert!(matches!(again, Err(db::seed::SeedError::AlreadySeeded)));
    let n: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM layout_configs WHERE screen = 'ppt/dashboard'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(n, 1);
}
