//! Announcement read-path residency revocation tests
//! (PM #981 gap 6 / Story 3.3 AC3).
//!
//! Background
//! ----------
//! `AnnouncementRepository::list_published_rls` / `count_published_rls` gate the
//! visibility of published announcements with `announcement_targeting_predicate!`
//! (`announcement.rs`). For `building`- and `units`-targeted announcements the
//! predicate requires an ACTIVE residency:
//!
//! ```sql
//! EXISTS (SELECT 1 FROM unit_residents ur ...
//!         WHERE ur.user_id = current_setting('app.current_user_id') ...
//!           AND ur.end_date IS NULL ...)
//! ```
//!
//! AC3 requires that when a manager ends a resident's association the resident
//! "loses access to unit content" while "historical records are preserved".
//! This test pins that the predicate enforces revocation on move-out:
//!   - a current resident (`end_date IS NULL`) sees the building/unit-targeted
//!     announcement,
//!   - a moved-out resident (past `end_date`) does NOT, and
//!   - an `all`-targeted (org-wide) announcement stays visible to both, because
//!     org membership is separate from unit residency.
//!
//! The targeting predicate reads `app.current_user_id`, set on the connection via
//! `db::set_request_context` — the same call `RlsConnection` makes in production.
//! The CI/test database role bypasses RLS, so these assertions pin the
//! application-level predicate inside the query.

#[allow(dead_code)]
mod common;

use db::repositories::AnnouncementRepository;
use sqlx::PgPool;
use uuid::Uuid;

use common::seed_org;

async fn seed_user(pool: &PgPool, label: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO users (email, password_hash, name)
           VALUES ($1, 'x', $2) RETURNING id"#,
    )
    .bind(format!("{label}-{}@ann-res.test", Uuid::new_v4()))
    .bind(format!("Ann Res {label}"))
    .fetch_one(pool)
    .await
    .expect("seed_user")
}

async fn seed_building(pool: &PgPool, org_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO buildings (organization_id, name, street, city, postal_code, country)
           VALUES ($1, 'Bldg', 'Addr', 'City', '00000', 'Country') RETURNING id"#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed_building")
}

async fn seed_unit(pool: &PgPool, building_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO units (building_id, designation) VALUES ($1, 'U-1') RETURNING id"#,
    )
    .bind(building_id)
    .fetch_one(pool)
    .await
    .expect("seed_unit")
}

/// Insert a `unit_residents` row; `ended` makes it inactive (moved out 30d ago).
async fn seed_resident(pool: &PgPool, unit_id: Uuid, user_id: Uuid, ended: bool) {
    let sql = if ended {
        // `start_date` precedes `end_date` to satisfy the `valid_date_range`
        // check constraint (end_date >= start_date).
        r#"INSERT INTO unit_residents (unit_id, user_id, resident_type, start_date, end_date)
           VALUES ($1, $2, 'tenant', CURRENT_DATE - 60, CURRENT_DATE - 30)"#
    } else {
        r#"INSERT INTO unit_residents (unit_id, user_id, resident_type)
           VALUES ($1, $2, 'tenant')"#
    };
    sqlx::query(sql)
        .bind(unit_id)
        .bind(user_id)
        .execute(pool)
        .await
        .expect("seed_resident");
}

/// Insert a published announcement and return its id. `target_ids` is the JSONB
/// array of building/unit uuids (empty for `all`).
async fn seed_published_announcement(
    pool: &PgPool,
    org_id: Uuid,
    author_id: Uuid,
    title: &str,
    target_type: &str,
    target_ids: &[Uuid],
) -> Uuid {
    let target_json =
        serde_json::json!(target_ids.iter().map(Uuid::to_string).collect::<Vec<_>>());
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO announcements
               (organization_id, author_id, title, content, target_type, target_ids,
                status, published_at)
           VALUES ($1, $2, $3, 'body', $4::announcement_target_type, $5::jsonb,
                   'published', NOW())
           RETURNING id"#,
    )
    .bind(org_id)
    .bind(author_id)
    .bind(title)
    .bind(target_type)
    .bind(target_json)
    .fetch_one(pool)
    .await
    .expect("seed_published_announcement")
}

/// Story 3.3 AC3: building/unit-targeted announcements are revoked on move-out.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn published_announcements_revoke_on_move_out(pool: PgPool) {
    let repo = AnnouncementRepository::new(pool.clone());

    let manager = seed_user(&pool, "manager").await;
    let org = seed_org(&pool, "ann-moveout").await;
    let building = seed_building(&pool, org).await;
    let unit = seed_unit(&pool, building).await;

    let current = seed_user(&pool, "current").await;
    let ended = seed_user(&pool, "ended").await;
    seed_resident(&pool, unit, current, false).await;
    seed_resident(&pool, unit, ended, true).await;

    seed_published_announcement(&pool, org, manager, "Org Wide", "all", &[]).await;
    seed_published_announcement(&pool, org, manager, "Building Notice", "building", &[building])
        .await;
    seed_published_announcement(&pool, org, manager, "Unit Notice", "units", &[unit]).await;

    let mut conn = pool.acquire().await.expect("acquire");

    // ---- Current resident sees org-wide + building + unit announcements ----
    db::set_request_context(&mut *conn, Some(org), Some(current), false)
        .await
        .expect("set context (current)");
    let current_titles: Vec<String> = repo
        .list_published_rls(&mut *conn, org, None, None)
        .await
        .expect("list (current)")
        .into_iter()
        .map(|a| a.title)
        .collect();
    assert!(
        current_titles.iter().any(|t| t == "Org Wide")
            && current_titles.iter().any(|t| t == "Building Notice")
            && current_titles.iter().any(|t| t == "Unit Notice"),
        "current resident must see org-wide + building + unit announcements: {current_titles:?}"
    );

    // ---- Moved-out resident sees ONLY the org-wide announcement ----
    db::set_request_context(&mut *conn, Some(org), Some(ended), false)
        .await
        .expect("set context (ended)");
    let ended_titles: Vec<String> = repo
        .list_published_rls(&mut *conn, org, None, None)
        .await
        .expect("list (ended)")
        .into_iter()
        .map(|a| a.title)
        .collect();
    assert!(
        ended_titles.iter().any(|t| t == "Org Wide"),
        "moved-out resident keeps org-wide announcements: {ended_titles:?}"
    );
    assert!(
        !ended_titles.iter().any(|t| t == "Building Notice")
            && !ended_titles.iter().any(|t| t == "Unit Notice"),
        "moved-out resident must NOT see building/unit announcements: {ended_titles:?}"
    );
}
