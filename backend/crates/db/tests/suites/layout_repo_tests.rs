use crate::common::{seed_org, set_ctx};
use db::repositories::LayoutRepository;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn draft_upsert_and_get_round_trip(pool: PgPool) {
    let repo = LayoutRepository::new();
    let mut conn = pool.acquire().await.unwrap();

    assert!(repo
        .get_config(&mut *conn, "reality/listing-detail")
        .await
        .unwrap()
        .is_none());

    let draft = json!({"screen": "reality/listing-detail", "version": 0,
                       "sections": [{"type": "gallery.v1"}]});
    let row = repo
        .upsert_draft(&mut *conn, "reality/listing-detail", &draft, None)
        .await
        .unwrap();
    assert_eq!(row.screen, "reality/listing-detail");
    assert_eq!(row.draft, draft);
    assert_eq!(row.published, None);
    assert_eq!(row.published_version, 0);

    // upsert overwrites the draft, keeps identity
    let draft2 = json!({"screen": "reality/listing-detail", "version": 0, "sections": []});
    let row2 = repo
        .upsert_draft(&mut *conn, "reality/listing-detail", &draft2, None)
        .await
        .unwrap();
    assert_eq!(row2.id, row.id);
    assert_eq!(row2.draft, draft2);

    let rails = json!({"reorderable": true});
    let row3 = repo
        .set_rails(&mut *conn, "reality/listing-detail", &rails, None)
        .await
        .unwrap();
    assert_eq!(row3.rails, rails);

    let all = repo.list_configs(&mut *conn).await.unwrap();
    assert_eq!(all.len(), 1);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn publish_snapshots_versions_and_rollback_restores(pool: PgPool) {
    let repo = LayoutRepository::new();
    let mut conn = pool.acquire().await.unwrap();

    let v1 = json!({"screen": "ppt/dashboard", "version": 0,
                    "sections": [{"type": "kpi.v1"}]});
    repo.upsert_draft(&mut *conn, "ppt/dashboard", &v1, None)
        .await
        .unwrap();

    let published = repo
        .publish(&mut conn, "ppt/dashboard", &v1, None)
        .await
        .unwrap();
    assert_eq!(published.published_version, 1);
    assert_eq!(published.published, Some(v1.clone()));

    let v2 = json!({"screen": "ppt/dashboard", "version": 0,
                    "sections": [{"type": "kpi.v1"}, {"type": "news.v1"}]});
    repo.upsert_draft(&mut *conn, "ppt/dashboard", &v2, None)
        .await
        .unwrap();
    let published2 = repo
        .publish(&mut conn, "ppt/dashboard", &v2, None)
        .await
        .unwrap();
    assert_eq!(published2.published_version, 2);
    assert_eq!(published2.published, Some(v2.clone()));

    let versions = repo
        .list_versions(&mut *conn, "ppt/dashboard")
        .await
        .unwrap();
    assert_eq!(
        versions.iter().map(|v| v.version).collect::<Vec<_>>(),
        vec![2, 1]
    );

    // rollback to v1: published AND draft become v1's config, history grows to 3
    let rolled = repo
        .rollback(&mut conn, "ppt/dashboard", 1, None)
        .await
        .unwrap();
    assert_eq!(rolled.published_version, 3);
    assert_eq!(rolled.published, Some(v1.clone()));
    assert_eq!(rolled.draft, v1);
    let versions = repo
        .list_versions(&mut *conn, "ppt/dashboard")
        .await
        .unwrap();
    assert_eq!(versions.len(), 3);

    // unknown screen / version → not-found errors
    assert!(matches!(
        repo.publish(&mut conn, "no/such-screen", &v1, None).await,
        Err(db::repositories::LayoutPublishError::ScreenNotFound)
    ));
    assert!(matches!(
        repo.rollback(&mut conn, "ppt/dashboard", 99, None).await,
        Err(sqlx::Error::RowNotFound)
    ));
}

/// TOCTOU guard: publishing with a stale expected draft (a concurrent draft
/// PUT changed it after validation) must fail with `DraftChanged` and leave
/// published state and version history untouched.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn publish_with_stale_draft_is_rejected(pool: PgPool) {
    let repo = LayoutRepository::new();
    let mut conn = pool.acquire().await.unwrap();

    let validated = json!({"screen": "ppt/dashboard", "version": 0,
                           "sections": [{"type": "kpi.v1"}]});
    repo.upsert_draft(&mut *conn, "ppt/dashboard", &validated, None)
        .await
        .unwrap();

    // Concurrent editor swaps the draft between validation and publish.
    let sneaky = json!({"screen": "ppt/dashboard", "version": 0,
                        "sections": [{"type": "evil.v1"}]});
    repo.upsert_draft(&mut *conn, "ppt/dashboard", &sneaky, None)
        .await
        .unwrap();

    // Publish guarded by the previously validated draft → DraftChanged.
    assert!(matches!(
        repo.publish(&mut conn, "ppt/dashboard", &validated, None)
            .await,
        Err(db::repositories::LayoutPublishError::DraftChanged)
    ));

    // Nothing was published, no version snapshot recorded.
    let cfg = repo
        .get_config(&mut *conn, "ppt/dashboard")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(cfg.published, None);
    assert_eq!(cfg.published_version, 0);
    assert!(repo
        .list_versions(&mut *conn, "ppt/dashboard")
        .await
        .unwrap()
        .is_empty());

    // Publishing the CURRENT draft still works.
    let ok = repo
        .publish(&mut conn, "ppt/dashboard", &sneaky, None)
        .await
        .unwrap();
    assert_eq!(ok.published_version, 1);
    assert_eq!(ok.published, Some(sneaky));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn kills_and_manifests_round_trip(pool: PgPool) {
    let repo = LayoutRepository::new();
    let mut conn = pool.acquire().await.unwrap();

    repo.kill(&mut *conn, "ppt/dashboard", "news.v1", None)
        .await
        .unwrap();
    // idempotent
    repo.kill(&mut *conn, "ppt/dashboard", "news.v1", None)
        .await
        .unwrap();
    let kills = repo.list_kills(&mut *conn, "ppt/dashboard").await.unwrap();
    assert_eq!(kills.len(), 1);
    assert_eq!(kills[0].section_type, "news.v1");

    assert!(repo
        .unkill(&mut *conn, "ppt/dashboard", "news.v1")
        .await
        .unwrap());
    assert!(!repo
        .unkill(&mut *conn, "ppt/dashboard", "news.v1")
        .await
        .unwrap());
    assert!(repo
        .list_kills(&mut *conn, "ppt/dashboard")
        .await
        .unwrap()
        .is_empty());

    let manifest = json!({"platform": "web", "components": {"kpi.v1": {"required": true}}});
    let row = repo
        .upsert_manifest(&mut *conn, "web", &manifest, None)
        .await
        .unwrap();
    assert_eq!(row.manifest, manifest);
    assert!(repo
        .get_manifest(&mut *conn, "mobile")
        .await
        .unwrap()
        .is_none());
    assert_eq!(repo.list_manifests(&mut *conn).await.unwrap().len(), 1);
}

/// RLS on `layout_tenant_overrides` must hide org-A's override from an
/// org-B caller. `#[sqlx::test]` connects as SUPERUSER which bypasses FORCE
/// RLS, so we mirror the canonical sensor_rls_repo_tests.rs pattern: create a
/// NOSUPERUSER NOBYPASSRLS role, grant it the required privileges, then SET
/// ROLE on a single dedicated connection so FORCE actually binds.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn tenant_overrides_are_org_isolated(pool: PgPool) {
    let repo = LayoutRepository::new();

    // Seed two organizations as superuser (satisfies the RLS WITH CHECK on
    // organizations that INSERT fires under super-admin context).
    set_ctx(&pool, None, None, true).await;
    let org_a = seed_org(&pool, "layout-org-a").await;
    let org_b = seed_org(&pool, "layout-org-b").await;

    // Seed org_a's override as superuser so the INSERT bypasses FORCE RLS.
    let ov = json!({"sections": {"news.v1": {"visible": false}}});
    {
        let mut su_conn = pool.acquire().await.unwrap();
        repo.upsert_tenant_override(&mut *su_conn, org_a, "ppt/dashboard", &ov, None)
            .await
            .unwrap();
    }

    // Create a NOSUPERUSER NOBYPASSRLS role so FORCE actually binds.
    let role = format!("ppt_rls_layout_{}", Uuid::new_v4().simple());
    for stmt in [
        format!("CREATE ROLE \"{role}\" NOSUPERUSER NOBYPASSRLS"),
        format!("GRANT SELECT, INSERT, UPDATE ON layout_tenant_overrides TO \"{role}\""),
        format!("GRANT EXECUTE ON FUNCTION get_current_org_id(), is_super_admin() TO \"{role}\""),
        format!("GRANT SELECT ON organizations TO \"{role}\""),
    ] {
        sqlx::query(sqlx::AssertSqlSafe(stmt))
            .execute(&pool)
            .await
            .expect("grant");
    }

    // All role-bound work on ONE dedicated connection — SET ROLE and the GUC
    // are session state and must not be spread across pool acquisitions.
    {
        let mut conn = pool.acquire().await.unwrap();

        // Phase 1: org_a context → own override is visible.
        db::set_request_context(&mut *conn, Some(org_a), None, false)
            .await
            .unwrap();
        sqlx::query(sqlx::AssertSqlSafe(format!("SET ROLE \"{role}\"")))
            .execute(&mut *conn)
            .await
            .expect("set role");

        let own = repo
            .get_tenant_override(&mut *conn, org_a, "ppt/dashboard")
            .await
            .expect("get_tenant_override (org-A own)");
        assert!(
            own.is_some(),
            "org-A must see its own override under org-A context"
        );

        // Phase 2: switch to org_b context on the same connection — org_a's
        // override must become invisible (cross-tenant IDOR blocked by FORCE RLS).
        sqlx::query("RESET ROLE")
            .execute(&mut *conn)
            .await
            .expect("reset role before ctx switch");
        db::set_request_context(&mut *conn, Some(org_b), None, false)
            .await
            .unwrap();
        sqlx::query(sqlx::AssertSqlSafe(format!("SET ROLE \"{role}\"")))
            .execute(&mut *conn)
            .await
            .expect("set role");

        let cross = repo
            .get_tenant_override(&mut *conn, org_a, "ppt/dashboard")
            .await
            .expect("get_tenant_override (cross-tenant)");
        assert!(
            cross.is_none(),
            "org-B must NOT see org-A's override (cross-tenant IDOR blocked by RLS)"
        );

        // Drop back to superuser before returning connection to the pool.
        sqlx::query("RESET ROLE")
            .execute(&mut *conn)
            .await
            .expect("reset role");
        db::clear_request_context(&mut *conn).await.unwrap();
    }

    // Cleanup the test role.
    set_ctx(&pool, None, None, true).await;
    for stmt in [
        format!("REVOKE ALL ON layout_tenant_overrides FROM \"{role}\""),
        format!("REVOKE ALL ON organizations FROM \"{role}\""),
        format!("DROP ROLE IF EXISTS \"{role}\""),
    ] {
        sqlx::query(sqlx::AssertSqlSafe(stmt))
            .execute(&pool)
            .await
            .ok();
    }
}
