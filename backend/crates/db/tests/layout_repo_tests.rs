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
