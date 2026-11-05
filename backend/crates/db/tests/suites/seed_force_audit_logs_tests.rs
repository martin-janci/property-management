//! Regression test: `ppt-seed --force` failed with
//! `audit_logs is append-only: UPDATE rejected` because deleting the seed
//! users/organizations cascades `ON DELETE SET NULL` into `audit_logs`, which
//! the append-only trigger rejects. The cleanup now disables only that
//! trigger inside its own transaction; afterwards the append-only guarantee
//! must hold again for normal sessions.

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
async fn force_reseed_succeeds_with_audit_rows_and_keeps_append_only(pool: PgPool) {
    let first = SeedRunner::new(pool.clone(), config(false))
        .run()
        .await
        .expect("initial seed");

    // An audit row pointing at the seed admin and org, as normal use creates.
    sqlx::query(
        "INSERT INTO audit_logs (user_id, org_id, action, resource_type) \
         VALUES ($1, $2, 'login', 'user')",
    )
    .bind(first.admin_user_id)
    .bind(first.organization_id)
    .execute(&pool)
    .await
    .expect("insert audit row");

    let second = SeedRunner::new(pool.clone(), config(true))
        .run()
        .await
        .expect("--force re-seed must succeed with existing audit rows");
    assert!(second.cleanup_stats.is_some());

    // The historical audit row is preserved (FKs nulled), not deleted.
    let kept: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_logs WHERE resource_type = 'user' AND user_id IS NULL",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(kept, 1);

    // The append-only trigger is enabled again for every other session.
    let update = sqlx::query("UPDATE audit_logs SET resource_type = 'x'")
        .execute(&pool)
        .await;
    let err = update.expect_err("UPDATE on audit_logs must still be rejected");
    assert!(err.to_string().contains("append-only"), "{err}");
    let delete = sqlx::query("DELETE FROM audit_logs").execute(&pool).await;
    assert!(
        delete.is_err(),
        "DELETE on audit_logs must still be rejected"
    );
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
