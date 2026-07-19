use db::repositories::LayoutRepository;
use serde_json::json;
use sqlx::PgPool;

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

    let published = repo.publish(&mut *conn, "ppt/dashboard", None).await.unwrap();
    assert_eq!(published.published_version, 1);
    assert_eq!(published.published, Some(v1.clone()));

    let v2 = json!({"screen": "ppt/dashboard", "version": 0,
                    "sections": [{"type": "kpi.v1"}, {"type": "news.v1"}]});
    repo.upsert_draft(&mut *conn, "ppt/dashboard", &v2, None)
        .await
        .unwrap();
    let published2 = repo.publish(&mut *conn, "ppt/dashboard", None).await.unwrap();
    assert_eq!(published2.published_version, 2);
    assert_eq!(published2.published, Some(v2.clone()));

    let versions = repo.list_versions(&mut *conn, "ppt/dashboard").await.unwrap();
    assert_eq!(
        versions.iter().map(|v| v.version).collect::<Vec<_>>(),
        vec![2, 1]
    );

    // rollback to v1: published AND draft become v1's config, history grows to 3
    let rolled = repo.rollback(&mut *conn, "ppt/dashboard", 1, None).await.unwrap();
    assert_eq!(rolled.published_version, 3);
    assert_eq!(rolled.published, Some(v1.clone()));
    assert_eq!(rolled.draft, v1);
    let versions = repo.list_versions(&mut *conn, "ppt/dashboard").await.unwrap();
    assert_eq!(versions.len(), 3);

    // unknown screen / version → RowNotFound
    assert!(matches!(
        repo.publish(&mut *conn, "no/such-screen", None).await,
        Err(sqlx::Error::RowNotFound)
    ));
    assert!(matches!(
        repo.rollback(&mut *conn, "ppt/dashboard", 99, None).await,
        Err(sqlx::Error::RowNotFound)
    ));
}
