//! Regression test for issue #920 — announcement targeting must be enforced on
//! the published-announcement read path, not just notification routing.
//!
//! Before the fix `AnnouncementRepository::list_published_rls` /
//! `count_published_rls` returned every published announcement in the org,
//! ignoring `target_type`/`target_ids`, so any authenticated user saw
//! announcements aimed at buildings / units / roles they don't belong to
//! (intra-org disclosure). After the fix the feed is a personal feed for
//! everyone: a user sees `target_type = 'all'` announcements plus those
//! targeting their own building, unit, or org role. There is no manager bypass
//! on this endpoint — managers get full visibility via the management list.
//!
//! Targeting contract (migration 00015): `building`/`units` target_ids hold the
//! UUIDs as text; `roles` target_ids hold role-name strings matching
//! `user_memberships.role`.

use db::repositories::AnnouncementRepository;
use serde_json::Value as Json;
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

async fn set_ctx(conn: &mut PgConnection, org: Option<Uuid>, user: Option<Uuid>, su: bool) {
    sqlx::query("SELECT set_request_context($1, $2, $3)")
        .bind(org)
        .bind(user)
        .bind(su)
        .execute(conn)
        .await
        .expect("set_request_context");
}

async fn insert_org(conn: &mut PgConnection, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO organizations (name, slug, contact_email, status) \
         VALUES ($1, $2, $3, 'active') RETURNING id",
    )
    .bind(format!("Issue 920 {slug}"))
    .bind(slug)
    .bind(format!("{slug}@issue920.test"))
    .fetch_one(conn)
    .await
    .expect("insert org")
}

async fn insert_user(conn: &mut PgConnection, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO users (email, password_hash, name, status, email_verified_at) \
         VALUES ($1, 'test_hash', 'U920', 'active', NOW()) RETURNING id",
    )
    .bind(email)
    .fetch_one(conn)
    .await
    .expect("insert user")
}

async fn insert_building(conn: &mut PgConnection, org: Uuid, name: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO buildings (organization_id, name, street, city, postal_code, country) \
         VALUES ($1, $2, 'Addr', 'City', '00000', 'Country') RETURNING id",
    )
    .bind(org)
    .bind(name)
    .fetch_one(conn)
    .await
    .expect("insert building")
}

async fn insert_unit(conn: &mut PgConnection, building: Uuid, designation: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO units (building_id, designation) VALUES ($1, $2) RETURNING id",
    )
    .bind(building)
    .bind(designation)
    .fetch_one(conn)
    .await
    .expect("insert unit")
}

async fn insert_resident(conn: &mut PgConnection, unit: Uuid, user: Uuid, kind: &str) {
    sqlx::query(
        "INSERT INTO unit_residents (unit_id, user_id, resident_type) \
         VALUES ($1, $2, $3::resident_type)",
    )
    .bind(unit)
    .bind(user)
    .bind(kind)
    .execute(conn)
    .await
    .expect("insert unit resident");
}

async fn insert_membership(conn: &mut PgConnection, user: Uuid, org: Uuid, role: &str) {
    sqlx::query(
        "INSERT INTO user_memberships (user_id, organization_id, role) VALUES ($1, $2, $3)",
    )
    .bind(user)
    .bind(org)
    .bind(role)
    .execute(conn)
    .await
    .expect("insert membership");
}

async fn insert_published_ann(
    conn: &mut PgConnection,
    org: Uuid,
    author: Uuid,
    target_type: &str,
    target_ids: Json,
    title: &str,
) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO announcements \
            (organization_id, author_id, title, content, target_type, target_ids, status, published_at) \
         VALUES ($1, $2, $3, 'body', $4::announcement_target_type, $5, 'published', NOW()) \
         RETURNING id",
    )
    .bind(org)
    .bind(author)
    .bind(title)
    .bind(target_type)
    .bind(target_ids)
    .fetch_one(conn)
    .await
    .expect("insert published announcement")
}

/// Run `list_published_rls` as `user` (RLS context set) and return the ids.
async fn visible_ids(
    pool: &PgPool,
    repo: &AnnouncementRepository,
    org: Uuid,
    user: Uuid,
) -> Vec<Uuid> {
    let mut conn = pool.acquire().await.unwrap();
    set_ctx(&mut conn, Some(org), Some(user), false).await;
    repo.list_published_rls(&mut *conn, org, None, None)
        .await
        .expect("list_published_rls")
        .into_iter()
        .map(|a| a.id)
        .collect()
}

/// Run `count_published_rls` as `user` (RLS context set).
async fn visible_count(pool: &PgPool, repo: &AnnouncementRepository, org: Uuid, user: Uuid) -> i64 {
    let mut conn = pool.acquire().await.unwrap();
    set_ctx(&mut conn, Some(org), Some(user), false).await;
    repo.count_published_rls(&mut *conn, org)
        .await
        .expect("count_published_rls")
}

#[sqlx::test(migrations = "./migrations")]
async fn published_feed_is_filtered_by_targeting(pool: PgPool) {
    // ---- seed everything under a super-admin context (bypasses RLS WITH CHECK) ----
    let mut sa = pool.acquire().await.unwrap();
    set_ctx(&mut sa, None, None, true).await;

    let org = insert_org(&mut sa, "vis").await;
    let author = insert_user(&mut sa, "author920@test.com").await;

    let building_a = insert_building(&mut sa, org, "Bldg A").await;
    let building_b = insert_building(&mut sa, org, "Bldg B").await;
    let unit_a1 = insert_unit(&mut sa, building_a, "A1").await;
    let unit_b1 = insert_unit(&mut sa, building_b, "B1").await;

    // resident_a lives in building_a / unit_a1; resident_b in building_b / unit_b1;
    // manager has the 'manager' org role but no residency.
    let resident_a = insert_user(&mut sa, "resident_a920@test.com").await;
    let resident_b = insert_user(&mut sa, "resident_b920@test.com").await;
    let manager = insert_user(&mut sa, "manager920@test.com").await;

    insert_membership(&mut sa, resident_a, org, "resident").await;
    insert_membership(&mut sa, resident_b, org, "tenant").await;
    insert_membership(&mut sa, manager, org, "manager").await;

    insert_resident(&mut sa, unit_a1, resident_a, "owner").await;
    insert_resident(&mut sa, unit_b1, resident_b, "tenant").await;

    let ann_all =
        insert_published_ann(&mut sa, org, author, "all", serde_json::json!([]), "All").await;
    let ann_building_a = insert_published_ann(
        &mut sa,
        org,
        author,
        "building",
        serde_json::json!([building_a.to_string()]),
        "Building A only",
    )
    .await;
    let ann_unit_a1 = insert_published_ann(
        &mut sa,
        org,
        author,
        "units",
        serde_json::json!([unit_a1.to_string()]),
        "Unit A1 only",
    )
    .await;
    let ann_role_manager = insert_published_ann(
        &mut sa,
        org,
        author,
        "roles",
        serde_json::json!(["manager"]),
        "Managers only",
    )
    .await;
    drop(sa);

    let repo = AnnouncementRepository::new(pool.clone());

    // resident_a: 'all' + building A + unit A1, but NOT the manager-role one.
    let a = visible_ids(&pool, &repo, org, resident_a).await;
    assert!(
        a.contains(&ann_all),
        "resident_a must see the 'all' announcement"
    );
    assert!(a.contains(&ann_building_a), "resident_a is in building A");
    assert!(a.contains(&ann_unit_a1), "resident_a is in unit A1");
    assert!(
        !a.contains(&ann_role_manager),
        "resident_a is not a manager — must NOT see the manager-targeted announcement"
    );
    assert_eq!(a.len(), 3, "resident_a sees exactly all + building + unit");
    assert_eq!(visible_count(&pool, &repo, org, resident_a).await, 3);

    // resident_b: only the org-wide 'all' announcement (different building/unit, non-manager).
    let b = visible_ids(&pool, &repo, org, resident_b).await;
    assert!(b.contains(&ann_all));
    assert!(
        !b.contains(&ann_building_a) && !b.contains(&ann_unit_a1) && !b.contains(&ann_role_manager),
        "resident_b must NOT see announcements targeted at building A / unit A1 / managers"
    );
    assert_eq!(b.len(), 1, "resident_b sees only the 'all' announcement");
    assert_eq!(visible_count(&pool, &repo, org, resident_b).await, 1);

    // manager: 'all' + the manager-role one; no residency, so no building/unit ones
    // (verifies there is NO manager bypass on this feed).
    let m = visible_ids(&pool, &repo, org, manager).await;
    assert!(m.contains(&ann_all));
    assert!(
        m.contains(&ann_role_manager),
        "manager role matches the role target"
    );
    assert!(
        !m.contains(&ann_building_a) && !m.contains(&ann_unit_a1),
        "manager has no residency — building/unit targeted announcements stay hidden (no bypass)"
    );
    assert_eq!(m.len(), 2, "manager sees exactly all + role");
    assert_eq!(visible_count(&pool, &repo, org, manager).await, 2);
}
